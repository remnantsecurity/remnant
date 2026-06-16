//! Archive intake error types.
//!
//! Archive errors are kept separate from traversal logic so the archive module's
//! public failure modes remain easy to review independently.

use crate::output::escape_terminal_path;
use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// Errors that can occur while opening and reading an npm package archive.
///
/// These errors are separate from CLI command errors so archive intake can be
/// reused by metadata parsing, hashing, static analysis, and policy evaluation.
#[derive(Debug, PartialEq, Eq)]
pub enum ArchiveError {
    /// The artifact file could not be opened after validation.
    ArtifactOpenFailed {
        /// The artifact path that could not be opened.
        path: PathBuf,
        /// The underlying IO error kind.
        kind: io::ErrorKind,
    },

    /// The artifact could not be decoded as a gzip-compressed tar archive.
    ArchiveReadFailed {
        /// The artifact path that could not be read as an archive.
        path: PathBuf,
        /// The underlying IO error kind.
        kind: io::ErrorKind,
    },

    /// The decompressed archive stream exceeded Remnant's current read limit.
    ArchiveDecompressedTooLarge {
        /// The artifact path whose decompressed stream exceeded the limit.
        path: PathBuf,
        /// The maximum allowed decompressed archive stream bytes.
        limit: u64,
    },

    /// The archive contained no entries.
    ArchiveIsEmpty(PathBuf),

    /// The archive contained more entries than Remnant currently allows.
    ArchiveTooManyEntries {
        /// The artifact path whose archive entry count exceeded the limit.
        path: PathBuf,
        /// The first archive entry count that exceeded the limit.
        count: usize,
        /// The maximum allowed archive entry count.
        limit: usize,
    },

    /// The archive's declared total uncompressed size exceeded Remnant's current limit.
    ArchiveTooLarge {
        /// The artifact path whose archive exceeded the limit.
        path: PathBuf,
        /// The declared total size when the limit was exceeded.
        size: u64,
        /// The maximum allowed declared total archive size.
        limit: u64,
    },

    /// An archive entry path exceeded Remnant's current path length limit.
    ///
    /// The path itself is intentionally not included to avoid echoing oversized
    /// attacker-controlled data in reports.
    ArchiveEntryPathTooLong {
        /// The archive entry path length in UTF-8 bytes.
        length: usize,
        /// The maximum allowed archive entry path length in UTF-8 bytes.
        limit: usize,
    },

    /// An archive entry path was unsafe for filesystem extraction.
    ///
    /// This includes absolute paths, parent-directory traversal with `..`,
    /// Windows-style paths, and non-UTF-8 paths.
    ArchiveEntryPathUnsafe(PathBuf),

    /// An archive entry path appeared more than once.
    ///
    /// Duplicate paths are rejected because they make future extraction and
    /// deterministic reporting ambiguous.
    ArchiveEntryPathDuplicate(PathBuf),

    /// An archive entry exceeded Remnant's current single-entry size limit.
    ArchiveEntryTooLarge {
        /// The archive entry path whose declared size exceeded the limit.
        path: PathBuf,
        /// The entry size declared by the archive header.
        size: u64,
        /// The maximum allowed archive entry size.
        limit: u64,
    },

    /// An archive entry was a symlink.
    ///
    /// Symlinks are rejected until Remnant has an explicit, justified design for
    /// handling them.
    ArchiveEntryIsSymlink(PathBuf),

    /// An archive entry was a hardlink.
    ///
    /// Hardlinks are rejected until Remnant has an explicit, justified design
    /// for handling them.
    ArchiveEntryIsHardlink(PathBuf),

    /// An archive entry type is not currently supported by Remnant.
    ArchiveEntryTypeUnsupported {
        /// The archive entry path whose type was unsupported.
        path: PathBuf,
        /// The raw tar entry type byte.
        entry_type: u8,
    },

    /// The archive did not contain the required npm package metadata file.
    PackageJsonMissing(PathBuf),

    /// The required npm package metadata file exceeded Remnant's current size limit.
    PackageJsonTooLarge {
        /// The archive entry path for the oversized package metadata file.
        path: PathBuf,
        /// The package metadata entry size declared by the archive header.
        size: u64,
        /// The maximum allowed package metadata size.
        limit: u64,
    },
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveError::ArtifactOpenFailed { path, kind } => {
                write!(
                    f,
                    "artifact could not be opened: {} ({kind:?})",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveReadFailed { path, kind } => {
                write!(
                    f,
                    "archive could not be read: {} ({kind:?})",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveDecompressedTooLarge { path, limit } => {
                write!(
                    f,
                    "decompressed archive stream exceeds maximum size: {} ({limit} byte limit)",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveIsEmpty(path) => {
                write!(
                    f,
                    "archive contains no entries: {}",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveTooManyEntries { path, count, limit } => {
                write!(
                    f,
                    "archive contains too many entries: {} ({count} entries > {limit} entry limit)",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveTooLarge { path, size, limit } => {
                write!(
                    f,
                    "archive exceeds maximum declared total size: {} ({size} bytes > {limit} byte limit)",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveEntryPathTooLong { length, limit } => {
                write!(
                    f,
                    "archive entry path exceeds maximum length: {length} bytes > {limit} byte limit"
                )
            }
            ArchiveError::ArchiveEntryPathUnsafe(path) => {
                write!(
                    f,
                    "archive entry path is unsafe: {}",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveEntryPathDuplicate(path) => {
                write!(
                    f,
                    "archive entry path is duplicated: {}",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveEntryTooLarge { path, size, limit } => {
                write!(
                    f,
                    "archive entry exceeds maximum size: {} ({size} bytes > {limit} byte limit)",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveEntryIsSymlink(path) => {
                write!(
                    f,
                    "archive entry is a symlink: {}",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveEntryIsHardlink(path) => {
                write!(
                    f,
                    "archive entry is a hardlink: {}",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::ArchiveEntryTypeUnsupported { path, entry_type } => {
                write!(
                    f,
                    "archive entry type is unsupported: {} ({entry_type:#04x})",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::PackageJsonMissing(path) => {
                write!(
                    f,
                    "archive is missing package/package.json: {}",
                    escape_terminal_path(path)
                )
            }
            ArchiveError::PackageJsonTooLarge { path, size, limit } => {
                write!(
                    f,
                    "package/package.json exceeds maximum size: {} ({size} bytes > {limit} byte limit)",
                    escape_terminal_path(path)
                )
            }
        }
    }
}

impl Error for ArchiveError {}
