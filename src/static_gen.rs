//! Static site generator for RUITL projects
//!
//! This module provides functionality to generate static HTML files from RUITL
//! components and templates for deployment to static hosting services.

use crate::component::{ComponentContext, ComponentRegistry};
use crate::config::{RuitlConfig, StaticConfig};
use crate::error::{Result, ResultExt, RuitlError};
use crate::html::Html;
use crate::render::{
    DocumentRenderer, PageData, RenderContext, RenderOptions, RenderTarget, Renderer,
    RendererConfig, UniversalRenderer,
};
use crate::template::TemplateEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Static site generator
pub struct StaticGenerator {
    config: StaticConfig,
    project_config: RuitlConfig,
    renderer: UniversalRenderer,
    document_renderer: DocumentRenderer,
    route_resolver: RouteResolver,
}

/// Route resolver for discovering and generating routes
struct RouteResolver {
    static_routes: Vec<String>,
    dynamic_routes: Vec<DynamicRoute>,
    pages_dir: PathBuf,
}

/// Dynamic route definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicRoute {
    /// Route pattern (e.g., "/blog/:slug")
    pub pattern: String,
    /// Component or template to render
    pub component: String,
    /// Data source for generating routes
    pub data_source: Option<String>,
    /// Static parameters
    pub params: HashMap<String, String>,
}

/// Route generation context
#[derive(Debug, Clone)]
pub struct RouteContext {
    /// Route path
    pub path: String,
    /// Route parameters
    pub params: HashMap<String, String>,
    /// Page metadata
    pub metadata: PageMetadata,
    /// Additional context data
    pub data: HashMap<String, serde_json::Value>,
}

/// Page metadata for SEO and social sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    /// Page title
    pub title: Option<String>,
    /// Meta description
    pub description: Option<String>,
    /// Meta keywords
    pub keywords: Vec<String>,
    /// Open Graph image
    pub image: Option<String>,
    /// Canonical URL
    pub canonical: Option<String>,
    /// Page language
    pub lang: Option<String>,
    /// Custom meta tags
    pub meta: HashMap<String, String>,
}

/// Static generation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticStats {
    /// Number of pages generated
    pub pages_generated: usize,
    /// Number of assets copied
    pub assets_copied: usize,
    /// Total size of generated files
    pub total_size: u64,
    /// Generation time in seconds
    pub generation_time: f64,
    /// Generated routes
    pub routes: Vec<String>,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
}

/// Asset copying configuration
#[derive(Debug, Clone)]
pub struct AssetCopyOptions {
    /// Source directory
    pub source_dir: PathBuf,
    /// Destination directory
    pub dest_dir: PathBuf,
    /// File patterns to include
    pub include_patterns: Vec<String>,
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Whether to optimize assets
    pub optimize: bool,
}

impl StaticGenerator {
    /// Create a new static generator
    pub fn new(config: StaticConfig, project_config: RuitlConfig) -> Result<Self> {
        let renderer_config = RendererConfig::default();
        let renderer = UniversalRenderer::new(renderer_config.clone());
        let document_renderer = DocumentRenderer::new(renderer_config);

        let route_resolver = RouteResolver::new(
            config.routes.clone(),
            config.dynamic_routes.clone(),
            project_config.build.src_dir.join("pages"),
        );

        Ok(Self {
            config,
            project_config,
            renderer,
            document_renderer,
            route_resolver,
        })
    }

    /// Generate static site
    pub async fn generate(&self, output_dir: &Path) -> Result<StaticStats> {
        let start_time = std::time::Instant::now();
        println!("🏗️  Generating static site...");

        let mut stats = StaticStats {
            pages_generated: 0,
            assets_copied: 0,
            total_size: 0,
            generation_time: 0.0,
            routes: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Create output directory
        if output_dir.exists() {
            fs::remove_dir_all(output_dir)
                .static_gen_context("Failed to clean output directory")?;
        }
        fs::create_dir_all(output_dir).static_gen_context("Failed to create output directory")?;

        // Generate routes
        let routes = self.route_resolver.resolve_routes().await?;
        println!("📄 Found {} routes to generate", routes.len());

        // Generate pages
        for route_context in routes {
            match self.generate_page(&route_context, output_dir).await {
                Ok(size) => {
                    stats.pages_generated += 1;
                    stats.total_size += size;
                    stats.routes.push(route_context.path.clone());
                    println!("  ✓ {}", route_context.path);
                }
                Err(e) => {
                    let error_msg = format!("Failed to generate {}: {}", route_context.path, e);
                    stats.errors.push(error_msg.clone());
                    eprintln!("  ❌ {}", error_msg);
                }
            }
        }

        // Copy static assets
        stats.assets_copied = self.copy_static_assets(output_dir).await?;

        // Generate additional files
        if self.config.generate_sitemap {
            self.generate_sitemap(&stats.routes, output_dir).await?;
        }

        if self.config.generate_robots {
            self.generate_robots_txt(output_dir).await?;
        }

        if self.config.generate_404 {
            self.generate_404_page(output_dir).await?;
        }

        stats.generation_time = start_time.elapsed().as_secs_f64();

        println!(
            "✅ Generated {} pages and {} assets in {:.2}s",
            stats.pages_generated, stats.assets_copied, stats.generation_time
        );

        Ok(stats)
    }

    /// Generate a single page
    async fn generate_page(&self, route_context: &RouteContext, output_dir: &Path) -> Result<u64> {
        // Create render context
        let render_context = RenderContext::new()
            .with_path(&route_context.path)
            .with_target(RenderTarget::Static)
            .with_base_url(&self.config.base_url);

        // Create page data from metadata
        let page_data = self.create_page_data(&route_context.metadata, &route_context.path);

        // Render page content
        let body_content = self
            .renderer
            .render_template("page", &render_context)
            .await
            .unwrap_or_else(|_| {
                // Fallback to basic HTML if template not found
                crate::html::Html::Element(
                    crate::html::div()
                        .child(Html::Element(crate::html::h1().text("Page Not Found")))
                        .child(Html::Element(
                            crate::html::p().text("This page could not be rendered."),
                        )),
                )
            });

        // Render full document
        let render_options = RenderOptions::new().minified();
        let html = self
            .document_renderer
            .render_document(&page_data, body_content, &render_context, &render_options)
            .await?;

        // Write to file
        let file_path = self.resolve_output_path(&route_context.path, output_dir);

        // Create parent directories
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).static_gen_context("Failed to create parent directories")?;
        }

        fs::write(&file_path, &html).static_gen_context("Failed to write HTML file")?;

        Ok(html.len() as u64)
    }

    /// Create page data from metadata
    fn create_page_data(&self, metadata: &PageMetadata, path: &str) -> PageData {
        let mut page_data = PageData::default();

        page_data.title = metadata.title.clone();
        page_data.description = metadata.description.clone();
        page_data.keywords = metadata.keywords.clone();
        page_data.lang = metadata.lang.clone();

        // Set canonical URL
        if let Some(canonical) = &metadata.canonical {
            page_data.canonical = Some(canonical.clone());
        } else {
            page_data.canonical = Some(format!(
                "{}{}",
                self.config.base_url.trim_end_matches('/'),
                path
            ));
        }

        // Add Open Graph data
        if let Some(title) = &metadata.title {
            page_data.og.insert("og:title".to_string(), title.clone());
        }
        if let Some(description) = &metadata.description {
            page_data
                .og
                .insert("og:description".to_string(), description.clone());
        }
        if let Some(image) = &metadata.image {
            page_data.og.insert("og:image".to_string(), image.clone());
        }
        page_data.og.insert(
            "og:url".to_string(),
            page_data.canonical.clone().unwrap_or_default(),
        );

        // Add custom meta tags
        for (key, value) in &metadata.meta {
            page_data.meta.push(crate::render::MetaTag {
                name: Some(key.clone()),
                property: None,
                content: value.clone(),
                charset: None,
                http_equiv: None,
            });
        }

        page_data
    }

    /// Resolve output file path for a route
    fn resolve_output_path(&self, route_path: &str, output_dir: &Path) -> PathBuf {
        let clean_path = route_path.strip_prefix('/').unwrap_or(route_path);

        if clean_path.is_empty() || clean_path == "/" {
            output_dir.join("index.html")
        } else if clean_path.ends_with('/') {
            output_dir.join(clean_path).join("index.html")
        } else if clean_path.contains('.') {
            // Has file extension
            output_dir.join(clean_path)
        } else {
            // No extension, create directory with index.html
            output_dir.join(clean_path).join("index.html")
        }
    }

    /// Copy static assets to output directory
    async fn copy_static_assets(&self, output_dir: &Path) -> Result<usize> {
        let assets_dir = &self.project_config.build.static_dir;
        if !assets_dir.exists() {
            return Ok(0);
        }

        let mut assets_copied = 0;
        let assets_output_dir = output_dir.join("assets");

        for entry in WalkDir::new(assets_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let src_path = entry.path();
                let rel_path = src_path.strip_prefix(assets_dir).unwrap();
                let dest_path = assets_output_dir.join(rel_path);

                // Create parent directories
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::copy(src_path, dest_path)?;
                assets_copied += 1;
            }
        }

        Ok(assets_copied)
    }

    /// Generate sitemap.xml
    async fn generate_sitemap(&self, routes: &[String], output_dir: &Path) -> Result<()> {
        let mut sitemap = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
        );

        for route in routes {
            let url = format!("{}{}", self.config.base_url.trim_end_matches('/'), route);
            sitemap.push_str(&format!(
                r#"
  <url>
    <loc>{}</loc>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>"#,
                html_escape::encode_text(&url)
            ));
        }

        sitemap.push_str("\n</urlset>");

        let sitemap_path = output_dir.join("sitemap.xml");
        fs::write(sitemap_path, sitemap)?;

        Ok(())
    }

    /// Generate robots.txt
    async fn generate_robots_txt(&self, output_dir: &Path) -> Result<()> {
        let robots_content = format!(
            r#"User-agent: *
Allow: /

Sitemap: {}/sitemap.xml
"#,
            self.config.base_url.trim_end_matches('/')
        );

        let robots_path = output_dir.join("robots.txt");
        fs::write(robots_path, robots_content)?;

        Ok(())
    }

    /// Generate 404 error page
    async fn generate_404_page(&self, output_dir: &Path) -> Result<()> {
        let route_context = RouteContext {
            path: "/404".to_string(),
            params: HashMap::new(),
            metadata: PageMetadata {
                title: Some("Page Not Found".to_string()),
                description: Some("The requested page could not be found.".to_string()),
                keywords: vec![],
                image: None,
                canonical: None,
                lang: None,
                meta: HashMap::new(),
            },
            data: HashMap::new(),
        };

        self.generate_page(&route_context, output_dir).await?;

        Ok(())
    }
}

impl RouteResolver {
    /// Create a new route resolver
    fn new(
        static_routes: Vec<String>,
        dynamic_routes: Vec<crate::config::DynamicRoute>,
        pages_dir: PathBuf,
    ) -> Self {
        let dynamic_routes = dynamic_routes
            .into_iter()
            .map(|dr| DynamicRoute {
                pattern: dr.pattern,
                component: dr.template,
                data_source: Some(dr.data_source),
                params: HashMap::new(),
            })
            .collect();

        Self {
            static_routes,
            dynamic_routes,
            pages_dir,
        }
    }

    /// Resolve all routes to generate
    async fn resolve_routes(&self) -> Result<Vec<RouteContext>> {
        let mut routes = Vec::new();

        // Add static routes
        for route in &self.static_routes {
            routes.push(RouteContext {
                path: route.clone(),
                params: HashMap::new(),
                metadata: PageMetadata::default(),
                data: HashMap::new(),
            });
        }

        // Discover routes from pages directory
        if self.pages_dir.exists() {
            routes.extend(self.discover_page_routes().await?);
        }

        // Generate dynamic routes
        for dynamic_route in &self.dynamic_routes {
            routes.extend(self.resolve_dynamic_route(dynamic_route).await?);
        }

        // Ensure index route exists
        if !routes.iter().any(|r| r.path == "/" || r.path.is_empty()) {
            routes.push(RouteContext {
                path: "/".to_string(),
                params: HashMap::new(),
                metadata: PageMetadata {
                    title: Some("Home".to_string()),
                    description: None,
                    keywords: vec![],
                    image: None,
                    canonical: None,
                    lang: None,
                    meta: HashMap::new(),
                },
                data: HashMap::new(),
            });
        }

        Ok(routes)
    }

    /// Discover routes from pages directory
    async fn discover_page_routes(&self) -> Result<Vec<RouteContext>> {
        let mut routes = Vec::new();

        for entry in WalkDir::new(&self.pages_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "rs" || ext == "html" || ext == "ruitl" {
                        let route_path = self.file_path_to_route(path)?;
                        routes.push(RouteContext {
                            path: route_path,
                            params: HashMap::new(),
                            metadata: self.extract_page_metadata(path).await?,
                            data: HashMap::new(),
                        });
                    }
                }
            }
        }

        Ok(routes)
    }

    /// Convert file path to route path
    fn file_path_to_route(&self, file_path: &Path) -> Result<String> {
        let rel_path = file_path
            .strip_prefix(&self.pages_dir)
            .map_err(|_| RuitlError::static_gen("Invalid page path"))?;

        let route_path = rel_path
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");

        let route = if route_path == "index" {
            "/".to_string()
        } else {
            format!("/{}", route_path)
        };

        Ok(route)
    }

    /// Extract metadata from page file
    async fn extract_page_metadata(&self, _file_path: &Path) -> Result<PageMetadata> {
        // TODO: Parse frontmatter or comments to extract metadata
        Ok(PageMetadata::default())
    }

    /// Resolve dynamic route instances
    async fn resolve_dynamic_route(
        &self,
        _dynamic_route: &DynamicRoute,
    ) -> Result<Vec<RouteContext>> {
        // TODO: Load data from data source and generate route instances
        Ok(Vec::new())
    }
}

impl Default for PageMetadata {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            keywords: Vec::new(),
            image: None,
            canonical: None,
            lang: None,
            meta: HashMap::new(),
        }
    }
}

impl Default for StaticStats {
    fn default() -> Self {
        Self {
            pages_generated: 0,
            assets_copied: 0,
            total_size: 0,
            generation_time: 0.0,
            routes: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_static_generator_creation() {
        let static_config = StaticConfig::default();
        let project_config = RuitlConfig::default();

        let generator = StaticGenerator::new(static_config, project_config);
        assert!(generator.is_ok());
    }

    #[test]
    fn test_route_resolution() {
        let temp_dir = tempdir().unwrap();
        let pages_dir = temp_dir.path().join("pages");
        fs::create_dir_all(&pages_dir).unwrap();

        let resolver = RouteResolver::new(vec!["/about".to_string()], vec![], pages_dir);

        assert_eq!(resolver.static_routes.len(), 1);
        assert_eq!(resolver.static_routes[0], "/about");
    }

    #[test]
    fn test_output_path_resolution() {
        let static_config = StaticConfig::default();
        let project_config = RuitlConfig::default();
        let generator = StaticGenerator::new(static_config, project_config).unwrap();

        let output_dir = Path::new("/output");

        assert_eq!(
            generator.resolve_output_path("/", output_dir),
            Path::new("/output/index.html")
        );

        assert_eq!(
            generator.resolve_output_path("/about", output_dir),
            Path::new("/output/about/index.html")
        );

        assert_eq!(
            generator.resolve_output_path("/blog/", output_dir),
            Path::new("/output/blog/index.html")
        );

        assert_eq!(
            generator.resolve_output_path("/sitemap.xml", output_dir),
            Path::new("/output/sitemap.xml")
        );
    }

    #[test]
    fn test_page_data_creation() {
        let static_config = StaticConfig {
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };
        let project_config = RuitlConfig::default();
        let generator = StaticGenerator::new(static_config, project_config).unwrap();

        let metadata = PageMetadata {
            title: Some("Test Page".to_string()),
            description: Some("A test page".to_string()),
            keywords: vec!["test".to_string()],
            image: Some("/test.jpg".to_string()),
            canonical: None,
            lang: Some("en".to_string()),
            meta: {
                let mut meta = HashMap::new();
                meta.insert("author".to_string(), "Test Author".to_string());
                meta
            },
        };

        let page_data = generator.create_page_data(&metadata, "/test");

        assert_eq!(page_data.title, Some("Test Page".to_string()));
        assert_eq!(page_data.description, Some("A test page".to_string()));
        assert_eq!(page_data.keywords, vec!["test".to_string()]);
        assert_eq!(page_data.lang, Some("en".to_string()));
        assert_eq!(
            page_data.canonical,
            Some("https://example.com/test".to_string())
        );
        assert_eq!(page_data.og.get("og:title"), Some(&"Test Page".to_string()));
    }

    #[tokio::test]
    async fn test_sitemap_generation() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path();

        let static_config = StaticConfig {
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };
        let project_config = RuitlConfig::default();
        let generator = StaticGenerator::new(static_config, project_config).unwrap();

        let routes = vec![
            "/".to_string(),
            "/about".to_string(),
            "/contact".to_string(),
        ];
        generator
            .generate_sitemap(&routes, output_dir)
            .await
            .unwrap();

        let sitemap_path = output_dir.join("sitemap.xml");
        assert!(sitemap_path.exists());

        let sitemap_content = fs::read_to_string(sitemap_path).unwrap();
        assert!(sitemap_content.contains("https://example.com/"));
        assert!(sitemap_content.contains("https://example.com/about"));
        assert!(sitemap_content.contains("https://example.com/contact"));
    }

    #[tokio::test]
    async fn test_robots_txt_generation() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path();

        let static_config = StaticConfig {
            base_url: "https://example.com".to_string(),
            ..Default::default()
        };
        let project_config = RuitlConfig::default();
        let generator = StaticGenerator::new(static_config, project_config).unwrap();

        generator.generate_robots_txt(output_dir).await.unwrap();

        let robots_path = output_dir.join("robots.txt");
        assert!(robots_path.exists());

        let robots_content = fs::read_to_string(robots_path).unwrap();
        assert!(robots_content.contains("User-agent: *"));
        assert!(robots_content.contains("https://example.com/sitemap.xml"));
    }
}
