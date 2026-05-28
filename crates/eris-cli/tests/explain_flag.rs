use std::process::{Command, Stdio};

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_eris"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("spawn eris")
}

#[test]
fn explain_known_code_prints_name() {
    let output = run(&["--explain", "E0203"]);
    assert!(
        output.status.success(),
        "expected exit 0, got {:?}; stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("E0203"),
        "stdout missing code, got: {}",
        stdout
    );
    assert!(
        stdout.contains("TypeMismatch"),
        "stdout missing name, got first 200 chars: {}",
        &stdout[..stdout.len().min(200)]
    );
}

#[test]
fn explain_known_code_includes_cookbook_example() {
    let output = run(&["--explain", "E0203"]);
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("negative_E0203_type_mismatch"),
        "stdout missing cookbook entry id, got: {}",
        stdout
    );
}

#[test]
fn explain_unknown_code_exits_nonzero() {
    let output = run(&["--explain", "E9999"]);
    assert!(
        !output.status.success(),
        "expected nonzero exit, got {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown error code"),
        "stderr missing unknown-code message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("valid codes"),
        "stderr missing valid-codes listing, got: {}",
        stderr
    );
    assert!(
        stderr.contains("E0203"),
        "stderr missing at least one valid code in the listing"
    );
}

#[test]
fn explain_with_source_file_rejected() {
    let output = run(&["--explain", "E0203", "some_file.eris"]);
    assert!(!output.status.success(), "expected nonzero exit");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be combined with a source file"),
        "stderr missing mutex message, got: {}",
        stderr
    );
}

#[test]
fn explain_json_format() {
    let output = run(&["--explain", "E0203", "--format", "json"]);
    assert!(
        output.status.success(),
        "expected exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("parse JSON output");
    assert_eq!(parsed["code"], "E0203");
    assert_eq!(parsed["name"], "TypeMismatch");
    let entries = parsed["cookbook_entries"]
        .as_array()
        .expect("cookbook_entries array");
    assert!(!entries.is_empty(), "expected at least one cookbook entry");
}

#[test]
fn explain_with_emit_ir_rejected() {
    let output = run(&["--emit-ir", "--explain", "E0203"]);
    assert!(
        !output.status.success(),
        "expected nonzero exit, got {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be combined with --emit-ir"),
        "stderr missing emit-ir mutex message, got: {}",
        stderr
    );
}

#[test]
fn explain_with_output_dir_rejected() {
    let output = run(&["--explain", "E0203", "-o", "/tmp"]);
    assert!(
        !output.status.success(),
        "expected nonzero exit, got {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be combined with -o"),
        "stderr missing -o mutex message, got: {}",
        stderr
    );
}
