use serde_json::Value;

use super::{AuditRecord, format_audit_record};

#[test]
fn format_audit_record_serializes_all_required_fields() {
    let record = audit_record_with_defaults();

    let body = serde_json::from_str::<Value>(&format_audit_record(&record)).unwrap();

    assert_eq!(body["requestId"], "test-request-id");
    assert_eq!(body["packageName"], "left-pad");
    assert_eq!(body["version"], "1.3.0");
    assert_eq!(
        body["artifactKey"],
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(body["integrityStatus"], "verified");
    assert!(
        body["computedDigest"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert_eq!(body["remnantVersion"], "remnant 0.1.1");
    assert_eq!(body["responseCategory"], "admitted");
    assert_eq!(body["findingIds"], serde_json::json!([]));
    assert_eq!(body["durationMs"], 42);
    assert_eq!(body["mode"], "enforce");
    assert_eq!(body["enforced"], true);
    assert_eq!(body["upstreamRegistryHost"], "registry.npmjs.org");
    assert_eq!(body["tarballByteLength"], 1024);
}

#[test]
fn format_audit_record_serializes_finding_ids() {
    let record = AuditRecord {
        response_category: String::from("blocked_policy"),
        finding_ids: vec![String::from("install-scripts-disallowed")],
        ..audit_record_with_defaults()
    };

    let body = serde_json::from_str::<Value>(&format_audit_record(&record)).unwrap();

    assert_eq!(
        body["findingIds"],
        serde_json::json!(["install-scripts-disallowed"])
    );
    assert_eq!(body["responseCategory"], "blocked_policy");
}

#[test]
fn format_audit_record_serializes_audit_mode_and_unenforced() {
    let record = AuditRecord {
        mode: String::from("audit"),
        enforced: false,
        ..audit_record_with_defaults()
    };

    let body = serde_json::from_str::<Value>(&format_audit_record(&record)).unwrap();

    assert_eq!(body["mode"], "audit");
    assert_eq!(body["enforced"], false);
}

#[test]
fn format_audit_record_omits_optional_fields_when_none() {
    let record = AuditRecord {
        upstream_registry_host: None,
        tarball_byte_length: None,
        ..audit_record_with_defaults()
    };

    let body = serde_json::from_str::<Value>(&format_audit_record(&record)).unwrap();

    assert!(body.get("upstreamRegistryHost").is_none());
    assert!(body.get("tarballByteLength").is_none());
}

#[test]
fn format_audit_record_omits_policy_status() {
    let record = audit_record_with_defaults();

    let body = serde_json::from_str::<Value>(&format_audit_record(&record)).unwrap();

    assert!(body.get("policyStatus").is_none());
}

#[test]
fn format_audit_record_output_is_valid_json() {
    let record = AuditRecord {
        package_name: String::from("left-pad\"\ntest"),
        ..audit_record_with_defaults()
    };

    assert!(serde_json::from_str::<Value>(&format_audit_record(&record)).is_ok());
}

fn audit_record_with_defaults() -> AuditRecord {
    AuditRecord {
        timestamp: String::from("2026-07-01T00:00:00.000Z"),
        request_id: String::from("test-request-id"),
        package_name: String::from("left-pad"),
        version: String::from("1.3.0"),
        artifact_key: String::from(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        ),
        integrity_status: String::from("verified"),
        computed_digest: String::from("abc123"),
        remnant_version: String::from("remnant 0.1.1"),
        response_category: String::from("admitted"),
        finding_ids: vec![],
        duration_ms: 42,
        mode: String::from("enforce"),
        enforced: true,
        upstream_registry_host: Some(String::from("registry.npmjs.org")),
        tarball_byte_length: Some(1024),
    }
}
