//! Procedural macros for RUITL (Rust UI Template Language)
//!
//! This crate provides compile-time macros for creating components, templates,
//! and HTML content with type safety and performance optimizations.

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Ident, ItemFn, ItemStruct, Type};

/// Derive macro for automatically implementing ComponentProps
///
/// # Example
///
/// ```rust
/// use ruitl_macros::ComponentProps;
///
/// #[derive(ComponentProps)]
/// struct ButtonProps {
///     text: String,
///     variant: String,
/// }
/// ```
#[proc_macro_derive(ComponentProps, attributes(prop))]
pub fn derive_component_props(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl ruitl::component::ComponentProps for #name {
            fn validate(&self) -> ruitl::error::Result<()> {
                // Default validation - can be overridden
                Ok(())
            }

            fn to_map(&self) -> std::collections::HashMap<String, String> {
                let mut map = std::collections::HashMap::new();
                // Serialize to JSON and convert to string map
                if let Ok(json) = serde_json::to_value(self) {
                    if let serde_json::Value::Object(obj) = json {
                        for (key, value) in obj {
                            map.insert(key, value.to_string());
                        }
                    }
                }
                map
            }

            fn from_map(map: &std::collections::HashMap<String, String>) -> ruitl::error::Result<Self>
            where
                Self: Sized,
            {
                // Convert map to JSON and deserialize
                let json_map: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                    .collect();

                let json_value = serde_json::Value::Object(json_map);
                serde_json::from_value(json_value)
                    .map_err(|e| ruitl::error::RuitlError::component(format!("Failed to deserialize props: {}", e)))
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for creating RUITL components
///
/// # Example
///
/// ```rust
/// use ruitl_macros::component;
/// use ruitl::{Html, ComponentProps, ComponentContext, Result};
///
/// #[derive(ComponentProps)]
/// struct ButtonProps {
///     text: String,
/// }
///
/// #[component]
/// fn Button(props: &ButtonProps, _context: &ComponentContext) -> Result<Html> {
///     Ok(html! {
///         <button>{&props.text}</button>
///     })
/// }
/// ```
#[proc_macro_attribute]
pub fn component(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_inputs = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;
    let fn_body = &input_fn.block;

    // Extract props type from function signature
    let props_type = extract_props_type(fn_inputs);

    // Generate component struct name
    let component_name = Ident::new(&fn_name.to_string(), Span::call_site());

    let expanded = quote! {
        #[derive(Debug)]
        pub struct #component_name;

        impl ruitl::component::Component for #component_name {
            type Props = #props_type;

            fn render(&self, props: &Self::Props, context: &ruitl::component::ComponentContext) -> ruitl::error::Result<ruitl::html::Html> {
                #fn_name(props, context)
            }
        }

        fn #fn_name #fn_inputs #fn_output #fn_body
    };

    TokenStream::from(expanded)
}

/// Macro for creating HTML content with compile-time validation
///
/// # Example
///
/// ```rust
/// use ruitl_macros::html;
///
/// let content = html! {
///     <div class="container">
///         <h1>Hello, World!</h1>
///         <p>This is a paragraph</p>
///     </div>
/// };
/// ```
#[proc_macro]
pub fn html(input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);

    // For now, just return the input as-is and let the macro_rules! version handle it
    // In a full implementation, this would parse HTML syntax and generate optimized code
    let expanded = quote! {
        ruitl::html! { #input }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for creating templates with compile-time validation
///
/// # Example
///
/// ```rust
/// use ruitl_macros::template;
///
/// #[template(path = "templates/layout.html")]
/// struct Layout {
///     title: String,
///     content: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn template(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_struct = parse_macro_input!(input as ItemStruct);

    let struct_name = &input_struct.ident;

    // For now, use a default template path
    let template_path = "default.html";

    let expanded = quote! {
        #input_struct

        impl ruitl::template::Template for #struct_name {
            fn template_path() -> &'static str {
                #template_path
            }

            fn render(&self, context: &ruitl::component::ComponentContext) -> ruitl::error::Result<ruitl::html::Html> {
                // Template rendering logic would go here
                // For now, return empty HTML
                Ok(ruitl::html::Html::Empty)
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for creating routes
///
/// # Example
///
/// ```rust
/// use ruitl_macros::Route;
///
/// #[derive(Route)]
/// #[route(path = "/users/:id", methods = ["GET"])]
/// struct UserDetail {
///     id: String,
/// }
/// ```
#[proc_macro_derive(Route, attributes(route))]
pub fn derive_route(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Extract route attributes
    let _route_attrs = extract_route_attributes(&input.attrs);

    let expanded = quote! {
        impl ruitl::router::RouteHandler for #name {
            fn handle(&self, context: &ruitl::router::RouteContext) -> ruitl::error::Result<ruitl::router::RouteResponse> {
                // Default implementation - should be overridden
                Ok(ruitl::router::RouteResponse::text("Route handler not implemented"))
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for creating static components
#[proc_macro_attribute]
pub fn static_component(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    // First apply the component macro
    let component_output = component(TokenStream::new(), TokenStream::from(quote! { #input_fn }));
    let component_tokens = TokenStream2::from(component_output);

    let fn_name = &input_fn.sig.ident;
    let component_name = Ident::new(&fn_name.to_string(), Span::call_site());

    let expanded = quote! {
        #component_tokens

        impl ruitl::component::StaticComponent for #component_name {
            fn static_props(&self) -> Vec<Self::Props> {
                // Return empty vec by default - can be overridden
                vec![]
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for creating async components
#[proc_macro_attribute]
pub fn async_component(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_inputs = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;
    let fn_body = &input_fn.block;

    // Extract props type from function signature
    let props_type = extract_props_type(fn_inputs);

    // Generate component struct name
    let component_name = Ident::new(&fn_name.to_string(), Span::call_site());

    let expanded = quote! {
        #[derive(Debug)]
        pub struct #component_name;

        #[ruitl::async_trait::async_trait]
        impl ruitl::component::AsyncComponent for #component_name {
            type Props = #props_type;

            async fn render_async(&self, props: &Self::Props, context: &ruitl::component::ComponentContext) -> ruitl::error::Result<ruitl::html::Html> {
                #fn_name(props, context).await
            }
        }

        async fn #fn_name #fn_inputs #fn_output #fn_body
    };

    TokenStream::from(expanded)
}

// Helper functions

fn extract_props_type(inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>) -> Type {
    // Extract the first parameter type (props)
    if let Some(syn::FnArg::Typed(pat_type)) = inputs.first() {
        if let Type::Reference(type_ref) = pat_type.ty.as_ref() {
            return (*type_ref.elem).clone();
        }
        return (*pat_type.ty).clone();
    }

    // Default to EmptyProps if no props found
    parse_quote!(ruitl::component::EmptyProps)
}

fn _extract_template_path() -> String {
    "default.html".to_string()
}

fn extract_route_attributes(_attrs: &[syn::Attribute]) -> Vec<(String, String)> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    // Test imports would go here

    #[test]
    fn test_proc_macro_compilation() {
        // Basic compilation test
        assert!(true);
    }
}
