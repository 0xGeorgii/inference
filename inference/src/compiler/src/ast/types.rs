pub struct Position {
    pub row: usize,
    pub column: usize,
}

pub struct Location {
    pub start: Position,
    pub end: Position,
}

pub struct SourceFile {
    location: Location,
    use_directives: Vec<UseDirective>,
}

impl SourceFile {
    pub fn new(start_row: usize, start_column: usize, end_row: usize, end_column: usize) -> Self {
        SourceFile {
            location: Location {
                start: Position {
                    row: start_row,
                    column: start_column,
                },
                end: Position {
                    row: end_row,
                    column: end_column,
                },
            },
            use_directives: Vec::new(),
        }
    }

    pub fn add_use_directive(&mut self, use_directive: UseDirective) {
        self.use_directives.push(use_directive);
    }
}

pub struct UseDirective {
    location: Location,
}

impl UseDirective {
    pub fn new(start_row: usize, start_column: usize, end_row: usize, end_column: usize) -> Self {
        UseDirective {
            location: Location {
                start: Position {
                    row: start_row,
                    column: start_column,
                },
                end: Position {
                    row: end_row,
                    column: end_column,
                },
            },
        }
    }
}

pub struct Identifier {
    location: Location,
    name: String,
}

impl Identifier {
    pub fn new(
        start_row: usize,
        start_column: usize,
        end_row: usize,
        end_column: usize,
        name: String,
    ) -> Self {
        Identifier {
            location: Location {
                start: Position {
                    row: start_row,
                    column: start_column,
                },
                end: Position {
                    row: end_row,
                    column: end_column,
                },
            },
            name,
        }
    }
}
