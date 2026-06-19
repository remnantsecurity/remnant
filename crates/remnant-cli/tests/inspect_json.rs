use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::Value;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tar::{Builder, Header};

#[test]
fn inspect_json_reports_passing_fixture() {
    let artifact_path = create_fixture_tgz(
        "benign-minimal-package.tgz",
        fixture_package_json_path("benign", "minimal-package"),
    );

    let output = run_inspect_json(&artifact_path);
    let report = parse_json_stdout(&output);

    remove_path_if_exists(&artifact_path);

    assert_exit_code(&output, 0);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty in JSON mode"
    );
    assert_eq!(report["schema_version"], "remnant.inspect.report.v0");
    assert_eq!(report["command"], "inspect");
    assert_eq!(report["status"], "passed");
    assert_eq!(report["exit_code"], 0);
    assert_eq!(report["artifact"]["type"], "npm_tgz");
    assert!(report["artifact"].get("path").is_none());
    assert_eq!(report["package"]["name"], "fixture-minimal");
    assert_eq!(report["package"]["version"], "1.0.0");
    assert_eq!(
        report["package"]["lifecycle_scripts"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        report["package"]["install_hooks"].as_array().unwrap().len(),
        0
    );
    assert_eq!(report["archive"]["entry_count"], 1);
    assert_eq!(
        report["archive"]["entries"][0]["path"],
        "package/package.json"
    );
    assert_eq!(report["policy"]["status"], "passed");
    assert_eq!(report["policy"]["findings"].as_array().unwrap().len(), 0);
    assert!(report["error"].is_null());
}

#[test]
fn inspect_json_reports_policy_failure_fixture() {
    let artifact_path = create_fixture_tgz(
        "suspicious-install-script-postinstall.tgz",
        fixture_package_json_path("suspicious", "install-script-postinstall"),
    );

    let output = run_inspect_json(&artifact_path);
    let report = parse_json_stdout(&output);

    remove_path_if_exists(&artifact_path);

    assert_exit_code(&output, 2);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty in JSON mode"
    );
    assert_eq!(report["status"], "failed");
    assert_eq!(report["exit_code"], 2);
    assert_eq!(
        report["package"]["name"],
        "fixture-install-script-postinstall"
    );
    assert_eq!(report["package"]["lifecycle_scripts"][0], "postinstall");
    assert_eq!(report["package"]["install_hooks"][0], "postinstall");
    assert_eq!(report["policy"]["status"], "failed");
    assert_eq!(report["policy"]["findings"].as_array().unwrap().len(), 1);
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
fn inspect_json_reports_suspicious_file_policy_failure_fixture() {
    let artifact_path = create_npmrc_fixture_tgz("suspicious-npmrc-file.tgz");

    let output = run_inspect_json(&artifact_path);
    let report = parse_json_stdout(&output);

    remove_path_if_exists(&artifact_path);

    assert_exit_code(&output, 2);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty in JSON mode"
    );
    assert_eq!(report["status"], "failed");
    assert_eq!(report["exit_code"], 2);
    assert_eq!(report["package"]["name"], "fixture-npmrc-file");
    assert_eq!(report["archive"]["entry_count"], 2);
    assert_eq!(report["archive"]["entries"][0]["path"], "package/.npmrc");
    assert_eq!(
        report["archive"]["entries"][1]["path"],
        "package/package.json"
    );
    assert_eq!(report["policy"]["status"], "failed");
    assert_eq!(report["policy"]["findings"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["policy"]["findings"][0]["rule_id"],
        "suspicious-file-detected"
    );
    assert_eq!(
        report["policy"]["findings"][0]["message"],
        "package contains suspicious files: package/.npmrc"
    );
    assert!(report["error"].is_null());
}

#[test]
fn inspect_json_reports_local_dependency_specifier_policy_failure_fixture() {
    let artifact_path = create_fixture_tgz(
        "suspicious-local-dependency-specifier.tgz",
        fixture_package_json_path("suspicious", "local-dependency-specifier"),
    );

    let output = run_inspect_json(&artifact_path);
    let report = parse_json_stdout(&output);

    remove_path_if_exists(&artifact_path);

    assert_exit_code(&output, 2);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty in JSON mode"
    );
    assert_eq!(report["status"], "failed");
    assert_eq!(report["exit_code"], 2);
    assert_eq!(
        report["package"]["name"],
        "fixture-local-dependency-specifier"
    );
    assert_eq!(report["policy"]["status"], "failed");
    assert_eq!(report["policy"]["findings"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["policy"]["findings"][0]["rule_id"],
        "local-dependency-specifier-disallowed"
    );
    assert_eq!(
        report["policy"]["findings"][0]["message"],
        "package declares local dependency specifiers: dependencies/local-tool"
    );
    assert!(report["error"].is_null());
}

#[test]
fn inspect_json_reports_archive_error_fixture() {
    let artifact_path = manifest_dir()
        .join("fixtures")
        .join("malformed")
        .join("missing-package-json")
        .join("artifact.tgz");

    let output = run_inspect_json(&artifact_path);
    let report = parse_json_stdout(&output);

    assert_exit_code(&output, 1);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty in JSON mode"
    );
    assert_eq!(report["status"], "error");
    assert_eq!(report["exit_code"], 1);
    assert_eq!(report["artifact"]["type"], "npm_tgz");
    assert!(report["artifact"].get("path").is_none());
    assert!(report["package"].is_null());
    assert!(report["archive"].is_null());
    assert_eq!(report["policy"]["status"], "not_evaluated");
    assert_eq!(report["policy"]["findings"].as_array().unwrap().len(), 0);
    assert_eq!(report["error"]["kind"], "archive");
    assert_eq!(
        report["error"]["message"],
        "archive is missing package/package.json"
    );
}

fn run_inspect_json(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_remnant"))
        .arg("inspect")
        .arg("--json")
        .arg(path)
        .output()
        .expect("inspect command should run")
}

fn parse_json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be a JSON inspect report")
}

fn assert_exit_code(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "unexpected exit status; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_fixture_tgz(output_name: &str, package_json_path: PathBuf) -> PathBuf {
    let package_json =
        fs::read(&package_json_path).expect("fixture package.json should be readable");

    create_tgz_with_entries(output_name, &[("package/package.json", package_json)])
}

fn create_npmrc_fixture_tgz(output_name: &str) -> PathBuf {
    let package_root = manifest_dir()
        .join("fixtures")
        .join("suspicious")
        .join("npmrc-file")
        .join("package");
    let package_json = fs::read(package_root.join("package.json"))
        .expect("fixture package.json should be readable");
    let npmrc = fs::read(package_root.join(".npmrc")).expect("fixture .npmrc should be readable");

    create_tgz_with_entries(
        output_name,
        &[
            ("package/.npmrc", npmrc),
            ("package/package.json", package_json),
        ],
    )
}

fn create_tgz_with_entries(output_name: &str, entries: &[(&str, Vec<u8>)]) -> PathBuf {
    let output_path = test_root().join(output_name);
    remove_path_if_exists(&output_path);

    let file = File::create(&output_path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    for (archive_path, contents) in entries {
        append_file_entry(&mut builder, archive_path, contents);
    }

    let encoder = builder
        .into_inner()
        .expect("tar builder should finish successfully");
    encoder
        .finish()
        .expect("gzip encoder should finish successfully");

    output_path
}

fn append_file_entry(builder: &mut Builder<GzEncoder<File>>, archive_path: &str, contents: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_size(contents.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();

    builder
        .append_data(&mut header, archive_path, io::Cursor::new(contents))
        .expect("archive entry should be appended");
}

fn fixture_package_json_path(category: &str, name: &str) -> PathBuf {
    manifest_dir()
        .join("fixtures")
        .join(category)
        .join(name)
        .join("package")
        .join("package.json")
}

fn test_root() -> PathBuf {
    let root = manifest_dir()
        .join("target")
        .join("remnant-tests")
        .join("inspect-json");

    fs::create_dir_all(&root).expect("test root should be created");

    root
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn remove_path_if_exists(path: &Path) {
    if path.exists() {
        fs::remove_file(path).expect("test artifact should be removed");
    }
}
