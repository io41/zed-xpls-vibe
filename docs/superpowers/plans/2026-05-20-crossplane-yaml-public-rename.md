# Crossplane YAML Public Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the pre-public Zed extension identity from `zed-xpls-vibe` to `crossplane-yaml`, including the GitHub repository, without changing Crossplane YAML highlighting or `vibe-xpls` resolver behavior.

**Architecture:** Treat the rename as a clean identity migration before Zed registry publication. Keep runtime behavior in `src/lib.rs` and `src/resolver.rs`, keep packaging identity in `extension.toml`/Cargo/Release Please, and keep user-facing instructions in README/active specs. Perform the GitHub repository rename only after local code and docs are verified.

**Tech Stack:** Rust Zed extension API `0.7.0`, Cargo, Zed extension manifest TOML, Release Please, GitHub CLI/API, Markdown docs.

---

## File Structure

- Modify `extension.toml`: public extension id/name/repository and language-server table.
- Modify `Cargo.toml`: Rust package name.
- Modify `Cargo.lock`: package lock entry after the Cargo package rename.
- Modify `src/lib.rs`: language-server id, extension type name, and tests.
- Modify `src/resolver.rs`: user-facing old settings-key strings and tests.
- Modify `README.md`: public name, settings examples, troubleshooting, and registry publishing instructions.
- Modify `AGENTS.md`: future-agent guardrails for the new id and old-id regression prevention.
- Modify `release-please-config.json`: package identity for future Release Please PRs.
- Modify `.github/workflows/dev-build.yml`: artifact name.
- Modify `docs/superpowers/specs/2026-05-20-vibe-xpls-download-and-version-policy-design.md`: active policy doc settings keys and examples.
- Use GitHub API to rename `io41/zed-xpls-vibe` to `io41/crossplane-yaml`.
- Update local `origin` remote to `https://github.com/io41/crossplane-yaml.git`.

## Task 0: Preconditions And Baseline

**Files:**
- Read: `docs/superpowers/specs/2026-05-20-crossplane-yaml-public-rename-design.md`
- Read: `extension.toml`
- Read: `Cargo.toml`
- Read: `release-please-config.json`
- Read: `.github/workflows/dev-build.yml`

- [ ] **Step 1: Confirm working tree state**

Run:

```bash
git status --short --branch
```

Expected:

```text
## main...origin/main [ahead 2]
?? .superpowers/
```

The ahead count may be higher if the spec or plan has already been pushed or amended. The important condition is that there are no tracked modifications before implementation starts, and `.superpowers/` remains untracked.

- [ ] **Step 2: Verify the new Zed registry id is unclaimed**

Run:

```bash
gh api repos/zed-industries/extensions/contents/extensions.toml --jq '.content' \
  | base64 --decode \
  | rg -n '^\[crossplane-yaml\]|crossplane-yaml|zed-xpls-vibe'
```

Expected: no output and exit code `1` from `rg`.

If the command prints a `crossplane-yaml` or `zed-xpls-vibe` entry, stop before any rename work.

- [ ] **Step 3: Verify the old repository has no releases or tags**

Run:

```bash
git tag --list
```

Expected: no output.

Run:

```bash
gh release list --repo io41/zed-xpls-vibe --limit 20
```

Expected: no output.

- [ ] **Step 4: Verify the existing Release Please PR is still unmerged**

Run:

```bash
gh pr view 1 --repo io41/zed-xpls-vibe --json state,mergedAt,headRefName,title
```

Expected JSON fields:

```json
{
  "state": "OPEN",
  "mergedAt": null,
  "headRefName": "release-please--branches--main--components--zed-xpls-vibe",
  "title": "chore: release 0.0.2"
}
```

If this PR is merged or absent, stop and re-evaluate Release Please state before continuing.

- [ ] **Step 5: Run baseline formatting, tests, and build**

Run:

```bash
cargo fmt --check
cargo test
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected:

- `cargo fmt --check` exits `0`.
- `cargo test` reports all tests passing.
- `cargo build --target wasm32-wasip2` exits `0`.

## Task 1: Runtime And Manifest Identity Rename

**Files:**
- Modify: `extension.toml`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs`
- Modify: `src/resolver.rs`

- [ ] **Step 1: Update the failing Rust tests first**

In `src/lib.rs`, update the identity assertions to expect `crossplane-yaml`:

```rust
#[test]
fn uses_unique_language_server_id() {
    assert_eq!(LANGUAGE_SERVER_ID, "crossplane-yaml");
}
```

Also update the download error assertion to expect the new language-server id:

```rust
assert!(message.contains("Could not download vibe-xpls v0.0.2 for crossplane-yaml."));
```

Run:

```bash
cargo test tests::uses_unique_language_server_id
cargo test tests::download_error_sanitizes_github_json
```

Expected: FAIL because `LANGUAGE_SERVER_ID` and error strings still use `zed-xpls-vibe`.

- [ ] **Step 2: Rename the extension manifest**

Update `extension.toml` to this identity while preserving the existing version, authors, grammar, and language label:

```toml
id = "crossplane-yaml"
name = "Crossplane YAML"
version = "0.0.1"
schema_version = 1
authors = ["Tim Kersten"]
description = "Crossplane package diagnostics and Crossplane YAML highlighting powered by vibe-xpls"
repository = "https://github.com/io41/crossplane-yaml"
languages = ["languages/crossplane-yaml"]

[grammars.gotmpl]
repository = "https://github.com/io41/tree-sitter-go-template"
rev = "19d1900dad709d8746cf978d1c561a2b5e075d2b"

[language_servers.crossplane-yaml]
name = "Crossplane YAML"
languages = ["Crossplane YAML"]
```

- [ ] **Step 3: Rename the Cargo package**

Update `Cargo.toml`:

```toml
[package]
name = "crossplane-yaml"
version = "0.0.1"
edition = "2021"
license = "MIT"

[lib]
crate-type = ["cdylib"]

[dependencies]
zed_extension_api = "0.7.0"
```

Run:

```bash
cargo test --no-run
```

Expected: compilation succeeds and `Cargo.lock` updates its package entry from `zed-xpls-vibe` to `crossplane-yaml`.

- [ ] **Step 4: Rename runtime ids and internal extension type**

In `src/lib.rs`, update the constant and internal type:

```rust
const LANGUAGE_SERVER_ID: &str = "crossplane-yaml";

struct CrossplaneYamlExtension {
    cached_downloaded_binary: Option<String>,
}
```

Rename all impl blocks and the registration call:

```rust
impl CrossplaneYamlExtension {
```

```rust
impl zed::Extension for CrossplaneYamlExtension {
```

```rust
zed::register_extension!(CrossplaneYamlExtension);
```

- [ ] **Step 5: Rename resolver-facing settings hints**

In `src/resolver.rs`, update user-facing messages so they mention `lsp.crossplane-yaml.binary.path` and `crossplane-yaml`:

```rust
"Could not verify vibe-xpls at {path}. {}\n\nInstall the pinned server with:\n{}\n\nOr configure lsp.crossplane-yaml.binary.path if you intentionally want to use a different server version."
```

```rust
"Found vibe-xpls {found} at {path}, but crossplane-yaml requires vibe-xpls {VIBE_XPLS_VERSION}.\n\nInstall the pinned server with:\n{}\n\nOr configure lsp.crossplane-yaml.binary.path if you intentionally want to use a different server version."
```

- [ ] **Step 6: Run focused tests**

Run:

```bash
cargo test resolver::tests::go_bin_mismatch_hard_fails
cargo test resolver::tests::path_lookup_mismatch_hard_fails_before_go_bin
cargo test tests::uses_unique_language_server_id
cargo test tests::download_error_sanitizes_github_json
```

Expected: all selected tests pass.

- [ ] **Step 7: Run formatting and full Rust tests**

Run:

```bash
cargo fmt --check
cargo test
```

Expected: formatting is clean and all tests pass.

- [ ] **Step 8: Commit runtime rename**

Run:

```bash
git add extension.toml Cargo.toml Cargo.lock src/lib.rs src/resolver.rs
git commit -m "refactor: rename extension identity to crossplane yaml"
```

Expected: commit succeeds with author and committer `Tim Kersten <tim@io41.com>`.

## Task 2: Public Docs, Guardrails, Release Automation, And Workflow Rename

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`
- Modify: `release-please-config.json`
- Modify: `.github/workflows/dev-build.yml`
- Modify: `docs/superpowers/specs/2026-05-20-vibe-xpls-download-and-version-policy-design.md`

- [ ] **Step 1: Update README public identity and settings examples**

In `README.md`, make these replacements in future-facing content:

```text
# Zed xpls Vibe
```

becomes:

```text
# Crossplane YAML
```

```text
lsp.zed-xpls-vibe.binary.path
```

becomes:

```text
lsp.crossplane-yaml.binary.path
```

```jsonc
"zed-xpls-vibe": {
```

becomes:

```jsonc
"crossplane-yaml": {
```

```text
`zed-xpls-vibe` runs for `Crossplane YAML` files
```

becomes:

```text
`crossplane-yaml` runs for `Crossplane YAML` files
```

```text
extensions/zed-xpls-vibe
```

becomes:

```text
extensions/crossplane-yaml
```

Also update any public repository URL in README to:

```text
https://github.com/io41/crossplane-yaml
```

- [ ] **Step 2: Update AGENTS guardrails**

Replace the old id-specific bullets in `AGENTS.md` with these bullets:

```markdown
- Keep the extension id and language server id as `crossplane-yaml`; do not change them back to `zed-xpls-vibe` or `up-xpls`.
- The extension starts the `vibe-xpls` language server with the default argument `serve`.
- Do not reintroduce the `up xpls serve` fallback or a `VIBE_XPLS_BIN` environment override.
- Do not add a default gh fallback for installing or resolving the language server.
- The public extension resolves `vibe-xpls` in this order: Zed `lsp.crossplane-yaml.binary.path` as an explicit user override, shell `PATH` with pinned-version compatibility check, standard Go bin directories with pinned-version compatibility check, then the pinned `io41/vibe-xpls` GitHub release recorded in the source.
- The pinned auto-download path must use the direct pinned release asset URL with `zed::download_file`; do not add GitHub release API lookup behavior for that path.
- Rust tests must preserve the extension id, language server id, resolver order, local binary version checks, explicit override behavior, pinned release behavior, and default `serve` argument.
- Local milestone validation with `<temporary-vibe-xpls-binary>` is development-only. If it is needed for a one-off manual check, keep it out of public README usage and do not hardcode it as the production path.
- Zed manual validation should install this repository as a dev extension, not the original `up-xpls` extension.
```

- [ ] **Step 3: Update Release Please package identity**

In `release-please-config.json`, change the package name:

```json
"package-name": "crossplane-yaml"
```

Keep `.release-please-manifest.json` unchanged unless Release Please itself changes it in a later release PR; the manifest maps the root package path to version `0.0.1`, not the package name.

- [ ] **Step 4: Update Dev Build artifact name**

In `.github/workflows/dev-build.yml`, update the artifact name:

```yaml
name: crossplane-yaml-wasm
```

- [ ] **Step 5: Update the active download/version policy spec**

In `docs/superpowers/specs/2026-05-20-vibe-xpls-download-and-version-policy-design.md`, update future-facing id and settings examples:

```text
lsp.zed-xpls-vibe.binary.path
```

becomes:

```text
lsp.crossplane-yaml.binary.path
```

```text
for zed-xpls-vibe
```

becomes:

```text
for crossplane-yaml
```

```text
zed-xpls-vibe 0.0.1 requires
```

becomes:

```text
crossplane-yaml 0.0.2 requires
```

Remove or rephrase the reference to `docs/superpowers/specs/2026-05-19-zed-xpls-vibe-public-release-design.md` so the active policy doc does not contain `zed-xpls-vibe`.

Also update future-facing pinned language-server examples in that active policy
doc from `v0.0.1` to the current source pin `v0.0.2`, matching
`src/resolver.rs::VIBE_XPLS_VERSION`. Historical background text that quotes the
original `v0.0.1` rate-limit failure may remain if clearly framed as history,
but the policy, expected version output, install hints, asset examples, and
error examples should describe `v0.0.2`.

- [ ] **Step 6: Run active-surface old-name grep**

Run:

```bash
rg -n 'zed-xpls-vibe|Zed xpls Vibe' \
  Cargo.toml Cargo.lock extension.toml src README.md AGENTS.md \
  release-please-config.json .github/workflows \
  docs/superpowers/specs/2026-05-20-vibe-xpls-download-and-version-policy-design.md
```

Expected: no output and exit code `1`.

- [ ] **Step 7: Run docs and config sanity checks**

Run:

```bash
python3 -c 'import tomllib; tomllib.load(open("extension.toml", "rb")); tomllib.load(open("languages/crossplane-yaml/config.toml", "rb")); print("toml ok")'
git diff --check
cargo fmt --check
cargo test
```

Expected:

- TOML command prints `toml ok`.
- whitespace check exits `0`.
- formatting is clean.
- all tests pass.

- [ ] **Step 8: Commit docs and automation rename**

Run:

```bash
git add README.md AGENTS.md release-please-config.json .github/workflows/dev-build.yml docs/superpowers/specs/2026-05-20-vibe-xpls-download-and-version-policy-design.md
git commit -m "docs: rename public extension references"
```

Expected: commit succeeds with author and committer `Tim Kersten <tim@io41.com>`.

## Task 3: Full Local Verification Before GitHub Rename

**Files:**
- Read: all modified files

- [ ] **Step 1: Verify repository state before remote operations**

Run:

```bash
git status --short --branch
git log -3 --format='%h %an <%ae> %cn <%ce> %s'
```

Expected:

- no tracked modifications
- `.superpowers/` remains untracked
- recent commits are authored and committed by `Tim Kersten <tim@io41.com>`

- [ ] **Step 2: Run full local verification**

Run:

```bash
python3 -c 'import tomllib; tomllib.load(open("extension.toml", "rb")); tomllib.load(open("languages/crossplane-yaml/config.toml", "rb")); print("toml ok")'
cargo fmt --check
cargo test
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
git diff-tree --check --no-commit-id -r HEAD
```

Expected:

- TOML command prints `toml ok`.
- formatting is clean.
- all tests pass.
- WASM build exits `0`.
- commit whitespace check exits `0`.

- [ ] **Step 3: Verify old name only remains in historical docs and the rename spec**

Run:

```bash
rg -n 'zed-xpls-vibe|Zed xpls Vibe' \
  Cargo.toml Cargo.lock extension.toml src README.md AGENTS.md \
  release-please-config.json .github/workflows \
  docs/superpowers/specs/2026-05-20-vibe-xpls-download-and-version-policy-design.md
```

Expected: no output and exit code `1`.

Run:

```bash
rg -n 'zed-xpls-vibe|Zed xpls Vibe' docs/superpowers/specs/2026-05-20-crossplane-yaml-public-rename-design.md
```

Expected: output is allowed because the rename spec describes the source identity.

## Task 4: Rename GitHub Repository, Remote, And Release Please PR State

**Files:**
- Modify GitHub repository metadata.
- Modify local git remote configuration.
- No repository files should be edited in this task.

- [ ] **Step 1: Re-run preflight immediately before remote rename**

Run:

```bash
gh api repos/zed-industries/extensions/contents/extensions.toml --jq '.content' \
  | base64 --decode \
  | rg -n '^\[crossplane-yaml\]|crossplane-yaml|zed-xpls-vibe'
git tag --list
gh release list --repo io41/zed-xpls-vibe --limit 20
```

Expected:

- registry grep prints no output
- `git tag --list` prints no output
- `gh release list` prints no output

- [ ] **Step 2: Rename the GitHub repository**

Run:

```bash
gh api --method PATCH repos/io41/zed-xpls-vibe -f name=crossplane-yaml
```

Expected: JSON output for repository `io41/crossplane-yaml`.

- [ ] **Step 3: Update local `origin` remote**

Run:

```bash
git remote set-url origin https://github.com/io41/crossplane-yaml.git
git remote -v
```

Expected:

```text
origin	https://github.com/io41/crossplane-yaml.git (fetch)
origin	https://github.com/io41/crossplane-yaml.git (push)
```

- [ ] **Step 4: Verify the renamed public repository**

Run:

```bash
gh repo view io41/crossplane-yaml --json nameWithOwner,visibility,defaultBranchRef,url
```

Expected JSON fields:

```json
{
  "nameWithOwner": "io41/crossplane-yaml",
  "visibility": "PUBLIC",
  "defaultBranchRef": { "name": "main" },
  "url": "https://github.com/io41/crossplane-yaml"
}
```

- [ ] **Step 5: Push local commits to the renamed repository**

Run:

```bash
git push origin main
```

Expected: push succeeds to `https://github.com/io41/crossplane-yaml.git`.

- [ ] **Step 6: Close the old Release Please PR if still open**

Run:

```bash
gh pr view 1 --repo io41/crossplane-yaml --json number,state,headRefName,title,url
```

If PR #1 is open and `headRefName` contains `zed-xpls-vibe`, run:

```bash
gh pr close 1 --repo io41/crossplane-yaml --comment "Closed after the repository and package rename to crossplane-yaml. Release Please should create a fresh release PR for the new package identity."
```

Expected: the old PR is closed, or the command is skipped because it no longer exists or no longer refers to the old identity.

## Task 5: Post-Push CI, Release Automation, And Dev Extension Validation

**Files:**
- Read: GitHub Actions runs.
- Read: Zed logs if manual validation is performed.

- [ ] **Step 1: Watch post-push workflows**

Run:

```bash
gh run list --repo io41/crossplane-yaml --limit 5 --json databaseId,workflowName,event,status,conclusion,headBranch,url
```

Expected: `CI`, `Release`, and `Dev Build` runs appear for the push to `main`.

For any in-progress run, use:

```bash
gh run list --repo io41/crossplane-yaml --limit 5 --json databaseId,status \
  --jq '.[] | select(.status == "in_progress") | .databaseId'
```

Expected: prints the database ids of any in-progress runs.

For each printed database id, run `gh run watch` with that concrete id. Example
for run `26160188002`:

```bash
gh run watch 26160188002 --repo io41/crossplane-yaml --exit-status
```

Expected: each watched run exits `0`.

- [ ] **Step 2: Verify Release Please recreated or did not recreate a PR under the new identity**

Run:

```bash
gh pr list --repo io41/crossplane-yaml --state open --json number,title,headRefName,baseRefName,author,url
```

Expected:

- no open old-id PR with `zed-xpls-vibe` in `headRefName`
- if a Release Please PR exists, it is for the current `main` branch state and new `crossplane-yaml` package identity

- [ ] **Step 3: Perform feasible CLI-side Zed checks**

Run:

```bash
zed --version
```

Expected: Zed CLI prints a version.

Run:

```bash
command -v vibe-xpls || true
```

Expected: output depends on local setup. If a path is printed, run:

```bash
vibe-xpls --version
```

Expected for an auto-discovered server: `vibe-xpls v0.0.2`.

- [ ] **Step 4: Manual Zed dev-extension validation**

In Zed, install this repository as a dev extension from the renamed local repository path:

```text
<local checkout path>
```

Open a file classified as `Crossplane YAML`, for example:

```text
<Crossplane YAML sample file>
```

Expected:

- Zed shows the extension as `Crossplane YAML`.
- The `Crossplane YAML` language remains available.
- The YAML and Go-template highlighting behavior is unchanged.
- The language server starts with id `crossplane-yaml`.

If the Zed UI cannot be validated in this session, record that manual validation remains pending and do not claim it completed.

- [ ] **Step 5: Final repository status check**

Run:

```bash
git status --short --branch
git log -5 --format='%h %an <%ae> %cn <%ce> %s'
gh repo view io41/crossplane-yaml --json nameWithOwner,visibility,url,defaultBranchRef
```

Expected:

- branch is synced with `origin/main`
- only `.superpowers/` remains untracked
- recent commits use `Tim Kersten <tim@io41.com>`
- GitHub repo is public at `https://github.com/io41/crossplane-yaml`

## Task 6: Registry Publishing Follow-Up

**Files:**
- No files in this repository unless the user asks to submit the Zed registry PR immediately.

- [ ] **Step 1: Report the registry PR command sequence**

Do not open a `zed-industries/extensions` PR unless the user asks for it. Report that the next publishing step is:

```bash
git clone https://github.com/zed-industries/extensions.git
cd extensions
git submodule add https://github.com/io41/crossplane-yaml.git extensions/crossplane-yaml
```

Then add:

```toml
[crossplane-yaml]
submodule = "extensions/crossplane-yaml"
version = "0.0.1"
```

Then run:

```bash
pnpm sort-extensions
```

Expected: this prepares the Zed registry PR after the repository rename is complete and verified.
