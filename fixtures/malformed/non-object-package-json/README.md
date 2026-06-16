# Non-Object Package JSON Fixture

## Category

Malformed

## Purpose

Validates that Remnant rejects `package.json` when the top-level JSON value is not an object.

## Expected Result

- package metadata parsing: fail with `TopLevelIsNotObject`
- install-script policy: not evaluated because metadata parsing fails
- expected exit behavior when packaged as a tarball: `1`

## Safety Notes

This fixture contains only malformed package metadata and no executable package code.
