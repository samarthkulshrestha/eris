use eris_compiler::diagnostics::{Diagnostic, ErrorCode};
use eris_compiler::lexer::Lexer;

#[test]
fn lexer_invalid_token_emits_e0002_with_fields() {
    let mut lexer = Lexer::new("int x := @;");
    let err = lexer.tokenize().unwrap_err();
    let d: Diagnostic = err.into();
    assert_eq!(d.code, ErrorCode::E0002);
    assert_eq!(d.fields.get("token").and_then(|v| v.as_str()), Some("@"));
}

#[test]
fn lexer_invalid_identifier_emits_e0001_with_fields() {
    let mut lexer = Lexer::new("123abc");
    let err = lexer.tokenize().unwrap_err();
    let d: Diagnostic = err.into();
    assert_eq!(d.code, ErrorCode::E0001);
    assert_eq!(
        d.fields.get("identifier").and_then(|v| v.as_str()),
        Some("123abc")
    );
}
