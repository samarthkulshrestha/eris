# eris

> Code at the edge of order.

## introduction

Eris is a ergonomic and opinionated programming language designed for simplicity and expressiveness. It focuses on being explicit in all aspects of code, with deliberate choices to enforce a consistent developer experience.

Eris emphasizes:
- Explicit variable typing to eliminate ambiguity.
- Minimal syntax that stays out of your way, allowing you to focus on logic.
- Intentional mutability, where change is a conscious decision, not the default.

## philosophy

> Eris is a language of **deliberate acts.**
Types are explicit. Change is explicit. Thought is explicit.
In Eris, you shape the code — the code does not shape you.

## features
- static typing
- first-class mutability control
- minimal, consistent syntax
- strong, predictable scoping rules
- clear, explicit import
- first-class functions
- destructuring support
- pattern matching / smart conditionals
- strict error handling
- simple, explicit modules / namespaces

## later features
- lambda functions
- algebraic data types
- traits / interfaces
- zero-cost abstractions
- first-class immutability for collections
- minimal but clear concurrency primitives
- optional compile-time metaprogramming
- tooling-first design

## syntax

```eris
-- this is a single-line comment
-+ this is a
   multi-line
   comment +-
--- this is a doc-comment

-- variable declaration
int x := 10;
mut int y := 20;        -- vars are immutable by default
y = 20 * 2;             -- re-assign mutable values using `=`

-- multiple bindings
int a, b, c := 1, 2, 3;
(int i, int j) := (10, 20);
int (i, j) := (10, 20);
mut int (x, y) := (10, 20);
(str person_name, (int age, str address)) := ("alice", (30, "wonderland"));

-- blocks and expressions
int result := {         -- blocks return the value of the last expr without a semicolon
    int x := 10;
    x * 2
}                       -- result = 20
int x := (2 + 3) * 4;   -- evaluates to 20, following PEMDAS

-- control flow
int x := 10;
mut int y := 0;
if x > 10 {
    y = 1;
} else if x = 10 {
    y = 2;
} else {
    y = 3;
}

-- functions
int factorial(int n) {
    if n <= 1 {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}
int square(int x) {
    x * x               -- implicit return, no semicolon
}
int result := factorial(5);

-- lambdas
int add := (int x, int y) => x + y;

-- arrays
arr[int] n := [1, 2, 3]; -- fixed-length array of integers
```
