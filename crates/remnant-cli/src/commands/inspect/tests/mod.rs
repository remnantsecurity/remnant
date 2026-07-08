mod setup;

use super::*;
use crate::archive::{ArchiveEntry, ArchiveError, ArchiveInspection};
use crate::package_json::PackageMetadata;
use crate::policy::{INSTALL_SCRIPTS_DISALLOWED_RULE_ID, PolicyFinding, PolicyResult};
use setup::*;
use std::fs::{self, File};
use std::path::PathBuf;

#[test]
fn rejects_missing_artifact() {
    let path = test_path("missing.tgz");
    remove_path_if_exists(&path);

    let result = validate_artifact_path(&path);

    assert_eq!(result.err(), Some(InspectError::ArtifactDoesNotExist(path)));
}

#[test]
fn rejects_directory_artifact() {
    let path = test_path("artifact-dir.tgz");
    remove_path_if_exists(&path);

    fs::create_dir(&path).expect("test directory should be created");

    let result = validate_artifact_path(&path);

    remove_path_if_exists(&path);

    assert_eq!(result.err(), Some(InspectError::ArtifactIsNotFile(path)));
}

#[test]
fn rejects_non_tgz_artifact() {
    let path = test_path("artifact.tar");
    remove_path_if_exists(&path);

    File::create(&path).expect("test file should be created");

    let result = validate_artifact_path(&path);

    remove_path_if_exists(&path);

    assert_eq!(result.err(), Some(InspectError::ArtifactIsNotTgz(path)));
}

#[test]
fn accepts_tgz_file() {
    let path = test_path("artifact.tgz");
    remove_path_if_exists(&path);

    File::create(&path).expect("test file should be created");

    let result = validate_artifact_path(&path);

    remove_path_if_exists(&path);

    assert!(result.is_ok());
}

#[test]
fn inspect_outcome_exit_codes_are_deterministic() {
    assert_eq!(InspectOutcome::PolicyPassed.exit_code(), 0);
    assert_eq!(InspectOutcome::PolicyFailed.exit_code(), 2);
}

#[test]
fn inspect_errors_use_deterministic_failure_exit_code() {
    let error = InspectError::ArtifactIsNotTgz(PathBuf::from("artifact.tar"));

    assert_eq!(error.exit_code(), 1);
}

#[test]
fn inspect_errors_escape_terminal_control_characters() {
    let error = InspectError::ArtifactIsNotTgz(PathBuf::from("artifact\nname.tar"));

    assert_eq!(
        error.to_string(),
        r"artifact must have .tgz extension: artifact\nname.tar"
    );
}

#[test]
fn formats_inspect_error_summary() {
    let error = InspectError::ArtifactIsNotTgz(PathBuf::from("artifact.tar"));

    let summary = format_error_summary(&error);

    assert_eq!(
        summary,
        vec![
            "error: inspect failed".to_string(),
            "error kind: inspect".to_string(),
            "error message: artifact must have .tgz extension: artifact.tar".to_string(),
            "exit code: 1".to_string(),
        ]
    );
}

#[test]
fn run_returns_passing_outcome_when_policy_passes() {
    let path = test_path("policy-pass.tgz");
    create_tgz_with_package_json(&path, br#"{"name":"demo","version":"1.0.0"}"#);

    let result = run(path.clone(), InspectOutputFormat::Human);

    remove_path_if_exists(&path);

    assert_eq!(result, Ok(InspectOutcome::PolicyPassed));
}

#[test]
fn run_returns_policy_failed_outcome_when_policy_fails() {
    let path = test_path("policy-fail.tgz");
    create_tgz_with_package_json(
        &path,
        br#"{"name":"demo","version":"1.0.0","scripts":{"postinstall":"node postinstall.js"}}"#,
    );

    let result = run(path.clone(), InspectOutputFormat::Human);

    remove_path_if_exists(&path);

    assert_eq!(result, Ok(InspectOutcome::PolicyFailed));
}

#[test]
fn formats_passing_policy_summary() {
    let result = PolicyResult::passed();

    let summary = format_policy_summary(&result);

    assert_eq!(
        summary,
        vec![
            "policy status: passed".to_string(),
            "policy findings: 0".to_string(),
        ]
    );
}

#[test]
fn formats_failing_policy_summary_with_rule_ids_and_messages() {
    let result = PolicyResult::from_findings(vec![PolicyFinding::new(
        INSTALL_SCRIPTS_DISALLOWED_RULE_ID,
        "package declares install hooks: install, postinstall",
    )]);

    let summary = format_policy_summary(&result);

    assert_eq!(
        summary,
        vec![
            "policy status: failed".to_string(),
            "policy findings: 1".to_string(),
            " - install-scripts-disallowed: package declares install hooks: install, postinstall"
                .to_string(),
        ]
    );
}

#[test]
fn formats_policy_summary_with_escaped_finding_text() {
    let result = PolicyResult::from_findings(vec![PolicyFinding::new(
        "rule\nid",
        "message with\ncontrol\tcharacters",
    )]);

    let summary = format_policy_summary(&result);

    assert_eq!(
        summary,
        vec![
            "policy status: failed".to_string(),
            "policy findings: 1".to_string(),
            r" - rule\nid: message with\ncontrol\tcharacters".to_string(),
        ]
    );
}

#[test]
fn builds_json_report_for_passing_policy() {
    let archive_inspection = ArchiveInspection {
        entries: vec![ArchiveEntry {
            path: PathBuf::from("package/package.json"),
            size: 58,
        }],
        package_json: br#"{"name":"fixture-minimal","version":"1.0.0"}"#.to_vec(),
    };
    let package_metadata = PackageMetadata {
        name: "fixture-minimal".to_string(),
        version: "1.0.0".to_string(),
        lifecycle_scripts: Vec::new(),
        install_hooks: Vec::new(),
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        optional_dependencies: Vec::new(),
        peer_dependencies: Vec::new(),
    };
    let policy_result = PolicyResult::passed();

    let report = build_json_success_report(
        &archive_inspection,
        &package_metadata,
        &policy_result,
        InspectOutcome::PolicyPassed,
    );

    assert_eq!(report["schema_version"], "remnant.inspect.report.v0");
    assert_eq!(report["tool"]["name"], "remnant");
    assert_eq!(report["command"], "inspect");
    assert_eq!(report["status"], "passed");
    assert_eq!(report["exit_code"], 0);
    assert_eq!(report["artifact"]["type"], "npm_tgz");
    assert_eq!(report["package"]["name"], "fixture-minimal");
    assert_eq!(report["package"]["version"], "1.0.0");
    assert_eq!(report["archive"]["entry_count"], 1);
    assert_eq!(
        report["archive"]["entries"][0]["path"],
        "package/package.json"
    );
    assert_eq!(report["policy"]["status"], "passed");
    assert_eq!(report["policy"]["findings"].as_array().unwrap().len(), 0);
    assert!(report["error"].is_null());
    assert!(report["artifact"].get("path").is_none());
}

#[test]
fn builds_json_report_for_failing_policy() {
    let archive_inspection = ArchiveInspection {
        entries: vec![ArchiveEntry {
            path: PathBuf::from("package/package.json"),
            size: 120,
        }],
        package_json:
            br#"{"name":"demo","version":"1.0.0","scripts":{"postinstall":"node postinstall.js"}}"#
                .to_vec(),
    };
    let package_metadata = PackageMetadata {
        name: "demo".to_string(),
        version: "1.0.0".to_string(),
        lifecycle_scripts: vec!["postinstall".to_string()],
        install_hooks: vec!["postinstall".to_string()],
        dependencies: Vec::new(),
        dev_dependencies: Vec::new(),
        optional_dependencies: Vec::new(),
        peer_dependencies: Vec::new(),
    };
    let policy_result = PolicyResult::from_findings(vec![PolicyFinding::new(
        INSTALL_SCRIPTS_DISALLOWED_RULE_ID,
        "package declares install hooks: postinstall",
    )]);

    let report = build_json_success_report(
        &archive_inspection,
        &package_metadata,
        &policy_result,
        InspectOutcome::PolicyFailed,
    );

    assert_eq!(report["status"], "failed");
    assert_eq!(report["exit_code"], 2);
    assert_eq!(report["package"]["lifecycle_scripts"][0], "postinstall");
    assert_eq!(report["package"]["install_hooks"][0], "postinstall");
    assert_eq!(report["policy"]["status"], "failed");
    assert_eq!(
        report["policy"]["findings"][0]["rule_id"],
        "install-scripts-disallowed"
    );
    assert_eq!(
        report["policy"]["findings"][0]["message"],
        "package declares install hooks: postinstall"
    );
    assert!(report["error"].is_null());
}

#[test]
fn builds_json_report_for_inspection_error_without_host_path() {
    let error = InspectError::ArtifactIsNotTgz(PathBuf::from("/tmp/private/package.tar"));

    let report = build_json_error_report(&error);

    assert_eq!(report["schema_version"], "remnant.inspect.report.v0");
    assert_eq!(report["status"], "error");
    assert_eq!(report["exit_code"], 1);
    assert_eq!(report["artifact"]["type"], "npm_tgz");
    assert!(report["artifact"].get("path").is_none());
    assert!(report["package"].is_null());
    assert!(report["archive"].is_null());
    assert_eq!(report["policy"]["status"], "not_evaluated");
    assert_eq!(report["policy"]["findings"].as_array().unwrap().len(), 0);
    assert_eq!(report["error"]["kind"], "inspect");
    assert_eq!(
        report["error"]["message"],
        "artifact must have .tgz extension"
    );
}

#[test]
fn builds_json_report_for_package_json_error() {
    let error = InspectError::PackageJson(PackageJsonError::TopLevelIsNotObject);

    let report = build_json_error_report(&error);

    assert_eq!(report["status"], "error");
    assert_eq!(report["error"]["kind"], "package_json");
    assert_eq!(
        report["error"]["message"],
        "package.json top-level value must be an object"
    );
}

#[test]
fn machine_archive_error_messages_escape_attacker_controlled_path_components() {
    assert_eq!(
        machine_archive_error_message(&ArchiveError::ArchiveEntryPathUnsafe(PathBuf::from(
            "package/evil\npath.js",
        ))),
        r"archive entry path is unsafe: package/evil\npath.js"
    );
    assert_eq!(
        machine_archive_error_message(&ArchiveError::ArchiveEntryPathDuplicate(PathBuf::from(
            "package/dup\tpath.js",
        ))),
        r"archive entry path is duplicated: package/dup\tpath.js"
    );
    assert_eq!(
        machine_archive_error_message(&ArchiveError::ArchiveEntryTooLarge {
            path: PathBuf::from("package/large\rfile.js"),
            size: 33,
            limit: 32,
        }),
        r"archive entry exceeds maximum size: package/large\rfile.js (33 bytes > 32 byte limit)"
    );
    assert_eq!(
        machine_archive_error_message(&ArchiveError::ArchiveEntryIsSymlink(PathBuf::from(
            "package/link\u{1b}.js",
        ))),
        r"archive entry is a symlink: package/link\u{1b}.js"
    );
    assert_eq!(
        machine_archive_error_message(&ArchiveError::ArchiveEntryIsHardlink(PathBuf::from(
            "package/hard\0link.js",
        ))),
        r"archive entry is a hardlink: package/hard\0link.js"
    );
    assert_eq!(
        machine_archive_error_message(&ArchiveError::ArchiveEntryTypeUnsupported {
            path: PathBuf::from("package/type\nentry"),
            entry_type: 0x44,
        }),
        r"archive entry type is unsupported: package/type\nentry (0x44)"
    );
    assert_eq!(
        machine_archive_error_message(&ArchiveError::PackageJsonTooLarge {
            path: PathBuf::from("package/package\n.json"),
            size: 2,
            limit: 1,
        }),
        r"package/package.json exceeds maximum size: package/package\n.json (2 bytes > 1 byte limit)"
    );
}

#[test]
fn machine_package_json_error_messages_escape_attacker_controlled_components() {
    assert_eq!(
        machine_package_json_error_message(&PackageJsonError::ScriptValueIsNotString {
            script_name: "post\ninstall".to_string(),
        }),
        r"package.json scripts entry must be a string: post\ninstall"
    );
    assert_eq!(
        machine_package_json_error_message(&PackageJsonError::DependencySectionIsNotObject {
            section_name: "dependencies\tdev".to_string(),
        }),
        r"package.json dependency section must be an object: dependencies\tdev"
    );
    assert_eq!(
        machine_package_json_error_message(&PackageJsonError::DependencySectionHasTooManyEntries {
            section_name: "dependencies\nextra".to_string(),
            max_entries: 1_000,
        }),
        r"package.json dependency section must not contain more than 1000 entries: dependencies\nextra"
    );
    assert_eq!(
        machine_package_json_error_message(&PackageJsonError::DependencyNameIsTooLong {
            section_name: "optional\ndependencies".to_string(),
            max_bytes: 214,
        }),
        r"package.json dependency name must not exceed 214 UTF-8 bytes: optional\ndependencies"
    );
    assert_eq!(
        machine_package_json_error_message(
            &PackageJsonError::DependencyVersionSpecifierIsNotString {
                section_name: "dependencies".to_string(),
                dependency_name: "left\tpad".to_string(),
            },
        ),
        r"package.json dependency version specifier must be a string: dependencies/left\tpad"
    );
    assert_eq!(
        machine_package_json_error_message(
            &PackageJsonError::DependencyVersionSpecifierIsTooLong {
                section_name: "dev\ndependencies".to_string(),
                dependency_name: "left\rpad".to_string(),
                max_bytes: 512,
            },
        ),
        r"package.json dependency version specifier must not exceed 512 UTF-8 bytes: dev\ndependencies/left\rpad"
    );
}

#[test]
fn builds_json_report_for_dependency_section_has_too_many_entries_error() {
    let error = InspectError::PackageJson(PackageJsonError::DependencySectionHasTooManyEntries {
        section_name: "dependencies".to_string(),
        max_entries: 1_000,
    });

    let report = build_json_error_report(&error);

    assert_eq!(report["status"], "error");
    assert_eq!(report["error"]["kind"], "package_json");
    assert_eq!(
        report["error"]["message"],
        "package.json dependency section must not contain more than 1000 entries: dependencies"
    );
}

#[test]
fn builds_json_report_for_dependency_name_is_too_long_error() {
    let error = InspectError::PackageJson(PackageJsonError::DependencyNameIsTooLong {
        section_name: "dependencies".to_string(),
        max_bytes: 214,
    });

    let report = build_json_error_report(&error);

    assert_eq!(report["status"], "error");
    assert_eq!(report["error"]["kind"], "package_json");
    assert_eq!(
        report["error"]["message"],
        "package.json dependency name must not exceed 214 UTF-8 bytes: dependencies"
    );
}

#[test]
fn builds_json_report_for_dependency_version_specifier_is_too_long_error() {
    let error = InspectError::PackageJson(PackageJsonError::DependencyVersionSpecifierIsTooLong {
        section_name: "dependencies".to_string(),
        dependency_name: "left-pad".to_string(),
        max_bytes: 512,
    });

    let report = build_json_error_report(&error);

    assert_eq!(report["status"], "error");
    assert_eq!(report["error"]["kind"], "package_json");
    assert_eq!(
        report["error"]["message"],
        "package.json dependency version specifier must not exceed 512 UTF-8 bytes: dependencies/left-pad"
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlink_to_tgz_file() {
    use std::os::unix::fs::symlink;

    let target_path = test_path("symlink-target.tgz");
    let symlink_path = test_path("artifact-symlink.tgz");

    remove_path_if_exists(&symlink_path);
    remove_path_if_exists(&target_path);

    File::create(&target_path).expect("test target file should be created");
    symlink(&target_path, &symlink_path).expect("test symlink should be created");

    let result = validate_artifact_path(&symlink_path);

    remove_path_if_exists(&symlink_path);
    remove_path_if_exists(&target_path);

    assert_eq!(
        result.err(),
        Some(InspectError::ArtifactIsNotFile(symlink_path))
    );
}
