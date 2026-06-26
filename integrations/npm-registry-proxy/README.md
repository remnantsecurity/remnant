# Remnant npm Registry Proxy Prototype

This directory contains an experimental prototype for an npm-compatible registry
proxy. It is not production-ready.

The prototype is intended to sit between package managers and an upstream npm
registry. Package managers such as npm, yarn, pnpm, and bun can be configured to
use a custom registry URL. The proxy will eventually intercept those install
requests, fetch metadata and package tarballs from upstream, run `remnant inspect`
against each tarball before serving it, and block or admit the artifact based on
the inspection result.

This package is intentionally a standalone Cargo project rather than a member of
the root Remnant workspace. Its async runtime, HTTP client, and future proxy
dependencies are isolated from `remnant-cli` and its local artifact inspection
dependency set.

`Cargo.lock` may include optional dependency metadata from upstream crates that
is not active in the compiled feature graph. For example, reqwest's optional
HTTP/3 dependencies are locked by Cargo but are not compiled by this prototype
because the `http3` feature is not enabled. Verify active dependencies with
`cargo tree -e features`.

Current implemented scope:

- fetch abbreviated install metadata from a configured upstream registry;
- request npm's install-v1 packument representation;
- reject redirects instead of following them;
- enforce connection, total fetch, and response body byte limits;
- validate that the response body is a JSON object.

Out of scope for this step:

- HTTP server behavior;
- metadata URL rewriting;
- tarball fetching;
- invoking `remnant inspect`;
- package admission or blocking responses.

Before any future server step returns fetch errors to clients or external logs,
error formatting must avoid exposing upstream URLs or HTTP client internals.
