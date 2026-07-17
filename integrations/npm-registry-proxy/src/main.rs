mod admission;
mod artifact;
mod audit;
mod config;
mod inspection;
mod output;
mod package_name;
mod server;
mod upstream;

use std::process::ExitCode;

use server::{AppState, build_router};
use upstream::UpstreamFetcher;

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

fn capture_commit_sha() -> String {
    std::env::var("REMNANT_BUILD_COMMIT_SHA")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| String::from("unknown"))
}

#[tokio::main]
async fn main() -> ExitCode {
    let config = match config::load_proxy_config() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };
    eprintln!("proxy origin: {}", config.proxy_origin);

    let fetcher = match UpstreamFetcher::from_env() {
        Ok(fetcher) => fetcher,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };

    let remnant_version = capture_remnant_version();
    let commit_sha = capture_commit_sha();
    let state = AppState::new(fetcher, &config.proxy_origin, remnant_version, commit_sha);
    let app = build_router(state);

    let listener = match tokio::net::TcpListener::bind(&config.listen_addr).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("error: failed to bind {}: {error}", config.listen_addr);
            return ExitCode::from(1);
        }
    };

    eprintln!("listening on {}", config.listen_addr);

    if let Err(error) = axum::serve(listener, app).await {
        eprintln!("error: {error}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}
