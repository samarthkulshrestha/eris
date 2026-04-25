use super::Diagnostic;
use schemars::JsonSchema;
use serde::Serialize;

pub const SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeStatus {
    Ok,
    CompileError,
    InternalError,
}

#[derive(Serialize, JsonSchema)]
pub struct DiagnosticEnvelope {
    pub schema_version: &'static str,
    pub compiler_version: &'static str,
    pub status: EnvelopeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticEnvelope {
    pub fn ok() -> Self {
        DiagnosticEnvelope {
            schema_version: SCHEMA_VERSION,
            compiler_version: env!("CARGO_PKG_VERSION"),
            status: EnvelopeStatus::Ok,
            error_message: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn compile_error(diagnostics: Vec<Diagnostic>) -> Self {
        DiagnosticEnvelope {
            schema_version: SCHEMA_VERSION,
            compiler_version: env!("CARGO_PKG_VERSION"),
            status: EnvelopeStatus::CompileError,
            error_message: None,
            diagnostics,
        }
    }

    pub fn internal_error(message: String) -> Self {
        DiagnosticEnvelope {
            schema_version: SCHEMA_VERSION,
            compiler_version: env!("CARGO_PKG_VERSION"),
            status: EnvelopeStatus::InternalError,
            error_message: Some(message),
            diagnostics: Vec::new(),
        }
    }
}
