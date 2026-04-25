use eris_compiler::diagnostics::json::DiagnosticEnvelope;
use schemars::schema_for;

#[test]
fn diagnostic_schema_matches_committed_file() {
    let committed = std::fs::read_to_string("../../schemas/diagnostic-envelope.json")
        .expect("schemas/diagnostic-envelope.json missing — run `cargo run -p eris-compiler --bin dump-schema > schemas/diagnostic-envelope.json` to regenerate");
    let live = serde_json::to_string_pretty(&schema_for!(DiagnosticEnvelope)).unwrap();
    assert_eq!(
        committed.trim(),
        live.trim(),
        "schema drift detected — regenerate with: cargo run -p eris-compiler --bin dump-schema > schemas/diagnostic-envelope.json"
    );
}
