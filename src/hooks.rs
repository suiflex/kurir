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
