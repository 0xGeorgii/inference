#![warn(clippy::pedantic)]

//! New project command for the infs CLI.
//!
//! Creates a new Inference project with a standard directory structure.
//!
//! ## Usage
//!
//! ```bash
//! infs new myproject                    # Create with default template
//! infs new myproject --no-git           # Skip git initialization
//! infs new myproject ./path             # Create in specified directory
//! infs new myproject --template default # Use specific template
//! ```
//!
//! ## Project Structure
//!
//! The default template creates the following structure:
//!
//! ```text
//! myproject/
//! +-- Inference.toml
//! +-- src/
//! |   +-- main.inf
//! +-- tests/
//! |   +-- .gitkeep
//! +-- proofs/
//! |   +-- .gitkeep
//! +-- .gitignore
//! ```

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use crate::project::{available_templates, create_project, get_template};

/// Arguments for the `new` command.
#[derive(Args)]
pub struct NewArgs {
    /// Name of the project to create.
    ///
    /// Must start with a letter or underscore and contain only
    /// alphanumeric characters, underscores, or hyphens.
    /// Cannot be a reserved Inference keyword.
    pub name: String,

    /// Parent directory for the project (defaults to current directory).
    #[clap(default_value = ".")]
    pub path: PathBuf,

    /// Project template to use.
    ///
    /// Templates define the initial file structure and content.
    /// Use `--list-templates` to see available templates.
    #[clap(long, default_value = "default")]
    pub template: String,

    /// Skip git repository initialization.
    ///
    /// By default, `infs new` initializes a git repository in the
    /// new project directory. Use this flag to create a project
    /// without git.
    #[clap(long = "no-git", action = clap::ArgAction::SetTrue)]
    pub no_git: bool,

    /// List available project templates.
    #[clap(long = "list-templates", action = clap::ArgAction::SetTrue)]
    pub list_templates: bool,
}

/// Executes the `new` command.
///
/// Creates a new Inference project with the standard directory structure.
///
/// # Errors
///
/// Returns an error if:
/// - The project name is invalid (reserved word or invalid characters)
/// - The target directory already exists
/// - The template is not found
/// - File creation fails
pub fn execute(args: &NewArgs) -> Result<()> {
    if args.list_templates {
        println!("Available templates:");
        println!();
        for (name, description) in available_templates() {
            println!("  {name:<12} - {description}");
        }
        return Ok(());
    }

    let template = get_template(&args.template).ok_or_else(|| {
        let available: Vec<_> = available_templates()
            .iter()
            .map(|(name, _)| format!("  - {name}"))
            .collect();
        anyhow::anyhow!(
            "Unknown template '{}'. Available templates:\n{}",
            args.template,
            available.join("\n")
        )
    })?;

    let init_git = !args.no_git;
    let parent = if args.path.as_os_str() == "." {
        None
    } else {
        Some(args.path.as_path())
    };

    let project_path = create_project(&args.name, parent, template.as_ref(), init_git)?;

    println!("Created project '{}' using template '{}'", args.name, args.template);
    println!();
    println!("Next steps:");
    println!("  cd {}", project_path.display());
    println!("  infs build src/main.inf --codegen -o");
    println!();
    println!("To learn more about Inference, visit:");
    println!("  https://inferara.com");

    Ok(())
}
