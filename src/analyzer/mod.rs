mod error;
mod symbol;

pub use error::{AnalyzeError, AnalyzeErrorKind};
use symbol::{Scope, Symbol};

use crate::parser::ast::{
    BinOp, Expr, ExprKind, Program, Stmt, StmtKind, TypeKind,
};

pub struct Analyzer {
    scopes: Vec<Scope>,
    errors: Vec<AnalyzeError>,
}

impl Analyzer {
    pub fn new() -> Self {
        let mut analyzer = Analyzer {
            scopes: vec![Scope::default()],
            errors: Vec::new(),
        };

        // Register built-in I/O functions
        analyzer.register_builtin_functions();

        analyzer
    }

    /// Register built-in I/O functions in the global scope
    fn register_builtin_functions(&mut self) {
        use crate::parser::ast::TypeKind;

        // print_int(int) -> void
        let _ = self.declare("print_int", Symbol::Function {
            name: "print_int".to_string(),
            parameters: vec![TypeKind::Int],
            return_type: None,
        });

        // print_bool(bool) -> void
        let _ = self.declare("print_bool", Symbol::Function {
            name: "print_bool".to_string(),
            parameters: vec![TypeKind::Bool],
            return_type: None,
        });

        // print_char(char) -> void
        let _ = self.declare("print_char", Symbol::Function {
            name: "print_char".to_string(),
            parameters: vec![TypeKind::Char],
            return_type: None,
        });

        // print_str(str) -> void
        let _ = self.declare("print_str", Symbol::Function {
            name: "print_str".to_string(),
            parameters: vec![TypeKind::Str],
            return_type: None,
        });

        // println() -> void
        let _ = self.declare("println", Symbol::Function {
            name: "println".to_string(),
            parameters: vec![],
            return_type: None,
        });

        // read_int() -> int
        let _ = self.declare("read_int", Symbol::Function {
            name: "read_int".to_string(),
            parameters: vec![],
            return_type: Some(TypeKind::Int),
        });
    }

    fn enter_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, symbol: Symbol) -> Result<(), String> {
        self.scopes.last_mut().unwrap().declare(name, symbol)
    }

    fn find_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.find(name) {
                return Some(symbol);
            }
        }
        None
    }

    pub fn analyze_program(&mut self, program: &Program) -> Result<(), Vec<AnalyzeError>> {
        for statement in &program.statements {
            self.analyze_statement(statement, None);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn analyze_statement(&mut self, statement: &Stmt, expected_return_type: Option<TypeKind>) {
        match &statement.kind {
            StmtKind::VarDecl {
                mutable,
                typ,
                ident,
                initializer,
            } => {
                let init_type = self.analyze_expression(initializer, None);
                let var_type = typ.as_ref().map(|t| t.kind.clone());

                if let Some(var_type) = var_type {
                    if let Some(init_type) = init_type.clone() {
                        if var_type != init_type {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: format!("{:?}", var_type),
                                    found: format!("{:?}", init_type),
                                },
                                statement.location.clone(),
                            ));
                        }
                    }
                }

                let symbol = Symbol::Variable {
                    name: ident.name.clone(),
                    type_kind: init_type.unwrap_or(TypeKind::Int), // Default to Int if not specified, should be improved
                    mutable: *mutable,
                };

                if let Err(_e) = self.declare(&ident.name, symbol) {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::VariableAlreadyDeclared(ident.name.clone()),
                        ident.location.clone(),
                    ));
                }
            }
            StmtKind::ExprStmt(expr) => {
                self.analyze_expression(expr, None);
            }
            StmtKind::Assignment { target, value } => {
                // Check if target is mutable
                if let ExprKind::IdentExpr(ident) = &target.kind {
                    if let Some(symbol) = self.find_symbol(&ident.name) {
                        match symbol {
                            Symbol::Variable { mutable, .. } => {
                                if !mutable {
                                    self.errors.push(AnalyzeError::new(
                                        AnalyzeErrorKind::AssignmentToImmutable(ident.name.clone()),
                                        target.location.clone(),
                                    ));
                                }
                            }
                            _ => {
                                // Not a variable, this will be caught elsewhere
                            }
                        }
                    }
                }

                let target_type = self.analyze_expression(target, None);
                let value_type = self.analyze_expression(value, None);

                if target_type != value_type {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::TypeMismatch {
                            expected: format!("{:?}", target_type),
                            found: format!("{:?}", value_type),
                        },
                        statement.location.clone(),
                    ));
                }
            }
            StmtKind::FuncDecl {
                name,
                parameters,
                return_type,
                body,
            } => {
                let param_types = parameters
                    .iter()
                    .map(|(_, t)| t.kind.clone())
                    .collect::<Vec<_>>();

                let symbol = Symbol::Function {
                    name: name.name.clone(),
                    parameters: param_types,
                    return_type: return_type.as_ref().map(|t| t.kind.clone()),
                };

                if let Err(_e) = self.declare(&name.name, symbol) {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::VariableAlreadyDeclared(name.name.clone()),
                        name.location.clone(),
                    ));
                }

                self.enter_scope();
                for (ident, typ) in parameters {
                    let param_symbol = Symbol::Variable {
                        name: ident.name.clone(),
                        type_kind: typ.kind.clone(),
                        mutable: false,
                    };
                    if let Err(_e) = self.declare(&ident.name, param_symbol) {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::VariableAlreadyDeclared(ident.name.clone()),
                            ident.location.clone(),
                        ));
                    }
                }

                self.analyze_expression(body, return_type.as_ref().map(|t| t.kind.clone()));

                self.exit_scope();
            }
            StmtKind::ReturnStmt(expr) => {
                let expr_type = expr.as_ref().and_then(|e| self.analyze_expression(e, expected_return_type.clone()));

                if expected_return_type != expr_type {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::TypeMismatch {
                            expected: format!("{:?}", expected_return_type),
                            found: format!("{:?}", expr_type),
                        },
                        statement.location.clone(),
                    ));
                }
            }
            _ => {}
        }
    }

    fn analyze_expression(&mut self, expression: &Expr, expected_return_type: Option<TypeKind>) -> Option<TypeKind> {
        match &expression.kind {
            ExprKind::IntLit(_) => Some(TypeKind::Int),
            ExprKind::FloatLit(_) => Some(TypeKind::Float),
            ExprKind::CharLit(_) => Some(TypeKind::Char),
            ExprKind::StringLit(_) => Some(TypeKind::Str),
            ExprKind::ArrayLit(elements) => {
                // Infer array type from first element
                if elements.is_empty() {
                    // Empty array - we can't infer the type
                    None
                } else {
                    let first_type = self.analyze_expression(&elements[0], None)?;

                    // Check that all elements have the same type
                    for elem in &elements[1..] {
                        let elem_type = self.analyze_expression(elem, None);
                        if elem_type != Some(first_type.clone()) {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: format!("{:?}", first_type),
                                    found: format!("{:?}", elem_type),
                                },
                                elem.location.clone(),
                            ));
                        }
                    }

                    Some(TypeKind::Array(Box::new(first_type)))
                }
            }
            ExprKind::BoolTrue | ExprKind::BoolFalse => Some(TypeKind::Bool),
            ExprKind::IdentExpr(ident) => {
                if let Some(symbol) = self.find_symbol(&ident.name) {
                    match symbol {
                        Symbol::Variable { type_kind, .. } => Some(type_kind.clone()),
                        _ => {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::UndeclaredVariable(ident.name.clone()),
                                ident.location.clone(),
                            ));
                            None
                        }
                    }
                } else {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::UndeclaredVariable(ident.name.clone()),
                        ident.location.clone(),
                    ));
                    None
                }
            }
            ExprKind::BinaryExpr { left, op, right } => {
                let left_type = self.analyze_expression(left, None);
                let right_type = self.analyze_expression(right, None);

                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                        if left_type != right_type {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: format!("{:?}", left_type),
                                    found: format!("{:?}", right_type),
                                },
                                expression.location.clone(),
                            ));
                        }
                        left_type
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        if left_type != right_type {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: format!("{:?}", left_type),
                                    found: format!("{:?}", right_type),
                                },
                                expression.location.clone(),
                            ));
                        }
                        Some(TypeKind::Bool)
                    }
                    BinOp::LAnd | BinOp::LOr => {
                        if left_type != Some(TypeKind::Bool) || right_type != Some(TypeKind::Bool) {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: "Bool".to_string(),
                                    found: format!("{:?}", left_type),
                                },
                                expression.location.clone(),
                            ));
                        }
                        Some(TypeKind::Bool)
                    }
                }
            }
            ExprKind::FunctionCall {
                function,
                arguments,
            } => {
                if let Some(Symbol::Function {
                    parameters,
                    return_type,
                    ..
                }) = self.find_symbol(&function.name).cloned()
                {
                    if arguments.len() != parameters.len() {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::IncorrectArgumentCount {
                                expected: parameters.len(),
                                found: arguments.len(),
                            },
                            expression.location.clone(),
                        ));
                    }

                    for (arg, param_type) in arguments.iter().zip(parameters.iter()) {
                        let arg_type = self.analyze_expression(arg, None);
                        if arg_type.as_ref() != Some(param_type) {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: format!("{:?}", param_type),
                                    found: format!("{:?}", arg_type),
                                },
                                arg.location.clone(),
                            ));
                        }
                    }
                    return_type
                } else {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::UndeclaredFunction(function.name.clone()),
                        function.location.clone(),
                    ));
                    None
                }
            }
            ExprKind::ParenExpr(expr) => self.analyze_expression(expr, expected_return_type),
            ExprKind::BlockExpr { statements, ret } => {
                self.enter_scope();
                for stmt in statements {
                    self.analyze_statement(stmt, expected_return_type.clone());
                }
                let ret_type = ret.as_ref().and_then(|e| self.analyze_expression(e, expected_return_type));
                self.exit_scope();
                ret_type
            }
            ExprKind::IfExpr {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_type = self.analyze_expression(condition, None);
                if cond_type != Some(TypeKind::Bool) {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::TypeMismatch {
                            expected: "Bool".to_string(),
                            found: format!("{:?}", cond_type),
                        },
                        condition.location.clone(),
                    ));
                }

                let then_type = then_branch
                    .as_ref()
                    .and_then(|b| self.analyze_expression(b, expected_return_type.clone()));

                if let Some(else_branch) = else_branch {
                    let else_type = self.analyze_expression(else_branch, expected_return_type);
                    if then_type != else_type {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::TypeMismatch {
                                expected: format!("{:?}", then_type),
                                found: format!("{:?}", else_type),
                            },
                            expression.location.clone(),
                        ));
                    }
                    then_type
                } else {
                    None
                }
            }
            ExprKind::UnaryExpr { op, expression: expr } => {
                use crate::parser::ast::UnaryOp;
                let expr_type = self.analyze_expression(expr, None);

                match op {
                    UnaryOp::LNot => {
                        // Logical NOT requires Bool operand
                        if expr_type != Some(TypeKind::Bool) {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: "Bool".to_string(),
                                    found: format!("{:?}", expr_type),
                                },
                                expr.location.clone(),
                            ));
                        }
                        Some(TypeKind::Bool)
                    }
                    UnaryOp::BNot => {
                        // Bitwise NOT requires Int operand
                        if expr_type != Some(TypeKind::Int) {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: "Int".to_string(),
                                    found: format!("{:?}", expr_type),
                                },
                                expr.location.clone(),
                            ));
                        }
                        Some(TypeKind::Int)
                    }
                    UnaryOp::Neg => {
                        // Negation works on Int or Float
                        match expr_type {
                            Some(TypeKind::Int) => Some(TypeKind::Int),
                            Some(TypeKind::Float) => Some(TypeKind::Float),
                            _ => {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: "Int or Float".to_string(),
                                        found: format!("{:?}", expr_type),
                                    },
                                    expr.location.clone(),
                                ));
                                None
                            }
                        }
                    }
                }
            }
            _ => None,
        }
    }
}
