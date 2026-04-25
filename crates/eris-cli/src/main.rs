use std::env;
use std::path::PathBuf;
use std::process;

use eris_compiler::CompileOptions;

fn print_usage(prog: &str) {
    eprintln!("Usage: {} [--emit-ir] [-o <output_dir>] <source_file>", prog);
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

    if let Err(e) = eris_compiler::process_file(file, options) {
        eprint!("{}", e);
        process::exit(1);
    }
}
