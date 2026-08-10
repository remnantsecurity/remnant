mod archive;
mod commands;
mod output;
mod package_json;
mod policy;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "remnant", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Inspect {
        #[arg(long)]
        json: bool,
        path: PathBuf,
    },
    Install {
        #[arg(long, conflicts_with = "dry_run")]
        accept_risk: bool,
        #[arg(long, conflicts_with = "accept_risk")]
        dry_run: bool,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        npm_args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { json, path } => run_inspect(json, path),
        Commands::Install {
            accept_risk,
            dry_run,
            npm_args,
        } => run_install(accept_risk, dry_run, npm_args),
    }
}

fn run_inspect(json: bool, path: PathBuf) {
    let output_format = if json {
        commands::inspect::InspectOutputFormat::Json
    } else {
        commands::inspect::InspectOutputFormat::Human
    };

    match commands::inspect::run(path, output_format) {
        Ok(outcome) => {
            let exit_code = outcome.exit_code();

            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
        Err(error) => {
            let exit_code = error.exit_code();

            if !json {
                for line in commands::inspect::format_error_summary(&error) {
                    eprintln!("{line}");
                }
            }

            process::exit(exit_code);
        }
    }
}

fn run_install(accept_risk: bool, dry_run: bool, npm_args: Vec<String>) {
    match commands::install::run(accept_risk, dry_run, npm_args) {
        Ok(outcome) => {
            let exit_code = outcome.exit_code();

            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
        Err(error) => {
            for line in commands::install::format_error_summary(&error) {
                eprintln!("{line}");
            }

            process::exit(error.exit_code());
        }
    }
}
