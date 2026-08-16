use crate::dataset::{DatasetManifest, NamePair};
use crate::similarity::CanonicalPackageName;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) struct TestFile {
    path: PathBuf,
}

impl TestFile {
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn basename(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("test fixture path should have a UTF-8 basename")
    }
}

impl Drop for TestFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(super) fn write_test_file(label: &str, contents: &[u8]) -> TestFile {
    loop {
        let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "remnant-package-name-similarity-evaluation-{}-{sequence}-{label}",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(contents)
                    .expect("test fixture contents should be written");
                return TestFile { path };
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("test fixture should be created: {error}"),
        }
    }
}

pub(super) fn name(value: &str) -> CanonicalPackageName {
    CanonicalPackageName::parse(value).expect("test package name should be valid")
}

pub(super) fn pair(candidate_name: &str, reference_name: &str) -> NamePair {
    NamePair {
        candidate_name: name(candidate_name),
        reference_name: name(reference_name),
    }
}

pub(super) fn manifest(dataset_file: &str) -> DatasetManifest {
    DatasetManifest {
        dataset_file: dataset_file.to_string(),
        source: "synthetic-placeholder-for-harness-testing".to_string(),
        retrieved: "2026-08-15".to_string(),
        selection_rule: "synthetic test records".to_string(),
        cutoff: "n/a".to_string(),
        content_sha256: "a".repeat(64),
        sort_rule: "ascending byte order".to_string(),
        license: "n/a".to_string(),
        includes_scoped_and_unscoped: false,
        is_synthetic_placeholder: true,
    }
}

pub(super) fn manifest_json(
    dataset_file: &str,
    content_sha256: &str,
    is_synthetic_placeholder: bool,
) -> String {
    serde_json::json!({
        "dataset_file": dataset_file,
        "source": "synthetic-placeholder-for-harness-testing",
        "retrieved": "2026-08-15",
        "selection_rule": "synthetic test records",
        "cutoff": "n/a",
        "content_sha256": content_sha256,
        "sort_rule": "ascending byte order",
        "license": "n/a",
        "includes_scoped_and_unscoped": false,
        "is_synthetic_placeholder": is_synthetic_placeholder,
    })
    .to_string()
}

pub(super) fn assembled_report(
    pairs: &[NamePair],
    npm_sample: &[CanonicalPackageName],
) -> serde_json::Value {
    let analysis = crate::report::analyze(pairs, npm_sample).expect("analysis should succeed");
    crate::report::assemble_report(
        &analysis,
        &manifest("pairs.jsonl"),
        &manifest("sample.jsonl"),
        pairs.len(),
        npm_sample.len(),
        123,
    )
}
