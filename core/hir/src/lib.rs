#![warn(clippy::pedantic)]
pub mod hir;
mod nodes;
mod nodes_impl;
mod symbol_table;
// mod type_inference;
mod arena;
pub mod type_info;
