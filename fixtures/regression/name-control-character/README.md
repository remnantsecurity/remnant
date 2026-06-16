# Name Control Character Fixture

## Category

Regression

## Purpose

Preserves behavior for package names containing JSON-escaped control characters. The parser currently accepts the value because full npm name validation is deferred, and terminal output escaping is responsible for rendering control characters safely.

## Expected Result

- package metadata parsing: pass
- install-script policy: pass
- expected exit behavior when packaged as a tarball: `0`

## Safety Notes

This fixture contains only package metadata and no executable package code.
