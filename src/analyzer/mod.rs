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
        Analyzer {
            scopes: vec![Scope::default()],
            errors: Vec::new(),
        }
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
                body: _,
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
            _ => None,
        }
    }
}
