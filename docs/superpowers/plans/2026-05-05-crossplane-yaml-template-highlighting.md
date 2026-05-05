# Crossplane YAML Template Highlighting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Go-template syntax highlighting for Crossplane Composition YAML files without changing `up xpls` diagnostics.

**Architecture:** Add a new Zed language named `Crossplane YAML` that uses the `gotmpl` Tree-sitter grammar as the outer parser and injects YAML into template text. Keep native `YAML` untouched for normal YAML files, and attach `up-xpls` only to `Crossplane YAML`.

**Tech Stack:** Zed language extension files, Tree-sitter query files, `ngalaiko/tree-sitter-go-template` pinned at `aa71f63de226c5592dfbfc1f29949522d7c95fac`, Rust/WASM extension build for `wasm32-wasip2`.

---

## File Structure

- Modify: `extension.toml` - register `Crossplane YAML`, the `gotmpl` grammar, and attach `up-xpls` to that language.
- Create: `languages/crossplane-yaml/config.toml` - language metadata and bracket/comment behavior.
- Create: `languages/crossplane-yaml/highlights.scm` - Go-template highlights with Crossplane/Sprig helpers.
- Create: `languages/crossplane-yaml/injections.scm` - inject YAML into template text.
- Create: `fixtures/crossplane-package/api/xsetup-composition.yaml` - fixture that exercises inline `function-go-templating`.
- Modify: `README.md` - document the new language and fallback `file_types` mapping.

## Task 0: Starting State

**Files:**
- No files changed directly.

- [ ] **Step 1: Check the working tree**

Run:

```bash
git status --short
```

Expected: only intentional uncommitted changes are present. If unrelated user changes exist, leave them alone.

- [ ] **Step 2: Confirm current tests pass before changing behavior**

Run:

```bash
cargo test
```

Expected: all existing unit tests pass.

## Task 1: Register the Crossplane YAML Language

**Files:**
- Modify: `extension.toml`

- [ ] **Step 1: Update the extension manifest**

Edit `extension.toml` to include these exact changes:

```toml
id = "up-xpls"
name = "Up xpls"
version = "0.0.5"
schema_version = 1
authors = ["Tim Kersten"]
description = "Crossplane package diagnostics powered by up xpls"
repository = "https://github.com/io41/zed-up-xpls-vibe"
languages = ["languages/crossplane-yaml"]

[grammars.gotmpl]
repository = "https://github.com/ngalaiko/tree-sitter-go-template"
rev = "aa71f63de226c5592dfbfc1f29949522d7c95fac"

[language_servers.up-xpls]
name = "Up xpls"
languages = ["Crossplane YAML"]
```

This intentionally uses the root `gotmpl` grammar, not the `helm` dialect, to avoid a grammar-name collision with the community Helm extension.

- [ ] **Step 2: Verify the manifest shape**

Run:

```bash
sed -n '1,80p' extension.toml
```

Expected: the file contains top-level `languages = ["languages/crossplane-yaml"]`, a `[grammars.gotmpl]` block, and `up-xpls` lists `Crossplane YAML`.

## Task 2: Add Language Metadata and Queries

**Files:**
- Create: `languages/crossplane-yaml/config.toml`
- Create: `languages/crossplane-yaml/highlights.scm`
- Create: `languages/crossplane-yaml/injections.scm`

- [ ] **Step 1: Create the language directory**

Run:

```bash
mkdir -p languages/crossplane-yaml
```

Expected: `languages/crossplane-yaml` exists.

- [ ] **Step 2: Add language metadata**

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

- [ ] **Step 3: Add Go-template highlights**

Create `languages/crossplane-yaml/highlights.scm`:

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
  (#match? @function.builtin "^(and|call|html|index|slice|js|len|not|or|print|printf|println|urlquery|eq|ne|lt|le|gt|ge|default|dig|empty|fail|quote|setResourceNameAnnotation|toJson|toYaml|trim|indent|nindent|b64enc|b64dec)$"))

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

- [ ] **Step 4: Inject YAML into template text**

Create `languages/crossplane-yaml/injections.scm`:

```scheme
((text) @content
  (#set! "language" "yaml")
  (#set! "combined"))
```

This follows the same pattern used by the installed Helm extension, where the Go-template parser owns the file and YAML is injected into non-template text.

## Task 3: Add a Fixture for Manual Highlighting Checks

**Files:**
- Create: `fixtures/crossplane-package/api/xsetup-composition.yaml`

- [ ] **Step 1: Create the fixture directory**

Run:

```bash
mkdir -p fixtures/crossplane-package/api
```

Expected: `fixtures/crossplane-package/api` exists.

- [ ] **Step 2: Add a composition fixture**

Create `fixtures/crossplane-package/api/xsetup-composition.yaml`:

```yaml
---
apiVersion: apiextensions.crossplane.io/v1
kind: Composition
metadata:
  name: xsetup.example.org
spec:
  compositeTypeRef:
    apiVersion: example.org/v1alpha1
    kind: XSetup
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
            {{- $xr := .observed.composite.resource -}}
            {{- $name := $xr.metadata.name | default "example" -}}
            {{- if empty $xr.spec.region -}}
              {{- fail "spec.region is required" -}}
            {{- end -}}
            ---
            apiVersion: v1
            kind: ConfigMap
            metadata:
              name: {{ $name | quote }}
              annotations:
                {{ setResourceNameAnnotation "example-config" }}
            data:
              region: {{ $xr.spec.region | quote }}
```

## Task 4: Document Usage and Fallback Mapping

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add a syntax highlighting section**

Add this section after the current Usage section in `README.md`:

````markdown
## Syntax Highlighting

The extension includes a `Crossplane YAML` language for `*-composition.yaml`, `*-composition.yml`, `*-definition.yaml`, `*-definition.yml`, `crossplane.yaml`, and `crossplane.yml` files. It uses Go-template highlighting for `{{ ... }}` actions and injects YAML highlighting into the surrounding template text.

`up-xpls` diagnostics are attached to `Crossplane YAML`. Native `YAML` files remain handled by Zed's normal YAML support.

If Zed does not automatically select `Crossplane YAML` for your Crossplane files, add a file type mapping to your Zed settings:

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
````

- [ ] **Step 2: Update the opening description**

Change the first paragraph in `README.md` from diagnostics-only wording to:

```markdown
Adds Crossplane package diagnostics and Composition template highlighting to Zed by starting the `up xpls serve --verbose` language server for `Crossplane YAML` files.
```

## Task 5: Verify Build and Editor Behavior

**Files:**
- No files changed directly.

- [ ] **Step 1: Run formatting check**

Run:

```bash
cargo fmt --check
```

Expected: no output and exit code 0.

- [ ] **Step 2: Run unit tests**

Run:

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 3: Build the Zed WASM extension**

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: build succeeds.

- [ ] **Step 4: Reinstall the dev extension**

In Zed, run `zed: install dev extension` and select `<local-zed-up-xpls-repo>`.

Expected: the extension installs without a Rust compile error.

- [ ] **Step 5: Check the local fixture**

Open:

```text
<local-zed-up-xpls-repo>/fixtures/crossplane-package/api/xsetup-composition.yaml
```

Expected:

- The language selector shows `Crossplane YAML` if compound suffix detection works.
- `{{ ... }}` actions have Go-template highlighting.
- YAML text inside the template still has YAML highlighting.
- `up-xpls` starts for the worktree because `fixtures/crossplane-package/crossplane.yaml` exists.

- [ ] **Step 6: Check the user's real composition**

Open:

```text
/path/to/external/crossplane-package/api/xsetup-composition.yaml
```

Expected:

- The language selector shows `Crossplane YAML`.
- The `template: | # go` blocks highlight Go-template actions.
- Existing `xpls` diagnostics still appear.

- [ ] **Step 7: Check normal YAML is untouched**

Open:

```text
<local-zed-up-xpls-repo>/fixtures/not-crossplane/config.yaml
```

Expected: the language selector remains native `YAML`, not `Crossplane YAML`, and `up-xpls` is not started for this file.

## Task 6: Commit

**Files:**
- All files modified in this plan.

- [ ] **Step 1: Review the final diff**

Run:

```bash
git diff --stat
git diff --check
```

Expected: changed files match this plan, and `git diff --check` has no output.

- [ ] **Step 2: Commit with the required author**

Run:

```bash
git add extension.toml languages/crossplane-yaml README.md fixtures/crossplane-package/api/xsetup-composition.yaml docs/superpowers/specs/2026-05-05-crossplane-yaml-template-highlighting-design.md docs/superpowers/plans/2026-05-05-crossplane-yaml-template-highlighting.md
GIT_AUTHOR_NAME='Tim Kersten' GIT_AUTHOR_EMAIL='tim@io41.com' GIT_COMMITTER_NAME='Tim Kersten' GIT_COMMITTER_EMAIL='tim@io41.com' git commit -m 'feat: add crossplane template highlighting'
```

Expected: a commit is created with author and committer `Tim Kersten <tim@io41.com>`.

## Plan Self-Review

- Spec coverage: The plan registers a Crossplane-specific language, adds Go-template highlights, injects YAML, keeps `up-xpls` diagnostics on Crossplane YAML, documents fallback file mapping, and verifies real and fixture files.
- Completeness scan: All file paths and implementation details are defined.
- Type consistency: The language name is consistently `Crossplane YAML`, the grammar is consistently `gotmpl`, and `up-xpls` attaches to `Crossplane YAML`.
