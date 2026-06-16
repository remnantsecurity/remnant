# Unsupported Directory Entry Fixture

## Category

Malformed

## Purpose

Validates the strict archive entry-type posture for unsupported tar entry kinds.

## Expected Result

- archive inspection: fail with `ArchiveEntryTypeUnsupported`
- package metadata parsing: not_evaluated
- install-script policy: not_evaluated
- expected exit behavior: `1`

## Safety Notes

This fixture is synthetic, minimal, and safe to inspect. It does not execute code, perform network access, or attempt host persistence.
