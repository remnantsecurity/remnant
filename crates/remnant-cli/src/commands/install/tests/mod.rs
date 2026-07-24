use super::*;

#[test]
fn returns_none_for_admitted_package() {
    let line = r#"{"responseCategory":"admitted","enforced":true,"packageName":"is-odd","version":"3.0.1","findingIds":[]}"#;

    assert_eq!(summarize_audit_line(line), None);
}

#[test]
fn summarizes_enforced_block_as_blocked() {
    let line = r#"{"responseCategory":"blocked_policy","enforced":true,"packageName":"husky","version":"9.1.0","findingIds":["install-scripts-disallowed"]}"#;

    assert_eq!(
        summarize_audit_line(line),
        Some(String::from(
            "remnant: blocked husky@9.1.0: blocked_policy [install-scripts-disallowed]"
        ))
    );
}

#[test]
fn summarizes_unenforced_block_as_audit_pass_through() {
    let line = r#"{"responseCategory":"blocked_policy","enforced":false,"packageName":"husky","version":"9.1.0","findingIds":["install-scripts-disallowed"]}"#;

    assert_eq!(
        summarize_audit_line(line),
        Some(String::from(
            "remnant: audit - husky@9.1.0 would have blocked: blocked_policy [install-scripts-disallowed]"
        ))
    );
}

#[test]
fn joins_multiple_finding_ids() {
    let line = r#"{"responseCategory":"blocked_policy","enforced":true,"packageName":"example","version":"1.0.0","findingIds":["install-scripts-disallowed","suspicious-file-detected"]}"#;

    assert_eq!(
        summarize_audit_line(line),
        Some(String::from(
            "remnant: blocked example@1.0.0: blocked_policy [install-scripts-disallowed, suspicious-file-detected]"
        ))
    );
}

#[test]
fn escapes_control_characters_in_audit_metadata() {
    let line = r#"{"responseCategory":"blocked_policy","enforced":true,"packageName":"evil\npackage","version":"1.0.0\u001b","findingIds":["rule\tid"]}"#;

    assert_eq!(
        summarize_audit_line(line),
        Some(String::from(
            r"remnant: blocked evil\npackage@1.0.0\u{1b}: blocked_policy [rule\tid]"
        ))
    );
}

#[test]
fn returns_none_for_malformed_json() {
    assert_eq!(summarize_audit_line("not json"), None);
}

#[test]
fn returns_none_for_missing_required_field() {
    let line = r#"{"responseCategory":"blocked_policy"}"#;

    assert_eq!(summarize_audit_line(line), None);
}

#[test]
fn defaults_empty_npm_args_to_install() {
    assert_eq!(
        build_npm_install_args(vec![]),
        vec![String::from("install")]
    );
}

#[test]
fn prepends_install_to_a_bare_package_name() {
    assert_eq!(
        build_npm_install_args(vec![String::from("is-odd")]),
        vec![String::from("install"), String::from("is-odd")]
    );
}

#[test]
fn prepends_install_to_flags_and_package_name() {
    assert_eq!(
        build_npm_install_args(vec![String::from("--save-dev"), String::from("foo")]),
        vec![
            String::from("install"),
            String::from("--save-dev"),
            String::from("foo")
        ]
    );
}

#[test]
fn selects_a_bindable_ephemeral_port() {
    let port = select_ephemeral_port();

    assert!(port.is_ok());
    assert!(port.expect("ephemeral port selection should succeed") > 0);
}
