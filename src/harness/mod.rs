use std::{fmt, str::FromStr};

use crate::Error;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Harness {
    ClaudeCode,
    ClaudeCodeCli,
    ClaudeDesktop,
    Codex,
    Cursor,
    Vscode,
    GeminiCli,
    CopilotCli,
    OpenCode,
    Windsurf,
    Zed,
    OpenClaw,
    Hermes,
    Omp,
    AntigravityCli,
    AntigravityDesktop,
}

impl Harness {
    pub const ALL: [Self; 16] = [
        Self::ClaudeCode,
        Self::ClaudeCodeCli,
        Self::ClaudeDesktop,
        Self::Codex,
        Self::Cursor,
        Self::Vscode,
        Self::GeminiCli,
        Self::CopilotCli,
        Self::OpenCode,
        Self::Windsurf,
        Self::Zed,
        Self::OpenClaw,
        Self::Hermes,
        Self::Omp,
        Self::AntigravityCli,
        Self::AntigravityDesktop,
    ];

    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::ClaudeCodeCli => "claude-code-cli",
            Self::ClaudeDesktop => "claude-desktop",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::Vscode => "vscode",
            Self::GeminiCli => "gemini-cli",
            Self::CopilotCli => "copilot-cli",
            Self::OpenCode => "opencode",
            Self::Windsurf => "windsurf",
            Self::Zed => "zed",
            Self::OpenClaw => "openclaw",
            Self::Hermes => "hermes",
            Self::Omp => "omp",
            Self::AntigravityCli => "antigravity-cli",
            Self::AntigravityDesktop => "antigravity-desktop",
        }
    }

    #[must_use]
    pub const fn summary(self) -> &'static str {
        match self {
            Self::ClaudeCode => "merge ~/.claude.json or project .mcp.json",
            Self::ClaudeCodeCli => "delegate to claude mcp add",
            Self::ClaudeDesktop => "merge Claude Desktop config",
            Self::Codex => "delegate to codex mcp add",
            Self::Cursor => "merge Cursor mcp.json",
            Self::Vscode => "delegate to code --add-mcp",
            Self::GeminiCli => "delegate to gemini mcp add",
            Self::CopilotCli => "merge ~/.copilot/mcp-config.json",
            Self::OpenCode => "merge opencode.json",
            Self::Windsurf => "merge ~/.codeium/windsurf/mcp_config.json",
            Self::Zed => "merge Zed context_servers",
            Self::OpenClaw => "merge openclaw.json mcp.servers",
            Self::Hermes => "delegate to hermes mcp add",
            Self::Omp => "print a portable snippet",
            Self::AntigravityCli => "merge Antigravity CLI mcp_config.json",
            Self::AntigravityDesktop => "merge Antigravity Desktop mcp_config.json",
        }
    }

    #[must_use]
    pub const fn is_delegated(self) -> bool {
        matches!(
            self,
            Self::ClaudeCodeCli | Self::Codex | Self::Vscode | Self::GeminiCli | Self::Hermes
        )
    }

    #[must_use]
    pub const fn is_snippet_only(self) -> bool {
        matches!(self, Self::Omp)
    }
}

impl FromStr for Harness {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "claude" | "claude-code" => Ok(Self::ClaudeCode),
            "claude-code-cli" => Ok(Self::ClaudeCodeCli),
            "claude-desktop" => Ok(Self::ClaudeDesktop),
            "codex" => Ok(Self::Codex),
            "cursor" => Ok(Self::Cursor),
            "vscode" | "copilot-vscode" => Ok(Self::Vscode),
            "gemini" | "gemini-cli" => Ok(Self::GeminiCli),
            "copilot-cli" => Ok(Self::CopilotCli),
            "opencode" => Ok(Self::OpenCode),
            "windsurf" => Ok(Self::Windsurf),
            "zed" => Ok(Self::Zed),
            "openclaw" => Ok(Self::OpenClaw),
            "hermes" => Ok(Self::Hermes),
            "omp" => Ok(Self::Omp),
            "antigravity" | "antigravity-cli" => Ok(Self::AntigravityCli),
            "antigravity-desktop" => Ok(Self::AntigravityDesktop),
            _ => Err(Error::UnknownHarness(value.to_owned())),
        }
    }
}

impl fmt::Display for Harness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_harness_ids_round_trip() {
        for harness in Harness::ALL {
            assert_eq!(
                harness.id().parse::<Harness>().expect("known harness"),
                harness
            );
        }
    }

    #[test]
    fn antigravity_alias_is_cli_surface() {
        assert_eq!(
            "antigravity".parse::<Harness>().expect("known alias"),
            Harness::AntigravityCli
        );
    }
}
