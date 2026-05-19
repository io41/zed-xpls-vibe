# Crossplane Generated YAML Stock Capture Spike Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Determine whether `Crossplane YAML` can make generated YAML visually clearer using extension-owned query captures and normal Zed theme settings.

**Architecture:** Treat generated-YAML contrast as a spike before committing to implementation. Add temporary query captures that use documented Zed capture names, verify their precision with Tree-sitter, validate the result in Zed without adding user theme settings, then remove temporary query edits and record the outcome.

**Tech Stack:** Zed language extension queries, Tree-sitter `gotmpl` grammar, documented Zed syntax captures, Rust/WASM extension build for `wasm32-wasip2`.

---

## File Structure

- Temporarily modify: `languages/crossplane-yaml/highlights.scm` - add spike-only generated YAML capture candidates, then remove them before the final commit.
- Read: `languages/crossplane-yaml/injections.scm` - confirm the existing combined plus fragment YAML injections remain unchanged.
- Read: `fixtures/crossplane-package/api/mixed-template-composition.yaml` - primary committed validation sample.
- Read manually: `/path/to/external/crossplane-package/api/xtopic-composition.yaml` - secondary real-world validation sample.
- Create: `docs/superpowers/spikes/2026-05-19-crossplane-generated-yaml-stock-capture-results.md` - spike outcome and decision record.

## Task 0: Baseline Check

**Files:**
- No file changes.

- [ ] **Step 1: Confirm tracked files are clean**

Run:

```bash
git status --short --branch
```

Expected: no tracked modifications before starting the spike. The existing untracked `.superpowers/` brainstorming artifacts may appear and should not be committed.

- [ ] **Step 2: Confirm the extension test baseline**

Run:

```bash
cargo fmt --check
```

Expected: exits successfully.

Run:

```bash
cargo test
```

Expected: all Rust unit tests pass.

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: the WASM extension build succeeds.

- [ ] **Step 3: Confirm the primary fixture parses**

Run:

```bash
cp fixtures/crossplane-package/api/mixed-template-composition.yaml /tmp/mixed-template-composition.gotmpl
```

Expected: exits successfully.

Run:

```bash
tree-sitter parse --grammar-path grammars/gotmpl --quiet /tmp/mixed-template-composition.gotmpl
```

Expected: exits successfully. A warning about missing global parser directories is acceptable if the command exits with status 0.

## Task 1: Add Temporary Range Capture Candidate

**Files:**
- Modify temporarily: `languages/crossplane-yaml/highlights.scm`

- [ ] **Step 1: Add the spike-only captures**

Insert this block in `languages/crossplane-yaml/highlights.scm` immediately after the existing `yaml_document_marker` / `yaml_no_injection_text` punctuation block:

```scheme
; SPIKE ONLY: generated YAML range capture candidates.
; These captures must be removed before the spike result commit.
; They use documented Zed capture names only; do not add custom capture names
; or user theme settings for this spike.

(template
  (variable_definition)+
  (yaml_document_marker)
  (text) @text.literal @embedded)

(if_action
  consequence: (yaml_document_marker)
  consequence: (text) @text.literal @embedded)

(if_action
  alternative: (yaml_document_marker)
  alternative: (text) @text.literal @embedded)

(range_action
  body: (text) @text.literal @embedded)
```

- [ ] **Step 2: Verify the query is syntactically valid**

Run:

```bash
tree-sitter query --grammar-path grammars/gotmpl languages/crossplane-yaml/highlights.scm /tmp/mixed-template-composition.gotmpl
```

Expected: exits successfully. The missing-parser-directory warning is acceptable if the command exits with status 0.

- [ ] **Step 3: Check generated capture precision**

Run:

```bash
tree-sitter query --grammar-path grammars/gotmpl languages/crossplane-yaml/highlights.scm /tmp/mixed-template-composition.gotmpl | rg -C 2 "embedded|text.literal"
```

Expected:

- Output contains `embedded` and `text.literal` captures for generated YAML text ranges after the template setup actions, including rows around `23`, `37`, `45`, and `49`.
- Output does not contain `embedded` or `text.literal` captures for outer YAML rows `0` through `19`.
- If outer YAML rows are captured, Outcome A fails unless the query can be narrowed without brittle indentation-only matching.

- [ ] **Step 4: Check that template highlighting still appears in query output**

Run:

```bash
tree-sitter query --grammar-path grammars/gotmpl languages/crossplane-yaml/highlights.scm /tmp/mixed-template-composition.gotmpl | rg "function.builtin|variable|punctuation.bracket"
```

Expected: output still includes existing template captures such as `function.builtin`, `variable`, and `punctuation.bracket`.

## Task 2: Validate Normal-Theme Rendering In Zed

**Files:**
- Temporarily modified: `languages/crossplane-yaml/highlights.scm`
- No Zed user settings changes.

- [ ] **Step 1: Build the extension with the temporary capture query**

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: the WASM extension build succeeds.

- [ ] **Step 2: Rebuild the local milestone binary before Zed validation**

Run:

```bash
go build -o <temporary-vibe-xpls-binary> ./cmd/vibe-xpls
```

Working directory for this command:

```text
<local-vibe-xpls-worktree>
```

Expected: `<temporary-vibe-xpls-binary>` is rebuilt successfully.

- [ ] **Step 3: Install this repository as the Zed dev extension**

In Zed, install `<local-zed-xpls-vibe-repo>` as a dev extension. Do not install the original `up-xpls` extension for this validation.

Expected: `Crossplane YAML` appears as an available language, and the extension still launches language server id `zed-xpls-vibe` with `<temporary-vibe-xpls-binary> serve`.

- [ ] **Step 4: Validate the committed fixture without changing settings**

Do not add or edit user theme settings. Use the current active Zed theme and existing file association settings.

Open:

```text
<local-zed-xpls-vibe-repo>/fixtures/crossplane-package/api/mixed-template-composition.yaml
```

Expected for Outcome A:

- Generated YAML text after `template: | # go` has a visible but not distracting distinction from outer YAML.
- Outer YAML keys such as `pipeline`, `step`, `functionRef`, `input`, `inline`, and `template` do not receive the generated capture treatment.
- Go-template actions still highlight with template colors.
- YAML inside the generated document remains readable.

- [ ] **Step 5: Validate the real-world sample without changing settings**

Open:

```text
/path/to/external/crossplane-package/api/xtopic-composition.yaml
```

Expected for Outcome A:

- The `ExtraResources` document in the go-template block receives the generated capture treatment.
- Outer Crossplane pipeline YAML does not receive the generated capture treatment.
- The generated YAML `---` marker stays aligned with generated YAML top-level keys.

## Task 3: Remove Temporary Query Edits

**Files:**
- Modify: `languages/crossplane-yaml/highlights.scm`

- [ ] **Step 1: Remove the spike-only capture block**

Remove the block that starts with:

```scheme
; SPIKE ONLY: generated YAML range capture candidates.
```

and ends with:

```scheme
(range_action
  body: (text) @text.literal @embedded)
```

Expected: `languages/crossplane-yaml/highlights.scm` returns to its pre-spike content.

- [ ] **Step 2: Confirm the query file has no spike captures**

Run:

```bash
rg -n "SPIKE ONLY|@embedded|@text.literal" languages/crossplane-yaml/highlights.scm
```

Expected: no output.

## Task 4: Record The Spike Outcome

**Files:**
- Create: `docs/superpowers/spikes/2026-05-19-crossplane-generated-yaml-stock-capture-results.md`

- [ ] **Step 1: Create the spike results document**

Create `docs/superpowers/spikes/2026-05-19-crossplane-generated-yaml-stock-capture-results.md` with these headings and the observed facts from Tasks 1 and 2:

```markdown
# Crossplane Generated YAML Stock Capture Spike Results

Date: 2026-05-19

## Decision

## Tree-Sitter Evidence

## Zed Visual Evidence

## Follow-Up
```

The decision must be exactly one of:

- `Outcome A: generated range capture is viable`
- `Outcome B: generated token captures need a generated-YAML language`
- `Outcome C: no viable extension-owned contrast`

The evidence sections must state whether outer YAML rows were incorrectly captured, whether template captures were preserved, which active Zed theme was used, and whether any Zed settings were changed. The Zed settings line must say `No new Zed user settings were added for this spike.`

- [ ] **Step 2: Review the result document for decision clarity**

Read `docs/superpowers/spikes/2026-05-19-crossplane-generated-yaml-stock-capture-results.md`.

Expected:

- The `Decision` section contains exactly one of the three allowed outcome lines.
- The evidence sections include the active Zed theme and the line `No new Zed user settings were added for this spike.`
- The document does not propose user theme settings as a deliverable.

- [ ] **Step 3: Verify no temporary query edits remain**

Run:

```bash
rg -n "SPIKE ONLY|@embedded|@text.literal" languages/crossplane-yaml/highlights.scm
```

Expected: no output.

## Task 5: Final Verification And Commit

**Files:**
- Create: `docs/superpowers/spikes/2026-05-19-crossplane-generated-yaml-stock-capture-results.md`

- [ ] **Step 1: Run formatting and tests**

Run:

```bash
cargo fmt --check
```

Expected: exits successfully.

Run:

```bash
cargo test
```

Expected: all Rust unit tests pass.

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: the WASM extension build succeeds.

- [ ] **Step 2: Confirm the final diff**

Run:

```bash
git status --short
```

Expected: only `docs/superpowers/spikes/2026-05-19-crossplane-generated-yaml-stock-capture-results.md` is tracked as a new file. The existing untracked `.superpowers/` artifacts may still appear and should not be committed.

Run:

```bash
git diff --check
```

Expected: no whitespace errors.

- [ ] **Step 3: Commit the spike result**

Run:

```bash
git add docs/superpowers/spikes/2026-05-19-crossplane-generated-yaml-stock-capture-results.md
git commit -m "docs: record generated yaml stock capture spike"
```

Expected: a commit records the spike decision document.
