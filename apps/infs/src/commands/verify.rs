#![warn(clippy::pedantic)]

//! Verify command for the infs CLI.
//!
//! Compiles Inference source files to WASM, translates to Rocq (.v),
//! and runs coqc to verify the generated proofs.
//!
//! ## Verification Pipeline
//!
//! 1. **Compile** - Parse, type check, analyze, and generate WASM
//! 2. **Translate** - Convert WASM to Rocq (.v) format
//! 3. **Verify** - Run coqc on the generated .v file
//!
//! ## Prerequisites
//!
//! This command requires coqc (the Rocq/Coq proof assistant) to be
//! installed and available in PATH.

use anyhow::{Context, Result, bail};
use clap::Args;
use inference::{analyze, codegen, parse, type_check, wasm_to_v};
use std::{fs, path::{Path, PathBuf}, process::Command};

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
/// 1. Checks for coqc availability
/// 2. Validates source file exists
/// 3. Compiles source to WASM (unless skip-compile is set and .v is fresh)
/// 4. Translates WASM to Rocq (.v)
/// 5. Runs coqc verification
/// 6. Reports results
///
/// ## Errors
///
/// Returns an error if:
/// - coqc is not found in PATH
/// - The source file does not exist
/// - Any compilation phase fails
/// - coqc verification fails
pub fn execute(args: &VerifyArgs) -> Result<()> {
    check_coqc_availability()?;

    if !args.path.exists() {
        bail!("Path not found: {}", args.path.display());
    }

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
        let v_content = compile_to_rocq(&args.path, source_fname)?;
        write_v_file(&args.output_dir, &v_file_path, &v_content)?;
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

/// Compiles source file to Rocq (.v) format.
fn compile_to_rocq(source_path: &Path, source_fname: &str) -> Result<String> {
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

    let v_content = wasm_to_v(source_fname, &wasm)
        .with_context(|| format!("WASM to Rocq translation failed for {source_fname}"))?;
    println!("Rocq translation generated");

    Ok(v_content)
}

/// Writes the .v file to the output directory.
fn write_v_file(output_dir: &Path, v_file_path: &Path, v_content: &str) -> Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

    fs::write(v_file_path, v_content)
        .with_context(|| format!("Failed to write Rocq file: {}", v_file_path.display()))?;

    println!("Rocq file written to: {}", v_file_path.display());
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
        fs::copy(v_file_path, &dest)
            .with_context(|| format!("Failed to copy .v file to {}", dest.display()))?;
    }

    Ok(())
}
