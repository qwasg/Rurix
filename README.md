# Rurix

[![pr-smoke](https://img.shields.io/github/actions/workflow/status/rurix/rurix/pr-smoke.yml?branch=main&label=pr-smoke&style=flat-square)](https://github.com/rurix/rurix/actions/workflows/pr-smoke.yml)
[![nightly](https://img.shields.io/github/actions/workflow/status/rurix/rurix/nightly.yml?branch=main&label=nightly&style=flat-square)](https://github.com/rurix/rurix/actions/workflows/nightly.yml)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square)](LICENSE)
[![rust](https://img.shields.io/badge/rust-1.93.1-orange?style=flat-square)](rust-toolchain.toml)

A GPU systems programming language — Rust's safety discipline meets CUDA's compute power.

Rurix is a statically-compiled language that gives GPU programming its own Rust: borrow-checked memory safety, host/device coloring boundaries, and compile-time correctness guarantees — all the way down to PTX.

## Features

- **Borrow-checked GPU code** — NLL borrow checking extends to device kernels; aliasing XOR mutability enforced at compile time, not by sanitizer
- **Host/device coloring** — Functions are colored `host`, `device`, or `kernel`; cross-boundary calls are statically rejected (RXS-0066)
- **Views & shared memory** — `View<global, T>` / `ViewMut<global, T>` for device memory; `shared<T>` + barrier coherence checked at compile time (RXS-0078/0079)
- **Zero external dependencies** — Compiler, runtime, and package manager have near-zero Rust crate dependencies; supply-chain attack surface is minimal
- **Evidence-driven engineering** — Every performance claim backed by measured JSON evidence; no numbers from memory
- **Spec-first development** — Language semantics defined in `spec/` before implementation; test-to-spec traceability enforced by CI

## Quick Start

### Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.93.1 | Pinned via `rust-toolchain.toml` |
| clang | 22.1.x | Host codegen backend (LLVM IR → COFF) |
| MSVC link.exe | | Windows linker |
| CUDA Driver | 525+ | Runtime; no CUDA Toolkit import lib needed |
| Python | 3.10+ | CI scripts and benchmarks |
| NVIDIA GPU | compute_89+ | Ada Lovelace or later (e.g. RTX 4070 Ti) |

### Build

```bash
# Clone
git clone https://github.com/rurix/rurix.git
cd rurix

# Build the full workspace
cargo build --workspace

# Run tests
cargo test --workspace
```

### Hello World

Create `hello.rx`:

```rurix
fn main() {
    print("Hello from Rurix!");
}
```

Compile and run:

```bash
rx build hello.rx -o hello.exe
.\hello.exe
```

### GPU Kernel Example

SAXPY (`out[i] = a * x[i] + y[i]`) in Rurix:

```rurix
kernel fn saxpy(
    out: ViewMut<global, f32>,
    x:    View<global, f32>,
    y:    View<global, f32>,
    a:    f32,
    n:    usize,
    t:    ThreadCtx<1>,
) {
    let i = t.global_id();
    if i < n {
        out[i] = a * x[i] + y[i];
    }
}
```

See [src/rurix-rt/kernels/](src/rurix-rt/kernels/) for complete end-to-end GPU examples (SAXPY, reduce, scan, transpose, GEMM).

## Architecture

```
Source (.rx)
  │
  ├─ Lexer ──► Token Stream
  ├─ Parser ──► AST
  ├─ Resolve ──► Name Resolution
  ├─ Typeck ──► Type Inference & Checking
  ├─ Coloring ──► Host/Device/Kernel Boundary Check
  ├─ Const Eval ──► Compile-time Evaluation
  ├─ MIR ──► Monomorphized IR
  ├─ Borrow Check ──► NLL Alias Analysis
  ├─ Move/Views/Shared Checks
  │
  ├──── Host Channel ────► LLVM IR (x86-64) ──► clang ──► COFF .obj ──► link.exe ──► PE EXE
  │
  └──── Device Channel ──► NVPTX IR ──► clang NVPTX ──► PTX ──► ptxas verify ──► embedded in EXE
```

### Workspace Crates

| Crate | Description |
|-------|-------------|
| [`rurixc`](src/rurixc/) | Compiler core — lexer, parser, type checker, borrow checker, MIR, codegen |
| [`rurix-rt`](src/rurix-rt/) | CUDA runtime — thin RAII wrapper over Driver API, zero external deps |
| [`rx`](src/rx/) | CLI toolchain — build, run, check, test, fmt, bench, vendor |
| [`rurix-pkg`](src/rurix-pkg/) | Package manager — TOML manifest, dependency resolution, content-addressed vendor |

## CLI Reference

```bash
rx build   <input.rx>            # Compile to host executable
rx run     <input.rx>            # Build and run
rx check   <input.rx>            # Full static analysis, no codegen
rx test    <path>                # Discover and run tests (CPU + GPU)
rx fmt     <path>                # Format source code
rx bench   <benchmark>           # Run performance benchmarks
rx vendor  [--locked] [--offline] # Resolve dependencies, vendor, write lock
```

## Project Status

Rurix is in **pre-MVP development**. The current milestone is **M6** (toolchain & package management).

| Milestone | Status | Focus |
|-----------|--------|-------|
| M0 | Closed | Infrastructure & evidence pipeline |
| M1 | Closed | Compiler frontend (lexer, parser, AST) |
| M2 | Closed | Host codegen (LLVM IR → COFF → PE EXE) |
| M3 | Closed | Borrow checker & MIR |
| M4 | Closed | Device codegen (NVPTX → PTX) |
| M5 | Closed | Atomics, views, shared memory |
| M6 | Active | Toolchain, package manager, LSP |

## Documentation

- [Language Specification](spec/) — The single source of truth for semantics (RXS-0001 ~ RXS-0097)
- [Milestone Contracts](milestones/) — Per-milestone contracts, plans, and CI gates
- [Design Documents](00_VISION.md) — 15 design-phase documents (vision, principles, architecture, governance)
- [Deep Research](deep-research/) — 12 technical research memoranda
- [RFCs](rfcs/) — Request for Comments proposals
- [Unsafe Audit](unsafe-audit/) — RustBelt-style verification obligations for all unsafe code

## License

Licensed under [Apache License 2.0](LICENSE).
