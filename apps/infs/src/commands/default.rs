#![warn(clippy::pedantic)]

//! Default command for the infs CLI.
//!
//! Sets the default toolchain version to use for compilation.
//!
//! ## Usage
//!
//! ```bash
//! infs default 0.2.0    # Set version 0.2.0 as default
//! ```

use anyhow::{Result, bail};
use clap::Args;

use crate::toolchain::ToolchainPaths;

/// Arguments for the default command.
#[derive(Args)]
pub struct DefaultArgs {
    /// Version to set as default (e.g., "0.2.0").
    pub version: String,
}

/// Executes the default command.
///
/// # Process
///
/// 1. Verify the version is installed
/// 2. Update the default file
/// 3. Update symlinks in the bin directory
///
/// # Errors
///
/// Returns an error if:
/// - The version is not installed
/// - Symlink creation fails
#[allow(clippy::unused_async)]
pub async fn execute(args: &DefaultArgs) -> Result<()> {
    let paths = ToolchainPaths::new()?;
    let version = &args.version;

    if !paths.is_version_installed(version) {
        bail!(
            "Toolchain version {version} is not installed.\n\
             Run 'infs install {version}' to install it first."
        );
    }

    let current_default = paths.get_default_version()?;
    if current_default.as_deref() == Some(version.as_str()) {
        println!("Toolchain {version} is already the default.");
        return Ok(());
    }

    paths.set_default_version(version)?;
    paths.update_symlinks(version)?;

    println!("Default toolchain set to {version}.");

    Ok(())
}
