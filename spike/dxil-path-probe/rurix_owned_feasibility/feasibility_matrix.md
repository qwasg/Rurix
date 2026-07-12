# rurix_owned Feasibility Matrix — GRX passes after MR-0006/0007

Scope: can each GRX offline pass retire its `hlsl_bridge_workaround` and compile
its kernel natively through `rurixc --target dxil` (rurixc → patched `llc`
`-filetype=obj` → `dxv` validator), now that MR-0006 (u32/i32 buffer views,
`& | ^ << >>`, `find_lsb/find_msb/popcount`) and MR-0007 (`sqrt/rsqrt/sin/cos`)
have landed (cc059da).

Everything below is **measured**, not estimated. Toolchain:
`rurixc` release `dxil-backend shader-stages` · `llc` =
`H:/llvm-clean-82c5bce5-build/bin/llc.exe` · `dxv` =
`H:/dxc-round7/extracted/bin/x64`. Evidence:
`capability_probe_evidence.json`, `pass_direct_compile_evidence.json`,
`evidence/*.dxil` (all `DXBC`-magic, dxv-validated), `evidence/*.stderr.txt`.

---

## 1. Verdict summary

| Pass | Verdict | Native-today artifact (measured) | Residual blocker for full pass |
|------|---------|----------------------------------|--------------------------------|
| **luminance_reduction** | ✅ **FULL rurix_owned feasible NOW** | `src/lib.rx` **and** `src/lib_texture.rx` both dxv-PASS | none |
| cluster_store | 🟡 partial | arith core (popcount + z-decode + pack, write-only) dxv-PASS | u32→usize **cast** for scan-derived gather/scatter index; **ViewMut read** for min/max merge |
| instance_compaction | 🟡 partial | D3 scatter move (precomputed rank) dxv-PASS | **groupshared+barrier** prefix-scan (D1/D2); u32 **cast** for bitmask decode |
| particles_copy | 🟡 partial | ALIGN_DISABLED f32 lane dxv-PASS (param reorder) | ALIGN_BILLBOARD flags-bit test + aggregate write; billboard math itself now unblocked by MR-0007 |
| indirect_args | 🟡 partial | WRITE kernel dxv-PASS (cap-as-view adaptation) | resident VALIDATION kernel needs **atomics** + **ViewMut read** |
| tonemap | ❌ bridge stays | — | **pow** (no transcendental beyond sqrt/rsqrt/sin/cos) + float4 texel |
| ssao_blur | ❌ bridge stays | — | f32↔u32 **cast** (edge unpack) + float2 texel |
| taa_resolve | ❌ bridge stays | — | texture **SAMPLE** (Catmull-Rom) + 2D dispatch + groupshared + multi-channel |
| gpu_culling | ❌ bridge stays | — | **atomics** (InterlockedAdd/Or) + **ViewMut read** + u32 **cast** |
| fused_post_chain | ❌ bridge stays | — | **pow** + float4 + 2D dispatch + **ViewMut read** (EMA) |

**1 full · 4 partial · 5 bridge-stays.** MR-0006/0007 delivered exactly the
*operator* surface the pass docs named as the cluster_store/culling blockers
(u32 views, bitops, findLSB/MSB) — but three gaps the docs did not name
(`as` casts, ViewMut read/RMW, atomics) keep the Tier-2 buffer passes' *data
movement* on the bridge even though their *arithmetic cores* are now native.

---

## 2. Measured capability boundary (post MR-0006/0007, this toolchain)

### Supported (dxv-verified)
| Capability | Witness probe | Notes |
|---|---|---|
| `View/ViewMut<global, f32\|u32\|i32>` | luminance_reduction, cluster_store_arith_core | u32/i32 = MR-0006 |
| `Texture2D<f32>`/`RWTexture2D<f32>` texel **load/store** | `probe_texture_loadstore_2d` (2604 B) | **single-channel f32; NO sample.** Settles the GRX-009 llc blocker — this `llc` build DOES lower `load/store.texture.2d` |
| `& \| ^ << >>` (same-width int), shift auto-mask | cluster_store_arith_core, probe_xor_clear_bit | MR-0006 |
| `find_lsb/find_msb/popcount` (u32 → **u32**) | cluster_store_arith_core (2808/3092 B) | MR-0006 |
| `sqrt/rsqrt/sin/cos` (f32) | conformance `math_*` accept | MR-0007 |
| value-if scalar select `let x = if c {a} else {b}` | cluster_store_arith_core | both arms values; **operands must be same source-type** (§2 note) |
| no-else statement-if · nested `while` · mutable local | luminance_reduction | |
| integer `%` · `+ - * /` | luminance_reduction | |
| ThreadCtx<1> `global_id()` (usize) | all passing probes | |
| scalar root-constants (usize/u32/i32/f32) | luminance_reduction | **i64/usize must sit on even dword offset** (see particles) |

### NOT supported (measured reject — each is a distinct pass blocker)
| Gap | RX code | Witness | Passes it blocks |
|---|---|---|---|
| `as` cast (u32↔usize, f32↔u32) | RX6007 | `probe_cast_f32_u32`, `probe_u32_bitmask_decode`, `cluster_store_native_probe` | cluster_store, gpu_culling, instance_compaction, ssao_blur |
| **ViewMut read (read-modify-write on UAV)** | RX6007 | `probe_viewmut_read_rmw` | cluster_store, gpu_culling, indirect_args (validate), fused_post_chain |
| atomics (AtomicView/fetch_add) | RX6001 | `probe_atomic` | gpu_culling, indirect_args (validate) |
| groupshared `shared let` + `block.sync()` | RX6007 | `probe_groupshared_barrier` | instance_compaction, taa_resolve |
| unary bitwise NOT `!x` | RX6007 | `probe_unary_not` | cluster_store (`~(1<<b)` → use `^ 0xFFFFFFFF`) |
| buffer index must be `usize`; find_lsb/etc return u32 | RX2001 | inline probe | forces the cast gap above for every bitmask/bit-scan-indexed pass |
| value-if: view-u32 vs **scalar-param-u32** compare | RX6007 | `indirect_args_write_kernel` note | any `min(x, scalar_cap)` — workaround: pass cap as a u32 view |
| 2D dispatch (ThreadCtx<2> / global_id_2d) | RX2004 | direct taa/fused compile | taa_resolve, fused_post_chain (linearize with 1D + div/mod) |
| else / else-if / break / continue / for | RX6007 | conformance reject corpus | rewrite to while + no-else if + value-if |
| f32 `%` modulo | RX6007 | conformance `f32_modulo` reject | — |
| transcendental beyond sqrt/rsqrt/sin/cos (pow/exp/log) | RX6006/RX1001 | direct tonemap compile | tonemap, fused_post_chain |
| aggregate element types (struct/vec4/mat4 views) | RX6007 | direct particles/instance compile | all SSBO-struct passes (scalar-decomposable **iff** casts existed) |
| texture SAMPLE (bilinear/Catmull-Rom/gather) | not a lang item | — | taa_resolve, (tonemap/fused if they sampled) |
| multi-channel texel (float4/float2) | single-channel f32 only | — | tonemap, ssao_blur, taa_resolve, fused_post_chain |

---

## 3. Operation × pass matrix

Legend: ✅ used & supported · ⛔ used & **blocked** (gap named) · — not used

| Operation dimension | lumin | tonemap | ssao | taa | particles | cluster | culling | compact | indirect | fused |
|---|---|---|---|---|---|---|---|---|---|---|
| f32 buffer view load/store | ✅ | ✅ | ✅ | — | ✅ | — | ✅ | ✅ | — | ✅ |
| u32/i32 buffer view | — | — | — | — | (flags) | ✅ | ✅ | ✅ | ✅ | — |
| bitops `& \| ^ << >>` | — | — | ⛔cast-adj | — | (flags) | ✅ | ✅ | ✅ | ✅ | — |
| find_lsb/find_msb/popcount | — | — | — | — | — | ✅ | ✅ | ✅(scan) | — | — |
| **u32→usize cast for index** | — | — | ⛔ | — | — | ⛔ | ⛔ | ⛔ | — | — |
| **ViewMut read (RMW/merge)** | — | — | — | — | — | ⛔ | ⛔ | — | ⛔(validate) | ⛔(EMA) |
| atomics | — | — | — | — | — | — | ⛔ | — | ⛔(validate) | — |
| groupshared + barrier | — | — | — | ⛔ | — | — | — | ⛔ | — | (recompute) |
| sqrt/rsqrt/sin/cos | — | — | — | ✅* | ✅(billboard) | — | ✅(sqrt) | — | — | — |
| **pow / exp / log** | — | ⛔ | — | ✅* | — | — | — | — | — | ⛔ |
| texture load/store (1-ch f32) | ✅(tex var) | ⛔4ch | ⛔2ch | ⛔ | — | — | — | — | — | ⛔4ch |
| **texture SAMPLE** | — | — | — | ⛔ | — | — | — | — | — | — |
| 2D dispatch (ThreadCtx<2>) | — | — | — | ⛔ | — | (linearize) | — | — | — | ⛔ |
| aggregate struct/vec4/mat4 | — | — | — | ✅* | ⛔→decomp | ⛔→decomp | ⛔→decomp | ⛔→decomp | — | — |
| value-if / no-else if / while | ✅ | ✅ | ✅ | — | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

`*` taa's math is fully elided in `src/lib.rx`; the real arithmetic lives in the
HLSL bridge and needs texture sample + groupshared regardless.

---

## 4. Per-pass detail (blocker attribution)

**luminance_reduction — ✅ FULL.** `src/lib.rx` (raw-buffer, `View<global,f32>`)
and `src/lib_texture.rx` (single-channel `Texture2D<f32>` texel load/store) both
compile to dxv-validated containers **unmodified** (3984 B / 3684 B). Uses only
f32 arithmetic, integer `%`, value-if, no-else if, nested `while`. No cast, no
RMW, no atomic, no aggregate, no sample, no 2D. The pass's own
`texture_artifact_provenance_policy.json` names
`dxv_validation_pass_on_rurix_owned_container` as revert-condition #1 — now
**met**; and `probe_texture_loadstore_2d` proves the "patched llc supporting
`llvm.dx.resource.load.texture.2d`" prerequisite it cited is satisfied by this
`llc`. → migrate first (see migration plan).

**cluster_store — 🟡 partial.** Doc-only `lib.rx` (no kernel). MR-0006 delivered
the exact operators the pass doc listed as blocked (u32 views + bitops +
findLSB/MSB); the write-only arith core (`cluster_store_arith_core.rx`:
popcount + z-range decode + packed min/max via value-if + XOR bit-walk) now
dxv-PASSES (3092 B). **But** the full "Pack 3D Cluster Elements" kernel gathers
`render_elements[index]` and scatters `cluster_store[dst_offset+i]` at
**scan-derived** indices (`index = offset*32 + findLSB(bits)`), which needs a
u32→usize cast (⛔), and its min/max merge **reads** `cluster_store[slice_ofs]`
(ViewMut read ⛔). Scalar-decomposition of the 80 B `RenderElementData` is fine
(stride-20 u32 view, 4 leading fields) — but only *if* the cast existed. `~` is
substituted by `^ 0xFFFFFFFF` (verified). 2D→1D linearizes cleanly.

**instance_compaction — 🟡 partial.** D3 scatter move
(`instance_compaction_scatter_lane.rx`, 3892 B) dxv-PASSES: bit-preserving
12-lane transform move given a precomputed `rank`. The rank itself (D1 local
Hillis-Steele scan + D2 group scan) needs **groupshared + barrier** (⛔, the
Rurix `shared let`/`block.sync()` lang items lower only on NVPTX) and the
bitmask decode needs the u32 **cast** (⛔).

**particles_copy — 🟡 partial.** ALIGN_DISABLED f32 lane
(`particles_copy_align_disabled_lane.rx`) compiles once the two `usize` params
are placed on adjacent (even) dword offsets — the raw `View<f32>` in `src/lib.rx`
also omitted the `global` address space. Verified passing (3032 B reordered
form). ALIGN_BILLBOARD's transcendentals (sin/cos Rodrigues, sqrt/rsqrt
normalize) are **now available** post-MR-0007 — the pass doc's blocker (2) is
**stale**; the residual is the `flags` active-bit test (bind flags as a u32 view)
and the transposed aggregate write (scalar-decompose). Plausible but unverified.

**indirect_args — 🟡 partial.** Doc-only `lib.rx`. The WRITE kernel
(`indirect_args_write_kernel.rx`, 3436 B) dxv-PASSES: `min(survivors, cap)` via
value-if + 5-dword template backfill, write-only. **Measured adaptation:** `cap`
must be read from a u32 **view**, not a scalar u32 param (view-u32 vs
scalar-u32 compare rejects). The resident VALIDATION kernel (contract §5.4)
**reads** the command buffer to compare (ViewMut read ⛔) and accumulates
mismatch counts via InterlockedAdd (atomics ⛔) — so the pair as-shipped stays
bridge.

**tonemap — ❌.** `linear_to_srgb` needs `pow(c, 1/2.4)` → RX1001 `powf` /
RX6006 pow; no exp/log substitute in the f32 math set. float4 RGBA texel is also
multi-channel. Two independent deep gaps.

**ssao_blur — ❌.** Edge unpack `(packed_edges*255.5) as u32` → cast ⛔ (RX6007),
independent of the float2 RG texel. Even the pure-arithmetic lane is blocked.

**taa_resolve — ❌.** `global_id_2d` → RX2004 (2D dispatch); the real resolve
needs Catmull-Rom **texture sample** (no lang item), a groupshared LDS tile, and
float4/float2 multi-channel. `src/lib.rx` elides all of it. Deepest of the set.

**gpu_culling — ❌.** Doc-only `lib.rx`. The f32 distance math (world-center
transform, Frobenius `sqrt`, plane distances) is now expressible — but **every
output** is a u32 **atomic** write (InterlockedAdd count / InterlockedOr
bitmask). A per-thread-slot redesign removes the atomics yet still needs the u32
**cast** (`instance = word*32+bit` index) and the **ViewMut read** for the OR
merge. Three gaps.

**fused_post_chain — ❌.** `global_id_2d` (2D) + `pow` (segment-B sRGB) + float4
texel + EMA **reads** `prev_luminance` (ViewMut read / or a second SRV). Segment
A is luminance_reduction (native), but the fusion point re-imports tonemap's pow.

---

## 5. Bridge-retirement benefit & workload

**What `rurix_owned=true` buys.** Today the canonical package for each pass is a
DXC-compiled HLSL container (`provenance: hlsl_bridge_workaround`,
`rurix_owned: false`, only the RTS0 root signature is Rurix-owned). Flipping a
pass to rurix_owned means the DXIL container is emitted by `rurixc` from Rurix
`.rx` source — the whole compile chain is Rurix-owned end to end (external tools
reduce to `llc` as backend + `dxv` as validator, same as conformance). It
removes the "the shader is actually HLSL" asterisk on any real_gpu_pass claim
and lets the pass drop its `compile_hlsl_bridge.py` + HLSL artifact + the
bridge-digest bookkeeping.

**Workload (measured basis):**

| Pass | Effort | Work |
|---|---|---|
| luminance_reduction | **XS** | source already compiles unmodified; wire `compile_offline.py` to emit the rurixc container as canonical, flip provenance, re-run math-parity + D3D12 dispatch on the rurix_owned container (revert-condition #2), retire HLSL. |
| particles_copy (ALIGN_DISABLED only) | S | reorder usize params, add `global`, scalar-decompose write; but the pass needs BOTH align modes → not a full retire until billboard verified. |
| indirect_args (WRITE only) | S | cap-as-view adaptation; **cannot** retire the pair while the resident VALIDATION leg needs atomics. |
| cluster_store / gpu_culling / instance_compaction | **L, compiler-gated** | blocked on new rurixc features (cast, ViewMut-read, atomics, groupshared), NOT on pass authoring. |
| tonemap / ssao_blur / taa_resolve / fused_post_chain | **L, compiler-gated** | blocked on pow/multi-channel-texel/sample. |

**Highest-leverage compiler features** (by passes unblocked), i.e. the Tier-2
main-battleground levers the task flags:
1. **`as` cast, esp. u32→usize** — unblocks cluster_store gather/scatter,
   gpu_culling index, instance_compaction bitmask decode, ssao edge unpack (4).
2. **ViewMut read (non-atomic RMW)** — cluster_store merge, gpu_culling OR,
   indirect_args validate, fused EMA (4).
3. **atomics** — gpu_culling, indirect_args validate (2).
4. **groupshared + barrier on DXIL** — instance_compaction rank, taa tile (2).

Cast + ViewMut-read together convert cluster_store from "arith core only" to a
plausible full native pass (still no atomics needed — threads own disjoint
clusters, confirmed in the bridge kernel), making cluster_store the best
next-target once those two land.
