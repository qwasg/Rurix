# GRX-009 Wave 2 — luminance multi-resource hook contract (v2)

Status: **bridge-side contract only** (cargo slice). This document defines the
multi-resource / multi-level hook the Godot runtime patch slice must implement
to drive the real multi-level luminance pyramid through the Rurix bridge. No
patch, probe, or milestone file is changed by the cargo slice that introduces
this contract; the later patch slice copies the shapes defined here verbatim.

It is the reference for:

- the bridge orchestration `LuminanceReductionGate::record_pyramid_attempt`
  (`src/rurix-godot/src/lib.rs`),
- the shim multi-level entry `rxgd_luminance_record_levels`
  (`src/rurix-godot/shim/rxgd_luminance_record.cpp`, shim ABI 2),
- the CPU parity reference (`math_parity_evidence.json`,
  `generate_math_parity_evidence.py`).

No performance / FPS / GPU-timestamp claim is made anywhere in this contract.
The Godot runtime luminance path stays **default disabled** and **fallback
only**; a real pyramid dispatch is armed only by an explicit default-false
opt-in and is linked only under the test-only `d3d12-recording-shim` feature.

---

## 1. Resource array semantics

The hook hands the bridge one ordered `RxGdResource` array. For a pyramid of
`num_levels` dispatches the array is:

```
[ source, reduce[0], reduce[1], …, reduce[L-1], current, prev ]
```

where `L = num_levels - 1` (the number of intermediate reduce targets), so the
array length is exactly `num_levels + 2`.

| index                | role       | extent                         | bound as        |
| -------------------- | ---------- | ------------------------------ | --------------- |
| `0`                  | `source`   | full-res luminance/HDR source  | SRV t0 (level 0)|
| `1 .. L`             | `reduce[i]`| level `i`'s destination extent | UAV then SRV    |
| `L + 1` (`num_levels`)   | `current` | 1×1 (this frame's luminance) | UAV u0 (final) |
| `L + 2` (`num_levels+1`) | `prev`    | 1×1 (previous frame's luminance) | SRV t1 (final) |

`source` already ≤ 8×8 ⇒ `num_levels == 1`, `L == 0`, array = `[source,
current, prev]` (length 3): the single dispatch is the final WRITE_LUMINANCE
level (SRV source → UAV current, + SRV prev).

All entries are **textures** (`RXGD_RESOURCE_TEXTURE`, R32F single-channel for
luminance). A buffer resource never conforms and fails closed (see §5).

### 1.1 Dispatch chain

Dispatch `i` (0-indexed) reads array index `i` and writes array index `i+1`,
except the final dispatch writes `current` and additionally reads `prev`:

```
level 0        : SRV source(0)      → UAV reduce[0](1)                 (reduce kernel)
level i (0<i<L): SRV reduce[i-1](i) → UAV reduce[i](i+1)              (reduce kernel)
level L (final): SRV reduce[L-1](L) → UAV current(L+1), SRV prev(L+2) (write kernel)
```

The bridge orchestration derives the indices as: `srv_index = i`,
`uav_index = i+1` for non-final and `num_levels` for the final level,
`prev_index = num_levels + 1`.

---

## 2. Level chain (planner)

`plan_luminance_pyramid_levels(source_width, source_height)` mirrors the native
Godot cascade — reduce by 8×8 tiles until the destination is 1×1 — using the
**tracked kernel's ceil-div-8** (`(dim > 1) ? (dim + 7) / 8 : 1`, matching
`src/lib_texture.rx` and `artifacts/hlsl_bridge/luminance_reduce_level.hlsl`):

```
(1920,1080) → (240,135) → (30,17) → (4,3) → (1,1)     4 dispatches
(16,16)     → (2,2)     → (1,1)                        2 dispatches
(8,8)       → (1,1)                                    1 dispatch  (final only)
```

### 2.1 Native-vs-kernel dimension divergence (integration note)

Godot's native luminance buffer sizing is floor-based
(`240×135 → 30×16 → 3×2 → 1×1`), while the tracked Rurix kernel is ceil-based
(`240×135 → 30×17 → 4×3 → 1×1`). The bridge planner follows the **kernel** so
the dispatched thread grid and the `reduce[]` extents stay self-consistent. The
patch slice must therefore allocate `reduce[]` targets at the **ceil-div-8
extents this planner reports**, not Godot's native floor extents. Reconciling
the two (or teaching the kernel Godot's floor rule) is an explicit integration
item for the patch slice; it is not silently papered over here.

Thread groups per level: `(ceil(dst_width/8), ceil(dst_height/8), 1)` — one
`[numthreads(8,8,1)]` thread per destination texel, guarded by
`x < dst_width && y < dst_height`.

---

## 3. Root constants (b0, 28 bytes / 7 dwords) — per level

Unchanged from the level-0 layout; carried per dispatch with **that level's
source extent**:

| dwords | field             | type | notes                                   |
| ------ | ----------------- | ---- | --------------------------------------- |
| 0–1    | `source_width`    | i64  | low, high; high must be 0               |
| 2–3    | `source_height`   | i64  | low, high; high must be 0               |
| 4      | `max_luminance`   | f32  | consumed only by the final write kernel |
| 5      | `min_luminance`   | f32  | consumed only by the final write kernel |
| 6      | `exposure_adjust` | f32  | consumed only by the final write kernel |

Reduce levels ignore `max/min/exposure` (per Godot: clamp/exposure belong only
to the final level); the bridge fills them anyway for a uniform 28-byte block.

---

## 4. Kernels

| kernel id | artifact                                             | bindings                     |
| --------- | ---------------------------------------------------- | ---------------------------- |
| reduce    | `artifacts/luminance_reduction.dxil` + `.rts0.bin`   | SRV t0 (src), UAV u0 (dst)   |
| write     | `artifacts/hlsl_bridge/luminance_reduce_level_write_luminance.dxil` + `root_signature_write_luminance.rts0.bin` | SRV t0 (src), SRV t1 (prev), UAV u0 (dst) |

Both kernels' embedded bytes are re-verified against baked SHA-256 digests
before any dispatch. The write kernel applies
`cur = clamp(avg, min, max)` then the EMA `out = prev + (cur - prev) * exposure_adjust`.

---

## 5. Fail-closed rules

The whole pyramid falls back to the native Godot luminance path (no partial
dispatch, no OK) when **any** of the following holds:

1. `level_count == 0`.
2. The resource array length ≠ `level_count + 2`.
3. **Any** array entry has a zero `native_handle` (a missing Godot buffer) —
   any single missing handle fails the WHOLE pyramid closed.
4. **Any** array entry is not a texture (`resource_type != RXGD_RESOURCE_TEXTURE`).
5. The device does not advertise the 64-bit integer shader capability, or the
   device/queue handle is null.
6. The compiled package identity does not match the tracked offline evidence.
7. The `d3d12-recording-shim` feature is not linked (shipping bridge): the
   attempt fails closed with `real_dispatch_path_not_linked`.

Each failure records the first missing prerequisite in the once-per-session
`RXGD_REAL_PASS_BLOCKED` diagnostic and keeps `enabled == false`.

---

## 6. One-frame latency (declared, not hidden)

When the hook records these dispatches from **within** a Godot frame, Godot has
not yet submitted that frame's own rendering to the command queue. A self-queue
dispatch that reads Godot's `internal_texture` (the `source`) therefore reads
the **previous frame's** content, and `prev` is likewise the previous frame's
1×1 luminance.

Because luminance auto-exposure uses **time-domain EMA feedback**
(`out = prev + (cur - prev) * exposure_adjust`), a 1-frame delay is
engineering-defensible — but it is recorded as such, never presented as
same-frame. It is documented here, in the bridge code comments
(`record_pyramid_attempt`, `rxgd_luminance_record_levels`), and in the parity
evidence `semantics.one_frame_latency` field.

**Ordering probe interface for the later enablement smoke:** in test-only
readback mode (`readback = true`) the shim reads back the final level, so an
enablement smoke may fingerprint the dispatch **input** (e.g. sample source
texels) across two frames to demonstrate the 1-frame ordering empirically. No
masking language is permitted in that evidence.

---

## 7. `p_set` first-frame semantics

Mirrors Godot's `p_set` uniform: on the **first** frame there is no previous
luminance to blend. The fixed WRITE_LUMINANCE kernel has no `p_set` branch, so
the hook expresses the first frame by supplying a **zero-cleared `prev`
texture**; the EMA `prev + (cur - prev) * exposure_adjust` then degenerates to
`cur * exposure_adjust`.

Divergence note: Godot's native `p_set == false` path writes `clamp(avg)`
directly (no exposure factor), whereas the fixed kernel with `prev == 0`
produces `clamp(avg) * exposure_adjust`. This differs only on the very first
frame and only when `exposure_adjust != 1`; auto-exposure converges within a
few frames, so the divergence is bounded and is documented rather than hidden.
The CPU parity reference (`math_parity_evidence.json → ema_sequence`) models the
**kernel** behavior (`prev == 0` first frame → `cur * adjust`).

Subsequent frames use `prev == current(previous frame)` (double-buffered by the
hook: `current` and `prev` swap each frame, mirroring Godot's `SWAP`).

---

## 8. Shim ABI

The multi-level path is shim ABI **2**
(`rxgd_luminance_record_shim_abi_version() == 2`), consistent on the C++ and
Rust sides. The public C ABI (`RXGD_ABI_VERSION == 1`, `rxgd_record_pass`
signature, `RxGdResource`/`RxGdCaps` layout) is **unchanged**: the multi-
resource generalization rides the existing `const RxGdResource*`,
`resource_count` array parameter of `rxgd_record_pass`.
