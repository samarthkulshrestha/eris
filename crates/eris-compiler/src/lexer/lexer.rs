use logos::Logos;
use schemars::JsonSchema;
use serde::Serialize;
use std::fmt;
use std::error::Error;

// Token definitions for Eris programming language
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")] // Skip whitespace
#[logos(skip r"--[^\n]*")]   // Skip single-line comments
#[logos(skip r"---[^\n]*")]  // Skip doc comments
// #[logos(skip r"-\+((?!\+-).)*\+-")] // Skip multi-line comments
pub enum Token {
    // Keywords
    #[token("mut")]
    Mut,
    #[token("int")]
    Int,
    #[token("bool")]
    Bool,
    #[token("char")]
    Char,
    #[token("float")]
    Float,
    #[token("str")]
    Str,
    #[token("arr")]
    Arr,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("return")]
    Return,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("while")]
    While,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("for")]
    For,
    #[token("in")]
    In,

    // Operators
    #[token(":=")]
    DeclareAssign,
    #[token("..")]
    DotDot,
    #[token("=")]
    Assign,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Multiply,
    #[token("/")]
    Divide,
    #[token("%")]
    Modulo,

    // Comparison operators
    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token("<")]
    LessThan,
    #[token("<=")]
    LessEqual,
    #[token(">")]
    GreaterThan,
    #[token(">=")]
    GreaterEqual,

    // Logical operators
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Not,

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,

    // Note: Using LBracket for array syntax
    // No separate ArrayStart token to avoid conflict

    // Literals
    #[regex(r#""([^"\\]|\\["\\nt])*""#, |lex| {
        let slice = lex.slice();
        // Remove quotes to get the actual string content
        let content = &slice[1..slice.len()-1];
        content.to_string()
    })]
    StringLit(String),

    #[regex(r#"'([^'\\]|\\[nrt'\\])'"#, |lex| {
        let slice = lex.slice();
        // Remove quotes to get the actual string content
        let content = &slice[1..slice.len()-1];
        content.chars().next().unwrap()
    })]
    CharLit(char),

    #[regex(r"[0-9]+\.[0-9]+", |lex| {
        lex.slice().parse::<f64>().ok()
    })]
    FloatLit(f64),

    #[regex(r"[0-9]+", |lex| {
        lex.slice().parse::<i64>().ok()
    })]
    IntLit(i64),

    // Identifiers must start with letter or underscore
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // Error cases
    #[regex(r"[0-9]+[a-zA-Z_]+[a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    InvalidIdent(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "Identifier: {}", s),
            Token::IntLit(i) => write!(f, "Integer: {}", i),
            Token::FloatLit(fl) => write!(f, "Float: {}", fl),
            Token::StringLit(s) => write!(f, "String: \"{}\"", s),
            Token::CharLit(c) => write!(f, "Char: \'{}\'", c),
            Token::True => write!(f, "Bool: true"),
            Token::False => write!(f, "Bool: false"),
            Token::InvalidIdent(s) => write!(f, "Invalid identifier: {}", s),
            Token::Mut => write!(f, "Keyword: mut"),
            Token::Int => write!(f, "Keyword: int"),
            Token::Bool => write!(f, "Keyword: bool"),
            Token::Str => write!(f, "Keyword: str"),
            Token::Char => write!(f, "Keyword: char"),
            Token::Float => write!(f, "Keyword: float"),
            Token::Arr => write!(f, "Keyword: arr"),
            Token::If => write!(f, "Keyword: if"),
            Token::Else => write!(f, "Keyword: else"),
            Token::Return => write!(f, "Keyword: return"),
            Token::DeclareAssign => write!(f, "Operator: :="),
            Token::Assign => write!(f, "Operator: ="),
            Token::For => write!(f, "Keyword: for"),
            Token::In => write!(f, "Keyword: in"),
            Token::DotDot => write!(f, "Operator: .."),
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LexerErrorKind {
    InvalidIdentifier(String),
    // UnterminatedString,
    InvalidToken(String),
}

#[derive(Debug, Clone)]
pub struct LexerError {
    kind: LexerErrorKind,
    location: SourceLocation,
}

impl LexerError {
    pub fn location(&self) -> &SourceLocation {
        &self.location
    }
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match &self.kind {
            LexerErrorKind::InvalidIdentifier(s) => format!("Invalid identifier: '{}'", s),
            // LexerErrorKind::UnterminatedString => "Unterminated string".to_string(),
            LexerErrorKind::InvalidToken(s) => format!("Invalid token: '{}'", s),
        };
        write!(
            f,
            "Lexer error at {}:{}: {}",
            self.location.line, self.location.column, message
        )
    }
}

impl Error for LexerError {}

impl From<LexerError> for crate::diagnostics::Diagnostic {
    fn from(err: LexerError) -> Self {
        use crate::diagnostics::{Diagnostic, ErrorCode, Severity};
        use crate::error_display::FormattedError;
        use serde_json::{Map, Value};

        let (code, message, fields) = match &err.kind {
            LexerErrorKind::InvalidIdentifier(s) => {
                let mut m = Map::new();
                m.insert("identifier".to_string(), Value::String(s.clone()));
                (
                    ErrorCode::E0001,
                    format!("invalid identifier: '{}'", s),
                    m,
                )
            }
            LexerErrorKind::InvalidToken(s) => {
                let mut m = Map::new();
                m.insert("token".to_string(), Value::String(s.clone()));
                (
                    ErrorCode::E0002,
                    format!("invalid token: '{}'", s),
                    m,
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

impl crate::error_display::FormattedError for LexerError {
    fn error_type(&self) -> &str {
        match &self.kind {
            LexerErrorKind::InvalidIdentifier(_) => "invalid identifier",
            LexerErrorKind::InvalidToken(_) => "invalid token",
        }
    }

    fn message(&self) -> String {
        match &self.kind {
            LexerErrorKind::InvalidIdentifier(s) => format!("Invalid identifier: '{}'", s),
            LexerErrorKind::InvalidToken(s) => format!("Invalid token: '{}'", s),
        }
    }

    fn location(&self) -> &SourceLocation {
        &self.location
    }

    fn hint(&self) -> Option<String> {
        match &self.kind {
            LexerErrorKind::InvalidIdentifier(_) => {
                Some("identifiers cannot start with a digit".to_string())
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, JsonSchema)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    source: &'a str,
    logos_lexer: logos::Lexer<'a, Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source,
            logos_lexer: Token::lexer(source),
        }
    }

    fn get_location(&self, byte_index: usize) -> SourceLocation {
        let mut line = 1;
        let mut current_index = 0;

        for line_str in self.source.lines() {
            let line_len = line_str.len() + 1; // +1 for the newline

            if current_index + line_len > byte_index {
                let column = byte_index - current_index + 1;
                return SourceLocation { line, column };
            }

            current_index += line_len;
            line += 1;
        }

        SourceLocation { line, column: 1 }
    }

    pub fn tokenize(&mut self) -> Result<Vec<(Token, SourceLocation)>, LexerError> {
        let mut tokens = Vec::new();

        while let Some(token_result) = self.logos_lexer.next() {
            let span = self.logos_lexer.span();
            let location = self.get_location(span.start);

            match token_result {
                Ok(Token::InvalidIdent(s)) => {
                    return Err(LexerError {
                        kind: LexerErrorKind::InvalidIdentifier(s.clone()),
                        location,
                    });
                }
                Ok(token) => {
                    tokens.push((token, location));
                }
                Err(_) => {
                    let invalid_token = self.logos_lexer.slice().to_string();
                    return Err(LexerError {
                        kind: LexerErrorKind::InvalidToken(invalid_token),
                        location,
                    });
                }
            }
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("mut int bool char float str arr if else return true false");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 12);
        assert_eq!(tokens[0].0, Token::Mut);
        assert_eq!(tokens[1].0, Token::Int);
        assert_eq!(tokens[2].0, Token::Bool);
        assert_eq!(tokens[3].0, Token::Char);
        assert_eq!(tokens[4].0, Token::Float);
        assert_eq!(tokens[5].0, Token::Str);
        assert_eq!(tokens[6].0, Token::Arr);
        assert_eq!(tokens[7].0, Token::If);
        assert_eq!(tokens[8].0, Token::Else);
        assert_eq!(tokens[9].0, Token::Return);
        assert_eq!(tokens[10].0, Token::True);
        assert_eq!(tokens[11].0, Token::False);
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new(":= = + - * / % == != < <= > >= && || !");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].0, Token::DeclareAssign);
        assert_eq!(tokens[1].0, Token::Assign);
        assert_eq!(tokens[2].0, Token::Plus);
        assert_eq!(tokens[3].0, Token::Minus);
        assert_eq!(tokens[4].0, Token::Multiply);
        assert_eq!(tokens[5].0, Token::Divide);
        assert_eq!(tokens[6].0, Token::Modulo);
        assert_eq!(tokens[7].0, Token::Equal);
        assert_eq!(tokens[8].0, Token::NotEqual);
        assert_eq!(tokens[9].0, Token::LessThan);
        assert_eq!(tokens[10].0, Token::LessEqual);
        assert_eq!(tokens[11].0, Token::GreaterThan);
        assert_eq!(tokens[12].0, Token::GreaterEqual);
        assert_eq!(tokens[13].0, Token::And);
        assert_eq!(tokens[14].0, Token::Or);
        assert_eq!(tokens[15].0, Token::Not);
    }

    #[test]
    fn test_delimiters() {
        let mut lexer = Lexer::new("( ) { } [ ] ; ,");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].0, Token::LParen);
        assert_eq!(tokens[1].0, Token::RParen);
        assert_eq!(tokens[2].0, Token::LBrace);
        assert_eq!(tokens[3].0, Token::RBrace);
        assert_eq!(tokens[4].0, Token::LBracket);
        assert_eq!(tokens[5].0, Token::RBracket);
        assert_eq!(tokens[6].0, Token::Semicolon);
        assert_eq!(tokens[7].0, Token::Comma);
    }

    #[test]
    fn test_literals() {
        let mut lexer = Lexer::new(r#"42 3.14 'a' "hello""#);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].0, Token::IntLit(42));
        assert_eq!(tokens[1].0, Token::FloatLit(3.14));
        assert_eq!(tokens[2].0, Token::CharLit('a'));
        assert_eq!(tokens[3].0, Token::StringLit("hello".to_string()));
    }

    #[test]
    fn test_identifiers() {
        let mut lexer = Lexer::new("x foo bar_baz test123");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].0, Token::Ident("x".to_string()));
        assert_eq!(tokens[1].0, Token::Ident("foo".to_string()));
        assert_eq!(tokens[2].0, Token::Ident("bar_baz".to_string()));
        assert_eq!(tokens[3].0, Token::Ident("test123".to_string()));
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("int x := 10; -- this is a comment\nint y := 20;");
        let tokens = lexer.tokenize().unwrap();

        // Comments should be skipped
        assert_eq!(tokens.len(), 10); // int, x, :=, 10, ;, int, y, :=, 20, ;
    }

    #[test]
    fn test_location_tracking() {
        let mut lexer = Lexer::new("int x := 10;");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].1.line, 1);
        assert_eq!(tokens[0].1.column, 1);
    }

    #[test]
    fn test_variable_declaration() {
        let mut lexer = Lexer::new("int x := 10;");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].0, Token::Int);
        assert_eq!(tokens[1].0, Token::Ident("x".to_string()));
        assert_eq!(tokens[2].0, Token::DeclareAssign);
        assert_eq!(tokens[3].0, Token::IntLit(10));
        assert_eq!(tokens[4].0, Token::Semicolon);
    }

    #[test]
    fn test_function_declaration() {
        let mut lexer = Lexer::new("int add(int a, int b) { return a + b; }");
        let tokens = lexer.tokenize().unwrap();

        assert!(tokens.iter().any(|(t, _)| t == &Token::Int));
        assert!(tokens.iter().any(|(t, _)| matches!(t, Token::Ident(s) if s == "add")));
        assert!(tokens.iter().any(|(t, _)| t == &Token::LParen));
        assert!(tokens.iter().any(|(t, _)| t == &Token::Return));
    }

    #[test]
    fn test_array_syntax() {
        let mut lexer = Lexer::new("arr[int] numbers := [1, 2, 3];");
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens[0].0, Token::Arr);
        assert_eq!(tokens[1].0, Token::LBracket);
        assert_eq!(tokens[2].0, Token::Int);
        assert_eq!(tokens[3].0, Token::RBracket);
    }

    #[test]
    fn test_string_escapes() {
        let mut lexer = Lexer::new(r#""hello\nworld""#);
        let tokens = lexer.tokenize().unwrap();

        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].0, Token::StringLit(_)));
    }

    #[test]
    fn test_empty_source() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_whitespace_only() {
        let mut lexer = Lexer::new("   \n\t  \n  ");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_comment_only() {
        let mut lexer = Lexer::new("-- this is a comment");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_invalid_identifier_starting_with_digit() {
        let mut lexer = Lexer::new("123abc");
        let result = lexer.tokenize();

        assert!(result.is_err());
        match result {
            Err(LexerError { kind: LexerErrorKind::InvalidIdentifier(s), .. }) => {
                assert_eq!(s, "123abc");
            }
            _ => panic!("Expected InvalidIdentifier error"),
        }
    }

    #[test]
    fn test_zero_literal() {
        let mut lexer = Lexer::new("0");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, Token::IntLit(0));
    }

    #[test]
    fn test_negative_float() {
        let mut lexer = Lexer::new("-3.14");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 2); // Minus and FloatLit
        assert_eq!(tokens[0].0, Token::Minus);
        assert!(matches!(tokens[1].0, Token::FloatLit(_)));
    }

    #[test]
    fn test_consecutive_operators() {
        let mut lexer = Lexer::new("+-*/");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].0, Token::Plus);
        assert_eq!(tokens[1].0, Token::Minus);
        assert_eq!(tokens[2].0, Token::Multiply);
        assert_eq!(tokens[3].0, Token::Divide);
    }

    #[test]
    fn test_comparison_operators_no_spaces() {
        // Test each comparison operator separately to avoid ambiguity
        let mut lexer = Lexer::new("== != < <= > >=");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].0, Token::Equal);
        assert_eq!(tokens[1].0, Token::NotEqual);
        assert_eq!(tokens[2].0, Token::LessThan);
        assert_eq!(tokens[3].0, Token::LessEqual);
        assert_eq!(tokens[4].0, Token::GreaterThan);
        assert_eq!(tokens[5].0, Token::GreaterEqual);
    }

    #[test]
    fn test_unix_line_endings() {
        // Test with Unix line endings only (\n)
        let mut lexer = Lexer::new("int x := 10;\nint y := 20;");
        let tokens = lexer.tokenize().unwrap();
        assert!(tokens.len() > 0);
    }

    #[test]
    fn test_char_escape_sequences() {
        let mut lexer = Lexer::new(r"'a' '\n' '\t' '\r' '\\' '\''");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 6);
        assert!(matches!(tokens[0].0, Token::CharLit('a')));
    }

    #[test]
    fn test_empty_string() {
        let mut lexer = Lexer::new(r#""""#);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        match &tokens[0].0 {
            Token::StringLit(s) => assert_eq!(s, ""),
            _ => panic!("Expected empty StringLit"),
        }
    }

    #[test]
    fn test_underscore_in_identifiers() {
        let mut lexer = Lexer::new("_foo foo_bar _");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].0, Token::Ident(_)));
        assert!(matches!(tokens[1].0, Token::Ident(_)));
        assert!(matches!(tokens[2].0, Token::Ident(_)));
    }

    #[test]
    fn test_long_identifier() {
        let long_name = "a".repeat(100);
        let mut lexer = Lexer::new(&long_name);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        match &tokens[0].0 {
            Token::Ident(s) => assert_eq!(s.len(), 100),
            _ => panic!("Expected Ident"),
        }
    }

    #[test]
    fn test_large_integer() {
        let mut lexer = Lexer::new("9223372036854775807"); // i64::MAX
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].0, Token::IntLit(_)));
    }

    #[test]
    fn test_float_with_trailing_zeros() {
        let mut lexer = Lexer::new("1.00");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].0, Token::FloatLit(_)));
    }
}
