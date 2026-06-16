use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tar::{Builder, Header};

#[test]
fn inspect_human_reports_structured_error_to_stderr() {
    let artifact_path = test_root().join("artifact.tar");
    remove_path_if_exists(&artifact_path);
    File::create(&artifact_path).expect("test artifact should be created");

    let output = run_inspect_human(&artifact_path);

    remove_path_if_exists(&artifact_path);

    assert_exit_code(&output, 1);
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty for input errors"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert_eq!(
        stderr,
        format!(
            "error: inspect failed\nerror kind: inspect\nerror message: artifact must have .tgz extension: {}\nexit code: 1\n",
            artifact_path.display()
        )
    );
}

#[test]
fn inspect_human_reports_install_script_policy_failure_to_stdout() {
    let artifact_path = test_root().join("install-script-policy-failure.tgz");
    let package_json =
        br#"{"name":"demo","version":"1.0.0","scripts":{"postinstall":"node postinstall.js"}}"#;
    create_tgz_with_package_json(&artifact_path, package_json);

    let output = run_inspect_human(&artifact_path);

    remove_path_if_exists(&artifact_path);

    assert_exit_code(&output, 2);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty for policy failures"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(
        stdout,
        format!(
            "Inspect command received valid artifact: {}\nArchive entries: 1\npackage/package.json: {} bytes\npackage name: demo\npackage version: 1.0.0\npolicy status: failed\npolicy findings: 1\n - install-scripts-disallowed: package declares install hooks: postinstall\n - package/package.json ({} bytes)\n",
            artifact_path.display(),
            package_json.len(),
            package_json.len()
        )
    );
}

#[test]
fn inspect_human_reports_suspicious_file_policy_failure_to_stdout() {
    let artifact_path = test_root().join("suspicious-file-policy-failure.tgz");
    let package_json = br#"{"name":"demo","version":"1.0.0"}"#;
    let npmrc = b"registry=https://example.invalid\n";
    create_tgz_with_entries(
        &artifact_path,
        &[
            ("package/.npmrc", npmrc.as_slice()),
            ("package/package.json", package_json.as_slice()),
        ],
    );

    let output = run_inspect_human(&artifact_path);

    remove_path_if_exists(&artifact_path);

    assert_exit_code(&output, 2);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty for policy failures"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(
        stdout,
        format!(
            "Inspect command received valid artifact: {}\nArchive entries: 2\npackage/package.json: {} bytes\npackage name: demo\npackage version: 1.0.0\npolicy status: failed\npolicy findings: 1\n - suspicious-file-detected: package contains suspicious files: package/.npmrc\n - package/.npmrc ({} bytes)\n - package/package.json ({} bytes)\n",
            artifact_path.display(),
            package_json.len(),
            npmrc.len(),
            package_json.len()
        )
    );
}

#[test]
fn inspect_human_reports_local_dependency_specifier_policy_failure_to_stdout() {
    let artifact_path = test_root().join("local-dependency-policy-failure.tgz");
    let package_json =
        br#"{"name":"demo","version":"1.0.0","dependencies":{"local-tool":"file:../local-tool"}}"#;
    create_tgz_with_package_json(&artifact_path, package_json);

    let output = run_inspect_human(&artifact_path);

    remove_path_if_exists(&artifact_path);

    assert_exit_code(&output, 2);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty for policy failures"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(
        stdout,
        format!(
            "Inspect command received valid artifact: {}\nArchive entries: 1\npackage/package.json: {} bytes\npackage name: demo\npackage version: 1.0.0\npolicy status: failed\npolicy findings: 1\n - local-dependency-specifier-disallowed: package declares local dependency specifiers: dependencies/local-tool\n - package/package.json ({} bytes)\n",
            artifact_path.display(),
            package_json.len(),
            package_json.len()
        )
    );
}

fn run_inspect_human(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_remnant"))
        .arg("inspect")
        .arg(path)
        .output()
        .expect("inspect command should run")
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

fn create_tgz_with_package_json(path: &Path, package_json: &[u8]) {
    create_tgz_with_entries(path, &[("package/package.json", package_json)]);
}

fn create_tgz_with_entries(path: &Path, entries: &[(&str, &[u8])]) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    for (archive_path, contents) in entries {
        let mut header = Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();

        builder
            .append_data(&mut header, *archive_path, io::Cursor::new(*contents))
            .expect("test archive entry should be appended");
    }

    let encoder = builder
        .into_inner()
        .expect("tar builder should finish successfully");

    encoder
        .finish()
        .expect("gzip encoder should finish successfully");
}

fn test_root() -> PathBuf {
    let root = manifest_dir()
        .join("target")
        .join("remnant-tests")
        .join("inspect-human");

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
