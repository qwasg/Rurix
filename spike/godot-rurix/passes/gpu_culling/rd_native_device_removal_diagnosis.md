# GRX-015 Route B gpu_culling rd_native — device-removal root-cause diagnosis

Status: **root cause convicted (empirical, RTX 4070 Ti, patch stack 0001-0029 +
0040-0048, exe `target/grx/godot-scratch-rb3`).** Conclusion: the current 0046
architecture (a same-frame compute dispatch that touches Godot's live
indirect-MultiMesh command buffer) is **mechanism-infeasible** on Godot 4.7's
D3D12 RenderingDevice draw-graph. The b0 / binding / kernel math is *correct* and
is NOT the cause. No container or kernel change is warranted; the blocker is
architectural.

All experiments were run with the tracked rb3 exe (unchanged) against hand-built
diagnostic projects/containers under the scratchpad. Nothing in the patch stack
or the built exe was modified.

---

## 1. Reproduction and the surfacing signature

Candidate leg (`backend==2`, real staged container) always dies frame 1-2:

```
RXGD_RD_NATIVE_GPU_CULLING active: backend=rd_native, in-frame compute cull engaged ...
ERROR: CreateCommandAllocator failed with error 0x887a0005.   (DXGI_ERROR_DEVICE_REMOVED)
   at: RenderingDeviceDriverD3D12::command_buffer_create (rendering_device_driver_d3d12.cpp:2575)
```

`0x887A0005` surfaces on the **next** GPU API call after the fault (a
`CreateCommandAllocator` for the next frame, or a lazy `CreateGraphicsPipelineState`
when a new material is first drawn). The device was already removed by the *prior*
submitted frame — i.e. the fault is an **asynchronous GPU-side execution fault**,
not a CPU-side API error at the point it is reported.

## 2. What was ruled OUT (static + empirical)

| Hypothesis | Verdict | Evidence |
|---|---|---|
| OOB UAV/SRV write in the kernel | **ruled out** | All buffer sizes are in-bounds: command buffer = `5*surface_count` dwords (`mesh_storage.cpp:1683`), kernel writes `s*5+1` ≤ `5*surface_count-4`; transforms = `instances*12` floats, kernel reads ≤ `instances*12-1`; visibility = `ceil(N/32)` words, kernel writes `< ceil(N/32)`. Moreover Godot binds every storage buffer as a **RAW view with `NumElements=(size+3)/4`** (`rendering_device_driver_d3d12.cpp:3546-3564`), so D3D12 bounds-clamps shader access — an OOB shader access on these views **cannot page-fault**. |
| RAW-vs-structured binding mismatch | **ruled out** | `buffer_probe_addendum.md` proved (real GPU, zero tolerance, stride-80 and stride-48 structs) that DXC bakes the element stride into the DXIL, so a RAW view (`StructureByteStride=0`) is runtime-equivalent to a structured view. |
| b0 packing / 48-byte layout math | **ruled out** | See §3: an **empty kernel** with the *identical* b0 + binding surface still removes the device. The layout is correct; there is no per-field correction to make. |
| Missing draw-graph tracker edge | **ruled out** | Every storage buffer gets a `draw_tracker` (`rendering_device.cpp:1105`); the compute UAV write (writable uniform) and the indirect read (`draw_list_draw_indirect` → `add_draw_list_usage(..., RESOURCE_USAGE_INDIRECT_BUFFER_READ)`, `rendering_device.cpp:5621`) are both tracked on the same tracker. |
| Scale / instance-count dependent | **ruled out** | A **1-instance / 1-surface** scene with the real kernel still removes the device (§3). |
| CPU-side D3D12 misuse (state/barrier/descriptor) | **ruled out** | Re-ran the candidate with `--gpu-validation` (Godot enables the D3D12 debug layer + InfoQueue callback, `main.cpp:1330`, `rendering_context_driver_d3d12.cpp:214/124-139`). The debug-layer callback (`_debug_message_func`) printed **zero** messages before the removal — the crash log is byte-identical to the non-validation run. CPU-side states/barriers/descriptors validate clean. |

## 3. Discriminator matrix (all real-GPU runs, `--gpu-validation`)

Every crashing leg prints the active marker then dies with `0x887a0005`.

| Leg | Kernel body | Clears cmd buf? | Binds cmd buf UAV (u0)? | Instances | Result |
|---|---|---|---|---|---|
| reference (`backend==0`) | native CPU count | no | no | 4096 | **clean** |
| fail_closed (garbage container) | never dispatches | no | no | 4096 | **clean** |
| candidate (real kernel) | writes count via `InterlockedAdd` | yes | yes | 4096 | **device removed** |
| **noop A (empty `main`)** | writes NOTHING | yes | yes | 4096 | **device removed** |
| **noop B (visibility-only write)** | writes only u1, never u0 | yes | yes | 4096 | **device removed** |
| **real, 1 instance** | writes count | yes | yes | 1 | **device removed** |

Discriminator containers were built by reusing the tracked
`gpu_culling_rd_native.rts0.bin` + descriptor layout and swapping only the DXIL
(`generate_rd_container.py --dxil … --rts0 … --layout …`), then pointed at via the
project's `rd_container_path`. The empty-kernel container's reflection still marks
`u0=dst_commands writable=1, u1=dst_visibility writable=1` (identical binding
surface).

**Reading:** the fault is invariant to (a) whether the kernel writes the command
buffer, (b) whether the kernel writes anything at all, and (c) instance/surface
count. The clean legs are exactly those that never touch the command buffer with
compute/clear. The **only** invariant across all crashing legs is:

> the module `RD::buffer_clear`s the MultiMesh command buffer's count dword AND
> binds it as a compute UAV (`u0`) + dispatches, and the same frame the native
> path consumes that same buffer as a `DrawIndexedInstanced` **indirect argument**
> (`render_forward_clustered.cpp:631`, `draw_list_draw_indirect`).

## 3.5 GPU-Based Validation result (instrumented rebuild, 2026-07-13)

Per §7 step 1 an instrumented exe was built (a copy of the rb3 tree,
`target/grx/godot-scratch-gbv-diag`, one-TU rebuild) that adds, right after
`EnableDebugLayer()` in `RenderingContextDriverD3D12::_initialize_debug_layers`
(`rendering_context_driver_d3d12.cpp`), a
`ID3D12Debug1::SetEnableGPUBasedValidation(TRUE)` (gated behind `--gpu-validation`
exactly like the debug layer). The candidate leg (`backend==2`, real staged
container, warm shader cache) was re-run with `--gpu-validation` on the RTX
4070 Ti. This is a diagnostic-only tree/exe — NOT committed, NOT part of the
patch stack.

**GBV enabled cleanly and the fault reproduced identically, but GBV named
nothing.** The exact captured tail
(`target/grx/gbv-diag.candidate_gbv_run.log`):

```
RXGD_GBV_DIAG: GPU-Based Validation ENABLED (ID3D12Debug1::SetEnableGPUBasedValidation(TRUE)).
...
GRXRBCull: scene ready backend=2 instances=4096
RXGD_RD_NATIVE_GPU_CULLING active: backend=rd_native, in-frame compute cull engaged ...
ERROR: Create(Graphics)PipelineState failed with error 0x887a0005.
   at: RenderingDeviceDriverD3D12::render_pipeline_create (rendering_device_driver_d3d12.cpp:5352)
ERROR: CreateCommandAllocator failed with error 0x887a0005.
   at: RenderingDeviceDriverD3D12::command_buffer_create (rendering_device_driver_d3d12.cpp:2575)
```

Findings, in order:

1. **GBV was genuinely active.** `ID3D12Debug1` is available on this
   machine/Agility-SDK; `SetEnableGPUBasedValidation(TRUE)` returned success and
   the confirmation marker printed. The `_debug_message_func` `ID3D12InfoQueue1`
   callback that would forward any GBV message (as an `ERROR`-severity
   `ERR_PRINT`) is the same one registered on the normal validation path.
2. **The removal reproduces bit-for-bit under GBV.** Same `0x887a0005`
   (`DXGI_ERROR_DEVICE_REMOVED`) surfacing on the NEXT GPU API call after the
   faulting frame (a lazy `Create(Graphics)PipelineState` for the first material,
   then the next-frame `CreateCommandAllocator`) — i.e. the fault is in the
   already-submitted frame's GPU execution, not the CPU-side recording. GBV does
   not change the outcome.
3. **GBV emits NO discrete validation message before the removal.** No
   `EXECUTION` / `RESOURCE_MANIPULATION` / state-transition / descriptor message
   is printed — the InfoQueue callback is as silent under GBV as the plain debug
   layer was (§2). The device is torn down before any GBV report surfaces.

**Interpretation.** This CORROBORATES §4: the fault is an *asynchronous GPU-side
execution fault* (an `EXECUTE_INDIRECT` / command-processor read racing a
same-command-list compute/clear write of the indirect-argument buffer without an
adequate sync scope), not a CPU-side-detectable state/barrier/descriptor error.
Neither the plain debug layer nor GBV can name the specific offending access —
the hazard manifests only as the device removal itself. (DRED breadcrumbs +
page-fault data would be the next stronger instrument, but Godot's D3D12 backend
leaves DRED unimplemented — `rendering_device_driver_d3d12.cpp:5640` — so that is
a larger, separate instrumentation task, not pursued here.)

**Consequence for R1 (see §6).** GBV did NOT discriminate R1's open premise
(fault *specific* to Godot's out-of-graph command-buffer state management vs a
*general* RDG gap for "a draw-indirect buffer written by compute and consumed the
same frame"). Because GBV is silent, that question stays open on the evidence and
can only be settled empirically. Per the fix-design fallback logic, R1 (a fully
RDG-owned scratch indirect buffer) therefore remains "worth trying": it removes
the ONE proven invariant (compute/clear touching the *live* MultiMesh command
buffer), and its own device-removal enablement smoke on a rebuilt exe is the
FINAL verdict. If R1 still removes the device, the fault is the general RDG gap
and the fallback is R2 (previous-frame count) or R3 (mechanism-blocked).

## 4. Convicted root cause

**Same-frame dual role of the indirect-MultiMesh command buffer.** Godot's
`multimesh->command_buffer` (`storage_buffer_create(..., STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT)`,
`mesh_storage.cpp:1693`) is designed to be **CPU-written** (`buffer_update` at
`mesh_storage.cpp:2210`) and then consumed by the fixed-function indirect draw.
0046 additionally, within the same frame, (1) `RD::buffer_clear`s its count dword
(a `ClearUnorderedAccessViewUint`, `rendering_device_driver_d3d12.cpp:3861`) and
(2) binds it as a compute UAV and dispatches. The native `draw_list_draw_indirect`
then reads it as `D3D12_RESOURCE_STATE_INDIRECT_ARGUMENT`.

This produces a GPU-side execution fault that the CPU-side debug layer does not
flag (the RDG's transitions validate as correct on the CPU side). The exact GPU
mechanism (an insufficient sync scope for the `EXECUTE_INDIRECT`/command-processor
read against the compute/clear write, versus a state-management conflict between
the RDG's per-frame tracking and the multimesh buffer's normally-stable state)
cannot be pinned further without DRED or GPU-Based Validation — **both require an
instrumented rebuild** (Godot's D3D12 backend leaves DRED unimplemented,
`rendering_device_driver_d3d12.cpp:5640 "TODO: Implement via DRED"`, and never
calls `SetEnableGPUBasedValidation`).

**This corrects/deepens the shim-era attribution.** The 322c1f10 shim run blamed
"the side-channel dispatch being invisible to the graph." This round proves that
making the dispatch a first-class graph citizen does **not** help: the fault is
not (only) graph visibility, it is the command buffer's dual role itself. The
common factor between the shim and rd_native crashes is exactly the `buffer_clear`
+ compute touch of the live indirect-argument buffer.

Corroborating context: every OTHER rd_native pass (tonemap, ssao_blur,
taa_resolve, particles_copy, cluster_store, fused_post_chain) injects a mid-frame
`compute_list_begin`/dispatch and works — none of them touches an
indirect-argument buffer, and gpu_culling is "the FIRST rd_native pass that itself
issues `buffer_clear`". So neither mid-frame compute injection nor buffer_clear in
isolation is the problem; the indirect-argument command buffer is.

## 5. Why "just add a barrier" is not available

`RenderingDevice::barrier()` and `RenderingDevice::full_barrier()` are
**deprecated no-ops** in this Godot (`rendering_device.cpp:6414-6420`, they only
`WARN_PRINT`). All synchronization is owned by the RDG's `ResourceTracker`; there
is no application-level escape hatch to force a stronger barrier from the module.
So the fix cannot be a manual barrier — it must be structural.

## 6. Fix design (all options require a rebuild → main session)

The b0 / kernel / RTS0 / descriptor layout are all **correct** and must not
change (proven: the empty kernel with the same layout still crashes). The blocker
is architectural. In increasing order of invasiveness:

- **R1 — Rurix-owned indirect-args buffer (candidate fix, must be re-validated).**
  Never touch `multimesh->command_buffer` with compute/clear. Instead allocate a
  Rurix-owned scratch indirect buffer (`5*surface_count` dwords, STORAGE +
  DISPATCH_INDIRECT). Each frame: `buffer_copy` the native command template into
  it (a *read* of the command buffer, once/when dirty), `buffer_clear` the count
  dword of the **Rurix** buffer, dispatch the cull into it, and patch the
  `backend==2` draw at `render_forward_clustered.cpp:631` to
  `draw_list_draw_indirect` from the **Rurix** buffer. Rationale: a fully
  RDG-owned buffer has no out-of-graph state management competing with the RDG.
  **Caveat:** if the fault is a general RDG gap for "compute-written draw-indirect
  buffer consumed same-frame" (not specific to the multimesh buffer), R1 will hit
  the same wall — so R1 must be device-removal-re-validated before any claim.
  The cheapest way to settle R1's premise, if a rebuild is done anyway, is a
  module-level A/B that isolates *clear* from *UAV-bind* (both are module-owned
  and cannot be isolated via container/scene from outside).

- **R2 — previous-frame count (safe fallback, adds 1-frame latency).** Compute
  writes the reduced count into a Rurix scratch buffer this frame; read it back
  and apply it to the command buffer via the proven-safe native CPU
  `buffer_update` path (`mesh_storage.cpp:2210`) on the *next* frame. Avoids the
  hazard entirely at the cost of one frame of latency and a transient break of
  strict same-frame picture-preservation.

- **R3 — defer / mark mechanism-blocked (honest status if R1 fails).** Analogous
  to the DXIL A-route "fundamental blocked" precedent (RD-010/D-131). Keep
  `default_enable_state=disabled` (already so), record `real_gpu_pass=false`, and
  file the blocker as "same-frame compute manipulation of a live Godot
  indirect-MultiMesh command buffer is not viable on Godot 4.7 D3D12 RDG; pending
  upstream RDG investigation of compute-written draw-indirect buffers."

The accompanying `patches/0046-*.patch.draft` documents R1 as the patch-header
design revision (no clause/b0/kernel change), with R2/R3 recorded as fallbacks.

## 7. Next steps (for the main session that owns rebuilds)

1. ~~Rebuild an instrumented exe with **GPU-Based Validation**~~ **DONE
   (2026-07-13, see §3.5).** GBV was added + enabled; the fault reproduced
   identically (`0x887a0005`) but GBV named nothing (silent), corroborating §4
   that this is an asynchronous GPU-side execution fault, not a
   CPU-side-detectable state error. GBV did not discriminate R1's premise, so R1
   proceeds and its own smoke is the verdict. (DRED remains a larger unimplemented
   instrument, not pursued.)
2. Implement R1 (Rurix-owned indirect buffer + draw-path patch) and re-run
   `ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py`; the headline pass/fail
   is still "no device removal in any leg".
3. If R1 still removes the device, fall back to R2, else conclude R3 and file the
   blocker (keep default disabled, no perf claim).

## Appendix — files touched by this diagnosis

Repo (design/notes only, no commits): this note, `patches/0046-*.patch.draft`,
and `passes/fused_post_chain/fused_ae_parameter_pipeline_design.md`. All
experiment artifacts (diagnostic projects, no-op DXIL/containers, `--gpu-validation`
logs) live under the session scratchpad, not the repo. The rb3 exe and the patch
stack were not modified.

## §6 Terminal verdict — R3 (2026-07-13, rb4)

R1 was implemented in full (patch 0046 revision: Rurix-owned scratch indirect
buffer, `buffer_copy` live->scratch as the only touch on the live buffer,
clear/dispatch confined to the scratch, `draw_list_draw_indirect` retargeted to
the scratch via a per-frame map) and put to its pre-registered final arbiter:
`ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py` on the rb4 exe
(0001-0029+0040-0048).

**Verdict: candidate leg still device-removes (`0x887A0005`).** Per the
pre-registered decision tree this convicts the GENERAL hypothesis: on this
Godot D3D12 backend the pattern *compute-written `DISPATCH_INDIRECT` buffer
consumed by a same-frame indirect draw* removes the device irrespective of
which buffer is involved, what the kernel writes (empty kernel reproduces), or
the instance count (1 reproduces). The debug layer and GPU-Based Validation are
both silent, i.e. the CPU-visible barrier/state chain validates clean and the
fault is on the GPU timeline.

R2 (apply last frame's cull results via a frame-boundary copy) is rejected:
1-frame-stale visibility violates the pass's conservative picture-preservation
invariant under camera motion — the visual gate would only pass on static
scenes, which would be dishonest coverage.

**R3 close-out**: `pass_manifest.json` flipped to
`grx015_rd_native_r1_final_verdict_mechanism_blocked_rdg_gap`; GRX-015/016/018
remain blocked on this engine/driver combination; default stays disabled; no
performance claim. This is an upstream Godot bug-report candidate (RDG /
D3D12 driver: missing GPU-timeline sync for UAV->INDIRECT_ARGUMENT within one
frame graph submission); the report recipe is this note plus the three-stage
evidence chain (shim / in-graph live / in-graph scratch).

> **This §6 R3 verdict is RETRACTED by §7 (2026-07-13). The "general RDG gap" /
> "compute-written DISPATCH_INDIRECT consumed same-frame" attribution was wrong;
> the confounding variable was a misaligned `buffer_clear`. Read §7.**

## §7 Conviction correction (2026-07-13) — misaligned `buffer_clear` was the confounding variable; R1b

**The §4/§6 conviction is RETRACTED.** The device-removal root cause is NOT a
same-frame "compute-written `DISPATCH_INDIRECT` buffer consumed by
`draw_list_draw_indirect`" RDG gap (nor the live command buffer's "dual role").
It is a **misaligned `RenderingDevice::buffer_clear`**: an out-of-spec RAW-UAV
clear at a byte offset that is not a multiple of 16.

### The confounding variable

Every crashing leg in §3 and §6 — shim era, in-graph clear of the live command
buffer, and R1's in-graph clear of the *scratch* command buffer — shared ONE
operation that the "clean" legs never had: an `RD::buffer_clear` of the
per-surface instance-count dword at byte offset
`(surface*command_stride_dwords + instance_count_dword_index)*4` =
`(s*5 + 1)*4` = **4, 24, 44, …**, none of which is a multiple of 16. That clear
— not the compute→indirect pattern — was the invariant across all removals. The
discriminator matrix in §3 mis-read it: the "clears cmd buf?" column was TRUE for
exactly the crashing legs, but the salient property was the *offset alignment* of
that clear, not *which* buffer it targeted. (The visibility-bitmask clear that all
legs also issued is at offset 0, which IS 16-aligned, so it was harmless — another
reason the offset, not the clear per se, is the cause.)

### Two side-by-side minimal reproducers (real GPU, RTX 4070 Ti, D3D12)

Isolated in `spike/godot-rurix/upstream-repro/` (stock `RenderingDevice`/D3D12,
no compute or draw needed to reproduce):

| Reproducer | Pattern | Result |
|---|---|---|
| `rd-buffer-clear-misaligned-offset/` | a bare `buffer_clear(buf, offset, 4)` on the main device, no compute/draw | offset 0/16/32/48 → **clean**; offset 4/8/12/20/36 → **device removed frame 1**. A perfect `offset % 16` law, zero exceptions. |
| `upstream_bug_repro/` (FALSIFIED) | pure compute UAV-write of a `DISPATCH_INDIRECT` buffer → same-frame `draw_list_draw_indirect`, NO misaligned clear | **300 frames clean**, no device removal |

The pure compute→indirect pattern that §4 convicted runs clean; the bare
misaligned clear that §4 dismissed removes the device on frame 1. This inverts
the §4 conclusion.

### The exact driver mechanism

`RenderingDeviceDriverD3D12::command_clear_buffer`
(`drivers/d3d12/rendering_device_driver_d3d12.cpp:3826`) implements `buffer_clear`
by building a RAW buffer UAV and calling `ClearUnorderedAccessViewUint`:

```cpp
uav_desc.Buffer.FirstElement = p_offset / 4;   // no alignment enforcement
uav_desc.Buffer.NumElements  = p_size / 4;
uav_desc.Buffer.Flags        = D3D12_BUFFER_UAV_FLAG_RAW;
```

D3D12 requires a RAW buffer UAV's byte offset to be a multiple of 16
(`D3D12_RAW_UAV_SRV_BYTE_ALIGNMENT` = 16). `RenderingDevice::buffer_clear`
(`rendering_device.cpp:875`) only enforces `p_size % 4 == 0` — it does NOT check
offset alignment — so a 4-byte-aligned-but-not-16-aligned offset produces an
out-of-spec UAV, and the clear removes the device asynchronously (0x887A0005,
silent under the debug layer and GPU-Based Validation — consistent with §3.5,
which is now re-read as "the misaligned RAW UAV is not a CPU-side-detectable
state error", not "the compute→indirect barrier is insufficient"). By contrast,
`buffer_copy` lowers to `command_copy_buffer` → `CopyBufferRegion`
(`rendering_device_driver_d3d12.cpp:3870`), which has NO RAW-UAV alignment
constraint; the RD-layer `buffer_copy` (`rendering_device.cpp:667`) only
bounds-checks a 4-byte offset/size, and is equally draw-graph tracked
(`add_buffer_copy`). This is a stock-`RenderingDevice`/D3D12 upstream bug (see
`spike/godot-rurix/upstream-repro/ISSUE_DRAFT.md`, DRAFT for owner review).

### R1b — the fix

R1b (patch 0046 revision) keeps R1's Rurix-owned scratch-indirect decoupling
(the live MultiMesh command buffer is only ever a `buffer_copy` READ source) but
replaces the misaligned per-surface count-dword `buffer_clear` with an ALIGNED
`RD::buffer_copy` from a persistent 16-byte all-zero SSBO
(`rd_native_gpu_culling_zero_buffer`, created once with zero data, never written).
The visibility-bitmask clear stays a `buffer_clear` at the 16-aligned offset 0.
Note R1's scratch decoupling turned out to be unnecessary for the removal (the
live-buffer dual role was never the cause) but is retained as clean, conservative
ownership. This is the ONLY code change from R1; no container / b0 / kernel / RTS0
change.

### R1b terminal verdict (rb5, 0001-0029 + 0040-0048)

R1b was put to the same pre-registered arbiter,
`ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py`, on the rb5 exe (incremental
rebuild carrying the R1b 0046, `RURIX_REQUIRE_REAL=1`).

**Verdict: R1b BREAKS the device-removal wall.** The candidate leg (backend==2) is
the FIRST rd_native leg to survive on real hardware (RTX 4070 Ti): it engages the
cull (`RXGD_RD_NATIVE_GPU_CULLING active` marker present), dispatches, is
frame-stable across 3 consecutive frames, and shows **NO device removal in any
leg** (reference / candidate / fail_closed all exit 0; every leg's
`device_removal_hits` is empty). This empirically confirms the misaligned-clear
conviction — swapping the misaligned RAW-UAV count-dword clear for an aligned copy
is sufficient to eliminate the removal. The §6 R3 "mechanism-blocked" verdict is
retracted; rd_native is no longer mechanism-blocked.

**A separate downstream finding is now cleanly exposed** (previously masked by the
removal): the cull is **NOT picture-preserving**. The candidate frame diverges
from the native reference (LDR **max_abs=155**, mean_abs≈0.0133), i.e. the
conservative frustum cull drops instances that are actually visible
(over-culling). Because a conservative cull must never drop a visible instance,
this is an honest cull-math correctness finding (frustum-plane sign /
bounding-sphere margin / b0 parameterization in the GRX-015 kernel), NOT a
device-removal or graph hazard. The gate correctly classifies it as
`status=skip / measured_prerequisite_blocked / rd_native_cull_not_picture_preserving`
(not a fail; a measured-prerequisite skip is not upgraded by
`RURIX_REQUIRE_REAL`). The fail_closed leg byte-matches the reference
(max_abs=0), confirming the fallback is intact.

`real_gpu_pass` stays false and `default_enable_state` stays disabled (strict
success requires picture-preservation), and no performance claim is made. Net:
with the confounding variable removed, the device-removal wall is broken and the
remaining GRX-015 blocker is a cull picture-preservation (over-culling) bug to be
resolved separately — a clean, trustworthy result now that the confound is gone.
`pass_manifest.json` `status` is updated to
`grx015_rd_native_r1b_device_removal_resolved_cull_picture_preservation_open`, and
the `rd_native_r3_closeout` block records the retraction alongside a new
`rd_native_r1b_verdict` block.

## §8 Over-cull conviction (2026-07-13) — count-only prefix draw, NOT a per-instance cull-math bug; R1c high-water-mark fix; FIRST STRICT SUCCESS (rb5)

The R1b "cull is NOT picture-preserving" finding (§7, `max_abs=155`,
`mean_abs≈0.0133`) is now convicted at the field level and fixed. **It was never
a per-instance cull-math bug** (frustum-plane sign, plane normalization, bounding
sphere, or b0 assembly were all correct); it is the **count-only-without-compaction
mechanism** dropping visible instances at the tail of the index range.

### The convicted mechanism

The rd_native kernel wrote `InstanceCount = count-of-visible` via
`InterlockedAdd(dst_commands[...], 1)`. The native indirect draw
(`render_forward_clustered.cpp`, `draw_list_draw_indirect`) then renders instances
`[0 .. InstanceCount-1]` **by index** from the transform buffer. This count-only
subset does **not** compact/remap transforms (GRX-016, explicitly out of subset),
so `InstanceCount` is a **prefix length**. Count-of-visible is a correct prefix
length **only if the visible set is exactly the prefix `[0 .. V-1]`**. For a
scattered visible set, reducing the count to `V` renders indices `[0 .. V-1]` —
which includes off-screen low-index instances and **excludes the visible
instances at index ≥ V**, deleting them from the image (over-cull).

### Numerical conviction (faithful Godot-math replay of the exact gate scene)

The gate scene (`ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py`): a 64×64
indirect MultiMesh of 0.35-unit boxes on the XZ plane (`gx = i%64`, `gz = i/64`,
`x = (gx-32)*1.5`, `z = -gz*1.5`), camera at `(0,6,18)` looking at `(0,0,-20)`,
default 75° vertical FOV, 256×144 viewport. Replaying Godot's exact
`Projection::set_perspective` + `get_projection_planes` + the 0046 plane-sign
mapping + the kernel sphere test:

- **per-instance over-cull (box-visible AND kernel-culled): 0** — confirms the
  per-instance kernel math is correct (matches the offline bit-exact parity).
- kernel `count-of-visible V = 3929`; ground-truth box-visible = 3923.
- **167 box-visible instances have index ≥ V=3929** — all in the deepest rows
  `gz = 61,62,63` (`z ≈ -91.5 .. -94.5`). The prefix draw `[0 .. 3928]` deletes
  them. They are the farthest boxes from the camera (~112 units), each ≈ sub-pixel
  to ~1px → a *scattering of tiny high-contrast specks*: small `mean_abs`, high
  local `max_abs` — quantitatively matching the observed `max_abs=155`,
  `mean_abs≈0.0133`.
- the visible index range is `15 .. 4095` (NOT a prefix), so **any** `count < 4096`
  drops the visible deepest-corner instance 4095.

### R1c fix — high-water-mark count (rd_native kernel; container-only)

The rd_native kernel's per-surface count write is changed from
`InterlockedAdd(dst_commands[...], 1)` to
`InterlockedMax(dst_commands[...], instance + 1)`, so after the dispatch
`InstanceCount == (highest visible instance index) + 1`. The prefix draw
`[0 .. InstanceCount-1]` then includes **every** visible instance (off-screen
instances inside that prefix are still drawn but contribute no pixels), while the
dropped tail `[InstanceCount .. N-1]` is entirely invisible. This is the **minimal
picture-preserving count** for an uncompacted prefix draw: it never over-culls and
still reduces the draw when the instance-array tail is off-screen. This is the ONLY
code change from R1b — a pure kernel edit
(`artifacts/hlsl_bridge/gpu_culling_rd_native.hlsl`), recompiled with the round-7
signing DXC (`H:\dxc-round7\extracted\bin\x64`, dxc 1.9.2602, DXV "Validation
succeeded"), the container regenerated by `generate_rd_container.py` and
re-`verify_container.py`'d **59/59**, restaged to
`target/grx/rd_containers/gpu_culling_rd_native.rd_container.bin`. **No 0046 patch /
b0 / RTS0 / descriptor-binding / exe change** (the container is runtime-loaded via
`rd_container_path`; the rb5 exe is reused unchanged). The shim `frustum_count`
kernel (backend==1, superseded/device-removing) keeps the count-of-visible and its
matching math-parity fixtures; it shares this latent prefix limitation but is not on
the enablement track.

### R1c terminal verdict (rb5, 0001-0029 + 0040-0048) — STRICT SUCCESS

`ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py` on the rb5 exe with the R1c
container: **`status=success` — the FIRST strict success for gpu_culling
rd_native.** Candidate `max_abs=0, mean_abs=0.000000` (**byte-exact
picture-preservation**); all 13 checks green (candidate engaged the active marker,
reference/fail_closed did not; **no device removal in any leg**; all three frames
byte-stable; fail_closed byte-matched the reference). `real_gpu_pass=true` recorded;
`rd_native_enablement_success_evidence.json` written.

**Honest scope note.** In THIS scene the deepest-corner instance (index 4095) is
visible, so the high-water-mark equals the full count (`4096`) → the candidate
scratch command buffer is byte-identical to the live one and there is **zero net
draw-count reduction here**. That is a property of the scene (visible tail), not a
weakness of the fix: the strict success genuinely demonstrates the gate's three
criteria (rd_native **engages** + **no device removal** + **picture-preserving**),
which is exactly what the gate certifies; a mid-frame count-readback proving an
actual reduction stays a deferred stronger check (`known_gaps`), and a *measurable*
draw-count reduction requires either a scene whose instance-array tail is off-screen
or GRX-016 transform compaction. `default_enable_state` stays **disabled** and
`performance_claim` stays **none**.
