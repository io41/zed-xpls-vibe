use zed_extension_api::{self as zed, Result};

const LANGUAGE_SERVER_ID: &str = "zed-xpls-vibe";
const MILESTONE_XPLS_BIN: &str = "<temporary-vibe-xpls-binary>";

struct ZedXplsVibeExtension;

fn default_vibe_xpls_args() -> Vec<String> {
    vec!["serve".to_string()]
}

impl zed::Extension for ZedXplsVibeExtension {
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

        Ok(zed::Command {
            command: MILESTONE_XPLS_BIN.to_string(),
            args: default_vibe_xpls_args(),
            env: worktree.shell_env(),
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
}
