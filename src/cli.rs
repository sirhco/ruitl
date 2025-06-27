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

        self.log_info("Compiling RUITL templates...");

        let compile_fn = || async {
            let mut templates_compiled = 0;

            // Create output directory if it doesn't exist
            if !out_dir.exists() {
                fs::create_dir_all(out_dir)?;
            }

            // Find all .ruitl files
            for entry in WalkDir::new(src_dir) {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("ruitl") {
                    // Read template file
                    let content = fs::read_to_string(path)?;

                    // Parse template
                    let mut parser = RuitlParser::new(content);
                    let ruitl_ast = parser.parse().map_err(|e| {
                        RuitlError::generic(format!("Failed to parse {}: {}", path.display(), e))
                    })?;

                    // Generate Rust code
                    let mut generator = CodeGenerator::new(ruitl_ast);
                    let rust_code = generator.generate().map_err(|e| {
                        RuitlError::generic(format!(
                            "Failed to generate code for {}: {}",
                            path.display(),
                            e
                        ))
                    })?;

                    // Write generated file
                    let relative_path = path.strip_prefix(src_dir).unwrap_or(path);
                    let mut rust_file = out_dir.join(relative_path);
                    rust_file.set_extension("rs");

                    if let Some(parent) = rust_file.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    fs::write(&rust_file, rust_code.to_string())?;
                    templates_compiled += 1;

                    if self.verbose {
                        self.log_info(&format!(
                            "Compiled {} -> {}",
                            path.display().to_string().bright_blue(),
                            rust_file.display().to_string().green()
                        ));
                    }
                }
            }

            self.log_success(&format!("✓ Compiled {} templates", templates_compiled));
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
