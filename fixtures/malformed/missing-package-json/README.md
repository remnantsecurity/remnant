# Missing Package Json Fixture

## Category

Malformed

## Purpose

Validates that Remnant rejects npm artifacts missing required package metadata.

## Expected Result

- archive inspection: fail with `PackageJsonMissing`
- package metadata parsing: not_evaluated
- install-script policy: not_evaluated
- expected exit behavior: `1`

## Safety Notes

This fixture is synthetic, minimal, and safe to inspect. It does not execute code, perform network access, or attempt host persistence.
