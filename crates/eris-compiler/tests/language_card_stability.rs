use eris_compiler::diagnostics::language_card::build_language_card;

#[test]
fn language_card_matches_committed_file() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("schemas/language-card.json");
    let committed = std::fs::read_to_string(&path)
        .expect("schemas/language-card.json missing — regenerate with: cargo run -p eris-compiler --bin dump-card > schemas/language-card.json");
    let live = serde_json::to_string_pretty(&build_language_card()).unwrap();
    assert_eq!(
        committed.trim(),
        live.trim(),
        "language-card.json drift detected — regenerate with: cargo run -p eris-compiler --bin dump-card > schemas/language-card.json"
    );
}
