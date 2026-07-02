use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::admission::ResponseCategory;
use crate::artifact::{
    ArtifactMapping, IntegrityStatus, compute_sha512_hex, rewrite_packument_tarball_urls,
    verify_sha512_integrity,
};
use crate::audit::{AuditRecord, write_audit_record};
use crate::inspection::run_inspection;
use crate::package_name::ValidatedPackageName;
use crate::upstream::UpstreamFetcher;

const ARTIFACT_KEY_HEX_LENGTH: usize = 64;
const MAX_BLOCK_RESPONSE_BYTES: usize = 2 * 1024;

#[derive(Clone)]
pub(crate) struct AppState {
    fetcher: Arc<UpstreamFetcher>,
    artifact_mapping: Arc<RwLock<HashMap<String, ArtifactMapping>>>,
    proxy_origin: String,
    remnant_version: String,
}

impl AppState {
    pub(crate) fn new(
        fetcher: UpstreamFetcher,
        proxy_origin: impl Into<String>,
        remnant_version: impl Into<String>,
    ) -> Self {
        Self {
            fetcher: Arc::new(fetcher),
            artifact_mapping: Arc::new(RwLock::new(HashMap::new())),
            proxy_origin: proxy_origin.into(),
            remnant_version: remnant_version.into(),
        }
    }
}

pub(crate) fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/remnant/tarballs/{filename}", get(handle_tarball_request))
        // Scoped package names (@scope/name) must be percent-encoded by the client
        // as a single path segment (@scope%2Fname). An unencoded /@scope/name request
        // matches a two-segment path and returns 404.
        .route("/{package}", get(handle_metadata_request))
        .with_state(state)
}

struct TempFile {
    path: PathBuf,
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

async fn handle_metadata_request(
    State(state): State<AppState>,
    Path(raw_package): Path<String>,
) -> Response<Body> {
    let request_id = Uuid::new_v4().to_string();

    let package_name = match ValidatedPackageName::parse(raw_package) {
        Ok(package_name) => package_name,
        Err(_) => {
            return build_block_response(
                &request_id,
                ResponseCategory::BlockedParse,
                "package name is not valid",
                vec![],
            );
        }
    };

    let response = match state
        .fetcher
        .fetch_abbreviated_packument(&package_name)
        .await
    {
        Ok(response) => response,
        Err(_) => {
            return build_block_response(
                &request_id,
                ResponseCategory::BlockedFetch,
                "upstream registry fetch failed",
                vec![],
            );
        }
    };

    debug_assert_eq!(response.status_code, StatusCode::OK);

    let rewritten = match rewrite_packument_tarball_urls(&response.bytes, &state.proxy_origin) {
        Ok(rewritten) => rewritten,
        Err(_) => {
            return build_block_response(
                &request_id,
                ResponseCategory::BlockedParse,
                "package metadata could not be parsed",
                vec![],
            );
        }
    };

    {
        let mut artifact_mapping = state.artifact_mapping.write().await;
        artifact_mapping.extend(rewritten.artifacts);
    }

    response_with_json_content_type(StatusCode::OK, Body::from(rewritten.bytes))
}

async fn handle_tarball_request(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Response<Body> {
    let request_id = Uuid::new_v4().to_string();
    let request_start = std::time::Instant::now();
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let Some(artifact_key) = valid_artifact_key_from_filename(&filename) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Some(mapping) = state
        .artifact_mapping
        .read()
        .await
        .get(artifact_key)
        .cloned()
    else {
        return Json(json!({ "error": "artifact key is not known to this instance" }))
            .into_response_with_status(StatusCode::NOT_FOUND);
    };

    let tarball_bytes = match state
        .fetcher
        .fetch_tarball_bytes(&mapping.upstream_url)
        .await
    {
        Ok(bytes) => bytes,
        Err(_) => {
            return build_block_response(
                &request_id,
                ResponseCategory::BlockedFetch,
                "upstream tarball fetch failed",
                vec![],
            );
        }
    };

    let computed_digest = compute_sha512_hex(&tarball_bytes);
    let tarball_byte_length = tarball_bytes.len() as u64;
    let upstream_host = state.fetcher.upstream_host().to_string();

    let integrity_status = verify_sha512_integrity(mapping.integrity.as_deref(), &tarball_bytes);

    if integrity_status != IntegrityStatus::Verified {
        let duration_ms = request_start.elapsed().as_millis() as u64;
        write_audit_record(&AuditRecord {
            timestamp,
            request_id: request_id.clone(),
            package_name: mapping.package_name.clone(),
            version: mapping.version.clone(),
            artifact_key: artifact_key.to_string(),
            integrity_status: integrity_status_str(&integrity_status).to_string(),
            computed_digest,
            remnant_version: state.remnant_version.clone(),
            response_category: String::from("blocked_integrity"),
            finding_ids: vec![],
            duration_ms,
            upstream_registry_host: Some(upstream_host),
            tarball_byte_length: Some(tarball_byte_length),
        });
        return build_block_response(
            &request_id,
            ResponseCategory::BlockedIntegrity,
            "artifact integrity verification failed",
            vec![],
        );
    }

    let temp_path = std::env::temp_dir().join(format!("remnant-{}.tgz", Uuid::new_v4()));
    let _temp_file = TempFile {
        path: temp_path.clone(),
    };

    if tokio::fs::write(&temp_path, &tarball_bytes).await.is_err() {
        let duration_ms = request_start.elapsed().as_millis() as u64;
        write_audit_record(&AuditRecord {
            timestamp,
            request_id: request_id.clone(),
            package_name: mapping.package_name.clone(),
            version: mapping.version.clone(),
            artifact_key: artifact_key.to_string(),
            integrity_status: String::from("verified"),
            computed_digest,
            remnant_version: state.remnant_version.clone(),
            response_category: String::from("error"),
            finding_ids: vec![],
            duration_ms,
            upstream_registry_host: Some(upstream_host),
            tarball_byte_length: Some(tarball_byte_length),
        });
        return build_block_response(
            &request_id,
            ResponseCategory::Error,
            "artifact could not be written for inspection",
            vec![],
        );
    }

    let outcome = run_inspection(&temp_path).await;
    let duration_ms = request_start.elapsed().as_millis() as u64;

    write_audit_record(&AuditRecord {
        timestamp,
        request_id: request_id.clone(),
        package_name: mapping.package_name.clone(),
        version: mapping.version.clone(),
        artifact_key: artifact_key.to_string(),
        integrity_status: String::from("verified"),
        computed_digest,
        remnant_version: state.remnant_version.clone(),
        response_category: response_category_name(&outcome.category).to_string(),
        finding_ids: outcome.finding_ids.clone(),
        duration_ms,
        upstream_registry_host: Some(upstream_host),
        tarball_byte_length: Some(tarball_byte_length),
    });

    match outcome.category {
        ResponseCategory::Admitted => {
            let mut response = Response::new(Body::from(tarball_bytes));
            *response.status_mut() = StatusCode::OK;
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            );
            response
        }
        ResponseCategory::BlockedPolicy => build_block_response(
            &request_id,
            ResponseCategory::BlockedPolicy,
            "artifact failed policy checks",
            outcome.finding_ids,
        ),
        ResponseCategory::BlockedParse => build_block_response(
            &request_id,
            ResponseCategory::BlockedParse,
            "artifact could not be inspected",
            vec![],
        ),
        ResponseCategory::BlockedFetch
        | ResponseCategory::BlockedIntegrity
        | ResponseCategory::Error => build_block_response(
            &request_id,
            ResponseCategory::Error,
            "artifact inspection failed",
            vec![],
        ),
    }
}

fn valid_artifact_key_from_filename(filename: &str) -> Option<&str> {
    let artifact_key = filename.strip_suffix(".tgz")?;

    if artifact_key.len() != ARTIFACT_KEY_HEX_LENGTH {
        return None;
    }

    if artifact_key
        .bytes()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        Some(artifact_key)
    } else {
        None
    }
}

fn build_block_response(
    request_id: &str,
    category: ResponseCategory,
    error: &'static str,
    finding_ids: Vec<String>,
) -> Response<Body> {
    let finding_ids =
        finding_ids_within_block_response_limit(&category, error, request_id, finding_ids);
    let body = Json(json!({
        "error": error,
        "category": response_category_name(&category),
        "findingIds": finding_ids,
        "requestId": request_id,
    }));

    body.into_response_with_status(response_category_status(&category))
}

fn finding_ids_within_block_response_limit(
    category: &ResponseCategory,
    error: &'static str,
    request_id: &str,
    finding_ids: Vec<String>,
) -> Vec<String> {
    let mut capped_finding_ids = Vec::new();

    for finding_id in finding_ids {
        capped_finding_ids.push(finding_id);

        if block_response_body_len(category, error, request_id, &capped_finding_ids)
            > MAX_BLOCK_RESPONSE_BYTES
        {
            capped_finding_ids.pop();
            break;
        }
    }

    capped_finding_ids
}

fn integrity_status_str(status: &IntegrityStatus) -> &'static str {
    match status {
        IntegrityStatus::Verified => "verified",
        IntegrityStatus::Mismatch => "mismatch",
        IntegrityStatus::Absent => "absent",
        IntegrityStatus::Unsupported => "unsupported",
    }
}

fn block_response_body_len(
    category: &ResponseCategory,
    error: &'static str,
    request_id: &str,
    finding_ids: &[String],
) -> usize {
    serde_json::to_vec(&json!({
        "error": error,
        "category": response_category_name(category),
        "findingIds": finding_ids,
        "requestId": request_id,
    }))
    .expect("block response JSON serialization should not fail")
    .len()
}

fn response_category_status(category: &ResponseCategory) -> StatusCode {
    match category {
        ResponseCategory::BlockedParse => StatusCode::UNPROCESSABLE_ENTITY,
        ResponseCategory::BlockedPolicy | ResponseCategory::BlockedIntegrity => {
            StatusCode::FORBIDDEN
        }
        ResponseCategory::BlockedFetch => StatusCode::BAD_GATEWAY,
        ResponseCategory::Error => StatusCode::INTERNAL_SERVER_ERROR,
        ResponseCategory::Admitted => unreachable!(
            "Admitted responses are returned directly; build_block_response must not be called with Admitted"
        ),
    }
}

fn response_category_name(category: &ResponseCategory) -> &'static str {
    match category {
        ResponseCategory::Admitted => "admitted",
        ResponseCategory::BlockedPolicy => "blocked_policy",
        ResponseCategory::BlockedParse => "blocked_parse",
        ResponseCategory::BlockedIntegrity => "blocked_integrity",
        ResponseCategory::BlockedFetch => "blocked_fetch",
        ResponseCategory::Error => "error",
    }
}

fn response_with_json_content_type(status: StatusCode, body: Body) -> Response<Body> {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
}

trait JsonResponseExt {
    fn into_response_with_status(self, status: StatusCode) -> Response<Body>;
}

impl JsonResponseExt for Json<Value> {
    fn into_response_with_status(self, status: StatusCode) -> Response<Body> {
        let mut response = self.into_response();
        *response.status_mut() = status;
        response
    }
}

#[cfg(test)]
mod tests;
