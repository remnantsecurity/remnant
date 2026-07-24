//! Install command implementation.
//!
//! This module owns the CLI-facing `install` command boundary: starting an
//! ephemeral local registry proxy, running npm through it, relaying findings,
//! and cleaning up the proxy process when npm exits.

use crate::output::escape_terminal_text;
use serde_json::Value;
use std::io::{self, BufRead, BufReader, Read};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const PROXY_BINARY_NAME: &str = "remnant-npm-registry-proxy";
const PROXY_READY_TIMEOUT: Duration = Duration::from_secs(5);
const PROXY_READY_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// The result of a completed `install` command. `remnant install` is a
/// transparent wrapper around `npm install` — its exit code is npm's own
/// exit code, not a Remnant-specific policy verdict.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct InstallOutcome {
    npm_exit_code: i32,
}

impl InstallOutcome {
    pub fn exit_code(self) -> i32 {
        self.npm_exit_code
    }
}

#[derive(Debug)]
pub enum InstallError {
    ProxyBinaryNotFound,
    ProxySpawnFailed(io::Error),
    ProxyPortSelectionFailed(io::Error),
    ProxyWaitFailed(io::Error),
    ProxyExitedBeforeReady { exit_status: Option<ExitStatus> },
    ProxyDidNotBecomeReady,
    NpmBinaryNotFound,
    NpmSpawnFailed(io::Error),
}

impl InstallError {
    pub fn exit_code(&self) -> i32 {
        1
    }
}

pub fn format_error_summary(error: &InstallError) -> Vec<String> {
    match error {
        InstallError::ProxyBinaryNotFound => vec![
            format!("error: `{PROXY_BINARY_NAME}` was not found on PATH"),
            String::from("remnant install requires the proxy binary to be built and on PATH."),
            String::from(
                "Build it from integrations/npm-registry-proxy/ in the remnant repository.",
            ),
        ],
        InstallError::ProxySpawnFailed(source) => {
            vec![format!(
                "error: failed to start {PROXY_BINARY_NAME}: {source}"
            )]
        }
        InstallError::ProxyPortSelectionFailed(source) => {
            vec![format!("error: failed to select a local port: {source}")]
        }
        InstallError::ProxyWaitFailed(source) => vec![format!(
            "error: failed waiting for {PROXY_BINARY_NAME} to exit: {source}"
        )],
        InstallError::ProxyExitedBeforeReady { exit_status } => {
            let exit_status = exit_status
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| String::from("unknown exit status"));

            vec![format!(
                "error: {PROXY_BINARY_NAME} exited before becoming ready ({exit_status})"
            )]
        }
        InstallError::ProxyDidNotBecomeReady => vec![format!(
            "error: {PROXY_BINARY_NAME} did not become ready within {PROXY_READY_TIMEOUT:?}"
        )],
        InstallError::NpmBinaryNotFound => {
            vec![String::from("error: `npm` was not found on PATH")]
        }
        InstallError::NpmSpawnFailed(source) => {
            vec![format!("error: failed to start npm: {source}")]
        }
    }
}

/// Runs `npm install` (or `npm <npm_args>`) through an ephemeral local proxy
/// instance. `audit` selects the proxy's `REMNANT_PROXY_MODE`: `enforce`
/// (default, blocking) when `false`, `audit` (non-blocking, findings still
/// reported) when `true`.
pub fn run(audit: bool, npm_args: Vec<String>) -> Result<InstallOutcome, InstallError> {
    let port = select_ephemeral_port().map_err(InstallError::ProxyPortSelectionFailed)?;
    let mut proxy_child = spawn_proxy(port, audit)?;

    let stdout_handle = proxy_child
        .stdout
        .take()
        .map(|stdout| spawn_line_relay_thread(stdout, summarize_audit_line));
    let stderr_handle = proxy_child.stderr.take().map(|stderr| {
        spawn_line_relay_thread(stderr, |line: &str| Some(format!("remnant-proxy: {line}")))
    });

    if let Err(error) = wait_for_proxy_ready(&mut proxy_child, port) {
        let _ = proxy_child.kill();
        let _ = proxy_child.wait();
        join_relay_threads(stdout_handle, stderr_handle);
        return Err(error);
    }

    let npm_exit_status = run_npm(port, build_npm_install_args(npm_args));

    let _ = proxy_child.kill();
    let _ = proxy_child.wait();
    join_relay_threads(stdout_handle, stderr_handle);

    let npm_exit_status = npm_exit_status?;

    Ok(InstallOutcome {
        npm_exit_code: npm_exit_status.code().unwrap_or(1),
    })
}

fn build_npm_install_args(npm_args: Vec<String>) -> Vec<String> {
    let mut args = vec![String::from("install")];
    args.extend(npm_args);
    args
}

fn select_ephemeral_port() -> io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);

    Ok(port)
}

fn spawn_proxy(port: u16, audit: bool) -> Result<Child, InstallError> {
    let proxy_address = format!("127.0.0.1:{port}");
    let proxy_mode = if audit { "audit" } else { "enforce" };

    // Decision 0047 accepts PATH resolution for the interim distribution posture.
    Command::new(PROXY_BINARY_NAME)
        .env("REMNANT_PROXY_ORIGIN", format!("http://{proxy_address}"))
        .env("REMNANT_PROXY_LISTEN_ADDR", &proxy_address)
        .env("REMNANT_ALLOW_INSECURE_ORIGIN", "1")
        .env("REMNANT_PROXY_MODE", proxy_mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                InstallError::ProxyBinaryNotFound
            } else {
                InstallError::ProxySpawnFailed(error)
            }
        })
}

fn wait_for_proxy_ready(proxy_child: &mut Child, port: u16) -> Result<(), InstallError> {
    let started_at = Instant::now();

    loop {
        match proxy_child.try_wait() {
            Ok(Some(exit_status)) => {
                return Err(InstallError::ProxyExitedBeforeReady {
                    exit_status: Some(exit_status),
                });
            }
            Ok(None) => {}
            Err(error) => return Err(InstallError::ProxyWaitFailed(error)),
        }

        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }

        if started_at.elapsed() >= PROXY_READY_TIMEOUT {
            return Err(InstallError::ProxyDidNotBecomeReady);
        }

        thread::sleep(PROXY_READY_POLL_INTERVAL);
    }
}

fn run_npm(port: u16, npm_args: Vec<String>) -> Result<ExitStatus, InstallError> {
    // Decision 0047 accepts PATH resolution for the interim distribution posture.
    Command::new("npm")
        .args(&npm_args)
        .env("npm_config_registry", format!("http://127.0.0.1:{port}"))
        .status()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                InstallError::NpmBinaryNotFound
            } else {
                InstallError::NpmSpawnFailed(error)
            }
        })
}

fn spawn_line_relay_thread<R, F>(reader: R, format_line: F) -> JoinHandle<()>
where
    R: Read + Send + 'static,
    F: Fn(&str) -> Option<String> + Send + 'static,
{
    thread::spawn(move || {
        let reader = BufReader::new(reader);

        for line in reader.lines().map_while(Result::ok) {
            if let Some(formatted_line) = format_line(&line) {
                println!("{formatted_line}");
            }
        }
    })
}

fn join_relay_threads(
    stdout_handle: Option<JoinHandle<()>>,
    stderr_handle: Option<JoinHandle<()>>,
) {
    if let Some(handle) = stdout_handle {
        let _ = handle.join();
    }

    if let Some(handle) = stderr_handle {
        let _ = handle.join();
    }
}

fn summarize_audit_line(line: &str) -> Option<String> {
    let record: Value = serde_json::from_str(line).ok()?;
    let response_category = record.get("responseCategory")?.as_str()?;
    let enforced = record.get("enforced")?.as_bool()?;
    let package_name = escape_terminal_text(record.get("packageName")?.as_str()?);
    let version = escape_terminal_text(record.get("version")?.as_str()?);
    let finding_ids = record
        .get("findingIds")
        .and_then(Value::as_array)
        .map(|finding_ids| {
            finding_ids
                .iter()
                .filter_map(Value::as_str)
                .map(escape_terminal_text)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    if response_category == "admitted" {
        return None;
    }

    if enforced {
        Some(format!(
            "remnant: blocked {package_name}@{version}: {response_category} [{finding_ids}]"
        ))
    } else {
        Some(format!(
            "remnant: audit - {package_name}@{version} would have blocked: {response_category} [{finding_ids}]"
        ))
    }
}

#[cfg(test)]
mod tests;
