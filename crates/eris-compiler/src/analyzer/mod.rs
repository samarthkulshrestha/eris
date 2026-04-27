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
    loop_depth: usize,
}

impl Analyzer {
    pub fn new() -> Self {
        let mut analyzer = Analyzer {
            scopes: vec![Scope::default()],
            errors: Vec::new(),
            loop_depth: 0,
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
                                    expected: var_type.clone(),
                                    found: init_type.clone(),
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

                if let (Some(a), Some(b)) = (&target_type, &value_type) {
                    if a != b {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::TypeMismatch {
                                expected: a.clone(),
                                found: b.clone(),
                            },
                            statement.location.clone(),
                        ));
                    }
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

                let saved_loop_depth = self.loop_depth;
                self.loop_depth = 0;
                self.analyze_expression(body, return_type.as_ref().map(|t| t.kind.clone()));
                self.loop_depth = saved_loop_depth;

                self.exit_scope();
            }
            StmtKind::ReturnStmt(expr) => {
                let expr_type = expr.as_ref().and_then(|e| self.analyze_expression(e, expected_return_type.clone()));

                if let (Some(expected), Some(found)) = (&expected_return_type, &expr_type) {
                    if expected != found {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::TypeMismatch {
                                expected: expected.clone(),
                                found: found.clone(),
                            },
                            statement.location.clone(),
                        ));
                    }
                }
            }
            StmtKind::WhileLoop { condition, body } => {
                let cond_type = self.analyze_expression(condition, None);
                if cond_type != Some(TypeKind::Bool) {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::TypeMismatch {
                            expected: TypeKind::Bool,
                            found: cond_type.unwrap_or(TypeKind::Int),
                        },
                        condition.location.clone(),
                    ));
                }
                self.loop_depth += 1;
                self.analyze_expression(body, expected_return_type);
                self.loop_depth -= 1;
            }
            StmtKind::Break => {
                if self.loop_depth == 0 {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::BreakOutsideLoop,
                        statement.location.clone(),
                    ));
                }
            }
            StmtKind::Continue => {
                if self.loop_depth == 0 {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::ContinueOutsideLoop,
                        statement.location.clone(),
                    ));
                }
            }
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
                        if let Some(found) = &elem_type {
                            if *found != first_type {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: first_type.clone(),
                                        found: found.clone(),
                                    },
                                    elem.location.clone(),
                                ));
                            }
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
                        if let (Some(a), Some(b)) = (&left_type, &right_type) {
                            if a != b {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: a.clone(),
                                        found: b.clone(),
                                    },
                                    expression.location.clone(),
                                ));
                            }
                        }
                        left_type
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        if let (Some(a), Some(b)) = (&left_type, &right_type) {
                            if a != b {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: a.clone(),
                                        found: b.clone(),
                                    },
                                    expression.location.clone(),
                                ));
                            }
                        }
                        Some(TypeKind::Bool)
                    }
                    BinOp::LAnd | BinOp::LOr => {
                        if let Some(a) = &left_type {
                            if *a != TypeKind::Bool {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: TypeKind::Bool,
                                        found: a.clone(),
                                    },
                                    expression.location.clone(),
                                ));
                            }
                        }
                        if let Some(b) = &right_type {
                            if *b != TypeKind::Bool {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: TypeKind::Bool,
                                        found: b.clone(),
                                    },
                                    expression.location.clone(),
                                ));
                            }
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
                        if let Some(found) = &arg_type {
                            if found != param_type {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: param_type.clone(),
                                        found: found.clone(),
                                    },
                                    arg.location.clone(),
                                ));
                            }
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
                if let Some(found) = &cond_type {
                    if *found != TypeKind::Bool {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::TypeMismatch {
                                expected: TypeKind::Bool,
                                found: found.clone(),
                            },
                            condition.location.clone(),
                        ));
                    }
                }

                let then_type = then_branch
                    .as_ref()
                    .and_then(|b| self.analyze_expression(b, expected_return_type.clone()));

                if let Some(else_branch) = else_branch {
                    let else_type = self.analyze_expression(else_branch, expected_return_type);
                    if let (Some(a), Some(b)) = (&then_type, &else_type) {
                        if a != b {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: a.clone(),
                                    found: b.clone(),
                                },
                                expression.location.clone(),
                            ));
                        }
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
                        if let Some(found) = &expr_type {
                            if *found != TypeKind::Bool {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: TypeKind::Bool,
                                        found: found.clone(),
                                    },
                                    expr.location.clone(),
                                ));
                            }
                        }
                        Some(TypeKind::Bool)
                    }
                    UnaryOp::BNot => {
                        // Bitwise NOT requires Int operand
                        if let Some(found) = &expr_type {
                            if *found != TypeKind::Int {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: TypeKind::Int,
                                        found: found.clone(),
                                    },
                                    expr.location.clone(),
                                ));
                            }
                        }
                        Some(TypeKind::Int)
                    }
                    UnaryOp::Neg => {
                        // Negation works on Int or Float
                        match expr_type {
                            Some(TypeKind::Int) => Some(TypeKind::Int),
                            Some(TypeKind::Float) => Some(TypeKind::Float),
                            Some(other) => {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: TypeKind::Float,
                                        found: other,
                                    },
                                    expr.location.clone(),
                                ));
                                None
                            }
                            None => None,
                        }
                    }
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn analyze_source(source: &str) -> Result<(), Vec<AnalyzeError>> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().unwrap();
        let mut analyzer = Analyzer::new();
        analyzer.analyze_program(&program)
    }

    #[test]
    fn test_simple_variable_declaration() {
        let source = "int x := 10;";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mutable_variable_declaration() {
        let source = "mut int y := 20;";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_mismatch_in_variable_declaration() {
        let source = "int x := 3.14;";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].kind, AnalyzeErrorKind::TypeMismatch { .. }));
    }

    #[test]
    fn test_undeclared_variable() {
        let source = "int x := y;";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::UndeclaredVariable(name) if name == "y")));
    }

    #[test]
    fn test_assignment_to_immutable() {
        let source = "int x := 10; x = 20;";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::AssignmentToImmutable(_))));
    }

    #[test]
    fn test_assignment_to_mutable() {
        let source = "mut int x := 10; x = 20;";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_declaration() {
        let source = "int add(int a, int b) { return a + b; }";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_call() {
        let source = r#"
            int add(int a, int b) { return a + b; }
            int x := add(1, 2);
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_undeclared_function() {
        let source = "int x := add(1, 2);";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::UndeclaredFunction(name) if name == "add")));
    }

    #[test]
    fn test_incorrect_argument_count() {
        let source = r#"
            int add(int a, int b) { return a + b; }
            int x := add(1, 2, 3);
        "#;
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::IncorrectArgumentCount { .. })));
    }

    #[test]
    fn test_incorrect_argument_type() {
        let source = r#"
            int add(int a, int b) { return a + b; }
            int x := add(1, 3.14);
        "#;
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::TypeMismatch { .. })));
    }

    #[test]
    fn test_binary_operation_type_checking() {
        let source = "int x := 5 + 10;";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_binary_operation_type_mismatch() {
        let source = "int x := 5 + 3.14;";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::TypeMismatch { .. })));
    }

    #[test]
    fn test_comparison_operators() {
        let source = "bool result := 5 > 3;";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_logical_operators() {
        let source = "bool x := true && false;";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_logical_operator_type_error() {
        let source = "bool x := 5 && 10;";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::TypeMismatch { .. })));
    }

    #[test]
    fn test_if_expression_type_checking() {
        let source = "int x := if true { 1 } else { 2 };";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_if_condition_must_be_bool() {
        let source = "int x := if 5 { 1 } else { 2 };";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::TypeMismatch { .. })));
    }

    #[test]
    fn test_if_branch_type_mismatch() {
        let source = "int x := if true { 1 } else { 3.14 };";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::TypeMismatch { .. })));
    }

    #[test]
    fn test_block_expression_scoping() {
        let source = r#"
            int x := 10;
            int y := {
                int z := 5;
                z + 1
            };
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_variable_shadowing() {
        let source = r#"
            int x := 10;
            int y := {
                int x := 20;
                x + 1
            };
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_scope() {
        let source = r#"
            int foo(int a) {
                int b := a + 1;
                return b;
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_variable_declaration() {
        let source = "int x := 10; int x := 20;";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::VariableAlreadyDeclared(_))));
    }

    #[test]
    fn test_unary_negation() {
        let source = "int x := -5;";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unary_logical_not() {
        let source = "bool x := !true;";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unary_logical_not_type_error() {
        let source = "bool x := !5;";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::TypeMismatch { .. })));
    }

    #[test]
    fn test_array_literal_homogeneous() {
        let source = "arr[int] nums := [1, 2, 3];";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_array_literal_heterogeneous() {
        let source = "arr[int] nums := [1, 2, 3.14];";
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::TypeMismatch { .. })));
    }

    #[test]
    fn test_return_type_checking() {
        let source = r#"
            int get_number() {
                return 42;
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_return_type_mismatch() {
        let source = r#"
            int get_number() {
                return 3.14;
            }
        "#;
        let result = analyze_source(source);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(&e.kind, AnalyzeErrorKind::TypeMismatch { .. })));
    }

    #[test]
    fn test_tuple_destructuring() {
        let source = "int (a, b) := (1, 2);";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_variable_bindings() {
        // The syntax is: int x, y := 1, 2; (same type, multiple variables)
        let source = "int x, y := 1, 2;";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_expression() {
        let source = "int x := (1 + 2) * (3 - 4);";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_function_calls() {
        let source = r#"
            int add(int a, int b) { return a + b; }
            int mul(int a, int b) { return a * b; }
            int result := add(mul(2, 3), mul(4, 5));
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }
}
