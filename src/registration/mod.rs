use std::{
    collections::BTreeMap,
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::{
    Error, Harness, Scope, ServerSpec, Transport, jsonc,
    model::{RegistrationOptions, RegistrationResult},
};

#[derive(Clone, Debug, Serialize)]
pub struct DoctorReport {
    pub harness: String,
    pub kind: &'static str,
    pub target: Option<PathBuf>,
    pub available: bool,
    pub detail: String,
}

/// Register one server with a harness.
///
/// # Errors
/// Returns an error when validation, config I/O, or delegated execution fails.
pub fn register(
    harness: Harness,
    spec: &ServerSpec,
    options: &RegistrationOptions,
) -> Result<RegistrationResult, Error> {
    spec.validate()?;
    if harness.is_snippet_only() {
        return Ok(result(harness, spec, None, false, "snippet-only"));
    }
    if harness.is_delegated() {
        return delegated(harness, spec, options);
    }
    file_registration(harness, spec, options)
}

/// Inspect one harness or the complete harness registry.
///
/// # Errors
/// Returns an error when a target path cannot be resolved.
pub fn doctor(
    harness: Option<Harness>,
    options: &RegistrationOptions,
) -> Result<Vec<DoctorReport>, Error> {
    harness
        .map_or_else(|| Harness::ALL.to_vec(), |value| vec![value])
        .into_iter()
        .map(|value| doctor_one(value, options))
        .collect()
}

/// Build a harness-specific server entry.
///
/// # Errors
/// Returns an error when the server spec is invalid or unsupported.
pub fn entry_for(harness: Harness, spec: &ServerSpec) -> Result<Value, Error> {
    spec.validate()?;
    match harness {
        Harness::OpenCode => open_code_entry(spec),
        Harness::OpenClaw => open_claw_entry(spec),
        Harness::Zed => zed_entry(spec),
        Harness::CopilotCli => copilot_entry(spec),
        Harness::Jcode | Harness::Zcode => typed_standard_entry(harness, spec),
        _ => standard_entry(spec),
    }
}

#[must_use]
pub fn snippet_for(spec: &ServerSpec) -> Value {
    json!({"mcpServers": {spec.name.clone(): redact(&standard_entry_unchecked(spec))}})
}

fn file_registration(
    harness: Harness,
    spec: &ServerSpec,
    options: &RegistrationOptions,
) -> Result<RegistrationResult, Error> {
    if harness == Harness::Zcode
        && options.scope == Scope::Project
        && (!spec.env.is_empty() || !spec.headers.is_empty())
    {
        return unsupported(harness, "project MCP credentials; use user scope");
    }
    let path = resolved_target_path(harness, options)?;
    let entry = entry_for(harness, spec)?;
    let mut root = jsonc::load_object(&path)?;
    let servers = jsonc::ensure_object_path(&mut root, key_path(harness), &path)?;
    if let Some(existing) = servers.get(&spec.name) {
        if existing == &entry {
            return Ok(result(
                harness,
                spec,
                Some(path),
                false,
                "already-configured",
            ));
        }
        if !options.force {
            return Err(Error::Conflict {
                name: spec.name.clone(),
            });
        }
    }
    servers.insert(spec.name.clone(), entry);
    if options.print {
        println!(
            "{}",
            serde_json::to_string_pretty(&redacted_snippet(harness, spec))
                .map_err(|e| Error::Update(e.to_string()))?
        );
    }
    if options.dry_run {
        return Ok(result(harness, spec, Some(path), false, "dry-run"));
    }
    jsonc::backup(&path)?;
    jsonc::write_object(&path, &root)?;
    Ok(result(harness, spec, Some(path), true, "registered"))
}

fn result(
    harness: Harness,
    spec: &ServerSpec,
    target: Option<PathBuf>,
    changed: bool,
    action: &str,
) -> RegistrationResult {
    RegistrationResult {
        harness: harness.id().to_owned(),
        name: spec.name.clone(),
        target,
        changed,
        action: action.to_owned(),
    }
}

fn resolved_target_path(harness: Harness, options: &RegistrationOptions) -> Result<PathBuf, Error> {
    if matches!(harness, Harness::Jcode | Harness::Zcode) {
        return new_harness_target_path(harness, options);
    }
    target_path(harness, options)
}

fn target_path(harness: Harness, options: &RegistrationOptions) -> Result<PathBuf, Error> {
    if let Some(path) = &options.config {
        return Ok(path.clone());
    }
    let home = env::home_dir().ok_or(Error::MissingArgument("home directory"))?;
    let path = match harness {
        Harness::ClaudeCode => match options.scope {
            Scope::Project => options.cwd.join(".mcp.json"),
            Scope::User => home.join(".claude.json"),
            Scope::Local => return unsupported(harness, "local file scope"),
        },
        Harness::ClaudeDesktop => {
            home.join("Library/Application Support/Claude/claude_desktop_config.json")
        }
        Harness::Cursor => match options.scope {
            Scope::Project => options.cwd.join(".cursor/mcp.json"),
            Scope::User => home.join(".cursor/mcp.json"),
            Scope::Local => return unsupported(harness, "local file scope"),
        },
        Harness::OpenCode => match options.scope {
            Scope::Project => options.cwd.join("opencode.json"),
            Scope::User => home.join(".config/opencode/opencode.json"),
            Scope::Local => return unsupported(harness, "local file scope"),
        },
        Harness::Windsurf => home.join(".codeium/windsurf/mcp_config.json"),
        Harness::Zed => home.join(".config/zed/settings.json"),
        Harness::OpenClaw => match options.scope {
            Scope::Project => options.cwd.join("openclaw.json"),
            Scope::User => home.join(".openclaw/openclaw.json"),
            Scope::Local => return unsupported(harness, "local file scope"),
        },
        Harness::CopilotCli => home.join(".copilot/mcp-config.json"),
        Harness::AntigravityCli => match options.scope {
            Scope::Project => options.cwd.join(".agents/mcp_config.json"),
            Scope::User => home.join(".gemini/config/mcp_config.json"),
            Scope::Local => return unsupported(harness, "local file scope"),
        },
        Harness::AntigravityDesktop => home.join(".gemini/antigravity/mcp_config.json"),
        _ => return unsupported(harness, "file registration"),
    };
    Ok(path)
}

fn new_harness_target_path(
    harness: Harness,
    options: &RegistrationOptions,
) -> Result<PathBuf, Error> {
    if harness == Harness::Jcode && options.scope != Scope::User {
        return unsupported(harness, "project or local MCP registration");
    }
    if options.scope == Scope::Local {
        return unsupported(harness, "local file scope");
    }
    if let Some(path) = &options.config {
        return Ok(path.clone());
    }
    if options.scope == Scope::Project {
        return Ok(options.cwd.join(".zcode/config.json"));
    }
    let home = env::home_dir().ok_or(Error::MissingArgument("home directory"))?;
    Ok(match harness {
        Harness::Jcode => home.join(".jcode/config.json"),
        Harness::Zcode => home.join(".zcode/cli/config.json"),
        _ => unreachable!("new harness target only"),
    })
}

fn key_path(harness: Harness) -> &'static [&'static str] {
    match harness {
        Harness::OpenCode => &["mcp"],
        Harness::OpenClaw | Harness::Zcode => &["mcp", "servers"],
        Harness::Zed => &["context_servers"],
        Harness::Jcode => &["mcp_servers"],
        _ => &["mcpServers"],
    }
}

fn standard_entry(spec: &ServerSpec) -> Result<Value, Error> {
    spec.validate()?;
    Ok(standard_entry_unchecked(spec))
}

fn standard_entry_unchecked(spec: &ServerSpec) -> Value {
    if spec.transport == Transport::Stdio {
        let mut entry = Map::new();
        entry.insert(
            "command".to_owned(),
            json!(spec.command.as_deref().unwrap_or_default()),
        );
        entry.insert("args".to_owned(), json!(spec.args));
        if !spec.env.is_empty() {
            entry.insert("env".to_owned(), json!(spec.env));
        }
        Value::Object(entry)
    } else {
        let mut entry = Map::new();
        entry.insert("type".to_owned(), json!(transport_name(spec.transport)));
        entry.insert("url".to_owned(), json!(spec.url));
        if !spec.headers.is_empty() {
            entry.insert("headers".to_owned(), json!(spec.headers));
        }
        Value::Object(entry)
    }
}

fn open_code_entry(spec: &ServerSpec) -> Result<Value, Error> {
    spec.validate()?;
    if spec.transport == Transport::Stdio {
        let mut command = vec![spec.command.clone().unwrap_or_default()];
        command.extend(spec.args.iter().cloned());
        let mut entry = json!({"type":"local","command":command,"enabled":true});
        if !spec.env.is_empty() {
            entry["environment"] = json!(spec.env);
        }
        Ok(entry)
    } else {
        let mut entry = json!({"type":"remote","url":spec.url,"enabled":true});
        if !spec.headers.is_empty() {
            entry["headers"] = json!(spec.headers);
        }
        Ok(entry)
    }
}

fn open_claw_entry(spec: &ServerSpec) -> Result<Value, Error> {
    spec.validate()?;
    if spec.transport == Transport::Stdio {
        let mut entry = json!({"command":spec.command,"args":spec.args,"transport":"stdio"});
        if !spec.env.is_empty() {
            entry["env"] = json!(spec.env);
        }
        Ok(entry)
    } else {
        let mut entry = json!({"url":spec.url,"transport":transport_name(spec.transport)});
        if !spec.headers.is_empty() {
            entry["headers"] = json!(spec.headers);
        }
        Ok(entry)
    }
}

fn zed_entry(spec: &ServerSpec) -> Result<Value, Error> {
    if spec.transport != Transport::Stdio {
        return unsupported(Harness::Zed, "remote transport registration");
    }
    let mut entry = json!({"source":"custom","command":spec.command,"args":spec.args});
    if !spec.env.is_empty() {
        entry["env"] = json!(spec.env);
    }
    Ok(entry)
}

fn copilot_entry(spec: &ServerSpec) -> Result<Value, Error> {
    if spec.transport != Transport::Stdio {
        return unsupported(Harness::CopilotCli, "remote transport registration");
    }
    let mut entry = json!({"type":"stdio","command":spec.command,"args":spec.args});
    if !spec.env.is_empty() {
        entry["env"] = json!(spec.env);
    }
    Ok(entry)
}

fn typed_standard_entry(harness: Harness, spec: &ServerSpec) -> Result<Value, Error> {
    if spec.transport == Transport::Ws {
        return unsupported(harness, "ws transport registration");
    }
    let mut entry = standard_entry(spec)?;
    if spec.transport == Transport::Stdio {
        entry["type"] = json!("stdio");
    }
    Ok(entry)
}

fn transport_name(transport: Transport) -> &'static str {
    match transport {
        Transport::Stdio => "stdio",
        Transport::Http => "http",
        Transport::Sse => "sse",
        Transport::Ws => "ws",
    }
}

fn unsupported<T>(harness: Harness, detail: &str) -> Result<T, Error> {
    Err(Error::Unsupported {
        harness: harness.id().to_owned(),
        detail: detail.to_owned(),
    })
}

#[derive(Debug)]
struct Step {
    program: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    cwd: Option<PathBuf>,
    redacted: Vec<bool>,
    ignore_failure: bool,
}

fn delegated(
    harness: Harness,
    spec: &ServerSpec,
    options: &RegistrationOptions,
) -> Result<RegistrationResult, Error> {
    let steps = delegated_steps(harness, spec, options)?;
    if options.print || options.dry_run {
        for step in &steps {
            println!("$ {}", preview(&step.program, &step.args, &step.redacted));
        }
    }
    if options.dry_run {
        return Ok(result(harness, spec, None, false, "dry-run"));
    }
    for step in steps {
        let mut command = Command::new(&step.program);
        command.args(&step.args).stdin(Stdio::null());
        if let Some(cwd) = &step.cwd {
            command.current_dir(cwd);
        }
        for (key, value) in &step.env {
            command.env(key, value);
        }
        let status = command.status().map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                Error::ProgramMissing {
                    program: step.program.clone(),
                }
            } else {
                Error::ProgramIo {
                    program: step.program.clone(),
                    source,
                }
            }
        })?;
        if !status.success() && !step.ignore_failure {
            return Err(Error::ProgramFailed {
                program: step.program,
                status: status
                    .code()
                    .map_or_else(|| "signal".to_owned(), |code| code.to_string()),
            });
        }
    }
    Ok(result(harness, spec, None, true, "registered"))
}

#[allow(clippy::too_many_lines)]
fn delegated_steps(
    harness: Harness,
    spec: &ServerSpec,
    options: &RegistrationOptions,
) -> Result<Vec<Step>, Error> {
    if harness == Harness::ArsyCode {
        return arsy_steps(spec, options);
    }
    if spec.transport != Transport::Stdio {
        return unsupported(harness, "remote delegated registration");
    }
    let command = spec.command.clone().unwrap_or_default();
    let (program, mut args) = match harness {
        Harness::ClaudeCodeCli => (
            "claude",
            vec![
                "mcp".into(),
                "add".into(),
                "--transport".into(),
                "stdio".into(),
                "--scope".into(),
                scope_name(options.scope).into(),
                spec.name.clone(),
                "--".into(),
                command,
            ],
        ),
        Harness::Codex => (
            "codex",
            vec![
                "mcp".into(),
                "add".into(),
                spec.name.clone(),
                "--".into(),
                command,
            ],
        ),
        Harness::GeminiCli => (
            "gemini",
            vec![
                "mcp".into(),
                "add".into(),
                "-s".into(),
                if options.scope == Scope::Project {
                    "project".into()
                } else {
                    "user".into()
                },
                spec.name.clone(),
                command,
            ],
        ),
        Harness::Hermes => (
            "hermes",
            vec![
                "mcp".into(),
                "add".into(),
                spec.name.clone(),
                "--command".into(),
                command,
            ],
        ),
        Harness::Vscode => {
            let entry = copilot_entry(spec)?;
            (
                "code",
                vec![
                    "--add-mcp".into(),
                    serde_json::to_string(&json!({
                        "name": spec.name,
                        "type": "stdio",
                        "command": entry["command"],
                        "args": entry["args"],
                        "env": entry["env"]
                    }))
                    .map_err(|e| Error::Update(e.to_string()))?,
                ],
            )
        }
        _ => return unsupported(harness, "delegated registration"),
    };
    args.extend(spec.args.iter().cloned());
    let mut redacted = vec![false; args.len()];
    let mut envs = BTreeMap::new();
    for (key, value) in &spec.env {
        if program == "codex" {
            args.splice(3..3, ["--env".into(), format!("{key}={value}")]);
            redacted.splice(3..3, [false, true]);
        } else {
            envs.insert(key.clone(), value.clone());
        }
    }
    let mut steps = Vec::new();
    if options.force {
        steps.push(Step {
            program: program.to_owned(),
            args: vec!["mcp".into(), "remove".into(), spec.name.clone()],
            env: BTreeMap::new(),
            cwd: Some(options.cwd.clone()),
            redacted: vec![false; 3],
            ignore_failure: true,
        });
    }
    steps.push(Step {
        program: program.to_owned(),
        args,
        env: envs,
        cwd: Some(options.cwd.clone()),
        redacted,
        ignore_failure: false,
    });
    Ok(steps)
}

fn arsy_steps(spec: &ServerSpec, options: &RegistrationOptions) -> Result<Vec<Step>, Error> {
    if !matches!(spec.transport, Transport::Stdio | Transport::Http) {
        return unsupported(Harness::ArsyCode, "sse or ws transport");
    }
    let scope = match options.scope {
        Scope::User => "user",
        Scope::Project => "workspace",
        Scope::Local => return unsupported(Harness::ArsyCode, "local file scope"),
    };
    // ARSY stores `${NAME}` references only, and only in the user file: the
    // secret stays in the environment, and a repository file may not read it.
    let (flag, references) = match spec.transport {
        Transport::Stdio => ("--env", &spec.env),
        _ => ("--header", &spec.headers),
    };
    if !references.is_empty() && scope != "user" {
        return unsupported(
            Harness::ArsyCode,
            "environment variables or headers outside the user scope",
        );
    }
    if let Some(key) = references
        .iter()
        .find(|(_, value)| !value.contains("${"))
        .map(|(key, _)| key)
    {
        return unsupported(
            Harness::ArsyCode,
            &format!(
                "a literal value for `{key}`; reference an environment variable as ${{{key}}}"
            ),
        );
    }
    let mut args = vec![
        "mcp".into(),
        "add".into(),
        spec.name.clone(),
        "--scope".into(),
        scope.into(),
    ];
    match spec.transport {
        Transport::Stdio => {
            args.extend(["--command".into(), spec.command.clone().unwrap_or_default()]);
        }
        Transport::Http => {
            args.extend([
                "--transport".into(),
                "http".into(),
                "--url".into(),
                spec.url.clone().unwrap_or_default(),
            ]);
        }
        Transport::Sse | Transport::Ws => unreachable!("validated above"),
    }
    for (key, value) in references {
        args.extend([flag.into(), format!("{key}={value}")]);
    }
    args.extend(spec.args.iter().cloned());
    let mut steps = Vec::new();
    if options.force {
        steps.push(Step {
            program: "arsy".into(),
            args: vec![
                "mcp".into(),
                "remove".into(),
                spec.name.clone(),
                "--scope".into(),
                scope.into(),
            ],
            env: BTreeMap::new(),
            cwd: Some(options.cwd.clone()),
            redacted: vec![false; 5],
            ignore_failure: true,
        });
    }
    steps.push(Step {
        program: "arsy".into(),
        redacted: vec![false; args.len()],
        args,
        env: BTreeMap::new(),
        cwd: Some(options.cwd.clone()),
        ignore_failure: false,
    });
    Ok(steps)
}

fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::User => "user",
        Scope::Project => "project",
        Scope::Local => "local",
    }
}

fn redacted_snippet(harness: Harness, spec: &ServerSpec) -> Value {
    let entry = entry_for(harness, spec).unwrap_or_else(|_| standard_entry_unchecked(spec));
    match harness {
        Harness::OpenCode => json!({"mcp":{spec.name.clone():redact(&entry)}}),
        Harness::OpenClaw | Harness::Zcode => {
            json!({"mcp":{"servers":{spec.name.clone():redact(&entry)}}})
        }
        Harness::Zed => json!({"context_servers":{spec.name.clone():redact(&entry)}}),
        Harness::Jcode => json!({"mcp_servers":{spec.name.clone():redact(&entry)}}),
        _ => json!({"mcpServers":{spec.name.clone():redact(&entry)}}),
    }
}

fn redact(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        if matches!(key.as_str(), "env" | "environment" | "headers") {
                            value.as_object().map_or_else(
                                || json!("<redacted>"),
                                |object| {
                                    Value::Object(
                                        object
                                            .keys()
                                            .map(|key| (key.clone(), json!("<redacted>")))
                                            .collect(),
                                    )
                                },
                            )
                        } else {
                            redact(value)
                        },
                    )
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact).collect()),
        _ => value.clone(),
    }
}

fn preview(program: &str, args: &[String], redacted: &[bool]) -> String {
    std::iter::once(program.to_owned())
        .chain(args.iter().enumerate().map(|(index, arg)| {
            if redacted.get(index).copied().unwrap_or(false) {
                "<redacted>".to_owned()
            } else {
                arg.clone()
            }
        }))
        .collect::<Vec<_>>()
        .join(" ")
}

fn doctor_one(harness: Harness, options: &RegistrationOptions) -> Result<DoctorReport, Error> {
    if harness == Harness::Jcode && options.scope != Scope::User {
        return Ok(DoctorReport {
            harness: harness.id().into(),
            kind: "file",
            target: None,
            available: false,
            detail: "project or local MCP registration is unsupported".into(),
        });
    }
    if harness.is_snippet_only() {
        return Ok(DoctorReport {
            harness: harness.id().into(),
            kind: "snippet",
            target: None,
            available: true,
            detail: "portable snippet available".into(),
        });
    }
    if let Some(program) = delegated_program(harness) {
        let available = Command::new(program)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        return Ok(DoctorReport {
            harness: harness.id().into(),
            kind: "delegated",
            target: None,
            available,
            detail: if available {
                format!("{program} CLI found")
            } else {
                format!("{program} CLI not found")
            },
        });
    }
    let path = resolved_target_path(harness, options)?;
    let exists = path.exists();
    let detail = if exists {
        jsonc::load_object(&path).map_or_else(
            |error| error.to_string(),
            |_| "configuration exists and is valid".into(),
        )
    } else {
        "configuration will be created".into()
    };
    Ok(DoctorReport {
        harness: harness.id().into(),
        kind: "file",
        target: Some(path),
        available: !exists || detail == "configuration exists and is valid",
        detail,
    })
}

const fn delegated_program(harness: Harness) -> Option<&'static str> {
    match harness {
        Harness::ArsyCode => Some("arsy"),
        Harness::ClaudeCodeCli => Some("claude"),
        Harness::Codex => Some("codex"),
        Harness::GeminiCli => Some("gemini"),
        Harness::Hermes => Some("hermes"),
        Harness::Vscode => Some("code"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    use tempfile::tempdir;

    fn opts(root: &Path) -> RegistrationOptions {
        RegistrationOptions {
            scope: Scope::Project,
            cwd: root.to_path_buf(),
            ..Default::default()
        }
    }

    #[test]
    fn registration_preserves_other_servers() {
        let dir = tempdir().expect("temp");
        let path = dir.path().join("mcp.json");
        fs::write(&path, r#"{"mcpServers":{"other":{"command":"other"}}}"#).expect("seed");
        let mut options = opts(dir.path());
        options.config = Some(path.clone());
        register(
            Harness::ClaudeCode,
            &ServerSpec::stdio("demo", "demo", vec!["mcp".into()]),
            &options,
        )
        .expect("register");
        let output: Value =
            serde_json::from_str(&fs::read_to_string(path).expect("output")).expect("json");
        assert!(output["mcpServers"]["other"].is_object());
        assert_eq!(output["mcpServers"]["demo"]["command"], "demo");
    }

    #[test]
    fn opencode_uses_local_entry() {
        let spec = ServerSpec::stdio("demo", "demo", vec!["mcp".into()]);
        assert_eq!(
            entry_for(Harness::OpenCode, &spec).expect("entry")["type"],
            "local"
        );
    }

    #[test]
    fn arsy_code_delegates_stdio_with_workspace_scope() {
        let mut options = opts(Path::new("/workspace"));
        options.scope = Scope::Project;
        let steps = delegated_steps(
            Harness::ArsyCode,
            &ServerSpec::stdio("demo", "demo", vec!["--serve".into()]),
            &options,
        )
        .expect("steps");

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].program, "arsy");
        assert_eq!(
            steps[0].args,
            vec![
                String::from("mcp"),
                String::from("add"),
                String::from("demo"),
                String::from("--scope"),
                String::from("workspace"),
                String::from("--command"),
                String::from("demo"),
                String::from("--serve"),
            ]
        );
    }

    #[test]
    fn arsy_code_delegates_http_and_rejects_unrepresentable_fields() {
        let options = opts(Path::new("/workspace"));
        let http = ServerSpec {
            name: "remote".into(),
            transport: Transport::Http,
            url: Some("https://example.test/mcp".into()),
            ..Default::default()
        };
        let steps = delegated_steps(Harness::ArsyCode, &http, &options).expect("steps");
        assert_eq!(
            steps[0].args,
            vec![
                String::from("mcp"),
                String::from("add"),
                String::from("remote"),
                String::from("--scope"),
                String::from("workspace"),
                String::from("--transport"),
                String::from("http"),
                String::from("--url"),
                String::from("https://example.test/mcp"),
            ]
        );

        let mut with_env = ServerSpec::stdio("demo", "demo", Vec::new());
        with_env.env.insert("TOKEN".into(), "secret".into());
        assert!(matches!(
            delegated_steps(Harness::ArsyCode, &with_env, &options),
            Err(Error::Unsupported { .. })
        ));
    }

    #[test]
    fn arsy_code_delegates_variable_references_at_user_scope() {
        let mut options = opts(Path::new("/workspace"));
        options.scope = Scope::User;
        let mut stdio = ServerSpec::stdio("demo", "demo", vec!["--serve".into()]);
        stdio.env.insert("TOKEN".into(), "${DEMO_TOKEN}".into());
        let steps = delegated_steps(Harness::ArsyCode, &stdio, &options).expect("steps");
        assert_eq!(
            steps[0].args,
            vec![
                String::from("mcp"),
                String::from("add"),
                String::from("demo"),
                String::from("--scope"),
                String::from("user"),
                String::from("--command"),
                String::from("demo"),
                String::from("--env"),
                String::from("TOKEN=${DEMO_TOKEN}"),
                String::from("--serve"),
            ]
        );

        let mut http = ServerSpec {
            name: "remote".into(),
            transport: Transport::Http,
            url: Some("https://example.test/mcp".into()),
            ..Default::default()
        };
        http.headers
            .insert("Authorization".into(), "Bearer ${API_TOKEN}".into());
        let steps = delegated_steps(Harness::ArsyCode, &http, &options).expect("steps");
        assert_eq!(
            &steps[0].args[steps[0].args.len() - 2..],
            [
                String::from("--header"),
                String::from("Authorization=Bearer ${API_TOKEN}"),
            ]
        );

        options.scope = Scope::Project;
        assert!(matches!(
            delegated_steps(Harness::ArsyCode, &stdio, &options),
            Err(Error::Unsupported { .. })
        ));
    }

    #[test]
    fn dry_run_does_not_write() {
        let dir = tempdir().expect("temp");
        let path = dir.path().join("mcp.json");
        let mut options = opts(dir.path());
        options.config = Some(path.clone());
        options.dry_run = true;
        register(
            Harness::ClaudeCode,
            &ServerSpec::stdio("demo", "demo", Vec::new()),
            &options,
        )
        .expect("dry run");
        assert!(!path.exists());
    }

    #[test]
    fn conflict_requires_force() {
        let dir = tempdir().expect("temp");
        let path = dir.path().join("mcp.json");
        fs::write(&path, r#"{"mcpServers":{"demo":{"command":"old"}}}"#).expect("seed");
        let mut options = opts(dir.path());
        options.config = Some(path);
        let error = register(
            Harness::ClaudeCode,
            &ServerSpec::stdio("demo", "new", Vec::new()),
            &options,
        )
        .expect_err("conflict");
        assert!(matches!(error, Error::Conflict { .. }));
    }

    #[test]
    fn jcode_merges_user_config_and_rejects_project_scope() {
        let dir = tempdir().expect("temp");
        let path = dir.path().join("config.json");
        fs::write(
            &path,
            r#"{"model":"example","mcp_servers":{"other":{"type":"stdio","command":"other"}}}"#,
        )
        .expect("seed");
        let mut options = opts(dir.path());
        options.scope = Scope::User;
        options.config = Some(path.clone());
        let spec = ServerSpec::stdio("demo", "demo", vec!["mcp".into()]);
        assert!(
            register(Harness::Jcode, &spec, &options)
                .expect("register")
                .changed
        );
        let config: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(config["model"], "example");
        assert_eq!(config["mcp_servers"]["other"]["command"], "other");
        assert_eq!(config["mcp_servers"]["demo"]["type"], "stdio");
        assert!(
            !register(Harness::Jcode, &spec, &options)
                .expect("again")
                .changed
        );
        options.scope = Scope::Project;
        assert!(matches!(
            register(Harness::Jcode, &spec, &options),
            Err(Error::Unsupported { .. })
        ));
    }

    #[test]
    fn zcode_merges_native_config_and_rejects_project_credentials() {
        let dir = tempdir().expect("temp");
        let mut options = opts(dir.path());
        let spec = ServerSpec::stdio("demo", "demo", vec![]);
        let path = dir.path().join(".zcode/config.json");
        assert!(
            register(Harness::Zcode, &spec, &options)
                .expect("register")
                .changed
        );
        let config: Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        assert_eq!(config["mcp"]["servers"]["demo"]["type"], "stdio");
        assert!(
            !register(Harness::Zcode, &spec, &options)
                .expect("again")
                .changed
        );
        options.dry_run = true;
        let mut secret = spec.clone();
        secret.env.insert("TOKEN".into(), "secret".into());
        assert!(matches!(
            register(Harness::Zcode, &secret, &options),
            Err(Error::Unsupported { .. })
        ));
    }

    #[test]
    fn new_harness_dry_runs_do_not_write_and_reject_ws() {
        let dir = tempdir().expect("temp");
        for harness in [Harness::Jcode, Harness::Zcode] {
            let path = dir.path().join(format!("{harness}.json"));
            let options = RegistrationOptions {
                scope: Scope::User,
                config: Some(path.clone()),
                dry_run: true,
                ..opts(dir.path())
            };
            let spec = ServerSpec::stdio("demo", "demo", vec![]);
            assert_eq!(
                register(harness, &spec, &options).expect("dry run").action,
                "dry-run"
            );
            assert!(!path.exists());
            let ws = ServerSpec {
                name: "remote".into(),
                transport: Transport::Ws,
                url: Some("wss://example.test/mcp".into()),
                ..Default::default()
            };
            assert!(matches!(
                entry_for(harness, &ws),
                Err(Error::Unsupported { .. })
            ));
        }
    }
}
