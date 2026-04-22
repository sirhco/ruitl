//! RUITL Template Parser
//!
//! Parses .ruitl files and converts them to an AST that can be compiled to Rust code

use crate::error::{CompileError, Result};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct RuitlFile {
    pub components: Vec<ComponentDef>,
    pub templates: Vec<TemplateDef>,
    pub imports: Vec<ImportDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentDef {
    pub name: String,
    pub props: Vec<PropDef>,
    pub generics: Vec<GenericParam>,
    /// Line / block comments that immediately precede this declaration.
    /// Stored verbatim (without the `//` or `/* */` markers) so the
    /// formatter can re-emit them in canonical position.
    pub leading_comments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropDef {
    pub name: String,
    pub prop_type: String,
    pub optional: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateDef {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub body: TemplateAst,
    pub generics: Vec<GenericParam>,
    /// See `ComponentDef::leading_comments`.
    pub leading_comments: Vec<String>,
}

/// A single generic type parameter: `T` or `T: Bound1 + Bound2`.
#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDef {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDef {
    pub path: String,
    pub items: Vec<String>,
    /// See `ComponentDef::leading_comments`.
    pub leading_comments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateAst {
    /// HTML element: <div class="foo">content</div>
    Element {
        tag: String,
        attributes: Vec<Attribute>,
        children: Vec<TemplateAst>,
        self_closing: bool,
    },
    /// Plain text content
    Text(String),
    /// Rust expression: {expr}
    Expression(String),
    /// Raw-HTML Rust expression: `{!expr}`. Content is emitted via
    /// `Html::raw(...)` instead of `Html::text(...)`, so the rendered
    /// result is injected verbatim without HTML-entity escaping. Use
    /// sparingly — caller is responsible for ensuring the expression
    /// produces safe HTML.
    RawExpression(String),
    /// Conditional rendering: if condition { ... } else { ... }
    If {
        condition: String,
        then_branch: Box<TemplateAst>,
        else_branch: Option<Box<TemplateAst>>,
    },
    /// Loop rendering: for item in items { ... }
    For {
        variable: String,
        iterable: String,
        body: Box<TemplateAst>,
    },
    /// Match expression: match expr { ... }
    Match {
        expression: String,
        arms: Vec<MatchArm>,
    },
    /// Component invocation: `@Button(props)` or `@Card(title: "x") { <p/>body }`.
    /// `children` carries the optional `{ ... }` body block passed to the
    /// callee as its `children: Html` prop.
    Component {
        name: String,
        props: Vec<PropValue>,
        children: Option<Box<TemplateAst>>,
    },
    /// `{children}` inside a template body — placeholder that is replaced at
    /// codegen with `props.children.clone()`. The props struct for the owning
    /// component auto-gains a `pub children: Html` field when this variant
    /// appears anywhere in the body.
    Children,
    /// Multiple nodes
    Fragment(Vec<TemplateAst>),
    /// Raw HTML (unescaped)
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub value: AttributeValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    /// Static string value: class="foo"
    Static(String),
    /// Expression value: class={expr}
    Expression(String),
    /// Conditional attribute: disabled?={condition}
    Conditional(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: String,
    pub body: TemplateAst,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
pub struct RuitlParser {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    /// Comments collected by `skip_whitespace_and_comments` that haven't
    /// yet been attached to a declaration. The next top-level `parse_*`
    /// drains this buffer into its `leading_comments` field.
    pending_comments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}

impl RuitlParser {
    pub fn new(input: String) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            pending_comments: Vec::new(),
        }
    }

    pub fn parse(&mut self) -> Result<RuitlFile> {
        let mut components = Vec::new();
        let mut templates = Vec::new();
        let mut imports = Vec::new();

        self.skip_whitespace_and_comments();

        while !self.is_at_end() {
            if self.match_keyword("import") {
                imports.push(self.parse_import()?);
            } else if self.match_keyword("component") {
                components.push(self.parse_component()?);
            } else if self.match_keyword("ruitl") {
                templates.push(self.parse_template()?);
            } else {
                return Err(self.error("Expected 'import', 'component', or 'ruitl'"));
            }
            self.skip_whitespace_and_comments();
        }

        Ok(RuitlFile {
            components,
            templates,
            imports,
        })
    }

    fn parse_import(&mut self) -> Result<ImportDef> {
        let leading_comments = self.take_pending_comments();
        self.skip_whitespace();
        let path = self.parse_string_literal()?;

        self.skip_whitespace();
        if !self.match_char('{') {
            return Err(self.error("Expected '{' after import path"));
        }

        let mut items = Vec::new();
        self.skip_whitespace();

        while !self.check_char('}') && !self.is_at_end() {
            let item = self.parse_identifier()?;
            items.push(item);

            self.skip_whitespace();
            if self.match_char(',') {
                self.skip_whitespace();
            } else if !self.check_char('}') {
                return Err(self.error("Expected ',' or '}' in import list"));
            }
        }

        if !self.match_char('}') {
            return Err(self.error("Expected '}' to close import list"));
        }

        Ok(ImportDef {
            path,
            items,
            leading_comments,
        })
    }

    fn parse_component(&mut self) -> Result<ComponentDef> {
        let leading_comments = self.take_pending_comments();
        self.skip_whitespace();
        let name = self.parse_identifier()?;

        self.skip_whitespace();
        let generics = if self.check_char('<') {
            self.parse_generics()?
        } else {
            Vec::new()
        };

        self.skip_whitespace();
        if !self.match_char('{') {
            return Err(self.error("Expected '{' after component name"));
        }

        self.skip_whitespace_and_comments();

        let mut props = Vec::new();

        if self.match_keyword("props") {
            self.skip_whitespace();
            if !self.match_char('{') {
                return Err(self.error("Expected '{' after 'props'"));
            }

            self.skip_whitespace_and_comments();
            while !self.check_char('}') && !self.is_at_end() {
                props.push(self.parse_prop_def()?);
                self.skip_whitespace_and_comments();
            }

            if !self.match_char('}') {
                return Err(self.error("Expected '}' to close props block"));
            }
            self.skip_whitespace_and_comments();
        }

        if !self.match_char('}') {
            return Err(self.error("Expected '}' to close component definition"));
        }

        Ok(ComponentDef {
            name,
            props,
            generics,
            leading_comments,
        })
    }

    fn parse_prop_def(&mut self) -> Result<PropDef> {
        let name = self.parse_identifier()?;

        self.skip_whitespace();
        if !self.match_char(':') {
            return Err(self.error("Expected ':' after prop name"));
        }

        self.skip_whitespace();
        let prop_type = self.parse_type()?;

        self.skip_whitespace();
        let mut optional = false;
        let mut default_value = None;

        if self.match_char('=') {
            self.skip_whitespace();
            default_value = Some(self.parse_expression_until(&[',', '\n', '}'])?);
        } else if self.match_char('?') {
            optional = true;
        }

        self.skip_whitespace();
        if self.match_char(',') {
            self.skip_whitespace();
        }

        Ok(PropDef {
            name,
            prop_type,
            optional,
            default_value,
        })
    }

    fn parse_template(&mut self) -> Result<TemplateDef> {
        let leading_comments = self.take_pending_comments();
        self.skip_whitespace();
        let name = self.parse_identifier()?;

        self.skip_whitespace();
        let generics = if self.check_char('<') {
            self.parse_generics()?
        } else {
            Vec::new()
        };

        self.skip_whitespace();
        if !self.match_char('(') {
            return Err(self.error("Expected '(' after template name"));
        }

        let mut params = Vec::new();
        self.skip_whitespace();

        while !self.check_char(')') && !self.is_at_end() {
            let param_name = self.parse_identifier()?;
            self.skip_whitespace();

            if !self.match_char(':') {
                return Err(self.error("Expected ':' after parameter name"));
            }

            self.skip_whitespace();
            let param_type = self.parse_type()?;

            params.push(ParamDef {
                name: param_name,
                param_type,
            });

            self.skip_whitespace();
            if self.match_char(',') {
                self.skip_whitespace();
            } else if !self.check_char(')') {
                return Err(self.error("Expected ',' or ')' in parameter list"));
            }
        }

        if !self.match_char(')') {
            return Err(self.error("Expected ')' to close parameter list"));
        }

        self.skip_whitespace();
        if !self.match_char('{') {
            return Err(self.error("Expected '{' to start template body"));
        }

        let body = self.parse_template_body()?;

        if !self.match_char('}') {
            return Err(self.error("Expected '}' to close template body"));
        }

        Ok(TemplateDef {
            name,
            params,
            body,
            generics,
            leading_comments,
        })
    }

    /// Parse `<T, U: Bound1 + Bound2>` — a comma-separated list of generic
    /// parameters, each optionally followed by `: Bound1 + Bound2 + ...`.
    fn parse_generics(&mut self) -> Result<Vec<GenericParam>> {
        if !self.match_char('<') {
            return Err(self.error("Expected '<' to start generic parameter list"));
        }

        let mut params = Vec::new();
        self.skip_whitespace();

        while !self.check_char('>') && !self.is_at_end() {
            // Reject lifetime parameters (`<'a>`) explicitly. RUITL components
            // use owned types only; lifetime-generic components would need
            // lifetime inference on the render method, which is out of scope
            // for v0.2.
            if self.check_char('\'') {
                return Err(self.error(
                    "Lifetime parameters in component declarations are not supported; use owned types",
                ));
            }
            let name = self.parse_identifier()?;
            self.skip_whitespace();

            let mut bounds = Vec::new();
            if self.match_char(':') {
                self.skip_whitespace();
                loop {
                    let bound = self.parse_generic_bound()?;
                    if !bound.is_empty() {
                        bounds.push(bound);
                    }
                    self.skip_whitespace();
                    if !self.match_char('+') {
                        break;
                    }
                    self.skip_whitespace();
                }
            }

            params.push(GenericParam { name, bounds });

            self.skip_whitespace();
            if self.match_char(',') {
                self.skip_whitespace();
            } else if !self.check_char('>') {
                return Err(self.error("Expected ',' or '>' in generic parameter list"));
            }
        }

        if !self.match_char('>') {
            return Err(self.error("Expected '>' to close generic parameter list"));
        }

        Ok(params)
    }

    /// Parse a single trait-bound inside a generic parameter list. Stops at
    /// `+` (next bound), `,` (next param), or `>` (end of list). Nested
    /// `<...>` is tracked so bounds like `Iterator<Item=u32>` work.
    fn parse_generic_bound(&mut self) -> Result<String> {
        let mut out = String::new();
        let mut angle_depth = 0i32;

        while !self.is_at_end() {
            let ch = self.current_char();
            match ch {
                '<' => angle_depth += 1,
                '>' if angle_depth > 0 => angle_depth -= 1,
                '>' | ',' | '+' if angle_depth == 0 => break,
                _ => {}
            }
            out.push(ch);
            self.advance();
        }

        Ok(out.trim().to_string())
    }

    fn parse_template_body(&mut self) -> Result<TemplateAst> {
        let mut nodes = Vec::new();
        self.skip_whitespace();

        while !self.check_char('}') && !self.is_at_end() {
            let node = self.parse_template_node()?;
            nodes.push(node);
            self.skip_whitespace();
        }

        if nodes.len() == 1 {
            Ok(nodes.into_iter().next().unwrap())
        } else {
            Ok(TemplateAst::Fragment(nodes))
        }
    }

    fn parse_template_node(&mut self) -> Result<TemplateAst> {
        // Whitespace between an expression (`{x}`) and adjacent text is
        // significant for HTML rendering. Only eat leading whitespace when the
        // next non-whitespace token is a structured node (element /
        // expression / component / control-flow keyword). When it's text,
        // keep the whitespace as part of the text node.
        let after_ws = self.cursor_after_whitespace();
        let next_is_structured = if after_ws >= self.input.len() {
            false
        } else {
            let c = self.input[after_ws];
            c == '<'
                || c == '{'
                || c == '@'
                || c == '}'
                || self.at_keyword_at(after_ws, &["if", "for", "match", "else"])
        };

        if next_is_structured {
            self.skip_whitespace();
        }

        if self.check_char('<') {
            // Check if this is a DOCTYPE declaration
            if self.peek_string(9) == "<!DOCTYPE" {
                self.parse_doctype()
            } else {
                self.parse_element()
            }
        } else if self.check_char('{') {
            self.parse_expression_node()
        } else if self.check_char('@') {
            self.parse_component_invocation()
        } else if self.match_keyword("if") {
            self.parse_if_statement()
        } else if self.match_keyword("for") {
            self.parse_for_statement()
        } else if self.match_keyword("match") {
            self.parse_match_statement()
        } else {
            self.parse_text()
        }
    }

    fn parse_element(&mut self) -> Result<TemplateAst> {
        if !self.match_char('<') {
            return Err(self.error("Expected '<' to start element"));
        }

        let tag = self.parse_identifier()?;
        let mut attributes = Vec::new();
        let mut self_closing = false;

        self.skip_whitespace();

        // Parse attributes
        while !self.check_char('>') && !self.check_char('/') && !self.is_at_end() {
            let attr = self.parse_attribute()?;
            attributes.push(attr);
            self.skip_whitespace();
        }

        // Check for self-closing
        if self.match_char('/') {
            self_closing = true;
            if !self.match_char('>') {
                return Err(self.error("Expected '>' after '/' in self-closing tag"));
            }
            return Ok(TemplateAst::Element {
                tag,
                attributes,
                children: Vec::new(),
                self_closing,
            });
        }

        if !self.match_char('>') {
            return Err(self.error("Expected '>' to close opening tag"));
        }

        // Parse children
        let mut children = Vec::new();
        while !self.check_closing_tag(&tag) && !self.is_at_end() {
            // If we hit a template-body close `}` before the closing tag, the
            // element is unclosed. Bail out so the caller raises the "Expected
            // closing tag" error below instead of spinning forever on an empty
            // text node.
            if self.check_char('}') {
                break;
            }
            let child = self.parse_template_node()?;
            children.push(child);
        }

        // Parse closing tag
        self.skip_whitespace();
        if !self.match_str(&format!("</{}>", tag)) {
            return Err(self.error(&format!("Expected closing tag '</{}>", tag)));
        }

        Ok(TemplateAst::Element {
            tag,
            attributes,
            children,
            self_closing,
        })
    }

    fn parse_attribute(&mut self) -> Result<Attribute> {
        let name = self.parse_attribute_name()?;

        // Check for conditional attribute (disabled?)
        let conditional = self.match_char('?');

        self.skip_whitespace();
        if !self.match_char('=') {
            // Boolean attribute
            return Ok(Attribute {
                name,
                value: AttributeValue::Static("true".to_string()),
            });
        }

        self.skip_whitespace();

        let value = if self.check_char('{') {
            self.advance(); // consume '{'
            let expr = self.parse_expression_until(&['}'])?;
            if !self.match_char('}') {
                return Err(self.error("Expected '}' to close attribute expression"));
            }

            if conditional {
                AttributeValue::Conditional(expr)
            } else {
                AttributeValue::Expression(expr)
            }
        } else {
            let value = self.parse_string_literal()?;
            AttributeValue::Static(value)
        };

        Ok(Attribute { name, value })
    }

    fn parse_expression_node(&mut self) -> Result<TemplateAst> {
        if !self.match_char('{') {
            return Err(self.error("Expected '{' to start expression"));
        }

        // `{!expr}` denotes a raw-HTML expression: its runtime value is
        // injected verbatim via `Html::raw(...)` instead of going through
        // `Html::text(...)` which would HTML-escape the output.
        let raw = self.match_char('!');

        let expr = self.parse_expression_until(&['}'])?;

        if !self.match_char('}') {
            return Err(self.error("Expected '}' to close expression"));
        }

        // `{children}` (not `{children.foo}` or `{my.children}`) is the
        // slot-placeholder form. Only recognise it when the bare identifier
        // `children` appears with no further path/call syntax.
        if !raw && expr.trim() == "children" {
            return Ok(TemplateAst::Children);
        }

        if raw {
            Ok(TemplateAst::RawExpression(expr))
        } else {
            Ok(TemplateAst::Expression(expr))
        }
    }

    fn parse_component_invocation(&mut self) -> Result<TemplateAst> {
        if !self.match_char('@') {
            return Err(self.error("Expected '@' to start component invocation"));
        }

        let name = self.parse_identifier()?;
        self.skip_whitespace();

        if !self.match_char('(') {
            return Err(self.error("Expected '(' after component name"));
        }

        let mut props = Vec::new();
        self.skip_whitespace();

        while !self.check_char(')') && !self.is_at_end() {
            let prop_name = self.parse_identifier()?;
            self.skip_whitespace();

            if !self.match_char(':') {
                return Err(self.error("Expected ':' after prop name"));
            }

            self.skip_whitespace();
            let value = self.parse_expression_until(&[',', ')'])?;

            props.push(PropValue {
                name: prop_name,
                value,
            });

            self.skip_whitespace();
            if self.match_char(',') {
                self.skip_whitespace();
            } else if !self.check_char(')') {
                return Err(self.error("Expected ',' or ')' in component props"));
            }
        }

        if !self.match_char(')') {
            return Err(self.error("Expected ')' to close component invocation"));
        }

        // Optional body block: `@Card(title: "x") { <p/>More }`. The body
        // becomes the callee's `children` prop.
        self.skip_whitespace();
        let children = if self.check_char('{') {
            self.advance(); // consume '{'
            let body = self.parse_template_body()?;
            if !self.match_char('}') {
                return Err(self.error("Expected '}' to close component body"));
            }
            Some(Box::new(body))
        } else {
            None
        };

        Ok(TemplateAst::Component {
            name,
            props,
            children,
        })
    }

    fn parse_if_statement(&mut self) -> Result<TemplateAst> {
        self.skip_whitespace();
        let condition = self.parse_expression_until(&['{'])?;

        self.skip_whitespace();
        if !self.match_char('{') {
            return Err(self.error("Expected '{' after if condition"));
        }

        let then_branch = Box::new(self.parse_template_body()?);

        if !self.match_char('}') {
            return Err(self.error("Expected '}' to close if block"));
        }

        self.skip_whitespace();
        let else_branch = if self.match_keyword("else") {
            self.skip_whitespace();
            if !self.match_char('{') {
                return Err(self.error("Expected '{' after else"));
            }
            let else_body = Box::new(self.parse_template_body()?);
            if !self.match_char('}') {
                return Err(self.error("Expected '}' to close else block"));
            }
            Some(else_body)
        } else {
            None
        };

        Ok(TemplateAst::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_for_statement(&mut self) -> Result<TemplateAst> {
        self.skip_whitespace();
        let variable = self.parse_for_binding()?;

        self.skip_whitespace();
        if !self.match_keyword("in") {
            return Err(self.error("Expected 'in' after for variable"));
        }

        self.skip_whitespace();
        let iterable = self.parse_expression_until(&['{'])?;

        self.skip_whitespace();
        if !self.match_char('{') {
            return Err(self.error("Expected '{' after for expression"));
        }

        let body = Box::new(self.parse_template_body()?);

        if !self.match_char('}') {
            return Err(self.error("Expected '}' to close for block"));
        }

        Ok(TemplateAst::For {
            variable,
            iterable,
            body,
        })
    }

    fn parse_match_statement(&mut self) -> Result<TemplateAst> {
        self.skip_whitespace();
        let expression = self.parse_expression_until(&['{'])?;

        self.skip_whitespace();
        if !self.match_char('{') {
            return Err(self.error("Expected '{' after match expression"));
        }

        let mut arms = Vec::new();
        self.skip_whitespace();

        while !self.check_char('}') && !self.is_at_end() {
            let pattern = self.parse_expression_until(&['='])?;

            if !self.match_str("=>") {
                return Err(self.error("Expected '=>' after match pattern"));
            }

            self.skip_whitespace();
            if !self.match_char('{') {
                return Err(self.error("Expected '{' after '=>'"));
            }

            let body = self.parse_template_body()?;

            if !self.match_char('}') {
                return Err(self.error("Expected '}' to close match arm"));
            }

            arms.push(MatchArm { pattern, body });
            self.skip_whitespace();
        }

        if !self.match_char('}') {
            return Err(self.error("Expected '}' to close match block"));
        }

        Ok(TemplateAst::Match { expression, arms })
    }

    fn parse_text(&mut self) -> Result<TemplateAst> {
        let mut text = String::new();

        while !self.is_at_end() {
            let ch = self.current_char();

            if ch == '<' || ch == '{' || ch == '@' || ch == '}' {
                break;
            }

            if self.at_keyword(&["if", "for", "match", "else"]) {
                break;
            }

            text.push(ch);
            self.advance();
        }

        if text.trim().is_empty() {
            text = text.trim().to_string();
        }

        Ok(TemplateAst::Text(text))
    }

    // Utility methods
    fn parse_identifier(&mut self) -> Result<String> {
        let mut identifier = String::new();

        if !self.current_char().is_ascii_alphabetic() && self.current_char() != '_' {
            return Err(self.error("Expected identifier"));
        }

        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_ascii_alphanumeric() || ch == '_' {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(identifier)
    }

    /// Parse a `for` loop binding. Accepts either a bare identifier (`item`)
    /// or a tuple destructure pattern (`(key, value)`). Returned verbatim so
    /// codegen can parse it as a `syn::Pat`.
    fn parse_for_binding(&mut self) -> Result<String> {
        if self.check_char('(') {
            let mut out = String::new();
            let mut depth = 0i32;
            while !self.is_at_end() {
                let ch = self.current_char();
                out.push(ch);
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        return Ok(out);
                    }
                    continue;
                }
                self.advance();
            }
            return Err(self.error("Unterminated tuple pattern in for binding"));
        }
        self.parse_identifier()
    }

    /// Parse an HTML/XML attribute name. Like `parse_identifier` but also
    /// allows `-` (e.g. `aria-hidden`) and `:` (e.g. `xmlns:xlink`).
    fn parse_attribute_name(&mut self) -> Result<String> {
        let mut name = String::new();

        if !self.current_char().is_ascii_alphabetic() && self.current_char() != '_' {
            return Err(self.error("Expected attribute name"));
        }

        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == ':' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(name)
    }

    fn parse_string_literal(&mut self) -> Result<String> {
        if !self.match_char('"') {
            return Err(self.error("Expected '\"' to start string literal"));
        }

        let mut string = String::new();

        while !self.is_at_end() && !self.check_char('"') {
            let ch = self.current_char();
            if ch == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(self.error("Unexpected end of input in string literal"));
                }
                let escaped = match self.current_char() {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '"' => '"',
                    c => c,
                };
                string.push(escaped);
            } else {
                string.push(ch);
            }
            self.advance();
        }

        if !self.match_char('"') {
            return Err(self.error("Expected '\"' to end string literal"));
        }

        Ok(string)
    }

    fn parse_type(&mut self) -> Result<String> {
        let mut type_str = String::new();
        let mut bracket_depth = 0;
        let mut angle_depth = 0;

        while !self.is_at_end() {
            let ch = self.current_char();

            match ch {
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                '<' => angle_depth += 1,
                '>' => angle_depth -= 1,
                ',' | '=' | '?' | ')' | '\n' if bracket_depth == 0 && angle_depth == 0 => break,
                '}' if bracket_depth == 0 && angle_depth == 0 => break,
                _ => {}
            }

            type_str.push(ch);
            self.advance();
        }

        Ok(type_str.trim().to_string())
    }

    fn parse_expression_until(&mut self, terminators: &[char]) -> Result<String> {
        let mut expr = String::new();
        let mut brace_depth = 0;
        let mut paren_depth = 0;
        let mut bracket_depth = 0;

        while !self.is_at_end() {
            let ch = self.current_char();

            match ch {
                '{' => {
                    if brace_depth == 0 && terminators.contains(&ch) {
                        break;
                    }
                    brace_depth += 1;
                }
                '}' => {
                    if brace_depth == 0 && terminators.contains(&ch) {
                        break;
                    }
                    brace_depth -= 1;
                }
                '(' => {
                    if paren_depth == 0 && terminators.contains(&ch) {
                        break;
                    }
                    paren_depth += 1;
                }
                ')' => {
                    if paren_depth == 0 && terminators.contains(&ch) {
                        break;
                    }
                    paren_depth -= 1;
                }
                '[' => {
                    if bracket_depth == 0 && terminators.contains(&ch) {
                        break;
                    }
                    bracket_depth += 1;
                }
                ']' => {
                    if bracket_depth == 0 && terminators.contains(&ch) {
                        break;
                    }
                    bracket_depth -= 1;
                }
                c if terminators.contains(&c)
                    && brace_depth == 0
                    && paren_depth == 0
                    && bracket_depth == 0 =>
                {
                    break;
                }
                _ => {}
            }

            expr.push(ch);
            self.advance();
        }

        Ok(expr.trim().to_string())
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && self.current_char().is_whitespace() {
            if self.current_char() == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += 1;
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            self.skip_whitespace();
            if self.match_str("//") {
                let mut text = String::new();
                while !self.is_at_end() && self.current_char() != '\n' {
                    text.push(self.current_char());
                    self.advance();
                }
                self.pending_comments.push(text.trim().to_string());
            } else if self.match_str("/*") {
                let mut text = String::new();
                while !self.is_at_end() && !self.match_str("*/") {
                    text.push(self.current_char());
                    self.advance();
                }
                self.pending_comments.push(text.trim().to_string());
            } else {
                break;
            }
        }
    }

    /// Drain any buffered comments. Called by each top-level parse_*
    /// so whatever the lexer has collected attaches to the next decl.
    fn take_pending_comments(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_comments)
    }

    fn current_char(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }

    fn peek_string(&self, len: usize) -> String {
        if self.position + len > self.input.len() {
            return String::new();
        }
        self.input.iter().skip(self.position).take(len).collect()
    }

    fn parse_doctype(&mut self) -> Result<TemplateAst> {
        // Consume the entire DOCTYPE declaration
        let start_pos = self.position;

        // Move past '<'
        self.advance();

        // Read until we find the closing '>'
        while !self.is_at_end() && self.current_char() != '>' {
            self.advance();
        }

        if self.is_at_end() {
            return Err(self.error("Unterminated DOCTYPE declaration"));
        }

        // Consume the closing '>'
        self.advance();

        // Extract the full DOCTYPE text
        let end_pos = self.position;
        let doctype_text: String = self
            .input
            .iter()
            .skip(start_pos)
            .take(end_pos - start_pos)
            .collect();

        Ok(TemplateAst::Text(doctype_text))
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            if self.current_char() == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    fn check_char(&self, expected: char) -> bool {
        !self.is_at_end() && self.current_char() == expected
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.check_char(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_str(&mut self, expected: &str) -> bool {
        let expected_chars: Vec<char> = expected.chars().collect();

        if self.position + expected_chars.len() > self.input.len() {
            return false;
        }

        for (i, &expected_char) in expected_chars.iter().enumerate() {
            if self.input[self.position + i] != expected_char {
                return false;
            }
        }

        for _ in 0..expected_chars.len() {
            self.advance();
        }
        true
    }

    fn match_keyword(&mut self, keyword: &str) -> bool {
        let start_pos = self.position;
        let start_line = self.line;
        let start_column = self.column;

        if self.match_str(keyword) {
            // Check that it's not part of a larger identifier
            if self.is_at_end()
                || !self.current_char().is_ascii_alphanumeric() && self.current_char() != '_'
            {
                return true;
            }
        }

        // Restore position if not a complete keyword
        self.position = start_pos;
        self.line = start_line;
        self.column = start_column;
        false
    }

    /// Return the position after skipping any run of whitespace starting at
    /// `self.position`, without mutating the parser state.
    fn cursor_after_whitespace(&self) -> usize {
        let mut i = self.position;
        while i < self.input.len() && self.input[i].is_whitespace() {
            i += 1;
        }
        i
    }

    /// Like `at_keyword`, but checks at an arbitrary position `pos` instead of
    /// `self.position`. Used for lookahead.
    fn at_keyword_at(&self, pos: usize, keywords: &[&str]) -> bool {
        for &keyword in keywords {
            let kw: Vec<char> = keyword.chars().collect();
            if pos + kw.len() <= self.input.len() {
                let mut matches = true;
                for (i, &ch) in kw.iter().enumerate() {
                    if self.input[pos + i] != ch {
                        matches = false;
                        break;
                    }
                }
                if matches {
                    let after = pos + kw.len();
                    if after >= self.input.len()
                        || !(self.input[after].is_ascii_alphanumeric() || self.input[after] == '_')
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn at_keyword(&self, keywords: &[&str]) -> bool {
        for &keyword in keywords {
            let keyword_chars: Vec<char> = keyword.chars().collect();

            if self.position + keyword_chars.len() <= self.input.len() {
                let mut matches = true;
                for (i, &expected_char) in keyword_chars.iter().enumerate() {
                    if self.input[self.position + i] != expected_char {
                        matches = false;
                        break;
                    }
                }

                if matches {
                    // Check that it's not part of a larger identifier
                    let next_pos = self.position + keyword_chars.len();
                    if next_pos >= self.input.len()
                        || (!self.input[next_pos].is_ascii_alphanumeric()
                            && self.input[next_pos] != '_')
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn check_closing_tag(&self, tag: &str) -> bool {
        let closing_tag = format!("</{}>", tag);
        let closing_chars: Vec<char> = closing_tag.chars().collect();

        // Skip whitespace to find the closing tag
        let mut pos = self.position;
        while pos < self.input.len() && self.input[pos].is_whitespace() {
            pos += 1;
        }

        if pos + closing_chars.len() > self.input.len() {
            return false;
        }

        for (i, &expected_char) in closing_chars.iter().enumerate() {
            if self.input[pos + i] != expected_char {
                return false;
            }
        }

        true
    }

    fn error(&self, message: &str) -> CompileError {
        CompileError::parse(self.format_error(message))
    }

    /// Build a rustc-style framed error message. Shows the offending line
    /// (plus up to two lines of leading context) with a caret under the
    /// offending column.
    fn format_error(&self, message: &str) -> String {
        let source: String = self.input.iter().collect();
        let lines: Vec<&str> = source.lines().collect();
        // `self.line` is 1-indexed; clamp so out-of-range errors don't panic.
        let err_line_idx = (self.line.saturating_sub(1)).min(lines.len().saturating_sub(1));
        let start_line_idx = err_line_idx.saturating_sub(2);

        let mut out = String::new();
        out.push_str(&format!(
            "{} at line {}, column {}\n",
            message, self.line, self.column
        ));

        // Width of the line-number gutter (at least 2 chars for aesthetics).
        let gutter = std::cmp::max(2, (err_line_idx + 1).to_string().len());

        out.push_str(&format!("{:>width$} |\n", "", width = gutter));
        for (offset, line) in lines[start_line_idx..=err_line_idx].iter().enumerate() {
            let lineno = start_line_idx + offset + 1;
            out.push_str(&format!("{:>width$} | {}\n", lineno, line, width = gutter));
        }
        // Caret under the offending column. column is 1-indexed.
        let caret_pad = self.column.saturating_sub(1);
        out.push_str(&format!(
            "{:>width$} | {}^ {}\n",
            "",
            " ".repeat(caret_pad),
            message,
            width = gutter
        ));

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_component_generics() {
        let input = r#"
component Box<T> {
    props {
        value: T,
    }
}
"#;
        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse().unwrap();
        let component = &result.components[0];
        assert_eq!(component.generics.len(), 1);
        assert_eq!(component.generics[0].name, "T");
        assert!(component.generics[0].bounds.is_empty());
    }

    #[test]
    fn test_parse_component_generics_with_bounds() {
        let input = r#"
component List<T: Clone + Display, U> {
    props {
        items: Vec<T>,
    }
}
"#;
        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse().unwrap();
        let component = &result.components[0];
        assert_eq!(component.generics.len(), 2);
        assert_eq!(component.generics[0].name, "T");
        assert_eq!(component.generics[0].bounds, vec!["Clone", "Display"]);
        assert_eq!(component.generics[1].name, "U");
        assert!(component.generics[1].bounds.is_empty());
    }

    #[test]
    fn test_parse_template_generics() {
        let input = r#"
component Box<T> {
    props {
        value: T,
    }
}

ruitl Box<T>(value: T) {
    <div>{value}</div>
}
"#;
        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse().unwrap();
        assert_eq!(result.templates.len(), 1);
        assert_eq!(result.templates[0].generics.len(), 1);
        assert_eq!(result.templates[0].generics[0].name, "T");
    }

    #[test]
    fn test_parse_identifier() {
        let mut parser = RuitlParser::new("hello_world".to_string());
        let result = parser.parse_identifier().unwrap();
        assert_eq!(result, "hello_world");
    }

    #[test]
    fn test_parse_string_literal() {
        let mut parser = RuitlParser::new("\"hello world\"".to_string());
        let result = parser.parse_string_literal().unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_parse_simple_component() {
        let input = r#"
component Button {
    props {
        text: String,
        disabled: bool = false,
    }
}
        "#;

        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse().unwrap();

        assert_eq!(result.components.len(), 1);
        let component = &result.components[0];
        assert_eq!(component.name, "Button");
        assert_eq!(component.props.len(), 2);

        assert_eq!(component.props[0].name, "text");
        assert_eq!(component.props[0].prop_type, "String");
        assert!(!component.props[0].optional);

        assert_eq!(component.props[1].name, "disabled");
        assert_eq!(component.props[1].prop_type, "bool");
        // A prop with a default value is not Option-wrapped; `optional`
        // tracks explicit `?` markers only.
        assert!(!component.props[1].optional);
        assert_eq!(component.props[1].default_value, Some("false".to_string()));
    }

    #[test]
    fn test_parse_simple_template() {
        let input = r#"
ruitl Greeting(name: String) {
    <div class="greeting">
        <h1>Hello, {name}!</h1>
    </div>
}
        "#;

        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse().unwrap();

        assert_eq!(result.templates.len(), 1);
        let template = &result.templates[0];
        assert_eq!(template.name, "Greeting");
        assert_eq!(template.params.len(), 1);
        assert_eq!(template.params[0].name, "name");
        assert_eq!(template.params[0].param_type, "String");
    }

    #[test]
    fn test_parse_import() {
        let input = r#"import "std::collections" { HashMap, Vec }"#;

        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse().unwrap();

        assert_eq!(result.imports.len(), 1);
        let import = &result.imports[0];
        assert_eq!(import.path, "std::collections");
        assert_eq!(import.items, vec!["HashMap", "Vec"]);
    }

    #[test]
    fn test_parse_element_with_attributes() {
        let input = r#"<button class="btn" disabled?={is_disabled}>Click me</button>"#;

        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse_element().unwrap();

        if let TemplateAst::Element {
            tag,
            attributes,
            children,
            ..
        } = result
        {
            assert_eq!(tag, "button");
            assert_eq!(attributes.len(), 2);

            assert_eq!(attributes[0].name, "class");
            if let AttributeValue::Static(value) = &attributes[0].value {
                assert_eq!(value, "btn");
            } else {
                panic!("Expected static attribute value");
            }

            assert_eq!(attributes[1].name, "disabled");
            if let AttributeValue::Conditional(expr) = &attributes[1].value {
                assert_eq!(expr, "is_disabled");
            } else {
                panic!("Expected conditional attribute value");
            }

            assert_eq!(children.len(), 1);
            if let TemplateAst::Text(text) = &children[0] {
                assert_eq!(text, "Click me");
            } else {
                panic!("Expected text child");
            }
        } else {
            panic!("Expected element AST node");
        }
    }

    #[test]
    fn test_parse_expression() {
        let input = r#"{user.name.to_uppercase()}"#;

        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse_expression_node().unwrap();

        if let TemplateAst::Expression(expr) = result {
            assert_eq!(expr, "user.name.to_uppercase()");
        } else {
            panic!("Expected expression AST node");
        }
    }

    #[test]
    fn test_parse_component_invocation() {
        let input = r#"@Button(text: "Click me", disabled: false)"#;

        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse_component_invocation().unwrap();

        if let TemplateAst::Component {
            name,
            props,
            children,
        } = result
        {
            assert_eq!(name, "Button");
            assert_eq!(props.len(), 2);
            assert!(children.is_none(), "no body block → children is None");

            assert_eq!(props[0].name, "text");
            assert_eq!(props[0].value, "\"Click me\"");

            assert_eq!(props[1].name, "disabled");
            assert_eq!(props[1].value, "false");
        } else {
            panic!("Expected component AST node");
        }
    }

    #[test]
    fn test_parse_component_with_body() {
        let input = r#"@Card(title: "Hi") { <p>Body</p> }"#;
        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse_component_invocation().unwrap();

        let TemplateAst::Component {
            name,
            props,
            children,
        } = result
        else {
            panic!("expected Component")
        };
        assert_eq!(name, "Card");
        assert_eq!(props.len(), 1);
        let body = children.expect("body block must be captured as children");
        // The body should contain an element `<p>Body</p>`.
        let children_vec = match *body {
            TemplateAst::Fragment(v) => v,
            other => vec![other],
        };
        let has_p = children_vec.iter().any(|n| matches!(n, TemplateAst::Element { tag, .. } if tag == "p"));
        assert!(has_p, "body must contain <p> element");
    }

    #[test]
    fn test_children_keyword_node() {
        let input = "{children}";
        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse_expression_node().unwrap();
        assert!(
            matches!(result, TemplateAst::Children),
            "bare `{{children}}` must emit TemplateAst::Children, got {:?}",
            result
        );
    }

    #[test]
    fn test_dotted_children_is_expression_not_slot() {
        let input = "{my.children}";
        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse_expression_node().unwrap();
        // Dotted `children` is a regular field access — NOT the slot form.
        assert!(
            matches!(result, TemplateAst::Expression(ref s) if s == "my.children"),
            "`{{my.children}}` must parse as Expression, got {:?}",
            result
        );
    }

    #[test]
    fn test_parse_if_statement() {
        let input = r#"if show_message { <p>Hello!</p> } else { <p>Goodbye!</p> }"#;

        let mut parser = RuitlParser::new(input.to_string());
        parser.match_keyword("if"); // Consume the "if" keyword first
        let result = parser.parse_if_statement().unwrap();

        if let TemplateAst::If {
            condition,
            then_branch,
            else_branch,
        } = result
        {
            assert_eq!(condition, "show_message");
            assert!(then_branch.as_ref().is_element_with_tag("p"));
            assert!(else_branch.is_some());
            assert!(else_branch.unwrap().as_ref().is_element_with_tag("p"));
        } else {
            panic!("Expected if AST node");
        }
    }

    #[test]
    fn test_parse_for_statement() {
        let input = r#"for item in items { <li>{item}</li> }"#;

        let mut parser = RuitlParser::new(input.to_string());
        parser.match_keyword("for"); // Consume the "for" keyword first
        let result = parser.parse_for_statement().unwrap();

        if let TemplateAst::For {
            variable,
            iterable,
            body,
        } = result
        {
            assert_eq!(variable, "item");
            assert_eq!(iterable, "items");
            assert!(body.as_ref().is_element_with_tag("li"));
        } else {
            panic!("Expected for AST node");
        }
    }

    #[test]
    fn test_parse_self_closing_element() {
        let input = r#"<img src="photo.jpg" alt="Photo" />"#;

        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse_element().unwrap();

        if let TemplateAst::Element {
            tag,
            attributes,
            children,
            self_closing,
        } = result
        {
            assert_eq!(tag, "img");
            assert!(self_closing);
            assert!(children.is_empty());
            assert_eq!(attributes.len(), 2);
        } else {
            panic!("Expected element AST node");
        }
    }

    #[test]
    fn test_parse_complex_template() {
        let input = r#"
import "std::collections" { HashMap }

component UserCard {
    props {
        user: User,
        show_email: bool = true,
    }
}

ruitl UserCard(props: UserCardProps) {
    <div class="user-card">
        <h2>{props.user.name}</h2>
        if props.show_email {
            <p class="email">{props.user.email}</p>
        }
        <ul>
            for skill in props.user.skills {
                <li>{skill}</li>
            }
        </ul>
    </div>
}
        "#;

        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse().unwrap();

        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.components.len(), 1);
        assert_eq!(result.templates.len(), 1);

        let component = &result.components[0];
        assert_eq!(component.name, "UserCard");
        assert_eq!(component.props.len(), 2);

        let template = &result.templates[0];
        assert_eq!(template.name, "UserCard");
        assert_eq!(template.params.len(), 1);
    }

    #[test]
    fn test_parse_error_handling() {
        let input = r#"component Button { props { text String } }"#; // Missing colon

        let mut parser = RuitlParser::new(input.to_string());
        let result = parser.parse();

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_nested_elements() {
        let input = r#"
<div class="container">
    <header>
        <h1>Title</h1>
        <nav>
            <a href="/">Home</a>
            <a href="/about">About</a>
        </nav>
    </header>
    <main>
        <p>Content goes here</p>
    </main>
</div>
        "#;

        let mut parser = RuitlParser::new(input.to_string());
        parser.skip_whitespace(); // Skip leading whitespace
        let result = parser.parse_element().unwrap();

        if let TemplateAst::Element { tag, children, .. } = result {
            assert_eq!(tag, "div");
            assert_eq!(children.len(), 2); // header and main (text nodes are trimmed)
        } else {
            panic!("Expected element AST node");
        }
    }

    // Helper trait for tests
    trait TestAstHelper {
        fn is_element_with_tag(&self, expected_tag: &str) -> bool;
    }

    impl TestAstHelper for TemplateAst {
        fn is_element_with_tag(&self, tag: &str) -> bool {
            match self {
                TemplateAst::Element {
                    tag: element_tag, ..
                } => element_tag == tag,
                _ => false,
            }
        }
    }
}
