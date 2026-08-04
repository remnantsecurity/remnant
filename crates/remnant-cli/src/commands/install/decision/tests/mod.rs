use super::*;

#[test]
fn proceeds_when_all_verdicts_are_admitted() {
    let verdicts = vec![package_verdict(VerdictCategory::Admitted)];

    assert_eq!(decide(&verdicts, false), InstallDecision::Proceed);
}

#[test]
fn aborts_when_any_verdict_is_blocked_in_enforce_mode() {
    let verdicts = vec![
        package_verdict(VerdictCategory::Admitted),
        package_verdict(VerdictCategory::BlockedPolicy),
    ];

    assert_eq!(decide(&verdicts, false), InstallDecision::Abort);
}

#[test]
fn proceeds_in_audit_mode_even_with_blocked_verdicts() {
    let verdicts = vec![package_verdict(VerdictCategory::BlockedPolicy)];

    assert_eq!(decide(&verdicts, true), InstallDecision::Proceed);
}

#[test]
fn returns_none_for_admitted_verdict_line() {
    assert_eq!(
        format_verdict_line(&package_verdict(VerdictCategory::Admitted), true),
        None
    );
}

#[test]
fn formats_enforced_block_line() {
    let verdict = policy_blocked_verdict();

    assert_eq!(
        format_verdict_line(&verdict, true),
        Some(String::from(
            "remnant: blocked example@1.0.0: blocked_policy [install-scripts-disallowed]"
        ))
    );
}

#[test]
fn formats_multiple_finding_ids_with_comma_space_separator() {
    let verdict = PackageVerdict {
        finding_ids: vec![
            String::from("install-scripts-disallowed"),
            String::from("suspicious-file-detected"),
        ],
        ..package_verdict(VerdictCategory::BlockedPolicy)
    };

    assert_eq!(
        format_verdict_line(&verdict, true),
        Some(String::from(
            "remnant: blocked example@1.0.0: blocked_policy [install-scripts-disallowed, suspicious-file-detected]"
        ))
    );
}

#[test]
fn formats_audit_would_have_blocked_line() {
    let verdict = policy_blocked_verdict();

    assert_eq!(
        format_verdict_line(&verdict, false),
        Some(String::from(
            "remnant: audit - example@1.0.0 would have blocked: blocked_policy [install-scripts-disallowed]"
        ))
    );
}

#[test]
fn summarizes_all_admitted_packages() {
    let verdicts = vec![
        package_verdict(VerdictCategory::Admitted),
        package_verdict(VerdictCategory::Admitted),
    ];

    assert_eq!(
        format_summary_line(&verdicts, false),
        "remnant: analyzed 2 package(s), 2 admitted, 0 blocked, 0 flagged in audit mode"
    );
}

#[test]
fn summarizes_blocked_packages_in_enforce_mode() {
    let verdicts = vec![
        package_verdict(VerdictCategory::Admitted),
        package_verdict(VerdictCategory::BlockedPolicy),
    ];

    assert_eq!(
        format_summary_line(&verdicts, false),
        "remnant: analyzed 2 package(s), 1 admitted, 1 blocked, 0 flagged in audit mode"
    );
}

#[test]
fn summarizes_flagged_packages_in_audit_mode() {
    let verdicts = vec![
        package_verdict(VerdictCategory::Admitted),
        package_verdict(VerdictCategory::BlockedPolicy),
    ];

    assert_eq!(
        format_summary_line(&verdicts, true),
        "remnant: analyzed 2 package(s), 1 admitted, 0 blocked, 1 flagged in audit mode"
    );
}

#[test]
fn escapes_control_characters_in_verdict_fields() {
    let verdict = PackageVerdict {
        name: String::from("example\npackage"),
        version: String::from("1.0.0"),
        category: VerdictCategory::BlockedPolicy,
        finding_ids: vec![String::from("install\tscripts")],
        detail: String::new(),
    };

    let line = format_verdict_line(&verdict, true).expect("blocked verdict should format");

    assert!(line.contains(r"example\npackage"));
    assert!(line.contains(r"install\tscripts"));
}

fn package_verdict(category: VerdictCategory) -> PackageVerdict {
    PackageVerdict {
        name: String::from("example"),
        version: String::from("1.0.0"),
        category,
        finding_ids: Vec::new(),
        detail: String::new(),
    }
}

fn policy_blocked_verdict() -> PackageVerdict {
    PackageVerdict {
        finding_ids: vec![String::from("install-scripts-disallowed")],
        ..package_verdict(VerdictCategory::BlockedPolicy)
    }
}
