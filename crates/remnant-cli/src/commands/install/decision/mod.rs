//! Install-level aggregation and deterministic verdict-line formatting.

use crate::commands::install::verdict::{PackageVerdict, VerdictCategory};
use crate::output::escape_terminal_text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallDecision {
    Proceed,
    Abort,
}

pub fn decide(verdicts: &[PackageVerdict], non_blocking: bool) -> InstallDecision {
    if non_blocking
        || verdicts
            .iter()
            .all(|verdict| verdict.category == VerdictCategory::Admitted)
    {
        InstallDecision::Proceed
    } else {
        InstallDecision::Abort
    }
}

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
            "remnant: flagged {name}@{version}: {category} [{finding_ids}]"
        ))
    }
}

pub fn format_summary_line(verdicts: &[PackageVerdict], non_blocking: bool) -> String {
    let total = verdicts.len();
    let admitted = verdicts
        .iter()
        .filter(|verdict| verdict.category == VerdictCategory::Admitted)
        .count();
    let non_admitted = total - admitted;

    if non_blocking {
        format!("remnant: analyzed {total} package(s), {admitted} admitted, {non_admitted} flagged")
    } else {
        format!("remnant: analyzed {total} package(s), {admitted} admitted, {non_admitted} blocked")
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
