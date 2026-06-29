//! Deterministic policy evaluation primitives.
//!
//! This module owns policy result construction and concrete policy checks over
//! already-parsed package metadata. Archive intake and package metadata parsing
//! remain separate trust boundaries.

use crate::archive::ArchiveEntry;
use crate::package_json::{PackageDependency, PackageMetadata};
use std::path::Path;

#[cfg(test)]
mod rules;

#[cfg(test)]
pub use rules::{PolicyRule, PolicyRuleRegistrationError, PolicyRuleRegistry};

pub const INSTALL_SCRIPTS_DISALLOWED_RULE_ID: &str = "install-scripts-disallowed";
pub const LOCAL_DEPENDENCY_SPECIFIER_DISALLOWED_RULE_ID: &str =
    "local-dependency-specifier-disallowed";
pub const SUSPICIOUS_FILE_DETECTED_RULE_ID: &str = "suspicious-file-detected";

const SUSPICIOUS_FILE_PATHS: &[&str] = &["package/.npmrc"];

/// The overall result of policy evaluation.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PolicyStatus {
    /// No policy findings were produced.
    Passed,
    /// One or more policy findings were produced.
    Failed,
}

/// A single explainable policy finding.
///
/// `rule_id` is intended to be a stable, deterministic identifier. `message` is
/// intended for human-readable explanation and future machine-readable reports.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PolicyFinding {
    rule_id: String,
    message: String,
}

impl PolicyFinding {
    pub fn new(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            message: message.into(),
        }
    }

    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// The deterministic output of policy evaluation.
#[derive(Debug, PartialEq, Eq)]
pub struct PolicyResult {
    findings: Vec<PolicyFinding>,
}

impl PolicyResult {
    pub fn passed() -> Self {
        Self {
            findings: Vec::new(),
        }
    }

    pub fn from_findings(mut findings: Vec<PolicyFinding>) -> Self {
        findings.sort_by(|left, right| {
            left.rule_id
                .cmp(&right.rule_id)
                .then_with(|| left.message.cmp(&right.message))
        });

        Self { findings }
    }

    pub fn status(&self) -> PolicyStatus {
        if self.findings.is_empty() {
            PolicyStatus::Passed
        } else {
            PolicyStatus::Failed
        }
    }

    pub fn findings(&self) -> &[PolicyFinding] {
        &self.findings
    }
}

/// Evaluates Remnant's current default local policy checks.
///
/// The current default policy combines deterministic checks over already parsed
/// package metadata and already validated archive entry facts. It does not
/// execute package-controlled code, extract archive contents, or inspect file
/// contents.
pub fn evaluate_default_policy(
    metadata: &PackageMetadata,
    archive_entries: &[ArchiveEntry],
) -> PolicyResult {
    let mut findings = Vec::new();

    findings.extend(evaluate_install_script_policy(metadata).findings().to_vec());
    findings.extend(
        evaluate_local_dependency_specifier_policy(metadata)
            .findings()
            .to_vec(),
    );
    findings.extend(
        evaluate_suspicious_file_policy(archive_entries)
            .findings()
            .to_vec(),
    );

    PolicyResult::from_findings(findings)
}

/// Evaluates the strict install-script policy against parsed package metadata.
///
/// This policy fails when npm install hooks are declared. It only reports the
/// detected hook names; it does not execute, parse, or expose script commands.
pub fn evaluate_install_script_policy(metadata: &PackageMetadata) -> PolicyResult {
    if metadata.install_hooks.is_empty() {
        return PolicyResult::passed();
    }

    PolicyResult::from_findings(vec![PolicyFinding::new(
        INSTALL_SCRIPTS_DISALLOWED_RULE_ID,
        format!(
            "package declares install hooks: {}",
            metadata.install_hooks.join(", ")
        ),
    )])
}

/// Evaluates strict local dependency specifier policy against parsed dependency metadata.
///
/// This policy fails when dependency version specifiers use npm's `file:` form. It
/// reports only dependency section/name pairs, not the full specifier strings.
pub fn evaluate_local_dependency_specifier_policy(metadata: &PackageMetadata) -> PolicyResult {
    let mut local_dependency_references = Vec::new();

    collect_local_dependency_references(
        "dependencies",
        &metadata.dependencies,
        &mut local_dependency_references,
    );
    collect_local_dependency_references(
        "devDependencies",
        &metadata.dev_dependencies,
        &mut local_dependency_references,
    );
    collect_local_dependency_references(
        "optionalDependencies",
        &metadata.optional_dependencies,
        &mut local_dependency_references,
    );
    collect_local_dependency_references(
        "peerDependencies",
        &metadata.peer_dependencies,
        &mut local_dependency_references,
    );

    if local_dependency_references.is_empty() {
        return PolicyResult::passed();
    }

    local_dependency_references.sort();

    PolicyResult::from_findings(vec![PolicyFinding::new(
        LOCAL_DEPENDENCY_SPECIFIER_DISALLOWED_RULE_ID,
        format!(
            "package declares local dependency specifiers: {}",
            local_dependency_references.join(", ")
        ),
    )])
}

/// Evaluates deterministic suspicious file policy over validated archive paths.
///
/// This policy currently flags exact archive paths that affect package-manager
/// behavior or admission review. It does not inspect file contents.
pub fn evaluate_suspicious_file_policy(archive_entries: &[ArchiveEntry]) -> PolicyResult {
    let mut suspicious_paths = archive_entries
        .iter()
        .filter_map(|entry| suspicious_file_path(&entry.path))
        .collect::<Vec<_>>();

    if suspicious_paths.is_empty() {
        return PolicyResult::passed();
    }

    suspicious_paths.sort();

    PolicyResult::from_findings(vec![PolicyFinding::new(
        SUSPICIOUS_FILE_DETECTED_RULE_ID,
        format!(
            "package contains suspicious files: {}",
            suspicious_paths.join(", ")
        ),
    )])
}

fn collect_local_dependency_references(
    section_name: &str,
    dependencies: &[PackageDependency],
    local_dependency_references: &mut Vec<String>,
) {
    for dependency in dependencies {
        if dependency.version_specifier.starts_with("file:") {
            local_dependency_references.push(format!("{section_name}/{}", dependency.name));
        }
    }
}

fn suspicious_file_path(path: &Path) -> Option<String> {
    // Accepted archive entry paths are normalized only after UTF-8 validation.
    let path = path
        .to_str()
        .expect("validated archive entry paths must be UTF-8");

    if SUSPICIOUS_FILE_PATHS.contains(&path) {
        return Some(path.to_string());
    }

    None
}

#[cfg(test)]
mod tests;
