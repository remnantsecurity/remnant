# Duplicate Archive Entry Fixture

## Category

Malformed

## Purpose

Validates that Remnant rejects ambiguous duplicate tar entries deterministically.

## Expected Result

- archive inspection: fail with `ArchiveEntryPathDuplicate`
- package metadata parsing: not_evaluated
- install-script policy: not_evaluated
- expected exit behavior: `1`

## Safety Notes

This fixture is synthetic, minimal, and safe to inspect. It does not execute code, perform network access, or attempt host persistence.
