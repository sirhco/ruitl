//! Template system for RUITL
//!
//! This module provides template parsing, compilation, and rendering capabilities.
//! Templates can contain HTML, components, and Rust expressions.

use crate::component::{Component, ComponentContext, ComponentProps};
use crate::error::{Result, RuitlError};
use crate::html::{Html, HtmlElement};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use syn::{parse_str, Expr, Ident, LitStr};

/// Template parsing and compilation engine
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    templates: HashMap<String, Template>,
    components: HashMap<String, String>,
    globals: HashMap<String, TemplateValue>,
}

/// Represents a parsed template
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub content: String,
    pub ast: TemplateAst,
    pub dependencies: Vec<String>,
}

/// Template AST (Abstract Syntax Tree)
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateAst {
    /// Raw HTML content
    Html(String),
    /// Text content (will be escaped)
    Text(String),
    /// Rust expression to be evaluated
    Expression(String),
    /// Template variable reference
    Variable(String),
    /// Component invocation
    Component {
        name: String,
        props: HashMap<String, TemplateValue>,
        children: Vec<TemplateAst>,
    },
    /// Conditional block
    If {
        condition: String,
        then_branch: Vec<TemplateAst>,
        else_branch: Option<Vec<TemplateAst>>,
    },
    /// Loop block
    For {
        variable: String,
        iterable: String,
        body: Vec<TemplateAst>,
    },
    /// Match block
    Match {
        expression: String,
        arms: Vec<MatchArm>,
    },
    /// Template fragment (multiple nodes)
    Fragment(Vec<TemplateAst>),
    /// Include another template
    Include {
        template: String,
        context: HashMap<String, TemplateValue>,
    },
    /// Block definition (for template inheritance)
    Block {
        name: String,
        content: Vec<TemplateAst>,
    },
    /// Extend another template
    Extend {
        template: String,
        blocks: HashMap<String, Vec<TemplateAst>>,
    },
}

/// Match arm for match expressions
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: String,
    pub body: Vec<TemplateAst>,
}

/// Template values that can be passed as props or variables
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<TemplateValue>),
    Object(HashMap<String, TemplateValue>),
    Null,
}

/// Template compilation context
#[derive(Debug, Default)]
pub struct CompileContext {
    pub variables: HashMap<String, String>,
    pub imports: Vec<String>,
    pub component_types: HashMap<String, String>,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            components: HashMap::new(),
            globals: HashMap::new(),
        }
    }

    /// Register a template
    pub fn register_template(&mut self, name: &str, content: &str) -> Result<()> {
        let template = self.parse_template(name, content)?;
        self.templates.insert(name.to_string(), template);
        Ok(())
    }

    /// Register a component type
    pub fn register_component(&mut self, name: &str, type_name: &str) {
        self.components
            .insert(name.to_string(), type_name.to_string());
    }

    /// Set a global variable
    pub fn set_global(&mut self, name: &str, value: TemplateValue) {
        self.globals.insert(name.to_string(), value);
    }

    /// Parse a template from source code
    pub fn parse_template(&self, name: &str, content: &str) -> Result<Template> {
        let ast = self.parse_content(content)?;
        let dependencies = self.extract_dependencies(&ast);

        Ok(Template {
            name: name.to_string(),
            content: content.to_string(),
            ast,
            dependencies,
        })
    }

    /// Parse template content into AST
    fn parse_content(&self, content: &str) -> Result<TemplateAst> {
        let mut parser = TemplateParser::new(content);
        parser.parse()
    }

    /// Extract template dependencies
    fn extract_dependencies(&self, ast: &TemplateAst) -> Vec<String> {
        let mut deps = Vec::new();
        self.collect_dependencies(ast, &mut deps);
        deps.sort();
        deps.dedup();
        deps
    }

    /// Recursively collect dependencies
    fn collect_dependencies(&self, ast: &TemplateAst, deps: &mut Vec<String>) {
        match ast {
            TemplateAst::Component { name, children, .. } => {
                deps.push(name.clone());
                for child in children {
                    self.collect_dependencies(child, deps);
                }
            }
            TemplateAst::Include { template, .. } => {
                deps.push(template.clone());
            }
            TemplateAst::Extend { template, blocks } => {
                deps.push(template.clone());
                for block_content in blocks.values() {
                    for node in block_content {
                        self.collect_dependencies(node, deps);
                    }
                }
            }
            TemplateAst::Fragment(children)
            | TemplateAst::If {
                then_branch: children,
                ..
            }
            | TemplateAst::For { body: children, .. }
            | TemplateAst::Block {
                content: children, ..
            } => {
                for child in children {
                    self.collect_dependencies(child, deps);
                }
            }
            TemplateAst::If {
                else_branch: Some(children),
                ..
            } => {
                for child in children {
                    self.collect_dependencies(child, deps);
                }
            }
            TemplateAst::Match { arms, .. } => {
                for arm in arms {
                    for node in &arm.body {
                        self.collect_dependencies(node, deps);
                    }
                }
            }
            _ => {}
        }
    }

    /// Compile a template to Rust code
    pub fn compile_template(&self, name: &str) -> Result<TokenStream> {
        let template = self
            .templates
            .get(name)
            .ok_or_else(|| RuitlError::template(format!("Template '{}' not found", name)))?;

        let mut context = CompileContext::default();
        self.compile_ast(&template.ast, &mut context)
    }

    /// Compile AST to Rust code
    fn compile_ast(&self, ast: &TemplateAst, context: &mut CompileContext) -> Result<TokenStream> {
        match ast {
            TemplateAst::Html(content) => {
                let content = content.as_str();
                Ok(quote! { Html::raw(#content) })
            }
            TemplateAst::Text(content) => {
                let content = content.as_str();
                Ok(quote! { Html::text(#content) })
            }
            TemplateAst::Expression(expr) => {
                let expr: Expr = parse_str(expr).map_err(|e| {
                    RuitlError::template(format!("Invalid expression '{}': {}", expr, e))
                })?;
                Ok(quote! { Html::text(format!("{}", #expr)) })
            }
            TemplateAst::Variable(var) => {
                let var_ident: Ident = parse_str(var).map_err(|e| {
                    RuitlError::template(format!("Invalid variable '{}': {}", var, e))
                })?;
                Ok(quote! { Html::text(format!("{}", #var_ident)) })
            }
            TemplateAst::Component {
                name,
                props,
                children,
            } => self.compile_component(name, props, children, context),
            TemplateAst::If {
                condition,
                then_branch,
                else_branch,
            } => self.compile_if(condition, then_branch, else_branch.as_ref(), context),
            TemplateAst::For {
                variable,
                iterable,
                body,
            } => self.compile_for(variable, iterable, body, context),
            TemplateAst::Match { expression, arms } => {
                self.compile_match(expression, arms, context)
            }
            TemplateAst::Fragment(children) => self.compile_fragment(children, context),
            TemplateAst::Include {
                template,
                context: ctx,
            } => self.compile_include(template, ctx, context),
            TemplateAst::Block { content, .. } => self.compile_fragment(content, context),
            TemplateAst::Extend { .. } => Err(RuitlError::template(
                "Template extension not supported in compilation",
            )),
        }
    }

    /// Compile component invocation
    fn compile_component(
        &self,
        name: &str,
        props: &HashMap<String, TemplateValue>,
        children: &[TemplateAst],
        context: &mut CompileContext,
    ) -> Result<TokenStream> {
        let component_type = self
            .components
            .get(name)
            .ok_or_else(|| RuitlError::template(format!("Unknown component '{}'", name)))?;

        let component_ident: Ident = parse_str(component_type).map_err(|e| {
            RuitlError::template(format!(
                "Invalid component type '{}': {}",
                component_type, e
            ))
        })?;

        // Compile props
        let mut prop_tokens = Vec::new();
        for (key, value) in props {
            let key_ident: Ident = parse_str(key)
                .map_err(|e| RuitlError::template(format!("Invalid prop name '{}': {}", key, e)))?;
            let value_token = self.compile_template_value(value)?;
            prop_tokens.push(quote! { #key_ident: #value_token });
        }

        // Compile children if any
        let children_tokens = if !children.is_empty() {
            let child_tokens: Result<Vec<_>> = children
                .iter()
                .map(|child| self.compile_ast(child, context))
                .collect();
            let child_tokens = child_tokens?;
            Some(quote! { vec![#(#child_tokens),*] })
        } else {
            None
        };

        if let Some(children_token) = children_tokens {
            Ok(quote! {
                {
                    let component = #component_ident;
                    let props = ComponentProps {
                        #(#prop_tokens),*,
                        children: #children_token,
                    };
                    component.render(&props, &context)?
                }
            })
        } else {
            Ok(quote! {
                {
                    let component = #component_ident;
                    let props = ComponentProps {
                        #(#prop_tokens),*
                    };
                    component.render(&props, &context)?
                }
            })
        }
    }

    /// Compile if statement
    fn compile_if(
        &self,
        condition: &str,
        then_branch: &[TemplateAst],
        else_branch: Option<&Vec<TemplateAst>>,
        context: &mut CompileContext,
    ) -> Result<TokenStream> {
        let condition_expr: Expr = parse_str(condition).map_err(|e| {
            RuitlError::template(format!("Invalid condition '{}': {}", condition, e))
        })?;

        let then_tokens: Result<Vec<_>> = then_branch
            .iter()
            .map(|node| self.compile_ast(node, context))
            .collect();
        let then_tokens = then_tokens?;

        if let Some(else_nodes) = else_branch {
            let else_tokens: Result<Vec<_>> = else_nodes
                .iter()
                .map(|node| self.compile_ast(node, context))
                .collect();
            let else_tokens = else_tokens?;

            Ok(quote! {
                if #condition_expr {
                    Html::fragment(vec![#(#then_tokens),*])
                } else {
                    Html::fragment(vec![#(#else_tokens),*])
                }
            })
        } else {
            Ok(quote! {
                if #condition_expr {
                    Html::fragment(vec![#(#then_tokens),*])
                } else {
                    Html::empty()
                }
            })
        }
    }

    /// Compile for loop
    fn compile_for(
        &self,
        variable: &str,
        iterable: &str,
        body: &[TemplateAst],
        context: &mut CompileContext,
    ) -> Result<TokenStream> {
        let var_ident: Ident = parse_str(variable)
            .map_err(|e| RuitlError::template(format!("Invalid variable '{}': {}", variable, e)))?;
        let iter_expr: Expr = parse_str(iterable)
            .map_err(|e| RuitlError::template(format!("Invalid iterable '{}': {}", iterable, e)))?;

        let body_tokens: Result<Vec<_>> = body
            .iter()
            .map(|node| self.compile_ast(node, context))
            .collect();
        let body_tokens = body_tokens?;

        Ok(quote! {
            {
                let mut items = Vec::new();
                for #var_ident in #iter_expr {
                    items.extend(vec![#(#body_tokens),*]);
                }
                Html::fragment(items)
            }
        })
    }

    /// Compile match expression
    fn compile_match(
        &self,
        expression: &str,
        arms: &[MatchArm],
        context: &mut CompileContext,
    ) -> Result<TokenStream> {
        let match_expr: Expr = parse_str(expression).map_err(|e| {
            RuitlError::template(format!("Invalid match expression '{}': {}", expression, e))
        })?;

        let mut arm_tokens = Vec::new();
        for arm in arms {
            let pattern_str = &arm.pattern;

            let body_tokens: Result<Vec<_>> = arm
                .body
                .iter()
                .map(|node| self.compile_ast(node, context))
                .collect();
            let body_tokens = body_tokens?;

            let pattern_tokens: TokenStream = pattern_str.parse().map_err(|e| {
                RuitlError::template(format!("Invalid pattern syntax '{}': {}", pattern_str, e))
            })?;

            arm_tokens.push(quote! {
                #pattern_tokens => Html::fragment(vec![#(#body_tokens),*])
            });
        }

        Ok(quote! {
            match #match_expr {
                #(#arm_tokens),*
            }
        })
    }

    /// Compile fragment
    fn compile_fragment(
        &self,
        children: &[TemplateAst],
        context: &mut CompileContext,
    ) -> Result<TokenStream> {
        let child_tokens: Result<Vec<_>> = children
            .iter()
            .map(|child| self.compile_ast(child, context))
            .collect();
        let child_tokens = child_tokens?;

        Ok(quote! { Html::fragment(vec![#(#child_tokens),*]) })
    }

    /// Compile include
    fn compile_include(
        &self,
        template: &str,
        ctx: &HashMap<String, TemplateValue>,
        context: &mut CompileContext,
    ) -> Result<TokenStream> {
        let template_name = template;
        let template_ident: Ident =
            parse_str(&format!("render_{}", template_name.replace('-', "_"))).map_err(|e| {
                RuitlError::template(format!("Invalid template name '{}': {}", template_name, e))
            })?;

        // Compile context
        let mut ctx_tokens = Vec::new();
        for (key, value) in ctx {
            let key_str = key.as_str();
            let value_token = self.compile_template_value(value)?;
            ctx_tokens.push(quote! { context.insert(#key_str.to_string(), #value_token); });
        }

        Ok(quote! {
            {
                let mut context = context.clone();
                #(#ctx_tokens)*
                #template_ident(&context)?
            }
        })
    }

    /// Compile template value
    fn compile_template_value(&self, value: &TemplateValue) -> Result<TokenStream> {
        match value {
            TemplateValue::String(s) => Ok(quote! { #s.to_string() }),
            TemplateValue::Number(n) => Ok(quote! { #n }),
            TemplateValue::Boolean(b) => Ok(quote! { #b }),
            TemplateValue::Array(arr) => {
                let item_tokens: Result<Vec<_>> = arr
                    .iter()
                    .map(|item| self.compile_template_value(item))
                    .collect();
                let item_tokens = item_tokens?;
                Ok(quote! { vec![#(#item_tokens),*] })
            }
            TemplateValue::Object(obj) => {
                let mut field_tokens = Vec::new();
                for (key, value) in obj {
                    let key_str = key.as_str();
                    let value_token = self.compile_template_value(value)?;
                    field_tokens.push(quote! { map.insert(#key_str.to_string(), #value_token); });
                }
                Ok(quote! {
                    {
                        let mut map = std::collections::HashMap::new();
                        #(#field_tokens)*
                        map
                    }
                })
            }
            TemplateValue::Null => Ok(quote! { () }),
        }
    }

    /// Render a template with context
    pub fn render(&self, name: &str, context: &ComponentContext) -> Result<Html> {
        let template = self
            .templates
            .get(name)
            .ok_or_else(|| RuitlError::template(format!("Template '{}' not found", name)))?;

        self.render_ast(&template.ast, context)
    }

    /// Render AST with context
    fn render_ast(&self, ast: &TemplateAst, context: &ComponentContext) -> Result<Html> {
        match ast {
            TemplateAst::Html(content) => Ok(Html::raw(content)),
            TemplateAst::Text(content) => Ok(Html::text(content)),
            TemplateAst::Fragment(children) => {
                let mut result = Vec::new();
                for child in children {
                    result.push(self.render_ast(child, context)?);
                }
                Ok(Html::fragment(result))
            }
            _ => Err(RuitlError::template(
                "Runtime template rendering not fully implemented",
            )),
        }
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Template parser
struct TemplateParser {
    content: String,
    position: usize,
}

impl TemplateParser {
    fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            position: 0,
        }
    }

    fn parse(&mut self) -> Result<TemplateAst> {
        let mut nodes = Vec::new();

        while !self.is_at_end() {
            if self.current_char() == '{' && self.peek_char() == '{' {
                nodes.push(self.parse_expression()?);
            } else if self.current_char() == '<' && self.peek_char() == '%' {
                nodes.push(self.parse_directive()?);
            } else {
                nodes.push(self.parse_text()?);
            }
        }

        if nodes.len() == 1 {
            Ok(nodes.into_iter().next().unwrap())
        } else {
            Ok(TemplateAst::Fragment(nodes))
        }
    }

    fn parse_expression(&mut self) -> Result<TemplateAst> {
        self.consume_string("{{")?;
        let expr = self.consume_until("}}")?;
        self.consume_string("}}")?;

        Ok(TemplateAst::Expression(expr.trim().to_string()))
    }

    fn parse_directive(&mut self) -> Result<TemplateAst> {
        self.consume_string("<%")?;
        let directive = self.consume_until("%>")?;
        self.consume_string("%>")?;

        // Parse different directive types
        let directive = directive.trim();
        if directive.starts_with("if ") {
            self.parse_if_directive(&directive[3..])
        } else if directive.starts_with("for ") {
            self.parse_for_directive(&directive[4..])
        } else if directive.starts_with("component ") {
            self.parse_component_directive(&directive[10..])
        } else {
            Err(RuitlError::template(format!(
                "Unknown directive: {}",
                directive
            )))
        }
    }

    fn parse_if_directive(&mut self, condition: &str) -> Result<TemplateAst> {
        let then_branch = self.parse_until_directive("else", "endif")?;

        let else_branch = if self.last_directive_was("else") {
            Some(self.parse_until_directive("endif", "")?)
        } else {
            None
        };

        Ok(TemplateAst::If {
            condition: condition.to_string(),
            then_branch,
            else_branch,
        })
    }

    fn parse_for_directive(&mut self, for_expr: &str) -> Result<TemplateAst> {
        // Parse "variable in iterable"
        let parts: Vec<&str> = for_expr.split(" in ").collect();
        if parts.len() != 2 {
            return Err(RuitlError::template("Invalid for directive syntax"));
        }

        let variable = parts[0].trim().to_string();
        let iterable = parts[1].trim().to_string();
        let body = self.parse_until_directive("endfor", "")?;

        Ok(TemplateAst::For {
            variable,
            iterable,
            body,
        })
    }

    fn parse_component_directive(&mut self, component_expr: &str) -> Result<TemplateAst> {
        // Simple component parsing - could be more sophisticated
        let name = component_expr.trim().to_string();
        Ok(TemplateAst::Component {
            name,
            props: HashMap::new(),
            children: Vec::new(),
        })
    }

    fn parse_text(&mut self) -> Result<TemplateAst> {
        let mut text = String::new();
        while !self.is_at_end() && !self.is_directive_start() {
            text.push(self.current_char());
            self.advance();
        }
        Ok(TemplateAst::Text(text))
    }

    fn parse_until_directive(&mut self, end1: &str, end2: &str) -> Result<Vec<TemplateAst>> {
        let mut nodes = Vec::new();
        // Simplified implementation
        Ok(nodes)
    }

    fn last_directive_was(&self, directive: &str) -> bool {
        // Simplified implementation
        false
    }

    fn is_directive_start(&self) -> bool {
        (self.current_char() == '{' && self.peek_char() == '{')
            || (self.current_char() == '<' && self.peek_char() == '%')
    }

    fn current_char(&self) -> char {
        self.content.chars().nth(self.position).unwrap_or('\0')
    }

    fn peek_char(&self) -> char {
        self.content.chars().nth(self.position + 1).unwrap_or('\0')
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.position += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.content.len()
    }

    fn consume_string(&mut self, expected: &str) -> Result<()> {
        if self.content[self.position..].starts_with(expected) {
            self.position += expected.len();
            Ok(())
        } else {
            Err(RuitlError::template(format!("Expected '{}'", expected)))
        }
    }

    fn consume_until(&mut self, delimiter: &str) -> Result<String> {
        let start = self.position;
        while !self.is_at_end() && !self.content[self.position..].starts_with(delimiter) {
            self.advance();
        }

        if self.is_at_end() {
            Err(RuitlError::template(format!("Expected '{}'", delimiter)))
        } else {
            Ok(self.content[start..self.position].to_string())
        }
    }
}

impl TemplateValue {
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            TemplateValue::String(s) => s.clone(),
            TemplateValue::Number(n) => n.to_string(),
            TemplateValue::Boolean(b) => b.to_string(),
            TemplateValue::Array(_) => "[Array]".to_string(),
            TemplateValue::Object(_) => "[Object]".to_string(),
            TemplateValue::Null => "null".to_string(),
        }
    }

    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            TemplateValue::Boolean(b) => *b,
            TemplateValue::String(s) => !s.is_empty(),
            TemplateValue::Number(n) => *n != 0.0,
            TemplateValue::Array(arr) => !arr.is_empty(),
            TemplateValue::Object(obj) => !obj.is_empty(),
            TemplateValue::Null => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_engine_creation() {
        let engine = TemplateEngine::new();
        assert!(engine.templates.is_empty());
        assert!(engine.components.is_empty());
    }

    #[test]
    fn test_template_registration() {
        let mut engine = TemplateEngine::new();
        let result = engine.register_template("test", "<h1>Hello</h1>");
        assert!(result.is_ok());
        assert!(engine.templates.contains_key("test"));
    }

    #[test]
    fn test_template_value_conversion() {
        let value = TemplateValue::String("test".to_string());
        assert_eq!(value.to_string(), "test");
        assert!(value.is_truthy());

        let value = TemplateValue::Boolean(false);
        assert!(!value.is_truthy());

        let value = TemplateValue::Null;
        assert!(!value.is_truthy());
    }

    #[test]
    fn test_template_parser() {
        let mut parser = TemplateParser::new("Hello {{name}}!");
        let ast = parser.parse().unwrap();

        if let TemplateAst::Fragment(nodes) = ast {
            assert_eq!(nodes.len(), 3);
            assert!(matches!(nodes[0], TemplateAst::Text(_)));
            assert!(matches!(nodes[1], TemplateAst::Expression(_)));
            assert!(matches!(nodes[2], TemplateAst::Text(_)));
        } else {
            panic!("Expected fragment");
        }
    }

    #[test]
    fn test_compile_context() {
        let context = CompileContext::default();
        assert!(context.variables.is_empty());
        assert!(context.imports.is_empty());
        assert!(context.component_types.is_empty());
    }
}
