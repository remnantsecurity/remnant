# Local Dependency Specifier Fixture

## Category

Suspicious

## Purpose

Validates that Remnant flags dependency version specifiers that use npm's local `file:` dependency form.

## Expected Result

- package metadata parsing: pass
- local dependency specifier policy: fail with `local-dependency-specifier-disallowed`
- expected exit behavior when packaged as a tarball: `2`

## Safety Notes

This fixture contains only package metadata. It does not include the referenced local path and does not execute package code.
