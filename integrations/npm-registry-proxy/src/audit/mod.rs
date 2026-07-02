use serde_json::{Map, Value, json};

pub struct AuditRecord {
    pub timestamp: String,
    pub request_id: String,
    pub package_name: String,
    pub version: String,
    pub artifact_key: String,
    pub integrity_status: String,
    pub computed_digest: String,
    pub remnant_version: String,
    pub response_category: String,
    pub finding_ids: Vec<String>,
    pub duration_ms: u64,
    pub upstream_registry_host: Option<String>,
    pub tarball_byte_length: Option<u64>,
}

/// Returns the NDJSON line for the record without a trailing newline.
pub fn format_audit_record(record: &AuditRecord) -> String {
    let mut body = Map::new();

    body.insert(String::from("timestamp"), json!(record.timestamp));
    body.insert(String::from("requestId"), json!(record.request_id));
    body.insert(String::from("packageName"), json!(record.package_name));
    body.insert(String::from("version"), json!(record.version));
    body.insert(String::from("artifactKey"), json!(record.artifact_key));
    body.insert(
        String::from("integrityStatus"),
        json!(record.integrity_status),
    );
    body.insert(
        String::from("computedDigest"),
        json!(record.computed_digest),
    );
    body.insert(
        String::from("remnantVersion"),
        json!(record.remnant_version),
    );
    body.insert(
        String::from("responseCategory"),
        json!(record.response_category),
    );
    body.insert(
        String::from("policyStatus"),
        json!(record.response_category),
    );
    body.insert(String::from("findingIds"), json!(record.finding_ids));
    body.insert(String::from("durationMs"), json!(record.duration_ms));

    if let Some(upstream_registry_host) = &record.upstream_registry_host {
        body.insert(
            String::from("upstreamRegistryHost"),
            json!(upstream_registry_host),
        );
    }

    if let Some(tarball_byte_length) = record.tarball_byte_length {
        body.insert(
            String::from("tarballByteLength"),
            json!(tarball_byte_length),
        );
    }

    Value::Object(body).to_string()
}

/// Writes one NDJSON line to stdout.
pub fn write_audit_record(record: &AuditRecord) {
    println!("{}", format_audit_record(record));
}

#[cfg(test)]
mod tests;
