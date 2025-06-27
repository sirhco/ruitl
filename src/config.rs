//! Configuration system for RUITL projects
//!
//! Simple configuration for RUITL template compilation.

use crate::error::{Result, RuitlError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure for RUITL projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuitlConfig {
    /// Project metadata
    pub project: ProjectConfig,
    /// Build configuration
    pub build: BuildConfig,
}

/// Project metadata configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
    /// Project description
    pub description: Option<String>,
    /// Project authors
    pub authors: Vec<String>,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Source directory containing .ruitl files
    pub template_dir: PathBuf,
    /// Output directory for generated Rust files
    pub out_dir: PathBuf,
    /// Source directory for the project
    pub src_dir: PathBuf,
}

impl Default for RuitlConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                name: "ruitl-project".to_string(),
                version: "0.1.0".to_string(),
                description: None,
                authors: vec![],
            },
            build: BuildConfig {
                template_dir: PathBuf::from("templates"),
                out_dir: PathBuf::from("generated"),
                src_dir: PathBuf::from("src"),
            },
        }
    }
}

impl RuitlConfig {
    /// Load configuration from a file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| RuitlError::config(format!("Failed to read config file: {}", e)))?;

        let config: RuitlConfig = toml::from_str(&content)
            .map_err(|e| RuitlError::config(format!("Failed to parse config file: {}", e)))?;

        Ok(config)
    }

    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| RuitlError::config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path.as_ref(), content)
            .map_err(|e| RuitlError::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Basic validation
        if self.project.name.is_empty() {
            return Err(RuitlError::config(
                "Project name cannot be empty".to_string(),
            ));
        }

        if self.project.version.is_empty() {
            return Err(RuitlError::config(
                "Project version cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = RuitlConfig::default();
        assert_eq!(config.project.name, "ruitl-project");
        assert_eq!(config.project.version, "0.1.0");
        assert_eq!(config.build.template_dir, PathBuf::from("templates"));
        assert_eq!(config.build.out_dir, PathBuf::from("generated"));
    }

    #[test]
    fn test_config_validation() {
        let config = RuitlConfig::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = config.clone();
        invalid_config.project.name = String::new();
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("ruitl.toml");

        let original_config = RuitlConfig::default();
        original_config.save_to_file(&config_path).unwrap();

        let loaded_config = RuitlConfig::from_file(&config_path).unwrap();
        assert_eq!(original_config.project.name, loaded_config.project.name);
        assert_eq!(
            original_config.project.version,
            loaded_config.project.version
        );
    }
}
