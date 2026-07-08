use eris_compiler::analyzer::{AnalyzeError, AnalyzeErrorKind, Analyzer};
use eris_compiler::lexer::Lexer;
use eris_compiler::parser::Parser;

fn analyze(source: &str) -> Result<(), Vec<AnalyzeError>> {
    let mut lex = Lexer::new(source);
    let toks = lex.tokenize().unwrap();
    let mut p = Parser::new(toks);
    let program = p.parse_program().unwrap();
    let mut a = Analyzer::new();
    a.analyze_program(&program)
}

fn has_unsupported_op(errs: &[AnalyzeError], op: &str) -> bool {
    errs.iter().any(|e| {
        matches!(&e.kind, AnalyzeErrorKind::UnsupportedOperandType { op: o, .. } if *o == op)
    })
}

#[test]
fn str_concat_rejected_with_e0227() {
    let result = analyze(
        r#"int main() { str a := "x"; str b := "y"; str c := a + b; return 0; }"#,
    );
    let errs = result.unwrap_err();
    assert!(has_unsupported_op(&errs, "+"), "got {:?}", errs);
}

#[test]
fn array_addition_rejected_with_e0227() {
    let result = analyze(
        "int main() { arr[int] a := [1, 2]; arr[int] b := [3, 4]; arr[int] c := a + b; return 0; }",
    );
    let errs = result.unwrap_err();
    assert!(has_unsupported_op(&errs, "+"), "got {:?}", errs);
}

#[test]
fn bool_arithmetic_rejected_with_e0227() {
    let result = analyze("int main() { bool c := true * false; return 0; }");
    let errs = result.unwrap_err();
    assert!(has_unsupported_op(&errs, "*"), "got {:?}", errs);
}

#[test]
fn int_and_float_arithmetic_still_pass() {
    assert!(analyze("int main() { int c := 1 + 2; return 0; }").is_ok());
    assert!(analyze("int main() { float c := 1.0 + 2.0; return 0; }").is_ok());
}

#[test]
fn str_equality_rejected_with_e0227() {
    let result = analyze(
        r#"int main() { str a := "x"; str b := "y"; bool c := a == b; return 0; }"#,
    );
    let errs = result.unwrap_err();
    assert!(has_unsupported_op(&errs, "=="), "got {:?}", errs);
}

#[test]
fn array_equality_rejected_with_e0227() {
    let result = analyze(
        "int main() { arr[int] a := [1, 2]; arr[int] b := [1, 2]; bool c := a == b; return 0; }",
    );
    let errs = result.unwrap_err();
    assert!(has_unsupported_op(&errs, "=="), "got {:?}", errs);
}

#[test]
fn bool_ordering_rejected_with_e0227() {
    let result = analyze("int main() { bool c := true < false; return 0; }");
    let errs = result.unwrap_err();
    assert!(has_unsupported_op(&errs, "<"), "got {:?}", errs);
}

#[test]
fn scalar_comparisons_still_pass() {
    assert!(analyze("int main() { bool c := 1 == 2; return 0; }").is_ok());
    assert!(analyze("int main() { bool c := 1.0 < 2.0; return 0; }").is_ok());
    assert!(analyze("int main() { bool c := 'a' < 'b'; return 0; }").is_ok());
    assert!(analyze("int main() { bool c := true == false; return 0; }").is_ok());
}
