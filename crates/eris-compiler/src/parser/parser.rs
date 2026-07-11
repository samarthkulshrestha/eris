use crate::lexer::{LexerError, SourceLocation, Token};

use crate::parser::ast::{
    BinOp, Expr, ExprKind, Identifier, Param, Precedence, Program, Stmt, StmtKind, Type, TypeKind,
};

#[derive(Debug, Clone)]
pub enum ParseErrorKind {
    UnexpectedToken {
        expected: String,
        found: Option<Token>,
    },
    InvalidSyntax(String),
    LexerError(LexerError),
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub location: Option<SourceLocation>,
}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        let location = Some(err.location().clone());
        ParseError {
            kind: ParseErrorKind::LexerError(err),
            location,
        }
    }
}

impl crate::error_display::FormattedError for ParseError {
    fn error_type(&self) -> &str {
        match &self.kind {
            ParseErrorKind::UnexpectedToken { .. } => "unexpected token",
            ParseErrorKind::InvalidSyntax(_) => "invalid syntax",
            ParseErrorKind::LexerError(_) => "lexer error",
        }
    }

    fn message(&self) -> String {
        match &self.kind {
            ParseErrorKind::UnexpectedToken { expected, found } => {
                if let Some(token) = found {
                    format!("Expected {}, found {:?}", expected, token)
                } else {
                    format!("Expected {}, found end of file", expected)
                }
            }
            ParseErrorKind::InvalidSyntax(msg) => msg.clone(),
            ParseErrorKind::LexerError(err) => err.to_string(),
        }
    }

    fn location(&self) -> &crate::lexer::SourceLocation {
        static DEFAULT_LOCATION: crate::lexer::SourceLocation = crate::lexer::SourceLocation {
            line: 0,
            column: 0,
        };

        if let Some(loc) = &self.location {
            loc
        } else {
            &DEFAULT_LOCATION
        }
    }

    fn hint(&self) -> Option<String> {
        match &self.kind {
            ParseErrorKind::UnexpectedToken { expected, .. } => {
                if expected.contains("Semicolon") {
                    Some("did you forget a semicolon?".to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn koan(&self) -> Option<String> {
        Some("the grammar is a ritual, and this token arrived out of turn. read the line as the parser must: left to right, expecting order".to_string())
    }
}

impl From<ParseError> for crate::diagnostics::Diagnostic {
    fn from(err: ParseError) -> Self {
        use crate::diagnostics::{Diagnostic, ErrorCode, Severity};
        use crate::error_display::FormattedError;
        use serde_json::{Map, Value};

        let location = err
            .location
            .clone()
            .unwrap_or_else(crate::lexer::SourceLocation::default);

        match err.kind {
            ParseErrorKind::LexerError(inner) => Diagnostic::from(inner),
            ParseErrorKind::UnexpectedToken { expected, found } => {
                let hint = {
                    // Re-construct a temporary ParseError to call hint() via the trait.
                    let tmp = ParseError {
                        kind: ParseErrorKind::UnexpectedToken {
                            expected: expected.clone(),
                            found: found.clone(),
                        },
                        location: None,
                    };
                    tmp.hint()
                };
                let mut fields = Map::new();
                fields.insert("expected".to_string(), Value::String(expected.clone()));
                let found_str = found
                    .as_ref()
                    .map(|t| format!("{:?}", t))
                    .map(Value::String)
                    .unwrap_or(Value::Null);
                fields.insert("found".to_string(), found_str);
                let message = match &found {
                    Some(t) => format!("expected {}, found {:?}", expected, t),
                    None => format!("expected {}, found end of file", expected),
                };
                Diagnostic {
                    code: ErrorCode::E0101,
                    severity: Severity::Error,
                    message,
                    location,
                    labels: vec![],
                    hint,
                    fields,
                }
            }
            ParseErrorKind::InvalidSyntax(msg) => {
                let mut fields = Map::new();
                fields.insert("detail".to_string(), Value::String(msg.clone()));
                Diagnostic {
                    code: ErrorCode::E0102,
                    severity: Severity::Error,
                    message: msg,
                    location,
                    labels: vec![],
                    hint: None,
                    fields,
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Parser {
    tokens: Vec<(Token, SourceLocation)>,
    pos: usize,
    current_token: Option<(Token, SourceLocation)>,
    last_location: SourceLocation,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, SourceLocation)>) -> Self {
        let current_token = tokens.first().cloned();
        Parser {
            tokens,
            pos: 0,
            current_token,
            last_location: SourceLocation { line: 1, column: 1 },
        }
    }

    fn advance(&mut self) -> Option<(Token, SourceLocation)> {
        if let Some((_, loc)) = &self.current_token {
            self.last_location = loc.clone();
        }
        self.pos += 1;
        self.current_token = self.tokens.get(self.pos).cloned();
        self.current_token.clone()
    }

    // fn _peek(&mut self) -> Option<&(Token, SourceLocation)> {
    //     self.tokens.peek()
    // }

    fn expect(&mut self, expected: &[Token]) -> Result<(Token, SourceLocation), ParseError> {
        if let Some((token, location)) = &self.current_token {
            if expected.contains(token) {
                let result = (token.clone(), location.clone());
                self.advance();
                return Ok(result);
            }

            let expected_str = expected
                .iter()
                .map(|t| format!("{:?}", t))
                .collect::<Vec<_>>()
                .join(", ");

            return Err(ParseError {
                kind: ParseErrorKind::UnexpectedToken {
                    expected: expected_str,
                    found: Some(token.clone()),
                },
                location: Some(location.clone()),
            });
        }

        Err(ParseError {
            kind: ParseErrorKind::UnexpectedToken {
                expected: expected
                    .iter()
                    .map(|t| format!("{:?}", t))
                    .collect::<Vec<_>>()
                    .join(", "),
                found: None,
            },
            location: Some(self.last_location.clone()),
        })
    }

    fn current_is(&self, tokens: &[Token]) -> bool {
        if let Some((token, _)) = &self.current_token {
            tokens.iter().any(|t| token_matches(t, token))
        } else {
            false
        }
    }

    /// Consume the trailing `;` of a statement whose value is `expr`. Block-
    /// like expressions (blocks, if-expressions) self-terminate via their
    /// closing brace, so the `;` is optional after them; everything else
    /// requires it.
    fn expect_statement_terminator(&mut self, expr: &Expr) -> Result<(), ParseError> {
        if is_block_like_expr(expr) {
            if self.current_is(&[Token::Semicolon]) {
                self.advance();
            }
            Ok(())
        } else {
            self.expect(&[Token::Semicolon]).map(|_| ())
        }
    }

    // fn _peek_is(&mut self, tokens: &[Token]) -> bool {
    //     if let Some((token, _)) = self.peek() {
    //         tokens.iter().any(|t| token_matches(t, token))
    //     } else {
    //         false
    //     }
    // }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();

        while self.current_token.is_some() {
            let mut stmts = self.parse_statement()?;
            statements.append(&mut stmts);
        }

        Ok(Program { statements })
    }

    pub fn parse_statement(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let location = self
            .current_token
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        let kind = match &self.current_token {
            Some((Token::Int, _))
            | Some((Token::Char, _))
            | Some((Token::Float, _))
            | Some((Token::Bool, _))
            | Some((Token::Str, _))
            | Some((Token::Arr, _)) => {
                // This could be a variable declaration or a function declaration
                // We need to look ahead to determine which one it is

                // Save current state to restore if needed
                let saved_token = self.current_token.clone();
                let saved_pos = self.pos;

                // Parse the type
                let _type_annotation = self.parse_type()?;

                // Check if we have an identifier or a tuple pattern (
                if !self.current_is(&[Token::Ident(String::new())]) && !self.current_is(&[Token::LParen]) {
                    return Err(ParseError {
                        kind: ParseErrorKind::UnexpectedToken {
                            expected: "identifier or tuple pattern".to_string(),
                            found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                        },
                        location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                    });
                }

                // Check what comes next - either identifier or tuple pattern
                let has_ident = matches!(&self.current_token, Some((Token::Ident(_), _)));
                let has_tuple_pattern = self.current_is(&[Token::LParen]);

                if has_ident {
                    // Get the identifier
                    let _name = if let Some((Token::Ident(id), _)) = &self.current_token {
                        let name = id.clone();
                        self.advance();
                        name
                    } else {
                        unreachable!()
                    };

                    // Check what comes after the identifier
                    if self.current_is(&[Token::LParen]) {
                        // It's a function declaration
                        // Reset the parser state
                        self.current_token = saved_token.clone();
                        self.pos = saved_pos;
                        // Parse as a function
                        self.parse_function_declaration()?.kind
                    } else {
                        // It's a variable declaration or part of a structured binding
                        // Reset the parser state
                        self.current_token = saved_token.clone();
                        self.pos = saved_pos;
                        // Parse as a variable - this returns Vec<Stmt>, so return early
                        return self.parse_variable_declarations();
                    }
                } else if has_tuple_pattern {
                    // It's a tuple pattern: type (id1, id2) := ...
                    // Reset the parser state
                    self.current_token = saved_token.clone();
                    self.pos = saved_pos;
                    // Parse as a variable declaration with tuple pattern
                    return self.parse_variable_declarations();
                } else {
                    return Err(ParseError {
                        kind: ParseErrorKind::UnexpectedToken {
                            expected: "identifier or tuple pattern".to_string(),
                            found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                        },
                        location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                    });
                }
            }

            Some((Token::Mut, _)) => {
                // Parse mutable variable declarations - returns Vec<Stmt>
                return self.parse_variable_declarations();
            }

            Some((Token::BitAnd, _)) => {
                // `&T name(...)` is a function declaration returning a reference type.
                // `&T name := ...` is a variable declaration.
                // `&expr` is a bare take-reference expression statement.
                let saved_token = self.current_token.clone();
                let saved_pos = self.pos;
                self.advance(); // consume '&' to peek at what follows
                let next_is_type_start = self.current_is(&[
                    Token::Mut,
                    Token::Int,
                    Token::Bool,
                    Token::Float,
                    Token::Char,
                    Token::Str,
                    Token::Arr,
                    Token::BitAnd,
                ]);
                self.current_token = saved_token.clone();
                self.pos = saved_pos;

                if next_is_type_start {
                    // Parse the full return type to determine whether an ident
                    // followed by `(` follows — that means function declaration.
                    let saved_token2 = self.current_token.clone();
                    let saved_pos2 = self.pos;
                    let _ref_ty = self.parse_type()?;
                    let is_func = self.current_is(&[Token::Ident(String::new())])
                        && {
                            // peek past ident
                            let ident_saved = self.current_token.clone();
                            let ident_pos = self.pos;
                            self.advance();
                            let next_lparen = self.current_is(&[Token::LParen]);
                            self.current_token = ident_saved;
                            self.pos = ident_pos;
                            next_lparen
                        };
                    self.current_token = saved_token2;
                    self.pos = saved_pos2;

                    if is_func {
                        self.parse_function_declaration()?.kind
                    } else {
                        return self.parse_variable_declarations();
                    }
                } else {
                    let expr = self.parse_expression()?;
                    self.expect_statement_terminator(&expr)?;
                    StmtKind::ExprStmt(expr)
                }
            }

            Some((Token::Ident(_name), _)) => {
                // Parse as expression (could be function call, identifier, etc.)
                let expr = self.parse_expression()?;

                // Check if this is an assignment
                if self.current_is(&[Token::Assign]) {
                    self.advance(); // consume '='
                    let value = self.parse_expression()?;
                    self.expect_statement_terminator(&value)?;
                    StmtKind::Assignment {
                        target: expr,
                        value,
                    }
                } else {
                    // Expression statement (including function calls)
                    self.expect_statement_terminator(&expr)?;
                    StmtKind::ExprStmt(expr)
                }
            }

            Some((Token::Return, _)) => {
                self.advance(); // consume 'return'
                let expr = if self.current_is(&[Token::Semicolon]) {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                self.expect(&[Token::Semicolon])?;
                StmtKind::ReturnStmt(expr)
            }

            Some((Token::LParen, _)) => {
                // Check if this is a tuple destructuring pattern: (type id, type id) := ...
                // Save state in case it's just a parenthesized expression
                let saved_token = self.current_token.clone();
                let saved_pos = self.pos;

                self.advance(); // consume '('

                // Try to parse as tuple pattern. The next token is either a
                // type keyword (flat pattern) or another '(' (nested pattern).
                let is_tuple_pattern = self.current_is(&[
                    Token::Int,
                    Token::Bool,
                    Token::Float,
                    Token::Char,
                    Token::Str,
                    Token::LParen,
                ]);

                // Restore state
                self.current_token = saved_token.clone();
                self.pos = saved_pos;

                if is_tuple_pattern {
                    // Parse as tuple destructuring
                    return self.parse_tuple_destructuring();
                } else {
                    // It's an expression statement (like a tuple expression or function call)
                    let expr = self.parse_expression()?;
                    self.expect_statement_terminator(&expr)?;
                    StmtKind::ExprStmt(expr)
                }
            }

            Some((Token::While, _)) => {
                self.advance(); // consume 'while'
                let condition = self.parse_expression()?;
                let body = self.parse_block_expression()?;
                StmtKind::WhileLoop { condition, body: Box::new(body) }
            }

            Some((Token::Break, _)) => {
                self.advance(); // consume 'break'
                self.expect(&[Token::Semicolon])?;
                StmtKind::Break
            }

            Some((Token::Continue, _)) => {
                self.advance(); // consume 'continue'
                self.expect(&[Token::Semicolon])?;
                StmtKind::Continue
            }

            Some((Token::For, _)) => {
                self.advance(); // consume 'for'
                let var = self.parse_ident()?;
                self.expect(&[Token::In])?;
                let start = self.parse_expression()?;
                self.expect(&[Token::DotDot])?;
                let end = self.parse_expression()?;
                let range_loc = start.location.clone();
                let iterable = Expr {
                    kind: ExprKind::Range {
                        start: Box::new(start),
                        end: Box::new(end),
                    },
                    location: range_loc,
                };
                let body = self.parse_block_expression()?;
                StmtKind::ForIn {
                    var,
                    iterable: Box::new(iterable),
                    body: Box::new(body),
                }
            }

            Some((Token::Discord, _)) => {
                self.advance(); // consume 'discord'
                let body = self.parse_block_expression()?;
                StmtKind::DiscordBlock { body: Box::new(body) }
            }

            Some((Token::Fnord, _)) => {
                self.advance(); // consume 'fnord'
                self.expect(&[Token::Semicolon])?;
                StmtKind::Fnord
            }

            Some((_, _loc)) => {
                // Try to parse it as an expression statement or deref assignment (*r = v).
                let expr = self.parse_expression()?;
                if self.current_is(&[Token::Assign])
                    && matches!(expr.kind, ExprKind::Deref(_))
                {
                    self.advance(); // consume '='
                    let value = self.parse_expression()?;
                    self.expect_statement_terminator(&value)?;
                    StmtKind::Assignment { target: expr, value }
                } else {
                    self.expect_statement_terminator(&expr)?;
                    StmtKind::ExprStmt(expr)
                }
            }

            None => {
                return Err(ParseError {
                    kind: ParseErrorKind::InvalidSyntax("Unexpected end of file".to_string()),
                    location: Some(self.last_location.clone()),
                })
            }
        };
        Ok(vec![Stmt { kind, location }])
    }

    fn parse_tuple_destructuring(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let location = self
            .current_token
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();

        let pattern = self.parse_tuple_pattern()?;
        self.expect(&[Token::DeclareAssign])?;

        let value = self.parse_expression()?;
        self.expect(&[Token::Semicolon])?;

        let mut statements = Vec::new();
        flatten_tuple_pattern(&pattern, &value, &location, &mut statements)?;
        Ok(statements)
    }

    /// Parse a tuple-destructuring pattern: `(<elem>, <elem>, ...)` where each
    /// element is either a typed binding (`type id`) or another tuple pattern.
    fn parse_tuple_pattern(&mut self) -> Result<TuplePattern, ParseError> {
        self.expect(&[Token::LParen])?;
        let mut elements = Vec::new();
        loop {
            elements.push(self.parse_tuple_pattern_element()?);
            if !self.current_is(&[Token::Comma]) {
                break;
            }
            self.advance(); // consume ','
        }
        self.expect(&[Token::RParen])?;
        Ok(TuplePattern::Tuple(elements))
    }

    fn parse_tuple_pattern_element(&mut self) -> Result<TuplePattern, ParseError> {
        if self.current_is(&[Token::LParen]) {
            self.parse_tuple_pattern()
        } else {
            let typ = self.parse_type()?;
            let ident = self.parse_ident()?;
            Ok(TuplePattern::Binding { typ, ident })
        }
    }

    fn parse_variable_declarations(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let location = self
            .current_token
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        let mutable = if self.current_is(&[Token::Mut]) {
            self.advance(); // consume 'mut'
            true
        } else {
            false
        };

        // Parse type annotation (if present)
        let typ = if self.current_is(&[Token::Int, Token::Bool, Token::Float, Token::Char, Token::Str, Token::Arr, Token::BitAnd]) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Check if we have a tuple pattern: type (id1, id2, ...)
        let idents = if self.current_is(&[Token::LParen]) && typ.is_some() {
            // Tuple pattern with shared type: int (a, b) := (1, 2);
            self.advance(); // consume '('
            let mut ids = vec![self.parse_ident()?];
            while self.current_is(&[Token::Comma]) {
                self.advance(); // consume comma
                ids.push(self.parse_ident()?);
            }
            self.expect(&[Token::RParen])?;
            ids
        } else {
            // Regular identifier(s) - collect all if comma-separated
            let mut ids = vec![self.parse_ident()?];
            while self.current_is(&[Token::Comma]) {
                self.advance(); // consume comma
                ids.push(self.parse_ident()?);
            }
            ids
        };

        // Expect declare-assign operator :=
        self.expect(&[Token::DeclareAssign])?;

        // Parse initializer expression(s) - collect all if comma-separated
        let mut initializers = vec![self.parse_expression()?];
        while self.current_is(&[Token::Comma]) {
            self.advance(); // consume comma
            initializers.push(self.parse_expression()?);
        }

        // Block-like initializers (block expressions, if-expressions) are
        // self-terminating: their closing brace marks the end of the
        // statement, so the trailing semicolon is optional. For everything
        // else the semicolon is required.
        if initializers.len() == 1 && is_block_like_expr(&initializers[0]) {
            if self.current_is(&[Token::Semicolon]) {
                self.advance();
            }
        } else {
            self.expect(&[Token::Semicolon])?;
        }

        // Check if we have a single tuple expression on the right with multiple identifiers
        if idents.len() > 1 && initializers.len() == 1 {
            // Check if the single initializer is a tuple expression
            if let ExprKind::TupleExpr(ref elements) = initializers[0].kind {
                if elements.len() != idents.len() {
                    return Err(ParseError {
                        kind: ParseErrorKind::InvalidSyntax(format!(
                            "Tuple destructuring mismatch: {} bindings but {} values",
                            idents.len(),
                            elements.len()
                        )),
                        location: Some(location),
                    });
                }

                // Create a statement for each identifier/tuple element pair
                let statements = idents
                    .into_iter()
                    .zip(elements.iter())
                    .map(|(ident, initializer)| Stmt {
                        kind: StmtKind::VarDecl {
                            ident,
                            mutable,
                            typ: typ.clone(),
                            initializer: Box::new(initializer.clone()),
                        },
                        location: location.clone(),
                    })
                    .collect();

                return Ok(statements);
            }
        }

        // Check that we have matching counts for regular multiple bindings
        if idents.len() != initializers.len() {
            return Err(ParseError {
                kind: ParseErrorKind::InvalidSyntax(format!(
                    "Mismatch between number of identifiers ({}) and initializers ({})",
                    idents.len(),
                    initializers.len()
                )),
                location: Some(location),
            });
        }

        // Create a statement for each identifier/initializer pair
        let statements = idents
            .into_iter()
            .zip(initializers.into_iter())
            .map(|(ident, initializer)| Stmt {
                kind: StmtKind::VarDecl {
                    ident,
                    mutable,
                    typ: typ.clone(),
                    initializer: Box::new(initializer),
                },
                location: location.clone(),
            })
            .collect();

        Ok(statements)
    }

    fn parse_function_declaration(&mut self) -> Result<Stmt, ParseError> {
        let location = self
            .current_token
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        // Parse return type (if any)
        let return_type = if self.current_is(&[Token::Int, Token::Bool, Token::Float, Token::Char, Token::Str, Token::Arr, Token::BitAnd]) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse function name
        let name = if let Some((Token::Ident(id), loc)) = &self.current_token {
            let name = id.clone();
            let location = loc.clone();
            self.advance();
            Identifier { name, location }
        } else {
            return Err(ParseError {
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "identifier".to_string(),
                    found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                },
                location: self.current_token.as_ref().map(|(_, l)| l.clone()),
            });
        };

        // Parse parameters
        self.expect(&[Token::LParen])?;
        let mut parameters = Vec::new();

        if !self.current_is(&[Token::RParen]) {
            loop {
                // Optional `mut` before parameter type
                let mutable = if self.current_is(&[Token::Mut]) {
                    self.advance();
                    true
                } else {
                    false
                };

                // Parse parameter type
                let param_type = self.parse_type()?;

                // Parse parameter name
                if let Some((Token::Ident(id), loc)) = &self.current_token {
                    let param_name = Identifier {
                        name: id.clone(),
                        location: loc.clone(),
                    };
                    self.advance();
                    parameters.push(Param { mutable, ident: param_name, typ: param_type });
                } else {
                    return Err(ParseError {
                        kind: ParseErrorKind::UnexpectedToken {
                            expected: "identifier".to_string(),
                            found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                        },
                        location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                    });
                }

                if !self.current_is(&[Token::Comma]) {
                    break;
                }
                self.advance(); // consume comma
            }
        }

        self.expect(&[Token::RParen])?;

        // Parse function body
        let body = self.parse_block_expression()?;

        Ok(Stmt {
            kind: StmtKind::FuncDecl {
                name,
                parameters,
                return_type,
                body,
            },
            location,
        })
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let location = self
            .current_token
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        if self.current_is(&[Token::BitAnd]) {
            self.advance(); // consume '&'
            let mutable = if self.current_is(&[Token::Mut]) {
                self.advance();
                true
            } else {
                false
            };
            let inner = self.parse_type()?;
            return Ok(Type {
                kind: TypeKind::Ref { inner: Box::new(inner.kind), mutable },
                location,
            });
        }
        let kind = match &self.current_token {
            Some((Token::Int, _)) => {
                self.advance();
                TypeKind::Int
            }
            Some((Token::Bool, _)) => {
                self.advance();
                TypeKind::Bool
            }
            Some((Token::Char, _)) => {
                self.advance();
                TypeKind::Char
            }
            Some((Token::Float, _)) => {
                self.advance();
                TypeKind::Float
            }
            Some((Token::Str, _)) => {
                self.advance();
                TypeKind::Str
            }
            Some((Token::Arr, _)) => {
                self.advance();
                self.expect(&[Token::LBracket])?;
                let element_type = self.parse_type()?;
                self.expect(&[Token::RBracket])?;
                TypeKind::Array { element: Box::new(element_type.kind), size: 0 }
            }
            _ => {
                return Err(ParseError {
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "type".to_string(),
                        found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                    },
                    location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                });
            }
        };
        Ok(Type { kind, location })
    }

    fn parse_ident(&mut self) -> Result<Identifier, ParseError> {
        match &self.current_token {
            Some((Token::Ident(name), loc)) => {
                let id = Identifier {
                    name: name.clone(),
                    location: loc.clone(),
                };
                self.advance();
                Ok(id)
            }
            _ => Err(ParseError {
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "pattern".to_string(),
                    found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                },
                location: self.current_token.as_ref().map(|(_, l)| l.clone()),
            }),
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary_expression(Precedence::Lowest)
    }

    fn parse_binary_expression(&mut self, precedence: Precedence) -> Result<Expr, ParseError> {
        let mut left = self.parse_postfix_expression()?;

        while let Some(op_precedence) = self.current_binary_operator_precedence() {
            if op_precedence < precedence {
                break;
            }

            let operator = self.parse_binary_operator()?;
            let right = self.parse_binary_expression(op_precedence + 1)?;
            let location = left.location.clone();

            left = Expr {
                kind: ExprKind::BinaryExpr {
                    left: Box::new(left),
                    op: operator,
                    right: Box::new(right),
                },
                location,
            };
        }

        Ok(left)
    }

    // Bitwise ops sit above comparisons (Rust-style), not below them (C-style),
    // so `a & b == c` parses as `(a & b) == c`. This avoids the C precedence footgun.
    fn current_binary_operator_precedence(&self) -> Option<Precedence> {
        match &self.current_token {
            Some((Token::LParen, _)) => Some(Precedence::Group),
            Some((Token::Equal, _)) | Some((Token::NotEqual, _)) => Some(Precedence::Equality),
            Some((Token::GreaterThan, _))
            | Some((Token::GreaterEqual, _))
            | Some((Token::LessThan, _))
            | Some((Token::LessEqual, _)) => Some(Precedence::Comparison),
            Some((Token::BitOr, _)) => Some(Precedence::BitOr),
            Some((Token::Caret, _)) => Some(Precedence::BitXor),
            Some((Token::BitAnd, _)) => Some(Precedence::BitAnd),
            Some((Token::Shl, _)) | Some((Token::Shr, _)) => Some(Precedence::Shift),
            Some((Token::Plus, _)) | Some((Token::Minus, _)) => Some(Precedence::Sum),
            Some((Token::Multiply, _)) | Some((Token::Divide, _)) | Some((Token::Modulo, _)) => {
                Some(Precedence::Product)
            }
            Some((Token::Or, _)) => Some(Precedence::LogicalOr),
            Some((Token::And, _)) => Some(Precedence::LogicalAnd),
            _ => None,
        }
    }

    fn parse_binary_operator(&mut self) -> Result<BinOp, ParseError> {
        let op = match &self.current_token {
            Some((Token::Plus, _)) => {
                self.advance();
                BinOp::Add
            }
            Some((Token::Minus, _)) => {
                self.advance();
                BinOp::Sub
            }
            Some((Token::Multiply, _)) => {
                self.advance();
                BinOp::Mul
            }
            Some((Token::Divide, _)) => {
                self.advance();
                BinOp::Div
            }
            Some((Token::Modulo, _)) => {
                self.advance();
                BinOp::Rem
            }
            Some((Token::Equal, _)) => {
                self.advance();
                BinOp::Eq
            }
            Some((Token::NotEqual, _)) => {
                self.advance();
                BinOp::Ne
            }
            Some((Token::LessThan, _)) => {
                self.advance();
                BinOp::Lt
            }
            Some((Token::LessEqual, _)) => {
                self.advance();
                BinOp::Le
            }
            Some((Token::GreaterThan, _)) => {
                self.advance();
                BinOp::Gt
            }
            Some((Token::GreaterEqual, _)) => {
                self.advance();
                BinOp::Ge
            }
            Some((Token::And, _)) => {
                self.advance();
                BinOp::LAnd
            }
            Some((Token::Or, _)) => {
                self.advance();
                BinOp::LOr
            }
            Some((Token::BitAnd, _)) => {
                self.advance();
                BinOp::BAnd
            }
            Some((Token::BitOr, _)) => {
                self.advance();
                BinOp::BOr
            }
            Some((Token::Caret, _)) => {
                self.advance();
                BinOp::BXor
            }
            Some((Token::Shl, _)) => {
                self.advance();
                BinOp::Shl
            }
            Some((Token::Shr, _)) => {
                self.advance();
                BinOp::Shr
            }
            _ => {
                return Err(ParseError {
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "binary operator".to_string(),
                        found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                    },
                    location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                });
            }
        };

        Ok(op)
    }

    fn parse_postfix_expression(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary_expression()?;
        loop {
            match &self.current_token {
                Some((Token::LBracket, _)) => {
                    self.advance(); // consume '['
                    let index = self.parse_expression()?;
                    self.expect(&[Token::RBracket])?;
                    let location = expr.location.clone();
                    expr = Expr {
                        kind: ExprKind::ArrayIndex {
                            array: Box::new(expr),
                            index: Box::new(index),
                        },
                        location,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary_expression(&mut self) -> Result<Expr, ParseError> {
        let location = self
            .current_token
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        let kind = match &self.current_token {
            // Handle unary operators (prefix expressions)
            Some((Token::Not, _)) => {
                self.advance(); // consume '!'
                let expression = Box::new(self.parse_postfix_expression()?);
                ExprKind::UnaryExpr {
                    op: crate::parser::ast::UnaryOp::LNot,
                    expression,
                }
            }
            Some((Token::Minus, _)) => {
                self.advance(); // consume '-'
                let expression = Box::new(self.parse_postfix_expression()?);
                ExprKind::UnaryExpr {
                    op: crate::parser::ast::UnaryOp::Neg,
                    expression,
                }
            }
            Some((Token::Tilde, _)) => {
                self.advance(); // consume '~'
                let expression = Box::new(self.parse_postfix_expression()?);
                ExprKind::UnaryExpr {
                    op: crate::parser::ast::UnaryOp::BNot,
                    expression,
                }
            }
            Some((Token::BitAnd, _)) => {
                self.advance(); // consume '&'
                let mutable = if self.current_is(&[Token::Mut]) {
                    self.advance();
                    true
                } else {
                    false
                };
                let place = Box::new(self.parse_postfix_expression()?);
                ExprKind::TakeRef { mutable, place }
            }
            Some((Token::Multiply, _)) => {
                self.advance(); // consume '*'
                let inner = Box::new(self.parse_postfix_expression()?);
                ExprKind::Deref(inner)
            }

            Some((Token::IntLit(value), _)) => {
                let val = *value;
                self.advance();
                ExprKind::IntLit(val)
            }

            Some((Token::FloatLit(value), _)) => {
                let val = *value;
                self.advance();
                ExprKind::FloatLit(val)
            }

            Some((Token::CharLit(value), _)) => {
                let val = *value;
                self.advance();
                ExprKind::CharLit(val)
            }

            Some((Token::StringLit(value), _)) => {
                let val = value.clone();
                self.advance();
                ExprKind::StringLit(val)
            }

            Some((Token::True, _)) => {
                self.advance();
                ExprKind::BoolTrue
            }

            Some((Token::False, _)) => {
                self.advance();
                ExprKind::BoolFalse
            }

            Some((Token::Ident(name), loc)) => {
                let id_name = name.clone();
                let id_loc = loc.clone();
                self.advance();

                // Check if this is a function call
                if self.current_is(&[Token::LParen]) {
                    self.advance(); // consume '('
                    let mut arguments = Vec::new();

                    if !self.current_is(&[Token::RParen]) {
                        loop {
                            arguments.push(self.parse_expression()?);

                            if !self.current_is(&[Token::Comma]) {
                                break;
                            }
                            self.advance(); // consume comma
                        }
                    }

                    self.expect(&[Token::RParen])?;

                    ExprKind::FunctionCall {
                        function: Box::new(Identifier {
                            name: id_name,
                            location: id_loc,
                        }),
                        arguments,
                    }
                } else {
                    ExprKind::IdentExpr(Identifier {
                        name: id_name,
                        location: id_loc,
                    })
                }
            }

            Some((Token::LParen, _)) => {
                self.advance(); // consume '('

                // Check if this is a tuple expression
                let mut expressions = Vec::new();
                expressions.push(self.parse_expression()?);

                if self.current_is(&[Token::Comma]) {
                    self.advance(); // consume comma

                    // It's a tuple expression
                    if !self.current_is(&[Token::RParen]) {
                        loop {
                            expressions.push(self.parse_expression()?);

                            if !self.current_is(&[Token::Comma]) {
                                break;
                            }
                            self.advance(); // consume comma
                        }
                    }

                    self.expect(&[Token::RParen])?;
                    ExprKind::TupleExpr(expressions)
                } else {
                    // It's a parenthesized expression
                    let expr = expressions.pop().unwrap();
                    self.expect(&[Token::RParen])?;
                    ExprKind::ParenExpr(Box::new(expr))
                }
            }

            Some((Token::LBrace, _)) => return self.parse_block_expression(),

            Some((Token::LBracket, _)) => {
                // Array literal: [expr, expr, ...]
                self.advance(); // consume '['

                let mut elements = Vec::new();

                if !self.current_is(&[Token::RBracket]) {
                    loop {
                        elements.push(self.parse_expression()?);

                        if !self.current_is(&[Token::Comma]) {
                            break;
                        }
                        self.advance(); // consume comma
                    }
                }

                self.expect(&[Token::RBracket])?;
                ExprKind::ArrayLit(elements)
            }

            Some((Token::If, _)) => {
                self.advance(); // consume 'if'

                let condition = self.parse_expression()?;

                // Parse the 'then' branch (block)
                // self.expect(&[Token::LBrace])?;
                let then_branch = if self.current_is(&[Token::LBrace]) {
                    Some(Box::new(self.parse_block_expression()?))
                } else {
                    None
                };

                // Parse the 'else' branch if present
                let else_branch = if self.current_is(&[Token::Else]) {
                    self.advance(); // consume 'else'

                    if self.current_is(&[Token::If]) {
                        // This is an 'else if'
                        Some(Box::new(self.parse_expression()?))
                    } else {
                        // This is a regular 'else'
                        // self.expect(&[Token::LBrace])?;
                        if self.current_is(&[Token::LBrace]) {
                            Some(Box::new(self.parse_block_expression()?))
                        } else {
                            None
                        }
                    }
                } else {
                    None
                };

                ExprKind::IfExpr {
                    condition: Box::new(condition),
                    then_branch,
                    else_branch,
                }
            }

            _ => {
                return Err(ParseError {
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "expr".to_string(),
                        found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                    },
                    location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                });
            }
        };
        Ok(Expr { kind, location })
    }

    fn parse_block_expression(&mut self) -> Result<Expr, ParseError> {
        let location = self
            .current_token
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        self.expect(&[Token::LBrace])?;

        let mut statements = Vec::new();
        let mut final_expr = None;

        while !self.current_is(&[Token::RBrace]) {
            if self.is_expression_start() && !self.look_ahead_for_semicolon() {
                final_expr = Some(Box::new(self.parse_expression()?));
                break;
            }

            let mut stmts = self.parse_statement()?;
            statements.append(&mut stmts);
        }

        self.expect(&[Token::RBrace])?;

        Ok(Expr {
            kind: ExprKind::BlockExpr {
                statements,
                ret: final_expr,
            },
            location,
        })
    }

    fn is_expression_start(&self) -> bool {
        match &self.current_token {
            Some((Token::IntLit(_), _))
            | Some((Token::FloatLit(_), _))
            | Some((Token::StringLit(_), _))
            | Some((Token::Ident(_), _))
            | Some((Token::LParen, _))
            | Some((Token::LBrace, _))
            | Some((Token::LBracket, _))
            | Some((Token::If, _)) => true,
            _ => false,
        }
    }

    fn look_ahead_for_semicolon(&self) -> bool {
        // Scan forward from the current token (inclusive). Track paren/brace
        // depth so a `;` only counts when it sits at the same nesting level
        // we started at — i.e. when we've returned to balance 0 outside any
        // group opened in the interim.
        let mut balance: i32 = 0;
        for (token, _) in &self.tokens[self.pos.min(self.tokens.len())..] {
            match token {
                Token::LBrace | Token::LParen => balance += 1,
                Token::RBrace | Token::RParen => {
                    balance -= 1;
                    if balance < 0 {
                        // We've exited a group we never entered — the
                        // statement boundary is enclosing us. No `;` at our
                        // level.
                        return false;
                    }
                }
                Token::Semicolon => {
                    if balance == 0 {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
}

// Helper function to check if two tokens match, handling special cases like Ident
fn token_matches(expected: &Token, actual: &Token) -> bool {
    match (expected, actual) {
        (Token::Ident(_), Token::Ident(_)) => true,
        (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

/// True for expressions whose closing `}` is sufficient to terminate a
/// statement — `let x := { ... }` and `let x := if c { ... } else { ... }`
/// don't require a trailing `;`.
fn is_block_like_expr(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::BlockExpr { .. } | ExprKind::IfExpr { .. })
}

/// Parser-internal representation of a tuple-destructuring pattern. Lowered
/// into flat `VarDecl` statements after parsing.
#[derive(Debug, Clone)]
enum TuplePattern {
    Binding { typ: Type, ident: Identifier },
    Tuple(Vec<TuplePattern>),
}

/// Walk `pattern` and `value` in parallel, emitting one `VarDecl` per leaf
/// binding into `out`. Errors when arities mismatch at any level or when the
/// pattern expects a sub-tuple but the value is a scalar (or vice versa).
fn flatten_tuple_pattern(
    pattern: &TuplePattern,
    value: &Expr,
    location: &SourceLocation,
    out: &mut Vec<Stmt>,
) -> Result<(), ParseError> {
    match pattern {
        TuplePattern::Binding { typ, ident } => {
            out.push(Stmt {
                kind: StmtKind::VarDecl {
                    ident: ident.clone(),
                    mutable: false,
                    typ: Some(typ.clone()),
                    initializer: Box::new(value.clone()),
                },
                location: location.clone(),
            });
            Ok(())
        }
        TuplePattern::Tuple(elements) => match &value.kind {
            ExprKind::TupleExpr(values) if values.len() == elements.len() => {
                for (sub_pat, sub_val) in elements.iter().zip(values.iter()) {
                    flatten_tuple_pattern(sub_pat, sub_val, location, out)?;
                }
                Ok(())
            }
            ExprKind::TupleExpr(values) => Err(ParseError {
                kind: ParseErrorKind::InvalidSyntax(format!(
                    "Tuple destructuring mismatch: pattern expects {} element(s), value has {}",
                    elements.len(),
                    values.len()
                )),
                location: Some(value.location.clone()),
            }),
            _ => Err(ParseError {
                kind: ParseErrorKind::InvalidSyntax(
                    "Expected tuple expression on right side of tuple destructuring".to_string(),
                ),
                location: Some(value.location.clone()),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_source(source: &str) -> Result<Program, ParseError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    #[test]
    fn test_simple_variable_declaration() {
        let source = "int x := 10;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { ident, mutable, typ, .. } => {
                assert_eq!(ident.name, "x");
                assert!(!mutable);
                assert!(matches!(typ.as_ref().unwrap().kind, TypeKind::Int));
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_mutable_variable_declaration() {
        let source = "mut int y := 20;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { ident, mutable, .. } => {
                assert_eq!(ident.name, "y");
                assert!(mutable);
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_multiple_variable_declarations() {
        // The syntax is: int x, y := 1, 2; (same type, multiple variables)
        let source = "int x, y := 1, 2;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 2);
        match &program.statements[0].kind {
            StmtKind::VarDecl { ident, .. } => assert_eq!(ident.name, "x"),
            _ => panic!("Expected VarDecl"),
        }
        match &program.statements[1].kind {
            StmtKind::VarDecl { ident, .. } => assert_eq!(ident.name, "y"),
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_tuple_destructuring() {
        let source = "int (a, b) := (1, 2);";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 2);
        match &program.statements[0].kind {
            StmtKind::VarDecl { ident, .. } => assert_eq!(ident.name, "a"),
            _ => panic!("Expected VarDecl"),
        }
        match &program.statements[1].kind {
            StmtKind::VarDecl { ident, .. } => assert_eq!(ident.name, "b"),
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_nested_tuple_destructuring_in_paren_form() {
        // (str a, (int b, str c)) := ("x", (1, "y"));
        // Lowers to three flat var decls: a := "x", b := 1, c := "y".
        let source = r#"(str a, (int b, str c)) := ("x", (1, "y"));"#;
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 3);
        let names: Vec<&str> = program.statements.iter().map(|s| match &s.kind {
            StmtKind::VarDecl { ident, .. } => ident.name.as_str(),
            _ => panic!("Expected VarDecl, got {:?}", s.kind),
        }).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_deeply_nested_tuple_destructuring() {
        // ((int a, int b), (int c, int d)) := ((1, 2), (3, 4));
        let source = "((int a, int b), (int c, int d)) := ((1, 2), (3, 4));";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 4);
        let names: Vec<&str> = program.statements.iter().map(|s| match &s.kind {
            StmtKind::VarDecl { ident, .. } => ident.name.as_str(),
            _ => panic!("Expected VarDecl"),
        }).collect();
        assert_eq!(names, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_block_initializer_trailing_semicolon_optional() {
        // The closing brace of a block expression marks the end of the
        // statement; the trailing semicolon is optional.
        let source = "int result := { int n := 5; n * 2 }";
        let program = parse_source(source).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { ident, initializer, .. } => {
                assert_eq!(ident.name, "result");
                assert!(matches!(initializer.kind, ExprKind::BlockExpr { .. }));
            }
            _ => panic!("Expected VarDecl with BlockExpr initializer"),
        }
    }

    #[test]
    fn test_block_initializer_followed_by_next_statement() {
        // Block as initializer, no trailing ;, then another statement.
        let source = "int a := { 1 + 2 } int b := 7;";
        let program = parse_source(source).unwrap();
        assert_eq!(program.statements.len(), 2);
        match &program.statements[1].kind {
            StmtKind::VarDecl { ident, .. } => assert_eq!(ident.name, "b"),
            _ => panic!("Expected second VarDecl"),
        }
    }

    #[test]
    fn test_if_initializer_trailing_semicolon_optional() {
        let source = "int x := if true { 1 } else { 2 }";
        let program = parse_source(source).unwrap();
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                assert!(matches!(initializer.kind, ExprKind::IfExpr { .. }));
            }
            _ => panic!("Expected VarDecl with IfExpr initializer"),
        }
    }

    #[test]
    fn test_non_block_initializer_still_requires_semicolon() {
        // A scalar initializer without ; is still an error.
        let source = "int x := 10";
        assert!(parse_source(source).is_err());
    }

    #[test]
    fn test_nested_tuple_destructuring_arity_mismatch_errors() {
        // Outer tuple has 2 patterns, value has 3 elements — should error.
        let source = "(int a, (int b, int c)) := (1, (2, 3), 4);";
        let result = parse_source(source);
        assert!(result.is_err(), "expected arity mismatch to error, got {:?}", result);
    }

    #[test]
    fn test_function_declaration() {
        let source = "int add(int a, int b) { return a + b; }";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::FuncDecl { name, parameters, return_type, .. } => {
                assert_eq!(name.name, "add");
                assert_eq!(parameters.len(), 2);
                assert!(matches!(return_type.as_ref().unwrap().kind, TypeKind::Int));
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_function_with_no_params() {
        let source = "int get_value() { return 42; }";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::FuncDecl { name, parameters, .. } => {
                assert_eq!(name.name, "get_value");
                assert_eq!(parameters.len(), 0);
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_binary_expressions() {
        let source = "int x := 1 + 2 * 3;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                // Should parse as 1 + (2 * 3) due to precedence
                match &initializer.kind {
                    ExprKind::BinaryExpr { op, .. } => {
                        assert!(matches!(op, BinOp::Add));
                    }
                    _ => panic!("Expected BinaryExpr"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_comparison_operators() {
        let source = "bool result := 5 > 3;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                match &initializer.kind {
                    ExprKind::BinaryExpr { op, .. } => {
                        assert!(matches!(op, BinOp::Gt));
                    }
                    _ => panic!("Expected BinaryExpr"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_unary_expressions() {
        let source = "int x := -5;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                match &initializer.kind {
                    ExprKind::UnaryExpr { op, .. } => {
                        assert!(matches!(op, crate::parser::ast::UnaryOp::Neg));
                    }
                    _ => panic!("Expected UnaryExpr"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_logical_not() {
        let source = "bool x := !true;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                match &initializer.kind {
                    ExprKind::UnaryExpr { op, .. } => {
                        assert!(matches!(op, crate::parser::ast::UnaryOp::LNot));
                    }
                    _ => panic!("Expected UnaryExpr"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_if_expression() {
        let source = "int x := if true { 1 } else { 2 };";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                match &initializer.kind {
                    ExprKind::IfExpr { condition, then_branch, else_branch } => {
                        assert!(matches!(condition.kind, ExprKind::BoolTrue));
                        assert!(then_branch.is_some());
                        assert!(else_branch.is_some());
                    }
                    _ => panic!("Expected IfExpr"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_function_call() {
        let source = "int result := add(1, 2);";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                match &initializer.kind {
                    ExprKind::FunctionCall { function, arguments } => {
                        assert_eq!(function.name, "add");
                        assert_eq!(arguments.len(), 2);
                    }
                    _ => panic!("Expected FunctionCall"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_assignment_statement() {
        let source = "mut int x := 10; x = 20;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 2);
        match &program.statements[1].kind {
            StmtKind::Assignment { .. } => {
                // Successfully parsed assignment
            }
            _ => panic!("Expected Assignment"),
        }
    }

    #[test]
    fn test_return_statement() {
        let source = "int foo() { return 42; }";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::FuncDecl { body, .. } => {
                match &body.kind {
                    ExprKind::BlockExpr { statements, .. } => {
                        assert_eq!(statements.len(), 1);
                        assert!(matches!(statements[0].kind, StmtKind::ReturnStmt(_)));
                    }
                    _ => panic!("Expected BlockExpr"),
                }
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_block_expression() {
        let source = "int x := { int y := 5; y + 1 };";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                match &initializer.kind {
                    ExprKind::BlockExpr { statements, ret } => {
                        assert_eq!(statements.len(), 1);
                        assert!(ret.is_some());
                    }
                    _ => panic!("Expected BlockExpr"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_tuple_expression() {
        let source = "int x := (1, 2, 3);";
        let _program = parse_source(source).unwrap();

        // Note: This will result in tuple destructuring since we have
        // int (x) which is parsed as tuple pattern
        // Let's test a simpler case
        let source2 = "int x := (1,);";
        let mut lexer = Lexer::new(source2);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program2 = parser.parse_program().unwrap();

        // The parser should handle this somehow
        assert!(!program2.statements.is_empty());
    }

    #[test]
    fn test_array_literal() {
        let source = "arr[int] nums := [1, 2, 3];";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, typ, .. } => {
                assert!(matches!(initializer.kind, ExprKind::ArrayLit(_)));
                assert!(matches!(typ.as_ref().unwrap().kind, TypeKind::Array { .. }));
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_parenthesized_expression() {
        let source = "int x := (1 + 2) * 3;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                // Should parse as (1 + 2) * 3, not 1 + (2 * 3)
                match &initializer.kind {
                    ExprKind::BinaryExpr { op, left, .. } => {
                        assert!(matches!(op, BinOp::Mul));
                        assert!(matches!(left.kind, ExprKind::ParenExpr(_)));
                    }
                    _ => panic!("Expected BinaryExpr"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_nested_if_expressions() {
        let source = "int x := if true { if false { 1 } else { 2 } } else { 3 };";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                assert!(matches!(initializer.kind, ExprKind::IfExpr { .. }));
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_char_literal() {
        let source = "char c := 'a';";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                match &initializer.kind {
                    ExprKind::CharLit('a') => {},
                    _ => panic!("Expected CharLit('a')"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_float_literal() {
        let source = "float f := 3.14;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                assert!(matches!(initializer.kind, ExprKind::FloatLit(_)));
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_string_literal() {
        let source = r#"str s := "hello";"#;
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                match &initializer.kind {
                    ExprKind::StringLit(s) => assert_eq!(s, "hello"),
                    _ => panic!("Expected StringLit"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_logical_operators() {
        let source = "bool x := true && false || true;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => {
                // Should parse with correct precedence
                assert!(matches!(initializer.kind, ExprKind::BinaryExpr { .. }));
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_logical_and_binds_tighter_than_or() {
        let source = "bool x := true || false && false;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => match &initializer.kind {
                ExprKind::BinaryExpr { op, right, .. } => {
                    assert!(matches!(op, BinOp::LOr), "outer op should be Or");
                    match &right.kind {
                        ExprKind::BinaryExpr { op: inner_op, .. } => {
                            assert!(matches!(inner_op, BinOp::LAnd), "inner op should be And");
                        }
                        _ => panic!("Expected nested BinaryExpr on the right"),
                    }
                }
                _ => panic!("Expected BinaryExpr"),
            },
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_logical_ops_bind_looser_than_comparisons() {
        let source = "bool x := 1 < 2 && 3 > 0;";
        let program = parse_source(source).unwrap();

        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StmtKind::VarDecl { initializer, .. } => match &initializer.kind {
                ExprKind::BinaryExpr { op, .. } => {
                    assert!(matches!(op, BinOp::LAnd), "outer op should be And, not a comparison");
                }
                _ => panic!("Expected BinaryExpr"),
            },
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_error_unexpected_token() {
        let source = "int x :=;";
        let result = parse_source(source);

        assert!(result.is_err());
        match result {
            Err(ParseError { kind: ParseErrorKind::UnexpectedToken { .. }, .. }) => {},
            _ => panic!("Expected UnexpectedToken error"),
        }
    }

    #[test]
    fn test_error_missing_semicolon() {
        let source = "int x := 10";
        let result = parse_source(source);

        assert!(result.is_err());
    }

    #[test]
    fn test_error_tuple_length_mismatch() {
        let source = "int (a, b) := (1, 2, 3);";
        let result = parse_source(source);

        assert!(result.is_err());
        match result {
            Err(ParseError { kind: ParseErrorKind::InvalidSyntax(msg), .. }) => {
                assert!(msg.contains("mismatch"));
            },
            _ => panic!("Expected InvalidSyntax error"),
        }
    }

    #[test]
    fn test_for_in_range_parses() {
        let source = "int main() { for i in 0..5 { } return 0; }";
        let program = parse_source(source).unwrap();
        match &program.statements[0].kind {
            StmtKind::FuncDecl { body, .. } => match &body.kind {
                ExprKind::BlockExpr { statements, .. } => {
                    match &statements[0].kind {
                        StmtKind::ForIn { var, iterable, .. } => {
                            assert_eq!(var.name, "i");
                            assert!(matches!(
                                iterable.kind,
                                ExprKind::Range { .. }
                            ));
                        }
                        other => panic!("expected ForIn, got {:?}", other),
                    }
                }
                _ => panic!("expected block body for main"),
            },
            _ => panic!("expected FuncDecl"),
        }
    }

    #[test]
    fn test_array_index_parses() {
        let source = "int main() { arr[int] a := [10, 20, 30]; return a[1]; }";
        let program = parse_source(source).unwrap();
        match &program.statements[0].kind {
            StmtKind::FuncDecl { body, .. } => match &body.kind {
                ExprKind::BlockExpr { statements, .. } => {
                    match &statements[1].kind {
                        StmtKind::ReturnStmt(Some(expr)) => {
                            assert!(matches!(expr.kind, ExprKind::ArrayIndex { .. }));
                        }
                        other => panic!("expected ReturnStmt with array index, got {:?}", other),
                    }
                }
                _ => panic!("expected block body"),
            },
            _ => panic!("expected FuncDecl"),
        }
    }

    #[test]
    fn test_while_loop_parses() {
        let source = "int main() { while true { break; } return 0; }";
        let program = parse_source(source).unwrap();
        match &program.statements[0].kind {
            StmtKind::FuncDecl { body, .. } => match &body.kind {
                ExprKind::BlockExpr { statements, .. } => {
                    assert!(matches!(statements[0].kind, StmtKind::WhileLoop { .. }));
                    match &statements[0].kind {
                        StmtKind::WhileLoop { body, .. } => match &body.kind {
                            ExprKind::BlockExpr { statements, .. } => {
                                assert!(matches!(statements[0].kind, StmtKind::Break));
                            }
                            _ => panic!("expected block body"),
                        },
                        _ => unreachable!(),
                    }
                }
                _ => panic!("expected block body for main"),
            },
            _ => panic!("expected FuncDecl"),
        }
    }

    #[test]
    fn test_bitwise_ops_parse() {
        let sources = [
            "int main() { return 12 & 10; }",
            "int main() { return 12 | 10; }",
            "int main() { return 12 ^ 10; }",
            "int main() { return 12 << 1; }",
            "int main() { return 12 >> 1; }",
            "int main() { return ~5; }",
        ];
        for src in sources.iter() {
            parse_source(src).unwrap_or_else(|e| panic!("failed to parse {:?}: {:?}", src, e));
        }
    }

    #[test]
    fn test_reference_type_and_take_ref_parse() {
        let source = "int main() { mut int x := 5; &mut int p := &mut x; return *p; }";
        let program = parse_source(source).unwrap();
        match &program.statements[0].kind {
            StmtKind::FuncDecl { body, .. } => match &body.kind {
                ExprKind::BlockExpr { statements, .. } => {
                    // statements[1] is `&mut int p := &mut x;` — verify the type
                    // annotation and initializer shape.
                    match &statements[1].kind {
                        StmtKind::VarDecl { typ, initializer, .. } => {
                            assert!(matches!(
                                typ.as_ref().unwrap().kind,
                                TypeKind::Ref { mutable: true, .. }
                            ));
                            assert!(matches!(
                                initializer.kind,
                                ExprKind::TakeRef { mutable: true, .. }
                            ));
                        }
                        other => panic!("expected VarDecl for p, got {:?}", other),
                    }
                    // statements[2] is `return *p;` — verify Deref.
                    match &statements[2].kind {
                        StmtKind::ReturnStmt(Some(expr)) => {
                            assert!(matches!(expr.kind, ExprKind::Deref(_)));
                        }
                        other => panic!("expected ReturnStmt with Deref, got {:?}", other),
                    }
                }
                _ => panic!("expected block body"),
            },
            _ => panic!("expected FuncDecl"),
        }
    }

    #[test]
    fn test_take_ref_immutable_parse() {
        // &x without mut should produce TakeRef { mutable: false }.
        let source = "int main() { int x := 5; &int r := &x; return *r; }";
        let program = parse_source(source).unwrap();
        match &program.statements[0].kind {
            StmtKind::FuncDecl { body, .. } => match &body.kind {
                ExprKind::BlockExpr { statements, .. } => match &statements[1].kind {
                    StmtKind::VarDecl { typ, initializer, .. } => {
                        assert!(matches!(
                            typ.as_ref().unwrap().kind,
                            TypeKind::Ref { mutable: false, .. }
                        ));
                        assert!(matches!(
                            initializer.kind,
                            ExprKind::TakeRef { mutable: false, .. }
                        ));
                    }
                    other => panic!("expected VarDecl, got {:?}", other),
                },
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_bitwise_above_comparison_precedence() {
        // a & b == c should parse as (a & b) == c (Rust precedence), not a & (b == c) (C).
        let program = parse_source("int main() { return 12 & 10 == 8; }").unwrap();
        match &program.statements[0].kind {
            StmtKind::FuncDecl { body, .. } => match &body.kind {
                ExprKind::BlockExpr { statements, .. } => match &statements[0].kind {
                    StmtKind::ReturnStmt(Some(expr)) => match &expr.kind {
                        ExprKind::BinaryExpr { op: BinOp::Eq, left, .. } => {
                            assert!(matches!(left.kind, ExprKind::BinaryExpr { op: BinOp::BAnd, .. }));
                        }
                        other => panic!("expected Eq at top, got {:?}", other),
                    },
                    _ => panic!(),
                },
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn test_bare_take_ref_statement_parses_as_expression() {
        // `&x;` should parse as an expression statement, NOT trigger var-decl path.
        // The statement is useless on its own but must parse cleanly.
        let source = "int main() { int x := 5; &x; return 0; }";
        let program = parse_source(source).unwrap();
        match &program.statements[0].kind {
            StmtKind::FuncDecl { body, .. } => match &body.kind {
                ExprKind::BlockExpr { statements, .. } => {
                    match &statements[1].kind {
                        StmtKind::ExprStmt(expr) => {
                            assert!(matches!(expr.kind, ExprKind::TakeRef { mutable: false, .. }));
                        }
                        other => panic!("expected ExprStmt with TakeRef, got {:?}", other),
                    }
                }
                _ => panic!("expected block body"),
            },
            _ => panic!("expected FuncDecl"),
        }
    }

    #[test]
    fn test_ref_var_decl_still_dispatches_to_variable_declarations() {
        // `&mut int p := &mut x;` must still parse as a var-decl.
        let source = "int main() { mut int x := 5; &mut int p := &mut x; return *p; }";
        parse_source(source).expect("ref var-decl should parse");
    }
}
