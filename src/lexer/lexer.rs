use logos::Logos;
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

    // Operators
    #[token(":=")]
    DeclareAssign,
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
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LexerErrorKind {
    InvalidIdentifier(String),
    // UnterminatedString,
    InvalidToken(String),
}

#[derive(Debug)]
pub struct LexerError {
    kind: LexerErrorKind,
    location: SourceLocation,
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    logos_lexer: logos::Lexer<'a, Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            logos_lexer: Token::lexer(source),
        }
    }

    fn get_location(&self, byte_index: usize) -> SourceLocation {
        // This is a simplified location tracking. For more accurate line/column
        // numbers, you might need to keep track of newlines.
        SourceLocation { line: 1, column: byte_index + 1 }
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
