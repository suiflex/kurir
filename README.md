<p align="center">
  <img src="./assets/brand/logo.svg" alt="Kurir logo" width="280">
</p>

<h1 align="center">Kurir — MCP harness integration</h1>

<p align="center">
  <strong>Register any MCP server with agent harnesses through one portable CLI and library.</strong>
</p>

<p align="center">
  <a href="https://github.com/suiflex/kurir/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/suiflex/kurir/ci.yml?branch=main&style=for-the-badge&label=CI" alt="CI status"></a>
  <a href="https://github.com/suiflex/kurir/releases"><img src="https://img.shields.io/github/v/tag/suiflex/kurir?include_prereleases&style=for-the-badge&label=release" alt="Release"></a>
  <a href="https://modelcontextprotocol.io"><img src="https://img.shields.io/badge/MCP-harness%20integration-4ade80?style=for-the-badge" alt="MCP harness integration"></a>
  <a href="https://m8ven.ai/mcp/suiflex-kurir-b1f3uw"><img src="https://m8ven.ai/badge/mcp/suiflex-kurir-b1f3uw" alt="M8ven Live Monitored"></a>
</p>

Kurir keeps MCP registration behavior in one reusable implementation. Products provide a generic server specification; Kurir handles harness-specific config paths, entry shapes, scopes, backups, delegated CLIs, dry runs, and diagnostics.

It is product-agnostic. Any MCP server can use it.

## Install

### Cargo

```sh
cargo install kurir
```

### npm

```sh
npm install --global @suiflex/kurir
```

The npm package downloads the matching native binary for the current platform. Set `KURIR_BIN` to use an existing binary.

### macOS and Linux

```sh
curl -fsSL https://raw.githubusercontent.com/suiflex/kurir/main/scripts/install.sh | sh
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/suiflex/kurir/main/scripts/install.ps1 | iex
```

### Homebrew and Scoop

```sh
brew install suiflex/tap/kurir
scoop bucket add suiflex https://github.com/suiflex/scoop-bucket
scoop install kurir
```

## Update

Check whether a newer release exists:

```sh
kurir update --check
```

Install the latest matching native release and verify its SHA-256 checksum:

```sh
kurir update
```

Use `--json` for scripts:

```sh
kurir update --check --json
```

Package managers remain authoritative for managed installations. Use `brew upgrade kurir`,
`scoop update kurir`, `cargo install kurir`, or `npm install --global @suiflex/kurir` when
you want the package manager to own the upgrade.

## Register a server

Register a server with ARSY CODE:

```sh
kurir register \
  --client arsy-code \
  --scope user \
  --name my-server \
  --command my-server \
  --arg mcp
```

Use `--scope project` to write the connection into ARSY's workspace
configuration. Kurir delegates to the installed `arsy mcp add` command and
supports ARSY's `stdio` and `http` transports. ARSY's CLI has no fields for
environment variables or HTTP headers, so Kurir rejects those fields instead
of silently dropping them.

```sh
kurir register \
  --client opencode \
  --name my-server \
  --command my-server \
  --arg mcp
```

Project-scoped registration:

```sh
kurir register \
  --client claude-code \
  --scope project \
  --name my-server \
  --command my-server \
  --arg mcp
```

Preview without writing:

```sh
kurir register \
  --client cursor \
  --name my-server \
  --command my-server \
  --arg mcp \
  --dry-run \
  --print
```

Load a complete server specification from JSON:

```sh
kurir register \
  --client opencode \
  --spec server.json
```

Kurir redacts environment values and headers in previews. Registration writes a `.bak` copy before replacing an existing configuration and refuses conflicts unless `--force` is explicit.

## Register lifecycle hooks

Register a lifecycle hook with a supported harness (e.g. Cursor, Claude Code, Codex):

```sh
kurir hook \
  --client cursor \
  --event PreToolUse \
  --command "my-linter-check" \
  --timeout 300
```

Preview without writing:

```sh
kurir hook \
  --client claude-code \
  --event Stop \
  --command "my-tool hook stop" \
  --dry-run \
  --print
```

## Install skills

Install a skill folder into the target harness skills directory:

```sh
kurir skill \
  --client cursor \
  --path ./my-skill \
  --scope project
```

## Supported harnesses

| Harness | Registration mode |
| --- | --- |
| ARSY CODE | `arsy mcp add` |
| Claude Code | JSON merge or `claude mcp add` |
| Claude Desktop | JSON merge |
| Codex | `codex mcp add` |
| Cursor | JSON merge |
| VS Code / Copilot | `code --add-mcp` |
| Gemini CLI | `gemini mcp add` |
| Copilot CLI | JSON merge |
| OpenCode | `opencode.json` merge |
| Windsurf | JSON merge |
| Zed | `context_servers` merge |
| OpenClaw | `mcp.servers` merge |
| Hermes | `hermes mcp add` |
| Antigravity CLI/Desktop | JSON merge |
| OMP | Redacted portable snippet |

Inspect the registry and local readiness:

```sh
kurir clients
kurir doctor
kurir doctor --client opencode
```

## Library

Rust consumers can use the `kurir` crate directly:

```rust
use kurir::{Harness, RegistrationOptions, ServerSpec, register};

let server = ServerSpec::stdio("my-server", "my-server", vec!["mcp".into()]);
let options = RegistrationOptions::default();
register(Harness::OpenCode, &server, &options)?;
```

To register lifecycle hooks or install skills programmatically:

```rust
use kurir::{Harness, HookSpec, RegistrationOptions, install_skill, register_hook};
use std::path::Path;

let hook = HookSpec {
    event: "Stop".into(),
    matcher: None,
    command: "my-tool hook stop".into(),
    timeout_seconds: 600,
};
let options = RegistrationOptions::default();
register_hook(Harness::Codex, &hook, &options)?;

install_skill(Harness::OpenCode, Path::new("./my-skill"), &options)?;
```

## Development

Requirements: Rust 1.88+, Cargo, Node 18+.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
npm test --prefix npm
```

## License

MIT. See [LICENSE](LICENSE).
