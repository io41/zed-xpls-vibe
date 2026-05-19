# Zed xpls Vibe Public Release Design

Date: 2026-05-19

Status: approved design; revised after external review; not yet an
implementation plan.

## Context

`zed-xpls-vibe` started as a local validation fork for the first runnable
`vibe-xpls` milestone. It currently keeps the Zed extension id and language
server id as `zed-xpls-vibe`, starts the local milestone server with
`<temporary-vibe-xpls-binary> serve`, and validates `Crossplane YAML` syntax
highlighting plus language-server integration.

This design intentionally moves the repository beyond that local-only
milestone behavior. Implementation must update the local agent guidance and
Rust guardrail tests that currently require the hardcoded `<temporary-directory>`
binary. The extension id, language server id, and default `serve` argument stay
stable; the fixed milestone binary path does not.

The next step is to make the extension publishable for other Zed users while
keeping the language server dependency deterministic. The language server
remains a separate Go project at `github.com/io41/vibe-xpls`. Its first public
release is `v0.0.1`, with GitHub Release archives for macOS, Linux, and Windows
on `amd64` and `arm64`.

Zed extension publishing requires a public extension repository, an accepted
root license, a stable extension id, and a PR to `zed-industries/extensions`
that adds the extension as an HTTPS submodule and records the matching version
in `extensions.toml`.

## Goals

- Keep `zed-xpls-vibe` usable as a normal Zed extension without requiring users
  to edit settings in the common case.
- Resolve `vibe-xpls` from local installs first, then fall back to an
  extension-managed download pinned to `v0.0.1`.
- Keep releases deterministic by avoiding "latest" language-server downloads.
- Use SemVer and automated changelog generation for the extension repository.
- Build and test on merges to `main`.
- Avoid GitHub Actions execution from pull-request events.
- Make the repository public once publish-readiness changes are in place.
- Document the steps needed to publish and update the extension in Zed's
  extension registry.
- Retire the local-only hardcoded binary guardrail without weakening the
  guardrails for the extension id, language server id, or `serve` argument.

## Non-Goals

- Do not bundle the `vibe-xpls` source tree into this repository.
- Do not make `vibe-xpls` release automation part of this extension change.
- Do not auto-track the latest `vibe-xpls` release.
- Do not require users to configure `lsp.zed-xpls-vibe.binary.path` for a
  standard install.
- Do not add pull-request-triggered GitHub Actions.
- Do not rename the extension id or language server id away from
  `zed-xpls-vibe`.
- Do not change Crossplane YAML syntax highlighting behavior as part of this
  release-readiness work.

## Retired Local Validation Constraints

The current `AGENTS.md` and Rust tests are intentionally scoped to local
milestone validation. They say the extension must launch `<temporary-vibe-xpls-binary>
serve` and that tests must preserve that hardcoded path.

This public-release design supersedes only that local binary-path constraint.
The implementation must:

- Update `AGENTS.md` so future agents do not restore `<temporary-vibe-xpls-binary>`.
- Replace the hardcoded-path test with resolver-order and fallback tests.
- Preserve tests that assert `LANGUAGE_SERVER_ID == "zed-xpls-vibe"`.
- Preserve tests that assert the default server arguments are `["serve"]`.
- Keep development-only notes for rebuilding `<temporary-vibe-xpls-binary>` only if
  they are clearly labeled as local validation instructions.

## Binary Resolution

The extension should start `vibe-xpls serve` through a small resolver with a
single clear responsibility: find or install the `vibe-xpls` executable.

Resolution order:

1. If `lsp.zed-xpls-vibe.binary.path` is set, use that path.
2. Otherwise, use `worktree.which("vibe-xpls")`.
3. Otherwise, check standard Go binary locations:
   - `$GOBIN/vibe-xpls`, when `GOBIN` is set.
   - `$GOPATH/bin/vibe-xpls`, when `GOPATH` is set.
   - `$HOME/go/bin/vibe-xpls`, when `HOME` is set.
4. Otherwise, download the pinned `io41/vibe-xpls` release `v0.0.1`.
5. Start the resolved binary with `["serve"]`, unless the user explicitly set
   `lsp.zed-xpls-vibe.binary.arguments`.

The explicit Zed LSP binary override is an escape hatch, not a required normal
path. It should use Zed's existing `LspSettings` shape so advanced users do not
need an extension-specific settings schema.

`binary.arguments` should be interpreted by presence. `None` means use the
default `["serve"]`; `Some(arguments)` means use the user-provided arguments,
even if the list is empty.

The Go-bin-dir step must not return an unverified absolute path. The
implementation plan must name and verify the probing mechanism before coding
that step. The preferred probe is a lightweight process check using
`zed::process::Command` to run the candidate with `--version` and the worktree
shell environment. If implementation testing shows Zed cannot safely probe
absolute Go-bin candidates from the extension, stop and revise this spec rather
than returning a guessed path.

The resolver should pass `worktree.shell_env()` when launching both
user-installed and extension-managed binaries. That preserves normal values
such as `HOME` and `PATH` for any server-side config or child-tool lookup.

For testability, the resolver core should be separated from Zed host calls. The
core should operate over an injected lookup/probe/download interface so unit
tests can exercise precedence, missing binary behavior, asset matching, and
error messages without calling Zed host APIs or the network.

## Pinned Auto-Download

The pinned language-server version should be a named code constant:

```text
VIBE_XPLS_VERSION = "v0.0.1"
```

The extension should call `github_release_by_tag_name("io41/vibe-xpls",
VIBE_XPLS_VERSION)` rather than `latest_github_release`. It should select the
release asset for the current Zed platform:

- macOS arm64: `vibe-xpls_v0.0.1_darwin_arm64.tar.gz`
- macOS amd64: `vibe-xpls_v0.0.1_darwin_amd64.tar.gz`
- Linux arm64: `vibe-xpls_v0.0.1_linux_arm64.tar.gz`
- Linux amd64: `vibe-xpls_v0.0.1_linux_amd64.tar.gz`
- Windows arm64: `vibe-xpls_v0.0.1_windows_arm64.zip`
- Windows amd64: `vibe-xpls_v0.0.1_windows_amd64.zip`

These asset names were verified against the GitHub release metadata for
`io41/vibe-xpls` tag `v0.0.1` on 2026-05-19.

Downloaded files should live under an extension-owned versioned directory such
as `vibe-xpls-v0.0.1/`, relative to the extension working directory Zed
provides at language-server startup. The extension should mark the extracted
binary executable on platforms where that is required.

Caching has two layers:

- In-process extension state may cache the resolved binary path for the current
  extension instance.
- File-system idempotency should come from checking whether the extracted
  extension-managed binary already exists in the versioned directory.

Downloads should avoid leaving a half-installed cache. Prefer downloading into
a fresh temporary directory such as `vibe-xpls-v0.0.1.tmp/`, validating that the
expected binary exists, then renaming it to `vibe-xpls-v0.0.1/`. If Zed's WASI
environment makes atomic rename impractical, the implementation must at least
delete the temporary directory on failure and never cache a path until the
binary exists.

If the platform is unsupported, the release is missing, or the expected asset is
not present, the error should include:

```sh
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1
```

and should describe which lookup steps failed.

The `go install` package path was verified against `github.com/io41/vibe-xpls`
tag `v0.0.1`: the module path is `github.com/io41/vibe-xpls` and the command
entrypoint exists at `cmd/vibe-xpls/main.go`.

## Installation Documentation

The README should present installation in this order:

1. Install the Zed extension.
2. Prefer a normal Go install:

   ```sh
   go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1
   ```

3. Confirm the server:

   ```sh
   vibe-xpls --version
   vibe-xpls serve
   ```

4. Explain that if `vibe-xpls` is not on `PATH` or in a standard Go bin
   directory, the extension will download the pinned `v0.0.1` release.
5. Document the optional override:

   ```jsonc
   {
     "lsp": {
       "zed-xpls-vibe": {
         "binary": {
           "path": "/absolute/path/to/vibe-xpls",
           "arguments": ["serve"]
         }
       }
     }
   }
   ```

The README should remove local-only `<temporary-vibe-xpls-binary>` instructions from
the public usage path. Developer notes may still mention local milestone
validation separately, but they must be clearly labeled as development-only.

## Release Automation

The extension repository should use SemVer and stay on the `v0.x.y` line until
maintainers explicitly decide it is stable enough for `v1.0.0`.

Release Please should own `CHANGELOG.md` and open release pull requests on
merges to `main`. Its config should update:

- `Cargo.toml`
- `extension.toml`
- `CHANGELOG.md`
- the release manifest

Release Please should use Conventional Commits and include the `v` prefix in
tags. The release pull request title should follow the existing style used in
`vibe-xpls`, such as `chore: release ${version}`.

The extension's `Cargo.toml` and `extension.toml` versions must stay aligned.
The `extensions.toml` entry in `zed-industries/extensions` must match the
`extension.toml` version at the submodule commit.

## GitHub Actions Policy

Repository workflows should not run on pull-request events.

The reason is deliberate: this repository will become public, and the desired
policy is to avoid executing untrusted pull-request code in GitHub Actions.
Trusted validation happens on `main` after merge and through explicit manual
dispatches.

Allowed workflow triggers:

- `push` to `main`
- `workflow_dispatch`, where useful for manual validation

Recommended workflows:

- `ci.yml`
  - `cargo fmt --check`
  - `cargo test`
  - `cargo build --target wasm32-wasip2`
  - `git diff --check`
- `release.yml`
  - Release Please on `push` to `main`
  - publish GitHub releases when Release Please creates one
- `dev-build.yml`
  - build the WASM extension artifact on `push` to `main`
  - upload the artifact for maintainer validation of the exact post-merge build

Dev-build artifacts are not formal releases and should not be treated as Zed
registry publication. Their consumer is a maintainer doing a manual
post-merge/dev-extension sanity check from the workflow artifact.

Dependabot or Renovate may be added later, but this design does not require it.
If added, repository settings should still prevent pull-request workflows from
running.

## Publishing To Zed

After publish-readiness changes land and verification passes, make
`io41/zed-xpls-vibe` public.

Initial Zed registry publication requires a PR to `zed-industries/extensions`:

1. Add this repository as an HTTPS submodule:

   ```sh
   git submodule add https://github.com/io41/zed-xpls-vibe.git extensions/zed-xpls-vibe
   ```

2. Add the matching top-level `extensions.toml` entry:

   ```toml
   [zed-xpls-vibe]
   submodule = "extensions/zed-xpls-vibe"
   version = "0.0.1"
   ```

3. Run the extension index sorting command required by the Zed extensions repo.
4. Ensure the checked-out submodule commit is on a branch, not a detached-only
   commit.

Future extension updates require a new PR to `zed-industries/extensions` that
updates the submodule commit and the `extensions.toml` version. The registry
version must match `extension.toml` at that commit.

## Security And Integrity

The extension uses a pinned release tag instead of "latest" to avoid silent
language-server upgrades.

The `vibe-xpls` release includes `checksums.txt`, but Zed's extension API
exposes release asset names and download URLs rather than asset SHA digests.
Checksum verification inside the extension is valuable but not required for the
first public cut unless implementation proves it is straightforward with the
available HTTP and file APIs.

Before deferring checksum verification, the implementation plan must include a
short feasibility check. The check should determine whether the extension can
fetch `checksums.txt`, associate the selected asset with its SHA-256, and verify
the downloaded bytes without adding disproportionate archive-handling
complexity. If it is practical within the existing Zed extension API and a small
dependency footprint, checksum verification should ship with the first public
download path. If it is not practical, record the reason in the implementation
notes and keep the pinned-version controls below.

Until in-extension checksum verification exists, release integrity relies on:

- HTTPS GitHub release downloads.
- The pinned `v0.0.1` tag.
- The published `checksums.txt` for manual verification.
- A deliberate source change to bump `VIBE_XPLS_VERSION`.

## Testing

Unit tests should cover:

- The language server id remains `zed-xpls-vibe`.
- The default arguments remain `["serve"]`.
- User override path and arguments win when configured.
- `PATH` lookup wins before Go bin lookup.
- Go bin lookup wins before auto-download.
- OS/arch asset naming for supported platforms.
- Unsupported OS/arch returns an actionable error.
- Missing assets return an actionable error.
- The pure resolver core uses injected lookup/probe/download behavior rather
  than direct Zed host calls.

Manual or integration validation should cover:

- Existing `Crossplane YAML` file association behavior.
- Zed starting a user-installed `vibe-xpls` on `PATH`.
- Zed starting a `vibe-xpls` from a standard Go bin path.
- Zed downloading and starting the pinned `v0.0.1` release when no local binary
  is available.
- Optional `lsp.zed-xpls-vibe.binary.path` override.

The pinned-download scenario is a pre-publish acceptance criterion, not an
optional smoke test. The procedure should force the local resolver past
`PATH` and Go-bin candidates, for example by launching Zed with a temporary
environment that omits `vibe-xpls` and points Go-bin-related variables at empty
directories, then confirming the extension downloads and starts `v0.0.1`.

Final verification before merging:

```sh
cargo fmt --check
cargo test
cargo build --target wasm32-wasip2
git diff --check
```

On this machine, the WASM build may need rustup ahead of Homebrew Rust:

```sh
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

## Open Risks

- Auto-download adds platform-specific archive and cache behavior that the
  local validation fork did not need.
- Probing standard Go bin directories may require `zed::process::Command`
  rather than direct file metadata, depending on what host paths Zed exposes to
  the extension.
- `vibe-xpls` is pre-1.0, so extension releases must be explicit about which
  language-server version they pin.
- Zed extension API support for checksum verification may be awkward enough to
  defer.
- The public README must avoid confusing normal installation with local
  milestone validation instructions.
- The repository cannot be accepted by Zed's registry until it is public and
  has a valid accepted license at the extension root.

## References

- Zed extension publishing docs: https://zed.dev/docs/extensions/developing-extensions
- Zed extension index: https://github.com/zed-industries/extensions
- `vibe-xpls` release `v0.0.1`: https://github.com/io41/vibe-xpls/releases/tag/v0.0.1
