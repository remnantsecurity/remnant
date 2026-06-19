# Minimal Package Fixture

## Category

Benign

## Purpose

Validates that Remnant accepts a minimal package metadata shape with a non-empty `name` and `version` and no lifecycle scripts.

## Expected Result

- package metadata parsing: pass
- install-script policy: pass
- expected exit behavior when packaged as a tarball: `0`

## Safety Notes

This fixture contains only package metadata and no executable package code.
