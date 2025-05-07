use logos::Logos;
use std::fmt;
use std::error::Error;

// Token definitions for Eris programming language
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]  // Skip whitespace
#[logos(skip r"--[^\n]*")]    // Skip single-line comments
#[logos(skip r"---[^\n]*")]   // Skip doc comments
// Custom handling for multi-line comments is implemented in the lexer
pub enum Token {
    // Keywords
    #[token("mut")]
    Mut,

    #[token("int")]
    Int,

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
            Token::InvalidIdent(s) => write!(f, "Invalid identifier: {}", s),
            Token::Mut => write!(f, "Keyword: mut"),
            Token::Int => write!(f, "Keyword: int"),
            Token::Str => write!(f, "Keyword: str"),
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

#[derive(Debug)]
pub struct LexerError {
    token: String,
    line: usize,
    column: usize,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Lexer error: Invalid token '{}' at {}:{}", 
            self.token, self.line, self.column)
    }
}

impl Error for LexerError {}

pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

pub struct Lexer<'a> {
    source: &'a str,
    logos_lexer: logos::Lexer<'a, Token>,
    // We need to preprocess the source to handle multi-line comments
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

    pub fn tokenize(&mut self) -> Vec<Result<(Token, SourceLocation), LexerError>> {
        let mut tokens = Vec::new();

        while let Some(token_result) = self.logos_lexer.next() {
            let span = self.logos_lexer.span();
            let location = self.get_location(span.start);

            match token_result {
                Ok(Token::InvalidIdent(s)) => {
                    tokens.push(Err(LexerError {
                        token: s.clone(),
                        line: location.line,
                        column: location.column,
                    }));
                },
                Ok(token) => {
                    tokens.push(Ok((token, location)));
                },
                Err(_) => {
                    let invalid_token = self.source[span.clone()].to_string();
                    tokens.push(Err(LexerError {
                        token: invalid_token,
                        line: location.line,
                        column: location.column,
                    }));
                }
            }
        }

        tokens
    }
}
