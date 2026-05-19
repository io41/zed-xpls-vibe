# vibe-xpls Checksum Feasibility

Date: 2026-05-19

## Question

Can `zed-xpls-vibe` verify the SHA-256 checksum of the pinned `vibe-xpls` release archive before executing the downloaded language server?

## Evidence

- `zed_extension_api` exposes `github_release_by_tag_name`, `download_file`, `make_file_executable`, and `http_client::HttpRequest`.
- `github_release_by_tag_name` exposes release asset names and download URLs, but not asset digests.
- The `vibe-xpls` `v0.0.1` release publishes `checksums.txt`.
- `download_file` downloads and extracts an archive from a URL into an extension-owned directory. It does not expose the downloaded archive bytes to extension code.
- Verifying `checksums.txt` against the archive bytes before extraction would require replacing `download_file` with custom HTTP download plus archive extraction for tar.gz and zip.

## Decision

Defer in-extension checksum verification for the first public cut.

The extension will use a pinned release tag, exact asset names, HTTPS GitHub downloads, and a deliberate source change for any future `VIBE_XPLS_VERSION` bump. The README will reference `checksums.txt` for manual verification. Adding checksum verification later is reasonable if the extension takes ownership of archive extraction or if Zed exposes downloaded archive bytes or asset digests.
