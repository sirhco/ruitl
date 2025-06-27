//! Router for handling URL routing and navigation in RUITL applications
//!
//! This module provides URL routing capabilities for both server-side and client-side
//! navigation, with support for dynamic routes, parameters, and middleware.

use crate::component::{Component, ComponentContext, ComponentProps};
use crate::error::{Result, RuitlError};
use crate::html::Html;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// Main router for handling URL routing
#[derive(Clone)]
pub struct Router {
    routes: Vec<Route>,
    not_found_handler: Option<Arc<dyn RouteHandler>>,
    middleware: Vec<Arc<dyn Middleware>>,
    base_path: String,
}

/// Route definition
#[derive(Clone)]
pub struct Route {
    /// Route pattern (e.g., "/users/:id", "/blog/*path")
    pub pattern: String,
    /// HTTP methods this route handles
    pub methods: Vec<HttpMethod>,
    /// Route handler
    pub handler: Arc<dyn RouteHandler>,
    /// Route-specific middleware
    pub middleware: Vec<Arc<dyn Middleware>>,
    /// Route metadata
    pub metadata: RouteMetadata,
}

/// HTTP methods
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
    Connect,
    Trace,
}

/// Route metadata
#[derive(Debug, Clone, Default)]
pub struct RouteMetadata {
    /// Route name for reverse routing
    pub name: Option<String>,
    /// Route description
    pub description: Option<String>,
    /// Custom metadata
    pub data: HashMap<String, String>,
}

/// Route matching result
#[derive(Clone)]
pub struct RouteMatch {
    /// Matched route
    pub route: Route,
    /// Extracted parameters
    pub params: HashMap<String, String>,
    /// Query parameters
    pub query: HashMap<String, String>,
    /// Matched path segments
    pub segments: Vec<String>,
}

/// Route context passed to handlers
#[derive(Debug, Clone)]
pub struct RouteContext {
    /// Request path
    pub path: String,
    /// HTTP method
    pub method: HttpMethod,
    /// Route parameters
    pub params: HashMap<String, String>,
    /// Query parameters
    pub query: HashMap<String, String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (for POST, PUT, etc.)
    pub body: Option<Vec<u8>>,
    /// Component context
    pub component_context: ComponentContext,
    /// Custom context data
    pub data: HashMap<String, serde_json::Value>,
}

/// Route handler trait
pub trait RouteHandler: Send + Sync {
    /// Handle the route
    fn handle(&self, context: &RouteContext) -> Result<RouteResponse>;
}

/// Route response
#[derive(Debug, Clone)]
pub struct RouteResponse {
    /// Response status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: RouteResponseBody,
}

/// Route response body types
#[derive(Debug, Clone)]
pub enum RouteResponseBody {
    /// HTML content
    Html(Html),
    /// JSON content
    Json(serde_json::Value),
    /// Plain text
    Text(String),
    /// Binary data
    Binary(Vec<u8>),
    /// Redirect
    Redirect(String),
    /// Empty response
    Empty,
}

/// Middleware trait for request/response processing
pub trait Middleware: Debug + Send + Sync {
    /// Process request before routing
    fn before_route(&self, context: &mut RouteContext) -> Result<()> {
        Ok(())
    }

    /// Process response after routing
    fn after_route(&self, context: &RouteContext, response: &mut RouteResponse) -> Result<()> {
        Ok(())
    }
}

/// Component-based route handler
pub struct ComponentHandler<C, P>
where
    C: Component<Props = P> + 'static,
    P: ComponentProps,
{
    component: C,
    props_extractor: Box<dyn Fn(&RouteContext) -> Result<P> + Send + Sync>,
}

/// Function-based route handler
pub struct FunctionHandler {
    handler: Box<dyn Fn(&RouteContext) -> Result<RouteResponse> + Send + Sync>,
}

/// Route builder for fluent route definition
pub struct RouteBuilder {
    pattern: String,
    methods: Vec<HttpMethod>,
    handler: Option<Arc<dyn RouteHandler>>,
    middleware: Vec<Arc<dyn Middleware>>,
    metadata: RouteMetadata,
}

/// Router builder for fluent router configuration
pub struct RouterBuilder {
    routes: Vec<Route>,
    not_found_handler: Option<Arc<dyn RouteHandler>>,
    middleware: Vec<Arc<dyn Middleware>>,
    base_path: String,
}

impl Router {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            not_found_handler: None,
            middleware: Vec::new(),
            base_path: String::new(),
        }
    }

    /// Create a router builder
    pub fn builder() -> RouterBuilder {
        RouterBuilder::new()
    }

    /// Add a route to the router
    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    /// Set the 404 not found handler
    pub fn set_not_found_handler<H>(&mut self, handler: H)
    where
        H: RouteHandler + 'static,
    {
        self.not_found_handler = Some(Arc::new(handler));
    }

    /// Add global middleware
    pub fn add_middleware<M>(&mut self, middleware: M)
    where
        M: Middleware + 'static,
    {
        self.middleware.push(Arc::new(middleware));
    }

    /// Set base path for all routes
    pub fn set_base_path<S: Into<String>>(&mut self, base_path: S) {
        self.base_path = base_path.into();
    }

    /// Route a request
    pub fn route(&self, path: &str, method: HttpMethod) -> Result<RouteResponse> {
        let mut context = self.create_route_context(path, method)?;

        // Apply global middleware (before routing)
        for middleware in &self.middleware {
            middleware.before_route(&mut context)?;
        }

        // Find matching route
        let route_match = self.find_route(&context.path, &context.method)?;

        // Apply route-specific middleware (before routing)
        for middleware in &route_match.route.middleware {
            middleware.before_route(&mut context)?;
        }

        // Update context with route parameters
        context.params = route_match.params;

        // Handle the route
        let mut response = route_match.route.handler.handle(&context)?;

        // Apply route-specific middleware (after routing)
        for middleware in route_match.route.middleware.iter().rev() {
            middleware.after_route(&context, &mut response)?;
        }

        // Apply global middleware (after routing)
        for middleware in self.middleware.iter().rev() {
            middleware.after_route(&context, &mut response)?;
        }

        Ok(response)
    }

    /// Find a matching route
    fn find_route(&self, path: &str, method: &HttpMethod) -> Result<RouteMatch> {
        let clean_path = self.normalize_path(path);

        for route in &self.routes {
            if route.methods.contains(method) {
                if let Some(params) = self.match_pattern(&route.pattern, &clean_path) {
                    return Ok(RouteMatch {
                        route: route.clone(),
                        params,
                        query: HashMap::new(), // TODO: Extract from URL
                        segments: clean_path.split('/').map(|s| s.to_string()).collect(),
                    });
                }
            }
        }

        // No route found, use 404 handler if available
        if let Some(handler) = &self.not_found_handler {
            let not_found_route = Route {
                pattern: "/*".to_string(),
                methods: vec![method.clone()],
                handler: handler.clone(),
                middleware: Vec::new(),
                metadata: RouteMetadata::default(),
            };

            return Ok(RouteMatch {
                route: not_found_route,
                params: HashMap::new(),
                query: HashMap::new(),
                segments: Vec::new(),
            });
        }

        Err(RuitlError::route(format!(
            "No route found for {} {}",
            method, path
        )))
    }

    /// Match a route pattern against a path
    fn match_pattern(&self, pattern: &str, path: &str) -> Option<HashMap<String, String>> {
        let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        let mut params = HashMap::new();
        let mut pattern_idx = 0;
        let mut path_idx = 0;

        while pattern_idx < pattern_segments.len() && path_idx < path_segments.len() {
            let pattern_segment = pattern_segments[pattern_idx];
            let path_segment = path_segments[path_idx];

            if pattern_segment.starts_with(':') {
                // Parameter segment
                let param_name = &pattern_segment[1..];
                params.insert(param_name.to_string(), path_segment.to_string());
                pattern_idx += 1;
                path_idx += 1;
            } else if pattern_segment == "*" {
                // Wildcard - matches rest of path
                let remaining_path = path_segments[path_idx..].join("/");
                params.insert("*".to_string(), remaining_path);
                break;
            } else if pattern_segment == path_segment {
                // Exact match
                pattern_idx += 1;
                path_idx += 1;
            } else {
                // No match
                return None;
            }
        }

        // Check if we matched the entire pattern
        if pattern_idx == pattern_segments.len() && path_idx == path_segments.len() {
            Some(params)
        } else {
            None
        }
    }

    /// Normalize a path by removing trailing slashes and adding leading slash
    fn normalize_path(&self, path: &str) -> String {
        let mut normalized = if !self.base_path.is_empty() {
            path.strip_prefix(&self.base_path)
                .unwrap_or(path)
                .to_string()
        } else {
            path.to_string()
        };

        if !normalized.starts_with('/') {
            normalized = format!("/{}", normalized);
        }

        if normalized.len() > 1 && normalized.ends_with('/') {
            normalized.pop();
        }

        normalized
    }

    /// Create route context from request
    fn create_route_context(&self, path: &str, method: HttpMethod) -> Result<RouteContext> {
        let (path_part, query_part) = if let Some(pos) = path.find('?') {
            (&path[..pos], &path[pos + 1..])
        } else {
            (path, "")
        };

        let query = self.parse_query_string(query_part);

        Ok(RouteContext {
            path: path_part.to_string(),
            method,
            params: HashMap::new(),
            query,
            headers: HashMap::new(),
            body: None,
            component_context: ComponentContext::new(),
            data: HashMap::new(),
        })
    }

    /// Parse query string into parameters
    fn parse_query_string(&self, query: &str) -> HashMap<String, String> {
        if query.is_empty() {
            return HashMap::new();
        }

        query
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                match (parts.next(), parts.next()) {
                    (Some(key), Some(value)) => Some((
                        urlencoding::decode(key).ok()?.to_string(),
                        urlencoding::decode(value).ok()?.to_string(),
                    )),
                    (Some(key), None) => {
                        Some((urlencoding::decode(key).ok()?.to_string(), String::new()))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Generate URL for a named route
    pub fn url_for(&self, route_name: &str, params: &HashMap<String, String>) -> Result<String> {
        for route in &self.routes {
            if let Some(name) = &route.metadata.name {
                if name == route_name {
                    return Ok(self.build_url(&route.pattern, params));
                }
            }
        }

        Err(RuitlError::route(format!(
            "Route '{}' not found",
            route_name
        )))
    }

    /// Build URL from pattern and parameters
    fn build_url(&self, pattern: &str, params: &HashMap<String, String>) -> String {
        let mut url = pattern.to_string();

        for (key, value) in params {
            let placeholder = format!(":{}", key);
            if url.contains(&placeholder) {
                url = url.replace(&placeholder, &urlencoding::encode(value));
            }
        }

        format!("{}{}", self.base_path, url)
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteBuilder {
    /// Create a new route builder
    pub fn new<S: Into<String>>(pattern: S) -> Self {
        Self {
            pattern: pattern.into(),
            methods: vec![HttpMethod::Get],
            handler: None,
            middleware: Vec::new(),
            metadata: RouteMetadata::default(),
        }
    }

    /// Set HTTP methods for the route
    pub fn methods(mut self, methods: Vec<HttpMethod>) -> Self {
        self.methods = methods;
        self
    }

    /// Set GET method
    pub fn get(mut self) -> Self {
        self.methods = vec![HttpMethod::Get];
        self
    }

    /// Set POST method
    pub fn post(mut self) -> Self {
        self.methods = vec![HttpMethod::Post];
        self
    }

    /// Set handler
    pub fn handler<H>(mut self, handler: H) -> Self
    where
        H: RouteHandler + 'static,
    {
        self.handler = Some(Arc::new(handler));
        self
    }

    /// Set component handler
    pub fn component<C, P>(
        self,
        component: C,
        props_extractor: impl Fn(&RouteContext) -> Result<P> + Send + Sync + 'static,
    ) -> Self
    where
        C: Component<Props = P> + 'static,
        P: ComponentProps,
    {
        let handler = ComponentHandler::new(component, Box::new(props_extractor));
        self.handler(handler)
    }

    /// Set function handler
    pub fn function<F>(self, f: F) -> Self
    where
        F: Fn(&RouteContext) -> Result<RouteResponse> + Send + Sync + 'static,
    {
        let handler = FunctionHandler::new(Box::new(f));
        self.handler(handler)
    }

    /// Add middleware
    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.middleware.push(Arc::new(middleware));
        self
    }

    /// Set route name
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.metadata.name = Some(name.into());
        self
    }

    /// Set route description
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.metadata.description = Some(description.into());
        self
    }

    /// Build the route
    pub fn build(self) -> Result<Route> {
        let handler = self
            .handler
            .ok_or_else(|| RuitlError::route("Route handler is required"))?;

        Ok(Route {
            pattern: self.pattern,
            methods: self.methods,
            handler,
            middleware: self.middleware,
            metadata: self.metadata,
        })
    }
}

impl RouterBuilder {
    /// Create a new router builder
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            not_found_handler: None,
            middleware: Vec::new(),
            base_path: String::new(),
        }
    }

    /// Add a route
    pub fn route(mut self, route: Route) -> Self {
        self.routes.push(route);
        self
    }

    /// Add a route using builder pattern
    pub fn add<S: Into<String>>(self, pattern: S) -> RouteBuilderContext {
        RouteBuilderContext {
            router_builder: self,
            route_builder: RouteBuilder::new(pattern),
        }
    }

    /// Set base path
    pub fn base_path<S: Into<String>>(mut self, base_path: S) -> Self {
        self.base_path = base_path.into();
        self
    }

    /// Add global middleware
    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.middleware.push(Arc::new(middleware));
        self
    }

    /// Set 404 handler
    pub fn not_found<H>(mut self, handler: H) -> Self
    where
        H: RouteHandler + 'static,
    {
        self.not_found_handler = Some(Arc::new(handler));
        self
    }

    /// Build the router
    pub fn build(self) -> Router {
        Router {
            routes: self.routes,
            not_found_handler: self.not_found_handler,
            middleware: self.middleware,
            base_path: self.base_path,
        }
    }
}

/// Context for building routes within a router builder
pub struct RouteBuilderContext {
    router_builder: RouterBuilder,
    route_builder: RouteBuilder,
}

impl RouteBuilderContext {
    /// Set HTTP methods
    pub fn methods(mut self, methods: Vec<HttpMethod>) -> Self {
        self.route_builder = self.route_builder.methods(methods);
        self
    }

    /// Set GET method
    pub fn get(mut self) -> Self {
        self.route_builder = self.route_builder.get();
        self
    }

    /// Set POST method
    pub fn post(mut self) -> Self {
        self.route_builder = self.route_builder.post();
        self
    }

    /// Set handler
    pub fn handler<H>(mut self, handler: H) -> RouterBuilder
    where
        H: RouteHandler + 'static,
    {
        self.route_builder = self.route_builder.handler(handler);
        let route = self.route_builder.build().expect("Failed to build route");
        self.router_builder.route(route)
    }

    /// Set component handler
    pub fn component<C, P>(
        mut self,
        component: C,
        props_extractor: impl Fn(&RouteContext) -> Result<P> + Send + Sync + 'static,
    ) -> RouterBuilder
    where
        C: Component<Props = P> + 'static,
        P: ComponentProps,
    {
        self.route_builder = self.route_builder.component(component, props_extractor);
        let route = self.route_builder.build().expect("Failed to build route");
        self.router_builder.route(route)
    }

    /// Set function handler
    pub fn function<F>(mut self, f: F) -> RouterBuilder
    where
        F: Fn(&RouteContext) -> Result<RouteResponse> + Send + Sync + 'static,
    {
        self.route_builder = self.route_builder.function(f);
        let route = self.route_builder.build().expect("Failed to build route");
        self.router_builder.route(route)
    }
}

impl<C, P> ComponentHandler<C, P>
where
    C: Component<Props = P> + 'static,
    P: ComponentProps,
{
    /// Create a new component handler
    pub fn new(
        component: C,
        props_extractor: Box<dyn Fn(&RouteContext) -> Result<P> + Send + Sync>,
    ) -> Self {
        Self {
            component,
            props_extractor,
        }
    }
}

impl<C, P> RouteHandler for ComponentHandler<C, P>
where
    C: Component<Props = P> + 'static,
    P: ComponentProps,
{
    fn handle(&self, context: &RouteContext) -> Result<RouteResponse> {
        let props = (self.props_extractor)(context)?;
        let html = self.component.render(&props, &context.component_context)?;

        Ok(RouteResponse {
            status: 200,
            headers: {
                let mut headers = HashMap::new();
                headers.insert(
                    "content-type".to_string(),
                    "text/html; charset=utf-8".to_string(),
                );
                headers
            },
            body: RouteResponseBody::Html(html),
        })
    }
}

impl FunctionHandler {
    /// Create a new function handler
    pub fn new(handler: Box<dyn Fn(&RouteContext) -> Result<RouteResponse> + Send + Sync>) -> Self {
        Self { handler }
    }
}

impl RouteHandler for FunctionHandler {
    fn handle(&self, context: &RouteContext) -> Result<RouteResponse> {
        (self.handler)(context)
    }
}

impl RouteResponse {
    /// Create an HTML response
    pub fn html(html: Html) -> Self {
        Self {
            status: 200,
            headers: {
                let mut headers = HashMap::new();
                headers.insert(
                    "content-type".to_string(),
                    "text/html; charset=utf-8".to_string(),
                );
                headers
            },
            body: RouteResponseBody::Html(html),
        }
    }

    /// Create a JSON response
    pub fn json(data: serde_json::Value) -> Self {
        Self {
            status: 200,
            headers: {
                let mut headers = HashMap::new();
                headers.insert("content-type".to_string(), "application/json".to_string());
                headers
            },
            body: RouteResponseBody::Json(data),
        }
    }

    /// Create a text response
    pub fn text<S: Into<String>>(text: S) -> Self {
        Self {
            status: 200,
            headers: {
                let mut headers = HashMap::new();
                headers.insert(
                    "content-type".to_string(),
                    "text/plain; charset=utf-8".to_string(),
                );
                headers
            },
            body: RouteResponseBody::Text(text.into()),
        }
    }

    /// Create a redirect response
    pub fn redirect<S: Into<String>>(url: S) -> Self {
        Self {
            status: 302,
            headers: {
                let mut headers = HashMap::new();
                headers.insert("location".to_string(), url.into());
                headers
            },
            body: RouteResponseBody::Empty,
        }
    }

    /// Set status code
    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    /// Add header
    pub fn header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

/// Simple logging middleware
#[derive(Debug)]
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn before_route(&self, context: &mut RouteContext) -> Result<()> {
        println!("→ {} {}", context.method, context.path);
        Ok(())
    }

    fn after_route(&self, context: &RouteContext, response: &mut RouteResponse) -> Result<()> {
        println!("← {} {} {}", context.method, context.path, response.status);
        Ok(())
    }
}

/// CORS middleware
#[derive(Debug)]
pub struct CorsMiddleware {
    allow_origin: String,
    allow_methods: Vec<String>,
    allow_headers: Vec<String>,
}

impl CorsMiddleware {
    pub fn new() -> Self {
        Self {
            allow_origin: "*".to_string(),
            allow_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
            ],
            allow_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
        }
    }

    pub fn allow_origin<S: Into<String>>(mut self, origin: S) -> Self {
        self.allow_origin = origin.into();
        self
    }
}

impl Middleware for CorsMiddleware {
    fn after_route(&self, _context: &RouteContext, response: &mut RouteResponse) -> Result<()> {
        response.headers.insert(
            "access-control-allow-origin".to_string(),
            self.allow_origin.clone(),
        );
        response.headers.insert(
            "access-control-allow-methods".to_string(),
            self.allow_methods.join(", "),
        );
        response.headers.insert(
            "access-control-allow-headers".to_string(),
            self.allow_headers.join(", "),
        );
        Ok(())
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Options => write!(f, "OPTIONS"),
            HttpMethod::Connect => write!(f, "CONNECT"),
            HttpMethod::Trace => write!(f, "TRACE"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::EmptyProps;

    #[derive(Debug)]
    struct TestComponent;

    impl Component for TestComponent {
        type Props = EmptyProps;

        fn render(&self, _props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
            Ok(crate::html::Html::text("Test Component"))
        }
    }

    #[test]
    fn test_router_creation() {
        let router = Router::new();
        assert_eq!(router.routes.len(), 0);
    }

    #[test]
    fn test_route_building() {
        let route = RouteBuilder::new("/test")
            .get()
            .name("test")
            .function(|_| Ok(RouteResponse::text("Test")))
            .build()
            .unwrap();

        assert_eq!(route.pattern, "/test");
        assert_eq!(route.methods, vec![HttpMethod::Get]);
        assert_eq!(route.metadata.name, Some("test".to_string()));
    }

    #[test]
    fn test_router_building() {
        let router = Router::builder()
            .base_path("/api")
            .middleware(LoggingMiddleware)
            .add("/users/:id")
            .get()
            .function(|ctx| {
                let id = ctx.params.get("id").unwrap_or("unknown");
                Ok(RouteResponse::json(serde_json::json!({"id": id})))
            })
            .add("/health")
            .get()
            .function(|_| Ok(RouteResponse::json(serde_json::json!({"status": "ok"}))))
            .build();

        assert_eq!(router.routes.len(), 2);
        assert_eq!(router.base_path, "/api");
        assert_eq!(router.middleware.len(), 1);
    }

    #[test]
    fn test_pattern_matching() {
        let router = Router::new();

        // Test exact match
        let params = router.match_pattern("/users", "/users");
        assert!(params.is_some());
        assert!(params.unwrap().is_empty());

        // Test parameter match
        let params = router.match_pattern("/users/:id", "/users/123");
        assert!(params.is_some());
        let params = params.unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        // Test wildcard match
        let params = router.match_pattern("/files/*", "/files/doc/test.txt");
        assert!(params.is_some());
        let params = params.unwrap();
        assert_eq!(params.get("*"), Some(&"doc/test.txt".to_string()));

        // Test no match
        let params = router.match_pattern("/users", "/posts");
        assert!(params.is_none());
    }

    #[test]
    fn test_path_normalization() {
        let router = Router::new();

        assert_eq!(router.normalize_path("/test"), "/test");
        assert_eq!(router.normalize_path("test"), "/test");
        assert_eq!(router.normalize_path("/test/"), "/test");
        assert_eq!(router.normalize_path("/"), "/");
    }

    #[test]
    fn test_query_string_parsing() {
        let router = Router::new();

        let query = router.parse_query_string("name=john&age=30");
        assert_eq!(query.get("name"), Some(&"john".to_string()));
        assert_eq!(query.get("age"), Some(&"30".to_string()));

        let query = router.parse_query_string("search=hello%20world");
        assert_eq!(query.get("search"), Some(&"hello world".to_string()));

        let query = router.parse_query_string("");
        assert!(query.is_empty());
    }

    #[test]
    fn test_url_generation() {
        let mut router = Router::new();

        let route = RouteBuilder::new("/users/:id")
            .name("user_detail")
            .function(|_| Ok(RouteResponse::text("User")))
            .build()
            .unwrap();

        router.add_route(route);

        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());

        let url = router.url_for("user_detail", &params).unwrap();
        assert_eq!(url, "/users/123");
    }

    #[test]
    fn test_component_handler() {
        let handler = ComponentHandler::new(TestComponent, Box::new(|_| Ok(EmptyProps)));

        let context = RouteContext {
            path: "/test".to_string(),
            method: HttpMethod::Get,
            params: HashMap::new(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            component_context: ComponentContext::new(),
            data: HashMap::new(),
        };

        let response = handler.handle(&context).unwrap();
        assert_eq!(response.status, 200);
        assert!(matches!(response.body, RouteResponseBody::Html(_)));
    }

    #[test]
    fn test_middleware() {
        let middleware = LoggingMiddleware;
        let mut context = RouteContext {
            path: "/test".to_string(),
            method: HttpMethod::Get,
            params: HashMap::new(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            component_context: ComponentContext::new(),
            data: HashMap::new(),
        };

        let mut response = RouteResponse::text("Test");

        // Test before route middleware
        assert!(middleware.before_route(&mut context).is_ok());

        // Test after route middleware
        assert!(middleware.after_route(&context, &mut response).is_ok());
    }

    #[test]
    fn test_cors_middleware() {
        let middleware = CorsMiddleware::new().allow_origin("https://example.com");
        let context = RouteContext {
            path: "/api/test".to_string(),
            method: HttpMethod::Get,
            params: HashMap::new(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            component_context: ComponentContext::new(),
            data: HashMap::new(),
        };

        let mut response = RouteResponse::json(serde_json::json!({"test": true}));
        middleware.after_route(&context, &mut response).unwrap();

        assert_eq!(
            response.headers.get("access-control-allow-origin"),
            Some(&"https://example.com".to_string())
        );
    }
}
