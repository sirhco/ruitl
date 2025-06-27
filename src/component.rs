//! Component system for RUITL
//!
//! This module provides the core component abstraction that allows users to create
//! reusable UI components with props, state, and lifecycle methods.

use crate::error::{Result, RuitlError};
use crate::html::Html;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;

/// Trait for component properties
pub trait ComponentProps: Debug + Clone + Send + Sync + 'static {
    /// Validate the props
    fn validate(&self) -> Result<()> {
        Ok(())
    }

    /// Convert props to a HashMap for serialization
    fn to_map(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Create props from a HashMap
    fn from_map(_map: &HashMap<String, String>) -> Result<Self>
    where
        Self: Sized,
    {
        Err(RuitlError::component(
            "from_map not implemented for this component",
        ))
    }
}

/// Empty props for components that don't need properties
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmptyProps;

impl ComponentProps for EmptyProps {}

/// Context passed to components during rendering
#[derive(Debug)]
pub struct ComponentContext {
    /// Request path (for server-side rendering)
    pub path: Option<String>,
    /// Query parameters
    pub query: HashMap<String, String>,
    /// Headers (for server-side rendering)
    pub headers: HashMap<String, String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Custom data
    pub data: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl Clone for ComponentContext {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            query: self.query.clone(),
            headers: self.headers.clone(),
            env: self.env.clone(),
            data: HashMap::new(), // Cannot clone Box<dyn Any>, so start with empty
        }
    }
}

impl Default for ComponentContext {
    fn default() -> Self {
        Self {
            path: None,
            query: HashMap::new(),
            headers: HashMap::new(),
            env: HashMap::new(),
            data: HashMap::new(),
        }
    }
}

impl ComponentContext {
    /// Create a new context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the request path
    pub fn with_path<S: Into<String>>(mut self, path: S) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Add a query parameter
    pub fn with_query<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.query.insert(key.into(), value.into());
        self
    }

    /// Add a header
    pub fn with_header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add environment variable
    pub fn with_env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Add custom data
    pub fn with_data<K: Into<String>, V: Any + Send + Sync>(mut self, key: K, value: V) -> Self {
        self.data.insert(key.into(), Box::new(value));
        self
    }

    /// Get query parameter
    pub fn get_query(&self, key: &str) -> Option<&String> {
        self.query.get(key)
    }

    /// Get header
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Get environment variable
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.env.get(key)
    }

    /// Get custom data
    pub fn get_data(&self, key: &str) -> Option<&Box<dyn Any + Send + Sync>> {
        self.data.get(key)
    }
}

/// Main trait for RUITL components
pub trait Component: Debug + Send + Sync + 'static {
    /// The props type for this component
    type Props: ComponentProps;

    /// Render the component to HTML
    fn render(&self, props: &Self::Props, context: &ComponentContext) -> Result<Html>;

    /// Get the component name (used for debugging and error messages)
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Validate props before rendering
    fn validate_props(&self, props: &Self::Props) -> Result<()> {
        props.validate()
    }

    /// Called before rendering (lifecycle hook)
    fn before_render(&self, _props: &Self::Props, _context: &ComponentContext) -> Result<()> {
        Ok(())
    }

    /// Called after rendering (lifecycle hook)
    fn after_render(&self, _props: &Self::Props, _context: &ComponentContext) -> Result<()> {
        Ok(())
    }

    /// Generate CSS for this component (optional)
    fn styles(&self) -> Option<String> {
        None
    }

    /// Generate JavaScript for this component (optional, for progressive enhancement)
    fn scripts(&self) -> Option<String> {
        None
    }
}

/// Trait for components that can be rendered statically (at build time)
pub trait StaticComponent: Component {
    /// Render the component with static props (no context needed)
    fn render_static(&self, props: &Self::Props) -> Result<Html> {
        let context = ComponentContext::default();
        self.render(props, &context)
    }

    /// Get static props for this component (for static site generation)
    fn static_props(&self) -> Vec<Self::Props> {
        vec![]
    }
}

/// Trait for async components (useful for data fetching)
#[async_trait::async_trait]
pub trait AsyncComponent: Debug + Send + Sync + 'static {
    /// The props type for this component
    type Props: ComponentProps;

    /// Render the component asynchronously
    async fn render_async(&self, props: &Self::Props, context: &ComponentContext) -> Result<Html>;

    /// Get the component name
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Validate props before rendering
    fn validate_props(&self, props: &Self::Props) -> Result<()> {
        props.validate()
    }

    /// Called before rendering (async lifecycle hook)
    async fn before_render_async(
        &self,
        _props: &Self::Props,
        _context: &ComponentContext,
    ) -> Result<()> {
        Ok(())
    }

    /// Called after rendering (async lifecycle hook)
    async fn after_render_async(
        &self,
        _props: &Self::Props,
        _context: &ComponentContext,
    ) -> Result<()> {
        Ok(())
    }

    /// Generate CSS for this component
    fn styles(&self) -> Option<String> {
        None
    }

    /// Generate JavaScript for this component
    fn scripts(&self) -> Option<String> {
        None
    }
}

/// Component registry for managing registered components
#[derive(Debug, Default)]
pub struct ComponentRegistry {
    components: HashMap<String, Box<dyn Any + Send + Sync>>,
    styles: HashMap<String, String>,
    scripts: HashMap<String, String>,
}

impl Clone for ComponentRegistry {
    fn clone(&self) -> Self {
        Self {
            components: HashMap::new(), // Cannot clone Box<dyn Any>, so start with empty
            styles: self.styles.clone(),
            scripts: self.scripts.clone(),
        }
    }
}

impl ComponentRegistry {
    /// Create a new component registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a component
    pub fn register<C>(&mut self, name: &str, component: C)
    where
        C: Component + 'static,
    {
        if let Some(styles) = component.styles() {
            self.styles.insert(name.to_string(), styles);
        }
        if let Some(scripts) = component.scripts() {
            self.scripts.insert(name.to_string(), scripts);
        }
        self.components
            .insert(name.to_string(), Box::new(component));
    }

    /// Get a component by name
    pub fn get<C>(&self, name: &str) -> Option<&C>
    where
        C: Component + 'static,
    {
        self.components
            .get(name)
            .and_then(|c| c.downcast_ref::<C>())
    }

    /// Get all component styles
    pub fn get_styles(&self) -> &HashMap<String, String> {
        &self.styles
    }

    /// Get all component scripts
    pub fn get_scripts(&self) -> &HashMap<String, String> {
        &self.scripts
    }

    /// Get combined CSS for all components
    pub fn combined_styles(&self) -> String {
        self.styles.values().cloned().collect::<Vec<_>>().join("\n")
    }

    /// Get combined JavaScript for all components
    pub fn combined_scripts(&self) -> String {
        self.scripts
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// List all registered component names
    pub fn list_components(&self) -> Vec<String> {
        self.components.keys().cloned().collect()
    }
}

/// Helper struct for component rendering
pub struct ComponentRenderer {
    registry: ComponentRegistry,
}

impl ComponentRenderer {
    /// Create a new component renderer
    pub fn new() -> Self {
        Self {
            registry: ComponentRegistry::new(),
        }
    }

    /// Create with a registry
    pub fn with_registry(registry: ComponentRegistry) -> Self {
        Self { registry }
    }

    /// Register a component
    pub fn register<C>(&mut self, name: &str, component: C)
    where
        C: Component + 'static,
    {
        self.registry.register(name, component);
    }

    /// Render a component by name
    pub fn render<C>(
        &self,
        name: &str,
        props: &C::Props,
        context: &ComponentContext,
    ) -> Result<Html>
    where
        C: Component + 'static,
    {
        let component = self
            .registry
            .get::<C>(name)
            .ok_or_else(|| RuitlError::component(format!("Component '{}' not found", name)))?;

        component.validate_props(props)?;
        component.before_render(props, context)?;
        let html = component.render(props, context)?;
        component.after_render(props, context)?;

        Ok(html)
    }

    /// Get the registry
    pub fn registry(&self) -> &ComponentRegistry {
        &self.registry
    }

    /// Get mutable registry
    pub fn registry_mut(&mut self) -> &mut ComponentRegistry {
        &mut self.registry
    }
}

impl Default for ComponentRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for implementing common component patterns
#[macro_export]
macro_rules! impl_component {
    ($name:ident, $props:ty, $render_fn:expr) => {
        #[derive(Debug)]
        pub struct $name;

        impl Component for $name {
            type Props = $props;

            fn render(&self, props: &Self::Props, context: &ComponentContext) -> Result<Html> {
                $render_fn(props, context)
            }

            fn name(&self) -> &'static str {
                stringify!($name)
            }
        }
    };
}

/// Macro for implementing static components
#[macro_export]
macro_rules! impl_static_component {
    ($name:ident, $props:ty, $render_fn:expr, $static_props:expr) => {
        impl_component!($name, $props, $render_fn);

        impl StaticComponent for $name {
            fn static_props(&self) -> Vec<Self::Props> {
                $static_props()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::{div, text, Html};

    #[derive(Debug, Clone)]
    struct TestProps {
        message: String,
    }

    impl ComponentProps for TestProps {}

    struct TestComponent;

    impl Component for TestComponent {
        type Props = TestProps;

        fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
            Ok(Html::Element(div().text(&props.message)))
        }
    }

    impl Debug for TestComponent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestComponent")
        }
    }

    #[test]
    fn test_component_render() {
        let component = TestComponent;
        let props = TestProps {
            message: "Hello, World!".to_string(),
        };
        let context = ComponentContext::new();

        let html = component.render(&props, &context).unwrap();
        assert_eq!(html.render(), "<div>Hello, World!</div>");
    }

    #[test]
    fn test_component_registry() {
        let mut registry = ComponentRegistry::new();
        registry.register("test", TestComponent);

        let component = registry.get::<TestComponent>("test");
        assert!(component.is_some());

        let components = registry.list_components();
        assert!(components.contains(&"test".to_string()));
    }

    #[test]
    fn test_component_renderer() {
        let mut renderer = ComponentRenderer::new();
        renderer.register("test", TestComponent);

        let props = TestProps {
            message: "Test message".to_string(),
        };
        let context = ComponentContext::new();

        let html = renderer
            .render::<TestComponent>("test", &props, &context)
            .unwrap();
        assert_eq!(html.render(), "<div>Test message</div>");
    }

    #[test]
    fn test_component_context() {
        let context = ComponentContext::new()
            .with_path("/test")
            .with_query("param", "value")
            .with_header("content-type", "text/html")
            .with_env("NODE_ENV", "production");

        assert_eq!(context.path, Some("/test".to_string()));
        assert_eq!(context.get_query("param"), Some(&"value".to_string()));
        assert_eq!(
            context.get_header("content-type"),
            Some(&"text/html".to_string())
        );
        assert_eq!(context.get_env("NODE_ENV"), Some(&"production".to_string()));
    }

    #[test]
    fn test_empty_props() {
        let props = EmptyProps;
        assert!(props.validate().is_ok());
        assert!(props.to_map().is_empty());
    }

    #[tokio::test]
    async fn test_async_component() {
        struct AsyncTestComponent;

        impl Debug for AsyncTestComponent {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "AsyncTestComponent")
            }
        }

        #[async_trait::async_trait]
        impl AsyncComponent for AsyncTestComponent {
            type Props = TestProps;

            async fn render_async(
                &self,
                props: &Self::Props,
                _context: &ComponentContext,
            ) -> Result<Html> {
                // Simulate async work
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                Ok(Html::Element(div().text(&props.message)))
            }
        }

        let component = AsyncTestComponent;
        let props = TestProps {
            message: "Async Hello!".to_string(),
        };
        let context = ComponentContext::new();

        let html = component.render_async(&props, &context).await.unwrap();
        assert_eq!(html.render(), "<div>Async Hello!</div>");
    }
}
