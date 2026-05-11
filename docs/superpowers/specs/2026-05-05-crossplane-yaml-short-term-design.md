# Crossplane YAML Short-Term Design Spec

**Status:** Active short-term spec.

**Workspace:** `<local-zed-up-xpls-repo>`

**Goal:** Make the Zed extension install reliably and provide pragmatic Crossplane YAML authoring support using existing grammar and language-server components.

## Context

The extension currently targets Crossplane package authoring in Zed. It starts `up xpls serve --verbose` for Crossplane package worktrees and adds a `Crossplane YAML` language for files that commonly contain Crossplane package metadata, XRDs, and Compositions.

The immediate feature need is editor usability for Go templates embedded in Crossplane Composition YAML, especially `function-go-templating` inline templates. The immediate defect is Zed failing to install the dev extension with:

```text
failed to compile grammar 'go_template'

Caused by:
    wasm-ld: error: symbol exported via --export not found: tree_sitter_go_template
```

The root parser in `ngalaiko/tree-sitter-go-template` exports `tree_sitter_gotmpl`, so the Zed grammar id must be `gotmpl`, not `go_template`.

The baseline implementation is complete as of extension `0.0.6`. The next short-term increment is tracked in `docs/superpowers/specs/2026-05-05-crossplane-yaml-template-highlighting-design.md` and focuses on fixture-driven improvements for mixed generated YAML plus Crossplane Go-template actions.

## Recommended Approach

Use a Crossplane-specific Zed language named `Crossplane YAML` backed by the root `gotmpl` Tree-sitter grammar from `ngalaiko/tree-sitter-go-template`.

Keep `Crossplane YAML` distinct from native `YAML` and from `Helm`:

- Native YAML should remain untouched for ordinary Kubernetes and configuration files.
- Helm should not be used because Crossplane templates do not have Helm chart semantics.
- `gotmpl` should be used because Crossplane `function-go-templating` uses normal Go template syntax plus Sprig and Crossplane helper functions.

Use Tree-sitter injections to parse the non-template text as YAML.

## Scope

In scope:

- Rename the Zed grammar id from `go_template` to `gotmpl`.
- Keep the visible language name `Crossplane YAML`.
- Keep automatic detection for:
  - exact package metadata filenames `crossplane.yaml` and `crossplane.yml`
- Use Zed user `file_types` globs for Crossplane Compositions and XRDs named `*-composition.yaml`, `*-composition.yml`, `*-definition.yaml`, or `*-definition.yml`.
- Expand highlight queries for Go template, Sprig, and Crossplane `function-go-templating` helpers.
- Keep `up-xpls` attached to `Crossplane YAML`.
- Document stale `xpls` diagnostics as an upstream server behavior when `up xpls serve` exits before publishing a clearing diagnostic set.

Out of scope:

- Do not fork or patch `up xpls`.
- Do not use the Helm dialect as the grammar for Crossplane files.
- Do not build a custom Crossplane parser.
- Do not build a new language server in the short term.
- Do not attempt semantic completion for `.observed`, `.desired`, `.context`, `.extraResources`, provider schemas, or XRD-derived fields.

## Grammar and Highlighting

`extension.toml` should use:

```toml
[grammars.gotmpl]
repository = "https://github.com/ngalaiko/tree-sitter-go-template"
rev = "aa71f63de226c5592dfbfc1f29949522d7c95fac"
```

`languages/crossplane-yaml/config.toml` should use:

```toml
name = "Crossplane YAML"
grammar = "gotmpl"
path_suffixes = ["crossplane.yaml", "crossplane.yml"]
```

Zed `path_suffixes` do not support glob-style suffixes like `*-composition.yaml`. Internally, a suffix such as `longer.rs` matches `foo.longer.rs` because Zed checks for `.longer.rs`, but `-composition.yaml` would be checked as `.-composition.yaml` and therefore will not match `xtopic-composition.yaml`.

A broad `first_line_pattern` such as `apiVersion:.*\\.crossplane\\.io/` is not a reliable replacement. Zed only evaluates it when no path suffix has already matched, so the built-in YAML `.yaml` match prevents content detection from selecting `Crossplane YAML` for normal YAML files. The short-term implementation therefore documents and uses `file_types` globs for Composition and XRD filenames.

`languages/crossplane-yaml/injections.scm` should inject YAML into `text` nodes:

```scheme
((text) @content
    (#set! "language" "yaml")
    (#set! "combined"))
```

Keep this as the safe baseline for nested YAML in Go templates. Injecting YAML into the parent template node or enabling `injection.include-children` would pull template actions into the YAML parse and can degrade `{{ ... }}` highlighting. A future experiment may test `yaml_no_injection_text` for list-marker-only gaps, but it should not replace the `text` injection without fixtures that cover malformed comments, trim markers, and list-heavy templates.

`languages/crossplane-yaml/highlights.scm` should keep generic Go template highlighting and include Crossplane helpers as built-ins:

- `randomChoice`
- `toYaml`
- `fromYaml`
- `getResourceCondition`
- `getComposedResource`
- `getComposedConnectionDetails`
- `getCompositeResource`
- `getExtraResources`
- `getExtraResourcesFromContext`
- `setResourceNameAnnotation`
- `include`

## Diagnostics

`up-xpls` remains the Crossplane package diagnostics provider.

The extension only launches `up xpls serve --verbose`; it does not own diagnostic production or clearing. If `up xpls` publishes a diagnostic and then exits or panics, Zed may keep the old diagnostic until the server restarts and publishes a newer diagnostic set for the same URI.

## Testing

Automated verification:

```bash
cargo fmt --check
cargo test
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
python3 -c 'import tomllib; tomllib.load(open("extension.toml", "rb")); tomllib.load(open("languages/crossplane-yaml/config.toml", "rb")); print("toml ok")'
git diff --check
```

Tree-sitter verification:

```bash
tree-sitter parse fixtures/crossplane-package/api/xsetup-composition.yaml
tree-sitter query languages/crossplane-yaml/highlights.scm fixtures/crossplane-package/api/xsetup-composition.yaml
```

Manual Zed verification:

- Install this repository with `zed: install dev extension`.
- Confirm no grammar compile error appears.
- Open `fixtures/crossplane-package/api/xsetup-composition.yaml`.
- Confirm the language is `Crossplane YAML`.
- Open a real package Composition such as `api/xtopic-composition.yaml`.
- Confirm it is selected as `Crossplane YAML` through the Zed `file_types` mapping.
- Confirm Go template expressions highlight inside the inline template.
- Confirm `up-xpls` starts in a Crossplane package worktree.

## Acceptance Criteria

- Zed installs the dev extension without grammar compile failure.
- `extension.toml` uses grammar id `gotmpl`.
- `Crossplane YAML` files still use YAML injection for non-template content.
- Crossplane/Sprig helper names are highlighted as functions or built-ins.
- `up-xpls` still attaches only to `Crossplane YAML`.
- README explains the relationship between stale diagnostics and `up xpls` process exits.
