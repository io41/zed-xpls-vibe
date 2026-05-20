# Vibe xpls Download and Version Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Avoid GitHub release API rate limits, improve download errors, and enforce the pinned `vibe-xpls` version for auto-discovered local binaries.

**Architecture:** Keep resolver policy in `src/resolver.rs` so behavior is unit-testable without Zed host APIs. Keep `src/lib.rs` as the Zed host adapter that reads settings, executes `--version` probes, downloads direct pinned assets, and formats host-facing errors. Documentation records the compatibility contract and explicit override behavior.

**Tech Stack:** Rust Zed extension API `0.7.0`, Rust unit tests, Zed `LspSettings`, GitHub release direct asset URLs, Markdown docs.

---

## Scope Check

This plan covers one subsystem: binary resolution/download policy for `vibe-xpls`. It does not change syntax highlighting, Zed file type mapping, Release Please configuration, or repository visibility.

## File Structure

- Modify `src/resolver.rs`: pure resolver constants, download URL planning, version probe result model, strict version parsing, local binary resolution policy, and resolver unit tests.
- Modify `src/lib.rs`: Zed `LocalLookup` adapter, PATH override probing, direct `zed::download_file` URL use, and friendly host error helpers/tests.
- Modify `README.md`: public install/resolution/troubleshooting documentation for direct download, version enforcement, and explicit override.
- Modify `AGENTS.md`: guardrails for no `gh` fallback, no GitHub release API lookup in fallback path, version-checked auto-discovery, and preserved ids/default args.

## Task 0: Baseline

**Files:**
- Read: `docs/superpowers/specs/2026-05-20-vibe-xpls-download-and-version-policy-design.md`
- Read: `src/resolver.rs`
- Read: `src/lib.rs`
- No file changes.

- [ ] **Step 1: Confirm worktree state**

Run:

```bash
git status --short --branch
```

Expected: branch is clean except the existing untracked `.superpowers/` directory. Do not stage `.superpowers/`.

- [ ] **Step 2: Confirm baseline tests**

Run:

```bash
cargo fmt --check
cargo test
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: all commands exit 0 before changes.

- [ ] **Step 3: Confirm no PR-triggered workflow drift**

Run:

```bash
rg -n 'pull_request|pull_request_target' .github
```

Expected: no output and exit status 1.

## Task 1: Pure Resolver Policy

**Files:**
- Modify: `src/resolver.rs`

- [ ] **Step 1: Add failing strict version parser tests**

In `src/resolver.rs`, inside `#[cfg(test)] mod tests`, add these tests:

```rust
    #[test]
    fn version_output_accepts_exact_pinned_version() {
        assert_eq!(
            parse_vibe_xpls_version("vibe-xpls v0.0.1\n").unwrap(),
            VIBE_XPLS_VERSION
        );
    }

    #[test]
    fn version_output_rejects_extra_tokens_and_build_metadata() {
        assert!(parse_vibe_xpls_version("vibe-xpls v0.0.1 extra").is_err());
        assert!(parse_vibe_xpls_version("vibe-xpls v0.0.1+dev").is_err());
        assert!(parse_vibe_xpls_version("prefix vibe-xpls v0.0.1").is_err());
    }
```

- [ ] **Step 2: Run parser tests and verify they fail**

Run:

```bash
cargo test version_output
```

Expected: FAIL because `parse_vibe_xpls_version` does not exist.

- [ ] **Step 3: Add probe result types and strict parser**

In `src/resolver.rs`, replace the current `LocalLookup` trait block with this model:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionProbeResult {
    Missing,
    Failed(String),
    Output { stdout: String, stderr: String },
}

pub trait LocalLookup {
    fn which(&mut self, binary: &str) -> Option<String>;
    fn env_var(&self, key: &str) -> Option<String>;
    fn probe_version(&mut self, path: &str) -> VersionProbeResult;
}
```

Add the parser near `manual_install_hint()`:

```rust
pub fn parse_vibe_xpls_version(stdout: &str) -> Result<&str, String> {
    let output = stdout.trim();
    let expected = format!("{VIBE_XPLS_BIN} {VIBE_XPLS_VERSION}");

    if output == expected {
        Ok(VIBE_XPLS_VERSION)
    } else {
        Err(format!(
            "expected `{expected}` from `{VIBE_XPLS_BIN} --version`, got `{output}`"
        ))
    }
}
```

- [ ] **Step 4: Run parser tests and verify they pass**

Run:

```bash
cargo test version_output
```

Expected: PASS.

- [ ] **Step 5: Add failing direct URL test**

In `src/resolver.rs`, extend `asset_plan_matches_v0_0_1_release_names` with:

```rust
        assert_eq!(
            plan.download_url,
            "https://github.com/io41/vibe-xpls/releases/download/v0.0.1/vibe-xpls_v0.0.1_darwin_arm64.tar.gz"
        );
```

- [ ] **Step 6: Run direct URL test and verify it fails**

Run:

```bash
cargo test resolver::tests::asset_plan_matches_v0_0_1_release_names
```

Expected: FAIL because `DownloadPlan` has no `download_url`.

- [ ] **Step 7: Add direct URL to `DownloadPlan`**

In `src/resolver.rs`, add a field to `DownloadPlan`:

```rust
    pub download_url: String,
```

Add this helper near `manual_install_hint()`:

```rust
pub fn release_asset_url(asset_name: &str) -> String {
    format!(
        "https://github.com/{VIBE_XPLS_REPO}/releases/download/{VIBE_XPLS_VERSION}/{asset_name}"
    )
}
```

In `download_plan`, build the asset name before `Ok(DownloadPlan { ... })`:

```rust
    let asset_name = format!("vibe-xpls_{VIBE_XPLS_VERSION}_{os_part}_{arch_part}.{extension}");
    let download_url = release_asset_url(&asset_name);
```

Then return:

```rust
    Ok(DownloadPlan {
        asset_name,
        download_url,
        version_dir,
        temp_dir,
        binary_path,
        temp_binary_path,
        archive_kind,
    })
```

- [ ] **Step 8: Run direct URL test and verify it passes**

Run:

```bash
cargo test resolver::tests::asset_plan_matches_v0_0_1_release_names
```

Expected: PASS.

- [ ] **Step 9: Add failing resolver policy tests**

Replace `FakeLookup` fields and `LocalLookup` implementation in `src/resolver.rs` tests with:

```rust
    struct FakeLookup {
        which_path: Option<String>,
        which_calls: Vec<String>,
        env: std::collections::BTreeMap<String, String>,
        probes: std::collections::BTreeMap<String, VersionProbeResult>,
        probed: Vec<String>,
    }

    impl Default for FakeLookup {
        fn default() -> Self {
            Self {
                which_path: None,
                which_calls: Vec::new(),
                env: std::collections::BTreeMap::new(),
                probes: std::collections::BTreeMap::new(),
                probed: Vec::new(),
            }
        }
    }

    impl FakeLookup {
        fn matching_version() -> VersionProbeResult {
            VersionProbeResult::Output {
                stdout: "vibe-xpls v0.0.1\n".to_string(),
                stderr: String::new(),
            }
        }

        fn mismatched_version(version: &str) -> VersionProbeResult {
            VersionProbeResult::Output {
                stdout: format!("vibe-xpls {version}\n"),
                stderr: String::new(),
            }
        }
    }

    impl LocalLookup for FakeLookup {
        fn which(&mut self, binary: &str) -> Option<String> {
            self.which_calls.push(binary.to_string());
            self.which_path.clone()
        }

        fn env_var(&self, key: &str) -> Option<String> {
            self.env.get(key).cloned()
        }

        fn probe_version(&mut self, path: &str) -> VersionProbeResult {
            self.probed.push(path.to_string());
            self.probes
                .get(path)
                .cloned()
                .unwrap_or(VersionProbeResult::Missing)
        }
    }
```

Update existing tests that used `lookup.probeable.insert(...)` to use:

```rust
        lookup
            .probes
            .insert("/example/path/vibe-xpls".to_string(), FakeLookup::matching_version());
```

Then add these tests:

```rust
    #[test]
    fn path_lookup_requires_matching_version() {
        let mut lookup = FakeLookup {
            which_path: Some("/path/vibe-xpls".to_string()),
            ..FakeLookup::default()
        };
        lookup
            .probes
            .insert("/path/vibe-xpls".to_string(), FakeLookup::matching_version());

        let binary = resolve_local_binary(None, HostOs::Mac, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(binary.path, "/path/vibe-xpls");
        assert_eq!(binary.source, LocalBinarySource::Path);
        assert_eq!(lookup.probed, vec!["/path/vibe-xpls".to_string()]);
    }

    #[test]
    fn path_lookup_mismatch_hard_fails_before_go_bin() {
        let mut lookup = FakeLookup {
            which_path: Some("/path/vibe-xpls".to_string()),
            ..FakeLookup::default()
        };
        lookup
            .env
            .insert("HOME".to_string(), "/home/tim".to_string());
        lookup.probes.insert(
            "/path/vibe-xpls".to_string(),
            FakeLookup::mismatched_version("v0.0.2"),
        );
        lookup.probes.insert(
            "/home/tim/go/bin/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );

        let error = resolve_local_binary(None, HostOs::Mac, &mut lookup).unwrap_err();

        assert!(error.contains("Found vibe-xpls v0.0.2 at /path/vibe-xpls"));
        assert!(error.contains("requires vibe-xpls v0.0.1"));
        assert_eq!(lookup.probed, vec!["/path/vibe-xpls".to_string()]);
    }

    #[test]
    fn go_bin_mismatch_hard_fails() {
        let mut lookup = FakeLookup::default();
        lookup.env.insert("GOBIN".to_string(), "/gobin".to_string());
        lookup.probes.insert(
            "/gobin/vibe-xpls".to_string(),
            FakeLookup::mismatched_version("v0.0.2"),
        );

        let error = resolve_local_binary(None, HostOs::Mac, &mut lookup).unwrap_err();

        assert!(error.contains("Found vibe-xpls v0.0.2 at /gobin/vibe-xpls"));
        assert!(error.contains("requires vibe-xpls v0.0.1"));
    }

    #[test]
    fn go_bin_unparseable_version_errors() {
        let mut lookup = FakeLookup::default();
        lookup.env.insert("GOBIN".to_string(), "/gobin".to_string());
        lookup.probes.insert(
            "/gobin/vibe-xpls".to_string(),
            VersionProbeResult::Output {
                stdout: "unexpected output\n".to_string(),
                stderr: String::new(),
            },
        );

        let error = resolve_local_binary(None, HostOs::Mac, &mut lookup).unwrap_err();

        assert!(error.contains("Could not verify vibe-xpls at /gobin/vibe-xpls"));
        assert!(error.contains("expected `vibe-xpls v0.0.1`"));
    }

    #[test]
    fn user_setting_path_bypasses_version_probe() {
        let settings = BinarySettings {
            path: Some("/custom/vibe-xpls".to_string()),
            arguments: None,
        };
        let mut lookup = FakeLookup::default();

        let binary = resolve_local_binary(Some(settings), HostOs::Mac, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(binary.path, "/custom/vibe-xpls");
        assert_eq!(binary.source, LocalBinarySource::UserSetting);
        assert!(lookup.probed.is_empty());
    }
```

- [ ] **Step 10: Run resolver tests and verify they fail**

Run:

```bash
cargo test resolver
```

Expected: FAIL because `resolve_local_binary` still returns `Option<LocalBinary>` and still uses boolean probes.

- [ ] **Step 11: Implement resolver hard-fail policy**

Change the resolver function signature:

```rust
pub fn resolve_local_binary<L: LocalLookup>(
    settings: Option<BinarySettings>,
    os: HostOs,
    lookup: &mut L,
) -> Result<Option<LocalBinary>, String> {
```

Add these helpers before `resolve_local_binary`:

```rust
fn local_binary_error(path: &str, message: impl AsRef<str>) -> String {
    format!(
        "Could not verify vibe-xpls at {path}. {message}\n\nInstall the pinned server with:\n{}\n\nOr configure lsp.zed-xpls-vibe.binary.path if you intentionally want to use a different server version.",
        manual_install_hint()
    )
}

fn version_mismatch_error(path: &str, found: &str) -> String {
    format!(
        "Found vibe-xpls {found} at {path}, but zed-xpls-vibe requires vibe-xpls {VIBE_XPLS_VERSION}.\n\nInstall the pinned server with:\n{}\n\nOr configure lsp.zed-xpls-vibe.binary.path if you intentionally want to use a different server version.",
        manual_install_hint()
    )
}

fn found_version(stdout: &str) -> Option<&str> {
    stdout
        .trim()
        .strip_prefix("vibe-xpls ")
        .filter(|version| version.chars().all(|ch| !ch.is_whitespace()))
}

fn verify_auto_discovered_binary<L: LocalLookup>(
    path: &str,
    lookup: &mut L,
) -> Result<bool, String> {
    match lookup.probe_version(path) {
        VersionProbeResult::Missing => Ok(false),
        VersionProbeResult::Failed(message) => Err(local_binary_error(path, message)),
        VersionProbeResult::Output { stdout, .. } => match parse_vibe_xpls_version(&stdout) {
            Ok(_) => Ok(true),
            Err(message) => {
                if let Some(version) = found_version(&stdout) {
                    Err(version_mismatch_error(path, version))
                } else {
                    Err(local_binary_error(path, message))
                }
            }
        },
    }
}
```

Update `resolve_local_binary`:

```rust
    if let Some(path) = settings_path {
        if !path.trim().is_empty() {
            return Ok(Some(LocalBinary {
                path,
                args,
                source: LocalBinarySource::UserSetting,
            }));
        }
    }

    let binary_name = host_binary_name(os);
    if let Some(path) = lookup.which(binary_name) {
        if verify_auto_discovered_binary(&path, lookup)? {
            return Ok(Some(LocalBinary {
                path,
                args: args.clone(),
                source: LocalBinarySource::Path,
            }));
        }
        return Err(local_binary_error(
            &path,
            format!("`{binary_name} --version` could not be executed."),
        ));
    }

    for (source, path) in go_bin_candidates(os, binary_name, lookup) {
        if verify_auto_discovered_binary(&path, lookup)? {
            return Ok(Some(LocalBinary {
                path,
                args: args.clone(),
                source: LocalBinarySource::GoBin(source),
            }));
        }
    }

    Ok(None)
```

Update all resolver tests that call `resolve_local_binary(...).unwrap()` to use `.unwrap().unwrap()` when expecting a binary.

- [ ] **Step 12: Run resolver tests and verify they pass**

Run:

```bash
cargo test resolver
```

Expected: all resolver tests pass.

- [ ] **Step 13: Commit resolver policy**

Run:

```bash
git add src/resolver.rs
git commit -m "feat: enforce pinned vibe xpls resolver policy"
```

Expected: commit succeeds. `.superpowers/` remains untracked.

## Task 2: Zed Runtime Adapter and Friendly Download Errors

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/resolver.rs` only if Task 1 left import or signature follow-up required by the runtime adapter.

- [ ] **Step 1: Add failing runtime helper tests**

In `src/lib.rs`, inside `#[cfg(test)] mod tests`, add:

```rust
    #[test]
    fn download_error_sanitizes_github_json() {
        let message = friendly_download_error(
            "vibe-xpls_v0.0.1_darwin_arm64.tar.gz",
            "status error 403, response: \"{\\\"message\\\":\\\"API rate limit exceeded\\\"}\"",
        );

        assert!(message.contains("Could not download vibe-xpls v0.0.1 for zed-xpls-vibe."));
        assert!(message.contains("GitHub refused the download"));
        assert!(message.contains("go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1"));
        assert!(!message.contains("{\\\"message\\\""));
    }

    #[test]
    fn download_error_names_missing_asset() {
        let message = friendly_download_error(
            "vibe-xpls_v0.0.1_linux_amd64.tar.gz",
            "status error 404",
        );

        assert!(message.contains("pinned release asset was not found"));
        assert!(message.contains("vibe-xpls_v0.0.1_linux_amd64.tar.gz"));
    }
```

- [ ] **Step 2: Run runtime helper tests and verify they fail**

Run:

```bash
cargo test download_error
```

Expected: FAIL because `friendly_download_error` does not exist.

- [ ] **Step 3: Add friendly error helpers**

In `src/lib.rs`, add these helpers near `zed_archive_kind`:

```rust
fn sanitize_host_error(error: &str) -> String {
    let before_response = error
        .split("response:")
        .next()
        .unwrap_or(error)
        .trim()
        .trim_end_matches(',');

    if before_response.is_empty() {
        "unknown error".to_string()
    } else {
        before_response.to_string()
    }
}

fn friendly_download_error(asset_name: &str, error: impl ToString) -> String {
    let raw = error.to_string();
    let sanitized = sanitize_host_error(&raw);
    let lower = raw.to_ascii_lowercase();
    let cause = if lower.contains("404") || lower.contains("not found") {
        format!("the pinned release asset was not found: `{asset_name}`")
    } else if lower.contains("403") || lower.contains("rate limit") {
        "GitHub refused the download, possibly because of rate limiting".to_string()
    } else {
        sanitized
    };

    format!(
        "Could not download {VIBE_XPLS_BIN} {VIBE_XPLS_VERSION} for {LANGUAGE_SERVER_ID}.\n\nThe extension downloads a pinned language-server binary when no compatible local {VIBE_XPLS_BIN} is found. The download failed: {cause}.\n\nInstall the pinned server with:\n{}\n\nOr configure lsp.{LANGUAGE_SERVER_ID}.binary.path to a compatible local binary.",
        manual_install_hint()
    )
}
```

- [ ] **Step 4: Run runtime helper tests and verify they pass**

Run:

```bash
cargo test download_error
```

Expected: PASS.

- [ ] **Step 5: Update `ZedLookup` for version probes**

In `src/lib.rs`, update imports:

```rust
use resolver::{
    default_args, download_plan, manual_install_hint, resolve_local_binary, ArchiveKind,
    BinarySettings, HostArch, HostOs, LocalLookup, VersionProbeResult, VIBE_XPLS_BIN,
    VIBE_XPLS_VERSION,
};
```

Remove `VIBE_XPLS_REPO` from the import list.

Add `version_probes` to `ZedLookup`:

```rust
    version_probes: BTreeMap<String, VersionProbeResult>,
```

Initialize it in `ZedLookup::new`:

```rust
            version_probes: BTreeMap::new(),
```

Replace `probe_executable` with:

```rust
    fn probe_version(&mut self, path: &str) -> VersionProbeResult {
        if let Some(result) = self.version_probes.get(path) {
            return result.clone();
        }

        let result = match zed::process::Command::new(path)
            .arg("--version")
            .envs(self.shell_env.clone())
            .output()
        {
            Ok(output) if output.status == Some(0) => VersionProbeResult::Output {
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            },
            Ok(output) => VersionProbeResult::Failed(
                String::from_utf8_lossy(&output.stderr)
                    .trim()
                    .to_string()
                    .if_empty_then("version command exited unsuccessfully"),
            ),
            Err(_) => VersionProbeResult::Missing,
        };

        self.version_probes
            .insert(path.to_string(), result.clone());
        result
    }
```

Add this small trait near `sanitize_host_error` so the `.if_empty_then(...)` call compiles:

```rust
trait EmptyStringFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl EmptyStringFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}
```

- [ ] **Step 6: Update PATH override lookup to return probed candidates**

Change `which_on_env_path` signature:

```rust
fn which_on_env_path(
    binary: &str,
    env: &[(String, String)],
    os: HostOs,
    mut probe: impl FnMut(&str) -> VersionProbeResult,
) -> Option<String> {
```

Change the loop body:

```rust
        let candidate = join_host_path(os, dir, binary);
        if !matches!(probe(&candidate), VersionProbeResult::Missing) {
            return Some(candidate);
        }
```

Update the `ZedLookup::which` closure:

```rust
            return which_on_env_path(binary, &shell_env, self.os, |path| {
                self.probe_version(path)
            });
```

Update `path_override_lookup_uses_merged_env_path` test closure to return `VersionProbeResult`:

```rust
        let found = which_on_env_path("vibe-xpls", &env, HostOs::Linux, |candidate| {
            probed.push(candidate.to_string());
            if candidate == "/custom/bin/vibe-xpls" {
                VersionProbeResult::Output {
                    stdout: "vibe-xpls v0.0.1\n".to_string(),
                    stderr: String::new(),
                }
            } else {
                VersionProbeResult::Missing
            }
        });
```

- [ ] **Step 7: Update `language_server_command` for resolver `Result`**

Replace the local resolution block with:

```rust
        if let Some(binary) =
            resolve_local_binary(resolver_binary_settings(settings.as_ref()), os, &mut lookup)?
        {
            return Ok(zed::Command {
                command: binary.path,
                args: binary.args,
                env,
            });
        }
```

- [ ] **Step 8: Remove release API lookup from download path**

In `downloaded_binary_path`, remove the `zed::github_release_by_tag_name(...)` call and asset search. Replace the `zed::download_file` call with:

```rust
        zed::download_file(
            &plan.download_url,
            &plan.temp_dir,
            zed_archive_kind(plan.archive_kind),
        )
        .map_err(|error| {
            fs::remove_dir_all(&plan.temp_dir).ok();
            friendly_download_error(&plan.asset_name, error)
        })?;
```

- [ ] **Step 9: Update pinning test**

In `pins_vibe_xpls_release`, remove the `VIBE_XPLS_REPO` assertion if the constant is no longer imported in `src/lib.rs`. The resolver unit tests cover URL construction through `download_plan`.

- [ ] **Step 10: Run tests**

Run:

```bash
cargo test
```

Expected: PASS. If the `zed::process::Command` API exposes timeout support, use it in `probe_version`; otherwise leave the probe without a custom timeout as the spec permits.

- [ ] **Step 11: Build WASM**

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: PASS.

- [ ] **Step 12: Commit runtime adapter**

Run:

```bash
git add src/lib.rs src/resolver.rs
git commit -m "feat: download pinned vibe xpls assets directly"
```

Expected: commit succeeds. `.superpowers/` remains untracked.

## Task 3: Documentation and Agent Guardrails

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: Update README requirements and resolution text**

In `README.md`, replace the paragraph after Requirements with:

```markdown
With network access on a supported release platform, the extension downloads the pinned language server release directly after local resolution fails. Unsupported platforms should install a compatible local `vibe-xpls` binary explicitly.

Local installs are supported, but binaries discovered automatically from `PATH` or standard Go bin directories must report the pinned version expected by this extension:
```

Keep the existing `go install` and `vibe-xpls --version` snippets.

- [ ] **Step 2: Update README binary resolution section**

In `README.md`, replace:

```markdown
No settings are needed when `vibe-xpls` is on `PATH` or installed in a standard Go bin directory.

Use an explicit path only for non-standard installs:
```

with:

```markdown
No settings are needed when a compatible `vibe-xpls v0.0.1` is on `PATH` or installed in a standard Go bin directory.

If an automatically discovered local binary reports a different version, the extension stops with a compatibility error instead of silently running it or falling through to download. This keeps the extension and language server as a tested pair.

Use an explicit path only for non-standard installs or development binaries. Explicit `binary.path` is an expert override; compatibility is then your responsibility:
```

- [ ] **Step 3: Update README troubleshooting**

In `README.md`, add this troubleshooting paragraph before the existing `zed --foreground` snippet:

````markdown
If Zed reports a download failure, install the pinned server manually:

```sh
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1
```

If Zed reports that a local `vibe-xpls` version is incompatible, either install the pinned version above or configure `lsp.zed-xpls-vibe.binary.path` to a specific binary you intentionally want to run.
````

- [ ] **Step 4: Update AGENTS guardrails**

In `AGENTS.md`, replace the resolver-order bullet with:

```markdown
- The public extension resolves `vibe-xpls` in this order: Zed `lsp.zed-xpls-vibe.binary.path`, shell `PATH`, standard Go bin directories, then the pinned `io41/vibe-xpls` GitHub release recorded in the source. PATH and Go-bin results must be version-checked against the pinned server version before use; explicit `binary.path` is the expert override.
```

Add these bullets after it:

```markdown
- Do not add a default `gh` fallback for language-server downloads.
- Do not use the GitHub release API for the pinned auto-download path; construct the exact pinned release asset URL and pass it to `zed::download_file`.
```

- [ ] **Step 5: Verify docs contain the intended policy**

Run:

```bash
rg -n 'gh fallback|github_release_by_tag_name|latest-version|VIBE_XPLS_BIN|/private/tmp' README.md AGENTS.md
```

Expected: no public README references to `github_release_by_tag_name`, `latest-version`, `VIBE_XPLS_BIN`, or `/private/tmp`. `AGENTS.md` may contain guardrails for no `gh` fallback and no `VIBE_XPLS_BIN` override.

- [ ] **Step 6: Commit docs**

Run:

```bash
git add README.md AGENTS.md
git commit -m "docs: document vibe xpls compatibility policy"
```

Expected: commit succeeds. `.superpowers/` remains untracked.

## Task 4: Full Verification

**Files:**
- No intended file changes.

- [ ] **Step 1: Run formatting**

Run:

```bash
cargo fmt --check
```

Expected: PASS.

- [ ] **Step 2: Run all tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 3: Build the Zed extension WASM**

Run:

```bash
PATH="/opt/homebrew/opt/rustup/bin:$PATH" cargo build --target wasm32-wasip2
```

Expected: PASS.

- [ ] **Step 4: Run whitespace and workflow checks**

Run:

```bash
git diff --check HEAD
git diff-tree --check --no-commit-id -r HEAD
rg -n 'pull_request|pull_request_target' .github
```

Expected: both Git whitespace checks pass; `rg` has no output and exits 1.

- [ ] **Step 5: Run policy string checks**

Run:

```bash
rg -n 'github_release_by_tag_name|latest_github_release|gh auth|gh release|@latest' src README.md AGENTS.md docs/superpowers/specs/2026-05-20-vibe-xpls-download-and-version-policy-design.md
```

Expected: no `src` usage of GitHub release API helpers, no `gh` fallback instructions, and no `@latest` install guidance. The spec/AGENTS may contain negative policy statements about `gh`.

- [ ] **Step 6: Confirm commit identity and status**

Run:

```bash
git log -3 --format='%h %an <%ae> %cn <%ce> %s'
git status --short --branch
```

Expected: new commits use `Tim Kersten <tim@io41.com>` as author and committer; only `.superpowers/` is untracked.

## Task 5: Manual Zed Validation

**Files:**
- No intended file changes unless validation discovers a bug.

- [ ] **Step 1: Install this repository as a Zed dev extension**

In Zed, run `zed: install dev extension` and choose this repository:

```text
/Users/tim.kersten/Code/gh/zed-xpls-vibe
```

Expected: `Crossplane YAML` remains available and the extension id remains `zed-xpls-vibe`.

- [ ] **Step 2: Validate direct download path**

Temporarily make sure `vibe-xpls` is not discoverable from the shell `PATH` or standard Go bin directories used by Zed. Open a `Crossplane YAML` file.

Expected: Zed downloads the direct pinned asset URL for `v0.0.1`, extracts it under the extension-owned cache directory, and starts it with `serve`.

- [ ] **Step 3: Validate matching local binary**

Install the pinned local binary:

```bash
go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.1
vibe-xpls --version
```

Expected version output:

```text
vibe-xpls v0.0.1
```

Reopen a `Crossplane YAML` file in Zed.

Expected: Zed uses the local binary and starts it with `serve`.

- [ ] **Step 4: Validate mismatch error**

Point `lsp.zed-xpls-vibe.binary.env.PATH` at a test directory containing an executable named `vibe-xpls` that prints:

```text
vibe-xpls v0.0.2
```

Open a `Crossplane YAML` file.

Expected: Zed shows the friendly mismatch error and does not fall through to Go-bin or download.

- [ ] **Step 5: Validate explicit override**

Configure `lsp.zed-xpls-vibe.binary.path` to the same mismatched test executable.

Expected: Zed starts the explicit override without version enforcement.

## Task 6: Push and Release PR Follow-Up

**Files:**
- No intended file changes.

- [ ] **Step 1: Check remote status**

Run:

```bash
git status --short --branch
git rev-list --left-right --count origin/main...main
```

Expected: local branch is ahead of `origin/main` by the implementation commits only; `.superpowers/` remains untracked.

- [ ] **Step 2: Push main**

Run:

```bash
git push origin main
```

Expected: push succeeds.

- [ ] **Step 3: Check workflows**

Run:

```bash
gh run list --repo io41/zed-xpls-vibe --limit 10 --json databaseId,workflowName,event,status,conclusion,headBranch,headSha,url
```

Expected: only `push` workflows run for `main`; no PR-triggered workflows appear.

- [ ] **Step 4: Handle Release Please PR**

Run:

```bash
gh pr list --repo io41/zed-xpls-vibe --state open --json number,title,headRefName,baseRefName,url,author
```

Expected: Release Please either updates the existing release PR or opens/reopens the expected release PR. Do not merge it until the implementation and manual Zed validation are complete.
