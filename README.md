# Up xpls for Zed

Adds Crossplane package diagnostics and Composition template highlighting to Zed by starting the `up xpls serve --verbose` language server for Crossplane YAML files.

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

The extension keeps Zed's native YAML support enabled for ordinary YAML and adds a `Crossplane YAML` language for:

- `crossplane.yaml`
- `crossplane.yml`
- files mapped to `Crossplane YAML` with Zed `file_types`, such as `*-composition.yaml` and `*-definition.yaml`

`up-xpls` runs for `Crossplane YAML` files in Crossplane package worktrees.

## Syntax Highlighting

`Crossplane YAML` uses Go-template highlighting for `{{ ... }}` actions and injects YAML highlighting into the surrounding template text. This is intended for `function-go-templating` inline templates in Crossplane Compositions.

Zed extension `path_suffixes` can match exact filenames and dot-delimited suffixes, but not glob-style names like `*-composition.yaml`. Zed's language `first_line_pattern` also cannot override the built-in YAML `.yaml` suffix match, so broad `apiVersion: ...crossplane.io/...` content detection is not reliable for YAML files.

Add a file type mapping to your Zed settings for Crossplane Composition and XRD naming conventions:

```jsonc
{
  "file_types": {
    "Crossplane YAML": [
      "**/*-composition.yaml",
      "**/*-composition.yml",
      "**/*-definition.yaml",
      "**/*-definition.yml",
      "**/crossplane.yaml",
      "**/crossplane.yml"
    ]
  }
}
```

## Repository

```text
https://github.com/io41/zed-up-xpls-vibe
```

## Troubleshooting

If `up` cannot be found, start Zed from a shell where `up xpls serve --help` works.

If diagnostics remain after fixing a file and running Zed's Refresh Diagnostics command, check whether `up-xpls` is still running. `up xpls` publishes diagnostics to Zed; stale diagnostics are only cleared when the language server publishes a newer empty diagnostic set for the same file.

If `up xpls serve` exits or panics, Zed has nothing new to replace the old diagnostics with.

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
