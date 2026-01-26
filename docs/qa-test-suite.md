# Comprehensive Manual QA Test Suite for `infs` CLI

This document provides a complete manual QA test suite for the Inference (`infs`) CLI toolchain. It covers all commands, flags, error scenarios, and cross-platform considerations.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Test Data Samples](#test-data-samples)
- [Test Categories](#test-categories)
  - [1. Help and Version](#1-help-and-version-10-tests)
  - [2. Build Command](#2-build-command-11-tests)
  - [3. Run Command](#3-run-command-6-tests)
  - [5. Project Scaffolding](#5-project-scaffolding-15-tests)
  - [6. Toolchain Management](#6-toolchain-management-16-tests)
  - [7. Doctor Command](#7-doctor-command-6-tests)
  - [8. Self Update](#8-self-update-5-tests)
  - [9. TUI and Headless](#9-tui-and-headless-5-tests)
  - [10. Environment Variables](#10-environment-variables-12-tests)
  - [11. Cross-Platform](#11-cross-platform-6-tests)
  - [12. Error Handling](#12-error-handling-6-tests)
  - [13. Non-deterministic Features](#13-non-deterministic-features-7-tests)
- [Test Execution Checklist](#test-execution-checklist)
- [Results Summary Template](#results-summary-template)

---

## Prerequisites

### Required Binaries

| Binary | Purpose | Installation |
|--------|---------|--------------|
| `infs` | CLI toolchain under test | Build from source or download release |
| `infc` | Inference compiler | Via `infs install` or manual setup |
| `inf-llc` | LLVM backend | Included in toolchain |
| `rust-lld` | Linker | Included in toolchain |

### Optional Tools

| Tool | Purpose | Required For |
|------|---------|--------------|
| `wasmtime` | WASM runtime | Run command tests |
| `coqc` | Rocq/Coq proof assistant | Verify command tests |
| `git` | Version control | Project scaffolding tests |

### Platform-Specific Setup

**Linux x64:**
```bash
# Ensure libLLVM is available
ldconfig -p | grep libLLVM
```

**macOS ARM64:**
```bash
# Ensure Xcode command line tools are installed
xcode-select --install
```

**Windows x64:**
```powershell
# Ensure Visual C++ Redistributable is installed
```

### Path Setup Guide

Before running the test suite, ensure `infs` is accessible from your terminal.

**Option 1: Add to PATH (Recommended for Testing)**

Linux/macOS:
```bash
# Add to current session
export PATH="$PATH:/path/to/infs/directory"

# Add permanently to ~/.bashrc or ~/.zshrc
echo 'export PATH="$PATH:/path/to/infs/directory"' >> ~/.bashrc
source ~/.bashrc
```

Windows (PowerShell):
```powershell
# Add to current session
$env:PATH += ";C:\path\to\infs\directory"

# Add permanently (run as Administrator)
[Environment]::SetEnvironmentVariable("PATH", $env:PATH + ";C:\path\to\infs\directory", "User")
```

Windows (Command Prompt):
```cmd
:: Add to current session
set PATH=%PATH%;C:\path\to\infs\directory

:: Add permanently via System Properties > Environment Variables
```

**Option 2: Use Full Path**

You can also run tests using the full path to the binary:
```bash
# Linux/macOS
/full/path/to/infs --version

# Windows
C:\full\path\to\infs.exe --version
```

**Option 3: Symlink to Standard Location**

Linux/macOS:
```bash
# Create symlink in /usr/local/bin (requires sudo)
sudo ln -s /path/to/infs /usr/local/bin/infs

# Or in user-local bin (no sudo required)
mkdir -p ~/.local/bin
ln -s /path/to/infs ~/.local/bin/infs
export PATH="$PATH:$HOME/.local/bin"
```

**Verifying Path Setup:**
```bash
# Check if infs is found
which infs        # Linux/macOS
where infs        # Windows

# Verify it runs correctly
infs --version
```

**Troubleshooting:**

| Issue | Solution |
|-------|----------|
| `command not found` / `not recognized` | Verify PATH includes the directory containing `infs` |
| Permission denied (Linux/macOS) | Run `chmod +x /path/to/infs` to make it executable |
| Wrong version running | Check `which infs` to see which binary is being used |
| PATH changes don't persist | Ensure you added to the correct shell config file (`.bashrc`, `.zshrc`, `.profile`) |

### Environment Setup

```bash
# Create a clean test directory
mkdir -p ~/infs-qa-test
cd ~/infs-qa-test

# Verify infs is accessible
infs --version
```

---

## Test Data Samples

Create these test files before running the test suite.

### trivial.inf
```inference
fn answer() -> i32 {
    42
}
```

### main_entry.inf
```inference
fn main() -> i32 {
    0
}
```

### nondet.inf
```inference
fn test_nondet() -> i32 {
    let x: i32 = @;

    forall y: i32 {
        assume y > 0;
    }

    exists z: i32 {
        z == x + 1
    }

    unique w: i32 {
        w == 42
    }

    x
}
```

### syntax_error.inf
```inference
fn broken( -> i32 {
    42
}
```

### type_error.inf
```inference
fn mistyped() -> i32 {
    "not an integer"
}
```

### empty.inf
```inference
```

### multi_function.inf
```inference
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

fn main() -> i32 {
    add(multiply(2, 3), 4)
}
```

---

## Test Categories

### 1. Help and Version (10 tests)

#### TC-1.1: Display Main Help
**Priority:** Critical
**Description:** Verify `infs --help` displays comprehensive help information.

**Preconditions:**
- `infs` binary is installed and in PATH

**Steps:**
1. Run `infs --help`

**Expected Result:**
- Exit code: 0
- Output contains:
  - Usage line: `infs [OPTIONS] [COMMAND]`
  - List of all subcommands: `new`, `init`, `build`, `run`, `verify`, `install`, `uninstall`, `list`, `default`, `doctor`, `self`, `version`
  - Global options: `--headless`, `-h`, `-V`

---

#### TC-1.2: Display Version (Short)
**Priority:** Critical
**Description:** Verify `infs --version` displays version string.

**Preconditions:**
- `infs` binary is installed and in PATH

**Steps:**
1. Run `infs --version`

**Expected Result:**
- Exit code: 0
- Output matches pattern: `infs X.Y.Z` (semantic version)

---

#### TC-1.3: Display Version (Verbose)
**Priority:** High
**Description:** Verify `infs version -v` displays detailed build information.

**Preconditions:**
- `infs` binary is installed and in PATH

**Steps:**
1. Run `infs version -v`

**Expected Result:**
- Exit code: 0
- Output contains:
  - Version number
  - Git commit hash
  - Platform identifier

---

#### TC-1.4: Subcommand Help - Build
**Priority:** High
**Description:** Verify `infs build --help` displays build-specific options.

**Preconditions:**
- `infs` binary is installed and in PATH

**Steps:**
1. Run `infs build --help`

**Expected Result:**
- Exit code: 0
- Output contains:
  - `--parse` flag
  - `--analyze` flag
  - `--codegen` flag
  - `-o` / `--generate-wasm-output` flag
  - `-v` / `--generate-v-output` flag

---

#### TC-1.5: Subcommand Help - Run
**Priority:** High
**Description:** Verify `infs run --help` displays run-specific usage.

**Preconditions:**
- `infs` binary is installed and in PATH

**Steps:**
1. Run `infs run --help`

**Expected Result:**
- Exit code: 0
- Output contains:
  - PATH argument description
  - PROGRAM_ARGS description
  - Usage example

---

#### TC-1.6: Unknown Subcommand Error
**Priority:** Medium
**Description:** Verify graceful error on unknown subcommand.

**Preconditions:**
- `infs` binary is installed and in PATH

**Steps:**
1. Run `infs unknown-command`

**Expected Result:**
- Exit code: Non-zero
- Error message indicates unknown subcommand
- Suggests similar commands if applicable

---

#### TC-1.7: Version Verbose Shows Git Commit Hash
**Priority:** High
**Description:** Verify verbose version displays a valid git commit hash.

**Preconditions:**
- `infs` binary built from git repository

**Steps:**
1. Run `infs version -v`
2. Examine the "Commit" field

**Expected Result:**
- Exit code: 0
- Commit field shows 7-character hex string (e.g., `abc1234`)
- Commit hash is not "unknown"

---

#### TC-1.8: Version Verbose Shows Platform
**Priority:** High
**Description:** Verify verbose version displays correct platform identifier.

**Preconditions:**
- `infs` binary is installed and in PATH

**Steps:**
1. Run `infs version -v`
2. Examine the "Platform" field

**Expected Result:**
- Exit code: 0
- Platform matches current system (e.g., `linux-x86_64`, `darwin-aarch64`, `windows-x86_64`)

---

#### TC-1.9: Version Flag vs Subcommand Consistency
**Priority:** Medium
**Description:** Verify `--version` flag and `version` subcommand produce consistent output.

**Preconditions:**
- `infs` binary is installed and in PATH

**Steps:**
1. Run `infs --version`
2. Run `infs version`
3. Compare outputs

**Expected Result:**
- Both commands exit with code 0
- Both show the same version number

---

#### TC-1.10: Version Without Git Repository
**Priority:** Low
**Description:** Verify version command handles missing git gracefully.

**Preconditions:**
- `infs` binary built outside a git repository (or `.git` removed)

**Steps:**
1. Build `infs` in a directory without `.git`
2. Run `infs version -v`

**Expected Result:**
- Exit code: 0
- Commit field shows "unknown"
- Other fields display normally

---

### 2. Build Command (11 tests)

#### TC-2.1: Parse Phase Only
**Priority:** Critical
**Description:** Verify `--parse` builds typed AST without further processing.

**Preconditions:**
- `trivial.inf` test file exists
- `infc` compiler is available

**Steps:**
1. Run `infs build trivial.inf --parse`

**Expected Result:**
- Exit code: 0
- No output files generated (no `-o` flag)
- Success message or silent completion

---

#### TC-2.2: Analyze Phase
**Priority:** Critical
**Description:** Verify `--analyze` performs semantic validation after parsing.

**Preconditions:**
- `trivial.inf` test file exists
- `infc` compiler is available

**Steps:**
1. Run `infs build trivial.inf --parse --analyze`

**Expected Result:**
- Exit code: 0
- Type inference completed successfully
- No output files generated

---

#### TC-2.3: Codegen Phase
**Priority:** Critical
**Description:** Verify `--codegen` generates WASM binary.

**Preconditions:**
- `trivial.inf` test file exists
- `infc` compiler with LLVM backend available

**Steps:**
1. Run `infs build trivial.inf --parse --codegen`

**Expected Result:**
- Exit code: 0
- Compilation completes successfully
- WASM binary can be generated when combined with `-o`

---

#### TC-2.4: Generate WASM Output
**Priority:** Critical
**Description:** Verify `-o` flag generates WASM binary file.

**Preconditions:**
- `trivial.inf` test file exists
- `infc` compiler with LLVM backend available

**Steps:**
1. Run `infs build trivial.inf --parse --codegen -o`
2. Check for `out/trivial.wasm`

**Expected Result:**
- Exit code: 0
- File `out/trivial.wasm` exists
- File is valid WebAssembly binary (starts with `\0asm`)

---

#### TC-2.5: Generate Rocq Output
**Priority:** High
**Description:** Verify `-v` flag generates Rocq (.v) translation file.

**Preconditions:**
- `trivial.inf` test file exists
- `infc` compiler with wasm-to-v support

**Steps:**
1. Run `infs build trivial.inf --parse --codegen -v`
2. Check for `out/trivial.v`

**Expected Result:**
- Exit code: 0
- File `out/trivial.v` exists
- File contains valid Rocq/Coq syntax

---

#### TC-2.6: Combined Output Flags
**Priority:** High
**Description:** Verify both `-o` and `-v` can be used together.

**Preconditions:**
- `trivial.inf` test file exists

**Steps:**
1. Run `infs build trivial.inf --parse --codegen -o -v`
2. Check for both output files

**Expected Result:**
- Exit code: 0
- Both `out/trivial.wasm` and `out/trivial.v` exist

---

#### TC-2.7: Syntax Error Detection
**Priority:** Critical
**Description:** Verify parser reports syntax errors clearly.

**Preconditions:**
- `syntax_error.inf` test file exists

**Steps:**
1. Run `infs build syntax_error.inf --parse`

**Expected Result:**
- Exit code: Non-zero
- Error message includes:
  - File name
  - Line and column number
  - Description of syntax error

---

#### TC-2.8: Type Error Detection
**Priority:** Critical
**Description:** Verify type checker reports type mismatches.

**Preconditions:**
- `type_error.inf` test file exists

**Steps:**
1. Run `infs build type_error.inf --parse --analyze`

**Expected Result:**
- Exit code: Non-zero
- Error message includes:
  - Expected type vs actual type
  - Location in source

---

#### TC-2.9: Missing File Error
**Priority:** High
**Description:** Verify graceful error for non-existent source file.

**Preconditions:**
- File `nonexistent.inf` does NOT exist

**Steps:**
1. Run `infs build nonexistent.inf --parse`

**Expected Result:**
- Exit code: Non-zero
- Error message indicates file not found
- No stack trace or panic

---

#### TC-2.10: Empty File Handling
**Priority:** Medium
**Description:** Verify empty file is handled gracefully.

**Preconditions:**
- `empty.inf` exists and is empty

**Steps:**
1. Run `infs build empty.inf --parse`

**Expected Result:**
- Exit code: 0 or specific empty-file error
- No crash or panic

---

#### TC-2.11: Phase Dependency Enforcement
**Priority:** High
**Description:** Verify phases execute in correct order regardless of flag order.

**Preconditions:**
- `trivial.inf` test file exists

**Steps:**
1. Run `infs build trivial.inf --codegen --parse -o`

**Expected Result:**
- Exit code: 0
- Phases execute in order: parse → analyze → codegen
- Output identical to `--parse --codegen -o`

---

### 3. Run Command (6 tests)

#### TC-3.1: Basic Execution
**Priority:** Critical
**Description:** Verify successful compilation and execution of simple program.

**Preconditions:**
- `main_entry.inf` test file exists
- `wasmtime` is installed and in PATH
- `infc` compiler is available

**Steps:**
1. Run `infs run main_entry.inf`

**Expected Result:**
- Exit code: 0 (from the WASM program)
- Program executes to completion

---

#### TC-3.2: Program Arguments
**Priority:** High
**Description:** Verify program arguments are passed to WASM runtime.

**Preconditions:**
- Test file with argument handling exists
- `wasmtime` is installed and in PATH

**Steps:**
1. Run `infs run main_entry.inf arg1 arg2`

**Expected Result:**
- Exit code: 0
- Arguments accessible to program (if program uses them)

---

#### TC-3.3: Compilation Failure in Run
**Priority:** High
**Description:** Verify run command fails gracefully on compilation error.

**Preconditions:**
- `syntax_error.inf` test file exists

**Steps:**
1. Run `infs run syntax_error.inf`

**Expected Result:**
- Exit code: Non-zero
- Error message indicates compilation failed
- Does not attempt to run non-existent WASM

---

#### TC-3.3b: Graceful Error Handling (One-liner)
**Priority:** High
**Description:** Verify compiler fails gracefully on syntax error without panic. No local setup required.

**Preconditions:**
- None (uses inline temp file)

**Steps:**
1. Run one-liner:
   ```bash
   echo 'p fn main() {}' > /tmp/syntax_error.inf && infs run /tmp/syntax_error.inf; echo "Exit: $?"
   ```

**Expected Result:**
- Exit code: Non-zero (1)
- Error message contains "error" or "Error" or "Syntax"
- NO panic or stack trace
- Output similar to:
  ```
  AST Builder Error: Syntax error at 1:1: unexpected or malformed token
  Parse error: AST building failed due to errors
  Exit: 1
  ```

---

#### TC-3.4: Missing wasmtime Error
**Priority:** High
**Description:** Verify clear error when wasmtime is not available.

**Preconditions:**
- `main_entry.inf` test file exists
- Ensure `wasmtime` is NOT in PATH (temporarily rename or remove from PATH)

**Steps:**
1. Run `infs run main_entry.inf`

**Expected Result:**
- Exit code: Non-zero
- Error message indicates wasmtime not found
- Suggests installation instructions

---

#### TC-3.4b: Missing wasmtime Error (One-liner)
**Priority:** High
**Description:** Verify clear error when wasmtime is not available. No local setup required.

**Preconditions:**
- None (uses inline temp file and modified PATH)

**Steps:**
1. Run one-liner:
   ```bash
   echo 'fn main() -> i32 { 0 }' > /tmp/main_entry.inf && env PATH="$(dirname $(which infs))" infs run /tmp/main_entry.inf; echo "Exit: $?"
   ```

**Expected Result:**
- Exit code: Non-zero (1)
- Error message contains "wasmtime" or "not found" or "runtime"
- NO panic or stack trace
- Output similar to:
  ```
  Error: wasmtime not found. Please install wasmtime...
  Exit: 1
  ```

---

#### TC-3.5: Runtime Error Propagation
**Priority:** Medium
**Description:** Verify runtime errors from WASM are propagated.

**Preconditions:**
- Test file that causes runtime error (e.g., division by zero)

**Steps:**
1. Run `infs run runtime_error.inf`

**Expected Result:**
- Exit code: Non-zero (from wasmtime)
- Runtime error message displayed

---

#### TC-3.6: Missing Source File
**Priority:** High
**Description:** Verify error handling for non-existent source file.

**Preconditions:**
- File `nonexistent.inf` does NOT exist

**Steps:**
1. Run `infs run nonexistent.inf`

**Expected Result:**
- Exit code: Non-zero
- Error message indicates file not found
- Does not proceed to compilation

---

### 5. Project Scaffolding (15 tests)

#### TC-5.1: New Project - Basic
**Priority:** Critical
**Description:** Verify `infs new` creates standard project structure.

**Preconditions:**
- Working directory is writable
- Directory `myproject` does NOT exist

**Steps:**
1. Run `infs new myproject`
2. Inspect created structure

**Expected Result:**
- Exit code: 0
- Directory structure created:
  ```
  myproject/
  ├── Inference.toml
  ├── src/
  │   └── main.inf
  ├── tests/
  │   └── .gitkeep
  ├── proofs/
  │   └── .gitkeep
  └── .gitignore
  ```
- `Inference.toml` contains valid `infc_version` field (semver format)

---

#### TC-5.2: New Project - Custom Path
**Priority:** High
**Description:** Verify project creation in specified parent directory.

**Preconditions:**
- Directory `~/projects` exists and is writable

**Steps:**
1. Run `infs new testproj ~/projects`
2. Check `~/projects/testproj/` exists

**Expected Result:**
- Exit code: 0
- Project created at `~/projects/testproj/`
- Full structure present

---

#### TC-5.3: New Project - No Git
**Priority:** High
**Description:** Verify `--no-git` skips git initialization and git-related files.

**Preconditions:**
- `git` is available
- Directory `nogitproject` does NOT exist

**Steps:**
1. Run `infs new nogitproject --no-git`
2. Check for `.git/` directory
3. Check for `.gitignore` file
4. Check for `.gitkeep` files

**Expected Result:**
- Exit code: 0
- Project created with `Inference.toml`, `src/main.inf`, `tests/`, `proofs/`
- `.git/` directory does NOT exist
- `.gitignore` does NOT exist
- `tests/.gitkeep` and `proofs/.gitkeep` do NOT exist

---

#### TC-5.4: New Project - Git Initialization
**Priority:** High
**Description:** Verify git repository is initialized by default.

**Preconditions:**
- `git` is available
- Directory `gitproject` does NOT exist

**Steps:**
1. Run `infs new gitproject`
2. Check for `.git/` directory
3. Run `git status` in project directory

**Expected Result:**
- Exit code: 0
- `.git/` directory exists
- `git status` shows clean working tree

---

#### TC-5.5: New Project - Invalid Name (Reserved Keyword)
**Priority:** High
**Description:** Verify rejection of reserved keyword as project name.

**Preconditions:**
- None

**Steps:**
1. Run `infs new fn`
2. Run `infs new let`
3. Run `infs new if`

**Expected Result:**
- Exit code: Non-zero for each
- Error message indicates reserved keyword

---

#### TC-5.7: New Project - Invalid Name (Invalid Characters)
**Priority:** High
**Description:** Verify rejection of names with invalid characters.

**Preconditions:**
- None

**Steps:**
1. Run `infs new "my project"`
2. Run `infs new "project@name"`
3. Run `infs new "123project"`

**Expected Result:**
- Exit code: Non-zero for each
- Error message indicates invalid characters

---

#### TC-5.8: New Project - Directory Exists Error
**Priority:** High
**Description:** Verify error when target directory already exists.

**Preconditions:**
- Directory `existing` already exists

**Steps:**
1. Create directory `existing/`
2. Run `infs new existing`

**Expected Result:**
- Exit code: Non-zero
- Error message indicates directory exists
- Existing directory not modified

---

#### TC-5.9: Init - Current Directory (No Git)
**Priority:** Critical
**Description:** Verify `infs init` initializes current directory without git.

**Preconditions:**
- Empty directory exists (no `.git/`)
- No `Inference.toml` present

**Steps:**
1. Create and enter empty directory `inittest`
2. Run `infs init`
3. Check created files

**Expected Result:**
- Exit code: 0
- `Inference.toml` created
- `src/main.inf` created
- `.gitignore` NOT created (no `.git/` detected)
- `tests/.gitkeep` and `proofs/.gitkeep` NOT created

---

#### TC-5.9b: Init - Current Directory (With Git)
**Priority:** High
**Description:** Verify `infs init` creates git files when `.git/` exists.

**Preconditions:**
- Directory with `.git/` initialized
- No `Inference.toml` present

**Steps:**
1. Create and enter directory `initgittest`
2. Run `git init`
3. Run `infs init`
4. Check created files

**Expected Result:**
- Exit code: 0
- `Inference.toml` created
- `src/main.inf` created
- `.gitignore` created
- `tests/.gitkeep` and `proofs/.gitkeep` created

---

#### TC-5.9c: Init - Does Not Overwrite Existing Git Files
**Priority:** Medium
**Description:** Verify `infs init` does not overwrite existing `.gitignore`.

**Preconditions:**
- Directory with `.git/` and existing `.gitignore`

**Steps:**
1. Create directory with `git init`
2. Create custom `.gitignore` with content `custom-ignore`
3. Run `infs init`
4. Check `.gitignore` contents

**Expected Result:**
- Exit code: 0
- `.gitignore` contains `custom-ignore` (not overwritten)

---

#### TC-5.10: Init - Custom Name
**Priority:** High
**Description:** Verify custom project name in init.

**Preconditions:**
- Empty directory exists

**Steps:**
1. Create and enter directory `somedir`
2. Run `infs init customname`
3. Check `Inference.toml` contents

**Expected Result:**
- Exit code: 0
- `Inference.toml` has `name = "customname"`

---

#### TC-5.11: Init - Directory Already Initialized
**Priority:** High
**Description:** Verify error when `Inference.toml` already exists.

**Preconditions:**
- Directory with existing `Inference.toml`

**Steps:**
1. Run `infs init` in initialized project directory

**Expected Result:**
- Exit code: Non-zero
- Error indicates already initialized
- Existing files not modified

---

#### TC-5.12: Init - Non-Empty Directory
**Priority:** Medium
**Description:** Verify init works in non-empty directory without Inference.toml.

**Preconditions:**
- Directory with some files but no `Inference.toml`

**Steps:**
1. Create directory with `README.md`
2. Run `infs init`

**Expected Result:**
- Exit code: 0
- `Inference.toml` and `src/main.inf` created
- `README.md` preserved

---

#### TC-5.13: Init - Read-Only Directory
**Priority:** Low
**Description:** Verify graceful error in read-only directory.

**Preconditions:**
- Read-only directory exists

**Steps:**
1. Create directory and make read-only: `chmod 555 readonly`
2. Run `infs init` inside it

**Expected Result:**
- Exit code: Non-zero
- Error indicates permission denied
- No partial files created

---

### 6. Toolchain Management (16 tests)

#### TC-6.1: Install Latest
**Priority:** Critical
**Description:** Verify installation of latest toolchain version.

**Preconditions:**
- Network access available
- `~/.inference/` writable or doesn't exist

**Steps:**
1. Run `infs install`
2. Check `~/.inference/toolchains/`

**Expected Result:**
- Exit code: 0
- Progress displayed during download
- Toolchain installed in `~/.inference/toolchains/X.Y.Z/`
- Set as default if first installation

---

#### TC-6.2: Install Specific Version
**Priority:** Critical
**Description:** Verify installation of specific version.

**Preconditions:**
- Network access available
- Version `0.1.0` available in manifest

**Steps:**
1. Run `infs install 0.1.0`
2. Check installed version

**Expected Result:**
- Exit code: 0
- Version 0.1.0 installed
- Listed in `infs list`

---

#### TC-6.3: Install Already Installed Version
**Priority:** High
**Description:** Verify handling of already installed version.

**Preconditions:**
- Version already installed

**Steps:**
1. Install a version: `infs install`
2. Run same install command again

**Expected Result:**
- Exit code: 0
- Message indicates already installed
- No re-download

---

#### TC-6.4: Install Invalid Version
**Priority:** High
**Description:** Verify error for non-existent version.

**Preconditions:**
- Network access available

**Steps:**
1. Run `infs install 99.99.99`

**Expected Result:**
- Exit code: Non-zero
- Error indicates version not found
- Suggests available versions

---

#### TC-6.5: List Installed Toolchains
**Priority:** Critical
**Description:** Verify listing of all installed versions.

**Preconditions:**
- At least one toolchain installed

**Steps:**
1. Run `infs list`

**Expected Result:**
- Exit code: 0
- Lists all installed versions
- Default marked with asterisk (*)
- Installation dates shown

---

#### TC-6.6: List Empty Toolchains
**Priority:** Medium
**Description:** Verify list output when no toolchains installed.

**Preconditions:**
- No toolchains installed (clean `~/.inference/`)

**Steps:**
1. Run `infs list`

**Expected Result:**
- Exit code: 0
- Message indicates no toolchains installed
- Suggests `infs install`

---

#### TC-6.7: Set Default Toolchain
**Priority:** Critical
**Description:** Verify changing default toolchain version.

**Preconditions:**
- Multiple versions installed (e.g., 0.1.0 and 0.2.0)

**Steps:**
1. Run `infs default 0.1.0`
2. Run `infs list` to verify

**Expected Result:**
- Exit code: 0
- Default changed to 0.1.0
- Asterisk moves to 0.1.0 in list

---

#### TC-6.8: Set Default - Not Installed
**Priority:** High
**Description:** Verify error when setting non-installed version as default.

**Preconditions:**
- Version `99.99.99` NOT installed

**Steps:**
1. Run `infs default 99.99.99`

**Expected Result:**
- Exit code: Non-zero
- Error indicates version not installed
- Current default unchanged

---

#### TC-6.9: Uninstall Toolchain
**Priority:** Critical
**Description:** Verify removal of installed toolchain.

**Preconditions:**
- Version installed but not set as default

**Steps:**
1. Install version: `infs install 0.1.0`
2. Set different default: `infs default 0.2.0`
3. Run `infs uninstall 0.1.0`
4. Run `infs list`

**Expected Result:**
- Exit code: 0
- Version removed from list
- Directory deleted from `~/.inference/toolchains/`

---

#### TC-6.10: Uninstall Default Toolchain
**Priority:** High
**Description:** Verify handling of uninstalling default version.

**Preconditions:**
- Multiple versions installed
- One set as default

**Steps:**
1. Note current default
2. Run `infs uninstall <default-version>`
3. Run `infs list`

**Expected Result:**
- Exit code: 0
- Version uninstalled
- If other versions exist, one becomes new default
- If no versions remain, default cleared

---

#### TC-6.11: Uninstall Last Toolchain
**Priority:** High
**Description:** Verify uninstalling the only installed toolchain.

**Preconditions:**
- Exactly one toolchain installed

**Steps:**
1. Run `infs uninstall <only-version>`
2. Run `infs list`

**Expected Result:**
- Exit code: 0
- Toolchain removed
- List shows no toolchains
- Default cleared

---

#### TC-6.12: Uninstall Non-Existent Version
**Priority:** Medium
**Description:** Verify error for uninstalling non-installed version.

**Preconditions:**
- Version `99.99.99` NOT installed

**Steps:**
1. Run `infs uninstall 99.99.99`

**Expected Result:**
- Exit code: Non-zero
- Error indicates version not installed

---

#### TC-6.13: Install - Network Error
**Priority:** High
**Description:** Verify graceful handling of network failure.

**Preconditions:**
- Simulate network failure (disconnect or block requests)

**Steps:**
1. Disable network or block manifest URL
2. Run `infs install`

**Expected Result:**
- Exit code: Non-zero
- Error indicates network failure
- Suggests checking connectivity

---

#### TC-6.14: Install - Checksum Verification
**Priority:** Critical
**Description:** Verify SHA256 checksum is validated on download.

**Preconditions:**
- Network access available

**Steps:**
1. Run `infs install` with verbose output if available
2. Observe checksum verification step

**Expected Result:**
- Exit code: 0
- Checksum verification performed
- Installation proceeds only if checksum matches

---

#### TC-6.15: Install - Corrupted Download
**Priority:** High
**Description:** Verify detection of corrupted/tampered download.

**Preconditions:**
- Way to intercept and corrupt download (e.g., proxy)

**Steps:**
1. Set up corrupted download scenario
2. Run `infs install`

**Expected Result:**
- Exit code: Non-zero
- Error indicates checksum mismatch
- Corrupted file deleted
- No partial installation

---

#### TC-6.16: Toolchain Directory Permissions
**Priority:** Medium
**Description:** Verify error when toolchain directory not writable.

**Preconditions:**
- `~/.inference/toolchains/` exists and is read-only

**Steps:**
1. Make directory read-only: `chmod 555 ~/.inference/toolchains`
2. Run `infs install`

**Expected Result:**
- Exit code: Non-zero
- Error indicates permission denied
- Suggests checking permissions

---

### 7. Doctor Command (6 tests)

#### TC-7.1: Healthy Installation
**Priority:** Critical
**Description:** Verify doctor reports all green on healthy installation.

**Preconditions:**
- Complete toolchain installed
- All binaries present

**Steps:**
1. Run `infs doctor`

**Expected Result:**
- Exit code: 0
- All checks pass (green checkmarks)
- Checks include:
  - Platform detection
  - Toolchain directory
  - Default toolchain
  - inf-llc binary
  - rust-lld binary

---

#### TC-7.2: Missing Default Toolchain
**Priority:** High
**Description:** Verify doctor detects missing default toolchain.

**Preconditions:**
- Toolchain directory exists but no default set

**Steps:**
1. Remove or rename default file
2. Run `infs doctor`

**Expected Result:**
- Exit code: Non-zero or warning
- Warning/error for missing default
- Suggests running `infs install`

---

#### TC-7.3: Missing inf-llc Binary
**Priority:** High
**Description:** Verify doctor detects missing inf-llc.

**Preconditions:**
- Toolchain installed
- `inf-llc` binary removed or renamed

**Steps:**
1. Remove/rename inf-llc from toolchain
2. Run `infs doctor`

**Expected Result:**
- Exit code: Non-zero or warning
- Error indicates inf-llc missing
- Shows expected path

---

#### TC-7.4: Missing rust-lld Binary
**Priority:** High
**Description:** Verify doctor detects missing rust-lld.

**Preconditions:**
- Toolchain installed
- `rust-lld` binary removed or renamed

**Steps:**
1. Remove/rename rust-lld from toolchain
2. Run `infs doctor`

**Expected Result:**
- Exit code: Non-zero or warning
- Error indicates rust-lld missing
- Shows expected path

---

#### TC-7.5: No Toolchain Directory
**Priority:** High
**Description:** Verify doctor handles missing toolchain directory.

**Preconditions:**
- `~/.inference/` does not exist

**Steps:**
1. Remove or rename `~/.inference/`
2. Run `infs doctor`

**Expected Result:**
- Exit code: Non-zero
- Error indicates toolchain directory missing
- Suggests running `infs install`

---

#### TC-7.6: Linux - Missing libLLVM
**Priority:** High (Linux only)
**Description:** Verify doctor detects missing libLLVM on Linux.

**Preconditions:**
- Running on Linux
- libLLVM not available

**Steps:**
1. Ensure libLLVM is not in library path
2. Run `infs doctor`

**Expected Result:**
- Exit code: Non-zero or warning
- Warning about missing libLLVM
- Suggests installation command

---

### 8. Self Update (5 tests)

#### TC-8.1: Update Available
**Priority:** Critical
**Description:** Verify self update when newer version available.

**Preconditions:**
- Network access available
- Newer version in manifest than current

**Steps:**
1. Run `infs self update`

**Expected Result:**
- Exit code: 0
- Download progress shown
- Binary updated to new version
- Version confirmed with `infs --version`

---

#### TC-8.2: Already Latest
**Priority:** High
**Description:** Verify message when already on latest version.

**Preconditions:**
- Running latest version

**Steps:**
1. Run `infs self update`

**Expected Result:**
- Exit code: 0
- Message indicates already on latest version
- No download attempted

---

#### TC-8.3: Network Error During Update
**Priority:** High
**Description:** Verify graceful handling of network failure.

**Preconditions:**
- Network disabled or blocked

**Steps:**
1. Disable network
2. Run `infs self update`

**Expected Result:**
- Exit code: Non-zero
- Error indicates network failure
- Current binary unchanged

---

#### TC-8.4: Permission Error During Update
**Priority:** High
**Description:** Verify error when binary not writable.

**Preconditions:**
- infs binary is read-only

**Steps:**
1. Make infs binary read-only
2. Run `infs self update`

**Expected Result:**
- Exit code: Non-zero
- Error indicates permission denied
- Current binary unchanged

---

#### TC-8.5: Windows Update Strategy
**Priority:** High (Windows only)
**Description:** Verify Windows-specific update rename strategy.

**Preconditions:**
- Running on Windows
- Update available

**Steps:**
1. Run `infs self update`
2. Check for `infs.old` file

**Expected Result:**
- Exit code: 0
- Update completes successfully
- `infs.old` may exist (old binary renamed)

---

### 9. TUI and Headless (5 tests)

#### TC-9.1: TUI Launch Default
**Priority:** Critical
**Description:** Verify TUI launches when run without arguments in terminal.

**Preconditions:**
- Running in interactive terminal
- No CI environment variables set

**Steps:**
1. Run `infs` (no arguments)

**Expected Result:**
- TUI interface launches
- Navigation menu visible
- Can exit with `q` or Ctrl+C

---

#### TC-9.2: Headless Flag
**Priority:** Critical
**Description:** Verify `--headless` prevents TUI launch.

**Preconditions:**
- Running in interactive terminal

**Steps:**
1. Run `infs --headless`

**Expected Result:**
- TUI does NOT launch
- Help message displayed
- Immediate return to shell

---

#### TC-9.3: INFS_NO_TUI Environment Detection
**Priority:** High
**Description:** Verify TUI disabled when INFS_NO_TUI is set.

**Preconditions:**
- Interactive terminal

**Steps:**
1. Run `INFS_NO_TUI=1 infs`

**Expected Result:**
- TUI does NOT launch
- Help or appropriate output shown

---

#### TC-9.5: Non-TTY Detection
**Priority:** High
**Description:** Verify TUI disabled when stdout not a terminal.

**Preconditions:**
- None

**Steps:**
1. Run `infs | cat`

**Expected Result:**
- TUI does NOT launch
- Output suitable for piping

---

#### TC-9.6: TUI Command Execution
**Priority:** High
**Description:** Verify TUI can execute commands and return to menu.

**Preconditions:**
- TUI launches successfully

**Steps:**
1. Launch `infs`
2. Navigate to a command (e.g., version)
3. Execute command
4. Observe return to TUI

**Expected Result:**
- Command executes
- Output displayed
- Press Enter to return to TUI
- Can exit cleanly

---

### 10. Environment Variables (12 tests)

#### TC-10.1: INFC_PATH Override
**Priority:** High
**Description:** Verify INFC_PATH overrides compiler location.

**Preconditions:**
- Custom `infc` binary at known location

**Steps:**
1. Set `INFC_PATH=/path/to/custom/infc`
2. Run `infs build trivial.inf --parse`
3. Verify custom infc was used (e.g., via strace or version output)

**Expected Result:**
- Custom infc binary used
- Build succeeds with custom compiler

---

#### TC-10.2: INFERENCE_HOME Override
**Priority:** High
**Description:** Verify INFERENCE_HOME overrides toolchain directory.

**Preconditions:**
- Writable directory for custom home

**Steps:**
1. Set `INFERENCE_HOME=/tmp/custom-infs`
2. Run `infs install`
3. Check `/tmp/custom-infs/toolchains/`

**Expected Result:**
- Toolchain installed in custom directory
- Default `~/.inference/` not modified

---

#### TC-10.3: INFS_DIST_SERVER Override
**Priority:** Medium
**Description:** Verify INFS_DIST_SERVER overrides distribution server for manifest fetching.

**Preconditions:**
- Custom distribution server with valid `releases.json` (or local test server)

**Steps:**
1. Set `INFS_DIST_SERVER=http://localhost:8080`
2. Run `infs install`

**Expected Result:**
- Manifest fetched from `http://localhost:8080/releases.json`
- Installation proceeds using artifacts from custom server
- If server unreachable, clear error message displayed

**Manifest Format Notes:**
- The `releases.json` manifest uses a simplified format with only 2 required fields per file:
  - `url`: Full download URL
  - `sha256`: SHA256 checksum
- Derived fields (`filename`, `os`, `tool`) are extracted from the URL
- Example minimal manifest:
  ```json
  [
    {
      "version": "0.2.0",
      "stable": true,
      "files": [
        {
          "url": "https://github.com/Inferara/inference/releases/download/v0.2.0/infc-linux-x64.tar.gz",
          "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        }
      ]
    }
  ]
  ```

---

#### TC-10.4: INFS_DIST_SERVER Empty/Invalid Handling
**Priority:** Medium
**Description:** Verify empty or whitespace-only INFS_DIST_SERVER falls back to default.

**Preconditions:**
- Network access available

**Steps:**
1. Set `INFS_DIST_SERVER=""` (empty string)
2. Run `infs install`
3. Verify manifest fetched from default server
4. Set `INFS_DIST_SERVER="   "` (whitespace only)
5. Run `infs install`
6. Verify manifest fetched from default server

**Expected Result:**
- Empty string value treated as unset, falls back to `https://inference-lang.org`
- Whitespace-only value treated as unset, falls back to `https://inference-lang.org`
- Installation proceeds normally using default distribution server

---

#### TC-10.5: Invalid Environment Variable
**Priority:** Medium
**Description:** Verify graceful handling of invalid environment values.

**Preconditions:**
- None

**Steps:**
1. Set `INFC_PATH=/nonexistent/path/infc`
2. Run `infs build trivial.inf --parse`

**Expected Result:**
- Exit code: Non-zero
- Error indicates compiler not found at specified path
- Clear error message

---

#### TC-10.6: Compiler Resolution Priority Order
**Priority:** High
**Description:** Verify compiler is found using correct priority order.

The `infc` compiler is located using a 3-tier priority system:
1. `INFC_PATH` environment variable (highest priority)
2. System PATH via `which infc`
3. Managed toolchain at `~/.inference/toolchains/VERSION/bin/infc` (lowest priority)

**Preconditions:**
- Managed toolchain installed via `infs install`
- `infc` available in system PATH (optional, for full test)

**Steps:**
1. Verify Priority 3 (managed toolchain):
   - Unset `INFC_PATH`
   - Ensure `infc` not in system PATH
   - Run `infs build trivial.inf --parse`
   - Confirm build uses managed toolchain

2. Verify Priority 2 overrides Priority 3:
   - Add different `infc` version to system PATH
   - Run `infs build trivial.inf --parse`
   - Confirm system PATH version is used (not managed toolchain)

3. Verify Priority 1 overrides all:
   - Set `INFC_PATH=/path/to/specific/infc`
   - Run `infs build trivial.inf --parse`
   - Confirm INFC_PATH version is used

**Expected Result:**
- Priority 1 (INFC_PATH) always wins when set
- Priority 2 (system PATH) used when INFC_PATH not set
- Priority 3 (managed) used only when neither INFC_PATH nor system PATH available

---

#### TC-10.7: Toolchain Source Detection - Managed
**Priority:** High
**Description:** Verify verbose version correctly identifies managed toolchain source.

**Preconditions:**
- Managed toolchain installed via `infs install`
- `INFC_PATH` not set
- `infc` not in system PATH

**Steps:**
1. Unset `INFC_PATH`
2. Ensure `infc` not in system PATH
3. Run `infs version -v`

**Expected Result:**
- Exit code: 0
- Toolchain source shows "managed"
- Location shows `~/.inference/toolchains/VERSION/bin/infc`

---

#### TC-10.8: Toolchain Source Detection - System PATH
**Priority:** High
**Description:** Verify verbose version correctly identifies system PATH toolchain source.

**Preconditions:**
- `infc` available in system PATH
- `INFC_PATH` not set

**Steps:**
1. Unset `INFC_PATH`
2. Ensure `infc` is in system PATH
3. Run `infs version -v`

**Expected Result:**
- Exit code: 0
- Toolchain source shows "system"
- Location shows path from `which infc`

---

#### TC-10.9: Toolchain Source Detection - Override
**Priority:** High
**Description:** Verify verbose version correctly identifies INFC_PATH override source.

**Preconditions:**
- Valid `infc` binary at custom location

**Steps:**
1. Set `INFC_PATH=/path/to/custom/infc`
2. Run `infs version -v`

**Expected Result:**
- Exit code: 0
- Toolchain source shows "override"
- Location shows the INFC_PATH value

---

#### TC-10.10: No Toolchain Available
**Priority:** High
**Description:** Verify verbose version handles missing toolchain gracefully.

**Preconditions:**
- No managed toolchain installed
- `infc` not in system PATH
- `INFC_PATH` not set

**Steps:**
1. Ensure no toolchain is available
2. Run `infs version -v`

**Expected Result:**
- Exit code: 0
- Toolchain shows "not found" or similar message
- No crash or panic

---

#### TC-10.11: Build Fails Without Toolchain
**Priority:** Critical
**Description:** Verify build command provides clear error when no toolchain available.

**Preconditions:**
- No managed toolchain installed
- `infc` not in system PATH
- `INFC_PATH` not set
- `trivial.inf` test file exists

**Steps:**
1. Ensure no toolchain is available
2. Run `infs build trivial.inf --parse`

**Expected Result:**
- Exit code: Non-zero
- Error message clearly indicates no compiler found
- Suggests running `infs install` or setting `INFC_PATH`

---

#### TC-10.12: INFC_PATH With Invalid Binary
**Priority:** Medium
**Description:** Verify graceful handling when INFC_PATH points to non-executable file.

**Preconditions:**
- File exists at specified path but is not executable

**Steps:**
1. Create a non-executable file: `touch /tmp/fake-infc`
2. Set `INFC_PATH=/tmp/fake-infc`
3. Run `infs build trivial.inf --parse`

**Expected Result:**
- Exit code: Non-zero
- Error indicates the file is not executable or not a valid compiler
- No crash or panic

---

### 11. Cross-Platform (6 tests)

#### TC-11.1: Linux x64 Build
**Priority:** Critical
**Description:** Verify complete build workflow on Linux x64.

**Preconditions:**
- Running on Linux x64
- Toolchain installed

**Steps:**
1. Run `infs build trivial.inf --parse --codegen -o`
2. Verify WASM output
3. Run `infs run main_entry.inf`

**Expected Result:**
- Build succeeds
- WASM binary valid
- Execution succeeds with wasmtime

---

#### TC-11.2: Windows x64 Build
**Priority:** Critical
**Description:** Verify complete build workflow on Windows x64.

**Preconditions:**
- Running on Windows x64
- Toolchain installed
- Visual C++ Redistributable installed

**Steps:**
1. Run `infs build trivial.inf --parse --codegen -o`
2. Verify WASM output
3. Run `infs run main_entry.inf`

**Expected Result:**
- Build succeeds
- WASM binary valid
- Execution succeeds with wasmtime

---

#### TC-11.3: macOS ARM64 Build
**Priority:** Critical
**Description:** Verify complete build workflow on macOS ARM64.

**Preconditions:**
- Running on macOS with Apple Silicon
- Toolchain installed
- Xcode command line tools installed

**Steps:**
1. Run `infs build trivial.inf --parse --codegen -o`
2. Verify WASM output
3. Run `infs run main_entry.inf`

**Expected Result:**
- Build succeeds
- WASM binary valid
- Execution succeeds with wasmtime

---

#### TC-11.4: Path Separator Handling
**Priority:** High
**Description:** Verify correct path handling across platforms.

**Preconditions:**
- Subdirectory structure exists

**Steps:**
1. Create `subdir/nested/test.inf`
2. Run `infs build subdir/nested/test.inf --parse`

**Expected Result:**
- Path resolved correctly on all platforms
- Build succeeds regardless of separator style

---

#### TC-11.5: Platform Detection
**Priority:** High
**Description:** Verify correct platform detection for downloads.

**Preconditions:**
- Network access

**Steps:**
1. Run `infs doctor`
2. Note detected platform
3. Run `infs install`

**Expected Result:**
- Platform correctly identified
- Correct binary downloaded for platform

---

#### TC-11.6: Unsupported Platform Error
**Priority:** Medium
**Description:** Verify clear error on unsupported platform.

**Preconditions:**
- Running on unsupported platform (e.g., 32-bit)

**Steps:**
1. Attempt to run `infs install`

**Expected Result:**
- Exit code: Non-zero
- Clear error about unsupported platform
- Lists supported platforms

---

### 12. Error Handling (6 tests)

#### TC-12.1: Read-Only Output Directory
**Priority:** High
**Description:** Verify error when output directory not writable.

**Preconditions:**
- `out/` directory exists and is read-only

**Steps:**
1. Create and set `out/` to read-only
2. Run `infs build trivial.inf --parse --codegen -o`

**Expected Result:**
- Exit code: Non-zero
- Error indicates permission denied
- No partial output

---

#### TC-12.2: Disk Full Scenario
**Priority:** Medium
**Description:** Verify graceful handling of disk full condition.

**Preconditions:**
- Very limited disk space or simulated full disk

**Steps:**
1. Fill available disk space
2. Run `infs install`

**Expected Result:**
- Exit code: Non-zero
- Error indicates disk full
- Partial downloads cleaned up

---

#### TC-12.3: Corrupted Toolchain State
**Priority:** High
**Description:** Verify recovery from corrupted toolchain metadata.

**Preconditions:**
- Toolchain installed
- Corrupt the metadata file

**Steps:**
1. Corrupt `~/.inference/toolchains/VERSION/.metadata.json`
2. Run `infs list`
3. Run `infs doctor`

**Expected Result:**
- No crash
- Warning about corrupted metadata
- Suggestion to reinstall

---

#### TC-12.4: Concurrent Access
**Priority:** Medium
**Description:** Verify handling of concurrent infs operations.

**Preconditions:**
- None

**Steps:**
1. Run `infs install` in two terminals simultaneously

**Expected Result:**
- At least one succeeds
- No corrupted state
- No file lock errors (or graceful handling)

---

#### TC-12.5: Interrupted Download Recovery
**Priority:** High
**Description:** Verify recovery from interrupted download.

**Preconditions:**
- Network access

**Steps:**
1. Start `infs install`
2. Interrupt mid-download (Ctrl+C)
3. Run `infs install` again

**Expected Result:**
- Partial download cleaned up
- Fresh download starts
- Installation completes

---

#### TC-12.6: Subprocess Failure Propagation
**Priority:** High
**Description:** Verify subprocess errors propagate correctly.

**Preconditions:**
- Source file that causes compiler crash

**Steps:**
1. Create file that triggers compiler error
2. Run `infs build crasher.inf --parse`

**Expected Result:**
- Exit code: Non-zero (from compiler)
- Error output from compiler visible
- No infs wrapper crash

---

### 13. Non-deterministic Features (7 tests)

#### TC-13.1: Oracle Operator (@)
**Priority:** Critical
**Description:** Verify compilation of oracle operator.

**Preconditions:**
- Source file with `@` operator

**Steps:**
1. Create file with `let x: i32 = @;`
2. Run `infs build oracle.inf --parse --codegen -o`

**Expected Result:**
- Exit code: 0
- WASM binary generated
- Contains custom intrinsic for oracle

---

#### TC-13.2: Forall Block
**Priority:** Critical
**Description:** Verify compilation of forall quantifier block.

**Preconditions:**
- Source file with forall block

**Steps:**
1. Create file:
   ```inference
   fn test() -> i32 {
       forall x: i32 {
           x >= 0
       }
       0
   }
   ```
2. Run `infs build forall.inf --parse --codegen -o`

**Expected Result:**
- Exit code: 0
- WASM binary generated
- Contains forall intrinsic

---

#### TC-13.3: Exists Block
**Priority:** Critical
**Description:** Verify compilation of exists quantifier block.

**Preconditions:**
- Source file with exists block

**Steps:**
1. Create file:
   ```inference
   fn test() -> i32 {
       exists x: i32 {
           x == 42
       }
       0
   }
   ```
2. Run `infs build exists.inf --parse --codegen -o`

**Expected Result:**
- Exit code: 0
- WASM binary generated
- Contains exists intrinsic

---

#### TC-13.4: Assume Block
**Priority:** Critical
**Description:** Verify compilation of assume block.

**Preconditions:**
- Source file with assume block

**Steps:**
1. Create file:
   ```inference
   fn test(x: i32) -> i32 {
       assume x > 0;
       x
   }
   ```
2. Run `infs build assume.inf --parse --codegen -o`

**Expected Result:**
- Exit code: 0
- WASM binary generated
- Contains assume intrinsic

---

#### TC-13.5: Unique Block
**Priority:** Critical
**Description:** Verify compilation of unique quantifier block.

**Preconditions:**
- Source file with unique block

**Steps:**
1. Create file:
   ```inference
   fn test() -> i32 {
       unique x: i32 {
           x == 42
       }
       0
   }
   ```
2. Run `infs build unique.inf --parse --codegen -o`

**Expected Result:**
- Exit code: 0
- WASM binary generated
- Contains unique intrinsic

---

#### TC-13.6: Combined Non-deterministic Blocks
**Priority:** High
**Description:** Verify compilation of file with all non-det features.

**Preconditions:**
- `nondet.inf` test file with all features

**Steps:**
1. Run `infs build nondet.inf --parse --codegen -o`
2. Run `infs build nondet.inf --parse --codegen -v`

**Expected Result:**
- Exit code: 0
- Both WASM and Rocq outputs generated
- All intrinsics present

---

#### TC-13.7: Non-det Feature Verification
**Priority:** High
**Description:** Verify Rocq translation of non-det features.

**Preconditions:**
- `coqc` installed
- `nondet.inf` test file

**Steps:**
1. Run `infs verify nondet.inf`

**Expected Result:**
- Exit code: 0 (if proofs valid)
- `.v` file contains proper Rocq translation
- coqc can process the file

---

## Test Execution Checklist

### Pre-Test Setup

- [ ] Fresh test directory created
- [ ] Test data files created (see Test Data Samples)
- [ ] `infs` binary accessible in PATH
- [ ] Note infs version: `infs --version`
- [ ] Note platform: `uname -a` or equivalent
- [ ] Optional tools checked:
  - [ ] `wasmtime --version`
  - [ ] `coqc --version`
  - [ ] `git --version`

### Environment Cleanup (Between Test Runs)

```bash
# Clean up test artifacts
rm -rf out/ proofs/ custom_proofs/ new_proofs/
rm -rf myproject testproj nogitproject gitproject existing inittest somedir

# Optional: Clean toolchain state
rm -rf ~/.inference/  # WARNING: Removes all installed toolchains
```

### Test Execution Order (Recommended)

1. **Help and Version** - Verify basic CLI functionality
2. **Doctor** - Verify installation health
3. **Toolchain Management** - Ensure compiler available
4. **Build Command** - Core compilation tests
5. **Run Command** - Execution tests
6. **Verify Command** - Proof verification tests
7. **Project Scaffolding** - Project management
8. **TUI and Headless** - Interface modes
9. **Environment Variables** - Configuration
10. **Error Handling** - Edge cases
11. **Non-deterministic Features** - Language-specific
12. **Cross-Platform** - Platform-specific tests
13. **Self Update** - Binary management (run last)

---

## Results Summary Template

### Test Execution Summary

| Field | Value |
|-------|-------|
| **Tester** | |
| **Date** | |
| **Platform** | |
| **infs Version** | |
| **Toolchain Version** | |

### Results by Category

| Category | Total | Passed | Failed | Skipped | Notes |
|----------|-------|--------|--------|---------|-------|
| 1. Help and Version | 10 | | | | |
| 2. Build Command | 11 | | | | |
| 3. Run Command | 6 | | | | |
| 4. Verify Command | 8 | | | | |
| 5. Project Scaffolding | 15 | | | | |
| 6. Toolchain Management | 16 | | | | |
| 7. Doctor Command | 6 | | | | |
| 8. Self Update | 5 | | | | |
| 9. TUI and Headless | 5 | | | | |
| 10. Environment Variables | 12 | | | | |
| 11. Cross-Platform | 6 | | | | |
| 12. Error Handling | 6 | | | | |
| 13. Non-deterministic Features | 7 | | | | |
| **TOTAL** | **113** | | | | |

### Detailed Failure Log

| Test ID | Description | Actual Result | Notes |
|---------|-------------|---------------|-------|
| | | | |
| | | | |
| | | | |

### Platform-Specific Notes

**Linux x64:**
-

**Windows x64:**
-

**macOS ARM64:**
-

### Recommendations

1.
2.
3.

---

*Document Version: 1.0*
*Last Updated: 2026-01-23*
