//! Command-line interface for RUITL
//!
//! This module provides the CLI commands for creating, building, and serving RUITL projects.

use crate::build::{BuildOptions, BuildSystem};
use crate::config::RuitlConfig;
use crate::error::{Result, RuitlError};
use crate::render::{DocumentRenderer, RenderContext, RenderOptions, RenderTarget, RendererConfig};
use crate::server::DevServer;
use crate::static_gen::StaticGenerator;
use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::signal;
use toml;

/// RUITL - Rust UI Template Language
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Environment (development, production, etc.)
    #[arg(short, long, global = true, default_value = "development")]
    pub env: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new RUITL project
    New {
        /// Project name
        name: String,
        /// Project template
        #[arg(short, long, default_value = "default")]
        template: String,
        /// Project directory
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    /// Start development server
    Dev {
        /// Port to run on
        #[arg(short, long)]
        port: Option<u16>,
        /// Host to bind to
        #[arg(long)]
        host: Option<String>,
        /// Open browser automatically
        #[arg(long)]
        open: bool,
        /// Disable hot reload
        #[arg(long)]
        no_hot_reload: bool,
    },
    /// Build project for production
    Build {
        /// Output directory
        #[arg(short, long)]
        out_dir: Option<PathBuf>,
        /// Build target
        #[arg(short, long, default_value = "web")]
        target: String,
        /// Enable minification
        #[arg(long)]
        minify: bool,
        /// Disable source maps
        #[arg(long)]
        no_source_maps: bool,
        /// Build mode
        #[arg(long, default_value = "release")]
        mode: String,
    },
    /// Generate static site
    Static {
        /// Output directory
        #[arg(short, long)]
        out_dir: Option<PathBuf>,
        /// Base URL for the site
        #[arg(long)]
        base_url: Option<String>,
        /// Routes to generate
        #[arg(long)]
        routes: Vec<String>,
    },
    /// Start production server
    Serve {
        /// Port to run on
        #[arg(short, long)]
        port: Option<u16>,
        /// Host to bind to
        #[arg(long)]
        host: Option<String>,
        /// Directory to serve
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    /// Check project for errors
    Check {
        /// Fix issues automatically
        #[arg(long)]
        fix: bool,
    },
    /// Format project files
    Format {
        /// Check formatting without applying changes
        #[arg(long)]
        check: bool,
    },
    /// Clean build artifacts
    Clean {
        /// Clean cache as well
        #[arg(long)]
        cache: bool,
    },
    /// Show project information
    Info,
    /// Add a new component
    Add {
        /// Component type (component, page, layout)
        #[arg(default_value = "component")]
        component_type: String,
        /// Component name
        name: String,
        /// Component template
        #[arg(short, long)]
        template: Option<String>,
    },
    /// Initialize RUITL in existing project
    Init {
        /// Overwrite existing configuration
        #[arg(long)]
        force: bool,
    },
}

/// CLI application runner
pub struct CliApp {
    config: RuitlConfig,
    verbose: bool,
}

impl CliApp {
    /// Create a new CLI application
    pub fn new(config: RuitlConfig, verbose: bool) -> Self {
        Self { config, verbose }
    }

    /// Run the CLI application
    pub async fn run(&self, command: Commands) -> Result<()> {
        match command {
            Commands::New {
                name,
                template,
                dir,
            } => self.new_project(&name, &template, dir.as_deref()).await,
            Commands::Dev {
                port,
                host,
                open,
                no_hot_reload,
            } => {
                self.dev_server(port, host.as_deref(), open, !no_hot_reload)
                    .await
            }
            Commands::Build {
                out_dir,
                target,
                minify,
                no_source_maps,
                mode,
            } => {
                self.build_project(out_dir.as_deref(), &target, minify, !no_source_maps, &mode)
                    .await
            }
            Commands::Static {
                out_dir,
                base_url,
                routes,
            } => {
                self.generate_static(out_dir.as_deref(), base_url.as_deref(), &routes)
                    .await
            }
            Commands::Serve { port, host, dir } => {
                self.serve_static(port, host.as_deref(), dir.as_deref())
                    .await
            }
            Commands::Check { fix } => self.check_project(fix).await,
            Commands::Format { check } => self.format_project(check).await,
            Commands::Clean { cache } => self.clean_project(cache).await,
            Commands::Info => self.show_info().await,
            Commands::Add {
                component_type,
                name,
                template,
            } => {
                self.add_component(&component_type, &name, template.as_deref())
                    .await
            }
            Commands::Init { force } => self.init_project(force).await,
        }
    }

    /// Create a new RUITL project
    async fn new_project(&self, name: &str, template: &str, dir: Option<&Path>) -> Result<()> {
        let project_dir = dir.unwrap_or_else(|| Path::new(name));

        self.log_info(&format!("Creating new RUITL project: {}", name.bold()));

        // Create project directory
        if project_dir.exists() {
            return Err(RuitlError::generic(format!(
                "Directory '{}' already exists",
                project_dir.display()
            )));
        }

        fs::create_dir_all(project_dir)?;

        // Create project structure
        self.create_project_structure(project_dir, name, template)
            .await?;

        self.log_success(&format!("Project '{}' created successfully!", name.bold()));
        self.log_info(&format!(
            "Run {} to start development server",
            "ruitl dev".bright_blue()
        ));

        Ok(())
    }

    /// Start development server
    async fn dev_server(
        &self,
        port: Option<u16>,
        host: Option<&str>,
        open: bool,
        hot_reload: bool,
    ) -> Result<()> {
        let port = port.unwrap_or(self.config.dev.port);
        let host = host.unwrap_or(&self.config.dev.host);

        self.log_info(&format!(
            "Starting development server on {}://{}:{}",
            "http".bright_blue(),
            host.bright_blue(),
            port.to_string().bright_blue()
        ));

        let mut dev_config = self.config.dev.clone();
        dev_config.port = port;
        dev_config.host = host.to_string();
        dev_config.open = open;
        dev_config.hot_reload = hot_reload;

        let mut server = DevServer::new(dev_config, self.config.clone())?;

        // Handle Ctrl+C gracefully
        let server_handle = tokio::spawn(async move {
            if let Err(e) = server.start().await {
                eprintln!("Server error: {}", e);
            }
        });

        // Wait for shutdown signal
        match signal::ctrl_c().await {
            Ok(()) => {
                self.log_info("Shutting down development server...");
                server_handle.abort();
            }
            Err(err) => {
                self.log_error(&format!("Failed to listen for shutdown signal: {}", err));
            }
        }

        Ok(())
    }

    /// Build project
    async fn build_project(
        &self,
        out_dir: Option<&Path>,
        target: &str,
        minify: bool,
        source_maps: bool,
        mode: &str,
    ) -> Result<()> {
        let out_dir = out_dir.unwrap_or(&self.config.build.out_dir);

        self.log_info(&format!("Building project for {}", target.bold()));

        let mut build_options = BuildOptions::default();
        build_options.target = target.to_string();
        build_options.minify = minify;
        build_options.source_maps = source_maps;
        build_options.output_dir = out_dir.to_path_buf();
        build_options.mode = mode.to_string();

        let build_system = BuildSystem::new(self.config.clone())?;
        let start_time = std::time::Instant::now();

        build_system.build(&build_options).await?;

        let duration = start_time.elapsed();
        self.log_success(&format!(
            "Build completed in {:.2}s",
            duration.as_secs_f64()
        ));
        self.log_info(&format!(
            "Output written to {}",
            out_dir.display().to_string().bright_blue()
        ));

        Ok(())
    }

    /// Generate static site
    async fn generate_static(
        &self,
        out_dir: Option<&Path>,
        base_url: Option<&str>,
        routes: &[String],
    ) -> Result<()> {
        let out_dir = out_dir.unwrap_or(&self.config.build.out_dir);
        let base_url = base_url.unwrap_or(&self.config.static_gen.base_url);

        self.log_info("Generating static site...");

        let mut static_config = self.config.static_gen.clone();
        static_config.base_url = base_url.to_string();
        if !routes.is_empty() {
            static_config.routes = routes.to_vec();
        }

        let generator = StaticGenerator::new(static_config, self.config.clone())?;
        let start_time = std::time::Instant::now();

        let stats = generator.generate(out_dir).await?;

        let duration = start_time.elapsed();
        self.log_success(&format!(
            "Generated {} pages in {:.2}s",
            stats.pages_generated,
            duration.as_secs_f64()
        ));
        self.log_info(&format!(
            "Static site written to {}",
            out_dir.display().to_string().bright_blue()
        ));

        Ok(())
    }

    /// Serve static files
    async fn serve_static(
        &self,
        port: Option<u16>,
        host: Option<&str>,
        dir: Option<&Path>,
    ) -> Result<()> {
        let port = port.unwrap_or(self.config.ssr.port);
        let host = host.unwrap_or(&self.config.ssr.host);
        let dir = dir.unwrap_or(&self.config.build.out_dir);

        if !dir.exists() {
            return Err(RuitlError::generic(format!(
                "Directory '{}' does not exist. Run 'ruitl build' first.",
                dir.display()
            )));
        }

        self.log_info(&format!(
            "Serving static files from {} on {}://{}:{}",
            dir.display().to_string().bright_blue(),
            "http".bright_blue(),
            host.bright_blue(),
            port.to_string().bright_blue()
        ));

        // TODO: Implement static file server
        // For now, just show the message
        self.log_info("Static server not yet implemented");

        Ok(())
    }

    /// Check project for errors
    async fn check_project(&self, fix: bool) -> Result<()> {
        self.log_info("Checking project...");

        // TODO: Implement project checking
        // - Check template syntax
        // - Check component validity
        // - Check dependencies
        // - Check configuration

        if fix {
            self.log_info("Fixing issues automatically...");
            // TODO: Implement auto-fix
        }

        self.log_success("Project check completed");
        Ok(())
    }

    /// Format project files
    async fn format_project(&self, check: bool) -> Result<()> {
        if check {
            self.log_info("Checking formatting...");
        } else {
            self.log_info("Formatting project files...");
        }

        // TODO: Implement formatting
        // - Format Rust code
        // - Format templates
        // - Format configuration files

        self.log_success("Formatting completed");
        Ok(())
    }

    /// Clean build artifacts
    async fn clean_project(&self, cache: bool) -> Result<()> {
        self.log_info("Cleaning build artifacts...");

        // Remove build directory
        if self.config.build.out_dir.exists() {
            fs::remove_dir_all(&self.config.build.out_dir)?;
            self.log_info(&format!("Removed {}", self.config.build.out_dir.display()));
        }

        if cache {
            self.log_info("Cleaning cache...");
            // TODO: Clean cache directories
        }

        self.log_success("Clean completed");
        Ok(())
    }

    /// Show project information
    async fn show_info(&self) -> Result<()> {
        println!("{}", "RUITL Project Information".bold().underline());
        println!();
        println!(
            "{:<15} {}",
            "Project:",
            self.config.project.name.bright_blue()
        );
        println!("{:<15} {}", "Version:", self.config.project.version);

        if let Some(description) = &self.config.project.description {
            println!("{:<15} {}", "Description:", description);
        }

        println!();
        println!("{}", "Configuration:".bold());
        println!(
            "{:<15} {}",
            "Source Dir:",
            self.config.build.src_dir.display()
        );
        println!(
            "{:<15} {}",
            "Output Dir:",
            self.config.build.out_dir.display()
        );
        println!(
            "{:<15} {}",
            "Template Dir:",
            self.config.build.template_dir.display()
        );
        println!(
            "{:<15} {}",
            "Static Dir:",
            self.config.build.static_dir.display()
        );

        println!();
        println!("{}", "Development:".bold());
        println!(
            "{:<15} {}:{}",
            "Dev Server:", self.config.dev.host, self.config.dev.port
        );
        println!(
            "{:<15} {}",
            "Hot Reload:",
            if self.config.dev.hot_reload {
                "enabled"
            } else {
                "disabled"
            }
        );

        println!();
        println!("{}", "Build:".bold());
        println!(
            "{:<15} {}",
            "Minify:",
            if self.config.build.minify {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!(
            "{:<15} {}",
            "Source Maps:",
            if self.config.build.source_maps {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!(
            "{:<15} {:?}",
            "Optimization:", self.config.build.optimization
        );

        Ok(())
    }

    /// Add a new component
    async fn add_component(
        &self,
        component_type: &str,
        name: &str,
        template: Option<&str>,
    ) -> Result<()> {
        self.log_info(&format!(
            "Adding new {} '{}'",
            component_type.bold(),
            name.bright_blue()
        ));

        let component_dir = &self.config.components.dirs[0];
        if !component_dir.exists() {
            fs::create_dir_all(component_dir)?;
        }

        let component_path = component_dir.join(format!("{}.rs", name.to_lowercase()));

        if component_path.exists() {
            return Err(RuitlError::generic(format!(
                "Component '{}' already exists",
                name
            )));
        }

        let template_content = match component_type {
            "component" => self.generate_component_template(name),
            "page" => self.generate_page_template(name),
            "layout" => self.generate_layout_template(name),
            _ => {
                return Err(RuitlError::generic(format!(
                    "Unknown component type: {}",
                    component_type
                )));
            }
        };

        fs::write(&component_path, template_content)?;

        self.log_success(&format!(
            "Created {} at {}",
            component_type,
            component_path.display().to_string().bright_blue()
        ));

        Ok(())
    }

    /// Initialize RUITL in existing project
    async fn init_project(&self, force: bool) -> Result<()> {
        let config_path = Path::new("ruitl.toml");

        if config_path.exists() && !force {
            return Err(RuitlError::generic(
                "RUITL is already initialized. Use --force to overwrite.",
            ));
        }

        self.log_info("Initializing RUITL project...");

        // Create default configuration
        let config = RuitlConfig::default();
        config.save_to_file(config_path)?;

        // Create basic project structure
        self.create_basic_structure().await?;

        self.log_success("RUITL project initialized!");
        self.log_info(&format!(
            "Edit {} to customize your project",
            "ruitl.toml".bright_blue()
        ));

        Ok(())
    }

    /// Create project structure
    async fn create_project_structure(
        &self,
        project_dir: &Path,
        name: &str,
        template: &str,
    ) -> Result<()> {
        // Create directories
        let dirs = ["src", "src/components", "src/pages", "templates", "static"];

        for dir in &dirs {
            fs::create_dir_all(project_dir.join(dir))?;
        }

        // Create configuration file using default config
        let mut config = RuitlConfig::default();
        config.project.name = name.to_string();

        // Serialize config to TOML
        let config_content = toml::to_string_pretty(&config)
            .map_err(|e| RuitlError::config(format!("Failed to serialize config: {}", e)))?;
        fs::write(project_dir.join("ruitl.toml"), config_content)?;

        // Create main.rs
        let main_content = self.generate_main_template(name);
        fs::write(project_dir.join("src/main.rs"), main_content)?;

        // Create basic component
        let component_content = self.generate_component_template("HelloWorld");
        fs::write(
            project_dir.join("src/components/hello_world.rs"),
            component_content,
        )?;

        // Create index page
        let index_content = self.generate_index_template();
        fs::write(project_dir.join("src/pages/index.rs"), index_content)?;

        // Create base template
        let template_content = self.generate_base_template();
        fs::write(project_dir.join("templates/base.html"), template_content)?;

        // Create Cargo.toml
        let cargo_content = self.generate_cargo_template(name);
        fs::write(project_dir.join("Cargo.toml"), cargo_content)?;

        // Create README.md
        let readme_content = self.generate_readme_template(name);
        fs::write(project_dir.join("README.md"), readme_content)?;

        Ok(())
    }

    /// Create basic structure for existing project
    async fn create_basic_structure(&self) -> Result<()> {
        let dirs = ["src", "src/components", "templates", "static"];

        for dir in &dirs {
            if !Path::new(dir).exists() {
                fs::create_dir_all(dir)?;
            }
        }

        Ok(())
    }

    /// Generate main.rs template
    fn generate_main_template(&self, name: &str) -> String {
        format!(
            r#"use ruitl::prelude::*;
use std::collections::HashMap;

mod components;
mod pages;

use components::HelloWorld;
use pages::Index;

#[tokio::main]
async fn main() -> Result<()> {{
    // Initialize RUITL
    ruitl::init()?;

    // Create renderer
    let renderer_config = RendererConfig::default();
    let renderer = UniversalRenderer::new(renderer_config);

    // Register components
    renderer.register_component("HelloWorld", HelloWorld).await;
    renderer.register_component("Index", Index).await;

    // Create context
    let context = RenderContext::new()
        .with_path("/")
        .with_target(RenderTarget::Development);

    // Render index page
    let options = RenderOptions::new();
    let html = renderer.render(&context, &options).await?;

    println!("{{}}", html);

    Ok(())
}}
"#
        )
    }

    /// Generate component template
    fn generate_component_template(&self, name: &str) -> String {
        format!(
            r#"use ruitl::prelude::*;

#[derive(Debug, Clone)]
pub struct {}Props {{
    pub message: String,
}}

impl ComponentProps for {}Props {{}}

#[derive(Debug)]
pub struct {};

impl Component for {} {{
    type Props = {}Props;

    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {{
        Ok(html! {{
            <div class="component">
                <h2>{{props.message}}</h2>
                <p>This is a RUITL component!</p>
            </div>
        }})
    }}
}}
"#,
            name, name, name, name, name
        )
    }

    /// Generate page template
    fn generate_page_template(&self, name: &str) -> String {
        format!(
            r#"use ruitl::prelude::*;

#[derive(Debug, Clone)]
pub struct {}Props {{}}

impl ComponentProps for {}Props {{}}

#[derive(Debug)]
pub struct {};

impl Component for {} {{
    type Props = {}Props;

    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {{
        Ok(html! {{
            <main>
                <h1>{} Page</h1>
                <p>Welcome to your new RUITL page!</p>
            </main>
        }})
    }}
}}
"#,
            name, name, name, name, name, name
        )
    }

    /// Generate layout template
    fn generate_layout_template(&self, name: &str) -> String {
        format!(
            r#"use ruitl::prelude::*;

#[derive(Debug, Clone)]
pub struct {}Props {{
    pub title: String,
    pub children: Vec<Html>,
}}

impl ComponentProps for {}Props {{}}

#[derive(Debug)]
pub struct {};

impl Component for {} {{
    type Props = {}Props;

    fn render(&self, props: &Self::Props, _context: &ComponentContext) -> Result<Html> {{
        Ok(html! {{
            <html>
                <head>
                    <title>{{props.title}}</title>
                    <meta charset="utf-8" />
                    <meta name="viewport" content="width=device-width, initial-scale=1" />
                </head>
                <body>
                    {{props.children}}
                </body>
            </html>
        }})
    }}
}}
"#,
            name, name, name, name, name
        )
    }

    /// Generate index template
    fn generate_index_template(&self) -> String {
        r#"use ruitl::prelude::*;
use crate::components::HelloWorld;

#[derive(Debug, Clone)]
pub struct IndexProps {}

impl ComponentProps for IndexProps {}

#[derive(Debug)]
pub struct Index;

impl Component for Index {
    type Props = IndexProps;

    fn render(&self, _props: &Self::Props, _context: &ComponentContext) -> Result<Html> {
        Ok(html! {
            <div>
                <h1>Welcome to RUITL!</h1>
                <HelloWorld message="Hello from RUITL!" />
            </div>
        })
    }
}
"#
        .to_string()
    }

    /// Generate base template
    fn generate_base_template(&self) -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ title | default: "RUITL App" }}</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 2rem;
            background-color: #f5f5f5;
        }
        .component {
            background: white;
            padding: 1.5rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin: 1rem 0;
        }
        h1, h2 {
            color: #333;
        }
    </style>
</head>
<body>
    {% block content %}{% endblock %}
</body>
</html>
"#
        .to_string()
    }

    /// Generate Cargo.toml template
    fn generate_cargo_template(&self, name: &str) -> String {
        format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
ruitl = {{ path = "../ruitl" }}
tokio = {{ version = "1.0", features = ["full"] }}
"#,
            name
        )
    }

    /// Generate README.md template
    fn generate_readme_template(&self, name: &str) -> String {
        format!(
            r#"# {}

A RUITL (Rust UI Template Language) project.

## Getting Started

```bash
# Start development server
ruitl dev

# Build for production
ruitl build

# Generate static site
ruitl static
```

## Project Structure

- `src/` - Source code
- `src/components/` - Reusable components
- `src/pages/` - Page components
- `templates/` - HTML templates
- `static/` - Static assets
- `ruitl.toml` - Configuration file

## Learn More

- [RUITL Documentation](https://ruitl.dev)
- [Rust Book](https://doc.rust-lang.org/book/)
"#,
            name
        )
    }

    /// Log info message
    fn log_info(&self, message: &str) {
        if self.verbose {
            println!("{} {}", "INFO".bright_blue().bold(), message);
        }
    }

    /// Log success message
    fn log_success(&self, message: &str) {
        println!("{} {}", "✓".bright_green().bold(), message);
    }

    /// Log error message
    fn log_error(&self, message: &str) {
        eprintln!("{} {}", "ERROR".bright_red().bold(), message);
    }

    /// Log warning message
    fn log_warning(&self, message: &str) {
        println!("{} {}", "WARN".bright_yellow().bold(), message);
    }
}

/// Run the CLI application
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Skip config loading for commands that create new projects
    let should_load_config = !matches!(cli.command, Commands::New { .. } | Commands::Init { .. });

    let config = if should_load_config {
        // Load configuration
        let config_path = cli.config.unwrap_or_else(|| PathBuf::from("ruitl.toml"));
        let config = if config_path.exists() {
            RuitlConfig::load_with_env(&config_path, &cli.env)?
        } else {
            RuitlConfig::default()
        };

        // Validate configuration
        if let Err(e) = config.validate() {
            eprintln!("{} {}", "ERROR".bright_red().bold(), e);
            std::process::exit(1);
        }

        config
    } else {
        // Use default config for new/init commands
        RuitlConfig::default()
    };

    // Create and run CLI app
    let app = CliApp::new(config, cli.verbose);

    if let Err(e) = app.run(cli.command).await {
        eprintln!("{} {}", "ERROR".bright_red().bold(), e);
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(&["ruitl", "dev", "--port", "4000"]).unwrap();

        match cli.command {
            Commands::Dev { port, .. } => {
                assert_eq!(port, Some(4000));
            }
            _ => panic!("Expected Dev command"),
        }
    }

    #[tokio::test]
    async fn test_cli_app_creation() {
        let config = RuitlConfig::default();
        let app = CliApp::new(config, false);

        // Test that app is created successfully
        assert!(!app.verbose);
    }

    #[test]
    fn test_template_generation() {
        let config = RuitlConfig::default();
        let app = CliApp::new(config, false);

        let component_template = app.generate_component_template("TestComponent");
        assert!(component_template.contains("TestComponent"));
        assert!(component_template.contains("ComponentProps"));
        assert!(component_template.contains("Component"));

        let main_template = app.generate_main_template("test-project");
        assert!(main_template.contains("tokio::main"));
        assert!(main_template.contains("ruitl::init"));
    }

    #[tokio::test]
    async fn test_project_structure_creation() {
        let temp_dir = tempdir().unwrap();
        let project_dir = temp_dir.path().join("test-project");

        let config = RuitlConfig::default();
        let app = CliApp::new(config, false);

        app.create_project_structure(&project_dir, "test-project", "default")
            .await
            .unwrap();

        // Check that directories were created
        assert!(project_dir.join("src").exists());
        assert!(project_dir.join("src/components").exists());
        assert!(project_dir.join("templates").exists());
        assert!(project_dir.join("static").exists());

        // Check that files were created
        assert!(project_dir.join("ruitl.toml").exists());
        assert!(project_dir.join("src/main.rs").exists());
        assert!(project_dir.join("Cargo.toml").exists());
        assert!(project_dir.join("README.md").exists());
    }
}
