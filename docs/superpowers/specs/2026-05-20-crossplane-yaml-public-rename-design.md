# Crossplane YAML Public Rename Design

## Goal

Rename the public Zed extension from the pre-public validation identity
`zed-xpls-vibe` to `crossplane-yaml` before publishing it through the Zed
extension registry.

The rename is a clean break. No public users depend on the old extension id or
settings key, so the implementation should not add compatibility fallback for
`lsp.zed-xpls-vibe.*`.

## Background

The extension started as a local validation fork for `vibe-xpls`. It now has a
public repository, release automation, Crossplane YAML syntax highlighting, and
a pinned `vibe-xpls` language-server download policy.

The current public identity is still tied to the validation fork:

- extension id: `zed-xpls-vibe`
- extension name: `Zed xpls Vibe`
- language server id: `zed-xpls-vibe`
- settings key: `lsp.zed-xpls-vibe.*`
- repository: `https://github.com/io41/zed-xpls-vibe`

Zed extension ids are effectively permanent after registry publication. The
published id should describe the user-facing language support rather than the
temporary validation fork.

## Decisions

Use `crossplane-yaml` as the public Zed extension id and GitHub repository
name.

Keep the visible Zed language label as `Crossplane YAML`.

Use `Crossplane YAML` as the public extension display name and language-server
display name in `extension.toml`. The extension id and language-server id should
both be `crossplane-yaml`.

Keep `vibe-xpls` as the language-server binary name and Go module. This rename
does not rename the server project.

Use a clean settings key change:

```jsonc
{
  "lsp": {
    "crossplane-yaml": {
      "binary": {
        "path": "/path/to/vibe-xpls",
        "arguments": ["serve"]
      }
    }
  }
}
```

Do not read or document `lsp.zed-xpls-vibe.*` as a fallback. Users who installed
the dev extension before publication can update their local settings manually.

## Scope

The implementation should rename current public identity references:

- `extension.toml` id, name, repository URL, and language server table:
  - `id = "crossplane-yaml"`
  - `name = "Crossplane YAML"`
  - `repository = "https://github.com/io41/crossplane-yaml"`
  - `[language_servers.crossplane-yaml]`
  - language-server `name = "Crossplane YAML"`
- `Cargo.toml` package name and the resulting `Cargo.lock` package entry.
- Rust `LANGUAGE_SERVER_ID`, internal extension type name, and tests.
- User-facing error text and settings hints.
- README title, examples, troubleshooting, and Zed registry publishing notes.
- `AGENTS.md` guardrails so future agents preserve `crossplane-yaml`.
- Release Please package name and related release metadata.
- `.github/workflows/dev-build.yml` artifact name.
- Local repository remote URL after the GitHub repository is renamed.

The implementation should not change:

- The visible language label `Crossplane YAML`.
- The language grammar directory `languages/crossplane-yaml`.
- Syntax highlighting behavior.
- Resolver order or pinned `vibe-xpls` version policy.
- The `vibe-xpls` binary name.
- The default language-server argument `serve`.

Historical docs may keep old references when they are explicitly about the old
validation fork or past plans. Active docs and future-facing instructions should
use the new public name.

The same-day download/version policy spec is an active policy document and
should be updated to use `crossplane-yaml` for settings keys, language-server
id, and repository references:

```text
docs/superpowers/specs/2026-05-20-vibe-xpls-download-and-version-policy-design.md
```

Completed implementation plans can remain historical unless they are copied into
new instructions or otherwise reused as future-facing guidance.

## Preconditions

Before renaming the GitHub repository or changing the implementation, verify the
new id is not already published in the Zed registry:

```bash
gh api repos/zed-industries/extensions/contents/extensions.toml --jq '.content' \
  | base64 --decode \
  | rg -n '^\[crossplane-yaml\]|crossplane-yaml|zed-xpls-vibe'
```

Expected: no output.

Also verify the old public identity has not shipped as a release or registry
entry:

```bash
git tag --list
gh release list --repo io41/zed-xpls-vibe --limit 20
gh pr view 1 --repo io41/zed-xpls-vibe --json state,mergedAt,headRefName,title
```

Expected:

- no local tags for this repository
- no GitHub releases for `io41/zed-xpls-vibe`
- Release Please PR #1 is open, unmerged, and still tied to the old identity

If any old-id tag, GitHub release, or Zed registry entry exists, stop and
re-evaluate the clean-break assumption before proceeding.

## GitHub Repository Rename

Rename the GitHub repository from:

```text
io41/zed-xpls-vibe
```

to:

```text
io41/crossplane-yaml
```

After the GitHub rename, update the local `origin` remote to:

```text
https://github.com/io41/crossplane-yaml.git
```

GitHub will usually redirect the old URL, but future docs and registry
instructions should use the canonical new URL.

## Release Automation

Update Release Please so future releases are for package `crossplane-yaml`.
This includes `release-please-config.json` and any manifest or generated release
metadata that records the package identity.

The existing Release Please PR for `zed-xpls-vibe` should not be merged as-is.
After the rename lands, close or supersede the old PR if it remains tied to the
old package identity. Let Release Please create a fresh release PR for
`crossplane-yaml`.

## Zed Registry Publishing

The Zed registry PR should target the new identity:

```bash
git submodule add https://github.com/io41/crossplane-yaml.git extensions/crossplane-yaml
pnpm sort-extensions
```

The `extensions.toml` entry should use:

```toml
[crossplane-yaml]
submodule = "extensions/crossplane-yaml"
version = "<matching extension.toml version>"
```

## Validation

Local verification should include:

- TOML parse check for `extension.toml` and `languages/crossplane-yaml/config.toml`.
- `cargo fmt --check`.
- `cargo test`.
- `cargo build --target wasm32-wasip2`.
- Grep checks that active code, public docs, release automation, and workflow
  artifact names no longer use `zed-xpls-vibe`.
- GitHub repo visibility and remote URL checks after the rename.

Manual Zed validation should install this repository as a dev extension after
the rename and confirm:

- The extension appears as `Crossplane YAML`.
- The `Crossplane YAML` language is still available.
- A Crossplane YAML file still gets the same highlighting behavior.
- The language server starts under id `crossplane-yaml`.
- `lsp.crossplane-yaml.binary.path` works as the explicit binary override.
