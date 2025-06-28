//! Command-line interface for RUITL
//!
//! This module provides the CLI commands for compiling RUITL templates.

use crate::config::RuitlConfig;
use crate::error::{Result, RuitlError};
use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};

/// RUITL - Rust UI Template Language
#[derive(Parser)]
#[clap(
    name = "ruitl",
    version = env!("CARGO_PKG_VERSION"),
    about = "A modern template compiler for building type-safe HTML components in Rust"
)]
pub struct Cli {
    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Environment
    #[arg(short, long, global = true, default_value = "development")]
    pub env: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compile .ruitl templates to Rust code
    Compile {
        /// Source directory containing .ruitl files
        #[arg(short, long, default_value = "templates")]
        src_dir: PathBuf,
        /// Output directory for generated Rust files
        #[arg(short, long, default_value = "generated")]
        out_dir: PathBuf,
        /// Watch for changes and recompile
        #[arg(short, long)]
        watch: bool,
    },
    /// Generate a scaffold project structure with example components
    Scaffold {
        /// Project name
        #[arg(short, long, default_value = "my-ruitl-project")]
        name: String,
        /// Target directory for the new project
        #[arg(short, long, default_value = ".")]
        target: PathBuf,
        /// Include server implementation
        #[arg(long)]
        with_server: bool,
        /// Include example components
        #[arg(long, default_value = "true")]
        with_examples: bool,
    },
    /// Show version information
    Version,
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
            Commands::Compile {
                src_dir,
                out_dir,
                watch,
            } => self.compile_templates(&src_dir, &out_dir, watch).await,
            Commands::Scaffold {
                name,
                target,
                with_server,
                with_examples,
            } => {
                self.scaffold_project(&name, &target, with_server, with_examples)
                    .await
            }
            Commands::Version => {
                println!("RUITL {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
        }
    }

    /// Compile .ruitl templates to Rust code
    async fn compile_templates(&self, src_dir: &Path, out_dir: &Path, watch: bool) -> Result<()> {
        use crate::codegen::CodeGenerator;
        use crate::parser::RuitlParser;
        use walkdir::WalkDir;

        // Validate input directory
        if !src_dir.exists() {
            return Err(RuitlError::config(format!(
                "Source directory '{}' does not exist",
                src_dir.display()
            )));
        }

        self.log_info("Compiling RUITL templates...");

        let compile_fn = || async {
            let mut templates_compiled = 0;
            let mut component_names = Vec::new();
            let mut errors = Vec::new();

            // Create output directory if it doesn't exist
            if !out_dir.exists() {
                fs::create_dir_all(out_dir).map_err(|e| {
                    RuitlError::config(format!(
                        "Failed to create output directory '{}': {}",
                        out_dir.display(),
                        e
                    ))
                })?;
            }

            // Find all .ruitl files
            for entry in WalkDir::new(src_dir) {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        errors.push(format!("Failed to read directory entry: {}", e));
                        continue;
                    }
                };
                let path = entry.path();

                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("ruitl") {
                    match self.compile_single_template(path, src_dir, out_dir) {
                        Ok(component_name) => {
                            templates_compiled += 1;
                            component_names.push(component_name);

                            if self.verbose {
                                let relative_path = path.strip_prefix(src_dir).unwrap_or(path);
                                let mut rust_file = out_dir.join(relative_path);
                                rust_file.set_extension("rs");

                                self.log_info(&format!(
                                    "Compiled {} -> {}",
                                    path.display().to_string().bright_blue(),
                                    rust_file.display().to_string().green()
                                ));
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Failed to compile {}: {}", path.display(), e));
                        }
                    }
                }
            }

            // Generate mod.rs file
            if templates_compiled > 0 {
                self.generate_mod_file(out_dir, &component_names)?;
            }

            // Report results
            if !errors.is_empty() {
                self.log_error("Compilation completed with errors:");
                for error in &errors {
                    self.log_error(&format!("  • {}", error));
                }

                if templates_compiled == 0 {
                    return Err(RuitlError::generic("No templates compiled successfully"));
                }
            }

            self.log_success(&format!("✓ Compiled {} templates", templates_compiled));
            if !errors.is_empty() {
                self.log_info(&format!("⚠ {} errors encountered", errors.len()));
            }

            Ok::<(), RuitlError>(())
        };

        if watch {
            self.log_info("Watching for changes...");
            // TODO: Implement file watching
            compile_fn().await?;
            self.log_info("Watch mode not yet implemented, compiled once");
        } else {
            compile_fn().await?;
        }

        Ok(())
    }

    /// Compile a single template file
    fn compile_single_template(
        &self,
        template_path: &Path,
        src_dir: &Path,
        out_dir: &Path,
    ) -> Result<String> {
        use crate::codegen::CodeGenerator;
        use crate::parser::RuitlParser;

        // Read template file
        let content = fs::read_to_string(template_path).map_err(|e| {
            RuitlError::config(format!(
                "Failed to read template file '{}': {}",
                template_path.display(),
                e
            ))
        })?;

        // Parse template
        let mut parser = RuitlParser::new(content);
        let ruitl_ast = parser.parse().map_err(|e| {
            RuitlError::template(format!(
                "Failed to parse {}: {}",
                template_path.display(),
                e
            ))
        })?;

        // Extract component name for mod.rs
        let component_name = if let Some(component) = ruitl_ast.components.first() {
            component.name.clone()
        } else {
            template_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        };

        // Generate Rust code
        let mut generator = CodeGenerator::new(ruitl_ast);
        let rust_code = generator.generate().map_err(|e| {
            RuitlError::codegen(format!(
                "Failed to generate code for {}: {}",
                template_path.display(),
                e
            ))
        })?;

        // Format the generated code
        let formatted_code = self.format_generated_code(&rust_code.to_string())?;

        // Write generated file
        let relative_path = template_path.strip_prefix(src_dir).unwrap_or(template_path);
        let mut rust_file = out_dir.join(relative_path);

        // Convert filename to lowercase to match module naming convention
        if let Some(filename) = rust_file.file_stem() {
            if let Some(filename_str) = filename.to_str() {
                let lowercase_filename = format!("{}.rs", filename_str.to_lowercase());
                rust_file.set_file_name(lowercase_filename);
            }
        } else {
            rust_file.set_extension("rs");
        }

        if let Some(parent) = rust_file.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                RuitlError::config(format!(
                    "Failed to create directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        fs::write(&rust_file, formatted_code).map_err(|e| {
            RuitlError::config(format!(
                "Failed to write generated file '{}': {}",
                rust_file.display(),
                e
            ))
        })?;

        Ok(component_name)
    }

    /// Format generated Rust code for better readability
    fn format_generated_code(&self, code: &str) -> Result<String> {
        // Basic formatting improvements
        let formatted = code
            .replace(" ; ", ";\n")
            .replace(" { ", " {\n    ")
            .replace(" } ", "\n}\n")
            .replace(" . ", ".\n    ");

        // Try to use rustfmt if available, otherwise return basic formatting
        match std::process::Command::new("rustfmt")
            .arg("--emit=stdout")
            .arg("--edition=2021")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.take() {
                    use std::io::Write;
                    let mut stdin = stdin;
                    let _ = stdin.write_all(formatted.as_bytes());
                    drop(stdin);
                }

                match child.wait_with_output() {
                    Ok(output) if output.status.success() => {
                        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
                    }
                    _ => {
                        if self.verbose {
                            self.log_info("rustfmt not available, using basic formatting");
                        }
                    }
                }
            }
            Err(_) => {
                if self.verbose {
                    self.log_info("rustfmt not found, using basic formatting");
                }
            }
        }

        Ok(formatted)
    }

    /// Generate a mod.rs file for the compiled templates
    fn generate_mod_file(&self, out_dir: &Path, component_names: &[String]) -> Result<()> {
        let mod_file = out_dir.join("mod.rs");

        let mut content = String::new();
        content.push_str("//! Generated RUITL components\n");
        content.push_str("//! This file is automatically generated by RUITL CLI\n");
        content.push_str("//! DO NOT EDIT MANUALLY\n\n");

        // Add module declarations
        for name in component_names {
            let module_name = name.to_lowercase();
            content.push_str(&format!("pub mod {};\n", module_name));
        }

        content.push('\n');

        // Add re-exports
        content.push_str("// Re-exports for convenience\n");
        for name in component_names {
            let module_name = name.to_lowercase();
            content.push_str(&format!(
                "pub use {}::{{{}, {}Props}};\n",
                module_name, name, name
            ));
        }

        fs::write(&mod_file, content).map_err(|e| {
            RuitlError::config(format!(
                "Failed to write mod.rs file '{}': {}",
                mod_file.display(),
                e
            ))
        })?;

        if self.verbose {
            self.log_info(&format!(
                "Generated module file: {}",
                mod_file.display().to_string().green()
            ));
        }

        Ok(())
    }

    /// Generate a scaffold project structure
    async fn scaffold_project(
        &self,
        name: &str,
        target: &Path,
        with_server: bool,
        with_examples: bool,
    ) -> Result<()> {
        self.log_info(&format!("Creating new RUITL project: {}", name));

        let project_dir = target.join(name);

        // Create project directory structure
        self.create_project_structure(&project_dir, with_server, with_examples)?;

        // Generate configuration files
        self.generate_config_files(&project_dir, name)?;

        // Generate example templates if requested
        if with_examples {
            self.generate_example_templates(&project_dir)?;
        }

        // Generate server implementation if requested
        if with_server {
            self.generate_server_implementation(&project_dir)?;
        }

        // Generate build files
        self.generate_build_files(&project_dir, name, with_server)?;

        // Generate placeholder generated files
        self.generate_placeholder_generated_files(&project_dir)?;

        // Generate RUITL binary wrapper
        self.generate_ruitl_binary_wrapper(&project_dir)?;

        // Compile example templates if they were generated
        if with_examples {
            self.compile_initial_templates(&project_dir).await?;
        }

        // Generate static assets
        self.generate_static_assets(&project_dir)?;

        self.log_success(&format!(
            "✓ Created RUITL project: {}",
            project_dir.display()
        ));
        self.print_next_steps(&project_dir, with_server);

        Ok(())
    }

    /// Create the basic project directory structure
    fn create_project_structure(
        &self,
        project_dir: &Path,
        with_server: bool,
        with_examples: bool,
    ) -> Result<()> {
        let dirs = vec![
            "src",
            "templates",
            "generated",
            "static",
            "static/css",
            "static/js",
        ];

        for dir in dirs {
            let path = project_dir.join(dir);
            fs::create_dir_all(&path).map_err(|e| {
                RuitlError::config(format!(
                    "Failed to create directory '{}': {}",
                    path.display(),
                    e
                ))
            })?;
        }

        if with_server {
            fs::create_dir_all(project_dir.join("src").join("handlers")).map_err(|e| {
                RuitlError::config(format!("Failed to create handlers directory: {}", e))
            })?;
        }

        // Create bin directory for RUITL binary
        fs::create_dir_all(project_dir.join("bin"))
            .map_err(|e| RuitlError::config(format!("Failed to create bin directory: {}", e)))?;

        if with_examples {
            fs::create_dir_all(project_dir.join("examples")).map_err(|e| {
                RuitlError::config(format!("Failed to create examples directory: {}", e))
            })?;
        }

        Ok(())
    }

    /// Generate configuration files
    fn generate_config_files(&self, project_dir: &Path, name: &str) -> Result<()> {
        // Generate ruitl.toml
        let ruitl_config = format!(
            r#"[project]
name = "{}"
version = "0.1.0"
description = "A RUITL project"
authors = ["Your Name <your.email@example.com>"]

[build]
template_dir = "templates"
out_dir = "generated"
src_dir = "src"
"#,
            name
        );

        fs::write(project_dir.join("ruitl.toml"), ruitl_config)
            .map_err(|e| RuitlError::config(format!("Failed to write ruitl.toml: {}", e)))?;

        // Generate .gitignore
        let gitignore = r#"# Rust
target/
Cargo.lock

# RUITL generated files
generated/

# IDE
.vscode/
.idea/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Logs
*.log
"#;

        fs::write(project_dir.join(".gitignore"), gitignore)
            .map_err(|e| RuitlError::config(format!("Failed to write .gitignore: {}", e)))?;

        // Generate README.md
        let readme = format!(
            r#"# {}

A RUITL (Rust UI Template Language) project for building type-safe HTML components with server-side rendering.

## 🚀 Features

- **Component-Based Rendering**: Server handlers use generated RUITL components (not static HTML!)
- **Type Safety**: Full Rust type checking for templates and props
- **Zero Runtime**: Templates compiled to efficient Rust code at build time
- **Hot Reload**: Watch mode for development workflow
- **Ready to Use**: Example templates and working server included

## Getting Started

### Prerequisites

- Rust 1.70 or later

### Quick Start

```bash
# 1. Compile templates (generates Rust components)
cargo run --bin ruitl -- compile

# 2. Start the server (uses generated components!)
cargo run

# 3. Visit http://localhost:3000
```

### Development Workflow

```bash
# Watch for template changes and auto-recompile
cargo run --bin ruitl -- compile --watch

# In another terminal, run the server
cargo run
```

## 🏗️ How It Works

1. **Templates** in `templates/` are written in RUITL syntax
2. **Compilation** generates Rust structs and render functions in `generated/`
3. **Server handlers** import and use these generated components
4. **Type-safe rendering** produces HTML at runtime

## Project Structure

```
{}
├── src/
│   ├── main.rs        # Server with component-based handlers
│   └── handlers/      # HTTP handlers using RUITL components
├── bin/ruitl.rs       # RUITL CLI binary wrapper
├── templates/         # RUITL template files (.ruitl)
│   ├── Button.ruitl   # Interactive button component
│   ├── Card.ruitl     # Content card component
│   ├── Layout.ruitl   # Basic HTML layout
│   └── Page.ruitl     # Complete page with navigation
├── generated/         # Generated Rust code (auto-generated)
├── static/css/        # CSS styles
├── ruitl.toml         # RUITL configuration
└── Cargo.toml         # Rust project configuration
```

## 🧩 Template Examples

### Button Component (`templates/Button.ruitl`)

```ruitl
component Button {{
    props {{
        text: String,
        variant: String = "primary",
        size: String = "medium",
        disabled: bool = false,
        onclick: String?,
    }}
}}

ruitl Button(props: ButtonProps) {{
    <button
        class={{format!("btn btn-{{}} btn-{{}}", props.variant, props.size)}}
        disabled?={{props.disabled}}
        onclick?={{props.onclick}}
        type="button"
    >
        {{props.text}}
    </button>
}}
```

### Usage in Handler

```rust
// In src/handlers/mod.rs - components are imported and used!
use crate::generated::{{Button, ButtonProps}};

let button = Button;
let props = ButtonProps {{
    text: "Click Me".to_string(),
    variant: "primary".to_string(),
    // ... other props
}};

let html = button.render(&props, &context)?;
```

## 🎯 What's Different?

Unlike typical web frameworks, this project demonstrates:

- **No Runtime Templates**: Templates are compiled away at build time
- **Component Imports**: Server code imports generated Rust structs
- **Type-Safe Props**: Component properties are validated at compile time
- **Direct Rendering**: Components render to HTML strings efficiently

## Learn More

- [RUITL Documentation](https://github.com/sirhco/ruitl)
- [Rust Documentation](https://doc.rust-lang.org/)
"#,
            name, name
        );

        fs::write(project_dir.join("README.md"), readme)
            .map_err(|e| RuitlError::config(format!("Failed to write README.md: {}", e)))?;

        Ok(())
    }

    /// Generate example templates
    fn generate_example_templates(&self, project_dir: &Path) -> Result<()> {
        // Generate Button.ruitl
        let button_template = r#"// RUITL Button Component
// Example demonstrating basic component structure with props and conditionals

component Button {
    props {
        text: String,
        variant: String = "primary",
        size: String = "medium",
        disabled: bool = false,
        onclick: String?,
    }
}

ruitl Button(props: ButtonProps) {
    <button
        class={format!("btn btn-{} btn-{}", props.variant, props.size)}
        disabled?={props.disabled}
        onclick?={props.onclick}
        type="button"
    >
        {props.text}
    </button>
}
"#;

        fs::write(project_dir.join("templates/Button.ruitl"), button_template)
            .map_err(|e| RuitlError::config(format!("Failed to write Button.ruitl: {}", e)))?;

        // Generate Card.ruitl
        let card_template = r#"// RUITL Card Component
// Example demonstrating conditional rendering and component composition

component Card {
    props {
        title: String,
        content: String,
        footer: String?,
        variant: String = "default",
    }
}

ruitl Card(props: CardProps) {
    <div class={format!("card card-{}", props.variant)}>
        <div class="card-header">
            <h3 class="card-title">{props.title}</h3>
        </div>

        <div class="card-body">
            <p class="card-content">{props.content}</p>
        </div>

        if let Some(footer) = &props.footer {
            <div class="card-footer">
                <p class="card-footer-text">{footer}</p>
            </div>
        }
    </div>
}
"#;

        fs::write(project_dir.join("templates/Card.ruitl"), card_template)
            .map_err(|e| RuitlError::config(format!("Failed to write Card.ruitl: {}", e)))?;

        // Generate Layout.ruitl
        let layout_template = r#"// RUITL Layout Component
// Example demonstrating flexible layout components

component Layout {
    props {
        title: String,
        children: String,
        head_content: String?,
    }
}

ruitl Layout(props: LayoutProps) {
    <html lang="en">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>{props.title}</title>
            if let Some(head_content) = &props.head_content {
                {head_content}
            }
        </head>
        {props.children}
    </html>
}
"#;

        fs::write(project_dir.join("templates/Layout.ruitl"), layout_template)
            .map_err(|e| RuitlError::config(format!("Failed to write Layout.ruitl: {}", e)))?;

        // Generate Page.ruitl
        let page_template = r#"// RUITL Page Component
// Example demonstrating complete page structure with navigation

component Page {
    props {
        title: String,
        content: String,
        current_page: String = "home",
    }
}

ruitl Page(props: PageProps) {
    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>{props.title}</title>
            <link rel="stylesheet" href="/static/css/styles.css" />
        </head>
        <body>
            <div class="container">
                {props.content}

                <nav class="nav">
                    if props.current_page == "home" {
                        <span>Home</span> | <a href="/about">About</a>
                    } else {
                        <a href="/">Home</a> | <span>About</span>
                    }
                </nav>
            </div>
        </body>
    </html>
}
"#;

        fs::write(project_dir.join("templates/Page.ruitl"), page_template)
            .map_err(|e| RuitlError::config(format!("Failed to write Page.ruitl: {}", e)))?;

        Ok(())
    }

    /// Generate server implementation
    fn generate_server_implementation(&self, project_dir: &Path) -> Result<()> {
        // Generate main.rs with server
        let main_rs = self.generate_main_rs_content();

        fs::write(project_dir.join("src/main.rs"), main_rs)
            .map_err(|e| RuitlError::config(format!("Failed to write main.rs: {}", e)))?;

        // Generate handlers/mod.rs
        let handlers_mod = self.generate_handlers_mod_content();

        fs::write(
            project_dir
                .join("src")
                .join("handlers")
                .join("mod")
                .with_extension("rs"),
            handlers_mod,
        )
        .map_err(|e| RuitlError::config(format!("Failed to write handlers/mod.rs: {}", e)))?;

        Ok(())
    }

    /// Generate build files
    fn generate_build_files(
        &self,
        project_dir: &Path,
        name: &str,
        with_server: bool,
    ) -> Result<()> {
        // Generate Cargo.toml
        let cargo_toml = if with_server {
            format!(
                r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
description = "A RUITL project with server support"

[[bin]]
name = "ruitl"
path = "bin/ruitl.rs"

[dependencies]
# RUITL dependency - Update this based on your setup:
# For published version: ruitl = "0.1.0"
# For git version: ruitl = {{ git = "https://github.com/sirhco/ruitl.git" }}
# For local development: ruitl = {{ path = "../path/to/ruitl" }}
ruitl = {{ git = "https://github.com/sirhco/ruitl.git" }}
tokio = {{ version = "1.0", features = ["full"] }}
hyper = {{ version = "0.14", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
anyhow = "1.0"

[dev-dependencies]
tempfile = "3.0"

# Custom scripts for development workflow
[package.metadata.scripts]
compile = "cargo run --bin ruitl -- compile"
watch = "cargo run --bin ruitl -- compile --watch"
dev = "cargo run --bin ruitl -- compile --watch & cargo run"
"#,
                name
            )
        } else {
            format!(
                r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
description = "A RUITL project"

[[bin]]
name = "ruitl"
path = "bin/ruitl.rs"

[dependencies]
# RUITL dependency - Update this based on your setup:
# For published version: ruitl = "0.1.0"
# For git version: ruitl = {{ git = "https://github.com/sirhco/ruitl.git" }}
# For local development: ruitl = {{ path = "../path/to/ruitl" }}
ruitl = {{ git = "https://github.com/sirhco/ruitl.git" }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
anyhow = "1.0"

[dev-dependencies]
tempfile = "3.0"

# Custom scripts for development workflow
[package.metadata.scripts]
compile = "cargo run --bin ruitl -- compile"
watch = "cargo run --bin ruitl -- compile --watch"
"#,
                name
            )
        };

        fs::write(project_dir.join("Cargo").with_extension("toml"), cargo_toml)
            .map_err(|e| RuitlError::config(format!("Failed to write Cargo.toml: {}", e)))?;

        // Generate lib.rs if no server, or basic lib.rs if server
        let lib_rs = if with_server {
            r#"//! RUITL project library

#[path = "../generated/mod.rs"]
pub mod generated;
pub use generated::*;
"#
        } else {
            r#"//! RUITL project library

#[path = "../generated/mod.rs"]
pub mod generated;
pub use generated::*;

pub fn main() {
    println!("Welcome to your RUITL project!");
    println!("Compile your templates with: ruitl compile");
    println!("Then use the generated components in your Rust code.");
}
"#
        };

        let lib_path = if with_server {
            project_dir.join("src").join("lib").with_extension("rs")
        } else {
            project_dir.join("src").join("main").with_extension("rs")
        };

        fs::write(&lib_path, lib_rs).map_err(|e| {
            RuitlError::config(format!("Failed to write {}: {}", lib_path.display(), e))
        })?;

        Ok(())
    }

    /// Generate placeholder generated files so project compiles initially
    fn generate_placeholder_generated_files(&self, project_dir: &Path) -> Result<()> {
        // Generate placeholder mod.rs in generated directory
        let placeholder_mod = r#"//! Generated RUITL components
//! This file is automatically generated by RUITL CLI
//! Run `ruitl compile` to generate actual components

// Placeholder components - will be replaced when templates are compiled
"#;

        fs::write(project_dir.join("generated/mod.rs"), placeholder_mod)
            .map_err(|e| RuitlError::config(format!("Failed to write generated/mod.rs: {}", e)))?;

        Ok(())
    }

    /// Compile initial templates in a new project
    async fn compile_initial_templates(&self, project_dir: &Path) -> Result<()> {
        self.log_info("Compiling example templates...");

        let templates_dir = project_dir.join("templates");
        let output_dir = project_dir.join("generated");

        // Use the existing compile_templates method
        match self
            .compile_templates(&templates_dir, &output_dir, false)
            .await
        {
            Ok(_) => {
                self.log_success("✓ Example templates compiled successfully");
                Ok(())
            }
            Err(e) => {
                self.log_warning(&format!("Could not compile templates: {}", e));
                self.log_info("You can compile them later with: ruitl compile");
                Ok(()) // Don't fail the scaffold process
            }
        }
    }

    /// Generate RUITL binary wrapper
    fn generate_ruitl_binary_wrapper(&self, project_dir: &Path) -> Result<()> {
        let binary_wrapper = r#"//! RUITL CLI Binary Wrapper
//! This file provides a local RUITL CLI for template compilation

use ruitl::cli;

#[tokio::main]
async fn main() {
    if let Err(e) = cli::run_cli().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
"#;

        fs::write(project_dir.join("bin").join("ruitl.rs"), binary_wrapper)
            .map_err(|e| RuitlError::config(format!("Failed to write bin/ruitl.rs: {}", e)))?;

        Ok(())
    }

    /// Generate static assets (CSS and JS)
    fn generate_static_assets(&self, project_dir: &Path) -> Result<()> {
        // Generate CSS
        let css = self.generate_css_content();

        fs::write(
            project_dir
                .join("static")
                .join("css")
                .join("styles")
                .with_extension("css"),
            css,
        )
        .map_err(|e| RuitlError::config(format!("Failed to write styles.css: {}", e)))?;

        // Generate JavaScript
        let js = self.generate_js_content();

        fs::write(
            project_dir
                .join("static")
                .join("js")
                .join("main")
                .with_extension("js"),
            js,
        )
        .map_err(|e| RuitlError::config(format!("Failed to write main.js: {}", e)))?;

        Ok(())
    }

    /// Print next steps for the user
    fn print_next_steps(&self, project_dir: &Path, with_server: bool) {
        println!();
        println!("{}", "🎉 Project created successfully!".green().bold());
        println!();
        println!(
            "📁 Project location: {}",
            project_dir.display().to_string().cyan()
        );
        println!();
        println!("{}", "Next steps:".bold());
        println!("  1. {} into the project directory:", "cd".cyan());
        println!(
            "     {}",
            format!("cd {}", project_dir.display()).bright_black()
        );
        println!();
        println!("  2. {} RUITL templates:", "Compile".cyan());
        println!(
            "     {}",
            format!("cargo run --bin ruitl -- {}", "compile").bright_black()
        );
        println!();
        println!("  3. Build the project:");
        println!("     {}", format!("cargo {}", "build").bright_black());
        println!();
        if with_server {
            println!("  4. Run the server:");
            println!("     {}", format!("cargo {}", "run").bright_black());
            println!();
            println!(
                "  🌐 Your server will be available at: {}",
                "http://localhost:3000".bright_blue().underline()
            );
        } else {
            println!("  4. Run the application:");
            println!("     {}", format!("cargo {}", "run").bright_black());
        }
        println!();
        println!("{}", "Development workflow:".bold());
        println!(
            "  • {} templates in the {} directory",
            "Edit".cyan(),
            "templates/".bright_black()
        );
        println!(
            "  • {} to regenerate Rust code",
            format!("cargo run --bin ruitl -- {}", "compile").bright_black()
        );
        println!(
            "  • {} for automatic recompilation",
            format!("cargo run --bin ruitl -- {} --watch", "compile").bright_black()
        );
        println!();
        println!("{}", "Learn more:".bold());
        println!(
            "  • {}",
            "https://github.com/sirhco/ruitl".bright_blue().underline()
        );
        println!(
            "  • Check out the {} directory for usage examples",
            "examples/".bright_black()
        );
        println!();
    }

    /// Log an info message
    fn log_info(&self, message: &str) {
        if self.verbose {
            println!("{} {}", "info:".bright_blue().bold(), message);
        }
    }

    /// Log a success message
    fn log_success(&self, message: &str) {
        println!("{}", message.green());
    }

    /// Log an error message
    fn log_error(&self, message: &str) {
        eprintln!("{} {}", "error:".bright_red().bold(), message);
    }

    /// Log a warning message
    fn log_warning(&self, message: &str) {
        println!("{} {}", "warning:".bright_yellow().bold(), message);
    }

    /// Generate main.rs content for server
    fn generate_main_rs_content(&self) -> String {
        format!(
            r#"//! Main application entry point with HTTP server

use hyper::service::{{make_service_fn, service_fn}};
use hyper::{{Body, Method, Request, Response, Server, StatusCode}};
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio;

mod handlers;
#[path = "../generated/mod.rs"]
mod generated;

use handlers::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {{
    println!("🚀 Starting RUITL server...");

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_svc = make_service_fn(|_conn| async {{
        Ok::<_, Infallible>(service_fn(handle_request))
    }});

    let server = Server::bind(&addr).serve(make_svc);

    println!("🌐 Server running at http://{{}}", addr);
    println!("📄 Available routes:");
    println!("   • http://localhost:3000/        - Home page");
    println!("   • http://localhost:3000/about   - About page");
    println!("   • http://localhost:3000/static/ - Static assets");
    println!();
    println!("Press Ctrl+C to stop the server");

    if let Err(e) = server.await {{
        eprintln!("Server error: {{}}", e);
    }}

    Ok(())
}}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {{
    let response = match (req.method(), req.uri().path()) {{
        (&Method::GET, "/") => serve_home().await,
        (&Method::GET, "/about") => serve_about().await,
        (&Method::GET, path) if path.starts_with("/static/") => serve_static(path).await,
        _ => serve_404().await,
    }};

    Ok(response)
}}
"#
        )
    }

    /// Generate handlers/mod.rs content
    fn generate_handlers_mod_content(&self) -> String {
        format!(
            r##"//! HTTP request handlers

use hyper::{{Body, Response, StatusCode}};
use std::fs;
use ruitl::{{Component, ComponentContext}};

// Import generated components (available after running `ruitl compile`)
use crate::generated::{{Button, ButtonProps, Card, CardProps}};

pub async fn serve_home() -> Response<Body> {{
    let context = ComponentContext::new();

    // Create a simple card component to demonstrate
    let card = Card;
    let card_props = CardProps {{
        title: "🚀 Fast".to_string(),
        content: "Compile-time template processing for maximum performance".to_string(),
        footer: Some("Powered by RUITL components!".to_string()),
        variant: Some("default".to_string()),
    }};

    let card_html = match card.render(&card_props, &context) {{
        Ok(html) => html.render(),
        Err(e) => return error_response(&format!("Card render error: {{}}", e)),
    }};

    // Create a button component
    let button = Button;
    let button_props = ButtonProps {{
        text: "Go to About".to_string(),
        variant: Some("primary".to_string()),
        size: Some("medium".to_string()),
        disabled: Some(false),
        onclick: Some("window.location.href='/about'".to_string()),
    }};

    let button_html = match button.render(&button_props, &context) {{
        Ok(html) => html.render(),
        Err(e) => return error_response(&format!("Button render error: {{}}", e)),
    }};

    // Create simple HTML structure with rendered components
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Welcome to RUITL</title>
    <link rel="stylesheet" href="/static/css/styles.css">
</head>
<body>
    <div class="container">
        <h1>Welcome to Your RUITL Project!</h1>
        <div class="hero">
            <h2>🚀 Successfully Created RUITL Project</h2>
            <p>You've successfully created a new RUITL project with server support.</p>
            <p><strong>This page now uses actual RUITL components!</strong></p>
        </div>

        <div class="demo-section">
            <h3>Component Demo</h3>
            <p>Here's a Card component rendered by RUITL:</p>
            {{}}

            <p>And here's a Button component:</p>
            {{}}
        </div>

        <div class="next-steps">
            <h3>Next Steps</h3>
            <ol>
                <li>Edit templates in the <code>templates/</code> directory</li>
                <li>Run <code>ruitl compile</code> to generate Rust components</li>
                <li>✅ Components are now being used in these handlers!</li>
                <li>Build and run with <code>cargo run</code></li>
            </ol>
        </div>

        <nav class="nav">
            <span>Home</span> | <a href="/about">About</a>
        </nav>
    </div>
</body>
</html>"#,
        card_html, button_html
    );

    Response::builder()
        .header("content-type", "text/html")
        .body(Body::from(html))
        .unwrap()
}}

pub async fn serve_about() -> Response<Body> {{
    let context = ComponentContext::new();

    // Create about info card
    let card = Card;
    let card_props = CardProps {{
        title: "About This Project".to_string(),
        content: "This is a RUITL project scaffold that demonstrates component-based architecture, type-safe templates, and server-side rendering.".to_string(),
        footer: Some("All content rendered by RUITL components!".to_string()),
        variant: Some("default".to_string()),
    }};

    let card_html = match card.render(&card_props, &context) {{
        Ok(html) => html.render(),
        Err(e) => return error_response(&format!("Card render error: {{}}", e)),
    }};

    // Create home button
    let button = Button;
    let button_props = ButtonProps {{
        text: "Go Home".to_string(),
        variant: Some("primary".to_string()),
        size: Some("medium".to_string()),
        disabled: Some(false),
        onclick: Some("window.location.href='/'".to_string()),
    }};

    let button_html = match button.render(&button_props, &context) {{
        Ok(html) => html.render(),
        Err(e) => return error_response(&format!("Button render error: {{}}", e)),
    }};

    // Create simple HTML structure
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>About - RUITL Project</title>
    <link rel="stylesheet" href="/static/css/styles.css">
</head>
<body>
    <div class="container">
        <h1>About This RUITL Project</h1>

        <div class="about-content">
            {{}}

            <h3>Features Demonstrated</h3>
            <ul>
                <li>✅ Component-based architecture</li>
                <li>✅ Type-safe templates</li>
                <li>✅ Server-side rendering</li>
                <li>✅ Generated component usage</li>
                <li>✅ Static asset serving</li>
            </ul>

            <h3>Template Examples</h3>
            <p>Check out the example templates created in your <code>templates/</code> directory:</p>
            <ul>
                <li><code>Button.ruitl</code> - Interactive button component</li>
                <li><code>Card.ruitl</code> - Content card component</li>
                <li><code>Layout.ruitl</code> - HTML layout component</li>
                <li><code>Page.ruitl</code> - Complete page component</li>
            </ul>

            <div style="margin: 20px 0;">
                {{}}
            </div>
        </div>

        <nav class="nav">
            <a href="/">Home</a> | <span>About</span>
        </nav>
    </div>
</body>
</html>"#,
        card_html, button_html
    );

    Response::builder()
        .header("content-type", "text/html")
        .body(Body::from(html))
        .unwrap()
}}

pub async fn serve_static(path: &str) -> Response<Body> {{
    let file_path = path.strip_prefix("/static/").unwrap_or(path);
    let full_path = format!("static/{{}}", file_path);

    match fs::read(&full_path) {{
        Ok(contents) => {{
            let content_type = match full_path.split('.').last() {{
                Some("css") => "text/css",
                Some("js") => "application/javascript",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("svg") => "image/svg+xml",
                _ => "application/octet-stream",
            }};

            Response::builder()
                .header("content-type", content_type)
                .body(Body::from(contents))
                .unwrap()
        }}
        Err(_) => serve_404().await,
    }}
}}

pub async fn serve_404() -> Response<Body> {{
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 - Page Not Found</title>
    <link rel="stylesheet" href="/static/css/styles.css">
</head>
<body>
    <div class="container">
        <h1>404 - Page Not Found</h1>

        <div class="error-page">
            <h2>Oops! Page Not Found</h2>
            <p>The page you're looking for doesn't exist.</p>
            <a href="/" class="btn btn-primary">Go Home</a>
        </div>

        <nav class="nav">
            <a href="/">Home</a> |
            <a href="/about">About</a>
        </nav>
    </div>
</body>
</html>"#;

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("content-type", "text/html")
        .body(Body::from(html))
        .unwrap()
}}

fn error_response(message: &str) -> Response<Body> {{
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("content-type", "text/plain")
        .body(Body::from(format!("Error: {{}}", message)))
        .unwrap()
}}
"##
        )
    }

    /// Generate CSS content
    fn generate_css_content(&self) -> String {
        r#"/* RUITL Project Styles */

* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, system-ui, sans-serif;
    line-height: 1.6;
    color: #333;
    background-color: #f8f9fa;
}

.header {
    background: #fff;
    border-bottom: 1px solid #e9ecef;
    padding: 1rem 0;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.nav {
    max-width: 1200px;
    margin: 0 auto;
    padding: 0 1rem;
}

.nav-title {
    color: #007bff;
    font-size: 1.5rem;
    font-weight: 600;
}

.main {
    max-width: 1200px;
    margin: 2rem auto;
    padding: 0 1rem;
    min-height: calc(100vh - 200px);
}

.footer {
    background: #fff;
    border-top: 1px solid #e9ecef;
    padding: 2rem 0;
    text-align: center;
    color: #6c757d;
    margin-top: 3rem;
}

/* Components */
.btn {
    display: inline-block;
    padding: 0.75rem 1.5rem;
    margin: 0.25rem;
    border: none;
    border-radius: 0.375rem;
    text-decoration: none;
    font-weight: 500;
    text-align: center;
    cursor: pointer;
    transition: all 0.2s ease;
}

.btn-primary {
    background-color: #007bff;
    color: white;
}

.btn-primary:hover {
    background-color: #0056b3;
}

.btn-secondary {
    background-color: #6c757d;
    color: white;
}

.btn-secondary:hover {
    background-color: #545b62;
}

.btn-small {
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
}

.btn-medium {
    padding: 0.75rem 1.5rem;
    font-size: 1rem;
}

.card {
    background: white;
    border-radius: 0.5rem;
    padding: 1.5rem;
    margin: 1rem 0;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    border: 1px solid #e9ecef;
}

.card-title {
    color: #333;
    margin-bottom: 1rem;
    font-size: 1.25rem;
    font-weight: 600;
}

.card-content {
    color: #555;
    margin-bottom: 1rem;
}

.card-footer {
    border-top: 1px solid #e9ecef;
    padding-top: 1rem;
    margin-top: 1rem;
}

.card-footer-text {
    color: #6c757d;
    font-size: 0.875rem;
}

/* Layout */
.hero {
    text-align: center;
    padding: 3rem 0;
    background: white;
    border-radius: 0.5rem;
    margin-bottom: 2rem;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.hero h2 {
    color: #333;
    font-size: 2.5rem;
    margin-bottom: 1rem;
    font-weight: 700;
}

.hero p {
    color: #6c757d;
    font-size: 1.25rem;
    max-width: 600px;
    margin: 0 auto;
}

.features {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 2rem;
    margin: 2rem 0;
}

.feature-card {
    background: white;
    padding: 2rem;
    border-radius: 0.5rem;
    text-align: center;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    border: 1px solid #e9ecef;
}

.feature-card h3 {
    color: #333;
    font-size: 1.5rem;
    margin-bottom: 1rem;
}

.feature-card p {
    color: #6c757d;
    line-height: 1.6;
}

.about-content {
    background: white;
    padding: 2rem;
    border-radius: 0.5rem;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.about-content h2 {
    color: #333;
    margin-bottom: 1rem;
}

.about-content h3 {
    color: #333;
    margin: 2rem 0 1rem 0;
}

.about-content ul, .about-content ol {
    margin: 1rem 0;
    padding-left: 2rem;
}

.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem 1rem;
}

.nav {
    margin: 2rem 0;
    padding: 1rem;
    text-align: center;
    background: white;
    border-radius: 0.5rem;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.nav a {
    color: #007bff;
    text-decoration: none;
    font-weight: 500;
    margin: 0 0.5rem;
}

.nav a:hover {
    text-decoration: underline;
}

.next-steps {
    background: white;
    padding: 2rem;
    border-radius: 0.5rem;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    margin: 2rem 0;
}

.next-steps h3 {
    color: #333;
    margin-bottom: 1rem;
}

.next-steps ol {
    padding-left: 2rem;
}

.next-steps li {
    margin: 0.5rem 0;
    color: #555;
}

.error-page {
    text-align: center;
    background: white;
    padding: 3rem;
    border-radius: 0.5rem;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    margin: 2rem 0;
}

.error-page h2 {
    color: #dc3545;
    margin-bottom: 1rem;
}

.error-page p {
    color: #6c757d;
    margin-bottom: 2rem;
}

.about-content li {
    margin: 0.5rem 0;
    color: #555;
}

.about-content code {
    background: #f8f9fa;
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-family: "Monaco", "Menlo", "Ubuntu Mono", monospace;
    font-size: 0.875rem;
    color: #e83e8c;
}

.error-page {
    text-align: center;
    padding: 3rem;
    background: white;
    border-radius: 0.5rem;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.error-page h2 {
    color: #dc3545;
    margin-bottom: 1rem;
}

.error-page p {
    color: #6c757d;
    margin-bottom: 2rem;
}

/* Responsive */
@media (max-width: 768px) {
    .main {
        margin: 1rem auto;
        padding: 0 0.5rem;
    }

    .hero h2 {
        font-size: 2rem;
    }

    .features {
        grid-template-columns: 1fr;
        gap: 1rem;
    }

    .feature-card {
        padding: 1.5rem;
    }
}
"#
        .to_string()
    }

    /// Generate JavaScript content
    fn generate_js_content(&self) -> String {
        r#"// RUITL Project JavaScript

document.addEventListener("DOMContentLoaded", function() {
    console.log("RUITL project loaded!");

    // Add any interactive functionality here
    initializeComponents();
});

function initializeComponents() {
    // Initialize button interactions
    const buttons = document.querySelectorAll(".btn");
    buttons.forEach(button => {
        button.addEventListener("click", function(e) {
            // Add click animation
            this.style.transform = "scale(0.98)";
            setTimeout(() => {
                this.style.transform = "scale(1)";
            }, 100);
        });
    });

    // Initialize cards
    const cards = document.querySelectorAll(".card");
    cards.forEach(card => {
        card.addEventListener("mouseenter", function() {
            this.style.transform = "translateY(-2px)";
            this.style.boxShadow = "0 4px 8px rgba(0,0,0,0.15)";
        });

        card.addEventListener("mouseleave", function() {
            this.style.transform = "translateY(0)";
            this.style.boxShadow = "0 2px 4px rgba(0,0,0,0.1)";
        });
    });
}

// Utility functions for RUITL components
window.RuitlUtils = {
    // Format dates
    formatDate: function(date) {
        return new Date(date).toLocaleDateString();
    },

    // Debounce function for input handling
    debounce: function(func, wait) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
        };
    },

    // Simple state management
    state: new Map(),

    setState: function(key, value) {
        this.state.set(key, value);
        this.notifyStateChange(key, value);
    },

    getState: function(key) {
        return this.state.get(key);
    },

    notifyStateChange: function(key, value) {
        // Dispatch custom event for state changes
        window.dispatchEvent(new CustomEvent("ruitl-state-change", {
            detail: { key, value }
        }));
    }
};
"#
        .to_string()
    }
}

/// Main CLI entry point
pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration
    let config = if let Some(config_path) = cli.config {
        RuitlConfig::from_file(&config_path)?
    } else {
        RuitlConfig::default()
    };

    let app = CliApp::new(config, cli.verbose);
    app.run(cli.command).await
}
