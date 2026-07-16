mod setup;

use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;
use serde_json::json;
use sha2::{Digest, Sha512};

use crate::admission::ResponseCategory;

use setup::{
    build_tgz_from_package_json, packument_bytes, read_fixture_package_json, spawn_proxy_server,
    spawn_proxy_server_with_audit_sink, spawn_upstream_https_server,
    spawn_upstream_https_server_for_packument_and_tarball,
};

#[test]
fn response_category_status_returns_unprocessable_entity_for_blocked_parse() {
    assert_eq!(
        super::response_category_status(&ResponseCategory::BlockedParse),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[test]
fn response_category_status_returns_forbidden_for_blocked_policy() {
    assert_eq!(
        super::response_category_status(&ResponseCategory::BlockedPolicy),
        StatusCode::FORBIDDEN
    );
}

#[test]
fn response_category_status_returns_forbidden_for_blocked_integrity() {
    assert_eq!(
        super::response_category_status(&ResponseCategory::BlockedIntegrity),
        StatusCode::FORBIDDEN
    );
}

#[test]
fn response_category_status_returns_bad_gateway_for_blocked_fetch() {
    assert_eq!(
        super::response_category_status(&ResponseCategory::BlockedFetch),
        StatusCode::BAD_GATEWAY
    );
}

#[test]
fn response_category_status_returns_internal_server_error_for_error() {
    assert_eq!(
        super::response_category_status(&ResponseCategory::Error),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[test]
#[should_panic(expected = "Admitted responses are returned directly")]
fn response_category_status_panics_for_admitted() {
    super::response_category_status(&ResponseCategory::Admitted);
}

#[test]
fn finding_id_description_returns_known_description_for_install_scripts_disallowed() {
    assert_eq!(
        super::finding_id_description("install-scripts-disallowed"),
        "package declares install hooks"
    );
}

#[test]
fn finding_id_description_returns_raw_id_for_unrecognized_finding_id() {
    assert_eq!(
        super::finding_id_description("some-future-rule-id"),
        "some-future-rule-id"
    );
}

#[test]
fn format_policy_block_message_formats_single_finding() {
    assert_eq!(
        super::format_policy_block_message(
            "esbuild",
            "0.28.1",
            &["install-scripts-disallowed".to_string()]
        ),
        "esbuild@0.28.1 blocked: package declares install hooks [findingID: install-scripts-disallowed]"
    );
}

#[test]
fn format_policy_block_message_formats_multiple_findings() {
    assert_eq!(
        super::format_policy_block_message(
            "demo",
            "1.0.0",
            &[
                "install-scripts-disallowed".to_string(),
                "suspicious-file-detected".to_string()
            ]
        ),
        "demo@1.0.0 blocked: package declares install hooks; package contains suspicious files [findingID: install-scripts-disallowed, suspicious-file-detected]"
    );
}

#[test]
fn valid_artifact_key_from_filename_returns_key_for_lowercase_hex_filename() {
    let artifact_key = "a".repeat(64);
    let filename = format!("{artifact_key}.tgz");

    assert_eq!(
        super::valid_artifact_key_from_filename(&filename),
        Some(artifact_key.as_str())
    );
}

#[test]
fn valid_artifact_key_from_filename_returns_none_for_uppercase_hex_filename() {
    let filename = format!("{}.tgz", "A".repeat(64));

    assert_eq!(super::valid_artifact_key_from_filename(&filename), None);
}

#[tokio::test]
async fn temp_file_removes_file_on_drop() {
    let path = std::env::temp_dir().join(format!("remnant-test-{}.tgz", uuid::Uuid::new_v4()));
    tokio::fs::write(&path, b"test").await.unwrap();
    assert!(path.exists(), "file should exist before drop");

    drop(super::TempFile { path: path.clone() });

    let deleted = async {
        loop {
            if !path.exists() {
                return;
            }
            tokio::task::yield_now().await;
        }
    };

    tokio::time::timeout(std::time::Duration::from_secs(1), deleted)
        .await
        .expect("TempFile drop should remove the file within 1 second");
}

#[tokio::test]
async fn metadata_route_returns_rewritten_packument_json() {
    let upstream_response = br#"{"name":"left-pad","versions":{"1.3.0":{"dist":{"tarball":"https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz","integrity":"sha512-abc123=="}}}}"#;
    let (upstream_registry_url, upstream_request) =
        spawn_upstream_https_server(upstream_response.to_vec()).await;
    let (proxy_base_url, proxy_server) = spawn_proxy_server(&upstream_registry_url).await;

    let response = reqwest::Client::new()
        .get(format!("{proxy_base_url}/left-pad"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/json")
    );

    let body = serde_json::from_slice::<Value>(&response.bytes().await.unwrap()).unwrap();
    let rewritten_tarball = body["versions"]["1.3.0"]["dist"]["tarball"]
        .as_str()
        .unwrap();
    let proxy_port = proxy_base_url.rsplit(':').next().unwrap();

    assert!(
        rewritten_tarball.starts_with(&format!("http://localhost:{proxy_port}/remnant/tarballs/"))
    );

    let _request = upstream_request.await.unwrap();
    proxy_server.abort();
}

#[tokio::test]
async fn metadata_route_returns_rewritten_packument_for_scoped_package_name() {
    let upstream_response = packument_bytes(
        "@babel/core",
        "7.0.0",
        "https://registry.npmjs.org/@babel/core/-/core-7.0.0.tgz",
        Some("sha512-abc123=="),
    );
    let (upstream_registry_url, upstream_request) =
        spawn_upstream_https_server(upstream_response).await;
    let (proxy_base_url, proxy_server) = spawn_proxy_server(&upstream_registry_url).await;

    let response = reqwest::Client::new()
        .get(format!("{proxy_base_url}/@babel%2Fcore"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let body = serde_json::from_slice::<Value>(&response.bytes().await.unwrap()).unwrap();
    let rewritten_tarball = body["versions"]["7.0.0"]["dist"]["tarball"]
        .as_str()
        .unwrap();
    let proxy_port = proxy_base_url.rsplit(':').next().unwrap();

    assert!(
        rewritten_tarball.starts_with(&format!("http://localhost:{proxy_port}/remnant/tarballs/"))
    );

    let request = upstream_request.await.unwrap();
    assert!(
        request.starts_with("GET /@babel%2Fcore "),
        "upstream should receive scoped package name as a single percent-encoded path segment"
    );

    proxy_server.abort();
}

#[tokio::test]
async fn metadata_route_returns_blocked_fetch_for_upstream_connection_failure() {
    let (proxy_base_url, proxy_server) = spawn_proxy_server("https://127.0.0.1:1").await;

    let response = reqwest::Client::new()
        .get(format!("{proxy_base_url}/left-pad"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);

    let body = serde_json::from_slice::<Value>(&response.bytes().await.unwrap()).unwrap();

    assert_eq!(body["category"], "blocked_fetch");
    assert_eq!(body["findingIds"], serde_json::json!([]));
    assert!(
        body["requestId"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );

    proxy_server.abort();
}

#[tokio::test]
async fn metadata_route_returns_blocked_parse_for_invalid_packument_json() {
    let (upstream_registry_url, upstream_request) =
        spawn_upstream_https_server(b"not json".to_vec()).await;
    let (proxy_base_url, proxy_server) = spawn_proxy_server(&upstream_registry_url).await;

    let response = reqwest::Client::new()
        .get(format!("{proxy_base_url}/left-pad"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);

    let body = serde_json::from_slice::<Value>(&response.bytes().await.unwrap()).unwrap();

    assert_eq!(body["category"], "blocked_parse");
    assert_eq!(body["error"], "package metadata could not be parsed");
    assert_eq!(body["findingIds"], serde_json::json!([]));
    assert!(
        body["requestId"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );

    let _request = upstream_request.await.unwrap();
    proxy_server.abort();
}

#[tokio::test]
async fn upstream_https_server_for_two_requests_serves_sequential_responses() {
    let (upstream_registry_url, upstream_server) =
        setup::spawn_upstream_https_server_for_two_requests(
            b"first".to_vec(),
            "application/json",
            b"second".to_vec(),
            "application/octet-stream",
        )
        .await;
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let first_response = client
        .get(format!("{upstream_registry_url}/first"))
        .send()
        .await
        .unwrap();
    assert_eq!(first_response.bytes().await.unwrap(), "first");

    let second_response = client
        .get(format!("{upstream_registry_url}/second"))
        .send()
        .await
        .unwrap();

    assert_eq!(second_response.bytes().await.unwrap(), "second");

    upstream_server.await.unwrap();
}

#[tokio::test]
async fn tarball_route_returns_not_found_for_unknown_artifact_key() {
    let (proxy_base_url, proxy_server) = spawn_proxy_server("https://127.0.0.1:1").await;

    let response = reqwest::Client::new()
        .get(format!(
            "{proxy_base_url}/remnant/tarballs/0000000000000000000000000000000000000000000000000000000000000000.tgz"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_returns_not_found_for_short_artifact_key() {
    let (proxy_base_url, proxy_server) = spawn_proxy_server("https://127.0.0.1:1").await;

    let response = reqwest::Client::new()
        .get(format!(
            "{proxy_base_url}/remnant/tarballs/000000000000000000000000000000000000000000000000000000000000000.tgz"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_returns_not_found_for_long_artifact_key() {
    let (proxy_base_url, proxy_server) = spawn_proxy_server("https://127.0.0.1:1").await;

    let response = reqwest::Client::new()
        .get(format!(
            "{proxy_base_url}/remnant/tarballs/00000000000000000000000000000000000000000000000000000000000000000.tgz"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_returns_not_found_for_unknown_uppercase_artifact_key() {
    let (proxy_base_url, proxy_server) = spawn_proxy_server("https://127.0.0.1:1").await;

    let response = reqwest::Client::new()
        .get(format!(
            "{proxy_base_url}/remnant/tarballs/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA.tgz"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_returns_not_found_for_malformed_filename() {
    let (proxy_base_url, proxy_server) = spawn_proxy_server("https://127.0.0.1:1").await;

    let response = reqwest::Client::new()
        .get(format!("{proxy_base_url}/remnant/tarballs/not-a-valid-key"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_returns_200_and_bytes_for_admitted_artifact() {
    let package_json = read_fixture_package_json("benign", "minimal-package");
    let tgz_bytes = build_tgz_from_package_json(&package_json);
    let integrity = sha512_integrity_for(&tgz_bytes);
    let (upstream_registry_url, upstream_server) =
        spawn_upstream_https_server_for_packument_and_tarball(
            "minimal-package",
            "1.0.0",
            "/minimal-package/-/minimal-package-1.0.0.tgz",
            Some(&integrity),
            tgz_bytes.clone(),
        )
        .await;
    let (proxy_base_url, proxy_server) = spawn_proxy_server(&upstream_registry_url).await;

    let rewritten_tarball = fetch_rewritten_tarball_url(&proxy_base_url, "minimal-package").await;
    let response = reqwest::Client::new()
        .get(rewritten_tarball)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap(),
        "application/octet-stream"
    );
    assert_eq!(response.bytes().await.unwrap(), tgz_bytes);

    upstream_server.await.unwrap();
    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_emits_audit_record_for_admitted_artifact() {
    let package_json = read_fixture_package_json("benign", "minimal-package");
    let tgz_bytes = build_tgz_from_package_json(&package_json);
    let integrity = sha512_integrity_for(&tgz_bytes);
    let (upstream_registry_url, upstream_server) =
        spawn_upstream_https_server_for_packument_and_tarball(
            "minimal-package",
            "1.0.0",
            "/minimal-package/-/minimal-package-1.0.0.tgz",
            Some(&integrity),
            tgz_bytes,
        )
        .await;
    let (proxy_base_url, proxy_server, mut audit_rx) =
        spawn_proxy_server_with_audit_sink(&upstream_registry_url).await;

    let rewritten_tarball = fetch_rewritten_tarball_url(&proxy_base_url, "minimal-package").await;
    let response = reqwest::Client::new()
        .get(rewritten_tarball)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    let record_line = tokio::time::timeout(std::time::Duration::from_secs(5), audit_rx.recv())
        .await
        .expect("audit record should be received within 5 seconds")
        .expect("audit channel should not be closed before record is sent");

    let record = serde_json::from_str::<Value>(&record_line).unwrap();

    assert_eq!(record["responseCategory"], "admitted");
    assert_eq!(record["packageName"], "minimal-package");
    assert_eq!(record["version"], "1.0.0");
    assert_eq!(record["integrityStatus"], "verified");
    assert!(
        record["artifactKey"]
            .as_str()
            .is_some_and(|key| key.len() == 64)
    );
    assert!(record["durationMs"].is_number());

    upstream_server.await.unwrap();
    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_returns_403_for_blocked_policy_artifact() {
    let package_json = read_fixture_package_json("suspicious", "install-script-postinstall");
    let tgz_bytes = build_tgz_from_package_json(&package_json);
    let integrity = sha512_integrity_for(&tgz_bytes);
    let upstream_tarball_path =
        "/install-script-postinstall/-/install-script-postinstall-1.0.0.tgz";
    let (upstream_registry_url, upstream_server) =
        spawn_upstream_https_server_for_packument_and_tarball(
            "install-script-postinstall",
            "1.0.0",
            upstream_tarball_path,
            Some(&integrity),
            tgz_bytes,
        )
        .await;
    let (proxy_base_url, proxy_server) = spawn_proxy_server(&upstream_registry_url).await;

    let rewritten_tarball =
        fetch_rewritten_tarball_url(&proxy_base_url, "install-script-postinstall").await;
    let response = reqwest::Client::new()
        .get(rewritten_tarball)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

    let body = serde_json::from_slice::<Value>(&response.bytes().await.unwrap()).unwrap();

    assert_eq!(body["category"], "blocked_policy");
    assert_eq!(
        body["findingIds"],
        serde_json::json!(["install-scripts-disallowed"])
    );
    assert_eq!(
        body["error"],
        "install-script-postinstall@1.0.0 blocked: package declares install hooks [findingID: install-scripts-disallowed]"
    );
    assert!(
        body["requestId"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );

    upstream_server.await.unwrap();
    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_returns_403_for_integrity_mismatch() {
    let package_json = read_fixture_package_json("benign", "minimal-package");
    let tgz_bytes = build_tgz_from_package_json(&package_json);
    let integrity = sha512_integrity_for(&tgz_bytes);
    let mut modified_tgz_bytes = tgz_bytes;
    modified_tgz_bytes[0] ^= 0xff;
    let (upstream_registry_url, upstream_server) =
        spawn_upstream_https_server_for_packument_and_tarball(
            "minimal-package",
            "1.0.0",
            "/minimal-package/-/minimal-package-1.0.0.tgz",
            Some(&integrity),
            modified_tgz_bytes,
        )
        .await;
    let (proxy_base_url, proxy_server) = spawn_proxy_server(&upstream_registry_url).await;

    let rewritten_tarball = fetch_rewritten_tarball_url(&proxy_base_url, "minimal-package").await;
    let response = reqwest::Client::new()
        .get(rewritten_tarball)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

    let body = serde_json::from_slice::<Value>(&response.bytes().await.unwrap()).unwrap();

    assert_eq!(body["category"], "blocked_integrity");

    upstream_server.await.unwrap();
    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_returns_403_for_absent_integrity() {
    let package_json = read_fixture_package_json("benign", "minimal-package");
    let tgz_bytes = build_tgz_from_package_json(&package_json);
    let (upstream_registry_url, upstream_server) =
        spawn_upstream_https_server_for_packument_and_tarball(
            "minimal-package",
            "1.0.0",
            "/minimal-package/-/minimal-package-1.0.0.tgz",
            None,
            tgz_bytes,
        )
        .await;
    let (proxy_base_url, proxy_server) = spawn_proxy_server(&upstream_registry_url).await;

    let rewritten_tarball = fetch_rewritten_tarball_url(&proxy_base_url, "minimal-package").await;
    let response = reqwest::Client::new()
        .get(rewritten_tarball)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

    let body = serde_json::from_slice::<Value>(&response.bytes().await.unwrap()).unwrap();

    assert_eq!(body["category"], "blocked_integrity");

    upstream_server.await.unwrap();
    proxy_server.abort();
}

#[tokio::test]
async fn tarball_route_returns_502_for_upstream_tarball_fetch_failure() {
    let upstream_response = packument_bytes(
        "minimal-package",
        "1.0.0",
        "https://127.0.0.1:1/minimal-package/-/minimal-package-1.0.0.tgz",
        Some("sha512-abc123=="),
    );
    let (upstream_registry_url, upstream_request) =
        spawn_upstream_https_server(upstream_response).await;
    let (proxy_base_url, proxy_server) = spawn_proxy_server(&upstream_registry_url).await;

    let rewritten_tarball = fetch_rewritten_tarball_url(&proxy_base_url, "minimal-package").await;
    let response = reqwest::Client::new()
        .get(rewritten_tarball)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);

    let body = serde_json::from_slice::<Value>(&response.bytes().await.unwrap()).unwrap();

    assert_eq!(body["category"], "blocked_fetch");

    let _request = upstream_request.await.unwrap();
    proxy_server.abort();
}

#[tokio::test]
async fn block_response_truncates_finding_ids_to_two_kibibytes() {
    let finding_ids = (0..512)
        .map(|index| format!("synthetic-finding-id-{index:04}"))
        .collect::<Vec<_>>();

    let response = super::build_block_response(
        "test-request-id",
        ResponseCategory::BlockedPolicy,
        "artifact failed policy checks".to_string(),
        finding_ids,
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice::<Value>(&body_bytes).unwrap();

    assert!(body_bytes.len() <= 2 * 1024);
    assert_eq!(body["category"], "blocked_policy");
    assert_eq!(body["error"], "artifact failed policy checks");
    assert_eq!(body["requestId"], "test-request-id");
    assert!(body["findingIds"].as_array().unwrap().len() < 512);
    assert_ne!(body["findingIds"], json!([]));
}

#[tokio::test]
async fn health_route_returns_ok() {
    let (upstream_url, _upstream_server) = spawn_upstream_https_server(vec![]).await;
    let (proxy_base_url, _proxy_server) = spawn_proxy_server(&upstream_url).await;

    let response = reqwest::get(format!("{proxy_base_url}/-/ping"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = serde_json::from_slice::<Value>(&response.bytes().await.unwrap()).unwrap();
    assert_eq!(body, json!({}));
}

async fn fetch_rewritten_tarball_url(proxy_base_url: &str, package_name: &str) -> String {
    let metadata_response = reqwest::Client::new()
        .get(format!("{proxy_base_url}/{package_name}"))
        .send()
        .await
        .unwrap();
    assert_eq!(metadata_response.status(), reqwest::StatusCode::OK);

    let body = serde_json::from_slice::<Value>(&metadata_response.bytes().await.unwrap()).unwrap();

    body["versions"]["1.0.0"]["dist"]["tarball"]
        .as_str()
        .unwrap()
        .to_string()
}

fn sha512_integrity_for(bytes: &[u8]) -> String {
    format!("sha512-{}", STANDARD.encode(Sha512::digest(bytes)))
}
