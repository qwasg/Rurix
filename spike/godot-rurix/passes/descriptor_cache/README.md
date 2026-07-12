# GRX-020 descriptor_cache — shim descriptor-heap ring hardening (telemetry-only)

## What this is

GRX-020's slice is a **hardening of the D3D12 recording shim's shader-visible
descriptor heap ring**, NOT a render pass. It has no kernel, no DXIL, no RTS0,
no bridge pass id, no `ci/grx_gates/` module, and makes **no FPS, p95,
GPU-timestamp, or performance claim**. It is telemetry-only + a correctness
hardening, and it stays `keep_disabled` in the sense that it never enables any
Godot runtime pass.

## The余量 it closes (reconnaissance定案)

Before this slice, `ShimSession::reserve_descriptors`
(`src/rurix-godot/shim/rxgd_luminance_record.cpp`) sub-allocated the single
shared `CBV_SRV_UAV` heap with a rolling cursor and, on wrap-around, simply reset
`heap_cursor = 0` with **no fence synchronization** — a self-declared
"test-only ring" heuristic. Under high dispatch pressure a wrapped reservation
could hand back descriptor slots still referenced by an in-flight submit,
stomping live descriptors.

## The hardening

`reserve_descriptors` now mirrors the allocator ring's fence discipline
(**segmented fence-value reclaim**):

- each reservation records its `[begin, end)` sub-range against the fence value
  the pending submit WILL signal (`next_fence_value + 1`, since `next_fence_value`
  is only bumped at `Signal`);
- on wrap-around, `wait_for_descriptor_range` waits for any committed segment
  that overlaps the reused range whose submit has been signaled but not yet
  completed, then prunes segments that have completed so the ring stays bounded;
- a segment whose fence value is beyond the last signaled value belongs to a
  record that reserved but never submitted (a failed record) and is dropped
  without waiting — the ring never deadlocks on an un-signaled fence.

No single record reserves more than the heap capacity, and all reserves within
one record predict the same submit value, so a record never waits on its own
unsubmitted fence.

## Telemetry (heap-segment health)

Pure telemetry, emitted once per session at close via `print_summary`:

```
RXGD_DESCRIPTOR_RING wraps=<n> waits=<m> capacity=<C>
```

- `wraps` — how often the descriptor cursor wrapped past the heap capacity.
- `waits` — how often a wrap had to wait for an in-flight submit to finish
  before reusing a range (segment-reclaim contention). `waits == 0` under a
  keep-up completion rate; `waits > 0` only under genuine high-pressure
  wrap-around.
- `capacity` — the shader-visible heap slot count (`kDescriptorHeapCapacity`).

The line rides the existing `RXGD_SUMMARY` engagement/close path; no separate
sidecar file is required.

## Red leg (high-pressure wrap-around correctness)

`src/rurix-godot/src/lib.rs` ships a pure-Rust executable spec of the C++
algorithm (`DescriptorFenceRing`, `#[cfg(test)]`) plus two stress tests:

- `descriptor_ring_waits_before_reusing_a_live_range` — a tiny 8-slot heap with
  3-descriptor records and completion lagging by two submits forces ≥20 wraps;
  after every reserve it asserts the handed-out range does NOT overlap any
  previously-submitted in-flight range without first waiting (the invariant the
  old blind `heap_cursor = 0` reset would violate). Also asserts ≥1 fence wait
  actually fired.
- `descriptor_ring_never_waits_when_completion_keeps_up` — the no-contention
  green path: when every submit completes before the next reservation, wrap is
  always safe and never blocks (`waits == 0`).

The model mirrors the C++ predict-the-pending-submit-fence + overlap-wait +
in-flight-only-prune algorithm exactly, so it is the executable spec for the shim
ring.

## Decision

- **keep_disabled / no enablement.** This is an internal reliability hardening of
  the recording shim, not a pass; there is nothing to default-enable and no
  owner default-enable decision is created. It ships always-on inside the
  recording-shim feature (the only place the ring exists) and is a no-op for the
  shipping feature-off bridge.
- No patch: the descriptor heap ring lives entirely in the shim; the Godot side
  is untouched.

See `descriptor_cache_decision.json` for the machine-readable decision record.
