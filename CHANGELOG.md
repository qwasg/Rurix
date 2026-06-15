# Changelog

This document records milestone-level changes to the Rurix project.

The format is based on [Keep a Changelog](https://keepachangelog.com/).
Rurix uses milestone-driven development (M0–M6); each milestone has a contract, plan, CI gates, and evidence.

## [M6] - Active — Toolchain & Package Management

### Added
- `rx` CLI toolchain: `build`, `run`, `check`, `test`, `fmt`, `bench`, `vendor` subcommands
- `rurix-pkg` crate: TOML manifest parsing, dependency resolution graph, `rurix.lock`, content-addressed vendor with SHA-256
- `rx vendor --locked --offline` reproducible build mode (`/Brepro` linker flag)
- `rx fmt` source formatter with `--check-idempotent` gate
- `rx test` with `#[test]` and `#[test(gpu)]` discovery, subprocess isolation for GPU tests
- LSP MVP: completion, goto definition, publish diagnostics, hover, document symbols, references
- Error codes RX7001–RX7011 (toolchain/package management segment)
- CI gates: rx CLI smoke, fmt idempotent, pkg offline resolve, offline rebuild reproducibility, LSP capabilities

### M0–M6 Review Close-out (2026-06-15)
Static-analysis review across M0–M6 tightened several conservative-approximation gaps (closed contracts unchanged; conclusions recorded here and in `registry/deferred.json`):
- Device MIR safety gate: kernel/device-fn bodies now run move + borrow checks (`check_device_safety` in driver, after host borrow check / shared coherence, before device codegen), surfacing use-after-move (RX4001), use-before-init for unwritten shared scalars (RX4002), move/assign-while-borrowed (RX4005), and dangling references (RX4006) inside device code.
- Aggregate reference/alias flow: borrow checker tracks references hidden in struct/tuple/variant aggregates (RX4005/RX4006); views checker conservatively rejects parameter/aggregate-hidden view aliasing on mutable writes (RX3007).
- Barrier / thread-id classification moved from method-name string matching to typeck `device_calls`, with thread-id taint propagation (RX3003); user-defined `sync()` no longer false-positives.
- Launch argument-count contract: tuple element count vs kernel formals (excluding ThreadCtx) mismatch now reported as RX2003 before per-element type checking (closes the old zip-truncation silent under-report).
- corpus↔driver consistency regression (`pipeline_consistency.rs`) pins driver `--emit=check` ordering against conformance `//@ expect-error` across 9 domains (3xxx/4xxx/6xxx reject).
- Governance: `ci/check_guardrails.py` guardrail #11 added — full-worktree scan rejecting `STUB(RD-###)` markers for closed RDs. RD-008 registered (deferred.json v1.8): scoped atomics PTX `atom.{order}.{scope}` mapping codegen deferred (D-406 manual implementation; not delivered in M5 beyond type contract + RX3010 + ignored skeleton).
- Deferred ledger: RD-007 inherited (owner M6→M7); RD-008 open (owner M7, D-406 manual gate). atom.{order}.{scope} PTX lowering and `atomics_ptx_mapping.rs` `#[ignore]` remain untouched.

## [M5] - Closed — Atomics, Views & Shared Memory

### Added
- `View<global, T>` / `ViewMut<global, T>` for device memory with disjointness checking (RXS-0078)
- `shared<T>` + barrier coherence checking (RXS-0079)
- Atomic operations with address space enforcement
- Launch contract checking: kernel coloring, dimensions, parameter/brand consistency (RXS-0074/0075)
- NVIDIA redistribution audit (CI guard: embedded PTX must not contain `__nv_*` symbols)
- Compute Sanitizer integration: racecheck + memcheck in nightly CI
- UI golden snapshot drift detection in nightly CI
- Traceability matrix freshness gate in CI

## [M4] - Closed — Device Codegen (NVPTX → PTX)

### Added
- Device codegen channel: MIR → NVPTX-constrained LLVM IR → PTX
- `ptxas` dry-validation gate (RXS-0073)
- PTX version negotiation at module load time (RXS-0076)
- End-to-end GPU examples: SAXPY, reduce, scan, transpose, GEMM tile
- `rurix-rt` build.rs: kernel `.rx` → PTX pipeline, embedded in host EXE data segment
- Poisoned context state machine (RXS-0077): deterministic failure after GPU errors

## [M3] - Closed — Borrow Checker & MIR

### Added
- NLL borrow checker (RXS-0057~0061): aliasing XOR mutability, borrow-during-move, dangling reference detection
- MIR (Monomorphized IR): CFG-based, explicit types, monomorphization instance collection
- Move/init dataflow checking (RXS-0054): use-after-move, use-before-init, move-out-of-ref
- Const evaluation (RXS-0062~0065): compile-time constant folding and validation
- TBIR (Typed-Body IR): temporary narrow gate for pattern exhaustiveness and method sugar
- Drop elaboration: deterministic destructor ordering
- Error codes RX4001–RX4006 (borrow/move segment)

## [M2] - Closed — Host Codegen (LLVM IR → COFF → PE EXE)

### Added
- Host codegen: MIR → text LLVM IR → clang → COFF .obj → link.exe → PE EXE
- Toolchain locator: clang 22.1.x pin, MSVC link.exe, CUDA_PATH
- Self-profile: per-phase timing JSON output
- CDB breakpoint verification (debug info quality gate)
- Hello-world compile loop smoke test
- Reproducible build support (`/Brepro` linker flag)

## [M1] - Closed — Compiler Frontend

### Added
- Lexer (RXS-0001~0010): tokenization with span tracking
- Parser: hand-written recursive descent (RXS-0011~0029)
- AST: full abstract syntax tree representation
- Name resolution (RXS-0032~0038): two-pass (collection + body walk)
- HIR (High-level IR): type-system-facing IR with item/body separation
- AST → HIR lowering with desugaring (for/?/while-let)
- Type checking: HM unification inference + signature-enforced annotation
- Feature gate checking (RXS-0010/0011)
- Function coloring check (RXS-0066/0068): host/device/kernel boundary enforcement
- Error codes RX0001–RX0010 (lexical/syntax), RX1001–RX1004 (names), RX2001–RX2009 (type check), RX3001–RX3010 (coloring/address space)
- CI gates: cargo fmt, clippy (deny warnings), cargo test, conformance corpus

## [M0] - Closed — Infrastructure & Evidence Pipeline

### Added
- Repository structure and Cargo workspace (4 crates: rurixc, rurix-rt, rx, rurix-pkg)
- CI PR Smoke pipeline (self-hosted GPU runner, RTX 4070 Ti)
- CI Nightly pipeline (scheduled + manual)
- L0 environment validation: NVML environment probe, frequency locking protocol
- Handwritten PTX + Driver API SAXPY and bandwidthTest baselines
- Contract/budget JSON templates, deferred registry, spike gating registry
- `agents/AGENTS.md` v1: AI contribution policy
- Guardrails system: 10 byte-level invariants enforced by CI
- Evidence discipline: measured_local / unlocked / estimated classification
- Error code registry: segmented scheme, frozen semantics, append-only
