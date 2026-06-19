# Contributing to Remnant

Thank you for considering a contribution to Remnant.

Remnant is security-sensitive infrastructure. The project is intentionally narrow, deterministic, and local-first. Contributions should make artifact inspection safer, more reproducible, easier to audit, or easier to use in CI without expanding trust boundaries implicitly.

## Project Principles

Remnant prioritizes:

1. deterministic behavior;
2. explicit trust boundaries;
3. reproducible inspection;
4. bounded parsing of untrusted input;
5. explainable policy enforcement;
6. auditability;
7. maintainability over feature velocity.

Avoid contributions that introduce:

- package code execution;
- archive extraction without an explicit design decision;
- network calls in local inspection paths;
- telemetry;
- opaque scoring;
- reputation-based trust decisions;
- broad framework abstractions;
- hidden side effects;
- nondeterministic output ordering.

## Good First Contributions

Useful early contributions include:

- clear documentation improvements;
- small test coverage improvements;
- fixture metadata corrections;
- deterministic error-message improvements;
- parser boundary tests;
- CI and developer-experience improvements;
- narrowly scoped policy-rule discussions backed by concrete examples.

For code changes, prefer small focused pull requests over broad rewrites.

## Development Setup

Remnant is a Rust workspace. Install the Rust toolchain with `rustup` if needed, then clone the repository and run the local validation commands below. The CLI package lives in `crates/remnant-cli`, and the workspace root is configured so common Cargo commands can run from the repository root.

Common local development commands:

```bash
cargo fmt
cargo test
cargo clippy -- -D warnings
```

CI parity checks:

```bash
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Run the CLI locally from source:

```bash
cargo run -- inspect example.tgz
cargo run -- inspect --json example.tgz
```

## Contribution Requirements

### Keep changes narrow

Each contribution should have a clear purpose. Avoid unrelated refactors in the same pull request as behavior changes.

### Preserve deterministic behavior

Output ordering, error behavior, policy findings, and report fields should be stable and explainable. If a change affects output, update or add tests that lock in the intended behavior.

### Treat artifacts as hostile input

Package archives and package metadata are untrusted. Parser changes should consider:

- path traversal;
- symlinks and hardlinks;
- unsupported archive entry types;
- duplicate paths after normalization;
- attacker-controlled allocation;
- archive entry count and size limits;
- decompressed stream limits;
- malformed JSON;
- terminal/report escaping.

### Do not execute package-controlled code

Remnant's local inspection model does not execute package scripts or package-controlled files. Do not add execution-based behavior without a documented design decision and explicit maintainer approval.

### Do not add network behavior to local inspection

Local artifact inspection should not require network access or hosted services. Future hosted or registry workflows must preserve the local-first trust boundary.

### Add tests for behavior changes

Behavior changes should include tests when practical. Prefer testing through natural module or CLI boundaries rather than exposing internals solely for tests.

## Fixtures

Fixtures should be safe, minimal, and deterministic.

Do not commit real malware. Malicious behavior should be simulated with inert package metadata or archive structures that exercise Remnant's parser and policy boundaries without introducing executable harmful content.

When adding fixtures, prefer clear names and metadata that explain the expected behavior.

For security reproductions and detailed inert artifact handling, see [`SECURITY.md`](SECURITY.md#safe-handling-of-test-artifacts).

## Policy Rules

Policy rules should be narrow, explainable, and deterministic.

Before proposing a new policy rule, be prepared to explain:

- what exact artifact condition is being checked;
- why the condition is security-relevant;
- whether the rule can be evaluated without executing package code;
- what false positives are expected;
- what accepted and rejected fixtures should exist;
- whether the rule changes an existing trust boundary.

Avoid broad heuristic rules unless they are backed by concrete ecosystem evidence and a documented decision.

## Commit Sign-Off

Remnant uses Developer Certificate of Origin (DCO) sign-off for contribution provenance.

By signing off a commit, you certify that you have the right to submit the contribution under Remnant's license terms.

Add a sign-off line to each commit:

```text
Signed-off-by: Your Name <your.email@example.com>
```

You can create signed-off commits with:

```bash
git commit -s
```

Contributions are licensed under Remnant's dual `MIT OR Apache-2.0` license.

## Security Issues

Please do not open public issues for suspected vulnerabilities in Remnant itself.

Follow the disclosure process in [`SECURITY.md`](SECURITY.md). Avoid publishing exploit details, proof-of-concept artifacts, crash inputs, or other sensitive information publicly before a fix or mitigation is available.

## Review Expectations

Security-sensitive changes may receive detailed review. Maintainers may ask for:

- smaller diffs;
- additional tests;
- clearer error messages;
- fixture coverage;
- documentation updates;
- a decision record for security-sensitive thresholds, parser behavior, dependency choices, or policy semantics.

This is intentional. Remnant earns trust by making important behavior explicit and reviewable.
