//! Inspection command implementation.
//!
//! This module owns the CLI-facing `inspect` command boundary: validating the
//! user-provided path, calling archive intake, and printing user-facing output.

use crate::archive::{ArchiveError, ArchiveInspection, inspect_archive};
use crate::output::{escape_terminal_path, escape_terminal_text};
use crate::package_json::{PackageJsonError, PackageMetadata, parse_package_json};
use crate::policy::{PolicyResult, PolicyStatus, evaluate_default_policy};
use serde_json::{Value, json};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const INSPECT_REPORT_SCHEMA_VERSION: &str = "remnant.inspect.report.v0";
const TOOL_NAME: &str = "remnant";
const ARTIFACT_TYPE_NPM_TGZ: &str = "npm_tgz";

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InspectOutputFormat {
    Human,
    Json,
}

/// The result of a completed `inspect` command.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InspectOutcome {
    /// Inspection completed and all evaluated policy checks passed.
    PolicyPassed,
    /// Inspection completed, but one or more evaluated policy checks failed.
    PolicyFailed,
}

impl InspectOutcome {
    pub fn exit_code(self) -> i32 {
        match self {
            InspectOutcome::PolicyPassed => 0,
            InspectOutcome::PolicyFailed => 2,
        }
    }
}

/// Errors that can occur while running the `inspect` command.
///
/// These errors are intentionally explicit and deterministic so Remnant can
/// explain why an artifact was rejected before or during archive parsing.
#[derive(Debug, PartialEq, Eq)]
pub enum InspectError {
    /// The provided artifact path does not exist.
    ArtifactDoesNotExist(PathBuf),

    /// The artifact path exists, but its filesystem metadata could not be read.
    ArtifactMetadataUnreadable {
        /// The artifact path whose metadata could not be read.
        path: PathBuf,
        /// The underlying IO error kind.
        kind: io::ErrorKind,
    },

    /// The artifact path exists, but it is not a regular file.
    ///
    /// Symlinks, directories, and other non-regular filesystem entries are
    /// rejected at this stage.
    ArtifactIsNotFile(PathBuf),

    /// The artifact path is a regular file, but does not use the `.tgz`
    /// extension expected for npm package tarballs.
    ArtifactIsNotTgz(PathBuf),

    /// Archive intake failed after the artifact path passed CLI validation.
    Archive(ArchiveError),

    /// Package metadata parsing failed after archive intake succeeded.
    PackageJson(PackageJsonError),
}

impl fmt::Display for InspectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InspectError::ArtifactDoesNotExist(path) => {
                write!(f, "artifact does not exist: {}", escape_terminal_path(path))
            }
            InspectError::ArtifactMetadataUnreadable { path, kind } => {
                write!(
                    f,
                    "artifact metadata could not be read: {} ({kind:?})",
                    escape_terminal_path(path)
                )
            }
            InspectError::ArtifactIsNotFile(path) => {
                write!(
                    f,
                    "artifact is not a regular file: {}",
                    escape_terminal_path(path)
                )
            }
            InspectError::ArtifactIsNotTgz(path) => {
                write!(
                    f,
                    "artifact must have .tgz extension: {}",
                    escape_terminal_path(path)
                )
            }
            InspectError::Archive(error) => write!(f, "{error}"),
            InspectError::PackageJson(error) => write!(f, "{error}"),
        }
    }
}

impl InspectError {
    pub fn exit_code(&self) -> i32 {
        1
    }
}

impl Error for InspectError {}

impl From<ArchiveError> for InspectError {
    fn from(error: ArchiveError) -> Self {
        InspectError::Archive(error)
    }
}

impl From<PackageJsonError> for InspectError {
    fn from(error: PackageJsonError) -> Self {
        InspectError::PackageJson(error)
    }
}

/// Runs the `inspect` command for a user-provided npm artifact path.
///
/// This validates that the path exists, is a regular file, has a `.tgz`
/// extension, and can be read as a gzip-compressed tar archive with safe entry
/// paths.
pub fn run(
    path: PathBuf,
    output_format: InspectOutputFormat,
) -> Result<InspectOutcome, InspectError> {
    match output_format {
        InspectOutputFormat::Human => run_human(path),
        InspectOutputFormat::Json => run_json(path),
    }
}

fn run_human(path: PathBuf) -> Result<InspectOutcome, InspectError> {
    let file = validate_artifact_path(&path)?;

    let archive_inspection = inspect_archive(file, &path)?;
    let package_metadata = parse_package_json(&archive_inspection.package_json)?;
    let policy_result = evaluate_default_policy(&package_metadata, &archive_inspection.entries);

    println!(
        "Inspect command received valid artifact: {}",
        escape_terminal_path(&path)
    );
    println!("Archive entries: {}", archive_inspection.entries.len());
    println!(
        "package/package.json: {} bytes",
        archive_inspection.package_json.len()
    );
    println!(
        "package name: {}",
        escape_terminal_text(&package_metadata.name)
    );
    println!(
        "package version: {}",
        escape_terminal_text(&package_metadata.version)
    );

    for line in format_policy_summary(&policy_result) {
        println!("{line}");
    }

    for entry in archive_inspection.entries {
        println!(
            " - {} ({} bytes)",
            escape_terminal_path(&entry.path),
            entry.size
        );
    }

    Ok(outcome_from_policy_result(&policy_result))
}

fn run_json(path: PathBuf) -> Result<InspectOutcome, InspectError> {
    let file = match validate_artifact_path(&path) {
        Ok(file) => file,
        Err(error) => {
            print_json_report(&build_json_error_report(&error));
            return Err(error);
        }
    };

    let archive_inspection = match inspect_archive(file, &path) {
        Ok(archive_inspection) => archive_inspection,
        Err(error) => {
            let error = InspectError::Archive(error);
            print_json_report(&build_json_error_report(&error));
            return Err(error);
        }
    };

    let package_metadata = match parse_package_json(&archive_inspection.package_json) {
        Ok(package_metadata) => package_metadata,
        Err(error) => {
            let error = InspectError::PackageJson(error);
            print_json_report(&build_json_error_report(&error));
            return Err(error);
        }
    };

    let policy_result = evaluate_default_policy(&package_metadata, &archive_inspection.entries);
    let outcome = outcome_from_policy_result(&policy_result);
    let report = build_json_success_report(
        &archive_inspection,
        &package_metadata,
        &policy_result,
        outcome,
    );

    print_json_report(&report);

    Ok(outcome)
}

fn outcome_from_policy_result(policy_result: &PolicyResult) -> InspectOutcome {
    match policy_result.status() {
        PolicyStatus::Passed => InspectOutcome::PolicyPassed,
        PolicyStatus::Failed => InspectOutcome::PolicyFailed,
    }
}

pub fn format_error_summary(error: &InspectError) -> Vec<String> {
    vec![
        "error: inspect failed".to_string(),
        format!("error kind: {}", error_kind(error)),
        format!("error message: {error}"),
        format!("exit code: {}", error.exit_code()),
    ]
}

fn format_policy_summary(policy_result: &PolicyResult) -> Vec<String> {
    let status = match policy_result.status() {
        PolicyStatus::Passed => "passed",
        PolicyStatus::Failed => "failed",
    };

    let mut lines = vec![
        format!("policy status: {status}"),
        format!("policy findings: {}", policy_result.findings().len()),
    ];

    for finding in policy_result.findings() {
        lines.push(format!(
            " - {}: {}",
            escape_terminal_text(finding.rule_id()),
            escape_terminal_text(finding.message())
        ));
    }

    lines
}

fn build_json_success_report(
    archive_inspection: &ArchiveInspection,
    package_metadata: &PackageMetadata,
    policy_result: &PolicyResult,
    outcome: InspectOutcome,
) -> Value {
    let status = match outcome {
        InspectOutcome::PolicyPassed => "passed",
        InspectOutcome::PolicyFailed => "failed",
    };

    json!({
        "schema_version": INSPECT_REPORT_SCHEMA_VERSION,
        "tool": {
            "name": TOOL_NAME,
            "version": env!("CARGO_PKG_VERSION"),
        },
        "command": "inspect",
        "status": status,
        "exit_code": outcome.exit_code(),
        "artifact": {
            "type": ARTIFACT_TYPE_NPM_TGZ,
        },
        "package": {
            "name": package_metadata.name,
            "version": package_metadata.version,
            "lifecycle_scripts": package_metadata.lifecycle_scripts,
            "install_hooks": package_metadata.install_hooks,
        },
        "archive": {
            "entry_count": archive_inspection.entries.len(),
            "package_json_size_bytes": archive_inspection.package_json.len(),
            "entries": archive_inspection.entries.iter().map(|entry| {
                json!({
                    "path": entry.path.to_string_lossy(),
                    "size_bytes": entry.size,
                })
            }).collect::<Vec<_>>(),
        },
        "policy": {
            "status": policy_status_text(policy_result.status()),
            "findings": policy_result.findings().iter().map(|finding| {
                json!({
                    "rule_id": finding.rule_id(),
                    "message": finding.message(),
                })
            }).collect::<Vec<_>>(),
        },
        "error": Value::Null,
    })
}

fn build_json_error_report(error: &InspectError) -> Value {
    json!({
        "schema_version": INSPECT_REPORT_SCHEMA_VERSION,
        "tool": {
            "name": TOOL_NAME,
            "version": env!("CARGO_PKG_VERSION"),
        },
        "command": "inspect",
        "status": "error",
        "exit_code": error.exit_code(),
        "artifact": {
            "type": ARTIFACT_TYPE_NPM_TGZ,
        },
        "package": Value::Null,
        "archive": Value::Null,
        "policy": {
            "status": "not_evaluated",
            "findings": [],
        },
        "error": {
            "kind": error_kind(error),
            "message": machine_error_message(error),
        },
    })
}

fn policy_status_text(status: PolicyStatus) -> &'static str {
    match status {
        PolicyStatus::Passed => "passed",
        PolicyStatus::Failed => "failed",
    }
}

fn error_kind(error: &InspectError) -> &'static str {
    match error {
        InspectError::Archive(_) => "archive",
        InspectError::PackageJson(_) => "package_json",
        _ => "inspect",
    }
}

fn machine_error_message(error: &InspectError) -> String {
    match error {
        InspectError::ArtifactDoesNotExist(_) => "artifact does not exist".to_string(),
        InspectError::ArtifactMetadataUnreadable { kind, .. } => {
            format!("artifact metadata could not be read ({kind:?})")
        }
        InspectError::ArtifactIsNotFile(_) => "artifact is not a regular file".to_string(),
        InspectError::ArtifactIsNotTgz(_) => "artifact must have .tgz extension".to_string(),
        InspectError::Archive(error) => machine_archive_error_message(error),
        InspectError::PackageJson(error) => machine_package_json_error_message(error),
    }
}

fn machine_archive_error_message(error: &ArchiveError) -> String {
    match error {
        ArchiveError::ArtifactOpenFailed { kind, .. } => {
            format!("artifact could not be opened ({kind:?})")
        }
        ArchiveError::ArchiveReadFailed { kind, .. } => {
            format!("archive could not be read ({kind:?})")
        }
        ArchiveError::ArchiveDecompressedTooLarge { limit, .. } => {
            format!("decompressed archive stream exceeds maximum size ({limit} byte limit)")
        }
        ArchiveError::ArchiveIsEmpty(_) => "archive contains no entries".to_string(),
        ArchiveError::ArchiveTooManyEntries { count, limit, .. } => {
            format!("archive contains too many entries ({count} entries > {limit} entry limit)")
        }
        ArchiveError::ArchiveTooLarge { size, limit, .. } => {
            format!(
                "archive exceeds maximum declared total size ({size} bytes > {limit} byte limit)"
            )
        }
        ArchiveError::ArchiveEntryPathTooLong { length, limit } => {
            format!(
                "archive entry path exceeds maximum length ({length} bytes > {limit} byte limit)"
            )
        }
        ArchiveError::ArchiveEntryPathUnsafe(path) => {
            format!("archive entry path is unsafe: {}", machine_path(path))
        }
        ArchiveError::ArchiveEntryPathDuplicate(path) => {
            format!("archive entry path is duplicated: {}", machine_path(path))
        }
        ArchiveError::ArchiveEntryTooLarge { path, size, limit } => {
            format!(
                "archive entry exceeds maximum size: {} ({size} bytes > {limit} byte limit)",
                machine_path(path)
            )
        }
        ArchiveError::ArchiveEntryIsSymlink(path) => {
            format!("archive entry is a symlink: {}", machine_path(path))
        }
        ArchiveError::ArchiveEntryIsHardlink(path) => {
            format!("archive entry is a hardlink: {}", machine_path(path))
        }
        ArchiveError::ArchiveEntryTypeUnsupported { path, entry_type } => {
            format!(
                "archive entry type is unsupported: {} ({entry_type:#04x})",
                machine_path(path)
            )
        }
        ArchiveError::PackageJsonMissing(_) => {
            "archive is missing package/package.json".to_string()
        }
        ArchiveError::PackageJsonTooLarge { path, size, limit } => {
            format!(
                "package/package.json exceeds maximum size: {} ({size} bytes > {limit} byte limit)",
                machine_path(path)
            )
        }
    }
}

fn machine_path(path: &Path) -> String {
    path.to_string_lossy().escape_debug().to_string()
}

fn machine_package_json_error_message(error: &PackageJsonError) -> String {
    match error {
        PackageJsonError::JsonParseFailed { line, column } => {
            format!("package.json could not be parsed as JSON at line {line}, column {column}")
        }
        PackageJsonError::TopLevelIsNotObject => {
            "package.json top-level value must be an object".to_string()
        }
        PackageJsonError::NameMissing => "package.json is missing required name field".to_string(),
        PackageJsonError::NameIsEmpty => "package.json name field must not be empty".to_string(),
        PackageJsonError::NameIsTooLong { max_bytes } => {
            format!("package.json name field must not exceed {max_bytes} UTF-8 bytes")
        }
        PackageJsonError::NameIsNotString => "package.json name field must be a string".to_string(),
        PackageJsonError::VersionMissing => {
            "package.json is missing required version field".to_string()
        }
        PackageJsonError::VersionIsEmpty => {
            "package.json version field must not be empty".to_string()
        }
        PackageJsonError::VersionIsTooLong { max_bytes } => {
            format!("package.json version field must not exceed {max_bytes} UTF-8 bytes")
        }
        PackageJsonError::VersionIsNotString => {
            "package.json version field must be a string".to_string()
        }
        PackageJsonError::ScriptsIsNotObject => {
            "package.json scripts field must be an object".to_string()
        }
        PackageJsonError::ScriptValueIsNotString { script_name } => {
            format!(
                "package.json scripts entry must be a string: {}",
                escaped_machine_component(script_name)
            )
        }
        PackageJsonError::DependencySectionIsNotObject { section_name } => {
            format!(
                "package.json dependency section must be an object: {}",
                escaped_machine_component(section_name)
            )
        }
        PackageJsonError::DependencySectionHasTooManyEntries {
            section_name,
            max_entries,
        } => {
            format!(
                "package.json dependency section must not contain more than {max_entries} entries: {}",
                escaped_machine_component(section_name)
            )
        }
        PackageJsonError::DependencyNameIsTooLong {
            section_name,
            max_bytes,
        } => {
            format!(
                "package.json dependency name must not exceed {max_bytes} UTF-8 bytes: {}",
                escaped_machine_component(section_name)
            )
        }
        PackageJsonError::DependencyVersionSpecifierIsNotString {
            section_name,
            dependency_name,
        } => {
            format!(
                "package.json dependency version specifier must be a string: {}/{}",
                escaped_machine_component(section_name),
                escaped_machine_component(dependency_name)
            )
        }
        PackageJsonError::DependencyVersionSpecifierIsTooLong {
            section_name,
            dependency_name,
            max_bytes,
        } => {
            format!(
                "package.json dependency version specifier must not exceed {max_bytes} UTF-8 bytes: {}/{}",
                escaped_machine_component(section_name),
                escaped_machine_component(dependency_name)
            )
        }
    }
}

fn escaped_machine_component(text: &str) -> String {
    text.escape_debug().to_string()
}

fn print_json_report(report: &Value) {
    println!("{report}");
}

/// Validates that an artifact path is safe to begin reading.
///
/// This function intentionally uses `fs::symlink_metadata` instead of following
/// symlinks. Remnant treats the user-provided artifact path as untrusted input,
/// so symlinks are rejected unless support for them is explicitly designed.
/// On success, this returns an open file handle for the validated artifact. The
/// caller must pass this handle directly to `inspect_archive` to avoid a second
/// path lookup.
fn validate_artifact_path(path: &Path) -> Result<fs::File, InspectError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(InspectError::ArtifactDoesNotExist(path.to_path_buf()));
        }

        Err(error) => {
            return Err(InspectError::ArtifactMetadataUnreadable {
                path: path.to_path_buf(),
                kind: error.kind(),
            });
        }
    };

    // `symlink_metadata` reports the type of the path itself, so this rejects
    // directories, symlinks, and other non-regular filesystem entries.
    if !metadata.file_type().is_file() {
        return Err(InspectError::ArtifactIsNotFile(path.to_path_buf()));
    }

    // Require the expected npm tarball extension before attempting archive
    // parsing. This is a deterministic intake check, not proof of valid content.
    if path.extension().and_then(|extension| extension.to_str()) != Some("tgz") {
        return Err(InspectError::ArtifactIsNotTgz(path.to_path_buf()));
    }

    fs::File::open(path).map_err(|error| {
        InspectError::Archive(ArchiveError::ArtifactOpenFailed {
            path: path.to_path_buf(),
            kind: error.kind(),
        })
    })
}

#[cfg(test)]
mod tests;
