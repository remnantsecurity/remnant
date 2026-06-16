//! Decompressed archive stream limiting.
//!
//! This module owns the gzip/tar decompression read boundary. It keeps stream
//! consumption bounded before tar entries reach archive traversal logic.

use crate::archive::ArchiveError;
use std::error::Error;
use std::fmt;
use std::io::{self, Read};
use std::path::Path;

// Initial permissive MVP safety ceiling
pub(super) const MAX_DECOMPRESSED_ARCHIVE_BYTES: u64 = 300 * 1024 * 1024;

#[derive(Debug)]
struct DecompressedArchiveReadLimitExceeded {
    limit: u64,
}

impl fmt::Display for DecompressedArchiveReadLimitExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "decompressed archive stream exceeded {} byte limit",
            self.limit
        )
    }
}

impl Error for DecompressedArchiveReadLimitExceeded {}

pub(super) struct DecompressedArchiveReader<R> {
    inner: R,
    bytes_read: u64,
    limit: u64,
}

impl<R> DecompressedArchiveReader<R> {
    pub(super) fn new(inner: R, limit: u64) -> Self {
        Self {
            inner,
            bytes_read: 0,
            limit,
        }
    }
}

impl<R: Read> Read for DecompressedArchiveReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }

        let remaining = self.limit.saturating_sub(self.bytes_read);

        if remaining == 0 {
            return self.read_after_limit();
        }

        let max_remaining = match usize::try_from(remaining) {
            Ok(value) => value,
            Err(_) => usize::MAX,
        };
        let read_limit = buffer.len().min(max_remaining);
        let bytes_read = self.inner.read(&mut buffer[..read_limit])?;

        self.bytes_read += bytes_read as u64;

        Ok(bytes_read)
    }
}

impl<R: Read> DecompressedArchiveReader<R> {
    fn read_after_limit(&mut self) -> io::Result<usize> {
        let mut extra_byte = [0u8; 1];
        let bytes_read = self.inner.read(&mut extra_byte)?;

        if bytes_read == 0 {
            return Ok(0);
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            DecompressedArchiveReadLimitExceeded { limit: self.limit },
        ))
    }
}

pub(super) fn map_archive_read_error(path: &Path, error: io::Error) -> ArchiveError {
    if let Some(limit_error) = error
        .get_ref()
        .and_then(|source| source.downcast_ref::<DecompressedArchiveReadLimitExceeded>())
    {
        return ArchiveError::ArchiveDecompressedTooLarge {
            path: path.to_path_buf(),
            limit: limit_error.limit,
        };
    }

    ArchiveError::ArchiveReadFailed {
        path: path.to_path_buf(),
        kind: error.kind(),
    }
}
