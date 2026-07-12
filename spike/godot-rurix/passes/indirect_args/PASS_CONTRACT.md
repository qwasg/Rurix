# GRX-018 indirect_args Pass — PASS CONTRACT

> **Status (2026-07-12, slice S1-S3: contract trio + offline kernel + math
> parity).**
> This slice delivers the OFFLINE face only, reusing the matured GRX-009..014
> per-pass template (`../PASS_TEMPLATE.md`): the pass contract trio (S1), the
> HLSL-bridge math-equivalent kernel PAIR (write kernel + RESIDENT validation
> red-leg kernel; DXC `cs_6_0` compile + DXV validation + Rurix-owned RTS0 via
> `rurixc::binding_layout`, owner-approved `hlsl_bridge_workaround` provenance)
> (S2), and the CPU integer-exact math-parity reference (S3).
>
> This slice does **NOT** author a bridge gate, a Godot patch, runtime resource
> binding, a GPU dispatch smoke, an in-engine visual diff, or a real-pass
> enablement. Those are later slices (S4-S9). Measured ceiling here is: DXC
> compile + DXV validation pass + CPU integer-exact parity reference (zero
> tolerance; the pass is pure u32 word math).
>
> The pass ships **default disabled**; any compile / validation / visual / perf
> failure runs the native Godot indirect command-buffer CPU path. Section 3's
> investigation only records paths and function names; Godot-side changes land
> only as `spike/godot-rurix/patches/` patch files, never by editing the
> `external/godot-master` snapshot.

## 1. Pass identity

- `pass_id = indirect_args`
- bridge pass id: `RXGD_PASS_INDIRECT_ARGS = 9` (already allocated in the
  `RXGD_PASS_*` enum, `src/rurix-godot/src/lib.rs:35` /
  `src/rurix-godot/include/rurix_godot.h:115`; wired in a later slice)
- real-pass cap bit: `RXGD_CAP_INDIRECT_ARGS_REAL_PASS = 1u << 12` (reserved in
  `PATCH_ALLOCATION.md` §3, bit 12; set only by the later real-pass opt-in
  patch 0035)
- Tier: Tier 2 (raw-buffer compute pass; GRX-018, "indirect draw argument
  generation" in `milestones/grx/GRX_PLAN.md`)
- target backend: `Godot 4.7-dev Windows D3D12 Forward+`
- default enable state: `disabled`
- upstream producer dependency: GRX-015 gpu_culling (count-only survivor
  counts) or GRX-016 instance_compaction (compacted survivor counts) — see
  §4.1; both are parallel in-flight passes and neither has landed its runtime
  interface yet, so this dependency is declared fail-closed here.

## 2. Target scenes

- `many_mesh_instances` (the CPU cull / draw-list stress bench scene; its
  MultiMesh component reserves `use_indirect` for GRX-015/016/018 per
  `spike/godot-rurix/bench/generate_benchmark_project.py` SCENE_NOTES)
- `mixed_forward_plus`

(indirect_args is the per-frame "survivor count -> indirect draw command
block" transform for indirect MultiMesh: it writes the complete 5-dword
command block per surface on the GPU so the CPU never has to round-trip the
visible-instance count through `buffer_update`.)

## 3. Godot-side hook / call-site / resource-flow investigation

Records paths and functions only; **`external/godot-master` is not modified.**

### 3.1 Storage class — command buffer lifecycle

- Source: `servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp`
- Header: `servers/rendering/renderer_rd/storage_rd/mesh_storage.h`
- Key constants / functions / types:
  - `MeshStorage::IndirectMultiMesh::INDIRECT_MULTIMESH_COMMAND_STRIDE = 5`
    (`mesh_storage.h:62-64`) — every surface owns one 5-dword (20-byte)
    command block inside `multimesh->command_buffer`.
  - `MultiMesh::command_buffer` (`mesh_storage.h:258`, "used if indirect
    setting is used").
  - `MeshStorage::_multimesh_allocate_data(..., bool p_use_indirect)`
    (`mesh_storage.cpp:1547`; `multimesh->indirect = p_use_indirect` at
    `L1583-1584`). The GDScript-facing switch is
    `RenderingServer::multimesh_allocate_data(..., use_indirect)`
    (`rendering_server.cpp:2471`, default `false`); the scene-level
    `MultiMesh` resource does not expose it.
  - **CPU template fill** — `MeshStorage::_multimesh_set_mesh`
    (`mesh_storage.cpp:1666`; indirect branch `L1674-1696`): allocates
    `sizeof(uint32_t) * INDIRECT_MULTIMESH_COMMAND_STRIDE * surface_count`
    bytes zero-initialized (`L1682-1683`), then for each surface `i` writes
    `count = mesh_surface_get_vertices_drawn_count(mesh->surfaces[i])`
    byte-wise into **dword 0** of block `i` (`L1685-1691`), and creates the
    buffer with `RD::STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT` (`L1693`).
    Dwords 1-4 of every block stay zero-initialized.
  - `mesh_surface_get_vertices_drawn_count` (`mesh_storage.h:460-463`):
    `index_count ? index_count : vertex_count` — dword 0 is correct for both
    indexed and non-indexed surfaces.
  - **CPU per-update write (the elimination target)** —
    `MeshStorage::_multimesh_set_visible_instances`
    (`mesh_storage.cpp:2187`; indirect branch `L2206-2213`): for each surface
    it issues `RD::buffer_update(command_buffer, (i * 5 + 1) * 4, 4,
    &p_visible)` at `L2210`, i.e. a CPU write of **dword 1**
    (`instance_count`) of every surface block. When a CPU-side culling/LOD
    loop drives `multimesh_set_visible_instances` every frame, this is a
    per-frame CPU->GPU `buffer_update` round-trip per multimesh. GRX-018
    replaces exactly this write with a GPU kernel fed by the GRX-015/016
    survivor count.
  - `_multimesh_get_command_buffer_rd_rid` (`mesh_storage.h:678`; bound at
    `rendering_server.cpp:2489`) — the RD-level handle the (later) runtime
    binding patch resolves to a native `ID3D12Resource*`.

### 3.2 Consumer — the indirect draw

- `render_forward_clustered.cpp:602` — `indirect = bool(surf->owner->
  base_flags & INSTANCE_DATA_FLAG_MULTIMESH_INDIRECT)` (flag `1 << 2`,
  `render_forward_clustered.h:269`, set at `render_forward_clustered.cpp:4356`
  when `mesh_storage->multimesh_uses_indirect(...)`).
- `render_forward_clustered.cpp:610` — `RD::draw_list_draw_indirect(draw_list,
  index_array_rd.is_valid(), _multimesh_get_command_buffer_rd_rid(...),
  surf->surface_index * sizeof(uint32_t) * INDIRECT_MULTIMESH_COMMAND_STRIDE,
  1, 0)`: one indirect draw per surface, `draw_count = 1`, offset = block `i`.
  (`render_forward_mobile.cpp:2574` is the mobile twin; out of scope.)
- 5-dword block layout consumed for indexed surfaces (the RD
  draw-indexed-indirect command layout, `VkDrawIndexedIndirectCommand`-
  compatible; the D3D12 driver consumes it through an equivalent command
  signature):

  | dword | field | native producer |
  | --- | --- | --- |
  | 0 | `index_count` | CPU fill (`_multimesh_set_mesh`, from `mesh_surface_get_vertices_drawn_count`) |
  | 1 | `instance_count` | CPU per-update write (`_multimesh_set_visible_instances` `L2210`) |
  | 2 | `first_index` | zero-init (never written natively) |
  | 3 | `vertex_offset` | zero-init (never written natively) |
  | 4 | `first_instance` | zero-init (never written natively) |

  For non-indexed surfaces `draw_list_draw_indirect(p_use_indices = false)`
  interprets the first four dwords as `{vertex_count, instance_count,
  first_vertex, first_instance}` and ignores dword 4; because dword 0 already
  carries `vertex_count` (the `vertices_drawn_count` fallback) and dwords 2-4
  are natively zero, the same block content is value-correct for both draw
  kinds.

### 3.3 Call / injection candidate point

- The GPU write must land **after** the GRX-015/016 producer pass has written
  the frame's survivor count and **before** the draw list consumes
  `command_buffer` (`render_forward_clustered.cpp:610`). Because neither
  GRX-015 (gpu_culling) nor GRX-016 (instance_compaction) has landed its
  runtime hook yet, the concrete injection point is **deferred to the patch
  slice (0033)** and constrained here instead:
  - candidate (a): immediately after the producer pass's survivor-count
    dispatch in the frame graph (same compute list, UAV barrier between);
  - candidate (b): a pre-render hook adjacent to the existing cull stage
    (the GRX-013 `particles_set_view_axis` precedent shows cull-stage hooks
    are workable but structurally different from post-process hooks).
- The native CPU path (`_multimesh_set_visible_instances` +
  `_multimesh_set_mesh` fill) is always preserved as fallback/continuation;
  the opt-in gate returning false (the default) leaves the native contents
  untouched.

### 3.4 Resource flow (native vs Rurix)

- Input: the survivor-count buffer produced by GRX-015 (count-only) or
  GRX-016 (compacted count) — a device-local `uint[]` SSBO; the word at
  `survivor_count_word_offset` carries THIS multimesh's surviving instance
  count for the frame (§4.1 producer interface).
- Output: `multimesh->command_buffer` (`uint[]`, 5 dwords per surface,
  created `STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT`, `mesh_storage.cpp:1693`).
- Validation output: a small Rurix-owned scratch `uint[]` buffer
  (`2 + surface_count` words, zeroed before the validation dispatch); not a
  Godot resource.

## 4. Input / output resources (Rurix mapping)

- Input: `src_survivor_counts = StructuredBuffer<uint>`, SRV `t0 space0`,
  `binding_kind = structured_buffer` (stride 4).
- Output: `dst_command_buffer = RWStructuredBuffer<uint>`, UAV `u0 space0`,
  `binding_kind = rwstructured_buffer` (stride 4; Godot
  `multimesh->command_buffer`, 5 dwords per surface).
- Validation output: `dst_validation = RWStructuredBuffer<uint>`, UAV
  `u1 space0`, `binding_kind = rwstructured_buffer` (stride 4; layout in §5.4).
- b0 root constants: 176-byte / 44-dword Rurix-owned parameter block
  (`root_parameter_index 0`; field-by-field in `resource_mapping.md` and
  `artifacts/indirect_args_descriptor_layout.json`): `surface_count`,
  `max_instance_count`, `survivor_count_word_offset`, `pad0`, then the
  per-surface 5-dword command TEMPLATE array (`MAX_SURFACES = 8` slots:
  `{index_count, instance_count_reserved(0), first_index, vertex_offset,
  first_instance}` per surface). Unlike the GRX-009..012 texture passes, b0
  carries **no i64** field, so `SHADER_INT64` is not part of the preflight
  (the GRX-013/014 precedent).
- Both kernels (write + validate) share this single binding surface and the
  single Rurix-owned RTS0 (the write kernel simply never references `u1`;
  a root signature superset is legal for both PSOs).
- tracked mapping: `resource_mapping.md`.

### 4.1 GRX-015/016 producer interface (explicit dependency declaration)

Neither producer pass has landed; this contract pins the interface the
indirect_args kernel consumes so the three passes can integrate without a
re-spec:

1. The producer publishes a device-local `uint[]` SSBO; the u32 word at
   `survivor_count_word_offset` (a b0 field chosen by the runtime filler,
   exercised at 0 and nonzero offline) is the multimesh's surviving instance
   count for the frame. Acceptable producers: GRX-015 gpu_culling
   **count-only** output, or GRX-016 instance_compaction **compacted count**
   output (the count word accompanying its compacted instance stream).
2. Producer guarantee: `0 <= count <= multimesh->instances`
   (= b0 `max_instance_count`). A count above `max_instance_count` is a
   producer-interface violation: the write kernel still clamps (defense in
   depth) and the validation kernel counts it as a `clamp_trigger` red flag
   (§5.4) — the frame falls back.
3. Visibility: the producer's UAV write must be fenced/barriered before this
   pass's dispatch (runtime patch responsibility; the offline slices use
   synthetic survivor buffers).
4. Fail-closed: until GRX-015/016 land a real runtime survivor buffer, the
   (later) bridge gate cannot arm — a missing/null survivor-count resource is
   a `runtime_binding_preflight` failure and the native CPU path continues.

## 5. Supported subset, kernels, and route choice

### 5.1 In-scope subset (this slice)

- One multimesh per dispatch; `surface_count` in `[1, MAX_SURFACES = 8]`;
  one thread per surface (`numthreads(64,1,1)`, dispatch `(1,1,1)`).
- **Write kernel** (`artifacts/hlsl_bridge/indirect_args_write.hlsl`), per
  surface `s`:
  - `survivors = src_survivor_counts[survivor_count_word_offset]` (one shared
    count for all surfaces — mirroring the native path, which writes the same
    `p_visible` into every surface block);
  - `clamped = min(survivors, max_instance_count)` (out-of-range clamp);
  - `dst_command_buffer[s*5 + 0] = template[s].index_count` (b0 backfill);
  - `dst_command_buffer[s*5 + 1] = clamped` (the ONLY GPU-dynamic dword);
  - `dst_command_buffer[s*5 + 2..4] = template[s].{first_index,
    vertex_offset, first_instance}` (b0 backfill; natively all zero).
- All five dwords are written every dispatch, so a stale/partial block cannot
  survive a stride or offset bug silently — any miswrite lands in a dword the
  validation kernel checks.
- Pure u32 math (`min`, compare, copy); the CPU reference is integer-exact
  with ZERO tolerance.

### 5.2 Out of scope (known gaps; `pass_manifest.json` `known_gaps` per line)

- `surface_count > 8`; multiple multimeshes per dispatch (no cross-multimesh
  batching); `draw_count > 1` multi-draw blocks.
- The `visible_instances == -1` "all visible" sentinel (native
  `_multimesh_set_visible_instances` accepts `-1`; producer counts here are
  absolute u32).
- Per-surface distinct survivor counts (the native path shares one count
  across surfaces; a per-surface count table is a GRX-016 follow-up if its
  compaction ever splits by surface).
- Nonzero `first_index` / `vertex_offset` / `first_instance` never occur
  natively (the CPU fill zero-inits dwords 2-4); template fidelity for
  nonzero statics is proven offline only.
- `render_forward_mobile` twin call-site; the runtime hook/patch (0033-0035);
  GPU-observed parity (pending the S6 real dispatch).

### 5.3 Offline kernel route: HLSL bridge (chosen), not rurixc-native

indirect_args is an **all raw-buffer / SSBO** pass, so the GRX-009
texture-intrinsic `llc` blocker does **not** apply. Even so, a rurixc-owned
`rx -> DXIL` compile is infeasible today (the GRX-014 cluster_store blocker
set, minus the findLSB item, plus atomics):

1. **No u32 buffer views.** The DXIL compute-body lowering accepts only
   `View<global, f32>` / `ViewMut<global, f32>` buffer views
   (`src/rurixc/src/dxil_codegen.rs:1754/1786`). Every buffer here is u32
   command/count words, and an f32 view cannot carry arbitrary u32 payloads
   bit-faithfully (an `index_count` above 2^24 would round).
2. **Integer bit operations are not wired on the DXIL path.** The validation
   kernel builds per-surface mismatch bitmasks from `|`, `<<`, `&`; MIR
   carries the ops (`src/rurixc/src/mir.rs:643-647`) but the DXIL backend has
   no lowering for them.
3. **No atomics.** The validation kernel's global mismatch / clamp counters
   need `InterlockedAdd`; the Rurix lang subset has no atomic intrinsic on
   any backend.

The canonical offline package is therefore the owner-approved
`hlsl_bridge_workaround`: DXC `cs_6_0` DXIL containers (validated by DXV) with
a **Rurix-owned RTS0** root signature (`rurixc::binding_layout::
{infer_root_signature, pack_root_constants, serialize_rts0}`), mirroring the
GRX-013/014 buffer-pass precedent (`../cluster_store/PASS_CONTRACT.md` §5.3 /
`../particles_copy/PASS_CONTRACT.md` §5.3, ultimately
`../luminance_reduction/texture_artifact_provenance_policy.json`).
`src/lib.rx` documents the kernel math and the three blockers; the executable
math lives only in the HLSL bridge kernels.

### 5.4 RESIDENT validation red leg (normative; GRX_PLAN mandate)

`GRX_PLAN.md` GRX-018 row: "任何 validation mismatch 立即 fallback". A wrong
args/surface-stride pairing is a GPU-hang-class risk (the indirect draw
consumes whatever dwords sit at `surface_index * 5`), so validation is a
**resident red leg** — part of the pass, not a test-only artifact:

- **Validation kernel** (`artifacts/hlsl_bridge/indirect_args_validate.hlsl`,
  compiled/validated alongside the write kernel, same b0/RTS0), per surface
  `s`, recomputes `expected_instance_count = min(survivors,
  max_instance_count)` and compares the generated block dword-by-dword
  against the b0 template + expected count. Per-surface mismatch bitmask in
  `dst_validation[2 + s]`:
  - bit 0..4: generated dword `c` != expected dword `c`;
  - bit 5: in-buffer `instance_count > max_instance_count` (an unclamped or
    foreign writer);
  - bit 6: `survivors > max_instance_count` (producer-interface violation;
    the clamp fired).
  `dst_validation[0]` counts surfaces with `(mask & 0x3F) != 0` (mismatch);
  `dst_validation[1]` counts surfaces with bit 6 (clamp trigger); both via
  `InterlockedAdd`; the buffer is zeroed before the dispatch.
- **Runtime policy (binds S4-S8 design)**: the real-pass arm writes into a
  Rurix-owned STAGING command buffer, runs the validation dispatch (UAV
  barrier between), reads back `dst_validation`, and only when
  `mismatch_count == 0 AND clamp_trigger_count == 0` copies the staging
  blocks over `multimesh->command_buffer` (GPU `buffer_copy`). On ANY nonzero
  counter the copy is skipped, the native CPU contents remain live for the
  draw, the pass records `fallback_reason = validation_failed`, and the
  once-per-session `RXGD_INDIRECT_ARGS_REAL_PASS_BLOCKED` diagnostic prints.
  Bad args are therefore never exposed to `draw_list_draw_indirect`, even
  transiently. The same-frame readback is a deliberate correctness-first
  cost; no performance claim exists until S8 measures the real chain.
- **Offline red leg (this slice)**: `generate_math_parity_evidence.py`
  includes a deliberately corrupted command-buffer case whose CPU reference
  PROVES the validation kernel's expected output flags the corruption
  (nonzero mismatch mask + count); a validation reference that reports clean
  on the corrupted fixture fails the generator's coverage gate.
- **S6/S8 red legs (later)**: the standalone dispatch smoke must run
  write -> barrier -> validate -> readback and additionally re-run validation
  against a corrupted staging buffer expecting nonzero counters; the
  enablement smoke keeps the forced-capability-downgrade leg from the
  GRX-010..014 template.

## 6. Fallback

- fallback reason enum (aligned with the GRX-008 five): `compile_failed` /
  `validation_failed` / `unsupported_device` / `visual_diff_failed` /
  `manual_disabled`.
- Any compile / validation / visual / perf failure -> native Godot indirect
  command-buffer CPU path (`godot_native_indirect_command_buffer`): the
  `_multimesh_set_mesh` template fill + `_multimesh_set_visible_instances`
  dword-1 update remain authoritative whenever the pass is not armed or any
  validation counter is nonzero.
- (Later slices) the default Godot config (per-pass settings all `false`) and
  the shipping bridge return `RXGD_STATUS_FALLBACK` for
  `RXGD_PASS_INDIRECT_ARGS`; the shipping feature-off bridge fails closed
  with `real_dispatch_path_not_linked`.

## 7. Bridge gate — later slice (S4)

Not authored here. The later `IndirectArgsGate` in `src/rurix-godot/src/lib.rs`
will mirror the GRX-014 `ClusterStoreGate` template with a
THREE-buffer binding surface (SRV t0 survivor counts + UAV u0 command buffer +
UAV u1 validation):

- runtime binding preflight: exactly 3 buffer resources in
  src_survivor_counts / dst_command_buffer / dst_validation order, the
  176-byte b0 block, `surface_count` in `[1, 8]`, nonzero
  `max_instance_count`, command-buffer byte size `>= surface_count * 20`,
  validation byte size `>= (2 + surface_count) * 4`, survivor byte size
  `> survivor_count_word_offset * 4`; NO int64 cap check (no i64 b0 fields).
- dispatch eligibility: opt-in `RXGD_CAP_INDIRECT_ARGS_REAL_PASS (1u << 12)` +
  the recording-harness capability + non-null native device/queue + non-null
  buffer handles + `IndirectArgsDispatchPackage` layout/digest match vs the S2
  offline evidence (**four** SHA-256 digests baked in: write DXIL, validate
  DXIL, RTS0, descriptor layout).
- per-slot kernel-binding-kind conformance: `["structured_buffer",
  "rwstructured_buffer", "rwstructured_buffer"]`; texture resources fail
  closed at any slot.
- math parity gate: `indirect_args_cpu_reference_proven_pending_gpu_dispatch`.
- real dispatch (write -> barrier -> validate -> readback -> copy-if-clean per
  §5.4) only under the `d3d12-recording-shim` feature; the shipping
  feature-off bridge fails closed.
- GRX-015/016 dependency: a missing producer survivor buffer never reaches
  eligibility (preflight fails on the null/zero-size resource).

## 8. Godot patches — later slice (S5/S7)

Not authored here. `PATCH_ALLOCATION.md` §2 reserves **0033-0035** for
indirect_args (0033 gate+callsite / 0034 runtime binding / 0035
recording+real-pass opt-in). Patch 0033's call-site placement additionally
depends on the GRX-015/016 runtime hook landing first (§3.3). Patches are
generated by `git diff --no-index` on a scratch copy with all prior patches
applied, verified by `ci/godot_rurix_patch_stack.py`; never hand-written;
serialized by the §4 stack-lock.

## 9. Evidence

- **offline compile** (this slice, measured): `offline_compile_evidence.json`
  — DXC `cs_6_0` compile + DXV validation of BOTH kernels, Rurix-owned RTS0
  (`emit_grx018_indirect_args_rts0` via `rurixc::binding_layout::
  {infer_root_signature, pack_root_constants, serialize_rts0}`), descriptor
  layout (per-slot binding kinds + 176-byte / 44-dword root constants), FOUR
  artifact SHA-256 recomputable on disk; `provenance =
  hlsl_bridge_workaround`, `rurix_owned = false`, `rurix_owned_rts0 = true`,
  `runtime_mappable = true`.
- **math parity** (this slice): `math_parity_evidence.json`
  (`generate_math_parity_evidence.py`) — deterministic synthetic survivor /
  template fixtures with an **integer-exact** CPU reference (pure u32 math,
  zero tolerance) covering single-surface, multi-surface, clamp-triggered,
  zero-survivor, max-surface-count, nonzero-static-template,
  nonzero-word-offset, and corrupted-buffer validation-red cases;
  `status = pending_gpu_dispatch` until S6 fills the GPU-observed side.
- **standalone dispatch / visual / telemetry / enablement**: later slices
  (S6/S8); this file claims none of them.
- Perf: reuse GRX-006 baseline / perf gate; no performance improvement
  claimed. (The §5.4 same-frame validation readback is an acknowledged cost;
  the pass's net benefit is strictly an S8+ measurement question.)

## 10. Exit criteria (this slice)

- pass default `disabled`; manifest `implemented=false`,
  `runtime_state=fallback_only`, `real_gpu_pass=false`,
  `real_d3d12_dispatch_recorded=false` (fail-closed initial values).
- `offline_compile_evidence.json` `status=success` with DXV
  `validation.status=pass` for BOTH kernels;
  `math_parity_evidence.json` `status=pending_gpu_dispatch` with the
  corruption red case present.
- This slice does NOT represent pass completion.

## 11. Remaining items

- S4 bridge gate (`IndirectArgsGate` + shim 3-buffer record entry); S5 patch
  0033 (blocked on the GRX-015/016 runtime hook); S6 standalone dispatch smoke
  (write + validate + corrupted-staging red leg); S7 patches 0034/0035; S8
  scratch rebuild + gated real-pass enablement; S9 close-out (gate module +
  probe registration + manifest flip + owner default-enable decision).
- GRX-015/016 producer integration (real survivor buffer + frame-graph
  ordering + barriers); per-surface count table follow-up; surface_count > 8.
- Deferred-validation (non-blocking readback) design if the S8 measurement
  shows the same-frame readback dominates — only with owner sign-off, and
  validation stays resident in any variant.
- full baseline / per-pass FPS comparison; any performance claim.
