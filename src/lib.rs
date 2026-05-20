mod resolver;

use std::{collections::BTreeMap, fs};

use resolver::{
    default_args, download_plan, manual_install_hint, resolve_local_binary, ArchiveKind,
    BinarySettings, HostArch, HostOs, LocalLookup, VersionProbeResult, VIBE_XPLS_BIN,
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
    version_probes: BTreeMap<String, VersionProbeResult>,
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
            version_probes: BTreeMap::new(),
        }
    }
}

impl LocalLookup for ZedLookup<'_> {
    fn which(&mut self, binary: &str) -> Option<String> {
        if self.path_overridden {
            let shell_env = self.shell_env.clone();
            return which_on_env_path(binary, &shell_env, self.os, |path| self.probe_version(path));
        }

        self.worktree.which(binary)
    }

    fn env_var(&self, key: &str) -> Option<String> {
        self.shell_env.iter().find_map(|(candidate, value)| {
            env_key_eq(self.os, candidate, key).then(|| value.clone())
        })
    }

    fn probe_version(&mut self, path: &str) -> VersionProbeResult {
        if let Some(result) = self.version_probes.get(path) {
            return result.clone();
        }

        // Zed Extension API 0.7.0 does not expose process timeouts, so this
        // probe relies on the host process API rather than a custom watchdog.
        match fs::metadata(path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return VersionProbeResult::Missing;
            }
            Err(error) => {
                let result = VersionProbeResult::Failed(format!(
                    "could not inspect `{path}` before running `{VIBE_XPLS_BIN} --version`: {error}"
                ));
                self.version_probes.insert(path.to_string(), result.clone());
                return result;
            }
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
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let message = if stderr.trim().is_empty() {
                    format!("`{path} --version` exited with status {:?}", output.status)
                } else {
                    stderr
                };
                VersionProbeResult::Failed(message)
            }
            Err(error) => {
                VersionProbeResult::Failed(format!("could not run `{path} --version`: {error}"))
            }
        };

        self.version_probes.insert(path.to_string(), result.clone());
        result
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
    mut probe: impl FnMut(&str) -> VersionProbeResult,
) -> Option<String> {
    let path = env
        .iter()
        .find_map(|(key, value)| env_key_eq(os, key, "PATH").then_some(value))?;

    for dir in path.split(path_separator(os)).filter(|dir| !dir.is_empty()) {
        let candidate = join_host_path(os, dir, binary);
        if !matches!(probe(&candidate), VersionProbeResult::Missing) {
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
        fs::remove_dir_all(&plan.temp_dir).ok();
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::Downloading,
        );
        zed::download_file(
            &plan.download_url,
            &plan.temp_dir,
            zed_archive_kind(plan.archive_kind),
        )
        .map_err(|error| {
            fs::remove_dir_all(&plan.temp_dir).ok();
            friendly_download_error(&plan.asset_name, error)
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
            resolve_local_binary(resolver_binary_settings(settings.as_ref()), os, &mut lookup)?
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
        assert_eq!(VIBE_XPLS_VERSION, "v0.0.1");
        assert_eq!(VIBE_XPLS_BIN, "vibe-xpls");
    }

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
        let message =
            friendly_download_error("vibe-xpls_v0.0.1_linux_amd64.tar.gz", "status error 404");

        assert!(message.contains("pinned release asset was not found"));
        assert!(message.contains("vibe-xpls_v0.0.1_linux_amd64.tar.gz"));
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
            if candidate == "/custom/bin/vibe-xpls" {
                VersionProbeResult::Output {
                    stdout: "vibe-xpls v0.0.1\n".to_string(),
                    stderr: String::new(),
                }
            } else {
                VersionProbeResult::Missing
            }
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
    fn path_override_lookup_selects_failed_existing_candidate() {
        let env = vec![(
            "PATH".to_string(),
            "/missing:/broken:/custom/bin".to_string(),
        )];
        let mut probed = Vec::new();

        let found = which_on_env_path("vibe-xpls", &env, HostOs::Linux, |candidate| {
            probed.push(candidate.to_string());
            if candidate == "/broken/vibe-xpls" {
                VersionProbeResult::Failed("permission denied".to_string())
            } else if candidate == "/custom/bin/vibe-xpls" {
                VersionProbeResult::Output {
                    stdout: "vibe-xpls v0.0.1\n".to_string(),
                    stderr: String::new(),
                }
            } else {
                VersionProbeResult::Missing
            }
        });

        assert_eq!(found, Some("/broken/vibe-xpls".to_string()));
        assert_eq!(
            probed,
            vec![
                "/missing/vibe-xpls".to_string(),
                "/broken/vibe-xpls".to_string(),
            ]
        );
    }

    #[test]
    fn windows_path_override_lookup_accepts_path_key_casing() {
        let env = vec![("Path".to_string(), r"C:\custom\bin".to_string())];
        let mut probed = Vec::new();

        let found = which_on_env_path("vibe-xpls.exe", &env, HostOs::Windows, |candidate| {
            probed.push(candidate.to_string());
            if candidate == r"C:\custom\bin\vibe-xpls.exe" {
                VersionProbeResult::Output {
                    stdout: "vibe-xpls v0.0.1\n".to_string(),
                    stderr: String::new(),
                }
            } else {
                VersionProbeResult::Missing
            }
        });

        assert_eq!(found, Some(r"C:\custom\bin\vibe-xpls.exe".to_string()));
        assert_eq!(probed, vec![r"C:\custom\bin\vibe-xpls.exe".to_string()]);
    }
}
