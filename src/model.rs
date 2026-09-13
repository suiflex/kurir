use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    #[default]
    Stdio,
    Http,
    Sse,
    Ws,
}

impl Transport {
    /// Parse a transport identifier accepted by the CLI and spec format.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported transport identifier.
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "stdio" => Ok(Self::Stdio),
            "http" | "streamable-http" => Ok(Self::Http),
            "sse" => Ok(Self::Sse),
            "ws" | "websocket" => Ok(Self::Ws),
            _ => Err(Error::InvalidArgument {
                field: "transport",
                value: value.to_owned(),
                expected: "stdio, http, sse, or ws",
            }),
        }
    }

    #[must_use]
    pub fn is_remote(self) -> bool {
        !matches!(self, Self::Stdio)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServerSpec {
    pub name: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub transport: Transport,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}

impl ServerSpec {
    /// Validate the server name and transport-specific fields.
    ///
    /// # Errors
    ///
    /// Returns an error when required command or URL fields are missing or mixed.
    pub fn validate(&self) -> Result<(), Error> {
        if self.name.trim().is_empty() {
            return Err(Error::InvalidArgument {
                field: "name",
                value: self.name.clone(),
                expected: "a non-empty server name",
            });
        }
        if self.name.chars().any(char::is_whitespace) {
            return Err(Error::InvalidArgument {
                field: "name",
                value: self.name.clone(),
                expected: "a name without whitespace",
            });
        }
        if self.transport.is_remote() {
            if self.url.as_deref().is_none_or(str::is_empty) {
                return Err(Error::MissingArgument("url for a remote transport"));
            }
            if self.command.is_some() || !self.args.is_empty() || !self.env.is_empty() {
                return Err(Error::InvalidArgument {
                    field: "server",
                    value: self.name.clone(),
                    expected: "remote transports use url and headers, not command or env",
                });
            }
        } else if self.command.as_deref().is_none_or(str::is_empty) {
            return Err(Error::MissingArgument("command for stdio transport"));
        }
        Ok(())
    }

    #[must_use]
    pub fn stdio(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            name: name.into(),
            command: Some(command.into()),
            args,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Scope {
    #[default]
    User,
    Project,
    Local,
}

impl Scope {
    /// Parse a registration scope.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported scope identifier.
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "user" | "global" => Ok(Self::User),
            "project" => Ok(Self::Project),
            "local" => Ok(Self::Local),
            _ => Err(Error::InvalidArgument {
                field: "scope",
                value: value.to_owned(),
                expected: "user, project, or local",
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RegistrationOptions {
    pub scope: Scope,
    pub config: Option<PathBuf>,
    pub cwd: PathBuf,
    pub force: bool,
    pub dry_run: bool,
    pub print: bool,
}

impl Default for RegistrationOptions {
    fn default() -> Self {
        Self {
            scope: Scope::User,
            config: None,
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            force: false,
            dry_run: false,
            print: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistrationResult {
    pub harness: String,
    pub name: String,
    pub target: Option<PathBuf>,
    pub changed: bool,
    pub action: String,
}
