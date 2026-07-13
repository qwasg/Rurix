# GRX-015 gpu_culling Pass — PASS CONTRACT

> **Status (2026-07-12, slice S1-S3: contract trio + offline kernel + math
> parity).**
> This slice delivers the OFFLINE face only, reusing the matured GRX-009..014
> per-pass template (`../PASS_TEMPLATE.md`): the pass contract trio (S1), the
> HLSL-bridge math-equivalent kernel (DXC `cs_6_0` compile + DXV validation +
> Rurix-owned RTS0 via `rurixc::binding_layout`, owner-approved
> `hlsl_bridge_workaround` provenance) (S2), and the CPU parity reference with
> zero-tolerance u32 outputs guarded by an asserted classification-margin floor
> (S3).
>
> This slice does **NOT** author a bridge gate, a Godot patch, runtime resource
> binding, a GPU dispatch smoke, an in-engine visual diff, or a real-pass
> enablement. Those are later slices (S4-S9). The measured ceiling here is:
> DXC compile + DXV validation pass + CPU parity reference (bitmask + counts,
> zero tolerance) with `status=pending_gpu_dispatch`.
>
> The pass ships **default disabled**; any compile / validation / visual / perf
> failure runs the native CPU-driven path. Section 3's investigation only
> records paths and function names; Godot-side changes land only as
> `spike/godot-rurix/patches/` patch files, never by editing the
> `external/godot-master` snapshot.

## 1. Pass identity

- `pass_id = gpu_culling`
- bridge pass id: `RXGD_PASS_GPU_CULLING = 8` (the `RXGD_PASS_*` enum namespace
  in `src/rurix-godot/src/lib.rs`, allocated per the `PATCH_ALLOCATION.md` §3
  namespace note; wired in a later slice)
- real-pass cap bit: `RXGD_CAP_GPU_CULLING_REAL_PASS = 1u << 9` (reserved in
  `PATCH_ALLOCATION.md` §3, bit 9; set only by the later real-pass opt-in patch
  0029)
- Tier: Tier 2 (raw-buffer compute pass; GRX-015 "GPU culling", GRX_PLAN §6)
- target backend: `Godot 4.7-dev Windows D3D12 Forward+`
- default enable state: `disabled`

## 2. Target scenes

- `many_mesh_instances`
- `mixed_forward_plus`

(GRX_PLAN §6 verification metric for GRX-015 is the `many_mesh_instances`
scene draw/dispatch count + FPS/p95 — later slices; this slice claims none of
it. The GRX_PLAN row also mandates: **CPU fallback must be retained** — that
is a hard red line of this contract, see §6.)

## 3. Godot-side hook / call-site / resource-flow investigation

Records paths and functions only; **`external/godot-master` is not modified.**

### 3.1 Indirect MultiMesh infrastructure (exists end-to-end, zero callers)

The tracked Godot snapshot carries a complete indirect-MultiMesh draw path
that **no tracked caller currently activates** (the Resource layer never sets
`use_indirect`; see §3.5). GRX-015 targets exactly this dormant path.

- Source: `servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp`
- Header: `servers/rendering/renderer_rd/storage_rd/mesh_storage.h`
- Key functions / types:
  - `MeshStorage::_multimesh_allocate_data(...)` (`mesh_storage.cpp:1547`) —
    signature carries `bool p_use_indirect`; sets `multimesh->indirect`
    (L1583) and resets `multimesh->command_buffer` (L1584); creates the
    transform SSBO `multimesh->buffer` sized
    `instances * stride_cache * sizeof(float)` (L1596-1598). `stride_cache`
    for 3D = 12 floats + 4 if colors + 4 if custom data (L1573-1577).
  - `MeshStorage::_multimesh_set_mesh(...)` (`mesh_storage.cpp:1666`) — when
    `multimesh->indirect` (L1676-1696): builds the **command_buffer** with
    `INDIRECT_MULTIMESH_COMMAND_STRIDE = 5` u32 per surface
    (`mesh_storage.h:63`), one 5-dword command block per mesh surface; dword 0
    is initialized to `mesh_surface_get_vertices_drawn_count(...)` (L1685-
    1690), the remaining dwords are zero-initialized (`resize_initialized`,
    L1683); created via `storage_buffer_create(..., 
    RD::STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT)` (L1693).
  - `MeshStorage::_multimesh_set_visible_instances(...)`
    (`mesh_storage.cpp:2187`) — **the CPU write point this pass aligns with**:
    when `multimesh->indirect`, it writes the visible count into **the second
    dword of every surface's command block**:
    `buffer_update(command_buffer, (i * sizeof(uint32_t) *
    INDIRECT_MULTIMESH_COMMAND_STRIDE) + sizeof(uint32_t), sizeof(uint32_t),
    &p_visible)` (`mesh_storage.cpp:2210`), i.e. byte offset
    `(surface_index * 5 + 1) * 4`.
  - `MeshStorage::_multimesh_get_command_buffer_rd_rid(...)`
    (`mesh_storage.cpp:2153`) — command-buffer RID accessor used by the draw
    side.
  - Transform layout: `_multimesh_instance_set_transform(...)`
    (`mesh_storage.cpp:1880-1915`) packs each 3D instance as **12 floats,
    row-major 3x4**: lanes 0-3 = `(basis.rows[0], origin.x)`, lanes 4-7 =
    `(basis.rows[1], origin.y)`, lanes 8-11 = `(basis.rows[2], origin.z)`.
- Observed initial-state quirk (runtime-hook design input): the only tracked
  writers of the command block's instance-count dword are the creation zeroing
  (L1683) and `_multimesh_set_visible_instances` (L2210); a fresh indirect
  MultiMesh therefore draws 0 instances until the first visible-instances
  write. The later patch slices must account for this when zeroing/accumulating
  the count dword.

### 3.2 Draw side (consumer of the command buffer)

- `servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp:602-613`
  (`_render_list_template`): `indirect = bool(surf->owner->base_flags &
  INSTANCE_DATA_FLAG_MULTIMESH_INDIRECT)` (L602); the indirect arm (L610)
  issues `draw_list_draw_indirect(draw_list, index_array_rd.is_valid(),
  mesh_storage->_multimesh_get_command_buffer_rd_rid(...),
  surf->surface_index * sizeof(uint32_t) * INDIRECT_MULTIMESH_COMMAND_STRIDE,
  1, 0)` — one command block per surface, at the 5-dword stride offset.
- The flag is derived at `render_forward_clustered.cpp:4355-4357` from
  `mesh_storage->multimesh_uses_indirect(...)`.
- D3D12 backend: `drivers/d3d12/rendering_device_driver_d3d12.cpp` —
  `command_render_draw_indirect` (L4909) executes via
  `cmd_list->ExecuteIndirect(indirect_cmd_signatures.draw...)` (L4918; indexed
  variant L4892). The device-level indirect command signatures are prebuilt —
  the whole chain down to `ExecuteIndirect` exists in the snapshot.

### 3.3 Call / injection candidate point

**Structural difference from GRX-009..014: gpu_culling is an ADDITIVE pass.**
There is no native Godot compute shader being replaced — the "native path" is
the CPU-driven command-buffer contents (§3.1) plus the non-indirect
`draw_list_draw` arm. Consequences:

- There is no Godot push-constant struct to mirror; the b0 layout in §4 is
  **Rurix-defined** (unlike the `CopyPushConstant` / `ClusterStore::
  PushConstant` mirrors of GRX-013/014).
- **Injection point** (later patches 0027-0029): a per-frame opt-in gate that
  dispatches the culling kernel over an indirect MultiMesh's transform buffer
  and writes each surface command block's instance-count dword (the same dword
  the CPU writes at `mesh_storage.cpp:2210`) — **zero latency, zero readback**
  on the render path; the count is produced and consumed on-GPU in the same
  frame. Candidate hook territory is the render-frame preparation before the
  draw lists execute; the concrete hook/binding design is deferred to the
  patch slices (0027 gate+callsite / 0028 runtime binding / 0029
  recording+real-pass opt-in).
- The gate returning false (always, by default) leaves the CPU-driven
  command-buffer contents and the non-indirect path untouched.

### 3.4 Resource flow

- Input: `multimesh->buffer` (Transforms SSBO, `float[]`;
  `mesh_storage.cpp:1598`; per-3D-instance 12-float row-major 3x4 lanes at
  `(motion_vectors_current_offset + instance) * stride_cache`).
- Output 1: `multimesh->command_buffer` (u32 SSBO,
  `5 * surface_count` dwords; `mesh_storage.cpp:1693`; this pass touches ONLY
  the instance-count dword of each 5-dword surface block).
- Output 2 (NEW, Rurix-allocated in a later slice): visibility bitmask SSBO,
  `u32[ceil(N/32)]` — **the shared interface handed to GRX-016
  (instance compaction) and GRX-018 (indirect args)**; see §5.1.

### 3.5 Resource-layer gap (patch-side bypass, later slice)

`scene/resources/multimesh.h` (the scene `MultiMesh` Resource) exposes **no
`use_indirect` property** (grep-confirmed zero hits), so no scene file or
GDScript property can turn the path on today — this is why the entire §3.1/3.2
infrastructure has zero callers (the bench generator carries
`TODO(GRX-015/016/018)` for the same reason). The server API does accept it:
`RS::multimesh_allocate_data(rid, instances, transform_format, use_colors,
use_custom_data, use_indirect=true)`
(`servers/rendering/rendering_server_default.h:429`, FUNC6). The later
patch/enablement slices reach the indirect path via this direct
RenderingServer call; plumbing a Resource-layer property is OUT of GRX-015
scope.

### 3.6 Honest benefit framing (contract-normative)

`RendererSceneCull::_scene_cull` (`renderer_scene_cull.cpp:2823`; frustum test
`IN_FRUSTUM` macro L2853, applied L2860) treats an entire MultiMesh as **one**
`InstanceData` — the CPU scene cull is per-`Instance`, not per-MultiMesh-
sub-instance. Therefore:

- **This pass does NOT touch the CPU cull O(N) cost** over the ~200k
  independent `MeshInstance3D` population of `many_mesh_instances`; that
  workload stays entirely on the native CPU cull.
- The benefit surface is the **GPU draw side only**: for indirect MultiMeshes,
  sub-instances outside the camera frustum stop generating vertex/pixel work
  (the native path draws all `visible_instances` regardless of the frustum).
- No FPS / p95 / draw-count improvement is claimed anywhere in this slice;
  whether the benefit is net-positive is a later measured question (GRX-006
  baseline / perf gate).

## 4. Input / output resources (Rurix mapping)

- Input: `src_transforms = StructuredBuffer<float>`, SRV `t0 space0`,
  `binding_kind = structured_buffer` (Godot `multimesh->buffer` native
  `ID3D12Resource*`; 12-float row-major 3x4 lanes per 3D instance, stride
  carried in b0).
- Output 1: `dst_commands = RWStructuredBuffer<uint>`, UAV `u0 space0`,
  `binding_kind = rwstructured_buffer` (Godot `multimesh->command_buffer`;
  5-dword command block per surface; the kernel atomically accumulates ONLY
  the instance-count dword `s * command_stride_dwords +
  instance_count_dword_index` and never touches the other dwords).
- Output 2: `dst_visibility = RWStructuredBuffer<uint>`, UAV `u1 space0`,
  `binding_kind = rwstructured_buffer` (Rurix-allocated visibility bitmask,
  `u32[ceil(N/32)]`, bit `i & 31` of word `i >> 5` = instance `i` visible; the
  GRX-016/018 input interface).
- b0 root constants: **144-byte / 36-dword Rurix-defined layout** (no Godot
  push constant exists to mirror — additive pass, §3.3): 6 frustum planes ×
  `(nx, ny, nz, d)` f32 (dwords 0-23), `instance_count`,
  `motion_vectors_current_offset`, `transform_stride_floats`, `surface_count`,
  `command_stride_dwords`, `instance_count_dword_index` u32 (dwords 24-29),
  `mesh_bound_center_local` xyz + `mesh_bound_radius_local` f32 (dwords
  30-33), `pad1`/`pad2` (dwords 34-35). Field-by-field in
  `resource_mapping.md` and `artifacts/gpu_culling_descriptor_layout.json`.
  Like GRX-013/014 — and unlike the GRX-009..012 texture passes — gpu_culling
  carries **no i64** field, so the `SHADER_INT64` capability is not part of
  its binding preflight.
- tracked mapping: `resource_mapping.md`.

## 5. Supported subset and route choice

### 5.1 In-scope subset (first slice) — count-only conservative sphere cull

- One thread per instance (`numthreads(64,1,1)`; dispatch `(ceil(N/64),1,1)`).
- Per-instance bound: a **conservative bounding sphere** derived from the mesh
  local AABB (host-side precompute, carried in b0: `center_local` = AABB
  center, `radius_local` = half the AABB size diagonal length) transformed by
  the instance basis: `world_center = rows · (center_local, 1)`;
  `world_radius = radius_local * frobenius_norm(basis)` — the Frobenius norm
  is a provable upper bound on the basis spectral norm, so the sphere test is
  **conservative in the safe direction** (may keep a truly-invisible instance
  visible; can never cull a visible one).
- 6-plane test, planes normalized with inward-facing normals:
  `dist_p = dot(n_p, world_center) + d_p`; instance culled iff **any** plane
  has `dist_p < -world_radius`; otherwise visible.
- Visible instance → (a) `InterlockedOr` its bit into the `dst_visibility`
  bitmask word; (b) accumulate **each** surface's instance-count dword of
  `dst_commands` (mirroring the CPU write loop over `surface_count` at
  `mesh_storage.cpp:2205-2212`). This is the **count-only** form: transforms are
  not remapped/compacted (GRX-016 territory).
- **Count-write semantics — picture-preservation refinement (rd_native kernel
  R1c; container-only, no 0046 patch/b0/RTS0 change).** Because transforms are NOT
  compacted, the native indirect draw
  renders instances `[0 .. InstanceCount-1]` **by index** from the un-compacted
  transform buffer. The written count is therefore a **prefix length**, and it
  must cover *every* visible instance for the cull to be picture-preserving.
  `InterlockedAdd(+1)` writes `InstanceCount = count-of-visible`, which is a
  correct prefix length **only when the visible set is exactly the prefix
  `[0 .. V-1]`**; for a *scattered* visible set it drops the visible instances at
  index ≥ V — a picture-breaking over-cull (convicted numerically + on real
  hardware, `rd_native_device_removal_diagnosis.md` §8). The rd_native kernel
  therefore uses the **high-water-mark** `InterlockedMax(count, instance + 1)`, so
  `InstanceCount = (highest visible index) + 1`: it never over-culls (the dropped
  tail `[count .. N-1]` is entirely invisible) and still reduces the draw when the
  instance-array *tail* is off-screen. The shim / canonical `frustum_count` kernel
  (backend==1, a superseded and device-removing side-channel path) still carries
  the `InterlockedAdd(+1)` count-of-visible and shares this latent prefix
  limitation; it is not corrected here because that path is not on the enablement
  track. The math-parity fixtures (§ math_parity) model the shim's count-of-visible
  and remain valid for the shim.
- Kernel assumes the instance-count dwords and the bitmask buffer are **zeroed
  before dispatch** (runtime responsibility of the later patch slices,
  mirroring the GRX-014 zeroed-destination convention); all other command
  dwords are preserved untouched.
- All parameters flow through b0; nothing is hardcoded (`command_stride_dwords
  = 5` and `instance_count_dword_index = 1` mirror
  `INDIRECT_MULTIMESH_COMMAND_STRIDE` and the `+sizeof(uint32_t)` offset of
  `mesh_storage.cpp:2210` but are carried as parameters).
- **Shared kernel interface fixed by this contract (GRX-016/018 dependency):**
  input = the 12-float row-major 3x4 transform lanes (§4) + the 36-dword b0;
  output = the visibility bitmask `u32[ceil(N/32)]` (bit `i` = instance `i`
  visible) + the per-surface instance-count dword semantics above. GRX-016
  (compaction) consumes the bitmask; GRX-018 (indirect args) extends the
  command-block writes. Changing this interface requires re-adjudicating all
  three contracts.

### 5.2 Out of scope (known gaps; `pass_manifest.json` `known_gaps` per line)

- Precise per-instance OBB / transformed-AABB test (the conservative sphere
  over-includes; exactness is a quality follow-up, not a correctness gap);
  occlusion culling; LOD selection; hierarchical/two-phase culling.
- 2D transform format (`MULTIMESH_TRANSFORM_2D`, 8-float stride); color /
  custom-data stride variants (`transform_stride_floats` is carried in b0 but
  exercised at 12 = 3D bare-transform in this slice's fixtures).
- `motion_vectors_current_offset != 0` (carried in b0, exercised at 0).
- Per-surface differing visibility (every surface command block receives the
  same count — matching the native CPU write, which also writes one value to
  all surfaces).
- Visible-instance compaction / transform remap (GRX-016) and indirect-args
  generation beyond the instance-count dword (GRX-018) — explicitly separate
  milestones, not merged into this pass (GRX_PLAN §6 "不合并 PR").
- The Resource-layer `use_indirect` plumbing (§3.5; patch-side
  `RS::multimesh_allocate_data(..., true)` bypass is a later patch slice).
- The `visible_instances >= 0` CPU clamp interplay (native `visible_instances`
  semantics stay CPU-owned; the kernel culls over `instance_count` as given).
- The CPU `_scene_cull` path is untouched (§3.6); the cull-stage runtime hook
  is a later patch slice.
- GPU-observed parity (pending the S6 standalone dispatch smoke).

### 5.3 Offline kernel route: HLSL bridge (chosen), not rurixc-native

gpu_culling is an **all raw-buffer / SSBO** pass, so the GRX-009
texture-intrinsic `llc` blocker does **not** apply. Even so, a rurixc-owned
`rx → DXIL` compile of the culling kernel is infeasible today, for four
different reasons:

1. **No u32 buffer views.** The DXIL compute-body lowering accepts only
   `View<global, f32>` / `ViewMut<global, f32>` (and f32 texture views)
   resource parameters (`src/rurixc/src/dxil_codegen.rs`,
   `require_view_global_f32` / `require_texture_or_view_global_f32`,
   ~L1750-1790). Both outputs (command dwords, visibility bitmask words) are
   u32 SSBOs; an f32 view cannot carry u32 bit patterns bit-faithfully.
2. **No atomic intrinsics on any backend.** The kernel's writes are
   `InterlockedAdd` (count accumulation across threads/groups) and
   `InterlockedOr` (bitmask bits; multiple threads target the same word). The
   Rurix lang subset has no atomic intrinsic on any backend (grep-confirmed:
   zero `Atomic`/`Interlocked` lowering in `src/rurixc/src/dxil_codegen.rs` /
   `mir.rs`). This is precisely the class of operation the bridge route
   exists for — bit operations and atomics are first-class in HLSL.
3. **Integer bit operations are not wired on the DXIL path.** The bitmask
   write needs `<<`, `&`, `>>`. MIR carries `BinOp::BitAnd/BitOr/BitXor/Shl/
   Shr` (`src/rurixc/src/mir.rs:643-647`), but the DXIL backend has no
   lowering for any of them (the GRX-014 cluster_store blocker, verbatim).
4. **sqrt on the DXIL path.** The Frobenius-norm radius needs `sqrt`. Rurix's
   `DeviceMathFn` lowers only on the NVPTX libdevice path
   (`src/rurixc/src/device_codegen.rs` / `tbir_build.rs`); the DXIL backend
   has no lowering (the GRX-013 particles_copy blocker, verbatim).

The canonical offline package is therefore the owner-approved
`hlsl_bridge_workaround`: a DXC `cs_6_0` DXIL container (validated by DXV)
with a **Rurix-owned RTS0** root signature (`rurixc::binding_layout::
{infer_root_signature, pack_root_constants, serialize_rts0}`). This mirrors
the GRX-014 cluster_store buffer-pass precedent (`../cluster_store/
PASS_CONTRACT.md` §5.3, itself mirroring `../particles_copy/PASS_CONTRACT.md`
§5.3 and `../luminance_reduction/texture_artifact_provenance_policy.json`),
with the raw-buffer blocker rationale above (the texture-intrinsic condition
does not apply here). `src/lib.rx` documents the kernel structure and the
four blockers; the executable math lives only in the HLSL bridge kernel.

### 5.4 Frustum-plane sign negation — HARD RED LINE

The call site converts the Godot camera frustum planes to the kernel's
inward-facing convention with an **exact, non-negotiable** sign rule; a sign
error silently flips the entire cull (keeping the off-screen half and dropping
the on-screen half):

- Godot's `Plane` obeys `distance_to(p) = dot(normal, p) - d` with the frustum
  **interior on the NEGATIVE side**.
- The gpu_culling kernel wants normalized **inward-facing** planes with
  `visible ⇔ dot(n, p) + d ≥ 0`.
- The conversion is therefore **`n_rurix = -plane.normal`, `d_rurix = plane.d`**
  (negate the normal, keep the distance) applied per plane at
  `render_forward_clustered.cpp` (both the shim path 0028 and the rd_native path
  0046 use it identically). This is the single most fragile line in the pass.

### 5.5 Route B rd_native variant (patch 0046)

The `passes/gpu_culling/backend == 2` selector runs the cull as an **in-frame RD
compute dispatch on the MAIN `RenderingDevice`** (bridge-independent; no rxgd
session, no cap bit), instead of the out-of-frame rxgd shim (0027-0029, which
keeps its own `enabled` gate byte-unchanged). Structural details:

- **b0 144B → 48B, off the CBV dead path.** The RD/D3D12 push-constant window is
  128 bytes (`rendering_device.cpp:6101`), so the shim's 144-byte b0 is rejected
  by `generate_rd_container.py` as `push_constant_too_large` and CANNOT drive an
  RD-native container; the CBV escape is also a dead path (the container
  generator's `parse_rts0` rejects root-descriptor CBVs and `binding_layout`
  always emits CBVs as root descriptors). The 6 frustum planes therefore move OUT
  of b0 into a **`StructuredBuffer<float4>` SRV at register t1** (SRV register
  aggregation with t0 — the taa_resolve 5-SRV precedent), shrinking b0 to the
  48-byte / 12-dword parameter+sphere block. The RD-native artifacts are
  co-located under `artifacts/gpu_culling_rd_native.{dxil,rts0.bin}` +
  `gpu_culling_rd_native_descriptor_layout.json` (HLSL
  `hlsl_bridge/gpu_culling_rd_native.hlsl`, RTS0 via
  `src/rurixc/examples/emit_grx015_gpu_culling_rd_native_rts0.rs`, container
  `rd-native-pipeline/out/gpu_culling_rd_native.rd_container.bin`, verify 59/59).
- **buffer_clear timing.** `RD::buffer_clear` is draw-graph tracked
  (`rendering_device.cpp` `draw_graph.add_buffer_clear`) but **hard-forbidden
  while a compute list is active**, so the module clears each surface's
  instance-count dword (`(s*command_stride_dwords + instance_count_dword_index)*4`)
  and the whole visibility bitmask, and `buffer_update`s the 96-byte t1 planes
  buffer, **all before `compute_list_begin()`**; then binds t0/t1/u0/u1 and
  dispatches `ceil(instance_count/64)`. Every other command dword (dword 0 =
  vertices-drawn count) is preserved.
- **Device-removal root cause (why rd_native is the correct structure).** The
  shim path's side-channel dispatch is INVISIBLE to the main draw graph, so after
  the count-dword clear the graph transitions the command buffer to
  `INDIRECT_ARGUMENT` while the side-channel UAV write is still in flight — a DXGI
  device-removal hazard. The rd_native path drives the main device's draw graph
  directly, so its `ResourceTracker` inserts the barriers and the clear → dispatch
  → `draw_list_draw_indirect` chain is a first-class, hazard-free citizen. The
  `ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py` gate's headline evidence
  is precisely the **no-device-removal judgement** of that chain, plus the
  picture-preservation invariant (a conservative cull only drops off-screen
  draws, so the candidate frame byte-matches the native reference).
- **Fail-closed / additive.** Default `backend == 0` (and empty container path)
  never engages; a failed record is a **no-op** (the CPU-written count survives),
  never a fallback dispatch. The CPU-driven arm remains the hard red-line fallback
  (§6).

## 6. Fallback

- fallback reason enum (aligned with the GRX-008 five): `compile_failed` /
  `validation_failed` / `unsupported_device` / `visual_diff_failed` /
  `manual_disabled`.
- Any compile / validation / visual / perf failure → the native CPU-driven
  path (`godot_native_cpu_driven_command_buffer`): CPU-written command-buffer
  contents (or the plain non-indirect `draw_list_draw` arm when the indirect
  path is not armed at all).
- **CPU fallback is a hard red line** (GRX_PLAN §6 GRX-015 row "必须保留 CPU
  fallback" / the G-GRX contract gate): no slice of this pass may remove or
  bypass the native CPU-driven arm.
- **Readback-validation red leg (design input recorded for S6/S8, not
  implemented here):** the verification legs must read back the GPU-produced
  count and compare `gpu_count == cpu_count` with **tolerance 0** (the CPU
  reference recomputes the same conservative-sphere classification); any
  mismatch is an immediate fallback + hard FAIL of the leg. The bitmask is
  compared word-exactly under the same zero tolerance.
- (Later slices) the default Godot config (per-pass settings all `false`) and
  the shipping bridge return `RXGD_STATUS_FALLBACK` for
  `RXGD_PASS_GPU_CULLING`; the shipping feature-off bridge fails closed with
  `real_dispatch_path_not_linked`.

## 7. Bridge gate — later slice (S4)

Not authored here. The later `GpuCullingGate` in `src/rurix-godot/src/lib.rs`
will mirror the GRX-014 `ClusterStoreGate` template with a
three-structured-buffer binding surface: runtime binding preflight (3 buffer
resources in src_transforms / dst_commands / dst_visibility order, the
144-byte / 36-dword b0, nonzero `instance_count` and `surface_count`, nonzero
buffer byte sizes; NO int64 cap check) → dispatch eligibility (opt-in
`RXGD_CAP_GPU_CULLING_REAL_PASS (1u << 9)` + the recording-harness capability
+ non-null native device/queue/buffer handles + `GpuCullingDispatchPackage`
layout/digest match vs the S2 offline evidence, three SHA-256 digests baked
in) → per-slot kernel-binding-kind conformance (`["structured_buffer",
"rwstructured_buffer", "rwstructured_buffer"]`; texture resources fail
closed) → math-parity gate
(`gpu_culling_cpu_reference_proven_pending_gpu_dispatch`) → real dispatch only
under the `d3d12-recording-shim` feature. Every failure prints the
once-per-session `RXGD_GPU_CULLING_REAL_PASS_BLOCKED
first_missing_prerequisite=...` diagnostic.

## 8. Godot patches — later serial slice (S5/S7)

Not authored here. `PATCH_ALLOCATION.md` §2 reserves **0027-0029** for
gpu_culling (0027 gate+callsite / 0028 runtime binding / 0029
recording+real-pass opt-in). Patches are generated by `git diff --no-index` on
a scratch copy with all prior patches applied, verified by
`ci/godot_rurix_patch_stack.py`; never hand-written; serialized by the §4
stack-lock. The 0027-0029 scope must include the `RS::multimesh_allocate_data
(..., use_indirect=true)` bypass (§3.5), the pre-dispatch zeroing of the count
dwords + bitmask (§5.1), and the §6 readback-validation red leg.

## 9. Evidence

- **offline compile** (this slice, measured): `offline_compile_evidence.json`
  — DXC `cs_6_0` compile, DXV validation, Rurix-owned RTS0
  (`emit_grx015_gpu_culling_rts0` via `rurixc::binding_layout::
  {infer_root_signature, pack_root_constants, serialize_rts0}`), descriptor
  layout (per-slot binding kinds + 144-byte / 36-dword root constants), three
  artifact SHA-256 recomputable on disk; `provenance =
  hlsl_bridge_workaround`, `rurix_owned = false`, `rurix_owned_rts0 = true`,
  `runtime_mappable = true`.
- **math parity** (this slice): `math_parity_evidence.json`
  (`generate_math_parity_evidence.py`) — deterministic synthetic transform /
  command / frustum fixtures; CPU reference computes the identical
  binary32-per-op float classification and the exact expected u32 outputs
  (visibility bitmask words + per-surface counts + untouched command dwords),
  compared at **zero tolerance**. Because the float intermediates feed only
  comparisons, the generator ASSERTS a per-comparison classification-margin
  floor (`|dist + world_radius| >= 1e-3` for every instance × plane) so
  ULP-level GPU reassociation/FMA/sqrt differences cannot flip any
  classification; the fixtures fail generation if the margin or any branch
  coverage degenerates. Covers ≥3 cases: all-visible / all-culled /
  boundary-mixed (crossing spheres), plus tail bitmask words, multi-group
  dispatch, multi-surface command blocks, per-plane cull coverage on all 6
  planes, rotated and non-uniform-scale bases. `status =
  pending_gpu_dispatch` until S6 fills the GPU-observed side.
- **standalone dispatch / visual / telemetry / enablement**: later slices
  (S6/S8); this file claims none of them.
- Perf: reuse GRX-006 baseline / perf gate; the GRX_PLAN draw/dispatch-count
  metric is later measured evidence; no performance improvement claimed.

## 10. Exit criteria (this slice)

- pass default `disabled`; manifest `implemented=false`,
  `runtime_state=fallback_only`, `real_gpu_pass=false`,
  `real_d3d12_dispatch_recorded=false` (fail-closed initial values).
- `offline_compile_evidence.json` `status=success` with DXV
  `validation.status=pass`; `math_parity_evidence.json`
  `status=pending_gpu_dispatch` with the margin floor and coverage assertions
  green.
- This slice does NOT represent pass completion.

## 11. Remaining items

- S4 bridge gate; S5 patch 0027; S6 standalone dispatch smoke (fills the
  GPU-observed parity side; enforces the §6 zero-tolerance readback); S7
  patches 0028/0029; S8 scratch rebuild + gated real-pass enablement smoke
  (incl. the forced-downgrade red leg); S9 close-out (gate module + probe
  registration + manifest flip + owner default-enable decision).
- Precise OBB test, occlusion, LOD, 2D format, color/custom strides,
  per-surface visibility, Resource-layer `use_indirect` plumbing.
- GRX-016 compaction / GRX-018 indirect-args consumers of the §5.1 interface
  (separate milestones, separate PRs).
- full baseline / per-pass FPS + draw/dispatch-count comparison; any
  performance claim.
