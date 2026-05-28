use crate::cookbook::CookbookEntry;
use serde::Serialize;

/// Card metadata for an error code plus any cookbook entries tagged with that code.
#[derive(Debug, Clone, Serialize)]
pub struct ExplainedError {
    pub code: String,
    pub name: String,
    pub fields: Vec<String>,
    pub hint: Option<String>,
    pub cookbook_entries: Vec<CookbookEntry>,
}

#[derive(Debug)]
pub enum ExplainError {
    UnknownCode {
        code: String,
        known_codes: Vec<String>,
    },
    MalformedCard(String),
}

impl std::fmt::Display for ExplainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExplainError::UnknownCode { code, known_codes } => write!(
                f,
                "unknown error code `{}`; valid codes: {}",
                code,
                known_codes.join(", ")
            ),
            ExplainError::MalformedCard(msg) => {
                write!(f, "malformed language card: {}", msg)
            }
        }
    }
}

impl std::error::Error for ExplainError {}

/// Look up an error code in the language card and collect cookbook entries
/// whose `tags` array contains the code string.
///
/// Pure: no I/O, no globals. The caller provides the already-loaded card
/// JSON (read from disk or embedded) and the parsed cookbook entries.
pub fn explain_error_lookup(
    card_json: &str,
    cookbook: &[CookbookEntry],
    code: &str,
) -> Result<ExplainedError, ExplainError> {
    let card: serde_json::Value = serde_json::from_str(card_json)
        .map_err(|e| ExplainError::MalformedCard(format!("parse: {}", e)))?;

    let entries = card
        .get("error_codes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            ExplainError::MalformedCard("card missing `error_codes` field".to_string())
        })?;

    let card_entry = entries
        .iter()
        .find(|e| e.get("code").and_then(|c| c.as_str()) == Some(code));

    let card_entry = match card_entry {
        Some(e) => e,
        None => {
            let known_codes: Vec<String> = entries
                .iter()
                .filter_map(|e| e.get("code").and_then(|c| c.as_str()).map(String::from))
                .collect();
            return Err(ExplainError::UnknownCode {
                code: code.to_string(),
                known_codes,
            });
        }
    };

    let name = card_entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let fields: Vec<String> = card_entry
        .get("fields")
        .and_then(|f| f.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let hint = card_entry
        .get("hint")
        .and_then(|h| h.as_str())
        .map(String::from);

    let cookbook_entries: Vec<CookbookEntry> = cookbook
        .iter()
        .filter(|e| e.tags.iter().any(|t| t == code))
        .cloned()
        .collect();

    Ok(ExplainedError {
        code: code.to_string(),
        name,
        fields,
        hint,
        cookbook_entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_card_json() -> &'static str {
        r#"{
            "version": "0.8.0",
            "schema_version": 1,
            "keywords": [],
            "operators": [],
            "types": [],
            "builtins": [],
            "error_codes": [
                {
                    "code": "E0203",
                    "name": "TypeMismatch",
                    "fields": ["expected", "found"],
                    "hint": "types must match exactly"
                },
                {
                    "code": "E0207",
                    "name": "BreakOutsideLoop",
                    "fields": []
                }
            ]
        }"#
    }

    fn make_entry(id: &str, tags: Vec<&str>) -> CookbookEntry {
        CookbookEntry {
            id: id.to_string(),
            title: format!("title for {}", id),
            description: "desc".to_string(),
            expected_output: Some("x\n".to_string()),
            expected_error: None,
            stdin: String::new(),
            tags: tags.into_iter().map(String::from).collect(),
            file: format!("cookbook/{}.eris", id),
            source: "int main() { return 0; }\n".to_string(),
        }
    }

    #[test]
    fn known_code_returns_populated_explanation() {
        let cookbook = vec![make_entry("e1", vec!["E0203"])];
        let explained =
            explain_error_lookup(minimal_card_json(), &cookbook, "E0203").expect("lookup");
        assert_eq!(explained.code, "E0203");
        assert_eq!(explained.name, "TypeMismatch");
        assert_eq!(
            explained.fields,
            vec!["expected".to_string(), "found".to_string()]
        );
        assert_eq!(explained.hint, Some("types must match exactly".to_string()));
        assert_eq!(explained.cookbook_entries.len(), 1);
        assert_eq!(explained.cookbook_entries[0].id, "e1");
    }

    #[test]
    fn unknown_code_returns_known_codes_list() {
        let cookbook: Vec<CookbookEntry> = Vec::new();
        let err = explain_error_lookup(minimal_card_json(), &cookbook, "E9999")
            .expect_err("should fail");
        match err {
            ExplainError::UnknownCode { code, known_codes } => {
                assert_eq!(code, "E9999");
                assert!(known_codes.contains(&"E0203".to_string()));
                assert!(known_codes.contains(&"E0207".to_string()));
            }
            other => panic!("expected UnknownCode, got {:?}", other),
        }
    }

    #[test]
    fn cookbook_filter_excludes_untagged() {
        let cookbook = vec![
            make_entry("with_tag", vec!["E0203", "types"]),
            make_entry("without_tag", vec!["recursion"]),
            make_entry("other_code", vec!["E0207"]),
        ];
        let explained =
            explain_error_lookup(minimal_card_json(), &cookbook, "E0203").expect("lookup");
        assert_eq!(explained.cookbook_entries.len(), 1);
        assert_eq!(explained.cookbook_entries[0].id, "with_tag");
    }

    #[test]
    fn missing_hint_returns_none() {
        let cookbook: Vec<CookbookEntry> = Vec::new();
        let explained =
            explain_error_lookup(minimal_card_json(), &cookbook, "E0207").expect("lookup");
        assert_eq!(explained.code, "E0207");
        assert_eq!(explained.hint, None);
        assert_eq!(explained.fields, Vec::<String>::new());
    }

    #[test]
    fn malformed_card_returns_error() {
        let cookbook: Vec<CookbookEntry> = Vec::new();
        let err = explain_error_lookup("not json", &cookbook, "E0203")
            .expect_err("should fail");
        match err {
            ExplainError::MalformedCard(_) => {}
            other => panic!("expected MalformedCard, got {:?}", other),
        }
    }

    #[test]
    fn card_without_error_codes_field_returns_malformed() {
        let cookbook: Vec<CookbookEntry> = Vec::new();
        let err = explain_error_lookup(r#"{"version": "0.8.0"}"#, &cookbook, "E0203")
            .expect_err("should fail");
        match err {
            ExplainError::MalformedCard(msg) => assert!(msg.contains("error_codes")),
            other => panic!("expected MalformedCard, got {:?}", other),
        }
    }
}
