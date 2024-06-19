#![warn(clippy::pedantic)]

mod ast;

use ast::builder::build_ast;
use std::{env, fs, process};

fn main() {
    if env::args().len() != 1 {
        eprintln!("One argument is expected: the source file path");
        process::exit(1);
    }

    let source_file_path = env::args().nth(1).unwrap();
    parse(&source_file_path);
}

fn parse(source_file_path: &str) {
    let inference_language = tree_sitter_inference::language();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&inference_language)
        .expect("Error loading Inference grammar");
    let text = fs::read_to_string(source_file_path).expect("Error reading source file");

    let tree = parser.parse(text, None).unwrap();
    let ast = build_ast(tree.root_node());
}

mod test {

    #[test]
    fn test_parse() {
        let current_dir = std::env::current_dir().unwrap();
        let path = current_dir.join("tests/example.inf");
        let absolute_path = path.canonicalize().unwrap();

        super::parse(absolute_path.to_str().unwrap());
    }
}
