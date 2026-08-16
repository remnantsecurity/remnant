mod dataset;
mod report;
mod similarity;

#[cfg(test)]
mod tests;

use clap::Parser;
use dataset::{load_manifest, load_npm_sample, load_pairs};
use report::{analyze, assemble_report};
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

const DEFAULT_PAIRS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/datasets/synthetic-pairs.jsonl"
);
const DEFAULT_NPM_SAMPLE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/datasets/npm-sample-synthetic-placeholder.jsonl"
);

#[derive(Parser)]
#[command(name = "package-name-similarity-evaluation")]
struct Args {
    #[arg(long, default_value = DEFAULT_PAIRS_PATH)]
    pairs: PathBuf,
    #[arg(long, default_value = DEFAULT_NPM_SAMPLE_PATH)]
    npm_sample: PathBuf,
    #[arg(long)]
    output: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let pairs =
        load_pairs(&args.pairs).unwrap_or_else(|error| fail(&format!("loading pairs: {error}")));
    let npm_sample = load_npm_sample(&args.npm_sample)
        .unwrap_or_else(|error| fail(&format!("loading npm sample: {error}")));
    let pairs_manifest = load_manifest(&manifest_path_for(&args.pairs), &args.pairs)
        .unwrap_or_else(|error| fail(&format!("loading pairs manifest: {error}")));
    let npm_sample_manifest = load_manifest(&manifest_path_for(&args.npm_sample), &args.npm_sample)
        .unwrap_or_else(|error| fail(&format!("loading npm sample manifest: {error}")));

    let started_at = Instant::now();
    let analysis =
        analyze(&pairs, &npm_sample).unwrap_or_else(|error| fail(&format!("analyzing: {error}")));
    let runtime_ns = started_at.elapsed().as_nanos();

    let report = assemble_report(
        &analysis,
        &pairs_manifest,
        &npm_sample_manifest,
        pairs.len(),
        npm_sample.len(),
        runtime_ns,
    );
    let rendered = serde_json::to_string_pretty(&report).expect("report must serialize");

    match args.output {
        Some(path) => std::fs::write(&path, rendered).unwrap_or_else(|error| {
            fail(&format!("writing output to {}: {error}", path.display()))
        }),
        None => println!("{rendered}"),
    }
}

fn manifest_path_for(dataset_path: &Path) -> PathBuf {
    let mut manifest_path = dataset_path.to_path_buf();
    manifest_path.set_extension("manifest.json");
    manifest_path
}

fn fail(message: &str) -> ! {
    eprintln!("package-name-similarity-evaluation: {message}");
    process::exit(1);
}
