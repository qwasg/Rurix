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

### 1.1a Native Godot buffer mapping (integration — errata)

The abstract `current`/`prev` slots above map onto Godot's native luminance
buffers **the way native `luminance.cpp` uses them**, not 1:1 by name. Native's
final WRITE dispatch (`i == reduce.size()-1 && !p_set`) **writes** `reduce[last]`
and **reads** `current` as the previous-frame luminance, then swaps
`SWAP(current, reduce[last])`. Therefore the Godot patch call site fills:

| contract slot | filled with native buffer | native role                     |
| ------------- | ------------------------- | ------------------------------- |
| `current` (final UAV write target) | `reduce[reduce.size()-1]` (`reduce[last]`) | the 1×1 buffer the WRITE kernel writes |
| `prev` (final SRV read)            | `current`                                  | last frame's 1×1 luminance the EMA blends against |

The intermediate `reduce[0..L-1]` slots are native `reduce[0..reduce.size()-2]`
(every native reduce buffer except the last). After a recorded pyramid the call
site performs the same `SWAP(current, reduce[last])` as native, so the
double-buffer advances identically. (Filling `current` with native `current` and
`prev` with native `reduce[last]` — the mirror image — is the historical Bug 2:
it makes the WRITE kernel read the wrong prev and write the wrong buffer.)

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
Godot cascade **byte-for-byte** — reduce by 8×8 tiles until the destination is
1×1 — using Godot's own **floor** rule `MAX(dim / 8, 1)` (verbatim from
`Luminance::LuminanceBuffers::configure` and `Luminance::luminance_reduction` in
`servers/rendering/renderer_rd/effects/luminance.cpp`):

```
(1920,1080) → (240,135) → (30,16) → (3,2) → (1,1)     4 dispatches
(256,144)   → (32,18)   → (4,2)   → (1,1)             3 dispatches
(16,16)     → (2,2)     → (1,1)                        2 dispatches
(8,8)       → (1,1)                                    1 dispatch  (final only)
```

Because the `reduce[]` slots the patch hands the bridge **are** Godot's native
luminance buffers (resolved via `get_driver_resource`), the planner must report
exactly the extents Godot allocated them at — the native floor extents above.

### 2.1 Floor buffers, ceil dispatch, native edge-drop (errata)

Godot allocates each `reduce[i]` at the **floor** extent `MAX(src/8, 1)` but
dispatches `ceil(src/8)` thread groups (native `compute_list_dispatch_threads`
divides the source extent by the 8×8 group with ceil). When `floor != ceil`
(the source is not a multiple of 8) the trailing partial 8×8 tile's destination
texel lands **out of bounds** of the floor-sized buffer and its write is dropped
by the hardware — so native silently discards that edge tile. The tracked reduce
kernel computes its write extent internally as ceil and guards
`x < dst_width && y < dst_height`, so writing into the same floor-sized buffer
drops the same partial texel; the in-bounds result is **bit-exact** with native.
(Following the kernel's ceil extent for the `reduce[]` chain — the historical
Bug 3 — over-sizes each buffer and reads past the previous level's floor buffer,
corrupting every downstream average.)

Thread groups per level: `(ceil(dst_width/8), ceil(dst_height/8), 1)` over the
floor destination — exactly enough `[numthreads(8,8,1)]` threads (one per dest
texel) to cover every in-bounds floor texel; the kernel's own ceil guard drops
the partial edge tile, matching native's ceil-dispatch + floor-buffer edge-drop.

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

## 7. `p_set` first-frame semantics (errata — native-delegated SET)

Godot's `p_set` (the call site's `set_immediate`) marks the frame that must
**SET** the exposure directly instead of blending: native's
`luminance_reduction` then uses the plain `LUMINANCE_REDUCE` kernel for the final
level too (no `WRITE_LUMINANCE`), writing the raw clamp-free mean straight into
`reduce[last]`, then `SWAP`s it into `current`. The fixed `WRITE_LUMINANCE`
kernel has **no** SET branch, and expressing SET as a zero-cleared `prev` is
wrong: the EMA `prev + (cur - prev) * exposure_adjust` with `prev == 0`
collapses to `cur * exposure_adjust` (≈`cur * 0.008/frame`), which starts near
zero and needs dozens of frames to converge — a large, visible exposure error
(historical Bug 1).

The contract therefore delegates the SET frame to native: **on any frame where
`set_immediate == p_set` is true the call site skips the whole pyramid arm** and
lets the native Godot `luminance_reduction` perform the SET + SWAP. The real
pyramid engages from the **next** frame onward, when `prev == current(previous
frame)` is a valid non-zero luminance. No `prev` texture is ever fabricated.

Subsequent (non-SET) frames use `prev == current(previous frame)`, double-
buffered by the call site's `SWAP(current, reduce[last])` each frame, mirroring
Godot's `SWAP`. The CPU parity reference
(`math_parity_evidence.json → ema_sequence`) models the steady-state kernel EMA;
the first-frame SET value is produced by the native path in both the reference
and candidate legs, so it never diverges.

---

## 8. Shim ABI

The multi-level path is shim ABI **2**
(`rxgd_luminance_record_shim_abi_version() == 2`), consistent on the C++ and
Rust sides. The public C ABI (`RXGD_ABI_VERSION == 1`, `rxgd_record_pass`
signature, `RxGdResource`/`RxGdCaps` layout) is **unchanged**: the multi-
resource generalization rides the existing `const RxGdResource*`,
`resource_count` array parameter of `rxgd_record_pass`.
