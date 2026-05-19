# Crossplane Generated YAML Highlighting Design

Date: 2026-05-19

## Context

The `Crossplane YAML` language now highlights Crossplane `function-go-templating` files by using the `gotmpl` grammar as the outer grammar and injecting YAML into template text. This fixes the functional issue where generated YAML inside `inline.template: | # go` blocks was previously highlighted like plain scalar text.

The next usability problem is visual hierarchy. In composition files, there are two YAML layers:

- Outer Crossplane composition YAML, such as `step`, `functionRef`, `input`, `inline`, and `template`.
- Inner generated YAML emitted by the go-template block, such as `apiVersion: meta.gotemplating.fn.crossplane.io/v1alpha1`, `kind: ExtraResources`, and nested `requirements`.

Both layers currently use the same syntax colors. That is correct semantically, but it makes dense files harder to scan because readers must infer the layer boundary from indentation and `template: | # go` alone.

## Goal

Make generated YAML inside Crossplane go-template blocks visually distinct while preserving the semantic relationship to normal YAML.

The selected visual direction is **Clear Hybrid**:

- Keep the same semantic color families as normal YAML.
- Use stronger shade differences for inner generated YAML keys, values, comments, and document markers.
- Add a visible but restrained generated-YAML block cue where Zed can support it safely.
- Keep the YAML document separator `---` aligned with the generated YAML document's top-level keys.

## Non-Goals

- Do not change parsing or diagnostics behavior.
- Do not change how ordinary YAML files are highlighted.
- Do not make generated YAML look like a different language.
- Do not require every Zed theme to adopt custom colors before the extension remains usable.
- Do not promise a full-width editor block background unless Zed's syntax renderer supports it for this case.

## Technical Design

The extension should continue to use the existing `Crossplane YAML` language for outer files. The generated-YAML overlay should be routed through a distinct injected language, tentatively named `Crossplane Generated YAML`.

`Crossplane Generated YAML` should reuse YAML parsing behavior but expose different highlight capture names for generated YAML. Example capture names:

- `property.crossplane.generated`
- `string.crossplane.generated`
- `comment.crossplane.generated`
- `punctuation.special.crossplane.generated`

The capture names should be chosen so Zed's theme lookup can fall back to the base capture class when a theme has no explicit generated-YAML override. For example, `property.crossplane.generated` should still resolve as `property` under ordinary themes.

The generated-YAML document marker should remain separate from injected YAML content, using the grammar's `yaml_document_marker` node. It should be highlighted as generated YAML punctuation and visually aligned with generated YAML top-level keys.

## Visual Behavior

The preferred theme treatment is:

- Outer YAML keeps the user's existing YAML colors.
- Inner generated YAML keys use the same hue family as YAML keys, with a clearer/lighter shade.
- Inner generated YAML scalar values use the same hue family as YAML values, with a clearer/lighter shade.
- Inner generated YAML comments are slightly brighter than normal comments but still subdued.
- Inner document markers use generated punctuation styling.
- If a background cue is possible, it should be subtle and local to generated YAML text, not a heavy full-width panel.

If Zed cannot provide a clean line/block background through syntax highlighting, the implementation should still ship the token shade differentiation and document the background cue as a best-effort or future enhancement.

## Constraints

Zed's syntax highlighting chooses the innermost active capture for each text chunk rather than merging multiple captures into a composited style stack. This means a parent range capture for the generated-YAML block may not reliably combine with child YAML token captures to create both custom token colors and a block background.

Because of that, the implementation should start with a small spike:

1. Confirm whether a distinct injected language can reuse the YAML grammar and queries in this extension.
2. Confirm whether generated-YAML capture names fall back cleanly to ordinary theme captures.
3. Confirm whether any background styling can be applied without replacing token-level highlighting.

Only after those checks should the final implementation choose between:

- Token shade differentiation only.
- Token shade differentiation plus a best-effort background capture.
- Documentation-only theme overrides if extension-level background styling is not viable.

## Validation

Use the existing `xtopic-composition.yaml` region around the go-template `ExtraResources` document as the primary manual sample.

Validation should include:

- Tree-sitter query checks showing generated YAML receives generated-YAML capture names.
- A Zed manual screenshot check with the user's theme.
- Confirmation that outer YAML colors are unchanged.
- Confirmation that go-template actions and variables still use template highlighting.
- Confirmation that themes without custom generated-YAML styles still display readable YAML through fallback capture names.
- Existing Rust extension tests and WASM build checks.

## Open Risk

The only material risk is the background cue. Zed may not support a reliable full generated-YAML block background through syntax captures while also preserving token colors. If that is confirmed, the design should degrade to stronger generated-YAML token shades and a documented optional theme override, rather than introducing brittle query tricks.
