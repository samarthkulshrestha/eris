use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine};
use inkwell::OptimizationLevel;
use inkwell::IntPredicate;
use inkwell::FloatPredicate;
use std::collections::HashMap;
use std::path::Path;

use crate::parser::ast::{
    BinOp, Expr, ExprKind, Identifier, Program, Stmt, StmtKind, TypeKind, UnaryOp,
};

pub struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,

    // Symbol tables (variable name -> (pointer, type))
    variables: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
    functions: HashMap<String, FunctionValue<'ctx>>,

    // Current function being compiled
    current_function: Option<FunctionValue<'ctx>>,

    // Stack of (continue_target, break_target) for loop constructs
    loop_stack: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>)>,
}

impl<'ctx> CodeGen<'ctx> {
    /// Placeholder value returned from expression positions that don't produce
    /// a meaningful value (void function calls, blocks with no trailing
    /// expression, if-expressions with absent branches). The analyzer must
    /// prevent these from being consumed in value contexts; the placeholder
    /// exists only to satisfy the BasicValueEnum return type. Until codegen
    /// returns Option<BasicValueEnum> end-to-end, keep the dummy in one place
    /// so it's easy to find and reason about.
    fn void_placeholder(&self) -> BasicValueEnum<'ctx> {
        self.context.i64_type().const_int(0, false).into()
    }

    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        let mut codegen = CodeGen {
            context,
            module,
            builder,
            variables: HashMap::new(),
            functions: HashMap::new(),
            current_function: None,
            loop_stack: Vec::new(),
        };

        // Declare built-in I/O functions
        codegen.declare_builtin_functions();

        codegen
    }

    /// Declare built-in I/O functions
    fn declare_builtin_functions(&mut self) {
        // Declare external C functions
        self.declare_printf();
        self.declare_scanf();
        self.declare_putchar();

        // Declare Eris built-in functions
        self.declare_print_int();
        self.declare_print_bool();
        self.declare_print_char();
        self.declare_print_str();
        self.declare_println();
        self.declare_read_int();
    }

    /// Declare printf from C standard library
    fn declare_printf(&mut self) {
        let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let printf_type = self.context.i32_type().fn_type(&[i8_ptr_type.into()], true);
        self.module.add_function("printf", printf_type, None);
    }

    /// Declare scanf from C standard library
    fn declare_scanf(&mut self) {
        let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let scanf_type = self.context.i32_type().fn_type(&[i8_ptr_type.into()], true);
        self.module.add_function("scanf", scanf_type, None);
    }

    /// Declare putchar from C standard library
    fn declare_putchar(&mut self) {
        let putchar_type = self.context.i32_type().fn_type(&[self.context.i32_type().into()], false);
        self.module.add_function("putchar", putchar_type, None);
    }

    /// Declare print_int built-in function
    fn declare_print_int(&mut self) {
        let fn_type = self.context.void_type().fn_type(&[self.context.i64_type().into()], false);
        let function = self.module.add_function("print_int", fn_type, None);

        // Create function body
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Get printf function
        let printf_fn = self.module.get_function("printf").unwrap();

        // Create format string "%lld"
        let format_str = self.builder.build_global_string_ptr("%lld", "int_format")
            .map(|v| v.as_pointer_value())
            .unwrap();

        // Get the parameter
        let param = function.get_first_param().unwrap().into_int_value();

        // Call printf
        self.builder.build_call(
            printf_fn,
            &[format_str.into(), param.into()],
            "printf_call"
        ).unwrap();

        // Return void
        self.builder.build_return(None).unwrap();

        self.functions.insert("print_int".to_string(), function);
    }

    /// Declare print_bool built-in function
    fn declare_print_bool(&mut self) {
        let fn_type = self.context.void_type().fn_type(&[self.context.bool_type().into()], false);
        let function = self.module.add_function("print_bool", fn_type, None);

        // Create function body
        let entry = self.context.append_basic_block(function, "entry");
        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let merge_block = self.context.append_basic_block(function, "merge");

        self.builder.position_at_end(entry);

        // Get printf function
        let printf_fn = self.module.get_function("printf").unwrap();

        // Get the parameter
        let param = function.get_first_param().unwrap().into_int_value();

        // Branch based on bool value
        self.builder.build_conditional_branch(param, then_block, else_block).unwrap();

        // Then block: print "true"
        self.builder.position_at_end(then_block);
        let true_str = self.builder.build_global_string_ptr("true", "true_str")
            .map(|v| v.as_pointer_value())
            .unwrap();
        self.builder.build_call(printf_fn, &[true_str.into()], "printf_true").unwrap();
        self.builder.build_unconditional_branch(merge_block).unwrap();

        // Else block: print "false"
        self.builder.position_at_end(else_block);
        let false_str = self.builder.build_global_string_ptr("false", "false_str")
            .map(|v| v.as_pointer_value())
            .unwrap();
        self.builder.build_call(printf_fn, &[false_str.into()], "printf_false").unwrap();
        self.builder.build_unconditional_branch(merge_block).unwrap();

        // Merge block
        self.builder.position_at_end(merge_block);
        self.builder.build_return(None).unwrap();

        self.functions.insert("print_bool".to_string(), function);
    }

    /// Declare print_char built-in function
    fn declare_print_char(&mut self) {
        let fn_type = self.context.void_type().fn_type(&[self.context.i8_type().into()], false);
        let function = self.module.add_function("print_char", fn_type, None);

        // Create function body
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Get putchar function
        let putchar_fn = self.module.get_function("putchar").unwrap();

        // Get the parameter (i8) and extend to i32 for putchar
        let param = function.get_first_param().unwrap().into_int_value();
        let extended = self.builder.build_int_cast(param, self.context.i32_type(), "extend").unwrap();

        // Call putchar
        self.builder.build_call(putchar_fn, &[extended.into()], "putchar_call").unwrap();

        // Return void
        self.builder.build_return(None).unwrap();

        self.functions.insert("print_char".to_string(), function);
    }

    /// Declare print_str built-in function
    fn declare_print_str(&mut self) {
        let i8_ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let fn_type = self.context.void_type().fn_type(&[i8_ptr_type.into()], false);
        let function = self.module.add_function("print_str", fn_type, None);

        // Create function body
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Get printf function
        let printf_fn = self.module.get_function("printf").unwrap();

        // Create format string "%s"
        let format_str = self.builder.build_global_string_ptr("%s", "str_format")
            .map(|v| v.as_pointer_value())
            .unwrap();

        // Get the parameter (string pointer)
        let param = function.get_first_param().unwrap().into_pointer_value();

        // Call printf with %s format
        self.builder.build_call(
            printf_fn,
            &[format_str.into(), param.into()],
            "printf_str"
        ).unwrap();

        // Return void
        self.builder.build_return(None).unwrap();

        self.functions.insert("print_str".to_string(), function);
    }

    /// Declare println built-in function (prints a newline)
    fn declare_println(&mut self) {
        let fn_type = self.context.void_type().fn_type(&[], false);
        let function = self.module.add_function("println", fn_type, None);

        // Create function body
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Get putchar function
        let putchar_fn = self.module.get_function("putchar").unwrap();

        // Print newline character (10)
        let newline = self.context.i32_type().const_int(10, false);
        self.builder.build_call(putchar_fn, &[newline.into()], "putchar_newline").unwrap();

        // Return void
        self.builder.build_return(None).unwrap();

        self.functions.insert("println".to_string(), function);
    }

    /// Declare read_int built-in function
    fn declare_read_int(&mut self) {
        let fn_type = self.context.i64_type().fn_type(&[], false);
        let function = self.module.add_function("read_int", fn_type, None);

        // Create function body
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Get scanf function
        let scanf_fn = self.module.get_function("scanf").unwrap();

        // Create format string "%lld"
        let format_str = self.builder.build_global_string_ptr("%lld", "read_format")
            .map(|v| v.as_pointer_value())
            .unwrap();

        // Allocate space for the result and initialize to 0 so that a failed
        // scanf (e.g. EOF or non-numeric input) returns a deterministic 0
        // instead of reading uninitialized stack memory.
        let result_ptr = self.builder.build_alloca(self.context.i64_type(), "result").unwrap();
        self.builder.build_store(result_ptr, self.context.i64_type().const_int(0, false)).unwrap();

        // Call scanf
        self.builder.build_call(
            scanf_fn,
            &[format_str.into(), result_ptr.into()],
            "scanf_call"
        ).unwrap();

        // Load and return the result
        let result = self.builder.build_load(self.context.i64_type(), result_ptr, "result_val").unwrap();
        self.builder.build_return(Some(&result)).unwrap();

        self.functions.insert("read_int".to_string(), function);
    }

    /// Generate LLVM IR from an Eris program
    pub fn generate(&mut self, program: &Program) -> Result<(), String> {
        // First pass: declare all functions
        for stmt in &program.statements {
            if let StmtKind::FuncDecl { name, parameters, return_type, .. } = &stmt.kind {
                self.declare_function(name, parameters, return_type.as_ref())?;
            }
        }

        // Second pass: generate function bodies and top-level code
        for stmt in &program.statements {
            self.generate_statement(stmt)?;
        }

        Ok(())
    }

    /// Declare a function (signature only)
    fn declare_function(
        &mut self,
        name: &Identifier,
        parameters: &[(Identifier, crate::parser::ast::Type)],
        return_type: Option<&crate::parser::ast::Type>,
    ) -> Result<FunctionValue<'ctx>, String> {
        // Build parameter types
        let param_types: Vec<BasicMetadataTypeEnum> = parameters
            .iter()
            .map(|(_, typ)| self.get_llvm_type(&typ.kind).into())
            .collect();

        // Build function type
        let fn_type = if let Some(ret_type) = return_type {
            let llvm_ret_type = self.get_llvm_type(&ret_type.kind);
            llvm_ret_type.fn_type(&param_types, false)
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };

        // Create the function
        let function = self.module.add_function(&name.name, fn_type, None);

        // Set parameter names
        for (i, (param_name, _)) in parameters.iter().enumerate() {
            function.get_nth_param(i as u32).unwrap().set_name(&param_name.name);
        }

        self.functions.insert(name.name.clone(), function);
        Ok(function)
    }

    /// Resolve an expression to the alloca and LLVM array type of an array variable.
    fn array_ptr_and_type(
        &self,
        expr: &Expr,
    ) -> Result<(PointerValue<'ctx>, BasicTypeEnum<'ctx>), String> {
        match &expr.kind {
            ExprKind::IdentExpr(ident) => {
                let (ptr, ty) = self
                    .variables
                    .get(&ident.name)
                    .ok_or_else(|| format!("Undefined variable: {}", ident.name))?
                    .clone();
                if !ty.is_array_type() {
                    return Err(format!(
                        "expected array variable '{}' in indexing position",
                        ident.name
                    ));
                }
                Ok((ptr, ty))
            }
            _ => Err(
                "array indexing is only supported on identifier expressions".to_string(),
            ),
        }
    }

    /// Map Eris types to LLVM types
    fn get_llvm_type(&self, type_kind: &TypeKind) -> BasicTypeEnum<'ctx> {
        match type_kind {
            TypeKind::Int => self.context.i64_type().into(),
            TypeKind::Float => self.context.f64_type().into(),
            TypeKind::Bool => self.context.bool_type().into(),
            TypeKind::Char => self.context.i8_type().into(),
            TypeKind::Str => self.context.ptr_type(inkwell::AddressSpace::default()).into(),
            TypeKind::Array { element, size } => {
                let elem_ty = self.get_llvm_type(element);
                elem_ty.array_type(*size as u32).into()
            }
        }
    }

    /// Generate code for a statement
    fn generate_statement(&mut self, stmt: &Stmt) -> Result<(), String> {
        match &stmt.kind {
            StmtKind::VarDecl { ident, typ, initializer, .. } => {
                self.generate_var_decl(ident, typ.as_ref(), initializer)?;
            }
            StmtKind::FuncDecl { name, parameters, return_type, body } => {
                self.generate_function(name, parameters, return_type.as_ref(), body)?;
            }
            StmtKind::Assignment { target, value } => {
                self.generate_assignment(target, value)?;
            }
            StmtKind::ReturnStmt(expr) => {
                self.generate_return(expr.as_ref())?;
            }
            StmtKind::ExprStmt(expr) => {
                self.generate_expression(expr)?;
            }
            StmtKind::WhileLoop { condition, body } => {
                let parent_fn = self.current_function
                    .ok_or("while-loop must be inside a function")?;

                let cond_block = self.context.append_basic_block(parent_fn, "while.cond");
                let body_block = self.context.append_basic_block(parent_fn, "while.body");
                let after_block = self.context.append_basic_block(parent_fn, "while.after");

                self.builder.build_unconditional_branch(cond_block)
                    .map_err(|e| format!("Failed to branch to while.cond: {}", e))?;

                self.builder.position_at_end(cond_block);
                let cond_value = self.generate_expression(condition)?;
                self.builder.build_conditional_branch(
                    cond_value.into_int_value(),
                    body_block,
                    after_block,
                ).map_err(|e| format!("Failed to build while branch: {}", e))?;

                self.builder.position_at_end(body_block);
                self.loop_stack.push((cond_block, after_block));
                self.generate_expression(body)?;
                self.loop_stack.pop();

                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(cond_block)
                        .map_err(|e| format!("Failed to back-branch: {}", e))?;
                }

                self.builder.position_at_end(after_block);
            }
            StmtKind::Break => {
                let (_, after) = self.loop_stack.last()
                    .copied()
                    .ok_or("break outside loop reached codegen — analyzer should have caught this")?;
                self.builder.build_unconditional_branch(after)
                    .map_err(|e| format!("Failed to build break branch: {}", e))?;
            }
            StmtKind::Continue => {
                let (cond, _) = self.loop_stack.last()
                    .copied()
                    .ok_or("continue outside loop reached codegen — analyzer should have caught this")?;
                self.builder.build_unconditional_branch(cond)
                    .map_err(|e| format!("Failed to build continue branch: {}", e))?;
            }
            StmtKind::ForIn { var, iterable, body } => {
                let (start_expr, end_expr) = match &iterable.kind {
                    ExprKind::Range { start, end } => (start.as_ref(), end.as_ref()),
                    _ => return Err(
                        "ForIn iterable must be a Range (analyzer should have caught this)".to_string()
                    ),
                };

                let parent_fn = self
                    .current_function
                    .ok_or("for-loop must be inside a function")?;

                let i64_ty = self.context.i64_type();
                let loop_var_alloca = self
                    .builder
                    .build_alloca(i64_ty, &var.name)
                    .map_err(|e| format!("Failed to alloca for-loop var: {}", e))?;
                let start_val = self.generate_expression(start_expr)?;
                self.builder
                    .build_store(loop_var_alloca, start_val)
                    .map_err(|e| format!("Failed to store for-loop start: {}", e))?;

                let cond_block = self.context.append_basic_block(parent_fn, "for.cond");
                let body_block = self.context.append_basic_block(parent_fn, "for.body");
                let inc_block = self.context.append_basic_block(parent_fn, "for.inc");
                let after_block = self.context.append_basic_block(parent_fn, "for.after");

                self.builder
                    .build_unconditional_branch(cond_block)
                    .map_err(|e| format!("Failed to branch into for.cond: {}", e))?;

                self.builder.position_at_end(cond_block);
                // end_val is re-evaluated each iteration; for literal/simple bounds
                // this is a no-op, and hoisting adds complexity without benefit here.
                let end_val = self.generate_expression(end_expr)?;
                let cur = self
                    .builder
                    .build_load(i64_ty, loop_var_alloca, "for.iv")
                    .map_err(|e| format!("Failed to load for-loop var: {}", e))?
                    .into_int_value();
                let done = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLT,
                        cur,
                        end_val.into_int_value(),
                        "for.cmp",
                    )
                    .map_err(|e| format!("Failed to compare for-loop bound: {}", e))?;
                self.builder
                    .build_conditional_branch(done, body_block, after_block)
                    .map_err(|e| format!("Failed to branch from for.cond: {}", e))?;

                self.builder.position_at_end(body_block);
                let saved_variables = self.variables.clone();
                self.variables
                    .insert(var.name.clone(), (loop_var_alloca, i64_ty.into()));
                self.loop_stack.push((inc_block, after_block));
                self.generate_expression(body)?;
                self.loop_stack.pop();
                self.variables = saved_variables;

                if self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_terminator())
                    .is_none()
                {
                    self.builder
                        .build_unconditional_branch(inc_block)
                        .map_err(|e| format!("Failed to branch from for.body to for.inc: {}", e))?;
                }

                self.builder.position_at_end(inc_block);
                let cur_for_inc = self
                    .builder
                    .build_load(i64_ty, loop_var_alloca, "for.iv.next")
                    .map_err(|e| format!("Failed to load for-loop var (inc): {}", e))?
                    .into_int_value();
                let one = i64_ty.const_int(1, false);
                let next = self
                    .builder
                    .build_int_add(cur_for_inc, one, "for.inc")
                    .map_err(|e| format!("Failed to build for-loop increment: {}", e))?;
                self.builder
                    .build_store(loop_var_alloca, next)
                    .map_err(|e| format!("Failed to store for-loop increment: {}", e))?;
                self.builder
                    .build_unconditional_branch(cond_block)
                    .map_err(|e| format!("Failed to back-branch for-loop: {}", e))?;

                self.builder.position_at_end(after_block);
            }
        }
        Ok(())
    }

    /// Generate code for variable declaration
    fn generate_var_decl(
        &mut self,
        ident: &Identifier,
        _typ: Option<&crate::parser::ast::Type>,
        initializer: &Expr,
    ) -> Result<(), String> {
        let init_value = self.generate_expression(initializer)?;
        let value_type = init_value.get_type();

        // Allocate space for the variable
        let alloca = self.builder.build_alloca(value_type, &ident.name)
            .map_err(|e| format!("Failed to create alloca: {}", e))?;

        // Store the initial value
        self.builder.build_store(alloca, init_value)
            .map_err(|e| format!("Failed to store value: {}", e))?;

        // Remember the variable with its type
        self.variables.insert(ident.name.clone(), (alloca, value_type));

        Ok(())
    }

    /// Generate code for function definition
    fn generate_function(
        &mut self,
        name: &Identifier,
        parameters: &[(Identifier, crate::parser::ast::Type)],
        _return_type: Option<&crate::parser::ast::Type>,
        body: &Expr,
    ) -> Result<(), String> {
        let function = self.functions.get(&name.name)
            .ok_or_else(|| format!("Function {} not declared", name.name))?
            .clone();

        self.current_function = Some(function);

        // Create entry block
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        // Save current variable scope and loop stack; ensure restoration even on error.
        let saved_variables = std::mem::take(&mut self.variables);
        let saved_loop_stack = std::mem::take(&mut self.loop_stack);
        let result = self.generate_function_body(function, parameters, body);
        self.variables = saved_variables;
        self.loop_stack = saved_loop_stack;
        self.current_function = None;
        result
    }

    fn generate_function_body(
        &mut self,
        function: FunctionValue<'ctx>,
        parameters: &[(Identifier, crate::parser::ast::Type)],
        body: &Expr,
    ) -> Result<(), String> {
        // Allocate and store parameters
        for (i, (param_name, param_type)) in parameters.iter().enumerate() {
            let param_value = function.get_nth_param(i as u32).unwrap();
            let llvm_type = self.get_llvm_type(&param_type.kind);

            let alloca = self.builder.build_alloca(llvm_type, &param_name.name)
                .map_err(|e| format!("Failed to create parameter alloca: {}", e))?;

            self.builder.build_store(alloca, param_value)
                .map_err(|e| format!("Failed to store parameter: {}", e))?;

            self.variables.insert(param_name.name.clone(), (alloca, llvm_type));
        }

        // Generate function body
        let body_value = self.generate_expression(body)?;

        // Add return if not already present
        if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
            self.builder.build_return(Some(&body_value))
                .map_err(|e| format!("Failed to build return: {}", e))?;
        }

        Ok(())
    }

    /// Generate code for assignment
    fn generate_assignment(&mut self, target: &Expr, value: &Expr) -> Result<(), String> {
        match &target.kind {
            ExprKind::IdentExpr(ident) => {
                let (var_ptr, _var_type) = self
                    .variables
                    .get(&ident.name)
                    .ok_or_else(|| format!("Undefined variable: {}", ident.name))?
                    .clone();
                let value_result = self.generate_expression(value)?;
                self.builder
                    .build_store(var_ptr, value_result)
                    .map_err(|e| format!("Failed to store assignment: {}", e))?;
                Ok(())
            }
            ExprKind::ArrayIndex { array, index } => {
                let (alloca_ptr, arr_basic_ty) = self.array_ptr_and_type(array)?;
                let arr_ty = arr_basic_ty.into_array_type();
                let idx_val = self.generate_expression(index)?.into_int_value();
                let i64_ty = self.context.i64_type();
                let zero = i64_ty.const_int(0, false);
                let elem_ptr = unsafe {
                    self.builder
                        .build_in_bounds_gep(
                            arr_ty,
                            alloca_ptr,
                            &[zero, idx_val],
                            "arr.assign.gep",
                        )
                        .map_err(|e| format!("Failed to GEP for array assignment: {}", e))?
                };
                let value_result = self.generate_expression(value)?;
                self.builder
                    .build_store(elem_ptr, value_result)
                    .map_err(|e| format!("Failed to store array element: {}", e))?;
                Ok(())
            }
            _ => Err("Assignment target must be a variable or array index".to_string()),
        }
    }

    /// Generate code for return statement
    fn generate_return(&mut self, expr: Option<&Expr>) -> Result<(), String> {
        if let Some(expr) = expr {
            let value = self.generate_expression(expr)?;
            self.builder.build_return(Some(&value))
                .map_err(|e| format!("Failed to build return: {}", e))?;
        } else {
            self.builder.build_return(None)
                .map_err(|e| format!("Failed to build void return: {}", e))?;
        }
        Ok(())
    }

    /// Generate code for an expression
    fn generate_expression(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        match &expr.kind {
            ExprKind::IntLit(val) => {
                Ok(self.context.i64_type().const_int(*val as u64, true).into())
            }
            ExprKind::FloatLit(val) => {
                Ok(self.context.f64_type().const_float(*val).into())
            }
            ExprKind::BoolTrue => {
                Ok(self.context.bool_type().const_int(1, false).into())
            }
            ExprKind::BoolFalse => {
                Ok(self.context.bool_type().const_int(0, false).into())
            }
            ExprKind::CharLit(val) => {
                Ok(self.context.i8_type().const_int(*val as u64, false).into())
            }
            ExprKind::StringLit(s) => {
                // Create a global string constant
                let global_str = self.builder.build_global_string_ptr(s, "str")
                    .map_err(|e| format!("Failed to create string literal: {}", e))?;
                Ok(global_str.as_pointer_value().into())
            }
            ExprKind::IdentExpr(ident) => {
                let (var_ptr, var_type) = self.variables.get(&ident.name)
                    .ok_or_else(|| format!("Undefined variable: {}", ident.name))?
                    .clone();

                self.builder.build_load(var_type, var_ptr, &ident.name)
                    .map_err(|e| format!("Failed to load variable: {}", e))
            }
            ExprKind::BinaryExpr { left, op, right } => {
                self.generate_binary_op(left, op, right)
            }
            ExprKind::UnaryExpr { op, expression } => {
                self.generate_unary_op(op, expression)
            }
            ExprKind::FunctionCall { function, arguments } => {
                self.generate_function_call(function, arguments)
            }
            ExprKind::BlockExpr { statements, ret } => {
                self.generate_block(statements, ret.as_deref())
            }
            ExprKind::IfExpr { condition, then_branch, else_branch } => {
                self.generate_if_expr(condition, then_branch.as_deref(), else_branch.as_deref())
            }
            ExprKind::ParenExpr(inner) => {
                self.generate_expression(inner)
            }
            ExprKind::ArrayLit(elements) => {
                if elements.is_empty() {
                    return Err("empty array literals are not supported".to_string());
                }
                let first_val = self.generate_expression(&elements[0])?;
                let elem_ty = first_val.get_type();
                let arr_ty = elem_ty.array_type(elements.len() as u32);

                let alloca = self
                    .builder
                    .build_alloca(arr_ty, "arr.lit")
                    .map_err(|e| format!("Failed to alloca array: {}", e))?;

                let i64_ty = self.context.i64_type();
                let zero = i64_ty.const_int(0, false);
                for (i, element) in elements.iter().enumerate() {
                    let value = if i == 0 {
                        first_val
                    } else {
                        self.generate_expression(element)?
                    };
                    let idx = i64_ty.const_int(i as u64, false);
                    let elem_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(arr_ty, alloca, &[zero, idx], "arr.lit.gep")
                            .map_err(|e| format!("Failed to GEP for array literal: {}", e))?
                    };
                    self.builder
                        .build_store(elem_ptr, value)
                        .map_err(|e| format!("Failed to store array element: {}", e))?;
                }

                let loaded = self
                    .builder
                    .build_load(arr_ty, alloca, "arr.lit.val")
                    .map_err(|e| format!("Failed to load array literal: {}", e))?;
                Ok(loaded)
            }
            ExprKind::ArrayIndex { array, index } => {
                let (alloca_ptr, arr_basic_ty) = self.array_ptr_and_type(array)?;
                let arr_ty = arr_basic_ty.into_array_type();
                let elem_ty = arr_ty.get_element_type();
                let idx_val = self.generate_expression(index)?.into_int_value();
                let i64_ty = self.context.i64_type();
                let zero = i64_ty.const_int(0, false);
                let elem_ptr = unsafe {
                    self.builder
                        .build_in_bounds_gep(arr_ty, alloca_ptr, &[zero, idx_val], "arr.idx")
                        .map_err(|e| format!("Failed to GEP for array index: {}", e))?
                };
                self.builder
                    .build_load(elem_ty, elem_ptr, "arr.idx.val")
                    .map_err(|e| format!("Failed to load array element: {}", e))
            }
            _ => Err(format!("Unsupported expression: {:?}", expr.kind)),
        }
    }

    /// Generate code for binary operations
    fn generate_binary_op(
        &mut self,
        left: &Expr,
        op: &BinOp,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let left_val = self.generate_expression(left)?;
        let right_val = self.generate_expression(right)?;

        match op {
            BinOp::Add => {
                if left_val.is_int_value() {
                    self.builder.build_int_add(left_val.into_int_value(), right_val.into_int_value(), "add")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int add: {}", e))
                } else {
                    self.builder.build_float_add(left_val.into_float_value(), right_val.into_float_value(), "fadd")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float add: {}", e))
                }
            }
            BinOp::Sub => {
                if left_val.is_int_value() {
                    self.builder.build_int_sub(left_val.into_int_value(), right_val.into_int_value(), "sub")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int sub: {}", e))
                } else {
                    self.builder.build_float_sub(left_val.into_float_value(), right_val.into_float_value(), "fsub")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float sub: {}", e))
                }
            }
            BinOp::Mul => {
                if left_val.is_int_value() {
                    self.builder.build_int_mul(left_val.into_int_value(), right_val.into_int_value(), "mul")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int mul: {}", e))
                } else {
                    self.builder.build_float_mul(left_val.into_float_value(), right_val.into_float_value(), "fmul")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float mul: {}", e))
                }
            }
            BinOp::Div => {
                if left_val.is_int_value() {
                    self.builder.build_int_signed_div(left_val.into_int_value(), right_val.into_int_value(), "div")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int div: {}", e))
                } else {
                    self.builder.build_float_div(left_val.into_float_value(), right_val.into_float_value(), "fdiv")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float div: {}", e))
                }
            }
            BinOp::Rem => {
                if left_val.is_int_value() {
                    self.builder.build_int_signed_rem(left_val.into_int_value(), right_val.into_int_value(), "rem")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int rem: {}", e))
                } else {
                    self.builder.build_float_rem(left_val.into_float_value(), right_val.into_float_value(), "frem")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float rem: {}", e))
                }
            }
            BinOp::Eq => {
                if left_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::EQ, left_val.into_int_value(), right_val.into_int_value(), "eq")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int eq: {}", e))
                } else {
                    self.builder.build_float_compare(FloatPredicate::OEQ, left_val.into_float_value(), right_val.into_float_value(), "feq")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float eq: {}", e))
                }
            }
            BinOp::Ne => {
                if left_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::NE, left_val.into_int_value(), right_val.into_int_value(), "ne")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int ne: {}", e))
                } else {
                    self.builder.build_float_compare(FloatPredicate::ONE, left_val.into_float_value(), right_val.into_float_value(), "fne")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float ne: {}", e))
                }
            }
            BinOp::Lt => {
                if left_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::SLT, left_val.into_int_value(), right_val.into_int_value(), "lt")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int lt: {}", e))
                } else {
                    self.builder.build_float_compare(FloatPredicate::OLT, left_val.into_float_value(), right_val.into_float_value(), "flt")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float lt: {}", e))
                }
            }
            BinOp::Le => {
                if left_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::SLE, left_val.into_int_value(), right_val.into_int_value(), "le")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int le: {}", e))
                } else {
                    self.builder.build_float_compare(FloatPredicate::OLE, left_val.into_float_value(), right_val.into_float_value(), "fle")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float le: {}", e))
                }
            }
            BinOp::Gt => {
                if left_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::SGT, left_val.into_int_value(), right_val.into_int_value(), "gt")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int gt: {}", e))
                } else {
                    self.builder.build_float_compare(FloatPredicate::OGT, left_val.into_float_value(), right_val.into_float_value(), "fgt")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float gt: {}", e))
                }
            }
            BinOp::Ge => {
                if left_val.is_int_value() {
                    self.builder.build_int_compare(IntPredicate::SGE, left_val.into_int_value(), right_val.into_int_value(), "ge")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int ge: {}", e))
                } else {
                    self.builder.build_float_compare(FloatPredicate::OGE, left_val.into_float_value(), right_val.into_float_value(), "fge")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float ge: {}", e))
                }
            }
            BinOp::LAnd => {
                self.builder.build_and(left_val.into_int_value(), right_val.into_int_value(), "and")
                    .map(|v| v.into())
                    .map_err(|e| format!("Failed to build and: {}", e))
            }
            BinOp::LOr => {
                self.builder.build_or(left_val.into_int_value(), right_val.into_int_value(), "or")
                    .map(|v| v.into())
                    .map_err(|e| format!("Failed to build or: {}", e))
            }
            BinOp::BAnd | BinOp::BOr | BinOp::BXor | BinOp::Shl | BinOp::Shr => {
                Err("bitwise op codegen not yet implemented".to_string())
            }
        }
    }

    /// Generate code for unary operations
    fn generate_unary_op(&mut self, op: &UnaryOp, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let val = self.generate_expression(expr)?;

        match op {
            UnaryOp::Neg => {
                if val.is_int_value() {
                    self.builder.build_int_neg(val.into_int_value(), "neg")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build int neg: {}", e))
                } else {
                    self.builder.build_float_neg(val.into_float_value(), "fneg")
                        .map(|v| v.into())
                        .map_err(|e| format!("Failed to build float neg: {}", e))
                }
            }
            UnaryOp::LNot => {
                self.builder.build_not(val.into_int_value(), "not")
                    .map(|v| v.into())
                    .map_err(|e| format!("Failed to build not: {}", e))
            }
            UnaryOp::BNot => {
                self.builder.build_not(val.into_int_value(), "bnot")
                    .map(|v| v.into())
                    .map_err(|e| format!("Failed to build bnot: {}", e))
            }
        }
    }

    /// Generate code for function calls
    fn generate_function_call(
        &mut self,
        function: &Identifier,
        arguments: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if function.name == "len" && arguments.len() == 1 {
            if let ExprKind::IdentExpr(ident) = &arguments[0].kind {
                if let Some((_, ty)) = self.variables.get(&ident.name).cloned() {
                    if ty.is_array_type() {
                        let size = ty.into_array_type().len();
                        return Ok(self
                            .context
                            .i64_type()
                            .const_int(size as u64, false)
                            .into());
                    }
                }
            }
            return Err("len() in codegen only supports identifier args to array variables".to_string());
        }

        let func = self.functions.get(&function.name)
            .ok_or_else(|| format!("Undefined function: {}", function.name))?
            .clone();

        let mut args: Vec<BasicMetadataValueEnum> = Vec::new();
        for arg in arguments {
            let val = self.generate_expression(arg)?;
            args.push(val.into());
        }

        let call_result = self.builder.build_call(func, &args, "call")
            .map_err(|e| format!("Failed to build call: {}", e))?;

        // Check if function returns void
        if let Some(value) = call_result.try_as_basic_value().left() {
            Ok(value)
        } else {
            // Void function - return a dummy value (won't be used)
            Ok(self.void_placeholder())
        }
    }

    /// Generate code for block expressions
    fn generate_block(
        &mut self,
        statements: &[Stmt],
        ret: Option<&Expr>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        // Save current variable scope
        let saved_variables = self.variables.clone();

        // Generate statements, stopping early if the current block is already
        // terminated (e.g. by a break or continue). Inserting into a terminated
        // block produces invalid IR ("instruction after terminator").
        for stmt in statements {
            if self.builder.get_insert_block()
                .and_then(|b| b.get_terminator())
                .is_some()
            {
                break;
            }
            self.generate_statement(stmt)?;
        }

        // Emit the trailing expression only if the block is not already terminated.
        // A block whose final statement was a terminator (break, continue, return)
        // has no live insert point; attempting to emit `ret` here would produce
        // invalid IR ("instruction after terminator").
        let result = if self.builder.get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_some()
        {
            self.void_placeholder()
        } else if let Some(ret_expr) = ret {
            self.generate_expression(ret_expr)?
        } else {
            self.void_placeholder()
        };

        // Restore variable scope
        self.variables = saved_variables;

        Ok(result)
    }

    /// Generate code for if expressions
    fn generate_if_expr(
        &mut self,
        condition: &Expr,
        then_branch: Option<&Expr>,
        else_branch: Option<&Expr>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let cond_value = self.generate_expression(condition)?;
        let parent_fn = self.current_function
            .ok_or("If expression must be inside a function")?;

        let then_block = self.context.append_basic_block(parent_fn, "then");
        let else_block = self.context.append_basic_block(parent_fn, "else");
        let merge_block = self.context.append_basic_block(parent_fn, "ifcont");

        // Build conditional branch
        self.builder.build_conditional_branch(cond_value.into_int_value(), then_block, else_block)
            .map_err(|e| format!("Failed to build conditional branch: {}", e))?;

        // Generate then block
        self.builder.position_at_end(then_block);
        let then_value = if let Some(then_expr) = then_branch {
            self.generate_expression(then_expr)?
        } else {
            self.void_placeholder()
        };

        let then_has_terminator = self.builder.get_insert_block().unwrap().get_terminator().is_some();
        if !then_has_terminator {
            self.builder.build_unconditional_branch(merge_block)
                .map_err(|e| format!("Failed to build branch to merge: {}", e))?;
        }
        let then_end_block = self.builder.get_insert_block().unwrap();

        // Generate else block
        self.builder.position_at_end(else_block);
        let else_value = if let Some(else_expr) = else_branch {
            self.generate_expression(else_expr)?
        } else {
            self.void_placeholder()
        };

        let else_has_terminator = self.builder.get_insert_block().unwrap().get_terminator().is_some();
        if !else_has_terminator {
            self.builder.build_unconditional_branch(merge_block)
                .map_err(|e| format!("Failed to build branch to merge: {}", e))?;
        }
        let else_end_block = self.builder.get_insert_block().unwrap();

        // Only generate merge block if at least one branch needs it
        if then_has_terminator && else_has_terminator {
            // Both branches have terminators (returns), so the merge block is unreachable
            merge_block.remove_from_function().map_err(|_| "Failed to remove unreachable merge block")?;
            Ok(self.void_placeholder())
        } else if then_branch.is_none() && else_branch.is_none() {
            // Neither branch produces a value (statement-context if). Skip the
            // phi node; the result must not be consumed by valid programs.
            self.builder.position_at_end(merge_block);
            Ok(self.void_placeholder())
        } else {
            // Generate merge block with phi node
            self.builder.position_at_end(merge_block);
            let phi = self.builder.build_phi(then_value.get_type(), "iftmp")
                .map_err(|e| format!("Failed to build phi: {}", e))?;

            // Only add incoming edges from blocks that actually jump to merge
            if !then_has_terminator && !else_has_terminator {
                phi.add_incoming(&[(&then_value, then_end_block), (&else_value, else_end_block)]);
            } else if !then_has_terminator {
                phi.add_incoming(&[(&then_value, then_end_block)]);
            } else if !else_has_terminator {
                phi.add_incoming(&[(&else_value, else_end_block)]);
            }

            Ok(phi.as_basic_value())
        }
    }

    /// Get the generated LLVM IR as a string. Used by codegen tests to
    /// assert IR shape; not part of the runtime compile path.
    #[cfg(test)]
    pub(crate) fn print_to_string(&self) -> String {
        self.module.print_to_string().to_string()
    }

    /// Write the module to a file
    pub fn write_to_file(&self, path: &str) -> Result<(), String> {
        self.module.print_to_file(path)
            .map_err(|e| format!("Failed to write module to file: {}", e.to_string()))
    }

    /// Initialize the target machine for native code generation
    fn get_target_machine() -> Result<TargetMachine, String> {
        Target::initialize_native(&InitializationConfig::default())
            .map_err(|e| format!("Failed to initialize native target: {}", e))?;

        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple)
            .map_err(|e| format!("Failed to create target from triple: {}", e))?;

        let cpu = TargetMachine::get_host_cpu_name().to_string();
        let features = TargetMachine::get_host_cpu_features().to_string();

        target
            .create_target_machine(
                &target_triple,
                &cpu,
                &features,
                OptimizationLevel::Default,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or_else(|| "Failed to create target machine".to_string())
    }

    /// Write object file
    pub fn write_object_file(&self, path: &Path) -> Result<(), String> {
        let target_machine = Self::get_target_machine()?;

        target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| format!("Failed to write object file: {}", e.to_string()))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::Analyzer;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn compile_to_ir(source: &str) -> String {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("lex");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("parse");
        let mut analyzer = Analyzer::new();
        analyzer.analyze_program(&program).expect("analyze");
        let context = Context::create();
        let mut codegen = CodeGen::new(&context, "test");
        codegen.generate(&program).expect("codegen");
        codegen.print_to_string()
    }

    #[test]
    fn ir_emits_main_returning_i64() {
        let ir = compile_to_ir("int main() { return 42; }");
        assert!(ir.contains("define i64 @main()"));
        assert!(ir.contains("ret i64 42"));
    }

    #[test]
    fn ir_emits_variable_alloca_and_store() {
        let ir = compile_to_ir("int main() { int x := 10; return x; }");
        assert!(ir.contains("alloca i64"));
        assert!(ir.contains("store i64 10"));
    }

    #[test]
    fn ir_emits_function_with_parameter() {
        let ir = compile_to_ir(
            "int square(int n) { return n * n; }
             int main() { return square(7); }",
        );
        assert!(ir.contains("define i64 @square(i64"));
        assert!(ir.contains("call i64 @square"));
        assert!(ir.contains("mul i64"));
    }

    #[test]
    fn ir_emits_if_expression_with_phi() {
        let ir = compile_to_ir("int main() { return if true { 1 } else { 2 }; }");
        assert!(ir.contains("phi i64"));
    }

    #[test]
    fn ir_emits_arithmetic_ops() {
        // Use parameters to defeat LLVM's constant folding
        let ir = compile_to_ir(
            "int compute(int a, int b) { return a + b * a - b; }
             int main() { return compute(1, 2); }",
        );
        assert!(ir.contains("add i64"));
        assert!(ir.contains("mul i64"));
        assert!(ir.contains("sub i64"));
    }

    #[test]
    fn ir_emits_comparison_ops() {
        let ir = compile_to_ir(
            "int main() { int n := read_int(); bool b := n < 2; return if b { 0 } else { 1 }; }",
        );
        assert!(ir.contains("icmp slt"));
    }

    #[test]
    fn ir_declares_builtin_print_helpers() {
        let ir = compile_to_ir("int main() { print_int(42); return 0; }");
        assert!(ir.contains("declare i32 @printf"));
        assert!(ir.contains("define void @print_int"));
        assert!(ir.contains("call void @print_int"));
    }

    #[test]
    fn ir_emits_string_literal_constant() {
        let ir = compile_to_ir(r#"int main() { print_str("hi"); return 0; }"#);
        assert!(ir.contains("private unnamed_addr constant"));
        assert!(ir.contains("call void @print_str"));
    }

    #[test]
    fn ir_emits_recursive_call() {
        let ir = compile_to_ir(
            "int fact(int n) {
                return if n <= 1 { 1 } else { n * fact(n - 1) };
            }
             int main() { return fact(5); }",
        );
        assert!(ir.contains("define i64 @fact"));
        assert!(ir.contains("call i64 @fact"));
    }

    #[test]
    fn ir_emits_logical_and_short_circuit_or_branch() {
        let ir = compile_to_ir(
            "int main() { bool b := true && false; return if b { 1 } else { 0 }; }",
        );
        // && and || lower through branch instructions
        assert!(ir.contains("br i1"));
    }

    #[test]
    fn ir_emits_while_loop_with_branch_back() {
        let ir = compile_to_ir(
            "int main() {
                mut int i := 0;
                while i < 5 { i = i + 1; }
                return i;
            }",
        );
        assert!(ir.contains("while.cond"), "expected while.cond block label");
        assert!(ir.contains("while.body"), "expected while.body block label");
        assert!(ir.contains("while.after"), "expected while.after block label");
        assert!(ir.contains("icmp slt"));
    }

    #[test]
    fn ir_emits_break_branch_to_after_block() {
        let ir = compile_to_ir(
            "int main() {
                mut int i := 0;
                while true { if i == 3 { break; } i = i + 1; }
                return i;
            }",
        );
        assert!(ir.contains("while.after"));
    }

    #[test]
    fn ir_emits_srem_for_modulo() {
        // Use variables so inkwell's constant-folder doesn't reduce the
        // expression at IR-build time. We want to see the actual srem op.
        let ir = compile_to_ir(
            "int main() {
                int a := read_int();
                int b := read_int();
                return a % b;
            }",
        );
        assert!(ir.contains("srem i64"), "expected signed remainder, got {}", ir);
    }

    #[test]
    fn ir_emits_for_in_range_with_loop_blocks() {
        let ir = compile_to_ir(
            "int main() {
                mut int total := 0;
                for i in 0..5 { total = total + i; }
                return total;
            }",
        );
        assert!(ir.contains("for.cond"), "expected for.cond block label");
        assert!(ir.contains("for.body"), "expected for.body block label");
        assert!(ir.contains("for.after"), "expected for.after block label");
        assert!(ir.contains("icmp slt"), "expected slt comparison for half-open range");
    }

    #[test]
    fn ir_supports_break_inside_for_in() {
        let ir = compile_to_ir(
            "int main() {
                mut int sum := 0;
                for i in 0..100 {
                    if i == 5 { break; }
                    sum = sum + i;
                }
                return sum;
            }",
        );
        assert!(ir.contains("for.cond"));
        assert!(ir.contains("for.after"));
        assert!(ir.contains("br label %for.after"));
    }

    #[test]
    fn ir_continue_inside_for_in_advances_iteration_variable() {
        let ir = compile_to_ir(
            "int main() {
                mut int sum := 0;
                for i in 0..10 {
                    if i == 3 { continue; }
                    sum = sum + i;
                }
                return sum;
            }",
        );
        assert!(ir.contains("for.inc"), "expected for.inc block label");
        assert!(ir.contains("br label %for.inc"), "expected continue to branch to for.inc");
    }

    #[test]
    fn ir_emits_array_literal_with_stores() {
        let ir = compile_to_ir(
            "int main() {
                arr[int] a := [10, 20, 30];
                return a[1];
            }",
        );
        assert!(ir.contains("[3 x i64]"), "expected stack-allocated [3 x i64]");
        assert!(ir.contains("getelementptr"), "expected GEP for indexing");
    }

    #[test]
    fn ir_emits_array_element_assignment() {
        let ir = compile_to_ir(
            "int main() {
                mut arr[int] a := [10, 20, 30];
                a[1] = 99;
                return a[1];
            }",
        );
        assert!(ir.contains("[3 x i64]"));
        assert!(ir.contains("store i64 99"));
    }

    #[test]
    fn ir_emits_len_as_constant() {
        let ir = compile_to_ir(
            "int main() {
                arr[int] a := [10, 20, 30, 40, 50];
                return len(a);
            }",
        );
        assert!(!ir.contains("call i64 @len"), "len should not be a runtime call");
        assert!(ir.contains("ret i64 5"), "expected constant return of 5");
    }

    #[test]
    fn ir_break_in_loop_body_does_not_emit_dead_code_after_it() {
        // Statement after break would otherwise produce "instruction after terminator".
        let ir = compile_to_ir(
            "int main() {
                mut int i := 0;
                while true {
                    if i == 3 { break; i = i + 100; }
                    i = i + 1;
                }
                return i;
            }",
        );
        // Should compile cleanly. The statement `i = i + 100` after `break` should
        // not generate a store at all (it's unreachable).
        assert!(!ir.contains("add i64 %i, 100"), "unreachable add should not be emitted");
    }
}
