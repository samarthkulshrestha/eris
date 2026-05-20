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
    if stdout != entry.expected_output {
        return Err(format!(
            "stdout mismatch\nexpected: {:?}\nactual:   {:?}",
            entry.expected_output, stdout
        ));
    }
    Ok(())
}
