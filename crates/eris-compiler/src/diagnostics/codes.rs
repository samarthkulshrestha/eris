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
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::E0001 => "E0001",
            ErrorCode::E0002 => "E0002",
            ErrorCode::E0101 => "E0101",
            ErrorCode::E0102 => "E0102",
            ErrorCode::E0201 => "E0201",
            ErrorCode::E0202 => "E0202",
            ErrorCode::E0203 => "E0203",
            ErrorCode::E0204 => "E0204",
            ErrorCode::E0205 => "E0205",
            ErrorCode::E0206 => "E0206",
        }
    }
}

impl From<ErrorCode> for &'static str {
    fn from(c: ErrorCode) -> Self {
        c.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_error_codes_have_unique_string_repr() {
        let all_codes = [
            ErrorCode::E0001,
            ErrorCode::E0002,
            ErrorCode::E0101,
            ErrorCode::E0102,
            ErrorCode::E0201,
            ErrorCode::E0202,
            ErrorCode::E0203,
            ErrorCode::E0204,
            ErrorCode::E0205,
            ErrorCode::E0206,
        ];
        let strs: HashSet<&'static str> = all_codes.iter().map(|c| c.as_str()).collect();
        assert_eq!(strs.len(), all_codes.len(), "duplicate code string detected");
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
}
