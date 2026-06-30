mod setup;

use serde_json::Value;

use setup::{spawn_proxy_server, spawn_upstream_https_server};

#[tokio::test]
async fn metadata_route_returns_rewritten_packument_json() {
    let upstream_response = br#"{"name":"left-pad","versions":{"1.3.0":{"dist":{"tarball":"https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz","integrity":"sha512-abc123=="}}}}"#;
    let (upstream_registry_url, upstream_request) =
        spawn_upstream_https_server(upstream_response).await;
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
    let (upstream_registry_url, upstream_request) = spawn_upstream_https_server(b"not json").await;
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
async fn tarball_route_returns_not_implemented_for_known_artifact_key() {
    let upstream_response = br#"{"name":"left-pad","versions":{"1.3.0":{"dist":{"tarball":"https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz","integrity":"sha512-abc123=="}}}}"#;
    let (upstream_registry_url, upstream_request) =
        spawn_upstream_https_server(upstream_response).await;
    let (proxy_base_url, proxy_server) = spawn_proxy_server(&upstream_registry_url).await;

    let metadata_response = reqwest::Client::new()
        .get(format!("{proxy_base_url}/left-pad"))
        .send()
        .await
        .unwrap();
    let body = serde_json::from_slice::<Value>(&metadata_response.bytes().await.unwrap()).unwrap();
    let rewritten_tarball = body["versions"]["1.3.0"]["dist"]["tarball"]
        .as_str()
        .unwrap();

    let tarball_response = reqwest::Client::new()
        .get(rewritten_tarball)
        .send()
        .await
        .unwrap();

    assert_eq!(
        tarball_response.status(),
        reqwest::StatusCode::NOT_IMPLEMENTED
    );

    let _request = upstream_request.await.unwrap();
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
