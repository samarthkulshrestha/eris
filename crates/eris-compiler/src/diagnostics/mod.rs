use crate::lexer::SourceLocation;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Map;

pub mod codes;
pub mod json;
pub mod language_card;

pub use codes::ErrorCode;

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Label {
    pub location: SourceLocation,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Diagnostic {
    pub code: ErrorCode,
    pub severity: Severity,
    pub message: String,
    pub location: SourceLocation,
    pub labels: Vec<Label>,
    pub hint: Option<String>,
    /// Per-code structured data. The shape varies by `code`; consumers should
    /// consult per-code documentation. Top-level keys are reserved for
    /// cross-cutting concerns and never collide with per-code fields under this
    /// design.
    #[serde(skip_serializing_if = "Map::is_empty", default)]
    #[schemars(skip)]
    pub fields: Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn diagnostic_skeleton_constructs_with_e0101() {
        let diag = Diagnostic {
            code: ErrorCode::E0101,
            severity: Severity::Error,
            message: "unexpected token".to_string(),
            location: SourceLocation { line: 1, column: 5 },
            labels: vec![],
            hint: None,
            fields: Map::new(),
        };
        assert_eq!(diag.code, ErrorCode::E0101);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.message, "unexpected token");
        assert_eq!(diag.location.line, 1);
        assert_eq!(diag.location.column, 5);

        // Verify it serializes correctly
        let v = serde_json::to_value(&diag).unwrap();
        assert_eq!(v["code"], json!("E0101"));
        assert_eq!(v["severity"], json!("error"));
    }
}
