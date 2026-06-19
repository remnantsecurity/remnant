//! Test setup helpers for the `inspect` command.

use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tar::{Builder, Header};

pub(super) fn test_path(name: &str) -> PathBuf {
    test_root().join(name)
}

pub(super) fn remove_path_if_exists(path: &Path) {
    if path.is_dir() {
        fs::remove_dir_all(path).expect("test directory should be removed");
    } else if path.exists() {
        fs::remove_file(path).expect("test file should be removed");
    }
}

pub(super) fn create_tgz_with_package_json(path: &Path, package_json: &[u8]) {
    remove_path_if_exists(path);

    let file = File::create(path).expect("test archive should be created");
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = Builder::new(encoder);
    let mut header = Header::new_gnu();

    header.set_size(package_json.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();

    builder
        .append_data(
            &mut header,
            "package/package.json",
            io::Cursor::new(package_json),
        )
        .expect("test package.json entry should be appended");

    let encoder = builder
        .into_inner()
        .expect("tar builder should finish successfully");

    encoder
        .finish()
        .expect("gzip encoder should finish successfully");
}

fn test_root() -> PathBuf {
    let root = std::env::current_dir()
        .expect("test should run from a working directory")
        .join("target")
        .join("remnant-tests")
        .join("inspect");

    fs::create_dir_all(&root).expect("test root should be created");

    root
}
