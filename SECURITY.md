# Security Policy

Remnant is security-sensitive software. It parses untrusted package artifacts and is intended for use in local and CI admission workflows, so vulnerabilities in Remnant itself should be reported carefully.

## Supported Versions

The current supported security review target is the `main` branch.

Version-specific support will be documented as release channels are established.

## Reporting a Vulnerability

Please do not open a public issue for suspected vulnerabilities in Remnant itself.

Email vulnerability reports to `security@remnantsecurity.dev`. GitHub private vulnerability reporting may also be used if it is enabled for this repository.

If neither email nor private vulnerability reporting is available, open a minimal public issue asking for a maintainer security contact, but do not include exploit details, proof-of-concept artifacts, crash inputs, or other sensitive information in the public issue.

A useful private report includes:

- the affected Remnant version or commit;
- the command or code path involved;
- a short description of the impact;
- whether the issue affects local CLI use, CI use, JSON output, archive parsing, package metadata parsing, policy evaluation, or terminal/report output;
- a minimal reproduction if it can be shared safely;
- whether the reproduction requires a crafted package artifact;
- any suggested fix or mitigation, if known.

Do not send real malware. If a proof of concept requires an npm package artifact, keep it inert and minimal. Prefer simulated package metadata or archive structure over executable harmful content.

## What to Report

Please report suspected vulnerabilities such as:

- archive path validation bypasses;
- path traversal or unsafe filesystem behavior;
- symlink or hardlink handling bugs;
- archive extraction behavior if any is ever introduced unintentionally;
- parser crashes or panics on untrusted input;
- unbounded memory allocation or decompression behavior;
- archive or package metadata resource-limit bypasses;
- terminal, log, or JSON output escaping issues involving attacker-controlled package data;
- nondeterministic policy or report behavior with security impact;
- accidental network access in local inspection paths;
- behavior that executes or enables execution of package-controlled code.

## What Not to Report Here

This process is for vulnerabilities in Remnant itself.

Please do not use private vulnerability reporting for:

- a package that Remnant correctly rejects;
- a package that seems suspicious but does not demonstrate a Remnant bug;
- requests for new policy rules;
- ecosystem intelligence or reputation questions;
- dependency vulnerability reports already covered by public RustSec advisories unless Remnant needs a specific mitigation.

Those topics can use normal public issues, as long as they do not include exploit details for a Remnant vulnerability.

## Disclosure and Fix Process

Remnant aims to handle vulnerability reports with care and transparency.

Expected process:

1. A maintainer reviews the private report.
2. The issue is scoped and reproduced when possible.
3. A fix is developed privately when public details would increase risk.
4. Tests or fixtures are added when they can safely preserve the security boundary.
5. A release or public advisory is prepared when appropriate.
6. Public details are shared after a fix or mitigation is available.

Security fixes should preserve Remnant's core principles: deterministic behavior, explicit trust boundaries, bounded parsing, no package execution, and explainable failure modes.

## Safe Handling of Test Artifacts

Remnant's tests and fixtures should not contain real malware.

Security reproductions should be reduced to inert artifacts whenever possible, such as:

- malformed archive paths;
- oversized metadata;
- duplicate archive entries;
- unsupported tar entry types;
- non-executable package metadata that exercises parser or policy behavior.

If an artifact may be harmful, do not attach it publicly.
