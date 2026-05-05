# Up xpls Zed Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an initial Zed extension that starts `up xpls serve --verbose` for Crossplane package YAML worktrees.

**Architecture:** The extension is a small Rust/WASM Zed extension. It registers `up-xpls` as an additional language server for Zed's built-in `YAML` language, detects Crossplane package worktrees by root `crossplane.yaml` or Upbound project worktrees by root `upbound.yaml`, resolves `up` from the worktree shell environment, and launches `up xpls serve --verbose`.

**Tech Stack:** Zed extension manifest, Rust 2021, `zed_extension_api` 0.7.0, WebAssembly target `wasm32-wasip2`, Upbound `up` CLI.

**2026-05-05 Update:** Upbound's VS Code extension uses the same thin language-client model, gates activation on `crossplane.yaml` or `upbound.yaml`, and runs `up xpls serve --verbose`. Current `up v0.48.0` can panic during function dependency validation; treat a Zed log stack trace under `internal/xpkg/snapshot/meta.go` as an upstream `xpls` failure, not a Zed extension attachment failure.

---

## File Structure

- Create: `extension.toml` - Zed extension metadata and language-server registration.
- Create: `Cargo.toml` - Rust extension crate configured as `cdylib`.
- Create: `src/lib.rs` - Extension implementation, package detection helpers, and unit tests.
- Create: `README.md` - Installation, usage, troubleshooting, and development notes.
- Create: `LICENSE` - Accepted extension license for future Zed registry submission.
- Create: `.gitignore` - Rust build output and editor noise.
- Create: `fixtures/crossplane-package/crossplane.yaml` - Minimal package metadata fixture.
- Create: `fixtures/crossplane-package/apis/example/definition.yaml` - Minimal XRD fixture.
- Create: `fixtures/crossplane-package/apis/example/composition.yaml` - Minimal Composition fixture.
- Create: `fixtures/not-crossplane/config.yaml` - Non-Crossplane YAML fixture.

### Task 0: Repository Setup

**Files:**
- No files changed directly.

- [ ] **Step 1: Initialize Git if needed**

Run:

```bash
git rev-parse --is-inside-work-tree || git init
```

Expected: command exits successfully. If `git init` runs, the repository is initialized in `<local-zed-up-xpls-repo>`.

- [ ] **Step 2: Check the starting tree**

Run:

```bash
git status --short
```

Expected: either no output in a fresh repository or only files intentionally created before implementation starts.

### Task 1: Extension Scaffold

**Files:**
- Create: `extension.toml`
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `.gitignore`
- Create: `LICENSE`

- [ ] **Step 1: Create the Zed manifest**

Write `extension.toml`:

```toml
id = "up-xpls"
name = "Up xpls"
version = "0.0.1"
schema_version = 1
authors = ["Tim Kersten"]
description = "Crossplane package diagnostics powered by up xpls"
repository = "https://github.com/io41/zed-up-xpls-vibe"

[language_servers.up-xpls]
name = "Up xpls"
languages = ["YAML"]
```

- [ ] **Step 2: Create the Rust crate manifest**

Write `Cargo.toml`:

```toml
[package]
name = "zed-up-xpls-vibe"
version = "0.0.1"
edition = "2021"
license = "MIT"

[lib]
crate-type = ["cdylib"]

[dependencies]
zed_extension_api = "0.7.0"
```

- [ ] **Step 3: Create the initial extension implementation**

Write `src/lib.rs`:

```rust
use zed_extension_api::{self as zed, Result};

struct UpXplsExtension;

impl zed::Extension for UpXplsExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(UpXplsExtension);
```

- [ ] **Step 4: Add repository ignore rules**

Write `.gitignore`:

```gitignore
/target/
**/*.wasm
.DS_Store
```

- [ ] **Step 5: Add an accepted extension license**

Write `LICENSE` with the MIT license text:

```text
MIT License

Copyright (c) 2026 Tim Kersten

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 6: Verify the scaffold builds**

Run:

```bash
cargo test
```

Expected: `test result: ok`.

- [ ] **Step 7: Commit**

Run:

```bash
git add extension.toml Cargo.toml src/lib.rs .gitignore LICENSE
git commit -m "feat: scaffold up xpls zed extension"
```

### Task 2: Crossplane Package Detection

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add package detection helper and unit tests**

Replace `src/lib.rs` with:

```rust
use zed_extension_api::{self as zed, Result};

const LANGUAGE_SERVER_ID: &str = "up-xpls";

struct UpXplsExtension;

fn is_crossplane_package_manifest(contents: &str) -> bool {
    let api_version = top_level_scalar(contents, "apiVersion");
    let kind = top_level_scalar(contents, "kind");

    let Some(api_version) = api_version else {
        return false;
    };
    let Some(kind) = kind else {
        return false;
    };

    let is_crossplane_meta = api_version.starts_with("meta.pkg.crossplane.io/")
        || api_version.starts_with("meta.pkg.upbound.io/");
    let is_package_kind = matches!(
        kind.as_str(),
        "Configuration" | "Provider" | "Function" | "AddOn"
    );

    is_crossplane_meta && is_package_kind
}

fn top_level_scalar(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");

    contents.lines().find_map(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            return None;
        }
        if line.len() != trimmed.len() {
            return None;
        }
        let value = trimmed.strip_prefix(&prefix)?.trim();
        Some(value.trim_matches('"').trim_matches('\'').to_string())
    })
}

impl zed::Extension for UpXplsExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(UpXplsExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_crossplane_configuration_manifest() {
        let manifest = r#"
apiVersion: meta.pkg.crossplane.io/v1
kind: Configuration
metadata:
  name: platform-example
"#;

        assert!(is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn detects_upbound_addon_manifest() {
        let manifest = r#"
apiVersion: "meta.pkg.upbound.io/v1beta1"
kind: "AddOn"
metadata:
  name: addon-example
"#;

        assert!(is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn rejects_non_package_yaml() {
        let manifest = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
"#;

        assert!(!is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn ignores_nested_keys() {
        let manifest = r#"
metadata:
  apiVersion: meta.pkg.crossplane.io/v1
  kind: Configuration
"#;

        assert!(!is_crossplane_package_manifest(manifest));
    }
}
```

- [ ] **Step 2: Run tests and verify they pass before integration**

Run:

```bash
cargo test
```

Expected: all four tests pass. These tests define the detection contract before it is wired into Zed.

- [ ] **Step 3: Commit**

Run:

```bash
git add src/lib.rs
git commit -m "test: define crossplane package detection"
```

### Task 3: Language Server Command

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add language server command and command tests**

Replace `src/lib.rs` with:

```rust
use zed_extension_api::{self as zed, Result};

const LANGUAGE_SERVER_ID: &str = "up-xpls";

struct UpXplsExtension;

fn is_crossplane_package_manifest(contents: &str) -> bool {
    let api_version = top_level_scalar(contents, "apiVersion");
    let kind = top_level_scalar(contents, "kind");

    let Some(api_version) = api_version else {
        return false;
    };
    let Some(kind) = kind else {
        return false;
    };

    let is_crossplane_meta = api_version.starts_with("meta.pkg.crossplane.io/")
        || api_version.starts_with("meta.pkg.upbound.io/");
    let is_package_kind = matches!(
        kind.as_str(),
        "Configuration" | "Provider" | "Function" | "AddOn"
    );

    is_crossplane_meta && is_package_kind
}

fn top_level_scalar(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");

    contents.lines().find_map(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            return None;
        }
        if line.len() != trimmed.len() {
            return None;
        }
        let value = trimmed.strip_prefix(&prefix)?.trim();
        Some(value.trim_matches('"').trim_matches('\'').to_string())
    })
}

fn xpls_args() -> Vec<String> {
    vec!["xpls".to_string(), "serve".to_string()]
}

fn missing_up_message() -> String {
    "Could not find the `up` CLI on PATH. Install it with `brew install upbound/tap/up` or `curl -sL https://cli.upbound.io | sh`, then restart Zed from a shell that can run `up xpls serve`."
        .to_string()
}

impl zed::Extension for UpXplsExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Err(format!(
                "Unsupported language server id `{}`",
                language_server_id.as_ref()
            ));
        }

        let manifest = worktree
            .read_text_file("crossplane.yaml")
            .map_err(|_| "No root crossplane.yaml found; up xpls is only started for Crossplane package worktrees.".to_string())?;

        if !is_crossplane_package_manifest(&manifest) {
            return Err("Root crossplane.yaml is not recognized as Crossplane package metadata.".to_string());
        }

        let command = worktree.which("up").ok_or_else(missing_up_message)?;

        Ok(zed::Command {
            command,
            args: xpls_args(),
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(UpXplsExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_crossplane_configuration_manifest() {
        let manifest = r#"
apiVersion: meta.pkg.crossplane.io/v1
kind: Configuration
metadata:
  name: platform-example
"#;

        assert!(is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn detects_upbound_addon_manifest() {
        let manifest = r#"
apiVersion: "meta.pkg.upbound.io/v1beta1"
kind: "AddOn"
metadata:
  name: addon-example
"#;

        assert!(is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn rejects_non_package_yaml() {
        let manifest = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
"#;

        assert!(!is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn ignores_nested_keys() {
        let manifest = r#"
metadata:
  apiVersion: meta.pkg.crossplane.io/v1
  kind: Configuration
"#;

        assert!(!is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn starts_xpls_over_stdio_compatible_command() {
        assert_eq!(xpls_args(), vec!["xpls".to_string(), "serve".to_string()]);
    }

    #[test]
    fn missing_up_message_is_actionable() {
        let message = missing_up_message();
        assert!(message.contains("brew install upbound/tap/up"));
        assert!(message.contains("https://cli.upbound.io"));
    }
}
```

- [ ] **Step 2: Fix any compile error caused by `LanguageServerId` access**

Run:

```bash
cargo test
```

Expected if `LanguageServerId::as_ref()` is unavailable: a compile error naming `as_ref`.

If that happens, inspect the local `zed_extension_api` type and replace both `language_server_id.as_ref()` calls with the supported accessor. In `zed_extension_api` 0.7.0, the expected accessor is `language_server_id.as_ref()`. If the API differs, use the smallest compatible accessor change only.

- [ ] **Step 3: Verify tests pass**

Run:

```bash
cargo test
```

Expected: six tests pass.

- [ ] **Step 4: Build the WASM target**

Run:

```bash
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2
```

Expected: build succeeds and produces `target/wasm32-wasip2/debug/zed_up_xpls_vibe.wasm` or the target-specific library artifact generated by Cargo.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/lib.rs Cargo.lock
git commit -m "feat: launch up xpls language server"
```

### Task 4: Fixtures and Documentation

**Files:**
- Create: `README.md`
- Create: `fixtures/crossplane-package/crossplane.yaml`
- Create: `fixtures/crossplane-package/apis/example/definition.yaml`
- Create: `fixtures/crossplane-package/apis/example/composition.yaml`
- Create: `fixtures/not-crossplane/config.yaml`

- [ ] **Step 1: Add a minimal Crossplane package fixture**

Write `fixtures/crossplane-package/crossplane.yaml`:

```yaml
apiVersion: meta.pkg.crossplane.io/v1
kind: Configuration
metadata:
  name: zed-up-xpls-vibe-fixture
spec:
  crossplane:
    version: ">=v1.17.0"
  dependsOn: []
```

- [ ] **Step 2: Add an XRD fixture**

Write `fixtures/crossplane-package/apis/example/definition.yaml`:

```yaml
apiVersion: apiextensions.crossplane.io/v1
kind: CompositeResourceDefinition
metadata:
  name: xexamples.platform.example.org
spec:
  group: platform.example.org
  names:
    kind: XExample
    plural: xexamples
  versions:
    - name: v1alpha1
      served: true
      referenceable: true
      schema:
        openAPIV3Schema:
          type: object
          properties:
            spec:
              type: object
              properties:
                region:
                  type: string
              required:
                - region
```

- [ ] **Step 3: Add a Composition fixture**

Write `fixtures/crossplane-package/apis/example/composition.yaml`:

```yaml
apiVersion: apiextensions.crossplane.io/v1
kind: Composition
metadata:
  name: xexample-basic
spec:
  compositeTypeRef:
    apiVersion: platform.example.org/v1alpha1
    kind: XExample
  mode: Pipeline
  pipeline:
    - step: render
      functionRef:
        name: function-patch-and-transform
      input:
        apiVersion: pt.fn.crossplane.io/v1beta1
        kind: Resources
        resources: []
```

- [ ] **Step 4: Add a non-Crossplane YAML fixture**

Write `fixtures/not-crossplane/config.yaml`:

```yaml
name: ordinary-yaml
enabled: true
```

- [ ] **Step 5: Add README usage docs**

Write `README.md`:

```markdown
# Up xpls for Zed

Adds Crossplane package diagnostics to Zed by starting the `up xpls serve` language server for YAML files in Crossplane package worktrees.

## Requirements

- Zed
- Rust installed with `rustup` for local development
- Upbound `up` CLI available on PATH

Install `up`:

```bash
brew install upbound/tap/up
```

or:

```bash
curl -sL https://cli.upbound.io | sh
```

Verify:

```bash
up version
up xpls serve --help
```

## Usage

Open a worktree that has a root `crossplane.yaml`, then install this repository with `zed: install dev extension`.

The extension keeps Zed's native YAML support enabled and adds `up xpls serve` as a Crossplane-specific language server.

## Troubleshooting

If `up` cannot be found, start Zed from a shell where `up xpls serve --help` works.

For extension logs, run Zed with:

```bash
zed --foreground
```

or use `zed: open log`.

## Development

```bash
cargo test
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2
```
```

- [ ] **Step 6: Verify docs and fixtures**

Run:

```bash
cargo test
up xpls serve --help
```

Expected: tests pass and `up xpls serve --help` prints the LSP server help text.

- [ ] **Step 7: Commit**

Run:

```bash
git add README.md fixtures
git commit -m "docs: document up xpls zed extension usage"
```

### Task 5: Manual Zed Verification

**Files:**
- No code files changed unless verification exposes a bug.

- [ ] **Step 1: Install the extension as a dev extension**

In Zed, run `zed: install dev extension` and select the repository root.

Expected: Zed accepts the extension manifest.

- [ ] **Step 2: Open the Crossplane fixture package**

Open:

```text
fixtures/crossplane-package
```

Expected: Zed starts `up-xpls` for YAML buffers in the worktree.

- [ ] **Step 3: Confirm normal YAML still works**

Open:

```text
fixtures/not-crossplane/config.yaml
```

Expected: native YAML editing still works. If Zed shows a noisy `up-xpls` startup error for non-Crossplane worktrees, record it and change the implementation to start `up xpls serve` without returning a non-package error, relying on `xpls` to no-op outside package context.

- [ ] **Step 4: Confirm diagnostics path**

Temporarily break `fixtures/crossplane-package/apis/example/definition.yaml` by deleting the `spec.group` field.

Expected: Zed shows a diagnostic from `up-xpls` or logs an `xpls` validation message. Restore the field after the check.

- [ ] **Step 5: Inspect logs**

Run `zed: open log`.

Expected: no repeated restart loop for `up-xpls`.

- [ ] **Step 6: Commit verification fixes only if needed**

If code changed during verification:

```bash
git add src/lib.rs README.md fixtures
git commit -m "fix: refine up xpls startup behavior"
```

If no code changed, do not create a commit.

### Task 6: Publishing Readiness Check

**Files:**
- Modify: `README.md` if publishing notes need correction.
- Modify: `extension.toml` if repository metadata changes.

- [ ] **Step 1: Validate extension id and metadata**

Check `extension.toml`:

```toml
id = "up-xpls"
name = "Up xpls"
version = "0.0.1"
schema_version = 1
```

Expected: id does not contain `zed` or `extension`, and metadata is ready for registry review.

- [ ] **Step 2: Confirm no language server binary is bundled**

Run:

```bash
find . -type f | sort
```

Expected: no `up`, `xpls`, or other language-server binary exists in the repository.

- [ ] **Step 3: Final verification**

Run:

```bash
cargo test
cargo build --target wasm32-wasip2
```

Expected: both commands pass.

- [ ] **Step 4: Commit metadata fixes if needed**

If files changed:

```bash
git add README.md extension.toml
git commit -m "docs: prepare extension for zed registry"
```

If no files changed, do not create a commit.

## Acceptance Criteria

- `cargo test` passes.
- `cargo build --target wasm32-wasip2` passes.
- Zed can install the repository as a dev extension.
- In a worktree with root `crossplane.yaml`, opening YAML starts `up xpls serve`.
- Missing `up` produces an actionable error.
- Normal YAML syntax support remains native Zed YAML.
- The repository does not bundle `up` or any other language-server binary.

## Known Risk

Registering `up-xpls` against the built-in `YAML` language may cause Zed to ask for the server in non-Crossplane YAML worktrees. The MVP should test how noisy this is. If it is disruptive, change the startup policy to either let `xpls` start and no-op or introduce a separate `Crossplane YAML` language with conservative first-line patterns.
