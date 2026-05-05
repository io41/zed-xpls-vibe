# Crossplane YAML Long-Term Design Spec

**Status:** Deferred. This spec is intentionally not part of the short-term implementation plan.

**Workspace:** `<local-zed-up-xpls-repo>`

**Goal:** Define the future Crossplane-aware authoring experience that would require language-server or language-server-proxy work beyond basic syntax highlighting.

## Deferred Scope Statement

This work is deferred because the current extension should first install reliably, classify Crossplane YAML predictably, and provide stable syntax highlighting. The long-term work depends on a stable semantic backend and should not be mixed into the short-term grammar and query changes.

## Problem

Crossplane Go templates are more than generic Go templates:

- `.` is a Crossplane `RunFunctionRequest`, not an arbitrary Go template data object.
- `.observed`, `.desired`, `.context`, and `.extraResources` have Crossplane-specific structure.
- XRD schemas define the shape of `.observed.composite.resource.spec` and related fields.
- Provider and function schemas define the shape of composed resources emitted by the template.
- Crossplane helper functions add domain semantics beyond Go built-ins and Sprig.

Syntax highlighting cannot provide completions, hovers, schema-aware diagnostics, or navigation for those concepts.

## Desired Future Experience

The future experience should provide:

- Completion for Crossplane helper functions and Sprig functions.
- Hover documentation for helper functions.
- Completion for `.observed.composite.resource`, `.desired.composite.resource`, `.desired.composed`, `.context`, and `.extraResources`.
- XRD-derived completion for XR fields.
- Provider-schema-derived completion for composed resources.
- Diagnostics for invalid template syntax.
- Diagnostics for references to fields that are not present in the inferred schema.
- Diagnostics that clear reliably after fixes.
- Optional commands or runnables for `crossplane render`, `crossplane beta validate`, or equivalent `up` flows.
- Content-based language classification for any YAML file with a top-level or early `apiVersion` containing `.crossplane.io/`, if Zed gains a detector hook that can outrank the built-in YAML suffix match.

## Candidate Architectures

### Option A: Improve `up xpls`

Extend or upstream fixes into `up xpls` so it becomes the authoritative Crossplane LSP.

Pros:

- Aligns with Upbound's VS Code extension.
- Avoids a competing semantic implementation.
- Keeps package dependency, XRD, and composition validation in one place.

Cons:

- Requires upstream changes outside this Zed extension.
- Current `up v0.48.0` can panic while validating package metadata.
- Release cadence and behavior are controlled by Upbound.

### Option B: Build a Crossplane Template LSP

Build a dedicated LSP for Crossplane Go-template authoring, then attach it alongside or instead of `up-xpls`.

Pros:

- Can focus specifically on template semantics and diagnostics.
- Can be designed to clear diagnostics reliably.
- Can read `.up/json/models`, XRD schemas, and local package files directly.

Cons:

- Large maintenance burden.
- Must model Crossplane, provider schemas, function inputs, Sprig, and helper functions.
- Risk of overlapping or conflicting diagnostics with `up-xpls`.

### Option C: Build an LSP Proxy Around `up xpls`

Run a local proxy language server that starts `up xpls`, forwards LSP traffic, and adds reliability behavior or extra Crossplane-template features.

Pros:

- Can clear diagnostics when the child `up xpls` process exits.
- Can restart `xpls` under controlled conditions.
- Can incrementally add template-specific features without replacing `xpls`.

Cons:

- More moving parts than a normal Zed extension.
- Proxying LSP correctly is non-trivial.
- Packaging an external proxy binary for Zed has platform and update implications.

## Recommended Long-Term Direction

Prefer Option A first: improve or track `up xpls` because it is already the Crossplane language server used by Upbound's editor integration.

If upstream `xpls` cannot provide stable template-aware features, evaluate Option C before Option B. A proxy can solve reliability and extension-specific behavior while still preserving upstream diagnostics.

Only build a standalone Crossplane Template LSP if the desired semantic features cannot reasonably land in `xpls` and the maintenance burden is acceptable.

## Deferred Research Tasks

- Map the current `up xpls` LSP capabilities.
- Reproduce and minimize the `VersionValidator` panic in `up v0.48.0`.
- Check whether `up xpls` can publish semantic tokens, completion, hover, or only diagnostics.
- Evaluate generic Go-template LSPs against Crossplane templates.
- Inspect `.up/json/models` structure produced by `up project build`.
- Define a schema inference model from XRD plus Composition pipeline context.
- Decide whether Zed extension packaging can reasonably ship or download an external proxy binary.
- Track whether Zed adds extension-provided language detection beyond `path_suffixes` and `first_line_pattern`, such as full-file or first-N-lines content matchers.

## Non-Goals

- Do not implement this work as part of the short-term grammar fix.
- Do not replace `up-xpls` until its current behavior and capability limits are fully understood.
- Do not introduce a proxy language server without a separate implementation plan.

## Acceptance Criteria For Future Activation

This deferred spec can move to active status only when:

- The short-term extension installs and highlights reliably.
- There is a documented capability matrix for `up xpls`.
- There is a minimized upstream `xpls` crash reproduction or an upstream fix.
- The team chooses one of the candidate architectures explicitly.
- A separate implementation plan is written and reviewed.
