# UC-04 PR-F2 Preflight Blocker Audit (2026-06-28)

Status: blocked-honest preflight. This note records why PR-F2 must not proceed to
an implementation / device-green claim until the listed deferred items are
closed or explicitly re-scoped by the owner. No G-G2-4 sign-off is claimed here.

## Scope Checked

- Branch: `codex/g2.4-uc04-pr-f2`, forked from `main` at `87f6bb9`.
- Target scope from `spec/d3d12_runtime.md`: RXS-0167~0170 clause bodies,
  `src/uc04-demo`, safe D3D12 runtime wrapper, manual barriers, offscreen
  readback, RX6018+ error codes, golden / bless / device run / CI step 48.
- Contract hard gate from `milestones/g2/G2_CONTRACT.md`: a green G-G2-4 run
  must prove Rurix source through `rurixc` graphics=B DXIL, RFC-0005 RTS0 /
  binding layout, D3D12 PSO, hardware multi-pass deferred draw, and offscreen
  readback. Hand-written HLSL/DXIL, CPU prefill, single-pass textured draw,
  fullscreen copy, fixed pixel injection, host-only simulation, window
  screenshot, and SKIP are not valid substitutes.

## Findings

1. RD-013 is a blocking predecessor for a true G-G2-4 shader path.

   `registry/deferred.json` RD-013 explicitly leaves DXIL shader-entry body
   dataflow deferred: reading signature inputs, doing real statements, and
   writing signature outputs are not implemented. The current graphics=B
   encoder in `src/rurixc/src/dxil_spirv.rs` emits the interface and a trivial
   `main` containing `OpFunction`, `OpLabel`, `OpReturn`, and
   `OpFunctionEnd`. `spec/dxil_backend.md` also states that entry body
   dataflow is RD-013 and outside the landed RXS-0159 body.

   Consequence: a PR-F2 device smoke could only be made green today by using
   shader bodies that do not come from Rurix source, by injecting fixed pixels,
   or by otherwise bypassing the body-dataflow gap. Those routes are exactly
   what G-G2-4 forbids.

2. Existing G2.2/G2.3 device smokes are useful precedents but not sufficient
   evidence for G-G2-4.

   `ci/dxil_device_smoke.py` proves a real D3D12 offscreen draw/readback path
   with validator-accepted DXIL, but its shaders are hard-coded HLSL for the
   G-G2-2 channel. `ci/dxil_binding_device_smoke.py` proves that a real D3D12
   device consumes Rurix-derived RTS0 bytes for the G-G2-3 binding-layout
   channel, but the shader body is still external HLSL. Neither can be reused
   as the G-G2-4 green without violating the anti-downgrade gate.

3. RD-021 is a second boundary for full deferred lighting.

   RFC-0006 / PR-F1 scoped the first implementation to opaque
   `Texture2D`/`Sampler` handles plus D3D12 RT/SRV view binding, while
   explicitly deferring texture memory-model semantics to RD-021. A true
   deferred lighting pass normally samples the G-buffer. If PR-F2 needs to
   define Rurix texture sampling/load/store semantics, LOD/derivatives,
   bounds behavior, cache visibility, or memory ordering, it must stop for an
   owner Full RFC instead of inventing those semantics inside the runtime PR.

4. A partial host/runtime-only slice would be misleading if labeled PR-F2
   completion.

   It is possible to build D3D12 helper scaffolding or a harness that accepts
   externally supplied DXIL, but such a slice would not satisfy RXS-0167~0170
   or G-G2-4 unless it is explicitly marked as preflight / blocked-on-RD-013
   and does not claim step 48 green.

## Non-Options Rejected

- Do not use hand-written HLSL or hand-written DXIL for the G-G2-4 green.
- Do not reuse G-G2-2 / G-G2-3 device smokes as if they closed G-G2-4.
- Do not inject fixed pixels, prefilled readback buffers, or a fullscreen copy
  and call it deferred rendering.
- Do not add CI step 48 as a SKIP-only or host-only green.
- Do not define texture sampling memory semantics in PR-F2 without owner Full
  RFC coverage.

## Recommended Next Owner Path

1. Split the unblock before PR-F2 proper:
   - Close RD-013 with a minimal graphics=B statement/body lowering slice:
     signature input reads, constants/arithmetic needed by the demo,
     signature output writes, validator-accepted golden, and red/green tests.
   - Decide whether the UC-04 deferred lighting pass requires RD-021 Full RFC
     before the runtime/demo can be honest-green. If yes, land the owner RFC
     before PR-F2 implementation.

2. Only after those predecessors are green, land PR-F2 proper:
   RXS-0167~0170 bodies, `src/uc04-demo`, D3D12 runtime wrapper, manual
   barrier sequence, offscreen readback, RX6018+ errors, golden/bless, and
   CI step 48 with `RURIX_REQUIRE_REAL=1`.

3. If owner intentionally wants an interim implementation slice, name it as a
   preflight or harness PR and keep G-G2-4 explicitly blocked.

## Validation

- Repository inspection only; no generated artifacts, no code changes, no
  device run, no golden bless.
- Follow-up registry edit records this audit under RD-013 and RD-021 without
  changing their status.
