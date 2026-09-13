use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid {field} '{value}'; expected {expected}")]
    InvalidArgument {
        field: &'static str,
        value: String,
        expected: &'static str,
    },
    #[error("missing required argument: {0}")]
    MissingArgument(&'static str),
    #[error("configuration file {path} must contain a JSON object")]
    ConfigRootNotObject { path: String },
    #[error("unknown harness '{0}'")]
    UnknownHarness(String),
    #[error("harness '{harness}' does not support {detail}")]
    Unsupported { harness: String, detail: String },
    #[error("configuration file {path} is not valid JSON/JSONC: {source}")]
    InvalidJson {
        path: String,
        source: serde_json::Error,
    },
    #[error("configuration field '{field}' in {path} must be a JSON object")]
    InvalidConfigField { path: String, field: String },
    #[error(
        "existing MCP entry '{name}' conflicts with the requested entry; pass --force to replace it"
    )]
    Conflict { name: String },
    #[error("failed to read {path}: {source}")]
    Read { path: String, source: io::Error },
    #[error("failed to write {path}: {source}")]
    Write { path: String, source: io::Error },
    #[error("failed to back up {path}: {source}")]
    Backup { path: String, source: io::Error },
    #[error("{program} is not installed or not executable")]
    ProgramMissing { program: String },
    #[error("{program} exited with status {status}")]
    ProgramFailed { program: String, status: String },
    #[error("failed to run {program}: {source}")]
    ProgramIo { program: String, source: io::Error },
    #[error("failed to parse environment assignment '{0}'; expected KEY=VALUE")]
    InvalidAssignment(String),
    #[error("failed to parse header assignment '{0}'; expected NAME=VALUE")]
    InvalidHeader(String),
    #[error("failed to parse spec {path}: {source}")]
    InvalidSpec {
        path: String,
        source: serde_json::Error,
    },
    #[error("update failed: {0}")]
    Update(String),
}
