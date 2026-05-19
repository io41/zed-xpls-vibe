# Crossplane Generated YAML Highlighting Design

Date: 2026-05-19

Status: revised after external review; not yet an implementation plan.

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
- Spike whether generated-YAML highlight hooks can let users or themes choose stronger shade differences for inner generated YAML keys, values, comments, and document markers.
- Prefer token-level shade differences over a block background.
- Keep the YAML document separator `---` aligned with the generated YAML document's top-level keys.

The extension owns semantic boundaries and readable fallback. It does not own the user's palette. Candidate hook names are not a committed configuration surface until the spike proves they are precise, stable, and styleable in Zed. Under stock themes, generated YAML must remain readable through ordinary capture fallback, but it may not look different. A Clear Hybrid override can be documented as an example only if the hook spike passes.

## Non-Goals

- Do not change parsing or diagnostics behavior.
- Do not change how ordinary YAML files are highlighted.
- Do not make generated YAML look like a different language.
- Do not claim visible generated-YAML colors under unmodified stock themes.
- Do not ship or maintain a bundled theme.
- Do not prescribe one universal palette for all users.
- Do not ship a full-width editor block background in the first milestone.
- Do not introduce a new injected language unless a spike shows the smaller approaches cannot meet the goal.

## Technical Design

The extension should continue to use the existing `Crossplane YAML` language for outer files.

The current injection setup has two YAML paths:

- A combined YAML injection over `text` nodes so outer Crossplane YAML is parsed as YAML instead of plain gotmpl text.
- A second YAML-looking fragment injection so generated YAML inside `function-go-templating` block scalars is parsed as YAML instead of inheriting the outer block-scalar highlight.

That asymmetry is intentional. Any implementation must either preserve it or replace it with an equivalent mechanism that keeps both outer YAML and generated YAML highlighted.

The first step should be a theme-hook viability spike, not a new language and not a committed implementation path:

1. Confirm a generated-range capture can target generated YAML text precisely enough without changing ordinary outer YAML.
2. Confirm whether that capture remains visible when injected YAML token captures are also active.
3. Confirm generated capture names are styleable in Zed and that fallback to base captures remains readable.
4. Confirm the candidate hook names are generic enough to be useful without becoming a theme-maintenance burden.

Only if the spike passes should the implementation expose the smallest useful capture surface, such as `text.generated` or `embedded.generated`, plus a documented example override snippet. This path avoids a new language and avoids duplicating YAML queries.

If the hook spike fails, do not ship hook names or theme guidance. The fallback candidate is a distinct injected language, tentatively named `Crossplane Generated YAML`, but only if the added complexity is justified.

`Crossplane Generated YAML` should reuse YAML parsing behavior but expose different highlight capture names for generated YAML. Example capture names:

- `property.generated`
- `string.generated`
- `comment.generated`
- `punctuation.special.generated`

The capture names should be chosen so Zed's theme lookup can fall back to the base capture class when a theme has no explicit generated-YAML override. For example, `property.generated` should still resolve as `property` under ordinary themes.

The distinct-language path is allowed only if the spike confirms:

- Zed can inject into an extension-defined language cleanly.
- The YAML grammar and queries can be reused or copied without losing important YAML behavior.
- The extra maintenance cost is justified by visibly better generated-YAML tokens.

The generated-YAML document marker is the `yaml_document_marker` node from the outer `gotmpl` grammar, not the injected YAML parser. It is already captured in `languages/crossplane-yaml/highlights.scm`. The implementation may refine that capture to generated punctuation, but it must not disturb document marker alignment. The `---` marker belongs at the same indentation level as the generated YAML document's top-level keys.

If the hook spike passes, theme guidance for this milestone should be documentation-only: provide a concrete `theme_overrides.syntax` snippet that demonstrates the Clear Hybrid colors for users who want that look. The extension should not ship or maintain a bundled companion theme.

## Visual Behavior

If the hook spike passes, the reference Clear Hybrid override is:

- Outer YAML keeps the user's existing YAML colors.
- Inner generated YAML keys use the same hue family as YAML keys, with a clearer/lighter shade.
- Inner generated YAML scalar values use the same hue family as YAML values, with a clearer/lighter shade.
- Inner generated YAML comments are slightly brighter than normal comments but still subdued.
- Inner document markers use generated punctuation styling.
- No first-milestone block background is required.

If Zed cannot provide generated-YAML token shade differentiation without a distinct injected language, stop and make that tradeoff explicit before implementation. Do not add a new language only to satisfy a visual preference unless the resulting maintenance cost is acceptable.

## Constraints

Zed's syntax highlighting chooses the innermost active capture for each text chunk rather than merging multiple captures into a composited style stack. This means a parent range capture for the generated-YAML block may not reliably combine with child YAML token captures to create both custom token colors and a block background.

Because of that, treat generated-YAML background styling as a future enhancement unless the spike proves it works with token colors.

Zed source indicates arbitrary dotted capture names should fall back through shorter prefixes and theme overrides should be able to introduce custom syntax keys. Still, the spec requires a Zed proof before implementation, because the acceptance criteria depend on visible editor behavior rather than source reading alone.

## Spike Decision Rules

Before writing the implementation plan, run the theme-hook viability spike and choose one outcome:

- **Outcome A: generated range hook.** Use this if one precise generated-range capture gives enough visual distinction when explicitly styled and does not interfere with YAML token highlighting.
- **Outcome B: generated token hooks.** Use this if token shade differences are required and an extension-defined generated-YAML language can reuse or copy YAML behavior safely.
- **Outcome C: no viable hook.** Use this if the extension cannot target generated YAML separately without excessive complexity. In that case, do not ship hook names, do not publish theme guidance, document the limitation, and do not add brittle query tricks.

Do not pursue a background cue as part of Outcome A or B unless the spike proves it composes correctly with token highlighting.

## Validation

Use the committed `fixtures/crossplane-package/api/mixed-template-composition.yaml` sample as the primary validation file. Use `/path/to/external/crossplane-package/api/xtopic-composition.yaml` as a secondary real-world manual sample, especially around the go-template `ExtraResources` document.

Validation should include:

- Tree-sitter query checks showing the selected generated-YAML capture behavior.
- A Zed manual screenshot check with a temporary Clear Hybrid override.
- Confirmation that outer YAML colors are unchanged.
- Confirmation that go-template actions and variables still use template highlighting.
- Confirmation that themes without custom generated-YAML styles still display readable YAML through fallback capture names, even if generated YAML is not visually distinct.
- Existing Rust extension tests and WASM build checks.
- A manual check that generated YAML `---` document markers are aligned with generated YAML top-level keys.

## Open Risks

- A range-level capture may be hidden by injected YAML token captures, leaving no useful visible cue.
- Per-token generated YAML colors may require a distinct injected language, which adds grammar/query maintenance cost.
- Stock themes will likely show no generated-YAML color difference unless they already style the chosen custom captures.
- Zed may not support a reliable generated-YAML block background through syntax captures while also preserving token colors.
