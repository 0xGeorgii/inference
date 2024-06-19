#![warn(clippy::pedantic)]

use crate::ast::types::{SourceFile, UseDirective};
use tree_sitter::Node;

pub fn build_ast(root: Node) -> SourceFile {
    assert!(
        root.kind() == "source_file",
        "Expected a root node of type {}",
        "source_file"
    );

    let mut ast = SourceFile::new(
        root.start_position().row,
        root.start_position().column,
        root.end_position().row,
        root.end_position().column,
    );

    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        let child_kind = child.kind();

        match child_kind {
            "use_directive" => build_use_directive(&mut ast, &child),
            _ => panic!("{}", format!("Unexpected child of type {child_kind}")),
        };
    }
    ast
}

fn build_use_directive(parent: &mut SourceFile, node: &Node) {
    let use_directive = UseDirective::new(
        node.start_position().row,
        node.start_position().column,
        node.end_position().row,
        node.end_position().column,
    );

    parent.add_use_directive(use_directive);
}
