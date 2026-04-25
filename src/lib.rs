use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;

pub mod analyzer;
pub mod codegen;
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

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Io(e) => write!(f, "{}", e),
            CompileError::Lex { source, filename, error } => {
                write!(f, "{}", error.format_with_source(source, filename))
            }
            CompileError::Parse { source, filename, error } => {
                write!(f, "{}", error.format_with_source(source, filename))
            }
            CompileError::Analyze { source, filename, errors } => {
                for err in errors {
                    writeln!(f, "{}", err.format_with_source(source, filename))?;
                }
                Ok(())
            }
            CompileError::Codegen(msg) => write!(f, "code generation error: {}", msg),
            CompileError::Link(msg) => write!(f, "linker error: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::Io(e)
    }
}

pub fn process_file(filename: &str) -> Result<(), CompileError> {
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
    println!("Analysis successful!");

    let context = Context::create();
    let module_name = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let mut codegen = crate::codegen::CodeGen::new(&context, module_name);
    codegen.generate(&program).map_err(CompileError::Codegen)?;
    println!("Code generation successful!");

    let object_file = format!("{}.o", module_name);
    let output_binary = module_name;

    codegen
        .write_object_file(Path::new(&object_file))
        .map_err(|e| CompileError::Codegen(format!("failed to write object file: {}", e)))?;
    println!("Object file written to: {}", object_file);

    let link_status = Command::new("clang")
        .arg(&object_file)
        .arg("-o")
        .arg(output_binary)
        .status()
        .map_err(|e| CompileError::Link(format!("failed to invoke clang: {}", e)))?;

    if !link_status.success() {
        return Err(CompileError::Link(format!(
            "clang exited with status {}",
            link_status
        )));
    }

    println!("Executable created: {}", output_binary);
    let _ = fs::remove_file(&object_file);

    let ir_file = format!("{}.ll", module_name);
    codegen
        .write_to_file(&ir_file)
        .map_err(|e| CompileError::Codegen(format!("failed to write IR: {}", e)))?;
    println!("LLVM IR written to: {} (for debugging)", ir_file);

    Ok(())
}
