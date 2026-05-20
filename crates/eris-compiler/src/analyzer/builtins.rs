use crate::parser::ast::TypeKind;

pub struct BuiltinFunction {
    pub name: &'static str,
    pub parameters: &'static [TypeKind],
    pub return_type: Option<TypeKind>,
}

pub static BUILTIN_FUNCTIONS: &[BuiltinFunction] = &[
    BuiltinFunction { name: "print_int",  parameters: &[TypeKind::Int],  return_type: None },
    BuiltinFunction { name: "print_bool", parameters: &[TypeKind::Bool], return_type: None },
    BuiltinFunction { name: "print_char", parameters: &[TypeKind::Char], return_type: None },
    BuiltinFunction { name: "print_str",  parameters: &[TypeKind::Str],  return_type: None },
    BuiltinFunction { name: "println",    parameters: &[],               return_type: None },
    BuiltinFunction { name: "read_int",   parameters: &[],               return_type: Some(TypeKind::Int) },
];
