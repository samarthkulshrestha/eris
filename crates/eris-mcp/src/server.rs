use std::sync::Arc;

use eris_compiler::cookbook::CookbookEntry;
use rmcp::{
    ErrorData,
    handler::server::wrapper::Parameters,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The MCP server. Caches the language card and cookbook at startup.
#[derive(Clone)]
pub struct ErisMcpServer {
    pub card_json: String,
    pub cookbook: Arc<Vec<CookbookEntry>>,
}

impl ErisMcpServer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let card_json = crate::artifacts::load_language_card()?;
        let cookbook = crate::artifacts::load_cookbook()?;
        Ok(Self {
            card_json,
            cookbook: Arc::new(cookbook),
        })
    }
}

impl Default for ErisMcpServer {
    fn default() -> Self {
        Self::new().expect("failed to load eris-mcp artifacts")
    }
}

/// Lightweight index entry returned by `list_cookbook_entries`.
#[derive(Debug, Clone, Serialize)]
pub struct CookbookIndexEntry {
    pub id: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
}

/// Output of `explain_error`: card metadata for the code plus matching cookbook entries.
#[derive(Debug, Serialize)]
pub struct ExplainedError {
    pub code: String,
    pub name: String,
    pub fields: Vec<String>,
    pub hint: Option<String>,
    pub cookbook_entries: Vec<CookbookEntry>,
}

// ── Parameter structs for tools that take input ──────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadCookbookEntryParams {
    /// The id of the cookbook entry to retrieve.
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchCookbookParams {
    /// The tag to filter cookbook entries by (exact match).
    pub tag: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExplainErrorParams {
    /// The error code to explain (e.g. "E0101").
    pub code: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    /// Eris source code to analyze.
    pub source: String,
}

// ── Tool implementations ──────────────────────────────────────────────────────

#[tool_router]
impl ErisMcpServer {
    /// Return the full Eris language card as a JSON string.
    #[tool(description = "Return the full Eris language card as a JSON string.")]
    async fn read_language_card(&self) -> Result<String, ErrorData> {
        Ok(self.card_json.clone())
    }

    /// List all cookbook entries (id, title, description, tags — no full source).
    #[tool(description = "List all cookbook entries (id, title, description, tags — no full source).")]
    async fn list_cookbook_entries(&self) -> Result<String, ErrorData> {
        let index: Vec<CookbookIndexEntry> = self
            .cookbook
            .iter()
            .map(|e| CookbookIndexEntry {
                id: e.id.clone(),
                title: e.title.clone(),
                description: e.description.clone(),
                tags: e.tags.clone(),
            })
            .collect();
        serde_json::to_string_pretty(&index).map_err(|e| {
            ErrorData::internal_error(e.to_string(), None)
        })
    }

    /// Read a specific cookbook entry by id; returns an error if the id is unknown.
    #[tool(description = "Read a specific cookbook entry by id; returns an error if the id is unknown.")]
    async fn read_cookbook_entry(
        &self,
        Parameters(params): Parameters<ReadCookbookEntryParams>,
    ) -> Result<String, ErrorData> {
        let entry = self
            .cookbook
            .iter()
            .find(|e| e.id == params.id)
            .ok_or_else(|| {
                let known: Vec<String> = self.cookbook.iter().map(|e| e.id.clone()).collect();
                ErrorData::invalid_params(
                    format!(
                        "unknown id `{}`; known: {}",
                        params.id,
                        known.join(", ")
                    ),
                    None,
                )
            })?;
        serde_json::to_string_pretty(entry).map_err(|e| {
            ErrorData::internal_error(e.to_string(), None)
        })
    }

    /// Filter cookbook entries by tag (exact match); returns all matching entries.
    #[tool(description = "Filter cookbook entries by tag (exact match); returns all matching entries.")]
    async fn search_cookbook(
        &self,
        Parameters(params): Parameters<SearchCookbookParams>,
    ) -> Result<String, ErrorData> {
        let matches: Vec<&CookbookEntry> = self
            .cookbook
            .iter()
            .filter(|e| e.tags.iter().any(|t| t == &params.tag))
            .collect();
        serde_json::to_string_pretty(&matches).map_err(|e| {
            ErrorData::internal_error(e.to_string(), None)
        })
    }

    /// Explain an error code: returns the card entry for that code plus cookbook entries tagged with it.
    #[tool(description = "Explain an error code: returns the card entry for that code plus cookbook entries tagged with it.")]
    async fn explain_error(
        &self,
        Parameters(params): Parameters<ExplainErrorParams>,
    ) -> Result<String, ErrorData> {
        let card: serde_json::Value =
            serde_json::from_str(&self.card_json).map_err(|e| {
                ErrorData::internal_error(format!("failed to parse language card: {e}"), None)
            })?;

        let entries = card
            .get("error_codes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ErrorData::internal_error("card missing error_codes field", None)
            })?;

        let card_entry = entries
            .iter()
            .find(|e| e.get("code").and_then(|c| c.as_str()) == Some(params.code.as_str()))
            .ok_or_else(|| {
                ErrorData::invalid_params(
                    format!("unknown error code `{}`", params.code),
                    None,
                )
            })?;

        let name = card_entry
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let fields: Vec<String> = card_entry
            .get("fields")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let hint = card_entry
            .get("hint")
            .and_then(|h| h.as_str())
            .map(String::from);

        let cookbook_entries: Vec<CookbookEntry> = self
            .cookbook
            .iter()
            .filter(|e| e.tags.iter().any(|t| t == &params.code))
            .cloned()
            .collect();

        let explained = ExplainedError {
            code: params.code,
            name,
            fields,
            hint,
            cookbook_entries,
        };
        serde_json::to_string_pretty(&explained).map_err(|e| {
            ErrorData::internal_error(e.to_string(), None)
        })
    }

    /// Run the Eris lexer, parser, and analyzer on source code and return diagnostics.
    #[tool(description = "Run the Eris lexer, parser, and analyzer on source code and return diagnostics.")]
    async fn analyze(
        &self,
        Parameters(params): Parameters<AnalyzeParams>,
    ) -> Result<String, ErrorData> {
        let source = params.source;
        let diagnostics = tokio::task::spawn_blocking(move || {
            eris_compiler::analyze_source(&source)
        })
        .await
        .map_err(|e| ErrorData::internal_error(format!("spawn_blocking failed: {e}"), None))?;

        serde_json::to_string_pretty(&diagnostics).map_err(|e| {
            ErrorData::internal_error(e.to_string(), None)
        })
    }
}

#[tool_handler]
impl rmcp::ServerHandler for ErisMcpServer {}
