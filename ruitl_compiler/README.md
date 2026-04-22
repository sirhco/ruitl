# ruitl_compiler

Parser and code generator for the RUITL template language. Runtime-free:
only `proc-macro2` / `quote` / `syn` plus a small set of build-time
utilities. Consumed by the main [`ruitl`](https://crates.io/crates/ruitl)
crate at build time and by its `build.rs` script; usable standalone if you
want to embed RUITL template compilation in your own tooling.

## What this crate does

Given a `.ruitl` template source:

```ruitl
component Card {
    props {
        title: String,
    }
}

ruitl Card(title: String) {
    <div class="card">
        <h2>{title}</h2>
        <div class="body">{children}</div>
    </div>
}
```

`ruitl_compiler` produces a sibling `Card_ruitl.rs` with a `CardProps`
struct, a `Card` unit struct, and a `Component` impl whose `render()`
returns the RUITL runtime's `Html` tree.

## Public API

```rust
use ruitl_compiler::{parse_str, generate, compile_file_sibling, compile_dir_sibling};

// Parse source → AST
let ast = parse_str(source)?;

// AST → formatted Rust source string
let code = generate(ast)?;

// Compile one file next to its source
let out_path = compile_file_sibling(std::path::Path::new("templates/Card.ruitl"))?;

// Compile an entire directory in parallel (with the `parallel` feature)
let outputs = compile_dir_sibling(std::path::Path::new("templates"))?;
```

## Features

- `parallel` (default) — fan out `compile_dir_sibling` across threads with
  `rayon`. Disable with `--no-default-features` for strictly sequential
  compile.

## Output contract

Each `Foo.ruitl` produces exactly one sibling `Foo_ruitl.rs` next to the
source. Files carry a `// ruitl-hash: …` header on line 1 that the
incremental build compares against `CODEGEN_VERSION` + source digest —
unchanged inputs are skipped without rewriting, so `git diff` stays clean
on no-op rebuilds.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../LICENSE-MIT))

at your option.
