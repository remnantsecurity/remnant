use std::process::{Command, Output};

#[test]
fn version_flag_prints_package_version_to_stdout() {
    let output = Command::new(env!("CARGO_BIN_EXE_remnant"))
        .arg("--version")
        .output()
        .expect("version command should run");

    assert_exit_code(&output, 0);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty for version output"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    assert_eq!(stdout, format!("remnant {}\n", env!("CARGO_PKG_VERSION")));
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
