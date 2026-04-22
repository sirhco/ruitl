//! RUITL Code Generator
//!
//! Converts parsed .ruitl templates into optimized Rust code that uses the RUITL runtime library.

use crate::error::{CompileError, Result};
use crate::parser::{
    Attribute, AttributeValue, ComponentDef, ImportDef, MatchArm, PropValue, RuitlFile,
    TemplateAst, TemplateDef,
};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use syn::{parse_str, Expr, Type};

/// Render `<T: Debug + Clone + ..., U>` declarations for use at a struct or
/// impl header. Always appends the bounds required by the `ComponentProps`
/// trait (`Debug + Clone + Send + Sync + 'static`) so downstream blanket impls
/// compile without the user having to spell them out.
fn render_generic_param_decls(generics: &[crate::parser::GenericParam]) -> Vec<TokenStream> {
    generics
        .iter()
        .map(|g| {
            let name = format_ident!("{}", g.name);
            let mut bounds_toks: Vec<TokenStream> = g
                .bounds
                .iter()
                .map(|b| {
                    b.parse::<TokenStream>()
                        .unwrap_or_else(|_| format_ident!("{}", b).to_token_stream())
                })
                .collect();
            // Auto-append the bounds required by the Component/ComponentProps
            // traits if the user didn't already list them.
            for required in &["Debug", "Clone", "Send", "Sync"] {
                if !g.bounds.iter().any(|b| b == required) {
                    let id = format_ident!("{}", required);
                    bounds_toks.push(quote! { #id });
                }
            }
            // 'static bound — always required for Component impls.
            bounds_toks.push(quote! { 'static });
            quote! { #name: #(#bounds_toks)+* }
        })
        .collect()
}

/// Render just the generic identifiers (e.g. `<T, U>`) for use at a call site.
fn render_generic_param_idents(generics: &[crate::parser::GenericParam]) -> Vec<Ident> {
    generics
        .iter()
        .map(|g| format_ident!("{}", g.name))
        .collect()
}

/// Scan a Rust-expression string for identifier tokens and collect them into
/// `out`. Keywords and numeric literals are skipped; a token is considered an
/// identifier if it matches `[A-Za-z_][A-Za-z0-9_]*` bounded by non-ident
/// characters. This is a deliberately lightweight scanner — we only need it
/// to decide which declared prop names are referenced by the template, not
/// to build a real expression AST.
fn scan_idents(src: &str, out: &mut std::collections::HashSet<String>) {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        // Skip string literals — their contents never reference props.
        if b == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if b.is_ascii_alphabetic() || b == b'_' {
            // An ident preceded by `.` is a field/method access, not a
            // reference to a top-level binding. Skip it so `props.variant`
            // only collects `props`, not `variant` — otherwise the binding
            // `let variant = &props.variant` appears used when it isn't.
            let preceded_by_dot = i > 0 && bytes[i - 1] == b'.';
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
            {
                i += 1;
            }
            if !preceded_by_dot {
                if let Ok(tok) = std::str::from_utf8(&bytes[start..i]) {
                    out.insert(tok.to_string());
                }
            }
        } else {
            i += 1;
        }
    }
}

/// Code generator for converting RUITL templates to Rust code
pub struct CodeGenerator {
    file: RuitlFile,
    generated_components: HashMap<String, TokenStream>,
    generated_imports: Vec<TokenStream>,
}

impl CodeGenerator {
    /// Create a new code generator for the given RUITL file
    pub fn new(file: RuitlFile) -> Self {
        Self {
            file,
            generated_components: HashMap::new(),
            generated_imports: Vec::new(),
        }
    }

    /// Generate complete Rust code for the entire file
    pub fn generate(&mut self) -> Result<TokenStream> {
        // Check templates for undefined `@Component` references, unknown
        // props at call sites, and reserved-name collisions before emitting
        // any tokens. Failing fast here produces cleaner error messages than
        // letting `syn`/`rustc` complain about the generated output.
        self.validate_references()?;

        // Generate imports
        self.generate_imports()?;

        // Generate component definitions and their props
        for component in &self.file.components.clone() {
            self.generate_component_definition(component)?;
        }

        // Generate template implementations
        for template in &self.file.templates.clone() {
            self.generate_template_implementation(template)?;
        }

        // Combine all generated code
        let imports = &self.generated_imports;
        // Emit components in source-declaration order (matches `self.file.components`).
        // HashMap iteration is non-deterministic and would cause spurious diffs
        // in the committed sibling *_ruitl.rs files between builds.
        let components: Vec<&TokenStream> = self
            .file
            .components
            .iter()
            .filter_map(|c| self.generated_components.get(&c.name))
            .collect();

        Ok(quote! {
            use ruitl::prelude::*;
            use ruitl::html::*;

            #(#imports)*

            #(#components)*
        })
    }

    /// Generate import statements
    fn generate_imports(&mut self) -> Result<()> {
        for import in &self.file.imports {
            let import_tokens = self.generate_import(import)?;
            self.generated_imports.push(import_tokens);
        }
        Ok(())
    }

    /// Generate a single import statement
    fn generate_import(&self, import: &ImportDef) -> Result<TokenStream> {
        // Parse the path as a token stream so `std::collections` is emitted
        // as a real module path, not as a string literal.
        let path: TokenStream = import.path.parse().map_err(|e| {
            CompileError::codegen(format!("Invalid import path '{}': {}", import.path, e))
        })?;

        if import.items.is_empty() {
            Ok(quote! {
                use #path;
            })
        } else {
            let items: Vec<Ident> = import
                .items
                .iter()
                .map(|item| format_ident!("{}", item))
                .collect();

            Ok(quote! {
                use #path::{#(#items),*};
            })
        }
    }

    /// Generate component definition (props struct and Component impl)
    fn generate_component_definition(&mut self, component: &ComponentDef) -> Result<()> {
        let component_name = format_ident!("{}", component.name);

        // Generate props struct
        let props_struct = self.generate_props_struct(component)?;

        // Generate component struct. When generics are present, the struct
        // carries `PhantomData` so the type params appear in the struct body
        // even if no field actually uses them. `PhantomData<fn() -> (T, U)>`
        // makes the struct invariant in T/U — avoids accidental subtyping
        // surprises.
        let component_struct = if component.generics.is_empty() {
            quote! {
                #[derive(Debug)]
                pub struct #component_name;
            }
        } else {
            let generic_decls = render_generic_param_decls(&component.generics);
            let generic_idents = render_generic_param_idents(&component.generics);
            quote! {
                #[derive(Debug)]
                pub struct #component_name<#(#generic_decls),*>(
                    pub ::core::marker::PhantomData<fn() -> (#(#generic_idents,)*)>
                );

                impl<#(#generic_decls),*> ::core::default::Default for #component_name<#(#generic_idents),*> {
                    fn default() -> Self {
                        Self(::core::marker::PhantomData)
                    }
                }
            }
        };

        // Store the generated code
        let combined = quote! {
            #props_struct
            #component_struct
        };

        self.generated_components
            .insert(component.name.clone(), combined);
        Ok(())
    }

    /// Generate props struct for a component
    fn generate_props_struct(&self, component: &ComponentDef) -> Result<TokenStream> {
        let props_name = format_ident!("{}Props", component.name);

        // Does this component's template body reference `{children}`? If so,
        // auto-emit a `pub children: Html` field on the Props struct — but
        // only when the user hasn't already declared a `children` prop of
        // their own. A user-declared `children` stays as-is (useful if they
        // want a non-`Html` type or a different default), and the
        // `{children}` slot simply reads whichever field is present.
        let user_declared_children = component.props.iter().any(|p| p.name == "children");
        let needs_children =
            self.component_needs_children(&component.name) && !user_declared_children;

        if component.props.is_empty() && !needs_children {
            return Ok(quote! {
                pub type #props_name = EmptyProps;
            });
        }

        let mut fields = Vec::new();
        let mut field_validations = Vec::new();

        for prop in &component.props {
            let field_name = format_ident!("{}", prop.name);
            let field_type: Type = parse_str(&prop.prop_type).map_err(|e| {
                CompileError::codegen(format!("Invalid type '{}': {}", prop.prop_type, e))
            })?;

            let field_type = if prop.optional {
                quote! { Option<#field_type> }
            } else {
                quote! { #field_type }
            };

            fields.push(quote! {
                pub #field_name: #field_type
            });

            // Add validation if needed
            if !prop.optional {
                field_validations.push(quote! {
                    // Non-optional field validation could go here
                });
            }
        }

        if needs_children {
            fields.push(quote! {
                pub children: Html
            });
        }

        let (struct_decl, impl_decl) = if component.generics.is_empty() {
            (quote! { pub struct #props_name }, quote! { impl ComponentProps for #props_name })
        } else {
            let generic_decls = render_generic_param_decls(&component.generics);
            let generic_idents = render_generic_param_idents(&component.generics);
            (
                quote! { pub struct #props_name<#(#generic_decls),*> },
                quote! {
                    impl<#(#generic_decls),*> ComponentProps for #props_name<#(#generic_idents),*>
                },
            )
        };

        Ok(quote! {
            #[derive(Debug, Clone)]
            #struct_decl {
                #(#fields),*
            }

            #impl_decl {
                fn validate(&self) -> Result<()> {
                    #(#field_validations)*
                    Ok(())
                }
            }
        })
    }

    /// Generate template implementation (Component trait impl)
    fn generate_template_implementation(&mut self, template: &TemplateDef) -> Result<()> {
        let component_name = format_ident!("{}", template.name);
        let props_name = format_ident!("{}Props", template.name);

        // Find the component definition for prop bindings
        let component = self
            .file
            .components
            .iter()
            .find(|c| c.name == template.name)
            .ok_or_else(|| {
                CompileError::codegen(format!(
                    "No component definition found for template '{}'",
                    template.name
                ))
            })?;

        // The template's generic list and the component's must agree. Prefer
        // the component's list (it owns the type-parameter identity).
        let generics = if component.generics.is_empty() {
            &template.generics
        } else {
            &component.generics
        };

        // Collect all identifiers referenced anywhere in the template body so
        // we only emit `let foo = &props.foo` for props that the body actually
        // uses. Unused bindings would trigger `unused_variables` warnings in
        // downstream crates.
        let referenced = Self::collect_referenced_idents(&template.body);

        // Generate prop bindings for local access
        let prop_bindings = self.generate_prop_bindings(component, &referenced)?;

        // Generate the render method body
        let render_body = self.generate_ast_code(&template.body)?;

        // Determine whether the body actually references `context` (only true
        // when composing child components via `@Component(...)` syntax). If
        // not, emit the parameter as `_context` to avoid unused-variable warnings.
        let context_ident = if Self::template_uses_context(&template.body) {
            format_ident!("context")
        } else {
            format_ident!("_context")
        };

        // Create the Component implementation.
        //
        // `#[allow(unused_variables)]` covers corner cases our ident-scanner
        // cannot statically prove are used — e.g. a prop bound at function
        // top that is immediately shadowed by an `if let Some(x) = &props.x`
        // pattern in the body. The scanner emits the outer binding defensively
        // because the pattern variable name matches the prop; the compiler
        // then sees the outer binding as unused.
        let impl_block = if generics.is_empty() {
            quote! {
                impl Component for #component_name {
                    type Props = #props_name;

                    #[allow(unused_variables)]
                    fn render(&self, props: &Self::Props, #context_ident: &ComponentContext) -> Result<Html> {
                        #prop_bindings
                        Ok(#render_body)
                    }
                }
            }
        } else {
            let generic_decls = render_generic_param_decls(generics);
            let generic_idents = render_generic_param_idents(generics);
            quote! {
                impl<#(#generic_decls),*> Component for #component_name<#(#generic_idents),*> {
                    type Props = #props_name<#(#generic_idents),*>;

                    #[allow(unused_variables)]
                    fn render(&self, props: &Self::Props, #context_ident: &ComponentContext) -> Result<Html> {
                        #prop_bindings
                        Ok(#render_body)
                    }
                }
            }
        };

        // Add to existing component definition
        if let Some(existing) = self.generated_components.get(&template.name) {
            let combined = quote! {
                #existing
                #impl_block
            };
            self.generated_components
                .insert(template.name.clone(), combined);
        } else {
            return Err(CompileError::codegen(format!(
                "Template '{}' has no corresponding component definition",
                template.name
            )));
        }

        Ok(())
    }

    /// Generate Rust code for a template AST node
    fn generate_ast_code(&self, ast: &TemplateAst) -> Result<TokenStream> {
        match ast {
            TemplateAst::Element {
                tag,
                attributes,
                children,
                self_closing,
            } => self.generate_element_code(tag, attributes, children, *self_closing),

            TemplateAst::Text(text) => {
                if text.trim().is_empty() {
                    Ok(quote! { Html::Empty })
                } else {
                    Ok(quote! { Html::text(#text) })
                }
            }

            TemplateAst::Expression(expr) => {
                let transformed_expr = self.transform_variable_access(expr);
                let expr: Expr = parse_str(&transformed_expr).map_err(|e| {
                    CompileError::codegen(format!("Invalid expression '{}': {}", transformed_expr, e))
                })?;
                Ok(quote! { Html::text(&format!("{}", #expr)) })
            }

            TemplateAst::RawExpression(expr) => {
                // `{!expr}` injects the runtime value as raw HTML — no
                // escaping. Callers are responsible for ensuring safety.
                let transformed_expr = self.transform_variable_access(expr);
                let expr: Expr = parse_str(&transformed_expr).map_err(|e| {
                    CompileError::codegen(format!(
                        "Invalid raw expression '{}': {}",
                        transformed_expr, e
                    ))
                })?;
                Ok(quote! { Html::raw(format!("{}", #expr)) })
            }

            TemplateAst::If {
                condition,
                then_branch,
                else_branch,
            } => self.generate_if_code(condition, then_branch, else_branch),

            TemplateAst::For {
                variable,
                iterable,
                body,
            } => self.generate_for_code(variable, iterable, body),

            TemplateAst::Match { expression, arms } => self.generate_match_code(expression, arms),

            TemplateAst::Component {
                name,
                props,
                children,
            } => self.generate_component_invocation_code(name, props, children.as_deref()),

            TemplateAst::Children => {
                // `{children}` — emit `props.children.clone()`. The owning
                // component's Props struct is augmented with a
                // `pub children: Html` field in `generate_props_struct` when
                // the body contains this variant.
                Ok(quote! { props.children.clone() })
            }

            TemplateAst::Fragment(nodes) => {
                let node_codes: Result<Vec<_>> = nodes
                    .iter()
                    .map(|node| self.generate_ast_code(node))
                    .collect();
                let node_codes = node_codes?;

                Ok(quote! {
                    Html::fragment(vec![#(#node_codes),*])
                })
            }

            TemplateAst::Raw(html) => Ok(quote! { Html::raw(#html) }),
        }
    }

    /// Generate code for an HTML element
    fn generate_element_code(
        &self,
        tag: &str,
        attributes: &[Attribute],
        children: &[TemplateAst],
        self_closing: bool,
    ) -> Result<TokenStream> {
        let tag_name = tag;

        // Start with element creation
        let mut element_code = if self_closing {
            quote! { HtmlElement::self_closing(#tag_name) }
        } else {
            quote! { HtmlElement::new(#tag_name) }
        };

        // Add attributes
        for attr in attributes {
            let attr_code = self.generate_attribute_code(attr)?;
            element_code = quote! { #element_code.#attr_code };
        }

        // Add children
        if !self_closing {
            for child in children {
                let child_code = self.generate_ast_code(child)?;
                element_code = quote! { #element_code.child(#child_code) };
            }
        }

        Ok(quote! { Html::Element(#element_code) })
    }

    /// Generate code for an HTML attribute
    fn generate_attribute_code(&self, attr: &Attribute) -> Result<TokenStream> {
        let attr_name = &attr.name;

        match &attr.value {
            AttributeValue::Static(value) => Ok(quote! { attr(#attr_name, #value) }),

            AttributeValue::Expression(expr) => {
                let expr: Expr = parse_str(expr).map_err(|e| {
                    CompileError::codegen(format!("Invalid attribute expression '{}': {}", expr, e))
                })?;
                Ok(quote! { attr(#attr_name, &format!("{}", #expr)) })
            }

            AttributeValue::Conditional(condition) => {
                let condition: Expr = parse_str(condition).map_err(|e| {
                    CompileError::codegen(format!(
                        "Invalid conditional expression '{}': {}",
                        condition, e
                    ))
                })?;

                // Check if this is a known boolean attribute
                let boolean_attrs = [
                    "disabled",
                    "checked",
                    "selected",
                    "readonly",
                    "multiple",
                    "autofocus",
                    "autoplay",
                    "controls",
                    "defer",
                    "hidden",
                    "loop",
                    "open",
                    "required",
                    "reversed",
                ];

                if boolean_attrs.contains(&attr_name.as_str()) {
                    // For boolean attributes, use attr_if
                    Ok(quote! {
                        attr_if(#attr_name, #condition, #attr_name)
                    })
                } else {
                    // For Option attributes, use attr_optional
                    Ok(quote! {
                        attr_optional(#attr_name, &#condition)
                    })
                }
            }
        }
    }

    /// Generate code for if statement
    fn generate_if_code(
        &self,
        condition: &str,
        then_branch: &TemplateAst,
        else_branch: &Option<Box<TemplateAst>>,
    ) -> Result<TokenStream> {
        let transformed_condition = self.transform_variable_access(condition);
        let condition: Expr = parse_str(&transformed_condition).map_err(|e| {
            CompileError::codegen(format!(
                "Invalid if condition '{}': {}",
                transformed_condition, e
            ))
        })?;

        let then_code = self.generate_ast_code(then_branch)?;

        if let Some(else_branch) = else_branch {
            let else_code = self.generate_ast_code(else_branch)?;
            Ok(quote! {
                if #condition {
                    #then_code
                } else {
                    #else_code
                }
            })
        } else {
            Ok(quote! {
                if #condition {
                    #then_code
                } else {
                    Html::Empty
                }
            })
        }
    }

    /// Generate code for for loop
    fn generate_for_code(
        &self,
        variable: &str,
        iterable: &str,
        body: &TemplateAst,
    ) -> Result<TokenStream> {
        // Parse the binding as a raw token stream so both simple identifiers
        // (`item`) and tuple patterns (`(k, v)`) are emitted verbatim into
        // the generated closure parameter list.
        let var_pat: TokenStream = variable.parse().map_err(|e| {
            CompileError::codegen(format!(
                "Invalid for-loop binding '{}': {}",
                variable, e
            ))
        })?;
        let transformed_iterable = self.transform_variable_access(iterable);
        let iterable: Expr = parse_str(&transformed_iterable).map_err(|e| {
            CompileError::codegen(format!(
                "Invalid for iterable '{}': {}",
                transformed_iterable, e
            ))
        })?;

        let body_code = self.generate_ast_code(body)?;

        Ok(quote! {
            Html::fragment(
                #iterable
                    .into_iter()
                    .map(|#var_pat| #body_code)
                    .collect::<Vec<_>>()
            )
        })
    }

    /// Generate code for match statement
    fn generate_match_code(&self, expression: &str, arms: &[MatchArm]) -> Result<TokenStream> {
        let expr: Expr = parse_str(expression).map_err(|e| {
            CompileError::codegen(format!("Invalid match expression '{}': {}", expression, e))
        })?;

        let mut match_arms = Vec::new();

        for arm in arms {
            // Parse the pattern as a token stream so that string-literal
            // patterns like `"active"` stay as `"active"` instead of being
            // re-quoted into `"\"active\""` (which happens if the &String is
            // interpolated via `quote!` directly).
            let pattern: proc_macro2::TokenStream =
                arm.pattern.parse().map_err(|e| {
                    CompileError::codegen(format!(
                        "Invalid match pattern '{}': {}",
                        arm.pattern, e
                    ))
                })?;
            let body_code = self.generate_ast_code(&arm.body)?;

            match_arms.push(quote! {
                #pattern => #body_code
            });
        }

        Ok(quote! {
            match #expr {
                #(#match_arms,)*
            }
        })
    }

    /// Walk a template AST and return true if any node invokes a child component
    /// (via `@Component(...)` syntax). Such invocations thread `context` through,
    /// so the render method needs the `context` parameter to be named — otherwise
    /// it can be `_context` to silence unused-variable warnings.
    fn template_uses_context(ast: &TemplateAst) -> bool {
        match ast {
            TemplateAst::Component { .. } => true,
            TemplateAst::Element { children, .. } => {
                children.iter().any(Self::template_uses_context)
            }
            TemplateAst::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::template_uses_context(then_branch)
                    || else_branch
                        .as_deref()
                        .map(Self::template_uses_context)
                        .unwrap_or(false)
            }
            TemplateAst::For { body, .. } => Self::template_uses_context(body),
            TemplateAst::Match { arms, .. } => {
                arms.iter().any(|arm| Self::template_uses_context(&arm.body))
            }
            TemplateAst::Fragment(nodes) => nodes.iter().any(Self::template_uses_context),
            TemplateAst::Text(_)
            | TemplateAst::Expression(_)
            | TemplateAst::RawExpression(_)
            | TemplateAst::Raw(_)
            | TemplateAst::Children => false,
        }
    }

    /// Walk a template AST and collect every identifier that appears in any
    /// embedded Rust expression, attribute value, control-flow condition, or
    /// child-component prop value. Used by `generate_prop_bindings` to skip
    /// binding props that the template body never references.
    fn collect_referenced_idents(ast: &TemplateAst) -> std::collections::HashSet<String> {
        let mut out = std::collections::HashSet::new();
        Self::collect_idents_rec(ast, &mut out);
        out
    }

    fn collect_idents_rec(ast: &TemplateAst, out: &mut std::collections::HashSet<String>) {
        match ast {
            TemplateAst::Text(_) | TemplateAst::Raw(_) => {}
            TemplateAst::Expression(expr) | TemplateAst::RawExpression(expr) => {
                scan_idents(expr, out)
            }
            TemplateAst::Element {
                attributes,
                children,
                ..
            } => {
                for attr in attributes {
                    match &attr.value {
                        AttributeValue::Static(_) => {}
                        AttributeValue::Expression(e) | AttributeValue::Conditional(e) => {
                            scan_idents(e, out);
                        }
                    }
                }
                for child in children {
                    Self::collect_idents_rec(child, out);
                }
            }
            TemplateAst::If {
                condition,
                then_branch,
                else_branch,
            } => {
                scan_idents(condition, out);
                Self::collect_idents_rec(then_branch, out);
                if let Some(e) = else_branch {
                    Self::collect_idents_rec(e, out);
                }
            }
            TemplateAst::For {
                iterable, body, ..
            } => {
                scan_idents(iterable, out);
                Self::collect_idents_rec(body, out);
            }
            TemplateAst::Match { expression, arms } => {
                scan_idents(expression, out);
                for arm in arms {
                    scan_idents(&arm.pattern, out);
                    Self::collect_idents_rec(&arm.body, out);
                }
            }
            TemplateAst::Component {
                props, children, ..
            } => {
                for pv in props {
                    scan_idents(&pv.value, out);
                }
                if let Some(body) = children {
                    Self::collect_idents_rec(body, out);
                }
            }
            TemplateAst::Children => {
                // The slot placeholder reads `props.children`; surface
                // "children" so the prop-binding pass keeps that binding
                // alive in the generated render body.
                out.insert("children".to_string());
            }
            TemplateAst::Fragment(nodes) => {
                for n in nodes {
                    Self::collect_idents_rec(n, out);
                }
            }
        }
    }

    /// Generate code for component invocation
    /// Generate local bindings for props with proper Option handling
    fn generate_prop_bindings(
        &self,
        component: &ComponentDef,
        referenced: &std::collections::HashSet<String>,
    ) -> Result<TokenStream> {
        let mut bindings = Vec::new();

        for prop in &component.props {
            // Only bind props that the template body references; binding
            // unused props would produce `unused_variables` warnings for
            // every downstream consumer.
            if !referenced.contains(&prop.name) {
                continue;
            }

            let prop_name = format_ident!("{}", prop.name);

            // For primitive types, copy the value; for complex types, use reference
            if self.is_primitive_type(&prop.prop_type) {
                bindings.push(quote! {
                    let #prop_name = props.#prop_name;
                });
            } else {
                bindings.push(quote! {
                    let #prop_name = &props.#prop_name;
                });
            }
        }

        Ok(quote! {
            #(#bindings)*
        })
    }

    /// Check if a type is primitive and should be copied rather than referenced
    fn is_primitive_type(&self, type_name: &str) -> bool {
        matches!(
            type_name.trim(),
            "bool"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "f32"
                | "f64"
                | "char"
        )
    }

    /// Transform variable access - now variables are local bindings
    fn transform_variable_access(&self, expr: &str) -> String {
        let mut transformed = expr.to_string();

        // Fix common borrowing issues with Option<String>
        // Replace .unwrap_or_default() with .as_deref().unwrap_or("") for Option<String>
        if transformed.contains(".unwrap_or_default()") {
            // This is a heuristic - assumes most unwrap_or_default() calls are on Option<String>
            // in template contexts where we want string output
            transformed =
                transformed.replace(".unwrap_or_default()", ".as_deref().unwrap_or(\"\")");
        }

        // Fix Option<T>.unwrap_or(String::new()) patterns
        if transformed.contains(".unwrap_or(String::new())") {
            transformed =
                transformed.replace(".unwrap_or(String::new())", ".as_deref().unwrap_or(\"\")");
        }

        // Fix if let Some patterns to use borrowing
        // Transform "if let Some(var) = props.field" to "if let Some(var) = &props.field"
        if transformed.contains("if let Some(") && transformed.contains("= props.") {
            // Simple heuristic replacement for common patterns
            let parts: Vec<&str> = transformed.split("= props.").collect();
            if parts.len() == 2 {
                let before_props = parts[0];
                let after_props = parts[1];
                // Check if this looks like an if let Some pattern
                if before_props.trim_end().ends_with("if let Some(")
                    || before_props.contains("if let Some(")
                {
                    transformed = format!("{}= &props.{}", before_props, after_props);
                }
            }
        }

        transformed
    }

    fn generate_component_invocation_code(
        &self,
        name: &str,
        props: &[PropValue],
        children: Option<&TemplateAst>,
    ) -> Result<TokenStream> {
        let component_ident = format_ident!("{}", name);
        let props_ident = format_ident!("{}Props", name);

        let mut prop_assignments = Vec::new();

        for prop in props {
            let prop_name = format_ident!("{}", prop.name);
            let prop_value: Expr = parse_str(&prop.value).map_err(|e| {
                CompileError::codegen(format!("Invalid prop value '{}': {}", prop.value, e))
            })?;

            prop_assignments.push(quote! {
                #prop_name: #prop_value
            });
        }

        // Feed the body block into the callee's auto-injected `children` prop.
        // If the call site has a body, always emit `children: <rendered>`.
        // If there is no body but the callee is defined locally and its
        // template references `{children}`, emit `children: Html::Empty` so
        // the generated struct literal is complete.
        if let Some(body) = children {
            let body_code = self.generate_ast_code(body)?;
            prop_assignments.push(quote! {
                children: #body_code
            });
        } else if self.component_needs_children(name) {
            prop_assignments.push(quote! {
                children: Html::Empty
            });
        }

        Ok(quote! {
            {
                let component = #component_ident;
                let props = #props_ident {
                    #(#prop_assignments),*
                };
                component.render(&props, context)?
            }
        })
    }

    /// Does the named component's template body reference `{children}`? If
    /// so, its generated Props struct carries a `pub children: Html` field
    /// and every call site must populate it. Only inspects components defined
    /// in the current file — out-of-file callees are on their own.
    fn component_needs_children(&self, name: &str) -> bool {
        self.file
            .templates
            .iter()
            .find(|t| t.name == name)
            .map(|t| Self::body_has_children_slot(&t.body))
            .unwrap_or(false)
    }

    /// Walk every template body once to surface broken `@Component(...)`
    /// call sites before codegen. For each invocation we check:
    ///   * component name is declared in this file or imported
    ///   * every prop name matches a field on the callee's Props struct
    ///     (only verifiable for same-file callees — out-of-file types are
    ///     opaque here and left to `rustc`)
    /// Suggestions are appended to the error message via `suggest::help_line`
    /// so both CLI consumers and the LSP pick them up without structural
    /// changes to `CompileError`.
    fn validate_references(&self) -> Result<()> {
        let known_components: Vec<&str> = self
            .file
            .components
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        let imported_items: Vec<&str> = self
            .file
            .imports
            .iter()
            .flat_map(|imp| imp.items.iter().map(String::as_str))
            .collect();

        for tpl in &self.file.templates {
            self.walk_validate(&tpl.body, &known_components, &imported_items, &tpl.name)?;
        }
        Ok(())
    }

    fn walk_validate(
        &self,
        ast: &TemplateAst,
        known_components: &[&str],
        imported_items: &[&str],
        current_template: &str,
    ) -> Result<()> {
        match ast {
            TemplateAst::Component {
                name,
                props,
                children,
            } => {
                // Cross-file `@Component` invocations are legal: callees are
                // resolved through the generated `mod.rs` module at Rust
                // compile time, not at ruitl-compile time. So *don't* error
                // on "unknown component" blindly — it would reject legit
                // multi-file projects. Only error when the name is close
                // enough to an in-file declaration to look like a typo.
                let is_known = known_components.iter().any(|k| k == name)
                    || imported_items.iter().any(|k| k == name);
                if !is_known {
                    let mut candidates: Vec<&str> = known_components.to_vec();
                    candidates.extend_from_slice(imported_items);
                    if let Some(suggestion) = crate::suggest::suggest(name, &candidates) {
                        return Err(CompileError::codegen(format!(
                            "Unknown component `{}` invoked via `@{}` in template `{}`.{}",
                            name,
                            name,
                            current_template,
                            crate::suggest::help_line(Some(suggestion.as_str()))
                        )));
                    }
                    // No close match → assume it's a cross-file callee.
                    // Fall through; skip prop validation (can't see the
                    // callee's props from here).
                    if let Some(body) = children {
                        self.walk_validate(
                            body,
                            known_components,
                            imported_items,
                            current_template,
                        )?;
                    }
                    return Ok(());
                }

                // Same-file callees: check prop names against the callee's
                // declared prop list. Out-of-file (imported) callees stay
                // opaque here; `rustc` will catch mistypes on the struct
                // literal downstream.
                if let Some(callee) = self.file.components.iter().find(|c| c.name == *name) {
                    let declared: Vec<&str> =
                        callee.props.iter().map(|p| p.name.as_str()).collect();
                    // `children` is auto-injected when the slot is used and
                    // is always a legal prop name at call sites.
                    let legal_extra = ["children"];
                    for pv in props {
                        let is_declared =
                            declared.contains(&pv.name.as_str()) || legal_extra.contains(&pv.name.as_str());
                        if !is_declared {
                            let suggestion = crate::suggest::suggest(&pv.name, &declared);
                            return Err(CompileError::codegen(format!(
                                "No prop `{}` on `{}Props` (called from template `{}`).{}",
                                pv.name,
                                name,
                                current_template,
                                crate::suggest::help_line(suggestion.as_deref())
                            )));
                        }
                    }
                }

                if let Some(body) = children {
                    self.walk_validate(body, known_components, imported_items, current_template)?;
                }
                Ok(())
            }
            TemplateAst::Element { children, .. } => {
                for c in children {
                    self.walk_validate(c, known_components, imported_items, current_template)?;
                }
                Ok(())
            }
            TemplateAst::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.walk_validate(
                    then_branch,
                    known_components,
                    imported_items,
                    current_template,
                )?;
                if let Some(e) = else_branch {
                    self.walk_validate(e, known_components, imported_items, current_template)?;
                }
                Ok(())
            }
            TemplateAst::For { body, .. } => {
                self.walk_validate(body, known_components, imported_items, current_template)
            }
            TemplateAst::Match { arms, .. } => {
                for arm in arms {
                    self.walk_validate(
                        &arm.body,
                        known_components,
                        imported_items,
                        current_template,
                    )?;
                }
                Ok(())
            }
            TemplateAst::Fragment(nodes) => {
                for n in nodes {
                    self.walk_validate(n, known_components, imported_items, current_template)?;
                }
                Ok(())
            }
            TemplateAst::Text(_)
            | TemplateAst::Expression(_)
            | TemplateAst::RawExpression(_)
            | TemplateAst::Raw(_)
            | TemplateAst::Children => Ok(()),
        }
    }

    /// Recursively checks whether `ast` contains a `TemplateAst::Children`
    /// node anywhere in its subtree. Used to decide whether a component's
    /// Props struct needs the auto-injected `children: Html` field.
    fn body_has_children_slot(ast: &TemplateAst) -> bool {
        match ast {
            TemplateAst::Children => true,
            TemplateAst::Element { children, .. } => {
                children.iter().any(Self::body_has_children_slot)
            }
            TemplateAst::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::body_has_children_slot(then_branch)
                    || else_branch
                        .as_deref()
                        .map(Self::body_has_children_slot)
                        .unwrap_or(false)
            }
            TemplateAst::For { body, .. } => Self::body_has_children_slot(body),
            TemplateAst::Match { arms, .. } => arms
                .iter()
                .any(|arm| Self::body_has_children_slot(&arm.body)),
            TemplateAst::Fragment(nodes) => nodes.iter().any(Self::body_has_children_slot),
            TemplateAst::Component { children, .. } => children
                .as_deref()
                .map(Self::body_has_children_slot)
                .unwrap_or(false),
            TemplateAst::Text(_)
            | TemplateAst::Expression(_)
            | TemplateAst::RawExpression(_)
            | TemplateAst::Raw(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ComponentDef, PropDef, RuitlFile, TemplateAst, TemplateDef};

    /// Collapse any run of whitespace in a `TokenStream::to_string()` output to
    /// single spaces. `proc_macro2` emits spaces around every punctuation
    /// (`text : String`, `HtmlElement :: new`) and rarely puts newlines in
    /// predictable places, so `contains("text : String")` checks are robust
    /// against formatting changes if we normalize first.
    fn normalize_ws(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn create_test_component() -> ComponentDef {
        ComponentDef {
            name: "Button".to_string(),
            props: vec![
                PropDef {
                    name: "text".to_string(),
                    prop_type: "String".to_string(),
                    optional: false,
                    default_value: None,
                },
                PropDef {
                    name: "disabled".to_string(),
                    prop_type: "bool".to_string(),
                    optional: true,
                    default_value: Some("false".to_string()),
                },
            ],
            generics: vec![],
            leading_comments: vec![],
        }
    }

    fn create_test_template() -> TemplateDef {
        TemplateDef {
            name: "Button".to_string(),
            params: vec![],
            body: TemplateAst::Element {
                tag: "button".to_string(),
                attributes: vec![
                    Attribute {
                        name: "class".to_string(),
                        value: AttributeValue::Static("btn".to_string()),
                    },
                    Attribute {
                        name: "disabled".to_string(),
                        value: AttributeValue::Conditional(
                            "props.disabled.unwrap_or(false)".to_string(),
                        ),
                    },
                ],
                children: vec![TemplateAst::Expression("props.text".to_string())],
                self_closing: false,
            },
            generics: vec![],
            leading_comments: vec![],
        }
    }

    #[test]
    fn test_generate_props_struct() {
        let component = create_test_component();
        let generator = CodeGenerator::new(RuitlFile {
            components: vec![],
            templates: vec![],
            imports: vec![],
        });

        let result = generator.generate_props_struct(&component).unwrap();
        let code = result.to_string();
        let normalized = normalize_ws(&code);

        assert!(normalized.contains("struct ButtonProps"));
        assert!(normalized.contains("text : String"));
        assert!(normalized.contains("disabled : Option < bool >"));
        assert!(normalized.contains("impl ComponentProps"));
    }

    #[test]
    fn test_generate_element_code() {
        let generator = CodeGenerator::new(RuitlFile {
            components: vec![],
            templates: vec![],
            imports: vec![],
        });

        let attributes = vec![Attribute {
            name: "class".to_string(),
            value: AttributeValue::Static("btn".to_string()),
        }];

        let children = vec![TemplateAst::Text("Click me".to_string())];

        let result = generator
            .generate_element_code("button", &attributes, &children, false)
            .unwrap();

        let code = result.to_string();
        let normalized = normalize_ws(&code);
        assert!(normalized.contains("HtmlElement :: new"));
        assert!(normalized.contains("attr"));
        assert!(normalized.contains("child"));
    }

    #[test]
    fn test_generate_expression_code() {
        let generator = CodeGenerator::new(RuitlFile {
            components: vec![],
            templates: vec![],
            imports: vec![],
        });

        let ast = TemplateAst::Expression("user.name".to_string());
        let result = generator.generate_ast_code(&ast).unwrap();

        let code = result.to_string();
        let normalized = normalize_ws(&code);
        assert!(normalized.contains("user . name"));
        assert!(normalized.contains("Html :: text"));
    }

    #[test]
    fn test_generate_if_code() {
        let generator = CodeGenerator::new(RuitlFile {
            components: vec![],
            templates: vec![],
            imports: vec![],
        });

        let then_branch = TemplateAst::Text("Yes".to_string());
        let else_branch = Some(Box::new(TemplateAst::Text("No".to_string())));

        let result = generator
            .generate_if_code("show_message", &then_branch, &else_branch)
            .unwrap();

        let code = result.to_string();
        assert!(code.contains("if show_message"));
        assert!(code.contains("else"));
    }

    #[test]
    fn test_generate_for_code() {
        let generator = CodeGenerator::new(RuitlFile {
            components: vec![],
            templates: vec![],
            imports: vec![],
        });

        let body = TemplateAst::Element {
            tag: "li".to_string(),
            attributes: vec![],
            children: vec![TemplateAst::Expression("item".to_string())],
            self_closing: false,
        };

        let result = generator.generate_for_code("item", "items", &body).unwrap();

        let code = result.to_string();
        assert!(code.contains("into_iter"));
        assert!(code.contains("map"));
        assert!(code.contains("item"));
    }

    #[test]
    fn test_generate_component_invocation() {
        let generator = CodeGenerator::new(RuitlFile {
            components: vec![],
            templates: vec![],
            imports: vec![],
        });

        let props = vec![
            PropValue {
                name: "text".to_string(),
                value: "\"Click me\"".to_string(),
            },
            PropValue {
                name: "disabled".to_string(),
                value: "false".to_string(),
            },
        ];

        let result = generator
            .generate_component_invocation_code("Button", &props, None)
            .unwrap();

        let code = result.to_string();
        let normalized = normalize_ws(&code);
        assert!(normalized.contains("Button"));
        assert!(normalized.contains("ButtonProps"));
        assert!(normalized.contains("text : \"Click me\""));
        assert!(normalized.contains("disabled : false"));
    }

    #[test]
    fn test_full_generation() {
        let file = RuitlFile {
            components: vec![create_test_component()],
            templates: vec![create_test_template()],
            imports: vec![],
        };

        let mut generator = CodeGenerator::new(file);
        let result = generator.generate().unwrap();

        let code = result.to_string();
        assert!(code.contains("struct ButtonProps"));
        assert!(code.contains("impl Component for Button"));
        assert!(code.contains("fn render"));
    }

    #[test]
    fn test_generics_emit_on_props_and_component_structs() {
        use crate::parser::GenericParam;
        let mut component = ComponentDef {
            name: "Box".to_string(),
            props: vec![PropDef {
                name: "value".to_string(),
                prop_type: "T".to_string(),
                optional: false,
                default_value: None,
            }],
            generics: vec![GenericParam {
                name: "T".to_string(),
                bounds: vec![],
            }],
            leading_comments: vec![],
        };
        // Trigger the "requires matching template" path below: simplest to
        // just test props struct emission here.
        component.generics = vec![GenericParam {
            name: "T".to_string(),
            bounds: vec!["Clone".to_string()],
        }];

        let file = RuitlFile {
            components: vec![component.clone()],
            templates: vec![],
            imports: vec![],
        };
        let mut gen = CodeGenerator::new(file);
        gen.generate_component_definition(&component).unwrap();
        let combined = gen
            .generated_components
            .get(&component.name)
            .expect("component code stored");
        let out = normalize_ws(&combined.to_string());

        assert!(out.contains("pub struct BoxProps < T :"));
        assert!(out.contains("Clone"));
        assert!(out.contains("'static"));
        assert!(out.contains("PhantomData"));
    }
}
