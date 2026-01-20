#![warn(clippy::pedantic)]

//! Default project template.
//!
//! The default template creates a minimal Inference project with the following
//! structure:
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

use std::path::PathBuf;

use super::{ProjectTemplate, TemplateFile};

/// The default Inference project template.
///
/// Creates a minimal project with standard directory structure:
/// - `Inference.toml` manifest file
/// - `src/main.inf` entry point
/// - `tests/` and `proofs/` directories with `.gitkeep` files
/// - `.gitignore` with common exclusions
pub struct DefaultTemplate;

impl ProjectTemplate for DefaultTemplate {
    fn name(&self) -> &'static str {
        "default"
    }

    fn description(&self) -> &'static str {
        "A minimal Inference project"
    }

    fn files(&self, project_name: &str) -> Vec<TemplateFile> {
        vec![
            TemplateFile::new("Inference.toml", manifest_content(project_name)),
            TemplateFile::new(
                PathBuf::from("src").join("main.inf"),
                main_inf_content(),
            ),
            TemplateFile::new(PathBuf::from("tests").join(".gitkeep"), String::new()),
            TemplateFile::new(PathBuf::from("proofs").join(".gitkeep"), String::new()),
            TemplateFile::new(".gitignore", gitignore_content()),
        ]
    }
}

/// Generates the content for `Inference.toml`.
fn manifest_content(project_name: &str) -> String {
    format!(
        r#"[package]
name = "{project_name}"
version = "0.1.0"
edition = "2024"
manifest_version = 1

# Optional fields:
# description = "A brief description of the project"
# authors = ["Your Name <you@example.com>"]
# license = "MIT"

# [dependencies]
# Future: package dependencies
# std = "0.1"

# [build]
# target = "wasm32"
# optimize = "release"

# [verification]
# output-dir = "proofs/"
"#
    )
}

/// Generates the content for `src/main.inf`.
fn main_inf_content() -> String {
    String::from(
        r"// Entry point for the Inference program

fn main() -> i32 {
    return 0;
}
",
    )
}

/// Generates the content for `.gitignore`.
fn gitignore_content() -> String {
    String::from(
        r"# Build outputs
/out/
/target/

# IDE and editor files
.idea/
.vscode/
*.swp
*.swo
*~

# OS files
.DS_Store
Thumbs.db
",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_template_name() {
        let template = DefaultTemplate;
        assert_eq!(template.name(), "default");
    }

    #[test]
    fn test_default_template_description() {
        let template = DefaultTemplate;
        assert!(!template.description().is_empty());
    }

    #[test]
    fn test_default_template_creates_all_files() {
        let template = DefaultTemplate;
        let files = template.files("test_project");

        let paths: Vec<_> = files
            .iter()
            .map(|f| f.path.to_string_lossy().to_string())
            .collect();

        assert!(paths.iter().any(|p| p == "Inference.toml"));
        assert!(paths.iter().any(|p| p.ends_with("main.inf")));
        assert!(paths.iter().any(|p| p.contains("tests")));
        assert!(paths.iter().any(|p| p.contains("proofs")));
        assert!(paths.iter().any(|p| p == ".gitignore"));
    }

    #[test]
    fn test_manifest_contains_project_name() {
        let template = DefaultTemplate;
        let files = template.files("my_awesome_project");
        let manifest = files
            .iter()
            .find(|f| f.path.to_string_lossy() == "Inference.toml")
            .unwrap();

        assert!(manifest.content.contains("my_awesome_project"));
        assert!(manifest.content.contains("version = \"0.1.0\""));
        assert!(manifest.content.contains("edition = \"2024\""));
        assert!(manifest.content.contains("manifest_version = 1"));
    }

    #[test]
    fn test_main_inf_has_entry_point() {
        let template = DefaultTemplate;
        let files = template.files("project");
        let main = files
            .iter()
            .find(|f| f.path.to_string_lossy().ends_with("main.inf"))
            .unwrap();

        assert!(main.content.contains("fn main()"));
        assert!(main.content.contains("return"));
    }

    #[test]
    fn test_gitignore_excludes_build_dirs() {
        let template = DefaultTemplate;
        let files = template.files("project");
        let gitignore = files
            .iter()
            .find(|f| f.path.to_string_lossy() == ".gitignore")
            .unwrap();

        assert!(gitignore.content.contains("/out/"));
        assert!(gitignore.content.contains("/target/"));
    }

    #[test]
    fn test_gitkeep_files_are_empty() {
        let template = DefaultTemplate;
        let files = template.files("project");

        for file in &files {
            if file.path.to_string_lossy().ends_with(".gitkeep") {
                assert!(file.content.is_empty(), "gitkeep should be empty");
            }
        }
    }
}
