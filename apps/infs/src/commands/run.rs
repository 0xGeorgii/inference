#![warn(clippy::pedantic)]

//! Run command for the infs CLI.
//!
//! Compiles Inference source files and executes the resulting WASM
//! using wasmtime in a single step.
//!
//! ## Execution Pipeline
//!
//! 1. **Validate** - Check source file exists
//! 2. **Check** - Verify wasmtime is available in PATH
//! 3. **Compile** - Parse, type check, analyze, and generate WASM
//! 4. **Write** - Persist WASM binary to ./out/ directory
//! 5. **Execute** - Run WASM with wasmtime, passing arguments
//!
//! ## Prerequisites
//!
//! This command requires wasmtime (a WebAssembly runtime) to be
//! installed and available in PATH.

use anyhow::{Context, Result, bail};
use clap::Args;
use inference::{analyze, codegen, parse, type_check};
use std::{fs, path::{Path, PathBuf}, process::Command};

use crate::errors::InfsError;

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
/// 3. Compiles source to WASM
/// 4. Writes WASM to ./out/ directory
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
/// - Any compilation phase fails
/// - WASM file writing fails
/// - wasmtime execution fails to start
/// - wasmtime exits with non-zero code (as `InfsError::ProcessExitCode`)
pub fn execute(args: &RunArgs) -> Result<()> {
    if !args.path.exists() {
        bail!("Path not found: {}", args.path.display());
    }

    check_wasmtime_availability()?;

    let wasm = compile_to_wasm(&args.path)?;

    let source_fname = args
        .path
        .file_stem()
        .unwrap_or_else(|| std::ffi::OsStr::new("module"))
        .to_str()
        .unwrap_or("module");

    let wasm_path = write_wasm_output(source_fname, &wasm)?;

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

/// Compiles source file to WASM binary.
fn compile_to_wasm(source_path: &Path) -> Result<Vec<u8>> {
    let source_code = fs::read_to_string(source_path)
        .with_context(|| format!("Failed to read source file: {}", source_path.display()))?;

    let arena = parse(source_code.as_str())
        .with_context(|| format!("Parse error in {}", source_path.display()))?;
    println!("Parsed: {}", source_path.display());

    let typed_context = type_check(arena)
        .with_context(|| format!("Type checking failed for {}", source_path.display()))?;

    analyze(&typed_context)
        .with_context(|| format!("Analysis failed for {}", source_path.display()))?;
    println!("Analyzed: {}", source_path.display());

    let wasm = codegen(&typed_context)
        .with_context(|| format!("Codegen failed for {}", source_path.display()))?;
    println!("WASM generated");

    Ok(wasm)
}

/// Writes WASM binary to the output directory.
fn write_wasm_output(source_fname: &str, wasm: &[u8]) -> Result<PathBuf> {
    let output_dir = PathBuf::from("out");
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

    let wasm_path = output_dir.join(format!("{source_fname}.wasm"));
    fs::write(&wasm_path, wasm)
        .with_context(|| format!("Failed to write WASM file: {}", wasm_path.display()))?;

    println!("WASM written to: {}", wasm_path.display());
    Ok(wasm_path)
}

/// Runs wasmtime with the given WASM file and arguments.
///
/// Returns `Ok(())` on success, or `Err(InfsError::ProcessExitCode)` if wasmtime
/// exits with a non-zero code. This allows the caller to propagate the exit code
/// without bypassing RAII cleanup.
fn run_wasmtime(wasm_path: &Path, args: &[String]) -> Result<()> {
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
