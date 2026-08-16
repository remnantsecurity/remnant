//! Analysis and deterministic assembly of the harness JSON report.
//!
//! The npm sample is compared against itself. Every matching unordered pair
//! therefore produces two directed rows, one from each candidate perspective.
//! Analytical fields are deterministic for fixed input. `benchmark.runtime_ns`
//! is observational and intentionally isolated from those fields.

use crate::dataset::{DatasetManifest, NamePair};
use crate::similarity::{CanonicalPackageName, EditOperation, distance_one_operation};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

pub(crate) const MAX_COMPARISONS: usize = 250_000;
pub(crate) const MAX_MATCHES: usize = 50_000;

const HARNESS_VALIDATION_NOTICE: &str = "This report was generated from synthetic placeholder datasets for evaluation-harness self-test purposes only. It is not research evidence. Real, cited dataset support is separate follow-up work.";

pub(crate) struct Analysis {
    pair_fixture_coverage: Value,
    matches: Vec<CorpusMatch>,
    candidate_frequency_per_1000: f64,
    stratification: Value,
    multi_match_candidate_names: Value,
}

struct CorpusMatch {
    candidate_name: String,
    reference_name: String,
    operation: EditOperation,
}

pub(crate) fn analyze(
    pairs: &[NamePair],
    npm_sample: &[CanonicalPackageName],
) -> Result<Analysis, String> {
    analyze_with_limits(pairs, npm_sample, MAX_COMPARISONS, MAX_MATCHES)
}

pub(crate) fn analyze_with_limits(
    pairs: &[NamePair],
    npm_sample: &[CanonicalPackageName],
    max_comparisons: usize,
    max_matches: usize,
) -> Result<Analysis, String> {
    let pair_fixture_coverage = build_pair_fixture_coverage(pairs);
    let matches = build_matches(npm_sample, max_comparisons, max_matches)?;
    let candidate_frequency_per_1000 =
        build_candidate_frequency_per_1000(&matches, npm_sample.len());
    let stratification = build_stratification(&matches);
    let multi_match_candidate_names = build_multi_match_candidate_names(&matches);

    Ok(Analysis {
        pair_fixture_coverage,
        matches,
        candidate_frequency_per_1000,
        stratification,
        multi_match_candidate_names,
    })
}

pub(crate) fn assemble_report(
    analysis: &Analysis,
    pairs_manifest: &DatasetManifest,
    npm_sample_manifest: &DatasetManifest,
    pairs_record_count: usize,
    npm_sample_record_count: usize,
    runtime_ns: u128,
) -> Value {
    json!({
        "algorithm": "restricted-damerau-levenshtein-distance-1",
        "harness_validation_notice": HARNESS_VALIDATION_NOTICE,
        "datasets": {
            "pairs": manifest_to_report_value(pairs_manifest, pairs_record_count),
            "npm_sample": manifest_to_report_value(npm_sample_manifest, npm_sample_record_count),
        },
        "pair_fixture_coverage": analysis.pair_fixture_coverage,
        "candidate_frequency_per_1000": analysis.candidate_frequency_per_1000,
        "matches": matches_to_report_value(&analysis.matches),
        "stratification": analysis.stratification,
        "multi_match_candidate_names": analysis.multi_match_candidate_names,
        "benchmark": { "runtime_ns": runtime_ns },
    })
}

fn manifest_to_report_value(manifest: &DatasetManifest, record_count: usize) -> Value {
    let mut fields = Map::new();
    fields.insert("cutoff".to_string(), json!(manifest.cutoff));
    fields.insert("dataset_file".to_string(), json!(manifest.dataset_file));
    fields.insert(
        "declared_content_sha256".to_string(),
        json!(manifest.content_sha256),
    );
    fields.insert(
        "includes_scoped_and_unscoped".to_string(),
        json!(manifest.includes_scoped_and_unscoped),
    );
    fields.insert(
        "is_synthetic_placeholder".to_string(),
        json!(manifest.is_synthetic_placeholder),
    );
    fields.insert("license".to_string(), json!(manifest.license));
    fields.insert("record_count".to_string(), json!(record_count));
    fields.insert("retrieved".to_string(), json!(manifest.retrieved));
    fields.insert("selection_rule".to_string(), json!(manifest.selection_rule));
    fields.insert("sort_rule".to_string(), json!(manifest.sort_rule));
    fields.insert("source".to_string(), json!(manifest.source));
    Value::Object(fields)
}

fn build_pair_fixture_coverage(pairs: &[NamePair]) -> Value {
    let mut unmatched_candidate_names = Vec::new();
    let mut matched = 0usize;

    for pair in pairs {
        if distance_one_operation(&pair.candidate_name, &pair.reference_name).is_some() {
            matched += 1;
        } else {
            unmatched_candidate_names.push(pair.candidate_name.as_str().to_string());
        }
    }

    unmatched_candidate_names.sort();
    json!({
        "matched": matched,
        "total": pairs.len(),
        "unmatched_candidate_names": unmatched_candidate_names,
    })
}

fn build_matches(
    npm_sample: &[CanonicalPackageName],
    max_comparisons: usize,
    max_matches: usize,
) -> Result<Vec<CorpusMatch>, String> {
    let mut length_index: HashMap<usize, Vec<usize>> = HashMap::new();
    for (index, name) in npm_sample.iter().enumerate() {
        length_index
            .entry(name.as_str().len())
            .or_default()
            .push(index);
    }

    let mut matches = Vec::new();
    let mut comparisons_performed = 0usize;

    for candidate in npm_sample {
        let candidate_length = candidate.as_str().len();
        let mut reference_indices: Vec<usize> = Vec::new();

        if let Some(shorter_length) = candidate_length.checked_sub(1)
            && let Some(bucket) = length_index.get(&shorter_length)
        {
            reference_indices.extend_from_slice(bucket);
        }
        if let Some(bucket) = length_index.get(&candidate_length) {
            reference_indices.extend_from_slice(bucket);
        }
        if let Some(bucket) = length_index.get(&(candidate_length + 1)) {
            reference_indices.extend_from_slice(bucket);
        }

        for reference_index in reference_indices {
            if comparisons_performed == max_comparisons {
                return Err(format!(
                    "corpus self-comparison would exceed the {max_comparisons}-comparison evaluation harness safety limit"
                ));
            }
            comparisons_performed += 1;

            let reference = &npm_sample[reference_index];
            if let Some(operation) = distance_one_operation(candidate, reference) {
                if matches.len() == max_matches {
                    return Err(format!(
                        "corpus self-comparison would exceed the {max_matches}-match evaluation harness safety limit"
                    ));
                }
                matches.push(CorpusMatch {
                    candidate_name: candidate.as_str().to_string(),
                    reference_name: reference.as_str().to_string(),
                    operation,
                });
            }
        }
    }

    matches.sort_by(|left, right| {
        (&left.candidate_name, &left.reference_name)
            .cmp(&(&right.candidate_name, &right.reference_name))
    });
    Ok(matches)
}

fn matches_to_report_value(matches: &[CorpusMatch]) -> Value {
    Value::Array(
        matches
            .iter()
            .map(|entry| {
                json!({
                    "candidate_name": entry.candidate_name,
                    "reference_name": entry.reference_name,
                    "operation": entry.operation.as_str(),
                })
            })
            .collect(),
    )
}

fn build_candidate_frequency_per_1000(matches: &[CorpusMatch], sample_size: usize) -> f64 {
    if sample_size == 0 {
        return 0.0;
    }

    let mut distinct_candidates: Vec<&str> = matches
        .iter()
        .map(|entry| entry.candidate_name.as_str())
        .collect();
    distinct_candidates.sort_unstable();
    distinct_candidates.dedup();
    (distinct_candidates.len() as f64 / sample_size as f64) * 1000.0
}

fn build_stratification(matches: &[CorpusMatch]) -> Value {
    let mut by_candidate_length = Map::new();
    let mut by_reference_length = Map::new();
    let mut by_scope = Map::new();
    let mut by_operation = Map::new();

    increment(&mut by_scope, "scoped", 0);
    increment(&mut by_scope, "unscoped", 0);
    for operation in [
        EditOperation::Insertion,
        EditOperation::Deletion,
        EditOperation::Substitution,
        EditOperation::Transposition,
    ] {
        increment(&mut by_operation, operation.as_str(), 0);
    }

    for entry in matches {
        increment(
            &mut by_candidate_length,
            &entry.candidate_name.len().to_string(),
            1,
        );
        increment(
            &mut by_reference_length,
            &entry.reference_name.len().to_string(),
            1,
        );
        let scope = if entry.candidate_name.starts_with('@') {
            "scoped"
        } else {
            "unscoped"
        };
        increment(&mut by_scope, scope, 1);
        increment(&mut by_operation, entry.operation.as_str(), 1);
    }

    json!({
        "by_candidate_length": by_candidate_length,
        "by_reference_length": by_reference_length,
        "by_scope": by_scope,
        "by_operation": by_operation,
    })
}

fn increment(map: &mut Map<String, Value>, key: &str, delta: u64) {
    let current = map.get(key).and_then(Value::as_u64).unwrap_or(0);
    map.insert(key.to_string(), json!(current + delta));
}

fn build_multi_match_candidate_names(matches: &[CorpusMatch]) -> Value {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for entry in matches {
        *counts.entry(entry.candidate_name.as_str()).or_insert(0) += 1;
    }

    let mut names: Vec<&str> = counts
        .into_iter()
        .filter_map(|(name, count)| (count > 1).then_some(name))
        .collect();
    names.sort_unstable();
    json!(names)
}
