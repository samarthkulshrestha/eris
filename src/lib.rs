use std::fs;
use std::error::Error;
use std::process::{self, Command};
use std::path::Path;

pub mod analyzer;
pub mod codegen;
pub mod error_display;
pub mod lexer;
pub mod parser;

use crate::error_display::FormattedError;
use crate::parser::Parser;
use inkwell::context::Context;

pub fn process_file(filename: &str) -> Result<(), Box<dyn Error>> {
    // println!("Processing file: {}", filename);

    let source = fs::read_to_string(filename)?;
    // println!("Source code loaded:");
    // println!("{}", raw_source);

    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("{}", err.format_with_source(&source, filename));
            process::exit(1);
        }
    };

    // Parsing
    let mut parser = Parser::new(tokens);

    match parser.parse_program() {
        Ok(program) => {
            // println!("\nParsing successful!");
            // println!("AST: {:#?}", program);

            let mut analyzer = crate::analyzer::Analyzer::new();
            match analyzer.analyze_program(&program) {
                Ok(_) => {
                    println!("Analysis successful!");

                    // Code generation
                    let context = Context::create();
                    let module_name = std::path::Path::new(filename)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("main");

                    let mut codegen = crate::codegen::CodeGen::new(&context, module_name);

                    match codegen.generate(&program) {
                        Ok(_) => {
                            println!("Code generation successful!");

                            // Generate output files
                            let object_file = format!("{}.o", module_name);
                            let output_binary = module_name;

                            // Write object file
                            codegen.write_object_file(Path::new(&object_file))
                                .map_err(|e| format!("Failed to write object file: {}", e))?;
                            println!("Object file written to: {}", object_file);

                            // Link to create executable
                            let link_status = Command::new("clang")
                                .arg(&object_file)
                                .arg("-o")
                                .arg(&output_binary)
                                .status()?;

                            if !link_status.success() {
                                eprintln!("Linking failed");
                                process::exit(1);
                            }

                            println!("Executable created: {}", output_binary);

                            // Clean up object file
                            let _ = fs::remove_file(&object_file);

                            // Optionally write LLVM IR for debugging
                            let ir_file = format!("{}.ll", module_name);
                            codegen.write_to_file(&ir_file)?;
                            println!("LLVM IR written to: {} (for debugging)", ir_file);
                        }
                        Err(e) => {
                            eprintln!("Code generation error: {}", e);
                            process::exit(1);
                        }
                    }
                }
                Err(errors) => {
                    for err in errors {
                        eprintln!("{}", err.format_with_source(&source, filename));
                    }
                    process::exit(1);
                }
            }
        },
        Err(err) => {
            eprintln!("{}", err.format_with_source(&source, filename));
            process::exit(1);
        }
    }

    Ok(())
}

