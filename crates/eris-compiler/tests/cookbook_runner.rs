use eris_compiler::analyze_source;
use eris_compiler::cookbook::{load_all_entries, CookbookEntry};
use eris_compiler::{CompileOptions, OutputFormat, process_file};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn every_cookbook_entry_runs_correctly() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let cookbook_dir = workspace_root.join("cookbook");
    let entries = load_all_entries(&cookbook_dir).expect("failed to load cookbook entries");
    assert!(!entries.is_empty(), "cookbook directory is empty");

    let mut failures: Vec<String> = Vec::new();
    for entry in &entries {
        if let Err(msg) = run_entry(entry) {
            failures.push(format!("[{}] {}", entry.id, msg));
        }
    }

    assert!(
        failures.is_empty(),
        "{} cookbook entries failed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

fn run_entry(entry: &CookbookEntry) -> Result<(), String> {
    match (&entry.expected_output, &entry.expected_error) {
        (Some(expected_output), None) => run_positive(entry, expected_output),
        (None, Some(expected_error)) => run_negative(entry, expected_error),
        _ => Err(
            "entry has neither or both of expected_output/expected_error".to_string(),
        ),
    }
}

fn run_positive(entry: &CookbookEntry, expected_output: &str) -> Result<(), String> {
    let tmp = tempfile::tempdir().map_err(|e| format!("tempdir: {}", e))?;
    let src_path = tmp.path().join(format!("{}.eris", entry.id));
    std::fs::write(&src_path, &entry.source)
        .map_err(|e| format!("write source: {}", e))?;

    let opts = CompileOptions {
        emit_ir: false,
        output_dir: Some(tmp.path().to_path_buf()),
        format: OutputFormat::Json,
    };
    process_file(src_path.to_str().unwrap(), opts)
        .map_err(|e| format!("compile failed: {:?}", e))?;

    let binary = tmp.path().join(&entry.id);
    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn binary: {}", e))?;

    if !entry.stdin.is_empty() {
        let mut child_stdin = child.stdin.take().ok_or("failed to capture stdin")?;
        child_stdin
            .write_all(entry.stdin.as_bytes())
            .map_err(|e| format!("write stdin: {}", e))?;
        // dropping closes stdin so read_int sees EOF
    } else {
        // close stdin so read_int doesn't block on a missing input
        drop(child.stdin.take());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait_with_output: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit_code = output.status.code().unwrap_or(-1);

    if exit_code != 0 {
        return Err(format!(
            "exit code {} (expected 0)\nstderr: {}",
            exit_code, stderr
        ));
    }
    if stdout != expected_output {
        return Err(format!(
            "stdout mismatch\nexpected: {:?}\nactual:   {:?}",
            expected_output, stdout
        ));
    }
    Ok(())
}

fn run_negative(entry: &CookbookEntry, expected_error: &str) -> Result<(), String> {
    let diagnostics = analyze_source(&entry.source);
    if diagnostics.is_empty() {
        return Err(format!(
            "expected diagnostic {} but analyzer produced none",
            expected_error
        ));
    }
    if diagnostics.len() != 1 {
        let codes: Vec<&'static str> =
            diagnostics.iter().map(|d| d.code.as_str()).collect();
        return Err(format!(
            "expected exactly 1 diagnostic, got {}: {}",
            diagnostics.len(),
            codes.join(", ")
        ));
    }
    let actual_code = diagnostics[0].code.as_str();
    if actual_code != expected_error {
        return Err(format!(
            "expected diagnostic {}, got {}",
            expected_error, actual_code
        ));
    }
    Ok(())
}
