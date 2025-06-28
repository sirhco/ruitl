//! RUITL Template Parser
//!
//! Parses .ruitl files and converts them to an AST that can be compiled to Rust code

use crate::error::{Result, RuitlError};
use std::collections::HashMap;
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
    pub generics: Vec<String>,
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
    /// Component invocation: @Button(props)
    Component { name: String, props: Vec<PropValue> },
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

        Ok(ImportDef { path, items })
    }

    fn parse_component(&mut self) -> Result<ComponentDef> {
        self.skip_whitespace();
        let name = self.parse_identifier()?;

        self.skip_whitespace();
        if !self.match_char('{') {
            return Err(self.error("Expected '{' after component name"));
        }

        self.skip_whitespace_and_comments();

        let mut props = Vec::new();
        let generics = Vec::new(); // TODO: implement generics parsing

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
        self.skip_whitespace();
        let name = self.parse_identifier()?;

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

        Ok(TemplateDef { name, params, body })
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
        self.skip_whitespace();

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
        let name = self.parse_identifier()?;

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

        let expr = self.parse_expression_until(&['}'])?;

        if !self.match_char('}') {
            return Err(self.error("Expected '}' to close expression"));
        }

        Ok(TemplateAst::Expression(expr))
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

        Ok(TemplateAst::Component { name, props })
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
        let variable = self.parse_identifier()?;

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
                while !self.is_at_end() && self.current_char() != '\n' {
                    self.advance();
                }
            } else if self.match_str("/*") {
                while !self.is_at_end() && !self.match_str("*/") {
                    self.advance();
                }
            } else {
                break;
            }
        }
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

    fn peek_char(&self) -> char {
        if self.position + 1 >= self.input.len() {
            '\0'
        } else {
            self.input[self.position + 1]
        }
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

    fn error(&self, message: &str) -> RuitlError {
        RuitlError::parse(format!(
            "{} at line {}, column {}",
            message, self.line, self.column
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(component.props[1].optional);
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

        if let TemplateAst::Component { name, props } = result {
            assert_eq!(name, "Button");
            assert_eq!(props.len(), 2);

            assert_eq!(props[0].name, "text");
            assert_eq!(props[0].value, "\"Click me\"");

            assert_eq!(props[1].name, "disabled");
            assert_eq!(props[1].value, "false");
        } else {
            panic!("Expected component AST node");
        }
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
