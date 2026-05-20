# Zed xpls Vibe

Zed extension for Crossplane package diagnostics and Crossplane YAML highlighting powered by [`vibe-xpls`](https://github.com/io41/vibe-xpls).

## Requirements

- Zed
- network access to download the pinned `vibe-xpls` release on a supported platform, or a compatible local `vibe-xpls` install

With network access on a supported release platform, the extension downloads the pinned language server release directly after local resolution fails. Unsupported release platforms should install a compatible local `vibe-xpls` binary.

Optionally install the pinned language server with Go for offline use or to control the local binary:

```sh
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1
```

Confirm the binary:

```sh
vibe-xpls --version
```

Expected version:

```text
vibe-xpls v0.0.1
```

## Binary Resolution

The extension starts `vibe-xpls serve`.

It resolves the binary in this order:

1. `lsp.zed-xpls-vibe.binary.path`, when configured.
2. `vibe-xpls` on the worktree shell `PATH`.
3. Standard Go bin directories: `GOBIN`, `GOPATH/bin`, and `HOME/go/bin` (`USERPROFILE/go/bin` on Windows).
4. The pinned GitHub release `io41/vibe-xpls@v0.0.1`, downloaded directly on supported release platforms.

No settings are needed when `vibe-xpls` is on `PATH` or installed in a standard Go bin directory and reports the pinned version:

```text
vibe-xpls v0.0.1
```

If an auto-discovered local binary reports any other version, the extension stops with a compatibility error instead of silently running it or falling through to another source.

Use an explicit path only as an expert override for non-standard installs. Compatibility is your responsibility when `binary.path` is configured:

```jsonc
{
  "lsp": {
    "zed-xpls-vibe": {
      "binary": {
        "path": "/absolute/path/to/vibe-xpls",
        "arguments": ["serve"]
      }
    }
  }
}
```

## Usage

Install the extension from Zed once it is published, or use `zed: install dev extension` when developing from this repository.

Open a file classified as `Crossplane YAML`.

The extension keeps Zed's native YAML support enabled for ordinary YAML and adds a `Crossplane YAML` language for:

- `crossplane.yaml`
- `crossplane.yml`
- `upbound.yaml`
- `upbound.yml`
- `composition.yaml`
- `composition.yml`
- `definition.yaml`
- `definition.yml`
- files mapped to `Crossplane YAML` with Zed `file_types`, such as `*-composition.yaml` and `*-definition.yaml`

`zed-xpls-vibe` runs for `Crossplane YAML` files and leaves package detection to the `vibe-xpls` language server. This allows root package, nested package, multi-package, and no-root validation to exercise the same analyzer path.

`Crossplane YAML` uses two-space, space-only indentation to match YAML and avoid Zed's default four-space indentation in this custom language.

## Syntax Highlighting

`Crossplane YAML` uses Go-template highlighting for `{{ ... }}` actions and injects YAML highlighting into surrounding template text. This is intended for Crossplane `function-go-templating` inline templates where the block scalar emits YAML.

The mixed YAML/template case is best-effort. Template actions remain highlighted, and plain generated YAML text is injected into the YAML parser, but some YAML constructs can still look imperfect when a scalar, list item, or indentation level is split by `{{ ... }}` actions.

Zed extension `path_suffixes` can match exact filenames and dot-delimited suffixes, but not glob-style names like `xexample-composition.yaml`. Zed's language `first_line_pattern` also cannot override the built-in YAML `.yaml` suffix match, so broad `apiVersion: ...crossplane.io/...` content detection is not reliable for YAML files.

The extension config covers the exact filenames above. Add a `file_types` mapping to your Zed settings for hyphenated or custom Crossplane Composition and XRD filenames. The `languages` entry is optional with the current extension, but is useful as a local override and documents the intended indentation behavior:

```jsonc
{
  "file_types": {
    "Crossplane YAML": [
      "**/*-composition.yaml",
      "**/*-composition.yml",
      "**/*-definition.yaml",
      "**/*-definition.yml"
    ]
  },
  "languages": {
    "Crossplane YAML": {
      "tab_size": 2,
      "hard_tabs": false
    }
  }
}
```

## Development

```sh
cargo test
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

## Releases

This extension uses SemVer and stays on the `v0.x.y` line until maintainers explicitly approve a `v1.0.0` release.

Release Please maintains `CHANGELOG.md` from Conventional Commits and opens release pull requests on merges to `main`.

The extension pins the `vibe-xpls` language server release in source. Bumping the pinned language server is a deliberate source change, not an automatic lookup.

## Publishing To Zed

Zed registry publication happens through a PR to [`zed-industries/extensions`](https://github.com/zed-industries/extensions).

The extension must be public, licensed, and added as an HTTPS submodule under `extensions/zed-xpls-vibe` with a matching `extensions.toml` version.

## Troubleshooting

If Zed does not start this server, first confirm that the original `up-xpls` extension is uninstalled or disabled.

If Zed logs show that the worktree is not trusted, trust the worktree in Zed and reopen it. Zed will not start language servers for untrusted worktrees.

If Zed logs show `vibe-xpls` starting but diagnostics, hover, or completion are absent, inspect the Zed logs for the resolved binary path and server errors. For a local install, also check:

```sh
vibe-xpls --version
```

If the pinned release download fails, install the pinned language server locally and restart Zed:

```sh
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1
```

If Zed reports an incompatible auto-discovered local `vibe-xpls` version, update or remove the binary path named in the error. The resolver checks `PATH` before standard Go bin directories and stops on a version mismatch, so installing the pinned Go binary will not help if another `vibe-xpls` earlier on `PATH` still wins.

Install the pinned language server:

```sh
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1
vibe-xpls --version
```

The expected output is:

```text
vibe-xpls v0.0.1
```

If another `PATH` entry keeps winning, configure `lsp.zed-xpls-vibe.binary.path` to the pinned binary you want Zed to run.

For extension logs, run Zed with:

```sh
zed --foreground
```

or use `zed: open log`.

If the WASM build reports that `wasm32-wasip2` is missing even after installing the target, make sure Cargo is using the same rustup toolchain that owns the target:

```sh
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

## License

MIT. See [LICENSE](LICENSE).
