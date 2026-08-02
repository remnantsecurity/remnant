//! Install-level aggregation and deterministic verdict-line formatting.

use crate::commands::install::verdict::{PackageVerdict, VerdictCategory};
use crate::output::escape_terminal_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallDecision {
    Proceed,
    Abort,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "not yet wired into install::run() — Step 2c wires this into the live path"
    )
)]
pub fn decide(verdicts: &[PackageVerdict], audit: bool) -> InstallDecision {
    if audit
        || verdicts
            .iter()
            .all(|verdict| verdict.category == VerdictCategory::Admitted)
    {
        InstallDecision::Proceed
    } else {
        InstallDecision::Abort
    }
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "not yet wired into install::run() — Step 2c wires this into the live path"
    )
)]
pub fn format_verdict_line(verdict: &PackageVerdict, enforced: bool) -> Option<String> {
    if verdict.category == VerdictCategory::Admitted {
        return None;
    }

    let name = escape_terminal_text(&verdict.name);
    let version = escape_terminal_text(&verdict.version);
    let category = verdict_category_label(verdict.category);
    let finding_ids = verdict
        .finding_ids
        .iter()
        .map(|finding_id| escape_terminal_text(finding_id))
        .collect::<Vec<_>>()
        .join(", ");

    if enforced {
        Some(format!(
            "remnant: blocked {name}@{version}: {category} [{finding_ids}]"
        ))
    } else {
        Some(format!(
            "remnant: audit - {name}@{version} would have blocked: {category} [{finding_ids}]"
        ))
    }
}

fn verdict_category_label(category: VerdictCategory) -> &'static str {
    match category {
        VerdictCategory::Admitted => "admitted",
        VerdictCategory::BlockedIntegrity => "blocked_integrity",
        VerdictCategory::BlockedPolicy => "blocked_policy",
        VerdictCategory::BlockedParse => "blocked_parse",
        VerdictCategory::Error => "error",
    }
}

#[cfg(test)]
mod tests;
