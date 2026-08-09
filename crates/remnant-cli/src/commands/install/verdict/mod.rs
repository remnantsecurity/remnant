//! Per-package fetch, integrity verification, inspection, and classification.

use crate::archive::inspect_archive;
use crate::commands::inspect::InspectError;
use crate::commands::install::lockfile::ResolvedPackage;
use crate::package_json::parse_package_json;
use crate::policy::{PolicyStatus, evaluate_default_policy};
use remnant_core::{
    FetchPackumentError, IntegrityStatus, UpstreamFetcher, verify_sha512_integrity,
};
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
struct FallbackDistMetadata {
    tarball_url: String,
    integrity: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum PackumentFallbackError {
    PackumentFetchFailed(FetchPackumentError),
    ResponseIsNotValidJson,
    ResponseIsNotAnObject,
    VersionsFieldMissing,
    VersionsFieldIsNotObject,
    PinnedVersionMissingFromVersions,
    VersionEntryIsNotObject,
    DistFieldMissing,
    DistFieldIsNotObject,
    TarballFieldMissing,
    TarballFieldIsNotString,
    IntegrityFieldIsNotString,
}

impl fmt::Display for PackumentFallbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackumentFallbackError::PackumentFetchFailed(error) => {
                write!(f, "packument fallback fetch failed: {error}")
            }
            PackumentFallbackError::ResponseIsNotValidJson => {
                write!(f, "packument fallback response is not valid JSON")
            }
            PackumentFallbackError::ResponseIsNotAnObject => {
                write!(
                    f,
                    "packument fallback response top-level value is not an object"
                )
            }
            PackumentFallbackError::VersionsFieldMissing => {
                write!(
                    f,
                    "packument fallback response is missing the versions field"
                )
            }
            PackumentFallbackError::VersionsFieldIsNotObject => {
                write!(
                    f,
                    "packument fallback response versions field is not an object"
                )
            }
            PackumentFallbackError::PinnedVersionMissingFromVersions => {
                write!(
                    f,
                    "packument fallback response does not include the pinned version"
                )
            }
            PackumentFallbackError::VersionEntryIsNotObject => {
                write!(
                    f,
                    "packument fallback response version entry is not an object"
                )
            }
            PackumentFallbackError::DistFieldMissing => {
                write!(
                    f,
                    "packument fallback response version entry is missing the dist field"
                )
            }
            PackumentFallbackError::DistFieldIsNotObject => {
                write!(f, "packument fallback response dist field is not an object")
            }
            PackumentFallbackError::TarballFieldMissing => {
                write!(
                    f,
                    "packument fallback response dist field is missing the tarball field"
                )
            }
            PackumentFallbackError::TarballFieldIsNotString => {
                write!(
                    f,
                    "packument fallback response dist tarball field is not a string"
                )
            }
            PackumentFallbackError::IntegrityFieldIsNotString => {
                write!(
                    f,
                    "packument fallback response dist integrity field is not a string"
                )
            }
        }
    }
}

async fn dist_metadata_for_package(
    fetcher: &UpstreamFetcher,
    package: &ResolvedPackage,
) -> Result<FallbackDistMetadata, PackumentFallbackError> {
    match &package.resolved_url {
        Some(resolved_url) => Ok(FallbackDistMetadata {
            tarball_url: resolved_url.clone(),
            integrity: package.integrity.clone(),
        }),
        None => {
            let response = fetcher
                .fetch_abbreviated_packument(&package.name)
                .await
                .map_err(PackumentFallbackError::PackumentFetchFailed)?;
            parse_dist_metadata_for_pinned_version(&response.bytes, &package.version)
        }
    }
}

fn parse_dist_metadata_for_pinned_version(
    bytes: &[u8],
    pinned_version: &str,
) -> Result<FallbackDistMetadata, PackumentFallbackError> {
    let packument: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| PackumentFallbackError::ResponseIsNotValidJson)?;
    let root = packument
        .as_object()
        .ok_or(PackumentFallbackError::ResponseIsNotAnObject)?;
    let versions = root
        .get("versions")
        .ok_or(PackumentFallbackError::VersionsFieldMissing)?
        .as_object()
        .ok_or(PackumentFallbackError::VersionsFieldIsNotObject)?;
    let version_entry = versions
        .get(pinned_version)
        .ok_or(PackumentFallbackError::PinnedVersionMissingFromVersions)?
        .as_object()
        .ok_or(PackumentFallbackError::VersionEntryIsNotObject)?;
    let dist = version_entry
        .get("dist")
        .ok_or(PackumentFallbackError::DistFieldMissing)?
        .as_object()
        .ok_or(PackumentFallbackError::DistFieldIsNotObject)?;
    let tarball_url = dist
        .get("tarball")
        .ok_or(PackumentFallbackError::TarballFieldMissing)?
        .as_str()
        .ok_or(PackumentFallbackError::TarballFieldIsNotString)?
        .to_owned();
    let integrity = match dist.get("integrity") {
        Some(value) => Some(
            value
                .as_str()
                .ok_or(PackumentFallbackError::IntegrityFieldIsNotString)?
                .to_owned(),
        ),
        None => None,
    };

    Ok(FallbackDistMetadata {
        tarball_url,
        integrity,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictCategory {
    Admitted,
    BlockedIntegrity,
    BlockedPolicy,
    BlockedParse,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageVerdict {
    pub name: String,
    pub version: String,
    pub category: VerdictCategory,
    pub finding_ids: Vec<String>,
    pub detail: String,
}

pub async fn inspect_resolved_packages(
    fetcher: &UpstreamFetcher,
    packages: &[ResolvedPackage],
) -> Vec<PackageVerdict> {
    let mut verdicts = Vec::with_capacity(packages.len());

    for (sequence, package) in packages.iter().enumerate() {
        verdicts
            .push(inspect_resolved_package(fetcher, package, &temp_tarball_path(sequence)).await);
    }

    verdicts
}

pub async fn inspect_resolved_package(
    fetcher: &UpstreamFetcher,
    package: &ResolvedPackage,
    temp_path: &Path,
) -> PackageVerdict {
    let dist_metadata = match dist_metadata_for_package(fetcher, package).await {
        Ok(dist_metadata) => dist_metadata,
        Err(error) => {
            return package_verdict(
                package,
                VerdictCategory::Error,
                Vec::new(),
                error.to_string(),
            );
        }
    };

    let bytes = match fetcher
        .fetch_tarball_bytes(&dist_metadata.tarball_url)
        .await
    {
        Ok(bytes) => bytes,
        Err(error) => {
            return package_verdict(
                package,
                VerdictCategory::Error,
                Vec::new(),
                error.to_string(),
            );
        }
    };

    let integrity_status = verify_sha512_integrity(dist_metadata.integrity.as_deref(), &bytes);
    if integrity_status != IntegrityStatus::Verified {
        return package_verdict(
            package,
            VerdictCategory::BlockedIntegrity,
            Vec::new(),
            format!("integrity status: {integrity_status:?}"),
        );
    }

    let _temp_tarball = match TempTarballFile::write(temp_path, &bytes) {
        Ok(temp_tarball) => temp_tarball,
        Err(error) => {
            return package_verdict(
                package,
                VerdictCategory::Error,
                Vec::new(),
                format!(
                    "artifact could not be written for inspection ({:?})",
                    error.kind()
                ),
            );
        }
    };
    let file = match File::open(temp_path) {
        Ok(file) => file,
        Err(error) => {
            return package_verdict(
                package,
                VerdictCategory::Error,
                Vec::new(),
                format!(
                    "artifact could not be reopened for inspection ({:?})",
                    error.kind()
                ),
            );
        }
    };
    let archive_inspection = match inspect_archive(file, temp_path) {
        Ok(archive_inspection) => archive_inspection,
        Err(error) => {
            return package_verdict(
                package,
                VerdictCategory::BlockedParse,
                Vec::new(),
                InspectError::Archive(error).to_string(),
            );
        }
    };
    let package_metadata = match parse_package_json(&archive_inspection.package_json) {
        Ok(package_metadata) => package_metadata,
        Err(error) => {
            return package_verdict(
                package,
                VerdictCategory::BlockedParse,
                Vec::new(),
                InspectError::PackageJson(error).to_string(),
            );
        }
    };
    let policy_result = evaluate_default_policy(&package_metadata, &archive_inspection.entries);

    match policy_result.status() {
        PolicyStatus::Passed => package_verdict(
            package,
            VerdictCategory::Admitted,
            Vec::new(),
            String::new(),
        ),
        PolicyStatus::Failed => package_verdict(
            package,
            VerdictCategory::BlockedPolicy,
            policy_result
                .findings()
                .iter()
                .map(|finding| finding.rule_id().to_owned())
                .collect(),
            format!("policy findings: {}", policy_result.findings().len()),
        ),
    }
}

fn package_verdict(
    package: &ResolvedPackage,
    category: VerdictCategory,
    finding_ids: Vec<String>,
    detail: String,
) -> PackageVerdict {
    PackageVerdict {
        name: package.name.clone(),
        version: package.version.clone(),
        category,
        finding_ids,
        detail,
    }
}

fn temp_tarball_path(sequence: usize) -> PathBuf {
    std::env::temp_dir().join(format!(
        "remnant-install-{}-{sequence}.tgz",
        std::process::id()
    ))
}

struct TempTarballFile {
    path: PathBuf,
}

impl TempTarballFile {
    fn write(path: &Path, bytes: &[u8]) -> std::io::Result<Self> {
        std::fs::write(path, bytes)?;
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl Drop for TempTarballFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests;
