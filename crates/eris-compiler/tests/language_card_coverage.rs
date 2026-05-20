use eris_compiler::diagnostics::language_card::build_language_card;

#[test]
fn every_card_entry_appears_in_language_md() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("LANGUAGE.md");
    let md = std::fs::read_to_string(&path)
        .expect("LANGUAGE.md missing — see docs/superpowers/plans/2026-05-20-eris-language-card.md");
    let card = build_language_card();
    let mut missing: Vec<String> = Vec::new();

    for kw in &card.keywords {
        if !md.contains(kw.as_str()) {
            missing.push(format!("keyword: {}", kw));
        }
    }
    for op in &card.operators {
        if !md.contains(op.token.as_str()) {
            missing.push(format!("operator: {}", op.token));
        }
    }
    for t in &card.types {
        if !md.contains(t.name.as_str()) {
            missing.push(format!("type: {}", t.name));
        }
    }
    for b in &card.builtins {
        if !md.contains(b.name.as_str()) {
            missing.push(format!("builtin: {}", b.name));
        }
    }
    for e in &card.error_codes {
        if !md.contains(e.code.as_str()) {
            missing.push(format!("error code: {}", e.code));
        }
    }

    assert!(
        missing.is_empty(),
        "LANGUAGE.md is missing {} card entries:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
}
