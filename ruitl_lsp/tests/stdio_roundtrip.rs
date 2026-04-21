//! End-to-end LSP roundtrip over an in-memory duplex pipe.
//!
//! Drives `tower_lsp::Server` through JSON-RPC messages framed with the
//! standard LSP `Content-Length` header. Verifies the server publishes a
//! `textDocument/publishDiagnostics` notification after `didOpen` with a
//! broken template, and publishes an empty diagnostics list for a valid one.
//!
//! Intentionally minimal — we don't depend on any LSP-client crate. A
//! hand-rolled reader/writer is cheaper than adding another dep and makes
//! the framing assumptions explicit.

use ruitl_lsp::Backend;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn frame_write<W: AsyncWriteExt + Unpin>(w: &mut W, body: &str) {
    let msg = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    w.write_all(msg.as_bytes()).await.unwrap();
    w.flush().await.unwrap();
}

/// Read one Content-Length-framed JSON payload from `r`. Returns the
/// body as a String. Panics on malformed framing (fine for tests).
async fn frame_read<R: AsyncReadExt + Unpin>(r: &mut R) -> String {
    // Read headers byte-by-byte until `\r\n\r\n`.
    let mut headers = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        let n = r.read(&mut buf).await.unwrap();
        assert!(n == 1, "stream closed while reading headers");
        headers.push(buf[0]);
        if headers.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_text = String::from_utf8(headers).unwrap();
    let content_length: usize = header_text
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length: "))
        .expect("Content-Length header")
        .trim()
        .parse()
        .unwrap();

    let mut body = vec![0u8; content_length];
    r.read_exact(&mut body).await.unwrap();
    String::from_utf8(body).unwrap()
}

/// Spin up the server on an in-memory duplex pair. Returns the client-side
/// read/write halves + a join handle so the test can await clean shutdown.
async fn spawn_server() -> (
    tokio::io::DuplexStream,
    tokio::task::JoinHandle<()>,
) {
    let (client_side, server_side) = tokio::io::duplex(64 * 1024);
    let (server_read, server_write) = tokio::io::split(server_side);

    let handle = tokio::spawn(async move {
        let (service, socket) = tower_lsp::LspService::build(Backend::new).finish();
        tower_lsp::Server::new(server_read, server_write, socket)
            .serve(service)
            .await;
    });
    (client_side, handle)
}

async fn initialize<W: AsyncWriteExt + Unpin, R: AsyncReadExt + Unpin>(w: &mut W, r: &mut R) {
    frame_write(
        w,
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    )
    .await;
    let _init_response = frame_read(r).await;
    frame_write(
        w,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    )
    .await;
}

// No explicit LSP shutdown — the duplex pair's `Drop` closes the client
// side, which the server observes as EOF and exits its read loop. Tests
// `.abort()` the join handle to avoid waiting on the serve future.

/// Drain server notifications until we see a `publishDiagnostics` for `uri`.
/// Skips informational messages (`window/logMessage`, etc.).
async fn next_diagnostics_for<R: AsyncReadExt + Unpin>(r: &mut R, uri: &str) -> String {
    for _ in 0..10 {
        let msg = frame_read(r).await;
        if msg.contains(r#""method":"textDocument/publishDiagnostics""#) && msg.contains(uri) {
            return msg;
        }
    }
    panic!("timed out waiting for diagnostics for {}", uri);
}

#[tokio::test]
async fn diagnostic_published_for_invalid_template() {
    let (stream, handle) = spawn_server().await;
    let (mut r, mut w) = tokio::io::split(stream);

    initialize(&mut w, &mut r).await;

    // Invalid: props block not closed.
    let open = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/bad.ruitl","languageId":"ruitl","version":1,"text":"component Hello { props { name: String \n"}}}"#;
    frame_write(&mut w, open).await;

    let msg = next_diagnostics_for(&mut r, "file:///tmp/bad.ruitl").await;
    assert!(
        msg.contains(r#""severity":1"#),
        "expected ERROR-severity diagnostic, got: {}",
        msg
    );
    assert!(
        msg.contains(r#""source":"ruitl""#),
        "diagnostic should carry `source: ruitl`: {}",
        msg
    );

    handle.abort();
}

#[tokio::test]
async fn no_diagnostics_for_valid_template() {
    let (stream, handle) = spawn_server().await;
    let (mut r, mut w) = tokio::io::split(stream);

    initialize(&mut w, &mut r).await;

    let open = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/good.ruitl","languageId":"ruitl","version":1,"text":"component Hello { props { name: String } }\nruitl Hello(name: String) { <p>{name}</p> }"}}}"#;
    frame_write(&mut w, open).await;

    let msg = next_diagnostics_for(&mut r, "file:///tmp/good.ruitl").await;
    assert!(
        msg.contains(r#""diagnostics":[]"#),
        "expected empty diagnostics array, got: {}",
        msg
    );

    handle.abort();
}
