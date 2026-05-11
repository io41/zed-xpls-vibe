# Crossplane Mixed Template Highlighting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve Zed highlighting for Crossplane Composition YAML files that mix generated YAML with Crossplane `function-go-templating` actions.

**Architecture:** Keep `Crossplane YAML` backed by the root `gotmpl` Tree-sitter grammar and keep YAML injection limited to plain `text` nodes. Improve behavior with fixture-driven query changes: expand Crossplane/Sprig helper captures, add safe highlight-only handling for `yaml_no_injection_text`, document the remaining parser limits, and bump the extension version for the visible highlighting change.

**Tech Stack:** Zed language extension files, Tree-sitter query files, `ngalaiko/tree-sitter-go-template` pinned at `aa71f63de226c5592dfbfc1f29949522d7c95fac`, Rust/WASM extension build for `wasm32-wasip2`.

---

## File Structure

- Create: `fixtures/crossplane-package/api/mixed-template-composition.yaml` - fixture covering generated YAML mixed with Crossplane Go-template actions.
- Modify: `languages/crossplane-yaml/highlights.scm` - expand built-in helper highlighting and add safe highlight-only punctuation capture.
- Verify unchanged: `languages/crossplane-yaml/injections.scm` - keep YAML injection limited to `text` nodes.
- Modify: `README.md` - document current mixed-template highlighting expectations and limitations.
- Modify: `extension.toml` - bump the extension version from `0.0.6` to `0.0.7`.
- Modify: `docs/superpowers/specs/2026-05-05-crossplane-yaml-template-highlighting-design.md` - keep the design spec aligned with the implemented query behavior.

## Task 0: Baseline Check

**Files:**
- No file changes.

- [ ] **Step 1: Confirm the worktree is clean**

Run:

```bash
git status --short --branch
```

Expected: output shows `## main...origin/main` and no modified files.

- [ ] **Step 2: Confirm the baseline extension builds**

Run:

```bash
cargo fmt --check
```

Expected: exits successfully.

Run:

```bash
cargo test
```

Expected: all unit tests pass.

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: the WASM build finishes without errors.

## Task 1: Add Mixed Template Fixture

**Files:**
- Create: `fixtures/crossplane-package/api/mixed-template-composition.yaml`

- [ ] **Step 1: Create the fixture**

Create `fixtures/crossplane-package/api/mixed-template-composition.yaml` with this exact content:

```yaml
---
apiVersion: apiextensions.crossplane.io/v1
kind: Composition
metadata:
  name: xmixed.example.org
spec:
  compositeTypeRef:
    apiVersion: example.org/v1alpha1
    kind: XMixed
  mode: Pipeline
  pipeline:
    - step: render
      functionRef:
        name: function-go-templating
      input:
        apiVersion: gotemplating.fn.crossplane.io/v1beta1
        kind: GoTemplate
        source: Inline
        inline:
          template: | # go
            {{- $xr := getCompositeResource . -}}
            {{- $name := dig "metadata" "name" "example" $xr | default "example" -}}
            {{- $extra := getExtraResourcesFromContext . "network" -}}
            ---
            apiVersion: v1
            kind: ConfigMap
            metadata:
              name: {{ $name | quote }}
              annotations:
                {{ setResourceNameAnnotation "example-config" }}
                example.org/region: {{ dig "spec" "region" "eastus" $xr | quote }}
            data:
              observed: {{ getComposedResource . "example-config" | toJson | quote }}
              connection: {{ getComposedConnectionDetails . "example-config" | toJson | quote }}
              ready: {{ getResourceCondition $xr "Ready" | toJson | quote }}
              extra: {{ $extra | toYaml | nindent 16 }}
            {{- if empty (dig "spec" "topics" list $xr) }}
            ---
            apiVersion: v1
            kind: Secret
            metadata:
              name: {{ printf "%s-empty" $name | quote }}
            stringData:
              reason: {{ "no topics configured" | quote }}
            {{- else }}
            ---
            apiVersion: v1
            kind: List
            items:
              {{- range $topic := dig "spec" "topics" list $xr }}
              - apiVersion: v1
                kind: ConfigMap
                metadata:
                  name: {{ printf "%s-%s" $name $topic.name | quote }}
                data:
                  topic: {{ $topic.name | quote }}
                  labels: {{ include "topic.labels" $topic | fromYaml | toYaml | nindent 18 }}
              {{- end }}
            {{- end }}
```

- [ ] **Step 2: Parse the fixture with the gotmpl grammar**

The Tree-sitter CLI may select a global YAML or Helm parser for `.yaml` files on this machine. Use a temporary `.gotmpl` copy for parser-level verification.

Run:

```bash
cp fixtures/crossplane-package/api/mixed-template-composition.yaml /tmp/mixed-template-composition.gotmpl
```

Expected: exits successfully.

Run:

```bash
tree-sitter parse --grammar-path grammars/gotmpl --quiet /tmp/mixed-template-composition.gotmpl
```

Expected: exits successfully. A warning about missing global parser directories is acceptable if the command still exits with status 0.

- [ ] **Step 3: Commit the fixture**

Run:

```bash
git add fixtures/crossplane-package/api/mixed-template-composition.yaml
GIT_AUTHOR_NAME="Tim Kersten" GIT_AUTHOR_EMAIL="tim@io41.com" GIT_COMMITTER_NAME="Tim Kersten" GIT_COMMITTER_EMAIL="tim@io41.com" git commit -m "test: add mixed crossplane template fixture"
```

Expected: a commit is created with author and committer `Tim Kersten <tim@io41.com>`.

## Task 2: Expand Highlight Query Coverage

**Files:**
- Modify: `languages/crossplane-yaml/highlights.scm`

- [ ] **Step 1: Replace `languages/crossplane-yaml/highlights.scm`**

Update `languages/crossplane-yaml/highlights.scm` to this exact content:

```scheme
; Identifiers

[
  (field)
  (field_identifier)
] @property

(variable) @variable

; Function calls

(function_call
  function: (identifier) @function)

(method_call
  method: (selector_expression
    field: (field_identifier) @function))

; Operators

"|" @operator
":=" @operator

; Builtin, Sprig, and Crossplane go-templating helpers

((identifier) @function.builtin
  (#match? @function.builtin "^(and|call|html|index|slice|js|len|not|or|print|printf|println|urlquery|eq|ne|lt|le|gt|ge|default|dig|empty|fail|quote|randomChoice|toJson|toYaml|fromYaml|trim|indent|nindent|b64enc|b64dec|getResourceCondition|getComposedResource|getComposedConnectionDetails|getCompositeResource|getExtraResources|getExtraResourcesFromContext|setResourceNameAnnotation|include)$"))

; YAML-looking punctuation emitted by the go-template grammar.
; Keep this highlight-only; do not inject it as YAML content by default.
; Scope this to top-level template children so malformed actions under ERROR
; do not look like YAML list markers.

(template
  (yaml_no_injection_text) @punctuation.list_marker)

; Delimiters

"." @punctuation.delimiter
"," @punctuation.delimiter

"{{" @punctuation.bracket
"}}" @punctuation.bracket
"{{-" @punctuation.bracket
"-}}" @punctuation.bracket
")" @punctuation.bracket
"(" @punctuation.bracket

; Keywords

"else" @keyword
"if" @keyword
"range" @keyword
"with" @keyword
"end" @keyword
"template" @keyword
"define" @keyword
"block" @keyword

; Literals

[
  (interpreted_string_literal)
  (raw_string_literal)
  (rune_literal)
] @string

(escape_sequence) @string.special

[
  (int_literal)
  (float_literal)
  (imaginary_literal)
] @number

[
  (true)
  (false)
  (nil)
] @constant.builtin

(comment) @comment
(ERROR) @error
```

- [ ] **Step 2: Verify helper captures exist**

Run:

```bash
cp fixtures/crossplane-package/api/mixed-template-composition.yaml /tmp/mixed-template-composition.gotmpl
```

Expected: exits successfully.

Run:

```bash
tree-sitter query --grammar-path grammars/gotmpl languages/crossplane-yaml/highlights.scm /tmp/mixed-template-composition.gotmpl
```

Expected: output includes `function.builtin` captures for `getCompositeResource`, `getExtraResourcesFromContext`, `setResourceNameAnnotation`, `getComposedResource`, `getComposedConnectionDetails`, `getResourceCondition`, `toYaml`, `fromYaml`, and `include`.

- [ ] **Step 3: Verify YAML injection query is still scoped to text nodes**

Run:

```bash
tree-sitter query --grammar-path grammars/gotmpl languages/crossplane-yaml/injections.scm /tmp/mixed-template-composition.gotmpl
```

Expected: output includes `content` captures for `text` nodes. `languages/crossplane-yaml/injections.scm` remains:

```scheme
((text) @content
  (#set! "language" "yaml")
  (#set! "combined"))
```

- [ ] **Step 4: Commit query updates**

Run:

```bash
git add languages/crossplane-yaml/highlights.scm
GIT_AUTHOR_NAME="Tim Kersten" GIT_AUTHOR_EMAIL="tim@io41.com" GIT_COMMITTER_NAME="Tim Kersten" GIT_COMMITTER_EMAIL="tim@io41.com" git commit -m "feat: expand crossplane template highlights"
```

Expected: a commit is created with author and committer `Tim Kersten <tim@io41.com>`.

## Task 3: Document Behavior and Bump Version

**Files:**
- Modify: `README.md`
- Modify: `extension.toml`
- Modify: `docs/superpowers/specs/2026-05-05-crossplane-yaml-template-highlighting-design.md`

- [ ] **Step 1: Update README syntax highlighting section**

In `README.md`, replace the first paragraph under `## Syntax Highlighting` with:

```markdown
`Crossplane YAML` uses Go-template highlighting for `{{ ... }}` actions and injects YAML highlighting into surrounding template text. This is intended for Crossplane `function-go-templating` inline templates where the block scalar emits YAML.

The mixed YAML/template case is best-effort. Template actions remain highlighted, and plain generated YAML text is injected into the YAML parser, but some YAML constructs can still look imperfect when a scalar, list item, or indentation level is split by `{{ ... }}` actions.
```

Expected: the existing `file_types` explanation remains below this text.

- [ ] **Step 2: Bump the extension version**

In `extension.toml`, change:

```toml
version = "0.0.6"
```

to:

```toml
version = "0.0.7"
```

- [ ] **Step 3: Update the spec baseline version**

In `docs/superpowers/specs/2026-05-05-crossplane-yaml-template-highlighting-design.md`, change:

```markdown
- The extension version is `0.0.6`.
```

to:

```markdown
- The extension version is `0.0.7`.
```

- [ ] **Step 4: Commit docs and version**

Run:

```bash
git add README.md extension.toml docs/superpowers/specs/2026-05-05-crossplane-yaml-template-highlighting-design.md
GIT_AUTHOR_NAME="Tim Kersten" GIT_AUTHOR_EMAIL="tim@io41.com" GIT_COMMITTER_NAME="Tim Kersten" GIT_COMMITTER_EMAIL="tim@io41.com" git commit -m "docs: describe mixed template highlighting"
```

Expected: a commit is created with author and committer `Tim Kersten <tim@io41.com>`.

## Task 4: Full Verification

**Files:**
- No file changes expected unless verification reveals a problem.

- [ ] **Step 1: Run automated checks**

Run:

```bash
cargo fmt --check
```

Expected: exits successfully.

Run:

```bash
cargo test
```

Expected: all unit tests pass.

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: the WASM build finishes without errors.

Run:

```bash
python3 -c 'import tomllib; tomllib.load(open("extension.toml", "rb")); tomllib.load(open("languages/crossplane-yaml/config.toml", "rb")); print("toml ok")'
```

Expected: prints `toml ok`.

Run:

```bash
git diff --check
```

Expected: exits successfully.

- [ ] **Step 2: Rebuild the dev extension in Zed**

In Zed, open:

```text
zed: extensions
```

Then open the dev extension card for `Up xpls` and click `Rebuild`.

Expected: Zed recompiles the extension without a grammar compile error.

- [ ] **Step 3: Manually inspect fixture highlighting**

Open:

```text
fixtures/crossplane-package/api/mixed-template-composition.yaml
```

Expected:

- language label is `Crossplane YAML`
- Go-template delimiters and keywords are visible
- Crossplane helpers are styled as functions or built-ins
- generated YAML around template actions is still readable
- ordinary YAML files continue to open as native `YAML`

- [ ] **Step 4: Manually inspect a real package Composition**

Open:

```text
/path/to/external/crossplane-package/api/xtopic-composition.yaml
```

Expected:

- language label is `Crossplane YAML` through the Zed `file_types` mapping
- `up-xpls` starts in the package worktree
- template actions remain highlighted
- generated YAML around actions is more readable than in version `0.0.6`

- [ ] **Step 5: Push commits**

Run:

```bash
git status --short --branch
```

Expected: the branch is ahead of `origin/main` only by the planned commits and has no uncommitted changes.

Run:

```bash
git push
```

Expected: commits push to `origin/main`.
