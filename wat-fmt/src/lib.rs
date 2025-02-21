#![no_std]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

enum Token {
    LParen,
    RParen,
    Atom(String),
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        } else if c == '(' {
            tokens.push(Token::LParen);
        } else if c == ')' {
            tokens.push(Token::RParen);
        } else if c == '"' {
            let mut s = String::new();
            s.push('"');
            while let Some(&next) = chars.peek() {
                s.push(next);
                chars.next();
                if next == '"' {
                    break;
                }
            }
            tokens.push(Token::Atom(s));
        } else {
            let mut s = String::new();
            s.push(c);
            while let Some(&next) = chars.peek() {
                if next.is_whitespace() || next == '(' || next == ')' {
                    break;
                }
                s.push(next);
                chars.next();
            }
            tokens.push(Token::Atom(s));
        }
    }

    tokens
}

enum Node {
    Atom(String),
    List(Vec<Node>),
}

fn parse_node(tokens: &[Token], mut i: usize) -> (Node, usize) {
    if i >= tokens.len() {
        return (Node::Atom(String::new()), i);
    }
    match &tokens[i] {
        Token::LParen => {
            i += 1;
            let mut children = Vec::new();
            while i < tokens.len() {
                match tokens[i] {
                    Token::RParen => {
                        i += 1; // consume the RParen
                        break;
                    }
                    _ => {
                        let (child, new_i) = parse_node(tokens, i);
                        children.push(child);
                        i = new_i;
                    }
                }
            }
            (Node::List(children), i)
        }
        Token::RParen => (Node::Atom(String::from(")")), i + 1),
        Token::Atom(ref s) => (Node::Atom(s.clone()), i + 1),
    }
}

fn parse_all(tokens: &[Token]) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let (node, new_i) = parse_node(tokens, i);
        nodes.push(node);
        i = new_i;
    }
    nodes
}

fn indent_str(indent: usize) -> String {
    let mut s = String::new();
    for _ in 0..indent {
        s.push_str("  ");
    }
    s
}

fn is_flat_node(node: &Node) -> bool {
    match node {
        Node::Atom(_) => true,
        Node::List(children) => children.iter().all(is_flat_node),
    }
}

fn is_flat_list(nodes: &[Node]) -> bool {
    nodes.iter().all(is_flat_node)
}

fn format_node_inline(node: &Node) -> String {
    match node {
        Node::Atom(s) => s.clone(),
        Node::List(children) => {
            let mut s = String::new();
            s.push('(');
            let mut first = true;
            for child in children {
                if !first {
                    s.push(' ');
                }
                s.push_str(&format_node_inline(child));
                first = false;
            }
            s.push(')');
            s
        }
    }
}

#[allow(dead_code)]
fn format_inline(node: &Node) -> Option<String> {
    if is_flat_node(node) {
        Some(format_node_inline(node))
    } else {
        None
    }
}

fn is_inline_signature(node: &Node) -> bool {
    if let Node::List(children) = node {
        if let Some(Node::Atom(ref keyword)) = children.first() {
            return keyword == "export" || keyword == "param" || keyword == "result";
        }
    }
    false
}

fn is_opcode(token: &str) -> bool {
    if token.starts_with('$') {
        return false;
    }
    if token.starts_with('"') {
        return false;
    }
    let mut chars = token.chars();
    if let Some(first) = chars.next() {
        if (first == '-' || first == '+') && chars.clone().all(|c| c.is_ascii_digit()) {
            return false;
        }
        if first.is_ascii_digit() && token.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    true
}

fn format_instructions(nodes: &[Node], indent: usize) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i < nodes.len() {
        match &nodes[i] {
            Node::Atom(token) => {
                if is_opcode(token) {
                    let mut line = token.clone();
                    i += 1;
                    while i < nodes.len() {
                        if let Node::Atom(next_token) = &nodes[i] {
                            if is_opcode(next_token) {
                                break;
                            } else {
                                line.push(' ');
                                line.push_str(next_token);
                                i += 1;
                            }
                        } else {
                            break;
                        }
                    }
                    result.push('\n');
                    result.push_str(&indent_str(indent));
                    result.push_str(&line);
                } else {
                    result.push('\n');
                    result.push_str(&indent_str(indent));
                    result.push_str(token);
                    i += 1;
                }
            }
            Node::List(_) => {
                result.push('\n');
                result.push_str(&indent_str(indent));
                result.push_str(&format_node(&nodes[i], indent));
                i += 1;
            }
        }
    }
    result
}

fn format_node(node: &Node, indent: usize) -> String {
    match node {
        Node::Atom(s) => s.clone(),
        Node::List(children) => {
            if children.is_empty() {
                return String::from("()");
            }
            if let Some(Node::Atom(ref ident)) = children.first() {
                if ident == "module" {
                    let mut s = String::new();
                    s.push('(');
                    s.push_str(ident);
                    for child in children.iter().skip(1) {
                        s.push('\n');
                        s.push_str(&indent_str(indent + 1));
                        s.push_str(&format_node(child, indent + 1));
                    }
                    s.push('\n');
                    s.push_str(&indent_str(indent));
                    s.push(')');
                    return s;
                } else if ident == "func" {
                    let mut s = String::new();
                    s.push('(');
                    s.push_str(&format_node_inline(&children[0]));
                    let mut i = 1;
                    while i < children.len() {
                        match &children[i] {
                            Node::Atom(_) => {
                                s.push(' ');
                                s.push_str(&format_node_inline(&children[i]));
                                i += 1;
                            }
                            Node::List(_) if is_inline_signature(&children[i]) => {
                                s.push(' ');
                                s.push_str(&format_node_inline(&children[i]));
                                i += 1;
                            }
                            _ => break,
                        }
                    }
                    s.push_str(&format_instructions(&children[i..], indent + 1));
                    s.push('\n');
                    s.push_str(&indent_str(indent));
                    s.push(')');
                    return s;
                } else if ["forall", "exists", "assume", "unique"].contains(&ident.as_str()) {
                    let mut s = String::new();
                    s.push('(');
                    s.push_str(ident);
                    s.push_str(&format_instructions(&children[1..], indent + 1));
                    s.push('\n');
                    s.push_str(&indent_str(indent));
                    s.push(')');
                    return s;
                }
            }
            if is_flat_list(children) {
                format_node_inline(node)
            } else {
                let mut s = String::new();
                s.push('(');
                let mut first = true;
                for child in children {
                    if first {
                        s.push_str(&format_node(child, indent + 1));
                        first = false;
                    } else {
                        s.push('\n');
                        s.push_str(&indent_str(indent + 1));
                        s.push_str(&format_node(child, indent + 1));
                    }
                }
                s.push('\n');
                s.push_str(&indent_str(indent));
                s.push(')');
                s
            }
        }
    }
}

pub fn format(input: &str) -> String {
    let tokens = tokenize(input);
    let nodes = parse_all(&tokens);
    if nodes.len() == 1 {
        format_node(&nodes[0], 0)
    } else {
        let mut s = String::new();
        for node in nodes {
            s.push_str(&format_node(&node, 0));
            s.push('\n');
        }
        s
    }
}
