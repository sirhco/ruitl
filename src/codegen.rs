//! RUITL Code Generator
//!
//! Converts parsed .ruitl templates into optimized Rust code that uses the RUITL runtime library.

use crate::error::{Result, RuitlError};
use crate::parser::{
    Attribute, AttributeValue, ComponentDef, ImportDef, MatchArm, ParamDef, PropDef, PropValue,
    RuitlFile, TemplateAst, TemplateDef,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::collections::HashMap;
use syn::{parse_str, Expr, Type};

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
        let components: Vec<_> = self.generated_components.values().collect();

        Ok(quote! {
            use ruitl::prelude::*;
            use ruitl::html::*;
            use std::collections::HashMap;

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
        let path = &import.path;

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
        let props_name = format_ident!("{}Props", component.name);

        // Generate props struct
        let props_struct = self.generate_props_struct(component)?;

        // Generate component struct
        let component_struct = quote! {
            #[derive(Debug)]
            pub struct #component_name;
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

        if component.props.is_empty() {
            return Ok(quote! {
                pub type #props_name = ruitl::component::EmptyProps;
            });
        }

        let mut fields = Vec::new();
        let mut field_validations = Vec::new();

        for prop in &component.props {
            let field_name = format_ident!("{}", prop.name);
            let field_type: Type = parse_str(&prop.prop_type).map_err(|e| {
                RuitlError::codegen(format!("Invalid type '{}': {}", prop.prop_type, e))
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

        Ok(quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            pub struct #props_name {
                #(#fields),*
            }

            impl ruitl::component::ComponentProps for #props_name {
                fn validate(&self) -> ruitl::error::Result<()> {
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
                RuitlError::codegen(format!(
                    "No component definition found for template '{}'",
                    template.name
                ))
            })?;

        // Generate prop bindings for local access
        let prop_bindings = self.generate_prop_bindings(component)?;

        // Generate the render method body
        let render_body = self.generate_ast_code(&template.body)?;

        // Create the Component implementation
        let impl_block = quote! {
            impl ruitl::component::Component for #component_name {
                type Props = #props_name;

                fn render(&self, props: &Self::Props, context: &ruitl::component::ComponentContext) -> ruitl::error::Result<ruitl::html::Html> {
                    #prop_bindings
                    Ok(#render_body)
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
            return Err(RuitlError::codegen(format!(
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
                    Ok(quote! { ruitl::html::Html::Empty })
                } else {
                    Ok(quote! { ruitl::html::Html::text(#text) })
                }
            }

            TemplateAst::Expression(expr) => {
                let transformed_expr = self.transform_variable_access(expr);
                let expr: Expr = parse_str(&transformed_expr).map_err(|e| {
                    RuitlError::codegen(format!("Invalid expression '{}': {}", transformed_expr, e))
                })?;
                Ok(quote! { ruitl::html::Html::text(&format!("{}", #expr)) })
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

            TemplateAst::Component { name, props } => {
                self.generate_component_invocation_code(name, props)
            }

            TemplateAst::Fragment(nodes) => {
                let node_codes: Result<Vec<_>> = nodes
                    .iter()
                    .map(|node| self.generate_ast_code(node))
                    .collect();
                let node_codes = node_codes?;

                Ok(quote! {
                    ruitl::html::Html::fragment(vec![#(#node_codes),*])
                })
            }

            TemplateAst::Raw(html) => Ok(quote! { ruitl::html::Html::raw(#html) }),
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
            quote! { ruitl::html::HtmlElement::self_closing(#tag_name) }
        } else {
            quote! { ruitl::html::HtmlElement::new(#tag_name) }
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

        Ok(quote! { ruitl::html::Html::Element(#element_code) })
    }

    /// Generate code for an HTML attribute
    fn generate_attribute_code(&self, attr: &Attribute) -> Result<TokenStream> {
        let attr_name = &attr.name;

        match &attr.value {
            AttributeValue::Static(value) => Ok(quote! { attr(#attr_name, #value) }),

            AttributeValue::Expression(expr) => {
                let expr: Expr = parse_str(expr).map_err(|e| {
                    RuitlError::codegen(format!("Invalid attribute expression '{}': {}", expr, e))
                })?;
                Ok(quote! { attr(#attr_name, &format!("{}", #expr)) })
            }

            AttributeValue::Conditional(condition) => {
                let condition: Expr = parse_str(condition).map_err(|e| {
                    RuitlError::codegen(format!(
                        "Invalid conditional expression '{}': {}",
                        condition, e
                    ))
                })?;
                Ok(quote! {
                    attr_if(#attr_name, #condition, #attr_name)
                })
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
            RuitlError::codegen(format!(
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
                    ruitl::html::Html::Empty
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
        let var_name = format_ident!("{}", variable);
        let transformed_iterable = self.transform_variable_access(iterable);
        let iterable: Expr = parse_str(&transformed_iterable).map_err(|e| {
            RuitlError::codegen(format!(
                "Invalid for iterable '{}': {}",
                transformed_iterable, e
            ))
        })?;

        let body_code = self.generate_ast_code(body)?;

        Ok(quote! {
            ruitl::html::Html::fragment(
                #iterable
                    .into_iter()
                    .map(|#var_name| #body_code)
                    .collect::<Vec<_>>()
            )
        })
    }

    /// Generate code for match statement
    fn generate_match_code(&self, expression: &str, arms: &[MatchArm]) -> Result<TokenStream> {
        let expr: Expr = parse_str(expression).map_err(|e| {
            RuitlError::codegen(format!("Invalid match expression '{}': {}", expression, e))
        })?;

        let mut match_arms = Vec::new();

        for arm in arms {
            let pattern = &arm.pattern;
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

    /// Generate code for component invocation
    /// Generate local bindings for props with proper Option handling
    fn generate_prop_bindings(&self, component: &ComponentDef) -> Result<TokenStream> {
        let mut bindings = Vec::new();

        for prop in &component.props {
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
    ) -> Result<TokenStream> {
        let component_ident = format_ident!("{}", name);
        let props_ident = format_ident!("{}Props", name);

        let mut prop_assignments = Vec::new();

        for prop in props {
            let prop_name = format_ident!("{}", prop.name);
            let prop_value: Expr = parse_str(&prop.value).map_err(|e| {
                RuitlError::codegen(format!("Invalid prop value '{}': {}", prop.value, e))
            })?;

            prop_assignments.push(quote! {
                #prop_name: #prop_value
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
}

/// Helper extension for HtmlElement to support conditional attributes
pub trait HtmlElementExt {
    fn attr_if(self, name: &str, condition: bool, value: &str) -> Self;
}

impl HtmlElementExt for crate::html::HtmlElement {
    fn attr_if(self, name: &str, condition: bool, value: &str) -> Self {
        if condition {
            self.attr(name, value)
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ComponentDef, PropDef, RuitlFile, TemplateAst, TemplateDef};

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

        assert!(code.contains("struct ButtonProps"));
        assert!(code.contains("text: String"));
        assert!(code.contains("disabled: Option<bool>"));
        assert!(code.contains("impl ruitl::component::ComponentProps"));
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
        assert!(code.contains("HtmlElement::new"));
        assert!(code.contains("attr"));
        assert!(code.contains("child"));
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
        assert!(code.contains("user.name"));
        assert!(code.contains("Html::text"));
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
            .generate_component_invocation_code("Button", &props)
            .unwrap();

        let code = result.to_string();
        assert!(code.contains("Button"));
        assert!(code.contains("ButtonProps"));
        assert!(code.contains("text: \"Click me\""));
        assert!(code.contains("disabled: false"));
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
}
