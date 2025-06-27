//! Build system for RUITL projects
//!
//! This module handles the compilation, optimization, and bundling of RUITL projects
//! for different deployment targets.

use crate::component::ComponentRegistry;
use crate::config::{BuildConfig, BuildTarget, OptimizationLevel, Platform, RuitlConfig};
use crate::error::{Result, ResultExt, RuitlError};
use crate::render::{RendererConfig, UniversalRenderer};
use crate::template::TemplateEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime};
use tokio::task;
use walkdir::WalkDir;

/// Build options for controlling the build process
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Target platform to build for
    pub target: String,
    /// Output directory
    pub output_dir: PathBuf,
    /// Build mode (debug, release)
    pub mode: String,
    /// Enable minification
    pub minify: bool,
    /// Generate source maps
    pub source_maps: bool,
    /// Custom environment variables
    pub env_vars: HashMap<String, String>,
    /// Additional build flags
    pub flags: Vec<String>,
    /// Parallelism level
    pub parallel: Option<usize>,
    /// Clean build (remove previous artifacts)
    pub clean: bool,
}

/// Build statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStats {
    /// Build start time
    pub start_time: SystemTime,
    /// Build duration in seconds
    pub duration: f64,
    /// Number of components compiled
    pub components_compiled: usize,
    /// Number of templates processed
    pub templates_processed: usize,
    /// Number of static assets copied
    pub assets_copied: usize,
    /// Output file sizes
    pub output_sizes: HashMap<String, u64>,
    /// Build warnings
    pub warnings: Vec<String>,
    /// Build errors (if any)
    pub errors: Vec<String>,
    /// Success status
    pub success: bool,
}

/// Build artifact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildArtifact {
    /// Artifact file path
    pub path: PathBuf,
    /// Original source path
    pub source: Option<PathBuf>,
    /// Artifact type
    pub artifact_type: ArtifactType,
    /// File size in bytes
    pub size: u64,
    /// Content hash for caching
    pub hash: String,
    /// Dependencies
    pub dependencies: Vec<PathBuf>,
}

/// Types of build artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactType {
    /// Compiled binary
    Binary,
    /// HTML file
    Html,
    /// CSS stylesheet
    Css,
    /// JavaScript file
    Javascript,
    /// Image asset
    Image,
    /// Font file
    Font,
    /// Other static asset
    Static,
    /// Source map
    SourceMap,
}

/// Main build system
pub struct BuildSystem {
    config: RuitlConfig,
    template_engine: TemplateEngine,
    component_registry: ComponentRegistry,
    cache: BuildCache,
}

/// Build cache for incremental builds
#[derive(Debug)]
struct BuildCache {
    /// Cache of file hashes
    file_hashes: HashMap<PathBuf, String>,
    /// Cache of build artifacts
    artifacts: HashMap<PathBuf, BuildArtifact>,
    /// Cache directory
    cache_dir: PathBuf,
}

/// Asset processor for handling static files
struct AssetProcessor {
    config: RuitlConfig,
    output_dir: PathBuf,
}

/// Component compiler for generating optimized component code
struct ComponentCompiler {
    config: RuitlConfig,
    registry: ComponentRegistry,
}

/// Template processor for compiling templates
struct TemplateProcessor {
    engine: TemplateEngine,
    config: RuitlConfig,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            target: "web".to_string(),
            output_dir: PathBuf::from("dist"),
            mode: "release".to_string(),
            minify: false,
            source_maps: true,
            env_vars: HashMap::new(),
            flags: Vec::new(),
            parallel: None,
            clean: false,
        }
    }
}

impl BuildOptions {
    /// Create new build options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set target platform
    pub fn target<S: Into<String>>(mut self, target: S) -> Self {
        self.target = target.into();
        self
    }

    /// Set output directory
    pub fn output_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.output_dir = dir.into();
        self
    }

    /// Set build mode
    pub fn mode<S: Into<String>>(mut self, mode: S) -> Self {
        self.mode = mode.into();
        self
    }

    /// Enable minification
    pub fn minified(mut self) -> Self {
        self.minify = true;
        self
    }

    /// Disable source maps
    pub fn no_source_maps(mut self) -> Self {
        self.source_maps = false;
        self
    }

    /// Add environment variable
    pub fn env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Add build flag
    pub fn flag<S: Into<String>>(mut self, flag: S) -> Self {
        self.flags.push(flag.into());
        self
    }

    /// Set parallelism level
    pub fn parallel(mut self, level: usize) -> Self {
        self.parallel = Some(level);
        self
    }

    /// Enable clean build
    pub fn clean(mut self) -> Self {
        self.clean = true;
        self
    }
}

impl BuildSystem {
    /// Create a new build system
    pub fn new(config: RuitlConfig) -> Result<Self> {
        let cache_dir = config.build.out_dir.join(".cache");
        let cache = BuildCache::new(cache_dir)?;

        Ok(Self {
            config,
            template_engine: TemplateEngine::new(),
            component_registry: ComponentRegistry::new(),
            cache,
        })
    }

    /// Build the project with the given options
    pub async fn build(&self, options: &BuildOptions) -> Result<BuildStats> {
        let start_time = SystemTime::now();
        let build_start = Instant::now();

        println!("🔨 Building {} project...", options.target);

        // Initialize build stats
        let mut stats = BuildStats {
            start_time,
            duration: 0.0,
            components_compiled: 0,
            templates_processed: 0,
            assets_copied: 0,
            output_sizes: HashMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            success: false,
        };

        // Clean output directory if requested
        if options.clean && options.output_dir.exists() {
            fs::remove_dir_all(&options.output_dir)
                .build_context("Failed to clean output directory")?;
        }

        // Create output directory
        fs::create_dir_all(&options.output_dir)
            .build_context("Failed to create output directory")?;

        // Build steps
        match self.build_internal(options, &mut stats).await {
            Ok(()) => {
                stats.success = true;
                stats.duration = build_start.elapsed().as_secs_f64();
                println!("✅ Build completed in {:.2}s", stats.duration);
                self.print_build_summary(&stats);
            }
            Err(e) => {
                stats.success = false;
                stats.duration = build_start.elapsed().as_secs_f64();
                stats.errors.push(e.to_string());
                println!("❌ Build failed: {}", e);
                return Err(e);
            }
        }

        Ok(stats)
    }

    /// Internal build implementation
    async fn build_internal(&self, options: &BuildOptions, stats: &mut BuildStats) -> Result<()> {
        // Step 1: Compile components
        println!("📦 Compiling components...");
        let component_compiler =
            ComponentCompiler::new(self.config.clone(), self.component_registry.clone());
        stats.components_compiled = component_compiler
            .compile_components(&options.output_dir)
            .await?;

        // Step 2: Process templates
        println!("📄 Processing templates...");
        let template_processor =
            TemplateProcessor::new(self.template_engine.clone(), self.config.clone());
        stats.templates_processed = template_processor
            .process_templates(&options.output_dir)
            .await?;

        // Step 3: Process assets
        println!("🎨 Processing assets...");
        let asset_processor = AssetProcessor::new(self.config.clone(), options.output_dir.clone());
        stats.assets_copied = asset_processor.process_assets().await?;

        // Step 4: Generate main binary/application
        match options.target.as_str() {
            "web" => self.build_web_target(options, stats).await?,
            "server" => self.build_server_target(options, stats).await?,
            "static" => self.build_static_target(options, stats).await?,
            "serverless" => self.build_serverless_target(options, stats).await?,
            _ => {
                return Err(RuitlError::build(format!(
                    "Unknown target: {}",
                    options.target
                )));
            }
        }

        // Step 5: Apply optimizations
        if options.minify {
            println!("🗜️  Applying optimizations...");
            self.apply_optimizations(&options.output_dir, stats).await?;
        }

        // Step 6: Generate manifest
        self.generate_manifest(&options.output_dir, stats).await?;

        Ok(())
    }

    /// Build for web target
    async fn build_web_target(&self, options: &BuildOptions, stats: &mut BuildStats) -> Result<()> {
        // Generate HTML pages
        let html_files = self.generate_html_pages(&options.output_dir).await?;

        for (path, size) in html_files {
            stats.output_sizes.insert(path, size);
        }

        // Compile Rust to WebAssembly if needed
        if self
            .config
            .build
            .targets
            .iter()
            .any(|t| t.platform == Platform::Web)
        {
            self.compile_wasm(&options.output_dir).await?;
        }

        Ok(())
    }

    /// Build for server target
    async fn build_server_target(
        &self,
        options: &BuildOptions,
        stats: &mut BuildStats,
    ) -> Result<()> {
        // Compile server binary
        let binary_path = self
            .compile_server_binary(&options.output_dir, &options.mode)
            .await?;
        let binary_size = fs::metadata(&binary_path)?.len();
        stats
            .output_sizes
            .insert(binary_path.to_string_lossy().to_string(), binary_size);

        Ok(())
    }

    /// Build for static target
    async fn build_static_target(
        &self,
        options: &BuildOptions,
        stats: &mut BuildStats,
    ) -> Result<()> {
        // Generate all static pages
        let static_files = self.generate_static_pages(&options.output_dir).await?;

        for (path, size) in static_files {
            stats.output_sizes.insert(path, size);
        }

        Ok(())
    }

    /// Build for serverless target
    async fn build_serverless_target(
        &self,
        options: &BuildOptions,
        stats: &mut BuildStats,
    ) -> Result<()> {
        // Compile serverless function
        let function_path = self
            .compile_serverless_function(&options.output_dir, &options.mode)
            .await?;
        let function_size = fs::metadata(&function_path)?.len();
        stats
            .output_sizes
            .insert(function_path.to_string_lossy().to_string(), function_size);

        Ok(())
    }

    /// Generate HTML pages
    async fn generate_html_pages(&self, output_dir: &Path) -> Result<Vec<(String, u64)>> {
        let mut html_files = Vec::new();

        // TODO: Implement HTML generation based on routes and components
        let index_html = r#"<!DOCTYPE html>
<html>
<head>
    <title>RUITL App</title>
</head>
<body>
    <div id="app">Loading...</div>
</body>
</html>"#;

        let index_path = output_dir.join("index.html");
        fs::write(&index_path, index_html)?;
        let size = fs::metadata(&index_path)?.len();
        html_files.push(("index.html".to_string(), size));

        Ok(html_files)
    }

    /// Compile to WebAssembly
    async fn compile_wasm(&self, output_dir: &Path) -> Result<()> {
        let wasm_output = output_dir.join("app.wasm");

        // Use wasm-pack to compile Rust to WebAssembly
        let output = Command::new("wasm-pack")
            .args(&[
                "build",
                "--target",
                "web",
                "--out-dir",
                &output_dir.to_string_lossy(),
                "--out-name",
                "app",
            ])
            .output()
            .build_context("Failed to run wasm-pack")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RuitlError::build(format!(
                "WebAssembly compilation failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Compile server binary
    async fn compile_server_binary(&self, output_dir: &Path, mode: &str) -> Result<PathBuf> {
        let binary_path = output_dir.join("server");

        let mut cmd = Command::new("cargo");
        cmd.args(&["build", "--bin", "server"]);

        if mode == "release" {
            cmd.arg("--release");
        }

        let output = cmd
            .output()
            .build_context("Failed to compile server binary")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RuitlError::build(format!(
                "Server compilation failed: {}",
                stderr
            )));
        }

        // Copy binary to output directory
        let source_binary = if mode == "release" {
            PathBuf::from("target/release/server")
        } else {
            PathBuf::from("target/debug/server")
        };

        if source_binary.exists() {
            fs::copy(&source_binary, &binary_path)?;
        }

        Ok(binary_path)
    }

    /// Generate static pages
    async fn generate_static_pages(&self, output_dir: &Path) -> Result<Vec<(String, u64)>> {
        // This would integrate with the static site generator
        self.generate_html_pages(output_dir).await
    }

    /// Compile serverless function
    async fn compile_serverless_function(&self, output_dir: &Path, mode: &str) -> Result<PathBuf> {
        // Similar to server compilation but for serverless deployment
        self.compile_server_binary(output_dir, mode).await
    }

    /// Apply optimizations to built files
    async fn apply_optimizations(&self, output_dir: &Path, stats: &mut BuildStats) -> Result<()> {
        // Minify HTML files
        for entry in WalkDir::new(output_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();

                match path.extension().and_then(|s| s.to_str()) {
                    Some("html") => {
                        self.minify_html_file(path).await?;
                    }
                    Some("css") => {
                        self.minify_css_file(path).await?;
                    }
                    Some("js") => {
                        self.minify_js_file(path).await?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Minify HTML file
    async fn minify_html_file(&self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;

        #[cfg(feature = "minify")]
        {
            let minified =
                minify_html::minify(content.as_bytes(), &minify_html::Cfg::spec_compliant());
            fs::write(path, minified)?;
        }
        #[cfg(not(feature = "minify"))]
        {
            // Simple whitespace removal if minify feature is not enabled
            let simple_minified = content
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            fs::write(path, simple_minified)?;
        }

        Ok(())
    }

    /// Minify CSS file
    async fn minify_css_file(&self, path: &Path) -> Result<()> {
        // Simple CSS minification
        let content = fs::read_to_string(path)?;
        let minified = content
            .replace('\n', "")
            .replace('\r', "")
            .replace('\t', "")
            .replace("  ", " ");
        fs::write(path, minified)?;
        Ok(())
    }

    /// Minify JavaScript file
    async fn minify_js_file(&self, path: &Path) -> Result<()> {
        // Basic JS minification (remove comments and extra whitespace)
        let content = fs::read_to_string(path)?;
        let minified = content
            .lines()
            .map(|line| {
                // Remove comments
                if let Some(pos) = line.find("//") {
                    &line[..pos]
                } else {
                    line
                }
            })
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(path, minified)?;
        Ok(())
    }

    /// Generate build manifest
    async fn generate_manifest(&self, output_dir: &Path, stats: &BuildStats) -> Result<()> {
        let manifest_path = output_dir.join("manifest.json");
        let manifest = serde_json::to_string_pretty(stats)?;
        fs::write(manifest_path, manifest)?;
        Ok(())
    }

    /// Print build summary
    fn print_build_summary(&self, stats: &BuildStats) {
        println!();
        println!("📊 Build Summary:");
        println!("   Components: {}", stats.components_compiled);
        println!("   Templates:  {}", stats.templates_processed);
        println!("   Assets:     {}", stats.assets_copied);

        if !stats.output_sizes.is_empty() {
            println!("   Output files:");
            for (file, size) in &stats.output_sizes {
                println!("     {} ({} bytes)", file, size);
            }
        }

        if !stats.warnings.is_empty() {
            println!("   Warnings: {}", stats.warnings.len());
            for warning in &stats.warnings {
                println!("     ⚠️  {}", warning);
            }
        }
    }
}

impl BuildCache {
    fn new(cache_dir: PathBuf) -> Result<Self> {
        fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            file_hashes: HashMap::new(),
            artifacts: HashMap::new(),
            cache_dir,
        })
    }

    fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let content = fs::read(path)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }

    fn is_file_changed(&mut self, path: &Path) -> Result<bool> {
        let new_hash = self.calculate_file_hash(path)?;
        let old_hash = self.file_hashes.get(path);

        if let Some(old_hash) = old_hash {
            Ok(new_hash != *old_hash)
        } else {
            self.file_hashes.insert(path.to_path_buf(), new_hash);
            Ok(true)
        }
    }
}

impl AssetProcessor {
    fn new(config: RuitlConfig, output_dir: PathBuf) -> Self {
        Self { config, output_dir }
    }

    async fn process_assets(&self) -> Result<usize> {
        let mut assets_copied = 0;
        let assets_dir = &self.config.build.static_dir;

        if !assets_dir.exists() {
            return Ok(0);
        }

        for entry in WalkDir::new(assets_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let src_path = entry.path();
                let rel_path = src_path.strip_prefix(assets_dir).unwrap();
                let dest_path = self.output_dir.join(rel_path);

                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::copy(src_path, dest_path)?;
                assets_copied += 1;
            }
        }

        Ok(assets_copied)
    }
}

impl ComponentCompiler {
    fn new(config: RuitlConfig, registry: ComponentRegistry) -> Self {
        Self { config, registry }
    }

    async fn compile_components(&self, output_dir: &Path) -> Result<usize> {
        let mut components_compiled = 0;
        let components_dir = &self.config.components.dirs[0];

        if !components_dir.exists() {
            return Ok(0);
        }

        // For now, just copy component files
        // In a real implementation, this would compile components to optimized code
        for entry in WalkDir::new(components_dir) {
            let entry = entry?;
            if entry.file_type().is_file()
                && entry.path().extension().map_or(false, |ext| ext == "rs")
            {
                components_compiled += 1;
            }
        }

        Ok(components_compiled)
    }
}

impl TemplateProcessor {
    fn new(engine: TemplateEngine, config: RuitlConfig) -> Self {
        Self { engine, config }
    }

    async fn process_templates(&self, output_dir: &Path) -> Result<usize> {
        let mut templates_processed = 0;
        let templates_dir = &self.config.build.template_dir;

        if !templates_dir.exists() {
            return Ok(0);
        }

        for entry in WalkDir::new(templates_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "html" || ext == "ruitl" {
                        templates_processed += 1;
                    }
                }
            }
        }

        Ok(templates_processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_build_options() {
        let options = BuildOptions::new()
            .target("web")
            .mode("release")
            .minified()
            .env("NODE_ENV", "production");

        assert_eq!(options.target, "web");
        assert_eq!(options.mode, "release");
        assert!(options.minify);
        assert_eq!(
            options.env_vars.get("NODE_ENV"),
            Some(&"production".to_string())
        );
    }

    #[test]
    fn test_build_stats() {
        let stats = BuildStats {
            start_time: SystemTime::now(),
            duration: 1.5,
            components_compiled: 5,
            templates_processed: 3,
            assets_copied: 10,
            output_sizes: HashMap::new(),
            warnings: vec!["test warning".to_string()],
            errors: Vec::new(),
            success: true,
        };

        assert_eq!(stats.components_compiled, 5);
        assert_eq!(stats.templates_processed, 3);
        assert_eq!(stats.assets_copied, 10);
        assert!(stats.success);
        assert_eq!(stats.warnings.len(), 1);
    }

    #[tokio::test]
    async fn test_build_system_creation() {
        let config = RuitlConfig::default();
        let build_system = BuildSystem::new(config);
        assert!(build_system.is_ok());
    }

    #[test]
    fn test_build_cache() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = temp_dir.path().join("cache");
        let cache = BuildCache::new(cache_dir);
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn test_asset_processor() {
        let temp_dir = tempdir().unwrap();
        let config = RuitlConfig::default();
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        let processor = AssetProcessor::new(config, output_dir);
        let result = processor.process_assets().await;
        assert!(result.is_ok());
    }
}
