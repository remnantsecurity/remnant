use super::*;

#[test]
fn accepts_unscoped_package_name_at_max_byte_length() {
    let package_name = "a".repeat(MAX_PACKAGE_NAME_BYTES);

    let validated = ValidatedPackageName::parse(package_name.clone()).unwrap();

    assert_eq!(validated.as_str(), package_name);
}

#[test]
fn accepts_scoped_package_name() {
    let validated = ValidatedPackageName::parse(String::from("@babel/core")).unwrap();

    assert_eq!(validated.as_str(), "@babel/core");
}

#[test]
fn rejects_empty_package_name() {
    let error = ValidatedPackageName::parse(String::new()).err().unwrap();

    assert_eq!(error, PackageNameError::Empty);
}

#[test]
fn rejects_package_name_over_max_byte_length() {
    let package_name = "a".repeat(MAX_PACKAGE_NAME_BYTES + 1);

    let error = ValidatedPackageName::parse(package_name).err().unwrap();

    assert_eq!(
        error,
        PackageNameError::TooLong {
            max_bytes: MAX_PACKAGE_NAME_BYTES
        }
    );
}

#[test]
fn rejects_uppercase_package_name() {
    let error = ValidatedPackageName::parse(String::from("Left-Pad"))
        .err()
        .unwrap();

    assert_eq!(error, PackageNameError::ContainsUppercase);
}

#[test]
fn rejects_unscoped_package_name_starting_with_dot() {
    let error = ValidatedPackageName::parse(String::from(".left-pad"))
        .err()
        .unwrap();

    assert_eq!(error, PackageNameError::StartsWithDisallowedCharacter);
}

#[test]
fn rejects_unscoped_package_name_starting_with_underscore() {
    let error = ValidatedPackageName::parse(String::from("_left-pad"))
        .err()
        .unwrap();

    assert_eq!(error, PackageNameError::StartsWithDisallowedCharacter);
}

#[test]
fn rejects_unscoped_package_name_with_slash() {
    let error = ValidatedPackageName::parse(String::from("left/pad"))
        .err()
        .unwrap();

    assert_eq!(error, PackageNameError::InvalidScopedName);
}

#[test]
fn rejects_leading_slash_package_name() {
    let error = ValidatedPackageName::parse(String::from("//evil.example/path"))
        .err()
        .unwrap();

    assert_eq!(error, PackageNameError::InvalidScopedName);
}

#[test]
fn rejects_scoped_package_name_without_name() {
    let error = ValidatedPackageName::parse(String::from("@scope/"))
        .err()
        .unwrap();

    assert_eq!(error, PackageNameError::InvalidScopedName);
}

#[test]
fn rejects_scoped_package_name_with_nested_slash() {
    let error = ValidatedPackageName::parse(String::from("@scope/name/extra"))
        .err()
        .unwrap();

    assert_eq!(error, PackageNameError::InvalidScopedName);
}

#[test]
fn rejects_unsupported_package_name_character() {
    let error = ValidatedPackageName::parse(String::from("left pad"))
        .err()
        .unwrap();

    assert_eq!(error, PackageNameError::ContainsUnsupportedCharacter);
}
