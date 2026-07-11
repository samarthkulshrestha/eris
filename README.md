# eris

> Code at the edge of order.

## What this is

Eris is a small compiled language built for the pleasure of building it —
and for the reader. It will not replace your systems language, and it is not
trying to. It is trying to make you go *"ahhh, clever."*

Eris is a language of **deliberate acts**, dedicated to the goddess of
discord. That is not a contradiction; it is the whole idea:

- **Explicit typing.** Every variable carries a type annotation. No inference.
- **Intentional mutability.** Bindings are immutable unless marked `mut`.
- **Expression-oriented.** Blocks and `if` produce values; the trailing
  expression of a block is its result.
- **Order is the default — chaos is opt-in, and it must be earned.**

## discord

Every language executes statements top to bottom, whether or not the order
means anything. Eris lets you say when it doesn't:

    discord {
        int width  := 4 * 5;
        int height := 3 * 5;
        int depth  := 2 * 5;
    }

Statements in a `discord` block have **no defined execution order**. In
exchange, the compiler verifies they are independent — no statement may
touch what another writes, and nothing with side effects may enter. The
result: every order produces the same answer, so the chaos is provably
unobservable. Eris only accepts freedom she can prove harmless.

If your chaos is impure, you'll hear about it:

    error: Statements in this discord block are not independent: more than one touches 'a'
      --> examples/witnessed.eris:8:9
      |
    8 |         int b := a + 1;
      |         ^ discord dependency
      |
      = hint: these statements share 'a', so their order matters — and
              ordered chaos is no chaos at all. separate them

Every error speaks in two registers: the message is surgical, the hint has
had some wine. `--boring` silences the second register. The first is never
compromised.

## fnord

`fnord;` marks the path that cannot be reached — and the return-checker
believes you. If a running program ever reaches one, it prints
`you saw the fnord.` and exits with code 5.

## For whom

Readers. Eris optimizes for the person looking at it, not a production
deployment: a handful of jewel-box examples (`examples/`), error messages
worth screenshotting, and a reference (`LANGUAGE.md`) short enough to
actually read. If you only run one thing:

    cargo run -p eris-cli -- examples/witnessed.eris

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

The compiler is a library first. The evaluation harness lives in `eval/` (Python).

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
