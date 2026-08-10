# remnant-core

`remnant-core` is a shared library for fetching package artifacts and verifying their integrity, developed as part of [Remnant](https://github.com/remnantsecurity/remnant).

It provides:

- `UpstreamFetcher`: fetches abbreviated packument metadata and tarball bytes from an upstream registry, with explicit size and timeout limits.
- `verify_sha512_integrity`: verifies fetched bytes against a declared integrity hash.

Today, `remnant-core` is primarily consumed by [`remnant-cli`](https://crates.io/crates/remnant-cli), where it powers `remnant install`'s in-process fetch-and-verify pipeline. It isn't yet a documented, general-purpose public API; expect its surface to change as more consumers are built against it.

For project overview, architecture, and contribution guidance, see the repository README:

<https://github.com/remnantsecurity/remnant>

## License

Licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
