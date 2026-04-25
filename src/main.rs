use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <source_file>", args[0]);
        process::exit(1);
    }

    if let Err(e) = eris::process_file(&args[1]) {
        eprint!("{}", e);
        process::exit(1);
    }
}
