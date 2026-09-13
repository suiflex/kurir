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

## Supported harnesses

| Harness | Registration mode |
| --- | --- |
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
