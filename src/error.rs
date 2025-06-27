//! Error handling for RUITL

use std::fmt;
use thiserror::Error;

/// Main error type for RUITL operations
#[derive(Error, Debug)]
pub enum RuitlError {
    /// Template parsing errors
    #[error("Template error: {message}")]
    Template { message: String },

    /// Component compilation errors
    #[error("Component error: {message}")]
    Component { message: String },

    /// HTML rendering errors
    #[error("Render error: {message}")]
    Render { message: String },

    /// File I/O errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Build system errors
    #[error("Build error: {message}")]
    Build { message: String },

    /// Server errors
    #[error("Server error: {message}")]
    Server { message: String },

    /// Route resolution errors
    #[error("Route error: {message}")]
    Route { message: String },

    /// Static generation errors
    #[error("Static generation error: {message}")]
    StaticGen { message: String },

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// TOML parsing errors
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Template parsing errors
    #[error("Parse error: {message}")]
    Parse { message: String },

    /// Code generation errors
    #[error("Code generation error: {message}")]
    Codegen { message: String },

    /// HTTP errors
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),

    /// File walking errors
    #[error("File system error: {0}")]
    WalkDir(#[from] walkdir::Error),

    /// Address parsing errors
    #[error("Address parsing error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    /// HTTP header/request building errors
    #[error("HTTP error: {0}")]
    HttpError(#[from] hyper::http::Error),

    /// Generic errors
    #[error("Error: {message}")]
    Generic { message: String },
}

impl RuitlError {
    /// Create a new template error
    pub fn template<S: Into<String>>(message: S) -> Self {
        Self::Template {
            message: message.into(),
        }
    }

    /// Create a new component error
    pub fn component<S: Into<String>>(message: S) -> Self {
        Self::Component {
            message: message.into(),
        }
    }

    /// Create a new render error
    pub fn render<S: Into<String>>(message: S) -> Self {
        Self::Render {
            message: message.into(),
        }
    }

    /// Create a new config error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a new build error
    pub fn build<S: Into<String>>(message: S) -> Self {
        Self::Build {
            message: message.into(),
        }
    }

    /// Create a new server error
    pub fn server<S: Into<String>>(message: S) -> Self {
        Self::Server {
            message: message.into(),
        }
    }

    /// Create a new route error
    pub fn route<S: Into<String>>(message: S) -> Self {
        Self::Route {
            message: message.into(),
        }
    }

    /// Create a new parse error
    pub fn parse<S: Into<String>>(message: S) -> Self {
        Self::Parse {
            message: message.into(),
        }
    }

    /// Create a new code generation error
    pub fn codegen<S: Into<String>>(message: S) -> Self {
        Self::Codegen {
            message: message.into(),
        }
    }

    /// Create a new static generation error
    pub fn static_gen<S: Into<String>>(message: S) -> Self {
        Self::StaticGen {
            message: message.into(),
        }
    }

    /// Create a new generic error
    pub fn generic<S: Into<String>>(message: S) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }

    /// Get the error message
    pub fn message(&self) -> String {
        self.to_string()
    }

    /// Check if this is a template error
    pub fn is_template(&self) -> bool {
        matches!(self, Self::Template { .. })
    }

    /// Check if this is a component error
    pub fn is_component(&self) -> bool {
        matches!(self, Self::Component { .. })
    }

    /// Check if this is a render error
    pub fn is_render(&self) -> bool {
        matches!(self, Self::Render { .. })
    }

    /// Check if this is an IO error
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io(_))
    }

    /// Check if this is a config error
    pub fn is_config(&self) -> bool {
        matches!(self, Self::Config { .. })
    }

    /// Check if this is a build error
    pub fn is_build(&self) -> bool {
        matches!(self, Self::Build { .. })
    }

    /// Check if this is a server error
    pub fn is_server(&self) -> bool {
        matches!(self, Self::Server { .. })
    }
}

/// Result type alias for RUITL operations
pub type Result<T> = std::result::Result<T, RuitlError>;

/// Extension trait for Results to add context
pub trait ResultExt<T> {
    /// Add template context to the error
    fn template_context<S: Into<String>>(self, context: S) -> Result<T>;

    /// Add component context to the error
    fn component_context<S: Into<String>>(self, context: S) -> Result<T>;

    /// Add render context to the error
    fn render_context<S: Into<String>>(self, context: S) -> Result<T>;

    /// Add config context to the error
    fn config_context<S: Into<String>>(self, context: S) -> Result<T>;

    /// Add build context to the error
    fn build_context<S: Into<String>>(self, context: S) -> Result<T>;

    /// Add server context to the error
    fn server_context<S: Into<String>>(self, context: S) -> Result<T>;

    /// Add static generation context to the error
    fn static_gen_context<S: Into<String>>(self, context: S) -> Result<T>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: Into<RuitlError>,
{
    fn template_context<S: Into<String>>(self, context: S) -> Result<T> {
        self.map_err(|e| {
            let original = e.into();
            RuitlError::template(format!("{}: {}", context.into(), original.message()))
        })
    }

    fn component_context<S: Into<String>>(self, context: S) -> Result<T> {
        self.map_err(|e| {
            let original = e.into();
            RuitlError::component(format!("{}: {}", context.into(), original.message()))
        })
    }

    fn render_context<S: Into<String>>(self, context: S) -> Result<T> {
        self.map_err(|e| {
            let original = e.into();
            RuitlError::render(format!("{}: {}", context.into(), original.message()))
        })
    }

    fn config_context<S: Into<String>>(self, context: S) -> Result<T> {
        self.map_err(|e| {
            let original = e.into();
            RuitlError::config(format!("{}: {}", context.into(), original.message()))
        })
    }

    fn build_context<S: Into<String>>(self, context: S) -> Result<T> {
        self.map_err(|e| {
            let original = e.into();
            RuitlError::build(format!("{}: {}", context.into(), original.message()))
        })
    }

    fn server_context<S: Into<String>>(self, context: S) -> Result<T> {
        self.map_err(|e| {
            let original = e.into();
            RuitlError::server(format!("{}: {}", context.into(), original.message()))
        })
    }

    fn static_gen_context<S: Into<String>>(self, context: S) -> Result<T> {
        self.map_err(|e| {
            let original = e.into();
            RuitlError::static_gen(format!("{}: {}", context.into(), original.message()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = RuitlError::template("test template error");
        assert!(err.is_template());
        assert!(!err.is_component());
        assert!(err.message().contains("test template error"));
    }

    #[test]
    fn test_result_ext() {
        let result: std::result::Result<(), std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));

        let with_context = result.template_context("Failed to load template");
        assert!(with_context.is_err());

        let err = with_context.unwrap_err();
        assert!(err.is_template());
        assert!(err.message().contains("Failed to load template"));
    }

    #[test]
    fn test_error_types() {
        assert!(RuitlError::component("test").is_component());
        assert!(RuitlError::render("test").is_render());
        assert!(RuitlError::config("test").is_config());
        assert!(RuitlError::build("test").is_build());
        assert!(RuitlError::server("test").is_server());
        assert!(RuitlError::generic("test").message().contains("test"));
    }
}
