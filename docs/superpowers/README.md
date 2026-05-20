# Crossplane YAML Planning Docs

Status: no active plans or implementation specs.

This directory keeps only current planning context and concise decision records.
Completed task-by-task plans and superseded pre-public specs were removed because
they had stale names, unchecked historical task lists, and implementation details
that are no longer useful. Git history remains the archive for those documents.

## Current State

- Public Zed extension id: `crossplane-yaml`.
- Public language server id: `crossplane-yaml`.
- Visible language name: `Crossplane YAML`.
- Grammar: `gotmpl`, from `io41/tree-sitter-go-template`.
- Highlighting: outer Crossplane YAML and generated YAML inside Go-template
  blocks are highlighted through Tree-sitter queries and YAML injections.
- Language server: `vibe-xpls`, started with the default `serve` argument.
- Server resolution order: explicit `lsp.crossplane-yaml.binary.path`, shell
  `PATH`, standard Go bin directories, then the pinned GitHub release.
- Automatic downloads use a direct pinned release asset URL, not the GitHub
  release API.
- Auto-discovered local server binaries must report the pinned server version.
- Explicit user binary overrides are allowed and are not version-gated.
- GitHub Actions do not use `pull_request` or `pull_request_target` triggers.

## Pending Work

There is no active local implementation plan in this directory.

Release Please PRs may be created by normal docs or source changes. Leave them
open until there is a reason to publish a new extension release.

## Marketplace Distribution

Zed marketplace work is tracked separately from local extension functionality.
The extension is usable today as a dev extension by cloning this repository and
installing it through `zed: install dev extension`.

The Zed extension registry PR only affects marketplace discoverability and the
normal `zed: extensions` install flow. It is not a blocker for dev-extension
usage.

## Retained Docs

- `decisions.md` records the current behavior decisions and the small set of
  deferred ideas that still matter.
