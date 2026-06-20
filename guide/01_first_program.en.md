# 01 · Your first program

[English](01_first_program.en.md) · [简体中文](01_first_program.md)

> API-convergence notice (RD-008): the syntax may change between versions. All three examples in this chapter are exercised live by the CI gates `rx check` + `rx run`.

This chapter walks through the language basics on the host side: the program entry point, bindings, control flow, structs, and function coloring. GPU kernels are left for the [next chapter](02_first_kernel.en.md).

## 1. Hello, Rurix

Single source of truth: [`conformance/tutorial/01_hello.rx`](../conformance/tutorial/01_hello.rx)

```rurix
fn main() {
    let greeting = "你好,Rurix";
    println(greeting);
}
```

(The string literal is a greeting — "Hello, Rurix"; the bundled, CI-tested example prints `你好,Rurix`.)

`fn main()` is the entry point of an executable target (a file with no `main` makes `rx check` report `RX6002: no main function found for executable target`). `let` introduces an immutable binding; a string literal plus `println` is all you need to print.

```sh
cargo run -p rx -- check conformance/tutorial/01_hello.rx       # static check only, exit 0
cargo run -p rx -- run   conformance/tutorial/01_hello.rx -o build/hello.exe   # build and execute
```

`rx check` runs the full front end (borrow / resource / type) but emits no codegen — this is the fastest feedback loop while you write code. `rx run` passes through the artifact's exit code (RXS-0085).

## 2. Bindings, control flow, structs

Single source of truth: [`conformance/tutorial/02_host_basics.rx`](../conformance/tutorial/02_host_basics.rx)

```rurix
struct Point {
    x: f32,
    y: f32,
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    let origin = Point { x: 0.0, y: 0.0 };
    let mut total = 0;
    for i in 0..5 {
        total = add(total, i);
    }
    if total > 3 {
        println("total 偏大");       // "total is large"
    } else {
        println("total 偏小");       // "total is small"
    }
}
```

Key points (these should look familiar to Rust users):

- `struct` defines an aggregate type; construct one with a literal, `Point { x: .., y: .. }`.
- Bindings are immutable by default; reassignment requires `let mut`.
- `for i in 0..5` iterates over a range; `if/else` is expression-style control flow.
- The last expression of a function body (no semicolon) is its return value (`a + b`).

## 3. Function coloring: host / device / const

Single source of truth: [`conformance/tutorial/03_fn_colors.rx`](../conformance/tutorial/03_fn_colors.rx)

Rurix uses **function coloring** (RXS-0014) to distinguish where code executes, while sharing one type system and one module system — unlike CUDA C++, which keeps a separate set for host and device:

```rurix
device fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

const fn inc(n: usize) -> usize {
    n + 1
}

fn main() {
    let mid = lerp(0.0, 10.0, 0.5);
    println("device fn 可从 host 调用");   // "a device fn can be called from the host"
}
```

- **`fn`** (host): an ordinary host function.
- **`device fn`**: a device-side function, callable **one-directionally** from both host and kernel (avoiding the combinatorial explosion of CUDA's `__host__ __device__`).
- **`const fn`**: evaluable at compile time.
- **`kernel fn`**: a GPU entry point — see the [next chapter](02_first_kernel.en.md).

A full sample with all four colorings side by side is in [`conformance/syntax/fn_colors.rx`](../conformance/syntax/fn_colors.rx).

---

Next → [02 Your first kernel](02_first_kernel.en.md)

In-depth reference (Chinese-only): `spec/lexical.md` (identifiers/keywords/literals, RXS-0004~), `spec/names.md` (function items & coloring, RXS-0014).
