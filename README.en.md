# Rurix

> Give GPU systems programming a Rust of its own.

[English](README.en.md) · [简体中文](README.md)

**Rurix** is a standalone, statically compiled GPU systems-programming language and toolchain. It promotes *resource ownership, address spaces, and the parallel execution hierarchy* to first-class citizens of the type system, so graphics and GPU-compute programs gain **statically provable safety, predictable performance, and a governable long-term ecosystem** — without giving up CUDA-level low-level control.

CUDA-first, Windows-native, single-stack NVIDIA done deep: the backend emits PTX, and the runtime talks directly to the CUDA Driver API.

> **Language note:** the in-depth design dossier (`01`–`14`), the testable specification (`spec/`), and the milestone contracts are currently Chinese-only. For English readers, [`OVERVIEW.en.md`](OVERVIEW.en.md) distills the whole dossier into a single page, and the [`guide/`](guide/README.en.md) tutorial is available in English. This page, plus [`OVERVIEW.en.md`](OVERVIEW.en.md), [`CONTRIBUTING.en.md`](CONTRIBUTING.en.md), [`SECURITY.en.md`](SECURITY.en.md), and [`CODE_OF_CONDUCT.en.md`](CODE_OF_CONDUCT.en.md), are the English entry points. Contributions that translate more of the corpus are welcome (see *Contributing* below).

---

## What it solves

| Today's pain | Rurix's answer |
|---|---|
| GPU memory/concurrency safety rests entirely on humans (CUDA C++) or all-`unsafe` device code (Rust-CUDA) | Rust-style ownership on the host layer + execution-resource / view / address-space types on the device layer; structured parallelism is statically proven race-free, while weakly-ordered protocols are explicit `unsafe` with verification obligations |
| host/device resource lifetimes blow up at runtime (cross-thread `cuCtxDestroy`, stream-ordered-allocation use-after-free) | Context/Stream/Event/Buffer are **affine types** — lifetime errors become **compile errors** |
| Toolchains silently degrade and compile permissively | **strict-only**: a lowering failure is a structured compile error; capability bits are driven by real device probing |
| GPU development is a second-class citizen on Windows | Native COFF/PE/PDB/Authenticode toolchain + first-class CUDA Driver API runtime |
| Three languages and three type systems for host C++ / shader / kernel | **One language, two layers**: host and kernel share the type system, generics, and module system; the compiler statically checks launch boundaries |
| Ecosystems grow chaotically + AI hallucinates APIs | A triangle of spec-clause numbers ↔ conformance tests ↔ PRs that must cite them; package management with no arbitrary build scripts |

The full argument lives in [`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md) and [`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md) (Chinese).

## Project status: MVP + G1 + G2 complete (`g2-closed`)

The first-layer full acceptance (01 §6) is met. The three flagship use cases run end-to-end on real hardware, performance criteria are satisfied, the resource-lifetime error classes are 100% intercepted at compile time, and every budget threshold is `measured_local` (zero `estimated`):

- **UC-01 — PyTorch operator replacement**: `rx build --emit=pyd` produces a PYD (nanobind + scikit-build-core), zero-copy-bridged into PyTorch CUDA tensors over both `__cuda_array_interface__` v3 and DLPack; SAXPY/Reduction/GEMM operator replacements reach **≥ 90% of hand-written CUDA C++** (measured_local).
- **UC-02 — three-stream overlapped pipeline**: affine Context/Stream/Event/Buffer + cross-thread ownership transfer + typed stream-ordered allocation; the four resource-lifetime error classes (use-after-free / double-free / cross-thread / cross-stream-unsynchronized) are **intercepted at compile time**.
- **UC-03 — SPH simulation + compute soft rasterizer**: a single executable — particle update + spatial hashing + rasterization kernels + host frame loop — producing deterministic images.
- **cublas binding package**: three-layer GEMM/GEMV bindings (raw FFI / safe wrapper / high-level API).
- **Release pipeline**: rurixup + MSI + winget + Azure Artifact Signing (Authenticode) + SBOM (SPDX/CycloneDX) + NVIDIA redistribution-whitelist audit.
- **Bilingual diagnostics with full coverage** (Chinese/English) + **documentation site** (`rx doc`).

**Since the MVP, the G1 and G2 phases have both closed.** **G1** (`g1-closed`, PR #77): CUDA–D3D12 interop with real-time windowed present (RFC-0001), stream-ordered `AsyncBuffer` allocation (MR-0001), a first engine integration via a Rurix C-ABI DLL embedded in a C++/D3D12 harness (MR-0002), open-source community infrastructure plus a `geometry` crate (MR-0003/0004), and production fatbin distribution (MR-0005). **G2** (`g2-closed`, PR #117): the shader-stage type surface (RFC-0002, RXS-0153–0156), a DXIL backend (D-131 adjudicated = **hybrid**: compute via direct LLVM-DirectX emit / graphics via SPIR-V→DXIL), binding-layout derivation, a UC-04 deferred renderer + texture sampling (RFC-0006/0007), and a stable API + edition (RFC-0008, RD-008). Separately, an out-of-tree **GRX showcase** — a Godot 4.7-dev D3D12 integration/demo spike (**not a core-roadmap milestone**) — reached gated, opt-in, *measured* real-D3D12-dispatch compute passes with pixel-exact LDR parity (`max_abs = 0`); honest ceiling: **default-disabled / fallback-only, no performance claim, Amdahl 1.0669× hard ceiling**.

> Stable-API snapshot freeze: the MVP close-out keeps it `not_frozen` (the public surface is still converging); the mechanism activates at the first stable release ([`RD-008`](registry/deferred.json)).

## Workspace

| Crate | Responsibility |
|---|---|
| `src/rurixc` | Compiler (frontend + MIR + NVPTX backend + borrow/resource checks + formatter + LSP session) |
| `src/rurix-rt` | Runtime (CUDA Driver API bindings, execution resources) |
| `src/rx` | Toolchain CLI (`build`/`check`/`run`/`fmt`/`bench`/`test`/`doc`/`watch`/`vendor`) |
| `src/rurix-pkg` | Package management (lockfile + vendor + checksum) |
| `src/rurix-interop` | PyTorch interop (PYD / `__cuda_array_interface__` / DLPack boundary) |
| `src/rurix-cublas` | cublas v2 binding package |
| `src/rurixup` | Installer / bootstrapper (release pipeline) |
| `src/image-io` · `src/soft-raster` | Image I/O · compute soft-rasterizer library |
| `src/uc02-demo` · `src/uc03-demo` | Flagship use-case demos |

## Getting started

**Environment**: Windows 11 + an NVIDIA GPU (reference machine: RTX 4070 Ti), the CUDA Toolkit, and MSVC 2022. The Rurix toolchain itself is built with Rust (D-201).

```sh
# Build the workspace
cargo build --workspace

# Use the rx toolchain
cargo run -p rx -- build <manifest>      # compile (emit PTX / PYD)
cargo run -p rx -- check <manifest>      # check only (borrow / resource / type)
cargo run -p rx -- bench saxpy           # microbenchmark (BENCH_PROTOCOL sampling)
cargo run -p rx -- doc --root . --out target/doc   # generate the documentation site
```

The documentation site (`rx doc`) is generated deterministically from a single source of truth (`spec/*.md`, `registry/error_codes.json`, `conformance/`): a spec-clause index, an error-code index, and a traceability matrix.

**Want to learn how to write Rurix code?** See the [`guide/`](guide/README.en.md) tutorial (available in English) — a progressive path from your first host program to your first kernel, with every example exercised live by CI gates (`rx check` / `rx run`). (API is converging; see [`RD-008`](registry/deferred.json).)

## Governance & quality gates

Rurix builds governance in as a product capability from day one (language infrastructure for the AI era; see [`10_GOVERNANCE.md`](10_GOVERNANCE.md)):

- **Spec ↔ test ↔ PR triangle**: every RXS spec clause is anchored by ≥1 test (`ci/trace_matrix.py`).
- **measured_local budgets**: all performance/diagnostics baselines are measured on real hardware, with zero `estimated` placeholders (`ci/budget_eval.py --strict`).
- **Real red-green**: every CI gate is validated by "introduce a defect → red → restore → green" (anti-YAML-only), with run URLs archived in [`evidence/`](evidence/).
- **Byte-level guardrails**, schema validation, structure validation, all-green conformance, and blessed UI/MIR/PTX goldens.
- **deferred / spike-gating registries**: the single source of truth for deferred debt and expansion directions — append-only.

Milestone contracts and close-out trails live in [`milestones/`](milestones/); the governance mechanism overview is in [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md).

## Statement of restraint

Rurix does **not** replace the CUDA ecosystem (it provides a safe compile frontend and runtime on top of it), does **not** lead with cross-platform support (single-stack NVIDIA done deep first), and does **not** build an ML framework (it interoperates zero-copy with PyTorch via DLPack). Each act of restraint maps to a verified "dead route" ([`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md) §4).

## Documentation map

`00_MASTER_INDEX.md` is the master index; `01`–`14` are the planning dossier (vision / positioning / design principles / language & compiler architecture / GPU programming model / runtime & toolchain / standard library & ecosystem / governance / roadmap / engineering discipline). `spec/` is the testable specification (FLS-style, RXS clauses), and `conformance/` is the sole acceptance boundary. These are currently Chinese-only; for a single-page English distillation of `01`–`14`, see [`OVERVIEW.en.md`](OVERVIEW.en.md), and the per-file English summaries below are a quick map.

| File | Topic |
|---|---|
| [`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md) | Vision & mission: why Rurix should exist |
| [`02_USERS_AND_USE_CASES.md`](02_USERS_AND_USE_CASES.md) | Target users & use cases; flagship use cases; adoption criteria |
| [`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md) | Positioning & competitive landscape; gap market; "dead route" red lines |
| [`04_DESIGN_PRINCIPLES.md`](04_DESIGN_PRINCIPLES.md) | 14 numbered, citable design axioms |
| [`05_LANGUAGE_ARCHITECTURE.md`](05_LANGUAGE_ARCHITECTURE.md) | Two-layer model, type system, ownership, address spaces, generics, modules, FFI |
| [`06_GPU_GRAPHICS_PROGRAMMING_MODEL.md`](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) | Kernel abstraction, memory-model mapping, synchronization, three-phase graphics roadmap |
| [`07_COMPILER_ARCHITECTURE.md`](07_COMPILER_ARCHITECTURE.md) | IR layering, query-based compilation, borrow checking, NVPTX codegen, diagnostics |
| [`08_RUNTIME_AND_TOOLING.md`](08_RUNTIME_AND_TOOLING.md) | Driver API object model, Windows toolchain, LSP, dev tools |
| [`09_STDLIB_AND_ECOSYSTEM.md`](09_STDLIB_AND_ECOSYSTEM.md) | core/std layering, math library, Buffer, interop, package management |
| [`10_GOVERNANCE.md`](10_GOVERNANCE.md) | Governance & project organization: change gates, RFCs, stability, AI-contribution policy |
| [`11_ROADMAP.md`](11_ROADMAP.md) | Roadmap: MVP scope, milestone sequence, 3-year / 5-year vision |
| [`12_RISKS.md`](12_RISKS.md) | Risk register: six risk classes; probability / impact / mitigation |
| [`13_DECISION_LOG.md`](13_DECISION_LOG.md) | Decision log: every major decision numbered and registered |
| [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md) | Engineering discipline: milestone contracts, guardrails, budget gates, evidence tiers, deferred model |

**Reading paths**: *only 15 minutes* → 01 → 04 → 13; *evaluate whether the project is sound* → 01 → 03 → 12 → 11; *contribute to language design* → 04 → 05 → 06 → 13; *contribute to the compiler* → 04 → 07 → 14 → 05.

## Contributing

Contributions are welcome. Please first read [`CONTRIBUTING.en.md`](CONTRIBUTING.en.md) (the spec↔test↔PR triangle, change tiers, the AI-contribution policy, and `unsafe` discipline) and [`CODE_OF_CONDUCT.en.md`](CODE_OF_CONDUCT.en.md); for security issues, see [`SECURITY.en.md`](SECURITY.en.md).

## License

Dual-licensed, at your option (D-003):

- Apache License 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT License ([`LICENSE-MIT`](LICENSE-MIT))

`SPDX-License-Identifier: MIT OR Apache-2.0`. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in this project shall be dual-licensed as above, with no additional terms.
