use eris_compiler::cookbook::load_all_entries;
use std::path::Path;

#[test]
fn cookbook_json_matches_committed_file() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let committed_path = workspace_root.join("schemas/cookbook.json");
    let cookbook_dir = workspace_root.join("cookbook");

    let committed = std::fs::read_to_string(&committed_path)
        .expect("schemas/cookbook.json missing — regenerate with: cargo run -p eris-compiler --bin dump-cookbook > schemas/cookbook.json");
    let entries = load_all_entries(&cookbook_dir).expect("failed to load cookbook entries");
    let live = serde_json::to_string_pretty(&entries).expect("serialize");

    assert_eq!(
        committed.trim(),
        live.trim(),
        "cookbook.json drift detected — regenerate with: cargo run -p eris-compiler --bin dump-cookbook > schemas/cookbook.json"
    );
}
