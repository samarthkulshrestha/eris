use eris_compiler::lexer::Lexer;
use eris_compiler::parser::Parser;

fn parse_debug(source: &str) -> String {
    let mut lex = Lexer::new(source);
    let toks = lex.tokenize().unwrap();
    let mut p = Parser::new(toks);
    format!("{:?}", p.parse_program().unwrap())
}

#[test]
fn discord_block_parses_as_statement() {
    let ast = parse_debug("int main() { discord { int a := 1; } return 0; }");
    assert!(ast.contains("DiscordBlock"), "got {ast}");
}

#[test]
fn fnord_parses_as_statement() {
    let ast = parse_debug("int main() { fnord; }");
    assert!(ast.contains("Fnord"), "got {ast}");
}
