# Vibe xpls Download and Version Policy Design

Date: 2026-05-20

Status: approved concept; written for user review before implementation
planning.

## Context

The first public extension build tries local `vibe-xpls` resolution first and
falls back to a pinned `io41/vibe-xpls` GitHub release. A real install hit a
GitHub REST API rate limit while calling the release API:

```text
failed to fetch vibe-xpls v0.0.2: status error 403 ... API rate limit exceeded
```

That error is too raw for an editor user, and the API call is avoidable. The
extension already pins `vibe-xpls` to `v0.0.2` and computes the exact platform
asset name, so it can download the exact release asset URL directly instead of
querying GitHub release metadata.

This also exposed a version ownership question. The extension and language
server should be treated as a compatibility pair. The extension does not bundle
the language server binary, but each extension release owns one pinned
`vibe-xpls` version. Automatic local discovery must not silently run a different
server version.

The public extension repository is `io41/crossplane-yaml`; the pinned language
server release remains under `io41/vibe-xpls`.

This design supersedes the pinned auto-download mechanism from the prior
2026-05-19 public-release design. That earlier design used
`zed::github_release_by_tag_name`; this design replaces that release metadata
lookup with a direct pinned release asset URL.

## Goals

- Avoid GitHub REST API rate limits during the normal auto-download fallback.
- Keep the language server version deterministic and pinned by extension source.
- Produce friendly, actionable Zed errors instead of raw GitHub JSON.
- Prevent accidental use of incompatible local `vibe-xpls` binaries discovered
  from `PATH` or standard Go bin directories.
- Preserve an explicit expert override through Zed `LspSettings`.

## Non-Goals

- Do not bundle `vibe-xpls` binaries inside the Zed extension repository.
- Do not use `gh` as the default download path.
- Do not auto-track the latest `vibe-xpls` release.
- Do not add user settings writes or automatic settings migration.
- Do not change Crossplane YAML syntax highlighting.

## Download Policy

The extension should stop using `zed::github_release_by_tag_name` for the
fallback download path. Instead, it should construct the exact browser download
URL from the existing pinned constants and platform asset plan:

```text
https://github.com/io41/vibe-xpls/releases/download/v0.0.2/<asset-name>
```

The asset name remains platform-specific and exact, for example:

```text
vibe-xpls_v0.0.2_darwin_arm64.tar.gz
```

The extension should pass that URL directly to `zed::download_file`, preserving
the existing temporary directory, archive type, expected binary path,
executable-bit, and final rename behavior.

This avoids the unauthenticated GitHub REST API limit for the common fallback
path. It does not eliminate all possible network failures; it should convert
them into concise messages that point to local installation and binary override
options.

Removing the release API lookup also removes the current pre-flight check that
the computed asset exists in GitHub release metadata. That tradeoff is accepted
because the extension pins both the release tag and asset naming convention in
source. If the direct download returns a not-found response, the error should
say the pinned release asset could not be found and include the computed asset
name.

## Local Version Policy

Resolution order remains:

1. `lsp.crossplane-yaml.binary.path`, when configured.
2. `vibe-xpls` from the merged shell/settings `PATH`.
3. Standard Go bin directories.
4. Direct pinned download from the GitHub release asset URL.

The PATH step currently uses `worktree.which()` without a version probe. This
design intentionally adds a version probe for PATH results before startup.

Version enforcement depends on the source:

- Explicit `binary.path` is an expert override. The extension should run it
  without enforcing the pinned version. Documentation must say compatibility is
  the user's responsibility for this override.
- Auto-discovered binaries from `PATH` or Go bin directories must pass a
  `vibe-xpls --version` check before startup.
- If an auto-discovered local binary reports a version different from
  `v0.0.2`, startup should stop with a friendly error.
- If the version command cannot be run or cannot be parsed, startup should stop
  with a friendly error for auto-discovered binaries rather than silently
  falling back or launching an unknown server.
- The extension-managed downloaded binary is trusted by construction because it
  comes from the exact pinned version URL and asset name.

The mismatch policy is hard-fail, not skip-and-continue. If `vibe-xpls` is found
on PATH and reports `v9.9.9`, the extension must stop instead of falling through
to a matching Go-bin candidate or the pinned download. If a Go-bin candidate
exists but reports the wrong version, the extension must stop instead of trying
later Go-bin candidates. Missing Go-bin candidate files may still be skipped.

The resolver core should own version parsing and policy decisions so unit tests
can cover them without calling Zed host APIs. Replace the current boolean
executable probe with a probe that can distinguish:

- candidate missing, so Go-bin lookup may continue
- command failed or could not run, so an auto-discovered existing candidate is
  rejected with a friendly error
- command succeeded and returned version output

The Zed host adapter should execute `<candidate> --version` and return the
status plus captured stdout/stderr, but the resolver core should parse and
compare the output against `VIBE_XPLS_VERSION`.

The expected version output for `v0.0.2` is:

```text
vibe-xpls v0.0.2
```

The parser should inspect stdout from a successful `--version` command, trim
surrounding ASCII whitespace, and require the exact string
`vibe-xpls v0.0.2`. It should not accept loose substrings, build metadata such
as `vibe-xpls v0.0.2+gitsha`, or extra tokens. Stderr should only be used for a
short sanitized failure reason when the command exits unsuccessfully or stdout
cannot be parsed.

The downloaded binary is not re-probed after extraction in this iteration. That
is a deliberate tradeoff: the direct URL is pinned to the release tag and exact
asset name, while checksum or post-download version verification remains
deferred.

## Error UX

Errors should be written for someone seeing a Zed language server popup.

Network fallback error shape:

```text
Could not download vibe-xpls v0.0.2 for crossplane-yaml.

The extension downloads a pinned language-server binary when no compatible
local vibe-xpls is found. The download failed: <short cause>.

Install the pinned server with:
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.2

Or configure lsp.crossplane-yaml.binary.path to a compatible local binary.
```

Download errors should be mapped into a small set of user-facing causes when
the host error contains enough information:

- 404 or equivalent: the pinned release asset was not found; include the asset
  name.
- 403 or rate-limit wording: GitHub refused the download; suggest the manual
  `go install` fallback.
- other network or extraction failures: preserve a short sanitized cause and
  suggest the same manual fallback.

Version mismatch error shape:

```text
Found vibe-xpls v9.9.9 at <path>, but crossplane-yaml 0.0.1 requires
vibe-xpls v0.0.2.

Install the pinned server with:
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.2

Or configure lsp.crossplane-yaml.binary.path if you intentionally want to use a
different server version.
```

The implementation should sanitize host errors before presenting them. Raw JSON
response bodies from GitHub should not be shown in Zed popups.

## Documentation

README changes should explain:

- Normal users can rely on the extension-managed pinned download.
- Local installs are supported, but auto-discovered local binaries must match
  the extension's pinned `vibe-xpls` version.
- `binary.path` is the intentional escape hatch for non-standard or development
  binaries.
- If a download fails, `go install
  github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.2` is the preferred manual
  fallback.

`AGENTS.md` should add guardrails for the new policy:

- Preserve the `crossplane-yaml` extension id, language server id, resolver order, and
  default `serve` guardrails.
- Update the resolver-order guardrail to say PATH and Go-bin results are
  version-checked before use.
- No `gh` fallback by default.
- No GitHub release API lookup for the pinned auto-download path.
- Auto-discovered local binaries must be version-checked against the pinned
  server version.

## Tests

Unit tests should cover:

- The direct asset URL for each supported platform.
- Existing resolver-order tests stay, but their fake lookup/probe model should
  be updated from boolean executability to version probe results.
- `path_lookup_wins_before_go_bin` should assert that a PATH binary with a
  matching version wins before Go-bin lookup.
- PATH and Go-bin binaries with matching versions are accepted.
- PATH and Go-bin binaries with mismatched versions hard-fail instead of falling
  through to later candidates or download.
- Unparseable or failing `--version` output is rejected for auto-discovered
  binaries.
- Explicit `binary.path` bypasses version enforcement.
- Network errors are converted into friendly messages that do not include raw
  GitHub JSON.
- Default arguments remain `["serve"]`.
- User-provided `binary.arguments` still override by presence.

The implementation should check whether `zed::process::Command` exposes a
timeout for the version probe. If it does, use a short timeout for
auto-discovered local binaries. If it does not, document that a hanging
`vibe-xpls --version` is a residual risk and do not add a custom watchdog in
this change.

Manual Zed validation should cover:

- No local `vibe-xpls`: extension downloads the pinned direct asset and starts
  `serve`.
- Local matching `vibe-xpls`: extension uses the local binary.
- Local mismatched `vibe-xpls`: Zed shows the friendly mismatch error.
- Explicit `binary.path` to a development binary: extension starts it without
  version enforcement.

## References

- Zed extension publishing guidance says language support extensions should
  download or find language servers rather than ship binaries:
  https://zed.dev/docs/extensions/developing-extensions
- GitHub REST API unauthenticated requests are rate limited per IP:
  https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api
- GitHub release assets expose browser download URLs for direct downloads:
  https://docs.github.com/en/rest/releases/assets
