# Rurix

> Give GPU systems programming a Rust of its own.

[English](README.en.md) · [简体中文](README.md)

**Rurix** is a standalone, statically compiled GPU systems-programming language and toolchain. It promotes *resource ownership, address spaces, and the parallel execution hierarchy* to first-class citizens of the type system, so graphics and GPU-compute programs gain **statically provable safety, predictable performance, and a governable long-term ecosystem** — without giving up CUDA-level low-level control.

CUDA-first, Windows-native, single-stack NVIDIA done deep: three backends emit PTX (the runtime talks directly to the CUDA Driver API), DXIL (a native D3D12 graphics runtime), and SPIR-V (the single Vulkan/SPIR-V cross-vendor backend since MB1 — AMD desktop + Android, compute + graphics; preview, behind a default-off feature flag).

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

## Project status: language 1.0 released (`v1.0.0`); 30 milestones closed through G16 (latest tag `g16-closed`); G17 DLSS performance-gap closure in progress

The first-layer full acceptance (01 §6) is met. The three flagship use cases run end-to-end on real hardware, the resource-lifetime error classes are 100% intercepted at compile time, and every existing performance threshold is backed by `measured_local` evidence with zero `estimated` entries. The narrative below through G7 is a historical snapshot; progress after it is carried by the append-only status errata further down (latest: 2026-08-24, G9~G17):

- **UC-01 — PyTorch operator replacement**: `rx build --emit=pyd` produces a PYD (nanobind + scikit-build-core), zero-copy-bridged into PyTorch CUDA tensors over both `__cuda_array_interface__` v3 and DLPack; SAXPY/Reduction/GEMM operator replacements reach **≥ 90% of hand-written CUDA C++** (measured_local).
- **UC-02 — three-stream overlapped pipeline**: affine Context/Stream/Event/Buffer + cross-thread ownership transfer + typed stream-ordered allocation; the four resource-lifetime error classes (use-after-free / double-free / cross-thread / cross-stream-unsynchronized) are **intercepted at compile time**.
- **UC-03 — SPH simulation + compute soft rasterizer**: a single executable — particle update + spatial hashing + rasterization kernels + host frame loop — producing deterministic images.
- **UC-04 — deferred renderer (D3D12)**: the DXIL second backend (D-131 hybrid: compute via a direct minimal-subset DXIL channel, graphics via a SPIR-V→HLSL→dxc validation bridge) + binding-layout derivation (root signature RTS0) + multi-pass orchestration with anchored barriers; the lighting pass truly samples the G-buffer, accepted on real hardware via off-screen readback pixel comparison.
- **UC-07 — ruridrop, an all-`.rx` application**: `std::gpu` single-source host orchestration (one `.rx` entry → one EXE with embedded PTX+cubin); a GPU SPH dam-break simulation + sphere ray tracing, where the offline path-traced PPM and the realtime D3D12 present share the same kernel core; GPU frames match a CPU replay golden **byte-for-byte** (CI smoke tier); ~68 fps realtime at 1280×720 / 131k particles (measured_local).
- **cublas binding package**: three-layer GEMM/GEMV bindings (raw FFI / safe wrapper / high-level API).
- **Release pipeline**: rurixup (stable-channel manifest) + an Authenticode sign/verify release gate (currently a **self-signed test certificate**; the of-record production backend is Azure Artifact Signing behind a secret-gated manual step) + SBOM (SPDX/CycloneDX) + NVIDIA redistribution-whitelist audit.
- **Bilingual diagnostics with full coverage** (Chinese/English) + **documentation site** (`rx doc`).

**Since the MVP, the G1 and G2 phases have both closed.** **G1** (`g1-closed`, PR #77): CUDA–D3D12 interop with real-time windowed present (RFC-0001), stream-ordered `AsyncBuffer` allocation (MR-0001), a first engine integration via a Rurix C-ABI DLL embedded in a C++/D3D12 harness (MR-0002), open-source community infrastructure plus a `geometry` crate (MR-0003/0004), and production fatbin distribution (MR-0005). **G2** (`g2-closed`, PR #117): the shader-stage type surface (RFC-0002, RXS-0153–0156), a DXIL backend (D-131 adjudicated = **hybrid**: compute via direct LLVM-DirectX emit / graphics via SPIR-V→DXIL), binding-layout derivation, a UC-04 deferred renderer + texture sampling (RFC-0006/0007), and a stable API + edition (RFC-0008, RD-008). Separately, an out-of-tree **GRX showcase** — a Godot 4.7-dev D3D12 integration/demo spike (**not a core-roadmap milestone**) — reached gated, opt-in, *measured* real-D3D12-dispatch compute passes with pixel-exact LDR parity (`max_abs = 0`); honest ceiling: **default-disabled / fallback-only, no performance claim, Amdahl 1.0669× hard ceiling**.

**Since then, three more phases have closed.** **V1** (`v1-closed`, 2026-07-14): the first stable release of the language — tag `v1.0.0`, stabilization report, FCP-lite notice, stable-channel manifest (rurixup), and the first GitHub Release. **MS1** (`ms1-closed`, 2026-07-15): single-source host GPU orchestration (`std::gpu`, RFC-0009 — one `.rx` source produces one EXE with embedded PTX) and **ruridrop**, the first production-grade renderer/simulation written with Rurix as its primary language (application layer contains zero `.rs`; GPU frames match a CPU replay golden **byte-for-byte** in the CI smoke tier; ~68 fps realtime at 1280×720 / 131k particles, measured_local). **MB1** (`mb1-closed`, 2026-07-16): a single Vulkan/SPIR-V cross-platform backend (RFC-0011) covering AMD desktop + Android, compute + graphics; Android on-device runs are **measured on real hardware** (compute bit-exact across three vendors, windowed present + validation-clean); the AMD real-card gate (G-MB1-6) honestly stays **open pending hardware**, and the backend ships as a **preview behind a default-off feature flag** — no cross-vendor performance claim.

**Since then, five more phases have closed.** **G3** (`g3-closed`, 2026-07-19): the industrial-rendering phase — the RD-027 poison-path attribution gate plus the full five-feature surface (sampling superset / bindless / render-graph automatic barriers / UC-04 windowed present / mesh-task-RT dual backends). **EI1** (`ei1-closed`, 2026-07-23): the engine-integration phase — UC-05 minimal RHI + render-graph core (the U5 flagship use case) and RD-009 `#[export(c)]` C-ABI export codegen with built-in header generation (D-113). **G4** (`g4-closed`, 2026-07-24): the engine-rendering phase — a graphics RHI raster/mesh library surface + automatic barriers + engine_host v3 embedding + a single-source `.rx` Vulkan RHI channel + BLACKHOLE production-tier acceptance (RD-036 stays open). **EA1** (`ea1-closed`, 2026-07-28): the distribution & storefront phase — real rurixup distribution (RD-025 redeemed) + prebuilt toolchain bundles (the `v1.0.1-dist` series, pre-release) + the documentation storefront + cold-start acceptance. **G5** (closed per contract §8.1, 2026-07-29): the native-renderer phase — a declarative render graph (`rurix-render`), an RHI graphics dispatch bridge, virtualized geometry (meshlets / two-level GPU culling / VisBuffer), VSM shadows, screen-probe GI, ray-traced effects, material streaming, and temporal reconstruction (TAA/TSR), with the UC-06 full-pipeline demo running on device (P3+ long-tail items registered as RD-037+; RD-038 wave redemption in progress).

**G6** (closed per contract §8.2, 2026-08-01) added the production-default Jolt physics library, a default-off Rapier fast path, a one-way Physics→GpuScene bridge, the UC-08 confluence demo, and the Taichi Vulkan AOT effects side track. **G7** (active since 2026-08-01) is [Production Frame Closure](milestones/g7/G7_CONTRACT.md): compute SPIR-V 1.4/RayQuery, real W3 GI/RTAO/hard-shadow kernels, VisBuffer software/hardware raster parity, a literal RD-038 residual audit, and one continuously connected real device frame.

> **Status erratum (2026-08-08, append-only)**: the heading and the paragraph above are snapshots — **G7 closed on 2026-08-05** (`g7-closed`, RD-038 closed) and **G8 closed on 2026-08-06** (`status: closed`, close-out flip commit `b4189e79`; see [`milestones/g8/G8_CONTRACT.md`](milestones/g8/G8_CONTRACT.md) close-out section for per-gate terminal states, including honest no-go/defer/degraded entries kept on record rather than rewritten as PASS). The stale lines are kept verbatim per append-only discipline.

> **Status erratum (2026-08-24, append-only)**: since the erratum above, **eight more milestones have closed** on the UE5-benchmarking mainline (facts of record live in `milestones/*/G*_CONTRACT.md` close-out sections and `registry/`; this line is an index mirror only). **G9** (2026-08-15): UE5-target rendering/physics platform phase. **G10** (2026-08-16): UE5 visual-benchmark baseline phase — an 11-row gap register locked as G11's statutory input. **G11** (2026-08-17): GI & lighting quality closure — re-test register final state converged 8 + aligned_closed 3. **G12** (2026-08-17): path-tracing productionization — 10-row gap register locked as G13's statutory input. **G13** (2026-08-19): vendor upscaling & Lumen comparison — DLSS SR (Streamline 2.10.3 NGX Vulkan) / FSR 3.1.5 / TSR device lane, dual gap registers (8+2 rows) locked as G14/G15 statutory input. **G14** (2026-08-23): formal frame-rate parity & render-pipeline performance — M-d **18/18 pass at the ×1.00 line**, empty-register terminal state. **G15** (2026-08-23): image-quality closure & commercial final review — **double miss honestly recorded, not dressed up**: commercial sign-off 0/18 on the absolute-quality line and performance 17/18 (single-cell environment event), with three carry-over anchors handed to G16+. **G16** (2026-08-24, tag `g16-closed`, incl. G16plus): UE reference-arm repair (cornell arm no longer dead-black) and forced quality closure — M-g absolute quality 18/18 under a programmatically calibrated threshold while the historical G15 M-c 0/18 record stays unrewritten. **G17 (DLSS performance-gap closure, the statutory carrier of G15-MD-F1, `milestones/g17/`) is now in progress**: chartered and implementation-unlocked on 2026-08-24 (governance gates at CI steps 293–295; RFC-0032 D3D12-host NGX lane Agent Approved after D-409 adversarial review; measured baseline = an honest-red 17/18 rerun of the G14 M-d gate, focus cell bistro-interior/t100/dlss_sr ratio 0.9810; implementation gates materialized at steps 296–308). **G17.2 M-a (dual-end re-test & warm-state recalibration) is accepted** — ten facts green plus the wave-2 aggregate gate, four-round re-test window focus-cell ratios [0.981, 0.8157, 0.7966, 0.8086] recorded as registration (the pass/fail verdict belongs to the M-d final-verdict gate). **As of this line, waves G17.3–G17.7b are not yet accepted** (the same-day campaign session keeps executing; wave terminal states land append-only in contract §8); the single-cell performance miss stays honestly recorded, and both final outcomes (18/18, or a maintained miss registration if the physical floor makes ×1.00 unreachable) are legitimate ways to close (contract §7, adjudication 5).

> Stable-API snapshot freeze has been **active since the 1.0 release** ([`RD-008`](registry/deferred.json) closed): the stable surface (spec clause IDs + error-code meanings + edition values + the `rx` CLI command set) is anchored by snapshot comparison with bless-gated approval — additive-only within an edition; breaking changes require a new edition.

## Workspace

| Crate | Responsibility |
|---|---|
| `src/rurixc` | Compiler (frontend + MIR + NVPTX/DXIL/SPIR-V backends + borrow/resource checks + formatter + LSP session) |
| `src/rurix-rt` | Runtime (CUDA Driver API bindings, execution resources) |
| `src/rurix-rt-cabi` | Host-orchestration C-ABI runtime boundary (`rxrt_*`/`rxp_*`/`rxio_*`: single-source `.rx` apps ↔ the runtime — fatbin loading / launch / present / image dump) |
| `src/rx` | Toolchain CLI (`build`/`check`/`run`/`fmt`/`bench`/`test`/`doc`/`vendor`) |
| `src/rurix-pkg` | Package management (lockfile + vendor + checksum) |
| `src/rurix-interop` | PyTorch interop (PYD / `__cuda_array_interface__` / DLPack boundary) |
| `src/rurix-cublas` | cublas v2 binding package |
| `src/rurixup` | Installer / bootstrapper (release pipeline) |
| `src/rurix-d3d12` | D3D12/DXGI present shim (the CUDA–D3D12 interop realtime-present boundary) |
| `src/rurix-engine` | Engine-integration DLL (C-ABI cdylib; embedded in C++/D3D12 hosts to run compute passes) |
| `src/rurix-geometry` | Geometry library (mesh/BVH, zero-dependency, all-safe) |
| `src/rurix-android-present` | Android on-device present glue (MB1; zero-Java NativeActivity cdylib shell, compiles to an empty lib on desktop) |
| `src/rurix-render` | Native engine renderer library (G5: declarative render graph / virtualized geometry / VSM / probe GI / ray-traced effects / material streaming / temporal reconstruction; the renderer is a library, not part of the language) |
| `src/rurix-geom-build` | Offline geometry builder (G5: mesh → meshletization → grouped-and-simplified hierarchical DAG + a CPU reference culler; deterministic all-safe host code) |
| `src/image-io` · `src/soft-raster` | Image I/O · compute soft-rasterizer library |
| `src/uc02-demo` · `src/uc03-demo` · `src/uc04-demo` | Flagship use-case demos |
| `apps/uc06-renderer` | UC-06 full-pipeline demo (G5: culling → VisBuffer → deferred shading → GI/VSM/RTAO → TAA/TSR → headless readback pixel assertions) |
| `apps/ruridrop` | UC-07 all-`.rx` application (renderer/simulation in one; not a Cargo crate — a declarative `rurix.toml` package, zero `.rs`) |

## Getting started

**Environment**: Windows 11 + an NVIDIA GPU (reference machine: RTX 4070 Ti), the CUDA Toolkit, and MSVC 2022. The Rurix toolchain itself is built with Rust (D-201).

```sh
# Build the workspace
cargo build --workspace

# Use the rx toolchain
cargo run -p rx -- build <input.rx>      # compile (produces a host EXE; --emit=ptx / pyd etc.)
cargo run -p rx -- check <input.rx>      # check only (borrow / resource / type)
cargo run -p rx -- bench saxpy           # microbenchmark (BENCH_PROTOCOL sampling)
cargo run -p rx -- doc --root . --out target/doc   # generate the documentation site
```

The documentation site (`rx doc`) is generated deterministically from a single source of truth (`spec/*.md`, `registry/error_codes.json`, `conformance/`): a spec-clause index, an error-code index, and a traceability matrix.

**Want to learn how to write Rurix code?** See the [`guide/`](guide/README.en.md) tutorial (available in English) — a progressive path from your first host program to your first kernel, with every example exercised live by CI gates (`rx check` / `rx run`). (API is converging; see [`RD-008`](registry/deferred.json).)

## Governance & quality gates

Rurix builds governance in as a product capability from day one (language infrastructure for the AI era; see [`10_GOVERNANCE.md`](10_GOVERNANCE.md)):

- **Spec ↔ test ↔ PR triangle**: every RXS spec clause is anchored by ≥1 test (`ci/trace_matrix.py`, currently 278/278).
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
