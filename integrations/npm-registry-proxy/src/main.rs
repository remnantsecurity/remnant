mod admission;
mod artifact;
mod audit;
mod inspection;
mod output;
mod package_name;
mod server;
mod upstream;

use std::process::ExitCode;

use server::{AppState, build_router};
use upstream::UpstreamFetcher;

const PROXY_LISTEN_ADDR: &str = "127.0.0.1:4873";
const PROXY_ORIGIN: &str = "http://localhost:4873";

fn capture_remnant_version() -> String {
    std::process::Command::new("remnant")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| String::from("unknown"))
}

#[tokio::main]
async fn main() -> ExitCode {
    let fetcher = match UpstreamFetcher::from_env() {
        Ok(fetcher) => fetcher,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };

    let remnant_version = capture_remnant_version();
    let state = AppState::new(fetcher, PROXY_ORIGIN, remnant_version);
    let app = build_router(state);

    let listener = match tokio::net::TcpListener::bind(PROXY_LISTEN_ADDR).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("error: failed to bind {PROXY_LISTEN_ADDR}: {error}");
            return ExitCode::from(1);
        }
    };

    eprintln!("listening on {PROXY_LISTEN_ADDR}");

    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("error: {error}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}
