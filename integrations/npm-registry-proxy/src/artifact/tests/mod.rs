use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;
use sha2::{Digest, Sha512};

use super::*;

const LEFT_PAD_TARBALL_URL: &str = "https://registry.npmjs.org/left-pad/-/left-pad-1.3.0.tgz";
const LEFT_PAD_ARTIFACT_KEY: &str =
    "33172f3e7416928880852dd5c3bfe96d6152c6d326974716724c486513436db8";

#[test]
fn rewrites_tarball_url_and_builds_artifact_mapping() {
    let packument = format!(
        r#"{{
            "name": "left-pad",
            "dist-tags": {{"latest": "1.3.0"}},
            "versions": {{
                "1.3.0": {{
                    "name": "left-pad",
                    "version": "1.3.0",
                    "dist": {{
                        "tarball": "{LEFT_PAD_TARBALL_URL}",
                        "integrity": "sha512-{integrity}"
                    }}
                }}
            }}
        }}"#,
        integrity = sha512_sri_payload(b"artifact bytes")
    );

    let rewritten =
        rewrite_packument_tarball_urls(packument.as_bytes(), "http://localhost:4873").unwrap();
    let rewritten_json: Value = serde_json::from_slice(&rewritten.bytes).unwrap();
    let rewritten_tarball_url = rewritten_json["versions"]["1.3.0"]["dist"]["tarball"]
        .as_str()
        .unwrap();
    let mapping = rewritten.artifacts.get(LEFT_PAD_ARTIFACT_KEY).unwrap();

    assert_eq!(
        rewritten_tarball_url,
        format!("http://localhost:4873/remnant/tarballs/{LEFT_PAD_ARTIFACT_KEY}.tgz")
    );
    assert_eq!(mapping.package_name, "left-pad");
    assert_eq!(mapping.version, "1.3.0");
    assert_eq!(mapping.upstream_url, LEFT_PAD_TARBALL_URL);
    assert!(mapping.integrity.as_ref().unwrap().starts_with("sha512-"));
}

#[test]
fn computes_artifact_key_from_null_separated_tuple() {
    let artifact_key = compute_artifact_key("left-pad", "1.3.0", LEFT_PAD_TARBALL_URL);

    assert_eq!(artifact_key, LEFT_PAD_ARTIFACT_KEY);
}

#[test]
fn preserves_absent_integrity_in_artifact_mapping() {
    let packument = format!(
        r#"{{
            "name": "left-pad",
            "versions": {{
                "1.3.0": {{
                    "dist": {{
                        "tarball": "{LEFT_PAD_TARBALL_URL}",
                        "shasum": "5b8a3a7765dfe001261dde915589e782f8c94d1e"
                    }}
                }}
            }}
        }}"#
    );

    let rewritten =
        rewrite_packument_tarball_urls(packument.as_bytes(), "http://localhost:4873").unwrap();
    let mapping = rewritten.artifacts.get(LEFT_PAD_ARTIFACT_KEY).unwrap();

    assert_eq!(mapping.integrity, None);
}

#[test]
fn rejects_packument_that_is_not_valid_json() {
    let error = rewrite_packument_tarball_urls(b"not json", "http://localhost:4873")
        .err()
        .unwrap();

    assert_eq!(error, PackumentRewriteError::InvalidJson);
}

#[test]
fn rejects_packument_root_that_is_not_object() {
    let error = rewrite_packument_tarball_urls(br#"["not", "object"]"#, "http://localhost:4873")
        .err()
        .unwrap();

    assert_eq!(error, PackumentRewriteError::RootIsNotObject);
}

#[test]
fn rejects_packument_missing_package_name() {
    let error = rewrite_error(serde_json::json!({
        "versions": {}
    }));

    assert_eq!(error, PackumentRewriteError::MissingPackageName);
}

#[test]
fn rejects_packument_with_invalid_package_name() {
    let error = rewrite_error(serde_json::json!({
        "name": "Left-Pad",
        "versions": {}
    }));

    assert_eq!(error, PackumentRewriteError::InvalidPackageName);
}

#[test]
fn rejects_packument_missing_versions_object() {
    let error = rewrite_error(serde_json::json!({
        "name": "left-pad"
    }));

    assert_eq!(error, PackumentRewriteError::VersionsMissing);
}

#[test]
fn rejects_packument_versions_that_is_not_object() {
    let error = rewrite_error(serde_json::json!({
        "name": "left-pad",
        "versions": []
    }));

    assert_eq!(error, PackumentRewriteError::VersionsIsNotObject);
}

#[test]
fn rejects_packument_with_too_many_versions_before_mapping() {
    let mut versions = serde_json::Map::new();
    for index in 0..=MAX_VERSIONS {
        versions.insert(
            format!("1.0.{index}"),
            serde_json::json!({
                "dist": {
                    "tarball": format!("https://registry.npmjs.org/pkg/-/pkg-1.0.{index}.tgz")
                }
            }),
        );
    }

    let packument = serde_json::json!({
        "name": "pkg",
        "versions": versions
    });

    let error = rewrite_packument_tarball_urls(
        serde_json::to_vec(&packument).unwrap().as_slice(),
        "http://localhost:4873",
    )
    .err()
    .unwrap();

    assert_eq!(
        error,
        PackumentRewriteError::TooManyVersions {
            limit: MAX_VERSIONS
        }
    );
}

#[test]
fn rejects_packument_with_dist_tags_that_is_not_object() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "dist-tags": "latest",
        "versions": {}
    }));

    assert_eq!(error, PackumentRewriteError::DistTagsIsNotObject);
}

#[test]
fn rejects_packument_with_too_many_dist_tags() {
    let mut dist_tags = serde_json::Map::new();
    for index in 0..=MAX_DIST_TAGS {
        dist_tags.insert(format!("tag-{index}"), serde_json::json!("1.0.0"));
    }

    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "dist-tags": dist_tags,
        "versions": {}
    }));

    assert_eq!(
        error,
        PackumentRewriteError::TooManyDistTags {
            limit: MAX_DIST_TAGS
        }
    );
}

#[test]
fn rejects_empty_version_string() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "": {
                "dist": {
                    "tarball": "https://registry.npmjs.org/pkg/-/pkg-1.0.0.tgz"
                }
            }
        }
    }));

    assert_eq!(error, PackumentRewriteError::VersionStringIsEmpty);
}

#[test]
fn rejects_over_limit_version_string() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1".repeat(MAX_VERSION_BYTES + 1): {
                "dist": {
                    "tarball": "https://registry.npmjs.org/pkg/-/pkg-1.0.0.tgz"
                }
            }
        }
    }));

    assert_eq!(
        error,
        PackumentRewriteError::VersionStringTooLong {
            limit: MAX_VERSION_BYTES
        }
    );
}

#[test]
fn rejects_version_entry_that_is_not_object() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": []
        }
    }));

    assert_eq!(error, PackumentRewriteError::VersionEntryIsNotObject);
}

#[test]
fn rejects_version_entry_missing_dist() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": {}
        }
    }));

    assert_eq!(error, PackumentRewriteError::DistIsNotObject);
}

#[test]
fn rejects_dist_that_is_not_object() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": {
                "dist": []
            }
        }
    }));

    assert_eq!(error, PackumentRewriteError::DistIsNotObject);
}

#[test]
fn rejects_missing_tarball_url() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": {
                "dist": {}
            }
        }
    }));

    assert_eq!(error, PackumentRewriteError::TarballUrlMissing);
}

#[test]
fn rejects_tarball_url_that_is_not_string() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": {
                "dist": {
                    "tarball": 1
                }
            }
        }
    }));

    assert_eq!(error, PackumentRewriteError::TarballUrlIsNotString);
}

#[test]
fn rejects_over_limit_tarball_url() {
    let tarball_url = format!("https://registry.npmjs.org/pkg/-/{}.tgz", "a".repeat(256));
    let packument = serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": {
                "dist": {
                    "tarball": tarball_url
                }
            }
        }
    });

    let error = rewrite_packument_tarball_urls(
        serde_json::to_vec(&packument).unwrap().as_slice(),
        "http://localhost:4873",
    )
    .err()
    .unwrap();

    assert_eq!(
        error,
        PackumentRewriteError::TarballUrlTooLong {
            limit: MAX_TARBALL_URL_BYTES
        }
    );
}

#[test]
fn rejects_invalid_tarball_url() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": {
                "dist": {
                    "tarball": "not a url"
                }
            }
        }
    }));

    assert_eq!(error, PackumentRewriteError::TarballUrlInvalid);
}

#[test]
fn rejects_tarball_url_scheme_that_is_not_https() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": {
                "dist": {
                    "tarball": "http://registry.npmjs.org/pkg/-/pkg-1.0.0.tgz"
                }
            }
        }
    }));

    assert_eq!(error, PackumentRewriteError::TarballUrlSchemeNotHttps);
}

#[test]
fn rejects_integrity_that_is_not_string() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": {
                "dist": {
                    "tarball": "https://registry.npmjs.org/pkg/-/pkg-1.0.0.tgz",
                    "integrity": 1
                }
            }
        }
    }));

    assert_eq!(error, PackumentRewriteError::IntegrityIsNotString);
}

#[test]
fn rejects_over_limit_integrity() {
    let error = rewrite_error(serde_json::json!({
        "name": "pkg",
        "versions": {
            "1.0.0": {
                "dist": {
                    "tarball": "https://registry.npmjs.org/pkg/-/pkg-1.0.0.tgz",
                    "integrity": "a".repeat(MAX_INTEGRITY_BYTES + 1)
                }
            }
        }
    }));

    assert_eq!(
        error,
        PackumentRewriteError::IntegrityTooLong {
            limit: MAX_INTEGRITY_BYTES
        }
    );
}

#[test]
fn reports_verified_integrity_for_matching_sha512_sri() {
    let artifact_bytes = b"artifact bytes";
    let integrity = format!("sha512-{}", sha512_sri_payload(artifact_bytes));

    let status = verify_sha512_integrity(Some(&integrity), artifact_bytes);

    assert_eq!(status, IntegrityStatus::Verified);
}

#[test]
fn reports_mismatch_integrity_for_non_matching_sha512_sri() {
    let integrity = format!("sha512-{}", sha512_sri_payload(b"different bytes"));

    let status = verify_sha512_integrity(Some(&integrity), b"artifact bytes");

    assert_eq!(status, IntegrityStatus::Mismatch);
}

#[test]
fn reports_absent_integrity_when_metadata_has_no_integrity_value() {
    let status = verify_sha512_integrity(None, b"artifact bytes");

    assert_eq!(status, IntegrityStatus::Absent);
}

#[test]
fn reports_unsupported_integrity_for_sha1_sri() {
    let status = verify_sha512_integrity(Some("sha1-deadbeef"), b"artifact bytes");

    assert_eq!(status, IntegrityStatus::Unsupported);
}

#[test]
fn reports_unsupported_integrity_for_malformed_sha512_sri() {
    let status = verify_sha512_integrity(Some("sha512-not-valid-base64"), b"artifact bytes");

    assert_eq!(status, IntegrityStatus::Unsupported);
}

#[test]
fn reports_unsupported_integrity_for_sha512_sri_with_embedded_whitespace() {
    let status = verify_sha512_integrity(Some("sha512-abc def"), b"artifact bytes");

    assert_eq!(status, IntegrityStatus::Unsupported);
}

#[test]
fn reports_unsupported_integrity_for_wrong_sha512_digest_length() {
    let digest = STANDARD.encode([0_u8; 32]);
    let integrity = format!("sha512-{digest}");

    let status = verify_sha512_integrity(Some(&integrity), b"artifact bytes");

    assert_eq!(status, IntegrityStatus::Unsupported);
}

fn rewrite_error(packument: Value) -> PackumentRewriteError {
    rewrite_packument_tarball_urls(
        serde_json::to_vec(&packument).unwrap().as_slice(),
        "http://localhost:4873",
    )
    .err()
    .unwrap()
}

fn sha512_sri_payload(bytes: &[u8]) -> String {
    STANDARD.encode(Sha512::digest(bytes))
}
