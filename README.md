# Remnant

Remnant is a deterministic artifact admission tool. Today, that means npm packages: individual `.tgz` artifacts and full `npm install` dependency trees.

The artifact is the source of truth. Remnant inspects the bytes, archive structure, and package metadata that would actually enter your build. It does not decide trust from package popularity, maintainer reputation, download counts, opaque risk scores, or hidden vendor intelligence.

Remnant's goal is simple:

> Make package admission reproducible, explainable, and safe to automate.

## The Problem

Modern JavaScript builds routinely admit third-party package artifacts into developer machines, CI runners, and production build pipelines. Those artifacts may contain install hooks, unusual archive structures, unsafe paths, malformed metadata, or dependency declarations that change how code enters the build, and by the time you notice, the code has already run.

Many tools answer supply-chain risk with broad scoring systems or platform-specific intelligence. A risk score describes how a package compares inside someone else's model. It doesn't show whether _this exact artifact_ satisfies reproducible admission rules, and you can't reproduce it yourself.

This project started from direct exposure to modern developer tooling attack surfaces, not a theoretical supply-chain scenario.

## What Remnant Does About It

Remnant starts from a narrower rule:

> If a package artifact cannot pass reproducible local inspection, it should not enter the build.

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

Remnant intentionally does not:

- execute package-controlled code;
- extract package archives during inspection;
- follow package-controlled symlinks or hardlinks;
- use package popularity as a trust input;
- use maintainer reputation as a trust input;
- rely on opaque reputation or risk scores;
- send package data to a hosted service.

That's not because ecosystem intelligence is useless. Admission decisions should be reproducible from explicit evidence, and a hosted analysis step works against that.

### Trust Model

Remnant treats package artifacts as untrusted input. A package should pass because the artifact satisfies documented checks, not because it is popular, familiar, or produced by a historically trusted maintainer.

The inspection model is built around these principles:

1. Deterministic behavior.
2. Explicit trust boundaries.
3. Bounded parsing of untrusted input.
4. No implicit package execution.
5. Explainable policy failures.
6. Reproducible output for humans and automation.

### Current Policy Checks

Remnant currently evaluates a strict baseline policy after archive and metadata parsing succeeds:

- rejecting install lifecycle hooks;
- rejecting the suspicious archive path `package/.npmrc`;
- rejecting local `file:` dependency specifiers.

Policy checks use already-validated archive paths and package metadata. They do not execute package code or inspect source behavior dynamically.

### Resource Limits & Archive Safety

Before policy ever runs, every artifact has to clear deterministic, bounded parsing. These checks reject malformed or resource-exhausting input outright, regardless of policy configuration:

| Limit | Value |
|---|--:|
| Archive entries | 10,000 |
| Single archive entry size | 32 MiB |
| Total declared archive size | 256 MiB |
| Decompressed stream read limit | 300 MiB |
| `package/package.json` size | 1 MiB |
| Archive entry path length | 1,024 bytes |
| Package name length | 214 bytes |
| Package version length | 128 bytes |
| Dependency name length | 214 bytes |
| Dependency version specifier length | 512 bytes |
| Dependencies per section | 1,000 |

Alongside those bounds, archive traversal enforces:

- path traversal (`../`), absolute paths, and backslash separators are rejected;
- two entries that normalize to the same logical path are rejected as duplicates;
- symlinks, hard links, and any other non-regular-file entry type are rejected;
- directory entries are accepted structurally (they carry no content of their own), but everything else must be a regular file.

## Current Offerings

Remnant ships two CLI commands and a GitHub Action, all built on the same inspection engine:

- **`remnant inspect`**: inspect a single npm `.tgz` artifact you already have on disk. No network access required.
- **`remnant install`**: a drop-in gate in front of your real `npm install`. It resolves your full dependency tree via `npm` itself, fetches and inspects every resolved package in-process, and only materializes the install (via `npm ci`) if every package clears, or if you've explicitly accepted the risk of proceeding anyway (`--accept-risk`). A `--dry-run` mode reports the same findings without ever running `npm ci` at all.
- **GitHub Action** (`remnant-inspect`): wraps `remnant inspect` for CI, so a malformed or policy-failing artifact fails the build with a deterministic exit code instead of a heuristic risk score.

## Installation

Install the Remnant CLI from crates.io:

```bash
cargo install remnant-cli
```

The crates.io package is named `remnant-cli`; the installed command is `remnant`.

## Usage

### `remnant inspect`

Inspect an npm package artifact with human-readable output:

```bash
remnant inspect example.tgz
```

Emit machine-readable JSON output for CI and scripts:

```bash
remnant inspect --json example.tgz
```

### `remnant install`

`remnant install` requires nothing beyond the `remnant` binary itself and `npm` on `PATH`. No separate proxy process, no configuration.

Reinstall exactly what's already declared, blocking on any non-admitted package (enforce mode, the default):

```bash
remnant install
```

Add a new dependency the same way you would with `npm install <package>`. Remnant resolves it via npm first, then inspects the resulting tree before installing:

```bash
remnant install husky
```

Report findings but proceed anyway, consciously accepting the risk (`npm ci` still runs):

```bash
remnant install --accept-risk
```

Preview findings without installing anything at all (`npm ci` never runs):

```bash
remnant install --dry-run
```

`--accept-risk` and `--dry-run` are mutually exclusive. In enforce mode (the default), a single non-admitted package aborts before `npm ci` ever runs, and nothing gets written to `node_modules`. Under `--accept-risk`, every non-admitted package is reported, but `npm ci` still runs regardless. This does not suppress whatever risk was found; it only tells you about it. Under `--dry-run`, every non-admitted package is reported and `npm ci` never runs at all.

Example enforce-mode output when a package with an install hook is in the tree:

```text
remnant: inspecting 42 package(s)
remnant: blocked some-package@1.2.3: blocked_policy [install-scripts-disallowed]
remnant: analyzed 42 package(s), 41 admitted, 1 blocked
```

The same package under `--accept-risk` or `--dry-run`:

```text
remnant: inspecting 42 package(s)
remnant: flagged some-package@1.2.3: blocked_policy [install-scripts-disallowed]
remnant: analyzed 42 package(s), 41 admitted, 1 flagged
```

### During local development from source

```bash
cargo run -- inspect example.tgz
cargo run -- inspect --json example.tgz
cargo run -- install
cargo run -- install --accept-risk
cargo run -- install --dry-run
```

## GitHub Actions

Remnant includes a composite GitHub Action for CI admission checks. The action builds Remnant from the tagged action repository source with Cargo and then runs `remnant inspect`; it does not download npm packages, execute package-controlled code, or use a hosted analysis service.

Use it after your workflow has produced or obtained the `.tgz` artifact you want to inspect:

```yaml
- name: Inspect npm package artifact with Remnant
  uses: remnantsecurity/remnant/.github/actions/remnant-inspect@v0.1.0
  with:
    artifact: path/to/package.tgz
    json: "true"
```

Replace `v0.1.0` with the release tag you intend to trust. Pinning to a full commit SHA is also supported by GitHub Actions and may be preferable for stricter CI supply-chain control.

## Example

`remnant inspect` never extracts an archive to evaluate it. Every entry path is validated up front, before package metadata or policy is even evaluated.

For an artifact containing an unsafe archive entry path (for example, a parent-directory traversal like `../../etc/passwd`), inspection stops immediately:

```text
error: inspect failed
error kind: archive
error message: archive entry path is unsafe: ../../etc/passwd
exit code: 1
```

The archive is rejected before a single byte is written to disk: a deterministic, explainable rejection instead of a heuristic risk score. The same failure is reported as structured JSON via `--json`.

## Exit Codes

### `remnant inspect`

| Exit code | Meaning |
|--:|---|
| `0` | Inspection completed and all evaluated policy checks passed. |
| `1` | Inspection could not complete because of CLI input, filesystem, archive, or package metadata errors. |
| `2` | Inspection completed, but one or more evaluated policy checks failed. |

### `remnant install`

| Exit code | Meaning |
|--:|---|
| `0` | `npm ci` completed successfully (default enforce mode or `--accept-risk`), or every resolved package would have been admitted under `--dry-run`. |
| `1` | Remnant couldn't complete resolution or inspection at all (npm not on `PATH`, lockfile unreadable or unparseable, upstream registry misconfigured), reported with an `error: ...` line on stderr. This code also covers the underlying `npm install --package-lock-only` process, or `npm ci` under default/`--accept-risk`, exiting non-zero itself, in which case that exit code is passed through unchanged rather than reinterpreted. Consult [npm's own CLI documentation](https://docs.npmjs.com/cli/v10/commands/npm) for what a given npm failure means. |
| `2` | Enforce mode (the default) blocked the install: at least one resolved package did not clear inspection, and `npm ci` never ran. Under `--dry-run`, this instead means at least one resolved package would not have cleared inspection; `npm ci` still never ran. See Verdict Categories below for what each category means. Never returned under `--accept-risk`, which always defers to `npm ci`'s own exit code regardless of findings. |

This makes Remnant suitable for CI admission workflows where malformed artifacts, npm's own failures, and policy failures all need different handling.

## Verdict Categories

Every non-admitted package in `remnant install`'s output is tagged with a category, printed as `remnant: blocked <name>@<version>: <category> [<rule-ids>]` (enforce mode, the default) or `remnant: flagged <name>@<version>: <category> [<rule-ids>]` (`--accept-risk` or `--dry-run`):

| Category | Meaning |
|---|---|
| `blocked_policy` | The package's `package.json` or archive contents tripped one of the policy rules below. The specific rule ID(s) are in the trailing `[...]`. |
| `blocked_integrity` | The fetched tarball's bytes didn't match the version pinned in your lockfile, or no integrity hash was available to check against at all. |
| `blocked_parse` | The tarball failed archive-safety validation (unsafe path, a resource limit exceeded, a duplicate archive entry; see Known Limitations) or its `package.json` failed to parse. |
| `error` | Inspection couldn't complete for a reason unrelated to the package's own content: a network fetch failed, or (for a lockfile entry missing `resolved` metadata) the fallback registry lookup failed. Not a judgment about the package itself; treat it as "try again" rather than "this package is bad." |

### Policy Rules

When the category is `blocked_policy`, the rule ID(s) in `[...]` name exactly which check failed:

| Rule ID | Fails when |
|---|---|
| `install-scripts-disallowed` | The package declares an npm install lifecycle hook (`preinstall`, `install`, or `postinstall`). |
| `suspicious-file-detected` | The archive contains a file at a known-suspicious path (currently: `package/.npmrc`). |
| `local-dependency-specifier-disallowed` | One of the package's own dependency specifiers starts with `file:`. |

## Known Limitations

Remnant is under active development. These are current, known gaps, not permanent design decisions, and are being worked on:

- **Some legitimately-packaged tarballs are currently rejected.** A small number of real, popular npm packages ship a full duplicate copy of every file at two equivalent archive paths (for example, `package/dist/index.js` and `package/./dist/index.js`) as a side effect of their own build tooling. Remnant's archive-safety check treats any two entries that resolve to the same logical path as suspicious, which is correct in general: that same pattern is how a malicious package could smuggle different content past two disagreeing tools. It just has no way yet to tell "harmless exact duplicate" apart from "genuinely different content at a colliding path." An explicit override is in progress. Until then, a package hitting this will fail with `archive entry path is duplicated: <path>`.
- **A bundled dependency's own `package.json` isn't independently policy-evaluated.** When a dependency ships bundled inside its parent's tarball (`inBundle: true` in the lockfile, no independent registry artifact of its own), Remnant verifies and inspects the *parent* tarball as a whole, but doesn't separately evaluate policy against the bundled dependency's own `package.json` (for example, an install hook declared specifically on the bundled sub-package). This mirrors the same accepted boundary for npm workspace members.
- **`remnant install` processes packages sequentially, not concurrently.** This is deliberate (simpler, more predictable behavior) rather than an oversight, but it means large dependency trees take longer than a plain `npm install`. Expect roughly a couple of minutes for a tree in the thousand-package range.

## Development

Remnant is written in Rust and keeps the CLI entrypoint thin. Parser, archive, package metadata, policy, and output behavior live in focused modules so security-sensitive logic remains reviewable.

Repository layout:

```text
Cargo.toml                    # workspace root
crates/remnant-core/          # shared artifact fetch and integrity verification library for Remnant
crates/remnant-cli/           # crates.io package; installs the remnant binary
crates/remnant-cli/fixtures/  # inert package fixture source material
evaluations/                  # non-publishable, reproducible capability evaluations
integrations/                 # standalone experimental integrations
.github/                      # CI workflows and local composite actions
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for development setup, validation commands, DCO sign-off requirements, fixture safety expectations, and contribution guidance.

## License

Remnant is licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
