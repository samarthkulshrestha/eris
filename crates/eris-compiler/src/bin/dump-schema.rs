use eris_compiler::diagnostics::json::DiagnosticEnvelope;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(DiagnosticEnvelope);
    let json = serde_json::to_string_pretty(&schema).unwrap();
    println!("{}", json);
}
