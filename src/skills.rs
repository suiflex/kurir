use crate::{Harness, Scope};

/// Directory a harness loads skills from, relative to the project root for
/// [`Scope::Project`] or the home directory for [`Scope::User`]. Install a skill
/// as a subdirectory of it.
///
/// `None` means the harness documents no skill directory for that scope.
#[must_use]
pub const fn skills_dir(harness: Harness, scope: Scope) -> Option<&'static str> {
    match (harness, scope) {
        (Harness::ClaudeCode | Harness::ClaudeCodeCli, Scope::Project | Scope::User) => {
            Some(".claude/skills")
        }
        // Codex and Cursor read the shared `.agents/skills` in both scopes.
        (Harness::Codex | Harness::Cursor, Scope::Project | Scope::User)
        | (
            Harness::OpenCode | Harness::Hermes | Harness::OpenClaw | Harness::AntigravityCli,
            Scope::Project,
        ) => Some(".agents/skills"),
        (Harness::OpenCode, Scope::User) => Some(".config/opencode/skills"),
        (Harness::Hermes, Scope::User) => Some(".hermes/skills"),
        (Harness::OpenClaw, Scope::User) => Some(".openclaw/skills"),
        (Harness::AntigravityCli, Scope::User) => Some(".gemini/antigravity-cli/skills"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_directories_follow_each_harness_scope() {
        let cases = [
            (Harness::ClaudeCode, Scope::User, Some(".claude/skills")),
            (Harness::Codex, Scope::User, Some(".agents/skills")),
            (Harness::Cursor, Scope::Project, Some(".agents/skills")),
            (Harness::OpenCode, Scope::Project, Some(".agents/skills")),
            (
                Harness::OpenCode,
                Scope::User,
                Some(".config/opencode/skills"),
            ),
            (Harness::Hermes, Scope::User, Some(".hermes/skills")),
            (Harness::OpenClaw, Scope::User, Some(".openclaw/skills")),
            (
                Harness::AntigravityCli,
                Scope::User,
                Some(".gemini/antigravity-cli/skills"),
            ),
            (Harness::ClaudeCode, Scope::Local, None),
            (Harness::Zed, Scope::User, None),
        ];
        for (harness, scope, expected) in cases {
            assert_eq!(skills_dir(harness, scope), expected, "{harness} {scope:?}");
        }
    }
}
