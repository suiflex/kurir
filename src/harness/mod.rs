use std::{fmt, str::FromStr};

use crate::Error;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum Harness {
    ArsyCode,
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
    pub const ALL: [Self; 17] = [
        Self::ArsyCode,
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

    const IDS: [&'static str; 17] = [
        "arsy-code",
        "claude-code",
        "claude-code-cli",
        "claude-desktop",
        "codex",
        "cursor",
        "vscode",
        "gemini-cli",
        "copilot-cli",
        "opencode",
        "windsurf",
        "zed",
        "openclaw",
        "hermes",
        "omp",
        "antigravity-cli",
        "antigravity-desktop",
    ];

    const SUMMARIES: [&'static str; 17] = [
        "delegate to arsy mcp add",
        "merge ~/.claude.json or project .mcp.json",
        "delegate to claude mcp add",
        "merge Claude Desktop config",
        "delegate to codex mcp add",
        "merge Cursor mcp.json",
        "delegate to code --add-mcp",
        "delegate to gemini mcp add",
        "merge ~/.copilot/mcp-config.json",
        "merge opencode.json",
        "merge ~/.codeium/windsurf/mcp_config.json",
        "merge Zed context_servers",
        "merge openclaw.json mcp.servers",
        "delegate to hermes mcp add",
        "print a portable snippet",
        "merge Antigravity CLI mcp_config.json",
        "merge Antigravity Desktop mcp_config.json",
    ];

    const ALIASES: [(&'static str, Self); 6] = [
        ("arsy", Self::ArsyCode),
        ("claude", Self::ClaudeCode),
        ("vscode", Self::Vscode),
        ("copilot-vscode", Self::Vscode),
        ("gemini", Self::GeminiCli),
        ("antigravity", Self::AntigravityCli),
    ];

    #[must_use]
    pub const fn id(self) -> &'static str {
        Self::IDS[self as usize]
    }

    #[must_use]
    pub const fn summary(self) -> &'static str {
        Self::SUMMARIES[self as usize]
    }

    #[must_use]
    pub const fn is_delegated(self) -> bool {
        matches!(
            self,
            Self::ArsyCode
                | Self::ClaudeCodeCli
                | Self::Codex
                | Self::Vscode
                | Self::GeminiCli
                | Self::Hermes
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
        Self::ALIASES
            .iter()
            .find_map(|(alias, harness)| (*alias == value).then_some(*harness))
            .or_else(|| Self::ALL.into_iter().find(|harness| harness.id() == value))
            .ok_or_else(|| Error::UnknownHarness(value.to_owned()))
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
