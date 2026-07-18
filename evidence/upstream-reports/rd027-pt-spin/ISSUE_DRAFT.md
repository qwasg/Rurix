> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# Upstream issue draft — NVIDIA (ptxas / driver-JIT compiler)

> Public-facing text. Channel: <FILL: filing channel — NVIDIA Developer Forums (CUDA) thread
> and/or the developer bug portal at developer.nvidia.com/bugs — filed under the owner's NVIDIA
> developer account>. Complete the `<FILL: …>` placeholders and the pre-filing checklist at the
> bottom before submitting. Companion files in this pack: `repro_log_20260718.md` (verbatim
> commands + outputs), `ptx_excerpt.md` (PTX loop-skeleton excerpt). The full 3838-line `.ptx`
> and the O0–O3 SASS dumps live in the workspace (`build/spike-rd027/`, regeneration commands
> in `PROVENANCE.md`) and should be attached in full at file time.

---

## Title

ptxas -O1 and above (and the driver JIT) produce SASS that deadlocks at BSYNC on valid PTX; -O0 is correct

---

## Environment

- GPU: NVIDIA GeForce RTX 4070 Ti (sm_89, compute capability 8.9), WDDM driver model, HAGS on, TDR at OS defaults
- Driver: 620.02 (NVML 13.620.02; CUDA driver API version 13.2) — <FILL: confirm installed driver package branch (Game Ready / Studio) at file time>
- CUDA Toolkit (AOT leg): 13.3 — ptxas `Cuda compilation tools, release 13.3, V13.3.33`; nvdisasm and compute-sanitizer from the same toolkit
- OS: Windows 11, build 10.0.28120
- PTX producer: LLVM NVPTX back end (clang 22.1.x) driven by the Rurix compiler (`rurixc`); PTX ISA 8.8 on the AOT leg, 7.8 on the JIT-only leg (same code; the diff is the 4-line version header)

## Expected vs actual

The kernel is a single `.visible .entry rx_pt_render_176` — a path-tracing render kernel with
three nested divergent loops (samples-per-pixel × bounces × grid-DDA march) plus inner
cell/particle loops and multi-level breaks. All ~20 loops are compile-time bounded, statically
reducible, with unique counter def-chains and zero spills at both the PTX and SASS level
(static audit in the repro log; loop skeleton in `ptx_excerpt.md`).

- **Expected:** bounded work, seconds-scale completion. The *same PTX* assembled with
  `ptxas -O0` completes the failing configuration in **0.66 s** and the full production
  configuration (256 spp / 4 bounces / 1280×720) in **9.49 s/frame**.
- **Actual (ptxas -O1/-O2/-O3, and driver JIT):** no completion. GPU pinned at
  **100% utilization / ~63 W (far below compute-load power) / SM sustained at max boost
  2745 MHz**, flat for the entire observation window; zero frames of progress (>15 min
  observed before a watchdog was adopted; campaign runs killed by a 120 s watchdog,
  exit 124). The signature is consistent with warps parked at a `BSYNC` barrier
  (no issue, no memory traffic), not with forward compute.

## Discrimination matrix (summary)

All runs under a process-tree watchdog; classification by exit code. Verbatim commands and
outputs: `repro_log_20260718.md`.

| Leg | Result |
|---|---|
| ptxas **-O0** (AOT cubin) | **completes** — failing config 0.66 s; full production config 9.49 s/frame |
| ptxas **-O1** (AOT cubin) | **hangs** (watchdog kill at 120 s) |
| ptxas **-O2 / -O3** = default (AOT cubin) | **hangs**; -O2 and -O3 SASS are byte-identical |
| **driver JIT** (PTX-only embed, driver 620.02) | **hangs** |
| all source loops hard-capped (DDA≤1000 / cell≤4096 / bounce≤8 / spp≤64), default O3 | **still hangs** — the spin is not source-loop iteration |
| compute-sanitizer memcheck, green control | completes, `ERROR SUMMARY: 0 errors` |
| compute-sanitizer memcheck, failing config | still hangs (300 s watchdog) |
| code-deletion ladder | **non-monotonic**: d1–d4 green, d5/d6 hang, d7 green; sub-ladder d6a hang / d6b green / d6c hang / d6d green — code-shape-sensitive, not a single source construct |
| single-artifact check | one byte-identical PTX (sha256 `85d597dd…`) for all five run configs; hang/no-hang is selected purely by runtime kernel parameters + data |

The trigger is also data-dependent: the initial particle layout does not trigger; the particle
distribution after 4 simulation substeps does (SUBSTEPS=0 probe completes).

## Key SASS evidence

The behavioral boundary (green at O0 | hang at O1+) coincides exactly with a latch-exit
protocol restructuring that ptxas introduces at -O1 (line numbers are 1-based file lines of
the nvdisasm `-c` dumps; regeneration commands below).

**Hanging levels (O1; pattern identical at O3): count-exit rewritten as an unaccounted
`CALL.REL.NOINC` edge.** Main DDA loop, `aot_O1.sass` lines 2941–2948 (@0x3630–0x3680):

```
.L_x_225: BSYNC B1
.L_x_223: IADD3 R48, R48, 0x1, RZ
          ISETP.NE.U32.AND P0, PT, R48, UR5, PT
     @!P0 CALL.REL.NOINC `(.L_x_222)      ; count exit: no BREAK, call-as-branch
          BRA `(.L_x_227)                 ; back edge
.L_x_222: BSYNC B7                        ; loop-scope barrier
```

The same loop's *spatial* exits stay properly accounted (`@!P0 BREAK B1; @!P0 BRA .L_x_222`,
e.g. lines 2915–2916) — two exit protocols coexist on one loop. Four such
`@!P0 CALL.REL.NOINC` latch exits appear at O1 and persist 1:1 at O3 (O1 lines
2945 / 3850 / 5306 / 5625 = main DDA / shadow DDA-1 / shadow DDA-2 / spp; O3 lines
3664 / 4576 / 6049 / 6377). O0 has **zero** local-label `CALL.REL.NOINC`.

**Green level (O0), same DDA loop — every exit BREAK-accounted.** `aot_O0.sass` lines
6534–6540 (@0x6aa0), spatial exit same protocol at 6922–6926, converging at 6961
(`.L_x_314: BSYNC B7`):

```
.L_x_343: IADD3 R104, R104, 0x1, RZ
          ISETP.EQ.U32.AND P0, PT, R104, R62
      @P0 BREAK B1                        ; count exit = BREAK accounting
      @P0 BRA `(.L_x_314)                 ; plain branch -> BSYNC B7 (line 6961)
```

Supporting observations (full detail in the analysis referenced by the repro log):

- `CALL.REL.NOINC` used as a loop exit is a predicated control-flow edge **invisible to
  reconvergence accounting**; its legality rests on a static "exit predicate is uniform
  across the remaining barrier participants" assumption with no hardware backstop.
- Race window: at sites 2 and 3 the exit continuation reaches a `BSSY B5` within 2–3
  instructions (O1 3845–3855 / 5301–5310) while the abandoned loop body re-arms
  B7/B6/B5/B8 every iteration — a non-uniform trigger splits the warp without accounting
  and both halves re-arm the *same physical barrier id* concurrently. Barrier-id
  allocation is **9/9, zero headroom** (B0–B8 all in use, dynamic peak 9 concurrently live).
- Counter-example in the same binary: site 4 (spp latch) computes its exit on the uniform
  datapath (`UIADD3 UR4 / UISETP UP2`, O1 5617–5626) — ptxas *can* build a structurally
  safe exit; sites 1–3 instead rely on the static uniformity assumption.
- `cmp` proves `aot_O2.sass` ≡ `aot_O3.sass` byte-for-byte; registers 141 (O0) vs 77
  (O1 = O2 = O3). The behavior boundary and the protocol boundary coincide at O0→O1.
- The loop-capped variant's SASS retains all 4 CALL exits (`e7b_O1.sass`
  2969 / 3891 / 5352 / 5670) — consistent with "capping does not fix the hang".
- Once a barrier participation mask is corrupted, any downstream `BSYNC` waits forever —
  matching the observed 100%-util / low-power / max-clock signature.
- Honest residual: 10 unrolled sqrt loops (constant bound 30, step −5, equality exit)
  were not in the cap set; a counter-wraparound spin confined to them is not fully
  excluded by the cap experiment, but O0 runs the same equality-exit sqrt loops green,
  so the corruption source would still be an O1+-introduced transform. A cuda-gdb warp-PC
  sample at hang would discriminate (see checklist).

## Reproduction

**PTX level (assembler only — the transform is visible without a GPU):**

```
ptxas -arch=sm_89 -O0 ctrl_b2.ptx -o O0.cubin   # nvdisasm -c: 0 local-label CALL.REL.NOINC; all count exits BREAK-accounted
ptxas -arch=sm_89 -O1 ctrl_b2.ptx -o O1.cubin   # 4 x "@!P0 CALL.REL.NOINC" latch exits appear (lines above)
ptxas -arch=sm_89 -O2 ctrl_b2.ptx -o O2.cubin   # SASS byte-identical to -O3
ptxas -arch=sm_89 -O3 ctrl_b2.ptx -o O3.cubin   # default level of the production build; hangs on GPU
```

**GPU level:** with the same PTX and the same runtime arguments, the hang/no-hang boundary is
exactly O0 | O1; the driver-JIT path (embedding PTX only, no ptxas) hangs identically —
i.e. both NVIDIA optimizing back ends apply the same class of transform.

`ctrl_b2.ptx` (3838 lines, single entry, PTX ISA 8.8) is generated from the open-source Rurix
repository (https://github.com/qwasg/Rurix); a minimized 276-line kernel source variant with
all loops bounded is in-repo at `spike/rd027-pt-poison/mrp/render_pt_mrp_d6a.rx`, and
`spike/rd027-pt-poison/mrp/README.md` gives the exact build/run recipe. The full app-level
reproduction needs the repository plus its simulated particle data (the trigger is
data-dependent, see matrix); a fully self-contained synthetic-data MRP can be prepared on
request. Attach at file time: <FILL: repro package upload — full ctrl_b2.ptx + O0–O3
cubin/SASS dumps + minimized kernel source + repro_log_20260718.md>.

## Workaround

Pinning `ptxas -O0` on the AOT-cubin path completes correctly (failing config 0.66 s; full
production config 9.49 s/frame, previously unmeasurable at >15 min hang). No equivalent
optimization-level control exists for the driver-JIT path, so PTX-only deployments remain
exposed.

---

## Pre-filing checklist (owner — remove this whole section before posting)

1. **Optional strengthening — E6 independent-frontend control:** build a semantically
   equivalent kernel via nvcc / clang-CUDA and check whether -O1+ applies the same
   CALL.REL.NOINC latch protocol (pre-empts "LLVM-NVPTX-specific PTX quirk" triage).
2. **Optional strengthening — cuda-gdb warp-parking capture:** attach at hang and sample
   warp PCs. Predicted parking points (aot_O1 addresses): 0x3680 (BSYNC B7), 0x6800 /
   0xb8e0 (BSYNC B6), 0x6a10 (BSYNC B4), 0x36b0 (BSYNC B3), 0xca70 (BSYNC B0). PC at a
   BSYNC → confirms the reconvergence-accounting deadlock; PC inside a sqrt body with a
   wrapped counter → the residual sqrt-wraparound alternative.
3. **Sensitive-info scrub** of all attachments: local paths (`H:\rurix\…`), machine/user
   names, and any non-public data in logs.
4. **Package the repro zip** (PTX + cubins + SASS + minimized source + repro log); check the
   channel's attachment size limits; <FILL: attach the channel-required system dump
   (nvidia-bug-report / DxDiag) if the template asks for one>.
5. **Duplicate search at file time** on the forum / bug portal (keywords: `CALL.REL.NOINC`,
   `BSYNC` hang, ptxas -O0 workaround, sm_89 deadlock).
6. **File personally** under the owner's NVIDIA developer account — agents do not file
   externally (EA1 G-EA1-7 precedent).
