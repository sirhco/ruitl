//! # RUITL - Rust UI Template Language
//!
//! A modern template engine for building HTML components and applications in Rust.
//!
//! ## Features
//!
//! - **Component-based**: Create reusable HTML components
//! - **Server-side rendering**: Deploy as serverless functions or containers
//! - **Static generation**: Generate static HTML files
//! - **Compiled templates**: High-performance compiled Rust code
//! - **No JavaScript**: Pure Rust, no client-side JS required
//! - **Great DX**: IDE autocompletion and type safety
//!
//! ## Quick Start
//!
//! Write a `.ruitl` template in `templates/Hello.ruitl`:
//!
//! ```text
//! component Hello {
//!     props { name: String }
//! }
//!
//! ruitl Hello(name: String) {
//!     <div class="greeting">
//!         <h1>{format!("Hello, {}!", name)}</h1>
//!     </div>
//! }
//! ```
//!
//! `cargo build` invokes `build.rs`, which compiles it to a sibling
//! `templates/Hello_ruitl.rs` (checked in, templ-style). Use it from Rust:
//!
//! ```ignore
//! use ruitl::prelude::*;
//!
//! let component = Hello;
//! let props = HelloProps { name: "World".to_string() };
//! let ctx = ComponentContext::new();
//! let html = component.render(&props, &ctx).unwrap();
//! println!("{}", html.render());
//! ```

pub mod build;
pub mod cli;
pub mod component;
pub mod config;
pub mod error;
pub mod html;

/// Test-support helpers (`ComponentTestHarness`, `HtmlAssertion`,
/// `assert_html_contains!`, `assert_renders_to!`). Feature-gated so they
/// don't bloat release binaries — consumers enable with
/// `features = ["testing"]` in their `[dev-dependencies]`. Always available
/// inside this crate's own tests.
#[cfg(any(test, feature = "testing"))]
pub mod testing;

/// Parser AST and tokenizer — re-exported from the shared `ruitl_compiler` crate.
pub use ruitl_compiler::parser;
/// Template → Rust code generator — re-exported from the shared `ruitl_compiler` crate.
pub use ruitl_compiler::codegen;

// Re-export commonly used items
pub use component::{Component, ComponentContext, ComponentProps, EmptyProps};
pub use error::{Result, RuitlError};
pub use html::{Html, HtmlAttribute, HtmlElement};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::component::{Component, ComponentContext, ComponentProps, EmptyProps};
    pub use crate::error::{Result, RuitlError};
    pub use crate::html::{Html, HtmlAttribute, HtmlElement};

    // Common std imports for templates
    pub use std::collections::HashMap;
    pub use std::fmt::Write;
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration
pub mod defaults {
    /// Default port for development server
    pub const DEV_PORT: u16 = 3000;

    /// Default build directory
    pub const BUILD_DIR: &str = "dist";

    /// Default source directory
    pub const SRC_DIR: &str = "src";

    /// Default templates directory
    pub const TEMPLATES_DIR: &str = "templates";

    /// Default static assets directory
    pub const STATIC_DIR: &str = "static";

    /// Default configuration file name
    pub const CONFIG_FILE: &str = "ruitl.toml";
}

/// Initialize RUITL with default configuration
pub fn init() -> Result<()> {
    // Initialize logging, configuration, etc.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }
}
