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
//! ```rust
//! use ruitl::prelude::*;
//!
//! #[component]
//! fn hello_world(name: &str) -> Html {
//!     html! {
//!         <div class="greeting">
//!             <h1>Hello, {name}!</h1>
//!             <p>Welcome to RUITL</p>
//!         </div>
//!     }
//! }
//!
//! fn main() {
//!     let html = hello_world("World");
//!     println!("{}", html.render());
//! }
//! ```

pub mod build;
pub mod cli;
pub mod component;
pub mod config;
pub mod error;
pub mod html;
pub mod macros;
pub mod render;
pub mod router;
pub mod server;
pub mod static_gen;
pub mod template;

// Re-export commonly used items
pub use component::{Component, ComponentContext, ComponentProps, EmptyProps};
pub use error::{Result, RuitlError};
pub use html::{Html, HtmlAttribute, HtmlElement};
pub use render::{RenderContext, RenderOptions, RenderTarget, Renderer};
pub use router::{RouteContext, RouteResponse};
pub use template::{Template, TemplateEngine};

// Proc macro re-exports
pub use ruitl_macros::*;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::component::{Component, ComponentContext, ComponentProps, EmptyProps};
    pub use crate::error::{Result, RuitlError};
    pub use crate::html::{Html, HtmlAttribute, HtmlElement};
    pub use crate::render::{RenderContext, RenderOptions, RenderTarget, Renderer};
    pub use crate::router::{RouteContext, RouteResponse};
    pub use crate::template::{Template, TemplateEngine};
    pub use ruitl_macros::*;

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
