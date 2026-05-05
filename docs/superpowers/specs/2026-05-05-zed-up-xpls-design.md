# Up xpls Zed Extension Design Spec

**Status:** Draft  
**Date:** 2026-05-05  
**Workspace:** `<local-zed-up-xpls-repo>`

## Goal

Build a Zed extension that adds Crossplane package diagnostics to YAML authoring by launching the `up xpls serve` language server from the user's installed `up` CLI.

The extension should preserve Zed's native YAML experience and add `xpls` as an additional, Crossplane-aware language server for worktrees that are Crossplane packages.

## Verified Context

- This workspace is currently empty and is not a Git repository yet.
- Zed supports extensions as repositories with `extension.toml`; procedural extension behavior is Rust compiled to WebAssembly.
- Zed has native YAML support with tree-sitter YAML and `yaml-language-server`.
- Zed language-server extensions register a server in `extension.toml` and implement `language_server_command` in Rust.
- Zed publishing rules say extensions that provide language support must not ship the language server binary; they should either download it or find it in the user's environment.
- The local machine has `/opt/homebrew/bin/up`, version `v0.48.0`.
- `up xpls serve --help` reports that it runs a server for Crossplane definitions using the Language Server Protocol and supports `--cache` and `--verbose`.
- Upbound's VS Code extension activates in a workspace containing `crossplane.yaml` or `upbound.yaml`, starts `up xpls serve --verbose`, and provides diagnostics through `xpls`.
- The upstream VS Code extension is intentionally thin: it contributes no commands, attaches to file-backed YAML documents, and exposes a small config surface headed by `xpls.up.path`.
- `up v0.48.0` can crash in `up xpls serve` while validating function dependencies in package metadata. When that happens, Zed logs show that the server started, but diagnostics may disappear because the language server process exits.

## Product Behavior

1. A user installs the extension as a dev extension or from the Zed extension registry.
2. The user opens a Crossplane package or Upbound project worktree, identified by a root `crossplane.yaml` or `upbound.yaml`.
3. YAML files keep normal Zed YAML syntax highlighting, formatting behavior, and existing `yaml-language-server` support.
4. The extension starts `up xpls serve --verbose` as an additional language server for YAML in that worktree.
5. `xpls` supplies Crossplane-aware diagnostics for package metadata, XRD schemas, compositions, composed resources, and XRC examples.
6. If `up` is not on the worktree shell `PATH`, Zed shows an actionable language-server startup error telling the user how to install `up`.
7. If the current worktree is not a Crossplane package, the extension does not attempt to validate that workspace with `xpls`.

## Detection Strategy

Use package-level detection for the MVP:

- A worktree is treated as Crossplane-enabled when either root marker succeeds:
  - `crossplane.yaml` contains Crossplane package metadata.
  - `upbound.yaml` contains Upbound project metadata.
- `crossplane.yaml` should be recognized when it contains:
  - `apiVersion: meta.pkg.crossplane.io/...` or `apiVersion: meta.pkg.upbound.io/...`
  - `kind: Configuration`, `Provider`, `Function`, or `AddOn`
- `upbound.yaml` should be recognized when it contains:
  - `apiVersion: meta.dev.upbound.io/...`
  - `kind: Project`
- File-level detection is delegated to `xpls`, because individual YAML files can be compositions, XRDs, package metadata, examples, functions, composed resources, or arbitrary XR/XRC instances.

This deliberately avoids creating a separate `Crossplane YAML` language for the MVP. A custom language based on first-line matching would miss user-defined XR/XRC API groups and could also steal unrelated Kubernetes YAML from Zed's native YAML language.

## Architecture

### `extension.toml`

Defines the extension metadata and an `up-xpls` language server attached to Zed's existing `YAML` language:

```toml
id = "up-xpls"
name = "Up xpls"
version = "0.0.1"
schema_version = 1
authors = ["Tim Kersten"]
description = "Crossplane package diagnostics powered by up xpls"
repository = "https://github.com/io41/zed-up-xpls-vibe"

[language_servers.up-xpls]
name = "Up xpls"
languages = ["YAML"]
```

### `src/lib.rs`

Implements `zed::Extension`:

- `new()` returns a stateless extension.
- `language_server_command()` handles only the `up-xpls` server id.
- It checks whether the worktree is a Crossplane package.
- It resolves `up` via `worktree.which("up")`.
- It launches:

```text
up xpls serve --verbose
```

- It passes `worktree.shell_env()` to preserve the user's normal `PATH`, `HOME`, proxy settings, and Upbound configuration.

### Documentation

`README.md` should explain:

- Install `up`.
- Open a worktree with `crossplane.yaml`.
- Install the extension as a Zed dev extension.
- Use `zed: open log` or `zed --foreground` when troubleshooting.
- `xpls` diagnostics are additive to Zed's native YAML support.

## Error Handling

- Missing `up`: return an error that includes `brew install upbound/tap/up` and `curl -sL https://cli.upbound.io | sh`.
- Missing both root markers: return a quiet, clear error only during development. If this proves noisy in non-Crossplane YAML worktrees, remove the hard error and allow `xpls` to start only after a confirmed package root.
- `up xpls` process exit: document that the extension may be working even when no diagnostics remain visible. Users should check the Zed log for an upstream panic, especially `VersionValidator` or `TypeValidator` stack traces in `internal/xpkg/snapshot/meta.go`.
- Unknown language server id: return an error naming the unsupported id.
- Do not pass `--quiet` or `--silent` to `up xpls serve` in the MVP because language servers use stdout for protocol messages.

## Test Strategy

Automated:

- Unit-test the package-manifest detection helper with Crossplane, Upbound, and non-Crossplane YAML samples.
- Unit-test Upbound project marker detection.
- Unit-test the command argument builder so it always returns `["xpls", "serve", "--verbose"]`.
- Run `cargo test`.
- Run `cargo build --target wasm32-wasip2`.

Manual:

- Install `up` and confirm `up version`.
- Confirm `up xpls serve --help`.
- Install the extension with `zed: install dev extension`.
- Open a fixture Crossplane package or Upbound project containing:
  - root `crossplane.yaml`
  - root `upbound.yaml`, in a separate fixture
  - XRD YAML
  - Composition YAML
  - XR or XRC example YAML
- Introduce a known invalid dependency or schema mismatch and verify Zed receives diagnostics from `up-xpls`.
- Open a normal YAML-only project and confirm the extension does not degrade normal YAML editing.

## Non-Goals

- Do not implement a custom Crossplane YAML parser.
- Do not replace Zed's native YAML language or `yaml-language-server`.
- Do not bundle the `up` binary.
- Do not download `up` automatically in the MVP.
- Do not implement code actions, completion shaping, semantic token styling, or schema generation until diagnostics are proven.

## Future Enhancements

- Optional setting for a custom `up` binary path.
- Optional setting for `xpls` cache path, passed as `--cache=<path>`.
- Optional setting to disable verbose logging. Verbose should be the default because the upstream VS Code extension runs that way and it is useful for diagnosing `xpls` exits.
- Optional support for `xpls.up.path`, mirroring the VS Code extension's custom `up` executable setting if Zed exposes a suitable extension settings API.
- Fallback loose-file support through a `Crossplane YAML` language if Zed adds a clean way to avoid stealing unrelated YAML.
- Download-managed `up` or standalone `xpls` only if Upbound publishes a supported language-server binary separate from the full CLI.

## References

- Zed language extensions: https://zed.dev/docs/extensions/languages
- Zed extension development: https://zed.dev/docs/extensions/developing-extensions
- Zed extension capabilities: https://zed.dev/docs/extensions/capabilities
- Zed YAML language support: https://zed.dev/docs/languages/yaml
- Upbound VS Code extension: https://marketplace.visualstudio.com/items?itemName=Upboundio.upbound
- Upbound VS Code extension source: https://github.com/upbound/vscode-up
- `up xpls` Go package: https://pkg.go.dev/github.com/upbound/up/cmd/up/xpls
- Up CLI module: https://pkg.go.dev/github.com/upbound/up
