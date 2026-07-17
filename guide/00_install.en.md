# 00 · Installation & toolchain

[English](00_install.en.md) · [简体中文](00_install.md)

> Language 1.0 has shipped (v1.0.0): the stable surface (including the `rx` command surface) is frozen — additive-only within an edition (RD-008 closed).

## Environment prerequisites

| Item | Requirement |
|---|---|
| Operating system | Windows 11 (native COFF/PE/PDB toolchain) |
| GPU | NVIDIA (reference machine: RTX 4070 Ti) |
| Driver / runtime | CUDA Toolkit + CUDA Driver API |
| C++ toolchain | MSVC 2022 (only needed at link time for `rx build` / `rx run`; `rx check` has zero such prerequisite) |
| Build host | The Rust toolchain (**only needed for Option B, building from source**; Option A prebuilt install has zero Rust prerequisite, D-201) |

> A bare `rx check` (pure front-end static checking) does **not** need a GPU nor any of the system-level prerequisites above; only execution paths such as `rx run` / `rx bench` require a real device. GPU / MSVC / CUDA are **documented system-level prerequisites and are not counted toward install time** (same framing as rustup).

## Option A: install the prebuilt toolchain (rurixup, recommended)

1. Download `rurixup.exe` from [GitHub Releases](https://github.com/qwasg/Rurix/releases) (honest bootstrap-gap note: this step is protected by TLS + manually checking `SHA256SUMS`, same shape as rustup-init; the Authenticode signature is a self-signed test certificate — defense in depth, **not** a trust root).
2. Install (via the in-repo trust-root anchor, four-level content-addressed verification; any level mismatch refuses the install):

```sh
rurixup.exe install v1.0.1-dist.1 --channel-file https://raw.githubusercontent.com/qwasg/Rurix/main/channels/stable.json
```

   On success it prints `RURIXUP_INSTALL: ... digest_levels_verified=4` and materializes the toolchain under `%USERPROFILE%\.rurix\toolchains\<version>\bin\` (including `rx.exe` and `bin\lib\rurix_rt_cabi.lib` — `rx build` works without a Rust environment).
3. PATH setup: `rurixup setup` prints the instructions (does not modify your environment by default); `rurixup setup --add-path` explicitly writes the user PATH. Switch / list versions: `rurixup default <version>` / `rurixup list`.
4. Verify: `rx check <file.rx>` exits 0.

> The currently installable version is `v1.0.1-dist.1` (the EA1.2 first release-rehearsal artifact, pre-release; the trust-root anchor `channels/stable.json` only lists versions merged through the owner's manual gate). Install time depends on your bandwidth; cold-start acceptance uses a two-segment protocol (clean VM to `rx check` / clean account to first kernel, each ≤10 minutes, measured — RFC-0012 §4.10). This document makes no unqualified duration claims.

## Option B: build from source (contributor path)

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
