use std::{
    env,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    Error, Harness, Scope, jsonc,
    model::{RegistrationOptions, RegistrationResult},
};

/// One lifecycle hook a product asks a harness to run.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookSpec {
    /// Event name exactly as the harness spells it (`Stop`, `sessionStart`, ...).
    pub event: String,
    /// Tool or source matcher, when the event supports one.
    pub matcher: Option<String>,
    pub command: String,
    pub timeout_seconds: u64,
}

impl HookSpec {
    /// Validate required hook fields.
    ///
    /// # Errors
    ///
    /// Returns an error when event or command is empty.
    pub fn validate(&self) -> Result<(), Error> {
        if self.event.trim().is_empty() {
            return Err(Error::InvalidArgument {
                field: "event",
                value: self.event.clone(),
                expected: "a non-empty hook event name",
            });
        }
        if self.command.trim().is_empty() {
            return Err(Error::InvalidArgument {
                field: "command",
                value: self.command.clone(),
                expected: "a non-empty hook command",
            });
        }
        Ok(())
    }
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

/// Resolve the hook configuration path for a harness and scope.
///
/// # Errors
///
/// Returns an error if the harness does not support hooks or target path
/// cannot be resolved.
pub fn hook_path(harness: Harness, options: &RegistrationOptions) -> Result<PathBuf, Error> {
    if let Some(path) = &options.config {
        return Ok(path.clone());
    }
    let relative = hook_file(harness).ok_or_else(|| Error::Unsupported {
        harness: harness.id().to_owned(),
        detail: "hook registration".to_owned(),
    })?;
    match options.scope {
        Scope::Project => Ok(options.cwd.join(relative)),
        Scope::User => {
            let home = env::home_dir().ok_or(Error::MissingArgument("home directory"))?;
            Ok(home.join(relative))
        }
        Scope::Local => Err(Error::Unsupported {
            harness: harness.id().to_owned(),
            detail: "local scope for hooks".to_owned(),
        }),
    }
}

/// Check if a hook matching `spec` is already configured in `document`.
#[must_use]
pub fn is_hook_present(harness: Harness, document: &Value, spec: &HookSpec) -> bool {
    let Some(handlers) = document
        .get("hooks")
        .and_then(|h| h.get(&spec.event))
        .and_then(Value::as_array)
    else {
        return false;
    };

    match harness {
        Harness::Cursor => handlers.iter().any(|h| {
            h.get("command").and_then(Value::as_str) == Some(&spec.command)
                && h.get("matcher").and_then(Value::as_str) == spec.matcher.as_deref()
        }),
        Harness::ClaudeCode | Harness::ClaudeCodeCli | Harness::Codex => handlers.iter().any(|h| {
            let matcher_match = h.get("matcher").and_then(Value::as_str) == spec.matcher.as_deref();
            let command_match = h.get("hooks").and_then(Value::as_array).is_some_and(|sub| {
                sub.iter()
                    .any(|item| item.get("command").and_then(Value::as_str) == Some(&spec.command))
            });
            matcher_match && command_match
        }),
        _ => false,
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

/// Register a lifecycle hook with a harness.
///
/// Loads the configuration file, merges the hook entry safely, backs up the
/// previous file, and writes the updated document.
///
/// # Errors
///
/// Returns an error when hook validation fails, the harness is unsupported,
/// or JSON/JSONC file I/O fails.
pub fn register_hook(
    harness: Harness,
    spec: &HookSpec,
    options: &RegistrationOptions,
) -> Result<RegistrationResult, Error> {
    spec.validate()?;
    let path = hook_path(harness, options)?;
    let mut root = jsonc::load_object(&path)?;

    if is_hook_present(harness, &root, spec) {
        return Ok(RegistrationResult {
            harness: harness.id().to_owned(),
            name: spec.event.clone(),
            target: Some(path),
            changed: false,
            action: "already-configured".to_owned(),
        });
    }

    if options.print {
        let mut preview_doc = json!({});
        add_hook(harness, &mut preview_doc, spec, &path)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&preview_doc).map_err(|source| Error::InvalidJson {
                path: "stdout".to_owned(),
                source,
            })?
        );
    }

    if options.dry_run {
        return Ok(RegistrationResult {
            harness: harness.id().to_owned(),
            name: spec.event.clone(),
            target: Some(path),
            changed: false,
            action: "dry-run".to_owned(),
        });
    }

    jsonc::backup(&path)?;
    add_hook(harness, &mut root, spec, &path)?;
    jsonc::write_object(&path, &root)?;

    Ok(RegistrationResult {
        harness: harness.id().to_owned(),
        name: spec.event.clone(),
        target: Some(path),
        changed: true,
        action: "registered".to_owned(),
    })
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

    #[test]
    fn register_hook_writes_to_file_and_creates_backup() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("settings.json");
        std::fs::write(&config_path, "{}").expect("write");

        let options = RegistrationOptions {
            config: Some(config_path.clone()),
            ..RegistrationOptions::default()
        };
        let hook = spec(None);

        let result = register_hook(Harness::Cursor, &hook, &options).expect("register");
        assert!(result.changed);
        assert_eq!(result.action, "registered");

        let backup_path = config_path.with_extension("json.bak");
        assert!(backup_path.exists());

        let result2 = register_hook(Harness::Cursor, &hook, &options).expect("register again");
        assert!(!result2.changed);
        assert_eq!(result2.action, "already-configured");
    }

    #[test]
    fn register_hook_dry_run_does_not_modify_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("hooks.json");
        std::fs::write(&config_path, "{}").expect("write");

        let options = RegistrationOptions {
            config: Some(config_path.clone()),
            dry_run: true,
            ..RegistrationOptions::default()
        };
        let hook = spec(None);

        let result = register_hook(Harness::Codex, &hook, &options).expect("dry run");
        assert!(!result.changed);
        assert_eq!(result.action, "dry-run");

        let content = std::fs::read_to_string(&config_path).expect("read");
        assert_eq!(content, "{}");
    }

    #[test]
    fn register_hook_rejects_empty_command_or_event() {
        let options = RegistrationOptions::default();
        let invalid = HookSpec {
            event: "  ".to_owned(),
            matcher: None,
            command: "echo test".to_owned(),
            timeout_seconds: 5,
        };
        assert!(register_hook(Harness::Cursor, &invalid, &options).is_err());
    }
}
