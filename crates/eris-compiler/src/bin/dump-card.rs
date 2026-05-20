use eris_compiler::diagnostics::language_card::build_language_card;

fn main() {
    let card = build_language_card();
    let json = serde_json::to_string_pretty(&card).unwrap();
    println!("{}", json);
}
