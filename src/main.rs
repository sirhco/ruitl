//! RUITL CLI entry point
//!
//! This is the main entry point for the RUITL command-line interface.

use ruitl::cli;

#[tokio::main]
async fn main() {
    if let Err(e) = cli::run_cli().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
