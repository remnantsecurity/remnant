# remnant-cli

`remnant-cli` is the crates.io package for the Remnant command-line tool.

It installs the `remnant` binary:

```bash
cargo install remnant-cli
```

Inspect a single npm `.tgz` artifact locally, without extracting the archive, executing package code, or sending package data to a hosted service:

```bash
remnant inspect package.tgz
```

Or gate a real `npm install`: resolve the dependency tree, fetch and inspect every resolved package, and only proceed if everything clears:

```bash
remnant install
```

For full usage, exit codes, policy rules, resource limits, known limitations, GitHub Action usage, contribution guidance, and the security disclosure process, see the repository README:

<https://github.com/remnantsecurity/remnant>

## License

Licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

at your option.
