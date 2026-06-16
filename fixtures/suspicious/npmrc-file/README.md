# npmrc File Fixture

## Category

Suspicious

## Purpose

Validates that Remnant flags `package/.npmrc` as a suspicious artifact file using deterministic archive path inspection.

The fixture does not rely on `.npmrc` contents for detection. The current rule is path-based only.

## Expected Result

- package metadata parsing: pass
- install-script policy: pass
- suspicious-file policy: fail with `suspicious-file-detected`
- expected exit behavior when packaged as a tarball: `2`

## Safety Notes

This fixture contains package metadata and a small inert `.npmrc` file. It does not execute code, perform network access, or attempt host persistence.
