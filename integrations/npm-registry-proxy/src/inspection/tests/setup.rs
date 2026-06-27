use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use flate2::Compression;
use flate2::write::GzEncoder;
use tar::{Builder, Header};

pub(super) fn fixture_package_json(category: &str, name: &str) -> Vec<u8> {
    fs::read(
        fixture_root()
            .join(category)
            .join(name)
            .join("package")
            .join("package.json"),
    )
    .unwrap()
}

pub(super) fn build_fixture_tgz(category: &str, name: &str, package_json: &[u8]) -> PathBuf {
    let output_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("remnant-tests")
        .join("inspection");
    fs::create_dir_all(&output_directory).unwrap();

    let artifact_path = output_directory.join(format!("{category}-{name}.tgz"));
    let artifact_file = fs::File::create(&artifact_path).unwrap();
    let encoder = GzEncoder::new(artifact_file, Compression::default());
    let mut archive = Builder::new(encoder);
    let mut header = Header::new_gnu();

    header.set_size(package_json.len() as u64);
    header.set_mode(0o644);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();

    archive
        .append_data(
            &mut header,
            "package/package.json",
            Cursor::new(package_json),
        )
        .unwrap();
    let encoder = archive.into_inner().unwrap();
    encoder.finish().unwrap();

    artifact_path
}

pub(super) fn committed_malformed_fixture_path(name: &str) -> PathBuf {
    fixture_root()
        .join("malformed")
        .join(name)
        .join("artifact.tgz")
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("crates")
        .join("remnant-cli")
        .join("fixtures")
}
