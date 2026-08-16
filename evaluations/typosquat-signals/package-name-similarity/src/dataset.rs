//! Bounded, streaming loading of research datasets and provenance manifests.
//!
//! These limits protect this local harness when it is pointed at malformed or
//! adversarial files. They are research-tool safety limits, not production
//! policy thresholds.

use crate::similarity::{CanonicalPackageName, CanonicalPackageNameError};
use serde_json::Value;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

pub(crate) const MAX_MANIFEST_BYTES: usize = 8 * 1024;
pub(crate) const MAX_JSONL_LINE_BYTES: usize = 1024;
pub(crate) const MAX_DATASET_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_DATASET_RECORDS: usize = 1_000;

#[derive(Debug)]
pub(crate) struct NamePair {
    pub(crate) candidate_name: CanonicalPackageName,
    pub(crate) reference_name: CanonicalPackageName,
}

#[derive(Debug)]
pub(crate) struct DatasetManifest {
    pub(crate) dataset_file: String,
    pub(crate) source: String,
    pub(crate) retrieved: String,
    pub(crate) selection_rule: String,
    pub(crate) cutoff: String,
    pub(crate) content_sha256: String,
    pub(crate) sort_rule: String,
    pub(crate) license: String,
    pub(crate) includes_scoped_and_unscoped: bool,
    pub(crate) is_synthetic_placeholder: bool,
}

pub(crate) fn load_pairs(path: &Path) -> Result<Vec<NamePair>, String> {
    let mut pairs = Vec::new();

    read_bounded_jsonl(path, |line_number, line| {
        let pair = parse_pair_line(line)
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        pairs.push(pair);
        validate_record_count(pairs.len()).map_err(|error| format!("{}: {error}", path.display()))
    })?;

    validate_pair_order(&pairs).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(pairs)
}

pub(crate) fn load_npm_sample(path: &Path) -> Result<Vec<CanonicalPackageName>, String> {
    let mut names = Vec::new();

    read_bounded_jsonl(path, |line_number, line| {
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        let name = required_str_field(&value, "name")
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        names.push(
            parse_name(name, "name")
                .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?,
        );
        validate_record_count(names.len()).map_err(|error| format!("{}: {error}", path.display()))
    })?;

    validate_name_order(&names).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(names)
}

pub(crate) fn load_manifest(
    manifest_path: &Path,
    dataset_path: &Path,
) -> Result<DatasetManifest, String> {
    let mut file = File::open(manifest_path)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    let mut bytes = Vec::with_capacity(MAX_MANIFEST_BYTES + 1);

    file.by_ref()
        .take((MAX_MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;

    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(format!(
            "{}: manifest exceeds the {MAX_MANIFEST_BYTES}-byte evaluation harness safety limit",
            manifest_path.display()
        ));
    }

    let contents = std::str::from_utf8(&bytes)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    let value: Value = serde_json::from_str(contents)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;

    let dataset_file = manifest_str(&value, "dataset_file", manifest_path)?.to_string();
    let content_sha256 = manifest_str(&value, "content_sha256", manifest_path)?.to_string();
    validate_content_sha256_format(&content_sha256)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;

    let is_synthetic_placeholder =
        manifest_bool(&value, "is_synthetic_placeholder", manifest_path)?;
    if !is_synthetic_placeholder {
        return Err(format!(
            "{}: is_synthetic_placeholder is false; real-corpus support is separate follow-up work",
            manifest_path.display()
        ));
    }

    let actual_basename = dataset_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "{}: dataset path has no valid UTF-8 file name",
                dataset_path.display()
            )
        })?;
    if dataset_file != actual_basename {
        return Err(format!(
            "{}: manifest dataset_file {dataset_file:?} does not match actual file name {actual_basename:?}",
            manifest_path.display()
        ));
    }

    Ok(DatasetManifest {
        dataset_file,
        source: manifest_str(&value, "source", manifest_path)?.to_string(),
        retrieved: manifest_str(&value, "retrieved", manifest_path)?.to_string(),
        selection_rule: manifest_str(&value, "selection_rule", manifest_path)?.to_string(),
        cutoff: manifest_str(&value, "cutoff", manifest_path)?.to_string(),
        content_sha256,
        sort_rule: manifest_str(&value, "sort_rule", manifest_path)?.to_string(),
        license: manifest_str(&value, "license", manifest_path)?.to_string(),
        includes_scoped_and_unscoped: manifest_bool(
            &value,
            "includes_scoped_and_unscoped",
            manifest_path,
        )?,
        is_synthetic_placeholder,
    })
}

fn read_bounded_jsonl<F>(path: &Path, mut visit_line: F) -> Result<(), String>
where
    F: FnMut(usize, &str) -> Result<(), String>,
{
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let limited_file = file.take((MAX_DATASET_BYTES + 1) as u64);
    let mut reader = BufReader::new(limited_file);
    let mut total_bytes = 0usize;
    let mut line_number = 0usize;
    let mut line_bytes = Vec::with_capacity(MAX_JSONL_LINE_BYTES + 2);

    loop {
        line_bytes.clear();
        let bytes_read = Read::by_ref(&mut reader)
            .take((MAX_JSONL_LINE_BYTES + 2) as u64)
            .read_until(b'\n', &mut line_bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;

        if bytes_read == 0 {
            break;
        }

        line_number += 1;
        total_bytes = total_bytes.saturating_add(bytes_read);
        if total_bytes > MAX_DATASET_BYTES {
            return Err(format!(
                "{}: dataset exceeds the {MAX_DATASET_BYTES}-byte evaluation harness safety limit",
                path.display()
            ));
        }

        if line_bytes.last() == Some(&b'\n') {
            line_bytes.pop();
            if line_bytes.last() == Some(&b'\r') {
                line_bytes.pop();
            }
        }

        if line_bytes.len() > MAX_JSONL_LINE_BYTES {
            return Err(format!(
                "{}:{line_number}: line is {} bytes, exceeds the {MAX_JSONL_LINE_BYTES}-byte evaluation harness safety limit",
                path.display(),
                line_bytes.len()
            ));
        }

        let line = std::str::from_utf8(&line_bytes)
            .map_err(|error| format!("{}:{line_number}: {error}", path.display()))?;
        if !line.trim().is_empty() {
            visit_line(line_number, line)?;
        }
    }

    Ok(())
}

fn parse_pair_line(line: &str) -> Result<NamePair, String> {
    let value: Value = serde_json::from_str(line).map_err(|error| error.to_string())?;
    let candidate_name = required_str_field(&value, "candidate_name")?;
    let reference_name = required_str_field(&value, "reference_name")?;

    Ok(NamePair {
        candidate_name: parse_name(candidate_name, "candidate_name")?,
        reference_name: parse_name(reference_name, "reference_name")?,
    })
}

fn parse_name(name: &str, field: &str) -> Result<CanonicalPackageName, String> {
    CanonicalPackageName::parse(name)
        .map_err(|error: CanonicalPackageNameError| format!("{field} {name:?}: {error}"))
}

fn validate_record_count(record_count: usize) -> Result<(), String> {
    if record_count > MAX_DATASET_RECORDS {
        Err(format!(
            "dataset has {record_count} records, exceeds the {MAX_DATASET_RECORDS}-record evaluation harness safety limit"
        ))
    } else {
        Ok(())
    }
}

fn validate_pair_order(pairs: &[NamePair]) -> Result<(), String> {
    for pair_window in pairs.windows(2) {
        let previous = (
            pair_window[0].candidate_name.as_str(),
            pair_window[0].reference_name.as_str(),
        );
        let current = (
            pair_window[1].candidate_name.as_str(),
            pair_window[1].reference_name.as_str(),
        );

        match previous.cmp(&current) {
            Ordering::Less => {}
            Ordering::Equal => {
                return Err(format!(
                    "duplicate (candidate_name, reference_name) pair {current:?}"
                ));
            }
            Ordering::Greater => {
                return Err(format!(
                    "pairs are not in ascending byte order at {current:?}"
                ));
            }
        }
    }
    Ok(())
}

fn validate_name_order(names: &[CanonicalPackageName]) -> Result<(), String> {
    for name_window in names.windows(2) {
        let previous = name_window[0].as_str();
        let current = name_window[1].as_str();

        match previous.cmp(current) {
            Ordering::Less => {}
            Ordering::Equal => return Err(format!("duplicate name entry {current:?}")),
            Ordering::Greater => {
                return Err(format!(
                    "name entries are not in ascending byte order at {current:?}"
                ));
            }
        }
    }
    Ok(())
}

fn validate_content_sha256_format(content_sha256: &str) -> Result<(), String> {
    let is_valid = content_sha256.len() == 64
        && content_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'));

    if is_valid {
        Ok(())
    } else {
        Err(format!(
            "content_sha256 {content_sha256:?} is not exactly 64 lowercase hexadecimal characters"
        ))
    }
}

fn manifest_str<'a>(
    value: &'a Value,
    field: &str,
    manifest_path: &Path,
) -> Result<&'a str, String> {
    required_str_field(value, field)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))
}

fn manifest_bool(value: &Value, field: &str, manifest_path: &Path) -> Result<bool, String> {
    required_bool_field(value, field)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))
}

fn required_str_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing or non-string field {field:?}"))
}

fn required_bool_field(value: &Value, field: &str) -> Result<bool, String> {
    value
        .get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("missing or non-bool field {field:?}"))
}
