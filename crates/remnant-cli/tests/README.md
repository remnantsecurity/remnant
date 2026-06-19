# Integration and End-to-End Tests

This directory contains Cargo integration tests that exercise Remnant through public boundaries.

In this package, `tests/` is used for end-to-end CLI behavior because it is Cargo's conventional integration-test directory and is automatically discovered by `cargo test`.

## Boundary

- `crates/remnant-cli/src/**/tests/` validates internal module behavior through Rust APIs.
- `crates/remnant-cli/tests/` validates user-visible behavior by running the compiled `remnant` binary or other public interfaces.

## Current Coverage

- `inspect_json.rs` runs `remnant inspect --json` against committed fixtures and synthetic test archives.
  - It checks deterministic JSON report fields, exit codes, and stderr behavior for successful inspection, policy failure, and archive intake failure.
- `inspect_human.rs` runs `remnant inspect` without `--json`.
  - It checks structured human-readable stderr formatting for inspection errors.

## Safety Notes

Tests in this directory may construct synthetic `.tgz` artifacts, but they must not execute package-controlled code, extract untrusted archives, contact networks, or depend on real downloaded npm packages.
