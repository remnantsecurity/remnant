# Remnant

**Deterministic npm artifact admission before installation.**

Remnant is an open-source Rust CLI for inspecting exact npm package artifacts and gating full dependency trees before package code is materialized in `node_modules`. It turns package admission into a reproducible policy decision grounded in artifact bytes, archive structure, package metadata, and lockfile integrity—not popularity, maintainer reputation, download counts, or an opaque risk score.

Rather than asking whether a package _looks trustworthy_, Remnant asks a narrower question that can be answered and audited:

> Can this exact artifact cross this trust boundary under these documented rules?

## Why Remnant Exists

Remnant began with direct exposure to modern developer-tooling attack surfaces and a simple realization: package installation often puts third-party code onto developer machines and CI runners before anyone makes an explicit trust decision about the artifact that arrived.

That artifact may contain install hooks, unsafe archive paths, malformed metadata, unexpected dependency declarations, or content that different tools interpret differently. Once it has been installed—or executed during installation—the most useful security boundary has already been crossed.

Remnant moves the decision earlier:

```text
exact npm artifact
        ↓
bounded, non-executing inspection
        ↓
deterministic artifact facts
        ↓
documented policy rules
        ↓
admit or block before installation
```

The artifact is the source of truth. A package should pass because the bytes that would enter the build satisfy explicit checks—not because the package is popular, familiar, or produced by a historically trusted maintainer.

## What Ships Today

The current release, v0.3.0, provides two CLI commands and a GitHub Action built on the same inspection engine:

- **`remnant inspect`** examines an npm `.tgz` already on disk. It performs read-only, offline inspection and emits human-readable or JSON results with deterministic exit codes.
- **`remnant install`** resolves a complete npm dependency tree, fetches and verifies each independently resolved package artifact, inspects the tree, and runs `npm ci` only after every package is admitted. `--dry-run` reports the same findings without installing; `--accept-risk` makes an explicit decision to continue after findings are reported.
- **GitHub Action** (`remnant-inspect`) applies the same artifact checks in CI.

Before policy evaluation, each inspected artifact must pass bounded archive and metadata parsing. Remnant rejects unsafe paths, duplicate normalized paths, links and unsupported entry types, malformed metadata, and inputs that exceed documented resource limits. During `remnant install`, it also blocks when downloaded bytes do not match the lockfile integrity value or when integrity cannot be established. It does not extract an archive to inspect it or execute package-controlled code.

The current strict policy baseline rejects:

- npm install lifecycle hooks;
- the suspicious archive path `package/.npmrc`; and
- local `file:` dependency specifiers.

Every block names its category, and policy blocks name the rule that caused them. There is no hidden score behind the verdict.

## What Remnant Does Not Claim

Remnant is an admission tool under active development, not a universal malware detector. Passing v0.3.0 means that an artifact passed the documented archive, integrity, metadata, and policy checks in that release. It does **not** prove that the package is benign.

The current release does not:

- execute packages in a behavioral sandbox;
- perform general JavaScript source-behavior analysis;
- replace vulnerability, license, or software-composition analysis;
- establish publisher identity or verify publish provenance;
- infer safety from popularity, reputation, or community behavior;
- use a hosted analysis service or require an account for CLI inspection; or
- detect every malicious package or attack technique.

Those boundaries are deliberate. Security tooling loses credibility when a narrow signal is marketed as proof of safety.

## Where Remnant Is Going

The immediate direction is deeper, demonstrated protection against malicious installation—not a longer feature checklist or a more impressive-looking risk score. New admission rules should be backed by reproducible artifact evidence, evaluated against malicious and benign examples, and shown to stop a defined behavior before installation without hiding uncertainty from the user.

Longer term, npm is the first proving ground for a broader software trust model:

```text
untrusted software → verifiable evidence → explicit policy → explainable decision
```

The artifact type and delivery mechanism can evolve. The trust contract should not: exact inputs, bounded inspection, documented rules, and evidence a user can independently evaluate.

The bet behind Remnant is that software trust should behave more like a build property than a vendor opinion—specific inputs, explicit rules, and reproducible outputs.

## Installation

Install the Remnant CLI from crates.io:

```bash
cargo install remnant-cli --version 0.3.0 --locked
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

Remnant includes a composite GitHub Action for CI admission checks. The action builds Remnant from the pinned action repository source with Cargo and then runs `remnant inspect`; it does not download npm packages, execute package-controlled code, or use a hosted analysis service.

Use it after your workflow has produced or obtained the `.tgz` artifact you want to inspect:

```yaml
- name: Inspect npm package artifact with Remnant
  uses: remnantsecurity/remnant/.github/actions/remnant-inspect@9cf1da0edb9ed7a185a2243791b26b8e8219e61e # v0.3.0
  with:
    artifact: path/to/package.tgz
    json: "true"
```

Always pin the action to a verified, full-length commit SHA. GitHub currently identifies this as the only immutable way to reference an action; a tag or branch can be moved if the repository is compromised. The `# v0.3.0` comment preserves the human-readable release mapping without weakening the pin. When upgrading, verify the new release commit in the official Remnant repository, review the change, and update the SHA and version comment together. See [GitHub's secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use#using-third-party-actions).

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

## Resource Limits & Archive Safety

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
