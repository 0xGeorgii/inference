#![warn(clippy::pedantic)]

//! Inference project manifest parsing and validation.
//!
//! This module handles the `Inference.toml` manifest file format, providing
//! parsing, validation, and serialization functionality.
//!
//! ## Manifest Format
//!
//! The Inference.toml file supports the following sections:
//!
//! ```toml
//! [package]
//! name = "myproject"
//! version = "0.1.0"
//! edition = "2024"
//! manifest_version = 1
//!
//! [dependencies]
//! # Future: package dependencies
//!
//! [build]
//! target = "wasm32"
//! optimize = "release"
//!
//! [verification]
//! output-dir = "proofs/"
//! ```
//!
//! ## Reserved Names
//!
//! Project names cannot use Inference keywords or problematic directory names.
//! See [`RESERVED_WORDS`] for the complete list.

use anyhow::{Context, Result, bail};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Reserved words that cannot be used as project names.
///
/// Includes Inference language keywords and problematic directory names.
pub const RESERVED_WORDS: &[&str] = &[
    // Inference keywords
    "fn",
    "let",
    "mut",
    "if",
    "else",
    "match",
    "return",
    "type",
    "struct",
    "impl",
    "trait",
    "pub",
    "use",
    "mod",
    "ndet",
    "assume",
    "assert",
    "forall",
    "exists",
    "spec",
    "requires",
    "ensures",
    "invariant",
    "const",
    "enum",
    "loop",
    "break",
    "continue",
    "external",
    "unique",
    // Problematic directory/file names
    "src",
    "out",
    "target",
    "proofs",
    "tests",
    "self",
    "super",
    "crate",
];

/// The root manifest structure for `Inference.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InferenceToml {
    /// Package metadata section.
    pub package: Package,

    /// Project dependencies.
    #[serde(default, skip_serializing_if = "Dependencies::is_empty")]
    pub dependencies: Dependencies,

    /// Build configuration.
    #[serde(default, skip_serializing_if = "BuildConfig::is_default")]
    pub build: BuildConfig,

    /// Verification configuration for Rocq output.
    #[serde(default, skip_serializing_if = "VerificationConfig::is_default")]
    pub verification: VerificationConfig,
}

/// Package metadata in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Package {
    /// The project name.
    pub name: String,

    /// The project version (semver format).
    pub version: String,

    /// The language edition.
    #[serde(default = "default_edition")]
    pub edition: String,

    /// Manifest schema version for future compatibility.
    #[serde(default = "default_manifest_version")]
    pub manifest_version: u32,

    /// Optional project description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional list of authors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<String>>,

    /// Optional license identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// Project dependencies section.
///
/// Currently a placeholder for future package management support.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependencies {
    /// Map of dependency name to version specification.
    #[serde(flatten)]
    pub packages: HashMap<String, String>,
}

impl Dependencies {
    /// Returns true if there are no dependencies.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// Build configuration section.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildConfig {
    /// Target platform for compilation.
    #[serde(default = "default_target")]
    pub target: String,

    /// Optimization level.
    #[serde(default = "default_optimize")]
    pub optimize: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: default_target(),
            optimize: default_optimize(),
        }
    }
}

impl BuildConfig {
    /// Returns true if this is the default configuration.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.target == default_target() && self.optimize == default_optimize()
    }
}

/// Verification configuration for Rocq output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationConfig {
    /// Output directory for generated Rocq proofs.
    #[serde(default = "default_output_dir", rename = "output-dir")]
    pub output_dir: String,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
        }
    }
}

impl VerificationConfig {
    /// Returns true if this is the default configuration.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.output_dir == default_output_dir()
    }
}

fn default_edition() -> String {
    String::from("2024")
}

fn default_manifest_version() -> u32 {
    1
}

fn default_target() -> String {
    String::from("wasm32")
}

fn default_optimize() -> String {
    String::from("debug")
}

fn default_output_dir() -> String {
    String::from("proofs/")
}

impl InferenceToml {
    /// Creates a new manifest with the given project name.
    ///
    /// The version defaults to "0.1.0", edition to "2024", and `manifest_version` to 1.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            package: Package {
                name: name.into(),
                version: String::from("0.1.0"),
                edition: default_edition(),
                manifest_version: 1,
                description: None,
                authors: None,
                license: None,
            },
            dependencies: Dependencies::default(),
            build: BuildConfig::default(),
            verification: VerificationConfig::default(),
        }
    }

    /// Parses a manifest from TOML content.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is invalid or missing required fields.
    #[allow(dead_code)]
    pub fn parse(content: &str) -> Result<Self> {
        toml::from_str(content).context("Failed to parse Inference.toml")
    }

    /// Serializes the manifest to TOML format.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).context("Failed to serialize Inference.toml")
    }

    /// Writes the manifest to a file.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file writing fails.
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let content = self.to_toml()?;
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write manifest: {}", path.display()))
    }

    /// Validates the manifest for correctness.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The project name is invalid
    /// - The version is not valid semver
    /// - The `manifest_version` is unsupported
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        validate_project_name(&self.package.name)?;

        if self.package.version.is_empty() {
            bail!("Package version cannot be empty");
        }

        Version::parse(&self.package.version).with_context(|| {
            format!(
                "Invalid semver version '{}'. Expected format: MAJOR.MINOR.PATCH (e.g., 1.0.0)",
                self.package.version
            )
        })?;

        if self.package.manifest_version != 1 {
            bail!(
                "Unsupported manifest_version: {}. Only version 1 is supported.",
                self.package.manifest_version
            );
        }

        Ok(())
    }

    /// Reads and parses a manifest from a file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    #[allow(dead_code)]
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest: {}", path.display()))?;
        Self::parse(&content)
    }
}

/// Validates a project name for use in Inference projects.
///
/// # Rules
///
/// - Must not be empty
/// - Must start with a letter or underscore
/// - Can only contain alphanumeric characters, underscores, and hyphens
/// - Must not be a reserved word
///
/// # Errors
///
/// Returns an error with a descriptive message if the name is invalid.
pub fn validate_project_name(name: &str) -> Result<()> {
    let Some(first_char) = name.chars().next() else {
        bail!("Project name cannot be empty");
    };

    if !first_char.is_ascii_alphabetic() && first_char != '_' {
        bail!("Project name '{name}' must start with a letter or underscore");
    }

    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' {
            bail!(
                "Project name '{name}' contains invalid character '{ch}'. \
                 Only letters, numbers, underscores, and hyphens are allowed."
            );
        }
    }

    let name_lower = name.to_lowercase();
    if RESERVED_WORDS.contains(&name_lower.as_str()) {
        bail!(
            "Project name '{name}' is a reserved word. \
             Please choose a different name."
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manifest_has_defaults() {
        let manifest = InferenceToml::new("myproject");
        assert_eq!(manifest.package.name, "myproject");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.package.edition, "2024");
        assert_eq!(manifest.package.manifest_version, 1);
        assert!(manifest.package.description.is_none());
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.build.is_default());
        assert!(manifest.verification.is_default());
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let content = r#"
[package]
name = "test-project"
version = "1.0.0"
"#;
        let manifest = InferenceToml::parse(content).unwrap();
        assert_eq!(manifest.package.name, "test-project");
        assert_eq!(manifest.package.version, "1.0.0");
        assert_eq!(manifest.package.edition, "2024");
        assert_eq!(manifest.package.manifest_version, 1);
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn test_parse_full_manifest() {
        let content = r#"
[package]
name = "full-project"
version = "2.1.0"
edition = "2024"
manifest_version = 1
description = "A test project"
authors = ["Alice", "Bob"]
license = "MIT"

[build]
target = "wasm64"
optimize = "release"

[verification]
output-dir = "custom-proofs/"
"#;
        let manifest = InferenceToml::parse(content).unwrap();
        assert_eq!(manifest.package.name, "full-project");
        assert_eq!(manifest.package.edition, "2024");
        assert_eq!(manifest.package.description, Some("A test project".into()));
        assert_eq!(
            manifest.package.authors,
            Some(vec!["Alice".into(), "Bob".into()])
        );
        assert_eq!(manifest.package.license, Some("MIT".into()));
        assert_eq!(manifest.build.target, "wasm64");
        assert_eq!(manifest.build.optimize, "release");
        assert_eq!(manifest.verification.output_dir, "custom-proofs/");
    }

    #[test]
    fn test_parse_invalid_toml() {
        let content = "not valid toml [[[";
        assert!(InferenceToml::parse(content).is_err());
    }

    #[test]
    fn test_parse_missing_package() {
        let content = r#"
[other]
name = "test"
"#;
        assert!(InferenceToml::parse(content).is_err());
    }

    #[test]
    fn test_to_toml() {
        let manifest = InferenceToml::new("myproject");
        let output = manifest.to_toml().unwrap();
        assert!(output.contains("name = \"myproject\""));
        assert!(output.contains("version = \"0.1.0\""));
        assert!(output.contains("edition = \"2024\""));
        assert!(output.contains("manifest_version = 1"));
    }

    #[test]
    fn test_roundtrip() {
        let original = InferenceToml::new("roundtrip-test");
        let serialized = original.to_toml().unwrap();
        let parsed = InferenceToml::parse(&serialized).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_validate_invalid_semver() {
        let mut manifest = InferenceToml::new("project");
        manifest.package.version = String::from("not-a-version");
        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid semver"));
    }

    #[test]
    fn test_validate_valid_semver() {
        let mut manifest = InferenceToml::new("project");
        manifest.package.version = String::from("1.2.3");
        assert!(manifest.validate().is_ok());

        manifest.package.version = String::from("0.0.1");
        assert!(manifest.validate().is_ok());

        manifest.package.version = String::from("10.20.30");
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_dependencies_is_empty() {
        let deps = Dependencies::default();
        assert!(deps.is_empty());

        let mut deps = Dependencies::default();
        deps.packages.insert(String::from("std"), String::from("0.1"));
        assert!(!deps.is_empty());
    }

    #[test]
    fn test_build_config_is_default() {
        let config = BuildConfig::default();
        assert!(config.is_default());

        let config = BuildConfig {
            target: String::from("wasm64"),
            optimize: String::from("debug"),
        };
        assert!(!config.is_default());
    }

    #[test]
    fn test_verification_config_is_default() {
        let config = VerificationConfig::default();
        assert!(config.is_default());

        let config = VerificationConfig {
            output_dir: String::from("custom/"),
        };
        assert!(!config.is_default());
    }

    #[test]
    fn test_validate_valid_manifest() {
        let manifest = InferenceToml::new("valid_project");
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_version() {
        let mut manifest = InferenceToml::new("project");
        manifest.package.version = String::new();
        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version"));
    }

    #[test]
    fn test_validate_unsupported_manifest_version() {
        let mut manifest = InferenceToml::new("project");
        manifest.package.manifest_version = 99;
        let result = manifest.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("manifest_version"));
    }

    #[test]
    fn test_validate_project_name_valid() {
        assert!(validate_project_name("myproject").is_ok());
        assert!(validate_project_name("my_project").is_ok());
        assert!(validate_project_name("my-project").is_ok());
        assert!(validate_project_name("_private").is_ok());
        assert!(validate_project_name("Project123").is_ok());
    }

    #[test]
    fn test_validate_project_name_empty() {
        let result = validate_project_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_project_name_starts_with_number() {
        let result = validate_project_name("123project");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("start with"));
    }

    #[test]
    fn test_validate_project_name_invalid_chars() {
        let result = validate_project_name("my.project");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid character")
        );

        let result = validate_project_name("my project");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid character")
        );
    }

    #[test]
    fn test_validate_project_name_reserved_keywords() {
        for &word in &["fn", "let", "struct", "type", "return", "if", "else"] {
            let result = validate_project_name(word);
            assert!(result.is_err(), "Expected '{word}' to be rejected");
            assert!(result.unwrap_err().to_string().contains("reserved"));
        }
    }

    #[test]
    fn test_validate_project_name_reserved_directories() {
        for &word in &["src", "target", "proofs", "tests", "out"] {
            let result = validate_project_name(word);
            assert!(result.is_err(), "Expected '{word}' to be rejected");
            assert!(result.unwrap_err().to_string().contains("reserved"));
        }
    }

    #[test]
    fn test_validate_project_name_reserved_case_insensitive() {
        let result = validate_project_name("FN");
        assert!(result.is_err());

        let result = validate_project_name("Struct");
        assert!(result.is_err());
    }
}
