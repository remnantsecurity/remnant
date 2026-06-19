//! Test setup helpers for package metadata parsing.

use super::super::{PackageDependency, PackageMetadata};
use serde_json::Value;

pub(super) const MINIMAL_PACKAGE_JSON: &[u8] = br#"{"name":"demo","version":"1.0.0"}"#;

pub(super) fn package_metadata(
    name: impl Into<String>,
    version: impl Into<String>,
    lifecycle_scripts: &[&str],
    install_hooks: &[&str],
) -> PackageMetadata {
    PackageMetadata {
        name: name.into(),
        version: version.into(),
        lifecycle_scripts: lifecycle_scripts
            .iter()
            .map(|script| script.to_string())
            .collect(),
        install_hooks: install_hooks
            .iter()
            .map(|script| script.to_string())
            .collect(),
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        optional_dependencies: Vec::new(),
        peer_dependencies: Vec::new(),
    }
}

pub(super) fn dependency(
    name: impl Into<String>,
    version_specifier: impl Into<String>,
) -> PackageDependency {
    PackageDependency {
        name: name.into(),
        version_specifier: version_specifier.into(),
    }
}

pub(super) fn package_json_with_dependency_section(
    section_name: &str,
    dependencies: &[(String, String)],
) -> String {
    let dependency_entries = dependencies
        .iter()
        .map(|(name, version_specifier)| {
            let escaped_name =
                serde_json::to_string(name).expect("dependency name should serialize");
            let escaped_version_specifier = serde_json::to_string(version_specifier)
                .expect("dependency version specifier should serialize");

            format!("{escaped_name}:{escaped_version_specifier}")
        })
        .collect::<Vec<_>>()
        .join(",");

    format!(r#"{{"name":"demo","version":"1.0.0","{section_name}":{{{dependency_entries}}}}}"#)
}

pub(super) fn parse_fixture_metadata(contents: &[u8]) -> Value {
    let value: Value = serde_json::from_slice(contents).expect("fixture metadata should be JSON");

    assert!(
        value.get("id").and_then(Value::as_str).is_some(),
        "fixture metadata should include id"
    );
    assert!(
        value.get("category").and_then(Value::as_str).is_some(),
        "fixture metadata should include category"
    );
    assert!(
        value.get("description").and_then(Value::as_str).is_some(),
        "fixture metadata should include description"
    );
    assert!(
        value.get("expected").and_then(Value::as_object).is_some(),
        "fixture metadata should include expected object"
    );
    assert!(
        value.get("safety").and_then(Value::as_object).is_some(),
        "fixture metadata should include safety object"
    );

    value
}
