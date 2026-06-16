mod setup;

use super::*;
use setup::{
    MINIMAL_PACKAGE_JSON, dependency, package_json_with_dependency_section, package_metadata,
    parse_fixture_metadata,
};

const BENIGN_MINIMAL_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/benign/minimal-package/package/package.json");
const BENIGN_MINIMAL_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/benign/minimal-package/fixture.json");
const BENIGN_DEPENDENCY_SECTIONS_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/benign/dependency-sections/package/package.json");
const BENIGN_DEPENDENCY_SECTIONS_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/benign/dependency-sections/fixture.json");
const SUSPICIOUS_INSTALL_SCRIPT_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/suspicious/install-script-postinstall/package/package.json");
const SUSPICIOUS_INSTALL_SCRIPT_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/suspicious/install-script-postinstall/fixture.json");
const SUSPICIOUS_NPMRC_FILE_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/suspicious/npmrc-file/package/package.json");
const SUSPICIOUS_NPMRC_FILE_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/suspicious/npmrc-file/fixture.json");
const MALFORMED_SCRIPTS_NOT_OBJECT_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/malformed/scripts-not-object/package/package.json");
const MALFORMED_SCRIPTS_NOT_OBJECT_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/malformed/scripts-not-object/fixture.json");
const MALFORMED_DEPENDENCIES_NOT_OBJECT_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/malformed/dependencies-not-object/package/package.json");
const MALFORMED_DEPENDENCIES_NOT_OBJECT_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/malformed/dependencies-not-object/fixture.json");
const MALFORMED_DEPENDENCY_SPECIFIER_NOT_STRING_FIXTURE: &[u8] = include_bytes!(
    "../../../fixtures/malformed/dependency-specifier-not-string/package/package.json"
);
const MALFORMED_DEPENDENCY_SPECIFIER_NOT_STRING_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/malformed/dependency-specifier-not-string/fixture.json");
const MALFORMED_NON_OBJECT_PACKAGE_JSON_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/malformed/non-object-package-json/package/package.json");
const MALFORMED_NON_OBJECT_PACKAGE_JSON_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/malformed/non-object-package-json/fixture.json");
const MALFORMED_INVALID_JSON_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/malformed/invalid-json/package/package.json");
const MALFORMED_INVALID_JSON_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/malformed/invalid-json/fixture.json");
const MALFORMED_NAME_TOO_LONG_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/malformed/name-too-long/package/package.json");
const MALFORMED_NAME_TOO_LONG_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/malformed/name-too-long/fixture.json");
const MALFORMED_VERSION_TOO_LONG_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/malformed/version-too-long/package/package.json");
const MALFORMED_VERSION_TOO_LONG_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/malformed/version-too-long/fixture.json");
const REGRESSION_NAME_CONTROL_CHARACTER_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/regression/name-control-character/package/package.json");
const REGRESSION_NAME_CONTROL_CHARACTER_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/regression/name-control-character/fixture.json");
const REGRESSION_VERSION_CONTROL_CHARACTER_FIXTURE: &[u8] =
    include_bytes!("../../../fixtures/regression/version-control-character/package/package.json");
const REGRESSION_VERSION_CONTROL_CHARACTER_FIXTURE_METADATA: &[u8] =
    include_bytes!("../../../fixtures/regression/version-control-character/fixture.json");

#[test]
fn parses_minimal_package_metadata() {
    let result = parse_package_json(MINIMAL_PACKAGE_JSON);

    assert_eq!(result, Ok(package_metadata("demo", "1.0.0", &[], &[])));
}

#[test]
fn parses_benign_minimal_fixture() {
    let result = parse_package_json(BENIGN_MINIMAL_FIXTURE);

    assert_eq!(
        result,
        Ok(package_metadata("fixture-minimal", "1.0.0", &[], &[]))
    );
}

#[test]
fn parses_benign_dependency_sections_fixture() {
    let result = parse_package_json(BENIGN_DEPENDENCY_SECTIONS_FIXTURE);

    let mut expected = package_metadata("fixture-dependency-sections", "1.0.0", &[], &[]);
    expected.dependencies = vec![dependency("alpha", "~2.0.0"), dependency("zeta", "^1.0.0")];
    expected.dev_dependencies = vec![dependency("test-tool", "3.0.0")];
    expected.optional_dependencies = vec![dependency("optional-tool", "4.0.0")];
    expected.peer_dependencies = vec![dependency("react", ">=18")];

    assert_eq!(result, Ok(expected));
}

#[test]
fn detects_install_script_fixture() {
    let result = parse_package_json(SUSPICIOUS_INSTALL_SCRIPT_FIXTURE);

    assert_eq!(
        result,
        Ok(package_metadata(
            "fixture-install-script-postinstall",
            "1.0.0",
            &["postinstall"],
            &["postinstall"],
        ))
    );
}

#[test]
fn parses_suspicious_npmrc_file_fixture() {
    let result = parse_package_json(SUSPICIOUS_NPMRC_FILE_FIXTURE);

    assert_eq!(
        result,
        Ok(package_metadata("fixture-npmrc-file", "1.0.0", &[], &[]))
    );
}

#[test]
fn rejects_scripts_not_object_fixture() {
    let result = parse_package_json(MALFORMED_SCRIPTS_NOT_OBJECT_FIXTURE);

    assert_eq!(result, Err(PackageJsonError::ScriptsIsNotObject));
}

#[test]
fn rejects_dependencies_not_object_fixture() {
    let result = parse_package_json(MALFORMED_DEPENDENCIES_NOT_OBJECT_FIXTURE);

    assert_eq!(
        result,
        Err(PackageJsonError::DependencySectionIsNotObject {
            section_name: "dependencies".to_string(),
        })
    );
}

#[test]
fn rejects_dependency_specifier_not_string_fixture() {
    let result = parse_package_json(MALFORMED_DEPENDENCY_SPECIFIER_NOT_STRING_FIXTURE);

    assert_eq!(
        result,
        Err(PackageJsonError::DependencyVersionSpecifierIsNotString {
            section_name: "dependencies".to_string(),
            dependency_name: "left-pad".to_string(),
        })
    );
}

#[test]
fn rejects_non_object_package_json_fixture() {
    let result = parse_package_json(MALFORMED_NON_OBJECT_PACKAGE_JSON_FIXTURE);

    assert_eq!(result, Err(PackageJsonError::TopLevelIsNotObject));
}

#[test]
fn rejects_invalid_json_fixture() {
    let result = parse_package_json(MALFORMED_INVALID_JSON_FIXTURE);

    match result {
        Err(PackageJsonError::JsonParseFailed { line, column }) => {
            assert_eq!(line, 2);
            assert_eq!(column, 0);
        }
        other => panic!("expected JSON parse failure, got {other:?}"),
    }
}

#[test]
fn rejects_name_too_long_fixture() {
    let result = parse_package_json(MALFORMED_NAME_TOO_LONG_FIXTURE);

    assert_eq!(
        result,
        Err(PackageJsonError::NameIsTooLong {
            max_bytes: MAX_PACKAGE_NAME_BYTES,
        })
    );
}

#[test]
fn rejects_version_too_long_fixture() {
    let result = parse_package_json(MALFORMED_VERSION_TOO_LONG_FIXTURE);

    assert_eq!(
        result,
        Err(PackageJsonError::VersionIsTooLong {
            max_bytes: MAX_PACKAGE_VERSION_BYTES,
        })
    );
}

#[test]
fn parses_name_control_character_regression_fixture() {
    let result = parse_package_json(REGRESSION_NAME_CONTROL_CHARACTER_FIXTURE);

    assert_eq!(
        result,
        Ok(package_metadata("fixture-name\ncontrol", "1.0.0", &[], &[],))
    );
}

#[test]
fn parses_version_control_character_regression_fixture() {
    let result = parse_package_json(REGRESSION_VERSION_CONTROL_CHARACTER_FIXTURE);

    assert_eq!(
        result,
        Ok(package_metadata(
            "fixture-version-control-character",
            "1.0.0\ncontrol",
            &[],
            &[],
        ))
    );
}

#[test]
fn validates_benign_minimal_fixture_metadata() {
    let metadata = parse_fixture_metadata(BENIGN_MINIMAL_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "minimal-package");
    assert_eq!(metadata["category"], "benign");
    assert_eq!(metadata["expected"]["package_json"], "pass");
    assert_eq!(metadata["expected"]["install_script_policy"], "pass");
    assert_eq!(metadata["expected"]["exit_code"], 0);
    assert_eq!(
        metadata["expected"]["policy_findings"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_dependency_sections_fixture_metadata() {
    let metadata = parse_fixture_metadata(BENIGN_DEPENDENCY_SECTIONS_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "dependency-sections");
    assert_eq!(metadata["category"], "benign");
    assert_eq!(metadata["expected"]["package_json"], "pass");
    assert_eq!(metadata["expected"]["install_script_policy"], "pass");
    assert_eq!(metadata["expected"]["exit_code"], 0);
    assert_eq!(
        metadata["expected"]["policy_findings"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_install_script_fixture_metadata() {
    let metadata = parse_fixture_metadata(SUSPICIOUS_INSTALL_SCRIPT_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "install-script-postinstall");
    assert_eq!(metadata["category"], "suspicious");
    assert_eq!(metadata["expected"]["package_json"], "pass");
    assert_eq!(metadata["expected"]["install_script_policy"], "fail");
    assert_eq!(metadata["expected"]["exit_code"], 2);
    assert_eq!(
        metadata["expected"]["policy_findings"][0],
        "install-scripts-disallowed"
    );
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_npmrc_file_fixture_metadata() {
    let metadata = parse_fixture_metadata(SUSPICIOUS_NPMRC_FILE_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "npmrc-file");
    assert_eq!(metadata["category"], "suspicious");
    assert_eq!(metadata["expected"]["package_json"], "pass");
    assert_eq!(metadata["expected"]["install_script_policy"], "pass");
    assert_eq!(metadata["expected"]["suspicious_file_policy"], "fail");
    assert_eq!(metadata["expected"]["exit_code"], 2);
    assert_eq!(
        metadata["expected"]["policy_findings"][0],
        "suspicious-file-detected"
    );
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_scripts_not_object_fixture_metadata() {
    let metadata = parse_fixture_metadata(MALFORMED_SCRIPTS_NOT_OBJECT_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "scripts-not-object");
    assert_eq!(metadata["category"], "malformed");
    assert_eq!(metadata["expected"]["package_json"], "fail");
    assert_eq!(
        metadata["expected"]["package_json_error"],
        "ScriptsIsNotObject"
    );
    assert_eq!(
        metadata["expected"]["install_script_policy"],
        "not_evaluated"
    );
    assert_eq!(metadata["expected"]["exit_code"], 1);
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_dependencies_not_object_fixture_metadata() {
    let metadata = parse_fixture_metadata(MALFORMED_DEPENDENCIES_NOT_OBJECT_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "dependencies-not-object");
    assert_eq!(metadata["category"], "malformed");
    assert_eq!(metadata["expected"]["package_json"], "fail");
    assert_eq!(
        metadata["expected"]["package_json_error"],
        "DependencySectionIsNotObject"
    );
    assert_eq!(
        metadata["expected"]["install_script_policy"],
        "not_evaluated"
    );
    assert_eq!(metadata["expected"]["exit_code"], 1);
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_dependency_specifier_not_string_fixture_metadata() {
    let metadata =
        parse_fixture_metadata(MALFORMED_DEPENDENCY_SPECIFIER_NOT_STRING_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "dependency-specifier-not-string");
    assert_eq!(metadata["category"], "malformed");
    assert_eq!(metadata["expected"]["package_json"], "fail");
    assert_eq!(
        metadata["expected"]["package_json_error"],
        "DependencyVersionSpecifierIsNotString"
    );
    assert_eq!(
        metadata["expected"]["install_script_policy"],
        "not_evaluated"
    );
    assert_eq!(metadata["expected"]["exit_code"], 1);
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_non_object_package_json_fixture_metadata() {
    let metadata = parse_fixture_metadata(MALFORMED_NON_OBJECT_PACKAGE_JSON_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "non-object-package-json");
    assert_eq!(metadata["category"], "malformed");
    assert_eq!(metadata["expected"]["package_json"], "fail");
    assert_eq!(
        metadata["expected"]["package_json_error"],
        "TopLevelIsNotObject"
    );
    assert_eq!(
        metadata["expected"]["install_script_policy"],
        "not_evaluated"
    );
    assert_eq!(metadata["expected"]["exit_code"], 1);
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_invalid_json_fixture_metadata() {
    let metadata = parse_fixture_metadata(MALFORMED_INVALID_JSON_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "invalid-json");
    assert_eq!(metadata["category"], "malformed");
    assert_eq!(metadata["expected"]["package_json"], "fail");
    assert_eq!(
        metadata["expected"]["package_json_error"],
        "JsonParseFailed"
    );
    assert_eq!(
        metadata["expected"]["install_script_policy"],
        "not_evaluated"
    );
    assert_eq!(metadata["expected"]["exit_code"], 1);
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_name_too_long_fixture_metadata() {
    let metadata = parse_fixture_metadata(MALFORMED_NAME_TOO_LONG_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "name-too-long");
    assert_eq!(metadata["category"], "malformed");
    assert_eq!(metadata["expected"]["package_json"], "fail");
    assert_eq!(metadata["expected"]["package_json_error"], "NameIsTooLong");
    assert_eq!(
        metadata["expected"]["install_script_policy"],
        "not_evaluated"
    );
    assert_eq!(metadata["expected"]["exit_code"], 1);
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_version_too_long_fixture_metadata() {
    let metadata = parse_fixture_metadata(MALFORMED_VERSION_TOO_LONG_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "version-too-long");
    assert_eq!(metadata["category"], "malformed");
    assert_eq!(metadata["expected"]["package_json"], "fail");
    assert_eq!(
        metadata["expected"]["package_json_error"],
        "VersionIsTooLong"
    );
    assert_eq!(
        metadata["expected"]["install_script_policy"],
        "not_evaluated"
    );
    assert_eq!(metadata["expected"]["exit_code"], 1);
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_name_control_character_regression_fixture_metadata() {
    let metadata = parse_fixture_metadata(REGRESSION_NAME_CONTROL_CHARACTER_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "name-control-character");
    assert_eq!(metadata["category"], "regression");
    assert_eq!(metadata["expected"]["package_json"], "pass");
    assert_eq!(metadata["expected"]["install_script_policy"], "pass");
    assert_eq!(metadata["expected"]["exit_code"], 0);
    assert_eq!(
        metadata["expected"]["policy_findings"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn validates_version_control_character_regression_fixture_metadata() {
    let metadata = parse_fixture_metadata(REGRESSION_VERSION_CONTROL_CHARACTER_FIXTURE_METADATA);

    assert_eq!(metadata["id"], "version-control-character");
    assert_eq!(metadata["category"], "regression");
    assert_eq!(metadata["expected"]["package_json"], "pass");
    assert_eq!(metadata["expected"]["install_script_policy"], "pass");
    assert_eq!(metadata["expected"]["exit_code"], 0);
    assert_eq!(
        metadata["expected"]["policy_findings"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(metadata["safety"]["executes_code"], false);
    assert_eq!(metadata["safety"]["network_access"], false);
    assert_eq!(metadata["safety"]["host_persistence"], false);
}

#[test]
fn rejects_malformed_json() {
    let result = parse_package_json(br#"{"name":"demo","version":"1.0.0""#);

    match result {
        Err(PackageJsonError::JsonParseFailed { line, column }) => {
            assert_eq!(line, 1);
            assert!(column > 0);
        }
        other => panic!("expected JSON parse failure, got {other:?}"),
    }
}

#[test]
fn rejects_top_level_array() {
    let result = parse_package_json(br#"[]"#);

    assert_eq!(result, Err(PackageJsonError::TopLevelIsNotObject));
}

#[test]
fn rejects_top_level_string() {
    let result = parse_package_json(br#""not an object""#);

    assert_eq!(result, Err(PackageJsonError::TopLevelIsNotObject));
}

#[test]
fn rejects_missing_name() {
    let result = parse_package_json(br#"{"version":"1.0.0"}"#);

    assert_eq!(result, Err(PackageJsonError::NameMissing));
}

#[test]
fn rejects_empty_name() {
    let result = parse_package_json(br#"{"name":"","version":"1.0.0"}"#);

    assert_eq!(result, Err(PackageJsonError::NameIsEmpty));
}

#[test]
fn rejects_whitespace_only_name() {
    let result = parse_package_json(br#"{"name":"  \t\n","version":"1.0.0"}"#);

    assert_eq!(result, Err(PackageJsonError::NameIsEmpty));
}

#[test]
fn rejects_non_string_name() {
    let result = parse_package_json(br#"{"name":123,"version":"1.0.0"}"#);

    assert_eq!(result, Err(PackageJsonError::NameIsNotString));
}

#[test]
fn accepts_name_at_max_byte_length() {
    let name = "a".repeat(MAX_PACKAGE_NAME_BYTES);
    let contents = format!(r#"{{"name":"{name}","version":"1.0.0"}}"#);

    let result = parse_package_json(contents.as_bytes());

    assert_eq!(result, Ok(package_metadata(name, "1.0.0", &[], &[])));
}

#[test]
fn rejects_name_over_max_byte_length() {
    let name = "a".repeat(MAX_PACKAGE_NAME_BYTES + 1);
    let contents = format!(r#"{{"name":"{name}","version":"1.0.0"}}"#);

    let result = parse_package_json(contents.as_bytes());

    assert_eq!(
        result,
        Err(PackageJsonError::NameIsTooLong {
            max_bytes: MAX_PACKAGE_NAME_BYTES,
        })
    );
}

#[test]
fn rejects_missing_version() {
    let result = parse_package_json(br#"{"name":"demo"}"#);

    assert_eq!(result, Err(PackageJsonError::VersionMissing));
}

#[test]
fn rejects_empty_version() {
    let result = parse_package_json(br#"{"name":"demo","version":""}"#);

    assert_eq!(result, Err(PackageJsonError::VersionIsEmpty));
}

#[test]
fn rejects_whitespace_only_version() {
    let result = parse_package_json(br#"{"name":"demo","version":"  \t\n"}"#);

    assert_eq!(result, Err(PackageJsonError::VersionIsEmpty));
}

#[test]
fn rejects_non_string_version() {
    let result = parse_package_json(br#"{"name":"demo","version":123}"#);

    assert_eq!(result, Err(PackageJsonError::VersionIsNotString));
}

#[test]
fn accepts_version_at_max_byte_length() {
    let version = "1".repeat(MAX_PACKAGE_VERSION_BYTES);
    let contents = format!(r#"{{"name":"demo","version":"{version}"}}"#);

    let result = parse_package_json(contents.as_bytes());

    assert_eq!(result, Ok(package_metadata("demo", version, &[], &[])));
}

#[test]
fn rejects_version_over_max_byte_length() {
    let version = "1".repeat(MAX_PACKAGE_VERSION_BYTES + 1);
    let contents = format!(r#"{{"name":"demo","version":"{version}"}}"#);

    let result = parse_package_json(contents.as_bytes());

    assert_eq!(
        result,
        Err(PackageJsonError::VersionIsTooLong {
            max_bytes: MAX_PACKAGE_VERSION_BYTES,
        })
    );
}

#[test]
fn preserves_accepted_metadata_strings_exactly() {
    let result = parse_package_json(br#"{"name":" demo ","version":" 1.0.0 "}"#);

    assert_eq!(result, Ok(package_metadata(" demo ", " 1.0.0 ", &[], &[])));
}

#[test]
fn accepts_missing_scripts_as_no_lifecycle_scripts_or_install_hooks() {
    let result = parse_package_json(MINIMAL_PACKAGE_JSON);

    assert_eq!(result, Ok(package_metadata("demo", "1.0.0", &[], &[])));
}

#[test]
fn accepts_empty_scripts_object() {
    let result = parse_package_json(br#"{"name":"demo","version":"1.0.0","scripts":{}}"#);

    assert_eq!(result, Ok(package_metadata("demo", "1.0.0", &[], &[])));
}

#[test]
fn detects_lifecycle_scripts_in_deterministic_order() {
    let result = parse_package_json(
        br#"{
            "name":"demo",
            "version":"1.0.0",
            "scripts":{
                "zzz-custom":"ignored",
                "prepare":"node prepare.js",
                "preversion":"node preversion.js",
                "build":"node build.js",
                "postpack":"node postpack.js"
            }
        }"#,
    );

    assert_eq!(
        result,
        Ok(package_metadata(
            "demo",
            "1.0.0",
            &["postpack", "prepare", "preversion"],
            &["prepare"],
        ))
    );
}

#[test]
fn detects_install_hooks_as_lifecycle_script_subset() {
    let result = parse_package_json(
        br#"{
            "name":"demo",
            "version":"1.0.0",
            "scripts":{
                "postinstall":"node postinstall.js",
                "install":"node install.js",
                "preinstall":"node preinstall.js",
                "postprepare":"node postprepare.js",
                "preprepare":"node preprepare.js",
                "prepublish":"node prepublish.js"
            }
        }"#,
    );

    assert_eq!(
        result,
        Ok(package_metadata(
            "demo",
            "1.0.0",
            &[
                "install",
                "postinstall",
                "postprepare",
                "preinstall",
                "preprepare",
                "prepublish",
            ],
            &[
                "install",
                "postinstall",
                "postprepare",
                "preinstall",
                "preprepare",
                "prepublish",
            ],
        ))
    );
}

#[test]
fn ignores_non_lifecycle_scripts() {
    let result = parse_package_json(
        br#"{
            "name":"demo",
            "version":"1.0.0",
            "scripts":{
                "build":"node build.js",
                "test":"node test.js",
                "Postinstall":"case sensitive and ignored"
            }
        }"#,
    );

    assert_eq!(result, Ok(package_metadata("demo", "1.0.0", &[], &[])));
}

#[test]
fn rejects_scripts_when_not_an_object() {
    let result = parse_package_json(br#"{"name":"demo","version":"1.0.0","scripts":[]}"#);

    assert_eq!(result, Err(PackageJsonError::ScriptsIsNotObject));
}

#[test]
fn accepts_missing_dependency_sections_as_empty_metadata() {
    let result = parse_package_json(MINIMAL_PACKAGE_JSON);

    assert_eq!(result, Ok(package_metadata("demo", "1.0.0", &[], &[])));
}

#[test]
fn parses_dependency_sections_in_deterministic_order() {
    let result = parse_package_json(
        br#"{
            "name":"demo",
            "version":"1.0.0",
            "dependencies":{
                "zeta":"^1.0.0",
                "alpha":"~2.0.0"
            },
            "devDependencies":{
                "test-tool":"3.0.0"
            },
            "optionalDependencies":{
                "optional-tool":"4.0.0"
            },
            "peerDependencies":{
                "react":">=18"
            }
        }"#,
    );

    let mut expected = package_metadata("demo", "1.0.0", &[], &[]);
    expected.dependencies = vec![dependency("alpha", "~2.0.0"), dependency("zeta", "^1.0.0")];
    expected.dev_dependencies = vec![dependency("test-tool", "3.0.0")];
    expected.optional_dependencies = vec![dependency("optional-tool", "4.0.0")];
    expected.peer_dependencies = vec![dependency("react", ">=18")];

    assert_eq!(result, Ok(expected));
}

#[test]
fn accepts_empty_dependency_sections() {
    let result = parse_package_json(
        br#"{
            "name":"demo",
            "version":"1.0.0",
            "dependencies":{},
            "devDependencies":{},
            "optionalDependencies":{},
            "peerDependencies":{}
        }"#,
    );

    assert_eq!(result, Ok(package_metadata("demo", "1.0.0", &[], &[])));
}

#[test]
fn preserves_dependency_names_and_version_specifiers_exactly() {
    let result = parse_package_json(
        br#"{
            "name":"demo",
            "version":"1.0.0",
            "dependencies":{
                " package ":" npm:other@1.0.0 ",
                "empty-specifier":""
            }
        }"#,
    );

    let mut expected = package_metadata("demo", "1.0.0", &[], &[]);
    expected.dependencies = vec![
        dependency(" package ", " npm:other@1.0.0 "),
        dependency("empty-specifier", ""),
    ];

    assert_eq!(result, Ok(expected));
}

#[test]
fn preserves_dependency_strings_containing_json_special_characters() {
    let dependency_name = r#"quote"and\backslash"#.to_string();
    let version_specifier = r#"npm:other@"1.0.0"\with\slashes"#.to_string();
    let contents = package_json_with_dependency_section(
        "dependencies",
        &[(dependency_name.clone(), version_specifier.clone())],
    );

    let result = parse_package_json(contents.as_bytes());

    let mut expected = package_metadata("demo", "1.0.0", &[], &[]);
    expected.dependencies = vec![dependency(dependency_name, version_specifier)];

    assert_eq!(result, Ok(expected));
}

#[test]
fn accepts_dependency_name_at_max_byte_length() {
    let dependency_name = "a".repeat(MAX_DEPENDENCY_NAME_BYTES);
    let contents = package_json_with_dependency_section(
        "dependencies",
        &[(dependency_name.clone(), "^1.0.0".to_string())],
    );

    let result = parse_package_json(contents.as_bytes());

    let mut expected = package_metadata("demo", "1.0.0", &[], &[]);
    expected.dependencies = vec![dependency(dependency_name, "^1.0.0")];

    assert_eq!(result, Ok(expected));
}

#[test]
fn rejects_dependency_name_over_max_byte_length() {
    let dependency_name = "a".repeat(MAX_DEPENDENCY_NAME_BYTES + 1);
    let contents = package_json_with_dependency_section(
        "dependencies",
        &[(dependency_name, "^1.0.0".to_string())],
    );

    let result = parse_package_json(contents.as_bytes());

    assert_eq!(
        result,
        Err(PackageJsonError::DependencyNameIsTooLong {
            section_name: "dependencies".to_string(),
            max_bytes: MAX_DEPENDENCY_NAME_BYTES,
        })
    );
}

#[test]
fn accepts_dependency_version_specifier_at_max_byte_length() {
    let version_specifier = "1".repeat(MAX_DEPENDENCY_VERSION_SPECIFIER_BYTES);
    let contents = package_json_with_dependency_section(
        "dependencies",
        &[("left-pad".to_string(), version_specifier.clone())],
    );

    let result = parse_package_json(contents.as_bytes());

    let mut expected = package_metadata("demo", "1.0.0", &[], &[]);
    expected.dependencies = vec![dependency("left-pad", version_specifier)];

    assert_eq!(result, Ok(expected));
}

#[test]
fn rejects_dependency_version_specifier_over_max_byte_length() {
    let version_specifier = "1".repeat(MAX_DEPENDENCY_VERSION_SPECIFIER_BYTES + 1);
    let contents = package_json_with_dependency_section(
        "dependencies",
        &[("left-pad".to_string(), version_specifier)],
    );

    let result = parse_package_json(contents.as_bytes());

    assert_eq!(
        result,
        Err(PackageJsonError::DependencyVersionSpecifierIsTooLong {
            section_name: "dependencies".to_string(),
            dependency_name: "left-pad".to_string(),
            max_bytes: MAX_DEPENDENCY_VERSION_SPECIFIER_BYTES,
        })
    );
}

#[test]
fn accepts_dependency_section_at_max_entry_count() {
    let dependencies = (0..MAX_DEPENDENCIES_PER_SECTION)
        .map(|index| (format!("dep-{index:04}"), "1.0.0".to_string()))
        .collect::<Vec<_>>();
    let contents = package_json_with_dependency_section("dependencies", &dependencies);

    let result = parse_package_json(contents.as_bytes());

    let metadata = result.expect("dependency section at entry limit should parse");
    assert_eq!(metadata.dependencies.len(), MAX_DEPENDENCIES_PER_SECTION);
    assert_eq!(metadata.dependencies[0], dependency("dep-0000", "1.0.0"));
    assert_eq!(
        metadata.dependencies[MAX_DEPENDENCIES_PER_SECTION - 1],
        dependency("dep-0999", "1.0.0")
    );
}

#[test]
fn rejects_dependency_section_over_max_entry_count() {
    let dependencies = (0..=MAX_DEPENDENCIES_PER_SECTION)
        .map(|index| (format!("dep-{index:04}"), "1.0.0".to_string()))
        .collect::<Vec<_>>();
    let contents = package_json_with_dependency_section("dependencies", &dependencies);

    let result = parse_package_json(contents.as_bytes());

    assert_eq!(
        result,
        Err(PackageJsonError::DependencySectionHasTooManyEntries {
            section_name: "dependencies".to_string(),
            max_entries: MAX_DEPENDENCIES_PER_SECTION,
        })
    );
}

#[test]
fn rejects_dependency_section_when_not_an_object() {
    let result =
        parse_package_json(br#"{"name":"demo","version":"1.0.0","optionalDependencies":[]}"#);

    assert_eq!(
        result,
        Err(PackageJsonError::DependencySectionIsNotObject {
            section_name: "optionalDependencies".to_string(),
        })
    );
}

#[test]
fn rejects_non_string_dependency_version_specifier() {
    let result = parse_package_json(
        br#"{"name":"demo","version":"1.0.0","dependencies":{"left-pad":true}}"#,
    );

    assert_eq!(
        result,
        Err(PackageJsonError::DependencyVersionSpecifierIsNotString {
            section_name: "dependencies".to_string(),
            dependency_name: "left-pad".to_string(),
        })
    );
}

#[test]
fn rejects_non_string_script_values() {
    let result =
        parse_package_json(br#"{"name":"demo","version":"1.0.0","scripts":{"postinstall":true}}"#);

    assert_eq!(
        result,
        Err(PackageJsonError::ScriptValueIsNotString {
            script_name: "postinstall".to_string(),
        })
    );
}
