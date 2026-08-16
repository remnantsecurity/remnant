mod setup;

use crate::dataset::{
    MAX_DATASET_BYTES, MAX_DATASET_RECORDS, MAX_JSONL_LINE_BYTES, MAX_MANIFEST_BYTES,
    load_manifest, load_npm_sample, load_pairs,
};
use crate::report::{MAX_COMPARISONS, MAX_MATCHES, analyze_with_limits};
use crate::similarity::{
    CanonicalPackageName, CanonicalPackageNameError, EditOperation, MAX_PACKAGE_NAME_BYTES,
    distance_one_operation,
};
use serde_json::json;
use setup::*;

#[test]
fn identical_names_are_distance_zero_and_excluded() {
    assert_eq!(
        distance_one_operation(&name("lodash"), &name("lodash")),
        None
    );
}

#[test]
fn matches_single_ascii_byte_insertion() {
    assert_eq!(
        distance_one_operation(&name("loda"), &name("lodas")),
        Some(EditOperation::Insertion)
    );
}

#[test]
fn matches_single_ascii_byte_deletion() {
    assert_eq!(
        distance_one_operation(&name("lodas"), &name("loda")),
        Some(EditOperation::Deletion)
    );
}

#[test]
fn matches_single_ascii_byte_substitution() {
    assert_eq!(
        distance_one_operation(&name("lodash"), &name("lodask")),
        Some(EditOperation::Substitution)
    );
}

#[test]
fn matches_adjacent_ascii_byte_transposition() {
    assert_eq!(
        distance_one_operation(&name("ab"), &name("ba")),
        Some(EditOperation::Transposition)
    );
}

#[test]
fn matches_lodahs_to_lodash_via_transposition() {
    assert_eq!(
        distance_one_operation(&name("lodahs"), &name("lodash")),
        Some(EditOperation::Transposition)
    );
}

#[test]
fn rejects_distance_two_pair_with_two_non_adjacent_substitutions() {
    assert_eq!(distance_one_operation(&name("abcd"), &name("axcy")), None);
}

#[test]
fn rejects_distance_two_pair_with_adjacent_mismatches_that_are_not_a_swap() {
    assert_eq!(distance_one_operation(&name("abcd"), &name("axyd")), None);
}

#[test]
fn rejects_distance_two_pair_with_length_difference_of_two() {
    assert_eq!(distance_one_operation(&name("ab"), &name("abcd")), None);
}

#[test]
fn compares_complete_scoped_identifiers_including_scope_text() {
    assert_eq!(
        distance_one_operation(&name("@scope-a/lodash"), &name("@scope-b/lodash")),
        Some(EditOperation::Substitution)
    );
}

#[test]
fn does_not_equate_scoped_and_unscoped_identical_local_names() {
    assert_eq!(
        distance_one_operation(&name("@scope/lodash"), &name("lodash")),
        None
    );
}

#[test]
fn accepts_names_at_maximum_214_byte_length() {
    let long_name = "a".repeat(MAX_PACKAGE_NAME_BYTES);
    let short_name = "a".repeat(MAX_PACKAGE_NAME_BYTES - 1);
    let candidate = CanonicalPackageName::parse(&long_name);
    let reference = CanonicalPackageName::parse(&short_name);

    assert_eq!(
        candidate.as_ref().map(CanonicalPackageName::as_str),
        Ok(long_name.as_str())
    );
    assert_eq!(
        distance_one_operation(
            candidate.as_ref().expect("candidate should be valid"),
            reference.as_ref().expect("reference should be valid")
        ),
        Some(EditOperation::Deletion)
    );
}

#[test]
fn rejects_empty_name() {
    assert_eq!(
        CanonicalPackageName::parse(""),
        Err(CanonicalPackageNameError::Empty)
    );
}

#[test]
fn rejects_name_over_214_byte_limit() {
    assert_eq!(
        CanonicalPackageName::parse(&"a".repeat(MAX_PACKAGE_NAME_BYTES + 1)),
        Err(CanonicalPackageNameError::TooLong {
            max_bytes: MAX_PACKAGE_NAME_BYTES
        })
    );
}

#[test]
fn rejects_uppercase_ascii_in_name() {
    assert_eq!(
        CanonicalPackageName::parse("Lodash"),
        Err(CanonicalPackageNameError::ContainsUppercase)
    );
}

#[test]
fn rejects_unscoped_name_starting_with_dot() {
    assert_eq!(
        CanonicalPackageName::parse(".hidden"),
        Err(CanonicalPackageNameError::StartsWithDisallowedCharacter)
    );
}

#[test]
fn rejects_unscoped_name_starting_with_underscore() {
    assert_eq!(
        CanonicalPackageName::parse("_private"),
        Err(CanonicalPackageNameError::StartsWithDisallowedCharacter)
    );
}

#[test]
fn rejects_scoped_name_missing_slash_separator() {
    assert_eq!(
        CanonicalPackageName::parse("@scope-no-slash"),
        Err(CanonicalPackageNameError::InvalidScopedName)
    );
}

#[test]
fn rejects_scoped_name_with_empty_scope() {
    assert_eq!(
        CanonicalPackageName::parse("@/name"),
        Err(CanonicalPackageNameError::InvalidScopedName)
    );
}

#[test]
fn rejects_scoped_name_with_empty_local_name() {
    assert_eq!(
        CanonicalPackageName::parse("@scope/"),
        Err(CanonicalPackageNameError::InvalidScopedName)
    );
}

#[test]
fn rejects_scoped_name_with_nested_slash() {
    assert_eq!(
        CanonicalPackageName::parse("@scope/sub/name"),
        Err(CanonicalPackageNameError::InvalidScopedName)
    );
}

#[test]
fn rejects_name_with_unsupported_character() {
    assert_eq!(
        CanonicalPackageName::parse("lo$dash"),
        Err(CanonicalPackageNameError::ContainsUnsupportedCharacter)
    );
}

#[test]
fn accepts_well_formed_scoped_name() {
    assert_eq!(
        CanonicalPackageName::parse("@scope/valid-name")
            .as_ref()
            .map(CanonicalPackageName::as_str),
        Ok("@scope/valid-name")
    );
}

#[test]
fn loads_well_formed_pairs_file() {
    let file = write_test_file(
        "pairs.jsonl",
        b"{\"candidate_name\":\"alpha\",\"reference_name\":\"alphaa\"}\n{\"candidate_name\":\"beta\",\"reference_name\":\"betaa\"}\n",
    );

    let pairs = load_pairs(file.path()).expect("pairs should load");

    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].candidate_name.as_str(), "alpha");
    assert_eq!(pairs[1].reference_name.as_str(), "betaa");
}

#[test]
fn rejects_jsonl_line_over_max_byte_limit() {
    let file = write_test_file("long-line.jsonl", &vec![b'a'; MAX_JSONL_LINE_BYTES + 1]);

    let error = load_pairs(file.path()).expect_err("oversized line should fail");

    assert!(error.contains(":1:"));
    assert!(error.contains("line is"));
}

#[test]
fn rejects_dataset_exceeding_max_byte_total_even_when_lines_are_blank() {
    let mut contents = Vec::with_capacity(MAX_DATASET_BYTES + 2);
    let blank_line = vec![b' '; MAX_JSONL_LINE_BYTES];
    while contents.len() <= MAX_DATASET_BYTES {
        contents.extend_from_slice(&blank_line);
        contents.push(b'\n');
    }
    let file = write_test_file("oversized-blank-lines.jsonl", &contents);

    let error = load_pairs(file.path()).expect_err("oversized dataset should fail");

    assert!(error.contains("dataset exceeds"));
}

#[test]
fn rejects_dataset_exceeding_max_record_count() {
    let contents = npm_sample_fixture(MAX_DATASET_RECORDS + 1);
    let file = write_test_file("too-many-records.jsonl", contents.as_bytes());

    let error = load_npm_sample(file.path()).expect_err("record count should be bounded");

    assert!(error.contains("records"));
}

#[test]
fn accepts_dataset_at_max_record_count() {
    let contents = npm_sample_fixture(MAX_DATASET_RECORDS);
    let file = write_test_file("max-records.jsonl", contents.as_bytes());

    let names = load_npm_sample(file.path()).expect("record count boundary should be accepted");

    assert_eq!(names.len(), MAX_DATASET_RECORDS);
}

#[test]
fn rejects_malformed_jsonl_line() {
    let file = write_test_file("malformed.jsonl", b"not json\n");

    assert!(load_pairs(file.path()).is_err());
}

#[test]
fn rejects_pair_record_missing_reference_name() {
    let file = write_test_file("missing-reference.jsonl", b"{\"candidate_name\":\"x\"}\n");

    let error = load_pairs(file.path()).expect_err("missing field should fail");

    assert!(error.contains("reference_name"));
}

#[test]
fn rejects_pair_record_with_non_string_field() {
    let file = write_test_file(
        "wrong-field-type.jsonl",
        b"{\"candidate_name\":5,\"reference_name\":\"x\"}\n",
    );

    assert!(load_pairs(file.path()).is_err());
}

#[test]
fn rejects_pair_record_with_invalid_package_name() {
    let file = write_test_file(
        "invalid-name.jsonl",
        b"{\"candidate_name\":\"Bad\",\"reference_name\":\"x\"}\n",
    );

    let error = load_pairs(file.path()).expect_err("invalid name should fail");

    assert!(error.contains("candidate_name"));
}

#[test]
fn loads_empty_pairs_file_as_empty_vector() {
    let file = write_test_file("empty.jsonl", b"\n  \n");

    let pairs = load_pairs(file.path()).expect("empty dataset should be accepted");

    assert!(pairs.is_empty());
}

#[test]
fn rejects_unsorted_pairs_file() {
    let file = write_test_file(
        "unsorted-pairs.jsonl",
        b"{\"candidate_name\":\"beta\",\"reference_name\":\"a\"}\n{\"candidate_name\":\"alpha\",\"reference_name\":\"a\"}\n",
    );

    let error = load_pairs(file.path()).expect_err("unsorted pairs should fail");

    assert!(error.contains("ascending byte order"));
}

#[test]
fn rejects_exact_duplicate_pair() {
    let line = b"{\"candidate_name\":\"alpha\",\"reference_name\":\"beta\"}\n";
    let mut contents = line.to_vec();
    contents.extend_from_slice(line);
    let file = write_test_file("duplicate-pairs.jsonl", &contents);

    let error = load_pairs(file.path()).expect_err("duplicate pair should fail");

    assert!(error.contains("duplicate"));
}

#[test]
fn accepts_same_candidate_with_distinct_tuple_sorted_references() {
    let file = write_test_file(
        "multiple-references.jsonl",
        b"{\"candidate_name\":\"alpha\",\"reference_name\":\"beta\"}\n{\"candidate_name\":\"alpha\",\"reference_name\":\"gamma\"}\n",
    );

    let pairs = load_pairs(file.path()).expect("distinct tuples should be accepted");

    assert_eq!(pairs.len(), 2);
}

#[test]
fn rejects_unsorted_npm_sample() {
    let file = write_test_file(
        "unsorted-names.jsonl",
        b"{\"name\":\"beta\"}\n{\"name\":\"alpha\"}\n",
    );

    let error = load_npm_sample(file.path()).expect_err("unsorted names should fail");

    assert!(error.contains("ascending byte order"));
}

#[test]
fn rejects_duplicate_npm_sample_name() {
    let file = write_test_file(
        "duplicate-names.jsonl",
        b"{\"name\":\"alpha\"}\n{\"name\":\"alpha\"}\n",
    );

    let error = load_npm_sample(file.path()).expect_err("duplicate name should fail");

    assert!(error.contains("duplicate"));
}

#[test]
fn loads_well_formed_manifest() {
    let dataset = write_test_file("manifest-dataset.jsonl", b"");
    let contents = manifest_json(dataset.basename(), &"a".repeat(64), true);
    let manifest_file = write_test_file("well-formed.manifest.json", contents.as_bytes());

    let loaded = load_manifest(manifest_file.path(), dataset.path()).expect("manifest should load");

    assert_eq!(loaded.dataset_file, dataset.basename());
    assert_eq!(loaded.content_sha256, "a".repeat(64));
}

#[test]
fn rejects_manifest_over_max_byte_limit() {
    let dataset = write_test_file("manifest-size-dataset.jsonl", b"");
    let manifest_file = write_test_file(
        "oversized.manifest.json",
        &vec![b' '; MAX_MANIFEST_BYTES + 1],
    );

    let error = load_manifest(manifest_file.path(), dataset.path())
        .expect_err("oversized manifest should fail");

    assert!(error.contains("manifest exceeds"));
}

#[test]
fn rejects_manifest_missing_required_field() {
    let dataset = write_test_file("missing-field-dataset.jsonl", b"");
    let mut value: serde_json::Value =
        serde_json::from_str(&manifest_json(dataset.basename(), &"a".repeat(64), true))
            .expect("test manifest should parse");
    value
        .as_object_mut()
        .expect("manifest should be an object")
        .remove("license");
    let manifest_file =
        write_test_file("missing-field.manifest.json", value.to_string().as_bytes());

    let error = load_manifest(manifest_file.path(), dataset.path())
        .expect_err("missing manifest field should fail");

    assert!(error.contains("license"));
}

#[test]
fn rejects_manifest_with_wrong_field_type() {
    let dataset = write_test_file("wrong-type-dataset.jsonl", b"");
    let mut value: serde_json::Value =
        serde_json::from_str(&manifest_json(dataset.basename(), &"a".repeat(64), true))
            .expect("test manifest should parse");
    value["includes_scoped_and_unscoped"] = json!("yes");
    let manifest_file = write_test_file("wrong-type.manifest.json", value.to_string().as_bytes());

    assert!(load_manifest(manifest_file.path(), dataset.path()).is_err());
}

#[test]
fn rejects_manifest_with_malformed_content_sha256() {
    let dataset = write_test_file("bad-hash-dataset.jsonl", b"");
    let contents = manifest_json(dataset.basename(), "not-a-valid-digest", true);
    let manifest_file = write_test_file("bad-hash.manifest.json", contents.as_bytes());

    let error = load_manifest(manifest_file.path(), dataset.path())
        .expect_err("malformed digest should fail");

    assert!(error.contains("content_sha256"));
}

#[test]
fn rejects_manifest_with_uppercase_content_sha256() {
    let dataset = write_test_file("uppercase-hash-dataset.jsonl", b"");
    let contents = manifest_json(dataset.basename(), &"A".repeat(64), true);
    let manifest_file = write_test_file("uppercase-hash.manifest.json", contents.as_bytes());

    assert!(load_manifest(manifest_file.path(), dataset.path()).is_err());
}

#[test]
fn rejects_manifest_where_dataset_file_does_not_match_actual_basename() {
    let dataset = write_test_file("actual-dataset.jsonl", b"");
    let contents = manifest_json("other.jsonl", &"a".repeat(64), true);
    let manifest_file = write_test_file("mismatch.manifest.json", contents.as_bytes());

    let error = load_manifest(manifest_file.path(), dataset.path())
        .expect_err("basename mismatch should fail");

    assert!(error.contains("other.jsonl"));
    assert!(error.contains(dataset.basename()));
}

#[test]
fn rejects_manifest_with_non_synthetic_dataset_claim() {
    let dataset = write_test_file("real-dataset.jsonl", b"");
    let contents = manifest_json(dataset.basename(), &"a".repeat(64), false);
    let manifest_file = write_test_file("real.manifest.json", contents.as_bytes());

    let error = load_manifest(manifest_file.path(), dataset.path())
        .expect_err("non-synthetic manifest should fail");

    assert!(error.contains("real-corpus support"));
}

#[test]
fn reports_full_pair_fixture_coverage_when_all_pairs_match() {
    let report = assembled_report(&[pair("lodas", "lodash"), pair("reactt", "react")], &[]);

    assert_eq!(report["pair_fixture_coverage"]["matched"], 2);
    assert_eq!(report["pair_fixture_coverage"]["total"], 2);
    assert_eq!(
        report["pair_fixture_coverage"]["unmatched_candidate_names"],
        json!([])
    );
}

#[test]
fn reports_unmatched_candidate_names_sorted() {
    let report = assembled_report(&[pair("zzz", "aaa"), pair("xxx", "bbb")], &[]);

    assert_eq!(
        report["pair_fixture_coverage"]["unmatched_candidate_names"],
        json!(["xxx", "zzz"])
    );
}

#[test]
fn produces_zero_frequency_and_empty_matches_for_empty_npm_sample() {
    let report = assembled_report(&[], &[]);

    assert_eq!(report["candidate_frequency_per_1000"], 0.0);
    assert_eq!(report["matches"], json!([]));
}

#[test]
fn finds_both_directed_rows_for_a_matching_pair() {
    let report = assembled_report(&[], &[name("lodash"), name("lodashx"), name("unrelated")]);

    assert_eq!(
        report["matches"],
        json!([
            {"candidate_name":"lodash","reference_name":"lodashx","operation":"insertion"},
            {"candidate_name":"lodashx","reference_name":"lodash","operation":"deletion"}
        ])
    );
}

#[test]
fn stratifies_directed_matches_by_candidate_and_reference_length() {
    let report = assembled_report(&[], &[name("lodash"), name("lodashx"), name("unrelated")]);

    assert_eq!(
        report["stratification"]["by_candidate_length"],
        json!({"6":1,"7":1})
    );
    assert_eq!(
        report["stratification"]["by_reference_length"],
        json!({"6":1,"7":1})
    );
}

#[test]
fn stratifies_matches_by_scope() {
    let report = assembled_report(
        &[],
        &[
            name("@scope/lodash"),
            name("@scope/lodahs"),
            name("lodash"),
            name("lodashx"),
        ],
    );

    assert_eq!(
        report["stratification"]["by_scope"],
        json!({"scoped":2,"unscoped":2})
    );
}

#[test]
fn stratifies_operations_with_all_keys_present_at_zero() {
    let report = assembled_report(&[], &[name("lodash"), name("lodask")]);

    assert_eq!(
        report["stratification"]["by_operation"],
        json!({"deletion":0,"insertion":0,"substitution":2,"transposition":0})
    );
}

#[test]
fn identifies_multi_match_candidates() {
    let report = assembled_report(&[], &[name("ab"), name("ac"), name("xb")]);

    assert_eq!(report["multi_match_candidate_names"], json!(["ab"]));
}

#[test]
fn report_matches_are_sorted_by_candidate_then_reference() {
    let report = assembled_report(&[], &[name("xb"), name("ab"), name("ac")]);
    let matches = report["matches"]
        .as_array()
        .expect("matches should be an array");

    assert_eq!(matches[0]["candidate_name"], "ab");
    assert_eq!(matches[0]["reference_name"], "ac");
    assert_eq!(matches[1]["candidate_name"], "ab");
    assert_eq!(matches[1]["reference_name"], "xb");
}

#[test]
fn report_omits_compile_time_manifest_directory_path() {
    let report = assembled_report(&[], &[]);
    let rendered = serde_json::to_string(&report).expect("report should serialize");

    assert!(!rendered.contains(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn report_includes_harness_validation_notice() {
    let report = assembled_report(&[], &[]);

    assert!(
        report["harness_validation_notice"]
            .as_str()
            .is_some_and(|notice| !notice.is_empty())
    );
}

#[test]
fn rejects_when_comparison_limit_would_be_exceeded() {
    let error = analyze_with_limits(&[], &[name("a"), name("b")], 3, MAX_MATCHES)
        .err()
        .expect("comparison limit should fail");

    assert!(error.contains("comparison"));
}

#[test]
fn accepts_at_comparison_limit() {
    assert!(analyze_with_limits(&[], &[name("a"), name("b")], 4, MAX_MATCHES).is_ok());
}

#[test]
fn rejects_when_match_limit_would_be_exceeded() {
    let error = analyze_with_limits(&[], &[name("a"), name("b")], MAX_COMPARISONS, 1)
        .err()
        .expect("match limit should fail");

    assert!(error.contains("match"));
}

#[test]
fn accepts_at_match_limit() {
    assert!(analyze_with_limits(&[], &[name("a"), name("b")], 4, 2).is_ok());
}

fn npm_sample_fixture(record_count: usize) -> String {
    let mut contents = String::new();
    for index in 0..record_count {
        contents.push_str(&format!("{{\"name\":\"pkg-{index:04}\"}}\n"));
    }
    contents
}
