//! Bounded, offline package-name lexical-similarity comparison.
//!
//! This module reports only a reproducible name relationship: one ASCII-byte
//! insertion, deletion, substitution, or adjacent transposition at edit
//! distance exactly one. It does not classify intent, normalize input, or
//! participate in CLI command or policy evaluation.
//!
//! `CanonicalPackageName` temporarily duplicates the validation grammar in
//! `integrations/npm-registry-proxy/src/package_name/mod.rs` because that
//! integration is a disconnected Cargo workspace.

use std::fmt;

pub(crate) const MAX_PACKAGE_NAME_BYTES: usize = 214;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalPackageName(String);

impl CanonicalPackageName {
    pub(crate) fn parse(name: &str) -> Result<Self, CanonicalPackageNameError> {
        validate_canonical_package_name(name)?;
        Ok(Self(name.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditOperation {
    Insertion,
    Deletion,
    Substitution,
    Transposition,
}

impl EditOperation {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Insertion => "insertion",
            Self::Deletion => "deletion",
            Self::Substitution => "substitution",
            Self::Transposition => "transposition",
        }
    }
}

/// Returns the operation that transforms `candidate` into `reference` at
/// restricted Damerau-Levenshtein distance exactly one.
///
/// Match existence is symmetric. Insertion and deletion labels reverse with
/// argument order, while substitution and transposition labels do not.
pub(crate) fn distance_one_operation(
    candidate: &CanonicalPackageName,
    reference: &CanonicalPackageName,
) -> Option<EditOperation> {
    let candidate = candidate.as_str().as_bytes();
    let reference = reference.as_str().as_bytes();

    match candidate.len().abs_diff(reference.len()) {
        0 => same_length_operation(candidate, reference),
        1 if candidate.len() < reference.len() => {
            one_edit_apart(candidate, reference).then_some(EditOperation::Insertion)
        }
        1 => one_edit_apart(reference, candidate).then_some(EditOperation::Deletion),
        _ => None,
    }
}

fn same_length_operation(left: &[u8], right: &[u8]) -> Option<EditOperation> {
    let mut first_mismatch = None;
    let mut second_mismatch = None;

    for (index, (left_byte, right_byte)) in left.iter().zip(right.iter()).enumerate() {
        if left_byte != right_byte {
            match (first_mismatch, second_mismatch) {
                (None, _) => first_mismatch = Some(index),
                (Some(_), None) => second_mismatch = Some(index),
                (Some(_), Some(_)) => return None,
            }
        }
    }

    match (first_mismatch, second_mismatch) {
        (None, _) => None,
        (Some(_), None) => Some(EditOperation::Substitution),
        (Some(first), Some(second))
            if second == first + 1
                && left[first] == right[second]
                && left[second] == right[first] =>
        {
            Some(EditOperation::Transposition)
        }
        (Some(_), Some(_)) => None,
    }
}

/// `long` must be exactly one byte longer than `short`.
fn one_edit_apart(short: &[u8], long: &[u8]) -> bool {
    debug_assert_eq!(long.len(), short.len() + 1);

    let mut short_index = 0;
    let mut long_index = 0;
    let mut found_difference = false;

    while short_index < short.len() && long_index < long.len() {
        if short[short_index] != long[long_index] {
            if found_difference {
                return false;
            }
            found_difference = true;
            long_index += 1;
        } else {
            short_index += 1;
            long_index += 1;
        }
    }

    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanonicalPackageNameError {
    Empty,
    TooLong { max_bytes: usize },
    ContainsUppercase,
    ContainsUnsupportedCharacter,
    StartsWithDisallowedCharacter,
    InvalidScopedName,
}

impl fmt::Display for CanonicalPackageNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "package name is empty"),
            Self::TooLong { max_bytes } => {
                write!(formatter, "package name exceeds {max_bytes} byte limit")
            }
            Self::ContainsUppercase => write!(formatter, "package name contains uppercase ASCII"),
            Self::ContainsUnsupportedCharacter => {
                write!(formatter, "package name contains unsupported characters")
            }
            Self::StartsWithDisallowedCharacter => write!(
                formatter,
                "unscoped package name starts with a disallowed character"
            ),
            Self::InvalidScopedName => write!(formatter, "scoped package name is invalid"),
        }
    }
}

impl std::error::Error for CanonicalPackageNameError {}

fn validate_canonical_package_name(name: &str) -> Result<(), CanonicalPackageNameError> {
    if name.is_empty() {
        return Err(CanonicalPackageNameError::Empty);
    }

    if name.len() > MAX_PACKAGE_NAME_BYTES {
        return Err(CanonicalPackageNameError::TooLong {
            max_bytes: MAX_PACKAGE_NAME_BYTES,
        });
    }

    if name.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return Err(CanonicalPackageNameError::ContainsUppercase);
    }

    if let Some(scoped_name) = name.strip_prefix('@') {
        let Some((scope, local_name)) = scoped_name.split_once('/') else {
            return Err(CanonicalPackageNameError::InvalidScopedName);
        };

        if scope.is_empty() || local_name.is_empty() || local_name.contains('/') {
            return Err(CanonicalPackageNameError::InvalidScopedName);
        }

        validate_package_name_part(scope)?;
        validate_package_name_part(local_name)
    } else {
        if name.starts_with('.') || name.starts_with('_') {
            return Err(CanonicalPackageNameError::StartsWithDisallowedCharacter);
        }

        if name.contains('/') {
            return Err(CanonicalPackageNameError::InvalidScopedName);
        }

        validate_package_name_part(name)
    }
}

fn validate_package_name_part(part: &str) -> Result<(), CanonicalPackageNameError> {
    if part.bytes().all(|byte| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'-' | b'.' | b'_' | b'~')
    }) {
        Ok(())
    } else {
        Err(CanonicalPackageNameError::ContainsUnsupportedCharacter)
    }
}
