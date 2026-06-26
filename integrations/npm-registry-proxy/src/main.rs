mod output;
mod upstream;

use std::env;
use std::process::ExitCode;

use output::escape_for_terminal;
use upstream::UpstreamFetcher;

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = env::args();
    let program_name = args
        .next()
        .unwrap_or_else(|| String::from("remnant-npm-registry-proxy"));

    let Some(package_name) = args.next() else {
        eprintln!("usage: {program_name} <package-name>");
        return ExitCode::from(1);
    };

    let fetcher = match UpstreamFetcher::from_env() {
        Ok(fetcher) => fetcher,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };

    match fetcher.fetch_abbreviated_packument(&package_name).await {
        Ok(packument) => {
            println!("status: {}", packument.status_code);
            println!("response byte length: {}", packument.bytes.len());
            println!(
                "first 200 bytes: {}",
                escape_for_terminal(first_response_bytes(&packument.bytes, 200))
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn first_response_bytes(bytes: &[u8], limit: usize) -> &[u8] {
    let end = bytes.len().min(limit);
    &bytes[..end]
}
