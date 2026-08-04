use super::*;

#[test]
fn resolution_failed_outcome_mirrors_npms_own_exit_code() {
    assert_eq!(
        InstallOutcome::ResolutionFailed { npm_exit_code: 1 }.exit_code(),
        1
    );
}

#[test]
fn blocked_outcome_uses_the_policy_failure_exit_code() {
    assert_eq!(InstallOutcome::Blocked.exit_code(), 2);
}

#[test]
fn proceeded_outcome_mirrors_npm_cis_own_exit_code() {
    assert_eq!(
        InstallOutcome::Proceeded { npm_exit_code: 137 }.exit_code(),
        137
    );
}
