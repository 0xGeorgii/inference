#![warn(clippy::pedantic)]

//! Archive extraction utilities for the infs toolchain.
//!
//! This module provides functionality for extracting ZIP archives
//! used during toolchain and self-update installations.

use anyhow::{Context, Result};
use std::path::Path;

/// Extracts a ZIP archive to the destination directory.
///
/// Creates the destination directory if it does not exist.
/// Preserves the directory structure from the archive.
///
/// # Errors
///
/// Returns an error if:
/// - The archive cannot be opened
/// - The archive is not a valid ZIP file
/// - Directory or file creation fails
/// - File extraction fails
///
/// # Example
///
/// ```ignore
/// use crate::toolchain::extract_zip;
/// extract_zip(Path::new("archive.zip"), Path::new("output_dir"))?;
/// ```
pub fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(archive_path)
        .with_context(|| format!("Failed to open archive: {}", archive_path.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP archive: {}", archive_path.display()))?;

    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("Failed to create directory: {}", dest_dir.display()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .with_context(|| format!("Failed to read archive entry {i}"))?;

        let entry_path = entry
            .enclosed_name()
            .with_context(|| format!("Invalid entry path in archive: entry {i}"))?;

        let output_path = dest_dir.join(entry_path);

        if entry.is_dir() {
            std::fs::create_dir_all(&output_path)
                .with_context(|| format!("Failed to create directory: {}", output_path.display()))?;
        } else {
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }

            let mut outfile = std::fs::File::create(&output_path)
                .with_context(|| format!("Failed to create file: {}", output_path.display()))?;

            std::io::copy(&mut entry, &mut outfile)
                .with_context(|| format!("Failed to extract: {}", output_path.display()))?;
        }
    }

    Ok(())
}
