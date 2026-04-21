//! Error types for the RUITL compiler.
//!
//! Kept deliberately narrow so this crate stays free of server/runtime deps.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Parse error: {message}")]
    Parse { message: String },

    #[error("Code generation error: {message}")]
    Codegen { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Walk error: {0}")]
    WalkDir(#[from] walkdir::Error),
}

impl CompileError {
    pub fn parse<S: Into<String>>(message: S) -> Self {
        Self::Parse {
            message: message.into(),
        }
    }

    pub fn codegen<S: Into<String>>(message: S) -> Self {
        Self::Codegen {
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, CompileError>;
