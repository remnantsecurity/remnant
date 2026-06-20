mod setup;

use super::*;
use crate::archive::ArchiveEntry;
use crate::package_json::parse_package_json;
use setup::{INSTALL_SCRIPTS_RULE_ID, dependency, package_metadata_with_scripts};
use std::path::PathBuf;

const SUSPICIOUS_INSTALL_SCRIPT_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/suspicious/install-script-postinstall/package/package.json");

#[test]
fn policy_result_without_findings_passes() {
    let result = PolicyResult::passed();

    assert_eq!(result.status(), PolicyStatus::Passed);
    assert!(result.findings().is_empty());
}

#[test]
fn policy_result_with_findings_fails() {
    let finding = PolicyFinding::new(INSTALL_SCRIPTS_RULE_ID, "package declares install scripts");
    let result = PolicyResult::from_findings(vec![finding]);

    assert_eq!(result.status(), PolicyStatus::Failed);
    assert_eq!(result.findings().len(), 1);
    assert_eq!(result.findings()[0].rule_id(), INSTALL_SCRIPTS_RULE_ID);
    assert_eq!(
        result.findings()[0].message(),
        "package declares install scripts"
    );
}

#[test]
fn policy_findings_are_ordered_deterministically() {
    let result = PolicyResult::from_findings(vec![
        PolicyFinding::new(SUSPICIOUS_FILE_DETECTED_RULE_ID, "z message"),
        PolicyFinding::new(INSTALL_SCRIPTS_RULE_ID, "install script found"),
        PolicyFinding::new(SUSPICIOUS_FILE_DETECTED_RULE_ID, "a message"),
    ]);

    let ordered_findings = result.findings();

    assert_eq!(ordered_findings[0].rule_id(), INSTALL_SCRIPTS_RULE_ID);
    assert_eq!(ordered_findings[0].message(), "install script found");
    assert_eq!(
        ordered_findings[1].rule_id(),
        SUSPICIOUS_FILE_DETECTED_RULE_ID
    );
    assert_eq!(ordered_findings[1].message(), "a message");
    assert_eq!(
        ordered_findings[2].rule_id(),
        SUSPICIOUS_FILE_DETECTED_RULE_ID
    );
    assert_eq!(ordered_findings[2].message(), "z message");
}

#[test]
fn policy_rule_registry_orders_rules_deterministically() {
    let registry = PolicyRuleRegistry::from_rules(vec![
        PolicyRule::new(SUSPICIOUS_FILE_DETECTED_RULE_ID, "Detect suspicious files"),
        PolicyRule::new(
            LOCAL_DEPENDENCY_SPECIFIER_DISALLOWED_RULE_ID,
            "Deny local dependency specifiers",
        ),
        PolicyRule::new(INSTALL_SCRIPTS_RULE_ID, "Deny install scripts"),
    ])
    .expect("rules should register successfully");

    let rules = registry.rules();

    assert_eq!(rules[0].id(), INSTALL_SCRIPTS_RULE_ID);
    assert_eq!(rules[0].description(), "Deny install scripts");
    assert_eq!(rules[1].id(), LOCAL_DEPENDENCY_SPECIFIER_DISALLOWED_RULE_ID);
    assert_eq!(rules[1].description(), "Deny local dependency specifiers");
    assert_eq!(rules[2].id(), SUSPICIOUS_FILE_DETECTED_RULE_ID);
    assert_eq!(rules[2].description(), "Detect suspicious files");
}

#[test]
fn policy_rule_registry_rejects_empty_rule_ids() {
    let result = PolicyRuleRegistry::from_rules(vec![PolicyRule::new("", "Missing id")]);

    assert_eq!(result, Err(PolicyRuleRegistrationError::EmptyRuleId));
}

#[test]
fn policy_rule_registry_rejects_duplicate_rule_ids() {
    let result = PolicyRuleRegistry::from_rules(vec![
        PolicyRule::new(INSTALL_SCRIPTS_RULE_ID, "First install script rule"),
        PolicyRule::new(INSTALL_SCRIPTS_RULE_ID, "Duplicate install script rule"),
    ]);

    assert_eq!(
        result,
        Err(PolicyRuleRegistrationError::DuplicateRuleId(
            INSTALL_SCRIPTS_RULE_ID.to_string()
        ))
    );
}

#[test]
fn install_script_policy_passes_without_install_hooks() {
    let metadata = package_metadata_with_scripts(&[], &[]);

    let result = evaluate_install_script_policy(&metadata);

    assert_eq!(result.status(), PolicyStatus::Passed);
    assert!(result.findings().is_empty());
}

#[test]
fn install_script_policy_ignores_non_install_lifecycle_scripts() {
    let metadata = package_metadata_with_scripts(&["version"], &[]);

    let result = evaluate_install_script_policy(&metadata);

    assert_eq!(result.status(), PolicyStatus::Passed);
    assert!(result.findings().is_empty());
}

#[test]
fn install_script_policy_fails_when_install_hooks_are_declared() {
    let metadata = package_metadata_with_scripts(&["install"], &["install"]);

    let result = evaluate_install_script_policy(&metadata);

    assert_eq!(result.status(), PolicyStatus::Failed);
    assert_eq!(result.findings().len(), 1);
    assert_eq!(
        result.findings()[0].rule_id(),
        INSTALL_SCRIPTS_DISALLOWED_RULE_ID
    );
    assert_eq!(
        result.findings()[0].message(),
        "package declares install hooks: install"
    );
}

#[test]
fn install_script_policy_fails_for_install_script_fixture() {
    let metadata = parse_package_json(SUSPICIOUS_INSTALL_SCRIPT_FIXTURE)
        .expect("fixture package metadata should parse");

    let result = evaluate_install_script_policy(&metadata);

    assert_eq!(result.status(), PolicyStatus::Failed);
    assert_eq!(result.findings().len(), 1);
    assert_eq!(
        result.findings()[0].message(),
        "package declares install hooks: postinstall"
    );
}

#[test]
fn install_script_policy_orders_reported_hooks_deterministically() {
    let metadata = package_metadata_with_scripts(
        &["prepare", "install", "postinstall"],
        &["prepare", "postinstall", "install"],
    );

    let result = evaluate_install_script_policy(&metadata);

    assert_eq!(result.status(), PolicyStatus::Failed);
    assert_eq!(result.findings().len(), 1);
    assert_eq!(
        result.findings()[0].message(),
        "package declares install hooks: install, postinstall, prepare"
    );
}

#[test]
fn local_dependency_specifier_policy_passes_without_file_specifiers() {
    let mut metadata = package_metadata_with_scripts(&[], &[]);
    metadata.dependencies = vec![dependency("left-pad", "^1.0.0")];
    metadata.dev_dependencies = vec![dependency("test-tool", "npm:test-tool@1.0.0")];

    let result = evaluate_local_dependency_specifier_policy(&metadata);

    assert_eq!(result.status(), PolicyStatus::Passed);
    assert!(result.findings().is_empty());
}

#[test]
fn local_dependency_specifier_policy_fails_for_file_dependency_specifier() {
    let mut metadata = package_metadata_with_scripts(&[], &[]);
    metadata.dependencies = vec![dependency("local-tool", "file:../local-tool")];

    let result = evaluate_local_dependency_specifier_policy(&metadata);

    assert_eq!(result.status(), PolicyStatus::Failed);
    assert_eq!(result.findings().len(), 1);
    assert_eq!(
        result.findings()[0].rule_id(),
        LOCAL_DEPENDENCY_SPECIFIER_DISALLOWED_RULE_ID
    );
    assert_eq!(
        result.findings()[0].message(),
        "package declares local dependency specifiers: dependencies/local-tool"
    );
}

#[test]
fn local_dependency_specifier_policy_orders_dependency_names_deterministically() {
    let mut metadata = package_metadata_with_scripts(&[], &[]);
    metadata.dependencies = vec![
        dependency("zeta", "file:../zeta"),
        dependency("alpha", "file:../alpha"),
    ];

    let result = evaluate_local_dependency_specifier_policy(&metadata);

    assert_eq!(result.status(), PolicyStatus::Failed);
    assert_eq!(
        result.findings()[0].message(),
        "package declares local dependency specifiers: dependencies/alpha, dependencies/zeta"
    );
}

#[test]
fn local_dependency_specifier_policy_reports_dependency_sections_deterministically() {
    let mut metadata = package_metadata_with_scripts(&[], &[]);
    metadata.dependencies = vec![dependency("zeta", "file:../zeta")];
    metadata.dev_dependencies = vec![dependency("alpha-dev", "file:../alpha-dev")];
    metadata.optional_dependencies = vec![dependency("optional-tool", "file:../optional-tool")];
    metadata.peer_dependencies = vec![dependency("peer-tool", "file:../peer-tool")];

    let result = evaluate_local_dependency_specifier_policy(&metadata);

    assert_eq!(result.status(), PolicyStatus::Failed);
    assert_eq!(
        result.findings()[0].message(),
        "package declares local dependency specifiers: dependencies/zeta, devDependencies/alpha-dev, optionalDependencies/optional-tool, peerDependencies/peer-tool"
    );
}

#[test]
fn suspicious_file_policy_passes_without_suspicious_paths() {
    let entries = vec![
        archive_entry("package/package.json"),
        archive_entry("package/index.js"),
    ];

    let result = evaluate_suspicious_file_policy(&entries);

    assert_eq!(result.status(), PolicyStatus::Passed);
    assert!(result.findings().is_empty());
}

#[test]
fn suspicious_file_policy_fails_for_npmrc() {
    let entries = vec![
        archive_entry("package/package.json"),
        archive_entry("package/.npmrc"),
    ];

    let result = evaluate_suspicious_file_policy(&entries);

    assert_eq!(result.status(), PolicyStatus::Failed);
    assert_eq!(result.findings().len(), 1);
    assert_eq!(
        result.findings()[0].rule_id(),
        SUSPICIOUS_FILE_DETECTED_RULE_ID
    );
    assert_eq!(
        result.findings()[0].message(),
        "package contains suspicious files: package/.npmrc"
    );
}

#[test]
fn default_policy_combines_findings_deterministically() {
    let mut metadata = package_metadata_with_scripts(&["postinstall"], &["postinstall"]);
    metadata.dependencies = vec![dependency("local-tool", "file:../local-tool")];
    let entries = vec![archive_entry("package/.npmrc")];

    let result = evaluate_default_policy(&metadata, &entries);

    assert_eq!(result.status(), PolicyStatus::Failed);
    assert_eq!(result.findings().len(), 3);
    assert_eq!(
        result.findings()[0].rule_id(),
        INSTALL_SCRIPTS_DISALLOWED_RULE_ID
    );
    assert_eq!(
        result.findings()[1].rule_id(),
        LOCAL_DEPENDENCY_SPECIFIER_DISALLOWED_RULE_ID
    );
    assert_eq!(
        result.findings()[2].rule_id(),
        SUSPICIOUS_FILE_DETECTED_RULE_ID
    );
}

fn archive_entry(path: &str) -> ArchiveEntry {
    ArchiveEntry {
        path: PathBuf::from(path),
        size: 0,
    }
}
