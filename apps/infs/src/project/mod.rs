#![warn(clippy::pedantic)]

//! Project management module.
//!
//! This module provides functionality for creating and managing Inference
//! projects, including manifest handling, project scaffolding, and templates.
//!
//! ## Modules
//!
//! - [`manifest`] - Inference.toml parsing and validation
//! - [`templates`] - Project template system
//! - [`scaffold`] - Project creation and initialization
//!
//! ## Key Types
//!
//! - [`InferenceToml`] - The manifest file structure
//! - [`ProjectConfig`] - Loaded and validated project configuration
//! - [`ProjectTemplate`] - Trait for project templates

pub mod manifest;
pub mod scaffold;
pub mod templates;

pub use manifest::{BuildConfig, InferenceToml, VerificationConfig};
#[allow(unused_imports)]
pub use manifest::{Dependencies, Package};
#[allow(unused_imports)]
pub use manifest::validate_project_name;
pub use scaffold::{create_project, init_project};
#[allow(unused_imports)]
pub use scaffold::create_project_default;
pub use templates::{ProjectTemplate, available_templates, get_template};
#[allow(unused_imports)]
pub use templates::{DefaultTemplate, TemplateFile};

/// Represents a loaded and validated project configuration.
///
/// This struct is created from a parsed and validated `InferenceToml` manifest,
/// providing convenient access to project settings.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// The project name.
    pub name: String,

    /// The project version.
    pub version: String,

    /// The language edition.
    pub edition: String,

    /// Build configuration.
    pub build: BuildConfig,

    /// Verification configuration.
    pub verification: VerificationConfig,
}

impl ProjectConfig {
    /// Creates a `ProjectConfig` from a validated manifest.
    ///
    /// # Arguments
    ///
    /// * `manifest` - A validated `InferenceToml` instance
    ///
    /// # Panics
    ///
    /// This function does not panic but assumes the manifest has been validated.
    #[allow(dead_code)]
    #[must_use]
    pub fn from_manifest(manifest: &InferenceToml) -> Self {
        Self {
            name: manifest.package.name.clone(),
            version: manifest.package.version.clone(),
            edition: manifest.package.edition.clone(),
            build: manifest.build.clone(),
            verification: manifest.verification.clone(),
        }
    }

    /// Loads a project configuration from a manifest file.
    ///
    /// This function reads, parses, and validates the manifest in one step.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, parsed, or validation fails.
    #[allow(dead_code)]
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let manifest = InferenceToml::from_file(path)?;
        manifest.validate()?;
        Ok(Self::from_manifest(&manifest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_config_from_manifest() {
        let manifest = InferenceToml::new("test_project");
        let config = ProjectConfig::from_manifest(&manifest);

        assert_eq!(config.name, "test_project");
        assert_eq!(config.version, "0.1.0");
        assert_eq!(config.edition, "2024");
    }
}
