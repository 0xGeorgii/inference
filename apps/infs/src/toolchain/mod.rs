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
pub mod resolver;
pub mod verify;

pub use archive::extract_zip;
pub use download::{ProgressCallback, ProgressEvent, download_file, download_file_with_callback};
pub use manifest::{fetch_artifact, fetch_manifest, latest_stable};
pub use paths::ToolchainPaths;
pub use platform::Platform;
pub use resolver::find_infc;
pub use verify::verify_checksum;
