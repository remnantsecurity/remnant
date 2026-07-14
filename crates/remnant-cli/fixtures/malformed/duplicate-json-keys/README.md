**Category**: malformed
**ID**: duplicate-json-keys

## Description

`package.json` contains a duplicate top-level key (`name` appears twice). Remnant
rejects such packages before serde_json parsing to prevent parser differential
risk: serde_json keeps the last value for duplicate keys while other JSON parsers
keep the first, so the effective metadata visible to Remnant could differ from
what a downstream consumer sees.

## Expected Result

- `package_json`: fail
- `package_json_error`: DuplicateKeys
- `install_script_policy`: not_evaluated
- `exit_code`: 1

## Safety

- `executes_code`: false
- `network_access`: false
- `host_persistence`: false
