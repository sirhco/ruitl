//! Rendering system for RUITL
//!
//! This module provides the core rendering functionality for different targets
//! including server-side rendering, static generation, and development mode.

use crate::component::{Component, ComponentContext, ComponentProps, ComponentRegistry};
use crate::error::{Result, RuitlError};
use crate::html::Html;
use crate::template::{Template, TemplateEngine};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Render target types
#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub enum RenderTarget {
    /// Server-side rendering
    Server,
    /// Static site generation
    Static,
    /// Development mode
    Development,
    /// Client-side hydration
    Client,
}

/// Render context containing request and environment information
#[derive(Debug, Clone)]
pub struct RenderContext {
    /// Request path
    pub path: String,
    /// Query parameters
    pub query: HashMap<String, String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Render target
    pub target: RenderTarget,
    /// Base URL
    pub base_url: String,
    /// Current locale
    pub locale: Option<String>,
    /// User data
    pub user_data: HashMap<String, serde_json::Value>,
    /// Component context
    pub component_context: ComponentContext,
}

/// Render options for customizing output
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Whether to minify HTML output
    pub minify: bool,
    /// Whether to pretty-print HTML
    pub pretty: bool,
    /// Include source maps
    pub source_maps: bool,
    /// Document type declaration
    pub doctype: Option<String>,
    /// Custom head elements
    pub head_elements: Vec<Html>,
    /// Custom body attributes
    pub body_attributes: HashMap<String, String>,
    /// CSS inclusion strategy
    pub css_strategy: CssStrategy,
    /// JavaScript inclusion strategy
    pub js_strategy: JsStrategy,
}

/// CSS inclusion strategies
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CssStrategy {
    /// Inline all CSS
    Inline,
    /// External CSS files
    External,
    /// Critical CSS inline, rest external
    Critical,
    /// No CSS inclusion
    None,
}

/// JavaScript inclusion strategies
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum JsStrategy {
    /// Inline all JavaScript
    Inline,
    /// External JavaScript files
    External,
    /// No JavaScript inclusion
    None,
    /// Progressive enhancement
    Progressive,
}

/// Main renderer trait
#[async_trait]
pub trait Renderer: Debug + Send + Sync {
    /// Render HTML for the given context
    async fn render(&self, context: &RenderContext, options: &RenderOptions) -> Result<String>;

    /// Render a specific component
    async fn render_component<C>(
        &self,
        component: &C,
        props: &C::Props,
        context: &RenderContext,
    ) -> Result<Html>
    where
        C: Component + 'static;

    /// Render a template
    async fn render_template(&self, template_name: &str, context: &RenderContext) -> Result<Html>;

    /// Get renderer capabilities
    fn capabilities(&self) -> RendererCapabilities;
}

/// Renderer capabilities
#[derive(Debug, Clone)]
pub struct RendererCapabilities {
    /// Supports server-side rendering
    pub ssr: bool,
    /// Supports static generation
    pub static_gen: bool,
    /// Supports hot reload
    pub hot_reload: bool,
    /// Supports streaming
    pub streaming: bool,
    /// Supports caching
    pub caching: bool,
}

/// Universal renderer that can handle multiple targets
#[derive(Debug)]
pub struct UniversalRenderer {
    template_engine: Arc<RwLock<TemplateEngine>>,
    component_registry: Arc<RwLock<ComponentRegistry>>,
    cache: Arc<RwLock<RenderCache>>,
    config: RendererConfig,
}

/// Renderer configuration
#[derive(Debug, Clone)]
pub struct RendererConfig {
    /// Enable caching
    pub cache_enabled: bool,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
    /// Maximum cache size
    pub max_cache_size: usize,
    /// Enable streaming
    pub streaming: bool,
    /// Template directories
    pub template_dirs: Vec<PathBuf>,
    /// Component directories
    pub component_dirs: Vec<PathBuf>,
}

/// Render cache for improved performance
#[derive(Debug)]
struct RenderCache {
    entries: HashMap<String, CacheEntry>,
    max_size: usize,
}

/// Cache entry
#[derive(Debug, Clone)]
struct CacheEntry {
    content: String,
    timestamp: std::time::SystemTime,
    ttl: std::time::Duration,
}

/// Document renderer for full HTML documents
#[derive(Debug)]
pub struct DocumentRenderer {
    renderer: UniversalRenderer,
}

/// Page data for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageData {
    /// Page title
    pub title: Option<String>,
    /// Meta description
    pub description: Option<String>,
    /// Meta keywords
    pub keywords: Vec<String>,
    /// Open Graph data
    pub og: HashMap<String, String>,
    /// Twitter Card data
    pub twitter: HashMap<String, String>,
    /// Canonical URL
    pub canonical: Option<String>,
    /// Language
    pub lang: Option<String>,
    /// Custom meta tags
    pub meta: Vec<MetaTag>,
    /// Custom link tags
    pub links: Vec<LinkTag>,
}

/// Meta tag representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaTag {
    pub name: Option<String>,
    pub property: Option<String>,
    pub content: String,
    pub charset: Option<String>,
    pub http_equiv: Option<String>,
}

/// Link tag representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkTag {
    pub rel: String,
    pub href: String,
    pub media: Option<String>,
    pub sizes: Option<String>,
    pub crossorigin: Option<String>,
}

impl Default for RenderContext {
    fn default() -> Self {
        Self {
            path: "/".to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            env: HashMap::new(),
            target: RenderTarget::Development,
            base_url: "/".to_string(),
            locale: None,
            user_data: HashMap::new(),
            component_context: ComponentContext::new(),
        }
    }
}

impl RenderContext {
    /// Create a new render context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the request path
    pub fn with_path<S: Into<String>>(mut self, path: S) -> Self {
        self.path = path.into();
        self
    }

    /// Set the render target
    pub fn with_target(mut self, target: RenderTarget) -> Self {
        self.target = target;
        self
    }

    /// Set the base URL
    pub fn with_base_url<S: Into<String>>(mut self, base_url: S) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Add query parameter
    pub fn with_query<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.query.insert(key.into(), value.into());
        self
    }

    /// Add header
    pub fn with_header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set locale
    pub fn with_locale<S: Into<String>>(mut self, locale: S) -> Self {
        self.locale = Some(locale.into());
        self
    }

    /// Add user data
    pub fn with_user_data<K: Into<String>, V: Serialize>(
        mut self,
        key: K,
        value: V,
    ) -> Result<Self> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| RuitlError::render(format!("Failed to serialize user data: {}", e)))?;
        self.user_data.insert(key.into(), json_value);
        Ok(self)
    }

    /// Get query parameter
    pub fn get_query(&self, key: &str) -> Option<&String> {
        self.query.get(key)
    }

    /// Get header
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Get user data
    pub fn get_user_data<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<Result<T>> {
        self.user_data.get(key).map(|value| {
            serde_json::from_value(value.clone())
                .map_err(|e| RuitlError::render(format!("Failed to deserialize user data: {}", e)))
        })
    }

    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        self.target == RenderTarget::Development
    }

    /// Check if running server-side
    pub fn is_server_side(&self) -> bool {
        matches!(self.target, RenderTarget::Server | RenderTarget::Static)
    }

    /// Get full URL for a path
    pub fn url_for(&self, path: &str) -> String {
        if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        }
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            minify: false,
            pretty: false,
            source_maps: false,
            doctype: Some("<!DOCTYPE html>".to_string()),
            head_elements: Vec::new(),
            body_attributes: HashMap::new(),
            css_strategy: CssStrategy::External,
            js_strategy: JsStrategy::External,
        }
    }
}

impl RenderOptions {
    /// Create new render options
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable minification
    pub fn minified(mut self) -> Self {
        self.minify = true;
        self
    }

    /// Enable pretty printing
    pub fn pretty(mut self) -> Self {
        self.pretty = true;
        self
    }

    /// Set CSS strategy
    pub fn css_strategy(mut self, strategy: CssStrategy) -> Self {
        self.css_strategy = strategy;
        self
    }

    /// Set JavaScript strategy
    pub fn js_strategy(mut self, strategy: JsStrategy) -> Self {
        self.js_strategy = strategy;
        self
    }

    /// Add head element
    pub fn with_head_element(mut self, element: Html) -> Self {
        self.head_elements.push(element);
        self
    }

    /// Add body attribute
    pub fn with_body_attribute<K: Into<String>, V: Into<String>>(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.body_attributes.insert(key.into(), value.into());
        self
    }
}

impl UniversalRenderer {
    /// Create a new universal renderer
    pub fn new(config: RendererConfig) -> Self {
        Self {
            template_engine: Arc::new(RwLock::new(TemplateEngine::new())),
            component_registry: Arc::new(RwLock::new(ComponentRegistry::new())),
            cache: Arc::new(RwLock::new(RenderCache::new(config.max_cache_size))),
            config,
        }
    }

    /// Register a component
    pub async fn register_component<C>(&self, name: &str, component: C)
    where
        C: Component + 'static,
    {
        let mut registry = self.component_registry.write().await;
        registry.register(name, component);
    }

    /// Register a template
    pub async fn register_template(&self, name: &str, content: &str) -> Result<()> {
        let mut engine = self.template_engine.write().await;
        engine.register_template(name, content)
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        (cache.entries.len(), cache.max_size)
    }
}

#[async_trait]
impl Renderer for UniversalRenderer {
    async fn render(&self, context: &RenderContext, options: &RenderOptions) -> Result<String> {
        // Check cache first
        if self.config.cache_enabled {
            let cache_key = self.generate_cache_key(context, options);
            let cache = self.cache.read().await;
            if let Some(entry) = cache.get(&cache_key) {
                if !entry.is_expired() {
                    return Ok(entry.content.clone());
                }
            }
        }

        // Render content
        let content = self.render_internal(context, options).await?;

        // Store in cache
        if self.config.cache_enabled {
            let cache_key = self.generate_cache_key(context, options);
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, content.clone(), self.config.cache_ttl);
        }

        Ok(content)
    }

    async fn render_component<C>(
        &self,
        component: &C,
        props: &C::Props,
        context: &RenderContext,
    ) -> Result<Html>
    where
        C: Component + 'static,
    {
        component.render(props, &context.component_context)
    }

    async fn render_template(&self, template_name: &str, context: &RenderContext) -> Result<Html> {
        let engine = self.template_engine.read().await;
        engine.render(template_name, &context.component_context)
    }

    fn capabilities(&self) -> RendererCapabilities {
        RendererCapabilities {
            ssr: true,
            static_gen: true,
            hot_reload: self.config.streaming,
            streaming: self.config.streaming,
            caching: self.config.cache_enabled,
        }
    }
}

impl UniversalRenderer {
    async fn render_internal(
        &self,
        context: &RenderContext,
        options: &RenderOptions,
    ) -> Result<String> {
        // This would contain the actual rendering logic
        // For now, return a basic HTML structure
        let mut html = String::new();

        if let Some(doctype) = &options.doctype {
            html.push_str(doctype);
            html.push('\n');
        }

        html.push_str("<html>");
        html.push_str("<head>");

        // Add meta tags and other head elements
        for element in &options.head_elements {
            html.push_str(&element.render());
        }

        html.push_str("</head>");
        html.push_str("<body");

        // Add body attributes
        for (key, value) in &options.body_attributes {
            html.push_str(&format!(" {}=\"{}\"", key, value));
        }

        html.push_str(">");

        // Add main content (this would be the actual page content)
        html.push_str(&format!("<h1>Page: {}</h1>", context.path));

        html.push_str("</body>");
        html.push_str("</html>");

        // Apply post-processing
        if options.minify {
            html = self.minify_html(&html)?;
        }

        if options.pretty {
            html = self.prettify_html(&html)?;
        }

        Ok(html)
    }

    fn generate_cache_key(&self, context: &RenderContext, options: &RenderOptions) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        context.path.hash(&mut hasher);
        context.target.hash(&mut hasher);
        options.minify.hash(&mut hasher);
        options.css_strategy.hash(&mut hasher);
        options.js_strategy.hash(&mut hasher);

        format!("render:{:x}", hasher.finish())
    }

    fn minify_html(&self, html: &str) -> Result<String> {
        #[cfg(feature = "minify")]
        {
            minify_html::minify(html.as_bytes(), &minify_html::Cfg::spec_compliant())
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                .map_err(|e| RuitlError::render(format!("Failed to minify HTML: {:?}", e)))
        }
        #[cfg(not(feature = "minify"))]
        {
            Ok(html.to_string())
        }
    }

    fn prettify_html(&self, html: &str) -> Result<String> {
        // Simple prettification - in a real implementation,
        // you might use a proper HTML formatter
        Ok(html.to_string())
    }
}

impl RenderCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
        }
    }

    fn get(&self, key: &str) -> Option<&CacheEntry> {
        self.entries.get(key)
    }

    fn insert(&mut self, key: String, content: String, ttl_seconds: u64) {
        // Remove expired entries
        self.cleanup_expired();

        // Remove oldest entries if at capacity
        while self.entries.len() >= self.max_size {
            if let Some(oldest_key) = self.find_oldest_key() {
                self.entries.remove(&oldest_key);
            } else {
                break;
            }
        }

        let entry = CacheEntry {
            content,
            timestamp: std::time::SystemTime::now(),
            ttl: std::time::Duration::from_secs(ttl_seconds),
        };

        self.entries.insert(key, entry);
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn cleanup_expired(&mut self) {
        let now = std::time::SystemTime::now();
        self.entries
            .retain(|_, entry| entry.timestamp.elapsed().unwrap_or_default() < entry.ttl);
    }

    fn find_oldest_key(&self) -> Option<String> {
        self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.timestamp)
            .map(|(key, _)| key.clone())
    }
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.timestamp.elapsed().unwrap_or_default() >= self.ttl
    }
}

impl DocumentRenderer {
    /// Create a new document renderer
    pub fn new(config: RendererConfig) -> Self {
        Self {
            renderer: UniversalRenderer::new(config),
        }
    }

    /// Render a complete HTML document
    pub async fn render_document(
        &self,
        page_data: &PageData,
        body_content: Html,
        context: &RenderContext,
        options: &RenderOptions,
    ) -> Result<String> {
        let mut html = String::new();

        // DOCTYPE
        if let Some(doctype) = &options.doctype {
            html.push_str(doctype);
            html.push('\n');
        }

        // HTML tag with language
        if let Some(lang) = &page_data.lang {
            html.push_str(&format!("<html lang=\"{}\">", lang));
        } else {
            html.push_str("<html>");
        }

        // Head section
        html.push_str("<head>");

        // Charset
        html.push_str("<meta charset=\"utf-8\">");

        // Title
        if let Some(title) = &page_data.title {
            html.push_str(&format!(
                "<title>{}</title>",
                html_escape::encode_text(title)
            ));
        }

        // Meta description
        if let Some(description) = &page_data.description {
            html.push_str(&format!(
                "<meta name=\"description\" content=\"{}\">",
                html_escape::encode_quoted_attribute(description)
            ));
        }

        // Meta keywords
        if !page_data.keywords.is_empty() {
            let keywords = page_data.keywords.join(", ");
            html.push_str(&format!(
                "<meta name=\"keywords\" content=\"{}\">",
                html_escape::encode_quoted_attribute(&keywords)
            ));
        }

        // Canonical URL
        if let Some(canonical) = &page_data.canonical {
            html.push_str(&format!("<link rel=\"canonical\" href=\"{}\">", canonical));
        }

        // Open Graph tags
        for (property, content) in &page_data.og {
            html.push_str(&format!(
                "<meta property=\"{}\" content=\"{}\">",
                html_escape::encode_quoted_attribute(property),
                html_escape::encode_quoted_attribute(content)
            ));
        }

        // Twitter Card tags
        for (name, content) in &page_data.twitter {
            html.push_str(&format!(
                "<meta name=\"{}\" content=\"{}\">",
                html_escape::encode_quoted_attribute(name),
                html_escape::encode_quoted_attribute(content)
            ));
        }

        // Custom meta tags
        for meta in &page_data.meta {
            html.push_str("<meta");
            if let Some(name) = &meta.name {
                html.push_str(&format!(
                    " name=\"{}\"",
                    html_escape::encode_quoted_attribute(name)
                ));
            }
            if let Some(property) = &meta.property {
                html.push_str(&format!(
                    " property=\"{}\"",
                    html_escape::encode_quoted_attribute(property)
                ));
            }
            if let Some(charset) = &meta.charset {
                html.push_str(&format!(
                    " charset=\"{}\"",
                    html_escape::encode_quoted_attribute(charset)
                ));
            }
            if let Some(http_equiv) = &meta.http_equiv {
                html.push_str(&format!(
                    " http-equiv=\"{}\"",
                    html_escape::encode_quoted_attribute(http_equiv)
                ));
            }
            html.push_str(&format!(
                " content=\"{}\">",
                html_escape::encode_quoted_attribute(&meta.content)
            ));
        }

        // Custom link tags
        for link in &page_data.links {
            html.push_str(&format!(
                "<link rel=\"{}\" href=\"{}\"",
                html_escape::encode_quoted_attribute(&link.rel),
                html_escape::encode_quoted_attribute(&link.href)
            ));
            if let Some(media) = &link.media {
                html.push_str(&format!(
                    " media=\"{}\"",
                    html_escape::encode_quoted_attribute(media)
                ));
            }
            if let Some(sizes) = &link.sizes {
                html.push_str(&format!(
                    " sizes=\"{}\"",
                    html_escape::encode_quoted_attribute(sizes)
                ));
            }
            if let Some(crossorigin) = &link.crossorigin {
                html.push_str(&format!(
                    " crossorigin=\"{}\"",
                    html_escape::encode_quoted_attribute(crossorigin)
                ));
            }
            html.push_str(">");
        }

        // Additional head elements
        for element in &options.head_elements {
            html.push_str(&element.render());
        }

        html.push_str("</head>");

        // Body
        html.push_str("<body");
        for (key, value) in &options.body_attributes {
            html.push_str(&format!(
                " {}=\"{}\"",
                key,
                html_escape::encode_quoted_attribute(value)
            ));
        }
        html.push_str(">");

        // Body content
        html.push_str(&body_content.render());

        html.push_str("</body>");
        html.push_str("</html>");

        // Apply post-processing
        if options.minify {
            html = self.renderer.minify_html(&html)?;
        }

        if options.pretty {
            html = self.renderer.prettify_html(&html)?;
        }

        Ok(html)
    }
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_ttl: 3600,
            max_cache_size: 1000,
            streaming: false,
            template_dirs: vec![PathBuf::from("templates")],
            component_dirs: vec![PathBuf::from("src/components")],
        }
    }
}

impl Default for PageData {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            keywords: Vec::new(),
            og: HashMap::new(),
            twitter: HashMap::new(),
            canonical: None,
            lang: None,
            meta: Vec::new(),
            links: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_context_creation() {
        let context = RenderContext::new()
            .with_path("/test")
            .with_target(RenderTarget::Static)
            .with_base_url("https://example.com")
            .with_query("param", "value");

        assert_eq!(context.path, "/test");
        assert_eq!(context.target, RenderTarget::Static);
        assert_eq!(context.base_url, "https://example.com");
        assert_eq!(context.get_query("param"), Some(&"value".to_string()));
    }

    #[test]
    fn test_render_options() {
        let options = RenderOptions::new()
            .minified()
            .pretty()
            .css_strategy(CssStrategy::Inline)
            .js_strategy(JsStrategy::None);

        assert!(options.minify);
        assert!(options.pretty);
        assert_eq!(options.css_strategy, CssStrategy::Inline);
        assert_eq!(options.js_strategy, JsStrategy::None);
    }

    #[test]
    fn test_page_data() {
        let mut page_data = PageData::default();
        page_data.title = Some("Test Page".to_string());
        page_data.description = Some("A test page".to_string());
        page_data.keywords = vec!["test".to_string(), "page".to_string()];

        assert_eq!(page_data.title, Some("Test Page".to_string()));
        assert_eq!(page_data.keywords.len(), 2);
    }

    #[tokio::test]
    async fn test_universal_renderer() {
        let config = RendererConfig::default();
        let renderer = UniversalRenderer::new(config);

        let context = RenderContext::new().with_path("/test");
        let options = RenderOptions::new();

        let result = renderer.render(&context, &options).await;
        assert!(result.is_ok());

        let html = result.unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn test_cache_entry() {
        let entry = CacheEntry {
            content: "test".to_string(),
            timestamp: std::time::SystemTime::now(),
            ttl: std::time::Duration::from_secs(1),
        };

        assert!(!entry.is_expired());

        // Test with expired entry
        let expired_entry = CacheEntry {
            content: "test".to_string(),
            timestamp: std::time::SystemTime::now() - std::time::Duration::from_secs(2),
            ttl: std::time::Duration::from_secs(1),
        };

        assert!(expired_entry.is_expired());
    }

    #[test]
    fn test_render_cache() {
        let mut cache = RenderCache::new(2);

        cache.insert("key1".to_string(), "content1".to_string(), 3600);
        cache.insert("key2".to_string(), "content2".to_string(), 3600);

        assert_eq!(cache.entries.len(), 2);

        // Test capacity limit
        cache.insert("key3".to_string(), "content3".to_string(), 3600);
        assert_eq!(cache.entries.len(), 2);

        cache.clear();
        assert_eq!(cache.entries.len(), 0);
    }
}
