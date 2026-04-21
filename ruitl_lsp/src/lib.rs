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
use ruitl_compiler::{format, parse_str, CodeGenerator, CompileError};
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

/// Common HTML5 element tag names. Intentionally a flat allowlist — no
/// attempt to distinguish void / self-closing because `.ruitl`'s codegen
/// handles that based on the element-building API.
const HTML_TAGS: &[&str] = &[
    "a", "abbr", "address", "area", "article", "aside", "audio", "b",
    "base", "bdi", "bdo", "blockquote", "body", "br", "button", "canvas",
    "caption", "cite", "code", "col", "colgroup", "data", "datalist", "dd",
    "del", "details", "dfn", "dialog", "div", "dl", "dt", "em", "embed",
    "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2", "h3",
    "h4", "h5", "h6", "head", "header", "hr", "html", "i", "iframe", "img",
    "input", "ins", "kbd", "label", "legend", "li", "link", "main", "map",
    "mark", "meta", "meter", "nav", "noscript", "ol", "optgroup", "option",
    "output", "p", "picture", "pre", "progress", "q", "rp", "rt", "ruby",
    "s", "samp", "script", "section", "select", "small", "source", "span",
    "strong", "style", "sub", "summary", "sup", "svg", "table", "tbody",
    "td", "template", "textarea", "tfoot", "th", "thead", "time", "title",
    "tr", "track", "u", "ul", "var", "video", "wbr",
];

fn html_tag_completion_items() -> Vec<CompletionItem> {
    HTML_TAGS
        .iter()
        .map(|tag| CompletionItem {
            label: tag.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("HTML element".to_string()),
            ..Default::default()
        })
        .collect()
}

/// Extract component names from the given document. Falls back to an
/// empty list when the document doesn't parse.
fn component_completion_items(text: &str) -> Vec<CompletionItem> {
    let Ok(file) = parse_str(text) else {
        return Vec::new();
    };
    file.components
        .iter()
        .map(|c| CompletionItem {
            label: c.name.clone(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some(format!("RUITL component ({} prop(s))", c.props.len())),
            insert_text: Some(format!("{}()", c.name)),
            ..Default::default()
        })
        .collect()
}

/// Best-effort: character immediately before `pos`. Returns None on
/// line 0 column 0 or malformed positions.
fn char_before_position(text: &str, pos: Position) -> Option<char> {
    let line = text.lines().nth(pos.line as usize)?;
    if pos.character == 0 {
        return None;
    }
    line.chars().nth((pos.character - 1) as usize)
}

fn trigger_slice(c: char) -> String {
    c.to_string()
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
                document_formatting_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    // Trigger on `@` (component invocation) and `<` (HTML
                    // tag). Without triggers, clients still invoke
                    // completion on manual request, so this only adds
                    // auto-fire points.
                    trigger_characters: Some(vec!["@".to_string(), "<".to_string()]),
                    ..Default::default()
                }),
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

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> RpcResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let text = match self.documents.get(&uri) {
            Some(t) => t.clone(),
            None => return Ok(None),
        };

        let trigger = params
            .context
            .as_ref()
            .and_then(|c| c.trigger_character.clone());
        let char_before = char_before_position(&text, pos);

        let items = match trigger.as_deref().or(char_before.map(trigger_slice).as_deref()) {
            Some("@") => component_completion_items(&text),
            Some("<") => html_tag_completion_items(),
            _ => {
                // Manual invocation without a trigger char. Offer both sets
                // so users can always get help.
                let mut both = component_completion_items(&text);
                both.extend(html_tag_completion_items());
                both
            }
        };

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> RpcResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let original = match self.documents.get(&uri) {
            Some(entry) => entry.clone(),
            None => return Ok(None),
        };

        let formatted = match format::format_source(&original) {
            Ok(s) => s,
            Err(_) => {
                // Don't modify a file we can't parse — editor will show the
                // parse error from the diagnostic channel instead.
                return Ok(None);
            }
        };

        if formatted == original {
            return Ok(Some(Vec::new()));
        }

        // Replace the entire document. The end position covers every line
        // at column 0, which is the LSP-idiomatic way to select "to EOF".
        let line_count = original.lines().count().max(1) as u32;
        let edit = TextEdit {
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(line_count, 0),
            },
            new_text: formatted,
        };
        Ok(Some(vec![edit]))
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

    #[test]
    fn component_completion_lists_declared_components() {
        let src = "component Alpha { props { x: String } }\n\
                   component Beta { props {} }\n\
                   ruitl Alpha(x: String) { <p>{x}</p> }";
        let items = component_completion_items(src);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"Alpha"));
        assert!(labels.contains(&"Beta"));
        // Insert text should include `()` so the editor lands the cursor
        // for the user to start typing props.
        assert!(items
            .iter()
            .any(|i| i.insert_text.as_deref() == Some("Alpha()")));
    }

    #[test]
    fn html_tag_completion_covers_common_tags() {
        let items = html_tag_completion_items();
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        for expected in &["div", "span", "button", "form", "input", "table"] {
            assert!(
                labels.contains(expected),
                "html tag completion missing `{}`",
                expected
            );
        }
    }

    #[test]
    fn char_before_position_handles_edges() {
        let text = "abc\ndef";
        assert_eq!(char_before_position(text, Position::new(0, 0)), None);
        assert_eq!(char_before_position(text, Position::new(0, 1)), Some('a'));
        assert_eq!(char_before_position(text, Position::new(1, 2)), Some('e'));
    }
}
