//! Archive resource-limit checks.
//!
//! These limits bound attacker-controlled archive metadata before broader
//! package inspection or policy evaluation consumes it.

use crate::archive::ArchiveError;
use std::path::Path;

// Initial permissive MVP safety ceilings
pub(super) const MAX_ARCHIVE_ENTRIES: usize = 10_000;
pub(super) const MAX_ARCHIVE_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
pub(super) const MAX_ARCHIVE_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
pub(super) const MAX_PACKAGE_JSON_BYTES: u64 = 1024 * 1024;

pub(super) fn validate_archive_entry_count(
    artifact_path: &Path,
    count: usize,
) -> Result<(), ArchiveError> {
    if count > MAX_ARCHIVE_ENTRIES {
        return Err(ArchiveError::ArchiveTooManyEntries {
            path: artifact_path.to_path_buf(),
            count,
            limit: MAX_ARCHIVE_ENTRIES,
        });
    }

    Ok(())
}

pub(super) fn validate_archive_entry_size(
    entry_path: &Path,
    size: u64,
) -> Result<(), ArchiveError> {
    if size > MAX_ARCHIVE_ENTRY_BYTES {
        return Err(ArchiveError::ArchiveEntryTooLarge {
            path: entry_path.to_path_buf(),
            size,
            limit: MAX_ARCHIVE_ENTRY_BYTES,
        });
    }

    Ok(())
}

pub(super) fn add_archive_entry_size(
    artifact_path: &Path,
    total_size: &mut u64,
    entry_size: u64,
) -> Result<(), ArchiveError> {
    let new_total =
        total_size
            .checked_add(entry_size)
            .ok_or_else(|| ArchiveError::ArchiveTooLarge {
                path: artifact_path.to_path_buf(),
                size: u64::MAX,
                limit: MAX_ARCHIVE_TOTAL_BYTES,
            })?;

    if new_total > MAX_ARCHIVE_TOTAL_BYTES {
        return Err(ArchiveError::ArchiveTooLarge {
            path: artifact_path.to_path_buf(),
            size: new_total,
            limit: MAX_ARCHIVE_TOTAL_BYTES,
        });
    }

    *total_size = new_total;

    Ok(())
}
