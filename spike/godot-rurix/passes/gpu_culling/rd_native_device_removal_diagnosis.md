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
