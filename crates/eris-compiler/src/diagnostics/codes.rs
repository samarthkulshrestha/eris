use schemars::JsonSchema;
use serde::Serialize;

/// Stable error codes. Once assigned, NEVER reuse.
/// E0001-E0099: lexer
/// E0100-E0199: parser
/// E0200-E0299: analyzer (semantic)
/// E0300-E0399: codegen
/// E1000+: lints (warnings)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(into = "&'static str")]
pub enum ErrorCode {
    // (JsonSchema is implemented manually below to emit a string enum schema)
    E0001,
    E0002,
    E0101,
    E0102,
    E0201,
    E0202,
    E0203,
    E0204,
    E0205,
    E0206,
    E0207,
    E0208,
    E0209,
    E0219,
    E0220,
    E0221,
    E0222,
    E0223,
    E0224,
    E0225,
    E0226,
}

/// Single source of truth for error code string values.
/// To add a new code: add the variant above AND one entry here — nothing else.
pub const ALL_CODES: &[(ErrorCode, &str)] = &[
    (ErrorCode::E0001, "E0001"),
    (ErrorCode::E0002, "E0002"),
    (ErrorCode::E0101, "E0101"),
    (ErrorCode::E0102, "E0102"),
    (ErrorCode::E0201, "E0201"),
    (ErrorCode::E0202, "E0202"),
    (ErrorCode::E0203, "E0203"),
    (ErrorCode::E0204, "E0204"),
    (ErrorCode::E0205, "E0205"),
    (ErrorCode::E0206, "E0206"),
    (ErrorCode::E0207, "E0207"),
    (ErrorCode::E0208, "E0208"),
    (ErrorCode::E0209, "E0209"),
    (ErrorCode::E0219, "E0219"),
    (ErrorCode::E0220, "E0220"),
    (ErrorCode::E0221, "E0221"),
    (ErrorCode::E0222, "E0222"),
    (ErrorCode::E0223, "E0223"),
    (ErrorCode::E0224, "E0224"),
    (ErrorCode::E0225, "E0225"),
    (ErrorCode::E0226, "E0226"),
];

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        for (code, s) in ALL_CODES {
            if *code == self {
                return s;
            }
        }
        unreachable!("ErrorCode variant missing from ALL_CODES")
    }

    pub fn name(self) -> &'static str {
        match self {
            ErrorCode::E0001 => "InvalidIdentifier",
            ErrorCode::E0002 => "InvalidToken",
            ErrorCode::E0101 => "UnexpectedToken",
            ErrorCode::E0102 => "LexerErrorInParse",
            ErrorCode::E0201 => "UndeclaredVariable",
            ErrorCode::E0202 => "UndeclaredFunction",
            ErrorCode::E0203 => "TypeMismatch",
            ErrorCode::E0204 => "IncorrectArgumentCount",
            ErrorCode::E0205 => "VariableAlreadyDeclared",
            ErrorCode::E0206 => "AssignmentToImmutable",
            ErrorCode::E0207 => "BreakOutsideLoop",
            ErrorCode::E0208 => "ContinueOutsideLoop",
            ErrorCode::E0209 => "MissingReturn",
            ErrorCode::E0219 => "ReturnMissingValue",
            ErrorCode::E0220 => "NonIterableInForLoop",
            ErrorCode::E0221 => "IndexOutOfRange",
            ErrorCode::E0222 => "IndexNotInteger",
            ErrorCode::E0223 => "TakeRefOfNonLvalue",
            ErrorCode::E0224 => "TakeMutRefOfImmutable",
            ErrorCode::E0225 => "DerefOfNonReference",
            ErrorCode::E0226 => "ReturnsLocalReference",
        }
    }

    pub fn fields(self) -> &'static [&'static str] {
        match self {
            ErrorCode::E0001 => &["identifier"],
            ErrorCode::E0002 => &["token"],
            ErrorCode::E0101 => &["expected", "found"],
            ErrorCode::E0102 => &["detail"],
            ErrorCode::E0201 => &["name"],
            ErrorCode::E0202 => &["name"],
            ErrorCode::E0203 => &["expected", "found"],
            ErrorCode::E0204 => &["expected", "found"],
            ErrorCode::E0205 => &["name"],
            ErrorCode::E0206 => &["name"],
            ErrorCode::E0207 => &[],
            ErrorCode::E0208 => &[],
            ErrorCode::E0209 => &["function"],
            ErrorCode::E0219 => &["function"],
            ErrorCode::E0220 => &["found"],
            ErrorCode::E0221 => &["array_size", "index"],
            ErrorCode::E0222 => &["found"],
            ErrorCode::E0223 => &[],
            ErrorCode::E0224 => &["name"],
            ErrorCode::E0225 => &["found"],
            ErrorCode::E0226 => &[],
        }
    }

    pub fn static_hint(self) -> Option<&'static str> {
        match self {
            ErrorCode::E0001 => Some("identifiers must start with a letter or '_'"),
            ErrorCode::E0002 => Some("check for stray characters or unterminated literals"),
            ErrorCode::E0101 => None,
            ErrorCode::E0102 => None,
            ErrorCode::E0201 => Some("make sure the variable is declared before use"),
            ErrorCode::E0202 => Some("functions must be declared before they are called"),
            ErrorCode::E0203 => Some("Eris does not insert implicit conversions; the types must match exactly"),
            ErrorCode::E0204 => None,
            ErrorCode::E0205 => Some("a name can only be bound once in the same scope"),
            ErrorCode::E0206 => Some("declare the variable with 'mut' to allow reassignment"),
            ErrorCode::E0207 => Some("break is only valid inside a loop body"),
            ErrorCode::E0208 => Some("continue is only valid inside a loop body"),
            ErrorCode::E0209 => Some("non-void functions must return on every control-flow path"),
            ErrorCode::E0219 => Some("supply a return value, or change the function's return type"),
            ErrorCode::E0220 => Some("use 'start..end' to iterate an integer range"),
            ErrorCode::E0221 => Some("indices must lie in 0..array_size"),
            ErrorCode::E0222 => Some("convert the index expression to 'int' before indexing"),
            ErrorCode::E0223 => Some("introduce a named variable first, then take a reference to it"),
            ErrorCode::E0224 => Some("declare the variable with 'mut' to allow mutable references"),
            ErrorCode::E0225 => Some("take a reference with '&' or '&mut' before dereferencing"),
            ErrorCode::E0226 => Some("return the value itself, or pass the destination as a '&mut' parameter"),
        }
    }
}

impl From<ErrorCode> for &'static str {
    fn from(c: ErrorCode) -> Self {
        c.as_str()
    }
}

impl JsonSchema for ErrorCode {
    fn schema_name() -> String {
        "ErrorCode".to_string()
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{InstanceType, SchemaObject, StringValidation};
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        };
        schema.enum_values = Some(
            ALL_CODES
                .iter()
                .map(|(_, s)| serde_json::json!(s))
                .collect(),
        );
        schema.string = Some(Box::new(StringValidation {
            pattern: Some(r"^E\d{4}$".to_string()),
            ..Default::default()
        }));
        schemars::schema::Schema::Object(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_error_codes_have_unique_string_repr() {
        let strs: HashSet<&'static str> = ALL_CODES.iter().map(|(_, s)| *s).collect();
        assert_eq!(strs.len(), ALL_CODES.len(), "duplicate code string detected");
    }

    #[test]
    fn error_codes_serialize_as_string() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::E0001).unwrap(),
            "\"E0001\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::E0203).unwrap(),
            "\"E0203\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::E0206).unwrap(),
            "\"E0206\""
        );
    }

    #[test]
    fn all_codes_roundtrip_through_as_str() {
        for (code, expected_str) in ALL_CODES {
            assert_eq!(
                code.as_str(),
                *expected_str,
                "as_str() mismatch for {:?}",
                code
            );
        }
    }

    #[test]
    fn error_codes_have_descriptive_names() {
        assert_eq!(ErrorCode::E0001.name(), "InvalidIdentifier");
        assert_eq!(ErrorCode::E0203.name(), "TypeMismatch");
        assert_eq!(ErrorCode::E0220.name(), "NonIterableInForLoop");
        assert_eq!(ErrorCode::E0226.name(), "ReturnsLocalReference");
    }

    #[test]
    fn error_codes_have_field_lists() {
        assert_eq!(ErrorCode::E0001.fields(), &["identifier"]);
        assert_eq!(ErrorCode::E0203.fields(), &["expected", "found"]);
        assert_eq!(ErrorCode::E0207.fields(), &[] as &[&str]);
        assert_eq!(ErrorCode::E0221.fields(), &["array_size", "index"]);
    }

    #[test]
    fn error_codes_have_static_hints() {
        assert!(ErrorCode::E0206.static_hint().is_some());
        assert!(ErrorCode::E0223.static_hint().is_some());
    }
}
