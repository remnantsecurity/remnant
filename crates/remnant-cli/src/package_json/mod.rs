//! npm `package.json` metadata parsing.
//!
//! This module owns parsing already-read package metadata bytes into the small
//! internal shape Remnant needs for the current inspection step. Archive safety
//! and tarball traversal stay in `archive`; this module only handles JSON shape
//! and required metadata fields.

use serde_json::Value;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;

const MAX_PACKAGE_NAME_BYTES: usize = 214;
const MAX_PACKAGE_VERSION_BYTES: usize = 128;
const MAX_DEPENDENCY_NAME_BYTES: usize = 214;
const MAX_DEPENDENCY_VERSION_SPECIFIER_BYTES: usize = 512;
const MAX_DEPENDENCIES_PER_SECTION: usize = 1_000;

const LIFECYCLE_SCRIPT_NAMES: &[&str] = &[
    "dependencies",
    "install",
    "postinstall",
    "postpack",
    "postprepare",
    "postpublish",
    "postversion",
    "preinstall",
    "prepack",
    "prepare",
    "preprepare",
    "prepublish",
    "prepublishOnly",
    "preversion",
    "publish",
    "version",
];

const INSTALL_HOOK_SCRIPT_NAMES: &[&str] = &[
    "install",
    "postinstall",
    "postprepare",
    "preinstall",
    "prepare",
    "preprepare",
    "prepublish",
];

/// One dependency entry parsed from an npm dependency section.
#[derive(Debug, PartialEq, Eq)]
pub struct PackageDependency {
    pub name: String,
    pub version_specifier: String,
}

/// The package metadata Remnant currently extracts from `package.json`.
#[derive(Debug, PartialEq, Eq)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub lifecycle_scripts: Vec<String>,
    pub install_hooks: Vec<String>,
    pub dependencies: Vec<PackageDependency>,
    pub dev_dependencies: Vec<PackageDependency>,
    pub optional_dependencies: Vec<PackageDependency>,
    pub peer_dependencies: Vec<PackageDependency>,
}

/// Errors that can occur while parsing `package.json` metadata.
///
/// These errors describe deterministic JSON and shape failures. They do not
/// include archive traversal failures; those remain owned by `archive`.
#[derive(Debug, PartialEq, Eq)]
pub enum PackageJsonError {
    /// The bytes could not be parsed as JSON.
    JsonParseFailed { line: usize, column: usize },

    /// The JSON parsed successfully, but its top-level value was not an object.
    TopLevelIsNotObject,

    /// The top-level object contains a duplicate key.
    ///
    /// Duplicate keys create a parser differential risk: serde_json keeps the last
    /// value while other JSON parsers keep the first. Legitimate npm packages do not
    /// contain duplicate top-level keys after registry normalization.
    DuplicateKeys,

    /// The required `name` field was missing.
    NameMissing,

    /// The `name` field existed, but was empty or whitespace-only.
    NameIsEmpty,

    /// The `name` field existed, but exceeded the maximum accepted byte length.
    NameIsTooLong { max_bytes: usize },

    /// The `name` field existed, but was not a JSON string.
    NameIsNotString,

    /// The required `version` field was missing.
    VersionMissing,

    /// The `version` field existed, but was empty or whitespace-only.
    VersionIsEmpty,

    /// The `version` field existed, but exceeded the maximum accepted byte length.
    VersionIsTooLong { max_bytes: usize },

    /// The `version` field existed, but was not a JSON string.
    VersionIsNotString,

    /// The optional `scripts` field existed, but was not a JSON object.
    ScriptsIsNotObject,

    /// A `scripts` object entry existed, but its value was not a JSON string.
    ScriptValueIsNotString { script_name: String },

    /// A dependency section existed, but was not a JSON object.
    DependencySectionIsNotObject { section_name: String },

    /// A dependency section exceeded the maximum accepted dependency count.
    DependencySectionHasTooManyEntries {
        section_name: String,
        max_entries: usize,
    },

    /// A dependency section entry name exceeded the maximum accepted byte length.
    DependencyNameIsTooLong {
        section_name: String,
        max_bytes: usize,
    },

    /// A dependency section entry existed, but its version specifier was not a JSON string.
    DependencyVersionSpecifierIsNotString {
        section_name: String,
        dependency_name: String,
    },

    /// A dependency section entry version specifier exceeded the maximum accepted byte length.
    DependencyVersionSpecifierIsTooLong {
        section_name: String,
        dependency_name: String,
        max_bytes: usize,
    },
}

impl fmt::Display for PackageJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageJsonError::JsonParseFailed { line, column } => {
                write!(
                    f,
                    "package.json could not be parsed as JSON at line {line}, column {column}"
                )
            }
            PackageJsonError::TopLevelIsNotObject => {
                write!(f, "package.json top-level value must be an object")
            }
            PackageJsonError::DuplicateKeys => {
                write!(f, "package.json top-level object contains a duplicate key")
            }
            PackageJsonError::NameMissing => {
                write!(f, "package.json is missing required name field")
            }
            PackageJsonError::NameIsEmpty => {
                write!(f, "package.json name field must not be empty")
            }
            PackageJsonError::NameIsTooLong { max_bytes } => {
                write!(
                    f,
                    "package.json name field must not exceed {max_bytes} UTF-8 bytes"
                )
            }
            PackageJsonError::NameIsNotString => {
                write!(f, "package.json name field must be a string")
            }
            PackageJsonError::VersionMissing => {
                write!(f, "package.json is missing required version field")
            }
            PackageJsonError::VersionIsEmpty => {
                write!(f, "package.json version field must not be empty")
            }
            PackageJsonError::VersionIsTooLong { max_bytes } => {
                write!(
                    f,
                    "package.json version field must not exceed {max_bytes} UTF-8 bytes"
                )
            }
            PackageJsonError::VersionIsNotString => {
                write!(f, "package.json version field must be a string")
            }
            PackageJsonError::ScriptsIsNotObject => {
                write!(f, "package.json scripts field must be an object")
            }
            PackageJsonError::ScriptValueIsNotString { script_name } => {
                write!(
                    f,
                    "package.json scripts entry must be a string: {}",
                    script_name.escape_debug()
                )
            }
            PackageJsonError::DependencySectionIsNotObject { section_name } => {
                write!(
                    f,
                    "package.json dependency section must be an object: {}",
                    section_name.escape_debug()
                )
            }
            PackageJsonError::DependencySectionHasTooManyEntries {
                section_name,
                max_entries,
            } => {
                write!(
                    f,
                    "package.json dependency section must not contain more than {max_entries} entries: {}",
                    section_name.escape_debug()
                )
            }
            PackageJsonError::DependencyNameIsTooLong {
                section_name,
                max_bytes,
            } => {
                write!(
                    f,
                    "package.json dependency name must not exceed {max_bytes} UTF-8 bytes: {}",
                    section_name.escape_debug()
                )
            }
            PackageJsonError::DependencyVersionSpecifierIsNotString {
                section_name,
                dependency_name,
            } => {
                write!(
                    f,
                    "package.json dependency version specifier must be a string: {}/{}",
                    section_name.escape_debug(),
                    dependency_name.escape_debug()
                )
            }
            PackageJsonError::DependencyVersionSpecifierIsTooLong {
                section_name,
                dependency_name,
                max_bytes,
            } => {
                write!(
                    f,
                    "package.json dependency version specifier must not exceed {max_bytes} UTF-8 bytes: {}/{}",
                    section_name.escape_debug(),
                    dependency_name.escape_debug()
                )
            }
        }
    }
}

impl Error for PackageJsonError {}

/// Parses the minimal package metadata Remnant needs from `package.json` bytes.
///
/// This function intentionally does not perform full npm package validation yet.
/// For this step, `name` and `version` must exist, must be JSON strings, must
/// not be empty or whitespace-only, and must stay within deterministic byte
/// length limits. Optional lifecycle scripts and install hooks are detected
/// from the `scripts` object without executing or interpreting script commands.
pub fn parse_package_json(contents: &[u8]) -> Result<PackageMetadata, PackageJsonError> {
    detect_duplicate_top_level_keys(contents)?;

    let value: Value =
        serde_json::from_slice(contents).map_err(|error| PackageJsonError::JsonParseFailed {
            line: error.line(),
            column: error.column(),
        })?;

    let Some(object) = value.as_object() else {
        return Err(PackageJsonError::TopLevelIsNotObject);
    };

    let name = match object.get("name") {
        Some(Value::String(name)) => parse_required_metadata_string(
            name,
            PackageJsonError::NameIsEmpty,
            PackageJsonError::NameIsTooLong {
                max_bytes: MAX_PACKAGE_NAME_BYTES,
            },
            MAX_PACKAGE_NAME_BYTES,
        )?,
        Some(_) => return Err(PackageJsonError::NameIsNotString),
        None => return Err(PackageJsonError::NameMissing),
    };

    let version = match object.get("version") {
        Some(Value::String(version)) => parse_required_metadata_string(
            version,
            PackageJsonError::VersionIsEmpty,
            PackageJsonError::VersionIsTooLong {
                max_bytes: MAX_PACKAGE_VERSION_BYTES,
            },
            MAX_PACKAGE_VERSION_BYTES,
        )?,
        Some(_) => return Err(PackageJsonError::VersionIsNotString),
        None => return Err(PackageJsonError::VersionMissing),
    };

    let (lifecycle_scripts, install_hooks) = parse_scripts(object.get("scripts"))?;
    let dependencies = parse_dependency_section(object.get("dependencies"), "dependencies")?;
    let dev_dependencies =
        parse_dependency_section(object.get("devDependencies"), "devDependencies")?;
    let optional_dependencies =
        parse_dependency_section(object.get("optionalDependencies"), "optionalDependencies")?;
    let peer_dependencies =
        parse_dependency_section(object.get("peerDependencies"), "peerDependencies")?;

    Ok(PackageMetadata {
        name,
        version,
        lifecycle_scripts,
        install_hooks,
        dependencies,
        dev_dependencies,
        optional_dependencies,
        peer_dependencies,
    })
}

fn detect_duplicate_top_level_keys(bytes: &[u8]) -> Result<(), PackageJsonError> {
    let mut pos = skip_ws(bytes, 0);

    if bytes.get(pos) != Some(&b'{') {
        return Ok(());
    }

    pos += 1;
    let mut seen: HashSet<Vec<u8>> = HashSet::new();

    loop {
        pos = skip_ws(bytes, pos);

        match bytes.get(pos) {
            Some(&b'}') => return Ok(()),
            Some(&b'"') => {
                pos += 1;
            }
            _ => return Ok(()),
        }

        let Some((key, next)) = read_json_string_bytes(bytes, pos) else {
            return Ok(());
        };
        pos = next;

        if !seen.insert(key) {
            return Err(PackageJsonError::DuplicateKeys);
        }

        pos = skip_ws(bytes, pos);
        if bytes.get(pos) != Some(&b':') {
            return Ok(());
        }

        pos += 1;
        pos = skip_ws(bytes, pos);

        let Some(next) = skip_json_value(bytes, pos) else {
            return Ok(());
        };
        pos = next;

        pos = skip_ws(bytes, pos);
        match bytes.get(pos) {
            Some(&b',') => {
                pos += 1;
            }
            Some(&b'}') => return Ok(()),
            _ => return Ok(()),
        }
    }
}

fn skip_ws(bytes: &[u8], mut pos: usize) -> usize {
    while matches!(bytes.get(pos), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        pos += 1;
    }

    pos
}

fn read_json_string_bytes(bytes: &[u8], mut pos: usize) -> Option<(Vec<u8>, usize)> {
    let mut string_bytes = Vec::new();

    while pos < bytes.len() {
        match bytes[pos] {
            b'\\' => {
                string_bytes.push(bytes[pos]);
                pos += 1;

                let escaped_byte = *bytes.get(pos)?;
                string_bytes.push(escaped_byte);
                pos += 1;
            }
            b'"' => return Some((string_bytes, pos + 1)),
            byte => {
                string_bytes.push(byte);
                pos += 1;
            }
        }
    }

    None
}

fn skip_json_string(bytes: &[u8], mut pos: usize) -> Option<usize> {
    while pos < bytes.len() {
        match bytes[pos] {
            b'\\' => {
                pos += 2;
            }
            b'"' => return Some(pos + 1),
            _ => {
                pos += 1;
            }
        }
    }

    None
}

fn skip_json_value(bytes: &[u8], mut pos: usize) -> Option<usize> {
    match bytes.get(pos)? {
        b'"' => {
            pos += 1;
            skip_json_string(bytes, pos)
        }
        b'{' | b'[' => {
            let open = bytes[pos];
            let close = if open == b'{' { b'}' } else { b']' };
            let mut depth = 1usize;
            pos += 1;

            while pos < bytes.len() {
                match bytes[pos] {
                    b'"' => {
                        pos += 1;
                        pos = skip_json_string(bytes, pos)?;
                    }
                    byte if byte == open => {
                        depth += 1;
                        pos += 1;
                    }
                    byte if byte == close => {
                        depth -= 1;
                        pos += 1;

                        if depth == 0 {
                            return Some(pos);
                        }
                    }
                    _ => {
                        pos += 1;
                    }
                }
            }

            None
        }
        _ => {
            while !matches!(
                bytes.get(pos),
                None | Some(b',' | b'}' | b']' | b' ' | b'\t' | b'\r' | b'\n')
            ) {
                pos += 1;
            }

            Some(pos)
        }
    }
}

fn parse_required_metadata_string(
    value: &str,
    empty_error: PackageJsonError,
    too_long_error: PackageJsonError,
    max_bytes: usize,
) -> Result<String, PackageJsonError> {
    if value.trim().is_empty() {
        return Err(empty_error);
    }

    if value.len() > max_bytes {
        return Err(too_long_error);
    }

    Ok(value.to_string())
}

fn parse_dependency_section(
    value: Option<&Value>,
    section_name: &str,
) -> Result<Vec<PackageDependency>, PackageJsonError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    let Some(dependencies) = value.as_object() else {
        return Err(PackageJsonError::DependencySectionIsNotObject {
            section_name: section_name.to_string(),
        });
    };

    if dependencies.len() > MAX_DEPENDENCIES_PER_SECTION {
        return Err(PackageJsonError::DependencySectionHasTooManyEntries {
            section_name: section_name.to_string(),
            max_entries: MAX_DEPENDENCIES_PER_SECTION,
        });
    }

    let mut dependency_entries: Vec<_> = dependencies.iter().collect();
    dependency_entries.sort_by_key(|(dependency_name, _)| *dependency_name);

    let mut parsed_dependencies = Vec::with_capacity(dependency_entries.len());

    for (dependency_name, version_specifier) in dependency_entries {
        if dependency_name.len() > MAX_DEPENDENCY_NAME_BYTES {
            return Err(PackageJsonError::DependencyNameIsTooLong {
                section_name: section_name.to_string(),
                max_bytes: MAX_DEPENDENCY_NAME_BYTES,
            });
        }

        let Some(version_specifier) = version_specifier.as_str() else {
            return Err(PackageJsonError::DependencyVersionSpecifierIsNotString {
                section_name: section_name.to_string(),
                dependency_name: dependency_name.to_string(),
            });
        };

        if version_specifier.len() > MAX_DEPENDENCY_VERSION_SPECIFIER_BYTES {
            return Err(PackageJsonError::DependencyVersionSpecifierIsTooLong {
                section_name: section_name.to_string(),
                dependency_name: dependency_name.to_string(),
                max_bytes: MAX_DEPENDENCY_VERSION_SPECIFIER_BYTES,
            });
        }

        parsed_dependencies.push(PackageDependency {
            name: dependency_name.to_string(),
            version_specifier: version_specifier.to_string(),
        });
    }

    Ok(parsed_dependencies)
}

fn parse_scripts(value: Option<&Value>) -> Result<(Vec<String>, Vec<String>), PackageJsonError> {
    let Some(value) = value else {
        return Ok((Vec::new(), Vec::new()));
    };

    let Some(scripts) = value.as_object() else {
        return Err(PackageJsonError::ScriptsIsNotObject);
    };

    let mut lifecycle_scripts = Vec::new();
    let mut install_hooks = Vec::new();

    for (script_name, script_command) in scripts {
        if !script_command.is_string() {
            return Err(PackageJsonError::ScriptValueIsNotString {
                script_name: script_name.to_string(),
            });
        }

        if LIFECYCLE_SCRIPT_NAMES.contains(&script_name.as_str()) {
            lifecycle_scripts.push(script_name.to_string());
        }

        if INSTALL_HOOK_SCRIPT_NAMES.contains(&script_name.as_str()) {
            install_hooks.push(script_name.to_string());
        }
    }

    lifecycle_scripts.sort();
    install_hooks.sort();

    Ok((lifecycle_scripts, install_hooks))
}

#[cfg(test)]
mod tests;
