use std::process::Command;

#[test]
fn json_format_emits_envelope_for_undeclared_variable() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("bad.eris");
    // A valid function that references an undeclared variable
    std::fs::write(&src, "int main() { return undefined_var; }").unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_eris"))
        .args([src.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();

    assert!(!out.status.success(), "compiler should exit non-zero on error");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)
        .expect("stdout must be valid JSON");
    assert_eq!(v["schema_version"], "1.0");
    assert_eq!(v["diagnostics"][0]["code"], "E0201");
    assert_eq!(
        v["diagnostics"][0]["name"],
        "undefined_var",
        "name field must contain the undeclared variable name"
    );
}

#[test]
fn default_text_format_prints_error_to_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("bad.eris");
    std::fs::write(&src, "int main() { return undefined_var; }").unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_eris"))
        .arg(src.to_str().unwrap())
        .output()
        .unwrap();

    assert!(!out.status.success(), "compiler should exit non-zero on error");
    // stdout must be empty (diagnostics go to stderr in text mode)
    assert!(
        out.stdout.is_empty(),
        "stdout must be empty in text mode, got: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    // stderr must contain the error
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("undefined_var") || stderr.contains("undeclared"),
        "stderr must contain error info, got: {}",
        stderr
    );
}
