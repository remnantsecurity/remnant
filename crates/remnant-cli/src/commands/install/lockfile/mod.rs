//! Parsing for npm package-lock files produced during install resolution.

use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub resolved_url: Option<String>,
    pub integrity: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LockfileParseError {
    JsonParseFailed { line: usize, column: usize },
    TopLevelIsNotObject,
    PackagesFieldMissing,
    PackagesFieldIsNotObject,
    PackageEntryIsNotObject { key: String },
    PackageEntryKeyHasNoPackageName { key: String },
    PackageEntryMissingVersion { key: String },
    PackageEntryVersionIsNotString { key: String },
    PackageEntryResolvedIsNotString { key: String },
    PackageEntryIntegrityIsNotString { key: String },
}

impl fmt::Display for LockfileParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockfileParseError::JsonParseFailed { line, column } => {
                write!(
                    f,
                    "package-lock.json is not valid JSON at line {line}, column {column}"
                )
            }
            LockfileParseError::TopLevelIsNotObject => {
                write!(f, "package-lock.json top-level value must be an object")
            }
            LockfileParseError::PackagesFieldMissing => {
                write!(f, "package-lock.json is missing the packages field")
            }
            LockfileParseError::PackagesFieldIsNotObject => {
                write!(f, "package-lock.json packages field must be an object")
            }
            LockfileParseError::PackageEntryIsNotObject { key } => {
                write!(
                    f,
                    "package-lock.json package entry {key:?} must be an object"
                )
            }
            LockfileParseError::PackageEntryKeyHasNoPackageName { key } => write!(
                f,
                "package-lock.json package entry key {key:?} has no package name"
            ),
            LockfileParseError::PackageEntryMissingVersion { key } => write!(
                f,
                "package-lock.json package entry {key:?} is missing the version field"
            ),
            LockfileParseError::PackageEntryVersionIsNotString { key } => write!(
                f,
                "package-lock.json package entry {key:?} version field must be a string"
            ),
            LockfileParseError::PackageEntryResolvedIsNotString { key } => write!(
                f,
                "package-lock.json package entry {key:?} resolved field must be a string"
            ),
            LockfileParseError::PackageEntryIntegrityIsNotString { key } => write!(
                f,
                "package-lock.json package entry {key:?} integrity field must be a string"
            ),
        }
    }
}

pub fn parse_resolved_packages(
    contents: &[u8],
) -> Result<Vec<ResolvedPackage>, LockfileParseError> {
    let value: Value =
        serde_json::from_slice(contents).map_err(|error| LockfileParseError::JsonParseFailed {
            line: error.line(),
            column: error.column(),
        })?;
    let top_level = value
        .as_object()
        .ok_or(LockfileParseError::TopLevelIsNotObject)?;
    let packages = top_level
        .get("packages")
        .ok_or(LockfileParseError::PackagesFieldMissing)?
        .as_object()
        .ok_or(LockfileParseError::PackagesFieldIsNotObject)?;

    let mut resolved_packages = Vec::with_capacity(packages.len().saturating_sub(1));

    for (key, entry) in packages {
        if key.is_empty() {
            continue;
        }

        let entry = entry
            .as_object()
            .ok_or_else(|| LockfileParseError::PackageEntryIsNotObject { key: key.clone() })?;
        let Some((_, name)) = key.rsplit_once("node_modules/") else {
            // No `node_modules/` in the key at all: this is an npm workspace-member
            // entry, keyed by its plain relative path (e.g. "packages/widget"). It
            // carries the workspace's own package.json fields, not resolved
            // dependency data — nothing to fetch or verify.
            continue;
        };

        if name.is_empty() {
            return Err(LockfileParseError::PackageEntryKeyHasNoPackageName { key: key.clone() });
        }

        if entry.get("link").and_then(Value::as_bool) == Some(true) {
            // A node_modules/<name> entry for an npm workspace member is npm's own
            // symlink-into-node_modules bookkeeping for that workspace: `resolved` is
            // a relative filesystem path (not a fetchable URL) and there is no
            // integrity hash. Not a registry-resolved dependency.
            continue;
        }

        let resolved_url = optional_resolved_url_field(entry, key)?;

        if resolved_url.is_none() && entry.get("inBundle").and_then(Value::as_bool) == Some(true) {
            // A bundled dependency missing `resolved` has no independent registry
            // artifact to fetch — its bytes ship inside the bundling parent's own
            // tarball, which is independently fetched and integrity-verified.
            // See docs/decisions/0053-lockfile-resolve-coverage-guarantee.md.
            continue;
        }

        let version = required_version_field(entry, key)?;
        let integrity = match entry.get("integrity") {
            Some(value) => Some(
                value
                    .as_str()
                    .ok_or_else(|| LockfileParseError::PackageEntryIntegrityIsNotString {
                        key: key.clone(),
                    })?
                    .to_owned(),
            ),
            None => None,
        };

        resolved_packages.push(ResolvedPackage {
            name: name.to_owned(),
            version,
            resolved_url,
            integrity,
        });
    }

    resolved_packages
        .sort_by(|left, right| (&left.name, &left.version).cmp(&(&right.name, &right.version)));
    Ok(resolved_packages)
}

fn required_version_field(
    entry: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, LockfileParseError> {
    entry
        .get("version")
        .ok_or_else(|| LockfileParseError::PackageEntryMissingVersion {
            key: key.to_owned(),
        })?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| LockfileParseError::PackageEntryVersionIsNotString {
            key: key.to_owned(),
        })
}

fn optional_resolved_url_field(
    entry: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<String>, LockfileParseError> {
    match entry.get("resolved") {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .map(|resolved| Some(resolved.to_owned()))
            .ok_or_else(|| LockfileParseError::PackageEntryResolvedIsNotString {
                key: key.to_owned(),
            }),
    }
}

#[cfg(test)]
mod tests;
