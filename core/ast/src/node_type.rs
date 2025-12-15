//! AST node types for Rust type annotations.
//!
//! Defines the `NodeType` enum representing parsed type expressions such as paths,
//! references, pointers, tuples, arrays, and more.

use std::rc::Rc;

use crate::nodes::{
    Definition, Directive, EnumDefinition, Expression, FunctionCallExpression, FunctionDefinition,
    Literal, Location, SourceFile, Statement, StructDefinition, Type,
};
pub type RcFile = Rc<SourceFile>;
pub type RcContract = Rc<StructDefinition>;
pub type RcFunction = Rc<FunctionDefinition>;
pub type RcExpression = Rc<Expression>;
pub type RcFunctionCall = Rc<FunctionCallExpression>;

pub type RcEnum = Rc<EnumDefinition>;
pub type RcStruct = Rc<StructDefinition>;

#[derive(Clone, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum NodeType {
    #[default]
    Empty,
    /// A named type or path, including any generics as represented in the token stream
    Path(String),
    /// A reference `&T` or `&mut T`, with explicit flag
    Reference {
        inner: Box<NodeType>,
        mutable: bool,
        is_explicit_reference: bool,
    },
    /// A raw pointer `*const T` or `*mut T`
    Ptr { inner: Box<NodeType>, mutable: bool },
    /// A tuple type `(T1, T2, ...)`
    Tuple(Vec<NodeType>),
    /// An array type `[T; len]`, with optional length if parseable
    Array {
        inner: Box<NodeType>,
        len: Option<usize>,
    },
    /// A slice type `[T]`
    Slice(Box<NodeType>),
    /// A bare function pointer `fn(a, b) -> R`
    BareFn {
        inputs: Vec<NodeType>,
        output: Box<NodeType>,
    },
    /// A generic type annotation, e.g., `Option<T>`, `Result<A, B>`
    Generic {
        base: Box<NodeType>,
        args: Vec<NodeType>,
    },
    /// A trait object type `dyn Trait1 + Trait2`
    TraitObject(Vec<String>),
    /// An `impl Trait` type
    ImplTrait(Vec<String>),
    Closure {
        inputs: Vec<NodeType>,
        output: Box<NodeType>,
    },
}

impl NodeType {
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            NodeType::Path(name) => name.clone(),
            NodeType::Reference {
                inner,
                is_explicit_reference,
                ..
            } => {
                if *is_explicit_reference {
                    format!("&{}", inner.name())
                } else {
                    inner.name()
                }
            }
            NodeType::Ptr { inner, mutable } => {
                let star = if *mutable { "*mut" } else { "*const" };
                format!("{} {}", star, inner.name())
            }
            NodeType::Tuple(elems) => format!(
                "({})",
                elems
                    .iter()
                    .map(NodeType::name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            NodeType::Array { inner, len } => format!(
                "[{}; {}]",
                inner.name(),
                len.map_or("..".to_string(), |l| l.to_string())
            ),
            NodeType::Slice(inner) => format!("[{}]", inner.name()),
            NodeType::BareFn { inputs, output } => {
                let mut result = inputs
                    .iter()
                    .map(NodeType::name)
                    .collect::<Vec<_>>()
                    .join(", ");
                if result.is_empty() {
                    result = "_".to_string();
                }
                let output = if output.name().is_empty() {
                    "_".to_string()
                } else {
                    output.name()
                };
                format!("fn({result}) -> {output}")
            }
            NodeType::Closure { inputs, output } => {
                let mut result = inputs
                    .iter()
                    .map(NodeType::name)
                    .collect::<Vec<_>>()
                    .join(", ");
                if result.is_empty() {
                    result = "_".to_string();
                }
                let output = if output.name().is_empty() {
                    "_".to_string()
                } else {
                    output.name()
                };
                format!("{result} || -> {output}")
            }
            NodeType::Generic { base, args } => format!(
                "{}<{}>",
                base.name(),
                args.iter()
                    .map(NodeType::name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            NodeType::TraitObject(bounds) => format!("dyn {}", bounds.join(" + ")),
            NodeType::ImplTrait(bounds) => format!("impl {}", bounds.join(" + ")),
            NodeType::Empty => String::from("_"),
        }
    }

    #[must_use]
    pub fn pure_name(&self) -> String {
        match self {
            NodeType::Path(name) => name.clone(),
            NodeType::Reference { inner, .. }
            | NodeType::Ptr { inner, .. }
            | NodeType::Array { inner, len: _ }
            | NodeType::Slice(inner) => inner.pure_name(),
            NodeType::Tuple(elems) => format!(
                "({})",
                elems
                    .iter()
                    .map(NodeType::pure_name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            NodeType::BareFn { inputs, output } => {
                let mut result = inputs
                    .iter()
                    .map(NodeType::pure_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                if result.is_empty() {
                    result = "_".to_string();
                }
                let output = output.pure_name();
                format!("fn({result}) -> {output}")
            }
            NodeType::Closure { inputs, output } => {
                let mut result = inputs
                    .iter()
                    .map(NodeType::pure_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                if result.is_empty() {
                    result = "_".to_string();
                }
                let output = output.pure_name();
                format!("{result} || -> {output}")
            }
            NodeType::Generic { base, args } => format!(
                "{}<{}>",
                base.pure_name(),
                args.iter()
                    .map(NodeType::pure_name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            NodeType::TraitObject(bounds) | NodeType::ImplTrait(bounds) => bounds.join(" + "),
            NodeType::Empty => String::from("_"),
        }
    }

    #[must_use]
    pub fn is_self(&self) -> bool {
        match self {
            NodeType::Path(name) => name.to_lowercase() == "self",
            NodeType::Reference { inner, .. }
            | NodeType::Ptr { inner, .. }
            | NodeType::Array { inner, .. }
            | NodeType::Slice(inner) => inner.is_self(),
            NodeType::Tuple(elems) => elems.iter().any(NodeType::is_self),
            NodeType::BareFn { inputs, output } | NodeType::Closure { inputs, output } => {
                inputs.iter().any(NodeType::is_self) || output.is_self()
            }
            NodeType::Generic { base, args } => {
                base.is_self() || args.iter().any(NodeType::is_self)
            }
            NodeType::TraitObject(bounds) | NodeType::ImplTrait(bounds) => {
                bounds.iter().any(|b| b.to_lowercase() == "self")
            }
            NodeType::Empty => false,
        }
    }

    #[allow(clippy::assigning_clones)]
    pub fn replace_path(&mut self, new_path: String) {
        match self {
            NodeType::Path(_) => {
                *self = NodeType::Path(new_path);
            }
            NodeType::Reference { inner, .. }
            | NodeType::Ptr { inner, .. }
            | NodeType::Array { inner, .. }
            | NodeType::Slice(inner) => {
                inner.replace_path(new_path);
            }
            NodeType::Tuple(elems) => {
                for elem in elems {
                    elem.replace_path(new_path.clone());
                }
            }
            NodeType::BareFn { inputs, output } | NodeType::Closure { inputs, output } => {
                for input in inputs {
                    input.replace_path(new_path.clone());
                }
                output.replace_path(new_path);
            }
            NodeType::Generic { base, args } => {
                base.replace_path(new_path.clone());
                for arg in args {
                    arg.replace_path(new_path.clone());
                }
            }
            NodeType::TraitObject(bounds) | NodeType::ImplTrait(bounds) => {
                for bound in bounds.iter_mut() {
                    if bound.to_lowercase() == "self" {
                        *bound = new_path.clone();
                    }
                }
            }
            NodeType::Empty => {}
        }
    }
}

#[derive(Clone, Debug)]
pub enum NodeKind {
    File(Rc<SourceFile>),
    Directive(Directive),
    Definition(Definition),
    Statement(Statement),
    Expression(Expression),
    Literal(Literal),
    Type(Type),
}

impl NodeKind {
    #[must_use]
    pub fn id(&self) -> u32 {
        match self {
            NodeKind::File(f) => f.id,
            NodeKind::Definition(d) => d.id(),
            NodeKind::Directive(d) => d.id(),
            NodeKind::Statement(s) => s.id(),
            NodeKind::Expression(e) => e.id(),
            NodeKind::Literal(l) => l.id(),
            NodeKind::Type(t) => t.id(),
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn location(&self) -> Location {
        match self {
            NodeKind::File(f) => f.location().clone(),
            NodeKind::Definition(d) => d.location().clone(),
            NodeKind::Directive(d) => d.location(),
            NodeKind::Statement(s) => s.location(),
            NodeKind::Expression(e) => e.location(),
            NodeKind::Literal(l) => l.location(),
            NodeKind::Type(t) => t.location(),
        }
    }

    #[must_use]
    pub fn children(&self) -> Vec<NodeKind> {
        match self {
            NodeKind::File(file) => file.children(),
            NodeKind::Definition(definition) => definition.children(),
            NodeKind::Directive(directive) => directive.children(),
            NodeKind::Statement(statement) => statement.children(),
            NodeKind::Expression(expression) => expression.children(),
            NodeKind::Literal(literal) => literal.children(),
            NodeKind::Type(ty) => ty.children(),
        }
    }
}
