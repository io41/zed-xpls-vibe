use zed_extension_api::{self as zed, Result};

const LANGUAGE_SERVER_ID: &str = "up-xpls";

struct UpXplsExtension;

fn is_crossplane_package_manifest(contents: &str) -> bool {
    let Some(api_version) = top_level_scalar(contents, "apiVersion") else {
        return false;
    };
    let Some(kind) = top_level_scalar(contents, "kind") else {
        return false;
    };

    let is_crossplane_meta = api_version.starts_with("meta.pkg.crossplane.io/")
        || api_version.starts_with("meta.pkg.upbound.io/");
    let is_package_kind = matches!(
        kind.as_str(),
        "Configuration" | "Provider" | "Function" | "AddOn"
    );

    is_crossplane_meta && is_package_kind
}

fn is_upbound_project_manifest(contents: &str) -> bool {
    let Some(api_version) = top_level_scalar(contents, "apiVersion") else {
        return false;
    };
    let Some(kind) = top_level_scalar(contents, "kind") else {
        return false;
    };

    api_version.starts_with("meta.dev.upbound.io/") && kind == "Project"
}

fn top_level_scalar(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");

    contents.lines().find_map(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            return None;
        }
        if line.len() != trimmed.len() {
            return None;
        }

        let value = trimmed.strip_prefix(&prefix)?.trim();
        Some(value.trim_matches('"').trim_matches('\'').to_string())
    })
}

fn xpls_args() -> Vec<String> {
    vec![
        "xpls".to_string(),
        "serve".to_string(),
        "--verbose".to_string(),
    ]
}

fn normalize_vibe_xpls_bin(value: &str) -> Option<String> {
    Some(value.trim().to_string()).filter(|value| !value.is_empty())
}

fn env_value(env: &[(String, String)], key: &str) -> Option<String> {
    env.iter()
        .find_map(|(name, value)| (name == key).then(|| value.as_str()))
        .and_then(normalize_vibe_xpls_bin)
}

fn missing_up_message() -> String {
    "Could not find the `up` CLI on PATH. Install it with `brew install upbound/tap/up` or `curl -sL https://cli.upbound.io | sh`, then restart Zed from a shell that can run `up xpls serve`."
        .to_string()
}

impl zed::Extension for UpXplsExtension {
    fn new() -> Self {
        Self
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

        let has_crossplane_manifest = match worktree.read_text_file("crossplane.yaml") {
            Ok(manifest) => is_crossplane_package_manifest(&manifest),
            Err(_) => false,
        };
        let has_upbound_project = match worktree.read_text_file("upbound.yaml") {
            Ok(manifest) => is_upbound_project_manifest(&manifest),
            Err(_) => false,
        };

        if !has_crossplane_manifest && !has_upbound_project {
            return Err(
                "No recognized root crossplane.yaml or upbound.yaml found; up xpls is only started for Crossplane package worktrees."
                    .to_string(),
            );
        }

        let env = worktree.shell_env();

        if let Some(command) = env_value(&env, "VIBE_XPLS_BIN") {
            return Ok(zed::Command {
                command,
                args: Vec::new(),
                env,
            });
        }

        Ok(zed::Command {
            command: worktree.which("up").ok_or_else(missing_up_message)?,
            args: xpls_args(),
            env,
        })
    }
}

zed::register_extension!(UpXplsExtension);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_crossplane_configuration_manifest() {
        let manifest = r#"
apiVersion: meta.pkg.crossplane.io/v1
kind: Configuration
metadata:
  name: platform-example
"#;

        assert!(is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn detects_upbound_addon_manifest() {
        let manifest = r#"
apiVersion: "meta.pkg.upbound.io/v1beta1"
kind: "AddOn"
metadata:
  name: addon-example
"#;

        assert!(is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn rejects_non_package_yaml() {
        let manifest = r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
"#;

        assert!(!is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn detects_upbound_project_manifest() {
        let manifest = r#"
apiVersion: meta.dev.upbound.io/v2alpha1
kind: Project
metadata:
  name: platform-example
"#;

        assert!(is_upbound_project_manifest(manifest));
    }

    #[test]
    fn rejects_non_project_upbound_yaml() {
        let manifest = r#"
apiVersion: meta.dev.upbound.io/v2alpha1
kind: Widget
metadata:
  name: platform-example
"#;

        assert!(!is_upbound_project_manifest(manifest));
    }

    #[test]
    fn ignores_nested_keys() {
        let manifest = r#"
metadata:
  apiVersion: meta.pkg.crossplane.io/v1
  kind: Configuration
"#;

        assert!(!is_crossplane_package_manifest(manifest));
    }

    #[test]
    fn starts_xpls_over_stdio_compatible_command() {
        assert_eq!(
            xpls_args(),
            vec![
                "xpls".to_string(),
                "serve".to_string(),
                "--verbose".to_string()
            ]
        );
    }

    #[test]
    fn ignores_empty_vibe_xpls_override() {
        assert_eq!(normalize_vibe_xpls_bin(" "), None);
    }

    #[test]
    fn trims_vibe_xpls_override() {
        assert_eq!(
            normalize_vibe_xpls_bin(" /tmp/vibe-xpls "),
            Some("/tmp/vibe-xpls".to_string())
        );
    }

    #[test]
    fn reads_vibe_xpls_override_from_shell_env() {
        let env = vec![
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("VIBE_XPLS_BIN".to_string(), " /tmp/vibe-xpls ".to_string()),
        ];

        assert_eq!(
            env_value(&env, "VIBE_XPLS_BIN"),
            Some("/tmp/vibe-xpls".to_string())
        );
    }

    #[test]
    fn missing_up_message_is_actionable() {
        let message = missing_up_message();
        assert!(message.contains("brew install upbound/tap/up"));
        assert!(message.contains("https://cli.upbound.io"));
    }
}
