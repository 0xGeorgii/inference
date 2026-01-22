#![warn(clippy::pedantic)]

//! Run command for the infs CLI.
//!
//! Compiles Inference source files and executes the resulting WASM
//! using wasmtime in a single step. This module delegates compilation
//! to the `infc` compiler via subprocess.
//!
//! ## Execution Pipeline
//!
//! 1. **Validate** - Check source file exists
//! 2. **Check** - Verify wasmtime is available in PATH
//! 3. **Locate** - Find the infc compiler binary
//! 4. **Compile** - Call infc with `--parse --codegen -o` to generate WASM
//! 5. **Execute** - Run WASM with wasmtime, passing arguments
//!
//! ## Prerequisites
//!
//! This command requires:
//! - `infc` compiler (via toolchain or PATH)
//! - `wasmtime` WebAssembly runtime (in PATH)

use anyhow::{Context, Result, bail};
use clap::Args;
use std::path::PathBuf;
use std::process::Command;

use crate::errors::InfsError;
use crate::toolchain::find_infc;

/// Arguments for the run command.
///
/// The run command compiles source to WASM and executes it with wasmtime.
/// Any arguments after the source path are passed to the WASM program.
#[derive(Args)]
pub struct RunArgs {
    /// Path to the source file to run.
    pub path: PathBuf,

    /// Arguments to pass to the WASM program.
    #[clap(trailing_var_arg = true)]
    pub args: Vec<String>,
}

/// Executes the run command with the given arguments.
///
/// ## Execution Flow
///
/// 1. Validates source file exists
/// 2. Checks for wasmtime availability
/// 3. Locates the infc compiler
/// 4. Compiles source to WASM via infc subprocess
/// 5. Executes WASM with wasmtime
/// 6. Propagates exit code from wasmtime
///
/// ## Exit Codes
///
/// - Returns `Ok(())` if wasmtime succeeds (exit code 0)
/// - Returns `Err(InfsError::ProcessExitCode)` if wasmtime exits with non-zero code
/// - Returns `Err` with other variants if compilation fails
///
/// ## Errors
///
/// Returns an error if:
/// - The source file does not exist
/// - wasmtime is not found in PATH
/// - infc compiler cannot be found
/// - Compilation fails
/// - WASM execution fails
pub fn execute(args: &RunArgs) -> Result<()> {
    if !args.path.exists() {
        bail!("Path not found: {}", args.path.display());
    }

    check_wasmtime_availability()?;

    let infc_path = find_infc()?;

    let wasm_path = compile_to_wasm(&infc_path, &args.path)?;

    run_wasmtime(&wasm_path, &args.args)
}

/// Checks if wasmtime is available in PATH.
fn check_wasmtime_availability() -> Result<()> {
    if which::which("wasmtime").is_err() {
        bail!(
            "wasmtime not found in PATH.\n\n\
            wasmtime is a WebAssembly runtime. To install:\n  \
            - macOS: brew install wasmtime\n  \
            - Linux: curl https://wasmtime.dev/install.sh -sSf | bash\n  \
            - Windows: winget install wasmtime\n  \
            - Or visit: https://wasmtime.dev/"
        );
    }
    Ok(())
}

/// Compiles source file to WASM binary using infc subprocess.
///
/// Calls infc with `--parse --codegen -o` flags to generate the WASM file
/// in the `out/` directory.
fn compile_to_wasm(infc_path: &PathBuf, source_path: &PathBuf) -> Result<PathBuf> {
    let mut cmd = Command::new(infc_path);
    cmd.arg(source_path)
        .arg("--parse")
        .arg("--codegen")
        .arg("-o");

    let status = cmd
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to execute infc at {}", infc_path.display()))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        return Err(InfsError::process_exit_code(code).into());
    }

    let source_fname = source_path
        .file_stem()
        .unwrap_or_else(|| std::ffi::OsStr::new("module"))
        .to_str()
        .unwrap_or("module");

    let wasm_path = PathBuf::from("out").join(format!("{source_fname}.wasm"));

    if !wasm_path.exists() {
        bail!(
            "Compilation succeeded but WASM file not found at: {}",
            wasm_path.display()
        );
    }

    Ok(wasm_path)
}

/// Runs wasmtime with the given WASM file and arguments.
///
/// Returns `Ok(())` on success, or `Err(InfsError::ProcessExitCode)` if wasmtime
/// exits with a non-zero code. This allows the caller to propagate the exit code
/// without bypassing RAII cleanup.
fn run_wasmtime(wasm_path: &PathBuf, args: &[String]) -> Result<()> {
    println!("Running with wasmtime...");

    let mut cmd = Command::new("wasmtime");
    cmd.arg(wasm_path);
    for arg in args {
        cmd.arg(arg);
    }

    let status = cmd
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| "Failed to execute wasmtime")?;

    if status.success() {
        Ok(())
    } else {
        let code = status.code().unwrap_or(1);
        Err(InfsError::process_exit_code(code).into())
    }
}
