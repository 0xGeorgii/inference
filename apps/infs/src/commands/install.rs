#![warn(clippy::pedantic)]

//! Install command for the infs CLI.
//!
//! Downloads and installs a specific version of the Inference toolchain.
//! If no version is specified, installs the latest stable version.
//!
//! ## Usage
//!
//! ```bash
//! infs install          # Install latest stable version
//! infs install 0.1.0    # Install specific version
//! infs install latest   # Explicitly install latest stable
//! ```

use anyhow::Result;
use clap::Args;
use std::path::Path;

use crate::toolchain::paths::ToolchainMetadata;
use crate::toolchain::{
    Platform, ToolchainPaths, download_file, extract_zip, fetch_artifact, verify_checksum,
};

/// Arguments for the install command.
#[derive(Args)]
pub struct InstallArgs {
    /// Version to install (e.g., "0.1.0" or "latest").
    ///
    /// If omitted, installs the latest stable version.
    #[clap(default_value = "latest")]
    pub version: String,
}

/// Executes the install command.
///
/// # Process
///
/// 1. Detect the current platform
/// 2. Fetch the release manifest
/// 3. Find the artifact for the requested version and platform
/// 4. Download the archive with progress display
/// 5. Verify the SHA256 checksum
/// 6. Extract to the toolchains directory
/// 7. Set as default if it's the first installation
///
/// # Errors
///
/// Returns an error if:
/// - Platform detection fails
/// - Manifest fetch fails
/// - Version is not found
/// - Download fails
/// - Checksum verification fails
/// - Extraction fails
pub async fn execute(args: &InstallArgs) -> Result<()> {
    let platform = Platform::detect()?;
    let paths = ToolchainPaths::new()?;

    paths.ensure_directories()?;

    let version_arg = if args.version == "latest" {
        None
    } else {
        Some(args.version.as_str())
    };

    println!("Fetching release manifest...");
    let (version, artifact) = fetch_artifact(version_arg, platform).await?;

    if paths.is_version_installed(&version) {
        println!("Toolchain version {version} is already installed.");
        return Ok(());
    }

    println!("Installing toolchain version {version} for {platform}...");

    let archive_filename = format!("toolchain-{version}-{platform}.zip");
    let archive_path = paths.download_path(&archive_filename);

    println!("Downloading from {}...", artifact.url);
    download_file(&artifact.url, &archive_path, artifact.size).await?;

    if let Some(ref checksum) = artifact.sha256 {
        println!("Verifying checksum...");
        verify_checksum(&archive_path, checksum)?;
    } else {
        println!("Skipping checksum verification (not available from GitHub API).");
    }

    println!("Extracting...");
    let toolchain_dir = paths.toolchain_dir(&version);
    extract_zip(&archive_path, &toolchain_dir)?;

    set_executable_permissions(&toolchain_dir)?;

    let metadata = ToolchainMetadata::now();
    paths.write_metadata(&version, &metadata)?;

    let installed_versions = paths.list_installed_versions()?;
    let is_first_install = installed_versions.len() == 1 && installed_versions[0] == version;
    let current_default = paths.get_default_version()?;

    if is_first_install || current_default.is_none() {
        println!("Setting {version} as default toolchain...");
        paths.set_default_version(&version)?;
        paths.update_symlinks(&version)?;
    }

    println!("Toolchain {version} installed successfully.");

    if current_default.is_some() && current_default.as_deref() != Some(&version) {
        println!("Run 'infs default {version}' to make it the default toolchain.");
    }

    std::fs::remove_file(&archive_path).ok();

    Ok(())
}

/// Sets executable permissions on binary files (Unix only).
#[cfg(unix)]
fn set_executable_permissions(dir: &Path) -> Result<()> {
    use anyhow::Context;
    use std::os::unix::fs::PermissionsExt;

    let bin_dir = dir.join("bin");
    if !bin_dir.exists() {
        return Ok(());
    }

    let entries = std::fs::read_dir(&bin_dir)
        .with_context(|| format!("Failed to read bin directory: {}", bin_dir.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| "Failed to read directory entry")?;
        let path = entry.path();
        if path.is_file() {
            let mut perms = std::fs::metadata(&path)
                .with_context(|| format!("Failed to get metadata: {}", path.display()))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms)
                .with_context(|| format!("Failed to set permissions: {}", path.display()))?;
        }
    }

    Ok(())
}

/// Sets executable permissions (no-op on Windows).
#[cfg(windows)]
#[allow(clippy::unnecessary_wraps)]
fn set_executable_permissions(_dir: &Path) -> Result<()> {
    Ok(())
}
