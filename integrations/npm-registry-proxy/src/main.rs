mod artifact;
mod output;
mod package_name;
mod upstream;

use std::env;
use std::process::ExitCode;

use artifact::rewrite_packument_tarball_urls;
use output::escape_for_terminal;
use package_name::ValidatedPackageName;
use upstream::UpstreamFetcher;

const PROTOTYPE_PROXY_ORIGIN: &str = "http://localhost:4873";

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = env::args();
    let program_name = args
        .next()
        .unwrap_or_else(|| String::from("remnant-npm-registry-proxy"));

    let Some(raw_package_name) = args.next() else {
        eprintln!("usage: {program_name} <package-name>");
        return ExitCode::from(1);
    };

    let package_name = match ValidatedPackageName::parse(raw_package_name) {
        Ok(package_name) => package_name,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
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
            let rewritten_packument =
                match rewrite_packument_tarball_urls(&packument.bytes, PROTOTYPE_PROXY_ORIGIN) {
                    Ok(rewritten_packument) => rewritten_packument,
                    Err(error) => {
                        eprintln!("error: {error}");
                        return ExitCode::from(1);
                    }
                };

            println!("status: {}", packument.status_code);
            println!(
                "rewritten response byte length: {}",
                rewritten_packument.bytes.len()
            );
            println!(
                "artifact mapping count: {}",
                rewritten_packument.artifacts.len()
            );
            println!(
                "first 200 bytes: {}",
                escape_for_terminal(first_response_bytes(&rewritten_packument.bytes, 200))
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
