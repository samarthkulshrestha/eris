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
}

/// Single source of truth for error code string values.
/// To add a new code: add the variant above AND one entry here — nothing else.
const ALL_CODES: &[(ErrorCode, &str)] = &[
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
}
