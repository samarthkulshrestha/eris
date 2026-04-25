use eris_compiler::diagnostics::{Diagnostic, ErrorCode};
use eris_compiler::lexer::Lexer;
use eris_compiler::parser::{ParseError, ParseErrorKind, Parser};

#[test]
fn parser_unexpected_token_emits_e0101_with_fields() {
    // "int x = 10;" uses single = which is assignment, not declaration (:=)
    // The parser should fail with UnexpectedToken
    let mut lexer = Lexer::new("int x = 10;");
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let err = parser.parse_program().unwrap_err();
    let d: Diagnostic = err.into();
    assert_eq!(d.code, ErrorCode::E0101);
    assert!(
        d.fields.get("expected").is_some(),
        "expected field must be present"
    );
    assert!(
        d.fields.get("found").is_some(),
        "found field must be present"
    );
}

#[test]
fn parser_lexer_error_preserves_inner_code() {
    // Build a ParseError wrapping a LexerError for InvalidToken
    let lex_err = {
        let mut lexer = Lexer::new("@");
        lexer.tokenize().unwrap_err()
    };
    let parse_err = ParseError::from(lex_err);
    assert!(matches!(parse_err.kind, ParseErrorKind::LexerError(_)));
    let d: Diagnostic = parse_err.into();
    // Inner code must be preserved
    assert_eq!(d.code, ErrorCode::E0002);
}
