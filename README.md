# eris

> Code at the edge of order.

Eris is a small, statically-typed, expression-oriented programming language with a working LLVM backend. It is also an experiment in compiler design: building a language whose compiler exposes its internal model — grammar, type rules, scope, errors, idioms — through structured APIs, so that Large Language Models can reason about Eris programs with the same rigor as the compiler itself.

This is a research project in early development.

## Status

**Version 0.8.0.** The language has a complete frontend (lexer → parser → semantic analyzer) and an LLVM IR backend that produces native binaries via `clang`. Programs can use built-in I/O (`print_int`, `print_str`, `print_bool`, `print_char`, `println`, `read_int`) and the standard control-flow constructs.

The LLM-facing surface is largely built: structured diagnostics with stable error codes (`--format json`), a machine-readable language card (`schemas/language-card.json`), a verified cookbook (`schemas/cookbook.json`), an MCP server exposing the compiler as tool calls (`crates/eris-mcp`), and an LLM-grounded `--explain` flag. What does not exist yet is the *evidence*: a measured evaluation showing this surface helps a model write Eris better than plain documentation does. That evaluation is the project's current focus; the language itself is frozen at v0.8.0 until it lands.

## Design principles

Eris is a language of **deliberate acts**. Three rules:

- **Explicit typing.** Every variable carries a type annotation. There is no inference.
- **Intentional mutability.** Bindings are immutable by default; use `mut` to opt in.
- **Expression-oriented.** Blocks and `if` are expressions that produce values. The trailing expression of a block — written without a semicolon — is its result.

The point is to make the choices visible. You should be able to read a piece of Eris and know exactly what it does without inferring intent.

## A compiler designed for LLM collaboration

Most compilers are designed for one audience: human programmers reading text errors. The terminal-rendered diagnostics, the prose documentation, the man-page-style help — all of it assumes the reader is human.

The thesis behind Eris is that a compiler should also serve another audience: a model writing or reasoning about code in the language. That second audience needs different things:

- **Structured errors with stable codes**, not just rendered text — so an automated fix loop can match on `E0203` instead of parsing English.
- **A machine-readable language description** — so a model that has never seen Eris in its training data can write valid programs from a single document in its context.
- **A tool-call interface to the compiler's internals** — so an agent can ask "what's in scope here?" or "would this expression typecheck?" with the same authority the compiler itself has.
- **A curated cookbook of canonical examples**, every one verified to compile — so generated code can be grounded in tested patterns rather than hallucinated ones.
- **An opinionated linter** — encoding what *idiomatic* Eris looks like, not just what compiles.

Eris is being built deliberately along these lines. The compiler exists for humans; the surrounding tooling will exist for both. The work-in-progress lives in the commit history.

## Quick start

Requires a recent Rust toolchain and LLVM 17 installed locally.

```bash
# Build the workspace
cargo build --workspace

# Run an example: factorial of 5, returned as the program's exit code
cargo run -p eris-cli -- examples/01.eris
./01 ; echo $?       # → 120

# Run an example with I/O
cargo run -p eris-cli -- examples/03_strings.eris
./03_strings

# Dump the generated LLVM IR alongside the binary
cargo run -p eris-cli -- --emit-ir examples/03_strings.eris

# Place outputs in a specific directory instead of the current one
cargo run -p eris-cli -- -o ./build examples/01.eris
```

## Syntax

```eris
-- single-line comment
--- doc comment

-- variable declarations
int x := 10;
mut int y := 20;        -- `mut` opts into mutability
y = y * 2;              -- re-assignment uses `=`

-- multiple bindings
int a, b, c := 1, 2, 3;
int (i, j) := (10, 20);

-- blocks return their trailing expression
int result := {
    int n := 5;
    n * n               -- no semicolon: this is the block's value
};

-- if as expression
int classify(int x) {
    return if x < 0 { -1 } else if x == 0 { 0 } else { 1 };
}

-- functions and recursion
int factorial(int n) {
    if n <= 1 {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

-- I/O via built-ins
int main() {
    print_str("the answer is: ");
    print_int(factorial(5));
    println();
    return 0;
}
```

## Project layout

```
crates/
  eris-compiler/     library: lexer, parser, analyzer, LLVM IR codegen
  eris-cli/          binary: the `eris` command-line driver
examples/            sample programs
```

The compiler is a library first. The MCP server lives as a sibling crate (`crates/eris-mcp`) that depends on `eris-compiler` and never reaches into its internals; the evaluation harness lives in `eval/` (Python).

## Development

```bash
cargo test --workspace          # full test suite
cargo check --workspace
cargo fmt
cargo clippy --workspace
```

## Editor support

A regex-based syntax-highlighting file for Vim/Neovim lives at `editor/vim/`. To enable it, add the directory to your `runtimepath`:

```vim
set runtimepath^=/path/to/eris/editor/vim
```

That picks up the syntax file (`syntax/eris.vim`) and the filetype detection (`ftdetect/eris.vim` — associates `*.eris` with the `eris` filetype).

## License

MIT.
