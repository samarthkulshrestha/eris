use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Parsed metadata + source for one cookbook entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CookbookEntry {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_error: Option<String>,
    pub stdin: String,
    pub tags: Vec<String>,
    pub file: String,
    pub source: String,
}

#[derive(Debug)]
pub enum CookbookError {
    Io { path: PathBuf, error: std::io::Error },
    Parse { path: PathBuf, line: usize, message: String },
}

impl std::fmt::Display for CookbookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CookbookError::Io { path, error } => {
                write!(f, "io error reading {}: {}", path.display(), error)
            }
            CookbookError::Parse { path, line, message } => {
                write!(f, "{}:{}: {}", path.display(), line, message)
            }
        }
    }
}

impl std::error::Error for CookbookError {}

fn parse_err(path: &Path, line: usize, msg: &str) -> CookbookError {
    CookbookError::Parse {
        path: path.to_path_buf(),
        line,
        message: msg.to_string(),
    }
}

/// Parse the doc-comment header from a cookbook entry's source.
///
/// Header grammar:
///   - Line 1: bare `---`.
///   - Subsequent lines: `--- key: value` OR `--- key: |` followed by
///     one or more `---   content` block-scalar lines (2-space indent).
///   - Closing: bare `---` line.
///
/// Tags use inline-array syntax: `--- tags: [a, b, c]`.
pub fn parse_entry(path: &Path, source: &str) -> Result<CookbookEntry, CookbookError> {
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| parse_err(path, 0, "filename has no stem"))?
        .to_string();
    let file = path
        .to_str()
        .ok_or_else(|| parse_err(path, 0, "non-UTF8 path"))?
        .to_string();

    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return Err(parse_err(path, 1, "empty file"));
    }
    if lines[0].trim() != "---" {
        return Err(parse_err(path, 1, "file must start with a `---` line"));
    }

    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut expected_output: Option<String> = None;
    let mut expected_error: Option<String> = None;
    let mut stdin: Option<String> = None;
    let mut tags: Option<Vec<String>> = None;
    let mut saw_closer = false;

    let mut i = 1;
    while i < lines.len() {
        let line_no = i + 1;
        let raw = lines[i];

        if raw.trim() == "---" {
            saw_closer = true;
            break;
        }

        let body = raw
            .strip_prefix("--- ")
            .ok_or_else(|| parse_err(path, line_no, "header line must start with `--- `"))?;

        let (key, value) = body
            .split_once(':')
            .ok_or_else(|| parse_err(path, line_no, "header line must be `key: value`"))?;
        let key = key.trim();
        let value = value.trim_start_matches(' ');

        let dup = match key {
            "title" => title.is_some(),
            "description" => description.is_some(),
            "expected_output" => expected_output.is_some(),
            "expected_error" => expected_error.is_some(),
            "stdin" => stdin.is_some(),
            "tags" => tags.is_some(),
            _ => false,
        };
        if dup {
            return Err(parse_err(path, line_no, &format!("duplicate key `{}`", key)));
        }

        if value == "|" {
            // Validate key supports block scalars before consuming content.
            match key {
                "expected_output" | "stdin" => {}
                "title" | "description" | "expected_error" => {
                    return Err(parse_err(
                        path,
                        line_no,
                        &format!("`{}` does not support block scalars", key),
                    ));
                }
                _ => {
                    return Err(parse_err(
                        path,
                        line_no,
                        &format!("unknown key `{}`", key),
                    ));
                }
            }

            // Block scalar — consume subsequent `---   content` lines.
            // Exit conditions: bare `---`, line not starting with `--- `,
            // or `--- `-prefixed line whose body lacks the 2-space indent
            // (that's the next metadata key, not block-scalar content).
            let mut content = String::new();
            i += 1;
            while i < lines.len() {
                let cl = lines[i];
                if cl.trim() == "---" {
                    break;
                }
                let cl_body = match cl.strip_prefix("--- ") {
                    Some(b) => b,
                    None => break,
                };
                let stripped = match cl_body.strip_prefix("  ") {
                    Some(s) => s,
                    None => break, // not block-scalar content; let outer loop process it
                };
                content.push_str(stripped);
                content.push('\n');
                i += 1;
            }
            if content.is_empty() {
                return Err(parse_err(
                    path,
                    line_no,
                    &format!("`{}` block scalar has no content lines", key),
                ));
            }
            match key {
                "expected_output" => expected_output = Some(content),
                "stdin" => stdin = Some(content),
                _ => unreachable!(),
            }
            // `i` already points at the terminator line; outer loop re-examines it.
            continue;
        }

        if value.starts_with('[') && value.ends_with(']') {
            if key != "tags" {
                return Err(parse_err(
                    path,
                    line_no,
                    &format!("`{}` does not accept array values", key),
                ));
            }
            let inner = &value[1..value.len() - 1];
            let parsed: Vec<String> = inner
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            tags = Some(parsed);
            i += 1;
            continue;
        }

        match key {
            "title" => title = Some(value.to_string()),
            "description" => description = Some(value.to_string()),
            "expected_output" => expected_output = Some(format!("{}\n", value)),
            "expected_error" => expected_error = Some(value.to_string()),
            "stdin" => stdin = Some(format!("{}\n", value)),
            "tags" => {
                return Err(parse_err(
                    path,
                    line_no,
                    "`tags` must be an inline array `[a, b, c]`",
                ));
            }
            _ => {
                return Err(parse_err(path, line_no, &format!("unknown key `{}`", key)));
            }
        }
        i += 1;
    }

    if !saw_closer {
        return Err(parse_err(path, lines.len(), "header missing closing `---` line"));
    }

    let title = title.ok_or_else(|| parse_err(path, 1, "missing required key `title`"))?;
    let description = description
        .ok_or_else(|| parse_err(path, 1, "missing required key `description`"))?;

    match (&expected_output, &expected_error) {
        (Some(_), Some(_)) => {
            return Err(parse_err(
                path,
                1,
                "exactly one of `expected_output` or `expected_error` must be set, not both",
            ));
        }
        (None, None) => {
            return Err(parse_err(
                path,
                1,
                "exactly one of `expected_output` or `expected_error` must be set",
            ));
        }
        _ => {}
    }

    Ok(CookbookEntry {
        id,
        title,
        description,
        expected_output,
        expected_error,
        stdin: stdin.unwrap_or_default(),
        tags: tags.unwrap_or_default(),
        file,
        source: source.to_string(),
    })
}

/// Load every `.eris` file in `cookbook_dir` (non-recursive), skipping files
/// whose name begins with `_`. Returns entries sorted by id.
pub fn load_all_entries(cookbook_dir: &Path) -> Result<Vec<CookbookEntry>, CookbookError> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(cookbook_dir)
        .map_err(|e| CookbookError::Io {
            path: cookbook_dir.to_path_buf(),
            error: e,
        })?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("eris"))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|n| !n.starts_with('_'))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();

    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let source = std::fs::read_to_string(&path).map_err(|e| CookbookError::Io {
            path: path.clone(),
            error: e,
        })?;
        // Make the stored `file` field relative to repo root if possible.
        let rel = path
            .strip_prefix(cookbook_dir.parent().unwrap_or(Path::new("")))
            .unwrap_or(&path)
            .to_path_buf();
        entries.push(parse_entry(&rel, &source)?);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_minimal_header() {
        let source = "\
---
--- title: Recursive factorial
--- description: Recursion with if/else as the base case.
--- expected_output: |
---   720
---

int main() { return 0; }
";
        let path = PathBuf::from("cookbook/factorial.eris");
        let entry = parse_entry(&path, source).expect("parse failed");
        assert_eq!(entry.id, "factorial");
        assert_eq!(entry.title, "Recursive factorial");
        assert_eq!(entry.description, "Recursion with if/else as the base case.");
        assert_eq!(entry.expected_output, Some("720\n".to_string()));
        assert_eq!(entry.expected_error, None);
        assert_eq!(entry.stdin, "");
        assert_eq!(entry.tags, Vec::<String>::new());
        assert!(entry.source.contains("int main() { return 0; }"));
    }

    #[test]
    fn parses_stdin_and_tags() {
        let source = "\
---
--- title: Echo
--- description: Reads an int and prints it.
--- stdin: |
---   42
--- expected_output: |
---   42
--- tags: [io, read-int]
---

int main() { return 0; }
";
        let path = PathBuf::from("cookbook/read_int_echo.eris");
        let entry = parse_entry(&path, source).expect("parse failed");
        assert_eq!(entry.stdin, "42\n");
        assert_eq!(entry.expected_output, Some("42\n".to_string()));
        assert_eq!(entry.expected_error, None);
        assert_eq!(entry.tags, vec!["io".to_string(), "read-int".to_string()]);
    }

    #[test]
    fn rejects_missing_required_key() {
        let source = "\
---
--- title: Has title
--- expected_output: |
---   x
---
";
        let path = PathBuf::from("cookbook/x.eris");
        let err = parse_entry(&path, source).expect_err("should fail");
        let msg = format!("{}", err);
        assert!(msg.contains("description"), "got: {}", msg);
    }

    #[test]
    fn rejects_duplicate_key() {
        let source = "\
---
--- title: One
--- title: Two
--- description: x
--- expected_output: |
---   y
---
";
        let path = PathBuf::from("cookbook/x.eris");
        let err = parse_entry(&path, source).expect_err("should fail");
        let msg = format!("{}", err);
        assert!(msg.contains("duplicate"), "got: {}", msg);
    }

    #[test]
    fn rejects_missing_opening_marker() {
        let source = "int main() { return 0; }\n";
        let path = PathBuf::from("cookbook/x.eris");
        let err = parse_entry(&path, source).expect_err("should fail");
        let msg = format!("{}", err);
        assert!(msg.contains("must start with a `---` line"), "got: {}", msg);
    }

    #[test]
    fn block_scalar_strips_two_space_indent() {
        let source = "\
---
--- title: Multiline
--- description: ok
--- expected_output: |
---   line one
---   line two
---
";
        let path = PathBuf::from("cookbook/x.eris");
        let entry = parse_entry(&path, source).expect("parse failed");
        assert_eq!(entry.expected_output, Some("line one\nline two\n".to_string()));
        assert_eq!(entry.expected_error, None);
    }

    #[test]
    fn load_all_entries_returns_sorted_by_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("b_one.eris"),
            "---\n--- title: B\n--- description: B\n--- expected_output: |\n---   B\n---\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("a_one.eris"),
            "---\n--- title: A\n--- description: A\n--- expected_output: |\n---   A\n---\n",
        )
        .unwrap();
        // Skipped:
        std::fs::write(
            dir.path().join("_skip.eris"),
            "---\n--- title: skip\n--- description: skip\n--- expected_output: |\n---   X\n---\n",
        )
        .unwrap();
        let entries = load_all_entries(dir.path()).expect("load");
        let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["a_one", "b_one"]);
    }

    #[test]
    fn rejects_missing_closing_marker() {
        let source = "---\n--- title: X\n--- description: Y\n--- expected_output: |\n---   Z\n";
        let path = PathBuf::from("cookbook/x.eris");
        let err = parse_entry(&path, source).expect_err("should fail");
        assert!(
            format!("{}", err).contains("missing closing"),
            "got: {}",
            err
        );
    }

    #[test]
    fn block_scalar_on_title_reports_unsupported_key() {
        let source = "\
---
--- title: |
---   x
--- description: D
--- expected_output: |
---   y
---
";
        let path = PathBuf::from("cookbook/x.eris");
        let err = parse_entry(&path, source).expect_err("should fail");
        let msg = format!("{}", err);
        assert!(
            msg.contains("does not support block scalars"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn parses_positive_entry_with_expected_output() {
        let source = "\
---
--- title: Positive
--- description: P
--- expected_output: |
---   ok
---
";
        let path = PathBuf::from("cookbook/p.eris");
        let entry = parse_entry(&path, source).expect("parse failed");
        assert_eq!(entry.expected_output, Some("ok\n".to_string()));
        assert_eq!(entry.expected_error, None);
    }

    #[test]
    fn parses_negative_entry_with_expected_error() {
        let source = "\
---
--- title: Negative
--- description: N
--- expected_error: E0203
--- tags: [E0203]
---

int main() { int x := 'a'; return 0; }
";
        let path = PathBuf::from("cookbook/neg.eris");
        let entry = parse_entry(&path, source).expect("parse failed");
        assert_eq!(entry.expected_output, None);
        assert_eq!(entry.expected_error, Some("E0203".to_string()));
        assert_eq!(entry.tags, vec!["E0203".to_string()]);
    }

    #[test]
    fn rejects_both_expected_output_and_expected_error() {
        let source = "\
---
--- title: Bad
--- description: B
--- expected_output: |
---   ok
--- expected_error: E0203
---
";
        let path = PathBuf::from("cookbook/bad.eris");
        let err = parse_entry(&path, source).expect_err("should fail");
        let msg = format!("{}", err);
        assert!(msg.contains("exactly one"), "got: {}", msg);
    }

    #[test]
    fn rejects_neither_expected_output_nor_expected_error() {
        let source = "\
---
--- title: Bad
--- description: B
---
";
        let path = PathBuf::from("cookbook/bad.eris");
        let err = parse_entry(&path, source).expect_err("should fail");
        let msg = format!("{}", err);
        assert!(msg.contains("exactly one"), "got: {}", msg);
    }
}
