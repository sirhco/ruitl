# RUITL — Zed Extension

Syntax highlighting + LSP integration for `.ruitl` files in Zed.

## Install as a dev extension (local)

This is the path for testing before anything is published to the Zed
extension registry.

1. **Build the LSP binary** from the RUITL workspace:
   ```bash
   cd /path/to/ruitl
   cargo install --path ruitl_lsp
   # Puts `ruitl-lsp` in ~/.cargo/bin/ — make sure that's on your PATH.
   which ruitl-lsp   # verify
   ```

2. **Install this directory as a dev extension** in Zed:
   - Open Zed.
   - Run the `zed: install dev extension` command (from the command
     palette).
   - Point it at the `zed-extension-ruitl/` directory (this folder).
   - Zed compiles the Rust source to WASM and registers the language.

3. **Open a `.ruitl` file.** You should see:
   - Syntax highlighting.
   - Parse errors underlined in red.
   - Hover/completion/go-to-definition on `@Component` references.

## Troubleshooting

- **"`ruitl-lsp` not found on PATH"** — the LSP binary must be reachable
  from the shell Zed inherits. On macOS, Zed inherits the login shell's
  PATH; running `zed` from a terminal after `cargo install` should work.
  If launched from the Dock, you may need to symlink `ruitl-lsp` into
  `/usr/local/bin/` or add `$HOME/.cargo/bin` to your shell's rc file.
- **No highlighting** — check Zed's logs (`zed: open log`). Missing
  grammar errors usually mean the grammar repo/commit in
  `extension.toml` isn't resolvable. For local dev, point
  `[grammars.ruitl]` at a local clone of the tree-sitter package:
  ```toml
  [grammars.ruitl]
  path = "../tree-sitter-ruitl"
  ```
- **LSP never starts** — run `ruitl-lsp` manually from the terminal. If
  it crashes standalone, fix that first. Open Zed's language-server log
  (`zed: open language server logs`) to see raw JSON-RPC traffic.

## Building for the registry (future)

Zed's extension publisher expects this repo structure:
```
extension.toml
src/ruitl.rs            (compiles to WASM)
Cargo.toml
languages/ruitl/
  config.toml
  highlights.scm
  injections.scm
grammars/               (optional if referenced remotely)
```

Publish flow (when ready):
```bash
# Zed extension cli required
cargo install zed_extension_cli
zed_extension publish zed-extension-ruitl
```

## Scope

This extension is **editor glue only**. All language semantics live in
the upstream crates:
- [`ruitl_compiler`](../ruitl_compiler/) — parser + codegen
- [`ruitl_lsp`](../ruitl_lsp/) — LSP server logic
- [`tree-sitter-ruitl`](../tree-sitter-ruitl/) — grammar + queries
