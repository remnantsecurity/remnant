use super::*;

mod setup;

use setup::{real_lockfile_fixture, workspace_lockfile_fixture};

#[test]
fn parses_every_non_root_entry_from_a_real_lockfile() {
    let packages = parse_resolved_packages(&real_lockfile_fixture())
        .expect("real package-lock fixture should parse");

    assert_eq!(
        packages,
        vec![
            ResolvedPackage {
                name: String::from("@babel/helper-validator-identifier"),
                version: String::from("8.0.4"),
                resolved_url: String::from(
                    "https://registry.npmjs.org/@babel/helper-validator-identifier/-/helper-validator-identifier-8.0.4.tgz",
                ),
                integrity: Some(String::from(
                    "sha512-4wFaiLd0bVo4cIoTXI3zKI038NIWE/cr3jvBjejOVYVxV/m8Ltav1USiGzG1fmS5J2RhgEOgXNNK46cRPnRsrg==",
                )),
            },
            ResolvedPackage {
                name: String::from("is-number"),
                version: String::from("6.0.0"),
                resolved_url: String::from(
                    "https://registry.npmjs.org/is-number/-/is-number-6.0.0.tgz",
                ),
                integrity: Some(String::from(
                    "sha512-Wu1VHeILBK8KAWJUAiSZQX94GmOE45Rg6/538fKwiloUu21KncEkYGPqob2oSZ5mUT73vLGrHQjKw3KMPwfDzg==",
                )),
            },
            ResolvedPackage {
                name: String::from("is-odd"),
                version: String::from("3.0.1"),
                resolved_url: String::from("https://registry.npmjs.org/is-odd/-/is-odd-3.0.1.tgz",),
                integrity: Some(String::from(
                    "sha512-CQpnWPrDwmP1+SMHXZhtLtJv90yiyVfluGsX5iNCVkrhQtU3TQHsUWPG9wkdk9Lgd5yNpAg9jQEo90CBaXgWMA==",
                )),
            },
        ]
    );
}

#[test]
fn derives_package_name_from_nested_deduped_node_modules_key() {
    let contents = br#"{"packages":{"":{},"node_modules/foo/node_modules/@scope/bar":{"version":"2.0.0","resolved":"https://registry.npmjs.org/@scope/bar/-/bar-2.0.0.tgz","integrity":"sha512-AAAA=="}}}"#;

    let packages = parse_resolved_packages(contents).expect("nested package entry should parse");

    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].name, "@scope/bar");
}

#[test]
fn skips_workspace_member_and_its_node_modules_link_entry() {
    let packages = parse_resolved_packages(&workspace_lockfile_fixture())
        .expect("workspace package-lock fixture should parse");

    assert_eq!(
        packages,
        vec![
            ResolvedPackage {
                name: String::from("is-number"),
                version: String::from("6.0.0"),
                resolved_url: String::from(
                    "https://registry.npmjs.org/is-number/-/is-number-6.0.0.tgz",
                ),
                integrity: Some(String::from(
                    "sha512-Wu1VHeILBK8KAWJUAiSZQX94GmOE45Rg6/538fKwiloUu21KncEkYGPqob2oSZ5mUT73vLGrHQjKw3KMPwfDzg==",
                )),
            },
            ResolvedPackage {
                name: String::from("is-odd"),
                version: String::from("3.0.1"),
                resolved_url: String::from("https://registry.npmjs.org/is-odd/-/is-odd-3.0.1.tgz",),
                integrity: Some(String::from(
                    "sha512-CQpnWPrDwmP1+SMHXZhtLtJv90yiyVfluGsX5iNCVkrhQtU3TQHsUWPG9wkdk9Lgd5yNpAg9jQEo90CBaXgWMA==",
                )),
            },
        ]
    );
}

#[test]
fn skips_a_bare_relative_path_workspace_member_key() {
    let contents =
        br#"{"packages":{"":{},"packages/widget":{"name":"@fixture/widget","version":"1.0.0"}}}"#;

    assert_eq!(parse_resolved_packages(contents), Ok(vec![]));
}

#[test]
fn skips_a_node_modules_entry_with_link_true() {
    let contents = br#"{"packages":{"":{},"node_modules/@fixture/widget":{"resolved":"packages/widget","link":true}}}"#;

    assert_eq!(parse_resolved_packages(contents), Ok(vec![]));
}

#[test]
fn does_not_skip_a_node_modules_entry_with_link_false() {
    let contents = br#"{"packages":{"":{},"node_modules/foo":{"version":"1.0.0","resolved":"https://x/foo.tgz","link":false}}}"#;

    assert_eq!(
        parse_resolved_packages(contents),
        Ok(vec![ResolvedPackage {
            name: String::from("foo"),
            version: String::from("1.0.0"),
            resolved_url: String::from("https://x/foo.tgz"),
            integrity: None,
        }])
    );
}

#[test]
fn rejects_invalid_json_with_location() {
    assert_eq!(
        parse_resolved_packages(b"{"),
        Err(LockfileParseError::JsonParseFailed { line: 1, column: 1 })
    );
}

#[test]
fn rejects_non_object_top_level_value() {
    assert_eq!(
        parse_resolved_packages(b"[]"),
        Err(LockfileParseError::TopLevelIsNotObject)
    );
}

#[test]
fn rejects_lockfile_missing_packages_field() {
    assert_eq!(
        parse_resolved_packages(br#"{"name":"x"}"#),
        Err(LockfileParseError::PackagesFieldMissing)
    );
}

#[test]
fn rejects_packages_field_that_is_not_an_object() {
    assert_eq!(
        parse_resolved_packages(br#"{"packages":[]}"#),
        Err(LockfileParseError::PackagesFieldIsNotObject)
    );
}

#[test]
fn rejects_package_entry_that_is_not_an_object() {
    assert_eq!(
        parse_resolved_packages(br#"{"packages":{"node_modules/foo":null}}"#),
        Err(LockfileParseError::PackageEntryIsNotObject {
            key: String::from("node_modules/foo")
        })
    );
}

#[test]
fn rejects_package_entry_key_without_a_package_name() {
    assert_eq!(
        parse_resolved_packages(
            br#"{"packages":{"node_modules/":{"version":"1.0.0","resolved":"https://x/foo.tgz"}}}"#
        ),
        Err(LockfileParseError::PackageEntryKeyHasNoPackageName {
            key: String::from("node_modules/")
        })
    );
}

#[test]
fn rejects_package_entry_missing_version_field() {
    assert_eq!(
        parse_resolved_packages(
            br#"{"packages":{"node_modules/foo":{"resolved":"https://x/foo.tgz"}}}"#
        ),
        Err(LockfileParseError::PackageEntryMissingVersion {
            key: String::from("node_modules/foo")
        })
    );
}

#[test]
fn rejects_package_entry_with_non_string_version() {
    assert_eq!(
        parse_resolved_packages(
            br#"{"packages":{"node_modules/foo":{"version":1,"resolved":"https://x/foo.tgz"}}}"#
        ),
        Err(LockfileParseError::PackageEntryVersionIsNotString {
            key: String::from("node_modules/foo")
        })
    );
}

#[test]
fn rejects_package_entry_missing_resolved_field() {
    assert_eq!(
        parse_resolved_packages(br#"{"packages":{"node_modules/foo":{"version":"1.0.0"}}}"#),
        Err(LockfileParseError::PackageEntryMissingResolved {
            key: String::from("node_modules/foo")
        })
    );
}

#[test]
fn rejects_package_entry_with_non_string_resolved() {
    assert_eq!(
        parse_resolved_packages(
            br#"{"packages":{"node_modules/foo":{"version":"1.0.0","resolved":1}}}"#
        ),
        Err(LockfileParseError::PackageEntryResolvedIsNotString {
            key: String::from("node_modules/foo")
        })
    );
}

#[test]
fn rejects_package_entry_with_non_string_integrity() {
    assert_eq!(
        parse_resolved_packages(br#"{"packages":{"node_modules/foo":{"version":"1.0.0","resolved":"https://x/foo.tgz","integrity":1}}}"#),
        Err(LockfileParseError::PackageEntryIntegrityIsNotString {
            key: String::from("node_modules/foo")
        })
    );
}
