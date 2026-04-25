use std::env;
use std::path::PathBuf;
use std::process;

use eris_compiler::diagnostics::json::DiagnosticEnvelope;
use eris_compiler::{CompileOptions, OutputFormat};

fn print_usage(prog: &str) {
    eprintln!(
        "Usage: {} [--emit-ir] [-o <output_dir>] [--format text|json] <source_file>",
        prog
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let prog = args.first().map(String::as_str).unwrap_or("eris");

    let mut options = CompileOptions::default();
    let mut source_file: Option<&str> = None;
    let mut iter = args[1..].iter().peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--emit-ir" => options.emit_ir = true,
            "-h" | "--help" => {
                print_usage(prog);
                return;
            }
            "-o" | "--output-dir" => {
                let dir = match iter.next() {
                    Some(d) => d,
                    None => {
                        eprintln!("error: flag '{}' requires an argument", arg);
                        print_usage(prog);
                        process::exit(2);
                    }
                };
                let path = PathBuf::from(dir);
                if let Err(e) = std::fs::create_dir_all(&path) {
                    eprintln!("error: could not create output directory '{}': {}", dir, e);
                    process::exit(1);
                }
                options.output_dir = Some(path);
            }
            "--format" => {
                let fmt = match iter.next() {
                    Some(f) => f,
                    None => {
                        eprintln!("error: flag '--format' requires an argument (text|json)");
                        print_usage(prog);
                        process::exit(2);
                    }
                };
                match fmt.as_str() {
                    "text" => options.format = OutputFormat::Text,
                    "json" => options.format = OutputFormat::Json,
                    other => {
                        eprintln!(
                            "error: unknown format '{}'; expected 'text' or 'json'",
                            other
                        );
                        print_usage(prog);
                        process::exit(2);
                    }
                }
            }
            s if s.starts_with("--") => {
                eprintln!("error: unknown flag '{}'", s);
                print_usage(prog);
                process::exit(2);
            }
            s => {
                if source_file.is_some() {
                    eprintln!("error: multiple source files not supported");
                    process::exit(2);
                }
                source_file = Some(s);
            }
        }
    }

    let Some(file) = source_file else {
        print_usage(prog);
        process::exit(1);
    };

    match eris_compiler::process_file(file, options.clone()) {
        Ok(()) => {
            if options.format == OutputFormat::Json {
                let envelope = DiagnosticEnvelope::ok();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&envelope)
                        .unwrap_or_else(|_| "{}".to_string())
                );
            }
        }
        Err(e) => {
            if options.format == OutputFormat::Json {
                use eris_compiler::CompileError;
                let envelope = match &e {
                    CompileError::Io(io_err) => {
                        DiagnosticEnvelope::internal_error(format!("io: {}", io_err))
                    }
                    CompileError::Codegen(msg) => {
                        DiagnosticEnvelope::internal_error(format!("codegen: {}", msg))
                    }
                    CompileError::Link(msg) => {
                        DiagnosticEnvelope::internal_error(format!("link: {}", msg))
                    }
                    _ => {
                        let diagnostics: Vec<eris_compiler::diagnostics::Diagnostic> =
                            Vec::from(&e);
                        DiagnosticEnvelope::compile_error(diagnostics)
                    }
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&envelope)
                        .unwrap_or_else(|_| "{}".to_string())
                );
                process::exit(1);
            } else {
                eprint!("{}", e);
                process::exit(1);
            }
        }
    }
}
