use std::env;
use std::path::PathBuf;
use std::process;

use eris_compiler::diagnostics::json::DiagnosticEnvelope;
use eris_compiler::{CompileOptions, OutputFormat};

/// Embedded language card JSON. Rebuilt automatically when
/// `schemas/language-card.json` changes (cargo tracks include_str! deps).
const CARD_JSON: &str = include_str!("../../../schemas/language-card.json");

/// Embedded cookbook index JSON. Same rebuild semantics as CARD_JSON.
const COOKBOOK_JSON: &str = include_str!("../../../schemas/cookbook.json");

fn print_usage(prog: &str) {
    eprintln!(
        "Usage: {} [--emit-ir] [-o <output_dir>] [--format text|json] <source_file>",
        prog
    );
    eprintln!(
        "       {} --explain <error_code> [--format text|json]",
        prog
    );
}

fn print_explained_text(
    explained: &eris_compiler::explain::ExplainedError,
    out: &mut impl std::io::Write,
) -> std::io::Result<()> {
    writeln!(out, "{} — {}", explained.code, explained.name)?;
    writeln!(out)?;
    if let Some(hint) = &explained.hint {
        writeln!(out, "  {}", hint)?;
        writeln!(out)?;
    }
    if !explained.fields.is_empty() {
        writeln!(out, "Fields: {}", explained.fields.join(", "))?;
        writeln!(out)?;
    }
    if explained.cookbook_entries.is_empty() {
        writeln!(out, "(no cookbook examples available)")?;
    } else {
        writeln!(out, "Examples from the cookbook:")?;
        writeln!(out)?;
        for entry in &explained.cookbook_entries {
            writeln!(out, "  {}  ({})", entry.id, entry.file)?;
            writeln!(out, "    {}", entry.title)?;
            for line in entry.description.lines() {
                writeln!(out, "    {}", line)?;
            }
            writeln!(out)?;
            for line in entry.source.lines() {
                writeln!(out, "      {}", line)?;
            }
            writeln!(out)?;
        }
    }
    Ok(())
}

fn run_explain(code: &str, format: eris_compiler::OutputFormat) -> ! {
    use eris_compiler::cookbook::CookbookEntry;
    use eris_compiler::explain::{explain_error_lookup, ExplainError};

    let cookbook: Vec<CookbookEntry> = match serde_json::from_str(COOKBOOK_JSON) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: embedded cookbook.json is malformed: {}", e);
            eprintln!("rebuild eris-cli or regenerate the cookbook with `cargo run -p eris-compiler --bin dump-cookbook > schemas/cookbook.json`");
            process::exit(70);
        }
    };

    match explain_error_lookup(CARD_JSON, &cookbook, code) {
        Ok(explained) => match format {
            eris_compiler::OutputFormat::Text => {
                let mut stdout = std::io::stdout().lock();
                if let Err(e) = print_explained_text(&explained, &mut stdout) {
                    eprintln!("error: write to stdout failed: {}", e);
                    process::exit(1);
                }
                process::exit(0);
            }
            eris_compiler::OutputFormat::Json => {
                match serde_json::to_string_pretty(&explained) {
                    Ok(json) => {
                        println!("{}", json);
                        process::exit(0);
                    }
                    Err(e) => {
                        eprintln!("error: failed to serialize explained: {}", e);
                        process::exit(1);
                    }
                }
            }
        },
        Err(ExplainError::UnknownCode { code, known_codes }) => {
            eprintln!("error: unknown error code `{}`", code);
            eprintln!("valid codes: {}", known_codes.join(", "));
            process::exit(2);
        }
        Err(ExplainError::MalformedCard(msg)) => {
            eprintln!("error: malformed language card: {}", msg);
            process::exit(70);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let prog = args.first().map(String::as_str).unwrap_or("eris");

    let mut options = CompileOptions::default();
    let mut source_file: Option<&str> = None;
    let mut explain_code: Option<String> = None;
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
            "--explain" => {
                let code = match iter.next() {
                    Some(c) => c.clone(),
                    None => {
                        eprintln!("error: flag '--explain' requires an argument (error code, e.g., E0203)");
                        print_usage(prog);
                        process::exit(2);
                    }
                };
                if explain_code.is_some() {
                    eprintln!("error: --explain specified more than once");
                    process::exit(2);
                }
                explain_code = Some(code);
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

    if let Some(code) = explain_code {
        if source_file.is_some() {
            eprintln!("error: --explain cannot be combined with a source file");
            print_usage(prog);
            process::exit(2);
        }
        if options.emit_ir {
            eprintln!("error: --explain cannot be combined with --emit-ir");
            print_usage(prog);
            process::exit(2);
        }
        if options.output_dir.is_some() {
            eprintln!("error: --explain cannot be combined with -o / --output-dir");
            print_usage(prog);
            process::exit(2);
        }
        run_explain(&code, options.format);
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
