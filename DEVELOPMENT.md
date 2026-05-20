# Development

This document is for maintainers and contributors. End-user setup is covered in [README.md](README.md).

## Local Checks

Run the Rust tests:

```sh
cargo test
```

Build the Zed extension WASM target:

```sh
cargo build --target wasm32-wasip2
```

If Cargo cannot find the `wasm32-wasip2` target, install it with:

```sh
rustup target add wasm32-wasip2
```

On systems with multiple Rust installations, make sure Cargo is using the rustup toolchain that owns the target:

```sh
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

## Language Server

The extension launches the `vibe-xpls` language server with the `serve` argument.

The pinned compatible server version is recorded in `src/resolver.rs`. Bumping it is a deliberate source change and should be released with the extension version that depends on it.

Binary resolution order:

1. `lsp.crossplane-yaml.binary.path`, when configured.
2. `vibe-xpls` on the worktree shell `PATH`.
3. Standard Go bin directories: `GOBIN`, `GOPATH/bin`, and `HOME/go/bin` (`USERPROFILE/go/bin` on Windows).
4. The pinned GitHub release from `io41/vibe-xpls`, downloaded directly on supported release platforms.

Auto-discovered local binaries must report the pinned version. A mismatched local binary is a hard compatibility error so the extension does not silently run an unsupported server.

Use an explicit path only as an expert override for non-standard installs:

```jsonc
{
  "lsp": {
    "crossplane-yaml": {
      "binary": {
        "path": "/absolute/path/to/vibe-xpls",
        "arguments": ["serve"]
      }
    }
  }
}
```

## Releases

This extension uses SemVer and stays on the `v0.x.y` line until maintainers explicitly approve a `v1.0.0` release.

Release Please maintains `CHANGELOG.md` from Conventional Commits and opens release pull requests on merges to `main`.

The extension version in `extension.toml`, `Cargo.toml`, and `.release-please-manifest.json` should remain aligned through Release Please.

## Troubleshooting

If Zed does not start the language server, first confirm that any conflicting earlier development extension is uninstalled or disabled.

If Zed logs show that the worktree is not trusted, trust the worktree in Zed and reopen it. Zed will not start language servers for untrusted worktrees.

For extension logs, run Zed with:

```sh
zed --foreground
```

or use `zed: open log`.

For a local `vibe-xpls` install, check:

```sh
vibe-xpls --version
```

If Zed reports an incompatible auto-discovered local version, update or remove the binary path named in the error. The resolver checks `PATH` before standard Go bin directories and stops on a version mismatch, so installing a pinned Go binary will not help if another `vibe-xpls` earlier on `PATH` still wins.
