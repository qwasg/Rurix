# 00 · Installation & toolchain

[English](00_install.en.md) · [简体中文](00_install.md)

> API-convergence notice (RD-008): commands and artifact shapes may change between versions.

## Environment prerequisites

| Item | Requirement |
|---|---|
| Operating system | Windows 11 (native COFF/PE/PDB toolchain) |
| GPU | NVIDIA (reference machine: RTX 4070 Ti) |
| Driver / runtime | CUDA Toolkit + CUDA Driver API |
| C++ toolchain | MSVC 2022 |
| Build host | The Rust toolchain (Rurix itself is built with Rust, D-201) |

> A bare `rx check` (pure front-end static checking) does **not** need a GPU; only execution paths such as `rx run` / `rx bench` require a real device.

## Building the toolchain

At the repository root:

```sh
cargo build --workspace
```

The most-used artifact is `rx` (the toolchain CLI). The examples in this tutorial use a debug build of `rx`:

```sh
cargo run -p rx -- <subcommand> ...
# or run the artifact directly: target/debug/rx (rx.exe on Windows)
```

## A tour of the `rx` subcommands

| Subcommand | Purpose | Exit-code convention (RXS-0083) |
|---|---|---|
| `rx check <input.rx>` | Full front-end static checking only (borrow / resource / type); no codegen (RXS-0086) | 0 = pass, 1 = diagnostic error |
| `rx build <input.rx> [-o <out>] [--emit=<target>]` | Compile; produces a host EXE by default, `--emit` can produce `ptx`/`pyd`/`mir`/`llvm-ir` (RXS-0084) | 0 = success |
| `rx run <input.rx> [-o <out>]` | Build, then execute the artifact, **passing through the artifact's exit code** (RXS-0085) | passthrough |
| `rx fmt [--check-idempotent] <file>` | Format / idempotency check | 0 = idempotent |
| `rx test [<file>] [--gpu]` | Discover and run `#[test]` / `#[test(gpu)]` | 0 = all passed |
| `rx bench <name> [--smoke]` | Protocolized microbenchmark | passthrough |
| `rx doc --root . --out target/doc` | Generate the reference documentation site from the single source of truth | 0 = success |

## Run your first command

The tutorial's first example is already in the repository — verify your environment first:

```sh
cargo run -p rx -- check conformance/tutorial/01_hello.rx   # expect exit 0
cargo run -p rx -- run   conformance/tutorial/01_hello.rx -o build/hello.exe
```

When you see the greeting printed (`你好,Rurix`, i.e. "Hello, Rurix"), your environment is ready. Next → [01 Your first program](01_first_program.en.md).

---

In-depth reference: `spec/toolchain.md` (RXS-0083~, the `rx` CLI semantics; Chinese-only).
