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
- Spike whether extension-owned captures can create clearer generated-YAML contrast under normal Zed themes, without asking users to edit theme settings.
- Prefer token-level shade differences over a block background.
- Keep the YAML document separator `---` aligned with the generated YAML document's top-level keys.

The extension owns semantic boundaries, capture choices, and readable fallback. It does not own the user's palette and should not require user settings changes for generated-YAML contrast. Candidate captures are not a committed implementation path until the spike proves they are precise, stable, and visibly useful in Zed with normal theme settings.

## Non-Goals

- Do not change parsing or diagnostics behavior.
- Do not change how ordinary YAML files are highlighted.
- Do not make generated YAML look like a different language.
- Do not require users to edit settings for generated-YAML contrast.
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

The first step should be a stock-capture viability spike, not a new language and not a committed implementation path:

1. Confirm a generated-range capture can target generated YAML text precisely enough without changing ordinary outer YAML.
2. Confirm whether that capture remains visible when injected YAML token captures are also active.
3. Confirm common Zed capture names, such as `embedded`, `emphasis`, `text.literal`, or existing semantic variants like `string.special`, produce useful contrast under normal theme settings.
4. Confirm the candidate capture choices are generic enough to remain semantically defensible and not theme-specific.

Only if the spike passes should the implementation expose the smallest useful capture behavior using existing Zed capture names. This path avoids a new language, avoids duplicating YAML queries, and avoids new user settings.

If the stock-capture spike fails, do not ship custom hook names or theme guidance. The fallback candidate is a distinct injected language, tentatively named `Crossplane Generated YAML`, but only if the added complexity is justified.

`Crossplane Generated YAML` should reuse YAML parsing behavior but expose different highlight capture names for generated YAML. Example capture names:

- `property` plus a defensible existing variant where Zed supports one
- `string.special`
- `comment.doc`
- `punctuation.special`

The capture names should stay within Zed's documented syntax captures. Avoid custom names such as `property.generated` unless the user explicitly chooses a settings-driven customization surface later.

The distinct-language path is allowed only if the spike confirms:

- Zed can inject into an extension-defined language cleanly.
- The YAML grammar and queries can be reused or copied without losing important YAML behavior.
- The extra maintenance cost is justified by visibly better generated-YAML tokens.

The generated-YAML document marker is the `yaml_document_marker` node from the outer `gotmpl` grammar, not the injected YAML parser. It is already captured in `languages/crossplane-yaml/highlights.scm`. The implementation may refine that capture to generated punctuation, but it must not disturb document marker alignment. The `---` marker belongs at the same indentation level as the generated YAML document's top-level keys.

## Visual Behavior

If the stock-capture spike passes, the reference Clear Hybrid behavior is:

- Outer YAML keeps the user's existing YAML colors.
- Inner generated YAML uses existing theme-defined syntax variants that read as the same general YAML family but are easier to distinguish.
- Inner generated YAML comments remain subdued.
- Inner document markers use generated punctuation styling.
- No first-milestone block background is required.

If Zed cannot provide generated-YAML token shade differentiation without a distinct injected language, stop and make that tradeoff explicit before implementation. Do not add a new language only to satisfy a visual preference unless the resulting maintenance cost is acceptable.

## Constraints

Zed's syntax highlighting chooses the innermost active capture for each text chunk rather than merging multiple captures into a composited style stack. This means a parent range capture for the generated-YAML block may not reliably combine with child YAML token captures to create both custom token colors and a block background.

Because of that, treat generated-YAML background styling as a future enhancement unless the spike proves it works with token colors.

Zed supports documented syntax captures and fallback captures in highlight queries. Still, the spec requires a Zed proof before implementation, because Tree-sitter query output proves capture assignment but not final theme rendering.

## Spike Decision Rules

Before writing the implementation plan, run the stock-capture viability spike and choose one outcome:

- **Outcome A: generated range capture.** Use this if one precise generated-range capture gives enough visual distinction under normal theme settings and does not interfere with YAML token highlighting.
- **Outcome B: generated token captures.** Use this if token shade differences are required and an extension-defined generated-YAML language can reuse or copy YAML behavior safely while staying within documented Zed capture names.
- **Outcome C: no viable extension-owned contrast.** Use this if the extension cannot target generated YAML separately without excessive complexity. In that case, do not ship custom hook names, do not ask users to edit theme settings, document the limitation, and do not add brittle query tricks.

Do not pursue a background cue as part of Outcome A or B unless the spike proves it composes correctly with token highlighting.

## Validation

Use the committed `fixtures/crossplane-package/api/mixed-template-composition.yaml` sample as the primary validation file. Use `/path/to/external/crossplane-package/api/xtopic-composition.yaml` as a secondary real-world manual sample, especially around the go-template `ExtraResources` document.

Validation should include:

- Tree-sitter query checks showing the selected generated-YAML capture behavior.
- A Zed manual screenshot check using normal user settings and the active theme.
- Confirmation that outer YAML colors are unchanged.
- Confirmation that go-template actions and variables still use template highlighting.
- Confirmation that no new user settings are required beyond the existing file association needed to apply `Crossplane YAML`.
- Existing Rust extension tests and WASM build checks.
- A manual check that generated YAML `---` document markers are aligned with generated YAML top-level keys.

## Open Risks

- A range-level capture may be hidden by injected YAML token captures, leaving no useful visible cue.
- Per-token generated YAML colors may require a distinct injected language, which adds grammar/query maintenance cost.
- Existing theme-defined capture variants may not provide enough generated-YAML contrast.
- Zed may not support a reliable generated-YAML block background through syntax captures while also preserving token colors.
