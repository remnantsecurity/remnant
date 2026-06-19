# remnant-cli

`remnant-cli` is the crates.io package for the Remnant command-line tool.

It installs the `remnant` binary for deterministic npm artifact admission checks:

```bash
cargo install remnant-cli
remnant inspect package.tgz
```

Remnant inspects npm `.tgz` package artifacts locally without extracting archives, executing package code, or sending package data to a hosted service.

For project overview, GitHub Action usage, contribution guidance, and security disclosure process, see the repository README:

<https://github.com/remnantsecurity/remnant>

## License

Licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
