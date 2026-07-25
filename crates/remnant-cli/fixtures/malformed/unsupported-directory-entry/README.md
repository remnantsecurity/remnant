# Unsupported Directory Entry Fixture

## Category

Malformed

## Purpose

Validates that Remnant recognizes and skips a directory tar entry during archive traversal without returning it as an archive entry.

## Expected Result

- archive traversal: pass with no returned archive entries
- full archive inspection: fail with `PackageJsonMissing` because the fixture does not contain `package/package.json`
- package metadata parsing: not_evaluated
- install-script policy: not_evaluated
- expected exit behavior: `1`

## Safety Notes

This fixture is synthetic, minimal, and safe to inspect. It does not execute code, perform network access, or attempt host persistence.
