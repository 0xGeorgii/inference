#![warn(clippy::pedantic)]

//! Doctor command for the infs CLI.
//!
//! Verifies the installation health of the Inference toolchain and
//! reports any issues with suggested remediation steps.
//!
//! ## Usage
//!
//! ```bash
//! infs doctor
//! ```
//!
//! ## Checks Performed
//!
//! - Platform detection
//! - Toolchain directory existence
//! - Default toolchain configuration
//! - inf-llc binary presence
//! - rust-lld binary presence
//! - libLLVM shared library (Linux only)

use anyhow::Result;

use crate::toolchain::doctor::{run_all_checks, DoctorCheckStatus};

/// Executes the doctor command.
///
/// Runs all health checks and displays the results.
///
/// # Errors
///
/// Returns an error if critical checks fail to execute (not if they report failures).
#[allow(clippy::unnecessary_wraps, clippy::unused_async)]
pub async fn execute() -> Result<()> {
    println!("Checking Inference toolchain installation...");
    println!();

    let checks = run_all_checks();

    let mut has_errors = false;
    let mut has_warnings = false;

    for check in &checks {
        let prefix = check.prefix();
        println!("  {prefix} {}: {}", check.name, check.message);
        match check.status {
            DoctorCheckStatus::Ok => {}
            DoctorCheckStatus::Warning => has_warnings = true,
            DoctorCheckStatus::Error => has_errors = true,
        }
    }

    println!();

    if has_errors {
        println!("Some checks failed. Run 'infs install' to install the toolchain.");
    } else if has_warnings {
        println!("Some warnings were found. The toolchain may work but could have issues.");
    } else {
        println!("All checks passed. The toolchain is ready to use.");
    }

    Ok(())
}
