use eris_compiler::analyzer::AnalyzeError;
use eris_compiler::analyzer::AnalyzeErrorKind;
use eris_compiler::diagnostics::{Diagnostic, ErrorCode};
use eris_compiler::lexer::SourceLocation;
use eris_compiler::parser::ast::TypeKind;

#[test]
fn analyze_type_mismatch_emits_e0203_with_type_fields() {
    let err = AnalyzeError::new(
        AnalyzeErrorKind::TypeMismatch {
            expected: TypeKind::Int,
            found: TypeKind::Bool,
        },
        SourceLocation { line: 3, column: 10 },
    );
    let d: Diagnostic = err.into();
    assert_eq!(d.code, ErrorCode::E0203);
    assert_eq!(
        d.fields.get("expected").and_then(|v| v.as_str()),
        Some("int")
    );
    assert_eq!(
        d.fields.get("found").and_then(|v| v.as_str()),
        Some("bool")
    );
}

#[test]
fn analyze_undeclared_variable_emits_e0201() {
    let err = AnalyzeError::new(
        AnalyzeErrorKind::UndeclaredVariable("foo".to_string()),
        SourceLocation { line: 1, column: 1 },
    );
    let d: Diagnostic = err.into();
    assert_eq!(d.code, ErrorCode::E0201);
    assert_eq!(d.fields.get("name").and_then(|v| v.as_str()), Some("foo"));
}

#[test]
fn analyze_incorrect_arg_count_emits_e0204_with_numbers() {
    let err = AnalyzeError::new(
        AnalyzeErrorKind::IncorrectArgumentCount {
            expected: 3,
            found: 1,
        },
        SourceLocation { line: 5, column: 2 },
    );
    let d: Diagnostic = err.into();
    assert_eq!(d.code, ErrorCode::E0204);
    assert_eq!(d.fields.get("expected").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(d.fields.get("found").and_then(|v| v.as_u64()), Some(1));
}
