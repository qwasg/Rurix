# rurix_owned Migration Plan — retiring hlsl_bridge workarounds

Grounded in the measured feasibility matrix (`feasibility_matrix.md`) and the
existing provenance-revert precedent
(`spike/godot-rurix/passes/luminance_reduction/texture_artifact_provenance_policy.json`,
GRX-009 segment 4l). Each migration segment follows the same evidence chain the
texture-line revert defined:

> **provenance revert chain (per pass):** (1) rurixc-owned `.rx` container
> dxv-PASSES → (2) math-parity gate GREEN on the rurix_owned container (not the
> HLSL one) → (3) D3D12 real dispatch GREEN on the rurix_owned container → (4)
> flip `pass_manifest.json` / `offline_compile_evidence.json` provenance to
> `rurix_owned: true`, update the bridge digest, **revert** the
> `compile_hlsl_bridge.py` canonical-switch, keep the HLSL only as a
> cross-check fixture. Fail-closed: if any of (1)-(3) regress, provenance stays
> `hlsl_bridge_workaround`.

No pass source, manifest, or CI file is modified by this spike — this is the
plan an implementer follows in a separate change.

---

## Priority 0 — luminance_reduction (ready now, XS)

**Status: all authoring done; only evidence re-gen + provenance flip remain.**

- Measured: `src/lib.rx` (raw-buffer) and `src/lib_texture.rx` (single-channel
  texel) both dxv-PASS unmodified (`pass_direct_compile_evidence.json`).
- The provenance policy's revert-condition #1
  (`dxv_validation_pass_on_rurix_owned_container`) is now **met**, and its cited
  prerequisite ("patched llc supporting `llvm.dx.resource.load.texture.2d`") is
  proven satisfied by this `llc` build (`probe_texture_loadstore_2d`).

Steps:
1. Point `compile_offline.py` canonical artifact paths at the rurixc container
   (it already invokes `rurixc --target dxil`; today it labels the output but
   keeps the HLSL package canonical). Make the rurixc `.dxil` the canonical
   `artifacts/luminance_reduction.dxil`.
2. Re-run `generate_math_parity_evidence.py` **against the rurix_owned
   container** (revert-condition #2) and the D3D12 dispatch smoke
   (`real_d3d12_dispatch_smoke.json`, revert-condition #3).
3. Flip `offline_compile_provenance` → `rurix_owned`, `rurix_owned: true` in the
   provenance policy + manifest; update `bridge_digest`; keep
   `luminance_reduce_level.hlsl` as a parity cross-check fixture only.
4. `fused_post_chain` segment A is this same kernel — note the shared provenance.

Risk: low. Only nuance is confirming the texture variant's single-channel
binding matches the runtime luminance image view (it does — luminance is 1-ch).

---

## Priority 1 — cluster_store (compiler-gated, L; Tier-2 main target)

Highest Tier-2 payoff, and closest to reach once two rurixc features land.

- Native today: the arithmetic core (`cluster_store_arith_core.rx`) — popcount +
  z-range decode + packed min/max — dxv-PASSES.
- Requires, in rurixc (NOT pass authoring):
  1. **u32→usize cast** (`index_bit as usize`) — for `render_elements[index]`
     gather and `cluster_store[dst_offset+i]` scatter at scan-derived indices.
  2. **ViewMut read** — for the `minmax = cluster_store[slice_ofs]` merge
     (non-atomic; threads own disjoint clusters, so no atomics are needed —
     confirmed in `cluster_store_pack.hlsl`).
- No atomics, no groupshared, no transcendentals, no textures. Aggregate
  `RenderElementData` scalar-decomposes to a stride-20 u32 view (4 leading
  fields). 2D→1D linearizes. `~` → `^ 0xFFFFFFFF` (verified).

Once (1)+(2) land: author `cluster_store/src/lib.rx` as the full
scalar-decomposed kernel (template: `cluster_store_native_probe.rx` in this
spike, minus the cast/RMW rewrites), then run the standard revert chain.

Order of compiler work: **cast first, ViewMut-read second** — cast alone already
unblocks gpu_culling's index and ssao's unpack; ViewMut-read completes
cluster_store and gpu_culling's merge.

---

## Priority 2 — gpu_culling (compiler-gated, L)

- f32 distance math (transform, Frobenius `sqrt`, plane distances) is already
  expressible post-MR-0007.
- Requires: **atomics** (InterlockedAdd count / InterlockedOr bitmask) **or** a
  per-thread-slot non-atomic redesign + a merge dispatch; plus u32 **cast**
  (instance index) and **ViewMut read** (OR merge).
- Even the atomic-free redesign needs cast + ViewMut-read, so it rides
  Priority-1's compiler work; the atomics feature additionally lets the natural
  (bridge-equivalent) design go native.

---

## Priority 3 — instance_compaction (compiler-gated, L)

- Native today: D3 scatter move (`instance_compaction_scatter_lane.rx`).
- Requires: **groupshared + barrier on the DXIL path** (D1 Hillis-Steele local
  scan + D2 group scan) and u32 **cast** (bitmask decode). The `shared let` /
  `block.sync()` lang items exist but lower only on NVPTX — DXIL support is the
  gate. Alternatively, a scan-free multi-pass redesign, but that changes the
  pass contract.

---

## Priority 4 — indirect_args (compiler-gated on the VALIDATION leg, L)

- Native today: the WRITE kernel (`indirect_args_write_kernel.rx`), with the
  cap-as-u32-view adaptation.
- The pass ships as a **pair** with a *resident* VALIDATION kernel (contract
  §5.4) that **reads** the command buffer (ViewMut read) and accumulates
  mismatch/clamp counts via **atomics**. Cannot retire the bridge until both
  land. (If the contract allowed a non-resident/host-side validation, the WRITE
  kernel could migrate alone — an owner call, out of scope here.)

---

## Priority 5 — particles_copy (partial, S but not a full retire)

- Native today: ALIGN_DISABLED f32 lane (param reorder + `global` fix).
- ALIGN_BILLBOARD math (sin/cos/sqrt) is **now unblocked** by MR-0007 (pass doc
  blocker 2 is stale) — remaining work is the `flags` active-bit test (bind
  `flags` as a u32 view) and the transposed aggregate write (scalar-decompose).
  Verify a billboard-lane probe before committing.
- Because one kernel serves both align modes, retire only after both lanes are
  proven; otherwise the pass would need runtime mode-splitting.

---

## Not on the near path (deep gaps)

| Pass | Needs |
|---|---|
| tonemap | `pow`/`exp`/`log` on DXIL + float4 multi-channel texel |
| ssao_blur | f32↔u32 cast + float2 multi-channel texel |
| taa_resolve | texture **sample** (Catmull-Rom) + groupshared + 2D + multi-channel |
| fused_post_chain | `pow` + float4 + 2D dispatch + ViewMut-read (EMA) |

These require lang-surface additions (transcendental set, multi-channel texel
types, texture sampler lang item) beyond MR-0006/0007's scope; they stay
`hlsl_bridge_workaround` and should NOT be forced.

---

## Compiler-feature dependency graph (what unblocks what)

```
as-cast (u32<->usize) ─┬─► cluster_store gather/scatter ─┐
                       ├─► gpu_culling instance index    ├─(+ ViewMut-read)─► cluster_store FULL
                       ├─► instance_compaction decode     │
                       └─► ssao_blur edge unpack           │
ViewMut-read (RMW) ────┬─► cluster_store merge ────────────┘
                       ├─► gpu_culling OR merge ─(+ atomics)─► gpu_culling FULL
                       ├─► indirect_args validate ─(+ atomics)─► indirect_args pair FULL
                       └─► fused EMA
groupshared+barrier ──────► instance_compaction rank ─► instance_compaction FULL
pow/exp/log ──────────────► tonemap, fused_post_chain
multi-channel texel ──────► tonemap, ssao, taa, fused
texture sample ───────────► taa_resolve
```

Land **`as`-cast then ViewMut-read** to convert the two Tier-2
main-battleground passes (cluster_store, then gpu_culling with atomics) from
"arith core only" to full rurix_owned — the biggest provenance win per
compiler-feature after the zero-cost luminance_reduction flip.
