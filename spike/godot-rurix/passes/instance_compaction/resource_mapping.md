# GRX-016 instance_compaction Resource Mapping

## Scope

This file records the GRX-016 instance_compaction resource mapping. It maps
the real Godot MultiMesh resources and the GRX-015 gpu_culling visibility
output into the Rurix three-dispatch compaction chain while keeping the
runtime fallback-first. This is the S1-S3 offline face: NOT a real GPU
runtime pass, does not skip or alter any native Godot draw, and provides no
visual, telemetry, or performance evidence.

## Godot Native Flow (what this pass attaches to)

Unlike GRX-009..014 there is **no native Godot compute pass being replaced**:
Godot has no GPU instance compaction. The native contract this pass plugs
into is the MultiMesh "draw the first N instances" convention:

- `MeshStorage::_multimesh_allocate_data(...)`
  (`external/godot-master/servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp`
  `L1547-1602`): 3D transform-only `stride_cache` = **12 floats per instance**
  (`color_offset_cache = 12` for 3D, `L1577-1580`); the GPU buffer is
  `storage_buffer_create(instances * stride_cache * sizeof(float))`
  (`L1596-1599`).
- `MeshStorage::_multimesh_instance_set_transform(...)`
  (`mesh_storage.cpp:1878-1915`): the 12-float layout is 3 rows of
  `(basis.rows[i][0..2], origin[i])` (`L1900-1911`) — i.e. **3 float4 rows**
  per instance.
- `MeshStorage::_multimesh_set_visible_instances(...)`
  (`mesh_storage.cpp:2187-2216`): sets `visible_instances`; when the
  multimesh is `indirect`, also rewrites the instance-count u32 (second word
  of each `INDIRECT_MULTIMESH_COMMAND_STRIDE` block) in the command buffer
  (`L2206-2213`).
- `MeshStorage::multimesh_get_instances_to_draw(...)`
  (`mesh_storage.h:721-728`): `visible_instances >= 0 ? visible_instances :
  instances` — the renderer draws **the first N instances of the buffer**.
  Consumed at `render_forward_clustered.cpp:4297`
  (`_geometry_instance_update`, `INSTANCE_MULTIMESH` case) and
  `render_forward_clustered.cpp:4785`
  (`DEPENDENCY_CHANGED_MULTIMESH_VISIBLE_INSTANCES` handler).

The compaction strategy exploits exactly this: move the surviving instances
to the **front** of a staging buffer and pair it with the survivor count, so
the untouched native "draw first N" path renders only survivors — no draw
shader gather indirection, no Godot shader changes.

## Buffers

| Buffer | Producer | Element / stride | GRX-016 mapping (per variant) |
| --- | --- | --- | --- |
| `visibility_mask` | **GRX-015 gpu_culling** (declared dependency; see PASS_CONTRACT.md §5.3) | `u32[ceil(N/32)]`, bit `p` = word `p>>5`, bit `p&31`; tail bits beyond `N-1` are don't-care (both kernels bound-check `p < total_instances`) | scan_local SRV `t0`; scatter SRV `t0`; `binding_kind = structured_buffer` |
| `src_transforms` | Godot MultiMesh buffer (`multimesh->buffer`, `mesh_storage.cpp:1598`) | 12 floats = **3 float4 per instance** (3D transform-only) | scatter SRV `t1`, `StructuredBuffer<uint4>` (bit-preserving), `binding_kind = structured_buffer` |
| `local_prefix` | GRX-016 D1 scan_local | `u32[N]`: exclusive prefix of the visibility bit within its 256-thread group | scan_local UAV `u0`; scatter SRV `t2` |
| `group_totals` | GRX-016 D1 scan_local | `u32[num_groups]`: survivors per group | scan_local UAV `u1`; scan_groups SRV `t0` |
| `group_offsets` | GRX-016 D2 scan_groups | `u32[num_groups]`: exclusive prefix over `group_totals` (global rank of each group's first survivor) | scan_groups UAV `u0`; scatter SRV `t3` |
| `survivor_count` | GRX-016 D2 scan_groups | `u32[1]`: total survivors (the "draw first N" value) | scan_groups UAV `u1` |
| `dst_transforms` | GRX-016 D3 scatter | 3 float4 per instance, sized for the full `N` worst case; elements at rank >= survivor_count stay untouched (don't-care under first-N draw) | scatter UAV `u0`, `RWStructuredBuffer<uint4>` |

`local_prefix` / `group_totals` / `group_offsets` / `survivor_count` /
`dst_transforms` are **Rurix-owned intermediates** (allocated by the later
runtime slice, not Godot resources). Only `visibility_mask` (GRX-015) and
`src_transforms` (Godot MultiMesh buffer) cross a pass boundary.

## Per-variant binding surfaces

### D1 `scan_local` — dispatch `(ceil(N/256), 1, 1)`

| Slot | Resource | HLSL type | binding_kind |
| --- | --- | --- | --- |
| `t0 space0` | `visibility_mask` | `StructuredBuffer<uint>` | `structured_buffer` |
| `u0 space0` | `local_prefix` | `RWStructuredBuffer<uint>` | `rwstructured_buffer` |
| `u1 space0` | `group_totals` | `RWStructuredBuffer<uint>` | `rwstructured_buffer` |

### D2 `scan_groups` — dispatch `(1, 1, 1)`, single group

| Slot | Resource | HLSL type | binding_kind |
| --- | --- | --- | --- |
| `t0 space0` | `group_totals` | `StructuredBuffer<uint>` | `structured_buffer` |
| `u0 space0` | `group_offsets` | `RWStructuredBuffer<uint>` | `rwstructured_buffer` |
| `u1 space0` | `survivor_count` | `RWStructuredBuffer<uint>` | `rwstructured_buffer` |

### D3 `scatter` — dispatch `(ceil(N/256), 1, 1)`

| Slot | Resource | HLSL type | binding_kind |
| --- | --- | --- | --- |
| `t0 space0` | `visibility_mask` | `StructuredBuffer<uint>` | `structured_buffer` |
| `t1 space0` | `src_transforms` | `StructuredBuffer<uint4>` | `structured_buffer` |
| `t2 space0` | `local_prefix` | `StructuredBuffer<uint>` | `structured_buffer` |
| `t3 space0` | `group_offsets` | `StructuredBuffer<uint>` | `structured_buffer` |
| `u0 space0` | `dst_transforms` | `RWStructuredBuffer<uint4>` | `rwstructured_buffer` |

## Parameters — b0 root constants (Rurix-defined)

There is **no native push constant to mirror** (no native compaction pass
exists). The 32-byte / 8-dword `CompactionParams` block is Rurix-defined and
**shared byte-identical by all three variants** (`root_parameter_index 0`),
so the runtime binds one blob for the whole chain:

| dword | byte | field | type | used by |
| --- | --- | --- | --- | --- |
| 0 | 0 | `total_instances` | u32 | scan_local (bounds), scatter (bounds) |
| 1 | 4 | `bitmask_words` | u32 | carried for shape/validation (`== ceil(N/32)`); kernels index the mask by `p>>5` directly |
| 2 | 8 | `num_groups` | u32 | scan_groups (bounds; MUST be `<= 256`) |
| 3 | 12 | `transform_stride_vec4` | u32 | scatter (payload move; `== 3` in scope, enforced by the later S4 gate) |
| 4-7 | 16 | `pad0..pad3` | u32 | reserved, 0 |

uint32 fields are carried in the Rurix RTS0 as 1-dword slots
(`RootConstantType` has no dedicated `U32`; the RTS0 encodes only the dword
layout — the descriptor JSON records the true per-field type). No i64 fields:
like GRX-013/014, `SHADER_INT64` is **NOT** part of this pass's binding
preflight.

## Dispatch chain and barrier contract

The chain is correct only with the following ordering (D3D12 terms; the Godot
RD equivalent is a `compute_list_add_barrier` between the dispatches):

1. **Before D1**: `visibility_mask` visible to compute (UAV barrier after the
   GRX-015 culling dispatch that wrote it — the GRX-015→016 handoff, declared
   in PASS_CONTRACT.md §5.3); `src_transforms` in a compute-SRV-readable
   state.
2. **D1 scan_local** `(ceil(N/256), 1, 1)`.
3. **UAV barrier** on `local_prefix` AND `group_totals` (D2/D3 read them).
4. **D2 scan_groups** `(1, 1, 1)` — eligibility: `num_groups <= 256`
   (`N <= 65536`), fail-closed BEFORE any dispatch of the chain.
5. **UAV barrier** on `group_offsets` AND `survivor_count`.
6. **D3 scatter** `(ceil(N/256), 1, 1)`.
7. **After D3**: transition `dst_transforms` UAV → vertex/SRV read for the
   draw; `survivor_count` UAV → copy-source (CPU readback) or SRV (the later
   GRX-018 indirect-args consumer).

Within D1/D2 the groupshared scan uses `GroupMemoryBarrierWithGroupSync()`
(two per scan step); the scan is deterministic dataflow (each groupshared
slot written only by its own thread), so results are bit-exact.

The chain is all-or-nothing: if ANY prerequisite fails, NONE of the three
dispatches runs (never dispatch a partial chain).

## Descriptor Layout

- Root constants: `b0 space0`, 8 dwords (32 bytes) at `root_parameter_index
  0`, identical across variants (table above).
- Per-variant SRV/UAV surfaces as listed; canonical JSON:
  `artifacts/instance_compaction_descriptor_layout.json` (one document, three
  `variants` entries).
- Required resource counts for the (later) bridge gate: scan_local 3 /
  scan_groups 3 / scatter 5, in the exact slot orders above.
- Required push constant size for the (later) bridge gate: 32 bytes (all
  three variants).
- 64-bit integer capability: **NOT required** (no i64 b0 fields).

## Fallback Rules

- The pass remains disabled by default and runtime remains `fallback_only`.
- GRX-015 absent/disabled → no `visibility_mask` exists → fallback (the
  native path draws all instances / the CPU-set `visible_instances`).
- `num_groups > 256` (`total_instances > 65536`) → fallback (capacity
  contract; no partial chain).
- `transform_stride_vec4 != 3` (colors / custom_data / 2D / motion-vector
  layouts) → fallback.
- Missing any buffer, zero `total_instances`, or descriptor layout mismatch →
  fallback.
- (Later) missing `RXGD_CAP_INSTANCE_COMPACTION_REAL_PASS (1u << 10)` opt-in
  → fallback (`manual_disabled`); texture (non-structured-buffer) resources
  fail the per-slot kernel-binding-kind conformance check; the shipping
  feature-off bridge fails closed.
- The native Godot MultiMesh draw path (all instances, or CPU
  `visible_instances`) remains the active path whenever the bridge does not
  return OK. Alpha-blended materials and absolute-instance-index-keyed
  consumers are NEVER eligible (PASS_CONTRACT.md §5.2), independent of any
  opt-in.

## Explicit Non-Goals (this slice)

- No real runtime compaction GPU pass is enabled by default.
- No bridge gate, no Godot patch, no runtime native-handle resource binding
  (S4/S5/S7-level work, later slices; includes the survivor-count →
  `visible_instances` consumption design and the GRX-015 runtime handoff).
- No colors / custom_data channels (stride 16/20 floats), no 2D (stride 8),
  no motion-vector double-buffer layout, no indirect command-buffer path
  (GRX-018), no alpha-blended applicability, no `N > 65536`.
- No standalone GPU dispatch smoke (S6), no in-engine visual diff (S8), no
  measured fallback telemetry.
- No performance number or acceleration claim is made.
