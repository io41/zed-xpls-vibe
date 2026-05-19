# Agent Instructions

This repository is the Zed extension for `vibe-xpls`.

- Keep the extension id and language server id as `zed-xpls-vibe`; do not change them back to `up-xpls`.
- The extension starts the `vibe-xpls` language server with the default argument `serve`.
- Do not reintroduce the `up xpls serve` fallback or a `VIBE_XPLS_BIN` environment override.
- The public extension resolves `vibe-xpls` in this order: Zed `lsp.zed-xpls-vibe.binary.path`, shell `PATH`, standard Go bin directories, then the pinned `io41/vibe-xpls` GitHub release recorded in the source.
- Rust tests must preserve the extension id, language server id, resolver order, pinned release behavior, and default `serve` argument.
- Local milestone validation with `<temporary-vibe-xpls-binary>` is development-only. If it is needed for a one-off manual check, keep it out of public README usage and do not hardcode it as the production path.
- Zed manual validation should install this repository as a dev extension, not the original `up-xpls` extension.
