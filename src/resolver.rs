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

    struct FakeLookup {
        which_path: Option<String>,
        which_calls: Vec<String>,
        env: std::collections::BTreeMap<String, String>,
        probeable: std::collections::BTreeSet<String>,
        probed: Vec<String>,
    }

    impl Default for FakeLookup {
        fn default() -> Self {
            Self {
                which_path: None,
                which_calls: Vec::new(),
                env: std::collections::BTreeMap::new(),
                probeable: std::collections::BTreeSet::new(),
                probed: Vec::new(),
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

        let binary = resolve_local_binary(Some(settings), HostOs::Mac, &mut lookup).unwrap();

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

        let binary = resolve_local_binary(Some(settings), HostOs::Mac, &mut lookup).unwrap();

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

        let binary = resolve_local_binary(Some(settings), HostOs::Mac, &mut lookup).unwrap();

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

        let binary = resolve_local_binary(Some(settings), HostOs::Mac, &mut lookup).unwrap();

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
        lookup
            .probeable
            .insert("/home/tim/go/bin/vibe-xpls".to_string());

        let binary = resolve_local_binary(None, HostOs::Mac, &mut lookup).unwrap();

        assert_eq!(binary.path, "/path/vibe-xpls");
        assert_eq!(binary.source, LocalBinarySource::Path);
        assert_eq!(lookup.which_calls, vec!["vibe-xpls".to_string()]);
        assert!(lookup.probed.is_empty());
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
        lookup.probeable.insert("/gopath/bin/vibe-xpls".to_string());

        let binary = resolve_local_binary(None, HostOs::Mac, &mut lookup).unwrap();

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
        lookup.probeable.insert("/first/bin/vibe-xpls".to_string());

        let binary = resolve_local_binary(None, HostOs::Linux, &mut lookup).unwrap();

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
        lookup
            .probeable
            .insert(r"D:\GoPath\bin\vibe-xpls.exe".to_string());

        let binary = resolve_local_binary(None, HostOs::Windows, &mut lookup).unwrap();

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
        lookup
            .probeable
            .insert(r"C:\Users\tim\go\bin\vibe-xpls.exe".to_string());

        let binary = resolve_local_binary(None, HostOs::Windows, &mut lookup).unwrap();

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

pub fn default_args() -> Vec<String> {
    vec!["serve".to_string()]
}

pub fn manual_install_hint() -> String {
    format!("go install github.com/{VIBE_XPLS_REPO}/cmd/vibe-xpls@{VIBE_XPLS_VERSION}")
}

pub fn resolve_local_binary<L: LocalLookup>(
    settings: Option<BinarySettings>,
    os: HostOs,
    lookup: &mut L,
) -> Option<LocalBinary> {
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
            return Some(LocalBinary {
                path,
                args,
                source: LocalBinarySource::UserSetting,
            });
        }
    }

    let binary_name = host_binary_name(os);
    if let Some(path) = lookup.which(binary_name) {
        return Some(LocalBinary {
            path,
            args: args.clone(),
            source: LocalBinarySource::Path,
        });
    }

    for (source, path) in go_bin_candidates(os, binary_name, lookup) {
        if lookup.probe_executable(&path) {
            return Some(LocalBinary {
                path,
                args: args.clone(),
                source: LocalBinarySource::GoBin(source),
            });
        }
    }

    None
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

    Ok(DownloadPlan {
        asset_name: format!("vibe-xpls_{VIBE_XPLS_VERSION}_{os_part}_{arch_part}.{extension}"),
        version_dir,
        temp_dir,
        binary_path,
        temp_binary_path,
        archive_kind,
    })
}
