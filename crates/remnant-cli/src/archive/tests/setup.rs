//! Test helpers for constructing archive fixtures.
//!
//! These helpers intentionally create both ordinary and malformed `.tgz` inputs
//! so archive tests can exercise parser safety boundaries without extracting
//! untrusted archives.

use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::{self, File};
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use tar::{Builder, EntryType, Header};

pub(super) fn test_path(name: &str) -> PathBuf {
    test_root().join(name)
}

pub(super) fn malformed_fixture_artifact_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("malformed")
        .join(name)
        .join("artifact.tgz")
}

pub(super) fn remove_path_if_exists(path: &Path) {
    if path.is_dir() {
        fs::remove_dir_all(path).expect("test directory should be removed");
    } else if path.exists() {
        fs::remove_file(path).expect("test file should be removed");
    }
}

pub(super) fn create_empty_tgz(path: &Path) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let builder = Builder::new(encoder);

    finish_archive(builder);
}

pub(super) fn create_gzip_with_bytes(path: &Path, contents: &[u8]) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test gzip file should be created");
    let mut encoder = GzEncoder::new(file, Compression::default());

    encoder
        .write_all(contents)
        .expect("test gzip contents should be written");
    encoder
        .finish()
        .expect("gzip encoder should finish successfully");
}

pub(super) fn create_tgz_with_file(path: &Path, archive_path: &str, contents: &[u8]) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    append_file_entry(&mut builder, archive_path, contents);

    finish_archive(builder);
}

pub(super) fn create_tgz_with_duplicate_file_path(path: &Path) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    append_file_entry(&mut builder, "package/package.json", br#"{"name":"first"}"#);
    append_file_entry(
        &mut builder,
        "package/package.json",
        br#"{"name":"second"}"#,
    );

    finish_archive(builder);
}

pub(super) fn create_tgz_with_file_count(path: &Path, entry_count: usize) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    for index in 0..entry_count {
        let archive_path = format!("package/file-{index}.js");
        append_file_entry(&mut builder, &archive_path, b"");
    }

    finish_archive(builder);
}

pub(super) fn create_tgz_with_directory(path: &Path, archive_path: &str) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Directory);
    header.set_size(0);
    header.set_mode(0o755);
    header.set_cksum();

    builder
        .append_data(&mut header, archive_path, io::empty())
        .expect("test directory entry should be appended");

    finish_archive(builder);
}

// `tar::Builder` intentionally refuses unsafe paths such as `/absolute` or
// `package/../escape`. These tests need malicious archive bytes so Remnant can
// prove it rejects them, so this helper writes a minimal tar entry by hand.
pub(super) fn create_tgz_with_raw_file_path(path: &Path, archive_path: &str, contents: &[u8]) {
    create_tgz_with_raw_file_entries(path, &[(archive_path, contents)]);
}

pub(super) fn create_tgz_with_raw_declared_file_size(path: &Path, archive_path: &str, size: u64) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let mut encoder = GzEncoder::new(file, Compression::default());
    let header = raw_tar_header(archive_path, size);

    encoder
        .write_all(&header)
        .expect("test tar header should be written");
    encoder
        .write_all(&[0; 1024])
        .expect("test tar end marker should be written");
    encoder
        .finish()
        .expect("gzip encoder should finish successfully");
}

pub(super) fn create_tgz_with_raw_entry_type(path: &Path, archive_path: &str, entry_type: u8) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let mut encoder = GzEncoder::new(file, Compression::default());
    let header = raw_tar_header_with_entry_type(archive_path, 0, entry_type);

    encoder
        .write_all(&header)
        .expect("test tar header should be written");
    encoder
        .write_all(&[0; 1024])
        .expect("test tar end marker should be written");
    encoder
        .finish()
        .expect("gzip encoder should finish successfully");
}

pub(super) fn create_tgz_with_package_json_and_raw_declared_file_size(
    path: &Path,
    archive_path: &str,
    size: u64,
) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let mut encoder = GzEncoder::new(file, Compression::default());
    let package_json = br#"{"name":"safe"}"#;
    let package_json_header = raw_tar_header("package/package.json", package_json.len() as u64);
    let oversized_header = raw_tar_header(archive_path, size);

    encoder
        .write_all(&package_json_header)
        .expect("test package.json header should be written");
    encoder
        .write_all(package_json)
        .expect("test package.json contents should be written");

    let padding = (512 - (package_json.len() % 512)) % 512;
    encoder
        .write_all(&vec![0; padding])
        .expect("test tar padding should be written");
    encoder
        .write_all(&oversized_header)
        .expect("test oversized tar header should be written");
    encoder
        .write_all(&[0; 1024])
        .expect("test tar end marker should be written");
    encoder
        .finish()
        .expect("gzip encoder should finish successfully");
}

// `tar::Builder` intentionally refuses unsafe paths such as `/absolute` or
// `package/../escape`. These tests need malicious archive bytes so Remnant can
// prove it rejects them, so this helper writes minimal tar entries by hand.
pub(super) fn create_tgz_with_raw_file_entries(path: &Path, entries: &[(&str, &[u8])]) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let mut encoder = GzEncoder::new(file, Compression::default());

    for (archive_path, contents) in entries {
        let header = raw_tar_header(archive_path, contents.len() as u64);

        encoder
            .write_all(&header)
            .expect("test tar header should be written");
        encoder
            .write_all(contents)
            .expect("test file contents should be written");

        let padding = (512 - (contents.len() % 512)) % 512;
        encoder
            .write_all(&vec![0; padding])
            .expect("test tar padding should be written");
    }

    encoder
        .write_all(&[0; 1024])
        .expect("test tar end marker should be written");
    encoder
        .finish()
        .expect("gzip encoder should finish successfully");
}

pub(super) fn create_tgz_with_symlink(path: &Path, archive_path: &str, target_path: &str) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    header.set_cksum();

    builder
        .append_link(&mut header, archive_path, target_path)
        .expect("test symlink entry should be appended");

    finish_archive(builder);
}

pub(super) fn create_tgz_with_hardlink(path: &Path, archive_path: &str, target_path: &str) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);

    let mut header = Header::new_gnu();
    header.set_entry_type(EntryType::Link);
    header.set_size(0);
    header.set_mode(0o644);
    header.set_cksum();

    builder
        .append_link(&mut header, archive_path, target_path)
        .expect("test hardlink entry should be appended");

    finish_archive(builder);
}

fn test_root() -> PathBuf {
    let root = std::env::current_dir()
        .expect("test should run from a working directory")
        .join("target")
        .join("remnant-tests")
        .join("archive");

    fs::create_dir_all(&root).expect("test root should be created");

    root
}

fn finish_archive(builder: Builder<GzEncoder<File>>) {
    let encoder = builder
        .into_inner()
        .expect("tar builder should finish successfully");

    encoder
        .finish()
        .expect("gzip encoder should finish successfully");
}

fn append_file_entry(builder: &mut Builder<GzEncoder<File>>, archive_path: &str, contents: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_size(contents.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();

    builder
        .append_data(&mut header, archive_path, io::Cursor::new(contents))
        .expect("test file entry should be appended");
}

// Tar headers are fixed-width 512-byte records. This helper only implements
// the fields needed by the unsafe-path tests; it is not intended to become a
// general-purpose tar writer.
fn raw_tar_header(archive_path: &str, size: u64) -> [u8; 512] {
    raw_tar_header_with_entry_type(archive_path, size, b'0')
}

fn raw_tar_header_with_entry_type(archive_path: &str, size: u64, entry_type: u8) -> [u8; 512] {
    assert!(
        archive_path.len() <= 100,
        "test helper only supports short archive paths"
    );

    let mut header = [0; 512];

    header[0..archive_path.len()].copy_from_slice(archive_path.as_bytes());
    write_octal_field(&mut header, 100..108, 0o644);
    write_octal_field(&mut header, 108..116, 0);
    write_octal_field(&mut header, 116..124, 0);
    write_octal_field(&mut header, 124..136, size);
    write_octal_field(&mut header, 136..148, 0);
    header[148..156].fill(b' ');
    header[156] = entry_type;
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");

    let checksum = header.iter().map(|byte| u32::from(*byte)).sum::<u32>();
    write_checksum_field(&mut header, checksum);

    header
}

fn write_octal_field(header: &mut [u8; 512], range: Range<usize>, value: u64) {
    let field_length = range.end - range.start;
    let value = format!("{:0width$o}\0", value, width = field_length - 1);

    header[range].copy_from_slice(value.as_bytes());
}

fn write_checksum_field(header: &mut [u8; 512], checksum: u32) {
    // Tar checksum fields are stored as six octal digits, a NUL byte, and a
    // trailing space. The checksum is computed while the checksum field itself
    // contains spaces.
    let value = format!("{:06o}\0 ", checksum);

    header[148..156].copy_from_slice(value.as_bytes());
}
