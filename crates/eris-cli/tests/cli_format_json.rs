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
    assert_eq!(v["status"], "compile_error");
    assert_eq!(v["diagnostics"][0]["code"], "E0201");
    assert_eq!(
        v["diagnostics"][0]["fields"]["name"],
        "undefined_var",
        "name field must be nested under 'fields'"
    );
}

#[test]
fn json_format_success_emits_ok_status_with_empty_diagnostics() {
    // Use examples/01.eris which is known-good. Direct outputs to a tempdir
    // so the produced binary doesn't leak into the test crate's directory.
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let src = workspace_root.join("examples/01.eris");
    let out_dir = tempfile::tempdir().unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_eris"))
        .args([
            "-o",
            out_dir.path().to_str().unwrap(),
            "--format",
            "json",
            src.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(out.status.success(), "compiler should exit zero on success, stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)
        .expect("stdout must be valid JSON");
    assert_eq!(v["schema_version"], "1.0");
    assert_eq!(v["status"], "ok");
    assert_eq!(
        v["diagnostics"].as_array().map(|a| a.len()).unwrap_or(1),
        0,
        "diagnostics must be empty on success"
    );
}

#[test]
fn json_format_nonexistent_file_emits_internal_error() {
    let out = Command::new(env!("CARGO_BIN_EXE_eris"))
        .args(["/nonexistent/path/to/file.eris", "--format", "json"])
        .output()
        .unwrap();

    assert!(!out.status.success(), "compiler should exit non-zero on IO error");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)
        .expect("stdout must be valid JSON");
    assert_eq!(v["schema_version"], "1.0");
    assert_eq!(v["status"], "internal_error");
    assert!(
        v["error_message"].as_str().map(|s| s.starts_with("io:")).unwrap_or(false),
        "error_message must start with 'io:', got: {:?}",
        v["error_message"]
    );
    assert_eq!(
        v["diagnostics"].as_array().map(|a| a.len()).unwrap_or(1),
        0,
        "diagnostics must be empty for internal_error"
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
