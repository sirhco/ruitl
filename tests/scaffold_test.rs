//! End-to-end scaffolder test: scaffold a project, point its `ruitl`
//! dependency at this working tree, and run `cargo check` against it.
//!
//! This is expensive (spawns a nested cargo invocation which has to resolve
//! the whole workspace), so it's marked `#[ignore]` by default. Opt in with:
//!
//!   RUITL_TEST_SCAFFOLD=1 cargo test --test scaffold_test -- --ignored

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scaffold(target: &std::path::Path, with_server: bool, with_examples: bool) {
    let repo = repo_root();
    let ruitl_bin = repo.join("target/debug/ruitl");
    assert!(
        ruitl_bin.exists(),
        "ruitl binary missing at {}. Run `cargo build` first.",
        ruitl_bin.display()
    );
    let mut cmd = Command::new(&ruitl_bin);
    cmd.arg("scaffold")
        .arg("--name")
        .arg("scaffold_probe")
        .arg("--target")
        .arg(target);
    if with_server {
        cmd.arg("--with-server");
    }
    if with_examples {
        cmd.arg("--with-examples");
    }
    let out = cmd.output().expect("spawn ruitl scaffold");
    assert!(
        out.status.success(),
        "scaffold failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn rewrite_ruitl_dep_to_path(cargo_toml: &std::path::Path) {
    let src = std::fs::read_to_string(cargo_toml).unwrap();
    let repo = repo_root();
    let patched = src.replace(
        "ruitl = { git = \"https://github.com/sirhco/ruitl.git\" }",
        &format!("ruitl = {{ path = \"{}\" }}", repo.display()),
    );
    std::fs::write(cargo_toml, patched).unwrap();
}

#[test]
#[ignore = "slow; opt in via RUITL_TEST_SCAFFOLD=1 cargo test -- --ignored"]
fn scaffolded_project_builds_warning_free() {
    if std::env::var("RUITL_TEST_SCAFFOLD").is_err() {
        return;
    }
    let dir = TempDir::new().unwrap();
    let project = dir.path().join("scaffold_probe");
    scaffold(dir.path(), false, true);
    rewrite_ruitl_dep_to_path(&project.join("Cargo.toml"));

    let out = Command::new("cargo")
        .arg("check")
        .arg("--message-format=short")
        .current_dir(&project)
        .output()
        .expect("run cargo check");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(out.status.success(), "cargo check failed:\n{}", stderr);

    // Count compiler warnings from the scaffolded crate itself. Build-script
    // `cargo:warning=` lines from the ruitl dep are informational and must be
    // excluded (they carry the "ruitl@" prefix).
    let warnings: Vec<&str> = stderr
        .lines()
        .filter(|l| l.starts_with("warning:") && !l.contains("ruitl@"))
        .collect();
    assert!(
        warnings.is_empty(),
        "scaffolded project has {} warnings:\n{}",
        warnings.len(),
        warnings.join("\n")
    );
}
