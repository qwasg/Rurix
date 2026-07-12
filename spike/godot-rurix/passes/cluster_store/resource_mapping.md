# GRX-014 cluster_store Resource Mapping

## Scope

This file records the GRX-014 cluster_store resource mapping. It maps the real
Godot cluster store (the compute merge segment of
`ClusterBuilderRD::bake_cluster()`) resources and parameters into the Rurix
bridge design while keeping the runtime fallback-first. This slice covers the
offline face + host-side gate + standalone dispatch smoke (S1-S4+S6): NOT a
real GPU runtime pass, does not skip the native Godot cluster_store, and
provides no visual, telemetry, or performance evidence.

## Godot Native Flow

- Driver: `ClusterBuilderRD::bake_cluster()` in
  `external/godot-master/servers/rendering/renderer_rd/cluster_builder_rd.cpp`
  (`L438-542`); the store **compute** dispatch is `L517-538`
  (`cluster_store.glsl`, `local_size = 8x8x1`,
  `compute_list_dispatch_threads(cluster_screen_size.x, cluster_screen_size.y,
  1)` at `L535`). The preceding **rasterization segment** (`L470-513`,
  `cluster_render.glsl` proxy-mesh draw) produces this kernel's input and is
  NOT replaceable (graphics pipeline, out of scope permanently).
- Upstream driver: `render_forward_clustered.cpp:1650`
  (`current_cluster_builder->bake_cluster()`); the builder is configured by
  `render_forward_clustered.cpp:158` → `ClusterBuilderRD::setup(...)`
  (`cluster_builder_rd.cpp:281-376`).
- Buffers created in `setup()`: `cluster_render_buffer` (`L307`),
  `cluster_buffer` (`L308`), `element_buffer` (`L313`); the
  `cluster_store_uniform_set` is `L349-375` (set 0: binding 1 =
  cluster_render_buffer, binding 2 = cluster_buffer, binding 3 =
  element_buffer).
- Guards the gate inherits: the store dispatch only runs when
  `render_element_count > 0` (`L446`); `cluster_buffer` is zero-cleared every
  frame before writes (`L444`); `cluster_render_buffer` is zero-cleared before
  the raster fill (`L448`).

## Godot Resources

| Godot resource | Role | Native binding shape | GRX-014 Rurix mapping |
| --- | --- | --- | --- |
| `cluster_render_buffer` (`uint[]` SSBO; `cluster_builder_rd.cpp:307`) | raster-segment output: per-cluster usage bitmap + per-element z ranges (input) | `std430 restrict readonly buffer ClusterRender { uint data[]; }` at set 0 binding 1 of `cluster_store.glsl` | `cluster_render` SRV `t0 space0`, `binding_kind = structured_buffer` (stride 4) |
| `element_buffer` (`RenderElementData[]` SSBO; `cluster_builder_rd.cpp:313`) | element table (input; store segment reads 4 leading u32 fields only) | `std430 restrict readonly buffer RenderElements { RenderElement data[]; }` at set 0 binding 3 | `render_elements` SRV `t1 space0`, `binding_kind = structured_buffer` (stride 80) |
| `cluster_buffer` (`uint[]` SSBO; `cluster_builder_rd.cpp:308`) | renderer-facing packed cluster table (output) | `std430 restrict buffer ClusterStore { uint data[]; }` at set 0 binding 2 | `cluster_store` UAV `u0 space0`, `binding_kind = rwstructured_buffer` (stride 4) |

Binding-order note: the Rurix descriptor order is `[t0 cluster_render,
t1 render_elements, u0 cluster_store]` (inputs first, output last — the
GRX-013 src-then-dst convention extended to two SRVs). Godot's set-0 binding
numbers (1 = cluster_render, 2 = cluster_store, 3 = element_buffer) are a
different order; the (later) patch 0024 native-handle mapping must hand the
bridge the resources in the Rurix slot order above.

### RenderElementData SSBO element layout (std430, 80 bytes; matches HLSL struct)

| Field | GLSL type | C++ field (`cluster_builder_rd.h:161-169`) | std430 offset | size | used by store kernel |
| --- | --- | --- | --- | --- | --- |
| `type` | `uint` (0-3, `ELEMENT_TYPE_MAX = 4`) | `uint32_t type` | 0 | 4 | yes (dst block select) |
| `touches_near` | `bool` (4-byte) | `uint32_t touches_near` | 4 | 4 | yes (`from_z = 0` override) |
| `touches_far` | `bool` (4-byte) | `uint32_t touches_far` | 8 | 4 | yes (`to_z = 32` override) |
| `original_index` | `uint` | `uint32_t original_index` | 12 | 4 | yes (minmax + bitmap payload) |
| `transform_inv` | `mat3x4` | `float transform_inv[12]` | 16 | 48 | no (raster segment only; carried) |
| `scale` | `vec3` | `float scale[3]` | 64 | 12 | no (raster segment only; carried) |
| `pad` | `uint` | `uint32_t has_wide_spot_angle` | 76 | 4 | no (raster segment only; carried) |

### cluster_render_buffer input layout (uint words)

Per cluster (`base_offset = pos.x + cluster_screen_size.x * pos.y`), at
`src_offset = base_offset * cluster_render_data_size`:

| word range | contents |
| --- | --- |
| `[src_offset, src_offset + max_render_element_count_div_32)` | usage bitmap: bit `index & 31` of word `index >> 5` set iff element `index` touches this cluster |
| `[src_offset + max_render_element_count_div_32, src_offset + cluster_render_data_size)` | per-element z_range word: bit `z` set iff the element covers depth slice `z` (0..31); `0` means untouched (guarded, "should always be > 0" when the usage bit is set) |

`cluster_render_data_size = render_element_max / 32 + render_element_max`
where `render_element_max = max_elements_by_type * ELEMENT_TYPE_MAX` (setup
`L300`, bake `L523`).

### cluster_buffer output layout (uint words)

Per `(cluster, type)` block at `dst_offset = (base_offset + type *
(cluster_screen_size.x * cluster_screen_size.y)) *
(max_cluster_element_count_div_32 + 32)`:

| word range | contents |
| --- | --- |
| `[dst_offset, dst_offset + max_cluster_element_count_div_32)` | existence bitmap over `original_index`: `dst[dst_offset + (orig >> 5)] \|= 1 << (orig & 31)` |
| `[dst_offset + max_cluster_element_count_div_32, dst_offset + max_cluster_element_count_div_32 + 32)` | 32 Z-slice words, each `(elem_min u16) \| ((elem_max + 1) u16 << 16)`; `0` = empty slice; first write initializes from `0xFFFF` (min 0xFFFF, max 0) |

Total size = `cluster_screen_size.x * cluster_screen_size.y *
(max_cluster_element_count_div_32 + 32) * 4 types * 4 bytes` (setup `L298`).
The buffer is zero-cleared by `bake_cluster()` before the dispatch; the kernel
(native and bridge alike) assumes a zeroed destination.

## Parameters — b0 root constants vs `ClusterStore::PushConstant`

b0 is a 32-byte / 8-dword mirror of
`ClusterBuilderSharedDataRD::ClusterStore::PushConstant`
(`cluster_builder_rd.h:91-100`) / the `cluster_store.glsl` `Params` push
constant (`L9-19`), so the runtime can bind Godot's push constant directly.
Every field is u32 and every field is consumed by the kernel (pads carried).

| dword | byte | `PushConstant` field | glsl `Params` field | type | used by kernel |
| --- | --- | --- | --- | --- | --- |
| 0 | 0 | `cluster_render_data_size` | `cluster_render_data_size` | u32 | yes (src stride) |
| 1 | 4 | `max_render_element_count_div_32` | `max_render_element_count_div_32` | u32 | yes (z_range region offset) |
| 2 | 8 | `cluster_screen_size[0]` | `cluster_screen_size.x` | u32 | yes (bounds + addressing) |
| 3 | 12 | `cluster_screen_size[1]` | `cluster_screen_size.y` | u32 | yes (bounds + addressing) |
| 4 | 16 | `render_element_count_div_32` | `render_element_count_div_32` | u32 | yes (usage word scan bound) |
| 5 | 20 | `max_cluster_element_count_div_32` | `max_cluster_element_count_div_32` | u32 | yes (dst block stride + slice base) |
| 6 | 24 | `pad1` | `pad1` | u32 | no (carried, 0) |
| 7 | 28 | `pad2` | `pad2` | u32 | no (carried, 0) |

uint32 fields are carried in the Rurix RTS0 as 1-dword slots (`RootConstantType`
has no dedicated `U32`; both `F32` and `U32` occupy 1 dword and the RTS0
encodes only the dword layout — the descriptor JSON records the true per-field
type).

## Descriptor Layout

- Root constants / root-cbuffer mapping: `b0 space0`, 8 dwords (32 bytes) at
  `root_parameter_index 0` (the `ClusterStore::PushConstant` mirror above).
- SRV: `cluster_render = t0 space0` (`binding_kind = structured_buffer`,
  stride 4); `render_elements = t1 space0` (`binding_kind =
  structured_buffer`, stride 80).
- UAV: `cluster_store = u0 space0` (`binding_kind = rwstructured_buffer`,
  stride 4).
- Required resource count for the bridge gate: 3 resources, in
  cluster_render / render_elements / cluster_store order.
- Required push constant size for the bridge gate: 32 bytes; nonzero
  `cluster_screen_size[0..1]` (dwords 2-3) and nonzero
  `render_element_count_div_32` (dword 4).
- 64-bit integer capability: **NOT required** — cluster_store carries no i64
  push-constant fields (contrast with the GRX-009/010/011/012 texture passes
  whose b0 carries i64 dims; same as GRX-013 particles_copy).
- Dispatch shape: `(ceil(cluster_screen_size.x / 8),
  ceil(cluster_screen_size.y / 8), 1)`, local 8x8x1.

## Fallback Rules

- The pass remains disabled by default and runtime remains `fallback_only`.
- Missing/extra resources, non-buffer resource at any slot, zero buffer byte
  size, wrong b0 size, zero `cluster_screen_size` dims, or zero
  `render_element_count_div_32` returns fallback.
- Descriptor layout / offline digest mismatch returns fallback.
- Missing `RXGD_CAP_CLUSTER_STORE_REAL_PASS` opt-in returns fallback
  (`manual_disabled`); texture (non-structured-buffer) resources fail the
  per-slot kernel-binding-kind conformance check; the shipping feature-off
  bridge fails closed (`real_dispatch_path_not_linked`).
- The native Godot cluster_store path remains the active path whenever the
  bridge does not return OK.

## Explicit Non-Goals (this slice)

- No real runtime cluster_store GPU pass is enabled by default.
- No Godot patch, no runtime native-handle resource binding (patches
  0023-0025, later serial slices).
- No replacement of the rasterization segment, the buffer clears, or the
  `render_element_count == 0` early-out (native permanently / native guards).
- No in-engine visual diff (S8), no measured fallback telemetry, no
  enablement.
- No performance number or acceleration claim is made.
