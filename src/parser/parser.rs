use crate::lexer::{LexerError, SourceLocation, Token};
use std::iter::Peekable;
use std::vec::IntoIter;

use crate::parser::ast::{
    BinOp, Expr, ExprKind, Identifier, Precedence, Program, Stmt, StmtKind, Type, TypeKind,
};

#[derive(Debug)]
pub enum ParseErrorKind {
    UnexpectedToken {
        expected: String,
        found: Option<Token>,
    },
    InvalidSyntax(String),
    LexerError(LexerError),
}

#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub location: Option<SourceLocation>,
}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        ParseError {
            kind: ParseErrorKind::LexerError(err),
            location: None,
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
}

#[derive(Debug)]
pub struct Parser {
    tokens: Peekable<IntoIter<(Token, SourceLocation)>>,
    current_token: Option<(Token, SourceLocation)>,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, SourceLocation)>) -> Self {
        let mut parser = Parser {
            tokens: tokens.into_iter().peekable(),
            current_token: None,
        };

        // Advance to the first token
        parser.advance();

        parser
    }

    fn advance(&mut self) -> Option<(Token, SourceLocation)> {
        self.current_token = self.tokens.next();
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
            location: None,
        })
    }

    fn current_is(&self, tokens: &[Token]) -> bool {
        if let Some((token, _)) = &self.current_token {
            tokens.iter().any(|t| token_matches(t, token))
        } else {
            false
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
                let current_token_clone = self.current_token.clone();
                let tokens_clone = self.tokens.clone();

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
                        self.current_token = current_token_clone;
                        self.tokens = tokens_clone;
                        // Parse as a function
                        self.parse_function_declaration()?.kind
                    } else {
                        // It's a variable declaration or part of a structured binding
                        // Reset the parser state
                        self.current_token = current_token_clone;
                        self.tokens = tokens_clone;
                        // Parse as a variable - this returns Vec<Stmt>, so return early
                        return self.parse_variable_declarations();
                    }
                } else if has_tuple_pattern {
                    // It's a tuple pattern: type (id1, id2) := ...
                    // Reset the parser state
                    self.current_token = current_token_clone;
                    self.tokens = tokens_clone;
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

            Some((Token::Ident(_name), _)) => {
                // Parse as expression (could be function call, identifier, etc.)
                let expr = self.parse_expression()?;

                // Check if this is an assignment
                if self.current_is(&[Token::Assign]) {
                    self.advance(); // consume '='
                    let value = self.parse_expression()?;
                    self.expect(&[Token::Semicolon])?;
                    StmtKind::Assignment {
                        target: expr,
                        value,
                    }
                } else {
                    // Expression statement (including function calls)
                    self.expect(&[Token::Semicolon])?;
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
                let current_token_clone = self.current_token.clone();
                let tokens_clone = self.tokens.clone();

                self.advance(); // consume '('

                // Try to parse as tuple pattern
                let is_tuple_pattern = self.current_is(&[Token::Int, Token::Bool, Token::Float, Token::Char, Token::Str]);

                // Restore state
                self.current_token = current_token_clone;
                self.tokens = tokens_clone;

                if is_tuple_pattern {
                    // Parse as tuple destructuring
                    return self.parse_tuple_destructuring();
                } else {
                    // It's an expression statement (like a tuple expression or function call)
                    let expr = self.parse_expression()?;
                    self.expect(&[Token::Semicolon])?;
                    StmtKind::ExprStmt(expr)
                }
            }

            Some((_, _loc)) => {
                // Try to parse it as an expression statement
                let expr = self.parse_expression()?;
                self.expect(&[Token::Semicolon])?;
                StmtKind::ExprStmt(expr)
            }

            None => {
                return Err(ParseError {
                    kind: ParseErrorKind::InvalidSyntax("Unexpected end of file".to_string()),
                    location: None,
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

        self.expect(&[Token::LParen])?; // consume '('

        // Parse (type id, type id, ...)
        let mut bindings: Vec<(Type, Identifier)> = Vec::new();
        loop {
            let typ = self.parse_type()?;
            let ident = self.parse_ident()?;
            bindings.push((typ, ident));

            if !self.current_is(&[Token::Comma]) {
                break;
            }
            self.advance(); // consume comma
        }

        self.expect(&[Token::RParen])?;
        self.expect(&[Token::DeclareAssign])?;

        // Parse the tuple expression on the right
        let tuple_expr = self.parse_expression()?;

        self.expect(&[Token::Semicolon])?;

        // Validate that the right side is a tuple expression
        if let ExprKind::TupleExpr(ref elements) = tuple_expr.kind {
            if elements.len() != bindings.len() {
                return Err(ParseError {
                    kind: ParseErrorKind::InvalidSyntax(format!(
                        "Tuple destructuring mismatch: {} bindings but {} values",
                        bindings.len(),
                        elements.len()
                    )),
                    location: Some(location),
                });
            }

            // Create individual variable declarations for each tuple element
            let statements = bindings
                .into_iter()
                .zip(elements.iter())
                .map(|((typ, ident), initializer)| Stmt {
                    kind: StmtKind::VarDecl {
                        ident,
                        mutable: false, // Tuple destructuring is immutable for now
                        typ: Some(typ),
                        initializer: Box::new(initializer.clone()),
                    },
                    location: location.clone(),
                })
                .collect();

            Ok(statements)
        } else {
            Err(ParseError {
                kind: ParseErrorKind::InvalidSyntax(
                    "Expected tuple expression on right side of tuple destructuring".to_string(),
                ),
                location: Some(tuple_expr.location),
            })
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
        let typ = if self.current_is(&[Token::Int, Token::Bool, Token::Float, Token::Char, Token::Str, Token::Arr]) {
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

        // Expect semicolon
        self.expect(&[Token::Semicolon])?;

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
        let return_type = if self.current_is(&[Token::Int, Token::Bool, Token::Float, Token::Char, Token::Str, Token::Arr]) {
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
                // Parse parameter type
                let param_type = self.parse_type()?;

                // Parse parameter name
                if let Some((Token::Ident(id), loc)) = &self.current_token {
                    let param_name = Identifier {
                        name: id.clone(),
                        location: loc.clone(),
                    };
                    self.advance();
                    parameters.push((param_name, param_type));
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
                TypeKind::Array(Box::new(element_type.kind))
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
        let mut left = self.parse_primary_expression()?;

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

    fn current_binary_operator_precedence(&self) -> Option<Precedence> {
        match &self.current_token {
            Some((Token::LParen, _)) => Some(Precedence::Group),
            Some((Token::Equal, _)) | Some((Token::NotEqual, _)) => Some(Precedence::Equality),
            Some((Token::GreaterThan, _))
            | Some((Token::GreaterEqual, _))
            | Some((Token::LessThan, _))
            | Some((Token::LessEqual, _)) => Some(Precedence::Comparison),
            Some((Token::Plus, _)) | Some((Token::Minus, _)) => Some(Precedence::Sum),
            Some((Token::Multiply, _)) | Some((Token::Divide, _)) | Some((Token::Modulo, _)) => {
                Some(Precedence::Product)
            }
            Some((Token::And, _)) | Some((Token::Or, _)) => Some(Precedence::Lowest),
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
                let expression = Box::new(self.parse_primary_expression()?);
                ExprKind::UnaryExpr {
                    op: crate::parser::ast::UnaryOp::LNot,
                    expression,
                }
            }
            Some((Token::Minus, _)) => {
                self.advance(); // consume '-'
                let expression = Box::new(self.parse_primary_expression()?);
                ExprKind::UnaryExpr {
                    op: crate::parser::ast::UnaryOp::Neg,
                    expression,
                }
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
        let mut balance = 0;
        for (token, _) in self.tokens.clone() {
            match token {
                Token::LBrace | Token::LParen => balance += 1,
                Token::RBrace | Token::RParen => {
                    if balance == 0 {
                        // This means we have an unmatched closing brace/paren, which is a syntax error
                        // that will be caught later. For now, we can assume it's not a semicolon.
                        return false;
                    }
                    balance -= 1;
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
