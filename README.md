# Remnant

Remnant is a deterministic artifact admission tool. Today, that means npm `.tgz` packages.

The artifact is the source of truth. Remnant inspects the bytes, archive structure, and package metadata that would actually enter your build. It does not decide trust from package popularity, maintainer reputation, download counts, opaque risk scores, or hidden vendor intelligence.

Remnant's goal is simple:

> Make package admission reproducible, explainable, and safe to automate.

## Why Remnant Exists

This project started from direct exposure to modern developer tooling attack surfaces, not a theoretical supply-chain scenario.

Modern JavaScript builds routinely admit third-party package artifacts into developer machines, CI runners, and production build pipelines. Those artifacts may contain install hooks, unusual archive structures, unsafe paths, malformed metadata, or dependency declarations that change how code enters the build.

Many tools answer supply-chain risk with broad scoring systems or platform-specific intelligence. A risk score describes how a package compares inside someone else's model — it does not show whether _this exact artifact_ satisfies reproducible admission rules, and you cannot reproduce it yourself.

Remnant starts from a narrower rule:

> If a package artifact cannot pass reproducible local inspection, it should not enter the build.

## What Remnant Does

Remnant currently inspects npm `.tgz` package artifacts locally and reports deterministic admission results.

It focuses on:

- read-only npm tarball intake;
- deterministic archive traversal;
- archive path safety validation;
- explicit archive and metadata resource limits;
- `package/package.json` inspection without archive extraction;
- deterministic package metadata parsing;
- install hook detection;
- suspicious file detection;
- bounded dependency metadata parsing;
- deterministic dependency policy checks;
- human-readable and JSON output;
- CI-friendly exit codes.

## What Remnant Does Not Do

Remnant intentionally does not:

- execute package-controlled code;
- extract package archives during inspection;
- follow package-controlled symlinks or hardlinks;
- require network access for local inspection;
- use package popularity as a trust input;
- use maintainer reputation as a trust input;
- rely on opaque reputation or risk scores;
- send package data to a hosted service.

This is not because ecosystem intelligence is useless. It is because admission decisions should be reproducible from explicit evidence.

## Trust Model

Remnant treats package artifacts as untrusted input.

A package should pass because the artifact satisfies documented checks, not because it is popular, familiar, or produced by a historically trusted maintainer.

The local inspection model is built around these principles:

1. Deterministic behavior.
2. Explicit trust boundaries.
3. Bounded parsing of untrusted input.
4. No implicit package execution.
5. Explainable policy failures.
6. Reproducible output for humans and automation.

## Current Policy Checks

Remnant currently evaluates a strict baseline policy after archive and metadata parsing succeeds.

Current policy checks include:

- rejecting install lifecycle hooks;
- rejecting the suspicious archive path `package/.npmrc`;
- rejecting local `file:` dependency specifiers.

Policy checks use already-validated archive paths and package metadata. They do not execute package code or inspect source behavior dynamically.

## Installation

Install the Remnant CLI from crates.io:

```bash
cargo install remnant-cli
```

The crates.io package is named `remnant-cli`; the installed command is `remnant`.

## Usage

Inspect an npm package artifact with human-readable output:

```bash
remnant inspect example.tgz
```

Emit machine-readable JSON output for CI and scripts:

```bash
remnant inspect --json example.tgz
```

During local development from source:

```bash
cargo run -- inspect example.tgz
cargo run -- inspect --json example.tgz
```

The repository is a Cargo workspace. The CLI package lives in `crates/remnant-cli`, while the installed binary remains `remnant`.

## GitHub Actions

Remnant includes a composite GitHub Action for CI admission checks. The initial public action builds Remnant from the tagged action repository source with Cargo and then runs `remnant inspect`; it does not download npm packages, execute package-controlled code, or use a hosted analysis service.

Use it after your workflow has produced or obtained the `.tgz` artifact you want to inspect:

```yaml
- name: Inspect npm package artifact with Remnant
  uses: remnantsecurity/remnant/.github/actions/remnant-inspect@v0.1.0
  with:
    artifact: path/to/package.tgz
    json: "true"
```

Replace `v0.1.0` with the release tag you intend to trust. Pinning to a full commit SHA is also supported by GitHub Actions and may be preferable for stricter CI supply-chain control.

The action preserves Remnant's deterministic exit-code behavior: `0` for pass, `1` for inspection or parsing errors, and `2` for policy failure. GitHub Actions treats non-zero exit codes as failed steps by default.

## Example

`remnant inspect` never extracts an archive to evaluate it — every entry path is validated up front, before package metadata or policy is even evaluated.

For an artifact containing an unsafe archive entry path (for example, a parent-directory traversal like `../../etc/passwd`), inspection stops immediately:

```text
error: inspect failed
error kind: archive
error message: archive entry path is unsafe: ../../etc/passwd
exit code: 1
```

The archive is rejected before a single byte is written to disk — a deterministic, explainable rejection instead of a heuristic risk score. The same failure is reported as structured JSON via `--json`.

## Exit Codes

`remnant inspect` uses deterministic exit codes:

| Exit code | Meaning                                                                                              |
| --------: | ---------------------------------------------------------------------------------------------------- |
|       `0` | Inspection completed and all evaluated policy checks passed.                                         |
|       `1` | Inspection could not complete because of CLI input, filesystem, archive, or package metadata errors. |
|       `2` | Inspection completed, but one or more evaluated policy checks failed.                                |

This makes Remnant suitable for CI admission workflows where malformed artifacts and policy failures need different handling.

## Development

Remnant is written in Rust and keeps the CLI entrypoint thin. Parser, archive, package metadata, policy, and output behavior live in focused modules so security-sensitive logic remains reviewable.

Repository layout:

```text
Cargo.toml                    # workspace root
crates/remnant-cli/           # crates.io package; installs the remnant binary
crates/remnant-cli/fixtures/  # inert package fixture source material
.github/                      # CI workflows and local composite actions
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for development setup, validation commands, DCO sign-off requirements, fixture safety expectations, and contribution guidance.

## License

Remnant is licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
