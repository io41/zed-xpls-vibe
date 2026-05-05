# Crossplane YAML Template Highlighting Design Spec

**Status:** Draft
**Date:** 2026-05-05
**Workspace:** `<local-zed-up-xpls-repo>`

## Goal

Add syntax highlighting for Go template expressions embedded in Crossplane Composition YAML, especially `function-go-templating` inline templates like:

```yaml
template: | # go
  {{- $xr := .observed.composite.resource -}}
  metadata:
    name: {{ $xr.metadata.name }}
```

The existing `up-xpls` diagnostics should continue to run. This feature is about editor syntax highlighting only.

## Previous Behavior

- The extension registers `up-xpls` against Zed's built-in `YAML` language.
- Zed therefore keeps the buffer language as `YAML`.
- `up xpls` diagnostics appear, including messages such as `package does not depend on function "function-go-templating"`.
- Inline Go template code inside YAML block scalars is treated as YAML scalar text, so `{{ ... }}` actions are not highlighted.

## Constraints

- Zed syntax highlighting is Tree-sitter based.
- Zed language injections are defined by `injections.scm` in a language extension.
- Zed extension docs describe adding languages through `languages/<name>/config.toml`, grammars through `[grammars.*]`, and embedded language regions through `injections.scm`.
- A Zed extension can attach a language server to a Crossplane-specific language, so `up-xpls` does not need to run for every native YAML buffer.
- There is no documented way for this extension to append an injection query to Zed's built-in `YAML` language without defining a language of its own.
- Crossplane package and API-extension files commonly use names ending in `-composition.yaml`, `-definition.yaml`, or root `crossplane.yaml`.

## Approach

Add an opt-in Crossplane-specific language named `Crossplane YAML`.

The language uses the root `gotmpl` grammar from `ngalaiko/tree-sitter-go-template` as the outer parser. Its `injections.scm` injects YAML into plain template text, following the same proven pattern used by the installed Helm extension:

```scheme
((text) @content
  (#set! "language" "yaml")
  (#set! "combined"))
```

This makes Go template actions the primary syntax and keeps non-template text highlighted as YAML. It mirrors how Helm template highlighting works, but avoids taking over all YAML files.

## File Detection

The automatic detection should be conservative:

- `path_suffixes = ["-composition.yaml", "-composition.yml", "-definition.yaml", "-definition.yml", "crossplane.yaml", "crossplane.yml"]`

This should pick up files like:

- `xsetup-composition.yaml`
- `xnamespace-composition.yaml`
- `xtopic-composition.yaml`
- `xsetup-definition.yaml`
- `crossplane.yaml`

It should not claim all `.yaml` files. If Zed's `path_suffixes` does not match compound suffixes as expected, the fallback is documented user configuration:

```jsonc
{
  "file_types": {
    "Crossplane YAML": [
      "**/*-composition.yaml",
      "**/*-composition.yml",
      "**/*-definition.yaml",
      "**/*-definition.yml",
      "**/crossplane.yaml",
      "**/crossplane.yml"
    ]
  }
}
```

## Extension Manifest

Update `extension.toml` to declare the language and grammar:

```toml
languages = ["languages/crossplane-yaml"]

[grammars.gotmpl]
repository = "https://github.com/ngalaiko/tree-sitter-go-template"
rev = "aa71f63de226c5592dfbfc1f29949522d7c95fac"

[language_servers.up-xpls]
name = "Up xpls"
languages = ["Crossplane YAML"]
```

Use the root `gotmpl` grammar instead of the `helm` dialect to avoid colliding with the community Helm extension, which also registers a `helm` grammar.

## Language Files

Create `languages/crossplane-yaml/config.toml`:

```toml
name = "Crossplane YAML"
grammar = "gotmpl"
path_suffixes = [
  "-composition.yaml",
  "-composition.yml",
  "-definition.yaml",
  "-definition.yml",
  "crossplane.yaml",
  "crossplane.yml",
]
line_comments = ["# "]
block_comment = ["{{/* ", " */}}"]
brackets = [
  { start = "{{", end = "}}", close = true, newline = false },
  { start = "{{-", end = "-}}", close = true, newline = false },
  { start = "(", end = ")", close = true, newline = false },
]
```

Create `languages/crossplane-yaml/highlights.scm` by adapting the upstream `tree-sitter-go-template` query and adding common Crossplane/Sprig functions used in compositions:

- `default`
- `dig`
- `empty`
- `fail`
- `quote`
- `setResourceNameAnnotation`
- `toJson`
- `toYaml`
- `trim`
- `indent`
- `nindent`

Create `languages/crossplane-yaml/injections.scm` to inject YAML into Go-template text:

```scheme
((text) @content
  (#set! "language" "yaml")
  (#set! "combined"))
```

## Diagnostics

Attach `up-xpls` to `Crossplane YAML`, not native `YAML`.

Expected behavior:

- A normal YAML file continues to use native `YAML` and native YAML highlighting.
- A `*-composition.yaml` file uses `Crossplane YAML`.
- A `*-definition.yaml` file uses `Crossplane YAML`.
- A root `crossplane.yaml` or `crossplane.yml` file uses `Crossplane YAML`.
- `up xpls` diagnostics still run in `*-composition.yaml` files.
- Native YAML files outside the Crossplane filename set do not start `up-xpls`.
- The language label may show `Crossplane YAML`, not `YAML`, for files claimed by this new language.

## Non-Goals

- Do not implement a Crossplane parser.
- Do not patch or fork `up xpls`.
- Do not replace normal YAML highlighting for every `.yaml` file.
- Do not add Go-template completions or language-server support.
- Do not attempt semantic awareness of `function-go-templating`; this is syntax highlighting only.

## Verification

Automated checks:

- `cargo test`
- `PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2`
- Confirm the extension manifest references an existing language directory and pinned grammar revision.

Manual checks in Zed:

- Reinstall the dev extension.
- Open `/path/to/external/crossplane-package/api/xsetup-composition.yaml`.
- Confirm the language is `Crossplane YAML` if automatic suffix detection works.
- Confirm `{{ ... }}` actions highlight as Go-template syntax.
- Confirm YAML surrounding the actions still highlights as YAML.
- Confirm `xpls` diagnostics still appear.
- Open a non-composition, non-definition YAML file and confirm it remains native `YAML`.

## Risks

- Zed may not treat the listed compound suffixes as expected. If so, users need the documented `file_types` mapping.
- The `gotmpl` parser may not perfectly understand all Sprig or Crossplane helper functions, but function identifiers can still be highlighted.
- Because the outer parser is Go template, malformed template delimiters may affect highlighting more than native YAML would.
- The buffer language may show `Crossplane YAML`, which is expected for this approach.

## References

- Zed language extensions: https://zed.dev/docs/extensions/languages
- Zed code injections: https://zed.dev/docs/extensions/languages#code-injections
- Zed Helm language docs: https://zed.dev/docs/languages/helm
- Go template Tree-sitter grammar: https://github.com/ngalaiko/tree-sitter-go-template
- Installed Helm extension pattern: `~/Library/Application Support/Zed/extensions/installed/helm/languages/helm/injections.scm`
