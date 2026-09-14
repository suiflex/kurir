use std::path::Path;

use serde_json::{Value, json};

use crate::{Error, Harness, jsonc};

/// One lifecycle hook a product asks a harness to run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookSpec {
    /// Event name exactly as the harness spells it (`Stop`, `sessionStart`, ...).
    pub event: String,
    /// Tool or source matcher, when the event supports one.
    pub matcher: Option<String>,
    pub command: String,
    pub timeout_seconds: u64,
}

/// Hook configuration file for a harness, relative to the project root or the
/// home directory. Both scopes use the same relative path.
#[must_use]
pub const fn hook_file(harness: Harness) -> Option<&'static str> {
    match harness {
        Harness::ClaudeCode | Harness::ClaudeCodeCli => Some(".claude/settings.json"),
        Harness::Codex => Some(".codex/hooks.json"),
        Harness::Cursor => Some(".cursor/hooks.json"),
        _ => None,
    }
}

/// Append `spec` to a hook document in the entry shape `harness` reads.
///
/// The document is only mutated; loading, de-duplication, and writing stay with
/// the caller, which owns the policy for existing entries.
///
/// # Errors
///
/// Returns an error when the harness has no hook file or an existing field in
/// the document has the wrong type.
pub fn add_hook(
    harness: Harness,
    document: &mut Value,
    spec: &HookSpec,
    path: &Path,
) -> Result<(), Error> {
    let handler = match harness {
        Harness::ClaudeCode | Harness::ClaudeCodeCli | Harness::Codex => {
            let mut handler = json!({"hooks": [{
                "type": "command",
                "command": spec.command,
                "timeout": spec.timeout_seconds,
            }]});
            if let Some(matcher) = &spec.matcher {
                handler["matcher"] = json!(matcher);
            }
            handler
        }
        Harness::Cursor => {
            if let Some(root) = document.as_object_mut() {
                root.entry("version").or_insert(json!(1));
            }
            let mut handler = json!({
                "command": spec.command,
                "timeout": spec.timeout_seconds,
            });
            if let Some(matcher) = &spec.matcher {
                handler["matcher"] = json!(matcher);
            }
            handler
        }
        _ => {
            return Err(Error::Unsupported {
                harness: harness.id().to_owned(),
                detail: "hook registration".to_owned(),
            });
        }
    };
    let hooks = jsonc::ensure_object_path(document, &["hooks"], path)?;
    let handlers = hooks
        .entry(spec.event.clone())
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| Error::InvalidConfigField {
            path: path.display().to_string(),
            field: format!("hooks.{}", spec.event),
        })?;
    handlers.push(handler);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(matcher: Option<&str>) -> HookSpec {
        HookSpec {
            event: "PreToolUse".to_owned(),
            matcher: matcher.map(str::to_owned),
            command: "demo hook".to_owned(),
            timeout_seconds: 5,
        }
    }

    #[test]
    fn grouped_harnesses_nest_the_command_under_hooks() {
        for harness in [Harness::ClaudeCode, Harness::Codex] {
            let mut document = json!({"keep": true});
            add_hook(
                harness,
                &mut document,
                &spec(Some("Edit")),
                Path::new("h.json"),
            )
            .expect("hook");
            assert_eq!(document["keep"], true);
            assert_eq!(
                document["hooks"]["PreToolUse"],
                json!([{"matcher": "Edit", "hooks": [{"type": "command", "command": "demo hook", "timeout": 5}]}])
            );
        }
    }

    #[test]
    fn cursor_uses_flat_handlers_and_a_version() {
        let mut document = json!({});
        add_hook(
            Harness::Cursor,
            &mut document,
            &spec(None),
            Path::new("h.json"),
        )
        .expect("hook");
        assert_eq!(
            document,
            json!({"version": 1, "hooks": {"PreToolUse": [{"command": "demo hook", "timeout": 5}]}})
        );
    }

    #[test]
    fn existing_handlers_are_appended_to() {
        let mut document = json!({"hooks": {"PreToolUse": [{"command": "other"}]}});
        add_hook(
            Harness::Cursor,
            &mut document,
            &spec(None),
            Path::new("h.json"),
        )
        .expect("hook");
        assert_eq!(
            document["hooks"]["PreToolUse"].as_array().map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn malformed_event_field_is_rejected() {
        let mut document = json!({"hooks": {"PreToolUse": {}}});
        let error = add_hook(
            Harness::Codex,
            &mut document,
            &spec(None),
            Path::new("h.json"),
        )
        .expect_err("not an array");
        assert!(matches!(error, Error::InvalidConfigField { .. }));
    }

    #[test]
    fn harness_without_hooks_is_unsupported() {
        assert_eq!(hook_file(Harness::Zed), None);
        let error = add_hook(
            Harness::Zed,
            &mut json!({}),
            &spec(None),
            Path::new("h.json"),
        )
        .expect_err("unsupported");
        assert!(matches!(error, Error::Unsupported { .. }));
    }
}
