# GRX-015 gpu_culling Resource Mapping

## Scope

This file records the GRX-015 gpu_culling resource mapping. It maps the real
Godot indirect-MultiMesh resources into the Rurix bridge design while keeping
the runtime fallback-first. This is the S1-S3 offline face: NOT a real GPU
runtime pass, does not skip or alter the native CPU-driven command-buffer
path, and provides no visual, telemetry, or performance evidence.

**Structural note:** unlike GRX-009..014, gpu_culling is an ADDITIVE pass —
there is no native Godot compute shader being replaced and therefore no Godot
push-constant struct to mirror. The b0 layout below is Rurix-defined. The
"native path" is the CPU-driven command buffer (`_multimesh_set_visible_
instances`) plus the non-indirect `draw_list_draw` arm.

## Godot Native Flow

- Indirect infrastructure (all in
  `external/godot-master/servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp`):
  `_multimesh_allocate_data` (`L1547`, carries `bool p_use_indirect`) →
  `_multimesh_set_mesh` (`L1666`; command-buffer creation `L1676-1696`,
  `INDIRECT_MULTIMESH_COMMAND_STRIDE = 5` per `mesh_storage.h:63`) →
  `_multimesh_set_visible_instances` (`L2187`; **CPU count write point
  `L2210`**: second dword of every surface command block).
- Draw side: `render_forward_clustered.cpp:602-613` —
  `draw_list_draw_indirect(..., command_buffer_rid, surface_index * 5 * 4, 1,
  0)` at `L610`, flag from `multimesh_uses_indirect` (`L4355-4357`). D3D12
  executes via `ExecuteIndirect`
  (`drivers/d3d12/rendering_device_driver_d3d12.cpp:4909/4918`).
- Zero tracked callers set `use_indirect` today (`scene/resources/multimesh.h`
  has no such property); the server API `RS::multimesh_allocate_data(...,
  use_indirect)` (`rendering_server_default.h:429`) is the later patch-side
  bypass.
- Pass shape (Rurix, later slices): one compute dispatch per culled indirect
  MultiMesh, one thread per instance, `numthreads(64,1,1)`, dispatch
  `(ceil(N/64), 1, 1)`, before the frame's draw lists consume the command
  buffer — zero readback on the render path.

## Godot Resources

| Godot resource | Role | Native shape | GRX-015 Rurix mapping |
| --- | --- | --- | --- |
| `multimesh->buffer` (Transforms SSBO; `mesh_storage.cpp:1598`) | instance transforms (input) | `float[]`, `stride_cache` floats per instance (12 for bare 3D), lanes at `(motion_vectors_current_offset + i) * stride_cache` | `src_transforms` SRV `t0 space0`, `binding_kind = structured_buffer` |
| `multimesh->command_buffer` (`mesh_storage.cpp:1693`) | per-surface indirect draw commands (output) | `uint32_t[5 * surface_count]`, `STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT` | `dst_commands` UAV `u0 space0`, `binding_kind = rwstructured_buffer` (ONLY the instance-count dword is written) |
| (none — Rurix-allocated, later slice) | visibility bitmask (output; GRX-016/018 interface) | `u32[ceil(N/32)]`, zeroed before dispatch | `dst_visibility` UAV `u1 space0`, `binding_kind = rwstructured_buffer` |

### Transform SSBO element layout (12 f32 lanes per 3D instance)

Packed by `_multimesh_instance_set_transform` (`mesh_storage.cpp:1880-1915`),
row-major 3x4:

| lane | contents |
| --- | --- |
| 0-2 | `basis.rows[0]` (r0.x, r0.y, r0.z) |
| 3 | `origin.x` |
| 4-6 | `basis.rows[1]` |
| 7 | `origin.y` |
| 8-10 | `basis.rows[2]` |
| 11 | `origin.z` |

`stride_cache` grows to 16 with colors and 20 with colors+custom data
(`mesh_storage.cpp:1573-1577`); the kernel carries the stride in b0
(`transform_stride_floats`) but this slice's fixtures exercise only the bare
12-float 3D form (known gap).

### Command buffer layout (5 u32 per surface; `INDIRECT_MULTIMESH_COMMAND_STRIDE`)

Command block for surface `s` starts at dword `s * 5`:

| dword | contents | writer |
| --- | --- | --- |
| 0 | index/vertex count per instance (`mesh_surface_get_vertices_drawn_count`) | CPU, `_multimesh_set_mesh` `L1685-1690` |
| 1 | **instance count** | CPU: `_multimesh_set_visible_instances` `L2210` (byte offset `(s*5+1)*4`). **GRX-015 write target**: `InterlockedAdd` accumulation, assumes pre-zeroed |
| 2 | start index / start vertex | zero-initialized (`L1683`); untouched by this pass |
| 3 | base vertex / start instance | zero-initialized; untouched |
| 4 | start instance / unused | zero-initialized; untouched |

(Dwords 2-4 map to the D3D12 draw / draw-indexed indirect argument tails; the
draw side points `ExecuteIndirect` at offset `s * 5 * 4` with stride-1 count
1, so dword 0 and dword 1 are the live fields for both indexed and non-indexed
paths.)

### Visibility bitmask layout (GRX-016/018 shared interface)

`dst_visibility[i >> 5]` bit `(i & 31)` set ⇔ instance `i` visible. Buffer is
`ceil(instance_count / 32)` words, zeroed before dispatch; tail bits past
`instance_count - 1` in the last word are never set. Writes use
`InterlockedOr` (multiple threads share a word).

## Parameters — b0 root constants (Rurix-defined, 144 bytes / 36 dwords)

No Godot push constant exists for this pass; the layout is Rurix-defined and
fixed by this contract (shared with GRX-016/018 consumers). Planes are
normalized, inward-facing: `dist = dot(n, p) + d >= 0` ⇔ point on the visible
side; instance culled iff any plane has `dist < -world_radius`.

| dword | byte | field | type | used by in-scope kernel |
| --- | --- | --- | --- | --- |
| 0-3 | 0 | `frustum_plane_0` (nx, ny, nz, d) | f32×4 | yes |
| 4-7 | 16 | `frustum_plane_1` | f32×4 | yes |
| 8-11 | 32 | `frustum_plane_2` | f32×4 | yes |
| 12-15 | 48 | `frustum_plane_3` | f32×4 | yes |
| 16-19 | 64 | `frustum_plane_4` | f32×4 | yes |
| 20-23 | 80 | `frustum_plane_5` | f32×4 | yes |
| 24 | 96 | `instance_count` | u32 | yes (bounds check) |
| 25 | 100 | `motion_vectors_current_offset` | u32 | yes (lane base; exercised at 0) |
| 26 | 104 | `transform_stride_floats` | u32 | yes (== 12 in scope) |
| 27 | 108 | `surface_count` | u32 | yes (count-dword replication loop) |
| 28 | 112 | `command_stride_dwords` | u32 | yes (== 5, mirrors `INDIRECT_MULTIMESH_COMMAND_STRIDE`) |
| 29 | 116 | `instance_count_dword_index` | u32 | yes (== 1, mirrors the `+sizeof(uint32_t)` of `mesh_storage.cpp:2210`) |
| 30-32 | 120 | `mesh_bound_center_local` (x, y, z) | f32×3 | yes (local AABB center) |
| 33 | 132 | `mesh_bound_radius_local` | f32 | yes (half local AABB diagonal) |
| 34 | 136 | `pad1` | u32 | no |
| 35 | 140 | `pad2` | u32 | no |

All scalar fields pack tightly in HLSL cbuffer rules (the `float4[6]` array
occupies exactly dwords 0-23; the following 12 scalars never straddle a
16-byte vector boundary), so the cbuffer dword offsets equal this table
exactly. uint32 fields are carried in the Rurix RTS0 as 1-dword slots
(`RootConstantType` has no dedicated `U32`; the RTS0 encodes only the dword
layout — the descriptor JSON records the true per-field type).

`mesh_bound_center_local` / `mesh_bound_radius_local` are a host-side
precompute from the mesh local AABB (`center = aabb.position + 0.5 *
aabb.size`, `radius = 0.5 * length(aabb.size)`); the conservative world
radius is `radius_local * frobenius_norm(basis)` (upper bound on the spectral
norm — never over-culls).

## Descriptor Layout

- Root constants: `b0 space0`, 36 dwords (144 bytes) at
  `root_parameter_index 0` (the Rurix-defined layout above).
- SRV: `src_transforms = t0 space0` (`binding_kind = structured_buffer`).
- UAV: `dst_commands = u0 space0` (`binding_kind = rwstructured_buffer`).
- UAV: `dst_visibility = u1 space0` (`binding_kind = rwstructured_buffer`).
- Required resource count for the (later) bridge gate: 3 resources, in
  src_transforms / dst_commands / dst_visibility order.
- Required push constant size for the (later) bridge gate: 144 bytes.
- 64-bit integer capability: **NOT required** — gpu_culling carries no i64
  fields (same as GRX-013/014; contrast with the GRX-009..012 texture passes).

## Fallback Rules

- The pass remains disabled by default and runtime remains `fallback_only`.
- Missing source or destination resource returns fallback.
- Descriptor layout mismatch returns fallback.
- (Later) missing `RXGD_CAP_GPU_CULLING_REAL_PASS (1u << 9)` opt-in returns
  fallback (`manual_disabled`); texture (non-structured-buffer) resources fail
  the per-slot kernel-binding-kind conformance check; the shipping feature-off
  bridge fails closed.
- (Later, contract-normative) readback validation: `gpu_count == cpu_count`
  tolerance 0 and word-exact bitmask; any mismatch → immediate fallback.
- The native CPU-driven command-buffer path (and the non-indirect draw arm)
  remains the active path whenever the bridge does not return OK — CPU
  fallback retention is a GRX_PLAN hard requirement for this pass.

## Explicit Non-Goals (this slice)

- No real runtime culling GPU pass is enabled by default.
- No bridge gate, no Godot patch, no runtime native-handle resource binding,
  no pre-dispatch zeroing wiring (S4/S5/S7-level work, later slices).
- No precise OBB test, no occlusion, no LOD, no 2D format, no color/custom
  strides, no per-surface visibility, no compaction (GRX-016), no indirect
  args beyond the count dword (GRX-018), no Resource-layer `use_indirect`
  plumbing.
- No standalone GPU dispatch smoke (S6), no in-engine visual diff (S8), no
  measured fallback telemetry.
- No FPS / p95 / draw-count / dispatch-count improvement claim is made.
