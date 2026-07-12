# GRX-021 pso_prewarm — bridge kernel prewarm (bridge-internal, not a pass)

## What this is

GRX-021's slice is a **bridge-internal warm-up**: at D3D12 session creation the
bridge prewarms every embedded kernel's root signature + compute PSO for the
`(device, queue)` so the FIRST real dispatch of any pass does not pay the lazy
create cost. It is NOT a Godot render pass — it has no kernel of its own, no
DXIL/RTS0, no bridge pass id, no `ci/grx_gates/` module, and makes **no FPS,
p95, GPU-timestamp, or performance claim**. It reuses the existing per-kernel
PSO cache (`ShimSession::get_or_create_kernel`); prewarm just populates it eagerly
instead of on first dispatch.

## Entry points

- Shim: `rxgd_prewarm_kernels(abi_version, device, queue, kernels, kernel_count)`
  (`src/rurix-godot/shim/rxgd_luminance_record.cpp`) — walks the kernel array,
  calling `ShimSession::prewarm_kernel` (= `get_or_create_kernel`, including the
  in-memory DXIL signing) for each. Returns the count warmed (`>= 0`) or a
  negative status on an ABI mismatch / null handle.
- Bridge: `d3d12_recording_shim::prewarm_session_kernels(device, queue)`
  (`src/rurix-godot/src/lib.rs`) — enumerates all 14 embedded kernels
  (luminance reduce + WRITE_LUMINANCE, tonemap, ssao_blur, taa_resolve,
  particles_copy, cluster_store, gpu_culling, the three instance_compaction
  kernels, both indirect_args kernels, fused_post_chain) and hands them to the
  shim entry. Returns `(warmed, total)`.

## Auto-trigger + the contract red line

`rxgd_create_d3d12_session` auto-prewarms after the caps check:

```rust
#[cfg(all(feature = "d3d12-recording-shim", not(test)))]
{
    let (warmed, total) = d3d12_recording_shim::prewarm_session_kernels(device, queue);
    if warmed < total { eprintln!("RXGD_PREWARM_DIAG warmed={warmed}/{total} (non-fatal; ...)"); }
}
```

**Red line — prewarm failure NEVER fails the session (best-effort, non-fatal).**
The session is created regardless of the prewarm outcome: a device without the
signed DXC validator / experimental shader models simply builds each kernel
lazily on first use, exactly as before. `rxgd_create_d3d12_session` ignores the
`(warmed, total)` return except for an optional diagnostic line.

**Feasibility caveat (`not(test)`):** auto-prewarm is compiled OUT of `cargo test`
builds because the unit tests construct sessions with SENTINEL device/queue
handles (e.g. `1`/`2`) that are not real `ID3D12Device*` objects and must not be
dereferenced to build PSOs. The shipping cdylib the Godot runtime and the
enablement smokes load is a non-test build, so it prewarms with the real
handles. The `rxgd_prewarm_kernels` non-fatal degrade (null/bad handle → negative
status, no crash) is exercised directly by the
`prewarm_null_handles_degrades_gracefully` unit test under the recording-shim
feature.

## Decision — `enable_by_default_in_bridge = true`

Prewarm is a **bridge-internal behaviour, not a render pass**, so the usual
"default disabled pending an owner decision" rule does not apply — there is no
pass to enable and no `RxGdCaps.flags` opt-in bit governs it (the reserved
`RXGD_CAP_PSO_PREWARM_REAL_PASS` bit 14 is a placeholder for a possible future
telemetry surface, not an on/off switch for this warm-up). It is safe to run
unconditionally at session creation because:

1. it only builds the SAME PSOs the passes would build lazily anyway (no new GPU
   state, no dispatch, no resource binding);
2. its failure is non-fatal by contract (degrade to lazy);
3. it emits no per-frame stdout and no performance claim.

So prewarm is **enabled by default inside the recording-shim bridge** (auto-
triggered in the shipping cdylib session-creation path), honestly characterized:
it is a startup-cost redistribution, not a measured speedup, and no perf claim is
made.

## Patch 0039 — NOT required

Because the bridge auto-triggers prewarm from `rxgd_create_d3d12_session`, no
Godot-side call site is needed: patch 0001's `try_create_session` already routes
through `rxgd_create_d3d12_session`, which now prewarms on its own. The reserved
patch `0039` (pso_prewarm) is therefore **marked not needed** in
`PATCH_ALLOCATION.md §2` (a permanent hole per the monotonic-numbering rule). If
a future slice wants a Godot-visible prewarm toggle or telemetry surface, it can
claim 0039 then; this slice does not.

See `pso_prewarm_decision.json` for the machine-readable decision record.
