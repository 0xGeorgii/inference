#![warn(clippy::pedantic)]

//! HTTP download functionality for the infs toolchain.
//!
//! This module provides async file downloads with progress tracking,
//! retry logic, and temporary file handling.
//!
//! ## Features
//!
//! - Streaming downloads with progress callbacks
//! - Automatic retry with exponential backoff (3 attempts)
//! - Downloads to temporary file, then renames on success
//! - Configurable timeout per request
//!
//! ## TUI Integration
//!
//! For TUI integration, use [`download_file_with_callback`] which reports
//! progress via a callback instead of using indicatif progress bars.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use rand::Rng;
use tokio::io::AsyncWriteExt;

/// Progress event emitted during downloads.
///
/// Used by [`download_file_with_callback`] to report progress to TUI or other consumers.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ProgressEvent {
    /// Download has started.
    Started {
        /// The URL being downloaded.
        url: String,
        /// Total file size in bytes.
        total: u64,
    },
    /// Download progress update.
    Progress {
        /// Bytes downloaded so far.
        downloaded: u64,
        /// Current download speed in bytes per second.
        speed: u64,
    },
    /// Download completed successfully.
    Completed,
    /// Download failed with an error.
    Failed {
        /// Error description.
        error: String,
    },
}

/// Callback type for receiving progress updates during downloads.
///
/// The callback is invoked on each progress event. It is wrapped in `Arc`
/// to allow sharing across async boundaries.
#[allow(dead_code)]
pub type ProgressCallback = Arc<dyn Fn(ProgressEvent) + Send + Sync>;

/// Maximum number of download retry attempts.
const MAX_RETRIES: u32 = 3;

/// Base delay between retries in milliseconds.
const BASE_RETRY_DELAY_MS: u64 = 1000;

/// Request timeout in seconds.
const REQUEST_TIMEOUT_SECS: u64 = 300;

/// Downloads a file from the given URL to the specified path with progress display.
///
/// The download uses streaming to avoid loading the entire file into memory.
/// Progress is displayed using an `indicatif` progress bar.
///
/// # Arguments
///
/// * `url` - The URL to download from
/// * `dest` - The destination file path
/// * `expected_size` - Expected file size in bytes for progress display
///
/// # Errors
///
/// Returns an error if:
/// - The network request fails after all retries
/// - The destination file cannot be created
/// - Writing to the file fails
pub async fn download_file(url: &str, dest: &Path, expected_size: u64) -> Result<()> {
    let temp_path = dest.with_extension("tmp");

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let delay = calculate_retry_delay(attempt);
            println!(
                "Retrying download (attempt {}/{})...",
                attempt + 1,
                MAX_RETRIES
            );
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }

        match download_with_progress(url, &temp_path, expected_size).await {
            Ok(()) => {
                tokio::fs::rename(&temp_path, dest).await.with_context(|| {
                    format!(
                        "Failed to rename {} to {}",
                        temp_path.display(),
                        dest.display()
                    )
                })?;
                return Ok(());
            }
            Err(e) => {
                last_error = Some(e);
                let _ = tokio::fs::remove_file(&temp_path).await;
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("Download failed after {MAX_RETRIES} attempts")))
}

/// Downloads a file with progress bar display.
async fn download_with_progress(url: &str, dest: &Path, expected_size: u64) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to {url}"))?;

    if !response.status().is_success() {
        bail!("HTTP error {}: {url}", response.status());
    }

    let total_size = response.content_length().unwrap_or(expected_size);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("Progress bar template should be valid")
            .progress_chars("#>-"),
    );

    let mut file = tokio::fs::File::create(dest)
        .await
        .with_context(|| format!("Failed to create file: {}", dest.display()))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("Failed to read chunk from {url}"))?;
        file.write_all(&chunk)
            .await
            .with_context(|| format!("Failed to write to {}", dest.display()))?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    file.flush()
        .await
        .with_context(|| format!("Failed to flush {}", dest.display()))?;

    pb.finish_with_message("Download complete");

    Ok(())
}

/// Calculates the retry delay with exponential backoff and jitter.
///
/// The delay doubles with each attempt (1s, 2s, 4s) with +/- 25% jitter.
fn calculate_retry_delay(attempt: u32) -> u64 {
    let base_delay = BASE_RETRY_DELAY_MS * 2u64.pow(attempt);
    let jitter_range = base_delay / 4;
    let jitter = rand::rng().random_range(0..=jitter_range * 2);
    base_delay - jitter_range + jitter
}

/// Downloads a file without progress display (for smaller files).
///
/// # Errors
///
/// Returns an error if the download or file writing fails.
#[allow(dead_code)]
pub async fn download_file_simple(url: &str, dest: &Path) -> Result<()> {
    let temp_path = dest.with_extension("tmp");

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to {url}"))?;

    if !response.status().is_success() {
        bail!("HTTP error {}: {url}", response.status());
    }

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("Failed to download from {url}"))?;

    tokio::fs::write(&temp_path, &bytes)
        .await
        .with_context(|| format!("Failed to write to {}", temp_path.display()))?;

    tokio::fs::rename(&temp_path, dest).await.with_context(|| {
        format!(
            "Failed to rename {} to {}",
            temp_path.display(),
            dest.display()
        )
    })?;

    Ok(())
}

/// Minimum interval between progress callback invocations in milliseconds.
const PROGRESS_CALLBACK_INTERVAL_MS: u128 = 100;

/// Downloads a file with progress callbacks for TUI integration.
///
/// Unlike [`download_file`], this function reports progress via a callback
/// instead of using an indicatif progress bar. This allows integration with
/// custom progress displays like the TUI.
///
/// # Arguments
///
/// * `url` - The URL to download from
/// * `dest` - The destination file path
/// * `expected_size` - Expected file size in bytes for progress display
/// * `callback` - Progress callback that receives [`ProgressEvent`]s
///
/// # Errors
///
/// Returns an error if:
/// - The network request fails after all retries
/// - The destination file cannot be created
/// - Writing to the file fails
#[allow(dead_code)]
pub async fn download_file_with_callback(
    url: &str,
    dest: &Path,
    expected_size: u64,
    callback: ProgressCallback,
) -> Result<()> {
    let temp_path = dest.with_extension("tmp");

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let delay = calculate_retry_delay(attempt);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }

        match download_with_callback(url, &temp_path, expected_size, callback.clone()).await {
            Ok(()) => {
                tokio::fs::rename(&temp_path, dest).await.with_context(|| {
                    format!(
                        "Failed to rename {} to {}",
                        temp_path.display(),
                        dest.display()
                    )
                })?;
                callback(ProgressEvent::Completed);
                return Ok(());
            }
            Err(e) => {
                let _ = tokio::fs::remove_file(&temp_path).await;
                last_error = Some(e);
            }
        }
    }

    let error = last_error
        .unwrap_or_else(|| anyhow::anyhow!("Download failed after {MAX_RETRIES} attempts"));
    callback(ProgressEvent::Failed {
        error: error.to_string(),
    });
    Err(error)
}

/// Downloads a file with callback-based progress reporting.
async fn download_with_callback(
    url: &str,
    dest: &Path,
    expected_size: u64,
    callback: ProgressCallback,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to connect to {url}"))?;

    if !response.status().is_success() {
        bail!("HTTP error {}: {url}", response.status());
    }

    let total_size = response.content_length().unwrap_or(expected_size);

    callback(ProgressEvent::Started {
        url: url.to_string(),
        total: total_size,
    });

    let mut file = tokio::fs::File::create(dest)
        .await
        .with_context(|| format!("Failed to create file: {}", dest.display()))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let start_time = Instant::now();
    let mut last_callback_time = Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.with_context(|| format!("Failed to read chunk from {url}"))?;
        file.write_all(&chunk)
            .await
            .with_context(|| format!("Failed to write to {}", dest.display()))?;

        downloaded += chunk.len() as u64;

        let now = Instant::now();
        let elapsed_since_callback = now.duration_since(last_callback_time).as_millis();

        if elapsed_since_callback >= PROGRESS_CALLBACK_INTERVAL_MS {
            let elapsed_secs = start_time.elapsed().as_secs_f64();
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let speed = if elapsed_secs > 0.0 {
                (downloaded as f64 / elapsed_secs) as u64
            } else {
                0
            };

            callback(ProgressEvent::Progress { downloaded, speed });
            last_callback_time = now;
        }
    }

    file.flush()
        .await
        .with_context(|| format!("Failed to flush {}", dest.display()))?;

    let elapsed_secs = start_time.elapsed().as_secs_f64();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let final_speed = if elapsed_secs > 0.0 {
        (downloaded as f64 / elapsed_secs) as u64
    } else {
        0
    };
    callback(ProgressEvent::Progress {
        downloaded,
        speed: final_speed,
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_increases_exponentially() {
        let delay_0 = calculate_retry_delay(0);
        let delay_1 = calculate_retry_delay(1);
        let delay_2 = calculate_retry_delay(2);

        assert!(
            (750..=1250).contains(&delay_0),
            "Attempt 0 delay should be ~1000ms with jitter"
        );
        assert!(
            (1500..=2500).contains(&delay_1),
            "Attempt 1 delay should be ~2000ms with jitter"
        );
        assert!(
            (3000..=5000).contains(&delay_2),
            "Attempt 2 delay should be ~4000ms with jitter"
        );
    }

    #[test]
    fn progress_event_started_contains_url_and_total() {
        let event = ProgressEvent::Started {
            url: "https://example.com/file.zip".to_string(),
            total: 1024,
        };
        match event {
            ProgressEvent::Started { url, total } => {
                assert_eq!(url, "https://example.com/file.zip");
                assert_eq!(total, 1024);
            }
            _ => panic!("Expected Started variant"),
        }
    }

    #[test]
    fn progress_event_progress_contains_downloaded_and_speed() {
        let event = ProgressEvent::Progress {
            downloaded: 512,
            speed: 1024 * 100,
        };
        match event {
            ProgressEvent::Progress { downloaded, speed } => {
                assert_eq!(downloaded, 512);
                assert_eq!(speed, 1024 * 100);
            }
            _ => panic!("Expected Progress variant"),
        }
    }

    #[test]
    fn progress_event_completed_variant() {
        let event = ProgressEvent::Completed;
        assert!(matches!(event, ProgressEvent::Completed));
    }

    #[test]
    fn progress_event_failed_contains_error() {
        let event = ProgressEvent::Failed {
            error: "Connection refused".to_string(),
        };
        match event {
            ProgressEvent::Failed { error } => {
                assert_eq!(error, "Connection refused");
            }
            _ => panic!("Expected Failed variant"),
        }
    }

    #[test]
    fn progress_event_is_clone() {
        let event = ProgressEvent::Progress {
            downloaded: 1000,
            speed: 500,
        };
        let cloned = event.clone();
        match cloned {
            ProgressEvent::Progress { downloaded, speed } => {
                assert_eq!(downloaded, 1000);
                assert_eq!(speed, 500);
            }
            _ => panic!("Expected Progress variant"),
        }
    }

    #[test]
    fn progress_event_is_debug() {
        let event = ProgressEvent::Started {
            url: "test".to_string(),
            total: 100,
        };
        let debug_str = format!("{event:?}");
        assert!(debug_str.contains("Started"));
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("100"));
    }
}
