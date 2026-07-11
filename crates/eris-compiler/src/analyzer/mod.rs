mod error;
mod symbol;
pub(crate) mod builtins;

pub use error::{AnalyzeError, AnalyzeErrorKind};
use symbol::{Scope, Symbol};

use crate::parser::ast::{
    BinOp, Expr, ExprKind, Program, Stmt, StmtKind, TypeKind,
};

fn bin_op_symbol(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
        BinOp::BAnd => "&",
        BinOp::BOr => "|",
        BinOp::BXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::LAnd => "&&",
        BinOp::LOr => "||",
    }
}

pub struct Analyzer {
    scopes: Vec<Scope>,
    errors: Vec<AnalyzeError>,
    loop_depth: usize,
    current_function_name: Option<String>,
    current_function_parameters: Option<Vec<String>>,
}

impl Analyzer {
    pub fn new() -> Self {
        let mut analyzer = Analyzer {
            scopes: vec![Scope::default()],
            errors: Vec::new(),
            loop_depth: 0,
            current_function_name: None,
            current_function_parameters: None,
        };

        // Register built-in I/O functions
        analyzer.register_builtin_functions();

        analyzer
    }

    /// Register built-in I/O functions in the global scope
    fn register_builtin_functions(&mut self) {
        use crate::analyzer::builtins::BUILTIN_FUNCTIONS;
        for builtin in BUILTIN_FUNCTIONS {
            let _ = self.declare(builtin.name, Symbol::Function {
                name: builtin.name.to_string(),
                parameters: builtin.parameters.to_vec(),
                return_type: builtin.return_type.clone(),
            });
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

    fn is_lvalue(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::IdentExpr(_)
            | ExprKind::ArrayIndex { .. }
            | ExprKind::Deref(_) => true,
            ExprKind::ParenExpr(inner) => Self::is_lvalue(inner),
            _ => false,
        }
    }

    fn check_range_endpoint(&mut self, expr: &Expr) {
        if let Some(found) = self.analyze_expression(expr, None) {
            if found != TypeKind::Int {
                self.errors.push(AnalyzeError::new(
                    AnalyzeErrorKind::TypeMismatch {
                        expected: TypeKind::Int,
                        found,
                    },
                    expr.location.clone(),
                ));
            }
        }
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
                        if !types_compatible(&var_type, &init_type) {
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
                // Check mutability based on the immediate shape of the target.
                match &target.kind {
                    ExprKind::IdentExpr(ident) => {
                        // Direct variable assignment requires the variable to be mutable.
                        if let Some(Symbol::Variable { mutable: false, .. }) = self.find_symbol(&ident.name) {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::AssignmentToImmutable(ident.name.clone()),
                                target.location.clone(),
                            ));
                        }
                    }
                    ExprKind::ArrayIndex { .. } => {
                        // Element assignment requires the root array variable to be mutable.
                        let mut root = target;
                        while let ExprKind::ArrayIndex { array, .. } = &root.kind {
                            root = array;
                        }
                        if let ExprKind::IdentExpr(ident) = &root.kind {
                            if let Some(Symbol::Variable { mutable: false, .. }) = self.find_symbol(&ident.name) {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::AssignmentToImmutable(ident.name.clone()),
                                    target.location.clone(),
                                ));
                            }
                        }
                    }
                    ExprKind::Deref(inner) => {
                        // Writing through a reference requires `&mut T`.
                        let inner_ty = self.analyze_expression(inner, None);
                        if let Some(TypeKind::Ref { mutable: false, .. }) = &inner_ty {
                            let name = if let ExprKind::IdentExpr(i) = &inner.kind {
                                i.name.clone()
                            } else {
                                "<deref target>".to_string()
                            };
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::AssignmentToImmutable(name),
                                target.location.clone(),
                            ));
                        }
                    }
                    _ => {}
                }

                let target_type = self.analyze_expression(target, None);
                let value_type = self.analyze_expression(value, None);

                if let (Some(a), Some(b)) = (&target_type, &value_type) {
                    if !types_compatible(a, b) {
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
                    .map(|p| p.typ.kind.clone())
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

                let saved_params = self.current_function_parameters.replace(
                    parameters.iter().map(|p| p.ident.name.clone()).collect()
                );

                self.enter_scope();
                for param in parameters {
                    let param_symbol = Symbol::Variable {
                        name: param.ident.name.clone(),
                        type_kind: param.typ.kind.clone(),
                        mutable: param.mutable,
                    };
                    if let Err(_e) = self.declare(&param.ident.name, param_symbol) {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::VariableAlreadyDeclared(param.ident.name.clone()),
                            param.ident.location.clone(),
                        ));
                    }
                }

                let saved_loop_depth = self.loop_depth;
                self.loop_depth = 0;
                let saved_fn_name = self.current_function_name.clone();
                self.current_function_name = Some(name.name.clone());
                self.analyze_expression(body, return_type.as_ref().map(|t| t.kind.clone()));
                self.current_function_name = saved_fn_name;
                self.loop_depth = saved_loop_depth;

                if return_type.is_some() && !expr_always_returns(body) {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::MissingReturn(name.name.clone()),
                        name.location.clone(),
                    ));
                }

                self.exit_scope();
                self.current_function_parameters = saved_params;
            }
            StmtKind::ReturnStmt(expr) => {
                let expr_type = expr.as_ref().and_then(|e| self.analyze_expression(e, expected_return_type.clone()));

                if expr.is_none() && expected_return_type.is_some() {
                    if let Some(fn_name) = self.current_function_name.as_ref() {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::ReturnMissingValue { function: fn_name.clone() },
                            statement.location.clone(),
                        ));
                    }
                }

                if let (Some(expected), Some(found)) = (&expected_return_type, &expr_type) {
                    if !types_compatible(expected, found) {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::TypeMismatch {
                                expected: expected.clone(),
                                found: found.clone(),
                            },
                            statement.location.clone(),
                        ));
                    }
                }

                // Reject returning a reference to a local variable; a reference
                // to a parameter is safe because the caller owns that storage.
                if let Some(TypeKind::Ref { .. }) = &expected_return_type {
                    if let Some(ret_expr) = expr {
                        if let ExprKind::TakeRef { place, .. } = &ret_expr.kind {
                            if let ExprKind::IdentExpr(ident) = &place.kind {
                                let is_param = self.current_function_parameters
                                    .as_ref()
                                    .map(|params| params.iter().any(|p| p == &ident.name))
                                    .unwrap_or(false);
                                if !is_param {
                                    self.errors.push(AnalyzeError::new(
                                        AnalyzeErrorKind::ReturnsLocalReference,
                                        ret_expr.location.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            StmtKind::WhileLoop { condition, body } => {
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
            StmtKind::ForIn { var, iterable, body } => {
                match &iterable.kind {
                    ExprKind::Range { start, end } => {
                        self.check_range_endpoint(start);
                        self.check_range_endpoint(end);
                    }
                    _ => {
                        if let Some(found) = self.analyze_expression(iterable, None) {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::NonIterableInForLoop { found },
                                iterable.location.clone(),
                            ));
                        }
                    }
                }

                self.enter_scope();
                let _ = self.declare(
                    &var.name,
                    Symbol::Variable {
                        name: var.name.clone(),
                        type_kind: TypeKind::Int,
                        mutable: false,
                    },
                );
                self.loop_depth += 1;
                self.analyze_expression(body, expected_return_type);
                self.loop_depth -= 1;
                self.exit_scope();
            }
            StmtKind::DiscordBlock { body } => {
                // No new scope: discord shares the enclosing scope so its
                // declarations are usable after the block.
                if let ExprKind::BlockExpr { statements, ret } = &body.kind {
                    for s in statements {
                        self.analyze_statement(s, expected_return_type.clone());
                    }
                    if let Some(r) = ret {
                        self.analyze_expression(r, None);
                    }
                }
            }
            StmtKind::Fnord => {}
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

                    Some(TypeKind::Array { element: Box::new(first_type), size: elements.len() })
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
                            } else if !matches!(a, TypeKind::Int | TypeKind::Float) {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::UnsupportedOperandType {
                                        op: bin_op_symbol(op),
                                        operand_type: a.clone(),
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
                            } else {
                                let allowed = match op {
                                    BinOp::Eq | BinOp::Ne => matches!(
                                        a,
                                        TypeKind::Int | TypeKind::Float | TypeKind::Char | TypeKind::Bool
                                    ),
                                    _ => matches!(a, TypeKind::Int | TypeKind::Float | TypeKind::Char),
                                };
                                if !allowed {
                                    self.errors.push(AnalyzeError::new(
                                        AnalyzeErrorKind::UnsupportedOperandType {
                                            op: bin_op_symbol(op),
                                            operand_type: a.clone(),
                                        },
                                        expression.location.clone(),
                                    ));
                                }
                            }
                        }
                        Some(TypeKind::Bool)
                    }
                    BinOp::BAnd | BinOp::BOr | BinOp::BXor | BinOp::Shl | BinOp::Shr => {
                        if let (Some(a), Some(b)) = (&left_type, &right_type) {
                            if *a != TypeKind::Int {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: TypeKind::Int,
                                        found: a.clone(),
                                    },
                                    left.location.clone(),
                                ));
                            }
                            if *b != TypeKind::Int {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::TypeMismatch {
                                        expected: TypeKind::Int,
                                        found: b.clone(),
                                    },
                                    right.location.clone(),
                                ));
                            }
                        }
                        Some(TypeKind::Int)
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
                // Polymorphic builtin: len(arr) -> Int, for any array type.
                if function.name == "len" {
                    if arguments.len() != 1 {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::IncorrectArgumentCount {
                                expected: 1,
                                found: arguments.len(),
                            },
                            expression.location.clone(),
                        ));
                        return Some(TypeKind::Int);
                    }
                    let arg_ty = self.analyze_expression(&arguments[0], None);
                    match arg_ty {
                        Some(TypeKind::Array { .. }) => {}
                        Some(found) => {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: TypeKind::Array {
                                        element: Box::new(TypeKind::Int),
                                        size: 0,
                                    },
                                    found,
                                },
                                arguments[0].location.clone(),
                            ));
                        }
                        None => {}
                    }
                    return Some(TypeKind::Int);
                }

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
            ExprKind::Range { start, end } => {
                self.analyze_expression(start, None);
                self.analyze_expression(end, None);
                None
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
            ExprKind::TakeRef { mutable, place } => {
                if !Self::is_lvalue(place) {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::TakeRefOfNonLvalue,
                        place.location.clone(),
                    ));
                }

                if *mutable {
                    // Walk to the root identifier through array indexing; check that
                    // the variable is declared mutable before allowing a mutable ref.
                    let mut root = place.as_ref();
                    while let ExprKind::ArrayIndex { array, .. } = &root.kind {
                        root = array;
                    }
                    if let ExprKind::IdentExpr(ident) = &root.kind {
                        if let Some(symbol) = self.find_symbol(&ident.name) {
                            if let Symbol::Variable { mutable: var_mut, .. } = symbol {
                                if !var_mut {
                                    self.errors.push(AnalyzeError::new(
                                        AnalyzeErrorKind::TakeMutRefOfImmutable {
                                            name: ident.name.clone(),
                                        },
                                        place.location.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }

                let inner_ty = self.analyze_expression(place, None)?;
                // Restrict references to `int` pointees until codegen tracks pointee types
                // through the variables map. Non-int references would silently miscompile
                // against the hardcoded i64 load type in `Deref` lowering.
                if inner_ty != TypeKind::Int {
                    self.errors.push(AnalyzeError::new(
                        AnalyzeErrorKind::TypeMismatch {
                            expected: TypeKind::Int,
                            found: inner_ty.clone(),
                        },
                        place.location.clone(),
                    ));
                }
                Some(TypeKind::Ref {
                    inner: Box::new(inner_ty),
                    mutable: *mutable,
                })
            }
            ExprKind::Deref(inner) => {
                let inner_ty = self.analyze_expression(inner, None);
                match inner_ty {
                    Some(TypeKind::Ref { inner: pointee, .. }) => {
                        // Mirror the TakeRef gate: only int pointees are safe to lower
                        // until codegen tracks pointee types end-to-end.
                        if *pointee != TypeKind::Int {
                            self.errors.push(AnalyzeError::new(
                                AnalyzeErrorKind::TypeMismatch {
                                    expected: TypeKind::Int,
                                    found: (*pointee).clone(),
                                },
                                inner.location.clone(),
                            ));
                        }
                        Some(*pointee)
                    }
                    Some(found) => {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::DerefOfNonReference {
                                found: found.clone(),
                            },
                            inner.location.clone(),
                        ));
                        None
                    }
                    None => None,
                }
            }
            ExprKind::ArrayIndex { array, index } => {
                let array_ty = self.analyze_expression(array, None);
                let index_ty = self.analyze_expression(index, None);

                if let Some(found) = &index_ty {
                    if *found != TypeKind::Int {
                        self.errors.push(AnalyzeError::new(
                            AnalyzeErrorKind::IndexNotInteger {
                                found: found.clone(),
                            },
                            index.location.clone(),
                        ));
                    }
                }

                if let Some(TypeKind::Array { size, .. }) = &array_ty {
                    if *size > 0 {
                        use crate::parser::ast::UnaryOp;
                        let literal_index = match &index.kind {
                            ExprKind::IntLit(n) => Some(*n),
                            ExprKind::UnaryExpr {
                                op: UnaryOp::Neg,
                                expression,
                            } => match &expression.kind {
                                ExprKind::IntLit(n) => Some(-(*n)),
                                _ => None,
                            },
                            _ => None,
                        };
                        if let Some(n) = literal_index {
                            if n < 0 || (n as usize) >= *size {
                                self.errors.push(AnalyzeError::new(
                                    AnalyzeErrorKind::IndexOutOfRange {
                                        array_size: *size,
                                        index: n,
                                    },
                                    index.location.clone(),
                                ));
                            }
                        }
                    }
                }

                match array_ty {
                    Some(TypeKind::Array { element, .. }) => Some(*element),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// Compare two types for assignment/declaration compatibility. Array types are
/// compatible when their element types match regardless of size, because the
/// declared annotation carries size 0 until the initializer is analyzed.
fn types_compatible(a: &TypeKind, b: &TypeKind) -> bool {
    match (a, b) {
        (TypeKind::Array { element: ea, .. }, TypeKind::Array { element: eb, .. }) => {
            types_compatible(ea, eb)
        }
        _ => a == b,
    }
}

fn expr_always_returns(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::BlockExpr { statements, ret } => {
            if let Some(trailing) = ret {
                // A trailing expression is the function's return value, but only
                // if it is itself guaranteed to produce a value on all paths.
                // Simple literals and identifiers always produce a value; a
                // one-armed IfExpr does not.
                expr_trailing_is_complete(trailing)
            } else {
                statements.last().map_or(false, stmt_always_returns)
            }
        }
        ExprKind::IfExpr { then_branch, else_branch, .. } => {
            then_branch.as_ref().map_or(false, |b| expr_always_returns(b))
                && else_branch.as_ref().map_or(false, |b| expr_always_returns(b))
        }
        _ => false,
    }
}

/// Returns true when `expr` is certain to produce a value on every control-flow
/// path (i.e. it is safe to use as a trailing return expression).
fn expr_trailing_is_complete(expr: &Expr) -> bool {
    match &expr.kind {
        // A nested block is complete if it always returns from inside, or its
        // own trailing expression is complete.
        ExprKind::BlockExpr { .. } => expr_always_returns(expr),
        // An if expression is only complete when both branches are present and
        // each is itself complete (either returns or produces a value).
        ExprKind::IfExpr { then_branch, else_branch, .. } => {
            let then_ok = then_branch
                .as_ref()
                .map_or(false, |b| expr_always_returns(b) || expr_trailing_is_complete(b));
            let else_ok = else_branch
                .as_ref()
                .map_or(false, |b| expr_always_returns(b) || expr_trailing_is_complete(b));
            then_ok && else_ok
        }
        // Everything else (literals, identifiers, binary/unary exprs, calls,
        // parens, tuples) always produces a value.
        _ => true,
    }
}

fn stmt_always_returns(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::ReturnStmt(_) => true,
        StmtKind::ExprStmt(expr) => expr_always_returns(expr),
        StmtKind::Fnord => true,
        _ => false,
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
    fn take_ref_of_literal_is_rejected() {
        let source = r#"
            int main() {
                &mut int p := &mut 42;
                return 0;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::TakeRefOfNonLvalue
        )));
    }

    #[test]
    fn take_mut_ref_of_immutable_is_rejected() {
        let source = r#"
            int main() {
                int x := 5;
                &mut int p := &mut x;
                return 0;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::TakeMutRefOfImmutable { ref name } if name == "x"
        )));
    }

    #[test]
    fn deref_of_non_reference_is_rejected() {
        let source = r#"
            int main() {
                int x := 5;
                return *x;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::DerefOfNonReference { .. }
        )));
    }

    #[test]
    fn returning_reference_to_local_is_rejected() {
        let source = r#"
            &int dangling() {
                int x := 5;
                return &x;
            }
            int main() { return 0; }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::ReturnsLocalReference
        )));
    }

    #[test]
    fn valid_reference_operations_typecheck() {
        let source = r#"
            int main() {
                mut int x := 5;
                &mut int p := &mut x;
                *p = 99;
                return *p;
            }
        "#;
        assert!(analyze_source(source).is_ok());
    }

    #[test]
    fn mut_parameter_can_be_assigned() {
        let source = r#"
            int reset(mut int n) {
                n = 0;
                return n;
            }
            int main() { return 0; }
        "#;
        assert!(analyze_source(source).is_ok());
    }

    #[test]
    fn immutable_parameter_cannot_be_assigned() {
        let source = r#"
            int reset(int n) {
                n = 0;
                return n;
            }
            int main() { return 0; }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::AssignmentToImmutable(ref name) if name == "n"
        )));
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

    #[test]
    fn for_in_rejects_non_int_range_endpoints() {
        let source = r#"
            int main() {
                for i in 0..true { print_int(i); }
                return 0;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::TypeMismatch { ref expected, ref found }
                if *expected == TypeKind::Int && *found == TypeKind::Bool
        )));
    }

    #[test]
    fn for_in_loop_var_is_int_and_in_scope_in_body() {
        let source = r#"
            int main() {
                mut int total := 0;
                for i in 0..3 { total = total + i; }
                return total;
            }
        "#;
        assert!(analyze_source(source).is_ok());
    }

    #[test]
    fn for_in_loop_var_leaves_scope_after_loop() {
        let source = r#"
            int main() {
                for i in 0..3 { print_int(i); }
                print_int(i);
                return 0;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::UndeclaredVariable(ref n) if n == "i"
        )));
    }

    #[test]
    fn for_in_loop_var_is_immutable() {
        let source = r#"
            int main() {
                for i in 0..3 { i = i + 1; }
                return 0;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::AssignmentToImmutable(ref n) if n == "i"
        )));
    }

    #[test]
    fn array_index_returns_element_type() {
        let source = r#"
            int main() {
                arr[int] a := [10, 20, 30];
                return a[1];
            }
        "#;
        assert!(analyze_source(source).is_ok());
    }

    #[test]
    fn array_index_rejects_non_int_index() {
        let source = r#"
            int main() {
                arr[int] a := [10, 20, 30];
                return a[true];
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::IndexNotInteger { ref found } if *found == TypeKind::Bool
        )));
    }

    #[test]
    fn array_index_rejects_static_out_of_range() {
        let source = r#"
            int main() {
                arr[int] a := [10, 20, 30];
                return a[5];
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::IndexOutOfRange { array_size: 3, index: 5 }
        )));
    }

    #[test]
    fn array_element_assignment_requires_mutable_array() {
        let source = r#"
            int main() {
                arr[int] a := [10, 20, 30];
                a[1] = 99;
                return 0;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::AssignmentToImmutable(ref n) if n == "a"
        )));
    }

    #[test]
    fn array_element_assignment_on_mutable_array_typechecks() {
        let source = r#"
            int main() {
                mut arr[int] a := [10, 20, 30];
                a[1] = 99;
                return a[1];
            }
        "#;
        assert!(analyze_source(source).is_ok());
    }

    #[test]
    fn len_returns_int_for_array_argument() {
        let source = r#"
            int main() {
                arr[int] a := [10, 20, 30];
                return len(a);
            }
        "#;
        assert!(analyze_source(source).is_ok());
    }

    #[test]
    fn len_rejects_non_array_argument() {
        let source = r#"
            int main() {
                int x := 5;
                return len(x);
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::TypeMismatch { .. }
        )));
    }

    #[test]
    fn array_index_with_negative_literal_is_out_of_range() {
        let source = r#"
            int main() {
                arr[int] a := [10, 20, 30];
                return a[-1];
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::IndexOutOfRange { array_size: 3, index: -1 }
        )));
    }

    #[test]
    fn function_returning_array_typechecks_against_inferred_size() {
        let source = r#"
            arr[int] make_three() {
                return [10, 20, 30];
            }
            int main() {
                return 0;
            }
        "#;
        assert!(analyze_source(source).is_ok());
    }

    #[test]
    fn index_into_size_zero_array_does_not_emit_static_oob() {
        let source = r#"
            int first(arr[int] xs) {
                return xs[0];
            }
            int main() {
                return 0;
            }
        "#;
        let result = analyze_source(source);
        if let Err(errors) = &result {
            let has_oob = errors.iter().any(|e| matches!(
                e.kind,
                AnalyzeErrorKind::IndexOutOfRange { .. }
            ));
            assert!(
                !has_oob,
                "expected no static OOB on size-0 (unknown) array type, got {:?}",
                errors
            );
        }
    }

    #[test]
    fn bitwise_ops_accept_int_operands() {
        let source = r#"
            int main() {
                int a := 12;
                int b := 10;
                int and_v := a & b;
                int or_v := a | b;
                int xor_v := a ^ b;
                int shl_v := a << 1;
                int shr_v := a >> 1;
                int not_v := ~a;
                return and_v + or_v + xor_v + shl_v + shr_v + not_v;
            }
        "#;
        assert!(analyze_source(source).is_ok());
    }

    #[test]
    fn bitwise_ops_reject_bool_operands() {
        let source = r#"
            int main() {
                return true & false;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::TypeMismatch { .. }
        )));
    }

    #[test]
    fn deref_assignment_through_mutable_ref_typechecks() {
        let source = r#"
            int main() {
                mut int x := 5;
                &mut int p := &mut x;
                *p = 99;
                return *p;
            }
        "#;
        assert!(analyze_source(source).is_ok());
    }

    #[test]
    fn deref_assignment_through_immutable_ref_is_rejected() {
        let source = r#"
            int main() {
                int x := 5;
                &int p := &x;
                *p = 99;
                return 0;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::AssignmentToImmutable(_)
        )));
    }

    #[test]
    fn take_ref_of_non_int_is_rejected_for_now() {
        // Refs to non-int types are deferred to a future slice-based representation.
        // Currently rejected to avoid codegen miscompilation.
        let source = r#"
            int main() {
                mut bool b := true;
                &mut bool p := &mut b;
                return 0;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::TypeMismatch { ref expected, .. } if *expected == TypeKind::Int
        )));
    }

    #[test]
    fn ref_to_ref_is_rejected_for_now() {
        // Refs to refs would require pointee-type-aware codegen.
        let source = r#"
            int main() {
                mut int x := 5;
                mut &mut int p := &mut x;
                &mut &mut int pp := &mut p;
                return **pp;
            }
        "#;
        let errors = analyze_source(source).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::TypeMismatch { ref expected, .. } if *expected == TypeKind::Int
        )));
    }

    #[test]
    fn take_ref_through_parens_typechecks() {
        let source = r#"
            int main() {
                mut int x := 5;
                &mut int p := &mut (x);
                *p = 99;
                return *p;
            }
        "#;
        assert!(
            analyze_source(source).is_ok(),
            "ParenExpr should be treated as transparent for lvalue checks"
        );
    }

    #[test]
    fn for_in_rejects_non_range_iterable() {
        use crate::lexer::SourceLocation;
        use crate::parser::ast::{Expr, ExprKind, Identifier, Program, Stmt, StmtKind};

        let loc = SourceLocation { line: 1, column: 1 };

        // Build: for i in 5 { }  — iterable is IntLit, not a Range
        let iterable = Expr {
            kind: ExprKind::IntLit(5),
            location: loc.clone(),
        };
        let body = Expr {
            kind: ExprKind::BlockExpr { statements: vec![], ret: None },
            location: loc.clone(),
        };
        let stmt = Stmt {
            kind: StmtKind::ForIn {
                var: Identifier { name: "i".to_string(), location: loc.clone() },
                iterable: Box::new(iterable),
                body: Box::new(body),
            },
            location: loc.clone(),
        };
        let program = Program { statements: vec![stmt] };

        let mut analyzer = Analyzer::new();
        let errors = analyzer.analyze_program(&program).unwrap_err();
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            AnalyzeErrorKind::NonIterableInForLoop { ref found }
                if *found == TypeKind::Int
        )));
    }
}
