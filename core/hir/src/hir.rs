use crate::{arena::Arena, symbol_table::SymbolTable};
use inference_ast::arena::Arena as AstArena;

#[derive(Clone, Default)]
pub struct Hir {
    pub arena: Arena,
    pub symbol_table: SymbolTable,
}

impl Hir {
    #[must_use]
    pub fn new(arena: AstArena) -> Self {
        Self {
            arena: Arena::default(),
            symbol_table: SymbolTable::default(),
        }
    }
}
