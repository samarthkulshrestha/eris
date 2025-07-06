use crate::lexer::{Lexer, LexerError, SourceLocation, Token};
use std::iter::Peekable;
use std::vec::IntoIter;

use crate::parser::ast::{
    BinOp, Expr, ExprKind, Identifier, Precedence, Program, Stmt, StmtKind, Type, TypeKind,
};

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: Option<Token>,
        location: Option<SourceLocation>,
    },
    InvalidSyntax {
        message: String,
        location: Option<SourceLocation>,
    },
    LexerError(LexerError),
}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        ParseError::LexerError(err)
    }
}

#[derive(Debug)]
pub struct Parser {
    tokens: Peekable<IntoIter<(Token, SourceLocation)>>,
    current_token: Option<(Token, SourceLocation)>,
}

impl Parser {
    pub fn new(source: &str) -> Result<Self, LexerError> {
        let mut lexer = Lexer::new(source);
        let token_results = lexer.tokenize();

        // Filter out any lexer errors
        let mut tokens = Vec::new();
        for result in token_results {
            match result {
                Ok(token_loc) => tokens.push(token_loc),
                Err(err) => return Err(err),
            }
        }

        let mut parser = Parser {
            tokens: tokens.into_iter().peekable(),
            current_token: None,
        };

        // Advance to the first token
        parser.advance();

        Ok(parser)
    }

    fn advance(&mut self) -> Option<(Token, SourceLocation)> {
        self.current_token = self.tokens.next();
        self.current_token.clone()
    }

    fn peek(&mut self) -> Option<&(Token, SourceLocation)> {
        self.tokens.peek()
    }

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

            return Err(ParseError::UnexpectedToken {
                expected: expected_str,
                found: Some(token.clone()),
                location: Some(location.clone()),
            });
        }

        Err(ParseError::UnexpectedToken {
            expected: expected
                .iter()
                .map(|t| format!("{:?}", t))
                .collect::<Vec<_>>()
                .join(", "),
            found: None,
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

    fn _peek_is(&mut self, tokens: &[Token]) -> bool {
        if let Some((token, _)) = self.peek() {
            tokens.iter().any(|t| token_matches(t, token))
        } else {
            false
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();

        while self.current_token.is_some() {
            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    pub fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        let location = self
            .current_token
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        let kind = match &self.current_token {
            Some((Token::Int, _))
            | Some((Token::Char, _))
            | Some((Token::Float, _))
            | Some((Token::Bool, _)) => {
                // This could be a variable declaration or a function declaration
                // We need to look ahead to determine which one it is

                // Save current state to restore if needed
                let current_token_clone = self.current_token.clone();
                let tokens_clone = self.tokens.clone();

                // Parse the type
                let _type_annotation = self.parse_type()?;

                // Check if we have an identifier
                if !self.current_is(&[Token::Ident(String::new())]) {
                    return Err(ParseError::UnexpectedToken {
                        expected: "identifier".to_string(),
                        found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                        location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                    });
                }

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
                    // Parse as a variable
                    self.parse_variable_declaration()?.kind
                }
            }

            Some((Token::Mut, _)) => self.parse_variable_declaration()?.kind,

            Some((Token::Ident(_name), _)) => {
                // Check if we're looking at a function declaration
                let peek_token = self.peek();
                if let Some((Token::LParen, _)) = peek_token {
                    self.parse_function_declaration()?.kind
                } else {
                    // It's either an assignment or an expression statement
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
                        self.expect(&[Token::Semicolon])?;
                        StmtKind::ExprStmt(expr)
                    }
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

            Some((_, _loc)) => {
                // Try to parse it as an expression statement
                let expr = self.parse_expression()?;
                self.expect(&[Token::Semicolon])?;
                StmtKind::ExprStmt(expr)
            }

            None => {
                return Err(ParseError::UnexpectedToken {
                    expected: "statement".to_string(),
                    found: None,
                    location: None,
                })
            }
        };
        Ok(Stmt { kind, location })
    }

    fn parse_variable_declaration(&mut self) -> Result<Stmt, ParseError> {
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
        let typ = if self.current_is(&[Token::Int, Token::Bool, Token::Float, Token::Char]) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse identifier
        let ident = self.parse_ident()?;

        // Expect declare-assign operator :=
        self.expect(&[Token::DeclareAssign])?;

        // Parse initializer expression
        let initializer = Box::new(self.parse_expression()?);

        // Expect semicolon
        self.expect(&[Token::Semicolon])?;

        Ok(Stmt {
            kind: StmtKind::VarDecl {
                ident,
                mutable,
                typ,
                initializer,
            },
            location,
        })
    }

    fn parse_function_declaration(&mut self) -> Result<Stmt, ParseError> {
        let location = self
            .current_token
            .as_ref()
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        // Parse return type (if any)
        let return_type = if self.current_is(&[Token::Int, Token::Str, Token::Arr]) {
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
            return Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                found: self.current_token.as_ref().map(|(t, _)| t.clone()),
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
                    return Err(ParseError::UnexpectedToken {
                        expected: "identifier".to_string(),
                        found: self.current_token.as_ref().map(|(t, _)| t.clone()),
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
            // Some((Token::Arr, _)) => {
            //     self.advance();
            //     self.expect(&[Token::LBracket])?;
            //     let element_type = self.parse_type()?;
            //     self.expect(&[Token::RBracket])?;
            //     Ok(Type::Array(Box::new(element_type)))
            // },
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "type".to_string(),
                    found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                    location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                })
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
            _ => Err(ParseError::UnexpectedToken {
                expected: "pattern".to_string(),
                found: self.current_token.as_ref().map(|(t, _)| t.clone()),
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
                return Err(ParseError::UnexpectedToken {
                    expected: "binary operator".to_string(),
                    found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                    location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                })
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
                return Err(ParseError::UnexpectedToken {
                    expected: "expr".to_string(),
                    found: self.current_token.as_ref().map(|(t, _)| t.clone()),
                    location: self.current_token.as_ref().map(|(_, l)| l.clone()),
                })
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
            // Check if we're at the final expression (no semicolon)
            if self.is_expression_start() {
                let peek_pos = self.look_ahead_until(&[Token::Semicolon, Token::RBrace]);

                if let Some((Token::RBrace, _)) = peek_pos {
                    // This is the final expression without semicolon
                    final_expr = Some(Box::new(self.parse_expression()?));
                    break;
                }
            }

            // Regular statement
            statements.push(self.parse_statement()?);
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

    fn look_ahead_until(&self, tokens: &[Token]) -> Option<(Token, SourceLocation)> {
        let mut iter = self.tokens.clone().into_iter();

        while let Some((token, location)) = iter.next() {
            if tokens.iter().any(|t| token_matches(t, &token)) {
                return Some((token, location));
            }
        }

        None
    }
}

// Helper function to check if two tokens match, handling special cases like Ident
fn token_matches(expected: &Token, actual: &Token) -> bool {
    match (expected, actual) {
        (Token::Ident(_), Token::Ident(_)) => true,
        (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}
