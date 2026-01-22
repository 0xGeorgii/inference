#![warn(clippy::pedantic)]

//! Verify command for the infs CLI.
//!
//! Compiles Inference source files to WASM, translates to Rocq (.v),
//! and runs coqc to verify the generated proofs. This module delegates
//! compilation to the `infc` compiler via subprocess.
//!
//! ## Verification Pipeline
//!
//! 1. **Locate** - Find the infc compiler binary
//! 2. **Compile** - Call infc with `--parse --codegen -v` to generate .v file in `out/`
//! 3. **Copy** - Move the .v file from `out/` to the user's output directory
//! 4. **Verify** - Run coqc on the generated .v file
//!
//! ## Prerequisites
//!
//! This command requires:
//! - `infc` compiler (via toolchain or PATH)
//! - `coqc` (the Rocq/Coq proof assistant) in PATH

use anyhow::{Context, Result, bail};
use clap::Args;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

use crate::errors::InfsError;
use crate::toolchain::find_infc;

/// Arguments for the verify command.
///
/// The verify command compiles source to WASM, translates to Rocq,
/// and runs coqc for proof verification.
#[derive(Args)]
pub struct VerifyArgs {
    /// Path to the source file to verify.
    pub path: PathBuf,

    /// Output directory for generated proofs (defaults to "proofs/").
    #[clap(long = "output-dir", default_value = "proofs")]
    pub output_dir: PathBuf,

    /// Skip compilation if .v file already exists and is newer than source.
    #[clap(long = "skip-compile", action = clap::ArgAction::SetTrue)]
    pub skip_compile: bool,
}

/// Executes the verify command with the given arguments.
///
/// ## Execution Flow
///
/// 1. Validates source file exists
/// 2. Checks for coqc availability
/// 3. Locates the infc compiler
/// 4. Compiles source to Rocq (.v) via infc subprocess (unless skip-compile is set and .v is fresh)
/// 5. Copies the .v file from out/ to the user's output directory
/// 6. Runs coqc verification
/// 7. Reports results
///
/// ## Errors
///
/// Returns an error if:
/// - The source file does not exist
/// - coqc is not found in PATH
/// - infc compiler cannot be found
/// - Any compilation phase fails
/// - coqc verification fails
pub fn execute(args: &VerifyArgs) -> Result<()> {
    if !args.path.exists() {
        bail!("Path not found: {}", args.path.display());
    }

    check_coqc_availability()?;

    let source_fname = args
        .path
        .file_stem()
        .unwrap_or_else(|| std::ffi::OsStr::new("module"))
        .to_str()
        .unwrap_or("module");

    let v_file_path = args.output_dir.join(format!("{source_fname}.v"));

    let should_compile = if args.skip_compile && v_file_path.exists() {
        is_source_newer_than_v(&args.path, &v_file_path)?
    } else {
        true
    };

    if should_compile {
        let infc_path = find_infc()?;
        compile_to_rocq(&infc_path, &args.path)?;

        let out_v_path = PathBuf::from("out").join(format!("{source_fname}.v"));
        copy_v_to_output_dir(&out_v_path, &args.output_dir, &v_file_path)?;
    } else {
        println!("Skipping compilation: {} is up to date", v_file_path.display());
    }

    run_coqc_verification(&v_file_path, source_fname)?;

    Ok(())
}

/// Checks if coqc is available in PATH.
fn check_coqc_availability() -> Result<()> {
    if which::which("coqc").is_err() {
        bail!(
            "coqc not found in PATH.\n\n\
            coqc is part of the Rocq/Coq proof assistant. To install:\n  \
            - Ubuntu/Debian: sudo apt install coq\n  \
            - macOS: brew install coq\n  \
            - Windows: See https://coq.inria.fr/download\n  \
            - Or use opam: opam install coq"
        );
    }
    Ok(())
}

/// Checks if the source file is newer than the .v file.
fn is_source_newer_than_v(source: &Path, v_file: &Path) -> Result<bool> {
    let source_modified = fs::metadata(source)
        .with_context(|| format!("Failed to get metadata for source file: {}", source.display()))?
        .modified()
        .with_context(|| format!("Failed to get modification time for: {}", source.display()))?;

    let v_modified = fs::metadata(v_file)
        .with_context(|| format!("Failed to get metadata for .v file: {}", v_file.display()))?
        .modified()
        .with_context(|| format!("Failed to get modification time for: {}", v_file.display()))?;

    Ok(source_modified > v_modified)
}

/// Compiles source file to Rocq (.v) format using infc subprocess.
///
/// Calls infc with `--parse --codegen -v` flags to generate the .v file
/// in the `out/` directory.
fn compile_to_rocq(infc_path: &PathBuf, source_path: &PathBuf) -> Result<()> {
    let mut cmd = Command::new(infc_path);
    cmd.arg(source_path)
        .arg("--parse")
        .arg("--codegen")
        .arg("-v");

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

    Ok(())
}

/// Copies the .v file from out/ to the user's output directory.
fn copy_v_to_output_dir(
    out_v_path: &Path,
    output_dir: &Path,
    dest_v_path: &Path,
) -> Result<()> {
    if !out_v_path.exists() {
        bail!(
            "Compilation succeeded but .v file not found at: {}",
            out_v_path.display()
        );
    }

    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

    fs::copy(out_v_path, dest_v_path)
        .with_context(|| format!(
            "Failed to copy .v file from {} to {}",
            out_v_path.display(),
            dest_v_path.display()
        ))?;

    println!("Rocq file written to: {}", dest_v_path.display());
    Ok(())
}

/// Runs coqc verification on the .v file.
fn run_coqc_verification(v_file_path: &Path, source_fname: &str) -> Result<()> {
    println!("Running coqc verification...");

    let output = Command::new("coqc")
        .arg(v_file_path)
        .output()
        .with_context(|| "Failed to execute coqc")?;

    if output.status.success() {
        println!("Verification passed: {source_fname}");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);

        copy_v_for_debugging(v_file_path)?;

        bail!(
            "Verification failed for {source_fname}.\n\n\
            coqc output:\n{stderr}\n\n\
            The .v file has been copied to ./out/ for debugging."
        );
    }
}

/// Copies the .v file to ./out/ for debugging when verification fails.
fn copy_v_for_debugging(v_file_path: &Path) -> Result<()> {
    let out_dir = PathBuf::from("out");
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("Failed to create out directory: {}", out_dir.display()))?;

    if let Some(file_name) = v_file_path.file_name() {
        let dest = out_dir.join(file_name);
        // Ignore errors here since the file might already be in out/
        let _ = fs::copy(v_file_path, &dest);
    }

    Ok(())
}
