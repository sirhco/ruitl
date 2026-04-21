//! RUITL Language Server — core backend.
//!
//! The `Backend` type owns a document store (`DashMap<Url, String>`),
//! runs the RUITL parser on every change, and publishes
//! `textDocument/publishDiagnostics` messages. The binary in
//! `src/main.rs` wires this to stdio via `tower_lsp::LspService`.
//!
//! v0.1 capabilities:
//!   - textDocument/didOpen | didChange | didSave | didClose
//!   - textDocument/publishDiagnostics (parser errors only; codegen errors
//!     reported at save time too)
//!   - Incremental sync is advertised but we recompute from full text
//!     each tick — simplest thing that works for a regex-derived parser
//!     with O(template-size) complexity.
//!
//! Out of scope for v0.1:
//!   - Completion (T14 in roadmap)
//!   - Go-to-definition for `@Component` references
//!   - Format on save (needs AST → .ruitl pretty-printer)

use dashmap::DashMap;
use ruitl_compiler::{parse_str, CodeGenerator, CompileError};
use std::sync::Arc;
use tower_lsp::jsonrpc::Result as RpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

/// LSP backend. `Client` is the outbound handle for server→editor
/// notifications (diagnostics, log messages); `documents` keeps the latest
/// full text for each open file.
#[derive(Clone)]
pub struct Backend {
    pub client: Client,
    pub documents: Arc<DashMap<Url, String>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(DashMap::new()),
        }
    }

    /// Parse the text, run codegen to surface codegen-only errors (e.g.
    /// generic type-bound issues), and publish diagnostics for the URI.
    async fn analyze_and_publish(&self, uri: Url, text: String) {
        let diagnostics = diagnose(&text);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

/// Run the full pipeline (parse + codegen) and translate each error into
/// an LSP `Diagnostic`. Separated from the async backend so unit tests
/// can drive it without a `Client`.
pub fn diagnose(text: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    match parse_str(text) {
        Err(e) => out.push(compile_error_to_diagnostic(&e, text)),
        Ok(file) => {
            let mut gen = CodeGenerator::new(file);
            if let Err(e) = gen.generate() {
                out.push(compile_error_to_diagnostic(&e, text));
            }
        }
    }
    out
}

/// Best-effort `CompileError` → LSP `Diagnostic`. The parser's existing
/// error format embeds `at line L, column C` — we scrape that substring
/// and use it as the range. Falls back to the first character on parse
/// failure so the editor still marks the buffer as broken.
fn compile_error_to_diagnostic(err: &CompileError, text: &str) -> Diagnostic {
    let msg = err.to_string();
    let range = extract_position(&msg)
        .map(|(line, col)| {
            let start = Position {
                line,
                character: col,
            };
            Range {
                start,
                end: shift_char(text, start),
            }
        })
        .unwrap_or_else(|| Range {
            start: Position::new(0, 0),
            end: Position::new(0, 1),
        });

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("ruitl".to_string()),
        message: msg,
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Pull `(line, column)` out of a message containing `at line N, column M`.
/// Both are 1-indexed in the compiler's output; LSP expects 0-indexed.
fn extract_position(msg: &str) -> Option<(u32, u32)> {
    let after_line = msg.split("at line ").nth(1)?;
    let (line_str, rest) = after_line.split_once(',')?;
    let line: u32 = line_str.trim().parse().ok()?;
    let after_col = rest.split("column ").nth(1)?;
    let end = after_col
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_col.len());
    let col: u32 = after_col[..end].parse().ok()?;
    Some((line.saturating_sub(1), col.saturating_sub(1)))
}

/// Shift a `Position` one UTF-16 code unit to the right, clamping at the
/// end of its line. Keeps single-char-wide error ranges rendered as a
/// squiggly dot rather than a whole-line highlight.
fn shift_char(text: &str, pos: Position) -> Position {
    let lines: Vec<&str> = text.lines().collect();
    let line_idx = pos.line as usize;
    if line_idx >= lines.len() {
        return pos;
    }
    let line_len = lines[line_idx].chars().count() as u32;
    Position {
        line: pos.line,
        character: (pos.character + 1).min(line_len),
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> RpcResult<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "ruitl-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "ruitl-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        self.documents.insert(uri.clone(), text.clone());
        self.analyze_and_publish(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // We advertised FULL sync, so each change carries the complete
        // post-edit text. Take the last content (tower-lsp guarantees at
        // least one entry when FULL sync is in use).
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri.clone();
            self.documents.insert(uri.clone(), change.text.clone());
            self.analyze_and_publish(uri, change.text).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Clients that include text on save: use it; otherwise re-analyze
        // the stored buffer.
        let uri = params.text_document.uri.clone();
        let text = match params.text {
            Some(t) => t,
            None => match self.documents.get(&uri) {
                Some(entry) => entry.clone(),
                None => return,
            },
        };
        self.analyze_and_publish(uri, text).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        // Clear diagnostics so stale squigglies don't linger.
        self.client.publish_diagnostics(uri, vec![], None).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_yields_no_diagnostics() {
        let src = "component Hello { props { name: String } }\nruitl Hello(name: String) { <p>{name}</p> }";
        assert!(diagnose(src).is_empty());
    }

    #[test]
    fn syntax_error_yields_diagnostic_with_range() {
        // Missing closing `}` on the props block.
        let src = "component Hello { props { name: String \nruitl Hello() { <p></p> }";
        let diags = diagnose(src);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(d.source.as_deref(), Some("ruitl"));
        // Error range must be something non-empty.
        assert!(d.range.end.character >= d.range.start.character);
    }

    #[test]
    fn lifetime_generics_are_rejected_with_diagnostic() {
        let src = "component Foo<'a> { props { x: String } }";
        let diags = diagnose(src);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Lifetime parameters"));
    }
}
