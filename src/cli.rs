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
        rust_file.set_extension("rs");

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
