use std::path::Path;
use std::process::Output;
use std::time::Duration;

use serde_json::Value;
use tokio::process::Command;

const REMNANT_BINARY: &str = "remnant";
const INSPECTION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, PartialEq, Eq)]
pub enum ResponseCategory {
    Admitted,
    BlockedPolicy,
    BlockedParse,
    Error,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Step 3 defines the inspection outcome; Step 4 will read it from request handling"
    )
)]
pub struct InspectionOutcome {
    pub category: ResponseCategory,
    pub finding_ids: Vec<String>,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Step 3 defines the inspection boundary; Step 4 will call it from request handling"
    )
)]
pub async fn run_inspection(artifact_path: &Path) -> InspectionOutcome {
    let inspection =
        tokio::time::timeout(INSPECTION_TIMEOUT, invoke_remnant_inspect(artifact_path)).await;

    match inspection {
        Ok(Ok(output)) => map_inspection_output(output),
        Ok(Err(_)) | Err(_) => error_outcome(),
    }
}

async fn invoke_remnant_inspect(artifact_path: &Path) -> std::io::Result<Output> {
    Command::new(REMNANT_BINARY)
        .arg("inspect")
        .arg("--json")
        .arg(artifact_path)
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
