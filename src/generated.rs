//! Re-exports of compiled RUITL templates living next to their sources.
//!
//! Every `.ruitl` file in `templates/` is compiled into a sibling `*_ruitl.rs`
//! file, and `ruitl_compiler` writes a `templates/mod.rs` that aggregates them.
//! This module simply hooks that aggregate into the crate's module tree.

#[path = "../templates/mod.rs"]
mod templates_mod;

pub use templates_mod::*;
