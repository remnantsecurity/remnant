use std::collections::HashMap;
use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::Url;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256, Sha512};

use crate::package_name::ValidatedPackageName;

const MAX_VERSIONS: usize = 7_500;
const MAX_DIST_TAGS: usize = 256;
const MAX_VERSION_BYTES: usize = 128;
const MAX_TARBALL_URL_BYTES: usize = 256;
const MAX_INTEGRITY_BYTES: usize = 128;

pub struct RewrittenPackument {
    pub bytes: Vec<u8>,
    pub artifacts: HashMap<String, ArtifactMapping>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactMapping {
    pub package_name: String,
    pub version: String,
    pub upstream_url: String,
    pub integrity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityStatus {
    Verified,
    Mismatch,
    Absent,
    Unsupported,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PackumentRewriteError {
    InvalidJson,
    RootIsNotObject,
    MissingPackageName,
    InvalidPackageName,
    VersionsMissing,
    VersionsIsNotObject,
    TooManyVersions { limit: usize },
    DistTagsIsNotObject,
    TooManyDistTags { limit: usize },
    VersionStringIsEmpty,
    VersionStringTooLong { limit: usize },
    VersionStringHasUnsupportedCharacter,
    VersionEntryIsNotObject,
    DistIsNotObject,
    TarballUrlMissing,
    TarballUrlIsNotString,
    TarballUrlTooLong { limit: usize },
    TarballUrlInvalid,
    TarballUrlSchemeNotHttps,
    IntegrityIsNotString,
    IntegrityTooLong { limit: usize },
    ProxyOriginInvalid,
    SerializationFailed,
}

impl fmt::Display for PackumentRewriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackumentRewriteError::InvalidJson => write!(formatter, "packument is not valid JSON"),
            PackumentRewriteError::RootIsNotObject => {
                write!(formatter, "packument JSON root is not an object")
            }
            PackumentRewriteError::MissingPackageName => {
                write!(formatter, "packument package name is missing")
            }
            PackumentRewriteError::InvalidPackageName => {
                write!(formatter, "packument package name is invalid")
            }
            PackumentRewriteError::VersionsMissing => {
                write!(formatter, "packument versions object is missing")
            }
            PackumentRewriteError::VersionsIsNotObject => {
                write!(formatter, "packument versions field is not an object")
            }
            PackumentRewriteError::TooManyVersions { limit } => {
                write!(formatter, "packument exceeds {limit} version entries")
            }
            PackumentRewriteError::DistTagsIsNotObject => {
                write!(formatter, "packument dist-tags field is not an object")
            }
            PackumentRewriteError::TooManyDistTags { limit } => {
                write!(formatter, "packument exceeds {limit} dist-tag entries")
            }
            PackumentRewriteError::VersionStringIsEmpty => write!(formatter, "version is empty"),
            PackumentRewriteError::VersionStringTooLong { limit } => {
                write!(formatter, "version exceeds {limit} byte limit")
            }
            PackumentRewriteError::VersionStringHasUnsupportedCharacter => {
                write!(formatter, "version contains an unsupported character")
            }
            PackumentRewriteError::VersionEntryIsNotObject => {
                write!(formatter, "version entry is not an object")
            }
            PackumentRewriteError::DistIsNotObject => {
                write!(formatter, "dist field is not an object")
            }
            PackumentRewriteError::TarballUrlMissing => write!(formatter, "tarball URL is missing"),
            PackumentRewriteError::TarballUrlIsNotString => {
                write!(formatter, "tarball URL is not a string")
            }
            PackumentRewriteError::TarballUrlTooLong { limit } => {
                write!(formatter, "tarball URL exceeds {limit} byte limit")
            }
            PackumentRewriteError::TarballUrlInvalid => write!(formatter, "tarball URL is invalid"),
            PackumentRewriteError::TarballUrlSchemeNotHttps => {
                write!(formatter, "tarball URL scheme is not https")
            }
            PackumentRewriteError::IntegrityIsNotString => {
                write!(formatter, "integrity value is not a string")
            }
            PackumentRewriteError::IntegrityTooLong { limit } => {
                write!(formatter, "integrity value exceeds {limit} byte limit")
            }
            PackumentRewriteError::ProxyOriginInvalid => {
                write!(formatter, "proxy origin is invalid")
            }
            PackumentRewriteError::SerializationFailed => {
                write!(formatter, "rewritten packument serialization failed")
            }
        }
    }
}

impl std::error::Error for PackumentRewriteError {}

pub fn rewrite_packument_tarball_urls(
    bytes: &[u8],
    proxy_origin: &str,
) -> Result<RewrittenPackument, PackumentRewriteError> {
    let mut packument: Value =
        serde_json::from_slice(bytes).map_err(|_| PackumentRewriteError::InvalidJson)?;
    let proxy_origin =
        Url::parse(proxy_origin).map_err(|_| PackumentRewriteError::ProxyOriginInvalid)?;

    let root = packument
        .as_object_mut()
        .ok_or(PackumentRewriteError::RootIsNotObject)?;
    let package_name = validated_packument_package_name(root)?;
    validate_dist_tags_count(root)?;

    let versions = root
        .get_mut("versions")
        .ok_or(PackumentRewriteError::VersionsMissing)?
        .as_object_mut()
        .ok_or(PackumentRewriteError::VersionsIsNotObject)?;

    if versions.len() > MAX_VERSIONS {
        return Err(PackumentRewriteError::TooManyVersions {
            limit: MAX_VERSIONS,
        });
    }

    let mut artifacts = HashMap::new();

    for (version, version_entry) in versions {
        validate_version_string(version)?;
        let version_object = version_entry
            .as_object_mut()
            .ok_or(PackumentRewriteError::VersionEntryIsNotObject)?;
        let dist_object = version_object
            .get_mut("dist")
            .ok_or(PackumentRewriteError::DistIsNotObject)?
            .as_object_mut()
            .ok_or(PackumentRewriteError::DistIsNotObject)?;
        let upstream_url = validated_tarball_url(dist_object)?;
        let integrity = validated_integrity_value(dist_object)?;
        let artifact_key = compute_artifact_key(package_name.as_str(), version, &upstream_url);
        let rewritten_url = build_rewritten_tarball_url(&proxy_origin, &artifact_key)?;

        dist_object.insert(String::from("tarball"), Value::String(rewritten_url));
        artifacts.insert(
            artifact_key,
            ArtifactMapping {
                package_name: package_name.as_str().to_string(),
                version: version.clone(),
                upstream_url,
                integrity,
            },
        );
    }

    let bytes =
        serde_json::to_vec(&packument).map_err(|_| PackumentRewriteError::SerializationFailed)?;

    Ok(RewrittenPackument { bytes, artifacts })
}

pub fn verify_sha512_integrity(integrity: Option<&str>, artifact_bytes: &[u8]) -> IntegrityStatus {
    let Some(integrity) = integrity else {
        return IntegrityStatus::Absent;
    };

    let Some(encoded_digest) = integrity.strip_prefix("sha512-") else {
        return IntegrityStatus::Unsupported;
    };

    if encoded_digest.is_empty() || encoded_digest.contains(char::is_whitespace) {
        return IntegrityStatus::Unsupported;
    }

    let Ok(expected_digest) = STANDARD.decode(encoded_digest) else {
        return IntegrityStatus::Unsupported;
    };

    if expected_digest.len() != 64 {
        return IntegrityStatus::Unsupported;
    }

    let computed_digest = Sha512::digest(artifact_bytes);

    if expected_digest.as_slice() == computed_digest.as_slice() {
        IntegrityStatus::Verified
    } else {
        IntegrityStatus::Mismatch
    }
}

pub fn compute_sha512_hex(bytes: &[u8]) -> String {
    lowercase_hex(&Sha512::digest(bytes))
}

fn validated_packument_package_name(
    root: &Map<String, Value>,
) -> Result<ValidatedPackageName, PackumentRewriteError> {
    let package_name = root
        .get("name")
        .and_then(Value::as_str)
        .ok_or(PackumentRewriteError::MissingPackageName)?;

    ValidatedPackageName::parse(package_name.to_string())
        .map_err(|_| PackumentRewriteError::InvalidPackageName)
}

fn validate_dist_tags_count(root: &Map<String, Value>) -> Result<(), PackumentRewriteError> {
    let Some(dist_tags) = root.get("dist-tags") else {
        return Ok(());
    };

    let Some(dist_tags) = dist_tags.as_object() else {
        return Err(PackumentRewriteError::DistTagsIsNotObject);
    };

    if dist_tags.len() > MAX_DIST_TAGS {
        Err(PackumentRewriteError::TooManyDistTags {
            limit: MAX_DIST_TAGS,
        })
    } else {
        Ok(())
    }
}

fn validate_version_string(version: &str) -> Result<(), PackumentRewriteError> {
    if version.is_empty() {
        return Err(PackumentRewriteError::VersionStringIsEmpty);
    }

    if version.len() > MAX_VERSION_BYTES {
        return Err(PackumentRewriteError::VersionStringTooLong {
            limit: MAX_VERSION_BYTES,
        });
    }

    if !version.bytes().all(is_supported_version_byte) {
        return Err(PackumentRewriteError::VersionStringHasUnsupportedCharacter);
    }

    Ok(())
}

fn is_supported_version_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+')
}

fn validated_tarball_url(
    dist_object: &Map<String, Value>,
) -> Result<String, PackumentRewriteError> {
    let tarball_url = dist_object
        .get("tarball")
        .ok_or(PackumentRewriteError::TarballUrlMissing)?
        .as_str()
        .ok_or(PackumentRewriteError::TarballUrlIsNotString)?;

    if tarball_url.len() > MAX_TARBALL_URL_BYTES {
        return Err(PackumentRewriteError::TarballUrlTooLong {
            limit: MAX_TARBALL_URL_BYTES,
        });
    }

    let tarball_url =
        Url::parse(tarball_url).map_err(|_| PackumentRewriteError::TarballUrlInvalid)?;

    if tarball_url.scheme() != "https" {
        return Err(PackumentRewriteError::TarballUrlSchemeNotHttps);
    }

    Ok(tarball_url.to_string())
}

fn validated_integrity_value(
    dist_object: &Map<String, Value>,
) -> Result<Option<String>, PackumentRewriteError> {
    let Some(integrity) = dist_object.get("integrity") else {
        return Ok(None);
    };

    let integrity = integrity
        .as_str()
        .ok_or(PackumentRewriteError::IntegrityIsNotString)?;

    if integrity.len() > MAX_INTEGRITY_BYTES {
        return Err(PackumentRewriteError::IntegrityTooLong {
            limit: MAX_INTEGRITY_BYTES,
        });
    }

    Ok(Some(integrity.to_string()))
}

fn compute_artifact_key(package_name: &str, version: &str, upstream_url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(package_name.as_bytes());
    hasher.update(b"\0");
    hasher.update(version.as_bytes());
    hasher.update(b"\0");
    hasher.update(upstream_url.as_bytes());

    lowercase_hex(&hasher.finalize())
}

fn build_rewritten_tarball_url(
    proxy_origin: &Url,
    artifact_key: &str,
) -> Result<String, PackumentRewriteError> {
    let mut url = proxy_origin.clone();
    url.set_path(&format!("/remnant/tarballs/{artifact_key}.tgz"));

    Ok(url.to_string())
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }

    output
}

#[cfg(test)]
mod tests;
