#![warn(clippy::pedantic)]

//! Project template system.
//!
//! This module provides the template infrastructure for generating new Inference
//! projects. Templates define the file structure and content for different
//! project types.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::project::templates::{DefaultTemplate, ProjectTemplate};
//!
//! let template = DefaultTemplate;
//! for file in template.files("my_project") {
//!     // Create each file in the project directory
//! }
//! ```
//!
//! ## Available Templates
//!
//! - [`DefaultTemplate`] - A minimal Inference project with standard structure

mod default;

pub use default::DefaultTemplate;

use std::path::PathBuf;

/// A file to be created as part of project scaffolding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateFile {
    /// Relative path from project root.
    pub path: PathBuf,
    /// File content.
    pub content: String,
}

impl TemplateFile {
    /// Creates a new template file.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
        }
    }
}

/// Trait for project templates.
///
/// Project templates define the structure and content of new Inference projects.
/// Implement this trait to create custom project templates.
pub trait ProjectTemplate {
    /// Returns the template name.
    ///
    /// This name is used for template selection via the `--template` flag.
    fn name(&self) -> &'static str;

    /// Returns a brief description of the template.
    fn description(&self) -> &'static str;

    /// Generates the template files for a project with the given name.
    ///
    /// # Arguments
    ///
    /// * `project_name` - The name of the project (used in manifest and content)
    ///
    /// # Returns
    ///
    /// A vector of files to create in the project directory.
    fn files(&self, project_name: &str) -> Vec<TemplateFile>;
}

/// Returns a template by name.
///
/// # Arguments
///
/// * `name` - The template name to look up
///
/// # Returns
///
/// The template if found, or `None` if no template matches the name.
#[must_use]
pub fn get_template(name: &str) -> Option<Box<dyn ProjectTemplate>> {
    match name {
        "default" => Some(Box::new(DefaultTemplate)),
        _ => None,
    }
}

/// Returns all available templates.
#[must_use]
pub fn available_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        (DefaultTemplate.name(), DefaultTemplate.description()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_file_new() {
        let file = TemplateFile::new("path/to/file.txt", "content here");
        assert_eq!(file.path, PathBuf::from("path/to/file.txt"));
        assert_eq!(file.content, "content here");
    }

    #[test]
    fn test_get_template_default() {
        let template = get_template("default");
        assert!(template.is_some());
        assert_eq!(template.unwrap().name(), "default");
    }

    #[test]
    fn test_get_template_unknown() {
        let template = get_template("nonexistent");
        assert!(template.is_none());
    }

    #[test]
    fn test_available_templates() {
        let templates = available_templates();
        assert!(!templates.is_empty());
        assert!(templates.iter().any(|(name, _)| *name == "default"));
    }
}
