mod archive;
mod commands;
mod output;
mod package_json;
mod policy;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "remnant")]
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
}

fn main() {
    let cli = Cli::parse();

    let (result, json_output) = match cli.command {
        Commands::Inspect { json, path } => {
            let output_format = if json {
                commands::inspect::InspectOutputFormat::Json
            } else {
                commands::inspect::InspectOutputFormat::Human
            };

            (commands::inspect::run(path, output_format), json)
        }
    };

    match result {
        Ok(outcome) => {
            let exit_code = outcome.exit_code();

            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
        Err(error) => {
            let exit_code = error.exit_code();

            if !json_output {
                for line in commands::inspect::format_error_summary(&error) {
                    eprintln!("{line}");
                }
            }

            process::exit(exit_code);
        }
    }
}
