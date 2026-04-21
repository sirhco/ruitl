//! # ruitl_compiler
//!
//! Parser and code generator for the RUITL template language.
//!
//! This crate is runtime-free: it contains only `syn`/`quote`/`proc-macro2`-based
//! AST and codegen logic so it can be depended on from both the `ruitl` runtime
//! crate and its `build.rs` without pulling in server-side deps like `hyper`/`tokio`.

pub mod codegen;
pub mod error;
pub mod parser;

use std::fs;
use std::path::{Path, PathBuf};

/// Bumped whenever codegen output changes shape. Used as a cache-buster in
/// the sibling-file hash header so `cargo build` invalidates cached output
/// after any codegen.rs change, even if the `.ruitl` source is unchanged.
pub const CODEGEN_VERSION: u32 = 2;

/// Marker on the first line of every generated sibling file. The build
/// pipeline reads the hash off this line before deciding whether to skip
/// regeneration.
const HASH_HEADER_PREFIX: &str = "// ruitl-hash: ";

pub use codegen::CodeGenerator;
pub use error::{CompileError, Result};
pub use parser::{
    Attribute, AttributeValue, ComponentDef, ImportDef, MatchArm, ParamDef, PropDef, PropValue,
    RuitlFile, RuitlParser, TemplateAst, TemplateDef,
};

/// Parse a `.ruitl` source string into a [`RuitlFile`] AST.
pub fn parse_str(source: &str) -> Result<RuitlFile> {
    RuitlParser::new(source.to_string()).parse()
}

/// Generate Rust code (as a formatted string) from a [`RuitlFile`].
pub fn generate(file: RuitlFile) -> Result<String> {
    let mut gen = CodeGenerator::new(file);
    let tokens = gen.generate()?;
    Ok(format_rust(tokens.to_string()))
}

/// Compile a single `.ruitl` file to a sibling `*_ruitl.rs` file.
///
/// The output path is `<parent>/<stem>_ruitl.rs` next to the source.
/// Returns the path that was written.
pub fn compile_file_sibling(source: &Path) -> Result<PathBuf> {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CompileError::parse(format!("invalid file name: {}", source.display())))?;
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    let out = parent.join(format!("{}_ruitl.rs", sanitize_stem(stem)));
    compile_file(source, &out)?;
    Ok(out)
}

/// Compile a single `.ruitl` file to the given output path.
///
/// If the output file already exists and carries a `// ruitl-hash: …` header
/// whose digest matches the current source + `CODEGEN_VERSION`, the file is
/// left untouched. This avoids touching `mtime` on every build and keeps
/// `git diff` clean after no-op rebuilds.
pub fn compile_file(source: &Path, output: &Path) -> Result<()> {
    let src = fs::read_to_string(source)?;
    let hash = compute_hash(&src);

    if output.exists() {
        if let Ok(existing) = fs::read_to_string(output) {
            if let Some(existing_hash) = extract_hash(&existing) {
                if existing_hash == hash {
                    return Ok(());
                }
            }
        }
    }

    let ast = parse_str(&src)?;
    let code = generate(ast)?;
    let final_text = format!("{}{}\n{}", HASH_HEADER_PREFIX, hash, code);

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(output, final_text)?;
    Ok(())
}

/// MD5 of the source + codegen version, hex-encoded. Not cryptographic —
/// just a cheap content fingerprint to detect unchanged inputs.
fn compute_hash(source: &str) -> String {
    let digest = md5::compute(format!("{}|v{}", source, CODEGEN_VERSION));
    format!("{:x}", digest)
}

/// Pull the digest out of a sibling file's first line, if present.
fn extract_hash(content: &str) -> Option<&str> {
    let first_line = content.lines().next()?;
    first_line.strip_prefix(HASH_HEADER_PREFIX).map(str::trim)
}

/// Walk a directory for `.ruitl` files and compile each into a sibling
/// `*_ruitl.rs` file. Also writes a top-level `mod.rs` in `dir` that declares
/// and re-exports each compiled module, so consumers can `mod templates;`.
/// Returns the list of written output paths.
pub fn compile_dir_sibling(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut outputs = Vec::new();
    if !dir.exists() {
        return Ok(outputs);
    }
    let mut module_stems: Vec<String> = Vec::new();
    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "ruitl").unwrap_or(false) {
            let out = compile_file_sibling(path)?;
            if let Some(stem) = out.file_stem().and_then(|s| s.to_str()) {
                module_stems.push(stem.to_string());
            }
            outputs.push(out);
        }
    }
    if !module_stems.is_empty() {
        write_sibling_mod_file(dir, &module_stems)?;
    }
    Ok(outputs)
}

fn write_sibling_mod_file(dir: &Path, stems: &[String]) -> Result<()> {
    let mut sorted = stems.to_vec();
    sorted.sort();
    let mut content = String::from(
        "// @generated by ruitl_compiler — do not edit. Regenerated on each compile.\n\n",
    );
    for stem in &sorted {
        content.push_str(&format!("#[allow(non_snake_case)] pub mod {};\n", stem));
    }
    content.push('\n');
    for stem in &sorted {
        content.push_str(&format!(
            "#[allow(unused_imports)] pub use {}::*;\n",
            stem
        ));
    }
    fs::write(dir.join("mod.rs"), content)?;
    Ok(())
}

/// Preserve the original file stem as-is. RUITL file names are PascalCase by
/// convention (e.g. `Button.ruitl`); the generated sibling keeps that casing
/// so `Button_ruitl.rs` matches Templ's `_templ.go` convention.
fn sanitize_stem(stem: &str) -> String {
    stem.to_string()
}

fn format_rust(raw: String) -> String {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let child = Command::new("rustfmt")
        .args(["--edition", "2021", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let Ok(mut child) = child else {
        return raw;
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(raw.as_bytes());
    }

    match child.wait_with_output() {
        Ok(out) if out.status.success() => {
            String::from_utf8(out.stdout).unwrap_or(raw)
        }
        _ => raw,
    }
}
