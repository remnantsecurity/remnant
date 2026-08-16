# Evaluations

This directory contains reproducible, non-publishable evaluations of proposed
Remnant capabilities. Evaluation code measures a narrowly stated question; it
is not part of the shipped CLI, does not define production policy, and must not
be called from code under `crates/`.

Each evaluation owns its implementation, tests, documentation, synthetic
fixtures, and permitted datasets in one directory. Its README must state:

- the question being evaluated;
- the observable behavior being measured;
- input provenance and redistribution constraints;
- resource limits and reproducibility commands;
- which outputs are deterministic and which are observational; and
- what work remains before any behavior can be promoted into Remnant.

Evaluation packages must set `publish = false`. Synthetic fixtures must be
clearly distinguished from sourced ecosystem data, and preliminary alert
frequency must not be described as a false-positive rate without a labeled
denominator. Network collection, production integration, policy changes, and
promotion into a crate under `crates/` require their own explicit review.

Evaluation packages are excluded from the workspace default members and
standing CI. When changing an evaluation, run the validation commands
documented in that evaluation's README. If an evaluation becomes continuously
maintained, add appropriately scoped CI as part of that transition.

Current evaluation areas:

- [`typosquat-signals/`](typosquat-signals/) — evaluates reproducible signals
  that may identify names for typosquatting review without treating any one
  signal as proof of malicious intent.
