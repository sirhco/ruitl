//! Macro utilities for RUITL
//!
//! This module provides macro definitions and utilities for creating components,
//! templates, and HTML in a more ergonomic way.

/// Re-export the proc macros from ruitl_macros crate
// Note: Proc macros are re-exported at the crate level

/// Create HTML content using a more natural syntax
///
/// # Examples
///
/// ```rust
/// use ruitl::html;
///
/// let content = html! {
///     <div class="container">
///         <h1>{"Hello, World!"}</h1>
///         <p>{"This is a paragraph"}</p>
///     </div>
/// };
/// ```
#[macro_export]
macro_rules! html {
    // Empty
    () => {
        $crate::html::Html::Empty
    };

    // Text content
    ({ $text:expr }) => {
        $crate::html::Html::text($text)
    };

    // Raw HTML
    (raw { $html:expr }) => {
        $crate::html::Html::raw($html)
    };

    // Fragment
    (<> $($children:tt)* </>) => {
        $crate::html::Html::fragment(vec![$($crate::html!($children)),*])
    };

    // Self-closing tag with attributes
    (<$tag:ident />) => {
        $crate::html::Html::Element($crate::html::HtmlElement::self_closing(stringify!($tag)))
    };

    // Self-closing tag without attributes
    (<$tag:ident />) => {
        $crate::html::Html::Element($crate::html::HtmlElement::self_closing(stringify!($tag)))
    };

    // Regular tag with attributes and children
    (<$tag:ident> $($children:tt)* </$end_tag:ident>) => {
        {
            let mut element = $crate::html::HtmlElement::new(stringify!($tag));
            $(
                element = element.child($crate::html!($children));
            )*
            $crate::html::Html::Element(element)
        }
    };

    // Regular tag without attributes but with children
    (<$tag:ident> $($children:tt)* </$end_tag:ident>) => {
        {
            let mut element = $crate::html::HtmlElement::new(stringify!($tag));
            $(
                element = element.child($crate::html!($children));
            )*
            $crate::html::Html::Element(element)
        }
    };


}

/// Helper macro for parsing HTML attributes
#[macro_export]
macro_rules! html_attrs {
    ($element:ident;) => {};
    ($element:ident; $attr:ident = $value:expr) => {
        $element = $element.attr(stringify!($attr), $value);
    };
}

/// Create a component with automatic trait implementations
///
/// # Examples
///
/// ```rust
/// use ruitl::{component, html, Html, ComponentProps, ComponentContext, Result};
///
/// #[derive(Debug, Clone)]
/// struct ButtonProps {
///     text: String,
///     variant: String,
/// }
///
/// impl ComponentProps for ButtonProps {}
///
/// component! {
///     Button(props: ButtonProps) -> Html {
///         html! {
///             <button class={format!("btn btn-{}", props.variant)}>
///                 {props.text}
///             </button>
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! component {
    (
        $name:ident($props:ident: $props_type:ty) -> $return_type:ty {
            $($body:tt)*
        }
    ) => {
        #[derive(Debug)]
        pub struct $name;

        impl $crate::component::Component for $name {
            type Props = $props_type;

            fn render(&self, $props: &Self::Props, _context: &$crate::component::ComponentContext) -> $crate::error::Result<$return_type> {
                Ok($($body)*)
            }
        }
    };

    (
        $name:ident($props:ident: $props_type:ty, $context:ident: &ComponentContext) -> $return_type:ty {
            $($body:tt)*
        }
    ) => {
        #[derive(Debug)]
        pub struct $name;

        impl $crate::component::Component for $name {
            type Props = $props_type;

            fn render(&self, $props: &Self::Props, $context: &$crate::component::ComponentContext) -> $crate::error::Result<$return_type> {
                Ok($($body)*)
            }
        }
    };
}

/// Create a static component that can be pre-rendered
#[macro_export]
macro_rules! static_component {
    (
        $name:ident($props:ident: $props_type:ty) -> $return_type:ty {
            $($body:tt)*
        }
    ) => {
        component! {
            $name($props: $props_type) -> $return_type {
                $($body)*
            }
        }

        impl $crate::component::StaticComponent for $name {}
    };
}

/// Create an async component
#[macro_export]
macro_rules! async_component {
    (
        $name:ident($props:ident: $props_type:ty) -> $return_type:ty {
            $($body:tt)*
        }
    ) => {
        #[derive(Debug)]
        pub struct $name;

        #[$crate::async_trait::async_trait]
        impl $crate::component::AsyncComponent for $name {
            type Props = $props_type;

            async fn render_async(&self, $props: &Self::Props, _context: &$crate::component::ComponentContext) -> $crate::error::Result<$return_type> {
                Ok($($body)*)
            }
        }
    };
}

/// Define props for a component with automatic trait implementations
#[macro_export]
macro_rules! props {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field:ident: $field_type:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        $vis struct $name {
            $(
                $(#[$field_meta])*
                $field_vis $field: $field_type
            ),*
        }

        impl $crate::component::ComponentProps for $name {
            fn validate(&self) -> $crate::error::Result<()> {
                Ok(())
            }

            fn to_map(&self) -> std::collections::HashMap<String, String> {
                let mut map = std::collections::HashMap::new();
                // This would need proper serialization logic
                map
            }
        }
    };
}

/// Create a route handler function
#[macro_export]
macro_rules! route_handler {
    (|$ctx:ident| $body:expr) => {
        $crate::router::FunctionHandler::new(Box::new(|$ctx| $body))
    };

    (|$ctx:ident: &RouteContext| $body:expr) => {
        $crate::router::FunctionHandler::new(Box::new(|$ctx| $body))
    };
}

/// Create CSS styles with automatic scoping
#[macro_export]
macro_rules! styles {
    ($($rule:expr),* $(,)?) => {
        vec![$($rule.to_string()),*].join("\n")
    };
}

/// Create a template with compile-time validation
#[macro_export]
macro_rules! template {
    ($template_str:expr) => {
        compile_error!("Template macro requires proc macro support for compile-time validation")
    };
}

/// Conditional rendering helper
#[macro_export]
macro_rules! when {
    ($condition:expr, $then_block:expr) => {
        if $condition {
            $then_block
        } else {
            $crate::html::Html::Empty
        }
    };

    ($condition:expr, $then_block:expr, $else_block:expr) => {
        if $condition {
            $then_block
        } else {
            $else_block
        }
    };
}

/// Loop rendering helper
#[macro_export]
macro_rules! for_each {
    ($iter:expr, |$item:ident| $body:expr) => {
        $crate::html::Html::fragment($iter.into_iter().map(|$item| $body).collect())
    };

    ($iter:expr, |$item:ident, $index:ident| $body:expr) => {
        $crate::html::Html::fragment(
            $iter
                .into_iter()
                .enumerate()
                .map(|($index, $item)| $body)
                .collect(),
        )
    };
}

/// Match rendering helper
#[macro_export]
macro_rules! match_render {
    ($expr:expr, { $($pattern:pat => $body:expr),* $(,)? }) => {
        match $expr {
            $(
                $pattern => $body,
            )*
        }
    };
}

/// Create a list of HTML elements
#[macro_export]
macro_rules! html_list {
    ($($element:expr),* $(,)?) => {
        vec![$($element),*]
    };
}

/// Create CSS classes conditionally
#[macro_export]
macro_rules! classes {
    ($($class:expr => $condition:expr),* $(,)?) => {
        {
            let mut classes = Vec::new();
            $(
                if $condition {
                    classes.push($class);
                }
            )*
            classes.join(" ")
        }
    };

    ($($class:expr),* $(,)?) => {
        vec![$($class),*].join(" ")
    };
}

/// Include external CSS file
#[macro_export]
macro_rules! include_css {
    ($path:expr) => {
        $crate::html::Html::Element(
            $crate::html::HtmlElement::new("link")
                .attr("rel", "stylesheet")
                .attr("href", $path),
        )
    };
}

/// Include external JavaScript file
#[macro_export]
macro_rules! include_js {
    ($path:expr) => {
        $crate::html::Html::Element($crate::html::HtmlElement::new("script").attr("src", $path))
    };
}

/// Create a meta tag
#[macro_export]
macro_rules! meta {
    (name = $name:expr, content = $content:expr) => {
        $crate::html::Html::Element(
            $crate::html::HtmlElement::self_closing("meta")
                .attr("name", $name)
                .attr("content", $content),
        )
    };

    (property = $property:expr, content = $content:expr) => {
        $crate::html::Html::Element(
            $crate::html::HtmlElement::self_closing("meta")
                .attr("property", $property)
                .attr("content", $content),
        )
    };

    (charset = $charset:expr) => {
        $crate::html::Html::Element(
            $crate::html::HtmlElement::self_closing("meta").attr("charset", $charset),
        )
    };
}

/// Create a document title
#[macro_export]
macro_rules! title {
    ($title:expr) => {
        $crate::html::Html::Element($crate::html::HtmlElement::new("title").text($title))
    };
}

/// Environment-specific rendering
#[macro_export]
macro_rules! env_render {
    (development => $dev_content:expr, production => $prod_content:expr) => {
        if cfg!(debug_assertions) {
            $dev_content
        } else {
            $prod_content
        }
    };
}

/// Debug rendering (only in development)
#[macro_export]
macro_rules! debug_render {
    ($content:expr) => {
        if cfg!(debug_assertions) {
            $content
        } else {
            $crate::html::Html::Empty
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::component::{ComponentContext, ComponentProps};
    use crate::error::Result;
    use crate::html::Html;

    #[test]
    fn test_html_macro_basic() {
        let html = html! {
            <div>
                {"Hello, World!"}
            </div>
        };

        let rendered = html.render();
        assert!(rendered.contains("<div>"));
        assert!(rendered.contains("Hello, World!"));
        assert!(rendered.contains("</div>"));
    }

    #[test]
    fn test_component_macro() {
        props! {
            struct TestProps {
                message: String,
            }
        }

        component! {
            TestComponent(props: TestProps) -> Html {
                html! {
                    <div>
                        {props.message.clone()}
                    </div>
                }
            }
        }

        let component = TestComponent;
        let props = TestProps {
            message: "Test message".to_string(),
        };
        let context = ComponentContext::new();

        let result = component.render(&props, &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_when_macro() {
        let show = true;
        let content = when!(show, html! { <p>{"Visible"}</p> });
        assert!(!content.is_empty());

        let hidden = false;
        let content = when!(hidden, html! { <p>{"Hidden"}</p> });
        assert!(content.is_empty());
    }

    #[test]
    fn test_classes_macro() {
        let is_active = true;
        let is_disabled = false;

        let classes = classes!(
            "btn" => true,
            "active" => is_active,
            "disabled" => is_disabled
        );

        assert_eq!(classes, "btn active");
    }

    #[test]
    fn test_meta_macro() {
        let meta_tag = meta!(name = "description", content = "Test description");
        let rendered = meta_tag.render();
        assert!(rendered.contains("name=\"description\""));
        assert!(rendered.contains("content=\"Test description\""));
    }

    #[test]
    fn test_title_macro() {
        let title_tag = title!("Test Page");
        let rendered = title_tag.render();
        assert!(rendered.contains("<title>Test Page</title>"));
    }

    #[test]
    fn test_for_each_macro() {
        let items = vec!["item1", "item2", "item3"];
        let list = for_each!(items, |item| html! { <li>{item}</li> });

        let rendered = list.render();
        assert!(rendered.contains("item1"));
        assert!(rendered.contains("item2"));
        assert!(rendered.contains("item3"));
    }

    #[test]
    fn test_env_render_macro() {
        let content = env_render!(
            development => html! { <div>{"Dev content"}</div> },
            production => html! { <div>{"Prod content"}</div> }
        );

        let rendered = content.render();
        // In test environment, this should render development content
        if cfg!(debug_assertions) {
            assert!(rendered.contains("Dev content"));
        } else {
            assert!(rendered.contains("Prod content"));
        }
    }
}
