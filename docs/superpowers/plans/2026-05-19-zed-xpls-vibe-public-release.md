# Zed xpls Vibe Public Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `zed-xpls-vibe` publishable by resolving `vibe-xpls` from local installs or a pinned `v0.0.1` download, adding release automation, and documenting the public Zed extension path.

**Architecture:** Split runtime behavior into a pure resolver module and a thin Zed host adapter. Keep release automation and publication docs separate from runtime code. Retire only the `<temporary-vibe-xpls-binary>` local-validation constraint while preserving the `zed-xpls-vibe` ids and default `serve` argument.

**Tech Stack:** Rust 2021, `zed_extension_api` 0.7.0, Zed WASM target `wasm32-wasip2`, GitHub Actions, Release Please.

---

## File Structure

- Modify `AGENTS.md`: replace local validation guidance with public-extension guidance while preserving ids and `serve`.
- Modify `README.md`: document public installation, resolver behavior, optional override, development build, release, and Zed publishing notes.
- Modify `src/lib.rs`: keep Zed API entrypoint, settings parsing, host adapter, and download orchestration.
- Create `src/resolver.rs`: pure resolver constants, argument defaults, asset naming, Go-bin candidate discovery, local resolution, and unit tests.
- Modify `Cargo.toml`: keep package metadata aligned and add no new runtime dependencies unless the checksum feasibility task proves one is required.
- Create `CHANGELOG.md`: initial Release Please-managed changelog.
- Create `release-please-config.json`: Rust release config plus `extension.toml` version updates.
- Create `.release-please-manifest.json`: initial manifest at `0.0.1`.
- Create `.github/workflows/ci.yml`: push/manual-only verification workflow.
- Create `.github/workflows/release.yml`: push/manual-only Release Please workflow.
- Create `.github/workflows/dev-build.yml`: push/manual-only WASM artifact workflow.
- Create `docs/superpowers/spikes/2026-05-19-vibe-xpls-checksum-feasibility.md`: record checksum verification feasibility.

## Task 1: Retire The Local Binary Constraint

**Files:**
- Modify: `AGENTS.md`
- Modify: `src/lib.rs`

- [ ] **Step 1: Update agent guidance**

Replace `AGENTS.md` with:

```markdown
# Agent Instructions

This repository is the Zed extension for `vibe-xpls`.

- Keep the extension id and language server id as `zed-xpls-vibe`; do not change them back to `up-xpls`.
- The extension starts the `vibe-xpls` language server with the default argument `serve`.
- Do not reintroduce the `up xpls serve` fallback or a `VIBE_XPLS_BIN` environment override.
- The public extension resolves `vibe-xpls` in this order: Zed `lsp.zed-xpls-vibe.binary.path`, shell `PATH`, standard Go bin directories, then the pinned `io41/vibe-xpls` GitHub release recorded in the source.
- Rust tests must preserve the extension id, language server id, resolver order, pinned release behavior, and default `serve` argument.
- Local milestone validation with `<temporary-vibe-xpls-binary>` is development-only. If it is needed for a one-off manual check, keep it out of public README usage and do not hardcode it as the production path.
- Zed manual validation should install this repository as a dev extension, not the original `up-xpls` extension.
```

- [ ] **Step 2: Replace hardcoded path tests with failing guardrail tests**

Edit only the test module in `src/lib.rs` for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_unique_language_server_id() {
        assert_eq!(LANGUAGE_SERVER_ID, "zed-xpls-vibe");
    }

    #[test]
    fn starts_vibe_xpls_serve_by_default() {
        assert_eq!(default_vibe_xpls_args(), vec!["serve".to_string()]);
    }
}
```

- [ ] **Step 3: Run tests to verify the expected failure**

Run:

```bash
cargo test
```

Expected: FAIL because `default_vibe_xpls_args` does not exist yet and `vibe_xpls_args` still exists.

- [ ] **Step 4: Add the minimal default args helper**

In `src/lib.rs`, rename `vibe_xpls_args` to:

```rust
fn default_vibe_xpls_args() -> Vec<String> {
    vec!["serve".to_string()]
}
```

Keep `MILESTONE_XPLS_BIN` untouched for this task. The hardcoded path is removed in the resolver task.

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test
```

Expected: PASS for the two tests.

- [ ] **Step 6: Commit**

```bash
git add AGENTS.md src/lib.rs
git commit --no-gpg-sign -m "test: retire local binary guardrail"
```

## Task 2: Add Pure Resolver Core

**Files:**
- Create: `src/resolver.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add module declaration**

At the top of `src/lib.rs`, add:

```rust
mod resolver;
```

- [ ] **Step 2: Create failing resolver tests**

Create `src/resolver.rs` with the tests first:

```rust
pub const VIBE_XPLS_VERSION: &str = "v0.0.1";
pub const VIBE_XPLS_REPO: &str = "io41/vibe-xpls";
pub const VIBE_XPLS_BIN: &str = "vibe-xpls";
pub const VIBE_XPLS_WINDOWS_BIN: &str = "vibe-xpls.exe";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Mac,
    Linux,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostArch {
    Aarch64,
    X8664,
    X86,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    GzipTar,
    Zip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadPlan {
    pub asset_name: String,
    pub version_dir: String,
    pub temp_dir: String,
    pub binary_path: String,
    pub temp_binary_path: String,
    pub archive_kind: ArchiveKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinarySettings {
    pub path: Option<String>,
    pub arguments: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalBinarySource {
    UserSetting,
    Path,
    GoBin(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBinary {
    pub path: String,
    pub args: Vec<String>,
    pub source: LocalBinarySource,
}

pub trait LocalLookup {
    fn which(&mut self, binary: &str) -> Option<String>;
    fn env_var(&self, key: &str) -> Option<String>;
    fn probe_executable(&mut self, path: &str) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeLookup {
        which_path: Option<String>,
        env: std::collections::BTreeMap<String, String>,
        probeable: std::collections::BTreeSet<String>,
        probed: Vec<String>,
    }

    impl LocalLookup for FakeLookup {
        fn which(&mut self, binary: &str) -> Option<String> {
            assert_eq!(binary, VIBE_XPLS_BIN);
            self.which_path.clone()
        }

        fn env_var(&self, key: &str) -> Option<String> {
            self.env.get(key).cloned()
        }

        fn probe_executable(&mut self, path: &str) -> bool {
            self.probed.push(path.to_string());
            self.probeable.contains(path)
        }
    }

    #[test]
    fn default_args_are_serve() {
        assert_eq!(default_args(), vec!["serve".to_string()]);
    }

    #[test]
    fn user_setting_path_wins() {
        let settings = BinarySettings {
            path: Some("/custom/vibe-xpls".to_string()),
            arguments: None,
        };
        let mut lookup = FakeLookup {
            which_path: Some("/usr/bin/vibe-xpls".to_string()),
            ..FakeLookup::default()
        };

        let binary = resolve_local_binary(Some(settings), &mut lookup).unwrap();

        assert_eq!(binary.path, "/custom/vibe-xpls");
        assert_eq!(binary.args, vec!["serve".to_string()]);
        assert_eq!(binary.source, LocalBinarySource::UserSetting);
    }

    #[test]
    fn user_setting_arguments_win_by_presence() {
        let settings = BinarySettings {
            path: Some("/custom/vibe-xpls".to_string()),
            arguments: Some(vec!["serve".to_string(), "--debug".to_string()]),
        };
        let mut lookup = FakeLookup::default();

        let binary = resolve_local_binary(Some(settings), &mut lookup).unwrap();

        assert_eq!(binary.args, vec!["serve".to_string(), "--debug".to_string()]);
    }

    #[test]
    fn path_lookup_wins_before_go_bin() {
        let mut lookup = FakeLookup {
            which_path: Some("/path/vibe-xpls".to_string()),
            ..FakeLookup::default()
        };
        lookup.env.insert("HOME".to_string(), "/home/tim".to_string());
        lookup.probeable.insert("/home/tim/go/bin/vibe-xpls".to_string());

        let binary = resolve_local_binary(None, &mut lookup).unwrap();

        assert_eq!(binary.path, "/path/vibe-xpls");
        assert_eq!(binary.source, LocalBinarySource::Path);
        assert!(lookup.probed.is_empty());
    }

    #[test]
    fn go_bin_candidates_are_probed_in_order() {
        let mut lookup = FakeLookup::default();
        lookup.env.insert("GOBIN".to_string(), "/gobin".to_string());
        lookup.env.insert("GOPATH".to_string(), "/gopath".to_string());
        lookup.env.insert("HOME".to_string(), "/home/tim".to_string());
        lookup.probeable.insert("/gopath/bin/vibe-xpls".to_string());

        let binary = resolve_local_binary(None, &mut lookup).unwrap();

        assert_eq!(binary.path, "/gopath/bin/vibe-xpls");
        assert_eq!(binary.source, LocalBinarySource::GoBin("GOPATH".to_string()));
        assert_eq!(
            lookup.probed,
            vec!["/gobin/vibe-xpls".to_string(), "/gopath/bin/vibe-xpls".to_string()]
        );
    }

    #[test]
    fn asset_plan_matches_v0_0_1_release_names() {
        let plan = download_plan(HostOs::Mac, HostArch::Aarch64).unwrap();
        assert_eq!(plan.asset_name, "vibe-xpls_v0.0.1_darwin_arm64.tar.gz");
        assert_eq!(plan.version_dir, "vibe-xpls-v0.0.1");
        assert_eq!(plan.temp_dir, "vibe-xpls-v0.0.1.tmp");
        assert_eq!(plan.binary_path, "vibe-xpls-v0.0.1/vibe-xpls");
        assert_eq!(plan.temp_binary_path, "vibe-xpls-v0.0.1.tmp/vibe-xpls");
        assert_eq!(plan.archive_kind, ArchiveKind::GzipTar);
    }

    #[test]
    fn windows_asset_uses_zip_and_exe() {
        let plan = download_plan(HostOs::Windows, HostArch::X8664).unwrap();
        assert_eq!(plan.asset_name, "vibe-xpls_v0.0.1_windows_amd64.zip");
        assert_eq!(plan.binary_path, "vibe-xpls-v0.0.1/vibe-xpls.exe");
        assert_eq!(plan.archive_kind, ArchiveKind::Zip);
    }

    #[test]
    fn x86_is_unsupported() {
        let error = download_plan(HostOs::Linux, HostArch::X86).unwrap_err();
        assert!(error.contains("unsupported architecture"));
        assert!(error.contains("go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1"));
    }
}
```

- [ ] **Step 3: Run tests to verify resolver functions are missing**

Run:

```bash
cargo test
```

Expected: FAIL for missing `default_args`, `resolve_local_binary`, and `download_plan`.

- [ ] **Step 4: Add resolver implementation**

Add this above the test module in `src/resolver.rs`:

```rust
pub fn default_args() -> Vec<String> {
    vec!["serve".to_string()]
}

pub fn manual_install_hint() -> String {
    format!("go install github.com/io41/vibe-xpls/cmd/vibe-xpls@{VIBE_XPLS_VERSION}")
}

pub fn resolve_local_binary<L: LocalLookup>(
    settings: Option<BinarySettings>,
    lookup: &mut L,
) -> Option<LocalBinary> {
    if let Some(settings) = settings {
        if let Some(path) = settings.path {
            return Some(LocalBinary {
                path,
                args: settings.arguments.unwrap_or_else(default_args),
                source: LocalBinarySource::UserSetting,
            });
        }
    }

    if let Some(path) = lookup.which(VIBE_XPLS_BIN) {
        return Some(LocalBinary {
            path,
            args: default_args(),
            source: LocalBinarySource::Path,
        });
    }

    for (source, path) in go_bin_candidates(lookup) {
        if lookup.probe_executable(&path) {
            return Some(LocalBinary {
                path,
                args: default_args(),
                source: LocalBinarySource::GoBin(source),
            });
        }
    }

    None
}

fn go_bin_candidates<L: LocalLookup>(lookup: &L) -> Vec<(String, String)> {
    let mut candidates = Vec::new();

    if let Some(gobin) = lookup.env_var("GOBIN").filter(|value| !value.is_empty()) {
        candidates.push(("GOBIN".to_string(), join_path(&gobin, VIBE_XPLS_BIN)));
    }

    if let Some(gopath) = lookup.env_var("GOPATH").filter(|value| !value.is_empty()) {
        candidates.push((
            "GOPATH".to_string(),
            join_path(&join_path(&gopath, "bin"), VIBE_XPLS_BIN),
        ));
    }

    if let Some(home) = lookup.env_var("HOME").filter(|value| !value.is_empty()) {
        candidates.push((
            "HOME".to_string(),
            join_path(&join_path(&home, "go/bin"), VIBE_XPLS_BIN),
        ));
    }

    candidates
}

fn join_path(left: &str, right: &str) -> String {
    format!("{}/{}", left.trim_end_matches('/'), right)
}

pub fn download_plan(os: HostOs, arch: HostArch) -> Result<DownloadPlan, String> {
    let os_part = match os {
        HostOs::Mac => "darwin",
        HostOs::Linux => "linux",
        HostOs::Windows => "windows",
    };

    let arch_part = match arch {
        HostArch::Aarch64 => "arm64",
        HostArch::X8664 => "amd64",
        HostArch::X86 => {
            return Err(format!(
                "unsupported architecture x86 for vibe-xpls {VIBE_XPLS_VERSION}; install manually with `{}`",
                manual_install_hint()
            ));
        }
    };

    let archive_kind = match os {
        HostOs::Windows => ArchiveKind::Zip,
        HostOs::Mac | HostOs::Linux => ArchiveKind::GzipTar,
    };
    let extension = match archive_kind {
        ArchiveKind::GzipTar => "tar.gz",
        ArchiveKind::Zip => "zip",
    };
    let binary_name = match os {
        HostOs::Windows => VIBE_XPLS_WINDOWS_BIN,
        HostOs::Mac | HostOs::Linux => VIBE_XPLS_BIN,
    };
    let version_dir = format!("vibe-xpls-{VIBE_XPLS_VERSION}");
    let temp_dir = format!("{version_dir}.tmp");

    Ok(DownloadPlan {
        asset_name: format!("vibe-xpls_{VIBE_XPLS_VERSION}_{os_part}_{arch_part}.{extension}"),
        binary_path: format!("{version_dir}/{binary_name}"),
        temp_binary_path: format!("{temp_dir}/{binary_name}"),
        version_dir,
        temp_dir,
        archive_kind,
    })
}
```

- [ ] **Step 5: Run resolver tests**

Run:

```bash
cargo test resolver
```

Expected: PASS for resolver tests.

- [ ] **Step 6: Run all tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/lib.rs src/resolver.rs
git commit --no-gpg-sign -m "feat: add vibe xpls binary resolver"
```

## Task 3: Wire Resolver Into Zed Runtime

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Replace local path startup with Zed host adapter**

Replace the contents of `src/lib.rs` with:

```rust
mod resolver;

use std::fs;

use resolver::{
    ArchiveKind, BinarySettings, HostArch, HostOs, LocalLookup, VIBE_XPLS_BIN, VIBE_XPLS_REPO,
    VIBE_XPLS_VERSION, default_args, download_plan, manual_install_hint, resolve_local_binary,
};
use zed::settings::LspSettings;
use zed_extension_api::{self as zed, Result};

const LANGUAGE_SERVER_ID: &str = "zed-xpls-vibe";

struct ZedXplsVibeExtension {
    cached_downloaded_binary: Option<String>,
}

struct ZedLookup<'a> {
    worktree: &'a zed::Worktree,
    shell_env: Vec<(String, String)>,
}

impl<'a> ZedLookup<'a> {
    fn new(worktree: &'a zed::Worktree) -> Self {
        Self {
            worktree,
            shell_env: worktree.shell_env(),
        }
    }
}

impl LocalLookup for ZedLookup<'_> {
    fn which(&mut self, binary: &str) -> Option<String> {
        self.worktree.which(binary)
    }

    fn env_var(&self, key: &str) -> Option<String> {
        self.shell_env
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then(|| value.clone()))
    }

    fn probe_executable(&mut self, path: &str) -> bool {
        zed::process::Command::new(path)
            .arg("--version")
            .envs(self.shell_env.clone())
            .output()
            .is_ok_and(|output| output.status == Some(0))
    }
}

fn default_vibe_xpls_args() -> Vec<String> {
    default_args()
}

fn binary_settings(worktree: &zed::Worktree) -> Option<BinarySettings> {
    LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)
        .ok()
        .and_then(|settings| settings.binary)
        .map(|binary| BinarySettings {
            path: binary.path,
            arguments: binary.arguments,
        })
}

fn host_platform() -> Result<(HostOs, HostArch)> {
    let (os, arch) = zed::current_platform();
    let os = match os {
        zed::Os::Mac => HostOs::Mac,
        zed::Os::Linux => HostOs::Linux,
        zed::Os::Windows => HostOs::Windows,
    };
    let arch = match arch {
        zed::Architecture::Aarch64 => HostArch::Aarch64,
        zed::Architecture::X8664 => HostArch::X8664,
        zed::Architecture::X86 => HostArch::X86,
    };
    Ok((os, arch))
}

fn zed_archive_kind(kind: ArchiveKind) -> zed::DownloadedFileType {
    match kind {
        ArchiveKind::GzipTar => zed::DownloadedFileType::GzipTar,
        ArchiveKind::Zip => zed::DownloadedFileType::Zip,
    }
}

impl ZedXplsVibeExtension {
    fn downloaded_binary_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
    ) -> Result<String> {
        if let Some(path) = &self.cached_downloaded_binary {
            if fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
                return Ok(path.clone());
            }
        }

        let (os, arch) = host_platform()?;
        let plan = download_plan(os, arch)?;

        if fs::metadata(&plan.binary_path).is_ok_and(|metadata| metadata.is_file()) {
            self.cached_downloaded_binary = Some(plan.binary_path.clone());
            return Ok(plan.binary_path);
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::github_release_by_tag_name(VIBE_XPLS_REPO, VIBE_XPLS_VERSION)
            .map_err(|error| format!("failed to fetch vibe-xpls {VIBE_XPLS_VERSION}: {error}; install manually with `{}`", manual_install_hint()))?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == plan.asset_name)
            .ok_or_else(|| {
                format!(
                    "vibe-xpls {VIBE_XPLS_VERSION} release is missing asset `{}`; install manually with `{}`",
                    plan.asset_name,
                    manual_install_hint()
                )
            })?;

        fs::remove_dir_all(&plan.temp_dir).ok();
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );
        zed::download_file(
            &asset.download_url,
            &plan.temp_dir,
            zed_archive_kind(plan.archive_kind),
        )
        .map_err(|error| {
            fs::remove_dir_all(&plan.temp_dir).ok();
            format!("failed to download `{}`: {error}; install manually with `{}`", plan.asset_name, manual_install_hint())
        })?;

        if !fs::metadata(&plan.temp_binary_path).is_ok_and(|metadata| metadata.is_file()) {
            fs::remove_dir_all(&plan.temp_dir).ok();
            return Err(format!(
                "downloaded `{}` but did not find expected binary `{}`; install manually with `{}`",
                plan.asset_name,
                plan.temp_binary_path,
                manual_install_hint()
            ));
        }

        zed::make_file_executable(&plan.temp_binary_path).map_err(|error| {
            fs::remove_dir_all(&plan.temp_dir).ok();
            format!("failed to make `{}` executable: {error}", plan.temp_binary_path)
        })?;

        fs::remove_dir_all(&plan.version_dir).ok();
        fs::rename(&plan.temp_dir, &plan.version_dir).map_err(|error| {
            fs::remove_dir_all(&plan.temp_dir).ok();
            format!("failed to finalize vibe-xpls download: {error}")
        })?;

        self.cached_downloaded_binary = Some(plan.binary_path.clone());
        Ok(plan.binary_path)
    }
}

impl zed::Extension for ZedXplsVibeExtension {
    fn new() -> Self {
        Self {
            cached_downloaded_binary: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        if language_server_id.as_ref() != LANGUAGE_SERVER_ID {
            return Err(format!(
                "Unsupported language server id `{language_server_id}`"
            ));
        }

        let shell_env = worktree.shell_env();
        let mut lookup = ZedLookup::new(worktree);
        if let Some(binary) = resolve_local_binary(binary_settings(worktree), &mut lookup) {
            return Ok(zed::Command {
                command: binary.path,
                args: binary.args,
                env: shell_env,
            });
        }

        Ok(zed::Command {
            command: self.downloaded_binary_path(language_server_id)?,
            args: default_vibe_xpls_args(),
            env: shell_env,
        })
    }
}

zed::register_extension!(ZedXplsVibeExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_unique_language_server_id() {
        assert_eq!(LANGUAGE_SERVER_ID, "zed-xpls-vibe");
    }

    #[test]
    fn starts_vibe_xpls_serve_by_default() {
        assert_eq!(default_vibe_xpls_args(), vec!["serve".to_string()]);
    }

    #[test]
    fn pins_vibe_xpls_release() {
        assert_eq!(VIBE_XPLS_REPO, "io41/vibe-xpls");
        assert_eq!(VIBE_XPLS_VERSION, "v0.0.1");
        assert_eq!(VIBE_XPLS_BIN, "vibe-xpls");
    }
}
```

- [ ] **Step 2: Run tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 3: Build WASM**

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs src/resolver.rs
git commit --no-gpg-sign -m "feat: resolve vibe xpls binary for zed"
```

## Task 4: Record Checksum Feasibility Decision

**Files:**
- Create: `docs/superpowers/spikes/2026-05-19-vibe-xpls-checksum-feasibility.md`

- [ ] **Step 1: Create feasibility note**

Create `docs/superpowers/spikes/2026-05-19-vibe-xpls-checksum-feasibility.md`:

```markdown
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
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/spikes/2026-05-19-vibe-xpls-checksum-feasibility.md
git commit --no-gpg-sign -m "docs: record checksum feasibility"
```

## Task 5: Update Public README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Replace README public usage sections**

Rewrite `README.md` to this content:

```markdown
# Zed xpls Vibe

Zed extension for Crossplane package diagnostics and Crossplane YAML highlighting powered by [`vibe-xpls`](https://github.com/io41/vibe-xpls).

## Requirements

- Zed
- `vibe-xpls` installed locally or network access to download the pinned release

Install the pinned language server with Go:

```sh
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1
```

Confirm the binary:

```sh
vibe-xpls --version
```

Expected version:

```text
vibe-xpls v0.0.1
```

## Binary Resolution

The extension starts `vibe-xpls serve`.

It resolves the binary in this order:

1. `lsp.zed-xpls-vibe.binary.path`, when configured.
2. `vibe-xpls` on the worktree shell `PATH`.
3. Standard Go bin directories: `GOBIN`, `GOPATH/bin`, and `HOME/go/bin`.
4. The pinned GitHub release `io41/vibe-xpls@v0.0.1`.

No settings are needed when `vibe-xpls` is on `PATH` or installed in a standard Go bin directory.

Use an explicit path only for non-standard installs:

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

## Usage

Install the extension from Zed once it is published, or use `zed: install dev extension` when developing from this repository.

Open a file classified as `Crossplane YAML`.

The extension keeps Zed's native YAML support enabled for ordinary YAML and adds a `Crossplane YAML` language for:

- `crossplane.yaml`
- `crossplane.yml`
- `upbound.yaml`
- `upbound.yml`
- `composition.yaml`
- `composition.yml`
- `definition.yaml`
- `definition.yml`
- files mapped to `Crossplane YAML` with Zed `file_types`, such as `*-composition.yaml` and `*-definition.yaml`

`zed-xpls-vibe` runs for `Crossplane YAML` files and leaves package detection to the `vibe-xpls` language server. This allows root package, nested package, multi-package, and no-root validation to exercise the same analyzer path.

`Crossplane YAML` uses two-space, space-only indentation to match YAML and avoid Zed's default four-space indentation in this custom language.

## Syntax Highlighting

`Crossplane YAML` uses Go-template highlighting for `{{ ... }}` actions and injects YAML highlighting into surrounding template text. This is intended for Crossplane `function-go-templating` inline templates where the block scalar emits YAML.

The mixed YAML/template case is best-effort. Template actions remain highlighted, and plain generated YAML text is injected into the YAML parser, but some YAML constructs can still look imperfect when a scalar, list item, or indentation level is split by `{{ ... }}` actions.

Zed extension `path_suffixes` can match exact filenames and dot-delimited suffixes, but not glob-style names like `xexample-composition.yaml`. Zed's language `first_line_pattern` also cannot override the built-in YAML `.yaml` suffix match, so broad `apiVersion: ...crossplane.io/...` content detection is not reliable for YAML files.

The extension config covers the exact filenames above. Add a `file_types` mapping to your Zed settings for hyphenated or custom Crossplane Composition and XRD filenames. The `languages` entry is optional with the current extension, but is useful as a local override and documents the intended indentation behavior:

```jsonc
{
  "file_types": {
    "Crossplane YAML": [
      "**/*-composition.yaml",
      "**/*-composition.yml",
      "**/*-definition.yaml",
      "**/*-definition.yml"
    ]
  },
  "languages": {
    "Crossplane YAML": {
      "tab_size": 2,
      "hard_tabs": false
    }
  }
}
```

## Development

```sh
cargo test
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

For one-off local milestone validation, you may build `vibe-xpls` to `<temporary-vibe-xpls-binary>`, but that path is not the public extension behavior:

```sh
cd <local-vibe-xpls-repo>
go build -o <temporary-vibe-xpls-binary> ./cmd/vibe-xpls
```

## Releases

This extension uses SemVer and stays on the `v0.x.y` line until maintainers explicitly approve a `v1.0.0` release.

Release Please maintains `CHANGELOG.md` from Conventional Commits and opens release pull requests on merges to `main`.

The extension pins the `vibe-xpls` language server release in source. Bumping the pinned language server is a deliberate source change, not an automatic latest-version lookup.

## Publishing To Zed

Zed registry publication happens through a PR to [`zed-industries/extensions`](https://github.com/zed-industries/extensions).

The extension must be public, licensed, and added as an HTTPS submodule under `extensions/zed-xpls-vibe` with a matching `extensions.toml` version.

## Troubleshooting

If Zed does not start this server, first confirm that the original `up-xpls` extension is uninstalled or disabled.

If Zed logs show that the worktree is not trusted, trust the worktree in Zed and reopen it. Zed will not start language servers for untrusted worktrees.

If Zed logs show `vibe-xpls` starting but diagnostics, hover, or completion are absent, check:

```sh
vibe-xpls --version
```

For extension logs, run Zed with:

```sh
zed --foreground
```

or use `zed: open log`.

If the WASM build reports that `wasm32-wasip2` is missing even after installing the target, make sure Cargo is using the same rustup toolchain that owns the target:

```sh
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

## License

MIT. See [LICENSE](LICENSE).
```

- [ ] **Step 2: Run docs search**

Run:

```bash
rg -n '<temporary-directory>|VIBE_XPLS_BIN|up xpls serve|latest' README.md AGENTS.md
```

Expected: only development-only `<temporary-directory>` mention in `README.md`; no `VIBE_XPLS_BIN`, no `up xpls serve`, no latest-version install instruction.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit --no-gpg-sign -m "docs: document public installation"
```

## Task 6: Add Release Please And Workflows

**Files:**
- Create: `CHANGELOG.md`
- Create: `release-please-config.json`
- Create: `.release-please-manifest.json`
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `.github/workflows/dev-build.yml`

- [ ] **Step 1: Add changelog**

Create `CHANGELOG.md`:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

This project uses [Release Please](https://github.com/googleapis/release-please) and Conventional Commits.
```

- [ ] **Step 2: Add Release Please config**

Create `release-please-config.json`:

```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "release-type": "rust",
  "include-v-in-tag": true,
  "include-component-in-tag": false,
  "bump-minor-pre-major": true,
  "bump-patch-for-minor-pre-major": true,
  "pull-request-title-pattern": "chore: release ${version}",
  "changelog-sections": [
    {
      "type": "feat",
      "section": "Features"
    },
    {
      "type": "fix",
      "section": "Bug Fixes"
    },
    {
      "type": "perf",
      "section": "Performance Improvements"
    },
    {
      "type": "docs",
      "section": "Documentation"
    },
    {
      "type": "test",
      "section": "Tests"
    },
    {
      "type": "ci",
      "section": "Continuous Integration"
    },
    {
      "type": "chore",
      "section": "Chores",
      "hidden": true
    }
  ],
  "packages": {
    ".": {
      "package-name": "zed-xpls-vibe",
      "changelog-path": "CHANGELOG.md",
      "extra-files": [
        {
          "type": "toml",
          "path": "extension.toml",
          "jsonpath": "$.version"
        }
      ]
    }
  }
}
```

- [ ] **Step 3: Add Release Please manifest**

Create `.release-please-manifest.json`:

```json
{
  ".": "0.0.1"
}
```

- [ ] **Step 4: Add CI workflow without PR trigger**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches:
      - main
  workflow_dispatch:

permissions:
  contents: read

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v6
        with:
          fetch-depth: 0

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-wasip2

      - name: Check formatting
        run: cargo fmt --check

      - name: Run tests
        run: cargo test

      - name: Build extension
        run: cargo build --target wasm32-wasip2

      - name: Check whitespace
        run: git diff --check HEAD
```

- [ ] **Step 5: Add Release Please workflow without PR trigger**

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    branches:
      - main
  workflow_dispatch:

permissions:
  contents: write
  pull-requests: write

jobs:
  release-please:
    name: Release Please
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v6
        with:
          fetch-depth: 0

      - name: Release Please
        uses: googleapis/release-please-action@v5
        with:
          token: ${{ secrets.RELEASE_PLEASE_TOKEN || secrets.GITHUB_TOKEN }}
          config-file: release-please-config.json
          manifest-file: .release-please-manifest.json
```

- [ ] **Step 6: Add dev build artifact workflow without PR trigger**

Create `.github/workflows/dev-build.yml`:

```yaml
name: Dev Build

on:
  push:
    branches:
      - main
  workflow_dispatch:

permissions:
  contents: read

jobs:
  wasm:
    name: WASM Extension
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v6

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-wasip2

      - name: Build extension
        run: cargo build --target wasm32-wasip2

      - name: Upload WASM artifact
        uses: actions/upload-artifact@v4
        with:
          name: zed-xpls-vibe-wasm
          path: target/wasm32-wasip2/debug/*.wasm
          if-no-files-found: error
```

- [ ] **Step 7: Validate workflow triggers**

Run:

```bash
rg -n 'pull_request|pull_request_target' .github release-please-config.json
```

Expected: no output.

- [ ] **Step 8: Commit**

```bash
git add CHANGELOG.md release-please-config.json .release-please-manifest.json .github/workflows/ci.yml .github/workflows/release.yml .github/workflows/dev-build.yml
git commit --no-gpg-sign -m "ci: add release automation"
```

## Task 7: Full Local Verification

**Files:**
- No code files expected unless verification exposes a defect.

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --check
```

Expected: PASS.

- [ ] **Step 2: Run tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 3: Build WASM**

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: PASS.

- [ ] **Step 4: Check whitespace**

Run:

```bash
git diff --check HEAD
```

Expected: no output.

- [ ] **Step 5: Check forbidden PR workflow triggers**

Run:

```bash
rg -n 'pull_request|pull_request_target' .github
```

Expected: no output.

- [ ] **Step 6: Check public docs avoid local-only install path**

Run:

```bash
rg -n '<temporary-vibe-xpls-binary>|VIBE_XPLS_BIN|up xpls serve|@latest' README.md AGENTS.md src docs/superpowers/specs docs/superpowers/plans
```

Expected: `<temporary-vibe-xpls-binary>` appears only in historical specs/plans and the README development-only note; no `VIBE_XPLS_BIN`, no `up xpls serve`, no `@latest`.

- [ ] **Step 7: Check license and extension version alignment**

Run:

```bash
sed -n '1,3p' LICENSE
rg -n '^version = "0.0.1"$' Cargo.toml extension.toml
```

Expected: `LICENSE` starts with `MIT License` and `Copyright (c) 2026 Tim Kersten`; both `Cargo.toml` and `extension.toml` contain `version = "0.0.1"`.

## Task 8: Manual Zed Validation

**Files:**
- No committed files expected unless validation exposes a defect.

- [ ] **Step 1: Force local PATH behavior**

Run:

```bash
command -v vibe-xpls
vibe-xpls --version
```

Expected: `vibe-xpls v0.0.1` or another local development version that is intentionally on `PATH`.

- [ ] **Step 2: Install this repository as a Zed dev extension**

Run Zed, execute `zed: install dev extension`, and select:

```text
<local-zed-xpls-vibe-repo>
```

Expected: Zed installs `zed-xpls-vibe` as a dev extension.

- [ ] **Step 3: Validate language server startup**

Open:

```text
<local-zed-xpls-vibe-repo>/fixtures/crossplane-package/api/mixed-template-composition.yaml
```

Then run:

```bash
zed --foreground
```

or use `zed: open log`.

Expected: logs show `vibe-xpls` starting with `serve`, not `<temporary-vibe-xpls-binary>` unless that binary is the one found on `PATH`.

- [ ] **Step 4: Validate pinned download path**

Temporarily launch Zed from a shell where `vibe-xpls` is absent from `PATH`, and where `GOBIN`, `GOPATH`, and `HOME` point to temporary locations without `vibe-xpls`.

Example shell setup:

```bash
mkdir -p /tmp/zed-xpls-empty-home /tmp/zed-xpls-empty-gobin /tmp/zed-xpls-empty-gopath/bin
PATH="/usr/bin:/bin" GOBIN="/tmp/zed-xpls-empty-gobin" GOPATH="/tmp/zed-xpls-empty-gopath" HOME="/tmp/zed-xpls-empty-home" zed --foreground
```

Expected: Zed downloads `io41/vibe-xpls` `v0.0.1`, extracts it under the extension working directory, and starts it with `serve`.

- [ ] **Step 5: Validate optional override**

Add a temporary Zed setting:

```jsonc
{
  "lsp": {
    "zed-xpls-vibe": {
      "binary": {
        "path": "<temporary-vibe-xpls-binary>",
        "arguments": ["serve"]
      }
    }
  }
}
```

Expected: Zed starts `<temporary-vibe-xpls-binary> serve`. Remove the temporary setting after validation.

## Task 9: Publish Readiness And Repository Visibility

**Files:**
- No code files expected.

- [ ] **Step 1: Review final status**

Run:

```bash
git status --short --branch
git log --oneline --decorate -8
```

Expected: clean tracked worktree except untracked `.superpowers/`; local branch may be ahead of `origin/main`.

- [ ] **Step 2: Push implementation commits**

Run:

```bash
git push origin main
```

Expected: push succeeds.

- [ ] **Step 3: Make GitHub repository public after final approval**

Run only after maintainer approval that public exposure is intended now:

```bash
gh repo edit io41/zed-xpls-vibe --visibility public --accept-visibility-change-consequences
```

Expected: repository visibility becomes public.

- [ ] **Step 4: Record Zed registry publication commands for the maintainer**

Do not open the Zed registry PR automatically in this task. Record these commands in the final handoff:

```bash
git clone https://github.com/zed-industries/extensions /tmp/zed-extensions-publish
cd /tmp/zed-extensions-publish
git submodule add https://github.com/io41/zed-xpls-vibe.git extensions/zed-xpls-vibe
```

Then add this entry to `extensions.toml`:

```toml
[zed-xpls-vibe]
submodule = "extensions/zed-xpls-vibe"
version = "0.0.1"
```

Run:

```bash
pnpm sort-extensions
```

Expected: the Zed extension index is ready for a PR after maintainer review.
