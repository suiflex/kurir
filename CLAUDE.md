# Kurir

Kurir is a public, product-agnostic MCP harness integration toolkit. Rust is the canonical implementation; the npm package launches the matching native binary.

## Contract

- `ServerSpec` describes an MCP server: name, transport, command/args or URL, environment, and headers.
- `Harness` describes the consuming client, not the MCP server.
- Registration is idempotent, merge-safe, and explicit about conflicts.
- Existing configuration is backed up before a rewrite.
- Dry runs never write files or invoke delegated harness commands.
- Previews redact environment values and headers.
- Project configuration must never require secrets to be committed.

## Modules

| Path | Responsibility |
| --- | --- |
| `src/model.rs` | Public server, transport, scope, and result contracts |
| `src/harness/mod.rs` | Harness registry and capability classification |
| `src/jsonc.rs` | Safe JSON/JSONC object loading and writing |
| `src/registration/mod.rs` | File merge, delegated CLI, doctor, and entry-shape logic |
| `src/hooks.rs` | Harness hook file paths and hook handler shapes |
| `src/skills.rs` | Harness skill directories per scope |
| `src/main.rs` | `kurir` CLI |
| `npm/` | Native binary launcher for npm consumers |
| `packaging/` | Homebrew and Scoop templates |
| `src/update.rs` | Release discovery, checksum verification, and self-update |
| `scripts/` | Cross-platform install scripts |

## Harness rules

- Keep harness paths and entry shapes in Kurir, never in product repositories.
- Prefer the harness's own CLI when it owns configuration migration.
- Use the official OpenCode global path `~/.config/opencode/opencode.json`.
- Keep ambiguous harness surfaces separate (`antigravity-cli` and `antigravity-desktop`).
- Snippet-only support is valid when a harness exposes no stable registration API.

- `kurir update --check` must remain network-only and read-only.
- Binary updates must verify the published SHA-256 before replacement.

## Security

Kurir writes configuration files for other applications and may launch delegated CLIs. Treat all server specs as untrusted input: validate names, constrain writes to explicit targets, never follow a product-specific secret convention, redact values in output, and never execute a command during `--dry-run`.

The MCP server command itself is user-selected and therefore intentionally executable. Do not broaden that capability with shell evaluation, implicit downloads, or hidden network calls.

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
npm test --prefix npm
```

Do not modify the SuiFlex product repositories as part of Kurir work. Migration adapters belong in separate changes after Kurir is released.
