# GRX-018 indirect_args Resource Mapping

## Scope

This file records the GRX-018 indirect_args resource mapping. It maps the real
Godot indirect-MultiMesh command buffer (and the declared GRX-015/016 survivor
count producer interface) into the Rurix bridge design while keeping the
runtime fallback-first. This slice covers the offline face only (S1-S3): NOT a
real GPU runtime pass, does not skip the native Godot CPU command-buffer path,
and provides no visual, telemetry, or performance evidence.

## Godot Native Flow

- Command block constant: `INDIRECT_MULTIMESH_COMMAND_STRIDE = 5` u32 dwords
  per surface (`external/godot-master/servers/rendering/renderer_rd/
  storage_rd/mesh_storage.h:62-64`).
- Buffer creation + static fill: `MeshStorage::_multimesh_set_mesh`
  (`mesh_storage.cpp:1666`, indirect branch `L1674-1696`) — zero-initialized
  `5 * surface_count` dwords; dword 0 of each block = `mesh_surface_get_
  vertices_drawn_count(surface)` (`mesh_storage.h:460-463`: `index_count ?
  index_count : vertex_count`); created with
  `RD::STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT` (`L1693`).
- Per-update CPU write (the GRX-018 elimination target):
  `MeshStorage::_multimesh_set_visible_instances` (`mesh_storage.cpp:2187`,
  indirect branch `L2206-2213`) — `RD::buffer_update` of dword 1
  (`instance_count`) of every surface block at `L2210`.
- Consumer: `render_forward_clustered.cpp:610` —
  `draw_list_draw_indirect(draw_list, index_array_rd.is_valid(),
  _multimesh_get_command_buffer_rd_rid(base), surface_index * 20, 1, 0)`,
  gated by `INSTANCE_DATA_FLAG_MULTIMESH_INDIRECT` (`L602`; flag set at
  `L4356`).
- GDScript-facing switches: `RenderingServer::multimesh_allocate_data(...,
  use_indirect)` (`rendering_server.cpp:2471`) and
  `multimesh_get_command_buffer_rd_rid` (`rendering_server.cpp:2489`).

## Godot / producer resources

| Resource | Role | Native shape | GRX-018 Rurix mapping |
| --- | --- | --- | --- |
| GRX-015/016 survivor-count buffer (producer interface, PASS_CONTRACT.md §4.1; not yet landed) | surviving instance count for this multimesh (input) | device-local `uint[]` SSBO; count at word `survivor_count_word_offset` | `src_survivor_counts` SRV `t0 space0`, `binding_kind = structured_buffer` (stride 4) |
| `multimesh->command_buffer` (`mesh_storage.h:258`; created `mesh_storage.cpp:1693`) | indirect draw command blocks, 5 dwords per surface (output) | `uint[]` storage buffer with `STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT` | `dst_command_buffer` UAV `u0 space0`, `binding_kind = rwstructured_buffer` (stride 4) |
| Rurix-owned validation scratch (not a Godot resource) | resident validation red-leg output (PASS_CONTRACT.md §5.4) | `uint[]`, `2 + surface_count` words, zeroed before the validate dispatch | `dst_validation` UAV `u1 space0`, `binding_kind = rwstructured_buffer` (stride 4) |

Binding-order note: the Rurix descriptor order is `[t0 src_survivor_counts,
u0 dst_command_buffer, u1 dst_validation]` (input first, outputs last — the
GRX-013/014 src-then-dst convention). Both kernels (write + validate) share
this single binding surface and one Rurix-owned RTS0; the write kernel never
references `u1` (a root-signature superset is legal for both PSOs).

Runtime staging note (binds the later S4/S7 design): the real-pass arm binds a
Rurix-owned STAGING buffer as `u0`, validates, reads back `dst_validation`,
and only when clean copies the staging blocks over the live
`multimesh->command_buffer` (`buffer_copy`), so bad args never reach
`draw_list_draw_indirect` (PASS_CONTRACT.md §5.4).

### dst_command_buffer output layout (per surface block `s`, base = `s * 5`)

| dword | field (indexed draw) | non-indexed interpretation | writer |
| --- | --- | --- | --- |
| 0 | `index_count` | `vertex_count` | b0 template backfill (`surface{s}_index_count`) |
| 1 | `instance_count` | `instance_count` | GPU-dynamic: `min(survivors, max_instance_count)` |
| 2 | `first_index` | `first_vertex` | b0 template backfill (natively 0) |
| 3 | `vertex_offset` | `first_instance` | b0 template backfill (natively 0) |
| 4 | `first_instance` | (unused) | b0 template backfill (natively 0) |

Native value equivalence: the CPU path writes dword 0 = vertices-drawn count,
dword 1 = visible instances, dwords 2-4 = 0; the runtime b0 filler must pass
exactly those statics, so the same block content is value-correct for indexed
and non-indexed surfaces alike.

### dst_validation output layout (`2 + surface_count` words, zeroed first)

| word | contents |
| --- | --- |
| 0 | `mismatch_count`: surfaces whose bitmask has any of bits 0-5 set (`InterlockedAdd`) |
| 1 | `clamp_trigger_count`: surfaces whose bitmask has bit 6 set (`InterlockedAdd`) |
| 2 + s | per-surface bitmask: bits 0-4 = generated dword c != expected dword c; bit 5 = in-buffer `instance_count > max_instance_count`; bit 6 = `survivors > max_instance_count` (producer violation; clamp fired) |

Runtime policy: `mismatch_count != 0 OR clamp_trigger_count != 0` -> the
staging copy is skipped, `fallback_reason = validation_failed`, native CPU
contents stay live ("任何 validation mismatch 立即 fallback", GRX_PLAN GRX-018).

## Parameters — b0 root constants (Rurix-owned block, 176 bytes / 44 dwords)

Unlike GRX-013/014 there is NO native push constant to mirror (the native path
is CPU `buffer_update`, not a dispatch); this block is Rurix-defined. cbuffer
packing note: dwords 0-3 fill one 16-byte register; the template array is
declared `uint4 surface_template[10]` so the 40 template dwords stay tightly
packed (a `uint[40]` cbuffer array would pad each element to 16 bytes and
break the dword mapping).

| dword | field | type | used by kernel |
| --- | --- | --- | --- |
| 0 | `surface_count` | u32 | both (thread guard; 1..=8) |
| 1 | `max_instance_count` | u32 | both (clamp ceiling = `multimesh->instances`) |
| 2 | `survivor_count_word_offset` | u32 | both (survivor buffer word index) |
| 3 | `pad0` | u32 | no (carried, 0) |
| 4 + s*5 + 0 | `surface{s}_index_count` | u32 | both (dword-0 backfill / expected) |
| 4 + s*5 + 1 | `surface{s}_instance_count_reserved` | u32 | no (carried, MUST be 0; dword 1 is GPU-dynamic) |
| 4 + s*5 + 2 | `surface{s}_first_index` | u32 | both (dword-2 backfill / expected) |
| 4 + s*5 + 3 | `surface{s}_vertex_offset` | u32 | both (dword-3 backfill / expected) |
| 4 + s*5 + 4 | `surface{s}_first_instance` | u32 | both (dword-4 backfill / expected) |

(`s` in `0..8`; template slots at `s >= surface_count` are carried zeros.)
uint32 fields are carried in the Rurix RTS0 as 1-dword slots
(`RootConstantType` has no dedicated `U32`; both `F32` and `U32` occupy 1
dword and the RTS0 encodes only the dword layout — the descriptor JSON records
the true per-field type).

Runtime filler contract (later patch 0034): `surface{s}_index_count` =
`mesh_surface_get_vertices_drawn_count(mesh->surfaces[s])` (the same source
the native CPU fill uses); `first_index/vertex_offset/first_instance` = 0
(native zero-init); `max_instance_count` = `multimesh->instances`;
`survivor_count_word_offset` = the producer's word slot for this multimesh.

## Descriptor Layout

- Root constants: `b0 space0`, 44 dwords (176 bytes) at
  `root_parameter_index 0`.
- SRV: `src_survivor_counts = t0 space0` (`binding_kind = structured_buffer`,
  stride 4).
- UAV: `dst_command_buffer = u0 space0` (`binding_kind = rwstructured_buffer`,
  stride 4); `dst_validation = u1 space0`
  (`binding_kind = rwstructured_buffer`, stride 4).
- Required resource count for the (later) bridge gate: 3 resources, in
  src_survivor_counts / dst_command_buffer / dst_validation order.
- Required b0 size: 176 bytes; `surface_count` in `[1, 8]`; nonzero
  `max_instance_count`; command-buffer bytes `>= surface_count * 20`;
  validation bytes `>= (2 + surface_count) * 4`; survivor-buffer bytes
  `> survivor_count_word_offset * 4`.
- 64-bit integer capability: **NOT required** — no i64 b0 fields (the
  GRX-013/014 buffer-pass precedent).
- Dispatch shape (both kernels): `(1, 1, 1)` groups, local `64x1x1`
  (`surface_count <= 8 < 64`; one thread per surface). Ordering: write ->
  UAV barrier -> validate -> readback.

## Fallback Rules

- The pass remains disabled by default and runtime remains `fallback_only`.
- Missing/extra resources, non-buffer resource at any slot, zero buffer byte
  size, wrong b0 size, `surface_count` outside `[1, 8]`, zero
  `max_instance_count`, or undersized buffers return fallback.
- Descriptor layout / offline digest mismatch returns fallback (four baked
  SHA-256: write DXIL, validate DXIL, RTS0, descriptor layout).
- Missing `RXGD_CAP_INDIRECT_ARGS_REAL_PASS (1u << 12)` opt-in returns
  fallback (`manual_disabled`); texture resources fail the per-slot
  kernel-binding-kind conformance check; the shipping feature-off bridge
  fails closed (`real_dispatch_path_not_linked`).
- Nonzero `mismatch_count` or `clamp_trigger_count` after the validate
  dispatch -> staging copy skipped, `fallback_reason = validation_failed`,
  native CPU command-buffer contents remain live.
- Missing GRX-015/016 producer buffer (interface not yet landed) -> the gate
  never arms (`runtime_binding_preflight` failure); the native
  `_multimesh_set_visible_instances` CPU path continues.

## Explicit Non-Goals (this slice)

- No real runtime indirect_args GPU pass is enabled by default.
- No bridge gate, no Godot patch, no runtime native-handle resource binding
  (S4-S7, later slices; patch 0033 additionally blocked on the GRX-015/016
  runtime hook).
- No standalone dispatch smoke (S6), no in-engine visual diff (S8), no
  measured fallback telemetry, no enablement.
- No performance number or acceleration claim is made (the resident
  validation readback cost is acknowledged and unmeasured).
