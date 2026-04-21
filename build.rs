//! Build script for the RUITL runtime crate.
//!
//! Compiles every `.ruitl` template found under `templates/` or `src/templates/`
//! into a sibling `*_ruitl.rs` file using the shared `ruitl_compiler` crate.
//! The CLI (`ruitl compile`) uses the same crate — there is no second parser.

use std::env;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_root = PathBuf::from(&manifest_dir);

    let candidates = [
        manifest_root.join("templates"),
        manifest_root.join("src").join("templates"),
        // `examples/demo_templates/` hosts compilable templates used by
        // `examples/server_integration.rs` to demonstrate real sibling-file
        // integration. `examples/templates/` holds the syntax showcases and
        // is intentionally NOT compiled — some showcases reference external
        // types (e.g. a `User` struct) that belong to a notional user project.
        manifest_root.join("examples").join("demo_templates"),
    ];

    let mut compiled: Vec<PathBuf> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for dir in &candidates {
        if !dir.exists() {
            continue;
        }

        // Rerun on any change under the templates dir.
        println!("cargo:rerun-if-changed={}", dir.display());
        emit_rerun_for_ruitl_files(dir);

        match ruitl_compiler::compile_dir_sibling(dir) {
            Ok(paths) => compiled.extend(paths),
            Err(e) => errors.push(format!("{}: {}", dir.display(), e)),
        }
    }

    if !errors.is_empty() {
        eprintln!("RUITL template compilation failed:");
        for err in &errors {
            eprintln!("  {}", err);
        }
        process::exit(1);
    }

    if !compiled.is_empty() {
        println!(
            "cargo:warning=Compiled {} RUITL templates (sibling *_ruitl.rs files)",
            compiled.len()
        );
    }
}

fn emit_rerun_for_ruitl_files(dir: &Path) {
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "ruitl").unwrap_or(false) {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
