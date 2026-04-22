# RUITL — VS Code extension

Syntax highlighting and Language Server support for [RUITL](https://github.com/sirhco/ruitl)
templates in Visual Studio Code.

## Features

- Syntax highlighting for `.ruitl` files (components, templates, HTML tags,
  embedded Rust expressions)
- LSP-powered diagnostics, formatting, completion, hover, and go-to-definition
  via the bundled `ruitl-lsp` language server

## Install

1. Install the language server from the RUITL repo:

   ```bash
   git clone https://github.com/sirhco/ruitl.git
   cd ruitl
   cargo install --path ruitl_lsp
   ```

2. Install this extension:

   ```bash
   cd vscode-extension-ruitl
   npm install
   npx vsce package
   code --install-extension ruitl-0.1.0.vsix
   ```

   Or publish once and install from the Marketplace:

   ```bash
   npx vsce publish
   ```

## Configuration

- `ruitl.server.path` — Path to the `ruitl-lsp` binary (default: resolved
  from `$PATH`).
- `ruitl.server.trace` — Controls LSP tracing in the Output panel
  (`off` | `messages` | `verbose`).

## Development

```bash
cd vscode-extension-ruitl
npm install
# Open this directory in VS Code and press F5 to launch an extension host.
```
