# Reviewable vibe-xpls Bump Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a manual, reviewable maintenance workflow that updates the pinned `vibe-xpls` version and archive digests for one explicit release.

**Architecture:** Refactor tests first so version-bearing expectations derive from `VIBE_XPLS_VERSION`. Add a checked-in shell script that owns the source update logic, then add a thin GitHub Actions wrapper that validates input, runs the script, verifies the extension, and opens a PR.

**Tech Stack:** Rust tests, Bash, `perl` for cross-platform source rewrites, GitHub CLI, GitHub Actions, `dtolnay/rust-toolchain@stable`.

---

## File Structure

- Modify `src/resolver.rs`: test helpers and version-derived assertions; runtime digest table remains the source of truth.
- Modify `src/lib.rs`: test helpers and version-derived friendly-error assertions.
- Create `scripts/bump-vibe-xpls-release.sh`: validates one tag, downloads or reads `checksums.txt`, validates the exact asset set, and rewrites `src/resolver.rs`.
- Create `.github/workflows/bump-vibe-xpls.yml`: manual workflow that calls the script, validates, commits to a review branch, uploads audit artifacts, and opens a PR.
- Modify `docs/superpowers/README.md`: point pending work at this implementation plan while active, then clear the active-plan note at the end.

## Task 1: Refactor Version-Hardcoded Rust Tests

**Files:**
- Modify: `src/resolver.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Run the current tests as a baseline**

Run:

```bash
cargo test
```

Expected: all existing tests pass before the refactor.

- [ ] **Step 2: Add test helpers in `src/resolver.rs`**

Inside `#[cfg(test)] mod tests`, after `impl FakeLookup`, add helpers like:

```rust
fn pinned_version_output() -> String {
    format!("{VIBE_XPLS_BIN} {VIBE_XPLS_VERSION}\n")
}

fn pinned_asset_name(os: &str, arch: &str, extension: &str) -> String {
    format!("vibe-xpls_{VIBE_XPLS_VERSION}_{os}_{arch}.{extension}")
}

fn pinned_version_dir() -> String {
    format!("vibe-xpls-{VIBE_XPLS_VERSION}")
}

fn pinned_temp_dir() -> String {
    format!("{}.tmp", pinned_version_dir())
}
```

Then change `FakeLookup::matching_version()` to use `pinned_version_output()` instead of the literal `"vibe-xpls v0.0.2\n"`.

- [ ] **Step 3: Replace version literals in `src/resolver.rs` tests**

Change version-bearing assertions to derive from the helpers and existing runtime functions:

```rust
let asset_name = pinned_asset_name("darwin", "arm64", "tar.gz");
assert_eq!(plan.asset_name, asset_name);
assert_eq!(plan.download_url, release_asset_url(&asset_name));
assert_eq!(plan.version_dir, pinned_version_dir());
assert_eq!(plan.temp_dir, pinned_temp_dir());
assert_eq!(plan.binary_path, format!("{}/vibe-xpls", pinned_version_dir()));
assert_eq!(
    plan.temp_archive_path,
    format!("{}/{}", pinned_temp_dir(), asset_name)
);
```

For error assertions, use:

```rust
assert!(error.contains(&format!("requires vibe-xpls {VIBE_XPLS_VERSION}")));
assert!(error.contains(&manual_install_hint()));
assert!(error.contains(&format!("expected `vibe-xpls {VIBE_XPLS_VERSION}`")));
```

Digest literals in `all_supported_assets_have_valid_sha256_digests` remain explicit, but asset names derive from `pinned_asset_name(...)`.

- [ ] **Step 4: Replace version literals in `src/lib.rs` tests**

Inside `#[cfg(test)] mod tests`, add:

```rust
fn pinned_asset_name(os: &str, arch: &str, extension: &str) -> String {
    format!("vibe-xpls_{VIBE_XPLS_VERSION}_{os}_{arch}.{extension}")
}
```

Then replace hardcoded friendly-error expectations with derived strings:

```rust
let asset_name = pinned_asset_name("darwin", "arm64", "tar.gz");
let message = friendly_download_error(&asset_name, "status error 403, response: ...");
assert!(message.contains(&format!(
    "Could not download vibe-xpls {VIBE_XPLS_VERSION} for crossplane-yaml."
)));
assert!(message.contains(&manual_install_hint()));
```

For `which_on_env_path` fake probes, use:

```rust
stdout: format!("vibe-xpls {VIBE_XPLS_VERSION}\n"),
```

- [ ] **Step 5: Verify the refactor removed source test pinning**

Run:

```bash
rg -n 'v0\.0\.2' src
cargo test
```

Expected: `rg` finds only the runtime `VIBE_XPLS_VERSION` constant and digest-related release fixtures if any remain; `cargo test` passes.

- [ ] **Step 6: Commit the refactor**

Run:

```bash
git add src/resolver.rs src/lib.rs
git commit -m "test: derive vibe-xpls version expectations"
```

## Task 2: Add the Local Bump Script

**Files:**
- Create: `scripts/bump-vibe-xpls-release.sh`

- [ ] **Step 1: Create the script with strict argument validation**

Create `scripts/bump-vibe-xpls-release.sh` with:

```bash
#!/usr/bin/env bash
set -euo pipefail

repo="io41/vibe-xpls"
checksums_file=""
artifact_dir="${ARTIFACT_DIR:-.tmp/vibe-xpls-bump}"

usage() {
  printf 'Usage: %s [--checksums-file PATH] vX.Y.Z\n' "$0" >&2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --checksums-file)
      checksums_file="${2:-}"
      [[ -n "$checksums_file" ]] || { usage; exit 2; }
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --*)
      printf 'Unknown option: %s\n' "$1" >&2
      usage
      exit 2
      ;;
    *)
      version="$1"
      shift
      [[ $# -eq 0 ]] || { usage; exit 2; }
      ;;
  esac
done

version="${version:-}"
[[ "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  printf 'Expected stable SemVer tag like v0.0.3, got: %s\n' "$version" >&2
  exit 2
}
```

- [ ] **Step 2: Download or copy `checksums.txt`**

Add:

```bash
mkdir -p "$artifact_dir"
downloaded_checksums="$artifact_dir/checksums.txt"

if [[ -n "$checksums_file" ]]; then
  cp "$checksums_file" "$downloaded_checksums"
else
  gh release download "$version" \
    --repo "$repo" \
    --pattern checksums.txt \
    --output "$downloaded_checksums" \
    --clobber
fi
```

- [ ] **Step 3: Validate the exact six asset checksums**

Add a Bash associative array parser:

```bash
expected_assets=(
  "vibe-xpls_${version}_darwin_amd64.tar.gz"
  "vibe-xpls_${version}_darwin_arm64.tar.gz"
  "vibe-xpls_${version}_linux_amd64.tar.gz"
  "vibe-xpls_${version}_linux_arm64.tar.gz"
  "vibe-xpls_${version}_windows_amd64.zip"
  "vibe-xpls_${version}_windows_arm64.zip"
)

declare -A expected=()
declare -A digest_by_asset=()

for asset in "${expected_assets[@]}"; do
  expected["$asset"]=1
done

while read -r digest asset extra; do
  [[ -z "${digest:-}" ]] && continue
  [[ -z "${extra:-}" ]] || {
    printf 'Unexpected extra checksum fields for asset %s\n' "$asset" >&2
    exit 1
  }
  [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'Invalid SHA-256 for asset %s: %s\n' "$asset" "$digest" >&2
    exit 1
  }
  [[ -n "${expected[$asset]:-}" ]] || {
    printf 'Unexpected asset in checksums.txt: %s\n' "$asset" >&2
    exit 1
  }
  digest_by_asset["$asset"]="$digest"
done < "$downloaded_checksums"

for asset in "${expected_assets[@]}"; do
  [[ -n "${digest_by_asset[$asset]:-}" ]] || {
    printf 'Missing checksum for expected asset: %s\n' "$asset" >&2
    exit 1
  }
done

[[ "${#digest_by_asset[@]}" -eq "${#expected_assets[@]}" ]] || {
  printf 'Expected %d assets, found %d\n' "${#expected_assets[@]}" "${#digest_by_asset[@]}" >&2
  exit 1
}
```

- [ ] **Step 4: Rewrite `src/resolver.rs`**

Use environment-driven `perl` replacements so quoting stays safe:

```bash
VERSION="$version" perl -0pi -e \
  's/pub const VIBE_XPLS_VERSION: &str = "v[0-9]+\.[0-9]+\.[0-9]+";/pub const VIBE_XPLS_VERSION: \&str = "$ENV{VERSION}";/' \
  src/resolver.rs

set_digest() {
  local os="$1"
  local arch="$2"
  local digest="$3"

  OS="$os" ARCH="$arch" DIGEST="$digest" perl -0pi -e '
    my $os = quotemeta($ENV{OS});
    my $arch = quotemeta($ENV{ARCH});
    my $digest = $ENV{DIGEST};
    my $replacement = "(HostOs::$ENV{OS}, HostArch::$ENV{ARCH}) => {\n            Ok(\"$digest\")\n        }";
    my $pattern = qr/\(HostOs::$os, HostArch::$arch\) => \{\n\s*Ok\("[0-9a-f]{64}"\)\n\s*\}/;
    my $count = s/$pattern/$replacement/g;
    die "expected one digest replacement for $ENV{OS}/$ENV{ARCH}, got $count\n" unless $count == 1;
  ' src/resolver.rs
}

set_digest Mac X8664 "${digest_by_asset["vibe-xpls_${version}_darwin_amd64.tar.gz"]}"
set_digest Mac Aarch64 "${digest_by_asset["vibe-xpls_${version}_darwin_arm64.tar.gz"]}"
set_digest Linux X8664 "${digest_by_asset["vibe-xpls_${version}_linux_amd64.tar.gz"]}"
set_digest Linux Aarch64 "${digest_by_asset["vibe-xpls_${version}_linux_arm64.tar.gz"]}"
set_digest Windows X8664 "${digest_by_asset["vibe-xpls_${version}_windows_amd64.zip"]}"
set_digest Windows Aarch64 "${digest_by_asset["vibe-xpls_${version}_windows_arm64.zip"]}"
```

- [ ] **Step 5: Make the script executable and commit it**

Run:

```bash
chmod +x scripts/bump-vibe-xpls-release.sh
scripts/bump-vibe-xpls-release.sh --checksums-file <(gh release download v0.0.2 --repo io41/vibe-xpls --pattern checksums.txt --output -) v0.0.2
cargo fmt --check
cargo test
git add scripts/bump-vibe-xpls-release.sh src/resolver.rs
git commit -m "ci: add vibe-xpls bump script"
```

Expected: running the script for the current `v0.0.2` release leaves no source diff after the commit's intended script addition.

## Task 3: Add the Manual Workflow Wrapper

**Files:**
- Create: `.github/workflows/bump-vibe-xpls.yml`

- [ ] **Step 1: Add the workflow shell**

Create `.github/workflows/bump-vibe-xpls.yml`:

```yaml
name: Bump vibe-xpls

on:
  workflow_dispatch:
    inputs:
      version:
        description: Exact vibe-xpls release tag, for example v0.0.3
        required: true
        type: string

permissions:
  contents: write
  pull-requests: write

concurrency:
  group: bump-vibe-xpls-${{ inputs.version }}
  cancel-in-progress: false

jobs:
  bump:
    name: Bump pinned server
    runs-on: ubuntu-latest
    env:
      VERSION: ${{ inputs.version }}
      ARTIFACT_DIR: ${{ runner.temp }}/vibe-xpls-bump
    steps:
      - name: Checkout
        uses: actions/checkout@v6
        with:
          fetch-depth: 0
```

- [ ] **Step 2: Add safe input and branch checks**

Add steps that use the `VERSION` environment variable only:

```yaml
      - name: Validate input
        run: |
          set -euo pipefail
          [[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
            echo "::error::Expected stable SemVer tag like v0.0.3, got: $VERSION"
            exit 2
          }
          branch="maintenance/bump-vibe-xpls-${VERSION}"
          echo "BRANCH=$branch" >> "$GITHUB_ENV"

      - name: Check for existing branch or PR
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail
          if git ls-remote --exit-code --heads origin "$BRANCH" >/dev/null 2>&1; then
            echo "::error::Branch already exists: $BRANCH"
            exit 1
          fi
          existing_prs="$(gh pr list --repo "$GITHUB_REPOSITORY" --head "$BRANCH" --state open --json number --jq 'length')"
          if [[ "$existing_prs" != "0" ]]; then
            echo "::error::An open PR already exists for branch: $BRANCH"
            exit 1
          fi
```

- [ ] **Step 3: Add toolchain, bump, validation, and audit artifacts**

Add:

```yaml
      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-wasip2

      - name: Bump pinned vibe-xpls release
        run: scripts/bump-vibe-xpls-release.sh "$VERSION"

      - name: Capture release metadata
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail
          mkdir -p "$ARTIFACT_DIR"
          gh api "repos/io41/vibe-xpls/releases/tags/${VERSION}" \
            --jq '.assets[] | [.name, (.digest // "")] | @tsv' \
            > "$ARTIFACT_DIR/release-assets.tsv"

      - name: Check formatting
        run: cargo fmt --check

      - name: Run tests
        run: cargo test

      - name: Build extension
        run: cargo build --target wasm32-wasip2

      - name: Capture source diff
        run: |
          set -euo pipefail
          git diff -- src/resolver.rs > "$ARTIFACT_DIR/source.diff"
          test -s "$ARTIFACT_DIR/source.diff"

      - name: Upload audit artifacts
        uses: actions/upload-artifact@v4
        with:
          name: vibe-xpls-bump-${{ inputs.version }}
          path: ${{ runner.temp }}/vibe-xpls-bump/
          if-no-files-found: error
```

- [ ] **Step 4: Commit, push, and open the PR**

Add:

```yaml
      - name: Commit bump
        run: |
          set -euo pipefail
          git config user.name "Tim Kersten"
          git config user.email "tim@io41.com"
          git switch -c "$BRANCH"
          git add src/resolver.rs
          git commit -m "feat(vibe-xpls): bump pinned server to ${VERSION}"
          git push --set-upstream origin "$BRANCH"

      - name: Open pull request
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail
          body="$ARTIFACT_DIR/pr-body.md"
          {
            printf 'Updates the pinned vibe-xpls language-server release to `%s`.\n\n' "$VERSION"
            printf 'Validation run by this workflow:\n'
            printf '- `cargo fmt --check`\n'
            printf '- `cargo test`\n'
            printf '- `cargo build --target wasm32-wasip2`\n\n'
            printf 'Audit artifacts include `checksums.txt`, release asset metadata, and the source diff.\n'
          } > "$body"
          gh pr create \
            --repo "$GITHUB_REPOSITORY" \
            --base main \
            --head "$BRANCH" \
            --title "feat(vibe-xpls): bump pinned server to ${VERSION}" \
            --body-file "$body"
```

- [ ] **Step 5: Verify workflow trigger policy**

Run:

```bash
rg -n 'pull_request|pull_request_target' .github/workflows/bump-vibe-xpls.yml
```

Expected: no matches.

- [ ] **Step 6: Commit the workflow**

Run:

```bash
git add .github/workflows/bump-vibe-xpls.yml
git commit -m "ci: add vibe-xpls bump workflow"
```

## Task 4: Local Fixture Verification

**Files:**
- No permanent source files unless fixes are needed.

- [ ] **Step 1: Create a temporary repo copy**

Run:

```bash
tmpdir="$(mktemp -d)"
git archive HEAD | tar -x -C "$tmpdir"
```

- [ ] **Step 2: Create a fake future checksum file**

Run:

```bash
cat > "$tmpdir/checksums-v0.0.3.txt" <<'EOF'
1111111111111111111111111111111111111111111111111111111111111111  vibe-xpls_v0.0.3_darwin_amd64.tar.gz
2222222222222222222222222222222222222222222222222222222222222222  vibe-xpls_v0.0.3_darwin_arm64.tar.gz
3333333333333333333333333333333333333333333333333333333333333333  vibe-xpls_v0.0.3_linux_amd64.tar.gz
4444444444444444444444444444444444444444444444444444444444444444  vibe-xpls_v0.0.3_linux_arm64.tar.gz
5555555555555555555555555555555555555555555555555555555555555555  vibe-xpls_v0.0.3_windows_amd64.zip
6666666666666666666666666666666666666666666666666666666666666666  vibe-xpls_v0.0.3_windows_arm64.zip
EOF
```

- [ ] **Step 3: Run the script in the temporary copy**

Run:

```bash
(
  cd "$tmpdir"
  scripts/bump-vibe-xpls-release.sh --checksums-file checksums-v0.0.3.txt v0.0.3
  git diff --name-only
)
```

Expected: only `src/resolver.rs` changes.

- [ ] **Step 4: Verify rejection paths**

Run:

```bash
(
  cd "$tmpdir"
  scripts/bump-vibe-xpls-release.sh v0.0.3-beta
)
```

Expected: exits non-zero with `Expected stable SemVer tag`.

Then delete one line from the fake checksum file and rerun the script in a fresh archive copy.
Expected: exits non-zero with `Missing checksum for expected asset`.

## Task 5: Final Verification and Planning Docs Cleanup

**Files:**
- Modify: `docs/superpowers/README.md`

- [ ] **Step 1: Clear the active planning note**

After implementation is complete, update `docs/superpowers/README.md`:

```markdown
Status: no active plans or implementation specs.
```

And restore the pending work section to:

```markdown
There is no active local implementation plan in this directory.
```

- [ ] **Step 2: Run full verification**

Run:

```bash
cargo fmt --check
cargo test
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
rg -n 'pull_request|pull_request_target' .github/workflows
git diff --check
```

Expected:

- Rust formatting passes.
- Rust tests pass.
- WASM build passes.
- `pull_request` search has no workflow trigger matches.
- Diff whitespace check is clean.

- [ ] **Step 3: Commit docs cleanup and any final fixes**

Run:

```bash
git add docs/superpowers/README.md
git commit -m "docs: clear vibe-xpls bump plan"
```

Only make this commit if the planning README still has active-plan text after the implementation commits.

## Self-Review Notes

- Spec coverage: tasks cover manual-only workflow, source-only updates, strict version input, exact asset-set validation, audit artifacts, Conventional Commit PR creation, Rust validation, and no PR triggers.
- The workflow uses `inputs.version` in YAML metadata and environment bindings only. Shell commands use `"$VERSION"`.
- Historical specs, plans, `CHANGELOG.md`, and Release Please state are intentionally excluded from the bump script.
