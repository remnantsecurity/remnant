//! npm subprocess argument construction for resolve and materialize phases.

use std::io;
use std::process::{Command, ExitStatus};

#[derive(Debug)]
pub enum NpmSpawnError {
    NpmBinaryNotFound,
    NpmSpawnFailed(io::Error),
}

pub fn build_resolve_args(npm_args: Vec<String>) -> Vec<String> {
    let mut args = vec![String::from("install"), String::from("--package-lock-only")];
    args.extend(npm_args);
    args
}

pub fn run_npm_resolve(npm_args: Vec<String>) -> Result<ExitStatus, NpmSpawnError> {
    run_npm(build_resolve_args(npm_args))
}

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
