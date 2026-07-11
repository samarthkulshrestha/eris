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

use eris_compiler::analyzer::{AnalyzeError, AnalyzeErrorKind, Analyzer};

fn analyze(source: &str) -> Result<(), Vec<AnalyzeError>> {
    let mut lex = Lexer::new(source);
    let toks = lex.tokenize().unwrap();
    let mut p = Parser::new(toks);
    let program = p.parse_program().unwrap();
    let mut a = Analyzer::new();
    a.analyze_program(&program)
}

fn has_dependency(errs: &[AnalyzeError], name: &str) -> bool {
    errs.iter().any(|e| {
        matches!(&e.kind, AnalyzeErrorKind::DiscordDependency { name: n } if n == name)
    })
}

fn has_impurity(errs: &[AnalyzeError], what: &str) -> bool {
    errs.iter().any(|e| {
        matches!(&e.kind, AnalyzeErrorKind::DiscordImpurity { what: w } if w == what)
    })
}

#[test]
fn independent_discord_statements_pass() {
    let r = analyze(
        "int main() { discord { int a := 1; int b := 2; int c := 3; } return a + b + c; }",
    );
    assert!(r.is_ok(), "got {:?}", r);
}

#[test]
fn discord_read_of_sibling_write_is_e0228() {
    let r = analyze("int main() { discord { int a := 1; int b := a + 1; } return b; }");
    let errs = r.unwrap_err();
    assert!(has_dependency(&errs, "a"), "got {:?}", errs);
}

#[test]
fn discord_double_write_is_e0228() {
    let r = analyze(
        "int main() { mut int x := 0; discord { x = 1; x = 2; } return x; }",
    );
    let errs = r.unwrap_err();
    assert!(has_dependency(&errs, "x"), "got {:?}", errs);
}

#[test]
fn discord_print_is_e0229() {
    let r = analyze("int main() { discord { print_int(1); } return 0; }");
    let errs = r.unwrap_err();
    assert!(has_impurity(&errs, "print_int"), "got {:?}", errs);
}

#[test]
fn discord_control_flow_is_e0229() {
    let r = analyze(
        "int main() { discord { while true { } } return 0; }",
    );
    let errs = r.unwrap_err();
    assert!(has_impurity(&errs, "while"), "got {:?}", errs);
}

#[test]
fn discord_len_call_is_allowed() {
    let r = analyze(
        "int main() { arr[int] xs := [1, 2, 3]; discord { int n := len(xs); int m := 7; } return n + m; }",
    );
    assert!(r.is_ok(), "got {:?}", r);
}

#[test]
fn discord_scope_leaks_by_design() {
    // Declarations inside discord are visible after the block.
    let r = analyze("int main() { discord { int a := 1; } return a; }");
    assert!(r.is_ok(), "got {:?}", r);
}

use eris_compiler::error_display::FormattedError;
use eris_compiler::lexer::SourceLocation;

#[test]
fn every_analyzer_error_kind_has_a_koan() {
    // Spot-check the two discord codes end-to-end, and assert the koan
    // register differs from the plain hint (dual-register, not a copy).
    let err = AnalyzeError::new(
        AnalyzeErrorKind::DiscordImpurity { what: "print_int".to_string() },
        SourceLocation::default(),
    );
    let koan = err.koan().expect("DiscordImpurity must have a koan");
    let hint = err.hint().expect("DiscordImpurity must keep its plain hint");
    assert_ne!(koan, hint);
    assert!(koan.contains("witnessed"));

    let err = AnalyzeError::new(
        AnalyzeErrorKind::DiscordDependency { name: "x".to_string() },
        SourceLocation::default(),
    );
    assert!(err.koan().expect("DiscordDependency must have a koan").contains("'x'"));
}
