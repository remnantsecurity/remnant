use super::*;

mod setup;

use setup::{
    build_tarball_with_package_json, packument_fallback_fetcher, spawn_https_packument_server,
    spawn_tarball_server, test_temp_path, unbound_https_upstream_registry_url, unbound_local_url,
};

const ADMITTED_PACKAGE_JSON: &[u8] = br#"{
  "name": "fixture-minimal",
  "version": "1.0.0"
}
"#;
const ADMITTED_INTEGRITY: &str = "sha512-vNpGa7F4nU4X8wDKSOvug+rXezp6YI+cNTS3/+Vx44foH7ynI2Nj1mY/hqAoAbFCsDTvww59dbRSm8gz43oVTQ==";
const WRONG_INTEGRITY: &str = "sha512-wNpGa7F4nU4X8wDKSOvug+rXezp6YI+cNTS3/+Vx44foH7ynI2Nj1mY/hqAoAbFCsDTvww59dbRSm8gz43oVTQ==";
const NON_GZIP_BYTES: &[u8] = b"not a valid gzip archive";
const NON_GZIP_INTEGRITY: &str = "sha512-lyMXzBuWcbgfedLvbshSxet2wzQJCdSpXDu8a5OC1qr30mGKSXr7TSQsPYrNW/FQgE166ezMrp2cCnepTToTzw==";
const POLICY_BLOCKED_INTEGRITY: &str = "sha512-ZbsxrH7gEB+Jaibt015kjjXkirp3yicGWi/2owoNmzhDnrY6U/l/FpGYcNawcZmfDL87iWbxot7f5U5r56B7/A==";

#[tokio::test]
async fn admits_a_clean_resolved_package() {
    let bytes = build_tarball_with_package_json(ADMITTED_PACKAGE_JSON);
    let resolved_url = spawn_tarball_server(bytes).await;
    let fetcher = test_fetcher();
    let package = ResolvedPackage {
        name: String::from("lockfile-name"),
        version: String::from("9.9.9"),
        resolved_url: Some(resolved_url),
        integrity: Some(String::from(ADMITTED_INTEGRITY)),
    };

    let verdict =
        inspect_resolved_package(&fetcher, &package, &test_temp_path("admitted.tgz")).await;

    assert_eq!(verdict.category, VerdictCategory::Admitted);
    assert!(verdict.finding_ids.is_empty());
    assert_eq!(verdict.name, "lockfile-name");
    assert_eq!(verdict.version, "9.9.9");
}

#[tokio::test]
async fn blocks_on_integrity_mismatch() {
    let bytes = build_tarball_with_package_json(ADMITTED_PACKAGE_JSON);
    let package = resolved_package(spawn_tarball_server(bytes).await, Some(WRONG_INTEGRITY));

    let verdict = inspect_resolved_package(
        &test_fetcher(),
        &package,
        &test_temp_path("integrity-mismatch.tgz"),
    )
    .await;

    assert_eq!(verdict.category, VerdictCategory::BlockedIntegrity);
    assert_eq!(verdict.detail, "integrity status: Mismatch");
}

#[tokio::test]
async fn blocks_on_missing_integrity() {
    let bytes = build_tarball_with_package_json(ADMITTED_PACKAGE_JSON);
    let package = resolved_package(spawn_tarball_server(bytes).await, None);

    let verdict = inspect_resolved_package(
        &test_fetcher(),
        &package,
        &test_temp_path("missing-integrity.tgz"),
    )
    .await;

    assert_eq!(verdict.category, VerdictCategory::BlockedIntegrity);
    assert_eq!(verdict.detail, "integrity status: Absent");
}

#[tokio::test]
async fn blocks_on_archive_parse_failure() {
    let package = resolved_package(
        spawn_tarball_server(NON_GZIP_BYTES.to_vec()).await,
        Some(NON_GZIP_INTEGRITY),
    );

    let verdict = inspect_resolved_package(
        &test_fetcher(),
        &package,
        &test_temp_path("archive-parse-failure.tgz"),
    )
    .await;

    assert_eq!(verdict.category, VerdictCategory::BlockedParse);
}

#[tokio::test]
async fn blocks_on_policy_violation() {
    let bytes = build_policy_blocked_tarball();
    let package = resolved_package(
        spawn_tarball_server(bytes).await,
        Some(POLICY_BLOCKED_INTEGRITY),
    );

    let verdict = inspect_resolved_package(
        &test_fetcher(),
        &package,
        &test_temp_path("policy-violation.tgz"),
    )
    .await;

    assert_eq!(verdict.category, VerdictCategory::BlockedPolicy);
    assert_eq!(verdict.finding_ids, vec!["install-scripts-disallowed"]);
    assert_eq!(verdict.detail, "policy findings: 1");
}

#[tokio::test]
async fn reports_error_when_upstream_fetch_fails() {
    let package = resolved_package(unbound_local_url(), Some(ADMITTED_INTEGRITY));

    let verdict = inspect_resolved_package(
        &test_fetcher(),
        &package,
        &test_temp_path("upstream-fetch-failure.tgz"),
    )
    .await;

    assert_eq!(verdict.category, VerdictCategory::Error);
    assert!(verdict.finding_ids.is_empty());
}

#[test]
fn parses_dist_metadata_for_the_pinned_version_from_a_packument_response() {
    let body = br#"{"versions":{"1.0.0":{"dist":{"tarball":"https://example.test/pkg-1.0.0.tgz","integrity":"sha512-AAAA=="}}}}"#;

    let metadata = parse_dist_metadata_for_pinned_version(body, "1.0.0")
        .expect("packument response should parse");

    assert_eq!(metadata.tarball_url, "https://example.test/pkg-1.0.0.tgz");
    assert_eq!(metadata.integrity, Some(String::from("sha512-AAAA==")));
}

#[test]
fn parses_dist_metadata_without_an_integrity_field() {
    let body =
        br#"{"versions":{"1.0.0":{"dist":{"tarball":"https://example.test/pkg-1.0.0.tgz"}}}}"#;

    let metadata = parse_dist_metadata_for_pinned_version(body, "1.0.0")
        .expect("packument response should parse");

    assert_eq!(metadata.integrity, None);
}

#[test]
fn rejects_packument_response_missing_the_pinned_version() {
    let body =
        br#"{"versions":{"2.0.0":{"dist":{"tarball":"https://example.test/pkg-2.0.0.tgz"}}}}"#;

    assert_eq!(
        parse_dist_metadata_for_pinned_version(body, "1.0.0"),
        Err(PackumentFallbackError::PinnedVersionMissingFromVersions)
    );
}

#[test]
fn rejects_packument_response_with_non_string_tarball_field() {
    let body = br#"{"versions":{"1.0.0":{"dist":{"tarball":1}}}}"#;

    assert_eq!(
        parse_dist_metadata_for_pinned_version(body, "1.0.0"),
        Err(PackumentFallbackError::TarballFieldIsNotString)
    );
}

#[tokio::test]
async fn admits_a_package_with_dist_metadata_resolved_via_packument_fallback() {
    let tarball_bytes = build_tarball_with_package_json(ADMITTED_PACKAGE_JSON);
    let tarball_url = spawn_tarball_server(tarball_bytes).await;
    let packument_body = format!(
        r#"{{"name":"fixture-package","versions":{{"1.0.0":{{"dist":{{"tarball":"{tarball_url}","integrity":"{ADMITTED_INTEGRITY}"}}}}}}}}"#
    );
    let upstream_registry = spawn_https_packument_server(packument_body.into_bytes()).await;
    let fetcher = packument_fallback_fetcher(&upstream_registry);
    let package = ResolvedPackage {
        name: String::from("fixture-package"),
        version: String::from("1.0.0"),
        resolved_url: None,
        integrity: None,
    };

    let verdict = inspect_resolved_package(
        &fetcher,
        &package,
        &test_temp_path("packument-fallback-admitted.tgz"),
    )
    .await;

    assert_eq!(verdict.category, VerdictCategory::Admitted);
}

#[tokio::test]
async fn reports_error_when_packument_fallback_fetch_fails() {
    let fetcher = packument_fallback_fetcher(&unbound_https_upstream_registry_url());
    let package = ResolvedPackage {
        name: String::from("fixture-package"),
        version: String::from("1.0.0"),
        resolved_url: None,
        integrity: None,
    };

    let verdict = inspect_resolved_package(
        &fetcher,
        &package,
        &test_temp_path("packument-fallback-fetch-failure.tgz"),
    )
    .await;

    assert_eq!(verdict.category, VerdictCategory::Error);
    assert!(verdict.finding_ids.is_empty());
}

#[tokio::test]
async fn inspects_every_resolved_package_in_the_batch() {
    let admitted_package = resolved_package(
        spawn_tarball_server(build_tarball_with_package_json(ADMITTED_PACKAGE_JSON)).await,
        Some(ADMITTED_INTEGRITY),
    );
    let blocked_package = resolved_package(
        spawn_tarball_server(build_policy_blocked_tarball()).await,
        Some(POLICY_BLOCKED_INTEGRITY),
    );

    let verdicts =
        inspect_resolved_packages(&test_fetcher(), &[admitted_package, blocked_package]).await;

    assert_eq!(verdicts.len(), 2);
    assert_eq!(verdicts[0].category, VerdictCategory::Admitted);
    assert_eq!(verdicts[1].category, VerdictCategory::BlockedPolicy);
}

fn test_fetcher() -> UpstreamFetcher {
    UpstreamFetcher::new("https://registry.npmjs.org").expect("npm registry URL should be accepted")
}

fn resolved_package(resolved_url: String, integrity: Option<&str>) -> ResolvedPackage {
    ResolvedPackage {
        name: String::from("fixture-package"),
        version: String::from("1.0.0"),
        resolved_url: Some(resolved_url),
        integrity: integrity.map(str::to_owned),
    }
}

fn build_policy_blocked_tarball() -> Vec<u8> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/suspicious/install-script-postinstall/package/package.json");
    let package_json = std::fs::read(fixture_path)
        .expect("policy-blocked package.json fixture should be readable");
    build_tarball_with_package_json(&package_json)
}
