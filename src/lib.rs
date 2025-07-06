use std::fs;
use std::error::Error;
use std::process;

pub mod analyzer;
pub mod lexer;
pub mod parser;

use crate::parser::Parser;

pub fn process_file(filename: &str) -> Result<(), Box<dyn Error>> {
    // println!("Processing file: {}", filename);

    let source = fs::read_to_string(filename)?;
    // println!("Source code loaded:");
    // println!("{}", raw_source);

    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(err) => {
            eprintln!("{}", err);
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
                }
                Err(errors) => {
                    for err in errors {
                        eprintln!("Analyze error: {:?} at {:?}", err.kind, err.location);
                    }
                    process::exit(1);
                }
            }
        },
        Err(err) => {
            eprintln!("Parse error: {:?}", err);
            process::exit(1);
        }
    }

    Ok(())
}

