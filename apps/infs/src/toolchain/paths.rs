#![warn(clippy::pedantic)]

//! Path management for the infs toolchain.
//!
//! This module provides utilities for managing toolchain installation paths.
//! The default root directory is `~/.inference/`, which can be overridden by
//! setting the `INFERENCE_HOME` environment variable.
//!
//! ## Directory Structure
//!
//! ```text
//! ~/.inference/               # Root directory (or INFERENCE_HOME)
//!   toolchains/               # Installed toolchain versions
//!     0.1.0/                  # Version-specific installation
//!       bin/
//!         infc
//!         inf-llc
//!         rust-lld
//!       .metadata.json        # Installation metadata (date, etc.)
//!     0.2.0/
//!       ...
//!   bin/                      # Symlinks to default toolchain binaries
//!   downloads/                # Download cache
//!   cache/                    # Cached data (manifest, etc.)
//!   default                   # File containing default version string
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Environment variable to override the default toolchain root directory.
pub const INFERENCE_HOME_ENV: &str = "INFERENCE_HOME";

/// Metadata file name stored in each toolchain version directory.
const METADATA_FILE: &str = ".metadata.json";

/// Metadata about a toolchain installation.
///
/// This is stored in each toolchain version directory as `.metadata.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainMetadata {
    /// ISO 8601 timestamp of when the toolchain was installed.
    pub installed_at: String,
}

impl ToolchainMetadata {
    /// Creates new metadata with the current timestamp.
    #[must_use = "returns new metadata without side effects"]
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            installed_at: format_timestamp(timestamp),
        }
    }

    /// Returns a human-readable relative time string (e.g., "2 days ago").
    #[must_use = "returns formatted time without side effects"]
    pub fn installed_ago(&self) -> String {
        parse_and_format_relative_time(&self.installed_at)
    }
}

/// Formats a Unix timestamp as an ISO 8601 date string (YYYY-MM-DD).
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp);
    let Ok(duration_since_unix) = datetime.duration_since(UNIX_EPOCH) else {
        return "unknown".to_string();
    };

    let secs = duration_since_unix.as_secs();
    let days_since_epoch = secs / 86400;

    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let month_days: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0;
    for (i, &days) in month_days.iter().enumerate() {
        if remaining_days < days {
            month = i + 1;
            break;
        }
        remaining_days -= days;
    }

    let day = remaining_days + 1;

    format!("{year:04}-{month:02}-{day:02}")
}

/// Checks if a year is a leap year.
fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Parses an ISO 8601 date and returns a relative time string.
fn parse_and_format_relative_time(date_str: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return date_str.to_string();
    }

    let Ok(year) = parts[0].parse::<u64>() else {
        return date_str.to_string();
    };
    let Ok(month) = parts[1].parse::<u64>() else {
        return date_str.to_string();
    };
    let Ok(day) = parts[2].parse::<u64>() else {
        return date_str.to_string();
    };

    let mut total_days: u64 = 0;
    for y in 1970..year {
        total_days += if is_leap_year(y) { 366 } else { 365 };
    }

    let month_days: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    #[allow(clippy::cast_possible_truncation)]
    let month_index = (month as usize).saturating_sub(1);
    for &days in month_days.iter().take(month_index) {
        total_days += days;
    }
    total_days += day - 1;

    let installed_secs = total_days * 86400;

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let diff_secs = now_secs.saturating_sub(installed_secs);
    let diff_days = diff_secs / 86400;

    match diff_days {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        2..=6 => format!("{diff_days} days ago"),
        7..=13 => "1 week ago".to_string(),
        14..=20 => "2 weeks ago".to_string(),
        21..=27 => "3 weeks ago".to_string(),
        28..=59 => "1 month ago".to_string(),
        60..=89 => "2 months ago".to_string(),
        90..=364 => format!("{} months ago", diff_days / 30),
        _ => format!("{} years ago", diff_days / 365),
    }
}

/// Manages paths for toolchain installations.
///
/// This struct provides access to all toolchain-related directories and files,
/// ensuring consistent path construction across the codebase.
#[derive(Debug, Clone)]
pub struct ToolchainPaths {
    /// Root directory for all toolchain data (`~/.inference` or `INFERENCE_HOME`).
    pub root: PathBuf,
    /// Directory containing installed toolchain versions.
    pub toolchains: PathBuf,
    /// Directory for binary symlinks to the default toolchain.
    pub bin: PathBuf,
    /// Directory for cached downloads.
    pub downloads: PathBuf,
}

impl ToolchainPaths {
    /// Creates a new `ToolchainPaths` instance.
    ///
    /// The root directory is determined by:
    /// 1. The `INFERENCE_HOME` environment variable if set
    /// 2. On Windows: `%APPDATA%\inference`
    /// 3. On Unix: `~/.inference` in the user's home directory
    ///
    /// # Errors
    ///
    /// Returns an error if the home directory cannot be determined.
    pub fn new() -> Result<Self> {
        let root = if let Ok(home) = std::env::var(INFERENCE_HOME_ENV) {
            PathBuf::from(home)
        } else {
            #[cfg(windows)]
            {
                dirs::data_dir()
                    .context("Cannot determine AppData directory. Set INFERENCE_HOME environment variable.")?
                    .join("inference")
            }
            #[cfg(not(windows))]
            {
                dirs::home_dir()
                    .context("Cannot determine home directory. Set INFERENCE_HOME environment variable.")?
                    .join(".inference")
            }
        };

        Ok(Self::with_root(root))
    }

    /// Creates a new `ToolchainPaths` instance with a specific root directory.
    ///
    /// This is useful for testing or when the root directory is known in advance.
    #[must_use = "returns new paths instance without side effects"]
    pub fn with_root(root: PathBuf) -> Self {
        Self {
            toolchains: root.join("toolchains"),
            bin: root.join("bin"),
            downloads: root.join("downloads"),
            root,
        }
    }

    /// Returns the path to a specific toolchain version's installation directory.
    #[must_use = "returns the path without side effects"]
    pub fn toolchain_dir(&self, version: &str) -> PathBuf {
        self.toolchains.join(version)
    }

    /// Returns the path to the bin directory within a specific toolchain version.
    #[must_use = "returns the path without side effects"]
    pub fn toolchain_bin_dir(&self, version: &str) -> PathBuf {
        self.toolchain_dir(version).join("bin")
    }

    /// Returns the path to the file storing the default toolchain version.
    #[must_use = "returns the path without side effects"]
    pub fn default_file(&self) -> PathBuf {
        self.root.join("default")
    }

    /// Returns the path for a downloaded archive file.
    #[must_use = "returns the path without side effects"]
    pub fn download_path(&self, filename: &str) -> PathBuf {
        self.downloads.join(filename)
    }

    /// Checks if a specific toolchain version is installed.
    #[must_use = "returns installation status without side effects"]
    pub fn is_version_installed(&self, version: &str) -> bool {
        self.toolchain_dir(version).exists()
    }

    /// Returns the currently set default toolchain version.
    ///
    /// # Errors
    ///
    /// Returns an error if the default file cannot be read.
    pub fn get_default_version(&self) -> Result<Option<String>> {
        let default_file = self.default_file();
        if !default_file.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&default_file)
            .with_context(|| format!("Failed to read default version from {}", default_file.display()))?;
        let version = content.trim();
        if version.is_empty() {
            Ok(None)
        } else {
            Ok(Some(version.to_string()))
        }
    }

    /// Sets the default toolchain version.
    ///
    /// # Errors
    ///
    /// Returns an error if the default file cannot be written.
    pub fn set_default_version(&self, version: &str) -> Result<()> {
        std::fs::create_dir_all(&self.root)
            .with_context(|| format!("Failed to create directory: {}", self.root.display()))?;
        std::fs::write(self.default_file(), version)
            .with_context(|| format!("Failed to write default version to {}", self.default_file().display()))?;
        Ok(())
    }

    /// Lists all installed toolchain versions.
    ///
    /// Returns a sorted list of version strings for all installed toolchains.
    ///
    /// # Errors
    ///
    /// Returns an error if the toolchains directory cannot be read.
    pub fn list_installed_versions(&self) -> Result<Vec<String>> {
        if !self.toolchains.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        let entries = std::fs::read_dir(&self.toolchains)
            .with_context(|| format!("Failed to read toolchains directory: {}", self.toolchains.display()))?;

        for entry in entries {
            let entry = entry.with_context(|| "Failed to read directory entry")?;
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = path.file_name()
                && let Some(name_str) = name.to_str()
            {
                versions.push(name_str.to_string());
            }
        }

        versions.sort();
        Ok(versions)
    }

    /// Ensures all required directories exist.
    ///
    /// Creates the root, toolchains, bin, and downloads directories if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if any directory cannot be created.
    pub fn ensure_directories(&self) -> Result<()> {
        for dir in [&self.root, &self.toolchains, &self.bin, &self.downloads] {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }
        Ok(())
    }

    /// Returns the path to a specific binary within a toolchain version.
    #[must_use = "returns the path without side effects"]
    pub fn binary_path(&self, version: &str, binary_name: &str) -> PathBuf {
        self.toolchain_bin_dir(version).join(binary_name)
    }

    /// Returns the path to a symlinked binary in the global bin directory.
    #[must_use = "returns the path without side effects"]
    pub fn symlink_path(&self, binary_name: &str) -> PathBuf {
        self.bin.join(binary_name)
    }

    /// Creates a symlink from the global bin directory to a toolchain binary.
    ///
    /// On Windows, this creates a hard link or copies the file if symlinks are not supported.
    ///
    /// # Errors
    ///
    /// Returns an error if the symlink cannot be created.
    pub fn create_symlink(&self, version: &str, binary_name: &str) -> Result<()> {
        let source = self.binary_path(version, binary_name);
        let target = self.symlink_path(binary_name);

        if !source.exists() {
            return Ok(());
        }

        if target.exists() {
            std::fs::remove_file(&target)
                .with_context(|| format!("Failed to remove existing symlink: {}", target.display()))?;
        }

        create_link(&source, &target)
    }

    /// Removes a symlink from the global bin directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the symlink cannot be removed.
    pub fn remove_symlink(&self, binary_name: &str) -> Result<()> {
        let target = self.symlink_path(binary_name);
        if target.exists() {
            std::fs::remove_file(&target)
                .with_context(|| format!("Failed to remove symlink: {}", target.display()))?;
        }
        Ok(())
    }

    /// Returns the path to the metadata file for a toolchain version.
    #[must_use = "returns the metadata path without side effects"]
    pub fn metadata_path(&self, version: &str) -> PathBuf {
        self.toolchain_dir(version).join(METADATA_FILE)
    }

    /// Writes installation metadata for a toolchain version.
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata file cannot be written.
    pub fn write_metadata(&self, version: &str, metadata: &ToolchainMetadata) -> Result<()> {
        let path = self.metadata_path(version);
        let content = serde_json::to_string_pretty(metadata)
            .context("Failed to serialize metadata")?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write metadata to {}", path.display()))?;
        Ok(())
    }

    /// Reads installation metadata for a toolchain version.
    ///
    /// Returns `None` if the metadata file does not exist or cannot be parsed.
    #[must_use = "returns metadata without side effects"]
    pub fn read_metadata(&self, version: &str) -> Option<ToolchainMetadata> {
        let path = self.metadata_path(version);
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Updates symlinks in the bin directory to point to the specified version.
    ///
    /// Creates symlinks for `infc`, `inf-llc`, and `rust-lld` binaries.
    ///
    /// # Errors
    ///
    /// Returns an error if the symlinks cannot be created.
    pub fn update_symlinks(&self, version: &str) -> Result<()> {
        let platform = crate::toolchain::Platform::detect()?;
        let ext = platform.executable_extension();

        let binaries = [
            format!("infc{ext}"),
            format!("inf-llc{ext}"),
            format!("rust-lld{ext}"),
        ];

        std::fs::create_dir_all(&self.bin)
            .with_context(|| format!("Failed to create bin directory: {}", self.bin.display()))?;

        for binary in &binaries {
            self.create_symlink(version, binary)?;
        }

        Ok(())
    }

    /// Removes all symlinks from the bin directory.
    ///
    /// Removes symlinks for `infc`, `inf-llc`, and `rust-lld` binaries.
    ///
    /// # Errors
    ///
    /// Returns an error if the symlinks cannot be removed.
    pub fn remove_symlinks(&self) -> Result<()> {
        let platform = crate::toolchain::Platform::detect()?;
        let ext = platform.executable_extension();

        let binaries = [
            format!("infc{ext}"),
            format!("inf-llc{ext}"),
            format!("rust-lld{ext}"),
        ];

        for binary in &binaries {
            self.remove_symlink(binary)?;
        }

        Ok(())
    }
}

/// Creates a symbolic link (Unix) or hard link (Windows) from source to target.
fn create_link(source: &Path, target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target)
            .with_context(|| format!("Failed to create symlink from {} to {}", source.display(), target.display()))?;
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(source, target)
            .or_else(|_| std::fs::hard_link(source, target))
            .or_else(|_| std::fs::copy(source, target).map(|_| ()))
            .with_context(|| format!("Failed to create link from {} to {}", source.display(), target.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn paths_with_infs_home_env() {
        // Use with_root directly to avoid race conditions with env vars
        let temp_dir = env::temp_dir().join("infs_test_home");
        let paths = ToolchainPaths::with_root(temp_dir.clone());

        assert_eq!(paths.root, temp_dir);
        assert_eq!(paths.toolchains, temp_dir.join("toolchains"));
        assert_eq!(paths.bin, temp_dir.join("bin"));
        assert_eq!(paths.downloads, temp_dir.join("downloads"));
    }

    #[test]
    fn toolchain_dir_constructs_correct_path() {
        let temp_dir = env::temp_dir().join("infs_test_toolchain_dir");
        let paths = ToolchainPaths::with_root(temp_dir.clone());

        assert_eq!(
            paths.toolchain_dir("0.1.0"),
            temp_dir.join("toolchains").join("0.1.0")
        );
    }

    #[test]
    fn default_file_path_is_correct() {
        let temp_dir = env::temp_dir().join("infs_test_default_file");
        let paths = ToolchainPaths::with_root(temp_dir.clone());

        assert_eq!(paths.default_file(), temp_dir.join("default"));
    }

    #[test]
    fn download_path_constructs_correctly() {
        let temp_dir = env::temp_dir().join("infs_test_download");
        let paths = ToolchainPaths::with_root(temp_dir.clone());

        assert_eq!(
            paths.download_path("toolchain.zip"),
            temp_dir.join("downloads").join("toolchain.zip")
        );
    }

    #[test]
    fn is_version_installed_returns_false_for_nonexistent() {
        let temp_dir = env::temp_dir().join("infs_test_installed");
        let paths = ToolchainPaths::with_root(temp_dir);

        assert!(!paths.is_version_installed("0.1.0"));
    }

    #[test]
    fn list_installed_versions_returns_empty_when_no_toolchains() {
        let temp_dir = env::temp_dir().join("infs_test_list_empty");
        let paths = ToolchainPaths::with_root(temp_dir);

        let versions = paths.list_installed_versions().expect("Should list versions");
        assert!(versions.is_empty());
    }

    #[test]
    fn metadata_path_constructs_correctly() {
        let temp_dir = env::temp_dir().join("infs_test_metadata_path");
        let paths = ToolchainPaths::with_root(temp_dir.clone());

        assert_eq!(
            paths.metadata_path("0.1.0"),
            temp_dir.join("toolchains").join("0.1.0").join(".metadata.json")
        );
    }

    #[test]
    fn toolchain_metadata_now_creates_valid_date() {
        let metadata = ToolchainMetadata::now();
        let parts: Vec<&str> = metadata.installed_at.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[0].parse::<u64>().is_ok());
        assert!(parts[1].parse::<u64>().is_ok());
        assert!(parts[2].parse::<u64>().is_ok());
    }

    #[test]
    fn format_timestamp_produces_valid_iso_date() {
        let date = format_timestamp(0);
        assert_eq!(date, "1970-01-01");

        let date = format_timestamp(86400);
        assert_eq!(date, "1970-01-02");

        let date = format_timestamp(1_704_067_200);
        assert_eq!(date, "2024-01-01");
    }

    #[test]
    fn is_leap_year_correct() {
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn relative_time_today() {
        let metadata = ToolchainMetadata::now();
        assert_eq!(metadata.installed_ago(), "today");
    }
}
