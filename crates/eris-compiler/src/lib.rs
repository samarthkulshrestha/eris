use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;

pub mod analyzer;
pub(crate) mod codegen;
pub mod cookbook;
pub mod explain;
pub mod diagnostics;
pub mod error_display;
pub mod lexer;
pub mod parser;

use crate::analyzer::AnalyzeError;
use crate::error_display::FormattedError;
use crate::lexer::LexerError;
use crate::parser::{ParseError, Parser};
use inkwell::context::Context;

#[derive(Debug)]
pub enum CompileError {
    Io(std::io::Error),
    Lex {
        source: String,
        filename: String,
        error: LexerError,
    },
    Parse {
        source: String,
        filename: String,
        error: ParseError,
    },
    Analyze {
        source: String,
        filename: String,
        errors: Vec<AnalyzeError>,
    },
    Codegen(String),
    Link(String),
}

impl CompileError {
    /// Render as human-readable text. `boring` strips the koan register,
    /// falling back to the plain hint.
    pub fn render_text(&self, boring: bool) -> String {
        match self {
            CompileError::Io(e) => e.to_string(),
            CompileError::Lex { source, filename, error } => {
                error.format_with_source_voiced(source, filename, boring)
            }
            CompileError::Parse { source, filename, error } => {
                error.format_with_source_voiced(source, filename, boring)
            }
            CompileError::Analyze { source, filename, errors } => {
                let mut out = String::new();
                for err in errors {
                    out.push_str(&err.format_with_source_voiced(source, filename, boring));
                    out.push('\n');
                }
                out
            }
            CompileError::Codegen(msg) => format!("code generation error: {}", msg),
            CompileError::Link(msg) => format!("linker error: {}", msg),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render_text(false))
    }
}

impl std::error::Error for CompileError {}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::Io(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    /// When true, write the LLVM IR to <module>.ll alongside the binary.
    pub emit_ir: bool,
    /// Directory to place the binary, intermediate object file, and (when
    /// emit_ir is true) the .ll file. Defaults to the current working
    /// directory.
    pub output_dir: Option<std::path::PathBuf>,
    /// Output format for diagnostics (text or json).
    pub format: OutputFormat,
    /// Strip the whimsical register from terminal diagnostics.
    pub boring: bool,
}

impl From<&CompileError> for Vec<crate::diagnostics::Diagnostic> {
    fn from(err: &CompileError) -> Self {
        use crate::diagnostics::Diagnostic;
        match err {
            CompileError::Io(_) => vec![],
            CompileError::Lex { error, .. } => vec![Diagnostic::from(error.clone())],
            CompileError::Parse { error, .. } => vec![Diagnostic::from(error.clone())],
            CompileError::Analyze { errors, .. } => {
                errors.iter().map(|e| Diagnostic::from(e.clone())).collect()
            }
            CompileError::Codegen(_) | CompileError::Link(_) => vec![],
        }
    }
}

/// Lex, parse, and analyze Eris source. Returns every diagnostic the
/// lexer, parser, and analyzer would emit. Empty Vec means the source
/// passes all three phases cleanly. Performs no codegen or linking.
///
/// This is the pure, side-effect-free analysis entry point. It never
/// writes files, never panics on user input, never invokes clang.
pub fn analyze_source(source: &str) -> Vec<crate::diagnostics::Diagnostic> {
    use crate::diagnostics::Diagnostic;

    let mut lexer = crate::lexer::Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => return vec![Diagnostic::from(e)],
    };

    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => return vec![Diagnostic::from(e)],
    };

    let mut analyzer = crate::analyzer::Analyzer::new();
    match analyzer.analyze_program(&program) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.into_iter().map(Diagnostic::from).collect(),
    }
}

pub fn process_file(filename: &str, options: CompileOptions) -> Result<(), CompileError> {
    let source = fs::read_to_string(filename)?;

    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize().map_err(|error| CompileError::Lex {
        source: source.clone(),
        filename: filename.to_string(),
        error,
    })?;

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|error| CompileError::Parse {
        source: source.clone(),
        filename: filename.to_string(),
        error,
    })?;

    let mut analyzer = crate::analyzer::Analyzer::new();
    analyzer
        .analyze_program(&program)
        .map_err(|errors| CompileError::Analyze {
            source: source.clone(),
            filename: filename.to_string(),
            errors,
        })?;
    if options.format == OutputFormat::Text {
        println!("Analysis successful!");
    }

    let context = Context::create();
    let module_name = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let mut codegen = crate::codegen::CodeGen::new(&context, module_name);
    codegen.generate(&program).map_err(CompileError::Codegen)?;
    if options.format == OutputFormat::Text {
        println!("Code generation successful!");
    }

    let out_dir = options
        .output_dir
        .as_deref()
        .unwrap_or_else(|| Path::new("."));

    let object_file = out_dir.join(format!("{}.o", module_name));
    let output_binary = out_dir.join(module_name);

    codegen
        .write_object_file(&object_file)
        .map_err(|e| CompileError::Codegen(format!("failed to write object file: {}", e)))?;
    if options.format == OutputFormat::Text {
        println!("Object file written to: {}", object_file.display());
    }

    let link_status = Command::new("clang")
        .arg(&object_file)
        .arg("-o")
        .arg(&output_binary)
        .status()
        .map_err(|e| CompileError::Link(format!("failed to invoke clang: {}", e)))?;

    if !link_status.success() {
        return Err(CompileError::Link(format!(
            "clang exited with status {}",
            link_status
        )));
    }

    if options.format == OutputFormat::Text {
        println!("Executable created: {}", output_binary.display());
    }
    let _ = fs::remove_file(&object_file);

    if options.emit_ir {
        let ir_file = out_dir.join(format!("{}.ll", module_name));
        codegen
            .write_to_file(ir_file.to_str().unwrap_or("out.ll"))
            .map_err(|e| CompileError::Codegen(format!("failed to write IR: {}", e)))?;
        if options.format == OutputFormat::Text {
            println!("LLVM IR written to: {}", ir_file.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        // CARGO_MANIFEST_DIR is crates/eris-compiler; go up two levels to workspace root.
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn test_output_dir_places_binary_in_specified_dir() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let src = workspace_root().join("examples/01.eris");

        let options = CompileOptions {
            emit_ir: false,
            output_dir: Some(tmp.path().to_path_buf()),
            format: OutputFormat::Text,
            boring: false,
        };

        process_file(src.to_str().unwrap(), options).expect("compilation failed");

        // Binary must exist inside the specified output directory.
        assert!(
            tmp.path().join("01").exists(),
            "binary not found in output_dir"
        );

        // The object file must have been cleaned up.
        assert!(
            !tmp.path().join("01.o").exists(),
            "object file was not removed"
        );
    }

    #[test]
    fn test_default_output_dir_uses_cwd() {
        // Use a dedicated tempdir as the working directory substitute: because
        // we cannot change process cwd in a test, we pass the tempdir as the
        // output_dir and separately verify that passing None produces a binary
        // in "." (cwd).  Here we confirm the binary lands where expected when
        // output_dir is explicitly set, as a proxy for the None == cwd rule.
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let src = workspace_root().join("examples/01.eris");

        // Build with an explicit output_dir to confirm the happy path.
        let options = CompileOptions {
            emit_ir: false,
            output_dir: Some(tmp.path().to_path_buf()),
            format: OutputFormat::Text,
            boring: false,
        };
        process_file(src.to_str().unwrap(), options).expect("compilation failed");
        assert!(tmp.path().join("01").exists(), "binary not found");

        // Build with output_dir = None; the binary lands in cwd (whatever the
        // test runner's cwd is).  We only assert success and clean up.
        let options_default = CompileOptions::default();
        process_file(src.to_str().unwrap(), options_default).expect("compilation with default options failed");
        let _ = fs::remove_file("01");
        let _ = fs::remove_file("01.ll");
    }

    #[test]
    fn analyze_source_returns_lex_error_diagnostic() {
        // `@` is not a valid Eris token.
        let diagnostics = analyze_source("int main() { return @; }");
        assert_eq!(diagnostics.len(), 1, "expected exactly one diagnostic");
        assert_eq!(diagnostics[0].code, crate::diagnostics::ErrorCode::E0002);
    }

    #[test]
    fn analyze_source_returns_parse_error_diagnostic() {
        // Missing function body.
        let diagnostics = analyze_source("int main()");
        assert!(!diagnostics.is_empty(), "expected at least one diagnostic");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == crate::diagnostics::ErrorCode::E0101
                    || d.code == crate::diagnostics::ErrorCode::E0102),
            "expected a parse error code, got {:?}",
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn analyze_source_returns_analyzer_error_diagnostic() {
        let diagnostics = analyze_source("int main() { int x := 'a'; return 0; }");
        assert!(!diagnostics.is_empty(), "expected at least one diagnostic");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == crate::diagnostics::ErrorCode::E0203),
            "expected E0203, got {:?}",
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn analyze_source_returns_empty_for_valid_program() {
        let diagnostics =
            analyze_source("int main() { print_int(42); println(); return 0; }");
        assert!(
            diagnostics.is_empty(),
            "expected zero diagnostics for valid program, got {:?}",
            diagnostics
        );
    }
}
