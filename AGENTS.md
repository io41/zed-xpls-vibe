# Agent Instructions

This repository is a local validation fork of `zed-up-xpls` for the `vibe-xpls` first runnable milestone.

- Keep the extension id and language server id as `zed-xpls-vibe`; do not change them back to `up-xpls`.
- The extension intentionally launches the local milestone binary at `<temporary-vibe-xpls-binary>` with the single argument `serve`.
- Before Zed validation, rebuild that binary from `<local-vibe-xpls-worktree>` with:
  `go build -o <temporary-vibe-xpls-binary> ./cmd/vibe-xpls`
- Rust tests must preserve the hardcoded binary path and `serve` argument so future agents do not accidentally reintroduce the `VIBE_XPLS_BIN` override or the `up xpls serve` fallback.
- Zed manual validation should install this repository as a dev extension, not the original `up-xpls` extension.
