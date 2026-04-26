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
            _ => None,
        }
    }
}
