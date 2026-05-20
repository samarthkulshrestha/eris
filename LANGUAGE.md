# Eris Language Reference — v0.8.0

This document is the canonical reference for the Eris programming language at version 0.8.0. It is self-contained: every rule, every operator, and every example needed to write a valid Eris program is defined here. Assume no prior knowledge of any other language's syntax.

---

## 1. Identity

Eris is a statically typed, expression-oriented systems language with three core design rules:

1. **Explicit typing.** Every variable declaration carries a type annotation written before the name. There is no type inference.
2. **Mutability opt-in.** Variables are immutable by default. Prepend `mut` to make a variable or parameter reassignable.
3. **Expression orientation.** Blocks (`{ ... }`) are expressions and yield the value of their last un-terminated expression. `if` is also an expression.

---

## 2. Lexical Syntax

### 2.1 Keywords

Eris reserves the following 17 identifiers as keywords. They may not be used as variable or function names.

| Keyword    | Role                                 |
|------------|--------------------------------------|
| `arr`      | array type constructor               |
| `bool`     | boolean type                         |
| `break`    | exit the enclosing loop              |
| `char`     | character type                       |
| `continue` | advance to the next loop iteration   |
| `else`     | alternate branch of `if`             |
| `false`    | boolean literal false                |
| `float`    | floating-point type                  |
| `for`      | range-based for loop                 |
| `if`       | conditional expression               |
| `in`       | separates loop variable from range   |
| `int`      | integer type                         |
| `mut`      | mutability qualifier                 |
| `return`   | early exit from a function           |
| `str`      | string type                          |
| `true`     | boolean literal true                 |
| `while`    | condition-tested loop                |

### 2.2 Operators

Operators are listed below from **lowest to highest** precedence. Within each group, operators have equal precedence and associate left-to-right except where noted.

| Precedence level | Operators   | Notes                                                  |
|------------------|-------------|--------------------------------------------------------|
| LogicalOr        | `\|\|`      | short-circuit logical OR; both operands must be `bool` |
| LogicalAnd       | `&&`        | short-circuit logical AND; both operands must be `bool`|
| Equality         | `==`  `!=`  | structural equality; operands must have the same type  |
| Comparison       | `<`  `<=`  `>`  `>=` | numeric comparison; returns `bool`          |
| BitOr            | `\|`        | bitwise OR; `int` operands only                        |
| BitXor           | `^`         | bitwise XOR; `int` operands only                       |
| BitAnd           | `&`         | bitwise AND; `int` operands only                       |
| Shift            | `<<`  `>>`  | left/right shift; `int` operands only                  |
| Sum              | `+`  `-`    | addition / subtraction                                 |
| Product          | `*`  `/`  `%` | multiply / divide / modulo                           |
| Group            | `( )`       | parentheses override precedence                        |
| Prefix           | `-X`  `!X`  `~X` | arithmetic negation, logical NOT, bitwise NOT     |

**Important precedence note.** Bitwise operators (`|`, `^`, `&`, `<<`, `>>`) sit _above_ comparisons, matching Rust's precedence rules rather than C's. This means `a & b == c` parses as `(a & b) == c`.

Additional special tokens that are not binary operators:

| Token | Name            | Usage                                               |
|-------|-----------------|-----------------------------------------------------|
| `:=`  | DeclareAssign   | declare-and-initialize a variable                   |
| `=`   | Assign          | reassign an existing mutable variable               |
| `..`  | DotDot          | half-open integer range used only in `for`-`in`     |

Unary prefix forms:

| Token | Operator         | Operand type | Result type |
|-------|------------------|--------------|-------------|
| `-`   | arithmetic negate| `int`, `float` | same as operand |
| `!`   | logical NOT      | `bool`       | `bool`      |
| `~`   | bitwise NOT      | `int`        | `int`       |
| `&`   | take shared ref  | lvalue of type `int` | `&int`  |
| `&mut`| take mutable ref | mutable lvalue of type `int` | `&mut int` |
| `*`   | dereference      | `&int` or `&mut int` | `int` |

### 2.3 Comments

```
-- single-line comment (extends to end of line)
--- doc comment (same lexical treatment as single-line comment)
```

Multi-line comments (`-+ ... +-`) are defined in the grammar but not yet processed by the lexer. Use consecutive `--` lines instead.

### 2.4 Literal forms

| Type    | Pattern / rule                                             | Example           |
|---------|------------------------------------------------------------|-------------------|
| `int`   | one or more ASCII digits `[0-9]+`; stored as i64           | `42`, `0`         |
| `float` | digits, decimal point, digits `[0-9]+\.[0-9]+`            | `3.14`, `1.00`    |
| `char`  | single-quoted character; supports `\n \t \r \\ \'`        | `'a'`, `'\n'`     |
| `str`   | double-quoted text; supports `\" \\ \n \t`                | `"hello\n"`       |
| `bool`  | keywords `true` or `false`                                 | `true`            |

Integer literals are always non-negative. Write a negative integer as a negation expression: `-5` (parsed as unary `-` applied to `5`).

### 2.5 Identifiers

An identifier starts with a letter (`a–z`, `A–Z`) or underscore (`_`), followed by zero or more letters, digits, or underscores. Identifiers that start with a digit are rejected as `E0001`.

### 2.6 Range syntax

`start..end` is a half-open integer range `[start, end)`. It is valid only as the iterable in a `for`-`in` statement. Both endpoints must have type `int`. The range expression is not a first-class value and cannot appear in other contexts.

---

## 3. Type System

Eris has no implicit conversions. Every assignment and call-site argument must match the expected type exactly (E0203).

### 3.1 `int`

64-bit signed integer. LLVM type: `i64`. Arithmetic operators `+`, `-`, `*`, `/`, `%` require both operands to be `int` (or both `float`; they cannot be mixed). Bitwise operators (`&`, `|`, `^`, `~`, `<<`, `>>`) accept only `int`.

```
int x := 42;
int y := x * 2 + 1;
```

### 3.2 `bool`

Boolean truth value. LLVM type: `i1`. Literals: `true`, `false`. Logical operators `&&` and `||` require `bool` operands and return `bool`. Comparison operators return `bool`.

```
bool flag := x > 0;
bool both := flag && true;
```

### 3.3 `char`

A single ASCII character. LLVM type: `i8`. Expressed as a single-quoted literal.

```
char c := 'A';
```

### 3.4 `float`

64-bit IEEE 754 double-precision float. LLVM type: `f64`. Literal must include a decimal point.

```
float pi := 3.14159;
```

### 3.5 `str`

A null-terminated C string. LLVM type: `ptr` (pointer to `i8`). String literals are interned as global constants. Tier 1 note: a slice-based `str` representation with length is deferred; for now `str` is equivalent to `char *` in C.

```
str greeting := "hello";
print_str(greeting);
```

### 3.6 `arr[T]` — fixed-size arrays

An array of elements of type `T`. LLVM type: `[N x T]` where `N` is the number of elements in the array literal. The size is inferred from the initializer; the type annotation `arr[int]` carries no explicit size.

```
arr[int] nums := [1, 2, 3, 4, 5];
int first := nums[0];
```

Array element type must be homogeneous: all elements of an array literal must share the same type. Accessing an element with a compile-time constant index that is out of range produces E0221. The loop variable in `for`-`in` is always `int`.

To mutate elements, the array variable must be declared `mut`:

```
mut arr[int] scores := [10, 20, 30];
scores[1] = 99;
```

`len(arr)` returns the number of elements as `int`.

### 3.7 `&T` and `&mut T` — references

References are typed pointers. LLVM type: `ptr`. At Tier 1, the pointee type is restricted to `int`; references to other types (`bool`, `float`, etc.) are rejected to avoid miscompilation while codegen tracks only a single pointee LLVM type (`i64`).

Forms:
- `&int` — shared (immutable) reference
- `&mut int` — mutable reference; required to write through the pointer

A mutable reference can only be taken from a variable declared `mut`. A reference may not be returned from a function if it was taken from a local variable (E0226); returning a reference to a parameter is allowed because the caller owns that storage.

```
mut int x := 5;
&mut int p := &mut x;
*p = 99;
int val := *p;
```

---

## 4. Statement and Expression Forms

A Eris program is a sequence of statements. Statements end with `;` unless they are control-flow constructs whose body is a block (`{ ... }`).

### 4.1 Variable declaration

```
TYPE NAME := EXPR;
mut TYPE NAME := EXPR;
```

Examples:

```
int count := 0;
mut int total := 100;
float ratio := 1.5;
bool done := false;
char grade := 'A';
str label := "value";
arr[int] data := [1, 2, 3];
```

Every variable must be declared with `:=` before use. Re-declaring a name in the same scope is E0205. Using an undeclared variable is E0201.

### 4.2 Assignment

Reassign a mutable variable with `=`. Assigning to an immutable variable is E0206.

```
mut int n := 0;
n = n + 1;
```

Array element assignment (requires `mut` on the array variable):

```
mut arr[int] a := [1, 2, 3];
a[0] = 99;
```

Assign through a mutable reference:

```
mut int x := 0;
&mut int p := &mut x;
*p = 42;
```

### 4.3 Function declaration

```
RETURN_TYPE NAME(PARAM_LIST) { BODY }
```

The return type is mandatory for non-void functions. A function with no return value has no return-type annotation. All parameters are written as `TYPE PARAM_NAME`. Parameters are immutable by default; use `mut TYPE PARAM_NAME` to allow assignment inside the body. Reference parameters use `&int` or `&mut int` as the type.

```
int add(int a, int b) {
    return a + b;
}

int clamp(int val, int lo, int hi) {
    if val < lo { return lo; }
    if val > hi { return hi; }
    return val;
}

int reset(mut int n) {
    n = 0;
    return n;
}
```

Functions must be declared before they are called. Recursive calls are supported as long as the declaration precedes the recursive call site (which is always true inside the function body).

### 4.4 Return statement

```
return EXPR;
return;
```

Every non-void function must have a `return` on every control-flow path (E0209). Returning without a value from a non-void function is E0219.

### 4.5 `if` expression

```
if CONDITION { THEN_BODY }
if CONDITION { THEN_BODY } else { ELSE_BODY }
```

The condition must have type `bool`. Both branches of a two-armed `if` must produce the same type when the `if` is used as an expression. A one-armed `if` used as a statement is allowed.

```
if x > 0 {
    print_str("positive");
}

int abs_val := if x >= 0 { x } else { -x };
```

### 4.6 `while` loop

```
while CONDITION { BODY }
```

The condition must have type `bool`. Loops indefinitely until the condition is false or a `break` statement is executed.

```
mut int i := 0;
while i < 10 {
    print_int(i);
    i = i + 1;
}
```

### 4.7 `for`-`in` loop

```
for VAR in START..END { BODY }
```

`START` and `END` must be `int` expressions. The range is half-open: `VAR` takes each value `START`, `START+1`, ..., `END-1`. The loop variable `VAR` is implicitly typed as `int`, is immutable, and is scoped to the loop body.

```
mut int sum := 0;
for i in 0..10 {
    sum = sum + i;
}
```

Only integer ranges (`start..end`) are iterable. Iterating any other expression is E0220.

### 4.8 `break` and `continue`

`break` exits the immediately enclosing `while` or `for` loop. `continue` advances to the next iteration. Both are illegal outside a loop body (E0207, E0208).

```
mut int i := 0;
while true {
    if i >= 5 { break; }
    i = i + 1;
}
```

### 4.9 Block as expression

A block `{ STMTS... EXPR }` evaluates to the value of its trailing expression. The trailing expression has no terminating `;`. Statements inside a block that are terminated with `;` are evaluated for side effects only.

```
int result := {
    int tmp := factorial(5);
    tmp * 2
};
```

The block introduces a new scope; variables declared inside are not visible outside.

### 4.10 Array literal

```
[EXPR, EXPR, ...]
```

All elements must share the same type. The size is inferred from the element count.

```
arr[int] primes := [2, 3, 5, 7, 11];
```

### 4.11 Array index (read and write)

Read: `ARRAY[INDEX]` — index must be `int`. If the index is a compile-time integer constant and the array size is known at compile time, an out-of-range index is E0221. Dynamic indices are not bounds-checked at compile time.

Write: `ARRAY[INDEX] = VALUE` — requires the array variable to be `mut`.

```
arr[int] a := [10, 20, 30];
int x := a[1];          -- x == 20

mut arr[int] b := [1, 2, 3];
b[0] = 99;
```

### 4.12 Take-ref and deref

Take a shared reference: `&LVALUE` — produces `&int`.
Take a mutable reference: `&mut LVALUE` — produces `&mut int`; the lvalue must be a `mut` variable.
Dereference: `*EXPR` — reads through a `&int` or `&mut int` reference.

The target of `&` or `&mut` must be an lvalue: a named variable, an array element expression, or a dereference expression. Taking a reference to a literal or temporary is E0223. Taking a mutable reference to an immutable variable is E0224. Dereferencing a non-reference expression is E0225.

```
mut int val := 10;
&mut int r := &mut val;
*r = 20;
int copy := *r;
```

---

## 5. Built-in Functions

Built-in functions are pre-declared in the global scope. They cannot be redefined.

| Name         | Signature                       | Behavior                                              |
|--------------|---------------------------------|-------------------------------------------------------|
| `print_int`  | `print_int(int) -> ()`          | Print a decimal integer followed by a newline         |
| `print_bool` | `print_bool(bool) -> ()`        | Print `true` or `false` followed by a newline         |
| `print_char` | `print_char(char) -> ()`        | Print a single character (no newline)                 |
| `print_str`  | `print_str(str) -> ()`          | Print a null-terminated string (no trailing newline)  |
| `println`    | `println() -> ()`               | Print a newline character                             |
| `read_int`   | `read_int() -> int`             | Read one integer from stdin; blocks until input       |
| `len`        | `len(arr[T]) -> int`            | Return the number of elements in an array             |

`len` is polymorphic over the element type `T`; it accepts any array. All other builtins have fixed types. LLVM lowering: `print_int`, `print_bool`, `print_char`, `print_str`, and `println` are implemented as calls to a C runtime helper linked in via `clang`. `read_int` calls a C `scanf`-based helper. `len` is lowered to a compile-time constant derived from the LLVM array type.

---

## 6. Error Codes

Diagnostics carry a structured code in the range E0001–E0226. The phase column is derived from the numeric range: codes E0001–E0099 are lexer errors, E0100–E0199 are parser errors, and E0200–E0299 are analyzer errors.

| Code   | Name                   | Phase    | Fields                           |
|--------|------------------------|----------|----------------------------------|
| E0001  | InvalidIdentifier      | lexer    | `identifier`                     |
| E0002  | InvalidToken           | lexer    | `token`                          |
| E0101  | UnexpectedToken        | parser   | `expected`, `found`              |
| E0102  | LexerErrorInParse      | parser   | `detail`                         |
| E0201  | UndeclaredVariable     | analyzer | `name`                           |
| E0202  | UndeclaredFunction     | analyzer | `name`                           |
| E0203  | TypeMismatch           | analyzer | `expected`, `found`              |
| E0204  | IncorrectArgumentCount | analyzer | `expected`, `found`              |
| E0205  | VariableAlreadyDeclared| analyzer | `name`                           |
| E0206  | AssignmentToImmutable  | analyzer | `name`                           |
| E0207  | BreakOutsideLoop       | analyzer | _(none)_                         |
| E0208  | ContinueOutsideLoop    | analyzer | _(none)_                         |
| E0209  | MissingReturn          | analyzer | `function`                       |
| E0219  | ReturnMissingValue     | analyzer | `function`                       |
| E0220  | NonIterableInForLoop   | analyzer | `found`                          |
| E0221  | IndexOutOfRange        | analyzer | `array_size`, `index`            |
| E0222  | IndexNotInteger        | analyzer | `found`                          |
| E0223  | TakeRefOfNonLvalue     | analyzer | _(none)_                         |
| E0224  | TakeMutRefOfImmutable  | analyzer | `name`                           |
| E0225  | DerefOfNonReference    | analyzer | `found`                          |
| E0226  | ReturnsLocalReference  | analyzer | _(none)_                         |

Diagnostics are emitted to stderr in human-readable text by default. Pass `--format=json` to receive a JSON array conforming to `schemas/diagnostic-envelope.json`. The JSON form includes `code`, `severity`, `message`, `location`, and a `fields` object whose keys match the "Fields" column above.

---

## 7. Design Rules and Idioms

### No type inference

Every variable declaration requires an explicit type keyword. Writing `x := 5` without a preceding type is a parse error. The type goes before the name: `int x := 5;`, not `x: int = 5`.

### Mutability opt-in

Variables are immutable by default. To allow reassignment, prefix the declaration with `mut`:

```
int a := 1;        -- immutable; a = 2 would be E0206
mut int b := 1;    -- mutable; b = 2 is fine
```

Function parameters follow the same rule. An immutable parameter may be read but not assigned inside the function body.

### Expression-oriented blocks

Any `{ ... }` block can serve as an expression if its last item has no `;`. This enables inline initialization:

```
int result := {
    int tmp := expensive_call(n);
    tmp * tmp
};
```

An `if`-`else` expression may similarly appear on the right-hand side of a declaration:

```
int max_val := if a > b { a } else { b };
```

### `for`-`in` vs `while`

Use `for VAR in START..END { ... }` when iterating a known integer range; the loop variable is automatically scoped and cannot be reassigned. Use `while COND { ... }` when the exit condition is dynamic or when you need to step by values other than 1.

### `mut int n` vs `&mut int n`

A `mut int` parameter receives a copy. Mutations inside the function body do not affect the caller's variable. To update the caller's variable through a reference, declare the parameter as `&mut int` and pass `&mut VAR` at the call site:

```
-- Value copy: caller's x is unchanged
int double_copy(mut int n) {
    n = n * 2;
    return n;
}

-- Reference: caller's x is modified through the pointer
int set_via_ref(&mut int p, int v) {
    *p = v;
    return 0;
}
```

Note: Tier 1 restricts reference pointees to `int` only. Attempting to take a reference to a `bool`, `float`, or other non-`int` variable is rejected with E0203 until a future release extends codegen support.

### Arrays are fixed-size

Once declared, an array's size is fixed. There is no `push`, `pop`, or dynamic resize. The size is known at compile time and encoded in the LLVM type. Use `len(arr)` to retrieve the size at runtime in generic contexts.

### Explicit references

References (`&T`, `&mut T`) must be taken explicitly with `&` or `&mut`. There is no automatic borrowing or coercion. A reference outlives the scope of the variable it points to only if the variable is a parameter (the caller owns the storage). Returning a reference to a local variable is E0226.

---

## 8. Worked Examples

All examples below compile against the Eris v0.8.0 compiler without modification.

### 8.1 Factorial (recursive)

```
int factorial(int n) {
    if n <= 1 {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

int main() {
    int result := factorial(6);
    print_int(result);
    return 0;
}
```

Expected output: `720`.

### 8.2 Sum of a range (`for`-`in`)

```
int sum_to(int n) {
    mut int total := 0;
    for i in 0..n {
        total = total + i;
    }
    return total;
}

int main() {
    int s := sum_to(10);
    print_int(s);
    return 0;
}
```

Expected output: `45` (sum of 0 through 9).

### 8.3 Counter with `while` and `break`

```
int main() {
    mut int n := 0;
    while true {
        if n >= 5 { break; }
        print_int(n);
        n = n + 1;
    }
    return 0;
}
```

Expected output: `0 1 2 3 4` (each on its own line).

### 8.4 Mutation through a reference

```
int main() {
    mut int x := 5;
    &mut int p := &mut x;
    *p = *p + 1;
    print_int(*p);
    return *p;
}
```

Expected output: `6`. Demonstrates taking a `&mut int` reference, reading through it with `*p`, and writing back.

### 8.5 Maximum value in an array

```
int main() {
    arr[int] nums := [3, 1, 4, 1, 5, 9, 2, 6];
    mut int max_val := nums[0];
    mut int i := 1;
    while i < len(nums) {
        if nums[i] > max_val {
            max_val = nums[i];
        }
        i = i + 1;
    }
    print_int(max_val);
    return 0;
}
```

Expected output: `9`. Uses `len` to avoid hardcoding the array size.

### 8.6 Insertion sort (in-place)

```
-- Sort [5, 2, 4, 6, 1, 3] in place; return a[2] (= 3) as exit code.
int main() {
    mut arr[int] a := [5, 2, 4, 6, 1, 3];
    mut int i := 1;
    while i < len(a) {
        mut int j := i;
        while j > 0 {
            mut int prev := j - 1;
            if a[prev] > a[j] {
                mut int tmp := a[prev];
                a[prev] = a[j];
                a[j] = tmp;
                j = j - 1;
            } else {
                j = 0;
            }
        }
        i = i + 1;
    }
    return a[2];
}
```

This is the canonical `examples/array_sort.eris`. After sorting, `a[2]` is `3`.

---

## 9. Compiler Invocation

The Eris compiler is invoked through the `eris-cli` binary in the Cargo workspace.

```
cargo run -p eris-cli -- FILE.eris
```

**Options:**

| Flag                | Behavior                                               |
|---------------------|--------------------------------------------------------|
| `-o DIR`            | Write the binary (and object file) to `DIR`; defaults to the current working directory |
| `--emit-ir`         | Also write an LLVM IR file `NAME.ll` alongside the binary |
| `--format=json`     | Emit diagnostics as a JSON array to stderr             |

**Exit codes:**

- `0` — compilation succeeded; the binary was produced.
- Non-zero — compilation failed; error details go to stderr.

The binary produced by the compiler is a native executable linked by `clang`. The `main` function's return value becomes the process exit code.

**Example session:**

```
cargo run -p eris-cli --quiet -- -o /tmp examples/for_range_sum.eris
# Produces /tmp/for_range_sum
/tmp/for_range_sum   # exit code 0; prints 45
```

**JSON diagnostics:**

```
cargo run -p eris-cli --quiet -- --format=json bad.eris 2>&1 | jq '.'
```

Each element of the JSON array conforms to `schemas/diagnostic-envelope.json` and includes `code` (e.g., `"E0203"`), `severity`, `message`, `location` (`line`, `column`), `hint`, and `fields`.
