# Up xpls for Zed

Adds Crossplane package diagnostics to Zed by starting the `up xpls serve --verbose` language server for YAML files in Crossplane package worktrees.

## Requirements

- Zed
- Rust installed with `rustup` for local development
- Upbound `up` CLI available on `PATH`

Install `up`:

```bash
brew install upbound/tap/up
```

or:

```bash
curl -sL https://cli.upbound.io | sh
```

Verify:

```bash
up version
up xpls serve --help
```

## Usage

Open a worktree that has a root `crossplane.yaml` or `upbound.yaml`, then install this repository with `zed: install dev extension`.

The extension keeps Zed's native YAML support enabled and adds `up xpls serve --verbose` as a Crossplane-specific language server.

## Repository

```text
https://github.com/io41/zed-up-xpls-vibe
```

## Troubleshooting

If `up` cannot be found, start Zed from a shell where `up xpls serve --help` works.

If the Zed log shows `starting language server process` for `up xpls serve --verbose`, the extension attached successfully. Diagnostics can still disappear if the `up` language server process exits. With `up v0.48.0`, function dependency validation can panic while checking `crossplane.yaml`; the stack trace includes `VersionValidator` or `TypeValidator` under `internal/xpkg/snapshot/meta.go`. That is an `up xpls` server failure rather than a Zed extension startup failure.

For extension logs, run Zed with:

```bash
zed --foreground
```

or use `zed: open log`.

If the WASM build reports that `wasm32-wasip2` is missing even after installing the target, make sure Cargo is using the same rustup toolchain that owns the target:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

On this machine, `/opt/homebrew/bin/cargo` is Homebrew Rust and cannot compile Zed dev extensions. Put `/opt/homebrew/opt/rustup/bin` before `/opt/homebrew/bin` when launching Zed.

## Development

```bash
cargo test
cargo build --target wasm32-wasip2
```
