use crate::parser::ast::TypeKind;

pub(crate) struct BuiltinFunction {
    pub(crate) name: &'static str,
    pub(crate) parameters: &'static [TypeKind],
    pub(crate) return_type: Option<TypeKind>,
}

pub(crate) static BUILTIN_FUNCTIONS: &[BuiltinFunction] = &[
    BuiltinFunction { name: "print_int",  parameters: &[TypeKind::Int],  return_type: None },
    BuiltinFunction { name: "print_bool", parameters: &[TypeKind::Bool], return_type: None },
    BuiltinFunction { name: "print_char", parameters: &[TypeKind::Char], return_type: None },
    BuiltinFunction { name: "print_str",  parameters: &[TypeKind::Str],  return_type: None },
    BuiltinFunction { name: "println",    parameters: &[],               return_type: None },
    BuiltinFunction { name: "read_int",   parameters: &[],               return_type: Some(TypeKind::Int) },
];
