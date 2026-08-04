//! Install command implementation.
//!
//! This module owns the CLI-facing `install` command boundary: resolving the
//! full dependency tree via npm itself, fetching and inspecting every resolved
//! package in-process via `remnant-core`, deciding whether to proceed, and —
//! only if cleared — materializing the install via `npm ci`. See
//! `docs/decisions/0051-proactive-install-inspection-architecture.md`.

mod decision;
mod lockfile;
mod npm;
mod verdict;

/// The result of a completed `install` command.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InstallOutcome {
    /// `npm install --package-lock-only` itself failed to resolve the
    /// dependency tree (e.g. a version that doesn't exist) — no admission
    /// decision was made because there was nothing to inspect yet.
    ResolutionFailed { npm_exit_code: i32 },
    /// Enforce mode: at least one resolved package did not clear inspection.
    /// `npm ci` never ran.
    Blocked,
    /// Cleared (or `--audit`) and `npm ci` ran; carries its exit code.
    Proceeded { npm_exit_code: i32 },
}

impl InstallOutcome {
    pub fn exit_code(self) -> i32 {
        match self {
            InstallOutcome::ResolutionFailed { npm_exit_code } => npm_exit_code,
            InstallOutcome::Blocked => 2,
            InstallOutcome::Proceeded { npm_exit_code } => npm_exit_code,
        }
    }
}

#[derive(Debug)]
pub enum InstallError {
    NpmResolveSpawnFailed(npm::NpmSpawnError),
    LockfileUnreadable { kind: std::io::ErrorKind },
    LockfileParseFailed(lockfile::LockfileParseError),
    UpstreamFetcherUnavailable(remnant_core::FetchPackumentError),
    AsyncRuntimeUnavailable(std::io::Error),
    NpmMaterializeSpawnFailed(npm::NpmSpawnError),
}

impl InstallError {
    pub fn exit_code(&self) -> i32 {
        1
    }
}

pub fn format_error_summary(error: &InstallError) -> Vec<String> {
    match error {
        InstallError::NpmResolveSpawnFailed(npm::NpmSpawnError::NpmBinaryNotFound) => {
            vec![String::from("error: `npm` was not found on PATH")]
        }
        InstallError::NpmResolveSpawnFailed(npm::NpmSpawnError::NpmSpawnFailed(source)) => {
            vec![format!(
                "error: failed to start npm for dependency resolution: {source}"
            )]
        }
        InstallError::LockfileUnreadable { kind } => vec![format!(
            "error: package-lock.json could not be read after resolution ({kind:?})"
        )],
        InstallError::LockfileParseFailed(error) => {
            vec![format!(
                "error: package-lock.json could not be parsed: {error}"
            )]
        }
        InstallError::UpstreamFetcherUnavailable(error) => {
            vec![format!(
                "error: upstream registry configuration is invalid: {error}"
            )]
        }
        InstallError::AsyncRuntimeUnavailable(source) => vec![format!(
            "error: could not start the async runtime for package inspection: {source}"
        )],
        InstallError::NpmMaterializeSpawnFailed(npm::NpmSpawnError::NpmBinaryNotFound) => {
            vec![String::from("error: `npm` was not found on PATH")]
        }
        InstallError::NpmMaterializeSpawnFailed(npm::NpmSpawnError::NpmSpawnFailed(source)) => {
            vec![format!(
                "error: failed to start npm for install materialization: {source}"
            )]
        }
    }
}

/// Runs `npm install <args> --package-lock-only` to resolve the full
/// dependency tree, fetches and inspects every resolved package in-process,
/// and — only if every package clears (or `audit` is set) — materializes the
/// install via `npm ci`. `audit` reports non-admitted packages without
/// blocking; enforce mode (the default) aborts before `npm ci` ever runs.
pub fn run(audit: bool, npm_args: Vec<String>) -> Result<InstallOutcome, InstallError> {
    let resolve_status =
        npm::run_npm_resolve(npm_args).map_err(InstallError::NpmResolveSpawnFailed)?;

    if !resolve_status.success() {
        return Ok(InstallOutcome::ResolutionFailed {
            npm_exit_code: resolve_status.code().unwrap_or(1),
        });
    }

    let lockfile_contents = std::fs::read("package-lock.json")
        .map_err(|error| InstallError::LockfileUnreadable { kind: error.kind() })?;
    let packages = lockfile::parse_resolved_packages(&lockfile_contents)
        .map_err(InstallError::LockfileParseFailed)?;

    println!("remnant: inspecting {} package(s)", packages.len());

    let fetcher = remnant_core::UpstreamFetcher::from_env()
        .map_err(InstallError::UpstreamFetcherUnavailable)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(InstallError::AsyncRuntimeUnavailable)?;

    let verdicts = runtime.block_on(verdict::inspect_resolved_packages(&fetcher, &packages));

    let enforced = !audit;
    for package_verdict in &verdicts {
        if let Some(line) = decision::format_verdict_line(package_verdict, enforced) {
            println!("{line}");
        }
    }
    println!("{}", decision::format_summary_line(&verdicts, audit));

    match decision::decide(&verdicts, audit) {
        decision::InstallDecision::Abort => Ok(InstallOutcome::Blocked),
        decision::InstallDecision::Proceed => {
            let ci_status = npm::run_npm_ci().map_err(InstallError::NpmMaterializeSpawnFailed)?;
            Ok(InstallOutcome::Proceeded {
                npm_exit_code: ci_status.code().unwrap_or(1),
            })
        }
    }
}

#[cfg(test)]
mod tests;
