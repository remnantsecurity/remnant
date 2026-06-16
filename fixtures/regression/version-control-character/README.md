# Version Control Character Fixture

## Category

Regression

## Purpose

Preserves behavior for package versions containing JSON-escaped control characters. The parser currently accepts the value because full version validation is deferred, and terminal output escaping is responsible for rendering control characters safely.

## Expected Result

- package metadata parsing: pass
- install-script policy: pass
- expected exit behavior when packaged as a tarball: `0`

## Safety Notes

This fixture contains only package metadata and no executable package code.
