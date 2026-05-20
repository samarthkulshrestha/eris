use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// Spawn the eris-mcp binary and return its handle + stdio.
fn spawn_server() -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let binary = env!("CARGO_BIN_EXE_eris-mcp");
    let mut child = Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn eris-mcp");
    let stdin = child.stdin.take().expect("stdin");
    let stdout = BufReader::new(child.stdout.take().expect("stdout"));
    (child, stdin, stdout)
}

fn send_request(stdin: &mut ChildStdin, method: &str, params: serde_json::Value, id: u64) {
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    writeln!(stdin, "{}", req).expect("write request");
    stdin.flush().expect("flush");
}

fn send_notification(stdin: &mut ChildStdin, method: &str, params: serde_json::Value) {
    let notif = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    writeln!(stdin, "{}", notif).expect("write notification");
    stdin.flush().expect("flush");
}

fn read_response(stdout: &mut BufReader<ChildStdout>) -> serde_json::Value {
    let mut line = String::new();
    stdout.read_line(&mut line).expect("read response");
    serde_json::from_str(&line).expect("parse response")
}

/// Complete the initialize handshake. Returns when the server is ready for tools/call.
fn initialize_server(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) {
    send_request(
        stdin,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "eris-test", "version": "0.0.1" },
        }),
        1,
    );
    let _init_response = read_response(stdout);
    // Per the MCP spec, the client must send `notifications/initialized` after
    // receiving the initialize response.
    send_notification(stdin, "notifications/initialized", serde_json::json!({}));
}

fn shutdown(child: &mut Child, stdin: ChildStdin) {
    drop(stdin);
    let _ = child.wait();
}

#[test]
fn server_handles_initialize() {
    let (mut child, mut stdin, mut stdout) = spawn_server();

    send_request(
        &mut stdin,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "eris-test", "version": "0.0.1" },
        }),
        1,
    );

    let response = read_response(&mut stdout);
    assert!(
        response.get("result").is_some(),
        "expected result, got {}",
        response
    );
    assert!(
        response["result"].get("capabilities").is_some(),
        "expected capabilities in result, got {}",
        response
    );

    shutdown(&mut child, stdin);
}

#[test]
fn server_lists_six_tools() {
    let (mut child, mut stdin, mut stdout) = spawn_server();
    initialize_server(&mut stdin, &mut stdout);

    send_request(&mut stdin, "tools/list", serde_json::json!({}), 2);
    let response = read_response(&mut stdout);
    let tools = response["result"]["tools"]
        .as_array()
        .expect("tools array");
    assert_eq!(
        tools.len(),
        6,
        "expected 6 tools, got {}: {:?}",
        tools.len(),
        tools
    );

    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for expected in &[
        "read_language_card",
        "list_cookbook_entries",
        "read_cookbook_entry",
        "search_cookbook",
        "explain_error",
        "analyze",
    ] {
        assert!(
            tool_names.contains(expected),
            "missing tool `{}`, got: {:?}",
            expected,
            tool_names
        );
    }

    shutdown(&mut child, stdin);
}

#[test]
fn read_language_card_returns_card_json() {
    let (mut child, mut stdin, mut stdout) = spawn_server();
    initialize_server(&mut stdin, &mut stdout);

    send_request(
        &mut stdin,
        "tools/call",
        serde_json::json!({
            "name": "read_language_card",
            "arguments": {},
        }),
        2,
    );
    let response = read_response(&mut stdout);
    let content = response["result"]["content"][0]["text"]
        .as_str()
        .expect("expected text content");
    assert!(
        content.contains("\"keywords\""),
        "expected keywords field in card, got first 200 chars: {:?}",
        &content[..content.len().min(200)]
    );
    assert!(
        content.contains("\"error_codes\""),
        "expected error_codes field in card"
    );

    shutdown(&mut child, stdin);
}

#[test]
fn explain_error_returns_card_entry_and_cookbook_matches() {
    let (mut child, mut stdin, mut stdout) = spawn_server();
    initialize_server(&mut stdin, &mut stdout);

    send_request(
        &mut stdin,
        "tools/call",
        serde_json::json!({
            "name": "explain_error",
            "arguments": { "code": "E0203" },
        }),
        2,
    );
    let response = read_response(&mut stdout);
    let content = response["result"]["content"][0]["text"]
        .as_str()
        .expect("expected text content");
    let explained: serde_json::Value =
        serde_json::from_str(content).expect("parse content");
    assert_eq!(explained["code"], "E0203");
    assert_eq!(explained["name"], "TypeMismatch");
    let entries = explained["cookbook_entries"]
        .as_array()
        .expect("cookbook_entries array");
    assert!(
        !entries.is_empty(),
        "expected at least one cookbook entry tagged with E0203 (negative_E0203_type_mismatch exists)"
    );

    shutdown(&mut child, stdin);
}

#[test]
fn analyze_returns_diagnostic_for_bad_source() {
    let (mut child, mut stdin, mut stdout) = spawn_server();
    initialize_server(&mut stdin, &mut stdout);

    send_request(
        &mut stdin,
        "tools/call",
        serde_json::json!({
            "name": "analyze",
            "arguments": {
                "source": "int main() { int x := 'a'; return 0; }"
            },
        }),
        2,
    );
    let response = read_response(&mut stdout);
    let content = response["result"]["content"][0]["text"]
        .as_str()
        .expect("expected text content");
    let diagnostics: serde_json::Value =
        serde_json::from_str(content).expect("parse content");
    let arr = diagnostics.as_array().expect("array");
    assert!(!arr.is_empty(), "expected at least one diagnostic");
    assert!(
        arr.iter().any(|d| d["code"] == "E0203"),
        "expected E0203 in diagnostics, got: {:?}",
        arr
    );

    shutdown(&mut child, stdin);
}
