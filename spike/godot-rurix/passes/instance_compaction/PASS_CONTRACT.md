# GRX-016 instance_compaction Pass — PASS CONTRACT

> **Status (2026-07-12, slice S1-S3: contract trio + offline kernel + math parity).**
> This slice delivers the OFFLINE face only, reusing the matured GRX-009..014
> per-pass template (`../PASS_TEMPLATE.md`): the pass contract trio, the
> three-variant HLSL-bridge kernel chain (DXC `cs_6_0` compile + DXV
> validation + Rurix-owned RTS0 via `rurixc::binding_layout` **per variant**,
> owner-approved `hlsl_bridge_workaround` provenance), and the integer-exact /
> byte-exact CPU math-parity reference (zero tolerance).
>
> This slice does **NOT** author a bridge gate, a Godot patch, runtime
> resource binding, a GPU dispatch smoke, an in-engine visual diff, or a
> real-pass enablement. Those are later slices (S4-S9). Measured ceiling here
> is: DXC compile + DXV validation pass (three variants) + CPU zero-tolerance
> parity reference.
>
> The pass ships **default disabled**; any compile / validation / visual /
> perf failure runs the native Godot MultiMesh draw path. Section 3's
> investigation only records paths and function names; Godot-side changes
> land only as `spike/godot-rurix/patches/` patch files, never by editing the
> `external/godot-master` snapshot.

## 1. Pass identity

- `pass_id = instance_compaction`
- bridge pass id: `RXGD_PASS_INSTANCE_COMPACTION = 11` — a **NEW id to be
  allocated** in the `RXGD_PASS_*` enum in `src/rurix-godot/src/lib.rs` by
  the later S4 slice (the enum currently ends at
  `RXGD_PASS_FUSED_POST_CHAIN = 10`; per `PATCH_ALLOCATION.md` §3 note the
  pass-id enum is a separate namespace allocated in `lib.rs`, not in the
  registry — this S1-S3 slice touches no `lib.rs` and only records the
  planned value)
- real-pass cap bit: `RXGD_CAP_INSTANCE_COMPACTION_REAL_PASS = 1u << 10`
  (reserved in `PATCH_ALLOCATION.md` §3, bit 10; set only by the later
  real-pass opt-in patch 0032)
- reserved patches: **0030-0032** (`PATCH_ALLOCATION.md` §2: 0030
  gate+callsite / 0031 runtime binding / 0032 recording+real-pass opt-in)
- Tier: Tier 2 (raw-buffer compute pass chain; GRX-016)
- target backend: `Godot 4.7-dev Windows D3D12 Forward+`
- default enable state: `disabled`
- **declared dependency: GRX-015 gpu_culling** (§5.3; `GRX_PLAN.md` GRX-016
  接续说明 — "与 GPU culling 依赖明确,不合并 PR")

## 2. Target scenes

- `many_mesh_instances`
- `mixed_forward_plus`

(instance_compaction is the second half of the GRX-015/016 pair: gpu_culling
produces a per-instance visibility bitmask for a MultiMesh; this pass moves
the surviving 3D transforms to the front of a staging buffer and emits the
survivor count, so Godot's untouched "draw the first N instances" contract
renders only survivors. The `many_mesh_instances` bench scene carries a 60k
MultiMesh component, within this slice's 65536-instance capacity bound.)

## 3. Godot-side hook / call-site / resource-flow investigation

Records paths and functions only; **`external/godot-master` is not modified.**

### 3.1 Storage class — the "draw first N" contract

- Source: `servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp`
- Header: `servers/rendering/renderer_rd/storage_rd/mesh_storage.h`
- Key functions / types:
  - `MeshStorage::_multimesh_allocate_data(...)` (`mesh_storage.cpp:1547-1602`)
    — 3D transform-only `stride_cache` = **12 floats per instance**
    (`color_offset_cache = 12` for 3D at L1577; `stride_cache` adds 4 per
    enabled color/custom channel, L1579-1580); the GPU buffer is
    `storage_buffer_create(instances * stride_cache * sizeof(float))`
    (L1596-1599).
  - `MeshStorage::_multimesh_instance_set_transform(...)`
    (`mesh_storage.cpp:1878-1915`) — the 12-float instance layout is 3 rows
    of `(basis.rows[i][0..2], origin[i])` (L1900-1911), i.e. 3 float4 rows.
  - `MeshStorage::_multimesh_set_visible_instances(...)`
    (`mesh_storage.cpp:2187-2216`) — the CPU-side visibility lever; for
    `indirect` multimeshes it also rewrites the instance-count u32 of each
    `INDIRECT_MULTIMESH_COMMAND_STRIDE` block (L2206-2213; the later GRX-018
    GPU-side count consumer).
  - `MeshStorage::multimesh_get_instances_to_draw(...)`
    (`mesh_storage.h:721-728`) — `visible_instances >= 0 ? visible_instances
    : instances`: the renderer draws **the first N instances of the buffer**.
    This is the assumption the whole compaction strategy rests on.
- Draw-count consumption:
  - `render_forward_clustered.cpp:4297`
    (`RenderForwardClustered::_geometry_instance_update`, `INSTANCE_MULTIMESH`
    case): `ginstance->instance_count = multimesh_get_instances_to_draw(...)`.
  - `render_forward_clustered.cpp:4783-4787`
    (`DEPENDENCY_CHANGED_MULTIMESH_VISIBLE_INSTANCES` dependency handler):
    re-reads the draw count when `visible_instances` changes.

### 3.2 Shader

**There is no native Godot compaction shader.** Unlike GRX-009..014, this
pass does not mirror an existing GLSL kernel — Godot's only per-MultiMesh
visibility mechanism is the CPU-set `visible_instances` above. The math
target is therefore the CPU stable-stream-compaction reference in
`generate_math_parity_evidence.py` (integer-exact, zero tolerance), and the
consumption contract is `multimesh_get_instances_to_draw`'s "first N"
semantics. The b0 push-constant block is Rurix-defined (no native struct to
mirror; `resource_mapping.md`).

### 3.3 Call / injection candidate point (later patch 0030 — design deferred)

The pass is ADDITIVE (no native dispatch is replaced), and its runtime hook
is **coupled to GRX-015's hook** (patches 0027-0029, which produce the
visibility bitmask at the cull stage). Candidate injection shape recorded for
the later serial patch slice:

- after the GRX-015 culling dispatch that writes the multimesh's visibility
  bitmask (cull stage, `renderer_scene_cull.cpp` — exact call-site owned by
  the GRX-015 contract), run the three-dispatch compaction chain on the same
  compute timeline;
- survivor-count consumption candidates: (a) CPU readback →
  `multimesh_set_visible_instances` (one-frame latency, to be measured and
  documented by the runtime slice), or (b) GPU-side count write via the
  `indirect` command-buffer path (`mesh_storage.cpp:2206-2213`) — the GRX-018
  indirect_args territory. The choice is a later-slice / owner decision; this
  slice wires nothing.
- the native draw path (all instances, or CPU `visible_instances`) always
  remains the fallback/continuation.

### 3.4 Resource flow

- Input 1: **GRX-015 gpu_culling visibility bitmask** — `u32[ceil(N/32)]`,
  bit `p` = word `p>>5`, bit `p&31`; a Rurix-pass output, not a Godot
  resource (declared interface, §5.3). Tail bits beyond `N-1` are don't-care
  (both consuming kernels bound-check `p < total_instances`).
- Input 2: `multimesh->buffer` (Godot MultiMesh SSBO, `mesh_storage.cpp:1598`)
  — 12 floats / 3 float4 per instance in the in-scope 3D transform-only
  layout.
- Intermediates (Rurix-owned, allocated by the later runtime slice):
  `local_prefix u32[N]`, `group_totals u32[num_groups]`,
  `group_offsets u32[num_groups]`, `survivor_count u32[1]`.
- Output: `dst_transforms` compacted staging buffer (3 float4 per instance,
  sized for the full `N` worst case; survivors packed at the front in stable
  index order; tail untouched = don't-care under first-N draw).
- Full tables: `resource_mapping.md`.

## 4. Input / output resources (Rurix mapping)

Three kernel variants, each with its own DXIL + Rurix-owned RTS0; one shared
Rurix-defined 32-byte / 8-dword b0 (`total_instances`, `bitmask_words`,
`num_groups`, `transform_stride_vec4`, 4 pads; `root_parameter_index 0`;
no i64 fields, so `SHADER_INT64` is NOT part of this pass's preflight):

- **D1 `scan_local`**: SRV `t0` `visibility_mask` (`structured_buffer`),
  UAV `u0` `local_prefix`, UAV `u1` `group_totals`
  (`rwstructured_buffer` ×2).
- **D2 `scan_groups`**: SRV `t0` `group_totals`, UAV `u0` `group_offsets`,
  UAV `u1` `survivor_count`.
- **D3 `scatter`**: SRV `t0` `visibility_mask`, SRV `t1` `src_transforms`
  (`StructuredBuffer<uint4>`, bit-preserving), SRV `t2` `local_prefix`,
  SRV `t3` `group_offsets`, UAV `u0` `dst_transforms`
  (`RWStructuredBuffer<uint4>`).
- tracked mapping: `resource_mapping.md`; canonical descriptor JSON:
  `artifacts/instance_compaction_descriptor_layout.json` (one document,
  three `variants` entries).

## 5. Supported subset, ordering contract, dependency, route choice

### 5.1 In-scope subset (this slice) — the three-dispatch chain

- **Stable stream compaction** of a 3D transform-only MultiMesh buffer
  (stride 12 floats = 3 float4 per instance), driven by the GRX-015
  visibility bitmask; one thread per instance; `GROUP_SIZE = 256`.
- Two-level exclusive prefix sum + scatter, one kernel per dispatch:
  1. **D1 `scan_local`** `(ceil(N/256), 1, 1)`: thread `p` reads visibility
     bit `p`; groupshared Hillis-Steele inclusive scan (8 fixed steps, two
     `GroupMemoryBarrierWithGroupSync` per step) over the group's 256 bits;
     writes `local_prefix[p]` (exclusive = inclusive − own bit) and
     `group_totals[gid]` (last lane's inclusive total).
  2. **D2 `scan_groups`** `(1, 1, 1)`: a single group scans `group_totals`
     into `group_offsets` (exclusive) and `survivor_count[0]` (grand total).
  3. **D3 `scatter`** `(ceil(N/256), 1, 1)`: surviving instance `p` moves its
     3-×-uint4 payload (bit-preserving, no arithmetic) to
     `rank = group_offsets[p/256] + local_prefix[p]`; non-survivors write
     nothing.
- **Barrier contract** (normative; `resource_mapping.md` for the full table):
  UAV barrier on `local_prefix`+`group_totals` between D1→D2; UAV barrier on
  `group_offsets`+`survivor_count` between D2→D3; `dst_transforms` and
  `survivor_count` transitioned for consumption after D3. The chain is
  all-or-nothing: if any prerequisite fails, none of the three dispatches
  runs.
- **Capacity contract**: the single-group second level requires
  `num_groups <= 256`, i.e. `total_instances <= 65536`. Larger N needs a
  third scan level (out of scope); the later S4 gate rejects it fail-closed
  BEFORE any dispatch.
- All rank math is u32 addition on exact values; the payload move is
  bit-preserving `uint4` copies → the CPU reference is integer-exact /
  byte-exact with **zero tolerance** (no float arithmetic anywhere in the
  chain).

### 5.2 Ordering-correctness contract (normative)

- Compaction is **STABLE**: `rank(p)` is an exclusive prefix sum by index, so
  survivors keep their relative index order. But **absolute instance indices
  change** (surviving instance `p` lands in slot `rank(p) <= p`), and
  non-survivors disappear from the drawn range entirely.
- Therefore the pass is **opaque-only**:
  - **Alpha-blended / transparent materials are OUT OF SCOPE** — order/depth
    -sensitive blending must not have its instance set re-indexed by an
    opt-in compaction pass; the gate must never engage for them.
  - Any consumer keyed on the **absolute instance index** (e.g.
    `INSTANCE_ID`/`INSTANCE_CUSTOM`-by-index shader effects, per-instance
    colors/custom_data addressed by slot) is OUT OF SCOPE — the first
    subset carries transform-only strides precisely so no such channel
    exists in the moved payload.
- **Disabled-path correctness (dependency boundary)**: when GRX-016 is
  disabled (the default), **GRX-015 must degrade to count-only** —
  visibility statistics/telemetry with NO buffer mutation and NO
  `visible_instances` change — which is trivially correct: the native path
  draws exactly what it drew before. GRX-016 is the only component that may
  reorder buffer contents, and it does so only into its own staging buffer
  (the source MultiMesh buffer is never mutated).

### 5.3 GRX-015 dependency (declared interface)

- **Input interface** (consumed, not defined here): the GRX-015 gpu_culling
  per-instance visibility bitmask, `u32[ceil(N/32)]`, bit `p` = word `p>>5`
  bit `p&31`, 1 = survives; produced on the compute timeline before this
  chain (handoff barrier in `resource_mapping.md`). Tail bits beyond `N-1`
  are don't-care (this pass bound-checks; GRX-015 need not zero-pad).
- The GRX-015 pass package (its own contract trio, patches **0027-0029**, cap
  bit `1u << 9`) is a SEPARATE milestone that had not landed its pass
  directory when this slice was authored; this contract pins the interface
  above as the compatibility surface, and the later S4 gate must re-verify it
  against the landed GRX-015 contract before any dispatch (mismatch =
  fail-closed fallback).
- Enable order: GRX-016's real-pass arm is eligible only when GRX-015's real
  pass is measured and enabled for the same multimesh; GRX-015 without
  GRX-016 stays correct (count-only degradation, §5.2); GRX-016 without
  GRX-015 has no input and falls back. Per `GRX_PLAN.md`, the two milestones
  are never merged into one PR.

### 5.4 Out of scope (known gaps; `pass_manifest.json` `known_gaps` per line)

- colors / custom_data channels (stride 16/20 floats) and the 2D layout
  (stride 8) — transform-only 3D (stride 12) is the first subset;
- motion-vector double-buffer MultiMesh layout (current/previous halves would
  compact to different ranks → temporal artifacts; requires a dedicated
  design);
- alpha-blended / transparent materials; absolute-instance-index-keyed
  consumers (§5.2);
- `total_instances > 65536` (needs a third scan level);
- the survivor-count consumption wiring (CPU readback vs GRX-018 indirect
  args) and the GRX-015 runtime handoff — later slices (§3.3);
- the runtime hook / native-handle binding (patches 0030-0032, later serial
  slices);
- GPU-observed math parity (pending the S6 standalone dispatch smoke).

### 5.5 Offline kernel route: HLSL bridge (chosen), not rurixc-native

instance_compaction is an **all raw-buffer / SSBO** pass, so the GRX-009
texture-intrinsic `llc` blocker does **not** apply. Even so, a rurixc-owned
`rx → DXIL` compile of the chain is infeasible today, for three reasons:

1. **No u32 buffer views on the DXIL path.** The DXIL compute-body lowering
   accepts only `View<global, f32>` / `ViewMut<global, f32>` (and f32
   textures) resource parameters (`src/rurixc/src/dxil_codegen.rs:1754/1786`).
   The bitmask / prefix / totals / offsets / count buffers are u32 words
   whose bit patterns an f32 view cannot carry bit-faithfully.
2. **Integer bit operations are not wired on the DXIL path.** Bitmask decode
   needs `>>` and `&`; MIR carries `BinOp::BitAnd/Shl/Shr`
   (`src/rurixc/src/mir.rs:643-647`) but the DXIL backend has no lowering for
   any of them.
3. **No groupshared scan on the DXIL path.** The lang subset HAS `shared let`
   (addrspace(3), M5.3/RXS-0079) and `barrier()`
   (`DeviceIntrinsic::Barrier`) — but both lower ONLY on the NVPTX device
   path; the DXIL compute-body lowering explicitly rejects `shared let`
   (`src/rurixc/src/dxil_codegen.rs:921-924`) and has no barrier lowering, so
   the per-group prefix scan cannot be expressed.

The canonical offline package is therefore the owner-approved
`hlsl_bridge_workaround`: three DXC `cs_6_0` DXIL containers (each validated
by DXV) with **Rurix-owned RTS0** root signatures per variant
(`rurixc::binding_layout::{infer_root_signature, pack_root_constants,
serialize_rts0}`). This mirrors the GRX-013/014 buffer-pass precedent
(`../cluster_store/PASS_CONTRACT.md` §5.3, itself mirroring
`../luminance_reduction/texture_artifact_provenance_policy.json`), with the
raw-buffer blocker rationale above. `src/lib.rx` documents the chain and the
three blockers and sketches the only expressible lane (the f32 payload move
with a precomputed rank); the executable math lives only in the HLSL bridge
kernels.

## 6. Fallback

- fallback reason enum (aligned with the GRX-008 five): `compile_failed` /
  `validation_failed` / `unsupported_device` / `visual_diff_failed` /
  `manual_disabled`.
- Any compile / validation / visual / perf failure → the native Godot
  MultiMesh draw path (`godot_native_multimesh_draw`: all instances, or the
  CPU-set `visible_instances`).
- GRX-015 missing/disabled, `num_groups > 256`, `transform_stride_vec4 != 3`,
  alpha-blended material, or any missing buffer → fallback (never a partial
  chain).
- (Later slices) the default Godot config (per-pass settings all `false`) and
  the shipping bridge return `RXGD_STATUS_FALLBACK` for
  `RXGD_PASS_INSTANCE_COMPACTION`.

## 7. Bridge gate — later slice (S4)

Not authored here. The later `InstanceCompactionGate` in
`src/rurix-godot/src/lib.rs` will mirror the GRX-014 `ClusterStoreGate`
template, extended to the three-variant chain: runtime binding preflight
(per-variant resource counts/orders, the shared 32-byte b0, nonzero
`total_instances`, `bitmask_words == ceil(N/32)`, `num_groups == ceil(N/256)
<= 256`, `transform_stride_vec4 == 3`, buffer byte sizes consistent with N;
NO int64 cap check) → dispatch eligibility (opt-in
`RXGD_CAP_INSTANCE_COMPACTION_REAL_PASS (1u << 10)` + non-null native
device/queue + non-null handles + per-variant `InstanceCompactionDispatch
Package` layout/digest match vs the S2 offline evidence — all SEVEN canonical
artifact digests baked in) → per-slot kernel-binding-kind conformance
(`structured_buffer`/`rwstructured_buffer` per the §4 tables; texture
resources fail closed) → math-parity gate
(`instance_compaction_cpu_reference_proven_pending_gpu_dispatch`) → real
three-dispatch chain with the §5.1 barrier contract (only under a
recording-shim feature; shipping feature-off fails closed with
`real_dispatch_path_not_linked`; all-or-nothing — no partial chain).

## 8. Godot patches — later slice (S5/S7)

Not authored here. `PATCH_ALLOCATION.md` §2 reserves **0030-0032** for
instance_compaction (0030 gate+callsite / 0031 runtime binding / 0032
recording+real-pass opt-in). Patches are generated by `git diff --no-index`
on a scratch copy with all prior patches applied, verified by
`ci/godot_rurix_patch_stack.py`; never hand-written; serialized by the §4
stack-lock. The 0030 call-site design is additionally blocked on the GRX-015
patches 0027-0029 landing first (§5.3).

## 9. Evidence

- **offline compile** (this slice, measured): `offline_compile_evidence.json`
  — per-variant DXC `cs_6_0` compile + DXV validation + Rurix-owned RTS0
  (`emit_grx016_instance_compaction_rts0 <variant>` via
  `rurixc::binding_layout::{infer_root_signature, pack_root_constants,
  serialize_rts0}`), one shared descriptor layout JSON (per-slot binding
  kinds + the 32-byte / 8-dword b0); SEVEN canonical artifacts (3 DXIL +
  3 RTS0 + 1 descriptor JSON) with SHA-256 recomputable on disk;
  `provenance = hlsl_bridge_workaround`, `rurix_owned = false`,
  `rurix_owned_rts0 = true`, `runtime_mappable = true`.
- **math parity** (this slice): `math_parity_evidence.json`
  (`generate_math_parity_evidence.py`) — deterministic synthetic bitmask +
  transform fixtures with an **integer-exact / byte-exact** CPU reference
  (zero tolerance): per-case `local_prefix` / `group_totals` /
  `group_offsets` element-wise-exact u32 arrays, `survivor_count`, and the
  full compacted `dst` buffer compared byte-for-byte (zero-initialized dst;
  untouched tail must stay zero). Case coverage REQUIRED (the generator fails
  if degenerate): sparse survival spanning multiple groups, all-survive,
  zero-survive, garbage tail bits in the last mask word, and an
  empty-leading-group case; `status = pending_gpu_dispatch` until the S6
  smoke fills the GPU-observed side.
- **standalone dispatch / visual / telemetry / enablement**: later slices
  (S6/S8); this file claims none of them.
- Perf: reuse GRX-006 baseline / perf gate; no performance improvement
  claimed. Correctness verification per `GRX_PLAN.md` GRX-016 =
  "visibility correctness" first.

## 10. Exit criteria (this slice)

- pass default `disabled`; manifest `implemented=false`,
  `runtime_state=fallback_only`, `real_gpu_pass=false`,
  `real_d3d12_dispatch_recorded=false` (fail-closed initial values).
- `offline_compile_evidence.json` `status=success` with DXV
  `validation.status=pass` for ALL THREE variants;
  `math_parity_evidence.json` `status=pending_gpu_dispatch` with the §9
  coverage counters all nonzero.
- This slice does NOT represent pass completion.

## 11. Remaining items

- S4 bridge gate (`InstanceCompactionGate`, three dispatch packages +
  `RXGD_PASS_INSTANCE_COMPACTION = 11` id allocation); S5 patch 0030 (blocked
  on GRX-015 patches 0027-0029); S6 standalone three-dispatch smoke (chain
  execution + zero-tolerance readback vs the S3 reference); S7 patches
  0031/0032; S8 scratch rebuild + gated real-pass enablement; S9 close-out.
- survivor-count consumption design (CPU readback `visible_instances` vs
  GRX-018 indirect args) — later slice / owner decision.
- colors/custom_data/2D/motion-vector layouts; `N > 65536` (third scan
  level); alpha applicability is permanently out of scope absent a separate
  ordering design.
- full baseline / per-pass FPS comparison; any performance claim.
