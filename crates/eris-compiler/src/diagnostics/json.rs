use super::Diagnostic;
use serde::Serialize;

pub const SCHEMA_VERSION: &str = "1.0";

#[derive(Serialize)]
pub struct DiagnosticEnvelope {
    pub schema_version: &'static str,
    pub compiler_version: &'static str,
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticEnvelope {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        DiagnosticEnvelope {
            schema_version: SCHEMA_VERSION,
            compiler_version: env!("CARGO_PKG_VERSION"),
            diagnostics,
        }
    }
}
