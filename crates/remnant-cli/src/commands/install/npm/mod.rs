//! npm subprocess argument construction for resolve and materialize phases.

use std::io;
use std::process::{Command, ExitStatus};

#[derive(Debug)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "not yet wired into install::run() — Step 2c wires this into the live path"
    )
)]
#[cfg_attr(
    test,
    expect(
        dead_code,
        reason = "real npm subprocess wrappers are intentionally not unit-tested"
    )
)]
pub enum NpmSpawnError {
    NpmBinaryNotFound,
    NpmSpawnFailed(io::Error),
}

#[cfg_attr(not(test), allow(unfulfilled_lint_expectations))]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "not yet wired into install::run() — Step 2c wires this into the live path"
    )
)]
pub fn build_resolve_args(npm_args: Vec<String>) -> Vec<String> {
    let mut args = vec![String::from("install"), String::from("--package-lock-only")];
    args.extend(npm_args);
    args
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "not yet wired into install::run() — Step 2c wires this into the live path"
    )
)]
#[cfg_attr(
    test,
    expect(
        dead_code,
        reason = "real npm subprocess wrappers are intentionally not unit-tested"
    )
)]
pub fn run_npm_resolve(npm_args: Vec<String>) -> Result<ExitStatus, NpmSpawnError> {
    run_npm(build_resolve_args(npm_args))
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "not yet wired into install::run() — Step 2c wires this into the live path"
    )
)]
#[cfg_attr(
    test,
    expect(
        dead_code,
        reason = "real npm subprocess wrappers are intentionally not unit-tested"
    )
)]
pub fn run_npm_ci() -> Result<ExitStatus, NpmSpawnError> {
    run_npm(vec![String::from("ci")])
}

fn run_npm(args: Vec<String>) -> Result<ExitStatus, NpmSpawnError> {
    Command::new("npm").args(&args).status().map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            NpmSpawnError::NpmBinaryNotFound
        } else {
            NpmSpawnError::NpmSpawnFailed(error)
        }
    })
}

#[cfg(test)]
mod tests;
