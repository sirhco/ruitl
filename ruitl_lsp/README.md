# ruitl-lsp

Language Server Protocol implementation for RUITL templates. Built on
[`tower-lsp`](https://github.com/ebkalderon/tower-lsp), talks JSON-RPC over
stdio.

## What it does (v0.1)

- Parses every `.ruitl` file on open/change/save.
- Runs codegen on the parsed AST to surface codegen-only errors (e.g.
  invalid generic bounds, unsupported constructs).
- Publishes `textDocument/publishDiagnostics` for each error, with range
  pulled from the compiler's `at line L, column C` format.

## What it doesn't do (yet)

- No completion — component names, props, HTML tags and attrs all still
  require typing. Roadmap: v0.5+.
- No go-to-definition for `@Component` references. Needs a cross-file
  index; roadmap: v0.5+.
- No format-on-save. That requires an AST → `.ruitl` pretty-printer which
  doesn't exist. Tracked as a separate feature.
- No rust-analyzer bridge for expressions inside `{...}`. Out of scope
  for any foreseeable version — delegated to the rust-analyzer server
  for the corresponding `.rs` file.

## Install

```bash
cargo install --path ruitl_lsp
# Installs `ruitl-lsp` binary into ~/.cargo/bin/
```

Or run from the workspace:

```bash
cargo build -p ruitl_lsp
# Binary at target/debug/ruitl-lsp
```

## Editor wiring

### Neovim (via `nvim-lspconfig`)

```lua
local lspconfig = require("lspconfig")
local configs = require("lspconfig.configs")

if not configs.ruitl then
  configs.ruitl = {
    default_config = {
      cmd = { "ruitl-lsp" },
      filetypes = { "ruitl" },
      root_dir = lspconfig.util.root_pattern("ruitl.toml", "Cargo.toml", ".git"),
      settings = {},
    },
  }
end

lspconfig.ruitl.setup({})

vim.filetype.add({ extension = { ruitl = "ruitl" } })
```

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "ruitl"
scope = "source.ruitl"
file-types = ["ruitl"]
roots = ["ruitl.toml", "Cargo.toml"]
language-servers = ["ruitl-lsp"]

[language-server.ruitl-lsp]
command = "ruitl-lsp"
```

### VS Code

VS Code needs a thin extension to translate language-id. Minimal wiring
in `extension.ts`:

```ts
import * as vscode from "vscode";
import { LanguageClient, ServerOptions, TransportKind } from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(ctx: vscode.ExtensionContext) {
  const server: ServerOptions = {
    command: "ruitl-lsp",
    transport: TransportKind.stdio,
  };
  client = new LanguageClient("ruitl-lsp", "RUITL", server, {
    documentSelector: [{ scheme: "file", language: "ruitl" }],
  });
  client.start();
}

export function deactivate() {
  return client?.stop();
}
```

Pair with a `languages` contribution in `package.json`:

```json
"contributes": {
  "languages": [{
    "id": "ruitl",
    "extensions": [".ruitl"],
    "aliases": ["RUITL"]
  }]
}
```

### Zed

Zed auto-discovers LSP servers via its extension registry. A published
extension ships this config; until then add a workspace-local
`lsp_settings.json`:

```json
{
  "ruitl-lsp": {
    "binary": { "path": "ruitl-lsp" },
    "enable_language_server": true
  }
}
```

## Debugging

- Run the server manually and pipe in a hand-crafted `initialize`
  request to verify framing:
  ```bash
  cargo run -p ruitl_lsp
  # Then type Content-Length:-framed JSON on stdin.
  ```
- LSP log messages (`window/logMessage`) appear in the editor's language
  server log view.
- The integration test at `ruitl_lsp/tests/stdio_roundtrip.rs` is a
  reference for the expected JSON-RPC traffic.

## Contributing

- Unit tests live in `src/lib.rs` (`mod tests`) — they exercise the pure
  `diagnose()` function without spawning the server.
- End-to-end tests live in `tests/stdio_roundtrip.rs`. Drive the server
  through an in-memory `tokio::io::duplex` pair; no real process spawn
  needed.
- When adding a new notification handler, add both a unit test for the
  synchronous logic and an integration test that exercises the wire
  protocol.
