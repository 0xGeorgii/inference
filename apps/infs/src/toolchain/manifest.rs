#![warn(clippy::pedantic)]

//! Release manifest handling for the infs toolchain.
//!
//! This module provides functionality for fetching and parsing the toolchain
//! release manifest from a static distribution server.
//!
//! ## Manifest Format
//!
//! The manifest is a flat JSON array of version entries:
//!
//! ```json
//! [
//!   {
//!     "version": "0.2.0",
//!     "stable": true,
//!     "files": [
//!       {
//!         "filename": "infc-linux-x64.tar.gz",
//!         "os": "linux",
//!         "arch": "x64",
//!         "tool": "infc",
//!         "url": "https://...",
//!         "sha256": "abc123...",
//!         "size": 12345678
//!       }
//!     ]
//!   }
//! ]
//! ```
//!
//! ## Data Source
//!
//! Release information is fetched from a static `releases.json` file hosted on
//! the distribution server (default: `https://inference-lang.org`). The server
//! can be overridden via the `INFS_DIST_SERVER` environment variable for testing
//! or using a mirror.
//!
//! ## Caching
//!
//! The manifest is cached locally at `~/.infs/cache/manifest.json` with a 15-minute
//! TTL (configurable via `INFS_MANIFEST_CACHE_TTL` environment variable for testing).
//! On cache miss or expiry, the manifest is fetched from the distribution server.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Platform;

/// Environment variable to override the distribution server URL.
pub const DIST_SERVER_ENV: &str = "INFS_DIST_SERVER";

/// Default distribution server URL.
const DEFAULT_DIST_SERVER: &str = "https://inference-lang.org";

/// Path to releases manifest on server.
const RELEASES_PATH: &str = "/releases.json";

/// Request timeout in seconds.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// User-Agent header for HTTP requests.
const USER_AGENT: &str = "infs-toolchain-manager";

/// Environment variable to override the cache TTL (in seconds) for testing.
pub const CACHE_TTL_ENV: &str = "INFS_MANIFEST_CACHE_TTL";

/// Default cache TTL in seconds (15 minutes).
const DEFAULT_CACHE_TTL_SECS: u64 = 15 * 60;

/// Platform-specific file entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    /// Filename of the artifact (e.g., "infc-linux-x64.tar.gz").
    pub filename: String,
    /// Operating system (e.g., "linux", "macos", "windows").
    pub os: String,
    /// Architecture (e.g., "x64", "arm64").
    pub arch: String,
    /// Tool name (e.g., "infc", "infs").
    pub tool: String,
    /// Download URL for the artifact.
    pub url: String,
    /// SHA256 checksum of the artifact (required).
    pub sha256: String,
    /// Size of the artifact in bytes.
    pub size: u64,
}

/// Version entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionEntry {
    /// The version string (e.g., "0.2.0").
    pub version: String,
    /// Whether this is a stable release.
    pub stable: bool,
    /// Platform-specific files for this version.
    pub files: Vec<FileEntry>,
}

impl VersionEntry {
    /// Finds the artifact for a specific platform and tool.
    ///
    /// # Arguments
    ///
    /// * `platform` - The target platform
    /// * `tool` - The tool name (e.g., "infc")
    ///
    /// # Returns
    ///
    /// The file entry, or `None` if no matching artifact exists.
    #[must_use = "returns artifact info without side effects"]
    pub fn find_artifact(&self, platform: Platform, tool: &str) -> Option<&FileEntry> {
        let (os, arch) = platform.os_arch();
        self.files
            .iter()
            .find(|f| f.os == os && f.arch == arch && f.tool == tool)
    }

    /// Finds the infc artifact for a specific platform.
    ///
    /// This is a convenience method for finding the compiler artifact.
    ///
    /// # Arguments
    ///
    /// * `platform` - The target platform
    ///
    /// # Returns
    ///
    /// The file entry, or `None` if no matching artifact exists.
    #[must_use = "returns artifact info without side effects"]
    pub fn find_infc_artifact(&self, platform: Platform) -> Option<&FileEntry> {
        self.find_artifact(platform, "infc")
    }

    /// Finds the infs CLI artifact for a specific platform.
    ///
    /// # Arguments
    ///
    /// * `platform` - The target platform
    ///
    /// # Returns
    ///
    /// The file entry, or `None` if no matching artifact exists.
    #[must_use = "returns artifact info without side effects"]
    pub fn find_infs_artifact(&self, platform: Platform) -> Option<&FileEntry> {
        self.find_artifact(platform, "infs")
    }
}

/// Release manifest - array of version entries.
pub type Manifest = Vec<VersionEntry>;

/// Finds the latest stable version in the manifest.
///
/// Stable versions are sorted by semver and the highest one is returned.
///
/// # Arguments
///
/// * `manifest` - The manifest to search
///
/// # Returns
///
/// The latest stable version entry, or `None` if no stable versions exist.
#[must_use = "returns version info without side effects"]
pub fn latest_stable(manifest: &Manifest) -> Option<&VersionEntry> {
    manifest.iter().filter(|v| v.stable).max_by(|a, b| {
        let a_ver = semver::Version::parse(&a.version).ok();
        let b_ver = semver::Version::parse(&b.version).ok();
        match (a_ver, b_ver) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Greater,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (None, None) => a.version.cmp(&b.version),
        }
    })
}

/// Finds a specific version in the manifest.
///
/// # Arguments
///
/// * `manifest` - The manifest to search
/// * `version` - The version string to find
///
/// # Returns
///
/// The version entry, or `None` if not found.
#[must_use = "returns version info without side effects"]
pub fn find_version<'a>(manifest: &'a Manifest, version: &str) -> Option<&'a VersionEntry> {
    manifest.iter().find(|v| v.version == version)
}

/// Returns all available version strings from the manifest.
///
/// # Arguments
///
/// * `manifest` - The manifest to query
///
/// # Returns
///
/// A vector of version strings.
#[allow(dead_code)]
#[must_use = "returns version list without side effects"]
pub fn available_versions(manifest: &Manifest) -> Vec<&str> {
    manifest.iter().map(|v| v.version.as_str()).collect()
}

/// Cached manifest with timestamp.
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedManifest {
    manifest: Manifest,
    timestamp: u64,
}

/// Returns the cache TTL in seconds, checking the environment variable first.
fn cache_ttl_secs() -> u64 {
    std::env::var(CACHE_TTL_ENV)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_CACHE_TTL_SECS)
}

/// Returns the path to the manifest cache file.
fn cache_path() -> Result<PathBuf> {
    let root = if let Ok(home) = std::env::var(super::paths::INFS_HOME_ENV) {
        PathBuf::from(home)
    } else {
        #[cfg(windows)]
        {
            dirs::data_dir()
                .context("Cannot determine AppData directory")?
                .join("infs")
        }
        #[cfg(not(windows))]
        {
            dirs::home_dir()
                .context("Cannot determine home directory")?
                .join(".infs")
        }
    };
    Ok(root.join("cache").join("manifest.json"))
}

/// Returns the current Unix timestamp.
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Attempts to load the manifest from cache if valid.
///
/// If the cache file exists but cannot be parsed (e.g., old format),
/// it will be deleted to allow a fresh fetch.
fn load_from_cache() -> Option<Manifest> {
    let cache_file = cache_path().ok()?;
    let content = std::fs::read_to_string(&cache_file).ok()?;

    let Ok(cached) = serde_json::from_str::<CachedManifest>(&content) else {
        // Old format or corrupted cache - delete it
        let _ = std::fs::remove_file(&cache_file);
        return None;
    };

    let now = current_timestamp();
    let ttl = cache_ttl_secs();

    if now.saturating_sub(cached.timestamp) < ttl {
        Some(cached.manifest)
    } else {
        None
    }
}

/// Saves the manifest to cache.
fn save_to_cache(manifest: &Manifest) {
    let Ok(cache_file) = cache_path() else {
        return;
    };

    if let Some(parent) = cache_file.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return;
    }

    let cached = CachedManifest {
        manifest: manifest.clone(),
        timestamp: current_timestamp(),
    };

    let Ok(content) = serde_json::to_string_pretty(&cached) else {
        return;
    };

    let _ = std::fs::write(cache_file, content);
}

/// Fetches the release manifest, using a local cache with 15-minute TTL.
///
/// The manifest is cached at `~/.infs/cache/manifest.json`. If the cache is valid,
/// returns the cached manifest without making a network request. On cache miss or
/// expiry, fetches from the static manifest URL and updates the cache.
///
/// # Errors
///
/// Returns an error if:
/// - The manifest URL cannot be fetched (and no valid cache exists)
/// - The response cannot be parsed as JSON
pub async fn fetch_manifest() -> Result<Manifest> {
    if let Some(manifest) = load_from_cache() {
        return Ok(manifest);
    }

    let manifest = fetch_manifest_from_network().await?;
    save_to_cache(&manifest);
    Ok(manifest)
}

/// Returns the URL to the releases manifest.
///
/// Checks the `INFS_DIST_SERVER` environment variable first, then falls back
/// to the default distribution server. Empty or whitespace-only values are
/// treated as unset.
fn releases_url() -> String {
    let server = std::env::var(DIST_SERVER_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_DIST_SERVER.to_string());
    let server = server.trim().trim_end_matches('/');
    format!("{server}{RELEASES_PATH}")
}

/// Handles HTTP errors with user-friendly messages.
fn handle_http_error(status: reqwest::StatusCode, url: &str) -> anyhow::Error {
    match status.as_u16() {
        404 => anyhow::anyhow!("Release manifest not found at {url}"),
        code if code >= 500 => anyhow::anyhow!("Server error ({code}): {url}"),
        code => anyhow::anyhow!("HTTP error {code}: {url}"),
    }
}

/// Fetches the release manifest directly from the distribution server, bypassing cache.
///
/// This function fetches the `releases.json` file from the configured distribution
/// server (default: `https://inference-lang.org`).
///
/// # Errors
///
/// Returns an error if:
/// - The HTTP request fails
/// - The server returns a non-success status code
/// - The response cannot be parsed as JSON
async fn fetch_manifest_from_network() -> Result<Manifest> {
    let url = releases_url();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch manifest from {url}"))?;

    if !response.status().is_success() {
        return Err(handle_http_error(response.status(), &url));
    }

    let text = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from {url}"))?;

    let manifest: Manifest = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse manifest from {url}"))?;

    Ok(manifest)
}

/// Fetches the release manifest and finds the artifact for a specific version and platform.
///
/// If `version` is `None` or "latest", returns the latest stable version's artifact.
///
/// # Errors
///
/// Returns an error if:
/// - The manifest cannot be fetched
/// - The specified version is not found
/// - No artifact exists for the current platform
pub async fn fetch_artifact(
    version: Option<&str>,
    platform: Platform,
) -> Result<(String, FileEntry)> {
    let manifest = fetch_manifest().await?;

    let version_entry = match version {
        None | Some("latest") => {
            latest_stable(&manifest).context("No stable version found in manifest")?
        }
        Some(v) => find_version(&manifest, v)
            .with_context(|| format!("Version {v} not found in manifest"))?,
    };

    let artifact = version_entry
        .find_infc_artifact(platform)
        .with_context(|| {
            format!(
                "No artifact found for platform {} in version {}",
                platform, version_entry.version
            )
        })?
        .clone();

    Ok((version_entry.version.clone(), artifact))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest_json() -> &'static str {
        r#"[
            {
                "version": "0.1.0",
                "stable": true,
                "files": [
                    {
                        "filename": "infc-linux-x64.tar.gz",
                        "os": "linux",
                        "arch": "x64",
                        "tool": "infc",
                        "url": "https://example.com/0.1.0/infc-linux-x64.tar.gz",
                        "sha256": "abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
                        "size": 1000
                    }
                ]
            },
            {
                "version": "0.2.0",
                "stable": true,
                "files": [
                    {
                        "filename": "infc-linux-x64.tar.gz",
                        "os": "linux",
                        "arch": "x64",
                        "tool": "infc",
                        "url": "https://example.com/0.2.0/infc-linux-x64.tar.gz",
                        "sha256": "def456abc123def456abc123def456abc123def456abc123def456abc123def4",
                        "size": 2000
                    },
                    {
                        "filename": "infc-macos-arm64.tar.gz",
                        "os": "macos",
                        "arch": "arm64",
                        "tool": "infc",
                        "url": "https://example.com/0.2.0/infc-macos-arm64.tar.gz",
                        "sha256": "ghi789abc123def456abc123def456abc123def456abc123def456abc123def4",
                        "size": 2100
                    },
                    {
                        "filename": "infs-linux-x64.tar.gz",
                        "os": "linux",
                        "arch": "x64",
                        "tool": "infs",
                        "url": "https://example.com/0.2.0/infs-linux-x64.tar.gz",
                        "sha256": "infs123abc123def456abc123def456abc123def456abc123def456abc123def",
                        "size": 500
                    }
                ]
            },
            {
                "version": "0.3.0-alpha",
                "stable": false,
                "files": []
            }
        ]"#
    }

    #[test]
    fn parse_manifest_json() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        assert_eq!(manifest.len(), 3);
        assert_eq!(manifest[0].version, "0.1.0");
        assert!(manifest[0].stable);
    }

    #[test]
    fn find_version_returns_correct_info() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let version = find_version(&manifest, "0.1.0").expect("Should find version");
        assert_eq!(version.version, "0.1.0");
        assert!(version.stable);
    }

    #[test]
    fn find_version_returns_none_for_missing() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        assert!(find_version(&manifest, "9.9.9").is_none());
    }

    #[test]
    fn latest_stable_returns_highest_stable_version() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let latest = latest_stable(&manifest).expect("Should find latest stable");
        assert_eq!(latest.version, "0.2.0");
    }

    #[test]
    fn latest_stable_ignores_prereleases() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let latest = latest_stable(&manifest).expect("Should find latest stable");
        assert!(latest.stable);
        assert_ne!(latest.version, "0.3.0-alpha");
    }

    #[test]
    fn find_infc_artifact_returns_correct_platform() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let version = find_version(&manifest, "0.2.0").expect("Should find version");
        let artifact = version
            .find_infc_artifact(Platform::LinuxX64)
            .expect("Should find artifact");

        assert_eq!(artifact.os, "linux");
        assert_eq!(artifact.arch, "x64");
        assert_eq!(artifact.tool, "infc");
    }

    #[test]
    fn find_infs_artifact_returns_cli_artifact() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let version = find_version(&manifest, "0.2.0").expect("Should find version");
        let artifact = version
            .find_infs_artifact(Platform::LinuxX64)
            .expect("Should find artifact");

        assert_eq!(artifact.tool, "infs");
        assert!(artifact.url.contains("infs-"));
    }

    #[test]
    fn find_artifact_returns_none_for_missing_platform() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let version = find_version(&manifest, "0.1.0").expect("Should find version");
        assert!(version.find_infc_artifact(Platform::WindowsX64).is_none());
    }

    #[test]
    fn available_versions_returns_all() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let versions = available_versions(&manifest);
        assert_eq!(versions.len(), 3);
        assert!(versions.contains(&"0.1.0"));
        assert!(versions.contains(&"0.2.0"));
        assert!(versions.contains(&"0.3.0-alpha"));
    }

    #[test]
    #[serial_test::serial]
    fn cache_ttl_uses_env_when_set() {
        unsafe { std::env::set_var(CACHE_TTL_ENV, "60") };
        assert_eq!(cache_ttl_secs(), 60);
        unsafe { std::env::remove_var(CACHE_TTL_ENV) };
    }

    #[test]
    #[serial_test::serial]
    fn cache_ttl_uses_default_when_env_not_set() {
        unsafe { std::env::remove_var(CACHE_TTL_ENV) };
        assert_eq!(cache_ttl_secs(), DEFAULT_CACHE_TTL_SECS);
    }

    #[test]
    fn cached_manifest_serializes_and_deserializes() {
        let manifest: Manifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let cached = CachedManifest {
            manifest: manifest.clone(),
            timestamp: 1_000_000,
        };

        let json = serde_json::to_string(&cached).expect("Should serialize");
        let deserialized: CachedManifest = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.timestamp, 1_000_000);
        assert_eq!(deserialized.manifest.len(), manifest.len());
    }

    #[test]
    fn file_entry_has_required_fields() {
        let json = r#"{
            "filename": "infc-linux-x64.tar.gz",
            "os": "linux",
            "arch": "x64",
            "tool": "infc",
            "url": "https://example.com/file.tar.gz",
            "sha256": "abc123def456abc123def456abc123def456abc123def456abc123def456abc1",
            "size": 1000
        }"#;

        let entry: FileEntry = serde_json::from_str(json).expect("Should parse");
        assert_eq!(entry.filename, "infc-linux-x64.tar.gz");
        assert_eq!(entry.sha256.len(), 64);
    }

    #[test]
    fn version_entry_stable_defaults_correctly() {
        let json = r#"{
            "version": "0.1.0",
            "stable": false,
            "files": []
        }"#;

        let entry: VersionEntry = serde_json::from_str(json).expect("Should parse");
        assert!(!entry.stable);
    }

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(CACHE_TTL_ENV, "INFS_MANIFEST_CACHE_TTL");
        assert_eq!(DIST_SERVER_ENV, "INFS_DIST_SERVER");
        assert_eq!(DEFAULT_DIST_SERVER, "https://inference-lang.org");
        assert_eq!(RELEASES_PATH, "/releases.json");
    }

    #[test]
    #[serial_test::serial]
    fn releases_url_uses_default_when_env_not_set() {
        unsafe { std::env::remove_var(DIST_SERVER_ENV) };
        let url = releases_url();
        assert!(url.starts_with("https://inference-lang.org"));
        assert!(url.ends_with("/releases.json"));
    }

    #[test]
    #[serial_test::serial]
    fn releases_url_uses_env_when_set() {
        unsafe { std::env::set_var(DIST_SERVER_ENV, "http://localhost:8080") };
        let url = releases_url();
        assert_eq!(url, "http://localhost:8080/releases.json");
        unsafe { std::env::remove_var(DIST_SERVER_ENV) };
    }

    #[test]
    #[serial_test::serial]
    fn releases_url_handles_trailing_slash() {
        unsafe { std::env::set_var(DIST_SERVER_ENV, "http://localhost:8080/") };
        let url = releases_url();
        assert_eq!(url, "http://localhost:8080/releases.json");
        unsafe { std::env::remove_var(DIST_SERVER_ENV) };
    }

    #[test]
    #[serial_test::serial]
    fn releases_url_uses_default_when_env_empty() {
        unsafe { std::env::set_var(DIST_SERVER_ENV, "") };
        let url = releases_url();
        assert!(url.starts_with("https://inference-lang.org"));
        assert!(url.ends_with("/releases.json"));
        unsafe { std::env::remove_var(DIST_SERVER_ENV) };
    }

    #[test]
    #[serial_test::serial]
    fn releases_url_uses_default_when_env_whitespace_only() {
        unsafe { std::env::set_var(DIST_SERVER_ENV, "   ") };
        let url = releases_url();
        assert!(url.starts_with("https://inference-lang.org"));
        assert!(url.ends_with("/releases.json"));
        unsafe { std::env::remove_var(DIST_SERVER_ENV) };
    }

    #[test]
    #[serial_test::serial]
    fn releases_url_trims_whitespace() {
        unsafe { std::env::set_var(DIST_SERVER_ENV, "  http://localhost:8080  ") };
        let url = releases_url();
        assert_eq!(url, "http://localhost:8080/releases.json");
        unsafe { std::env::remove_var(DIST_SERVER_ENV) };
    }

    #[test]
    fn handle_http_error_404() {
        let error = handle_http_error(reqwest::StatusCode::NOT_FOUND, "https://example.com");
        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn handle_http_error_500() {
        let error =
            handle_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "https://example.com");
        assert!(error.to_string().contains("500"));
    }

    #[test]
    fn handle_http_error_generic() {
        let error = handle_http_error(reqwest::StatusCode::BAD_REQUEST, "https://example.com");
        assert!(error.to_string().contains("400"));
    }

    #[test]
    fn latest_stable_returns_none_for_empty_manifest() {
        let manifest: Manifest = vec![];
        assert!(latest_stable(&manifest).is_none());
    }

    #[test]
    fn latest_stable_returns_none_when_all_prereleases() {
        let manifest: Manifest = vec![
            VersionEntry {
                version: "0.1.0-alpha".to_string(),
                stable: false,
                files: vec![],
            },
            VersionEntry {
                version: "0.2.0-beta".to_string(),
                stable: false,
                files: vec![],
            },
        ];

        assert!(latest_stable(&manifest).is_none());
    }

    #[test]
    fn find_artifact_with_tool_parameter() {
        let entry = VersionEntry {
            version: "0.1.0".to_string(),
            stable: true,
            files: vec![
                FileEntry {
                    filename: "infc-linux-x64.tar.gz".to_string(),
                    os: "linux".to_string(),
                    arch: "x64".to_string(),
                    tool: "infc".to_string(),
                    url: "https://example.com/infc".to_string(),
                    sha256: "a".repeat(64),
                    size: 100,
                },
                FileEntry {
                    filename: "infs-linux-x64.tar.gz".to_string(),
                    os: "linux".to_string(),
                    arch: "x64".to_string(),
                    tool: "infs".to_string(),
                    url: "https://example.com/infs".to_string(),
                    sha256: "b".repeat(64),
                    size: 50,
                },
            ],
        };

        let compiler_artifact = entry.find_artifact(Platform::LinuxX64, "infc");
        assert!(compiler_artifact.is_some());
        assert_eq!(compiler_artifact.unwrap().tool, "infc");

        let cli_artifact = entry.find_artifact(Platform::LinuxX64, "infs");
        assert!(cli_artifact.is_some());
        assert_eq!(cli_artifact.unwrap().tool, "infs");

        let other = entry.find_artifact(Platform::LinuxX64, "other");
        assert!(other.is_none());
    }
}
