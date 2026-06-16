# Scripts Not Object Fixture

## Category

Malformed

## Purpose

Validates that Remnant rejects a `scripts` field that is present but not a JSON object.

## Expected Result

- package metadata parsing: fail with `ScriptsIsNotObject`
- install-script policy: not evaluated because metadata parsing fails
- expected exit behavior when packaged as a tarball: `1`

## Safety Notes

This fixture contains only malformed package metadata and no executable package code.
