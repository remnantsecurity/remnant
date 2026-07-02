use std::ffi::OsStr;
use std::path::Path;
use std::process::Output;
use std::time::Duration;

use serde_json::Value;
use tokio::process::Command;

use crate::admission::{InspectionOutcome, ResponseCategory};

// Resolved from PATH at subprocess spawn time with no path validation.
// In production, ensure the `remnant` binary is installed and the deployment
// PATH is restricted to trusted directories before the proxy handles real traffic.
const REMNANT_BINARY: &str = "remnant";
const INSPECTION_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn run_inspection(artifact_path: &Path) -> InspectionOutcome {
    run_inspection_with(
        artifact_path,
        OsStr::new(REMNANT_BINARY),
        INSPECTION_TIMEOUT,
    )
    .await
}

async fn run_inspection_with(
    artifact_path: &Path,
    binary: &OsStr,
    timeout: Duration,
) -> InspectionOutcome {
    let inspection =
        tokio::time::timeout(timeout, invoke_remnant_inspect(artifact_path, binary)).await;

    match inspection {
        Ok(Ok(output)) => map_inspection_output(output),
        Ok(Err(_)) | Err(_) => error_outcome(),
    }
}

async fn invoke_remnant_inspect(artifact_path: &Path, binary: &OsStr) -> std::io::Result<Output> {
    Command::new(binary)
        .arg("inspect")
        .arg("--json")
        .arg(artifact_path)
        .kill_on_drop(true)
        .output()
        .await
}

fn map_inspection_output(output: Output) -> InspectionOutcome {
    match output.status.code() {
        Some(0) => InspectionOutcome {
            category: ResponseCategory::Admitted,
            finding_ids: Vec::new(),
        },
        Some(2) => InspectionOutcome {
            category: ResponseCategory::BlockedPolicy,
            finding_ids: collect_policy_finding_ids(&output.stdout),
        },
        Some(1) => map_exit_one_output(&output.stdout),
        _ => error_outcome(),
    }
}

fn map_exit_one_output(stdout: &[u8]) -> InspectionOutcome {
    let Ok(report) = serde_json::from_slice::<Value>(stdout) else {
        return error_outcome();
    };

    match report
        .get("error")
        .and_then(Value::as_object)
        .and_then(|error| error.get("kind"))
        .and_then(Value::as_str)
    {
        Some("archive" | "package_json") => InspectionOutcome {
            category: ResponseCategory::BlockedParse,
            finding_ids: Vec::new(),
        },
        Some("inspect") | None | Some(_) => error_outcome(),
    }
}

fn collect_policy_finding_ids(stdout: &[u8]) -> Vec<String> {
    let Ok(report) = serde_json::from_slice::<Value>(stdout) else {
        return Vec::new();
    };

    let Some(findings) = report
        .get("policy")
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("findings"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    findings
        .iter()
        .filter_map(|finding| {
            finding
                .as_object()
                .and_then(|finding| finding.get("rule_id"))
                .and_then(Value::as_str)
                .map(String::from)
        })
        .collect()
}

fn error_outcome() -> InspectionOutcome {
    InspectionOutcome {
        category: ResponseCategory::Error,
        finding_ids: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
