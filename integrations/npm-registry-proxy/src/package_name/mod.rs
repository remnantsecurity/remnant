use std::fmt;

const MAX_PACKAGE_NAME_BYTES: usize = 214;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPackageName(String);

impl ValidatedPackageName {
    pub fn parse(package_name: String) -> Result<Self, PackageNameError> {
        validate_package_name(&package_name)?;
        Ok(Self(package_name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PackageNameError {
    Empty,
    TooLong { max_bytes: usize },
    ContainsUppercase,
    ContainsUnsupportedCharacter,
    StartsWithDisallowedCharacter,
    InvalidScopedName,
}

impl fmt::Display for PackageNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageNameError::Empty => write!(formatter, "package name is empty"),
            PackageNameError::TooLong { max_bytes } => {
                write!(formatter, "package name exceeds {max_bytes} byte limit")
            }
            PackageNameError::ContainsUppercase => {
                write!(formatter, "package name contains uppercase ASCII")
            }
            PackageNameError::ContainsUnsupportedCharacter => {
                write!(formatter, "package name contains unsupported characters")
            }
            PackageNameError::StartsWithDisallowedCharacter => {
                write!(
                    formatter,
                    "unscoped package name starts with a disallowed character"
                )
            }
            PackageNameError::InvalidScopedName => {
                write!(formatter, "scoped package name is invalid")
            }
        }
    }
}

impl std::error::Error for PackageNameError {}

fn validate_package_name(package_name: &str) -> Result<(), PackageNameError> {
    if package_name.is_empty() {
        return Err(PackageNameError::Empty);
    }

    if package_name.len() > MAX_PACKAGE_NAME_BYTES {
        return Err(PackageNameError::TooLong {
            max_bytes: MAX_PACKAGE_NAME_BYTES,
        });
    }

    if package_name.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err(PackageNameError::ContainsUppercase);
    }

    if package_name.starts_with('@') {
        validate_scoped_package_name(package_name)
    } else {
        validate_unscoped_package_name(package_name)
    }
}

fn validate_unscoped_package_name(package_name: &str) -> Result<(), PackageNameError> {
    if package_name.starts_with('.') || package_name.starts_with('_') {
        return Err(PackageNameError::StartsWithDisallowedCharacter);
    }

    if package_name.contains('/') {
        return Err(PackageNameError::InvalidScopedName);
    }

    validate_package_name_part(package_name)
}

fn validate_scoped_package_name(package_name: &str) -> Result<(), PackageNameError> {
    let Some((scope, name)) = package_name[1..].split_once('/') else {
        return Err(PackageNameError::InvalidScopedName);
    };

    if scope.is_empty() || name.is_empty() || name.contains('/') {
        return Err(PackageNameError::InvalidScopedName);
    }

    validate_package_name_part(scope)?;
    validate_package_name_part(name)
}

fn validate_package_name_part(package_name_part: &str) -> Result<(), PackageNameError> {
    if package_name_part.bytes().all(is_url_safe_package_name_byte) {
        Ok(())
    } else {
        Err(PackageNameError::ContainsUnsupportedCharacter)
    }
}

fn is_url_safe_package_name_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

#[cfg(test)]
mod tests;
