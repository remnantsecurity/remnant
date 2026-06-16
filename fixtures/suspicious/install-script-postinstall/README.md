# Install Script Postinstall Fixture

## Category

Suspicious

## Purpose

Models a package that declares a `postinstall` hook so Remnant can test deterministic install-script policy failure behavior.

## Expected Result

- package metadata parsing: pass
- install-hook detection: `postinstall`
- install-script policy: fail with `install-scripts-disallowed`
- expected exit behavior when packaged as a tarball: `2`

## Safety Notes

The script command is inert fixture text for metadata inspection. Remnant must never execute package-controlled scripts during inspection.
