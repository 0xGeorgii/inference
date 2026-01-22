#![warn(clippy::pedantic)]

//! GitHub API client for the infs toolchain.
//!
//! This module provides functionality for fetching release information from
//! the GitHub Releases API, which serves as the source of truth for available
//! toolchain versions.
//!
//! ## Authentication
//!
//! The client optionally reads the `GITHUB_TOKEN` environment variable for
//! authentication. When present, this token is included in API requests,
//! which increases rate limits from 60 to 5000 requests per hour.
//!
//! ## Rate Limiting
//!
//! GitHub API has rate limits:
//! - Unauthenticated: 60 requests per hour
//! - Authenticated: 5000 requests per hour
//!
//! When rate limited, the client returns an error with a helpful message.
//!
//! ## Connection Pooling
//!
//! A shared HTTP client is used across requests to benefit from connection
//! pooling. The `GITHUB_TOKEN` is read at request time (not at client
//! initialization) to allow for dynamic token configuration.

use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// GitHub repository owner for Inference releases.
const GITHUB_REPO_OWNER: &str = "Inferara";

/// GitHub repository name for Inference releases.
const GITHUB_REPO_NAME: &str = "inference";

/// Base URL for the GitHub API.
const GITHUB_API_BASE: &str = "https://api.github.com";

/// Environment variable name for GitHub authentication token.
const GITHUB_TOKEN_ENV: &str = "GITHUB_TOKEN";

/// Environment variable to override the GitHub repository (format: "owner/repo").
pub const GITHUB_REPO_ENV: &str = "INFS_GITHUB_REPO";

/// User-Agent header value for API requests.
const USER_AGENT: &str = "infs-toolchain-manager";

/// Request timeout in seconds.
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// HTTP status code for rate limiting (forbidden).
const HTTP_FORBIDDEN: u16 = 403;

/// HTTP status code for rate limiting (too many requests).
const HTTP_TOO_MANY_REQUESTS: u16 = 429;

/// HTTP status code for not found.
const HTTP_NOT_FOUND: u16 = 404;

/// Shared HTTP client for GitHub API requests.
///
/// Using a shared client enables connection pooling for better performance.
/// Note: The `GITHUB_TOKEN` is read at request time, not at initialization,
/// allowing the token to be set or changed after the client is created.
static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(USER_AGENT),
    );

    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static("application/vnd.github+json"),
    );

    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .default_headers(headers)
        .build()
        .expect("Failed to create HTTP client")
});

/// Represents a GitHub release.
///
/// Contains metadata about a release and its associated downloadable assets.
///
/// Note: This struct and related GitHub API functions are currently used only in tests.
/// They provide infrastructure for potential future GitHub releases-based toolchain management.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    /// The git tag name for this release (e.g., "v0.1.0").
    pub tag_name: String,
    /// ISO 8601 timestamp when the release was published.
    pub published_at: String,
    /// Whether this is a prerelease version.
    pub prerelease: bool,
    /// List of downloadable assets attached to this release.
    pub assets: Vec<GitHubAsset>,
}

/// Represents a downloadable asset attached to a GitHub release.
///
/// Assets are typically platform-specific binaries or archives.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    /// The filename of the asset (e.g., "infc-linux-x64.tar.gz").
    pub name: String,
    /// Direct download URL for the asset.
    pub browser_download_url: String,
    /// Size of the asset in bytes.
    pub size: u64,
}

/// Returns the shared HTTP client reference.
///
/// The client is configured with:
/// - A timeout of 30 seconds
/// - User-Agent header: "infs-toolchain-manager"
/// - Accept header for GitHub JSON API
///
/// Note: Authentication is handled at request time via [`build_request`],
/// not at client creation time.
fn get_client() -> &'static reqwest::Client {
    &HTTP_CLIENT
}

/// Builds a request builder with optional authentication.
///
/// Reads `GITHUB_TOKEN` from the environment at request time and adds
/// an Authorization header if present.
fn build_request(url: &str) -> Result<reqwest::RequestBuilder> {
    let client = get_client();
    let mut request = client.get(url);

    if let Ok(token) = std::env::var(GITHUB_TOKEN_ENV) {
        let auth_value = format!("Bearer {token}");
        let header_value = reqwest::header::HeaderValue::from_str(&auth_value)
            .context("Invalid GITHUB_TOKEN value")?;
        request = request.header(reqwest::header::AUTHORIZATION, header_value);
    }

    Ok(request)
}

/// Returns the GitHub repository owner and name.
///
/// Checks `INFS_GITHUB_REPO` first (format: "owner/repo"), falls back to defaults.
/// Returns an error if the env var is set but has an invalid format.
fn repo_config() -> Result<(String, String)> {
    match std::env::var(GITHUB_REPO_ENV) {
        Ok(val) => {
            let parts: Vec<&str> = val.trim().splitn(2, '/').collect();
            if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                Ok((parts[0].to_string(), parts[1].to_string()))
            } else {
                bail!("Invalid {GITHUB_REPO_ENV} format. Expected 'owner/repo', got: '{val}'");
            }
        }
        Err(_) => Ok((GITHUB_REPO_OWNER.to_string(), GITHUB_REPO_NAME.to_string())),
    }
}

/// Handles HTTP errors from the GitHub API.
///
/// Provides user-friendly error messages for common failure scenarios.
fn handle_http_error(status: reqwest::StatusCode, url: &str) -> anyhow::Error {
    let code = status.as_u16();

    if code == HTTP_FORBIDDEN || code == HTTP_TOO_MANY_REQUESTS {
        return anyhow::anyhow!(
            "GitHub API rate limit exceeded. Set GITHUB_TOKEN environment variable to increase limits. \
             See: https://docs.github.com/en/rest/overview/resources-in-the-rest-api#rate-limiting"
        );
    }

    if code == HTTP_NOT_FOUND {
        return anyhow::anyhow!("Resource not found: {url}");
    }

    anyhow::anyhow!("HTTP error {code}: {url}")
}

/// Fetches all releases from the GitHub repository.
///
/// Returns a list of all releases, including prereleases. The list is ordered
/// by publication date, with the most recent releases first.
///
/// # Errors
///
/// Returns an error if:
/// - The network request fails
/// - The GitHub API returns an error (rate limiting, authentication, etc.)
/// - The response cannot be parsed as JSON
///
/// # Example
///
/// ```ignore
/// let releases = list_releases().await?;
/// for release in releases {
///     println!("{}: {} assets", release.tag_name, release.assets.len());
/// }
/// ```
#[allow(dead_code)]
pub async fn list_releases() -> Result<Vec<GitHubRelease>> {
    let (owner, repo) = repo_config()?;
    let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/releases");

    let response = build_request(&url)?
        .send()
        .await
        .with_context(|| format!("Failed to connect to GitHub API: {url}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(handle_http_error(status, &url));
    }

    let text = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from {url}"))?;

    let releases: Vec<GitHubRelease> = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse GitHub releases JSON from {url}"))?;

    Ok(releases)
}

/// Fetches a specific release by its git tag.
///
/// # Arguments
///
/// * `tag` - The git tag name (e.g., "v0.1.0")
///
/// # Errors
///
/// Returns an error if:
/// - The network request fails
/// - The release is not found (HTTP 404)
/// - The GitHub API returns an error (rate limiting, authentication, etc.)
/// - The response cannot be parsed as JSON
///
/// # Example
///
/// ```ignore
/// let release = fetch_release("v0.1.0").await?;
/// println!("Published: {}", release.published_at);
/// for asset in &release.assets {
///     println!("  {}: {} bytes", asset.name, asset.size);
/// }
/// ```
#[allow(dead_code)]
pub async fn fetch_release(tag: &str) -> Result<GitHubRelease> {
    let (owner, repo) = repo_config()?;
    let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/releases/tags/{tag}");

    let response = build_request(&url)?
        .send()
        .await
        .with_context(|| format!("Failed to connect to GitHub API: {url}"))?;

    let status = response.status();
    if !status.is_success() {
        if status.as_u16() == HTTP_NOT_FOUND {
            bail!("Release with tag '{tag}' not found");
        }
        return Err(handle_http_error(status, &url));
    }

    let text = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from {url}"))?;

    let release: GitHubRelease = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse GitHub release JSON from {url}"))?;

    Ok(release)
}

/// Fetches the latest release (excluding prereleases).
///
/// This is a convenience function that fetches all releases and returns
/// the most recent non-prerelease version.
///
/// # Errors
///
/// Returns an error if:
/// - The network request fails
/// - No stable releases exist
/// - The GitHub API returns an error
///
/// # Example
///
/// ```ignore
/// let latest = fetch_latest_release().await?;
/// println!("Latest stable: {}", latest.tag_name);
/// ```
#[allow(dead_code)]
pub async fn fetch_latest_release() -> Result<GitHubRelease> {
    let releases = list_releases().await?;

    releases
        .into_iter()
        .find(|r| !r.prerelease)
        .context("No stable releases found")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_release_json() -> &'static str {
        r#"{
            "tag_name": "v0.1.0",
            "published_at": "2024-01-15T10:30:00Z",
            "prerelease": false,
            "assets": [
                {
                    "name": "infc-linux-x64.tar.gz",
                    "browser_download_url": "https://github.com/Inferara/inference/releases/download/v0.1.0/infc-linux-x64.tar.gz",
                    "size": 12345678
                },
                {
                    "name": "infc-macos-arm64.tar.gz",
                    "browser_download_url": "https://github.com/Inferara/inference/releases/download/v0.1.0/infc-macos-arm64.tar.gz",
                    "size": 11234567
                }
            ]
        }"#
    }

    fn sample_releases_json() -> &'static str {
        r#"[
            {
                "tag_name": "v0.2.0",
                "published_at": "2024-02-20T15:00:00Z",
                "prerelease": false,
                "assets": [
                    {
                        "name": "infc-linux-x64.tar.gz",
                        "browser_download_url": "https://github.com/Inferara/inference/releases/download/v0.2.0/infc-linux-x64.tar.gz",
                        "size": 13000000
                    }
                ]
            },
            {
                "tag_name": "v0.2.1-alpha",
                "published_at": "2024-02-25T10:00:00Z",
                "prerelease": true,
                "assets": []
            },
            {
                "tag_name": "v0.1.0",
                "published_at": "2024-01-15T10:30:00Z",
                "prerelease": false,
                "assets": [
                    {
                        "name": "infc-linux-x64.tar.gz",
                        "browser_download_url": "https://github.com/Inferara/inference/releases/download/v0.1.0/infc-linux-x64.tar.gz",
                        "size": 12345678
                    }
                ]
            }
        ]"#
    }

    #[test]
    fn parse_single_release() {
        let release: GitHubRelease =
            serde_json::from_str(sample_release_json()).expect("Should parse release");

        assert_eq!(release.tag_name, "v0.1.0");
        assert_eq!(release.published_at, "2024-01-15T10:30:00Z");
        assert!(!release.prerelease);
        assert_eq!(release.assets.len(), 2);
    }

    #[test]
    fn parse_release_assets() {
        let release: GitHubRelease =
            serde_json::from_str(sample_release_json()).expect("Should parse release");

        let asset = &release.assets[0];
        assert_eq!(asset.name, "infc-linux-x64.tar.gz");
        assert!(asset.browser_download_url.contains("v0.1.0"));
        assert_eq!(asset.size, 12_345_678);
    }

    #[test]
    fn parse_multiple_releases() {
        let releases: Vec<GitHubRelease> =
            serde_json::from_str(sample_releases_json()).expect("Should parse releases");

        assert_eq!(releases.len(), 3);
        assert_eq!(releases[0].tag_name, "v0.2.0");
        assert_eq!(releases[1].tag_name, "v0.2.1-alpha");
        assert_eq!(releases[2].tag_name, "v0.1.0");
    }

    #[test]
    fn identify_prerelease() {
        let releases: Vec<GitHubRelease> =
            serde_json::from_str(sample_releases_json()).expect("Should parse releases");

        let prereleases: Vec<_> = releases.iter().filter(|r| r.prerelease).collect();
        assert_eq!(prereleases.len(), 1);
        assert_eq!(prereleases[0].tag_name, "v0.2.1-alpha");
    }

    #[test]
    fn filter_stable_releases() {
        let releases: Vec<GitHubRelease> =
            serde_json::from_str(sample_releases_json()).expect("Should parse releases");

        let stable: Vec<_> = releases.iter().filter(|r| !r.prerelease).collect();
        assert_eq!(stable.len(), 2);
        assert_eq!(stable[0].tag_name, "v0.2.0");
        assert_eq!(stable[1].tag_name, "v0.1.0");
    }

    #[test]
    fn release_is_clone() {
        let release: GitHubRelease =
            serde_json::from_str(sample_release_json()).expect("Should parse release");

        let cloned = release.clone();
        assert_eq!(cloned.tag_name, release.tag_name);
        assert_eq!(cloned.assets.len(), release.assets.len());
    }

    #[test]
    fn asset_is_clone() {
        let release: GitHubRelease =
            serde_json::from_str(sample_release_json()).expect("Should parse release");

        let asset = release.assets[0].clone();
        assert_eq!(asset.name, "infc-linux-x64.tar.gz");
    }

    #[test]
    fn release_is_debug() {
        let release: GitHubRelease =
            serde_json::from_str(sample_release_json()).expect("Should parse release");

        let debug_str = format!("{release:?}");
        assert!(debug_str.contains("v0.1.0"));
        assert!(debug_str.contains("GitHubRelease"));
    }

    #[test]
    fn asset_is_debug() {
        let release: GitHubRelease =
            serde_json::from_str(sample_release_json()).expect("Should parse release");

        let debug_str = format!("{:?}", release.assets[0]);
        assert!(debug_str.contains("infc-linux-x64.tar.gz"));
        assert!(debug_str.contains("GitHubAsset"));
    }

    #[test]
    fn constants_are_correct() {
        assert_eq!(GITHUB_REPO_OWNER, "Inferara");
        assert_eq!(GITHUB_REPO_NAME, "inference");
        assert_eq!(GITHUB_API_BASE, "https://api.github.com");
        assert_eq!(GITHUB_TOKEN_ENV, "GITHUB_TOKEN");
    }

    #[test]
    fn handle_rate_limit_error_403() {
        let status = reqwest::StatusCode::FORBIDDEN;
        let error = handle_http_error(status, "https://api.github.com/test");
        let error_msg = error.to_string();

        assert!(error_msg.contains("rate limit"));
        assert!(error_msg.contains("GITHUB_TOKEN"));
    }

    #[test]
    fn handle_rate_limit_error_429() {
        let status = reqwest::StatusCode::TOO_MANY_REQUESTS;
        let error = handle_http_error(status, "https://api.github.com/test");
        let error_msg = error.to_string();

        assert!(error_msg.contains("rate limit"));
    }

    #[test]
    fn handle_not_found_error() {
        let status = reqwest::StatusCode::NOT_FOUND;
        let url = "https://api.github.com/test";
        let error = handle_http_error(status, url);
        let error_msg = error.to_string();

        assert!(error_msg.contains("not found"));
        assert!(error_msg.contains(url));
    }

    #[test]
    fn handle_generic_http_error() {
        let status = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        let url = "https://api.github.com/test";
        let error = handle_http_error(status, url);
        let error_msg = error.to_string();

        assert!(error_msg.contains("500"));
        assert!(error_msg.contains(url));
    }

    #[test]
    fn parse_release_with_empty_assets() {
        let json = r#"{
            "tag_name": "v0.0.1",
            "published_at": "2024-01-01T00:00:00Z",
            "prerelease": true,
            "assets": []
        }"#;

        let release: GitHubRelease = serde_json::from_str(json).expect("Should parse release");
        assert!(release.assets.is_empty());
        assert!(release.prerelease);
    }

    #[test]
    fn find_first_stable_release() {
        let releases: Vec<GitHubRelease> =
            serde_json::from_str(sample_releases_json()).expect("Should parse releases");

        let first_stable = releases.into_iter().find(|r| !r.prerelease);
        assert!(first_stable.is_some());
        assert_eq!(first_stable.unwrap().tag_name, "v0.2.0");
    }

    #[test]
    #[serial_test::serial]
    fn repo_config_uses_default_when_env_not_set() {
        unsafe { std::env::remove_var(GITHUB_REPO_ENV) };
        let (owner, repo) = repo_config().unwrap();
        assert_eq!(owner, "Inferara");
        assert_eq!(repo, "inference");
    }

    #[test]
    #[serial_test::serial]
    fn repo_config_uses_env_when_set() {
        unsafe { std::env::set_var(GITHUB_REPO_ENV, "testowner/testrepo") };
        let (owner, repo) = repo_config().unwrap();
        assert_eq!(owner, "testowner");
        assert_eq!(repo, "testrepo");
        unsafe { std::env::remove_var(GITHUB_REPO_ENV) };
    }

    #[test]
    #[serial_test::serial]
    fn repo_config_fails_on_invalid_format() {
        unsafe { std::env::set_var(GITHUB_REPO_ENV, "invalid") };
        let result = repo_config();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid INFS_GITHUB_REPO format"));
        unsafe { std::env::remove_var(GITHUB_REPO_ENV) };
    }

    #[test]
    #[serial_test::serial]
    fn repo_config_fails_on_empty_owner() {
        unsafe { std::env::set_var(GITHUB_REPO_ENV, "/repo") };
        let result = repo_config();
        assert!(result.is_err());
        unsafe { std::env::remove_var(GITHUB_REPO_ENV) };
    }

    #[test]
    #[serial_test::serial]
    fn repo_config_fails_on_empty_repo() {
        unsafe { std::env::set_var(GITHUB_REPO_ENV, "owner/") };
        let result = repo_config();
        assert!(result.is_err());
        unsafe { std::env::remove_var(GITHUB_REPO_ENV) };
    }

    #[test]
    #[serial_test::serial]
    fn repo_config_trims_whitespace() {
        unsafe { std::env::set_var(GITHUB_REPO_ENV, "  owner/repo  ") };
        let (owner, repo) = repo_config().unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
        unsafe { std::env::remove_var(GITHUB_REPO_ENV) };
    }
}
