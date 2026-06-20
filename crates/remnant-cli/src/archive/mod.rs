//! Read-only npm tarball archive intake.
//!
//! This module owns archive traversal and entry safety checks. It treats npm
//! package artifacts as untrusted input and validates entry paths before Remnant
//! reports, hashes, analyzes, or eventually extracts anything from an archive.

use decompressed::{
    DecompressedArchiveReader, MAX_DECOMPRESSED_ARCHIVE_BYTES, map_archive_read_error,
};
use flate2::read::GzDecoder;
use limits::{
    MAX_PACKAGE_JSON_BYTES, add_archive_entry_size, validate_archive_entry_count,
    validate_archive_entry_size,
};
use path::normalize_archive_entry_path;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::{Archive, EntryType};

mod decompressed;
mod error;
mod limits;
mod path;

pub use error::ArchiveError;

#[cfg(test)]
use limits::{MAX_ARCHIVE_ENTRIES, MAX_ARCHIVE_ENTRY_BYTES, MAX_ARCHIVE_TOTAL_BYTES};
#[cfg(test)]
use path::MAX_ARCHIVE_ENTRY_PATH_BYTES;

const PACKAGE_JSON_PATH: &str = "package/package.json";

/// A read-only description of an entry inside an npm package tarball.
///
/// Remnant does not extract archive contents at this stage. Enumeration gives us
/// deterministic structure to inspect before any filesystem writes are allowed.
#[derive(Debug, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub path: PathBuf,
    pub size: u64,
}

/// A validated, read-only archive inspection result.
///
/// This contains the archive structure and the required npm package metadata
/// bytes collected during a single archive traversal.
#[derive(Debug, PartialEq, Eq)]
pub struct ArchiveInspection {
    pub entries: Vec<ArchiveEntry>,
    pub package_json: Vec<u8>,
}

/// Opens and enumerates a gzip-compressed tar archive without extracting it.
///
/// This is an archive trust boundary. Every entry path is treated as untrusted
/// and validated before Remnant proceeds. This remains crate-internal until
/// Remnant has a concrete external caller for entry-only archive enumeration.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "kept crate-internal until entry-only archive enumeration has a concrete caller"
    )
)]
pub(crate) fn read_archive_entries(path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    Ok(traverse_archive(path, PackageJsonReadMode::Ignore)?.entries)
}

/// Opens, validates, and inspects a gzip-compressed tar archive once.
///
/// This does not extract archive contents to disk. The full archive stream is
/// traversed so unsafe entries, duplicate paths, unsupported entry types, and
/// resource-limit violations are rejected even when `package/package.json` is
/// found early.
pub fn inspect_archive(path: &Path) -> Result<ArchiveInspection, ArchiveError> {
    let traversal = traverse_archive(path, PackageJsonReadMode::Capture)?;
    let package_json = traversal
        .package_json
        .ok_or_else(|| ArchiveError::PackageJsonMissing(path.to_path_buf()))?;

    Ok(ArchiveInspection {
        entries: traversal.entries,
        package_json,
    })
}

/// Opens and reads `package/package.json` from a gzip-compressed tar archive.
///
/// This does not extract archive contents to disk. The full archive stream is
/// still traversed so unsafe entries, duplicate paths, unsupported entry types,
/// and resource-limit violations are rejected even when `package/package.json`
/// is found early.
#[cfg(test)]
pub fn read_package_json(path: &Path) -> Result<Vec<u8>, ArchiveError> {
    Ok(inspect_archive(path)?.package_json)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackageJsonReadMode {
    Capture,
    Ignore,
}

#[derive(Debug, PartialEq, Eq)]
struct ArchiveTraversal {
    entries: Vec<ArchiveEntry>,
    package_json: Option<Vec<u8>>,
}

fn traverse_archive(
    path: &Path,
    package_json_read_mode: PackageJsonReadMode,
) -> Result<ArchiveTraversal, ArchiveError> {
    let file = File::open(path).map_err(|error| ArchiveError::ArtifactOpenFailed {
        path: path.to_path_buf(),
        kind: error.kind(),
    })?;

    let decoder = GzDecoder::new(file);
    let limited_decoder = DecompressedArchiveReader::new(decoder, MAX_DECOMPRESSED_ARCHIVE_BYTES);
    let mut archive = Archive::new(limited_decoder);

    let entries = archive
        .entries()
        .map_err(|error| map_archive_read_error(path, error))?;

    let mut archive_entries = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut entry_count = 0usize;
    let mut total_size = 0u64;
    let mut package_json = None;

    for entry in entries {
        let mut entry = entry.map_err(|error| map_archive_read_error(path, error))?;

        entry_count += 1;
        validate_archive_entry_count(path, entry_count)?;

        let raw_entry_path = entry
            .path()
            .map_err(|error| map_archive_read_error(path, error))?
            .into_owned();

        let entry_type = entry.header().entry_type();
        let entry_path = validate_archive_entry(&raw_entry_path, entry_type, &mut seen_paths)?;

        let size = entry
            .header()
            .size()
            .map_err(|error| map_archive_read_error(path, error))?;

        match package_json_read_mode {
            PackageJsonReadMode::Ignore => {
                validate_archive_entry_size(&entry_path, size)?;
                add_archive_entry_size(path, &mut total_size, size)?;
            }
            PackageJsonReadMode::Capture => {
                add_archive_entry_size(path, &mut total_size, size)?;

                if entry_path.as_path() == Path::new(PACKAGE_JSON_PATH) {
                    let contents = read_bounded_entry_contents(
                        &mut entry,
                        path,
                        &entry_path,
                        size,
                        MAX_PACKAGE_JSON_BYTES,
                    )?;

                    package_json = Some(contents);
                } else {
                    validate_archive_entry_size(&entry_path, size)?;
                }
            }
        }

        archive_entries.push(ArchiveEntry {
            path: entry_path,
            size,
        });
    }

    if entry_count == 0 {
        return Err(ArchiveError::ArchiveIsEmpty(path.to_path_buf()));
    }

    archive_entries.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(ArchiveTraversal {
        entries: archive_entries,
        package_json,
    })
}

fn read_bounded_entry_contents(
    reader: &mut impl Read,
    artifact_path: &Path,
    entry_path: &Path,
    declared_size: u64,
    limit: u64,
) -> Result<Vec<u8>, ArchiveError> {
    if declared_size > limit {
        return Err(ArchiveError::PackageJsonTooLarge {
            path: entry_path.to_path_buf(),
            size: declared_size,
            limit,
        });
    }

    let mut contents = Vec::with_capacity(declared_size as usize);

    reader
        .read_to_end(&mut contents)
        .map_err(|error| map_archive_read_error(artifact_path, error))?;

    Ok(contents)
}

fn validate_archive_entry(
    raw_entry_path: &Path,
    entry_type: EntryType,
    seen_paths: &mut HashSet<PathBuf>,
) -> Result<PathBuf, ArchiveError> {
    let entry_path = normalize_archive_entry_path(raw_entry_path)?;

    if !seen_paths.insert(entry_path.clone()) {
        return Err(ArchiveError::ArchiveEntryPathDuplicate(entry_path));
    }

    if entry_type.is_symlink() {
        return Err(ArchiveError::ArchiveEntryIsSymlink(entry_path));
    }

    if entry_type.is_hard_link() {
        return Err(ArchiveError::ArchiveEntryIsHardlink(entry_path));
    }

    if !entry_type.is_file() {
        return Err(ArchiveError::ArchiveEntryTypeUnsupported {
            path: entry_path,
            entry_type: entry_type.as_byte(),
        });
    }

    Ok(entry_path)
}

#[cfg(test)]
mod tests;
