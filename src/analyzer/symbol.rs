use crate::parser::ast::TypeKind;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Symbol {
    Variable {
        name: String,
        type_kind: TypeKind,
        mutable: bool,
    },
    Function {
        name: String,
        parameters: Vec<TypeKind>,
        return_type: Option<TypeKind>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Scope {
    symbols: HashMap<String, Symbol>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new(parent: Option<Box<Scope>>) -> Self {
        Scope {
            symbols: HashMap::new(),
            parent,
        }
    }

    pub fn declare(&mut self, name: &str, symbol: Symbol) -> Result<(), String> {
        if self.symbols.contains_key(name) {
            return Err(format!("Symbol '{}' already declared in this scope.", name));
        }
        self.symbols.insert(name.to_string(), symbol);
        Ok(())
    }

    pub fn find(&self, name: &str) -> Option<&Symbol> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.find(name)))
    }
}
