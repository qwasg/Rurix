# Rurix

> Give GPU systems programming a Rust of its own.

[English](README.en.md) · [简体中文](README.md)

**Rurix** is a standalone, statically compiled GPU systems-programming language and toolchain. It promotes *resource ownership, address spaces, and the parallel execution hierarchy* to first-class citizens of the type system, so graphics and GPU-compute programs gain **statically provable safety, predictable performance, and a governable long-term ecosystem** — without giving up CUDA-level low-level control.

CUDA-first, Windows-native, single-stack NVIDIA done deep: three backends emit PTX (the runtime talks directly to the CUDA Driver API), DXIL (a native D3D12 graphics runtime), and SPIR-V (the single Vulkan/SPIR-V cross-vendor backend since MB1 — AMD desktop + Android, compute + graphics; preview, behind a default-off feature flag).

> **Language note:** the in-depth design dossier (`01`–`14`), the testable specification (`spec/`), and the milestone contracts are currently Chinese-only. For English readers, [`OVERVIEW.en.md`](OVERVIEW.en.md) distills the whole dossier into a single page, and the [`guide/`](guide/README.en.md) tutorial is available in English. This page, plus [`OVERVIEW.en.md`](OVERVIEW.en.md), [`CONTRIBUTING.en.md`](CONTRIBUTING.en.md), [`SECURITY.en.md`](SECURITY.en.md), and [`CODE_OF_CONDUCT.en.md`](CODE_OF_CONDUCT.en.md), are the English entry points. Contributions that translate more of the corpus are welcome (see *Contributing* below).

---

**Contents**: [What it solves](#what-it-solves) · [Project status](#project-status) · [Workspace](#workspace) · [Getting started](#getting-started) · [Governance & quality gates](#governance--quality-gates) · [Statement of restraint](#statement-of-restraint) · [Documentation map](#documentation-map) · [Contributing](#contributing) · [License](#license)

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

## Project status

**Language 1.0 is released (tag `v1.0.0`). The milestone mainline M0 → G30 has closed 44 contracts (latest closed tag `g30-closed`); since 2026-08-27 the project runs in same-workspace campaign mode (G31–G39), with the six G31–G36 contracts honestly kept at `status: active` / `implementation_status: unlocked` rather than dressed up as closed.**

- The first-layer full acceptance (01 §6) is met; the first mission criterion (11 §6) is delivered — ruridrop is the first production-grade renderer/simulation written with Rurix as its primary language (first-party).
- The predefined resource-lifetime error classes are 100% intercepted at compile time; every existing performance threshold is backed by `measured_local` evidence with zero `estimated` entries (G6 explicitly set no hard performance gate).
- Every gate's terminal state — including no-go, defer, and honest red — is recorded as-is and never rewritten to PASS.

> **Facts of record** live in the milestone contracts (`milestones/<id>/*_CONTRACT.md` §8 close-out), the per-phase `*_P2_DECISIONS.md`, and [`registry/`](registry/); campaign deliverables and gate evidence live under `artifacts/day_*/` (hand-over sheets `HANDOVER.md`) and [`evidence/`](evidence/). This section is an index mirror only; the original append-only status errata are preserved in git history (`git log -p -- README.en.md`).

### Milestone mainline (closed)

| Phase | Closed | Theme / deliverables |
|---|---|---|
| M0–M8 (MVP) | 2026-06-17 `m8-closed` | Compiler / runtime / toolchain loop + the UC-01/02/03 flagships + cublas bindings + release pipeline + bilingual diagnostics / doc site |
| G1 | 2026-06-22 `g1-closed` | CUDA–D3D12 interop real-time present, stream-ordered `AsyncBuffer<'stream,T>`, engine-integration DLL (C ABI), production fatbin distribution, geometry crate |
| G2 | 2026-06-30 `g2-closed` | Shader stages in the type system, DXIL second backend (D-131 hybrid), binding-layout derivation (root signature), D3D12 runtime + UC-04 deferred renderer, language-1.0 machinery (edition "2026" + stable-surface snapshot freeze) |
| V1 | 2026-07-14 `v1-closed` | First stable release of the language (tag `v1.0.0`): stabilization report, FCP-lite notice, stable-channel manifest (rurixup), first GitHub Release |
| MS1 | 2026-07-15 `ms1-closed` | `std::gpu` single-source host orchestration (one `.rx` → one EXE) + ruridrop, the first all-`.rx` application (UC-07) |
| MB1 | 2026-07-16 `mb1-closed` | Single Vulkan/SPIR-V cross-vendor backend (RFC-0011; AMD desktop + Android, compute + graphics; Android measured on device; the AMD real-card gate G-MB1-6 honestly stays open pending hardware; preview, default-off feature) |
| G3 | 2026-07-19 `g3-closed` | Industrial rendering: RD-027 poison-path attribution gate + the full five-feature surface (sampling superset / bindless / render-graph auto barriers / UC-04 windowed present / mesh-task-RT dual backends) |
| EI1 | 2026-07-23 `ei1-closed` | Engine integration: UC-05 minimal RHI + render-graph core + RD-009 `#[export(c)]` C-ABI export codegen with built-in header generation (D-113) |
| G4 | 2026-07-24 `g4-closed` | Engine rendering: graphics-RHI raster/mesh library surface + auto barriers + engine_host v3 embedding + single-source `.rx` Vulkan RHI channel + BLACKHOLE production-tier acceptance (RD-036 stays open) |
| EA1 | 2026-07-28 `ea1-closed` | Distribution & storefront: real rurixup distribution (RD-025 redeemed) + prebuilt toolchain bundles (`v1.0.1-dist` series, pre-release) + documentation storefront + cold-start acceptance |
| G5 | 2026-07-29 `g5-closed` | Native renderer: declarative render graph (`rurix-render`) + RHI graphics dispatch bridge + virtualized geometry (meshlets / two-level GPU culling / VisBuffer) + VSM shadows + screen-probe GI + ray-traced effects + material streaming + temporal reconstruction (TAA/TSR); UC-06 full-pipeline demo on device |
| G6 | 2026-08-01 `g6-closed` | Rendering & physics dual track: production-default Jolt physics + default-off Rapier fast path + one-way Physics→GpuScene bridge + UC-08 confluence demo + Taichi Vulkan AOT effects side track |
| G7 | 2026-08-05 `g7-closed` | Production frame closure: RD-038 closed (compute SPIR-V 1.4 / RayQuery, W3 GI/RTAO/hard shadows, VisBuffer SW/HW diff=0, One True Device Frame + soak) |
| G8 | 2026-08-06 (contract §8) | UE5-tier prerequisite capabilities: RFC-0019/0020/0021 + asset pipeline (`rurix-asset` / geometry pages / basis_universal texture codec) + waves G8.2–G8.8 accepted, with no-go / defer terminal states honestly kept |
| G9 | 2026-08-15 (contract §8.10) | UE5-target rendering / physics platform: RFC-0022/0023/0024, five waves G9.2–G9.6 + 33-row P2 exhaustive decisions + ≥30 min soak; 15 P0 + 19 go-P1 gates green |
| G10 | 2026-08-16 (contract §8.10) | UE5 visual-benchmark baseline: UE5 5.8 rendering environment / stress corpus / metrics infrastructure / first A/B round; 11-row gap register locked as G11's statutory input |
| G11 | 2026-08-17 `g11-closed` | GI & lighting quality closure: caliber alignment, asset/scene repair, lamp seed set and multi-bounce GI (incl. M99-clipmap); re-test register final state converged 8 + aligned_closed 3 |
| G12 | 2026-08-17 `g12-closed` | Path-tracing productionization: denoiser pipeline + TSR coupling, UE PT dual-end comparison, PT throughput baseline (50×3 protocol); 10-row gap register locked as G13's statutory input |
| G13 | 2026-08-19 `g13-closed` | Vendor upscaling & Lumen comparison: DLSS SR (Streamline 2.10.3 NGX Vulkan) / FSR 3.1.5 / TSR device lane with runtime switching + UE DLSS / Lumen dual-arm comparison; 8+2-row registers locked as G14/G15 statutory input |
| G14 (incl. G14plus) | 2026-08-23 `g14-closed` | Formal frame-rate parity & render-pipeline performance: M-d **18/18 at the ×1.00 line**, empty gap-register terminal state; RD-045 stays open |
| G15 (incl. G15plus) | 2026-08-23 `g15-closed` | Image-quality closure & commercial final review: **double miss honestly recorded** — commercial sign-off 0/18 + performance 17/18 (single-cell environment event); three carry-over anchors handed to G16+ |
| G16 (incl. G16plus) | 2026-08-24 `g16-closed` | UE cornell reference-arm repair (no longer dead-black) + affected-gate re-test; M-g absolute quality 18/18 under a programmatically calibrated threshold (p100×2.0), the historical G15 M-c 0/18 left unrewritten |
| G17 | 2026-08-24 `g17-closed` | DLSS performance-gap closure: NGX 310.6.0 upgrade **rejected** (incompatible under the Streamline 2.10.3 pin), RFC-0032 D3D12-host lane **deferred**, M-d final verdict ratio 0.856326 keeps the honest-red 17/18 |
| G18 | 2026-08-24 `g18-closed` | One-shot all-directions closure: nine P0 gates green with honest terminal states (Streamline not-available / fps 17/18 / mesh shader no-go / frame generation defer-to-G19+), 25-row P2 exhaustive decisions |
| G19–G25 | 2026-08-24 `g19-closed` … `g25-closed` | Seven-phase serial campaign: frame-generation layer / virtualized geometry P4 / lighting P3+ / material·streaming·temporal / physics platform / present & tail gates / full commercial final review; 35 P0 gates green, 79-row P2, four host reference implementations (framegen / hzb / restir_reservoir / slab); summary in [`G19_G25_CAMPAIGN_RECORD.md`](milestones/g25/G19_G25_CAMPAIGN_RECORD.md) |
| G26–G30 | 2026-08-25 `g26-closed` … `g30-closed` | Five-phase device-ization campaign: the four pieces above landed as real `.rx` device kernels (`g26_framegen` / `g27_hzb_reduce` / `g28_restir` / `g29_slab`, bit-level double-run + frozen-tolerance parity) + material side-table arm; RFC-0043–0047 with all 63 findings dispositioned; the G30 final review pins fps at an honest-red 17/18 (focus-cell ratio 0.960479); hand-over archive `g30_campaign_handover_registry.json` = the sole statutory input for G31+ |

### Campaign mode (since 2026-08-27)

Contract flips and formal chartering stay owner-pending; deliverables and gate evidence land per campaign under `artifacts/day_*/`.

| Campaign | Date / commit | Summary |
|---|---|---|
| G31 / G32 / G33 + G34 | 2026-08-27 `058f8e68` | Real-time present (wave A) / visual completeness (wave B) / commercialization (wave C) delivered in one batch with §8 close-out records on file and contracts kept `active`; G34 full-feature merge acceptance: unified-lane foundation (8 facts) + HZB on the unified lane (6 facts) + skinning on the unified lane (9 criteria) + Stage A digest 18/18 zero drift + soak 5010/5010 frames zero crash |
| G35 / G36 | 2026-08-27 | G35 GPU particle system (RFC-0049), nine waves of artifacts in tree, G35-4 transparency sort/OIT dual-arm gate PASS; G36 exclusivity repair & composition rendering W1–W5 delivered, `g36.wave1.geo_composition` ten-fact gate PASS on real hardware |
| G37 commercial delivery wrap-up | 2026-08-30 `0e605c34` | `g31_window_present --quality` default **flipped off→full (19 arms)** inside the 11.11 ms frame budget (9.75 / 10.59 ms) + seven new arms (transparency / LUT / PSO ledger / VisBuffer evidence / RIS+NEE / per-frame cut / FG×full) + ten fixes (incl. rurixc if-while codegen back-edge pruning) + commercial GAP-01–03 closed + **SDK bundle candidate `sdk-1.1.0`** (24 components, dual SBOM, four-level verification) + two green soaks |
| G38 five-task push | 2026-08-30 `b05cd4ef` | Normals v2 consumption switch with batch re-anchoring + FIF×dynamic #90 closed (RFC-0030 v1.1 L2a + `slot_as` production wiring + per-slot AS memory budget gate) + incremental frame_cut refit (build 8.78 ms, inside the 90 fps budget) + #96 consumption closed + RIS/NEE quality quantification and the lamp-k ladder (default kept at 12/0.6) |
| DLSS 5 NR adaptation | 2026-08-30 `82a59ae3` | Hand-written NGX feature-18 D3D12 FFI integration + three-arm availability probe + NR lane harness; local Ada verdict = **not_available**, honestly recorded (fail-closed / default off / env opt-in; evaluation binaries not committed) |
| G39 five-task push | 2026-09-01 `1478859a` | ReSTIR high-tier temporal-reservoir lamp arm wired into the production lane (26-cluster tier off 11.546 ms → **on 7.526 ms, inside the 11.11 ms budget**) + skin batch B + `slot_as` single-source fold + profiling gate N=5 median criterion + device-cut P1 equivalence gate; zero re-anchoring + CPU guards 7/7 + soak 1936.2 s zero failures. Hand-over: [`artifacts/day_0831_g39/HANDOVER.md`](artifacts/day_0831_g39/HANDOVER.md) |

### Honestly recorded misses and open items

- **fps focus cell 17/18** (bistro-interior / t100 / dlss_sr): honest red since G15; the G17 / G25 / G30 final verdicts all missed ×1.00 (ratio 0.856326 → 0.960479, G34 re-test 0.921836); carry-over anchors = NGX decomposition profiling / UE instrumentation.
- **Commercial sign-off G15 M-c 0/18** stays on record unrewritten; G16plus M-g 18/18 is registered separately under a programmatically calibrated threshold.
- **Eight RD entries stay open** (incl. RD-045 long-window observation maintain-open, RD-034 blocked, RD-036 carried); vendor / hardware facts — Streamline 310.6.0 and DLSS 5 NR not-available, mesh shader (M61) no-go, Work Graphs and HDR output not-available, async triple (M59) no-go — all backed by measured evidence.

### Flagship use cases and key deliverables

All accepted end-to-end on real hardware:

- **UC-01 — PyTorch operator replacement**: `rx build --emit=pyd` produces a PYD (nanobind + scikit-build-core), zero-copy-bridged into PyTorch CUDA tensors over both `__cuda_array_interface__` v3 and DLPack; SAXPY/Reduction/GEMM operator replacements reach **≥ 90% of hand-written CUDA C++** (measured_local).
- **UC-02 — three-stream overlapped pipeline**: affine Context/Stream/Event/Buffer + cross-thread ownership transfer + typed stream-ordered allocation; the four resource-lifetime error classes (use-after-free / double-free / cross-thread / cross-stream-unsynchronized) are **intercepted at compile time**.
- **UC-03 — SPH simulation + compute soft rasterizer**: a single executable — deterministic SPH simulation + soft-raster kernels (binning / tile raster / depth / tonemap) + host frame loop — producing deterministic images.
- **UC-04 — deferred renderer (D3D12)**: the DXIL second backend (D-131 hybrid: compute via a direct minimal-subset DXIL channel, graphics via a SPIR-V→HLSL→dxc validation bridge) + binding-layout derivation (root signature RTS0) + multi-pass orchestration with anchored barriers; the lighting pass truly samples the G-buffer, accepted via off-screen readback pixel comparison.
- **UC-07 — ruridrop, an all-`.rx` application**: `std::gpu` single-source host orchestration (one `.rx` entry → one EXE with embedded PTX+cubin); GPU SPH dam-break simulation + sphere ray tracing, where the offline path-traced PPM and the realtime D3D12 present share the same kernel core; GPU frames match a CPU replay golden **byte-for-byte** (CI smoke tier); ~68 fps realtime at 1280×720 / 131k particles (measured_local).
- **cublas binding package**: three-layer GEMM/GEMV bindings (raw FFI / safe wrapper / high-level API).
- **Release pipeline**: rurixup (stable-channel manifest) + an Authenticode sign/verify release gate (currently a test certificate; the of-record production backend is Azure Artifact Signing behind a secret-gated step) + SBOM (SPDX/CycloneDX) + NVIDIA redistribution-whitelist audit.
- **Bilingual diagnostics with full coverage** (Chinese/English) + **documentation site** (`rx doc`).

> The stable-API snapshot freeze has been active since the 1.0 release ([`RD-008`](registry/deferred.json) closed): the stable surface (spec clause IDs + error-code meanings + edition values + the `rx` CLI command set) is anchored by snapshot comparison with bless-gated approval — additive-only within an edition; breaking changes require a new edition.

## Workspace

**Language and toolchain**

| Crate | Responsibility |
|---|---|
| `src/rurixc` | Compiler (frontend + MIR + NVPTX/DXIL/SPIR-V backends + borrow/resource checks + formatter + LSP session) |
| `src/rx` | Toolchain CLI (`build` / `check` / `run` / `fmt` / `bench` / `test` / `doc` / `vendor`) |
| `src/rurix-pkg` | Package management (lockfile + vendor + checksum) |
| `src/rurixup` | Installer / bootstrapper (release pipeline, stable-channel manifest) |

**Runtime and interop**

| Crate | Responsibility |
|---|---|
| `src/rurix-rt` | Runtime (thin CUDA Driver API layer: affine Context/Stream/Event/Buffer, launch, fatbin load negotiation, poisoned state machine) |
| `src/rurix-rt-cabi` | Host-orchestration C-ABI runtime boundary (`rxrt_*` / `rxp_*` / `rxio_*`: single-source `.rx` apps ↔ the runtime — fatbin loading / launch / present / image dump) |
| `src/rurix-interop` | PyTorch interop (PYD / `__cuda_array_interface__` / DLPack boundary) |
| `src/rurix-cublas` | cublas v2 binding package |
| `src/rurix-d3d12` | D3D12/DXGI present shim (the CUDA–D3D12 interop realtime-present boundary) |
| `src/rurix-engine` | Engine-integration DLL (C-ABI cdylib; embedded in C++/D3D12 hosts to run compute passes) |
| `src/rurix-android-present` | Android on-device present glue (MB1; zero-Java NativeActivity cdylib shell, compiles to an empty lib on desktop) |

**Renderer, geometry, and assets**

| Crate | Responsibility |
|---|---|
| `src/rurix-render` | Native engine renderer library (declarative render graph / virtualized geometry / VSM / probe GI / ray-traced effects / material streaming / temporal reconstruction / frame generation / ReSTIR; the renderer is a library, not part of the language) |
| `src/rurix-renderer-sdk` | Renderer SDK C-ABI implementation layer (`rxsdk_*` session surface, the first stable embedding ABI; G31+) |
| `src/rurix-geometry` | Geometry library (mesh/BVH, zero-dependency, all-safe) |
| `src/rurix-geom-build` | Offline geometry builder (mesh → meshletization → grouped-and-simplified hierarchical DAG + CPU reference culler; deterministic all-safe host code) |
| `src/rurix-geom-pages` | Geometry page formats (RXPL / RXPD / RXPM codecs, `spec/geometry_pages.md`) |
| `src/rurix-asset` | Asset pipeline (RFC-0020: geometry-page build, canon / graph / cook / verify, glTF import, texture codec) |
| `src/rurix-basis-sys` | basis_universal texture codec FFI boundary (UASTC→KTX2 / ETC1S / BCn·ASTC transcode; `unsafe` concentration point) |
| `src/image-io` · `src/soft-raster` | Image I/O · soft-rasterizer host CPU reference library (numerically equivalent to the device kernels) |

**Physics**

| Crate | Responsibility |
|---|---|
| `src/rurix-physics` | Engine physics library (RFC-0017: fixed-step `PhysicsWorld`; Jolt production default / Rapier default-off fast path) |
| `src/rurix-physics-sys` | JoltC FFI boundary (Jolt 5.3 baseline; the sole `unsafe` concentration point for physics) |
| `src/rurix-physics-sys56` | Jolt 5.6 evaluation-arm FFI (coexists with 5.3, used for A/B) |

**Applications and demos**

| Crate | Responsibility |
|---|---|
| `apps/ruridrop` | UC-07 all-`.rx` application (renderer/simulation in one; not a Cargo crate — a declarative `rurix.toml` package, zero `.rs`) |
| `apps/uc05-rhi` | UC-05 minimal RHI + render graph (`.rx` package, `--emit=dll` C-ABI export; the base of the renderer's minimal integration example) |
| `apps/uc06-renderer` | UC-06 full-pipeline demo (culling → VisBuffer → deferred shading → GI/VSM/RTAO → TAA/TSR → headless readback pixel assertions) |
| `apps/uc08-physics` · `apps/uc09-taichi-spike` | UC-08 rendering×physics confluence demo · Taichi Vulkan AOT effects side-track spike |
| `apps/blackhole` | BLACKHOLE production-tier rendering demo (G4 acceptance) |
| `apps/g31-renderer-sdk` | Renderer SDK `.rx` package + host example (with `API_VERSIONING.md`) |
| `apps/g8-physics-gates` · `apps/g9-physics-gates` | G8 / G9 physics acceptance-gate harnesses |
| `src/uc02-demo` · `src/uc03-demo` · `src/uc04-demo` | Flagship use-case demos |

## Getting started

**Environment**: Windows 11 + an NVIDIA GPU (reference machine: RTX 4070 Ti), the CUDA Toolkit, and MSVC 2022. The Rurix toolchain itself is built with Rust (D-201).

Prebuilt binaries (`rx.exe` / `rurixup.exe` + SBOM + `SHA256SUMS`) are on [GitHub Releases](https://github.com/qwasg/Rurix/releases) (since v1.0.0; currently Authenticode-signed with a test certificate, so SmartScreen may warn). To build from source:

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

- **Want to learn how to write Rurix code?** See the [`guide/`](guide/README.en.md) tutorial (available in English) — a progressive path from your first host program to your first kernel, with every example exercised live by CI gates (`rx check` / `rx run`).
- **Want to embed the renderer in your own engine?** See [`docs/renderer/integration_guide.md`](docs/renderer/integration_guide.md) (five-step C-ABI host + the minimal example project under `docs/renderer/examples/minimal_host/`), together with the [feature matrix](docs/renderer/feature_matrix.md), [performance tuning](docs/renderer/performance_tuning.md), [compatibility matrix](docs/renderer/compatibility_matrix.md), and the SDK bundle (`dist/sdk_bundle/`). These documents are Chinese-only for now.

## Governance & quality gates

Rurix builds governance in as a product capability from day one (language infrastructure for the AI era; see [`10_GOVERNANCE.md`](10_GOVERNANCE.md)):

- **Spec ↔ test ↔ PR triangle**: every RXS spec clause is anchored by ≥1 test (`ci/trace_matrix.py`, currently 278/278).
- **measured_local budgets**: all performance/diagnostics baselines are measured on real hardware, with zero `estimated` placeholders (`ci/budget_eval.py --strict`).
- **Real red-green**: every CI gate is validated by "introduce a defect → red → restore → green" (anti-YAML-only), with run URLs archived in [`evidence/`](evidence/).
- **Byte-level guardrails**, schema validation, structure validation, all-green conformance, and blessed UI/MIR/PTX/DXIL goldens plus the stable-API snapshot.
- **deferred / spike-gating registries**: the single source of truth for deferred debt and expansion directions — append-only.
- **Honest terminal states**: gate no-go / defer / honest-red outcomes are kept on record, never rewritten; contract close-outs are append-only and closed contracts are 0-byte immutable (machine-guarded by `ci/check_guardrails.py`).

Milestone contracts and close-out trails live in [`milestones/`](milestones/); the governance mechanism overview is in [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md).

## Statement of restraint

Rurix does **not** replace the CUDA ecosystem (it provides a safe compile frontend and runtime on top of it), does **not** lead with cross-platform support (single-stack NVIDIA done deep first), and does **not** build an ML framework (it interoperates zero-copy with PyTorch via DLPack). Each act of restraint maps to a verified "dead route" ([`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md) §4).

## Documentation map

| Location | Contents |
|---|---|
| [`00_MASTER_INDEX.md`](00_MASTER_INDEX.md) | Master index: document list, reading paths, glossary, maintenance rules |
| `01`–`14` | Planning dossier (see the per-file table below); [`15_EXTERNAL_ADOPTION_REGISTER.md`](15_EXTERNAL_ADOPTION_REGISTER.md) is the external-adoption register; [`OVERVIEW.en.md`](OVERVIEW.en.md) is the single-page English distillation |
| [`spec/`](spec/) | Testable specification (FLS-style, RXS clauses); [`conformance/`](conformance/) is the sole acceptance boundary |
| [`rfcs/`](rfcs/) | Language-evolution RFC / Mini-RFC series (templates, numbering ledger, and the FCP-lite review window in `rfcs/README.md`) |
| [`guide/`](guide/README.en.md) | Getting-started tutorial (Chinese / English) |
| [`docs/renderer/`](docs/renderer/) | Renderer product docs: integration guide, feature matrix, performance tuning, compatibility matrix, release checklist, support policy, vendor license matrix |
| [`milestones/`](milestones/) | Milestone contracts (four elements), P2 decision tables, close-out sign-offs |
| [`registry/`](registry/) | Append-only registries: `deferred.json` / `spike_gating.json` / `error_codes.json` / `number_ledger.json` |
| [`evidence/`](evidence/) | CI gate evidence (written only on PASS; fail-closed) |
| `artifacts/day_*/` | Campaign deliverables: `CAMPAIGN_LOG.md` / `HANDOVER.md` / per-task reports / evidence |
| [`dist/`](dist/) | SDK bundle (`sdk_bundle/sdk-1.1.0/`), SBOM, third-party notices, release notes |

The planning dossier (Chinese-only) at a glance:

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
