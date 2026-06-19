//! Test setup helpers for policy result primitives.

use crate::package_json::{PackageDependency, PackageMetadata};
pub(super) const INSTALL_SCRIPTS_RULE_ID: &str = "install-scripts-disallowed";

pub(super) fn package_metadata_with_scripts(
    lifecycle_scripts: &[&str],
    install_hooks: &[&str],
) -> PackageMetadata {
    PackageMetadata {
        name: "demo".to_string(),
        version: "1.0.0".to_string(),
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
