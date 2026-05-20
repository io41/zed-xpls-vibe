pub const VIBE_XPLS_VERSION: &str = "v0.0.2";
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
    pub download_url: String,
    pub version_dir: String,
    pub temp_dir: String,
    pub binary_path: String,
    pub temp_binary_path: String,
    pub temp_archive_path: String,
    pub archive_kind: ArchiveKind,
    pub sha256: &'static str,
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

#[cfg(test)]
mod tests {
    use super::*;

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
                stdout: "vibe-xpls v0.0.2\n".to_string(),
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

    #[test]
    fn version_output_accepts_exact_pinned_version() {
        assert_eq!(
            parse_vibe_xpls_version("vibe-xpls v0.0.2\n").unwrap(),
            VIBE_XPLS_VERSION
        );
    }

    #[test]
    fn version_output_rejects_extra_tokens_and_build_metadata() {
        assert!(parse_vibe_xpls_version("vibe-xpls v0.0.2 extra").is_err());
        assert!(parse_vibe_xpls_version("vibe-xpls v0.0.2+dev").is_err());
        assert!(parse_vibe_xpls_version("prefix vibe-xpls v0.0.2").is_err());
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

        let binary = resolve_local_binary(Some(settings), HostOs::Mac, &mut lookup)
            .unwrap()
            .unwrap();

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

        let binary = resolve_local_binary(Some(settings), HostOs::Mac, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(
            binary.args,
            vec!["serve".to_string(), "--debug".to_string()]
        );
    }

    #[test]
    fn blank_user_setting_path_falls_through_to_lookup() {
        let settings = BinarySettings {
            path: Some("  ".to_string()),
            arguments: Some(vec!["serve".to_string(), "--debug".to_string()]),
        };
        let mut lookup = FakeLookup {
            which_path: Some("/path/vibe-xpls".to_string()),
            ..FakeLookup::default()
        };

        lookup.probes.insert(
            "/path/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );

        let binary = resolve_local_binary(Some(settings), HostOs::Mac, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(binary.path, "/path/vibe-xpls");
        assert_eq!(
            binary.args,
            vec!["serve".to_string(), "--debug".to_string()]
        );
        assert_eq!(binary.source, LocalBinarySource::Path);
    }

    #[test]
    fn user_setting_arguments_apply_to_path_lookup_without_path_override() {
        let settings = BinarySettings {
            path: None,
            arguments: Some(vec!["serve".to_string(), "--debug".to_string()]),
        };
        let mut lookup = FakeLookup {
            which_path: Some("/path/vibe-xpls".to_string()),
            ..FakeLookup::default()
        };

        lookup.probes.insert(
            "/path/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );

        let binary = resolve_local_binary(Some(settings), HostOs::Mac, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(binary.path, "/path/vibe-xpls");
        assert_eq!(
            binary.args,
            vec!["serve".to_string(), "--debug".to_string()]
        );
        assert_eq!(binary.source, LocalBinarySource::Path);
    }

    #[test]
    fn path_lookup_wins_before_go_bin() {
        let mut lookup = FakeLookup {
            which_path: Some("/path/vibe-xpls".to_string()),
            ..FakeLookup::default()
        };
        lookup
            .env
            .insert("HOME".to_string(), "/home/tim".to_string());
        lookup.probes.insert(
            "/path/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );
        lookup.probes.insert(
            "/home/tim/go/bin/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );

        let binary = resolve_local_binary(None, HostOs::Mac, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(binary.path, "/path/vibe-xpls");
        assert_eq!(binary.source, LocalBinarySource::Path);
        assert_eq!(lookup.which_calls, vec!["vibe-xpls".to_string()]);
        assert_eq!(lookup.probed, vec!["/path/vibe-xpls".to_string()]);
    }

    #[test]
    fn go_bin_candidates_are_probed_in_order() {
        let mut lookup = FakeLookup::default();
        lookup.env.insert("GOBIN".to_string(), "/gobin".to_string());
        lookup
            .env
            .insert("GOPATH".to_string(), "/gopath".to_string());
        lookup
            .env
            .insert("HOME".to_string(), "/home/tim".to_string());
        lookup.probes.insert(
            "/gopath/bin/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );

        let binary = resolve_local_binary(None, HostOs::Mac, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(binary.path, "/gopath/bin/vibe-xpls");
        assert_eq!(
            binary.source,
            LocalBinarySource::GoBin("GOPATH".to_string())
        );
        assert_eq!(
            lookup.probed,
            vec![
                "/gobin/vibe-xpls".to_string(),
                "/gopath/bin/vibe-xpls".to_string()
            ]
        );
    }

    #[test]
    fn gopath_path_list_uses_first_entry() {
        let mut lookup = FakeLookup::default();
        lookup
            .env
            .insert("GOPATH".to_string(), "/first:/second".to_string());
        lookup.probes.insert(
            "/first/bin/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );

        let binary = resolve_local_binary(None, HostOs::Linux, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(binary.path, "/first/bin/vibe-xpls");
        assert_eq!(lookup.probed, vec!["/first/bin/vibe-xpls".to_string()]);
    }

    #[test]
    fn windows_local_lookup_uses_exe_and_windows_paths() {
        let mut lookup = FakeLookup::default();
        lookup
            .env
            .insert("GOBIN".to_string(), r"C:\GoBin".to_string());
        lookup
            .env
            .insert("GOPATH".to_string(), r"D:\GoPath;E:\Other".to_string());
        lookup
            .env
            .insert("HOME".to_string(), r"C:\Users\tim".to_string());
        lookup.probes.insert(
            r"D:\GoPath\bin\vibe-xpls.exe".to_string(),
            FakeLookup::matching_version(),
        );

        let binary = resolve_local_binary(None, HostOs::Windows, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(lookup.which_calls, vec!["vibe-xpls.exe".to_string()]);
        assert_eq!(binary.path, r"D:\GoPath\bin\vibe-xpls.exe");
        assert_eq!(
            binary.source,
            LocalBinarySource::GoBin("GOPATH".to_string())
        );
        assert_eq!(
            lookup.probed,
            vec![
                r"C:\GoBin\vibe-xpls.exe".to_string(),
                r"D:\GoPath\bin\vibe-xpls.exe".to_string()
            ]
        );
    }

    #[test]
    fn windows_userprofile_go_bin_is_probed_when_gopath_is_unset() {
        let mut lookup = FakeLookup::default();
        lookup
            .env
            .insert("USERPROFILE".to_string(), r"C:\Users\tim".to_string());
        lookup.probes.insert(
            r"C:\Users\tim\go\bin\vibe-xpls.exe".to_string(),
            FakeLookup::matching_version(),
        );

        let binary = resolve_local_binary(None, HostOs::Windows, &mut lookup)
            .unwrap()
            .unwrap();

        assert_eq!(binary.path, r"C:\Users\tim\go\bin\vibe-xpls.exe");
        assert_eq!(
            binary.source,
            LocalBinarySource::GoBin("USERPROFILE".to_string())
        );
        assert_eq!(
            lookup.probed,
            vec![r"C:\Users\tim\go\bin\vibe-xpls.exe".to_string()]
        );
    }

    #[test]
    fn asset_plan_matches_v0_0_2_release_names() {
        let plan = download_plan(HostOs::Mac, HostArch::Aarch64).unwrap();
        assert_eq!(plan.asset_name, "vibe-xpls_v0.0.2_darwin_arm64.tar.gz");
        assert_eq!(
            plan.download_url,
            "https://github.com/io41/vibe-xpls/releases/download/v0.0.2/vibe-xpls_v0.0.2_darwin_arm64.tar.gz"
        );
        assert_eq!(plan.version_dir, "vibe-xpls-v0.0.2");
        assert_eq!(plan.temp_dir, "vibe-xpls-v0.0.2.tmp");
        assert_eq!(plan.binary_path, "vibe-xpls-v0.0.2/vibe-xpls");
        assert_eq!(plan.temp_binary_path, "vibe-xpls-v0.0.2.tmp/vibe-xpls");
        assert_eq!(
            plan.temp_archive_path,
            "vibe-xpls-v0.0.2.tmp/vibe-xpls_v0.0.2_darwin_arm64.tar.gz"
        );
        assert_eq!(plan.archive_kind, ArchiveKind::GzipTar);
        assert_eq!(
            plan.sha256,
            "d98a35fd57334b0c6d070d283b5ff9c12e46beca0a453c44230f621a0cf56454"
        );
    }

    #[test]
    fn all_supported_assets_have_valid_sha256_digests() {
        let expected = [
            (
                HostOs::Mac,
                HostArch::X8664,
                "vibe-xpls_v0.0.2_darwin_amd64.tar.gz",
                "a034a9b2eab33ae30eb16909a65c2e885414104649a854a65b62940befba71de",
            ),
            (
                HostOs::Mac,
                HostArch::Aarch64,
                "vibe-xpls_v0.0.2_darwin_arm64.tar.gz",
                "d98a35fd57334b0c6d070d283b5ff9c12e46beca0a453c44230f621a0cf56454",
            ),
            (
                HostOs::Linux,
                HostArch::X8664,
                "vibe-xpls_v0.0.2_linux_amd64.tar.gz",
                "d87f77237b3405a7388110ab65713e764e60338bc49239322272d017ac971d03",
            ),
            (
                HostOs::Linux,
                HostArch::Aarch64,
                "vibe-xpls_v0.0.2_linux_arm64.tar.gz",
                "2b7735f6ec251fd381fa2b3f3e6ed7d1f55d702bde96893c809f1ff8ca37d018",
            ),
            (
                HostOs::Windows,
                HostArch::X8664,
                "vibe-xpls_v0.0.2_windows_amd64.zip",
                "f8bad966fe7970785a541aeffec7f7faf9e400d2256310aeb22220e8af826a94",
            ),
            (
                HostOs::Windows,
                HostArch::Aarch64,
                "vibe-xpls_v0.0.2_windows_arm64.zip",
                "87158951680b0fa942821ec28fa9d6492ca3b6cea81da42451b1ef33c2c3c0e5",
            ),
        ];

        for (os, arch, asset_name, sha256) in expected {
            let plan = download_plan(os, arch).unwrap();

            assert_eq!(plan.asset_name, asset_name, "{os:?} {arch:?}");
            assert_eq!(plan.sha256, sha256, "{os:?} {arch:?}");
            assert_eq!(plan.sha256.len(), 64, "{os:?} {arch:?}");
            assert!(
                plan.sha256.chars().all(|ch| ch.is_ascii_hexdigit()),
                "{os:?} {arch:?}: {}",
                plan.sha256
            );
            assert!(
                plan.temp_archive_path.ends_with(&plan.asset_name),
                "{os:?} {arch:?}: {}",
                plan.temp_archive_path
            );
        }
    }

    #[test]
    fn windows_asset_uses_zip_and_exe() {
        let plan = download_plan(HostOs::Windows, HostArch::X8664).unwrap();
        assert_eq!(plan.asset_name, "vibe-xpls_v0.0.2_windows_amd64.zip");
        assert_eq!(
            plan.download_url,
            "https://github.com/io41/vibe-xpls/releases/download/v0.0.2/vibe-xpls_v0.0.2_windows_amd64.zip"
        );
        assert_eq!(plan.binary_path, "vibe-xpls-v0.0.2/vibe-xpls.exe");
        assert_eq!(plan.archive_kind, ArchiveKind::Zip);
    }

    #[test]
    fn linux_asset_uses_direct_pinned_url() {
        let plan = download_plan(HostOs::Linux, HostArch::X8664).unwrap();
        assert_eq!(plan.asset_name, "vibe-xpls_v0.0.2_linux_amd64.tar.gz");
        assert_eq!(
            plan.download_url,
            "https://github.com/io41/vibe-xpls/releases/download/v0.0.2/vibe-xpls_v0.0.2_linux_amd64.tar.gz"
        );
    }

    #[test]
    fn path_lookup_requires_matching_version() {
        let mut lookup = FakeLookup {
            which_path: Some("/path/vibe-xpls".to_string()),
            ..FakeLookup::default()
        };
        lookup.probes.insert(
            "/path/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );

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
            FakeLookup::mismatched_version("v0.0.3"),
        );
        lookup.probes.insert(
            "/home/tim/go/bin/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );

        let error = resolve_local_binary(None, HostOs::Mac, &mut lookup).unwrap_err();

        assert!(error.contains("Found vibe-xpls v0.0.3 at /path/vibe-xpls"));
        assert!(error.contains("requires vibe-xpls v0.0.2"));
        assert_eq!(lookup.probed, vec!["/path/vibe-xpls".to_string()]);
    }

    #[test]
    fn path_lookup_failed_probe_hard_fails_before_go_bin() {
        let mut lookup = FakeLookup {
            which_path: Some("/path/vibe-xpls".to_string()),
            ..FakeLookup::default()
        };
        lookup
            .env
            .insert("HOME".to_string(), "/home/tim".to_string());
        lookup.probes.insert(
            "/path/vibe-xpls".to_string(),
            VersionProbeResult::Failed("permission denied".to_string()),
        );
        lookup.probes.insert(
            "/home/tim/go/bin/vibe-xpls".to_string(),
            FakeLookup::matching_version(),
        );

        let error = resolve_local_binary(None, HostOs::Mac, &mut lookup).unwrap_err();

        assert!(error.contains("Could not verify vibe-xpls at /path/vibe-xpls"));
        assert!(error.contains("permission denied"));
        assert_eq!(lookup.probed, vec!["/path/vibe-xpls".to_string()]);
    }

    #[test]
    fn go_bin_mismatch_hard_fails() {
        let mut lookup = FakeLookup::default();
        lookup.env.insert("GOBIN".to_string(), "/gobin".to_string());
        lookup.probes.insert(
            "/gobin/vibe-xpls".to_string(),
            FakeLookup::mismatched_version("v0.0.3"),
        );

        let error = resolve_local_binary(None, HostOs::Mac, &mut lookup).unwrap_err();

        assert!(error.contains("Found vibe-xpls v0.0.3 at /gobin/vibe-xpls"));
        assert!(error.contains("requires vibe-xpls v0.0.2"));
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
        assert!(error.contains("expected `vibe-xpls v0.0.2`"));
    }

    #[test]
    fn failed_version_probe_errors() {
        let mut lookup = FakeLookup::default();
        lookup.env.insert("GOBIN".to_string(), "/gobin".to_string());
        lookup.probes.insert(
            "/gobin/vibe-xpls".to_string(),
            VersionProbeResult::Failed("permission denied".to_string()),
        );

        let error = resolve_local_binary(None, HostOs::Mac, &mut lookup).unwrap_err();

        assert!(error.contains("Could not verify vibe-xpls at /gobin/vibe-xpls"));
        assert!(error.contains("permission denied"));
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

    #[test]
    fn x86_is_unsupported() {
        let error = download_plan(HostOs::Linux, HostArch::X86).unwrap_err();
        assert!(error.contains("unsupported architecture"));
        assert!(error.contains("go install github.com/io41/vibe-xpls/cmd/vibe-xpls@v0.0.2"));
    }
}

pub fn default_args() -> Vec<String> {
    vec!["serve".to_string()]
}

pub fn manual_install_hint() -> String {
    format!("go install github.com/{VIBE_XPLS_REPO}/cmd/vibe-xpls@{VIBE_XPLS_VERSION}")
}

pub fn release_asset_url(asset_name: &str) -> String {
    format!(
        "https://github.com/{VIBE_XPLS_REPO}/releases/download/{VIBE_XPLS_VERSION}/{asset_name}"
    )
}

fn release_asset_sha256(os: HostOs, arch: HostArch) -> Result<&'static str, String> {
    match (os, arch) {
        (HostOs::Mac, HostArch::X8664) => {
            Ok("a034a9b2eab33ae30eb16909a65c2e885414104649a854a65b62940befba71de")
        }
        (HostOs::Mac, HostArch::Aarch64) => {
            Ok("d98a35fd57334b0c6d070d283b5ff9c12e46beca0a453c44230f621a0cf56454")
        }
        (HostOs::Linux, HostArch::X8664) => {
            Ok("d87f77237b3405a7388110ab65713e764e60338bc49239322272d017ac971d03")
        }
        (HostOs::Linux, HostArch::Aarch64) => {
            Ok("2b7735f6ec251fd381fa2b3f3e6ed7d1f55d702bde96893c809f1ff8ca37d018")
        }
        (HostOs::Windows, HostArch::X8664) => {
            Ok("f8bad966fe7970785a541aeffec7f7faf9e400d2256310aeb22220e8af826a94")
        }
        (HostOs::Windows, HostArch::Aarch64) => {
            Ok("87158951680b0fa942821ec28fa9d6492ca3b6cea81da42451b1ef33c2c3c0e5")
        }
        (_, HostArch::X86) => Err(format!(
            "unsupported architecture x86 for vibe-xpls {VIBE_XPLS_VERSION}; install manually with `{}`",
            manual_install_hint()
        )),
    }
}

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

fn local_binary_error(path: &str, message: impl AsRef<str>) -> String {
    format!(
        "Could not verify vibe-xpls at {path}. {}\n\nInstall the pinned server with:\n{}\n\nOr configure lsp.crossplane-yaml.binary.path if you intentionally want to use a different server version.",
        message.as_ref(),
        manual_install_hint()
    )
}

fn version_mismatch_error(path: &str, found: &str) -> String {
    format!(
        "Found vibe-xpls {found} at {path}, but crossplane-yaml requires vibe-xpls {VIBE_XPLS_VERSION}.\n\nInstall the pinned server with:\n{}\n\nOr configure lsp.crossplane-yaml.binary.path if you intentionally want to use a different server version.",
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

pub fn resolve_local_binary<L: LocalLookup>(
    settings: Option<BinarySettings>,
    os: HostOs,
    lookup: &mut L,
) -> Result<Option<LocalBinary>, String> {
    let (settings_path, args) = settings
        .map(|settings| {
            (
                settings.path,
                settings.arguments.unwrap_or_else(default_args),
            )
        })
        .unwrap_or_else(|| (None, default_args()));

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
}

fn host_binary_name(os: HostOs) -> &'static str {
    match os {
        HostOs::Windows => VIBE_XPLS_WINDOWS_BIN,
        HostOs::Mac | HostOs::Linux => VIBE_XPLS_BIN,
    }
}

fn go_bin_candidates<L: LocalLookup>(
    os: HostOs,
    binary_name: &str,
    lookup: &L,
) -> Vec<(String, String)> {
    let mut candidates = Vec::new();

    if let Some(gobin) = lookup.env_var("GOBIN").filter(|value| !value.is_empty()) {
        candidates.push(("GOBIN".to_string(), join_host_path(os, &gobin, binary_name)));
    }

    if let Some(gopath) = lookup
        .env_var("GOPATH")
        .and_then(|value| first_path_entry(os, &value).map(str::to_string))
    {
        candidates.push((
            "GOPATH".to_string(),
            join_host_path(os, &join_host_path(os, &gopath, "bin"), binary_name),
        ));
    }

    if let Some((source, home)) = home_go_bin_root(os, lookup) {
        candidates.push((
            source,
            join_host_path(
                os,
                &join_host_path(os, &join_host_path(os, &home, "go"), "bin"),
                binary_name,
            ),
        ));
    }

    candidates
}

fn home_go_bin_root<L: LocalLookup>(os: HostOs, lookup: &L) -> Option<(String, String)> {
    match os {
        HostOs::Windows => lookup
            .env_var("USERPROFILE")
            .filter(|value| !value.is_empty())
            .map(|home| ("USERPROFILE".to_string(), home))
            .or_else(|| {
                lookup
                    .env_var("HOME")
                    .filter(|value| !value.is_empty())
                    .map(|home| ("HOME".to_string(), home))
            }),
        HostOs::Mac | HostOs::Linux => lookup
            .env_var("HOME")
            .filter(|value| !value.is_empty())
            .map(|home| ("HOME".to_string(), home)),
    }
}

fn first_path_entry(os: HostOs, value: &str) -> Option<&str> {
    let separator = match os {
        HostOs::Windows => ';',
        HostOs::Mac | HostOs::Linux => ':',
    };

    value.split(separator).find(|entry| !entry.is_empty())
}

fn join_host_path(os: HostOs, left: &str, right: &str) -> String {
    let separator = match os {
        HostOs::Windows => "\\",
        HostOs::Mac | HostOs::Linux => "/",
    };
    let left = match os {
        HostOs::Windows => left.trim_end_matches(['/', '\\']),
        HostOs::Mac | HostOs::Linux => left.trim_end_matches('/'),
    };
    let right = match os {
        HostOs::Windows => right.trim_start_matches(['/', '\\']),
        HostOs::Mac | HostOs::Linux => right.trim_start_matches('/'),
    };

    format!("{left}{separator}{right}")
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
    let binary_path = format!("{version_dir}/{binary_name}");
    let temp_binary_path = format!("{temp_dir}/{binary_name}");
    let sha256 = release_asset_sha256(os, arch)?;

    let asset_name = format!("vibe-xpls_{VIBE_XPLS_VERSION}_{os_part}_{arch_part}.{extension}");
    let temp_archive_path = format!("{temp_dir}/{asset_name}");
    let download_url = release_asset_url(&asset_name);

    Ok(DownloadPlan {
        asset_name,
        download_url,
        version_dir,
        temp_dir,
        binary_path,
        temp_binary_path,
        temp_archive_path,
        archive_kind,
        sha256,
    })
}
