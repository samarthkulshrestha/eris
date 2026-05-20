use eris_compiler::cookbook::load_all_entries;
use std::path::Path;

fn main() {
    let dir = Path::new("cookbook");
    let entries = load_all_entries(dir).expect("failed to load cookbook entries");
    let json = serde_json::to_string_pretty(&entries).expect("failed to serialize");
    println!("{}", json);
}
