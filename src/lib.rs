mod resolver;

use std::{collections::BTreeMap, fs};

use resolver::{
    default_args, download_plan, manual_install_hint, resolve_local_binary, ArchiveKind,
    BinarySettings, HostArch, HostOs, LocalLookup, VIBE_XPLS_BIN, VIBE_XPLS_REPO,
    VIBE_XPLS_VERSION,
};
use zed::settings::LspSettings;
use zed_extension_api::{self as zed, Result};

const LANGUAGE_SERVER_ID: &str = "zed-xpls-vibe";

struct ZedXplsVibeExtension {
    cached_downloaded_binary: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RuntimeBinarySettings {
    path: Option<String>,
    arguments: Option<Vec<String>>,
    env: Option<BTreeMap<String, String>>,
}

impl RuntimeBinarySettings {
    fn resolver_settings(&self) -> BinarySettings {
        BinarySettings {
            path: self.path.clone(),
            arguments: self.arguments.clone(),
        }
    }
}

struct ZedLookup<'a> {
    worktree: &'a zed::Worktree,
    shell_env: Vec<(String, String)>,
    os: HostOs,
    path_overridden: bool,
}

impl<'a> ZedLookup<'a> {
    fn new(
        worktree: &'a zed::Worktree,
        shell_env: Vec<(String, String)>,
        os: HostOs,
        path_overridden: bool,
    ) -> Self {
        Self {
            worktree,
            shell_env,
            os,
            path_overridden,
        }
    }
}

impl LocalLookup for ZedLookup<'_> {
    fn which(&mut self, binary: &str) -> Option<String> {
        if self.path_overridden {
            let shell_env = self.shell_env.clone();
            return which_on_env_path(binary, &shell_env, self.os, |path| {
                self.probe_executable(path)
            });
        }

        self.worktree.which(binary)
    }

    fn env_var(&self, key: &str) -> Option<String> {
        self.shell_env.iter().find_map(|(candidate, value)| {
            env_key_eq(self.os, candidate, key).then(|| value.clone())
        })
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

fn binary_settings(worktree: &zed::Worktree) -> Option<RuntimeBinarySettings> {
    LspSettings::for_worktree(LANGUAGE_SERVER_ID, worktree)
        .ok()
        .and_then(|settings| settings.binary)
        .map(|binary| RuntimeBinarySettings {
            path: binary.path,
            arguments: binary.arguments,
            env: binary.env.map(|env| env.into_iter().collect()),
        })
}

fn resolver_binary_settings(settings: Option<&RuntimeBinarySettings>) -> Option<BinarySettings> {
    settings.map(RuntimeBinarySettings::resolver_settings)
}

fn effective_args(settings: Option<&RuntimeBinarySettings>) -> Vec<String> {
    settings
        .and_then(|settings| settings.arguments.clone())
        .unwrap_or_else(default_vibe_xpls_args)
}

fn merged_env(
    os: HostOs,
    shell_env: Vec<(String, String)>,
    settings: Option<&RuntimeBinarySettings>,
) -> Vec<(String, String)> {
    let Some(overrides) = settings.and_then(|settings| settings.env.as_ref()) else {
        return shell_env;
    };

    let mut env = shell_env;
    for (key, value) in overrides {
        if let Some((_, existing_value)) = env
            .iter_mut()
            .find(|(existing_key, _)| env_key_eq(os, existing_key, key))
        {
            *existing_value = value.clone();
        } else {
            env.push((key.clone(), value.clone()));
        }
    }

    env
}

fn overrides_path(os: HostOs, settings: Option<&RuntimeBinarySettings>) -> bool {
    settings
        .and_then(|settings| settings.env.as_ref())
        .is_some_and(|env| env.keys().any(|key| env_key_eq(os, key, "PATH")))
}

fn which_on_env_path(
    binary: &str,
    env: &[(String, String)],
    os: HostOs,
    mut probe: impl FnMut(&str) -> bool,
) -> Option<String> {
    let path = env
        .iter()
        .find_map(|(key, value)| env_key_eq(os, key, "PATH").then_some(value))?;

    for dir in path.split(path_separator(os)).filter(|dir| !dir.is_empty()) {
        let candidate = join_host_path(os, dir, binary);
        if probe(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn env_key_eq(os: HostOs, left: &str, right: &str) -> bool {
    match os {
        HostOs::Windows => left.eq_ignore_ascii_case(right),
        HostOs::Mac | HostOs::Linux => left == right,
    }
}

fn path_separator(os: HostOs) -> char {
    match os {
        HostOs::Windows => ';',
        HostOs::Mac | HostOs::Linux => ':',
    }
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
            .map_err(|error| {
                format!(
                    "failed to fetch {VIBE_XPLS_BIN} {VIBE_XPLS_VERSION}: {error}; install manually with `{}`",
                    manual_install_hint()
                )
            })?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == plan.asset_name)
            .ok_or_else(|| {
                format!(
                    "{VIBE_XPLS_BIN} {VIBE_XPLS_VERSION} release is missing asset `{}`; install manually with `{}`",
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
            format!(
                "failed to download `{}`: {error}; install manually with `{}`",
                plan.asset_name,
                manual_install_hint()
            )
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
            format!(
                "failed to make `{}` executable: {error}",
                plan.temp_binary_path
            )
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

        let (os, _) = host_platform()?;
        let settings = binary_settings(worktree);
        let args = effective_args(settings.as_ref());
        let path_overridden = overrides_path(os, settings.as_ref());
        let env = merged_env(os, worktree.shell_env(), settings.as_ref());
        let mut lookup = ZedLookup::new(worktree, env.clone(), os, path_overridden);
        if let Some(binary) =
            resolve_local_binary(resolver_binary_settings(settings.as_ref()), os, &mut lookup)
        {
            return Ok(zed::Command {
                command: binary.path,
                args: binary.args,
                env,
            });
        }

        Ok(zed::Command {
            command: self.downloaded_binary_path(language_server_id)?,
            args,
            env,
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

    #[test]
    fn configured_arguments_are_effective_without_path_override() {
        let settings = RuntimeBinarySettings {
            path: None,
            arguments: Some(vec!["serve".to_string(), "--debug".to_string()]),
            env: None,
        };

        assert_eq!(
            effective_args(Some(&settings)),
            vec!["serve".to_string(), "--debug".to_string()]
        );
    }

    #[test]
    fn binary_env_overrides_shell_env() {
        let settings = RuntimeBinarySettings {
            path: None,
            arguments: None,
            env: Some(BTreeMap::from([
                ("GOBIN".to_string(), "/override/bin".to_string()),
                ("VIBE_XPLS_LOG".to_string(), "debug".to_string()),
            ])),
        };

        let env = merged_env(
            HostOs::Linux,
            vec![
                ("PATH".to_string(), "/usr/bin".to_string()),
                ("GOBIN".to_string(), "/shell/bin".to_string()),
            ],
            Some(&settings),
        );

        assert_eq!(
            env,
            vec![
                ("PATH".to_string(), "/usr/bin".to_string()),
                ("GOBIN".to_string(), "/override/bin".to_string()),
                ("VIBE_XPLS_LOG".to_string(), "debug".to_string()),
            ]
        );
    }

    #[test]
    fn windows_binary_env_overrides_existing_key_case_insensitively() {
        let settings = RuntimeBinarySettings {
            path: None,
            arguments: None,
            env: Some(BTreeMap::from([
                ("Path".to_string(), r"C:\custom\bin".to_string()),
                ("gobin".to_string(), r"C:\go\bin".to_string()),
            ])),
        };

        assert!(overrides_path(HostOs::Windows, Some(&settings)));

        let env = merged_env(
            HostOs::Windows,
            vec![
                ("PATH".to_string(), r"C:\Windows\System32".to_string()),
                ("GOBIN".to_string(), r"C:\old\go\bin".to_string()),
            ],
            Some(&settings),
        );

        assert_eq!(
            env,
            vec![
                ("PATH".to_string(), r"C:\custom\bin".to_string()),
                ("GOBIN".to_string(), r"C:\go\bin".to_string()),
            ]
        );
    }

    #[test]
    fn path_override_lookup_uses_merged_env_path() {
        let env = vec![("PATH".to_string(), "/missing:/custom/bin".to_string())];
        let mut probed = Vec::new();

        let found = which_on_env_path("vibe-xpls", &env, HostOs::Linux, |candidate| {
            probed.push(candidate.to_string());
            candidate == "/custom/bin/vibe-xpls"
        });

        assert_eq!(found, Some("/custom/bin/vibe-xpls".to_string()));
        assert_eq!(
            probed,
            vec![
                "/missing/vibe-xpls".to_string(),
                "/custom/bin/vibe-xpls".to_string(),
            ]
        );
    }

    #[test]
    fn windows_path_override_lookup_accepts_path_key_casing() {
        let env = vec![("Path".to_string(), r"C:\custom\bin".to_string())];
        let mut probed = Vec::new();

        let found = which_on_env_path("vibe-xpls.exe", &env, HostOs::Windows, |candidate| {
            probed.push(candidate.to_string());
            candidate == r"C:\custom\bin\vibe-xpls.exe"
        });

        assert_eq!(found, Some(r"C:\custom\bin\vibe-xpls.exe".to_string()));
        assert_eq!(probed, vec![r"C:\custom\bin\vibe-xpls.exe".to_string()]);
    }
}
