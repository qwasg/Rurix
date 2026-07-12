# GRX-014 cluster_store Pass — PASS CONTRACT

> **Status (2026-07-12, slice S1-S4+S6: contract trio + offline kernel + math
> parity + bridge gate + standalone dispatch smoke).**
> This slice delivers the OFFLINE face plus the host-side gate, reusing the
> matured GRX-009..013 per-pass template (`../PASS_TEMPLATE.md`): the pass
> contract trio (S1), the HLSL-bridge math-equivalent kernel (DXC `cs_6_0`
> compile + DXV validation + Rurix-owned RTS0 via `rurixc::binding_layout`,
> owner-approved `hlsl_bridge_workaround` provenance) (S2), the CPU
> integer-exact math-parity reference (S3), the fail-closed `ClusterStoreGate`
> in `src/rurix-godot/src/lib.rs` plus the shim's 3-structured-buffer record
> entry (S4), and the standalone real D3D12 dispatch smoke
> (`ci/grx014_cluster_store_d3d12_dispatch_smoke.py`) (S6).
>
> This slice does **NOT** author a Godot patch (patches 0023-0025 are a later
> serial slice under the §8 stack-lock), runtime resource binding, an in-engine
> visual diff, or a real-pass enablement (S5/S7/S8/S9). measured ceiling here
> is: DXC compile + DXV validation pass + CPU integer-exact parity reference +
> one standalone real D3D12 dispatch verified word-exact against that
> reference.
>
> The pass ships **default disabled**; any compile / validation / visual / perf
> failure runs the native Godot cluster_store path. Section 3's investigation
> only records paths and function names; Godot-side changes land only as
> `spike/godot-rurix/patches/` patch files, never by editing the
> `external/godot-master` snapshot.

## 1. Pass identity

- `pass_id = cluster_store`
- bridge pass id: `RXGD_PASS_CLUSTER_STORE = 1` (the `RXGD_PASS_*` enum in
  `src/rurix-godot/src/lib.rs`; the id has existed since the bridge scaffold,
  and this slice replaces its historical placeholder estimated-timing path with
  the fail-closed gate)
- real-pass cap bit: `RXGD_CAP_CLUSTER_STORE_REAL_PASS = 1u << 8` (reserved in
  `PATCH_ALLOCATION.md` §3, bit 8; set only by the later real-pass opt-in patch
  0025)
- Tier: Tier 2 opener (raw-buffer compute pass; GRX-014)
- target backend: `Godot 4.7-dev Windows D3D12 Forward+`
- default enable state: `disabled`

## 2. Target scenes

- `clustered_lights`
- `mixed_forward_plus`

(cluster_store is the per-frame compute merge segment of `bake_cluster()`: it
packs the rasterized per-cluster usage/depth bitfields into the renderer-facing
cluster buffer for omni lights / spot lights / decals / reflection probes. The
`clustered_lights` bench scene drives it hardest: 512 omni + 384 spot
overlapping lights.)

## 3. Godot-side hook / call-site / resource-flow investigation

Records paths and functions only; **`external/godot-master` is not modified.**

### 3.1 Builder class

- Source: `servers/rendering/renderer_rd/cluster_builder_rd.cpp`
- Header: `servers/rendering/renderer_rd/cluster_builder_rd.h`
- Key functions / types:
  - `ClusterBuilderRD::bake_cluster()` (`cluster_builder_rd.cpp:438-542`) — the
    per-frame driver. It clears `cluster_buffer` (L444), and when
    `render_element_count > 0`: clears `cluster_render_buffer` (L448), fills the
    state uniform (L450-464), uploads `render_elements` to `element_buffer`
    (L468), runs the **rasterization segment** ("Render 3D Cluster Elements",
    L470-513: a draw_list over sphere/cone/box proxy meshes through
    `cluster_render.shader_pipelines`), then runs the **compute store segment**
    ("Pack 3D Cluster Elements", L515-538: one `cluster_store.shader_pipeline`
    dispatch). The store segment is the GRX-014 target; the rasterization
    segment is NOT replaceable (see §5.2).
  - `ClusterBuilderRD::setup(...)` (`cluster_builder_rd.cpp:281-376`) — creates
    `cluster_render_buffer` (L307), `cluster_buffer` (L308) and `element_buffer`
    (L313), and the `cluster_store_uniform_set` (L349-375; set 0: binding 1 =
    cluster_render_buffer, binding 2 = cluster_buffer, binding 3 =
    element_buffer). `max_elements_by_type` is 32-aligned (L293-296);
    `cluster_screen_size = ceil(screen_size / cluster_size=32)` (L290-291).
  - `ClusterBuilderSharedDataRD::ClusterStore::PushConstant`
    (`cluster_builder_rd.h:91-100`, 32 bytes / 8 dwords).
  - `RenderElementData` (`cluster_builder_rd.h:161-169`, 80 bytes; the C++
    `has_wide_spot_angle` field is the glsl `pad`).

### 3.2 Shader

- `servers/rendering/renderer_rd/shaders/cluster_store.glsl` (this kernel's
  math target; compute, `local_size = 8x8x1`, main at L46-119):
  - bounds check `pos >= cluster_screen_size` (L47-50); `base_offset = pos.x +
    cluster_screen_size.x * pos.y`; `src_offset = base_offset *
    cluster_render_data_size` (L55-56).
  - outer while over `render_element_offset < render_element_count_div_32`
    scanning usage words (L61-62); inner while `bits != 0` with
    `findLSB`-driven per-bit iteration (L63-66); per element reads `type`
    (L67), the z_range word at `src_offset + max_render_element_count_div_32 +
    index` (L69-70).
  - `z_range != 0` guard (L73, "should always be > 0"); `from_z =
    findLSB(z_range)`, `to_z = findMSB(z_range) + 1` (L75-76);
    `touches_near → from_z = 0`, `touches_far → to_z = 32` (L78-84).
  - `dst_offset = (base_offset + type * (cluster_screen_size.x *
    cluster_screen_size.y)) * (max_cluster_element_count_div_32 + 32)` (L87).
  - per-slice packed min/max write (L91-105): `minmax == 0 → 0xFFFF`
    initialization; `elem_min = min(orig_index, minmax & 0xFFFF)`; `elem_max =
    max(orig_index + 1, minmax >> 16)` ("always store plus one, so zero means
    range is empty"); store `elem_min | (elem_max << 16)`.
  - existence bitmap `cluster_store.data[dst_offset + (orig_index >> 5)] |=
    1 << (orig_index & 0x1F)` (L107-111); clear-bit continue (L114).
  - **No atomics**: each thread owns exactly one cluster (`base_offset`), and
    every write lands in that cluster's own `(cluster, type)` blocks, so
    threads never contend.
- `servers/rendering/renderer_rd/shaders/cluster_data_inc.glsl` declares three
  layout macros (`CLUSTER_COUNTER_SHIFT` / `CLUSTER_LIGHT_COUNT_MASK` /
  `CLUSTER_POINTER_MASK`) that are **zero-reference historical leftovers** —
  no tracked shader or C++ file consumes them. They do NOT describe the
  cluster_buffer layout this pass writes; do not use them.

### 3.3 Call / injection candidate point

- `bake_cluster()` store segment (`cluster_builder_rd.cpp:517-538`):
  - push constants assembled at L522-531 (`cluster_render_data_size =
    render_element_max / 32 + render_element_max`,
    `max_render_element_count_div_32 = render_element_max / 32`,
    `cluster_screen_size[2]`, `render_element_count_div_32 =
    ceil(render_element_count / 32)`, `max_cluster_element_count_div_32 =
    max_elements_by_type / 32`, `pad1 = pad2 = 0`).
  - `compute_list_dispatch_threads(cluster_screen_size.x,
    cluster_screen_size.y, 1)` at L535 (local 8x8 → `ceil(cluster_screen_size
    / 8)²` groups).
  - **Injection point** (later patch 0023): an opt-in gate before this store
    dispatch; the native dispatch runs whenever the gate returns false (which
    is always by default). The store segment only runs when
    `render_element_count > 0` (L446), so the gate inherits that guard.
- Upstream driver: `render_forward_clustered.cpp:1650`
  (`current_cluster_builder->bake_cluster()`), inside the forward-clustered
  render loop; the builder is (re)configured by
  `render_forward_clustered.cpp:158` (`cluster_builder->setup(...)`) with
  `p_render_buffers->get_max_cluster_elements()`.
- `max_cluster_elements` default: 512 via the project setting
  `rendering/limits/cluster_builder/max_clustered_elements`
  (`rendering_server.cpp:3798`; cached at `renderer_scene_render_rd.h:163` /
  `renderer_scene_render_rd.cpp:1699`), 32-aligned by `setup()`. The kernel
  carries all sizes through b0 and never hardcodes 512.

### 3.4 Resource flow (native)

- Input 1: `cluster_render_buffer` (`uint[]` SSBO, set 0 binding 1 of
  `cluster_store.glsl`; created `setup()` L307, size `cluster_screen_size.x *
  cluster_screen_size.y * (render_element_max / 32 + render_element_max) * 4`).
  Per cluster: `render_element_max / 32` usage-bitmap words followed by
  `render_element_max` per-element z_range words. Written by the rasterization
  segment; the store segment reads it only.
- Input 2: `element_buffer` (`RenderElementData[]` SSBO, set 0 binding 3;
  created `setup()` L313, stride 80). The store segment reads only the four
  leading u32 fields `type` / `touches_near` / `touches_far` / `original_index`
  per element; `transform_inv` / `scale` / pad are raster-segment inputs.
- Output: `cluster_buffer` (`uint[]` SSBO, set 0 binding 2; created `setup()`
  L308, size `cluster_screen_size.x * cluster_screen_size.y *
  (max_elements_by_type / 32 + 32) * ELEMENT_TYPE_MAX(4) * 4`; consumed by the
  renderer as the per-cluster light/decal/probe index table). Layout: for each
  `(cluster, type)` block at `dst_offset` — `[existence bitmap:
  max_cluster_element_count_div_32 words][32 Z-slice (min u16 | (max+1) u16)
  words]`. Cleared to zero by `bake_cluster()` L444 before the frame's writes.

## 4. Input / output resources (Rurix mapping)

- Input 1: `cluster_render = StructuredBuffer<uint>`, SRV `t0 space0`,
  `binding_kind = structured_buffer` (Godot `cluster_render_buffer` native
  `ID3D12Resource*`).
- Input 2: `render_elements = StructuredBuffer<RenderElementData>`, SRV
  `t1 space0`, `binding_kind = structured_buffer` (Godot `element_buffer`;
  80-byte stride).
- Output: `cluster_store = RWStructuredBuffer<uint>`, UAV `u0 space0`,
  `binding_kind = rwstructured_buffer` (Godot `cluster_buffer`).
- b0 root constants: 32-byte / 8-dword mirror of
  `ClusterBuilderSharedDataRD::ClusterStore::PushConstant`
  (`root_parameter_index 0`; field-by-field in `resource_mapping.md` and
  `artifacts/cluster_store_descriptor_layout.json`). Like GRX-013
  particles_copy — and unlike the GRX-009/010/011/012 texture passes —
  cluster_store carries **no i64** field, so the `SHADER_INT64` capability is
  not part of its binding preflight.
- tracked mapping: `resource_mapping.md`.

## 5. Supported subset and route choice

### 5.1 In-scope subset (this slice)

- The **complete store-segment math** of `cluster_store.glsl` — the pass is a
  single kernel with no mode/variant switches, so there is no subset cut: usage
  word scan (findLSB per-bit iteration), per-element type / z_range read, the
  `z_range != 0` guard, `from_z/to_z` from findLSB/findMSB with
  `touches_near`/`touches_far` overrides, the per-slice packed
  `(min u16 | (max+1) u16)` write with the `minmax == 0 → 0xFFFF`
  initialization branch, and the existence-bitmap `|=` write.
- One thread per cluster (8x8 groups over `cluster_screen_size`); all four
  element types (omni light / spot light / decal / reflection probe) flow
  through the same `type`-indexed math.
- All sizes/counts come from b0 (`cluster_render_data_size`,
  `max_render_element_count_div_32`, `cluster_screen_size`,
  `render_element_count_div_32`, `max_cluster_element_count_div_32`); nothing
  is hardcoded (the Godot default `max_clustered_elements = 512` is only a
  deployment default, recorded as a known assumption for fixtures/scenes).

### 5.2 Out of scope (known gaps; `pass_manifest.json` `known_gaps` per line)

- The `bake_cluster()` **rasterization segment** ("Render 3D Cluster
  Elements": the `cluster_render.glsl` vertex/fragment proxy-mesh draw over
  sphere/cone/box geometry, plus its MSAA/attachment pipeline variants) is NOT
  replaced — it is a graphics pipeline, not a compute pass, and it produces
  this kernel's `cluster_render_buffer` input. The Rurix pass consumes its
  output as-is.
- The `buffer_clear` of `cluster_buffer` / `cluster_render_buffer`
  (`bake_cluster()` L444/L448) stays native (the kernel assumes a zeroed
  destination exactly like the native shader).
- The `render_element_count == 0` early-out stays native (no dispatch happens
  at all in that frame).
- The runtime hook / native-handle binding (patches 0023-0025) is a later
  serial slice; the synthetic parity fixtures use small cluster grids and
  32-aligned element capacities, with the Godot default
  `max_clustered_elements = 512` documented as the deployment-scale assumption
  rather than exercised at full scale offline.
- GPU-observed math parity beyond the standalone S6 smoke (in-engine
  observation is the later enablement slice).

### 5.3 Offline kernel route: HLSL bridge (chosen), not rurixc-native

cluster_store is an **all raw-buffer / SSBO** pass, so the GRX-009
texture-intrinsic `llc` blocker does **not** apply. Even so, a rurixc-owned
`rx → DXIL` compile of the store kernel is infeasible today, for three
different reasons:

1. **No u32 buffer views.** The DXIL compute-body lowering accepts only
   `View<global, f32>` / `ViewMut<global, f32>` (and `Texture2D<f32>` /
   `RWTexture2D<f32>`) resource parameters
   (`src/rurixc/src/dxil_codegen.rs:1754/1786`). All three cluster_store
   buffers are `uint[]` / u32-field SSBOs, and the math is bit-pattern math on
   u32 words — an f32 view cannot carry it (f32 round-tripping is not
   bit-faithful for arbitrary u32 payloads).
2. **Integer bit operations are not wired on the DXIL path.** The kernel is
   built from `&`, `|`, `~`, `<<`, `>>` word manipulation. MIR carries
   `BinOp::BitAnd/BitOr/BitXor/Shl/Shr` (`src/rurixc/src/mir.rs:643-647`), but
   the DXIL backend (`src/rurixc/src/dxil_codegen.rs`) has no lowering for any
   of them.
3. **No findLSB/findMSB intrinsics.** The scan loop and the z-range decode
   need `findLSB`/`findMSB` (HLSL `firstbitlow`/`firstbithigh`); the Rurix
   lang subset has no such intrinsic on any backend.

The canonical offline package is therefore the owner-approved
`hlsl_bridge_workaround`: a DXC `cs_6_0` DXIL container (validated by DXV) with
a **Rurix-owned RTS0** root signature (`rurixc::binding_layout::
{infer_root_signature, pack_root_constants, serialize_rts0}`). This mirrors the
GRX-013 particles_copy buffer-pass precedent (`../particles_copy/
PASS_CONTRACT.md` §5.3, itself mirroring `../luminance_reduction/
texture_artifact_provenance_policy.json`), with the raw-buffer blocker
rationale above (the texture-intrinsic condition does not apply here).
`src/lib.rx` documents the kernel's structure and the three blockers; the
executable math lives only in the HLSL bridge kernel.

## 6. Fallback

- fallback reason enum (aligned with the GRX-008 five): `compile_failed` /
  `validation_failed` / `unsupported_device` / `visual_diff_failed` /
  `manual_disabled`.
- Any compile / validation / visual / perf failure → native Godot
  cluster_store path (`godot_native_cluster_store`).
- The default Godot config (per-pass settings all `false`) and the shipping
  bridge return `RXGD_STATUS_FALLBACK` for `RXGD_PASS_CLUSTER_STORE`; the
  shipping feature-off bridge fails closed with
  `real_dispatch_path_not_linked`.

## 7. Bridge gate (S4, delivered this slice)

The fail-closed `ClusterStoreGate` in `src/rurix-godot/src/lib.rs` mirrors the
GRX-013 `ParticlesCopyGate` template with a THREE-structured-buffer binding
surface (SRV t0 cluster_render + SRV t1 render_elements + UAV u0
cluster_store):

- **runtime binding preflight**: exactly 3 buffer resources in
  cluster_render / render_elements / cluster_store order, the 32-byte b0
  `ClusterStore::PushConstant` mirror, nonzero `cluster_screen_size[0..1]`
  (dwords 2-3) and nonzero `render_element_count_div_32` (dword 4; the native
  call site only dispatches when `render_element_count > 0`), nonzero buffer
  byte sizes; NO int64 cap check (cluster_store's b0 carries no i64 fields).
- **dispatch eligibility**: opt-in `RXGD_CAP_CLUSTER_STORE_REAL_PASS
  (1u << 8)` + the int64 device/recording-harness capability (a
  harness/device capability, not a kernel binding requirement) + non-null
  native device/queue handles +
  non-null buffer handles + `ClusterStoreDispatchPackage` layout/digest match
  vs the S2 offline evidence (three SHA-256 digests baked into the gate).
- **per-slot kernel-binding-kind conformance**: `["structured_buffer",
  "structured_buffer", "rwstructured_buffer"]`; texture resources fail closed
  at any slot.
- **math parity gate**:
  `cluster_store_cpu_reference_proven_pending_gpu_dispatch`.
- **real dispatch** only under the `d3d12-recording-shim` feature through the
  shim's additive 3-buffer entry `rxgd_cluster_store_record_dispatch`
  (production/instrumented split per the Wave 4 `readback` selector +
  `RXGD_DISPATCH_INSTRUMENTED`; engagement counters via the shim session
  `note()`); the shipping feature-off bridge fails closed.

Every failure prints the once-per-session machine-readable
`RXGD_CLUSTER_STORE_REAL_PASS_BLOCKED first_missing_prerequisite=...`
diagnostic. Because int64 is NOT in the preflight, a forced capability
downgrade fails closed at `dispatch_eligibility_failed` (the GRX-013
precedent), not `runtime_binding_preflight_failed`.

## 8. Godot patches — later serial slice (S5/S7)

Not authored here. `PATCH_ALLOCATION.md` §2 reserves **0023-0025** for
cluster_store (0023 gate+callsite / 0024 runtime binding / 0025
recording+real-pass opt-in). Patches are generated by `git diff --no-index` on
a scratch copy with all prior patches applied, verified by
`ci/godot_rurix_patch_stack.py`; never hand-written; serialized by the §4
stack-lock.

## 9. Evidence

- **offline compile** (this slice, measured): `offline_compile_evidence.json` —
  DXC `cs_6_0` compile, DXV validation, Rurix-owned RTS0
  (`emit_grx014_cluster_store_rts0` via
  `rurixc::binding_layout::{infer_root_signature, pack_root_constants,
  serialize_rts0}`), descriptor layout (per-slot binding kinds + 32-byte /
  8-dword root constants), three artifact SHA-256 recomputable on disk;
  `provenance = hlsl_bridge_workaround`, `rurix_owned = false`,
  `rurix_owned_rts0 = true`, `runtime_mappable = true`.
- **math parity** (this slice): `math_parity_evidence.json`
  (`generate_math_parity_evidence.py`) — deterministic synthetic
  cluster_render / element_buffer fixtures with an **integer-exact** CPU
  reference (pure u32 word math; no float tolerance) covering touches_near /
  touches_far overrides, single- and multi-slice z ranges, the
  `minmax == 0 → 0xFFFF` initialization branch, same-slice min/max merging,
  the `z_range == 0` guard, empty clusters, and the existence bitmap;
  `status = pending_gpu_dispatch` until S6 fills the GPU-observed side.
- **standalone dispatch** (this slice, measured):
  `real_d3d12_dispatch_smoke.json` via
  `ci/grx014_cluster_store_d3d12_dispatch_smoke.py` — one real D3D12 dispatch
  per tracked case over the tracked DXIL/RTS0/descriptor artifacts, every
  output word compared **exactly** (integer equality, zero tolerance) against
  the tracked CPU reference; `real_d3d12_dispatch_recorded = true`,
  `cpu_reference_match = true` on success while `runtime_state` stays
  `fallback_only` and `real_gpu_pass` stays `false`.
- **visual / telemetry / enablement**: later slices (S8); this file claims none
  of them.
- Perf: reuse GRX-006 baseline / perf gate; no performance improvement claimed.

## 10. Exit criteria (this slice)

- pass default `disabled`; manifest `implemented=false`,
  `runtime_state=fallback_only`, `real_gpu_pass=false` (fail-closed values;
  the standalone S6 smoke records `real_d3d12_dispatch_recorded=true` in its
  own evidence document without flipping the runtime manifest state).
- `offline_compile_evidence.json` `status=success` with DXV
  `validation.status=pass`; `math_parity_evidence.json`
  `status=pending_gpu_dispatch`; `real_d3d12_dispatch_smoke.json`
  `status=success` with `cpu_reference_match=true`.
- `cargo test -p rurix-godot` (both feature legs) green; the probe walks
  grx011 → grx012 → grx013 ready and stops honestly at the not-ready grx014
  gate (`next_action` stays `start_grx014_cluster_store_pass_contract`).
- This slice does NOT represent pass completion.

## 11. Remaining items

- S5 patch 0023 (gate + `bake_cluster()` call-site opt-in); S7 patches
  0024/0025 (runtime native-handle binding; recording-smoke + real-pass
  opt-in); S8 scratch rebuild + gated real-pass enablement smoke; S9 close-out
  (manifest flip + owner default-enable decision + gate-module readiness).
- Raster/compute seam design (the rasterization segment stays native
  permanently; any net-benefit measurement must account for the seam).
- full baseline / per-pass FPS comparison; any performance claim.
