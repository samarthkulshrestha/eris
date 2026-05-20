use eris_compiler::cookbook::{load_all_entries, CookbookEntry};
use std::path::PathBuf;

/// Resolve the workspace root from this crate's CARGO_MANIFEST_DIR.
/// `crates/eris-mcp/` → workspace root.
pub fn workspace_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent")
        .parent()
        .expect("CARGO_MANIFEST_DIR/.. has no parent")
        .to_path_buf()
}

pub fn load_language_card() -> Result<String, std::io::Error> {
    let path = workspace_root().join("schemas/language-card.json");
    std::fs::read_to_string(&path)
}

pub fn load_cookbook() -> Result<Vec<CookbookEntry>, Box<dyn std::error::Error>> {
    let path = workspace_root().join("cookbook");
    Ok(load_all_entries(&path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_root_contains_cookbook() {
        let root = workspace_root();
        assert!(
            root.join("cookbook").is_dir(),
            "expected cookbook/ dir under workspace root, got {:?}",
            root
        );
    }

    #[test]
    fn loads_language_card() {
        let card = load_language_card().expect("load language card");
        assert!(
            card.contains("\"keywords\""),
            "expected card JSON to include keywords field"
        );
    }

    #[test]
    fn loads_cookbook() {
        let entries = load_cookbook().expect("load cookbook");
        assert!(
            entries.len() >= 19,
            "expected at least 19 entries (14 positive + 5 negative), got {}",
            entries.len()
        );
    }
}
