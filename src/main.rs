use std::{collections::BTreeMap, fs, path::PathBuf, process::ExitCode};

use clap::{Args, Parser, Subcommand};
use kurir::{
    Harness, HookSpec, RegistrationOptions, Scope, ServerSpec, Transport, install_skill,
    register_hook,
    registration::{entry_for, register, snippet_for},
    update,
};
#[derive(Debug, Parser)]
#[command(
    name = "kurir",
    version,
    about = "Portable MCP server registration and harness integration toolkit"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Register one MCP server with a harness.
    Register(Box<RegisterArgs>),
    /// Register one lifecycle hook with a harness.
    Hook(Box<HookArgs>),
    /// Install a skill into a harness skills directory.
    Skill(Box<SkillArgs>),
    /// Inspect harness configuration targets and delegated CLIs.
    Doctor(DoctorArgs),
    /// List supported harness adapters.
    Clients,
    /// Check for and install the latest Kurir release.
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
struct HookArgs {
    /// Harness adapter to use. `--harness` is an alias for `--client`.
    #[arg(long, alias = "harness")]
    client: String,
    /// Lifecycle event name (e.g. Stop, `PreToolUse`).
    #[arg(long)]
    event: String,
    /// Command to execute when the lifecycle event fires.
    #[arg(long)]
    command: String,
    /// Optional matcher for tool or event source.
    #[arg(long)]
    matcher: Option<String>,
    /// Command timeout in seconds.
    #[arg(long, default_value = "600")]
    timeout: u64,
    /// Registration scope.
    #[arg(long, default_value = "user")]
    scope: String,
    /// Override the harness hook configuration path.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Working directory used for project targets.
    #[arg(long, default_value = ".")]
    cwd: PathBuf,
    /// Show the hook entry without writing.
    #[arg(long)]
    print: bool,
    /// Validate and preview without writing to file.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct SkillArgs {
    /// Harness adapter to use. `--harness` is an alias for `--client`.
    #[arg(long, alias = "harness")]
    client: String,
    /// Path to the skill directory containing the skill to install.
    #[arg(long = "path")]
    path: PathBuf,
    /// Target registration scope.
    #[arg(long, default_value = "user")]
    scope: String,
    /// Override the harness skills directory path.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Working directory used for project targets.
    #[arg(long, default_value = ".")]
    cwd: PathBuf,
    /// Replace conflicting skill installation.
    #[arg(long)]
    force: bool,
    /// Preview installation path without copying.
    #[arg(long)]
    print: bool,
    /// Validate and preview without writing files.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct UpdateArgs {
    /// Only check whether a newer release exists.
    #[arg(long)]
    check: bool,
    /// Print a machine-readable JSON report.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RegisterArgs {
    /// Harness adapter to use. `--harness` is an alias for `--client`.
    #[arg(long, alias = "harness")]
    client: String,
    /// Name exposed by the harness; required unless `--spec` is provided.
    #[arg(long)]
    name: Option<String>,
    /// Local stdio executable or command.
    #[arg(long)]
    command: Option<String>,
    /// Argument passed to a local stdio command; repeat for multiple arguments.
    #[arg(long = "arg")]
    args: Vec<String>,
    /// Environment assignment; repeat for multiple values.
    #[arg(long = "env")]
    env: Vec<String>,
    /// MCP transport: stdio, http, sse, or ws.
    #[arg(long, default_value = "stdio")]
    transport: String,
    /// Remote MCP URL for http, sse, or ws transports.
    #[arg(long)]
    url: Option<String>,
    /// Header assignment for a remote transport; repeat for multiple values.
    #[arg(long = "header")]
    headers: Vec<String>,
    /// Registration scope.
    #[arg(long, default_value = "user")]
    scope: String,
    /// Override the harness configuration path.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Working directory used for project scope and delegated CLIs.
    #[arg(long, default_value = ".")]
    cwd: PathBuf,
    /// Replace a conflicting entry.
    #[arg(long)]
    force: bool,
    /// Show a redacted entry or delegated command without writing.
    #[arg(long)]
    print: bool,
    /// Validate and preview without writing or invoking a harness CLI.
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    spec: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct DoctorArgs {
    /// Inspect one harness instead of the complete registry.
    #[arg(long)]
    client: Option<String>,
    /// Scope used to resolve file targets.
    #[arg(long, default_value = "user")]
    scope: String,
    /// Override the harness configuration path.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Working directory used for project targets.
    #[arg(long, default_value = ".")]
    cwd: PathBuf,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("kurir: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), kurir::Error> {
    match Cli::parse().command {
        Command::Register(args) => register_command(*args),
        Command::Hook(args) => hook_command(*args),
        Command::Skill(args) => skill_command(*args),
        Command::Doctor(args) => doctor_command(args),
        Command::Clients => {
            for harness in Harness::ALL {
                println!("{:<22} {}", harness.id(), harness.summary());
            }
            Ok(())
        }
        Command::Update(args) => update(args.check, args.json),
    }
}

fn hook_command(args: HookArgs) -> Result<(), kurir::Error> {
    let harness = args.client.parse::<Harness>()?;
    let spec = HookSpec {
        event: args.event,
        matcher: args.matcher,
        command: args.command,
        timeout_seconds: args.timeout,
    };
    let options = RegistrationOptions {
        scope: Scope::parse(&args.scope)?,
        config: args.config,
        cwd: absolute_path(args.cwd)?,
        force: false,
        dry_run: args.dry_run,
        print: args.print,
    };
    let result = register_hook(harness, &spec, &options)?;
    if !args.print && !args.dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "harness": result.harness,
                "name": result.name,
                "target": result.target,
                "changed": result.changed,
                "action": result.action,
            }))
            .map_err(|source| kurir::Error::InvalidJson {
                path: "stdout".to_owned(),
                source,
            })?
        );
    }
    Ok(())
}

fn skill_command(args: SkillArgs) -> Result<(), kurir::Error> {
    let harness = args.client.parse::<Harness>()?;
    let options = RegistrationOptions {
        scope: Scope::parse(&args.scope)?,
        config: args.config,
        cwd: absolute_path(args.cwd)?,
        force: args.force,
        dry_run: args.dry_run,
        print: args.print,
    };
    let result = install_skill(harness, &args.path, &options)?;
    if !args.print && !args.dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "harness": result.harness,
                "name": result.name,
                "target": result.target,
                "changed": result.changed,
                "action": result.action,
            }))
            .map_err(|source| kurir::Error::InvalidJson {
                path: "stdout".to_owned(),
                source,
            })?
        );
    }
    Ok(())
}

fn register_command(args: RegisterArgs) -> Result<(), kurir::Error> {
    let harness = args.client.parse::<Harness>()?;
    let spec = if let Some(path) = args.spec {
        let raw = fs::read_to_string(&path).map_err(|source| kurir::Error::Read {
            path: path.display().to_string(),
            source,
        })?;
        serde_json::from_str::<ServerSpec>(&raw).map_err(|source| kurir::Error::InvalidSpec {
            path: path.display().to_string(),
            source,
        })?
    } else {
        ServerSpec {
            name: args
                .name
                .ok_or(kurir::Error::MissingArgument("name unless --spec is used"))?,
            command: args.command,
            args: args.args,
            env: parse_assignments(args.env, false)?,
            transport: Transport::parse(&args.transport)?,
            url: args.url,
            headers: parse_assignments(args.headers, true)?,
        }
    };
    let options = RegistrationOptions {
        scope: Scope::parse(&args.scope)?,
        config: args.config,
        cwd: absolute_path(args.cwd)?,
        force: args.force,
        dry_run: args.dry_run,
        print: args.print,
    };
    let result = register(harness, &spec, &options)?;
    if harness.is_snippet_only() {
        println!(
            "{}",
            serde_json::to_string_pretty(&snippet_for(&spec)).map_err(|source| {
                kurir::Error::InvalidJson {
                    path: "stdout".to_owned(),
                    source,
                }
            })?
        );
    } else if !args.print && !args.dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "harness": result.harness,
                "name": result.name,
                "target": result.target,
                "changed": result.changed,
                "action": result.action,
            }))
            .map_err(|source| kurir::Error::InvalidJson {
                path: "stdout".to_owned(),
                source,
            })?
        );
    }
    Ok(())
}

fn doctor_command(args: DoctorArgs) -> Result<(), kurir::Error> {
    let harness = args
        .client
        .as_deref()
        .map(str::parse::<Harness>)
        .transpose()?;
    let options = RegistrationOptions {
        scope: Scope::parse(&args.scope)?,
        config: args.config,
        cwd: absolute_path(args.cwd)?,
        ..RegistrationOptions::default()
    };
    for report in kurir::doctor(harness, &options)? {
        println!(
            "{}",
            serde_json::to_string(&report).map_err(|source| kurir::Error::InvalidJson {
                path: "stdout".to_owned(),
                source,
            })?
        );
    }
    Ok(())
}

fn parse_assignments(
    assignments: Vec<String>,
    header: bool,
) -> Result<BTreeMap<String, String>, kurir::Error> {
    let mut parsed = BTreeMap::new();
    for assignment in assignments {
        let Some((key, value)) = assignment.split_once('=') else {
            return if header {
                Err(kurir::Error::InvalidHeader(assignment))
            } else {
                Err(kurir::Error::InvalidAssignment(assignment))
            };
        };
        if key.is_empty() {
            return if header {
                Err(kurir::Error::InvalidHeader(assignment))
            } else {
                Err(kurir::Error::InvalidAssignment(assignment))
            };
        }
        parsed.insert(key.to_owned(), value.to_owned());
    }
    Ok(parsed)
}

fn absolute_path(path: PathBuf) -> Result<PathBuf, kurir::Error> {
    if path.is_absolute() {
        return Ok(path);
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|source| kurir::Error::Read {
            path: ".".to_owned(),
            source,
        })
}

#[allow(dead_code)]
fn _entry_for_cli_smoke(
    harness: Harness,
    spec: &ServerSpec,
) -> Result<serde_json::Value, kurir::Error> {
    entry_for(harness, spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_args_can_be_parsed() {
        let cli = Cli::try_parse_from([
            "kurir",
            "hook",
            "--client",
            "cursor",
            "--event",
            "PreToolUse",
            "--command",
            "echo 1",
        ])
        .expect("parse hook");
        assert!(matches!(cli.command, Command::Hook(_)));
    }

    #[test]
    fn skill_args_can_be_parsed() {
        let cli = Cli::try_parse_from([
            "kurir",
            "skill",
            "--client",
            "cursor",
            "--path",
            "/path/to/skill",
        ])
        .expect("parse skill");
        assert!(matches!(cli.command, Command::Skill(_)));
    }
}
