# Crossplane Generated YAML Stock Capture Spike Results
Date: 2026-05-19

## Decision

Outcome C: no viable extension-owned contrast

## Tree-Sitter Evidence

The temporary stock-capture query used documented captures `@text.literal @embedded` on generated text ranges. The query passed, and captures appeared at generated YAML rows around 23, 37, 45, and 49.

Outer YAML rows 0-19 were not incorrectly captured. Existing template captures were preserved, including `function.builtin`, `variable`, and `punctuation.bracket`.

Task 3 removed the temporary query block. `rg -n "SPIKE ONLY|@embedded|@text.literal" languages/crossplane-yaml/highlights.scm` produced no output, and `git diff -- languages/crossplane-yaml/highlights.scm` produced no output.

## Zed Visual Evidence

Zed was validated with `zed-xpls-vibe` installed as a dev extension. `Crossplane YAML` was registered, files opened, and the LSP started with `<temporary-vibe-xpls-binary> serve`.

The active Zed theme was inferred as `Monokai Pro`: Zed settings use `mode: system` with `dark: "Monokai Pro"`, and macOS reported Dark mode during follow-up verification.

Syntax highlighting still works, and generated `---` alignment is correct. The generated `---` token is sometimes red and sometimes green. There is no visible distinction between inner generated YAML and outer YAML, so the captured generated ranges did not provide a viable extension-owned contrast.

No new Zed user settings were added for this spike.

## Follow-Up

This spike does not propose user theme settings as a deliverable. A future implementation should avoid relying on stock captures alone for generated YAML contrast unless Zed exposes a reliable extension-owned styling path.
