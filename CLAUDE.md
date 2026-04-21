# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

RUITL — Rust UI Template Language. Compiles `.ruitl` template files into type-safe Rust components at build time. Templ-inspired (`.templ` → `_templ.go` model) syntax, zero runtime overhead, server-side rendering focus. Cargo workspace with two members: root crate `ruitl` (library + `ruitl` binary) and `ruitl_compiler` (build-time parser + code generator, runtime-free).

## Common Commands

```bash
# Build (build.rs compiles any .ruitl in templates/ or src/templates/ to sibling *_ruitl.rs)
cargo build

# Compile .ruitl → sibling *_ruitl.rs via CLI (same engine as build.rs)
cargo run -- compile
cargo run -- compile --src-dir templates --watch --verbose

# Scaffold a new RUITL project
cargo run -- scaffold --name my-project --with-server --with-examples

# Run the showcase HTTP server example (port 3000)
cargo run --example server_integration

# Other examples
cargo run --example basic_usage
cargo run --example hello_world
cargo run --example html_output_demo
cargo run --example template_compiler_demo
cargo run --example advanced_features_demo

# Tests
cargo test                                  # all tests (workspace)
cargo test --test template_compilation      # integration file: tests/template_compilation.rs
cargo test --test cli_generated_code_test
cargo test --test component_composition     # validates generated code parses via syn
cargo test <test_name>                      # single test by name
cargo test -- --nocapture                   # show println! output

# Feature flags (default = ["server", "static", "dev"])
cargo build --no-default-features
cargo build --features minify
```

Release profile uses `lto = true`, `codegen-units = 1`, `panic = "abort"`.

## Architecture

Pipeline: `.ruitl` files → `ruitl_compiler::parser` → AST (`RuitlFile`) → `ruitl_compiler::codegen` (via `proc-macro2`/`quote`/`syn`) → formatted Rust → sibling `*_ruitl.rs` file (checked in, templ-style) → `rustc`.

Two entry points share the same compiler library and must stay in sync:

1. **`build.rs`** — invoked by Cargo. Scans both `src/templates/` and `templates/` relative to `CARGO_MANIFEST_DIR` and calls `ruitl_compiler::compile_dir_sibling(dir)` for each. Rerun triggers: `src/templates`, `templates`.
2. **`src/cli.rs`** — the `ruitl` binary (`src/main.rs` → `cli::run_cli`). `compile` subcommand walks `--src-dir` (default `templates`) and calls `ruitl_compiler::compile_file_sibling(path)` per `.ruitl` file. `scaffold` emits a complete project skeleton including its own vendored `bin/ruitl.rs` wrapper so scaffolded projects don't need a global install.

Both entry points produce the **same** sibling `*_ruitl.rs` output — there is no separate artifact directory. Generated files are committed to source control so diffs are reviewable, matching Go templ's `_templ.go` convention.

### Module map

**`ruitl_compiler/src/`** (build-time only, no runtime deps):
- `parser.rs` — hand-written parser. Produces `RuitlFile { components, templates, imports }`. `ComponentDef` holds props + generics; `TemplateDef` holds a `TemplateAst` (HTML elements, text, expressions, conditionals, loops, matches, component composition via `@Component`) + generics. `GenericParam { name, bounds }` represents a single type parameter.
- `codegen.rs` — `CodeGenerator` consumes `RuitlFile` and emits `TokenStream` using `quote!`. Generates `{Name}Props` struct + `impl ComponentProps` + unit struct `{Name}` + `impl Component` whose `render()` returns `Html`. Generic components parse but codegen currently returns an explicit error — full generics support is a follow-up (trait-bound ergonomics RFC pending).
- `lib.rs` — hub: `parse_str`, `generate`, `compile_file_sibling`, `compile_dir_sibling`, `format_rust`.
- `error.rs` — `CompileError` type used by parser + codegen.

**`src/`** (runtime library + CLI):
- `cli.rs` — `ruitl` binary. `compile` subcommand + `scaffold` project generator.
- `component.rs` — runtime traits: `Component`, `ComponentProps`, `ComponentContext`, `EmptyProps`. Generated code targets these.
- `html.rs` — `Html`, `HtmlElement`, `HtmlAttribute`. Output target of rendered components; `.render()` produces escaped HTML strings. Attributes stored as `Vec<(String, HtmlAttribute)>` to preserve insertion order for deterministic rendering.
- `config.rs` — `RuitlConfig` loaded from `ruitl.toml` (sections: `[project]`, `[build]`, `[server]`, `[dev]`).
- `error.rs` — `RuitlError` + `Result` alias used throughout runtime code.
- `generated.rs` — thin re-export module that pulls in `templates/mod.rs` (`#[path = "../templates/mod.rs"]`). Exposes committed sibling-generated components at the crate's root.
- `lib.rs` — public API. Re-exports `ruitl_compiler::{parser, codegen}` publicly so tests and downstream tooling can hit the compiler directly.

### Generated code contract

Each `.ruitl` file produces one sibling `.rs` file with the `_ruitl.rs` suffix (`Button.ruitl` → `Button_ruitl.rs` in the same directory). A `mod.rs` is auto-emitted listing the modules. Consumers import via `#[path = "../templates/mod.rs"] mod templates; use templates::*;` (scaffolded projects) or `mod generated; use generated::*;` (root crate via `src/generated.rs`). Changing this contract requires updating both `build.rs` and `ruitl_compiler::compile_dir_sibling()`.

Generated files use short type names relying on `use ruitl::prelude::*; use ruitl::html::*;` emitted at the top. Props structs derive `Debug + Clone` (no `serde`). Render methods name the context parameter `_context` when the template body doesn't invoke child components, to avoid unused-variable warnings.

### Template syntax (what the parser accepts)

- `component Name<T, U: Bound1 + Bound2> { props { field: Type = default, optional: Type?, ... } }` — generics parse but codegen currently errors
- `ruitl Name<T>(param: Type, ...) { <html>{expr}</html> }`
- Inline Rust exprs in `{}`; attribute interpolation `class={expr}`; boolean attrs `disabled?={expr}`
- Control flow: `if`/`else`, `for x in iter`, `match expr { arm => ... }`
- Component composition: `@ChildComponent(prop=value)` — threads `context` through
- `import` statements at top of file
- Whitespace between `{expr}` and adjacent text is preserved (significant for HTML spacing)

## Working on templates

When modifying `.ruitl` files in `templates/` or `examples/templates/`, the build script recompiles automatically on next `cargo build` and updates the sibling `*_ruitl.rs`. If iterating on parser/codegen, regenerate explicitly with `cargo run -- compile` and inspect diffs via `git diff templates/`. Committed `*_ruitl.rs` files serve as both the build product and a reference of current codegen behavior — review them like normal code.
