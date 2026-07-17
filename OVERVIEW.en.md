# Rurix — Design Overview

[English] · the in-depth dossier (`01`–`14`) and the testable `spec/` are Chinese-only; this page distills them for English readers.

> **What this is.** A single, self-contained English distillation of the Chinese design dossier ([`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md) – [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md)) and the testable specification (`spec/`). It is a *reference for understanding*, not a normative spec — where this page and `spec/` disagree, `spec/` wins. Each section links to the Chinese source for the full argument. For a hands-on path, read the [tutorial](guide/README.en.md) instead.

---

## 1. What Rurix is

**Rurix is a standalone, statically compiled GPU systems-programming language and toolchain.** It promotes *resource ownership, address spaces, and the parallel execution hierarchy* to first-class citizens of the type system, so graphics and GPU-compute programs gain **statically provable safety, predictable performance, and a governable long-term ecosystem** — without giving up CUDA-level low-level control.

It is **CUDA-first, Windows-native, single-stack NVIDIA done deep**: the primary backend emits PTX and the runtime talks directly to the CUDA Driver API; a DXIL backend drives the native D3D12 graphics path, and since MB1 a single Vulkan/SPIR-V cross-platform backend (AMD desktop + Android; preview, default-off feature) exists alongside them.

**Status: language 1.0 released (`v1.0.0`); MVP + G1 + G2 + V1 + MS1 + MB1 all closed (baseline `mb1-closed`).** The three flagship use cases run end-to-end on real hardware, performance criteria are met, the targeted resource-lifetime error classes are 100% intercepted at compile time, and every budget threshold is `measured_local` (zero `estimated`). The **G1** phase closed (`g1-closed`) — CUDA–D3D12 interop with real-time windowed present, stream-ordered allocation, a first engine integration, and production fatbin distribution — then **G2** (`g2-closed`, PR #117) — the shader-stage type surface, a DXIL backend, binding-layout derivation, a UC-04 deferred renderer + texture sampling, and a stable API + edition. **V1** (`v1-closed`, 2026-07-14) shipped the first stable release of the language (tag `v1.0.0`, first GitHub Release). **MS1** (`ms1-closed`) delivered single-source host GPU orchestration (`std::gpu`) and **ruridrop**, the first production-grade renderer/simulation with Rurix as its primary language (zero `.rs` in the app layer). **MB1** (`mb1-closed`, 2026-07-16) delivered the Vulkan/SPIR-V cross-platform backend — Android on-device runs measured on real hardware; the AMD real-card gate honestly stays open pending hardware (see §7). Full vision and acceptance criteria: [`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md).

## 2. The problem, and the thesis

GPU programming today forces a bad trade. You either write CUDA C++ — where memory and concurrency safety rest entirely on humans — or you bolt a host language's type system onto the device and discover it doesn't fit (Rust-CUDA's device code ends up essentially all `unsafe`, because `&mut [T]` wrongly implies exclusivity when thousands of invocations share the buffer).

Rurix's thesis is that the *structured* part of GPU programming — resource lifetimes, address-space isolation, regular data partitioning, barrier legality — can be made **statically safe without performance loss**, while the genuinely *unstructured* high-performance idioms (weakly-ordered atomic protocols, runtime-indexed aliasing, inline PTX) remain reachable through explicit `unsafe` with written verification obligations. This split is taken from the research record (Descend proves the safe path is viable and codegen-equivalent to hand CUDA; RustBelt supplies the `unsafe`-with-obligations methodology). Positioning and the "dead routes" Rurix deliberately refuses are in [`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md).

## 3. Design principles (P-01 – P-14)

Fourteen numbered axioms underpin every later decision; designs, RFCs, and reviews cite them by number, and changing one is a Full-RFC-level change. Full text & evidence: [`04_DESIGN_PRINCIPLES.md`](04_DESIGN_PRINCIPLES.md).

| # | Principle | In one line |
|---|---|---|
| P-01 | **strict-only** | No silent degradation; every failure path is a structured, error-coded diagnostic. Leniency exists only as explicit `unsafe` or an explicit feature gate. |
| P-02 | **GPU-first** | Address spaces, execution hierarchy, and resource ownership are type-checked language constructs, not comments or runtime conventions. |
| P-03 | **Safe but not neutered** | The static safety envelope covers structured patterns; everything outside it stays reachable via `unsafe` + verification obligations. Experts never lose control of the hardware. |
| P-04 | **Capabilities from real probing** | Hardware/driver/toolkit capability bits come from runtime probing or official query APIs (NVML first) — never from configuration or assumption. |
| P-05 | **Explicit over implicit** | Copies, pinned staging, synchronization, and launch config are explicit. Unified Memory / zero-copy are opt-in with their platform caveats labeled. |
| P-06 | **Static compilation, predictable performance** | AOT to PE/COFF (host) + PTX/cubin (device); no JIT as language semantics. You can reason about generated code from source. |
| P-07 | **Observability first** | Every compiler stage and runtime resource op ships with counters and timing buckets from day one. Optimization needs an observation point *before* an implementation. |
| P-08 | **Diagnostics are a product** | Structured diagnostics (span/label/note/help/suggestion + error codes + `--explain` + JSON) and UI goldens are built from the first diagnostic. The diagnostics data structure is the LSP's only data source (single front end). |
| P-09 | **Evidence before performance milestones** | No performance claim ships before its `measured_local` evidence channel exists; `estimated` placeholders survive at most two milestones. |
| P-10 | **Public-surface freeze discipline** | The public surface (syntax, stable stdlib API, C ABI) changes only via RFC + feature gate + conformance test; internals evolve freely. |
| P-11 | **Single source of truth, generated views** | Anything needing "consistency in many places" (spec, API docs, error-code index, bindings) has one structured source; the rest are generated. More than two hand-maintained mirrors is a design error. |
| P-12 | **Scope restraint, formally** | Tempting directions (multi-backend, autodiff, fusion, proc-macros, registry, Python embedding) are explicitly registered as "not doing" with trigger conditions — not silently shelved. |
| P-13 | **Anti-AI-hallucination governance** | Humans are accountable, provenance is traceable, the spec leads the implementation. AI may not define UB / memory model / FFI ABI; acceptance numbers must come from command output. |
| P-14 | **Windows & WDDM are first-class** | The WDDM/TCC/MCDM driver model, TDR, HAGS, DLL search order, and Authenticode/SmartScreen are *design inputs*, not compatibility patches. |

**Tie-breaks when principles conflict:** P-03 (control) over ergonomics; P-09 (evidence) over schedule; P-12 (restraint) over completeness; and P-01 vs P-03 never actually conflict (`unsafe` is an explicit channel, not silent leniency).

## 4. Language architecture

Full design and the rejected alternatives: [`05_LANGUAGE_ARCHITECTURE.md`](05_LANGUAGE_ARCHITECTURE.md).

### 4.1 One language, two execution worlds

Rurix is a single language whose code runs in two semantically distinct worlds that **share one type system, generics, module system, const-eval, and diagnostics**:

- **Host layer** — a full systems language (ownership/borrowing/traits/heap/FFI/std), compiled to x86-64 COFF/PE. Owns resource management, kernel scheduling, I/O, interop.
- **Kernel sub-language (device)** — a restricted subset plus device extensions (execution-hierarchy types, address spaces, views), compiled to PTX today (DXIL in the G2 phase). Owns data-parallel compute.

### 4.2 Function coloring

The difference between the two worlds is expressed by **function coloring** + capability checks, statically enforced — not by two grammars:

| Coloring | Meaning |
|---|---|
| `fn` | Host function (default). |
| `kernel fn` | GPU entry point; callable only via the launch API; signature carries the execution shape. |
| `device fn` | Device-callable function; callable one-directionally from host and kernels; default force-inlined in the MVP (no device call-stack management; recursion is a compile error). |
| `const fn` | Compile-time-evaluable; callable from both layers. |

A function needing both sides is written `device fn` and used without host-only capabilities; the host can call it directly (device ⊂ host's callable set). This avoids CUDA C++'s `__host__ __device__` combinatorial explosion. Rurix deliberately does **not** do Mojo-style implicit dual-target compilation (it violates P-05).

### 4.3 Type system

Rust-style traits + generic bounds + monomorphization, heavily trimmed for the MVP. Native types include `i8…u64`/`usize`, `f16`/`f32`/`f64`/`bf16` (half/bf16 are first-class; `f64` is lint-flagged in device code as ~64× slower on consumer GPUs), `bool`/`char` (host-only `char`), and **language-built-in** `Vec2/3/4<T>` / `Mat2/3/4<T>` (column-major canonical, with swizzles and compiler layout/alignment guarantees). Key built-in traits:

| Trait | Semantics |
|---|---|
| `Copy` / `Clone` | As in Rust. |
| `DeviceCopy` | Bit-copyable to the device (no host pointers/refs/handles); the required bound for kernel value parameters. |
| `Record` | Pure-data type whose C-ABI mirror and serialization view can be generated by the compiler (the P-11 "state mirror by compiler" vehicle). |
| `Drop` | Destructor; the release hook for affine resource types. |

**Not in the MVP** (explicitly registered): trait objects (`dyn`), specialization, HKT, language-level `async` (GPU async goes through the stream/event model), proc-macros (a permanent red line — arbitrary compile-time code execution is a supply-chain + hallucination risk), and Polonius (kept out on `rustc`'s strongest warning).

### 4.4 Ownership & borrowing — a two-layer model

- **Host:** Rust-style affine ownership + NLL borrow checking implemented as MIR/CFG dataflow (**not Polonius**). Move-by-default, `&T` shared / `&mut T` exclusive, explicit lifetime params with elision (no HRTB in the MVP).
- **Device:** the hard problem is not single-thread aliasing but *thousands of threads concurrently writing the same memory*. Rurix uses the Descend-verified approach: **execution-resource types** (`Grid`/`Block`/`Warp`/`Thread` as type-level entities), **borrow narrowing** (mutable access narrows down the execution hierarchy, so each thread holds a `&mut` to a statically disjoint slice), and **views as the only safe narrowing syntax**. Bypassing views with arbitrary indexed mutation requires `unsafe`.

The safety envelope's boundary is explicit — for example, lifetime safety, address-space non-confusion, view-partitioned race-free writes, and barrier reachability are *static guarantees*, while inline PTX, runtime-decided shared mutable writes, custom weakly-ordered atomic protocols, and cross-kernel global sync protocols are the `unsafe`-with-obligations side.

### 4.5 Affine resources & context-brand lifetimes

GPU resources (`Context`, `Stream`, `Event`, `Buffer`, `Module`) are modeled as **affine types** (move-only, no `Copy`/`Clone`, with `Drop`). The key move: **context ownership is encoded as a lifetime parameter (a "brand")**. `Stream<'ctx>`, `Event<'ctx>`, `DeviceBuffer<T>` (carrying `'ctx` internally), etc. make two rules — "a resource must not outlive its context" and "must not be misused across contexts" — fall out of borrow checking. This directly defuses the classic Driver-API "nukes": a `cuCtxDestroy` while resources are still borrowed becomes a compile error, and a cross-context event record/wait becomes a lifetime mismatch.

### 4.6 Launch type contract, views, errors, generics, modules, FFI

- **Launch** is a fully typed API: argument tuples match the kernel signature at compile time; const-generic execution shapes are checked at compile time and runtime shapes get structured validation. Kernel parameter ABI follows PTX `.param` rules (over the 32764-byte cap is a compile error suggesting buffer indirection).
- **Views** adopt Descend's operator set — `split::<N>()`, `group::<K>()`, `transpose()`, `reverse()`/restricted `map_idx`, `zip()` — proven to cover transpose/reduce/scan/histogram/GEMM at hand-CUDA-level codegen. Mutable views are obtained only through the execution resource that owns the decomposition path (`per_block()` / `per_thread()`), giving statically disjoint `&mut`.
- **Errors:** host uses `Result<T,E>` + `?` (no exceptions; `panic = abort` in the MVP). Device has no panic/Result; it uses three channels — compile-time (shape/address-space/borrow), debug-runtime (out-of-bounds/assert → device trap → structured error + *poisoned context*), and release (documented UB on `unsafe` paths). FFI boundaries return C-ABI error codes.
- **Generics:** all monomorphized (each kernel instance is its own PTX symbol); const generics (tile sizes, unroll factors, shapes) with a MIR-interpreter const-eval subset.
- **Modules/packages:** file = module, `use` imports, `pub`/`pub(package)`/private visibility, no header files. A package is a `rurix.toml` + `rurix.lock`; host and device code live in the same module tree — putting a kernel next to its scheduling code is the core ergonomic payoff of the two-layer single language.
- **FFI:** `extern "C"` + raw pointers + `#[repr(C)]` for imports (always `unsafe`); `#[export(c)]` for exports with **compiler-built header generation** (cbindgen's role internalized — P-11). Windows x64 ABI only. No C++ ABI interop, ever; Python goes through C ABI + nanobind, not a language-level binding.

## 5. GPU & memory model

Full semantics: [`06_GPU_GRAPHICS_PROGRAMMING_MODEL.md`](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md).

- **Execution model:** `Grid → Block (→ Warp) → Thread` as type-level entities; the host submits via `Stream` (FIFO-ordered within a stream); cross-stream concurrency is connected explicitly by `Event`. MVP baseline is `compute_89` (Ada / RTX 4070 Ti); PTX is JIT-loaded by the Driver API as a *loading mechanism*, not language semantics.
- **MVP kernel scope:** 1D/2D/3D grids/blocks, POD scalar + view parameters. No dynamic parallelism, cooperative groups, clusters, or Tensor Core intrinsics (all registered as spike-gated).
- **Explicit data movement:** the default is explicit H2D/D2H + pinned staging. `ctx.alloc` (`cuMemAlloc`) → `DeviceBuffer`; `ctx.alloc_pinned` (`cuMemAllocHost`) → `PinnedBuffer`; `stream.copy(src, dst)` (`cuMemcpyAsync`) with **direction inferred from types** (no direction enum to get wrong). `MappedBuffer`/`ManagedBuffer` are opt-in with Windows-semantics warnings. Stream-ordered allocation (`AsyncBuffer<'stream, T>`, `cuMemAllocAsync`) is a **G1-phase** type, deferred to do the classic path correctly first.
- **Synchronization, three layers:** structured/safe (`block.sync()` barrier; grid-level sync = splitting kernels); scoped atomics/safe-but-restricted (`Atomic<T, Scope>` with `Scope ∈ {Block, Gpu, System}`, lowered to `atom.{order}.{scope}`); weakly-ordered protocols/`unsafe` (`fence`, volatile/mmio, custom spin/queue protocols). The source-to-PTX mapping layer is a core spec concern: `Atomic<T, Scope>` is guaranteed to compile to *morally strong* PTX instruction pairs, and safe code cannot construct mixed-size conflicting accesses. Safe code built from views + barriers + scoped atomics is **data-race-free** (a soundness proposition anchored by conformance tests); `unsafe` race semantics follow the PTX axioms with the UB boundary written into the spec.
- **Host–device sync:** `Event<'ctx>` (`record`/`wait`/`synchronize`, context-consistency type-guaranteed); `stream.synchronize()` blocks the host; timing standardizes on CUDA Events with a `Stream::timed_scope` that flushes the queue around the timed region (WDDM batching distorts timing).

## 6. Compiler & runtime

Compiler internals: [`07_COMPILER_ARCHITECTURE.md`](07_COMPILER_ARCHITECTURE.md). Runtime & tooling: [`08_RUNTIME_AND_TOOLING.md`](08_RUNTIME_AND_TOOLING.md). Stdlib & ecosystem: [`09_STDLIB_AND_ECOSYSTEM.md`](09_STDLIB_AND_ECOSYSTEM.md).

- **Compiler:** layered IR (AST → HIR → MIR/TBIR), query-based compilation with in-process memoization, NLL borrow check on MIR, MIR → LLVM (host COFF; NVPTX subset → PTX with `ptx_kernel`/addrspace/sreg intrinsics, `ptxas` dry-validation gate). Diagnostics are the single front end shared with the LSP (no second semantic engine). `rx fmt` and UI goldens guard against drift.
- **Runtime:** Driver-API-only object model (`Device` → `Context` → Stream/Module/Buffer/Event), PTX embedded in the executable's data section and loaded explicitly, all Driver-API errors mapped to a structured `enum CudaError` with a *poisoned context* state machine, and an `environment()` snapshot (driver version / WDDM-TCC-MCDM / HAGS / TDR) for programs and benchmark harnesses.
- **Toolchain & tools:** the `rx` CLI (`build`/`check`/`run`/`fmt`/`test`/`bench`/`doc`/`watch`/`vendor`), an LSP + VS Code extension, a native Windows COFF/PE/PDB toolchain, and a release pipeline (rurixup + MSI/winget + Authenticode signing + SBOM).

## 7. Graphics roadmap (the three phases)

```
G0 (within the MVP)     G1 (≈ MVP + 6 months)      G2 (3-year horizon)
compute soft raster  →  CUDA–D3D12 interop      →  native D3D12 + DXIL
offscreen output        real-time present (window)   full graphics pipeline
zero graphics-API dep   D3D12 only as present        raster/RT/mesh in-language
```

Each phase must keep a "produces an image" capability to feed motivation and demo value, while the language reserves extension points for graphics semantics. **G0** is a pure-CUDA-kernel offscreen soft rasterizer (the UC-03 acceptance demo; a real stress test for views/shared/atomics). **G1** imports D3D12 backbuffers via `cuImportExternalMemory`/`cuImportExternalSemaphore` (new affine types `ExternalBuffer`/`ExternalSemaphore`; D3D12 driven by a thin C FFI) to get real-time windowed present without shader codegen. **G2** (an owner-decided path, D-131) adds a DXIL backend and shading stages (`vertex/fragment/mesh/task` + RT stages) as extensions of the kernel sub-language, with binding layouts derived by the compiler from kernel signatures.

**Delivery status (2026-07-14).** All three phases have landed. **G0** shipped inside the MVP (`m8-closed`). **G1** closed (`g1-closed`, PR #77): CUDA–D3D12 interop with real-time windowed present (RFC-0001), stream-ordered `AsyncBuffer` allocation (MR-0001), a first engine integration — a Rurix C-ABI DLL embedded in a C++/D3D12 harness (MR-0002) — open-source community infrastructure plus a `geometry` crate (MR-0003/0004), and production fatbin distribution (MR-0005). **G2** closed (`g2-closed`, PR #117): the shader-stage type surface (RFC-0002), a DXIL backend (RFC-0003/0004; the D-131 path is adjudicated to a **hybrid** — compute uses direct LLVM-DirectX emit, graphics uses SPIR-V→DXIL), binding-layout derivation, a UC-04 deferred renderer + texture sampling (RFC-0006/0007), and a stable API + edition (RFC-0008, RD-008). Separately, an out-of-tree **GRX showcase** — a Godot 4.7-dev D3D12 integration/demo spike on branch `codex/grx-godot-dxil-workspace`, **not a core-roadmap milestone** — reached gated, opt-in, *measured* real-D3D12-dispatch compute passes (luminance / tonemap / SSAO-blur / TAA-resolve / particles-copy / cluster-store / GPU-culling / fused-post-chain) with pixel-exact LDR visual parity (`max_abs = 0`). Honest ceiling: those passes are **default-disabled / fallback-only**, writeback is still scaffolding with no net benefit, and the Amdahl headroom is a hard **1.0669×** — **no FPS or performance improvement is claimed**.

**Update (2026-07-16).** Three more phases have since closed. **V1** (`v1-closed`): the first stable release of the language — tag `v1.0.0`, stabilization report, FCP-lite notice, stable-channel manifest, first GitHub Release. **MS1** (`ms1-closed`): single-source host GPU orchestration (`std::gpu`, RFC-0009) and **ruridrop** (RFC-0010) — the first production-grade renderer/simulation with Rurix as its primary language: application layer zero `.rs`, one `.rx` source tree → one EXE, GPU frames byte-identical to a CPU-replay golden in the CI smoke tier, ~68 fps realtime (measured_local). **MB1** (`mb1-closed`): a single Vulkan/SPIR-V cross-platform backend (RFC-0011, AMD desktop + Android, compute + graphics) — Android on-device compute is bit-exact across three vendors and windowed present runs validation-clean on a real device; the AMD real-card acceptance gate (G-MB1-6) honestly remains **open pending hardware**; the backend is a **preview behind a default-off feature flag** with no cross-vendor performance claim.

## 8. Governance & engineering discipline

This is where Rurix is most unusual: governance is built in **as a product capability from day one** — "language infrastructure for the AI era." Full text: [`10_GOVERNANCE.md`](10_GOVERNANCE.md) and [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md).

- **The spec ↔ test ↔ PR triangle.** The sole acceptance boundary is `conformance/`, not a PR description. A semantic PR must cite a clause number `RXS-####`; a semantic change lacking a clause must add the spec first (and the clause PR precedes the implementation PR); every spec clause is anchored by ≥1 test (`ci/trace_matrix.py`).
- **Change tiers.** *Direct* (no semantic-surface change), *Mini-RFC* (small semantic surface or rule-file edits), *Full RFC* (a new language feature / semantic surface). When in doubt, round up to stricter.
- **`measured_local` budgets.** All performance/diagnostics baselines are measured on real hardware with zero `estimated` placeholders (`ci/budget_eval.py --strict`), following a benchmark protocol (clock lock + three independent process-level runs + trimmed mean), with evidence appended (never edited) to `evidence/`.
- **Real red-green.** Every CI gate is validated by "introduce a defect → red → restore → green" (anti-YAML-only), with run URLs archived.
- **AI-contribution policy (D-406, in force for everyone).** Human-in-the-loop approval; provenance tags (`Assisted-by: <tool>:<model>`); AI may **not** define UB clauses, memory-model mappings, FFI ABIs, or safety-envelope boundaries — those are human-only via Full RFC.
- **`unsafe` discipline.** The whole repo defaults to `unsafe_code = deny`; every `unsafe` block carries a `// SAFETY:` comment referencing an `unsafe-audit/` registry entry (one operation per block); an unregistered `unsafe` block is a CI error.
- **Single source of truth + generated views**, byte-level guardrails, schema/structure validation, blessed UI/MIR/PTX goldens, and append-only **deferred / spike-gating registries** for deferred debt and expansion directions.

## 9. Scope restraint & risks

Roadmap and the MVP red lines: [`11_ROADMAP.md`](11_ROADMAP.md). Risk register: [`12_RISKS.md`](12_RISKS.md). Decision log (every major decision numbered): [`13_DECISION_LOG.md`](13_DECISION_LOG.md).

Rurix is as defined by what it refuses as by what it does. It does **not** replace the CUDA ecosystem (it provides a safe compile front end + runtime on top), does **not** lead with cross-platform support (single-stack NVIDIA done deep first), and does **not** build an ML framework (it interoperates zero-copy with PyTorch via DLPack / `__cuda_array_interface__`). Explicitly *not in the MVP*, each registered with trigger conditions: auto kernel fusion, autodiff, multi-backend (AMD/Intel/Metal/Vulkan), proc/decl macros, a package registry, native Python embedding, Tensor Core / cluster / dynamic-parallelism intrinsics, the Graph API, multi-GPU, cross-session incremental compilation, trait objects/specialization/HKT/async.

## 10. The Chinese dossier map & reading paths

| File | Topic |
|---|---|
| [`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md) | Why Rurix should exist; acceptance criteria |
| [`02_USERS_AND_USE_CASES.md`](02_USERS_AND_USE_CASES.md) | Target users, flagship use cases, adoption criteria |
| [`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md) | Positioning, gap market, "dead route" red lines |
| [`04_DESIGN_PRINCIPLES.md`](04_DESIGN_PRINCIPLES.md) | The 14 numbered, citable design axioms |
| [`05_LANGUAGE_ARCHITECTURE.md`](05_LANGUAGE_ARCHITECTURE.md) | Two-layer model, types, ownership, address spaces, generics, modules, FFI |
| [`06_GPU_GRAPHICS_PROGRAMMING_MODEL.md`](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) | Kernels, memory-model mapping, synchronization, the graphics roadmap |
| [`07_COMPILER_ARCHITECTURE.md`](07_COMPILER_ARCHITECTURE.md) | IR layering, query compilation, borrow check, NVPTX codegen, diagnostics |
| [`08_RUNTIME_AND_TOOLING.md`](08_RUNTIME_AND_TOOLING.md) | Driver-API object model, Windows toolchain, LSP, dev tools |
| [`09_STDLIB_AND_ECOSYSTEM.md`](09_STDLIB_AND_ECOSYSTEM.md) | core/std layering, math library, Buffer, interop, package management |
| [`10_GOVERNANCE.md`](10_GOVERNANCE.md) | Change gates, RFCs, stability, the AI-contribution policy |
| [`11_ROADMAP.md`](11_ROADMAP.md) | MVP scope, milestone sequence, 3-year / 5-year vision |
| [`12_RISKS.md`](12_RISKS.md) | Six risk classes; probability / impact / mitigation |
| [`13_DECISION_LOG.md`](13_DECISION_LOG.md) | Every major decision numbered and registered |
| [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md) | Milestone contracts, guardrails, budget gates, evidence tiers |

The `spec/` directory is the testable specification (FLS-style, `RXS-####` clauses), and `conformance/` is the sole acceptance boundary.

**Reading paths:** *15 minutes* → 01 → 04 → 13; *evaluate whether the project is sound* → 01 → 03 → 12 → 11; *contribute to language design* → 04 → 05 → 06 → 13; *contribute to the compiler* → 04 → 07 → 14 → 05.

---

## Where to go next

- **Use it:** the [tutorial](guide/README.en.md) — install → first program → first kernel → resource lifetimes.
- **Contribute:** [`CONTRIBUTING.en.md`](CONTRIBUTING.en.md) and [`CODE_OF_CONDUCT.en.md`](CODE_OF_CONDUCT.en.md).
- **Report a vulnerability:** [`SECURITY.en.md`](SECURITY.en.md).
- **The pitch:** [`README.en.md`](README.en.md).

*This overview is a derived English view of the Chinese dossier; if it drifts from `01`–`14` or `spec/`, those are authoritative.*
