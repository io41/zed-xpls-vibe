# Vibe-Xpls Archive Checksum Verification Design

## Goal

Verify the pinned `vibe-xpls` release archive before extracting or executing the
downloaded language server.

This protects the managed auto-download path while preserving the current
resolver behavior: local compatible binaries are preferred, explicit
`lsp.crossplane-yaml.binary.path` remains an opt-out, and the managed fallback
uses the direct pinned GitHub release asset URL rather than GitHub release API
lookups or `gh`.

## Current Behavior

When no compatible local `vibe-xpls` binary is found, the extension builds a
platform-specific direct release URL for the pinned `io41/vibe-xpls` version and
calls `zed::download_file` with `GzipTar` or `Zip`. Zed downloads and extracts
the archive directly into the temporary version directory.

That path is simple, but the extension does not see the raw archive bytes before
extraction, so it cannot verify a checksum first.

## Zed API Findings

The published `zed_extension_api` version in this repo is `0.7.0`.

In that API, `zed::download_file` accepts a URL, destination path, and downloaded
file type. It does not accept an expected digest and does not expose checksum
verification as a host feature.

Zed's newer extension API work includes GitHub release asset digest metadata,
but that does not solve this extension's managed-download path on its own:

- the repo currently uses the published `0.7.0` API;
- using release asset metadata would require GitHub release API lookup again;
- the direct pinned release URL was chosen specifically to avoid GitHub API rate
  limits;
- even with a digest field, the extension still needs a way to verify the
  downloaded archive before extraction.

The viable near-term approach is therefore extension-owned verification of the
raw archive.

## Design

Add expected SHA-256 digests for every pinned `vibe-xpls` release archive
supported by the resolver:

| Asset | SHA-256 |
| --- | --- |
| `vibe-xpls_v0.0.2_darwin_amd64.tar.gz` | `a034a9b2eab33ae30eb16909a65c2e885414104649a854a65b62940befba71de` |
| `vibe-xpls_v0.0.2_darwin_arm64.tar.gz` | `d98a35fd57334b0c6d070d283b5ff9c12e46beca0a453c44230f621a0cf56454` |
| `vibe-xpls_v0.0.2_linux_amd64.tar.gz` | `d87f77237b3405a7388110ab65713e764e60338bc49239322272d017ac971d03` |
| `vibe-xpls_v0.0.2_linux_arm64.tar.gz` | `2b7735f6ec251fd381fa2b3f3e6ed7d1f55d702bde96893c809f1ff8ca37d018` |
| `vibe-xpls_v0.0.2_windows_amd64.zip` | `f8bad966fe7970785a541aeffec7f7faf9e400d2256310aeb22220e8af826a94` |
| `vibe-xpls_v0.0.2_windows_arm64.zip` | `87158951680b0fa942821ec28fa9d6492ca3b6cea81da42451b1ef33c2c3c0e5` |

Extend `DownloadPlan` with:

- `temp_archive_path`: the temporary archive path downloaded before
  verification;
- `sha256`: the expected lowercase hex SHA-256 digest for the selected asset.

### Temporary Layout

Use the existing `temp_dir` as the single workspace for one managed download
attempt:

- `temp_dir`: `vibe-xpls-v0.0.2.tmp`
- `temp_archive_path`: `vibe-xpls-v0.0.2.tmp/<asset_name>`
- extraction target: `temp_dir`
- final directory: `vibe-xpls-v0.0.2`

Because `zed::download_file(..., Uncompressed)` writes a file rather than
extracting into a directory, the implementation must create `temp_dir` before
calling it. The archive is downloaded to `temp_archive_path`, verified there,
extracted into `temp_dir`, then removed before finalizing the directory.

Cleanup rules:

- before starting, remove any stale `temp_dir`;
- on download, checksum, extraction, binary validation, or executable-bit
  errors, remove `temp_dir` and leave any existing `version_dir` untouched;
- after all validation succeeds, remove any existing `version_dir` and rename
  `temp_dir` to `version_dir`, preserving the current finalization behavior.

Change the managed download flow to:

1. Resolve the same direct pinned release URL as today.
2. Remove the temporary directory and recreate it.
3. Download the raw archive with `zed::download_file(..., Uncompressed)` into
   `temp_archive_path`.
4. Compute SHA-256 over the downloaded archive bytes.
5. If the digest differs, remove the temporary directory and return a friendly
   error that names the asset and expected version without dumping raw host JSON.
6. Extract the archive only after the digest matches.
7. Verify the expected binary exists and is a regular file.
8. Mark the binary executable where needed.
9. Replace the final version directory exactly as today.

The implementation should keep the current cache shape simple. The final cache
should not retain the archive unless implementation constraints make that
necessary; the stable output is the finalized version directory containing the
expected binary path.

### Digest Maintenance

The committed digest table is the source of truth used at runtime. It should
live with the download-plan logic as Rust constants or an equivalent static
manifest loaded by `download_plan`.

When `VIBE_XPLS_VERSION` changes, the same change must refresh every supported
asset digest. Use the release's `checksums.txt` as the canonical source and
cross-check it against GitHub release asset digest metadata when available. A
reviewer should be able to audit the update with commands like:

```bash
gh release download v0.0.2 --repo io41/vibe-xpls --pattern checksums.txt --output -
gh api repos/io41/vibe-xpls/releases/tags/v0.0.2 --jq '.assets[] | [.name,.digest] | @tsv'
```

The implementation must add a Rust test that fails if any supported
`download_plan` platform has a missing, empty, non-hex, or non-64-character
digest. A helper script is optional; do not add one unless the release bump
workflow becomes repetitive enough to justify it.

## Archive Extraction Safety

Because the extension will own extraction after checksum verification, archive
handling must reject unsafe entries:

- absolute paths;
- parent-directory traversal such as `..`;
- Windows drive prefixes in archive entry names;
- symlinks and hard links;
- any extracted path that does not remain under the temporary version directory.

Extraction should be narrow: the release archives are expected to contain the
`vibe-xpls` binary at the archive root. The implementation does not need to
support arbitrary archive layouts beyond safely extracting the pinned release
assets.

## Dependency Approach

Use small Rust crates for the missing primitives:

- `sha2` for SHA-256;
- `flate2` and `tar` for `.tar.gz`;
- `zip` for Windows `.zip` archives.

Before implementing the full change, run a short dependency viability check by
building the extension for `wasm32-wasip2`. If any archive crate is incompatible
with Zed's WASM target, stop and reassess rather than forcing a large custom
archive implementation.

## Error Handling

Checksum mismatch should fail closed:

- do not extract the archive;
- delete the temporary directory;
- keep any previously finalized version directory untouched;
- show a concise message explaining that the pinned asset did not match the
  expected checksum.

Download and extraction errors should continue to include the manual install
hint:

```text
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.2
```

The message should also mention that users can configure
`lsp.crossplane-yaml.binary.path` if they intentionally want to use a local
server binary.

Download errors should continue to use the existing `friendly_download_error`
and `sanitize_host_error` policy. Checksum and extraction errors should match
that tone rather than introducing a second, noisier error style.

## Tests

Add focused Rust tests for:

- every supported platform asset mapping includes the expected digest;
- managed downloads still use direct pinned release URLs;
- the resolver order remains unchanged;
- checksum comparison accepts the known digest and rejects mismatches;
- mismatch errors are friendly and do not expose raw GitHub JSON;
- unsafe tar and zip archive entries are rejected before extraction;
- the expected binary path is checked after extraction;
- explicit `binary.path` overrides remain outside managed checksum validation.

Test archive fixtures should be generated inline at test time with the same tar
and zip crates used by production code. Do not commit binary fixture archives
unless crate compatibility makes generated fixtures impractical. Hash tests can
use fixed byte strings with known SHA-256 values; unsafe-entry tests should build
small in-memory or temporary archives containing path traversal, absolute paths,
Windows drive paths, symlink entries, and hardlink entries.

Manual validation should cover:

- no local binary: extension downloads, verifies, extracts, and starts the
  pinned server;
- intentionally wrong digest in a temporary test build: install fails before
  extraction with the friendly checksum error;
- existing compatible local binary: no managed download is attempted;
- explicit binary override: override still wins.

## Non-Goals

This change does not:

- add a default `gh` fallback;
- reintroduce GitHub release API lookup for the managed download path;
- update or overwrite locally installed `vibe-xpls` binaries;
- checksum-verify explicit user-provided binary paths;
- change Crossplane YAML language detection or highlighting;
- change user settings.

## Documentation Updates On Landing

When the implementation lands, update `docs/superpowers/decisions.md` in the
same change:

- remove checksum verification from `Deferred Work`;
- add a short `Language Server Resolution` note that managed downloads verify
  the pinned release archive SHA-256 before extraction;
- record that every `VIBE_XPLS_VERSION` bump must update the platform digest
  table and tests in the same pull request.

## Open Risks

The main risk is dependency compatibility with Zed's WASM extension target. That
is why the implementation plan should start with a `wasm32-wasip2` build spike
before changing the runtime resolver.

Owning archive extraction also adds security-sensitive code. The implementation
should keep this small, test path validation directly, and avoid generalized
archive behavior that the extension does not need.
