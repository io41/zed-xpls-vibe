# Crossplane Mixed YAML Template Highlighting Design Spec

**Status:** Active next-phase spec
**Date:** 2026-05-11
**Workspace:** `<local-zed-up-xpls-repo>`

## Goal

Improve syntax highlighting for Crossplane Composition YAML files that contain `function-go-templating` inline templates. The target case is a YAML file whose `inline.template` block emits more YAML while also containing Crossplane-aware Go template actions:

```yaml
inline:
  template: | # go
    {{- $xr := .observed.composite.resource -}}
    ---
    apiVersion: v1
    kind: ConfigMap
    metadata:
      name: {{ $xr.metadata.name | quote }}
      annotations:
        {{ setResourceNameAnnotation "example-config" }}
```

This spec covers syntax highlighting only. Diagnostics continue to come from `up xpls serve --verbose`.

## Current Baseline

Implemented baseline:

- The extension defines a `Crossplane YAML` language.
- The language uses the root `gotmpl` grammar from `ngalaiko/tree-sitter-go-template`.
- `up-xpls` attaches to `Crossplane YAML`, not native `YAML`.
- The extension version is `0.0.6`.
- `crossplane.yaml` and `crossplane.yml` are matched directly by `path_suffixes`.
- Composition and XRD filenames such as `*-composition.yaml` and `*-definition.yaml` require a Zed user `file_types` mapping because Zed extension `path_suffixes` cannot express glob-style suffixes.
- `languages/crossplane-yaml/injections.scm` injects YAML into `text` nodes with `combined` enabled.
- Go template actions such as `{{ ... }}` highlight, and surrounding plain template text is offered to the YAML parser.

Known limitation:

- The mixed generated-YAML plus template case is not fully correct because YAML chunks are interrupted by template actions. Tree-sitter injections normally exclude child nodes. Adding `injection.include-children` would feed `{{ ... }}` actions into the YAML parser and would degrade template highlighting.

## Zed Matching Constraint

A broad matcher such as `apiVersion:.*\.crossplane\.io/` is desirable, but it is not reliable in current Zed language configs for normal YAML files.

The built-in `YAML` language wins by `.yaml` suffix before `first_line_pattern` can select `Crossplane YAML`. In addition, Crossplane files often begin with `---`, comments, or metadata before the meaningful `apiVersion` line. Therefore, short-term file selection stays with:

- exact `path_suffixes` for `crossplane.yaml` and `crossplane.yml`
- documented user `file_types` globs for `*-composition.yaml`, `*-composition.yml`, `*-definition.yaml`, and `*-definition.yml`

Full content-based language detection is deferred until Zed exposes an extension hook that can outrank built-in YAML suffix matching.

## Short-Term Scope

In scope:

- Add fixture coverage for mixed generated YAML plus Go template constructs.
- Expand helper-function highlighting to cover Crossplane `function-go-templating` helpers already listed in the short-term spec.
- Add safe highlight-only treatment for upstream grammar nodes that represent YAML-looking punctuation, especially `yaml_no_injection_text`.
- Keep the existing YAML injection into `text` nodes as the baseline.
- Document exactly which mixed-template cases are expected to improve and which remain parser limitations.
- Verify with `tree-sitter parse`, `tree-sitter query`, `cargo test`, and the Zed dev extension.

Out of scope:

- Do not use the Helm dialect.
- Do not attach `up-xpls` to native `YAML`.
- Do not enable `injection.include-children`.
- Do not inject YAML into the parent `template` node.
- Do not build or ship a custom parser in this phase.
- Do not add completions, hover, semantic tokens, or schema-aware validation in this phase.

## Target Highlighting Behavior

The short-term implementation should make these cases look better:

- Go template delimiters, keywords, variables, selectors, functions, pipelines, strings, numbers, booleans, and comments remain highlighted.
- Crossplane helpers such as `getCompositeResource`, `getExtraResources`, `getExtraResourcesFromContext`, `getComposedResource`, `getComposedConnectionDetails`, `getResourceCondition`, `setResourceNameAnnotation`, `include`, `toYaml`, and `fromYaml` highlight as built-ins.
- Common Sprig and Go-template helpers such as `default`, `dig`, `empty`, `fail`, `quote`, `trim`, `indent`, `nindent`, `b64enc`, and `b64dec` continue to highlight as built-ins.
- Plain generated YAML text still receives YAML highlighting through `text` injections.
- YAML document markers, keys, comments, string values, and list markers should be as readable as the parser allows when split by template actions.

The implementation is allowed to leave these imperfect:

- YAML parser errors caused by a scalar value being entirely or partly replaced by `{{ ... }}`.
- YAML indentation that depends on conditional or range blocks.
- Multi-document generated YAML where template actions determine whether a document exists.
- Semantic understanding of `.observed`, `.desired`, `.context`, `.extraResources`, or XRD-derived fields.

## Implementation Approach

Use fixture-driven query work rather than broad parser changes.

Add a fixture that includes:

- a normal Crossplane Composition header
- `function-go-templating` inline template source
- variable assignment from `.observed.composite.resource`
- conditionals
- ranges that emit YAML list items
- templated scalar values
- templated annotations
- `toYaml | nindent`
- Crossplane helper functions
- document separators

Then update `languages/crossplane-yaml/highlights.scm` only. The first pass should expand the built-in helper list and add highlight-only handling for `yaml_no_injection_text`:

```scheme
(yaml_no_injection_text) @punctuation.list_marker
```

Do not broaden `languages/crossplane-yaml/injections.scm` until a fixture proves the benefit outweighs the risk. If an injection experiment is needed, it must be a separate change and must compare:

```scheme
((text) @content
  (#set! "language" "yaml")
  (#set! "combined"))
```

against:

```scheme
((yaml_no_injection_text) @content
  (#set! "language" "yaml")
  (#set! "combined"))
```

The default recommendation is to keep `yaml_no_injection_text` highlight-only because upstream describes it as a YAML parser workaround rather than normal template text.

## Verification

Automated checks:

```bash
cargo fmt --check
cargo test
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
python3 -c 'import tomllib; tomllib.load(open("extension.toml", "rb")); tomllib.load(open("languages/crossplane-yaml/config.toml", "rb")); print("toml ok")'
git diff --check
```

Tree-sitter checks:

```bash
tree-sitter parse --grammar-path grammars/gotmpl --quiet fixtures/crossplane-package/api/mixed-template-composition.yaml
tree-sitter query --grammar-path grammars/gotmpl languages/crossplane-yaml/highlights.scm fixtures/crossplane-package/api/mixed-template-composition.yaml
tree-sitter query --grammar-path grammars/gotmpl languages/crossplane-yaml/injections.scm fixtures/crossplane-package/api/mixed-template-composition.yaml
```

Manual Zed checks:

- Rebuild or reinstall the dev extension.
- Open `fixtures/crossplane-package/api/mixed-template-composition.yaml`.
- Confirm the language is `Crossplane YAML`.
- Confirm Go template actions still highlight clearly.
- Confirm generated YAML around template actions highlights as YAML where possible.
- Open a real package Composition such as `api/xtopic-composition.yaml`.
- Confirm `up-xpls` still starts in a Crossplane package worktree.
- Confirm ordinary YAML files remain native `YAML`.

## Risks

- Some YAML injection gaps may be impossible to fix with query changes alone because the generated YAML is not syntactically complete until after template execution.
- Highlighting `yaml_no_injection_text` may improve list-marker readability but does not make the YAML parser understand template-driven structure.
- Broadening YAML injections can make template highlighting worse if template actions are fed into the YAML parser.
- Visual verification in Zed remains necessary because Tree-sitter query output proves captures, not final theme rendering.

## Deferred Work

Correct semantic support for Crossplane templates remains deferred to the long-term spec. That includes completions, hovers, schema-aware field validation, reliable template diagnostics, and a possible `up xpls` improvement, LSP proxy, or dedicated Crossplane template LSP.

## References

- Short-term spec: `docs/superpowers/specs/2026-05-05-crossplane-yaml-short-term-design.md`
- Long-term deferred spec: `docs/superpowers/specs/2026-05-05-crossplane-yaml-long-term-deferred-design.md`
- Implementation plan: `docs/superpowers/plans/2026-05-11-crossplane-mixed-template-highlighting.md`
- Zed language extensions: https://zed.dev/docs/extensions/languages
- Zed code injections: https://zed.dev/docs/extensions/languages#code-injections
- Go template Tree-sitter grammar: https://github.com/ngalaiko/tree-sitter-go-template
