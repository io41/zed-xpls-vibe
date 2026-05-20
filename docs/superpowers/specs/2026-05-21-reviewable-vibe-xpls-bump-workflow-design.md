# Reviewable vibe-xpls bump workflow design

Date: 2026-05-21

## Goal

Provide a reviewable maintenance path for updating the pinned `vibe-xpls`
language-server release used by the `crossplane-yaml` Zed extension.

The workflow should update source files for one explicitly requested
`io41/vibe-xpls` release, run the same relevant validation as CI, and open a pull
request for human review. It must not make the runtime track "latest" and must
not change how the extension resolves or downloads the language server.

## Requirements

- Trigger the automation only through `workflow_dispatch`.
- Require an exact stable SemVer tag input in the form `vX.Y.Z`, such as
  `v0.0.3`.
- Do not add `pull_request` or `pull_request_target` workflow triggers.
- Do not use `gh` or GitHub release API lookup in the extension runtime.
- Keep runtime downloads pinned to the direct release asset URL generated from
  `VIBE_XPLS_VERSION`.
- Update source files only. Do not rewrite historical specs, plans,
  `CHANGELOG.md`, or Release Please state.
- Create a review branch and PR instead of pushing directly to `main`.
- Use a Conventional Commit title and commit message, for example
  `feat(vibe-xpls): bump pinned server to v0.0.3`.

## Non-goals

- No automatic "latest release" tracking.
- No scheduled update job.
- No automatic merge.
- No runtime fallback to `gh`.
- No archive re-download and re-hash inside the maintenance workflow. Runtime
  checksum verification remains the load-bearing archive-integrity check.

## Design

Add a checked-in script:

```text
scripts/bump-vibe-xpls-release.sh <version>
```

The script is the source of truth for the bump logic. The GitHub Actions
workflow should be a thin wrapper around it.

The script will:

1. Validate `<version>` against `^v[0-9]+\.[0-9]+\.[0-9]+$`.
2. Download `checksums.txt` for exactly that release from
   `io41/vibe-xpls`.
3. Require exactly the six currently supported asset names:
   `darwin_amd64`, `darwin_arm64`, `linux_amd64`, `linux_arm64`,
   `windows_amd64`, and `windows_arm64`.
4. Update `src/resolver.rs`:
   - `VIBE_XPLS_VERSION`
   - the six digest literals returned by `release_asset_sha256`
5. Leave historical docs and `CHANGELOG.md` unchanged.
6. Run validation locally or in CI:
   - `cargo fmt --check`
   - `cargo test`
   - `cargo build --target wasm32-wasip2`

Before adding the script, refactor tests so version-bearing expectations derive
from `VIBE_XPLS_VERSION` where possible. Digest literals should remain explicit
because they are release-specific.

## Workflow

Add `.github/workflows/bump-vibe-xpls.yml`.

The workflow will:

1. Trigger only on `workflow_dispatch`.
2. Accept a required `version` input.
3. Bind the input through `env: VERSION: ${{ inputs.version }}` and use
   `"$VERSION"` in shell. Do not interpolate `${{ inputs.version }}` directly
   into `run:` scripts.
4. Set minimal permissions:
   - `contents: write`
   - `pull-requests: write`
5. Use a per-version concurrency group and `cancel-in-progress: false`.
6. Install Rust with the `wasm32-wasip2` target.
7. Run the bump script.
8. Create a branch named `maintenance/bump-vibe-xpls-<version>`.
9. Fail with a clear message if that branch or PR already exists; do not
   force-push over an existing review.
10. Commit and push with `feat(vibe-xpls): bump pinned server to <version>`.
11. Open a PR with the same title.
12. Upload small audit artifacts:
    - downloaded `checksums.txt`
    - release asset metadata from GitHub
    - the resulting `git diff`

Because the repository intentionally does not run CI on PRs, the workflow's
validation steps are the automated gate before the PR is opened. The PR remains
the human review gate before merging to `main`.

## Security notes

The workflow input is trusted only after validation. It must not be interpolated
directly into shell source. Branch names, URLs, and commit messages must use the
validated shell variable.

The workflow uses GitHub release metadata only during maintenance. This does not
violate the runtime decision to avoid GitHub release API lookup in the Zed
extension.

The workflow records the exact source changes in a PR. A reviewer can compare
the digest table against `checksums.txt` and the GitHub release metadata before
merging.

## Verification

The implementation is complete when:

- `cargo fmt --check` passes.
- `cargo test` passes.
- `cargo build --target wasm32-wasip2` passes.
- The script rejects invalid versions.
- The script rejects incomplete or unexpected checksum asset sets.
- A dry-run or local fixture test proves the script updates only the expected
  source locations.
- The workflow has no PR triggers.

## Open decisions

No open product decisions remain. During implementation, prefer the smallest
script that keeps the update reviewable and repeatable.
