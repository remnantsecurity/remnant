# Name Too Long Fixture

## Category

Malformed

## Purpose

Validates that Remnant rejects package `name` values longer than the current deterministic byte-length limit.

## Expected Result

- package metadata parsing: fail with `NameIsTooLong`
- install-script policy: not evaluated because metadata parsing fails
- expected exit behavior when packaged as a tarball: `1`

## Safety Notes

This fixture contains only package metadata and no executable package code.
