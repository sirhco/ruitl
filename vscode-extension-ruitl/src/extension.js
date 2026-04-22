// VS Code extension entry point. Boots a language client that talks to
// the `ruitl-lsp` binary over stdio. The binary must be installed by the
// user: `cargo install --path ruitl_lsp` from the RUITL repo.

const { workspace, window } = require("vscode");
const {
  LanguageClient,
  TransportKind,
} = require("vscode-languageclient/node");

let client;

function activate(context) {
  const config = workspace.getConfiguration("ruitl");
  const command = config.get("server.path") || "ruitl-lsp";

  const serverOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "ruitl" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.ruitl"),
    },
  };

  client = new LanguageClient(
    "ruitl",
    "RUITL Language Server",
    serverOptions,
    clientOptions
  );

  client.start().catch((err) => {
    window.showErrorMessage(
      `RUITL: failed to start ruitl-lsp (${command}). Install with \`cargo install --path ruitl_lsp\` from the RUITL repo. Error: ${err.message}`
    );
  });

  context.subscriptions.push({
    dispose: () => client && client.stop(),
  });
}

function deactivate() {
  if (!client) return undefined;
  return client.stop();
}

module.exports = { activate, deactivate };
