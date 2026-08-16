# Package-name lexical-similarity evaluation

This non-publishable evaluation package measures one reproducible relationship between two
validated npm package names: restricted Damerau-Levenshtein (optimal string
alignment) distance exactly one. It recognizes one ASCII-byte insertion,
deletion, substitution, or adjacent transposition. It does not infer malicious
intent, classify a package as a typosquat, normalize names, or affect Remnant's
CLI and policy behavior.

The committed datasets are synthetic placeholders used only to test the
harness's input, analysis, and report paths. This version rejects any manifest
whose `is_synthetic_placeholder` field is not `true`. Support for real registry
samples and cited historical incidents is separate follow-up work.

## Running the harness

Run from the repository workspace root:

```bash
cargo run --locked --release -p remnant-evaluation-package-name-similarity
```

Optional inputs are available through:

```text
--pairs <JSONL_PATH>
--npm-sample <JSONL_PATH>
--output <JSON_PATH>
```

Each input must have a sibling manifest whose name replaces `.jsonl` with
`.manifest.json`. The package is a workspace member for reproducible builds but
is not a default member and cannot be published.

## Matching semantics

The npm sample is used as both the candidate stream and reference set. A
matching unordered pair therefore produces two directed rows: one for each
candidate perspective. For unequal lengths, one row is labeled `insertion` and
the reverse row is labeled `deletion`.

`candidate_frequency_per_1000` counts distinct candidate names with at least
one outbound match. Matches are sorted by candidate name and then reference
name. Analytical fields are deterministic for fixed inputs. The separately
nested `benchmark.runtime_ns` field is an observational wall-clock measurement
and is not reproducible evidence.

## Dataset schemas

Pair records contain:

```json
{"candidate_name":"lodahs","reference_name":"lodash"}
```

They must be sorted by `(candidate_name, reference_name)` byte order. Exact
duplicate pairs are rejected, while one candidate may have multiple distinct
references.

Npm sample records contain:

```json
{"name":"lodash"}
```

They must be sorted by name byte order and contain no duplicate names.

Each manifest requires:

- `dataset_file`
- `source`
- `retrieved`
- `selection_rule`
- `cutoff`
- `content_sha256`
- `sort_rule`
- `license`
- `includes_scoped_and_unscoped`
- `is_synthetic_placeholder`

The loader validates the manifest schema, lowercase hexadecimal digest format,
synthetic-only marker, and dataset basename association. It does not recompute
SHA-256 at runtime. The report therefore names this value
`declared_content_sha256`. Dataset preparation must calculate the digest from
the exact committed JSONL bytes.

## Evaluation harness safety limits

| Limit | Value |
|---|---:|
| Manifest bytes | 8 KiB |
| JSONL line bytes | 1 KiB |
| Dataset bytes | 2 MiB |
| Dataset records | 1,000 |
| Comparisons | 250,000 |
| Emitted matches | 50,000 |

These limits bound this research tool; they are not production policy
thresholds. The record limit is deliberately small because length-bucketed
self-comparison approaches quadratic work when many names share the same byte
length. Raising a limit for real-corpus work requires algorithmic and
performance review, not a one-line constant change.

Peak memory remains a manual measurement. Wrap the release invocation with
`/usr/bin/time -l` on macOS or `/usr/bin/time -v` on Linux. The harness does not
invoke either platform-specific command itself.
