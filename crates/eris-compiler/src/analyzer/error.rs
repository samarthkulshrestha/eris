use crate::lexer::SourceLocation;
use crate::parser::ast::TypeKind;

#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzeError {
    pub kind: AnalyzeErrorKind,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnalyzeErrorKind {
    UndeclaredVariable(String),
    UndeclaredFunction(String),
    TypeMismatch {
        expected: TypeKind,
        found: TypeKind,
    },
    IncorrectArgumentCount {
        expected: usize,
        found: usize,
    },
    VariableAlreadyDeclared(String),
    AssignmentToImmutable(String),
    BreakOutsideLoop,
    ContinueOutsideLoop,
    MissingReturn(String),
    ReturnMissingValue { function: String },
    NonIterableInForLoop {
        found: TypeKind,
    },
    IndexOutOfRange {
        array_size: usize,
        index: i64,
    },
    IndexNotInteger {
        found: TypeKind,
    },
    TakeRefOfNonLvalue,
    TakeMutRefOfImmutable {
        name: String,
    },
    DerefOfNonReference {
        found: TypeKind,
    },
    ReturnsLocalReference,
    UnsupportedOperandType {
        op: &'static str,
        operand_type: TypeKind,
    },
    DiscordDependency {
        name: String,
    },
    DiscordImpurity {
        what: String,
    },
}

impl AnalyzeError {
    pub fn new(kind: AnalyzeErrorKind, location: SourceLocation) -> Self {
        AnalyzeError { kind, location }
    }
}

impl From<AnalyzeError> for crate::diagnostics::Diagnostic {
    fn from(err: AnalyzeError) -> Self {
        use crate::diagnostics::{Diagnostic, ErrorCode, Severity};
        use crate::error_display::FormattedError;
        use serde_json::{Map, Value};

        let (code, message, fields) = match &err.kind {
            AnalyzeErrorKind::UndeclaredVariable(name) => {
                let mut m = Map::new();
                m.insert("name".to_string(), Value::String(name.clone()));
                (
                    ErrorCode::E0201,
                    format!("undeclared variable '{}'", name),
                    m,
                )
            }
            AnalyzeErrorKind::UndeclaredFunction(name) => {
                let mut m = Map::new();
                m.insert("name".to_string(), Value::String(name.clone()));
                (
                    ErrorCode::E0202,
                    format!("undeclared function '{}'", name),
                    m,
                )
            }
            AnalyzeErrorKind::TypeMismatch { expected, found } => {
                let mut m = Map::new();
                m.insert("expected".to_string(), Value::String(expected.to_string()));
                m.insert("found".to_string(), Value::String(found.to_string()));
                (
                    ErrorCode::E0203,
                    format!("type mismatch: expected '{}', found '{}'", expected, found),
                    m,
                )
            }
            AnalyzeErrorKind::IncorrectArgumentCount { expected, found } => {
                let mut m = Map::new();
                m.insert(
                    "expected".to_string(),
                    Value::Number((*expected).into()),
                );
                m.insert("found".to_string(), Value::Number((*found).into()));
                (
                    ErrorCode::E0204,
                    format!("incorrect argument count: expected {}, found {}", expected, found),
                    m,
                )
            }
            AnalyzeErrorKind::VariableAlreadyDeclared(name) => {
                let mut m = Map::new();
                m.insert("name".to_string(), Value::String(name.clone()));
                (
                    ErrorCode::E0205,
                    format!("variable '{}' already declared in this scope", name),
                    m,
                )
            }
            AnalyzeErrorKind::AssignmentToImmutable(name) => {
                let mut m = Map::new();
                m.insert("name".to_string(), Value::String(name.clone()));
                (
                    ErrorCode::E0206,
                    format!("cannot assign to immutable variable '{}'", name),
                    m,
                )
            }
            AnalyzeErrorKind::BreakOutsideLoop => {
                (ErrorCode::E0207, "break outside loop".to_string(), Map::new())
            }
            AnalyzeErrorKind::ContinueOutsideLoop => {
                (ErrorCode::E0208, "continue outside loop".to_string(), Map::new())
            }
            AnalyzeErrorKind::MissingReturn(name) => {
                let mut fields = Map::new();
                fields.insert("function".to_string(), Value::String(name.clone()));
                (
                    ErrorCode::E0209,
                    format!("function '{}' may finish without returning a value", name),
                    fields,
                )
            }
            AnalyzeErrorKind::ReturnMissingValue { function } => {
                let mut fields = Map::new();
                fields.insert("function".to_string(), Value::String(function.clone()));
                (
                    ErrorCode::E0219,
                    format!("function '{}' returns without a value but is declared to return a value", function),
                    fields,
                )
            }
            AnalyzeErrorKind::NonIterableInForLoop { found } => {
                let mut fields = Map::new();
                fields.insert("found".to_string(), Value::String(found.to_string()));
                (
                    ErrorCode::E0220,
                    format!("expected a range in for-in, found '{}'", found),
                    fields,
                )
            }
            AnalyzeErrorKind::IndexOutOfRange { array_size, index } => {
                let mut fields = Map::new();
                fields.insert(
                    "array_size".to_string(),
                    Value::Number((*array_size).into()),
                );
                fields.insert("index".to_string(), Value::Number((*index).into()));
                (
                    ErrorCode::E0221,
                    format!(
                        "index {} is out of range for array of size {}",
                        index, array_size
                    ),
                    fields,
                )
            }
            AnalyzeErrorKind::IndexNotInteger { found } => {
                let mut fields = Map::new();
                fields.insert("found".to_string(), Value::String(found.to_string()));
                (
                    ErrorCode::E0222,
                    format!("array index must be 'int', found '{}'", found),
                    fields,
                )
            }
            AnalyzeErrorKind::TakeRefOfNonLvalue => (
                ErrorCode::E0223,
                "cannot take a reference to a temporary value".to_string(),
                Map::new(),
            ),
            AnalyzeErrorKind::TakeMutRefOfImmutable { name } => {
                let mut fields = Map::new();
                fields.insert("name".to_string(), Value::String(name.clone()));
                (
                    ErrorCode::E0224,
                    format!("cannot take a mutable reference to immutable variable '{}'", name),
                    fields,
                )
            }
            AnalyzeErrorKind::DerefOfNonReference { found } => {
                let mut fields = Map::new();
                fields.insert("found".to_string(), Value::String(found.to_string()));
                (
                    ErrorCode::E0225,
                    format!("cannot dereference value of type '{}'", found),
                    fields,
                )
            }
            AnalyzeErrorKind::ReturnsLocalReference => (
                ErrorCode::E0226,
                "function returns a reference to one of its own local variables".to_string(),
                Map::new(),
            ),
            AnalyzeErrorKind::UnsupportedOperandType { op, operand_type } => {
                let mut fields = Map::new();
                fields.insert("op".to_string(), Value::String(op.to_string()));
                fields.insert(
                    "operand_type".to_string(),
                    Value::String(operand_type.to_string()),
                );
                (
                    ErrorCode::E0227,
                    format!(
                        "operator '{}' is not supported for operands of type '{}'",
                        op, operand_type
                    ),
                    fields,
                )
            }
            AnalyzeErrorKind::DiscordDependency { name } => {
                let mut fields = Map::new();
                fields.insert("name".to_string(), Value::String(name.clone()));
                (
                    ErrorCode::E0228,
                    format!("discord block statements are not independent: more than one statement touches '{}'", name),
                    fields,
                )
            }
            AnalyzeErrorKind::DiscordImpurity { what } => {
                let mut fields = Map::new();
                fields.insert("what".to_string(), Value::String(what.clone()));
                (
                    ErrorCode::E0229,
                    format!("'{}' is not permitted inside a discord block", what),
                    fields,
                )
            }
        };

        Diagnostic {
            code,
            severity: Severity::Error,
            message,
            location: err.location.clone(),
            labels: vec![],
            hint: err.hint(),
            fields,
        }
    }
}

impl crate::error_display::FormattedError for AnalyzeError {
    fn error_type(&self) -> &str {
        match &self.kind {
            AnalyzeErrorKind::UndeclaredVariable(_) => "undeclared variable",
            AnalyzeErrorKind::UndeclaredFunction(_) => "undeclared function",
            AnalyzeErrorKind::TypeMismatch { .. } => "type mismatch",
            AnalyzeErrorKind::IncorrectArgumentCount { .. } => "incorrect argument count",
            AnalyzeErrorKind::VariableAlreadyDeclared(_) => "variable already declared",
            AnalyzeErrorKind::AssignmentToImmutable(_) => "assignment to immutable variable",
            AnalyzeErrorKind::BreakOutsideLoop => "break outside loop",
            AnalyzeErrorKind::ContinueOutsideLoop => "continue outside loop",
            AnalyzeErrorKind::MissingReturn(_) => "missing return",
            AnalyzeErrorKind::ReturnMissingValue { .. } => "return without value",
            AnalyzeErrorKind::NonIterableInForLoop { .. } => "non-iterable in for-in",
            AnalyzeErrorKind::IndexOutOfRange { .. } => "index out of range",
            AnalyzeErrorKind::IndexNotInteger { .. } => "non-integer array index",
            AnalyzeErrorKind::TakeRefOfNonLvalue => "take-reference of non-lvalue",
            AnalyzeErrorKind::TakeMutRefOfImmutable { .. } => "mutable reference to immutable",
            AnalyzeErrorKind::DerefOfNonReference { .. } => "dereference of non-reference",
            AnalyzeErrorKind::ReturnsLocalReference => "returns reference to local",
            AnalyzeErrorKind::UnsupportedOperandType { .. } => "unsupported operand type",
            AnalyzeErrorKind::DiscordDependency { .. } => "discord dependency",
            AnalyzeErrorKind::DiscordImpurity { .. } => "discord impurity",
        }
    }

    fn message(&self) -> String {
        match &self.kind {
            AnalyzeErrorKind::UndeclaredVariable(name) => {
                format!("Cannot find variable '{}' in this scope", name)
            }
            AnalyzeErrorKind::UndeclaredFunction(name) => {
                format!("Cannot find function '{}' in this scope", name)
            }
            AnalyzeErrorKind::TypeMismatch { expected, found } => {
                format!("Expected type '{}', found '{}'", expected, found)
            }
            AnalyzeErrorKind::IncorrectArgumentCount { expected, found } => {
                format!("Expected {} arguments, found {}", expected, found)
            }
            AnalyzeErrorKind::VariableAlreadyDeclared(name) => {
                format!("Variable '{}' is already declared in this scope", name)
            }
            AnalyzeErrorKind::AssignmentToImmutable(name) => {
                format!("Cannot assign to immutable variable '{}'", name)
            }
            AnalyzeErrorKind::BreakOutsideLoop => "break outside loop".to_string(),
            AnalyzeErrorKind::ContinueOutsideLoop => "continue outside loop".to_string(),
            AnalyzeErrorKind::MissingReturn(name) => {
                format!("function '{}' may finish without returning a value", name)
            }
            AnalyzeErrorKind::ReturnMissingValue { function } => {
                format!("function '{}' returns without a value but is declared to return a value", function)
            }
            AnalyzeErrorKind::NonIterableInForLoop { found } => {
                format!("for-in requires a range expression, but found '{}'", found)
            }
            AnalyzeErrorKind::IndexOutOfRange { array_size, index } => {
                format!("index {} is out of range for array of size {}", index, array_size)
            }
            AnalyzeErrorKind::IndexNotInteger { found } => {
                format!("array indices must be of type 'int', found '{}'", found)
            }
            AnalyzeErrorKind::TakeRefOfNonLvalue => {
                "Cannot take a reference to a temporary value; references must be to named storage".to_string()
            }
            AnalyzeErrorKind::TakeMutRefOfImmutable { name } => {
                format!("Cannot take a mutable reference to '{}' because it is not declared with 'mut'", name)
            }
            AnalyzeErrorKind::DerefOfNonReference { found } => {
                format!("The '*' operator requires a reference; got '{}'", found)
            }
            AnalyzeErrorKind::ReturnsLocalReference => {
                "Returning a reference to a local variable would leave a dangling pointer after the function returns".to_string()
            }
            AnalyzeErrorKind::UnsupportedOperandType { op, operand_type } => {
                format!(
                    "Operator '{}' cannot be applied to operands of type '{}'",
                    op, operand_type
                )
            }
            AnalyzeErrorKind::DiscordDependency { name } => {
                format!("Statements in this discord block are not independent: more than one touches '{}'", name)
            }
            AnalyzeErrorKind::DiscordImpurity { what } => {
                format!("'{}' is not permitted inside a discord block", what)
            }
        }
    }

    fn location(&self) -> &SourceLocation {
        &self.location
    }

    fn hint(&self) -> Option<String> {
        match &self.kind {
            AnalyzeErrorKind::UndeclaredVariable(_) => {
                Some("make sure the variable is declared before use".to_string())
            }
            AnalyzeErrorKind::AssignmentToImmutable(_) => {
                Some("consider declaring the variable as mutable with 'mut'".to_string())
            }
            AnalyzeErrorKind::TypeMismatch { .. } => {
                Some("all types in Eris must match exactly; there is no implicit type conversion".to_string())
            }
            AnalyzeErrorKind::BreakOutsideLoop => {
                Some("break is only valid inside a loop body".to_string())
            }
            AnalyzeErrorKind::ContinueOutsideLoop => {
                Some("continue is only valid inside a loop body".to_string())
            }
            AnalyzeErrorKind::MissingReturn(_) => {
                Some("non-void functions must end with a return statement or a trailing expression on every control-flow path".to_string())
            }
            AnalyzeErrorKind::ReturnMissingValue { .. } => {
                Some("supply a return value, e.g. `return 0;`, or change the function's return type".to_string())
            }
            AnalyzeErrorKind::NonIterableInForLoop { .. } => {
                Some("use `start..end` to iterate an integer range, e.g. `for i in 0..10`".to_string())
            }
            AnalyzeErrorKind::IndexOutOfRange { array_size, .. } => {
                Some(format!("valid indices are 0..{} (exclusive upper bound)", array_size))
            }
            AnalyzeErrorKind::IndexNotInteger { .. } => {
                Some("convert the index expression to 'int' before indexing".to_string())
            }
            AnalyzeErrorKind::TakeRefOfNonLvalue => {
                Some("introduce a named variable first, then take a reference to it".to_string())
            }
            AnalyzeErrorKind::TakeMutRefOfImmutable { name } => {
                Some(format!("declare '{}' with 'mut' to allow mutable references", name))
            }
            AnalyzeErrorKind::DerefOfNonReference { .. } => {
                Some("take a reference with '&' or '&mut' before dereferencing".to_string())
            }
            AnalyzeErrorKind::ReturnsLocalReference => {
                Some("return the value itself, or pass the destination as a '&mut' parameter".to_string())
            }
            AnalyzeErrorKind::UnsupportedOperandType { .. } => Some(
                "arithmetic operators require 'int' or 'float' operands; comparison operators additionally accept 'char' and 'bool'".to_string(),
            ),
            AnalyzeErrorKind::DiscordDependency { name } => Some(format!(
                "move the statements that share '{}' out of the discord block, or merge them into one statement",
                name
            )),
            AnalyzeErrorKind::DiscordImpurity { .. } => Some(
                "keep effects outside: only declarations, assignments, and pure expressions may be discordant".to_string(),
            ),
            _ => None,
        }
    }

    fn koan(&self) -> Option<String> {
        match &self.kind {
            AnalyzeErrorKind::UndeclaredVariable(name) => Some(format!(
                "you may not summon what was never named. declare '{}' before invoking it",
                name
            )),
            AnalyzeErrorKind::UndeclaredFunction(name) => Some(format!(
                "no rite by the name '{}' has been inscribed. declare the function before calling it",
                name
            )),
            AnalyzeErrorKind::TypeMismatch { .. } => Some(
                "all things must match exactly; Eris performs no silent transformations. convert with intent".to_string(),
            ),
            AnalyzeErrorKind::IncorrectArgumentCount { expected, .. } => Some(format!(
                "the rite demands exactly {} offering(s). count your arguments",
                expected
            )),
            AnalyzeErrorKind::VariableAlreadyDeclared(name) => Some(format!(
                "one name, one binding, one scope. choose a new name, or reassign '{}' with '='",
                name
            )),
            AnalyzeErrorKind::AssignmentToImmutable(name) => Some(format!(
                "what is fixed cannot be swayed. declare '{}' with 'mut' if it must change",
                name
            )),
            AnalyzeErrorKind::BreakOutsideLoop => Some(
                "there is nothing here to escape. 'break' belongs inside a loop".to_string(),
            ),
            AnalyzeErrorKind::ContinueOutsideLoop => Some(
                "you cannot continue what never repeats. 'continue' belongs inside a loop".to_string(),
            ),
            AnalyzeErrorKind::MissingReturn(name) => Some(format!(
                "every path must arrive somewhere, and one path through '{}' wanders off without returning",
                name
            )),
            AnalyzeErrorKind::ReturnMissingValue { .. } => Some(
                "you promised a value and returned with empty hands".to_string(),
            ),
            AnalyzeErrorKind::NonIterableInForLoop { .. } => Some(
                "'for' walks a range. give it 'start..end', or give it nothing".to_string(),
            ),
            AnalyzeErrorKind::IndexOutOfRange { array_size, .. } => Some(format!(
                "the array ends before your index does. valid indices stop at {}",
                array_size.saturating_sub(1)
            )),
            AnalyzeErrorKind::IndexNotInteger { .. } => Some(
                "arrays are counted in whole steps. index with an 'int'".to_string(),
            ),
            AnalyzeErrorKind::TakeRefOfNonLvalue => Some(
                "you cannot point at what has no home. name it first, then take the reference".to_string(),
            ),
            AnalyzeErrorKind::TakeMutRefOfImmutable { name } => Some(format!(
                "to lend the power of change over '{}', one must first possess it. declare it 'mut'",
                name
            )),
            AnalyzeErrorKind::DerefOfNonReference { .. } => Some(
                "'*' opens a door, and this is a wall. take a reference first".to_string(),
            ),
            AnalyzeErrorKind::ReturnsLocalReference => Some(
                "the local dies at the closing brace; your reference would point at a ghost. return the value itself".to_string(),
            ),
            AnalyzeErrorKind::UnsupportedOperandType { op, .. } => Some(format!(
                "the operator '{}' refuses these offerings. arithmetic wants 'int' or 'float'",
                op
            )),
            AnalyzeErrorKind::DiscordDependency { name } => Some(format!(
                "these statements share '{}', so their order matters — and ordered chaos is no chaos at all. separate them",
                name
            )),
            AnalyzeErrorKind::DiscordImpurity { .. } => Some(
                "chaos that can be witnessed is no longer harmless. only pure, independent bindings may enter the discord".to_string(),
            ),
        }
    }
}
