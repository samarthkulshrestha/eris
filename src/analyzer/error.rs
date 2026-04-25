use crate::lexer::SourceLocation;

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
        expected: String,
        found: String,
    },
    IncorrectArgumentCount {
        expected: usize,
        found: usize,
    },
    VariableAlreadyDeclared(String),
    AssignmentToImmutable(String),
}

impl AnalyzeError {
    pub fn new(kind: AnalyzeErrorKind, location: SourceLocation) -> Self {
        AnalyzeError { kind, location }
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
                format!("Expected type {}, found {}", expected, found)
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
            _ => None,
        }
    }
}
