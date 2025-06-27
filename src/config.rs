//! Configuration system for RUITL projects
//!
//! This module handles loading, parsing, and managing configuration for RUITL projects.
//! Configuration can be loaded from files, environment variables, and command-line arguments.

use crate::error::{Result, RuitlError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure for RUITL projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuitlConfig {
    /// Project metadata
    pub project: ProjectConfig,

    /// Build configuration
    pub build: BuildConfig,

    /// Development server configuration
    pub dev: DevConfig,

    /// Static site generation configuration
    pub static_gen: StaticConfig,

    /// Server-side rendering configuration
    pub ssr: SsrConfig,

    /// Template configuration
    pub templates: TemplateConfig,

    /// Component configuration
    pub components: ComponentConfig,

    /// Asset configuration
    pub assets: AssetConfig,

    /// Plugin configuration
    pub plugins: Vec<PluginConfig>,

    /// Environment-specific configurations
    pub environments: HashMap<String, EnvironmentConfig>,

    /// Custom configuration values
    pub custom: HashMap<String, toml::Value>,
}

/// Project metadata configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,

    /// Project version
    pub version: String,

    /// Project description
    pub description: Option<String>,

    /// Project author
    pub author: Option<String>,

    /// Project license
    pub license: Option<String>,

    /// Project homepage
    pub homepage: Option<String>,

    /// Project repository
    pub repository: Option<String>,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Source directory
    pub src_dir: PathBuf,

    /// Output directory
    pub out_dir: PathBuf,

    /// Template directory
    pub template_dir: PathBuf,

    /// Static assets directory
    pub static_dir: PathBuf,

    /// Whether to minify output
    pub minify: bool,

    /// Whether to generate source maps
    pub source_maps: bool,

    /// Target environments
    pub targets: Vec<BuildTarget>,

    /// Build optimization level
    pub optimization: OptimizationLevel,

    /// Whether to include debug information
    pub debug: bool,

    /// Custom build hooks
    pub hooks: BuildHooks,
}

/// Build target configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTarget {
    /// Target name (e.g., "web", "server", "static")
    pub name: String,

    /// Target platform
    pub platform: Platform,

    /// Target-specific configuration
    pub config: HashMap<String, toml::Value>,
}

/// Platform types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Web,
    Server,
    Static,
    Serverless,
    Desktop,
    Mobile,
}

/// Optimization levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OptimizationLevel {
    None,
    Basic,
    Full,
    Aggressive,
}

/// Build hooks configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildHooks {
    /// Commands to run before build
    pub pre_build: Vec<String>,

    /// Commands to run after build
    pub post_build: Vec<String>,

    /// Commands to run on build error
    pub on_error: Vec<String>,
}

/// Development server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevConfig {
    /// Server port
    pub port: u16,

    /// Server host
    pub host: String,

    /// Whether to open browser automatically
    pub open: bool,

    /// Whether to enable hot reload
    pub hot_reload: bool,

    /// Whether to enable live reload
    pub live_reload: bool,

    /// Watch patterns for file changes
    pub watch: Vec<String>,

    /// Files/patterns to ignore
    pub ignore: Vec<String>,

    /// Proxy configuration
    pub proxy: HashMap<String, ProxyConfig>,

    /// HTTPS configuration
    pub https: Option<HttpsConfig>,

    /// CORS configuration
    pub cors: CorsConfig,
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Target URL
    pub target: String,

    /// Whether to change origin
    pub change_origin: bool,

    /// Path rewrite rules
    pub path_rewrite: HashMap<String, String>,
}

/// HTTPS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpsConfig {
    /// Certificate file path
    pub cert: PathBuf,

    /// Private key file path
    pub key: PathBuf,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    pub origins: Vec<String>,

    /// Allowed methods
    pub methods: Vec<String>,

    /// Allowed headers
    pub headers: Vec<String>,

    /// Whether to allow credentials
    pub credentials: bool,
}

/// Static site generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticConfig {
    /// Base URL for the site
    pub base_url: String,

    /// Whether to generate 404 page
    pub generate_404: bool,

    /// Whether to generate sitemap
    pub generate_sitemap: bool,

    /// Whether to generate robots.txt
    pub generate_robots: bool,

    /// Routes to pre-render
    pub routes: Vec<String>,

    /// Dynamic route patterns
    pub dynamic_routes: Vec<DynamicRoute>,

    /// SEO configuration
    pub seo: SeoConfig,
}

/// Dynamic route configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicRoute {
    /// Route pattern
    pub pattern: String,

    /// Data source for generating routes
    pub data_source: String,

    /// Template to use
    pub template: String,
}

/// SEO configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoConfig {
    /// Default meta title
    pub default_title: Option<String>,

    /// Default meta description
    pub default_description: Option<String>,

    /// Default meta keywords
    pub default_keywords: Vec<String>,

    /// Open Graph configuration
    pub og: OpenGraphConfig,

    /// Twitter Card configuration
    pub twitter: TwitterConfig,
}

/// Open Graph configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenGraphConfig {
    /// Default OG title
    pub title: Option<String>,

    /// Default OG description
    pub description: Option<String>,

    /// Default OG image
    pub image: Option<String>,

    /// Site name
    pub site_name: Option<String>,
}

/// Twitter Card configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterConfig {
    /// Card type
    pub card: Option<String>,

    /// Twitter handle
    pub site: Option<String>,

    /// Creator handle
    pub creator: Option<String>,
}

/// Server-side rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsrConfig {
    /// Server port
    pub port: u16,

    /// Server host
    pub host: String,

    /// Whether to enable caching
    pub cache: bool,

    /// Cache configuration
    pub cache_config: CacheConfig,

    /// Session configuration
    pub session: SessionConfig,

    /// Database configuration
    pub database: Option<DatabaseConfig>,

    /// Environment variables to expose to templates
    pub expose_env: Vec<String>,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache type (memory, redis, file)
    pub cache_type: CacheType,

    /// Cache TTL in seconds
    pub ttl: u64,

    /// Maximum cache size
    pub max_size: Option<usize>,

    /// Cache key prefix
    pub prefix: String,
}

/// Cache types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheType {
    Memory,
    Redis,
    File,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session secret key
    pub secret: String,

    /// Session name
    pub name: String,

    /// Session max age in seconds
    pub max_age: u64,

    /// Whether session is secure
    pub secure: bool,

    /// SameSite attribute
    pub same_site: SameSite,
}

/// SameSite values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,

    /// Maximum number of connections
    pub max_connections: u32,

    /// Connection timeout in seconds
    pub timeout: u64,
}

/// Template configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    /// Template file extensions
    pub extensions: Vec<String>,

    /// Template engine settings
    pub engine: TemplateEngineConfig,

    /// Global template variables
    pub globals: HashMap<String, toml::Value>,

    /// Template filters
    pub filters: HashMap<String, String>,

    /// Template includes directory
    pub includes_dir: PathBuf,

    /// Template layouts directory
    pub layouts_dir: PathBuf,
}

/// Template engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEngineConfig {
    /// Whether to auto-escape HTML
    pub auto_escape: bool,

    /// Whether to trim whitespace
    pub trim_blocks: bool,

    /// Whether to strip line statements
    pub lstrip_blocks: bool,

    /// Custom delimiters
    pub delimiters: DelimiterConfig,
}

/// Delimiter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelimiterConfig {
    /// Variable delimiter start
    pub variable_start: String,

    /// Variable delimiter end
    pub variable_end: String,

    /// Block delimiter start
    pub block_start: String,

    /// Block delimiter end
    pub block_end: String,

    /// Comment delimiter start
    pub comment_start: String,

    /// Comment delimiter end
    pub comment_end: String,
}

/// Component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    /// Component directories
    pub dirs: Vec<PathBuf>,

    /// Component file extensions
    pub extensions: Vec<String>,

    /// Auto-import components
    pub auto_import: bool,

    /// Component naming convention
    pub naming: NamingConvention,

    /// Component style handling
    pub styles: ComponentStyleConfig,

    /// Component script handling
    pub scripts: ComponentScriptConfig,
}

/// Naming conventions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamingConvention {
    PascalCase,
    CamelCase,
    SnakeCase,
    KebabCase,
}

/// Component style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStyleConfig {
    /// Whether to scope styles
    pub scoped: bool,

    /// CSS modules configuration
    pub css_modules: bool,

    /// PostCSS configuration
    pub postcss: Option<PostCssConfig>,

    /// Sass configuration
    pub sass: Option<SassConfig>,
}

/// PostCSS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCssConfig {
    /// PostCSS plugins
    pub plugins: Vec<String>,

    /// PostCSS configuration file
    pub config_file: Option<PathBuf>,
}

/// Sass configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SassConfig {
    /// Include paths
    pub include_paths: Vec<PathBuf>,

    /// Output style
    pub output_style: SassOutputStyle,

    /// Whether to generate source maps
    pub source_maps: bool,
}

/// Sass output styles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SassOutputStyle {
    Nested,
    Expanded,
    Compact,
    Compressed,
}

/// Component script configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentScriptConfig {
    /// Whether to enable TypeScript
    pub typescript: bool,

    /// Babel configuration
    pub babel: Option<BabelConfig>,

    /// ESLint configuration
    pub eslint: Option<EslintConfig>,
}

/// Babel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BabelConfig {
    /// Babel presets
    pub presets: Vec<String>,

    /// Babel plugins
    pub plugins: Vec<String>,

    /// Babel configuration file
    pub config_file: Option<PathBuf>,
}

/// ESLint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EslintConfig {
    /// ESLint configuration file
    pub config_file: PathBuf,

    /// Whether to fail build on errors
    pub fail_on_error: bool,

    /// Whether to fail build on warnings
    pub fail_on_warning: bool,
}

/// Asset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    /// Asset directories
    pub dirs: Vec<PathBuf>,

    /// Public path for assets
    pub public_path: String,

    /// Asset file name template
    pub filename: String,

    /// Whether to hash file names
    pub hash: bool,

    /// Image optimization
    pub images: ImageConfig,

    /// Font optimization
    pub fonts: FontConfig,
}

/// Image configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Whether to optimize images
    pub optimize: bool,

    /// Image formats to generate
    pub formats: Vec<ImageFormat>,

    /// Image sizes to generate
    pub sizes: Vec<u32>,

    /// Image quality
    pub quality: u8,
}

/// Image formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Avif,
    Svg,
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// Whether to inline small fonts
    pub inline: bool,

    /// Font size threshold for inlining
    pub inline_limit: usize,

    /// Font formats to generate
    pub formats: Vec<FontFormat>,
}

/// Font formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontFormat {
    Woff,
    Woff2,
    Ttf,
    Eot,
    Svg,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin name
    pub name: String,

    /// Plugin version constraint
    pub version: Option<String>,

    /// Plugin configuration
    pub config: HashMap<String, toml::Value>,

    /// Whether plugin is enabled
    pub enabled: bool,
}

/// Environment-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Build configuration overrides
    pub build: Option<BuildConfigOverride>,

    /// Development server overrides
    pub dev: Option<DevConfigOverride>,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Custom configuration overrides
    pub custom: HashMap<String, toml::Value>,
}

/// Build configuration overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfigOverride {
    pub minify: Option<bool>,
    pub source_maps: Option<bool>,
    pub optimization: Option<OptimizationLevel>,
    pub debug: Option<bool>,
}

/// Development server configuration overrides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevConfigOverride {
    pub port: Option<u16>,
    pub host: Option<String>,
    pub open: Option<bool>,
    pub hot_reload: Option<bool>,
    pub live_reload: Option<bool>,
}

impl Default for RuitlConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            build: BuildConfig::default(),
            dev: DevConfig::default(),
            static_gen: StaticConfig::default(),
            ssr: SsrConfig::default(),
            templates: TemplateConfig::default(),
            components: ComponentConfig::default(),
            assets: AssetConfig::default(),
            plugins: Vec::new(),
            environments: HashMap::new(),
            custom: HashMap::new(),
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "ruitl-project".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            author: None,
            license: None,
            homepage: None,
            repository: None,
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            src_dir: PathBuf::from("src"),
            out_dir: PathBuf::from("dist"),
            template_dir: PathBuf::from("templates"),
            static_dir: PathBuf::from("static"),
            minify: false,
            source_maps: true,
            targets: vec![BuildTarget::default()],
            optimization: OptimizationLevel::Basic,
            debug: true,
            hooks: BuildHooks::default(),
        }
    }
}

impl Default for BuildTarget {
    fn default() -> Self {
        Self {
            name: "web".to_string(),
            platform: Platform::Web,
            config: HashMap::new(),
        }
    }
}

impl Default for BuildHooks {
    fn default() -> Self {
        Self {
            pre_build: Vec::new(),
            post_build: Vec::new(),
            on_error: Vec::new(),
        }
    }
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "localhost".to_string(),
            open: false,
            hot_reload: true,
            live_reload: true,
            watch: vec!["src/**/*".to_string(), "templates/**/*".to_string()],
            ignore: vec!["node_modules/**".to_string(), "dist/**".to_string()],
            proxy: HashMap::new(),
            https: None,
            cors: CorsConfig::default(),
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            origins: vec!["*".to_string()],
            methods: vec!["GET".to_string(), "POST".to_string()],
            headers: vec!["Content-Type".to_string()],
            credentials: false,
        }
    }
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            base_url: "/".to_string(),
            generate_404: true,
            generate_sitemap: true,
            generate_robots: true,
            routes: Vec::new(),
            dynamic_routes: Vec::new(),
            seo: SeoConfig::default(),
        }
    }
}

impl Default for SeoConfig {
    fn default() -> Self {
        Self {
            default_title: None,
            default_description: None,
            default_keywords: Vec::new(),
            og: OpenGraphConfig::default(),
            twitter: TwitterConfig::default(),
        }
    }
}

impl Default for OpenGraphConfig {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            image: None,
            site_name: None,
        }
    }
}

impl Default for TwitterConfig {
    fn default() -> Self {
        Self {
            card: None,
            site: None,
            creator: None,
        }
    }
}

impl Default for SsrConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            host: "0.0.0.0".to_string(),
            cache: true,
            cache_config: CacheConfig::default(),
            session: SessionConfig::default(),
            database: None,
            expose_env: Vec::new(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_type: CacheType::Memory,
            ttl: 3600,
            max_size: Some(1000),
            prefix: "ruitl:".to_string(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            secret: "change-me-in-production".to_string(),
            name: "ruitl.sid".to_string(),
            max_age: 86400,
            secure: false,
            same_site: SameSite::Lax,
        }
    }
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            extensions: vec!["ruitl".to_string(), "html".to_string()],
            engine: TemplateEngineConfig::default(),
            globals: HashMap::new(),
            filters: HashMap::new(),
            includes_dir: PathBuf::from("templates/includes"),
            layouts_dir: PathBuf::from("templates/layouts"),
        }
    }
}

impl Default for TemplateEngineConfig {
    fn default() -> Self {
        Self {
            auto_escape: true,
            trim_blocks: true,
            lstrip_blocks: true,
            delimiters: DelimiterConfig::default(),
        }
    }
}

impl Default for DelimiterConfig {
    fn default() -> Self {
        Self {
            variable_start: "{{".to_string(),
            variable_end: "}}".to_string(),
            block_start: "{%".to_string(),
            block_end: "%}".to_string(),
            comment_start: "{#".to_string(),
            comment_end: "#}".to_string(),
        }
    }
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            dirs: vec![PathBuf::from("src/components")],
            extensions: vec!["rs".to_string()],
            auto_import: true,
            naming: NamingConvention::PascalCase,
            styles: ComponentStyleConfig::default(),
            scripts: ComponentScriptConfig::default(),
        }
    }
}

impl Default for ComponentStyleConfig {
    fn default() -> Self {
        Self {
            scoped: true,
            css_modules: false,
            postcss: None,
            sass: None,
        }
    }
}

impl Default for ComponentScriptConfig {
    fn default() -> Self {
        Self {
            typescript: false,
            babel: None,
            eslint: None,
        }
    }
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            dirs: vec![PathBuf::from("static")],
            public_path: "/".to_string(),
            filename: "[name].[hash].[ext]".to_string(),
            hash: true,
            images: ImageConfig::default(),
            fonts: FontConfig::default(),
        }
    }
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            optimize: true,
            formats: vec![ImageFormat::Webp],
            sizes: vec![320, 640, 1024, 1920],
            quality: 80,
        }
    }
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            inline: true,
            inline_limit: 8192,
            formats: vec![FontFormat::Woff2, FontFormat::Woff],
        }
    }
}

impl RuitlConfig {
    /// Load configuration from a file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| RuitlError::config(format!("Failed to read config file: {}", e)))?;

        Self::load_from_str(&content)
    }

    /// Load configuration from a string
    pub fn load_from_str(content: &str) -> Result<Self> {
        toml::from_str(content)
            .map_err(|e| RuitlError::config(format!("Failed to parse config: {}", e)))
    }

    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| RuitlError::config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path.as_ref(), content)
            .map_err(|e| RuitlError::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Load configuration with environment overrides
    pub fn load_with_env<P: AsRef<Path>>(path: P, env: &str) -> Result<Self> {
        let mut config = Self::load_from_file(path)?;

        if let Some(env_config) = config.environments.get(env).cloned() {
            config.apply_environment_overrides(&env_config);
        }

        Ok(config)
    }

    /// Apply environment-specific configuration overrides
    fn apply_environment_overrides(&mut self, env_config: &EnvironmentConfig) {
        if let Some(build_override) = &env_config.build {
            if let Some(minify) = build_override.minify {
                self.build.minify = minify;
            }
            if let Some(source_maps) = build_override.source_maps {
                self.build.source_maps = source_maps;
            }
            if let Some(optimization) = &build_override.optimization {
                self.build.optimization = optimization.clone();
            }
            if let Some(debug) = build_override.debug {
                self.build.debug = debug;
            }
        }

        if let Some(dev_override) = &env_config.dev {
            if let Some(port) = dev_override.port {
                self.dev.port = port;
            }
            if let Some(host) = &dev_override.host {
                self.dev.host = host.clone();
            }
            if let Some(open) = dev_override.open {
                self.dev.open = open;
            }
            if let Some(hot_reload) = dev_override.hot_reload {
                self.dev.hot_reload = hot_reload;
            }
            if let Some(live_reload) = dev_override.live_reload {
                self.dev.live_reload = live_reload;
            }
        }
    }

    /// Get configuration for a specific environment
    pub fn for_environment(&self, env: &str) -> Self {
        let mut config = self.clone();

        if let Some(env_config) = self.environments.get(env) {
            config.apply_environment_overrides(env_config);
        }

        config
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate project configuration
        if self.project.name.is_empty() {
            return Err(RuitlError::config("Project name cannot be empty"));
        }

        // Validate build configuration
        if !self.build.src_dir.exists() {
            return Err(RuitlError::config(format!(
                "Source directory does not exist: {}",
                self.build.src_dir.display()
            )));
        }

        // Validate development server configuration
        if self.dev.port == 0 {
            return Err(RuitlError::config("Development server port cannot be 0"));
        }

        // Validate SSR configuration
        if self.ssr.port == 0 {
            return Err(RuitlError::config("SSR server port cannot be 0"));
        }

        // Validate static generation configuration
        if self.static_gen.base_url.is_empty() {
            return Err(RuitlError::config("Base URL cannot be empty"));
        }

        Ok(())
    }

    /// Get the full path for a relative path based on source directory
    pub fn resolve_src_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.build.src_dir.join(path)
    }

    /// Get the full path for a relative path based on output directory
    pub fn resolve_out_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.build.out_dir.join(path)
    }

    /// Get the full path for a relative path based on template directory
    pub fn resolve_template_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.build.template_dir.join(path)
    }

    /// Get the full path for a relative path based on static directory
    pub fn resolve_static_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.build.static_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = RuitlConfig::default();
        assert_eq!(config.project.name, "ruitl-project");
        assert_eq!(config.build.src_dir, PathBuf::from("src"));
        assert_eq!(config.dev.port, 3000);
    }

    #[test]
    fn test_config_validation() {
        let mut config = RuitlConfig::default();

        // Create temporary source directory
        let temp_dir = tempdir().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        config.build.src_dir = src_dir;

        assert!(config.validate().is_ok());

        // Test invalid configuration
        config.project.name = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization() {
        let config = RuitlConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        assert!(!serialized.is_empty());

        let deserialized: RuitlConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config.project.name, deserialized.project.name);
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("ruitl.toml");

        let config = RuitlConfig::default();
        assert!(config.save_to_file(&config_path).is_ok());
        assert!(config_path.exists());

        let loaded_config = RuitlConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.project.name, loaded_config.project.name);
    }

    #[test]
    fn test_environment_overrides() {
        let mut config = RuitlConfig::default();

        let mut env_config = EnvironmentConfig {
            build: Some(BuildConfigOverride {
                minify: Some(true),
                source_maps: Some(false),
                optimization: Some(OptimizationLevel::Full),
                debug: Some(false),
            }),
            dev: Some(DevConfigOverride {
                port: Some(4000),
                host: Some("0.0.0.0".to_string()),
                open: Some(true),
                hot_reload: Some(false),
                live_reload: Some(false),
            }),
            env: HashMap::new(),
            custom: HashMap::new(),
        };

        config
            .environments
            .insert("production".to_string(), env_config);

        let prod_config = config.for_environment("production");
        assert!(prod_config.build.minify);
        assert!(!prod_config.build.source_maps);
        assert_eq!(prod_config.dev.port, 4000);
        assert_eq!(prod_config.dev.host, "0.0.0.0");
    }

    #[test]
    fn test_path_resolution() {
        let config = RuitlConfig::default();

        let src_path = config.resolve_src_path("main.rs");
        assert_eq!(src_path, PathBuf::from("src/main.rs"));

        let out_path = config.resolve_out_path("index.html");
        assert_eq!(out_path, PathBuf::from("dist/index.html"));

        let template_path = config.resolve_template_path("layout.html");
        assert_eq!(template_path, PathBuf::from("templates/layout.html"));

        let static_path = config.resolve_static_path("style.css");
        assert_eq!(static_path, PathBuf::from("static/style.css"));
    }
}
