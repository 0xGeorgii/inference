#![warn(clippy::pedantic)]

//! Release manifest handling for the infs toolchain.
//!
//! This module provides functionality for fetching and parsing the toolchain
//! release manifest from GitHub Releases, which serves as the source of truth
//! for available toolchain versions.
//!
//! ## Data Source
//!
//! The manifest is built from GitHub Releases API data. Each release is converted
//! to a `VersionInfo` containing platform-specific artifacts parsed from the
//! release assets.
//!
//! ## Caching
//!
//! The manifest is cached locally at `~/.infs/cache/manifest.json` with a 15-minute
//! TTL (configurable via `INFS_MANIFEST_CACHE_TTL` environment variable for testing).
//! On cache miss or expiry, the manifest is fetched from the GitHub API.
//!
//! ## Checksums
//!
//! Since GitHub Releases API does not provide checksums in the response, the
//! `sha256` field in `PlatformArtifact` is initially empty. Use
//! [`fetch_artifact_checksum`] to fetch the checksum from sidecar `.sha256` files.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Platform;
use super::github::{GitHubAsset, GitHubRelease};

/// Environment variable to override the manifest URL.
///
/// **Deprecated**: The manifest is now fetched from GitHub Releases API.
/// This constant is kept for backwards compatibility but is no longer used.
#[allow(dead_code)]
#[deprecated(since = "0.1.0", note = "Manifest is now fetched from GitHub Releases API")]
pub const MANIFEST_URL_ENV: &str = "INFS_MANIFEST_URL";

/// Default URL for the release manifest.
///
/// **Deprecated**: The manifest is now fetched from GitHub Releases API.
/// This constant is kept for backwards compatibility but is no longer used.
#[allow(dead_code)]
#[deprecated(since = "0.1.0", note = "Manifest is now fetched from GitHub Releases API")]
pub const DEFAULT_MANIFEST_URL: &str = "https://inference-lang.org/releases/manifest.json";

/// Environment variable to override the cache TTL (in seconds) for testing.
pub const CACHE_TTL_ENV: &str = "INFS_MANIFEST_CACHE_TTL";

/// Pattern prefix for recognizing toolchain artifacts in release assets.
pub const ARTIFACT_PATTERN: &str = "infc-";

/// Pattern prefix for recognizing CLI artifacts in release assets.
pub const INFS_ARTIFACT_PATTERN: &str = "infs-";

/// Default cache TTL in seconds (15 minutes).
const DEFAULT_CACHE_TTL_SECS: u64 = 15 * 60;

/// Cached manifest with timestamp.
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedManifest {
    manifest: ReleaseManifest,
    timestamp: u64,
}

/// Release manifest containing available toolchain versions.
///
/// The manifest is built from GitHub Releases data and lists all available
/// toolchain versions along with their platform-specific download URLs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ReleaseManifest {
    /// Schema version of the manifest format.
    pub schema_version: u32,
    /// The latest stable toolchain version.
    pub latest_stable: String,
    /// Latest version of the `infs` CLI binary itself.
    #[serde(default)]
    pub latest_infs: Option<String>,
    /// List of all available toolchain versions.
    pub versions: Vec<VersionInfo>,
    /// Platform-specific artifacts for the `infs` CLI binary.
    #[serde(default)]
    pub infs_artifacts: Vec<PlatformArtifact>,
}

/// Information about a specific toolchain version.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct VersionInfo {
    /// The version string (e.g., "0.1.0").
    pub version: String,
    /// Release date in ISO 8601 format (e.g., "2024-01-15").
    pub date: String,
    /// Whether this is a prerelease version.
    #[serde(default)]
    pub prerelease: bool,
    /// Platform-specific artifacts for this version.
    pub platforms: Vec<PlatformArtifact>,
}

/// Platform-specific download artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformArtifact {
    /// Platform identifier (e.g., "linux-x64", "macos-arm64", "windows-x64").
    pub platform: String,
    /// Download URL for the artifact.
    pub url: String,
    /// SHA256 checksum of the artifact.
    ///
    /// This field is `None` when the manifest is fetched from the GitHub API,
    /// since the API does not include checksums in release asset metadata.
    /// Use [`fetch_artifact_checksum`] to fetch the actual checksum from
    /// sidecar `.sha256` files attached to the release.
    ///
    /// When present (e.g., from a static `manifest.json` file), this contains
    /// the SHA256 hash as a lowercase hex string.
    pub sha256: Option<String>,
    /// Size of the artifact in bytes.
    pub size: u64,
}

#[allow(dead_code)]
impl ReleaseManifest {
    /// Finds version information for a specific version string.
    ///
    /// Returns `None` if the version is not found in the manifest.
    #[must_use = "returns version info without side effects"]
    pub fn find_version(&self, version: &str) -> Option<&VersionInfo> {
        self.versions.iter().find(|v| v.version == version)
    }

    /// Returns the latest stable version information.
    ///
    /// Returns `None` if the latest stable version is not found in the manifest.
    #[must_use = "returns version info without side effects"]
    pub fn latest_stable_version(&self) -> Option<&VersionInfo> {
        self.find_version(&self.latest_stable)
    }

    /// Returns all available version strings.
    #[must_use = "returns version list without side effects"]
    pub fn available_versions(&self) -> Vec<&str> {
        self.versions.iter().map(|v| v.version.as_str()).collect()
    }

    /// Finds the infs CLI artifact for the given platform.
    #[must_use = "returns artifact info without side effects"]
    pub fn find_infs_artifact(&self, platform: Platform) -> Option<&PlatformArtifact> {
        self.infs_artifacts
            .iter()
            .find(|a| a.platform == platform.as_str())
    }
}

impl VersionInfo {
    /// Finds the artifact for a specific platform.
    ///
    /// Returns `None` if no artifact exists for the given platform.
    #[must_use = "returns artifact info without side effects"]
    pub fn find_artifact(&self, platform: Platform) -> Option<&PlatformArtifact> {
        self.platforms
            .iter()
            .find(|a| a.platform == platform.as_str())
    }
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
fn load_from_cache() -> Option<ReleaseManifest> {
    let cache_file = cache_path().ok()?;
    let content = std::fs::read_to_string(&cache_file).ok()?;
    let cached: CachedManifest = serde_json::from_str(&content).ok()?;

    let now = current_timestamp();
    let ttl = cache_ttl_secs();

    if now.saturating_sub(cached.timestamp) < ttl {
        Some(cached.manifest)
    } else {
        None
    }
}

/// Saves the manifest to cache.
fn save_to_cache(manifest: &ReleaseManifest) {
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

/// Extracts the date portion from an ISO 8601 timestamp.
///
/// # Arguments
///
/// * `timestamp` - ISO 8601 timestamp (e.g., "2024-01-15T10:30:00Z")
///
/// # Returns
///
/// The date portion (e.g., "2024-01-15"), or the original string if parsing fails.
#[must_use = "returns the date string without side effects"]
pub fn extract_date_from_timestamp(timestamp: &str) -> &str {
    timestamp.split('T').next().unwrap_or(timestamp)
}

/// Extracts the version string from a git tag.
///
/// Strips the leading 'v' if present.
///
/// # Arguments
///
/// * `tag` - Git tag (e.g., "v0.1.0" or "0.1.0")
///
/// # Returns
///
/// The version string without the 'v' prefix.
#[must_use = "returns the version string without side effects"]
pub fn extract_version_from_tag(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

/// Parses platform identifier from an artifact filename.
///
/// Supports the following patterns:
/// - `infc-linux-x64.tar.gz` -> `Some("linux-x64")`
/// - `infc-windows-x64.zip` -> `Some("windows-x64")`
/// - `infc-macos-arm64.tar.gz` -> `Some("macos-arm64")`
/// - `infc-macos-apple-silicon.tar.gz` -> `Some("macos-arm64")` (normalized)
/// - `infs-linux-x64.tar.gz` -> `Some("linux-x64")` (for infs artifacts)
///
/// # Arguments
///
/// * `filename` - The artifact filename
///
/// # Returns
///
/// The platform identifier, or `None` if the filename doesn't match expected patterns.
#[must_use = "returns the platform string without side effects"]
pub fn parse_platform_from_filename(filename: &str) -> Option<&'static str> {
    let name_without_ext = filename
        .strip_suffix(".tar.gz")
        .or_else(|| filename.strip_suffix(".zip"))
        .or_else(|| filename.strip_suffix(".sha256"))?;

    let platform_part = name_without_ext
        .strip_prefix(ARTIFACT_PATTERN)
        .or_else(|| name_without_ext.strip_prefix(INFS_ARTIFACT_PATTERN))?;

    match platform_part {
        "linux-x64" => Some("linux-x64"),
        "windows-x64" => Some("windows-x64"),
        "macos-arm64" | "macos-apple-silicon" => Some("macos-arm64"),
        _ => None,
    }
}

/// Checks if filename has a valid archive extension (.tar.gz or .zip).
fn has_archive_extension(filename: &str) -> bool {
    filename.ends_with(".tar.gz")
        || std::path::Path::new(filename)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

/// Determines if an asset is a toolchain artifact (infc-*).
#[must_use = "returns boolean without side effects"]
pub fn is_toolchain_artifact(filename: &str) -> bool {
    filename.starts_with(ARTIFACT_PATTERN)
        && !filename.ends_with(".sha256")
        && has_archive_extension(filename)
}

/// Determines if an asset is a CLI artifact (infs-*).
#[must_use = "returns boolean without side effects"]
pub fn is_infs_artifact(filename: &str) -> bool {
    filename.starts_with(INFS_ARTIFACT_PATTERN)
        && !filename.ends_with(".sha256")
        && has_archive_extension(filename)
}

/// Converts a GitHub asset to a `PlatformArtifact`.
///
/// # Arguments
///
/// * `asset` - The GitHub asset to convert
///
/// # Returns
///
/// A `PlatformArtifact` with the `sha256` field set to `None` (since the GitHub
/// API does not provide checksums), or `None` if the asset's platform cannot
/// be determined.
#[must_use = "returns artifact without side effects"]
pub fn github_asset_to_platform_artifact(asset: &GitHubAsset) -> Option<PlatformArtifact> {
    let platform = parse_platform_from_filename(&asset.name)?;

    Some(PlatformArtifact {
        platform: platform.to_string(),
        url: asset.browser_download_url.clone(),
        sha256: None,
        size: asset.size,
    })
}

/// Converts a GitHub release to a `VersionInfo`.
///
/// Parses the release tag to extract the version string, extracts the date
/// from the published timestamp, and converts assets to platform artifacts.
///
/// # Arguments
///
/// * `release` - The GitHub release to convert
///
/// # Returns
///
/// A `VersionInfo` containing the release information.
#[must_use = "returns version info without side effects"]
pub fn github_release_to_version_info(release: &GitHubRelease) -> VersionInfo {
    let version = extract_version_from_tag(&release.tag_name).to_string();
    let date = extract_date_from_timestamp(&release.published_at).to_string();

    let platforms: Vec<PlatformArtifact> = release
        .assets
        .iter()
        .filter(|a| is_toolchain_artifact(&a.name))
        .filter_map(github_asset_to_platform_artifact)
        .collect();

    VersionInfo {
        version,
        date,
        prerelease: release.prerelease,
        platforms,
    }
}

/// Extracts infs CLI artifacts from a GitHub release.
///
/// # Arguments
///
/// * `release` - The GitHub release to extract from
///
/// # Returns
///
/// A vector of platform artifacts for the infs CLI binary.
#[must_use = "returns artifacts without side effects"]
pub fn extract_infs_artifacts(release: &GitHubRelease) -> Vec<PlatformArtifact> {
    release
        .assets
        .iter()
        .filter(|a| is_infs_artifact(&a.name))
        .filter_map(github_asset_to_platform_artifact)
        .collect()
}

/// Converts a list of GitHub releases to a `ReleaseManifest`.
///
/// The versions are sorted with newest first. The `latest_stable` field is set
/// to the first non-prerelease version found.
///
/// # Arguments
///
/// * `releases` - The list of GitHub releases
///
/// # Returns
///
/// A `ReleaseManifest` containing all releases.
#[must_use = "returns manifest without side effects"]
pub fn github_releases_to_manifest(releases: &[GitHubRelease]) -> ReleaseManifest {
    let mut versions: Vec<VersionInfo> = releases
        .iter()
        .map(github_release_to_version_info)
        .collect();

    versions.sort_by(|a, b| {
        let a_ver = semver::Version::parse(&a.version).ok();
        let b_ver = semver::Version::parse(&b.version).ok();
        match (b_ver, a_ver) {
            (Some(b), Some(a)) => b.cmp(&a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.version.cmp(&a.version),
        }
    });

    let latest_stable = versions
        .iter()
        .find(|v| !v.prerelease)
        .map_or_else(String::new, |v| v.version.clone());

    let infs_artifacts: Vec<PlatformArtifact> = releases
        .iter()
        .find(|r| !r.prerelease)
        .map(extract_infs_artifacts)
        .unwrap_or_default();

    let latest_infs = if infs_artifacts.is_empty() {
        None
    } else {
        Some(latest_stable.clone())
    };

    ReleaseManifest {
        schema_version: 1,
        latest_stable,
        latest_infs,
        versions,
        infs_artifacts,
    }
}

/// Fetches the release manifest, using a local cache with 15-minute TTL.
///
/// The manifest is cached at `~/.infs/cache/manifest.json`. If the cache is valid,
/// returns the cached manifest without making a network request. On cache miss or
/// expiry, fetches from GitHub Releases API and updates the cache.
///
/// # Errors
///
/// Returns an error if:
/// - The GitHub API request fails (and no valid cache exists)
/// - The response cannot be parsed as JSON
pub async fn fetch_manifest() -> Result<ReleaseManifest> {
    if let Some(manifest) = load_from_cache() {
        return Ok(manifest);
    }

    let manifest = fetch_manifest_from_network().await?;
    save_to_cache(&manifest);
    Ok(manifest)
}

/// Fetches the release manifest directly from GitHub, bypassing cache.
///
/// # Errors
///
/// Returns an error if:
/// - The GitHub API request fails
/// - The response cannot be parsed as JSON
async fn fetch_manifest_from_network() -> Result<ReleaseManifest> {
    let releases = super::github::list_releases()
        .await
        .context("Failed to fetch releases from GitHub")?;

    Ok(github_releases_to_manifest(&releases))
}

/// Fetches the SHA256 checksum for an artifact from its sidecar file.
///
/// GitHub releases typically include `.sha256` sidecar files containing checksums.
/// This function fetches `{artifact.url}.sha256` and parses the checksum.
///
/// # Arguments
///
/// * `artifact` - The artifact to fetch the checksum for
///
/// # Returns
///
/// The SHA256 checksum as a hex string.
///
/// # Errors
///
/// Returns an error if:
/// - The checksum file cannot be fetched
/// - The checksum file format is invalid
#[allow(dead_code)]
pub async fn fetch_artifact_checksum(artifact: &PlatformArtifact) -> Result<String> {
    let checksum_url = format!("{}.sha256", artifact.url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(&checksum_url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch checksum from {checksum_url}"))?;

    if !response.status().is_success() {
        bail!(
            "Failed to fetch checksum: HTTP {} from {checksum_url}",
            response.status()
        );
    }

    let text = response
        .text()
        .await
        .with_context(|| format!("Failed to read checksum response from {checksum_url}"))?;

    parse_checksum_file(&text)
        .with_context(|| format!("Failed to parse checksum from {checksum_url}"))
}

/// Parses a checksum from a `.sha256` file content.
///
/// Supports two common formats:
/// - Just the checksum: `abc123...`
/// - Checksum with filename: `abc123...  filename`
///
/// # Arguments
///
/// * `content` - The content of the checksum file
///
/// # Returns
///
/// The checksum as a hex string.
///
/// # Errors
///
/// Returns an error if the content is empty or doesn't contain a valid checksum.
fn parse_checksum_file(content: &str) -> Result<String> {
    let line = content.lines().next().context("Empty checksum file")?;
    let checksum = line
        .split_whitespace()
        .next()
        .context("No checksum found in file")?;

    if checksum.len() != 64 || !checksum.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("Invalid SHA256 checksum format: {checksum}");
    }

    Ok(checksum.to_string())
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
pub async fn fetch_artifact(version: Option<&str>, platform: Platform) -> Result<(String, PlatformArtifact)> {
    let manifest = fetch_manifest().await?;

    let version_str = match version {
        None | Some("latest") => manifest.latest_stable.clone(),
        Some(v) => v.to_string(),
    };

    let version_info = manifest
        .find_version(&version_str)
        .with_context(|| format!("Version {version_str} not found in manifest"))?;

    let artifact = version_info
        .find_artifact(platform)
        .with_context(|| format!("No artifact found for platform {platform} in version {version_str}"))?
        .clone();

    Ok((version_str, artifact))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest_json() -> &'static str {
        r#"{
            "schema_version": 1,
            "latest_stable": "0.2.0",
            "latest_infs": "0.2.0",
            "versions": [
                {
                    "version": "0.1.0",
                    "date": "2024-01-01",
                    "platforms": [
                        {
                            "platform": "linux-x64",
                            "url": "https://example.com/0.1.0/linux-x64.zip",
                            "sha256": "abc123",
                            "size": 1000
                        }
                    ]
                },
                {
                    "version": "0.2.0",
                    "date": "2024-02-01",
                    "platforms": [
                        {
                            "platform": "linux-x64",
                            "url": "https://example.com/0.2.0/linux-x64.zip",
                            "sha256": "def456",
                            "size": 2000
                        },
                        {
                            "platform": "macos-arm64",
                            "url": "https://example.com/0.2.0/macos-arm64.zip",
                            "sha256": "ghi789",
                            "size": 2100
                        }
                    ]
                }
            ],
            "infs_artifacts": [
                {
                    "platform": "linux-x64",
                    "url": "https://example.com/infs/linux-x64.zip",
                    "sha256": "infs123",
                    "size": 500
                }
            ]
        }"#
    }

    #[test]
    fn parse_manifest_json() {
        let manifest: ReleaseManifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.latest_stable, "0.2.0");
        assert_eq!(manifest.versions.len(), 2);
    }

    #[test]
    fn find_version_returns_correct_info() {
        let manifest: ReleaseManifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let version = manifest.find_version("0.1.0").expect("Should find version");
        assert_eq!(version.version, "0.1.0");
        assert_eq!(version.date, "2024-01-01");
    }

    #[test]
    fn find_version_returns_none_for_missing() {
        let manifest: ReleaseManifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        assert!(manifest.find_version("9.9.9").is_none());
    }

    #[test]
    fn latest_stable_version_returns_correct_info() {
        let manifest: ReleaseManifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let version = manifest
            .latest_stable_version()
            .expect("Should find latest stable");
        assert_eq!(version.version, "0.2.0");
    }

    #[test]
    fn find_artifact_returns_correct_platform() {
        let manifest: ReleaseManifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let version = manifest.find_version("0.2.0").expect("Should find version");
        let artifact = version
            .find_artifact(Platform::LinuxX64)
            .expect("Should find artifact");

        assert_eq!(artifact.platform, "linux-x64");
        assert_eq!(artifact.sha256, Some("def456".to_string()));
    }

    #[test]
    fn find_artifact_returns_none_for_missing_platform() {
        let manifest: ReleaseManifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let version = manifest.find_version("0.1.0").expect("Should find version");
        assert!(version.find_artifact(Platform::WindowsX64).is_none());
    }

    #[test]
    fn available_versions_returns_all() {
        let manifest: ReleaseManifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let versions = manifest.available_versions();
        assert_eq!(versions, vec!["0.1.0", "0.2.0"]);
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
        let manifest: ReleaseManifest =
            serde_json::from_str(sample_manifest_json()).expect("Should parse manifest");

        let cached = CachedManifest {
            manifest: manifest.clone(),
            timestamp: 1_000_000,
        };

        let json = serde_json::to_string(&cached).expect("Should serialize");
        let deserialized: CachedManifest =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(deserialized.timestamp, 1_000_000);
        assert_eq!(deserialized.manifest.latest_stable, manifest.latest_stable);
    }

    #[test]
    fn extract_date_from_iso8601_timestamp() {
        assert_eq!(
            extract_date_from_timestamp("2024-01-15T10:30:00Z"),
            "2024-01-15"
        );
        assert_eq!(
            extract_date_from_timestamp("2024-02-20T15:00:00Z"),
            "2024-02-20"
        );
    }

    #[test]
    fn extract_date_handles_date_only() {
        assert_eq!(extract_date_from_timestamp("2024-01-15"), "2024-01-15");
    }

    #[test]
    fn extract_version_strips_v_prefix() {
        assert_eq!(extract_version_from_tag("v0.1.0"), "0.1.0");
        assert_eq!(extract_version_from_tag("v1.2.3-alpha"), "1.2.3-alpha");
    }

    #[test]
    fn extract_version_keeps_version_without_v() {
        assert_eq!(extract_version_from_tag("0.1.0"), "0.1.0");
        assert_eq!(extract_version_from_tag("1.0.0+build"), "1.0.0+build");
    }

    #[test]
    fn parse_platform_linux_x64_tar_gz() {
        assert_eq!(
            parse_platform_from_filename("infc-linux-x64.tar.gz"),
            Some("linux-x64")
        );
    }

    #[test]
    fn parse_platform_windows_x64_zip() {
        assert_eq!(
            parse_platform_from_filename("infc-windows-x64.zip"),
            Some("windows-x64")
        );
    }

    #[test]
    fn parse_platform_macos_arm64_tar_gz() {
        assert_eq!(
            parse_platform_from_filename("infc-macos-arm64.tar.gz"),
            Some("macos-arm64")
        );
    }

    #[test]
    fn parse_platform_macos_apple_silicon_normalized() {
        assert_eq!(
            parse_platform_from_filename("infc-macos-apple-silicon.tar.gz"),
            Some("macos-arm64")
        );
    }

    #[test]
    fn parse_platform_infs_artifacts() {
        assert_eq!(
            parse_platform_from_filename("infs-linux-x64.tar.gz"),
            Some("linux-x64")
        );
        assert_eq!(
            parse_platform_from_filename("infs-windows-x64.zip"),
            Some("windows-x64")
        );
    }

    #[test]
    fn parse_platform_unknown_returns_none() {
        assert_eq!(
            parse_platform_from_filename("infc-freebsd-x64.tar.gz"),
            None
        );
        assert_eq!(parse_platform_from_filename("random-file.zip"), None);
    }

    #[test]
    fn parse_platform_sha256_returns_none() {
        assert_eq!(
            parse_platform_from_filename("infc-linux-x64.tar.gz.sha256"),
            None
        );
    }

    #[test]
    fn is_toolchain_artifact_identifies_infc() {
        assert!(is_toolchain_artifact("infc-linux-x64.tar.gz"));
        assert!(is_toolchain_artifact("infc-windows-x64.zip"));
        assert!(!is_toolchain_artifact("infs-linux-x64.tar.gz"));
        assert!(!is_toolchain_artifact("infc-linux-x64.tar.gz.sha256"));
        assert!(!is_toolchain_artifact("README.md"));
    }

    #[test]
    fn is_infs_artifact_identifies_infs() {
        assert!(is_infs_artifact("infs-linux-x64.tar.gz"));
        assert!(is_infs_artifact("infs-windows-x64.zip"));
        assert!(!is_infs_artifact("infc-linux-x64.tar.gz"));
        assert!(!is_infs_artifact("infs-linux-x64.tar.gz.sha256"));
    }

    fn sample_github_release() -> GitHubRelease {
        GitHubRelease {
            tag_name: "v0.1.0".to_string(),
            published_at: "2024-01-15T10:30:00Z".to_string(),
            prerelease: false,
            assets: vec![
                GitHubAsset {
                    name: "infc-linux-x64.tar.gz".to_string(),
                    browser_download_url:
                        "https://github.com/Inferara/inference/releases/download/v0.1.0/infc-linux-x64.tar.gz"
                            .to_string(),
                    size: 12_345_678,
                },
                GitHubAsset {
                    name: "infc-macos-arm64.tar.gz".to_string(),
                    browser_download_url:
                        "https://github.com/Inferara/inference/releases/download/v0.1.0/infc-macos-arm64.tar.gz"
                            .to_string(),
                    size: 11_234_567,
                },
                GitHubAsset {
                    name: "infc-linux-x64.tar.gz.sha256".to_string(),
                    browser_download_url:
                        "https://github.com/Inferara/inference/releases/download/v0.1.0/infc-linux-x64.tar.gz.sha256"
                            .to_string(),
                    size: 100,
                },
            ],
        }
    }

    #[test]
    fn github_asset_to_platform_artifact_converts_correctly() {
        let asset = GitHubAsset {
            name: "infc-linux-x64.tar.gz".to_string(),
            browser_download_url: "https://example.com/file.tar.gz".to_string(),
            size: 1000,
        };

        let artifact = github_asset_to_platform_artifact(&asset).expect("Should convert");

        assert_eq!(artifact.platform, "linux-x64");
        assert_eq!(artifact.url, "https://example.com/file.tar.gz");
        assert!(artifact.sha256.is_none());
        assert_eq!(artifact.size, 1000);
    }

    #[test]
    fn github_asset_to_platform_artifact_returns_none_for_unknown() {
        let asset = GitHubAsset {
            name: "README.md".to_string(),
            browser_download_url: "https://example.com/README.md".to_string(),
            size: 100,
        };

        assert!(github_asset_to_platform_artifact(&asset).is_none());
    }

    #[test]
    fn github_release_to_version_info_converts_correctly() {
        let release = sample_github_release();
        let version_info = github_release_to_version_info(&release);

        assert_eq!(version_info.version, "0.1.0");
        assert_eq!(version_info.date, "2024-01-15");
        assert!(!version_info.prerelease);
        assert_eq!(version_info.platforms.len(), 2);
    }

    #[test]
    fn github_release_to_version_info_excludes_sha256_files() {
        let release = sample_github_release();
        let version_info = github_release_to_version_info(&release);

        for platform in &version_info.platforms {
            assert!(!platform.url.ends_with(".sha256"));
        }
    }

    #[test]
    fn github_release_to_version_info_handles_prerelease() {
        let release = GitHubRelease {
            tag_name: "v0.2.0-alpha".to_string(),
            published_at: "2024-02-01T00:00:00Z".to_string(),
            prerelease: true,
            assets: vec![],
        };

        let version_info = github_release_to_version_info(&release);
        assert!(version_info.prerelease);
        assert_eq!(version_info.version, "0.2.0-alpha");
    }

    fn sample_github_releases() -> Vec<GitHubRelease> {
        vec![
            GitHubRelease {
                tag_name: "v0.2.0".to_string(),
                published_at: "2024-02-20T15:00:00Z".to_string(),
                prerelease: false,
                assets: vec![GitHubAsset {
                    name: "infc-linux-x64.tar.gz".to_string(),
                    browser_download_url: "https://example.com/v0.2.0/infc-linux-x64.tar.gz"
                        .to_string(),
                    size: 13_000_000,
                }],
            },
            GitHubRelease {
                tag_name: "v0.2.1-alpha".to_string(),
                published_at: "2024-02-25T10:00:00Z".to_string(),
                prerelease: true,
                assets: vec![],
            },
            GitHubRelease {
                tag_name: "v0.1.0".to_string(),
                published_at: "2024-01-15T10:30:00Z".to_string(),
                prerelease: false,
                assets: vec![GitHubAsset {
                    name: "infc-linux-x64.tar.gz".to_string(),
                    browser_download_url: "https://example.com/v0.1.0/infc-linux-x64.tar.gz"
                        .to_string(),
                    size: 12_345_678,
                }],
            },
        ]
    }

    #[test]
    fn github_releases_to_manifest_creates_valid_manifest() {
        let releases = sample_github_releases();
        let manifest = github_releases_to_manifest(&releases);

        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.latest_stable, "0.2.0");
        assert_eq!(manifest.versions.len(), 3);
    }

    #[test]
    fn github_releases_to_manifest_sorts_versions_newest_first() {
        let releases = sample_github_releases();
        let manifest = github_releases_to_manifest(&releases);

        assert_eq!(manifest.versions[0].version, "0.2.1-alpha");
        assert_eq!(manifest.versions[1].version, "0.2.0");
        assert_eq!(manifest.versions[2].version, "0.1.0");
    }

    #[test]
    fn github_releases_to_manifest_sets_latest_stable_to_non_prerelease() {
        let releases = sample_github_releases();
        let manifest = github_releases_to_manifest(&releases);

        assert_eq!(manifest.latest_stable, "0.2.0");
    }

    #[test]
    fn github_releases_to_manifest_handles_all_prereleases() {
        let releases = vec![
            GitHubRelease {
                tag_name: "v0.1.0-alpha".to_string(),
                published_at: "2024-01-15T00:00:00Z".to_string(),
                prerelease: true,
                assets: vec![],
            },
            GitHubRelease {
                tag_name: "v0.1.0-beta".to_string(),
                published_at: "2024-01-20T00:00:00Z".to_string(),
                prerelease: true,
                assets: vec![],
            },
        ];

        let manifest = github_releases_to_manifest(&releases);
        assert_eq!(manifest.latest_stable, "");
    }

    #[test]
    fn extract_infs_artifacts_finds_cli_assets() {
        let release = GitHubRelease {
            tag_name: "v0.1.0".to_string(),
            published_at: "2024-01-15T00:00:00Z".to_string(),
            prerelease: false,
            assets: vec![
                GitHubAsset {
                    name: "infs-linux-x64.tar.gz".to_string(),
                    browser_download_url: "https://example.com/infs-linux-x64.tar.gz".to_string(),
                    size: 500,
                },
                GitHubAsset {
                    name: "infc-linux-x64.tar.gz".to_string(),
                    browser_download_url: "https://example.com/infc-linux-x64.tar.gz".to_string(),
                    size: 1000,
                },
            ],
        };

        let artifacts = extract_infs_artifacts(&release);
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].platform, "linux-x64");
        assert!(artifacts[0].url.contains("infs-"));
    }

    #[test]
    fn parse_checksum_file_simple_format() {
        let content = "a".repeat(64);
        let checksum = parse_checksum_file(&content).expect("Should parse");
        assert_eq!(checksum.len(), 64);
    }

    #[test]
    fn parse_checksum_file_with_filename() {
        let content = format!("{}  infc-linux-x64.tar.gz\n", "b".repeat(64));
        let checksum = parse_checksum_file(&content).expect("Should parse");
        assert_eq!(checksum.len(), 64);
    }

    #[test]
    fn parse_checksum_file_empty_fails() {
        let result = parse_checksum_file("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_checksum_file_invalid_length_fails() {
        let result = parse_checksum_file("abc123");
        assert!(result.is_err());
    }

    #[test]
    fn parse_checksum_file_invalid_chars_fails() {
        let content = format!("{}xyz", "a".repeat(61));
        let result = parse_checksum_file(&content);
        assert!(result.is_err());
    }

    #[test]
    fn version_info_prerelease_defaults_to_false() {
        let json = r#"{
            "version": "0.1.0",
            "date": "2024-01-01",
            "platforms": []
        }"#;

        let version: VersionInfo = serde_json::from_str(json).expect("Should parse");
        assert!(!version.prerelease);
    }

    #[test]
    fn constants_have_expected_values() {
        assert_eq!(ARTIFACT_PATTERN, "infc-");
        assert_eq!(INFS_ARTIFACT_PATTERN, "infs-");
        assert_eq!(CACHE_TTL_ENV, "INFS_MANIFEST_CACHE_TTL");
    }
}
