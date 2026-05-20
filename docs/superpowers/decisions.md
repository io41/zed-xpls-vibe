# Crossplane YAML Decisions

This is a concise replacement for the completed planning specs and execution
plans that existed during the pre-public extension work.

## Extension Identity

The public extension id, package identity, and language server id are
`crossplane-yaml`.

Earlier prototype identities are superseded and should not be reintroduced in
runtime code, user settings examples, release automation, or Zed registry
metadata.

## Language And Highlighting

`Crossplane YAML` is a separate Zed language so Crossplane composition files can
use Go-template-aware highlighting without changing native YAML behavior.

The language uses the `gotmpl` Tree-sitter grammar and injects YAML into
template text. This supports ordinary Crossplane YAML plus generated YAML inside
`function-go-templating` `inline.template` blocks while preserving Go-template
actions such as variables, pipelines, comments, and helper functions.

The extension does not use Helm highlighting. Crossplane Go templates do not
have Helm chart semantics.

## File Detection

Zed extension language configs cannot reliably claim arbitrary `*.yaml` files by
content before the built-in YAML suffix match. The extension therefore matches
known suffixes directly and documents user file-type mappings for project-specific
composition or definition file names.

Do not silently change native YAML behavior for unrelated Kubernetes or
configuration files.

## Generated YAML Visual Contrast

A spike tested whether normal Zed syntax captures could make generated YAML
inside Go-template blocks visually distinct from the outer YAML without asking
users to edit theme settings.

Result: no viable extension-owned contrast was found. The captures were precise,
but common themes did not produce a useful visible distinction. The extension
should not ship custom theme hooks, bundled themes, or required user settings for
this. Revisit only if Zed exposes a reliable extension-owned styling mechanism.

## Language Server Resolution

The extension starts `vibe-xpls` with the default argument `serve`.

Resolution order:

1. `lsp.crossplane-yaml.binary.path`, when explicitly configured.
2. `vibe-xpls` on the worktree shell `PATH`.
3. Standard Go bin directories.
4. The pinned `io41/vibe-xpls` GitHub release.

Auto-discovered local binaries must pass a strict `vibe-xpls --version` check
for the pinned server version. Explicit binary path overrides are not
version-gated because the user has intentionally opted out of managed resolution.

The `--version` probe has no custom watchdog. A hanging `vibe-xpls --version`
is a known residual risk until Zed exposes a process timeout or similar host API.

The extension must not add a default `gh` fallback or GitHub release API lookup
for the managed download path. Managed downloads use the direct pinned release
asset URL.

## Release And CI

Release Please manages extension releases and changelog updates.

CI, Release Please, and development build workflows should run from trusted
events only, such as pushes to `main` or manual dispatch. Do not add
`pull_request` or `pull_request_target` triggers.

## Deferred Work

Future Crossplane-aware authoring may need a semantic language server or proxy
rather than more syntax-query work. Useful future capabilities include:

- completions and hovers for Crossplane helper functions;
- completions for `.observed`, `.desired`, `.context`, and `.extraResources`;
- XRD-derived field completion for composite resources;
- provider-schema-derived completion for composed resources;
- diagnostics that understand Crossplane function request data;
- content-based language selection if Zed adds a hook that can outrank native
  YAML suffix matching;
- checksum verification if Zed exposes downloaded archive bytes or release asset
  digests, or if the extension later owns archive download and extraction.
