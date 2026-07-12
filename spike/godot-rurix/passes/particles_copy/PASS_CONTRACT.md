# GRX-013 particles_copy Pass — PASS CONTRACT

> **Status (2026-07-12, slice S1-S3: contract trio + offline kernel + math parity).**
> This slice delivers the OFFLINE face only, reusing the matured GRX-009/010/011
> per-pass template (`../PASS_TEMPLATE.md`): the pass contract trio, the
> HLSL-bridge math-equivalent kernel (DXC `cs_6_0` compile + DXV validation +
> Rurix-owned RTS0 via `rurixc::binding_layout`, owner-approved
> `hlsl_bridge_workaround` provenance), and the CPU math-parity reference.
>
> This slice does **NOT** author a bridge gate, a Godot patch, runtime resource
> binding, a GPU dispatch smoke, an in-engine visual diff, or a real-pass
> enablement. Those are later slices (S4-S9). measured ceiling here is: DXC
> compile + DXV validation pass + CPU float32 parity reference.
>
> The pass ships **default disabled**; any compile / validation / visual / perf
> failure runs the native Godot particles_copy path. Section 3's investigation
> only records paths and function names; Godot-side changes land only as
> `spike/godot-rurix/patches/` patch files, never by editing the
> `external/godot-master` snapshot.

## 1. Pass identity

- `pass_id = particles_copy`
- bridge pass id: `RXGD_PASS_PARTICLES_COPY = 7` (the `RXGD_PASS_*` enum in
  `src/rurix-godot/src/lib.rs`; wired in a later slice)
- real-pass cap bit: `RXGD_CAP_PARTICLES_COPY_REAL_PASS = 1u << 7` (reserved in
  `PATCH_ALLOCATION.md` §3, bit 7; set only by the later real-pass opt-in patch)
- Tier: Tier 1 (raw-buffer compute pass; GRX-013)
- target backend: `Godot 4.7-dev Windows D3D12 Forward+`
- default enable state: `disabled`

## 2. Target scenes

- `mixed_forward_plus`
- `gpu_particles_3d`

(particles_copy is the per-frame "simulation result → render instance buffer"
transform for 3D GPU particles; the first subset covers 3D billboard/disabled
alignment without trails or view-depth sort.)

## 3. Godot-side hook / call-site / resource-flow investigation

Records paths and functions only; **`external/godot-master` is not modified.**

### 3.1 Storage class

- Source: `servers/rendering/renderer_rd/storage_rd/particles_storage.cpp`
- Header: `servers/rendering/renderer_rd/storage_rd/particles_storage.h`
- Key functions / types:
  - `RendererRD::ParticlesStorage::particles_set_view_axis(...)`
    (`particles_storage.cpp:1235`) — the driver for the copy pass. It early-returns
    unless the draw order / transform align is a view-dependent variant
    (`PARTICLES_DRAW_ORDER_VIEW_DEPTH`, `Z_BILLBOARD`, `Z_BILLBOARD_Y_TO_VELOCITY`,
    `LOCAL_BILLBOARD`); assembles a `CopyPushConstant`; optionally runs a
    `FILL_SORT_BUFFER` pass + `sort_effects->sort_buffer(...)` when sorting; then
    dispatches `COPY_MODE_FILL_INSTANCES` (or `..._WITH_SORT_BUFFER`).
  - `ParticlesShader::CopyPushConstant` (`particles_storage.h:303-329`, 128 bytes).
  - `ParticlesStorage::_particles_update_buffers(...)`
    (`particles_storage.cpp:1354`) — creates `particle_buffer` (Particles SSBO)
    and `particle_instance_buffer` (Transforms SSBO), and the
    `particles_copy_uniform_set` (set 0: binding 1 = particles, binding 2 =
    instances).
- copy-mode enum: `COPY_MODE_FILL_INSTANCES` / `COPY_MODE_FILL_SORT_BUFFER` /
  `COPY_MODE_FILL_INSTANCES_WITH_SORT_BUFFER` (`particles_storage.h:334-338`).

### 3.2 Shader

- `servers/rendering/renderer_rd/shaders/particles_copy.glsl` (this kernel's
  math target; compute, `local_size_x = 64`):
  - `struct ParticleData { mat4 xform; vec3 velocity; uint flags; vec4 color;
    vec4 custom; }` (L13-22; `#ifdef USERDATA_COUNT` appends `vec4 userdata[]`).
  - `#ifdef MODE_FILL_INSTANCES` (L109-347): bounds check; optional sort-buffer /
    lifetime reindex (out of scope); active branch (L162) reads `xform`, applies
    the `align_mode` switch (L173-304), integrates `velocity * frame_remainder`
    (L306), optional trail multiply (L308-311) and 2D `inv_emission_transform`
    (L313-323); inactive branch (L324-328) sets a zero basis + `(-inf,-inf,-inf,0)`
    translation column; `transpose` (L329); write (L331-347): 3D writes 5 vec4
    (rows 0..2 + color + custom), 2D writes 4 vec4.
  - align modes (L77-81): `ALIGN_DISABLED 0`, `ALIGN_BILLBOARD 1`,
    `ALIGN_Y_TO_VELOCITY 2`, `ALIGN_Z_BILLBOARD_Y_TO_VELOCITY 3`,
    `ALIGN_LOCAL_BILLBOARD 4`. `ALIGN_BILLBOARD` (L176-207) reads a `custom`
    channel angle (`align_channel_filter`), builds a Rodrigues rotation about
    `normalize(sort_direction)`, and re-bases the transform.

### 3.3 Call / injection candidate point

- `particles_set_view_axis` `COPY_MODE_FILL_INSTANCES` dispatch
  (`particles_storage.cpp:1337-1351`):
  - push constants assembled at `L1273-1316`
    (`copy_push_constant.total_particles`, `sort_direction`, `align_up`,
    `align_mode`, `align_src` (= glsl `align_channel_filter`), `frame_remainder`,
    `motion_vectors_current_offset`, `order_by_lifetime`/`copy_mode_2d` bitfield).
  - `copy_mode_2d` set at `L1339`; the FILL_INSTANCES pipeline bound at `L1340`,
    dispatched `compute_list_dispatch_threads(total_particles, 1, 1)` at `L1349`.
  - **Injection point** (later patch 0020): an opt-in gate before this
    FILL_INSTANCES dispatch; the native dispatch runs whenever the gate returns
    false (which is always by default). The FILL_SORT_BUFFER block (`L1318-1331`)
    is out of scope.
- Upstream driver: `renderer_scene_cull.cpp:2933`
  (`call_on_render_thread(... _scene_particles_set_view_axis ...)`), which at
  `renderer_scene_cull.cpp:3195-3196` calls
  `particles_storage->particles_set_view_axis(...)`. **This is a cull-stage
  driver, structurally different from the post-process passes
  (luminance/tonemap/ssao_blur); the runtime hook/binding design is deferred to
  the later patch slice.**

### 3.4 Resource flow (native)

- Input: `particles->particle_buffer` (Particles SSBO, `ParticleData[]`, set 0
  binding 1; created in `_particles_update_buffers` `L1394`, stride
  `sizeof(ParticleData) + userdata_count*16`).
- Output: `particles->particle_instance_buffer` (Transforms SSBO, `vec4 data[]`,
  set 0 binding 2; `L1399/L1407`, size `total_amount*(xform_size+1+1)*16`, where
  `xform_size = 3` for 3D; doubled when motion vectors are enabled).
- Optional: `particles_sort_buffer` (set 1) and `trail_bind_pose_uniform_set`
  (set 2) — both out of scope.

## 4. Input / output resources (Rurix mapping)

- Input: `src_particles = StructuredBuffer<ParticleData>`, SRV `t0 space0`,
  `binding_kind = structured_buffer` (Godot Particles SSBO native
  `ID3D12Resource*`). ParticleData stride = 112 bytes (mat4 64 + velocity 12 +
  flags 4 + color 16 + custom 16), no userdata.
- Output: `dst_instances = RWStructuredBuffer<float4>`, UAV `u0 space0`,
  `binding_kind = rwstructured_buffer` (Godot Transforms SSBO; 3D stride = 5
  vec4 per instance).
- b0 root constants: 128-byte / 32-dword mirror of `CopyPushConstant`
  (`root_parameter_index 0`; field-by-field in `resource_mapping.md` and
  `artifacts/particles_copy_descriptor_layout.json`). Unlike the GRX-009/010/011
  texture passes, particles_copy carries **no i64** field, so the `SHADER_INT64`
  capability is not part of its preflight.
- tracked mapping: `resource_mapping.md`.

## 5. Supported subset and route choice

### 5.1 In-scope subset (first slice)

- `COPY_MODE_FILL_INSTANCES`, **3D mode**, one thread per instance.
- `align_mode ∈ {ALIGN_DISABLED (0), ALIGN_BILLBOARD (1)}`.
- active branch: `active = (flags & ACTIVE) || (flags & TRAILED)`; inactive →
  zero basis + `(-inf,-inf,-inf,0)` translation column.
- `ALIGN_BILLBOARD`: `axis = normalize(sort_direction)`; Rodrigues rotation by
  the selected `custom` channel angle; `new_up = rotated * align_up`;
  `local = mat3(normalize(cross(new_up, sort_direction)), new_up,
  sort_direction) * mat3(txform)`.
- `txform[3].xyz += velocity * frame_remainder`; `transpose`; write 5 vec4.
- column-major mat4 (Godot `txform[i]` = column i); the kernel loads the mat4 as
  4 explicit float4 columns and mirrors the GLSL column-major algebra.

### 5.2 Out of scope (known gaps; `pass_manifest.json` `known_gaps` per line)

- 2D copy mode (`PARAMS_FLAG_COPY_MODE_2D` + `inv_emission_transform` + 4-vec4
  write); `MODE_FILL_SORT_BUFFER` / `..._WITH_SORT_BUFFER` (VIEW_DEPTH sort);
  `ORDER_BY_LIFETIME`/`REVERSE_LIFETIME` reindex; trail interpolation +
  `trail_bind_poses` (`trail_size > 1`); userdata channels; align modes 2/3/4;
  `motion_vectors_current_offset ≠ 0` (carried, tested at 0); the cull-stage
  runtime hook; GPU-observed math parity (pending a real dispatch).

### 5.3 Offline kernel route: HLSL bridge (chosen), not rurixc-native

particles_copy is an **all raw-buffer / SSBO** pass, so the GRX-009
texture-intrinsic `llc` blocker does **not** apply. Even so, a rurixc-owned
`rx → DXIL` compile of the in-scope subset is infeasible today, for two
different reasons:

1. **Aggregate SSBO element types.** The Rurix lang subset for compute kernels
   models raw buffers as scalar `View<f32>` / `ViewMut<f32>` with dynamic
   indexing; there are no `struct` / `vec4` / `mat4` aggregate SSBO element
   types. `ParticleData` is a `mat4 + vec3 + uint + vec4 + vec4` aggregate.
2. **Transcendental / sqrt math on the DXIL path.** `ALIGN_BILLBOARD` needs
   `sin`/`cos` (Rodrigues) and `sqrt`/`rsqrt` (`normalize`/`cross`-`normalize`).
   Rurix's `DeviceMathFn` (`sqrt`/`rsqrt`/`sin`/`cos`/…) lowers **only** on the
   NVPTX device path (libdevice `__nv_*` external symbols, resolved by
   `-mlink-builtin-bitcode`; `src/rurixc/src/device_codegen.rs` /
   `tbir_build.rs`). The DXIL backend (`src/rurixc/src/dxil_codegen.rs`) has no
   lowering for these intrinsics, so the billboard subset cannot be lowered.

Because the in-scope subset **includes** `ALIGN_BILLBOARD`, the canonical
offline package is the owner-approved `hlsl_bridge_workaround`: a DXC `cs_6_0`
DXIL container (validated by DXV) with a **Rurix-owned RTS0** root signature
(`rurixc::binding_layout::{infer_root_signature, pack_root_constants,
serialize_rts0}`). This mirrors the GRX-009/010/011 precedent
(`../luminance_reduction/texture_artifact_provenance_policy.json`), with the
buffer-pass blocker rationale above (the texture-intrinsic condition does not
apply here). `src/lib.rx` records the `ALIGN_DISABLED` lane math in the scalar
raw-buffer form that IS in principle expressible, and documents the two
blockers; the `ALIGN_BILLBOARD` path lives only in the HLSL bridge kernel.

## 6. Fallback

- fallback reason enum (aligned with the GRX-008 five): `compile_failed` /
  `validation_failed` / `unsupported_device` / `visual_diff_failed` /
  `manual_disabled`.
- Any compile / validation / visual / perf failure → native Godot
  particles_copy path (`godot_native_particles_copy`).
- (Later slices) the default Godot config (per-pass settings all `false`) and
  the shipping bridge return `RXGD_STATUS_FALLBACK` for `RXGD_PASS_PARTICLES_COPY`.

## 7. Bridge gate — later slice (S4)

Not authored here. The later `ParticlesCopyGate` in
`src/rurix-godot/src/lib.rs` will mirror the GRX-010/011 template:
runtime binding preflight → dispatch eligibility (opt-in
`RXGD_CAP_PARTICLES_COPY_REAL_PASS (1u<<7)` + non-null native device/queue +
non-null handles + `ParticlesCopyDispatchPackage` layout/digest match vs the S2
offline evidence) → per-slot kernel-binding-kind conformance
(`["structured_buffer", "rwstructured_buffer"]`; texture resources fail closed)
→ math-parity gate (`fill_instances_cpu_reference_proven_pending_gpu_dispatch`)
→ real dispatch (only under a recording-shim feature; shipping feature-off fails
closed). The three S2 SHA-256 digests are baked into that gate.

## 8. Godot patches — later slice (S5/S7)

Not authored here. `PATCH_ALLOCATION.md` §2 reserves **0020-0022** for
particles_copy (0020 gate+callsite / 0021 runtime binding / 0022
recording+real-pass opt-in). Patches are generated by `git diff --no-index` on a
scratch copy with all prior patches applied, verified by
`ci/godot_rurix_patch_stack.py`; never hand-written.

## 9. Evidence

- **offline compile** (this slice, measured): `offline_compile_evidence.json` —
  DXC `cs_6_0` compile, DXV validation, Rurix-owned RTS0
  (`emit_grx013_particles_copy_rts0` via
  `rurixc::binding_layout::{infer_root_signature, pack_root_constants,
  serialize_rts0}`), descriptor layout (per-slot binding kinds + 128-byte / 32-
  dword root constants), three artifact SHA-256 recomputable on disk;
  `provenance = hlsl_bridge_workaround`, `rurix_owned = false`,
  `rurix_owned_rts0 = true`, `runtime_mappable = true`.
- **math parity** (this slice): `math_parity_evidence.json`
  (`generate_math_parity_evidence.py`) — deterministic synthetic ParticleData
  CPU float32 reference (binary32 per op) over ALIGN_DISABLED + ALIGN_BILLBOARD ×
  active/inactive; `status = pending_gpu_dispatch`. The GPU-observed side is
  filled by the later standalone dispatch smoke (S6).
- **standalone dispatch / visual / telemetry / enablement**: later slices
  (S6/S8); this file claims none of them.
- Perf: reuse GRX-006 baseline / perf gate; no performance improvement claimed.

## 10. Exit criteria (this slice)

- pass default `disabled`; manifest `implemented=false`,
  `runtime_state=fallback_only`, `real_gpu_pass=false`,
  `real_d3d12_dispatch_recorded=false` (fail-closed initial values).
- `offline_compile_evidence.json` `status=success` with DXV `validation.status=
  pass`; `math_parity_evidence.json` `status=pending_gpu_dispatch`.
- This slice does NOT represent pass completion.

## 11. Remaining items

- S4 bridge gate; S5 patch 0020; S6 standalone dispatch smoke; S7 patches
  0021/0022; S8 scratch rebuild + gated real-pass enablement; S9 close-out (gate
  module + probe registration + manifest flip + owner default-enable decision).
- align modes 2/3/4, 2D copy mode, sort/lifetime reindex, trail, userdata.
- full baseline / per-pass FPS comparison; any performance claim.
