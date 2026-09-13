# @suiflex/kurir

Portable MCP server registration for agent harnesses.

```sh
npm install --global @suiflex/kurir
kurir clients
```

The npm package is a thin launcher around the Kurir native binary. It downloads the matching release asset during installation. Use `KURIR_BIN=/path/to/kurir` to run a local build instead.

Register a server:

```sh
kurir register \
  --client opencode \
  --name my-server \
  --command my-server \
  --arg mcp
```

Check for a newer release or update the native binary:

```sh
kurir update --check
kurir update
```

Use `npm install --global @suiflex/kurir` when you want npm to own the update instead.

Set `KURIR_SKIP_INSTALL=1` to skip the postinstall download when a compatible binary is already available through `PATH` or `KURIR_BIN`.

Kurir is distributed under the MIT license.
