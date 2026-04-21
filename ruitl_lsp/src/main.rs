//! `ruitl-lsp` binary. Stdio transport only — editors spawn this as a
//! subprocess and communicate over JSON-RPC on stdin/stdout.

use ruitl_lsp::Backend;
use tower_lsp::{LspService, Server};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(Backend::new).finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
