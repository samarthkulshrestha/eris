use eris_mcp::artifacts;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load artifacts at startup; fail fast if missing.
    let _card = artifacts::load_language_card()?;
    let _cookbook = artifacts::load_cookbook()?;

    eprintln!("eris-mcp: artifact load successful");
    // Phase G replaces this stub with the actual MCP server.
    Ok(())
}
