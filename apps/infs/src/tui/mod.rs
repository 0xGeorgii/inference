#![warn(clippy::pedantic)]

//! Terminal User Interface for the infs CLI.
//!
//! This module provides an interactive TUI for the Inference toolchain,
//! allowing users to navigate commands and manage projects visually.
//!
//! ## Usage
//!
//! The TUI is launched automatically when `infs` is run without arguments
//! in an interactive terminal environment.
//!
//! ## Headless Detection
//!
//! The TUI will not launch in headless environments:
//! - When `CI=true` or `CI=1` environment variable is set
//! - When `NO_COLOR` environment variable is set (any value)
//! - When stdout is not a terminal (piped or redirected)
//!
//! ## Modules
//!
//! - [`terminal`] - Terminal setup and cleanup with RAII guard
//! - [`app`] - Main application state and event loop
//! - [`state`] - Screen state machine and view states
//! - [`theme`] - Color theme system
//! - [`menu`] - Menu navigation
//! - [`views`] - Screen rendering modules
//! - [`widgets`] - Reusable widget components

pub mod app;
pub mod install_task;
pub mod menu;
pub mod state;
pub mod terminal;
pub mod theme;
pub mod views;
pub mod widgets;

use std::io::IsTerminal;

use anyhow::{Context, Result};

use terminal::TerminalGuard;

/// Determines whether the TUI should be used based on environment.
///
/// Returns `false` in headless environments:
/// - `CI=true` or `CI=1` environment variable
/// - `NO_COLOR` environment variable (any value)
/// - Non-TTY stdout (piped or redirected)
#[must_use]
pub fn should_use_tui() -> bool {
    if let Ok(ci) = std::env::var("CI") {
        let ci_lower = ci.to_lowercase();
        if ci_lower == "true" || ci_lower == "1" {
            return false;
        }
    }

    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    std::io::stdout().is_terminal()
}

/// Runs the TUI application.
///
/// This function sets up the terminal, runs the main event loop,
/// and ensures proper cleanup on exit or error.
///
/// If the TUI exits with a pending command (e.g., `build`, `run`, `verify`),
/// this function restores the terminal, executes the command, waits for user
/// to press Enter, and then restarts the TUI.
///
/// # Errors
///
/// Returns an error if:
/// - Terminal setup fails
/// - Event handling fails
/// - Drawing fails
/// - Command execution fails
pub fn run() -> Result<()> {
    loop {
        let pending_command = {
            let mut guard = TerminalGuard::new().context("failed to initialize terminal")?;
            app::run_app(&mut guard).context("TUI application error")?
            // Guard is dropped here, restoring terminal
        };

        match pending_command {
            Some(command) => {
                execute_pending_command(&command)?;
                wait_for_enter();
                // Loop continues, restarting TUI
            }
            None => {
                // No pending command, exit normally
                break;
            }
        }
    }

    Ok(())
}

/// Executes a pending command after the TUI has exited.
fn execute_pending_command(command: &str) -> Result<()> {
    let exe = std::env::current_exe().context("failed to get current executable")?;

    println!();
    let status = std::process::Command::new(&exe)
        .arg(command)
        .status()
        .with_context(|| format!("failed to execute 'infs {command}'"))?;

    if !status.success() {
        // Log failure but don't exit - we'll return to TUI
        eprintln!(
            "\nCommand 'infs {command}' exited with status: {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

/// Waits for the user to press Enter before returning to the TUI.
fn wait_for_enter() {
    use std::io::{BufRead, Write};

    print!("\nPress Enter to return to TUI...");
    let _ = std::io::stdout().flush();

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();
    let _ = handle.read_line(&mut buffer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_use_tui_returns_bool() {
        // This test verifies the function can be called and returns a boolean.
        // The actual return value depends on the environment.
        let result = should_use_tui();
        // In CI, this should be false (CI=true is typically set)
        // We can't assert a specific value since test environments vary
        let _ = result;
    }

    #[test]
    fn should_use_tui_respects_ci_env() {
        // Save original value
        let original = std::env::var("CI").ok();

        // SAFETY: Tests run single-threaded with #[test], so setting env vars is safe
        // for the duration of this test.
        unsafe {
            std::env::set_var("CI", "true");
        }
        assert!(!should_use_tui());

        unsafe {
            std::env::set_var("CI", "1");
        }
        assert!(!should_use_tui());

        unsafe {
            std::env::set_var("CI", "TRUE");
        }
        assert!(!should_use_tui());

        // Restore original
        unsafe {
            match original {
                Some(val) => std::env::set_var("CI", val),
                None => std::env::remove_var("CI"),
            }
        }
    }

    #[test]
    fn should_use_tui_respects_no_color_env() {
        // Save original value
        let original = std::env::var("NO_COLOR").ok();

        // SAFETY: Tests run single-threaded with #[test], so setting env vars is safe
        // for the duration of this test.
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        assert!(!should_use_tui());

        unsafe {
            std::env::set_var("NO_COLOR", "");
        }
        assert!(!should_use_tui());

        // Restore original
        unsafe {
            match original {
                Some(val) => std::env::set_var("NO_COLOR", val),
                None => std::env::remove_var("NO_COLOR"),
            }
        }
    }
}
