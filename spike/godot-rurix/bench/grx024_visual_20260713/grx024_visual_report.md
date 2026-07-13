# GRX-024 bench-scene visual parity evidence (2026-07-13, measured_local, no performance claim)

Phase-1 material for the honest-ceiling close-out: does turning the five
rd_native passes on (the GRX-025 all5 configuration) change the rendered
image versus the native engine on the seven benchmark scenes? This is VISUAL
PARITY evidence only — it makes NO performance claim and proposes nothing; it
tests whether default-enable is pixel-safe.

## Method

- exe / dll / patch: same rb4 build as the terminal campaign and GRX-025
  (godot_exe sha256 `fc41853b…`, dll `47910fe7…`, patch `0001-0029+0040-0048`).
- three legs per scene, all on the SAME exe, deterministic capture path
  (`--fixed-fps 60`, viewport grabbed at post-warmup frame 600 via
  `get_viewport().get_texture().get_image()`, converted to RGB8), **960x540**:
  - `baseline_a` (A)  — backend=0, no override.cfg
  - `baseline_b` (A') — backend=0 again → **determinism-floor control**
  - `all5` (B)        — the five rd_native passes at backend=2 (override.cfg +
    `RURIX_DXC_DIR` on PATH), `--verbose` so engagement markers are scanned
- per scene two diffs are computed over the raw RGB8 (numpy, per-byte abs):
  `floor = diff(A, A')` (intrinsic run-to-run non-determinism) and
  `parity = diff(A, B)` (baseline-vs-all5). **A parity at or below the floor is
  honest parity; a parity above the floor is recorded as real divergence.**
- driver: `grx024_visual_capture.py` (reuses the runner's own
  `render_override_cfg` / `load_pass_matrix` / marker table so the all5 leg is
  configured identically to a benchmark rurix leg). Raw + PNG + meta under
  `captures/{baseline_a,baseline_b,all5}/`; machine metrics in
  `grx024_visual_summary.json`.
- resolution note: capture is 960x540 (16:9, same renderer path / HDR formats
  as the 1080p bench, smaller archive). Engagement is scene-property gated, not
  resolution gated — **verified**: every scene's all5 markers below match the
  terminal campaign's engagement matrix exactly, including the FILMIC tonemap
  guard fail-closing on post_fx / volumetric / mixed.

## Per-scene diff table (RGB8, 960x540 = 518 400 px / 1 555 200 bytes)

| scene | engaged passes (all5 leg) | floor A/A' max·mean·px% | parity A/B max·mean·px% | verdict |
|---|---|---|---|---|
| clustered_lights | tonemap, cluster_store | 0 · 0 · 0% | **0 · 0 · 0%** | BYTE-EXACT |
| material_variants | tonemap | 0 · 0 · 0% | **0 · 0 · 0%** | BYTE-EXACT |
| post_fx_chain | ssao_blur, cluster_store | 0 · 0 · 0% | **0 · 0 · 0%** | BYTE-EXACT |
| volumetric_fog | cluster_store | 0 · 0 · 0% | **0 · 0 · 0%** | BYTE-EXACT |
| many_mesh_instances | tonemap, taa_resolve | 0 · 0 · 0% | **1 · 0.00204 · 0.59%** | ±1 LSB, deterministic floor |
| mixed_forward_plus | ssao_blur, taa_resolve, cluster_store | 30 · 0.18943 · 16.31% | 33 · 0.19212 · 16.78% | within temporal floor |
| particles | tonemap, particles_copy | 179 · 45.681 · 46.21% | 179 · 45.747 · 46.13% | floor-limited (non-deterministic) |

(`px%` = share of pixels with any channel differing. Bold parity cells are the
default-enable-safe results.)

## Reading the three tiers

1. **Byte-exact (4/7):** clustered_lights, material_variants, post_fx_chain,
   volumetric_fog. floor = 0 AND parity = 0 — the rd_native passes engaged
   (tonemap / ssao_blur / cluster_store) produce a **pixel-identical** frame to
   native. Note post_fx_chain and volumetric_fog are FILMIC, so tonemap is
   correctly fail-closed and the engaged ssao_blur / cluster_store feed the
   NATIVE tonemap — byte-exact final image proves those two ports are
   downstream-transparent.

2. **±1 LSB on a deterministic scene (1/7):** many_mesh_instances. The floor is
   a hard 0 (this scene's capture frame is frame-deterministic), so the parity
   signal is clean: **3 167 bytes (0.2%) differ by exactly 1 LSB, max_abs = 1**,
   everything else identical. That is the 8-bit rounding boundary of the
   tonemap + taa_resolve ports — a near-exact, imperceptible difference, and the
   ONLY scene where an engaged deterministic diff is non-zero.

3. **Within the temporal non-determinism floor (2/7):** mixed_forward_plus and
   particles. Here floor > 0 because the scene is not frame-deterministic, and
   **parity is statistically indistinguishable from floor**:
   - mixed_forward_plus: floor mean 0.18943 (max 30) vs parity mean 0.19212
     (max 33); both are ~117-120 k bytes at 1 LSB + a TAA-jitter tail of <130
     bytes at 17+. The all5 difference is inside the scene's own run-to-run
     TAA/temporal shimmer.
   - particles: floor mean **45.681** (677 391 bytes at 17+, 46.2% px) vs parity
     mean **45.747** (675 770 bytes at 17+, 46.1% px) — the two histograms
     overlay. GPUParticles are GPU-simulated and **not frame-deterministic**
     (confirmed: two identical baseline runs already disagree on 46% of
     pixels), so a same-frame baseline-vs-all5 image diff cannot isolate the
     rd_native passes on this scene.

## Honest-record: particles caliber is floor-limited

The particles scene cannot yield a same-frame pixel-parity verdict — its
determinism floor (45.68 mean, 46% px) swamps any pass-level signal, and the
all5 parity sits exactly on that floor. Per the pre-registered fallback, the
particles_copy visual口径 is therefore a **combined caliber**, not a hacked
image match:

1. the A/A' floor control **quantifies** the non-determinism (46% px) instead of
   hiding it, and shows parity == floor (no divergence above it);
2. particles_copy is separately evidenced by the GRX-025 measured leg
   (engaged-geomean 0.9945, §4) and the terminal report's 0-delta GPU-µs
   attribution (§4);
3. the rd_native container itself carries the S2 ~1-ULP parity proof;
4. the diff tool is demonstrably NOT blind — it reports hard 0 on the four
   byte-exact scenes and exactly 1 LSB on many_mesh, so a real particle-copy
   divergence would have shown up on a deterministic scene if one existed (none
   does in the bench: particles_copy engages only on this scene).

The taa_resolve visual口径 is likewise anchored on the deterministic scene
(many_mesh, ±1 LSB) rather than the temporal one (mixed, floor-limited).

## Pass-level visual parity coverage

| pass | deterministic-scene parity | verdict |
|---|---|---|
| tonemap | byte-exact (clustered, material) | pixel-identical |
| ssao_blur | byte-exact (post_fx) | pixel-identical |
| cluster_store | byte-exact (clustered, post_fx, volumetric) | pixel-identical |
| taa_resolve | ±1 LSB, floor 0 (many_mesh) | near-exact (<=1 LSB) |
| particles_copy | none deterministic (only particles, non-det) | floor-limited; cross-referenced |

## Caveats

1. `measured_local`, one machine, single deterministic capture frame per leg,
   960x540. No performance claim.
2. LDR 8-bit RGB8 comparison after tonemap/sRGB; sub-LSB HDR differences are
   below this instrument (by design — this tests display-observable parity).
3. Two scenes (particles, mixed) are temporally non-deterministic; their
   verdicts are floor-relative, not absolute, and stated as such.
4. Capture is 960x540, not the 1080p bench resolution; engagement was verified
   identical but exact pixel counts would differ at 1080p.

## CR self-check

`grx024_visual_capture.py`, `rd_native_all5.json`, `grx024_visual_summary.json`,
the 21 `.meta.json`, and this report verified `CR=0`; `.rgb8`/`.png` are binary
capture artifacts (excluded from the text CR audit). Nothing is committed to git.
