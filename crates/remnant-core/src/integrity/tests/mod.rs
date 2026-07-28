use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::{Digest, Sha512};

use super::*;

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

fn sha512_sri_payload(bytes: &[u8]) -> String {
    STANDARD.encode(Sha512::digest(bytes))
}
