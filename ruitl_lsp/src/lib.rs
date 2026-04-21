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
use ruitl_compiler::{format, parse_str, CodeGenerator, CompileError, PropDef};
use std::sync::Arc;
use tower_lsp::jsonrpc::Result as RpcResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

/// One component's declaration metadata as known by the LSP. Enough to
/// answer completion, hover, and go-to-definition queries without
/// re-parsing the source document every time.
#[derive(Debug, Clone)]
pub struct IndexedComponent {
    pub name: String,
    pub props: Vec<PropDef>,
    /// 0-indexed `(line, column)` where the component name appears in
    /// the source file. Used as the go-to-definition target.
    pub decl_position: (u32, u32),
}

/// Per-document index entry: every component declared in that document.
/// A DashMap keyed by document URI gives us a simple workspace-wide
/// index — reconstructed on every parse, so it's always in sync with
/// the latest buffer contents.
pub type DocumentIndex = Vec<IndexedComponent>;

/// LSP backend. `Client` is the outbound handle for server→editor
/// notifications (diagnostics, log messages); `documents` keeps the latest
/// full text for each open file; `index` maps each URI to its component
/// metadata for completion / hover / go-to-definition.
#[derive(Clone)]
pub struct Backend {
    pub client: Client,
    pub documents: Arc<DashMap<Url, String>>,
    pub index: Arc<DashMap<Url, DocumentIndex>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(DashMap::new()),
            index: Arc::new(DashMap::new()),
        }
    }

    /// Rebuild the symbol index for a single document. Called on every
    /// successful parse; failed parses clear the entry so stale symbols
    /// don't linger.
    fn reindex(&self, uri: &Url, text: &str) {
        match parse_str(text) {
            Ok(file) => {
                let entries: DocumentIndex = file
                    .components
                    .iter()
                    .map(|c| IndexedComponent {
                        name: c.name.clone(),
                        props: c.props.clone(),
                        decl_position: locate_component_decl(text, &c.name)
                            .unwrap_or((0, 0)),
                    })
                    .collect();
                self.index.insert(uri.clone(), entries);
            }
            Err(_) => {
                self.index.remove(uri);
            }
        }
    }

    /// Walk every document's index and return all components whose name
    /// matches `name`. Returns `(uri, IndexedComponent)` pairs.
    fn lookup_component(&self, name: &str) -> Vec<(Url, IndexedComponent)> {
        let mut hits = Vec::new();
        for entry in self.index.iter() {
            for comp in entry.value() {
                if comp.name == name {
                    hits.push((entry.key().clone(), comp.clone()));
                }
            }
        }
        hits
    }

    /// Build a completion-item list for the declared props of the first
    /// component named `name` found in the workspace index. Used by the
    /// completion handler when the cursor is inside `@Name(...)`.
    fn prop_completion_items(&self, name: &str) -> Vec<CompletionItem> {
        let hits = self.lookup_component(name);
        let Some((_, comp)) = hits.into_iter().next() else {
            return Vec::new();
        };
        comp.props
            .iter()
            .map(|p| {
                let ty = if p.optional {
                    format!("Option<{}>", p.prop_type)
                } else {
                    p.prop_type.clone()
                };
                CompletionItem {
                    label: p.name.clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(format!("{}: {}", p.name, ty)),
                    insert_text: Some(format!("{}: ", p.name)),
                    ..Default::default()
                }
            })
            .collect()
    }

    /// Parse the text, rebuild the symbol index, run codegen to surface
    /// codegen-only errors, and publish diagnostics for the URI.
    async fn analyze_and_publish(&self, uri: Url, text: String) {
        self.reindex(&uri, &text);
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

/// Return the identifier token at `pos`. If `prefix` is `Some('@')`, the
/// token must be preceded by `@` (i.e. a component-invocation reference
/// or a `@Name` in a component declaration is NOT matched, only the
/// invocation form). Returns None if no identifier covers `pos`.
pub fn token_at_position(text: &str, pos: Position, prefix: Option<char>) -> Option<String> {
    let offset = position_to_offset(text, pos)?;
    let bytes = text.as_bytes();

    // Find identifier start by walking left.
    let mut start = offset.min(bytes.len());
    while start > 0 {
        let prev = bytes[start - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            start -= 1;
        } else {
            break;
        }
    }
    // Walk right to find identifier end.
    let mut end = offset.min(bytes.len());
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_alphanumeric() || b == b'_' {
            end += 1;
        } else {
            break;
        }
    }
    if start >= end {
        return None;
    }
    if let Some(p) = prefix {
        if start == 0 || bytes[start - 1] as char != p {
            return None;
        }
    }
    Some(text[start..end].to_string())
}

/// Render a component's metadata as GitHub-style markdown for hover.
fn render_component_markdown(comp: &IndexedComponent) -> String {
    let mut out = format!("**`@{}`** — RUITL component\n\n", comp.name);
    if comp.props.is_empty() {
        out.push_str("_No props._");
        return out;
    }
    out.push_str("```\nprops {\n");
    for p in &comp.props {
        let ty = if p.optional {
            format!("Option<{}>", p.prop_type)
        } else {
            p.prop_type.clone()
        };
        let suffix = match (&p.default_value, p.optional) {
            (Some(d), _) => format!(" = {}", d.trim()),
            (None, true) => String::new(), // Option<T> self-documents
            (None, false) => String::new(),
        };
        out.push_str(&format!("    {}: {}{},\n", p.name, ty, suffix));
    }
    out.push_str("}\n```");
    out
}

/// If the cursor at `pos` is inside an `@Component(...)` argument list,
/// return the component's name. Walks backward from the cursor character
/// by character until it finds either an unmatched `(` (match!) or hits a
/// structural boundary (`{`, `}`, `;`, newline-outside-arglist, or the
/// start of the buffer).
pub fn active_component_invocation(text: &str, pos: Position) -> Option<String> {
    let bytes = text.as_bytes();
    let target_offset = position_to_offset(text, pos)?;
    if target_offset > bytes.len() {
        return None;
    }

    let mut i = target_offset;
    let mut paren_depth: i32 = 0;
    while i > 0 {
        i -= 1;
        let c = bytes[i] as char;
        match c {
            ')' => paren_depth += 1,
            '(' => {
                if paren_depth == 0 {
                    // Found the opening paren of our enclosing call. The
                    // preceding token should be `@Name`.
                    return preceding_at_name(&text[..i]);
                }
                paren_depth -= 1;
            }
            '{' | '}' | ';' if paren_depth == 0 => return None,
            _ => {}
        }
    }
    None
}

/// From `text` ending just before a `(`, pull the identifier that
/// follows `@`. Returns None when `text` doesn't end with `@Name`.
fn preceding_at_name(text: &str) -> Option<String> {
    let trimmed = text.trim_end();
    // Walk backward collecting identifier chars.
    let bytes = trimmed.as_bytes();
    let mut end = bytes.len();
    while end > 0 {
        let b = bytes[end - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            end -= 1;
        } else {
            break;
        }
    }
    let name = &trimmed[end..];
    if name.is_empty() {
        return None;
    }
    if end == 0 || bytes[end - 1] != b'@' {
        return None;
    }
    Some(name.to_string())
}

/// Convert an LSP `Position` to a byte offset in `text`. UTF-16 aware-ish —
/// we treat `character` as a count of `char` (Unicode scalar values)
/// which is close enough for the ASCII-heavy `.ruitl` template syntax.
fn position_to_offset(text: &str, pos: Position) -> Option<usize> {
    let mut line = 0u32;
    let mut line_start = 0usize;
    for (idx, c) in text.char_indices() {
        if line == pos.line {
            // We're on the target line. Walk forward `pos.character` chars.
            let mut char_count = 0u32;
            for (jdx, _) in text[idx..].char_indices() {
                if char_count == pos.character {
                    return Some(idx + jdx);
                }
                char_count += 1;
            }
            // End of line reached before hitting the target column.
            return Some(text.len());
        }
        if c == '\n' {
            line += 1;
            line_start = idx + 1;
        }
    }
    if line == pos.line {
        // Position at EOF on the last line.
        return Some(line_start + pos.character as usize);
    }
    None
}

/// Locate the first `component <Name>` declaration in `text`. Returns
/// `(line, column)` of the name token (0-indexed). Best-effort — scans
/// line-by-line for `component <Name>` or `component <Name><`.
fn locate_component_decl(text: &str, name: &str) -> Option<(u32, u32)> {
    for (line_idx, line) in text.lines().enumerate() {
        // Look for `component ` followed by the target name as a whole
        // identifier (not a prefix of something longer).
        let prefix = "component ";
        if let Some(start) = line.find(prefix) {
            let after = &line[start + prefix.len()..];
            let ident_end = after
                .char_indices()
                .find(|(_, c)| !c.is_ascii_alphanumeric() && *c != '_')
                .map(|(i, _)| i)
                .unwrap_or(after.len());
            if &after[..ident_end] == name {
                let col = (start + prefix.len()) as u32;
                return Some((line_idx as u32, col));
            }
        }
    }
    None
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
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
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

    async fn hover(&self, params: HoverParams) -> RpcResult<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri;
        let pos = params.text_document_position_params.position;
        let Some(text) = self.documents.get(&uri).map(|e| e.clone()) else {
            return Ok(None);
        };

        let Some(name) = token_at_position(&text, pos, Some('@')) else {
            return Ok(None);
        };
        let hits = self.lookup_component(&name);
        let Some((_, comp)) = hits.into_iter().next() else {
            return Ok(None);
        };

        let md = render_component_markdown(&comp);
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: md,
            }),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> RpcResult<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri;
        let pos = params.text_document_position_params.position;
        let Some(text) = self.documents.get(&uri).map(|e| e.clone()) else {
            return Ok(None);
        };

        let Some(name) = token_at_position(&text, pos, Some('@')) else {
            return Ok(None);
        };
        let hits = self.lookup_component(&name);
        let locations: Vec<Location> = hits
            .into_iter()
            .map(|(uri, comp)| Location {
                uri,
                range: Range {
                    start: Position::new(comp.decl_position.0, comp.decl_position.1),
                    end: Position::new(
                        comp.decl_position.0,
                        comp.decl_position.1 + comp.name.chars().count() as u32,
                    ),
                },
            })
            .collect();

        if locations.is_empty() {
            Ok(None)
        } else if locations.len() == 1 {
            Ok(Some(GotoDefinitionResponse::Scalar(
                locations.into_iter().next().unwrap(),
            )))
        } else {
            Ok(Some(GotoDefinitionResponse::Array(locations)))
        }
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

        // Context detection takes priority over trigger char: if the
        // cursor sits inside an `@Component(...)` arg list, offer prop
        // names (scoped to that component's declaration) even when the
        // user typed a letter rather than hitting a trigger.
        if let Some(comp_name) = active_component_invocation(&text, pos) {
            let items = self.prop_completion_items(&comp_name);
            if !items.is_empty() {
                return Ok(Some(CompletionResponse::Array(items)));
            }
        }

        let items = match trigger
            .as_deref()
            .or(char_before.map(trigger_slice).as_deref())
        {
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
    fn token_at_position_finds_component_reference() {
        let text = "ruitl X() {\n    @Card(x: 1)\n}";
        // Cursor on `C` of `@Card`.
        let pos = Position::new(1, 5);
        let tok = token_at_position(text, pos, Some('@'));
        assert_eq!(tok.as_deref(), Some("Card"));
    }

    #[test]
    fn token_at_position_rejects_when_missing_prefix() {
        let text = "component Card {}";
        // Cursor on `Card` — preceded by space, not `@`.
        let pos = Position::new(0, 11);
        assert!(token_at_position(text, pos, Some('@')).is_none());
    }

    #[test]
    fn render_component_markdown_includes_props() {
        let comp = IndexedComponent {
            name: "Box".to_string(),
            props: vec![PropDef {
                name: "value".to_string(),
                prop_type: "String".to_string(),
                optional: false,
                default_value: None,
            }],
            decl_position: (0, 10),
        };
        let md = render_component_markdown(&comp);
        assert!(md.contains("@Box"));
        assert!(md.contains("value: String"));
    }

    #[test]
    fn active_component_detects_cursor_inside_arglist() {
        let text = "ruitl X() {\n    @MyCard(name: \"a\", age: 2)\n}";
        // Cursor after `@MyCard(` — line 1, col 12 (0-indexed).
        let pos = Position::new(1, 12);
        let name = active_component_invocation(text, pos);
        assert_eq!(name.as_deref(), Some("MyCard"));
    }

    #[test]
    fn active_component_returns_none_outside_arglist() {
        let text = "ruitl X() {\n    <div>@MyCard(a: 1)</div>\n}";
        // Cursor inside <div> but before `@MyCard(`. Should be None.
        let pos = Position::new(1, 5);
        assert!(active_component_invocation(text, pos).is_none());
    }

    #[test]
    fn char_before_position_handles_edges() {
        let text = "abc\ndef";
        assert_eq!(char_before_position(text, Position::new(0, 0)), None);
        assert_eq!(char_before_position(text, Position::new(0, 1)), Some('a'));
        assert_eq!(char_before_position(text, Position::new(1, 2)), Some('e'));
    }
}
