mod setup;

use std::ffi::OsStr;
use std::time::Duration;

use crate::admission::ResponseCategory;

use super::*;
#[cfg(unix)]
use setup::sleeping_binary_path;
use setup::{build_fixture_tgz, committed_malformed_fixture_path, fixture_package_json};

#[tokio::test]
async fn admitted_outcome_for_benign_fixture() {
    let artifact_path = build_fixture_tgz(
        "benign",
        "minimal-package",
        &fixture_package_json("benign", "minimal-package"),
    );

    let outcome = run_inspection(&artifact_path).await;

    assert_eq!(outcome.category, ResponseCategory::Admitted);
    assert!(outcome.finding_ids.is_empty());
}

#[tokio::test]
async fn blocked_policy_outcome_for_suspicious_fixture() {
    let artifact_path = build_fixture_tgz(
        "suspicious",
        "install-script-postinstall",
        &fixture_package_json("suspicious", "install-script-postinstall"),
    );

    let outcome = run_inspection(&artifact_path).await;

    assert_eq!(outcome.category, ResponseCategory::BlockedPolicy);
    assert_eq!(outcome.finding_ids, vec!["install-scripts-disallowed"]);
}

#[tokio::test]
async fn blocked_parse_outcome_for_malformed_fixture() {
    let artifact_path = committed_malformed_fixture_path("missing-package-json");

    let outcome = run_inspection(&artifact_path).await;

    assert_eq!(outcome.category, ResponseCategory::BlockedParse);
    assert!(outcome.finding_ids.is_empty());
}

#[tokio::test]
async fn error_outcome_for_unreadable_artifact_path() {
    let artifact_path = committed_malformed_fixture_path("missing-package-json")
        .with_file_name("does-not-exist.tgz");

    let outcome = run_inspection(&artifact_path).await;

    assert_eq!(outcome.category, ResponseCategory::Error);
    assert!(outcome.finding_ids.is_empty());
}

#[tokio::test]
async fn error_outcome_for_spawn_failure() {
    let artifact_path = committed_malformed_fixture_path("missing-package-json");

    let outcome = run_inspection_with(
        &artifact_path,
        OsStr::new("nonexistent-remnant-binary-abc123"),
        INSPECTION_TIMEOUT,
    )
    .await;

    assert_eq!(outcome.category, ResponseCategory::Error);
    assert!(outcome.finding_ids.is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn error_outcome_for_subprocess_timeout() {
    let artifact_path = committed_malformed_fixture_path("missing-package-json");
    let binary_path = sleeping_binary_path();

    let outcome = run_inspection_with(
        &artifact_path,
        binary_path.as_os_str(),
        Duration::from_millis(100),
    )
    .await;

    assert_eq!(outcome.category, ResponseCategory::Error);
    assert!(outcome.finding_ids.is_empty());
}
