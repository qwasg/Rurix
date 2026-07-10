# GRX-009 Segment 3b — Luminance Reduction Resource Mapping Scaffold

## Scope

This file records the segment 3b resource mapping scaffold for `luminance_reduction`. It maps the real Godot luminance resources and parameters into the Rurix bridge design while keeping the runtime fallback-first. This is not a real GPU runtime pass, does not skip Godot native luminance, and does not provide visual, telemetry, or performance evidence.

## Godot Native Flow

- Entry point: `RendererRD::Luminance::luminance_reduction(RID p_source_texture, Size2i p_source_size, Ref<LuminanceBuffers> p_luminance_buffers, float p_min_luminance, float p_max_luminance, float p_adjust, bool p_set)` in `external/godot-master/servers/rendering/renderer_rd/effects/luminance.cpp`.
- Auto Exposure call site: `external/godot-master/servers/rendering/renderer_rd/renderer_scene_render_rd.cpp` passes `rb->get_internal_texture()`, `rb->get_internal_size()`, `luminance_buffers`, min/max sensitivity, exposure adjust step, and `set_immediate`.
- Buffer allocation: `LuminanceBuffers::configure()` creates a `R32_SFLOAT` reduce chain where each destination level is `max(previous_size / 8, 1)` until 1x1, and creates `current` for the previous/final luminance value.
- Compute modes: first level reads the HDR source texture, intermediate levels read the previous reduce image, and final write mode may read `current` as previous luminance when `!p_set` before `SWAP(current, reduce[last])`.
- Dispatch shape: Godot dispatches compute threads over the current source size with an 8x8 local shader block; each level then divides source size by 8 and clamps to 1.

## Godot Resources

| Godot resource | Role | Native binding shape | Segment 3b Rurix mapping |
| --- | --- | --- | --- |
| `p_source_texture` / `rb->get_internal_texture()` | HDR source for level 0 | sampled texture at set 0 binding 0 in `READ_TEXTURE` mode | not directly implemented by runtime; scaffold records it as the source side of `src_luminance` |
| `p_luminance_buffers->reduce[i - 1]` | source for intermediate levels | readonly `r32f image2D` at set 0 binding 0 | `src_luminance` SRV `t0 space0` for the current scaffold level |
| `p_luminance_buffers->reduce[i]` | destination for current level | writeonly `r32f image2D` at set 1 binding 0 | `dst_luminance` UAV `u0 space0` for the current scaffold level |
| `p_luminance_buffers->current` | previous/final 1x1 luminance | sampled texture at set 2 binding 0 in `WRITE_LUMINANCE` mode | recorded as a future mapping requirement; not implemented in segment 3b runtime |

## Parameters

| Rurix root constant | Godot source | Type in current artifact | Notes |
| --- | --- | --- | --- |
| `source_width` | `source_size.x` for the current level | `i64` | Godot native push constant uses `int32_t`; current Rurix artifact uses 64-bit integer lowering and therefore requires a target device capability gate. |
| `source_height` | `source_size.y` for the current level | `i64` | Updated per level after each 8x8 reduction step. |
| `max_luminance` | `p_max_luminance` / auto exposure max sensitivity | `f32` | Used by final exposure adjustment clamp. |
| `min_luminance` | `p_min_luminance` / auto exposure min sensitivity | `f32` | Used by final exposure adjustment clamp. |
| `exposure_adjust` | `p_adjust` / auto exposure adjust speed times frame step | `f32` | Segment 3b records the scalar only; complete previous-luminance feedback remains future work. |

Current artifact root constants occupy 7 DWORDs: `source_width` at DWORD 0..1, `source_height` at DWORD 2..3, `max_luminance` at DWORD 4, `min_luminance` at DWORD 5, and `exposure_adjust` at DWORD 6.

## Descriptor Layout Scaffold

- Root constants / root-cbuffer mapping: `b0 space0` scaffold for `source_width`, `source_height`, `max_luminance`, `min_luminance`, and `exposure_adjust`.
- SRV: `src_luminance = t0 space0`.
- UAV: `dst_luminance = u0 space0`.
- Required resource count for the bridge scaffold: 2 resources, source then destination.
- Required push constant size for the bridge scaffold: 28 bytes, matching the current descriptor artifact root constant layout.
- Required target device gate: 64-bit integer shader capability must be confirmed on the D3D12 device before any runtime attempt may proceed.

## Fallback Rules

- The pass remains disabled by default and runtime remains `fallback_only`.
- Missing source or destination resource returns fallback.
- Descriptor layout mismatch returns fallback.
- ABI mismatch returns fallback through existing ABI validation paths.
- Missing 64-bit integer shader capability returns fallback.
- `rxgd_record_pass` must not return `RXGD_STATUS_OK` for `RXGD_PASS_LUMINANCE_REDUCTION` in segment 3b.
- Godot native `luminance_reduction` remains the active path whenever the bridge does not return OK.

## Explicit Non-Goals

- No real runtime luminance GPU pass is implemented.
- No complete Godot reduction pyramid replacement is implemented.
- No previous-luminance feedback binding is implemented.
- No visual diff evidence is produced.
- No measured fallback telemetry is produced.
- No performance number or acceleration claim is made.

## Segment 4a Addendum — Runtime Binding Preflight

Segment 4a wires this resource mapping scaffold into a runtime binding preflight layer. Runtime remains `fallback_only`; this is not a real GPU pass and does not skip Godot native luminance.

- Patch `spike/godot-rurix/patches/0005-rurix-accel-luminance-runtime-binding-preflight.patch` (stacked on 0001+0002+0003+0004) extends `D3D12Hooks::try_record_luminance_reduction()` so the Auto Exposure call site passes the real luminance binding: the source HDR texture and level-0 reduce destination as logical Godot RID ids (not D3D12 GPU handles), source/destination dimensions, and the `max_luminance` / `min_luminance` / `exposure_adjust` scalars.
- The `rurix_accel` module marshals two `RXGD_RESOURCE_TEXTURE` records in `src_luminance = t0` then `dst_luminance = u0` order plus the 28-byte `b0` root constant block into `rxgd_record_pass`.
- The bridge preflight (`record_runtime_binding_preflight` in `src/rurix-godot/src/lib.rs`) validates, in order: the 64-bit integer shader capability, the 2-resource descriptor shape, the 28-byte push constant size, texture resource kinds, nonzero source dimensions that match the bound `src_luminance` resource, and the `max(source / 8, 1)` level-0 reduce shape of `dst_luminance`.
- Any preflight failure returns `RXGD_STATUS_FALLBACK` with a recorded fallback reason (`validation_failed` or `unsupported_device`) and accumulates no estimated GPU/CPU time.
- A successful preflight also still returns `RXGD_STATUS_FALLBACK`: the gate stays disabled, no D3D12 dispatch is recorded, and the native Godot luminance path remains active.
- Segment 4a produces no visual diff evidence, no measured telemetry, and no performance claim; the next slice is a gated dispatch bring-up, not visual/perf evidence.

## Segment 4e Addendum — Native D3D12 Resource Handle Mapping

Segment 4e changes the resource the Godot runtime hands to the Rurix bridge from a logical Godot RID id into the real D3D12 `ID3D12Resource*` native handle. Runtime remains `fallback_only`; this is native handle mapping preflight only, not a real GPU pass, and it does not skip Godot native luminance by default.

- Patch `spike/godot-rurix/patches/0007-rurix-accel-luminance-native-resource-handle-mapping.patch` (stacked on 0001+0002+0003+0004+0005+0006) renames the `D3D12Hooks::try_record_luminance_reduction()` source/dest parameters from logical RID ids (`p_source_texture_id` / `p_dest_texture_id`) to real native handles (`p_source_native_handle` / `p_dest_native_handle`).
- The Auto Exposure call site now resolves the real D3D12 native handles through `RenderingDevice::get_driver_resource(DRIVER_RESOURCE_TEXTURE, RID, 0)`:
  - source: `rb->get_internal_texture()` → real `ID3D12Resource*`.
  - dest: `luminance_buffers->reduce[0]` → real `ID3D12Resource*`.
  - When `RenderingDevice` is unavailable or either native handle resolves to `0`, the pass falls back to the native Godot luminance path.
- Underlying seam: `RenderingDevice::get_driver_resource(DRIVER_RESOURCE_TEXTURE, RID)` returns the value produced by `RenderingDeviceDriverD3D12::get_resource_native_handle(DRIVER_RESOURCE_TEXTURE, TextureID)`, i.e. the real `ID3D12Resource*`.
- The `rurix_accel` module stores those real `ID3D12Resource*` handles in `RxGdResource.native_handle` (previously a logical RID id) and returns fallback if either handle is `0`.
- `RXGD_ABI_VERSION` is unchanged, the pass stays default-disabled, and the shipping/feature-off bridge still returns `RXGD_STATUS_FALLBACK` for `RXGD_PASS_LUMINANCE_REDUCTION`. Wording correction (from segment 4f): the earlier "the Godot module never sets `RXGD_CAP_LUMINANCE_DISPATCH_RECORD`" phrasing is superseded — as of segment 4f the module sets that harness-only record-arm flag **only** when the default-off `.../dispatch_recording_smoke` opt-in is explicitly enabled; the default Godot config and the shipping/feature-off bridge still never set it.
- Segment 4e is native handle mapping preflight only: it records no Godot-runtime-driven D3D12 dispatch, keeps `runtime_state = fallback_only`, `real_gpu_pass = false`, and `real_d3d12_dispatch_recorded = false`, and produces no visual diff, measured telemetry, or performance claim.

## Segment 4h Addendum — Kernel-Binding-Kind Conformance (real-pass gate)

Segment 4h adds the opt-in gated real-pass arm and, with it, an explicit **kernel-binding-kind conformance rule** that makes the tracked artifact's runtime unmappability a validated, honestly-reported fact instead of undefined behaviour:

- The tracked segment 3a kernel lowers `View<global, f32>` / `ViewMut<global, f32>` to **raw-buffer views** (`target("dx.RawBuffer", float, ...)` handles in the debug IR), NOT Texture2D SRV/UAV bindings. The Godot runtime (segment 4e) provides real **Texture2D** `ID3D12Resource*` native handles. Binding texture descriptors over raw-buffer-view shader declarations is descriptor-type undefined behaviour: the segment 4c/4d/4f recorded dispatches proved plumbing (PSO creation, dispatch submission, fence completion, readback) but NOT computational correctness.
- The bridge therefore validates, on the real-pass arm only (`RXGD_CAP_LUMINANCE_REAL_PASS`, armed by the default-false `.../dispatch_real_pass` opt-in from patch 0009): every bound runtime resource's binding kind must equal the tracked kernel binding kind (`raw_buffer_view`). Texture resources do not conform → `validation_failed` → `RXGD_STATUS_FALLBACK` plus the once-per-session machine-readable `RXGD_REAL_PASS_BLOCKED first_missing_prerequisite=kernel_binding_kind_mismatch ...` diagnostic. Note the segment 4a preflight REQUIRES texture resources (the Godot runtime binding contract), so with the tracked artifact the conformance check can never pass after preflight — the incompatibility is structural, and that is the point of the gate.
- Additional known gaps recorded by the segment 4h evidence (`known_gaps`): the tracked kernel applies `clamp(min,max) * exposure_adjust` at every level (not equivalent to any single native reduction step), and it covers one 8x8 reduction level while the patch-0003 call site replaces the whole native `luminance_reduction` call on bridge OK (pyramid continuation design missing).
- Consequence: the first missing prerequisite for real-pass enablement is a **runtime-mappable (texture-capable, math-parity) kernel artifact round** — a compiler/offline-compile slice with its own offline evidence — after which the bridge's kernel binding kind constant, the conformance rule, and the real dispatch wiring change together. Until then the pass stays default disabled, `runtime_state = fallback_only`, and no visual/GPU-timestamp/performance claim is made.

## Segment 4f Godot-runtime bridge recording smoke — evidence hygiene

- The runtime smoke `ci/grx009_godot_runtime_bridge_recording_smoke.py` tracks **two** evidence files with different roles:
  - `godot_runtime_bridge_recording_evidence.json` is the **latest** run evidence. It is rewritten on every run and is honestly reproducible: with no `RURIX_GRX009_SEGMENT4F_GODOT_EXE` (the scratch Godot exe env var) set it records `status=skip`. It never advances the readiness gate on its own.
  - `godot_runtime_bridge_recording_success_evidence.json` is the **historical measured success** artifact. It is written/updated **only** on a strict `status=success` run — recording the Godot exe fingerprint, the 0001..0008 patch stack identity, the feature-built DLL fingerprint, the artifact hashes, `godot_exit_code_zero=true`, and the `recorded=1` marker, with an explicit note that scratch Godot build binaries are not committed. A later SKIP/FAIL run must never delete or overwrite it.
- The segment 4f readiness gate (`grx009_segment4f_godot_runtime_bridge_recording_ready`) advances off the historical success artifact, not the reproducible-default SKIP latest file, so re-running the smoke without the scratch build reverts the latest evidence to SKIP while historical readiness stays true. Stale / hash-mismatched / tampered success evidence still does not advance readiness.
- Even a success keeps `runtime_state = fallback_only`, `real_gpu_pass = false`, `real_d3d12_dispatch_recorded = false`, `godot_runtime_luminance_path_enabled = false`, and `default_enable_state = disabled`, and makes no visual, telemetry, GPU-timestamp, or performance claim.

## Segment 4i Addendum — Texture-Capable Kernel Artifact Round (fail-closed)

Segment 4i attempts to advance the segment 4h real-pass gate's first missing prerequisite from `kernel_binding_kind_mismatch` to `math_pyramid_parity_not_proven` by landing a texture-capable luminance kernel artifact round. Runtime remains `fallback_only`; this is a compiler + offline-compile + bridge tracked-package slice only, not a real GPU pass, and it does not skip Godot native luminance by default.

**HONEST FAIL-CLOSED PATH (current state):** The texture-capable kernel source `src/lib_texture.rx` is in place, and the compiler-side forward-looking changes are in place (`RWTexture2D<F>` lang item, `MirResourceType::RWTexture2D`, `texture_target_ty`, `@llvm.dx.resource.load.texture.*`/`@llvm.dx.resource.store.texture.*` emit). However, the patched `llc` at `H:\llvm-dxil\build\bin\llc.exe` does NOT support the `llvm.dx.resource.load.texture.2d` intrinsic, so the texture-capable offline compile records `status=compile_failed` with blocker `dxil_container_missing`. The canonical `artifacts/luminance_reduction.{dxil,rts0.bin,_descriptor_layout.json}` paths therefore carry raw-buffer bytes copied from `artifacts/raw_buffer_historical/` so the bridge `include_bytes!` works; the bridge tracked package stays raw-buffer; the probe stays at `kernel_binding_kind_mismatch`. The forward-looking changes activate when a newer patched `llc` supports texture intrinsics.

- The tracked luminance kernel source is `src/lib_texture.rx`, declaring `kernel fn luminance_reduce_level_texture(src_luminance: Texture2D<f32>, dst_luminance: RWTexture2D<f32>, source_width: usize, source_height: usize, max_luminance: f32, min_luminance: f32, exposure_adjust: f32, t: ThreadCtx<1>)`. `Texture2D<f32>` / `RWTexture2D<f32>` replace the segment 3a `View<global, f32>` / `ViewMut<global, f32>` parameter declarations. `src/lib.rx` is preserved unchanged as the raw-buffer historical fixture and conformance corpus input; its compiled artifacts are at `artifacts/raw_buffer_historical/` and (in fail-closed state) also copied to the canonical `artifacts/` paths.
- The compiler recognizes a new `RWTexture2D<F>` lang item (compute-kernel UAV texture, distinct from the existing SRV `Texture2D<F>`, which is now also accepted in compute-kernel parameter position). The MIR resource type enum adds `MirResourceType::RWTexture2D(PrimTy)` with `class() -> Uav`. `derive_compute_bindings` has new `Texture2D` / `RWTexture2D` head-name branches, `require_view_global_f32` is relaxed to `require_texture_or_view_global_f32`, `texture_target_ty(mutable: bool)` emits `target("dx.Texture2D<float>", 0, 0)` / `target("dx.RWTexture2D<float>", 0, 0)`, and `render_lowered_ops` emits `@llvm.dx.resource.load.texture.*` / `@llvm.dx.resource.store.texture.*`. The raw-buffer lowering path is preserved unchanged.
- The descriptor layout now records a per-resource `binding_kind` string field on every resource record. Values are `texture2d` (SRV Texture2D), `rwtexture2d` (UAV RWTexture2D), `raw_buffer_view` (View/ViewMut), `sampler`, or `constant_buffer`. For the texture-capable kernel source, `src_luminance` records `binding_kind = "texture2d"` and `dst_luminance` records `binding_kind = "rwtexture2d"`; for the historical raw-buffer fixture (and the current fail-closed canonical artifact), both record `raw_buffer_view`.
- The bridge tracked `LuminanceDispatchPackage` STAYS raw-buffer (fail-closed): `LUMINANCE_KERNEL_RESOURCE_BINDING_KIND = "raw_buffer_view"` (NOT replaced with `texture2d`), and the three SHA-256 constants stay at the segment 3a raw-buffer values (`c77a54de...`/`f08794f9...`/`3ceee39b...`). The `runtime_resource_binding_kind` mapping is unchanged: `RXGD_RESOURCE_TEXTURE -> "texture2d"` and `RXGD_RESOURCE_BUFFER -> "raw_buffer_view"`. Texture resources (the Godot runtime scenario) still fail the segment 4h kernel-binding-kind conformance check with `kernel_binding_kind_mismatch`; buffer resources pass and advance to `check_real_pass_math_parity`.
- Math parity status (forward-looking): the tracked texture-capable kernel source's level 0 aligns with Godot's level-0 luminance reduction — per-pixel luminance = `max(R, G, B)` (equivalent to a direct R read for an R32F source), per-tile arithmetic mean with divisor = valid pixel count (partial-tile correct), dispatch tile 8x8, and no `clamp(min, max) * exposure_adjust` applied at level 0 (matching Godot, which only applies min/max/adjust at the final `WRITE_LUMINANCE` level). However, the pyramid cascade, EMA feedback, previous-luminance double buffering, and final-level clamp/min/max gating are still missing.
- Next blocker (fail-closed): `kernel_binding_kind_mismatch`. The bridge `check_real_pass_math_parity()` returns `Err(FallbackReason::ValidationFailed)` hard-coded; `record_real_pass_attempt` runs preflight -> eligibility -> binding_kind -> math_parity -> `real_dispatch_path_not_linked`. With the tracked raw-buffer package, texture resources fail the binding-kind check first, so the blocked diagnostic prints `first_missing_prerequisite=kernel_binding_kind_mismatch kernel_binding=raw_buffer_view`. The probe `next_action` stays at `provide_grx009_runtime_mappable_luminance_kernel_artifact` (the slice's main goal is blocked by patched llc not supporting texture intrinsics). When a newer patched llc supports texture intrinsics and the tracked package flips to texture-capable, the binding-kind check will pass and the FIRST missing prerequisite will advance to `math_pyramid_parity_not_proven` (probe `next_action` will then advance to `design_grx009_luminance_pyramid_continuation_kernel`).
- The raw-buffer artifact is retained as a historical fixture under `artifacts/raw_buffer_historical/`, with `offline_compile_evidence_raw_buffer.json` preserving the segment 3a raw-buffer measured state and a `notes` field marking it as a historical fixture. The canonical `artifacts/` paths also carry the same raw-buffer bytes in fail-closed state so the bridge `include_bytes!` works.
- Runtime semantics are unchanged: `runtime_state = fallback_only`, `real_gpu_pass = false`, `real_d3d12_dispatch_recorded = false`, `default_enable_state = disabled`, and no FPS, GPU-timestamp, or performance improvement claim is made. The shipping feature-off bridge still returns `RXGD_STATUS_FALLBACK` for `RXGD_PASS_LUMINANCE_REDUCTION`.

### Segment 4j Addendum — Texture Intrinsic Toolchain Blocker Evidence (2026-07-06)

The fail-closed state documented above is now backed by strict, reproducible three-way cross-investigation evidence: (1) 10 minimal `.ll` scratch cases (forms A–J) run through `H:\llvm-dxil\build\bin\llc.exe`, (2) `IntrinsicsDirectX.td` source review, (3) `llc.exe` binary `findstr`. The investigation definitively confirms patched llc (LLVM 22.1.7) has zero texture load/store intrinsic support — `.td` has no `int_dx_resource_load_texture*` definition, the binary contains no `llvm.dx.texture.*` strings, and `DXILResourceAccess.cpp` explicitly stubs `ResourceKind::Texture2D` with `reportFatalUsageError("Load not yet implemented for resource type")`. The `target("dx.Texture2D<float>", 0, 0)` type is also not a registered target-ext type (DXContainer emitter crashes when the handle is unused). See [PASS_CONTRACT.md §22](PASS_CONTRACT.md#22-segment-4j--texture-intrinsic-toolchain-blocker-evidence2026-07-06) for the full investigation narrative and patched llc capability list, and `texture_intrinsic_toolchain_blocker.json` for the structured machine-readable evidence. Reproduce via `py -3 artifacts/toolchain_probe/run_probe.py`. The fail-closed state (`status=compile_failed`, `runtime_mappable=false`, bridge=`raw_buffer_view`, probe=`kernel_binding_kind_mismatch`) is unchanged.

## Segment 4k Addendum — DXC Texture Artifact Bridge Mapping

The design contract for the future DXC texture artifact bridge is tracked in `dxc_texture_artifact_bridge.md`. It does not change the current runtime mapping or canonical artifacts.

- Godot source and destination native handles are real `Texture2D` `ID3D12Resource*` objects. A runtime-mappable texture package must bind them as `src_luminance = t0 space0 binding_kind=texture2d` and `dst_luminance = u0 space0 binding_kind=rwtexture2d`.
- The current canonical descriptor layout still records `raw_buffer_view` for both resources because the canonical artifact paths carry the historical raw-buffer fallback bytes. This must remain true during the design slice.
- `src/lib.rx` and `artifacts/raw_buffer_historical/` remain historical fallback fixtures only. They must not be treated as the texture package binding contract.
- The future texture package must keep the existing `b0 space0` root constant packing when it carries luminance constants: two `i64` dimensions followed by three `f32` scalars for 28 bytes total. A no-constant DXC feasibility shader must record `root_constants=none` and cannot become the canonical luminance artifact.
- Any attempt to bind Godot Texture2D handles to raw-buffer declarations is a descriptor-kind mismatch and must fail closed before runtime dispatch.
