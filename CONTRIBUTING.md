# Contributing to Kurir

Thanks for contributing. Kurir is a public MCP harness integration toolkit; changes should remain generic and useful outside any one product family.

## Requirements

- Rust 1.88 or newer with `rustfmt` and `clippy`.
- Node 18 or newer for the npm launcher tests.
- A clean, focused change with no product-repository migration bundled into it.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
npm test --prefix npm
```

Run the real CLI against a temporary configuration path when changing registration behavior:

```sh
cargo run -- register \
  --client opencode \
  --name example \
  --command example \
  --arg mcp \
  --config /tmp/kurir-opencode.json \
  --dry-run \
  --print
```

## Design rules

- Add a harness adapter once in Kurir; do not copy its paths into consumer repositories.
- Prefer delegated CLIs where the harness owns its config format.
- Preserve unrelated config entries.
- Back up before rewriting an existing file.
- Refuse conflicting entries unless `--force` is explicit.
- Never print environment values or headers in previews.
- Add a behavior test for every new harness path, entry shape, scope, and failure mode.
- Keep product-specific policy outside Kurir.

## Pull requests

Use a focused branch and a Conventional Commit title. Explain the user-visible behavior, affected harnesses, security implications, and checks run. Do not include personal credentials or machine-specific paths.

By contributing, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
