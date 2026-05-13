# Zed xpls Vibe

Local Zed dev extension for validating the first runnable `vibe-xpls` milestone.

This fork intentionally avoids the original `up-xpls` extension id, the `up` CLI fallback, and the `VIBE_XPLS_BIN` environment override. It starts the local milestone language server directly:

```text
<temporary-vibe-xpls-binary> serve
```

## Requirements

- Zed
- Rust installed with `rustup` for local development
- A `vibe-xpls` binary built at `<temporary-vibe-xpls-binary>`

Build the binary from the milestone worktree:

```bash
cd <local-vibe-xpls-worktree>
go build -o <temporary-vibe-xpls-binary> ./cmd/vibe-xpls
<temporary-vibe-xpls-binary> --version
```

Expected version output:

```text
vibe-xpls v0.0.1
```

## Usage

Open a worktree that has a root `crossplane.yaml` or `upbound.yaml`, then install this repository with `zed: install dev extension`.

The extension keeps Zed's native YAML support enabled for ordinary YAML and adds a `Crossplane YAML` language for:

- `crossplane.yaml`
- `crossplane.yml`
- files mapped to `Crossplane YAML` with Zed `file_types`, such as `*-composition.yaml`, `composition.yaml`, and `*-definition.yaml`

`zed-xpls-vibe` runs for `Crossplane YAML` files in Crossplane package worktrees.

## Syntax Highlighting

`Crossplane YAML` uses Go-template highlighting for `{{ ... }}` actions and injects YAML highlighting into surrounding template text. This is intended for Crossplane `function-go-templating` inline templates where the block scalar emits YAML.

The mixed YAML/template case is best-effort. Template actions remain highlighted, and plain generated YAML text is injected into the YAML parser, but some YAML constructs can still look imperfect when a scalar, list item, or indentation level is split by `{{ ... }}` actions.

Zed extension `path_suffixes` can match exact filenames and dot-delimited suffixes, but not glob-style names like `*-composition.yaml`. Zed's language `first_line_pattern` also cannot override the built-in YAML `.yaml` suffix match, so broad `apiVersion: ...crossplane.io/...` content detection is not reliable for YAML files.

Add a file type mapping to your Zed settings for Crossplane Composition and XRD naming conventions:

```jsonc
{
  "file_types": {
    "Crossplane YAML": [
      "**/*-composition.yaml",
      "**/*-composition.yml",
      "**/composition.yaml",
      "**/composition.yml",
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
https://github.com/io41/zed-xpls-vibe
```

## Troubleshooting

If Zed does not start this server, first confirm that the original `up-xpls` extension is uninstalled or disabled, then install this repository as a dev extension.

If Zed logs show that the worktree is not trusted, trust the fixture/package worktree in Zed and reopen it. Zed will not start language servers for untrusted worktrees.

If Zed logs show `<temporary-vibe-xpls-binary>` starting but diagnostics, hover, or completion are absent, check `<temporary-vibe-xpls-binary> --version` and run the protocol smoke tests from the `vibe-xpls` milestone worktree.

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
