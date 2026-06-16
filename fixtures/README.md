# Remnant Fixture Corpus

This directory contains committed, minimal, safe fixtures for deterministic Remnant tests.

These fixtures are not real downloaded npm package tarballs. Most fixtures are minimal source trees, and malformed archive fixtures may include small synthetic `artifact.tgz` files when raw archive bytes are required to exercise parser boundaries. Real downloaded packages used for local spot checks belong in the gitignored `artifacts/` directory. Fixtures in this directory should be small, auditable, and designed to exercise one behavior at a time.

## Current Categories

- `benign/` contains packages expected to pass current parsing and policy checks.
- `suspicious/` contains packages that model risky behavior such as install hooks.
- `malformed/` contains packages with invalid or rejected metadata shapes.
- `regression/` contains focused cases that preserve accepted or rejected edge behavior over time.

## Fixture Metadata

Each fixture directory should include:

- `README.md` with human-readable intent and safety notes;
- `fixture.json` with deterministic machine-readable expectations;
- `package/package.json` when the fixture models package metadata;
- `artifact.tgz` when the fixture requires exact archive bytes that cannot be represented as ordinary source files.

The initial `fixture.json` format is intentionally small:

```json
{
  "id": "install-script-postinstall",
  "category": "suspicious",
  "description": "Package metadata declares a postinstall hook.",
  "expected": {
    "package_json": "pass",
    "install_script_policy": "fail",
    "exit_code": 2,
    "policy_findings": ["install-scripts-disallowed"]
  },
  "safety": {
    "executes_code": false,
    "network_access": false,
    "host_persistence": false
  }
}
```

## Rules

Fixtures must:

- avoid real malicious payloads;
- avoid network behavior;
- avoid host persistence behavior;
- stay small and readable;
- document expected behavior;
- be safe to inspect locally and in CI.

Do not commit broad real-world npm package tarballs here. If a real package reveals an edge case, reduce it to a minimal synthetic fixture before committing it. Committed `artifact.tgz` files should be reserved for archive-level behavior that needs exact bytes, such as invalid gzip data, duplicate tar entries, unsafe paths, or unsupported entry types.
