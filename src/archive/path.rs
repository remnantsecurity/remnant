//! Archive entry path validation.
//!
//! Archive paths are attacker-controlled input. This module keeps path-specific
//! safety checks separate from archive traversal so normalization decisions can
//! be reviewed independently.

use crate::archive::ArchiveError;
use std::path::{Component, Path, PathBuf};

// Initial permissive MVP safety ceiling
pub(super) const MAX_ARCHIVE_ENTRY_PATH_BYTES: usize = 1024;

/// Validates and normalizes an archive entry path.
///
/// This rejects:
/// - empty paths
/// - non-UTF-8 paths
/// - paths longer than the current raw UTF-8 byte-length limit
/// - absolute paths for the current platform
/// - Windows-style drive prefixes such as `C:/...`
/// - Windows-style backslash separators
/// - parent-directory traversal with `..` components
/// - paths that normalize to no file path, such as `.` or `./`
///
/// Normalization is intentionally small and deterministic: `.` components are
/// removed, and all other accepted components are preserved as ordinary path
/// components. Remnant uses the normalized path for duplicate detection and
/// reporting so equivalent archive spellings cannot bypass path checks.
pub(super) fn normalize_archive_entry_path(path: &Path) -> Result<PathBuf, ArchiveError> {
    let Some(path_text) = path.to_str() else {
        return Err(ArchiveError::ArchiveEntryPathUnsafe(path.to_path_buf()));
    };

    if path_text.len() > MAX_ARCHIVE_ENTRY_PATH_BYTES {
        return Err(ArchiveError::ArchiveEntryPathTooLong {
            length: path_text.len(),
            limit: MAX_ARCHIVE_ENTRY_PATH_BYTES,
        });
    }

    if path_text.is_empty()
        || path.is_absolute()
        || has_windows_drive_prefix(path_text)
        || path_text.contains('\\')
    {
        return Err(ArchiveError::ArchiveEntryPathUnsafe(path.to_path_buf()));
    }

    let mut normalized_path = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Normal(component) => normalized_path.push(component),
            Component::CurDir => {}
            _ => return Err(ArchiveError::ArchiveEntryPathUnsafe(path.to_path_buf())),
        }
    }

    if normalized_path.as_os_str().is_empty() {
        return Err(ArchiveError::ArchiveEntryPathUnsafe(path.to_path_buf()));
    }

    Ok(normalized_path)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();

    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}
