#![warn(clippy::pedantic)]

//! Toolchain management module for the infs CLI.
//!
//! This module provides functionality for managing Inference toolchain installations,
//! including downloading, verifying, installing, and switching between versions.
//!
//! ## Module Structure
//!
//! - [`platform`] - OS and architecture detection
//! - [`paths`] - Toolchain directory path management
//! - [`manifest`] - Release manifest fetching and parsing
//! - [`download`] - HTTP download with progress tracking
//! - [`verify`] - SHA256 checksum verification
//! - [`archive`] - ZIP archive extraction utilities
//! - [`doctor`] - Toolchain health checks

pub mod archive;
pub mod doctor;
pub mod download;
pub mod manifest;
pub mod paths;
pub mod platform;
pub mod verify;

pub use archive::extract_zip;
pub use download::{ProgressCallback, ProgressEvent, download_file, download_file_with_callback};
pub use manifest::{fetch_artifact, fetch_manifest};
pub use paths::ToolchainPaths;
pub use platform::Platform;
pub use verify::verify_checksum;

use anyhow::Context;

/// A validated toolchain version.
///
/// This type ensures that version strings are valid semver versions.
/// It wraps `semver::Version` and provides parsing and display functionality.
///
/// Note: Currently unused but reserved for Phase 2 toolchain version management.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolchainVersion(semver::Version);

#[allow(dead_code)]
impl ToolchainVersion {
    /// Creates a new validated toolchain version from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid semver version.
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let version = semver::Version::parse(s)
            .with_context(|| format!("Invalid version string: {s}"))?;
        Ok(Self(version))
    }

    /// Returns the version as a string.
    #[must_use = "returns the version string without side effects"]
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }

    /// Returns a reference to the inner `semver::Version`.
    #[must_use = "returns the inner version without side effects"]
    pub fn inner(&self) -> &semver::Version {
        &self.0
    }
}

impl std::fmt::Display for ToolchainVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_semver_version() {
        let version = ToolchainVersion::parse("1.2.3").expect("Should parse valid version");
        assert_eq!(version.as_str(), "1.2.3");
    }

    #[test]
    fn parse_version_with_prerelease() {
        let version = ToolchainVersion::parse("1.0.0-alpha.1").expect("Should parse prerelease");
        assert_eq!(version.as_str(), "1.0.0-alpha.1");
    }

    #[test]
    fn parse_version_with_build_metadata() {
        let version = ToolchainVersion::parse("1.0.0+build.123").expect("Should parse build metadata");
        assert_eq!(version.as_str(), "1.0.0+build.123");
    }

    #[test]
    fn parse_invalid_version_fails() {
        let result = ToolchainVersion::parse("not-a-version");
        assert!(result.is_err());
    }

    #[test]
    fn parse_partial_version_fails() {
        let result = ToolchainVersion::parse("1.2");
        assert!(result.is_err());
    }

    #[test]
    fn display_matches_as_str() {
        let version = ToolchainVersion::parse("2.0.0").expect("Should parse");
        assert_eq!(format!("{version}"), version.as_str());
    }

    #[test]
    fn versions_can_be_compared() {
        let v1 = ToolchainVersion::parse("1.0.0").expect("Should parse");
        let v2 = ToolchainVersion::parse("2.0.0").expect("Should parse");
        let v3 = ToolchainVersion::parse("1.0.0").expect("Should parse");

        assert!(v1 < v2);
        assert!(v2 > v1);
        assert_eq!(v1, v3);
    }

    #[test]
    fn inner_returns_semver_version() {
        let version = ToolchainVersion::parse("1.2.3").expect("Should parse");
        assert_eq!(version.inner().major, 1);
        assert_eq!(version.inner().minor, 2);
        assert_eq!(version.inner().patch, 3);
    }
}
