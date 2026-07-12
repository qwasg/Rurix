# GRX-013 particles_copy Resource Mapping

## Scope

This file records the GRX-013 particles_copy resource mapping. It maps the real
Godot particles copy (`COPY_MODE_FILL_INSTANCES`, 3D) resources and parameters
into the Rurix bridge design while keeping the runtime fallback-first. This is
the S1-S3 offline face: NOT a real GPU runtime pass, does not skip the native
Godot particles_copy, and provides no visual, telemetry, or performance
evidence.

## Godot Native Flow

- Driver: `RendererRD::ParticlesStorage::particles_set_view_axis(...)` in
  `external/godot-master/servers/rendering/renderer_rd/storage_rd/particles_storage.cpp`
  (`L1235`); the `COPY_MODE_FILL_INSTANCES` dispatch is `L1337-1351`. This is a
  **compute** pass (`particles_copy.glsl`, `local_size_x = 64`).
- Upstream driver: `renderer_scene_cull.cpp:2933`
  (`call_on_render_thread(... _scene_particles_set_view_axis ...)`) →
  `renderer_scene_cull.cpp:3195-3196` → `particles_set_view_axis(...)`.
- Buffers created in `_particles_update_buffers` (`L1354-1427`):
  `particle_buffer` (Particles SSBO, set 0 binding 1) and
  `particle_instance_buffer` (Transforms SSBO, set 0 binding 2).
- Pass shape: one thread per instance;
  `compute_list_dispatch_threads(total_particles, 1, 1)` (`L1349`). A preceding
  `FILL_SORT_BUFFER` pass + `sort_effects->sort_buffer(...)` runs only when
  `draw_order == VIEW_DEPTH` (`L1318-1331`; out of scope).

## Godot Resources

| Godot resource | Role | Native binding shape | GRX-013 Rurix mapping |
| --- | --- | --- | --- |
| `particle_buffer` (Particles SSBO, `ParticleData[]`; `particles_storage.cpp:1394`) | simulation result (input) | `std430 readonly buffer Particles { ParticleData data[]; }` at set 0 binding 1 of `particles_copy.glsl` | `src_particles` SRV `t0 space0`, `binding_kind = structured_buffer` |
| `particle_instance_buffer` (Transforms SSBO, `vec4 data[]`; `particles_storage.cpp:1399/1407`) | render instance transforms (output) | `std430 writeonly buffer Transforms { vec4 data[]; }` at set 0 binding 2 | `dst_instances` UAV `u0 space0`, `binding_kind = rwstructured_buffer` (3D: 5 vec4 per instance) |
| `particles_sort_buffer` (set 1) | VIEW_DEPTH sort | `SortBuffer { vec2 data[]; }` | NOT mapped (known gap; no `USE_SORT_BUFFER`) |
| `trail_bind_pose_uniform_set` (set 2) | trail bind poses | `TrailBindPoses { mat4 data[]; }` | NOT mapped (known gap; `trail_size == 1`) |

### ParticleData SSBO element layout (std430, 112 bytes; matches HLSL struct)

| Field | GLSL type | std430 offset | size | HLSL bridge (explicit columns) |
| --- | --- | --- | --- | --- |
| `xform` col 0 | `mat4` col 0 | 0 | 16 | `float4 xform_c0` |
| `xform` col 1 | `mat4` col 1 | 16 | 16 | `float4 xform_c1` |
| `xform` col 2 | `mat4` col 2 | 32 | 16 | `float4 xform_c2` |
| `xform` col 3 | `mat4` col 3 (translation `.xyz`) | 48 | 16 | `float4 xform_c3` |
| `velocity` | `vec3` | 64 | 12 | `float3 velocity` |
| `flags` | `uint` | 76 | 4 | `uint flags` (packs into the vec3's 16-byte slot) |
| `color` | `vec4` | 80 | 16 | `float4 color` |
| `custom` | `vec4` | 96 | 16 | `float4 custom` |

Godot's `mat4` is column-major (`txform[i]` = column i). The HLSL bridge kernel
loads the mat4 as four explicit `float4` columns to keep the algebra bit-faithful
to a column-major reference, rather than relying on HLSL matrix conventions. No
`userdata` (USERDATA_COUNT undefined) — stride is fixed at 112 bytes.

### Transforms SSBO output layout (3D, 5 vec4 per instance)

`instances.data[instance_index * 5 + k]` (`particles_copy.glsl:340-346`), where
`instance_index = gl_GlobalInvocationID.x + motion_vectors_current_offset`:

| k | contents |
| --- | --- |
| 0 | `transpose(txform)` row 0 = `(txform[0].x, txform[1].x, txform[2].x, txform[3].x)` |
| 1 | `transpose(txform)` row 1 = `(txform[0].y, txform[1].y, txform[2].y, txform[3].y)` |
| 2 | `transpose(txform)` row 2 = `(txform[0].z, txform[1].z, txform[2].z, txform[3].z)` |
| 3 | `particles.data[particle].color` |
| 4 | `particles.data[particle].custom` |

(2D writes 4 vec4 with `xform` rows 0..1 only — out of scope.)

## Parameters — b0 root constants vs `CopyPushConstant`

b0 is a 128-byte / 32-dword mirror of `ParticlesShader::CopyPushConstant`
(`particles_storage.h:303-329`) / the `particles_copy.glsl` `Params` push
constant (`L51-75`), so the runtime can bind Godot's push constant directly. The
in-scope kernel consumes a subset; the rest are carried for byte-exact shape
parity.

| dword | byte | `CopyPushConstant` field | glsl `Params` field | type | used by in-scope kernel |
| --- | --- | --- | --- | --- | --- |
| 0-2 | 0 | `sort_direction[3]` | `sort_direction` | f32×3 | yes (billboard axis + local basis) |
| 3 | 12 | `total_particles` | `total_particles` | u32 | yes (bounds check) |
| 4 | 16 | `trail_size` | `trail_size` | u32 | in scope == 1 (trail_size>1 gap) |
| 5 | 20 | `trail_total` | `trail_total` | u32 | no (trail; carried) |
| 6 | 24 | `frame_delta` | `frame_delta` | f32 | no (trail; carried) |
| 7 | 28 | `frame_remainder` | `frame_remainder` | f32 | yes (velocity integration) |
| 8-10 | 32 | `align_up[3]` | `align_up` | f32×3 | yes (billboard) |
| 11 | 44 | `align_mode` | `align_mode` | u32 | yes (branch) |
| 12 | 48 | `lifetime_split` | `lifetime_split` | u32 | no (lifetime reindex; carried) |
| 13 | 52 | `lifetime_reverse` | `lifetime_reverse` | u32 | no (lifetime reindex; carried) |
| 14 | 56 | `motion_vectors_current_offset` | `motion_vectors_current_offset` | u32 | yes (write offset; tested at 0) |
| 15 | 60 | bitfield `order_by_lifetime:1, copy_mode_2d:1` | `flags` | u32 | in scope == 0 (2D/lifetime gaps) |
| 16-27 | 64 | `inv_emission_transform[12]` | `inv_emission_transform[12]` | f32×12 | no (2D only; carried) |
| 28 | 112 | `align_src` | `align_channel_filter` | u32 | yes (billboard angle channel) |
| 29 | 116 | `align_axis` | `align_axis` | u32 | no (align 2/3/4 only; carried) |
| 30 | 120 | `pad1` | `pad1` | u32 | no |
| 31 | 124 | `pad2` | `pad2` | u32 | no |

Note: the glsl `Params.flags` (dword 15) is the C++ `CopyPushConstant`
anonymous bitfield (`order_by_lifetime:1, copy_mode_2d:1`); bit 0 =
`PARAMS_FLAG_ORDER_BY_LIFETIME`, bit 1 = `PARAMS_FLAG_COPY_MODE_2D`. The glsl
`align_channel_filter` (dword 28) is the C++ `align_src`.

uint32 fields are carried in the Rurix RTS0 as 1-dword slots (`RootConstantType`
has no dedicated `U32`; both `F32` and `U32` occupy 1 dword and the RTS0 encodes
only the dword layout — the descriptor JSON records the true per-field type).

## Descriptor Layout

- Root constants / root-cbuffer mapping: `b0 space0`, 32 dwords (128 bytes) at
  `root_parameter_index 0` (the `CopyPushConstant` mirror above).
- SRV: `src_particles = t0 space0` (`binding_kind = structured_buffer`).
- UAV: `dst_instances = u0 space0` (`binding_kind = rwstructured_buffer`).
- Required resource count for the (later) bridge gate: 2 resources, source then
  destination.
- Required push constant size for the (later) bridge gate: 128 bytes.
- 64-bit integer capability: **NOT required** — particles_copy carries no i64
  push-constant fields (contrast with the GRX-009/010/011 texture passes whose
  b0 carries i64 dims).

## Fallback Rules

- The pass remains disabled by default and runtime remains `fallback_only`.
- Missing source or destination resource returns fallback.
- Descriptor layout mismatch returns fallback.
- (Later) missing `RXGD_CAP_PARTICLES_COPY_REAL_PASS` opt-in returns fallback
  (`manual_disabled`); texture (non-structured-buffer) resources fail the
  per-slot kernel-binding-kind conformance check; the shipping feature-off
  bridge fails closed.
- The native Godot particles_copy path remains the active path whenever the
  bridge does not return OK.

## Explicit Non-Goals (this slice)

- No real runtime particles_copy GPU pass is enabled by default.
- No bridge gate, no Godot patch, no runtime native-handle resource binding
  (S4/S5/S7-level work, later slices; includes the cull-stage hook design).
- No 2D copy mode, no sort/lifetime reindex, no trail, no userdata, no align
  modes 2/3/4.
- No standalone GPU dispatch smoke (S6), no in-engine visual diff (S8), no
  measured fallback telemetry.
- No performance number or acceleration claim is made.
