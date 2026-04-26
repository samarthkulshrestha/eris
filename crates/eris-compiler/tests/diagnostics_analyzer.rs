use eris_compiler::analyzer::AnalyzeError;
use eris_compiler::analyzer::AnalyzeErrorKind;
use eris_compiler::diagnostics::{Diagnostic, ErrorCode};
use eris_compiler::lexer::SourceLocation;
use eris_compiler::parser::ast::TypeKind;
use eris_compiler::analyzer::Analyzer;
use eris_compiler::lexer::Lexer;
use eris_compiler::parser::Parser;

#[test]
fn break_outside_loop_emits_e0207() {
    let source = "int main() { break; return 0; }";
    let mut lex = Lexer::new(source);
    let toks = lex.tokenize().unwrap();
    let mut p = Parser::new(toks);
    let program = p.parse_program().unwrap();
    let mut a = Analyzer::new();
    let errors = a.analyze_program(&program).unwrap_err();
    assert!(errors.iter().any(|e| matches!(e.kind, AnalyzeErrorKind::BreakOutsideLoop)));
}

#[test]
fn continue_outside_loop_emits_e0208() {
    let source = "int main() { continue; return 0; }";
    let mut lex = Lexer::new(source);
    let toks = lex.tokenize().unwrap();
    let mut p = Parser::new(toks);
    let program = p.parse_program().unwrap();
    let mut a = Analyzer::new();
    let errors = a.analyze_program(&program).unwrap_err();
    assert!(errors.iter().any(|e| matches!(e.kind, AnalyzeErrorKind::ContinueOutsideLoop)));
}

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

#[test]
fn undeclared_var_does_not_trigger_spurious_type_mismatch() {
    let source = "int main() { return undefined_var; }";
    let mut lex = Lexer::new(source);
    let toks = lex.tokenize().unwrap();
    let mut parser = Parser::new(toks);
    let program = parser.parse_program().unwrap();
    let mut analyzer = Analyzer::new();
    let errors = analyzer.analyze_program(&program).unwrap_err();

    // Should produce E0201 (undeclared variable) but NOT E0203 (type mismatch).
    let codes: Vec<_> = errors.iter().map(|e| match &e.kind {
        AnalyzeErrorKind::UndeclaredVariable(_) => "E0201",
        AnalyzeErrorKind::TypeMismatch { .. } => "E0203",
        _ => "other",
    }).collect();
    assert!(codes.contains(&"E0201"), "expected E0201, got {:?}", codes);
    assert!(!codes.contains(&"E0203"), "should not emit spurious E0203, got {:?}", codes);
}
