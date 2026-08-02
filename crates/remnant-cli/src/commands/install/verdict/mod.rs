//! Per-package fetch, integrity verification, inspection, and classification.

use crate::archive::inspect_archive;
use crate::commands::inspect::InspectError;
use crate::commands::install::lockfile::ResolvedPackage;
use crate::package_json::parse_package_json;
use crate::policy::{PolicyStatus, evaluate_default_policy};
use remnant_core::{IntegrityStatus, UpstreamFetcher, verify_sha512_integrity};
use std::fs::File;
use std::path::{Path, PathBuf};

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

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "not yet wired into install::run() — Step 2c wires this into the live path"
    )
)]
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
    let bytes = match fetcher.fetch_tarball_bytes(&package.resolved_url).await {
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

    let integrity_status = verify_sha512_integrity(package.integrity.as_deref(), &bytes);
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
