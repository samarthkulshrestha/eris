use std::fs;
use std::error::Error;

pub mod lexer;

use crate::lexer::Lexer;

pub fn preprocess_multi_line_comments(source: &str) -> String {
    // Simple state machine to remove multi-line comments
    let mut result = String::new();
    let mut chars = source.chars().peekable();
    let mut in_comment = false;

    while let Some(c) = chars.next() {
        if !in_comment {
            // Check for start of comment
            if c == '-' && chars.peek() == Some(&'+') {
                in_comment = true;
                chars.next(); // consume the '+'
                continue;
            }
            // Not in comment, add to result
            result.push(c);
        } else {
            // Look for end of comment
            if c == '+' && chars.peek() == Some(&'-') {
                in_comment = false;
                chars.next(); // consume the '-'
            }
            // Otherwise, skip characters while in comment
        }
    }

    result
}

pub fn process_file(filename: &str) -> Result<(), Box<dyn Error>> {
    println!("Processing file: {}", filename);

    let raw_source = fs::read_to_string(filename)?;
    println!("Source code loaded:");
    println!("{}\n", raw_source);

    // Preprocess source to handle multi-line comments
    let source = preprocess_multi_line_comments(&raw_source);

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    println!("Tokens:");
    let mut has_errors = false;

    for (idx, token_result) in tokens.iter().enumerate() {
        match token_result {
            Ok((token, location)) => {
                println!("{}: {} (at {}:{})", idx, token, location.line, location.column);
            },
            Err(err) => {
                eprintln!("{}", err);
                has_errors = true;
            }
        }
    }

    if has_errors {
        println!("\nLexing completed with errors.");
    } else {
        println!("\nLexing completed successfully.");
    }

    Ok(())
}

