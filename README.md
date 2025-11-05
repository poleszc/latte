# Latte Language Compiler

## Project Description

This project is an implementation of a compiler for **Latte**, a simple imperative language, as part 
of the "Methods of Programming Language Implementation" (MRJP) course. The project is developed in 
**Rust** and compiles Latte code to **LLVM IR**.

## Tech Stack
*   **Implementation Language:** Rust
*   **Parser:** [LALRPOP](https://crates.io/crates/lalrpop)
*   **Error Reporting:** [Ariadne](https://crates.io/crates/ariadne) for generating user-friendly compiler error messages.
*   **Lexer:** LALRPOP's default lexer or (optionally) [Logos](https://crates.io/crates/logos) for better error handling.
*   **LLVM IR Generation:** [Inkwell](https://crates.io/crates/inkwell) (pending approval from the course instructor to use a wrapper instead of raw string generation).

## Project Structure
<!-- TODO: Explain that here -->

## Compiler Usage
<!-- TODO: Explain that here -->


## Task List (Checklist)

The following table lists the features to be implemented, divided into stages, along with their point values (for the LLVM backend) and a space to track progress.

| Feature / Task                                   | Status      | Points (LLVM) | Notes                                                                                                         |
| ------------------------------------------------ | ----------- | :-----------: | ------------------------------------------------------------------------------------------------------------- |
| **Stage 1: Frontend**                            |             |      **4**      | _Required for passing. Points are awarded only if a working backend is submitted._                            |
| Lexical Analysis & Parsing                       | `[ ] To Do` |       -       | Using LALRPOP.                                                                                                |
| AST (Abstract Syntax Tree)                       | `[ ] To Do` |       -       | Represents the program after parsing.                                                                         |
| Semantic Analysis (Type Checking)                | `[ ] To Do` |       -       | Checks types, variable declarations, and function correctness.                                                |
| Scope and Shadowing Management                   | `[ ] To Do` |       -       | Handles local variables and function arguments.                                                               |
| User-Friendly Error Reporting                    | `[ ] To Do` |       -       | Using Ariadne.                                                                                                |
| **Stage 2: Backend (LLVM)**                      |             |      **8**      | _Basic code generation._                                                                                      |
| Code Generation for Functions & Expressions      | `[ ] To Do` |       -       | `int`, `boolean`, arithmetic and logical operators.                                                           |
| Code Generation for Control Flow                 | `[ ] To Do` |       -       | `if`, `while`, `return`.                                                                                      |
| Code Generation for Variables & Assignments      | `[ ] To Do` |       -       | Stack allocation for variables (`alloca`).                                                                    |
| String Handling                                  | `[ ] To Do` |       -       | Literals, concatenation (+).                                                                                  |
| Predefined Functions (`runtime.bc`)              | `[ ] To Do` |       -       | `printInt`, `printString`, `readInt`, `readString`, `error`.                                                  |
| **Optimizations (LLVM)**                         |             |               |                                                                                                               |
| Use Registers and `phi` instructions             | `[ ] To Do` |     **+2**    | Instead of `alloca` for variables that can live in registers (SSA form).                                      |
| Optimizations (LCSE/GCSE, loops, inlining)       | `[ ] To Do` |    up to 10   | _Likely not planned for this project, as LLVM handles these._                                                 |
| **Language Extensions**                          |             |               |                                                                                                               |
| Arrays and `for-each` loops                      | `[ ] To Do` |     **+1**    | `new int[size]`, `arr.length`, `for (int x : arr)`.                                                           |
| Structs (classes without methods)                | `[ ] To Do` |     **+2**    | `class C { int field; }`, `new C`, `c.field`, `null`.                                                         |
| Objects (methods, inheritance without override)  | `[ ] To Do` |     **+3**    | Methods, `extends`, superclass = subclass assignments.                                                        |
| Virtual Methods (method overriding)              | `[ ] To Do` |     **+3**    | V-tables for classes, dynamic dispatch for method calls.                                                      |
| Nested Functions                                 | `[ ] To Do` |     **+4**    | Access to non-local variables (static chains). _May be challenging in LLVM._                                  |
| **Extensions - Memory Management**               |             |               | _Choose one of the following two._                                                                            |
| Garbage Collection                               | `[ ] To Do` |     **+2**    | Simple reference counting for strings/objects.                                                                |
| Rust-style Memory Management                     | `[ ] To Do` |     **+2**    | Ownership and borrowing model for strings/objects.                                                            |
| **Other (not graded for MRJP)**                  |             |               |                                                                                                               |
| Simple FFI for `int` types                       | `[ ] To Do` |       -       | Ability to call simple C functions.                                                                           |

---
**Status Legend:**
*   `[ ] To Do`
*   `[/] In Progress`
*   `[x] Done`