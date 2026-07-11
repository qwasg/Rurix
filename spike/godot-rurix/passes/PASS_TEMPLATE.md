# GRX Per-Pass Template (nine stages)

Industrialized template for GRX-011..022 compute passes, distilled from the
matured GRX-009 `luminance_reduction` and GRX-010 `tonemap` slices. Each new
pass follows S1..S9. Multiple agents run this template in parallel; the
per-stage **Parallelism** line says what may overlap and what serializes.

Conventions used below:

- `<pass>` = the pass id (e.g. `ssao_blur`, `taa_resolve`).
- `NNN` = the GRX milestone number (e.g. `011`); `NNNN` = allocated patch
  numbers from `spike/godot-rurix/patches/PATCH_ALLOCATION.md`.
- `<PASS>` = the SCREAMING_SNAKE pass token (e.g. `SSAO_BLUR`).
- Pass dir: `spike/godot-rurix/passes/<pass>/`.
- Bridge crate: `src/rurix-godot/src/lib.rs`. RTS0 emit example:
  `src/rurixc/examples/emit_grx<NNN>_<pass>_rts0.rs`.
- Patch numbers and `RxGdCaps.flags` cap bits are pre-allocated — claim them
  from `PATCH_ALLOCATION.md`, never invent new ones.

Global invariants (every stage upholds these):

- Pass ships **default disabled**; the native Godot path is always the
  fallback/continuation. The shipping (feature-off) bridge returns
  `RXGD_STATUS_FALLBACK` for the pass.
- No FPS / p95 / GPU-timestamp / performance-improvement claim until real
  measured evidence exists (and even then, default stays disabled pending an
  owner decision).
- LF byte-exact for every tracked file (`.gitattributes * -text`); write with
  binary/`newline="\n"` I/O or the Edit tool, never Python text mode.
- `external/godot-master` is never edited directly; Godot changes land only as
  `spike/godot-rurix/patches/NNNN-*.patch`.
- Do not edit `milestones/` from a pass slice unless you hold that lock;
  `GRX_PLAN.md` / contract edits are owner/coordinator-serialized.

---

## S1 — Contract trio + package skeleton

Establish the pass contract and a fail-closed manifest before any code.

- **Inputs**: `external/godot-master` hook/call-site investigation (paths and
  function names only); `PATCH_ALLOCATION.md` reservation for the pass.
- **Outputs** (in the pass dir):
  - `PASS_CONTRACT.md` — pass id, target scenes, Godot hook/call-site/resource
    investigation, Rurix I/O mapping, supported math subset + `known_gaps`,
    fallback rules, bridge-gate chain, patch plan, evidence requirements, exit
    criteria.
  - `pass_manifest.json` — **fail-closed initial values**: `implemented=false`,
    `default_enable_state="disabled"`, `implementation_status.runtime_state=
    "fallback_only"`, `real_gpu_pass=false`, `real_d3d12_dispatch_recorded=
    false`, `offline_compile_status` unset/`skip`, full `known_gaps`.
  - `resource_mapping.md` — Godot resource → Rurix SRV/UAV/root-constant table,
    descriptor layout, fallback rules, explicit non-goals.
  - `rurix.toml` — package manifest for the math source.
  - `src/lib.rx` — Rurix math source (the parity reference target).
- **Verify**: JSON parses; manifest fail-closed values present; contract cites
  real Godot paths. `py -3 ci/check_guardrails.py`.
- **Red-leg**: a manifest that ships `implemented=true` / `real_gpu_pass=true`
  without S8 measured success must be rejected downstream (probe fail-closed
  manifest audit).
- **Parallelism**: fully cross-pass parallel (no lock).

## S2 — Offline kernel (DXC bridge + Rurix-owned RTS0)

Produce the canonical offline compute package.

- **Inputs**: `src/lib.rx`; the Godot reference shader (math-parity target);
  the owner-approved provenance policy
  (`spike/godot-rurix/passes/luminance_reduction/texture_artifact_provenance_policy.json`,
  which applies to every texture compute pass).
- **Tooling**: `compile_hlsl_bridge.py` in the pass dir —
  1. DXC `cs_6_0` compile of `artifacts/hlsl_bridge/<pass>_*.hlsl` (+ DXV
     validation);
  2. descriptor layout JSON (per-slot binding kinds
     `["texture2d","rwtexture2d"]` + the canonical 28-byte / 7-dword
     `[i64,i64,f32,f32,f32]` root-constant layout);
  3. Rurix-owned RTS0 via `cargo run -p rurixc --features "dxil-backend
     shader-stages" --example emit_grx<NNN>_<pass>_rts0`
     (`rurixc::binding_layout::{infer_root_signature, pack_root_constants,
     serialize_rts0}`).
- **Outputs**: canonical `artifacts/<pass>.dxil`, `artifacts/<pass>.rts0.bin`,
  `artifacts/<pass>_descriptor_layout.json`; `offline_compile_evidence.json`
  with `status=success`, `provenance="hlsl_bridge_workaround"`,
  `rurix_owned=false`, `runtime_mappable=true`, DXV `validation.status="pass"`,
  and the three artifacts' SHA-256 recomputable on disk.
- **Verify**: `py -3 ci/grx<NNN>_<pass>_offline_compile_smoke.py` (runs the
  compile then audits status/provenance + on-disk digest match).
- **Red-leg**: missing/broken DXC or DXV → honest `skip`/`compile_failed`/
  `validation_failed` evidence, exit non-zero under `RURIX_REQUIRE_REAL=1`;
  any on-disk hash mismatch is a hard FAIL. Bake the three SHA-256 digests into
  the S4 bridge gate — a runtime package that does not match fails closed.
- **Parallelism**: cross-pass parallel (needs a signed DXC/DXV suite via
  `RURIX_DXC_DIR`; independent per pass).

## S3 — Math parity (CPU reference)

Prove the kernel math against a CPU reference.

- **Inputs**: the supported math subset from S1/S2; deterministic synthetic
  inputs.
- **Tooling/Outputs**: `generate_math_parity_evidence.py` →
  `math_parity_evidence.json` with a per-case CPU float32 reference (per-op
  binary32 rounding) and `status=pending_gpu_dispatch` (GPU-observed side is
  filled by the S6 real dispatch).
- **Verify**: run the generator; JSON has a CPU-expected grid per case; the S4
  bridge gate's `<PASS>_KERNEL_MATH_PARITY_STATUS` constant matches this exact
  status string.
- **Red-leg**: any math-parity status string other than the exact
  `..._cpu_reference_proven_pending_gpu_dispatch` fails the bridge math gate
  closed.
- **Parallelism**: cross-pass parallel (CPU only, no lock).

## S4 — Bridge gate (`lib.rs` state machine)

Add the fail-closed bridge gate; the pass is wired but always falls back.

- **Inputs**: S2 digests + descriptor shape; the cap bit reserved in
  `PATCH_ALLOCATION.md` (`RXGD_CAP_<PASS>_REAL_PASS = 1 << N`).
- **Outputs** (`src/rurix-godot/src/lib.rs`, template-copied from `TonemapGate`
  / `LuminanceReductionGate`):
  - `<Pass>DispatchPackage` (baked resource count, root-constant bytes,
    SRV/UAV registers, three offline SHA-256 digests) + `verified_offline_
    package()` + `verify_matches_offline_evidence()`.
  - `<Pass>Gate` with the ordered chain: **runtime binding preflight →
    dispatch eligibility (opt-in cap bit + int64 cap + non-null device/queue +
    non-null handles + package digest match) → per-slot kernel-binding-kind
    conformance → math-parity gate → real dispatch** (linked only under the
    `d3d12-recording-shim` feature; feature-off = `real_dispatch_path_not_
    linked`).
  - Once-per-session machine-readable `RXGD_<PASS>_REAL_PASS_BLOCKED
    first_missing_prerequisite=... fallback_reason=... kernel_binding=... 
    default_enable_state=disabled` diagnostic (NOT an `ERROR:`/`RXGD_DIAG`
    line, so runtime log audits stay clean).
  - `#[cfg(test)]` unit tests for each gate branch.
- **Verify**: `cargo test -p rurix-godot`; `cargo clippy`/`fmt`.
- **Red-leg**: buffer (non-texture) resources fail the per-slot binding-kind
  check; a digest mismatch → `validation_failed`; missing opt-in cap →
  `manual_disabled`. The default path (no opt-in cap) always returns
  `RXGD_STATUS_FALLBACK`.
- **Parallelism**: cross-pass parallel at authoring, but note all passes touch
  the SAME `lib.rs` — coordinate merges (small, additive, per-pass blocks;
  distinct cap bits from `PATCH_ALLOCATION.md` avoid semantic collision).

## S5 — Patch A (gate + call-site, default false)

First Godot patch: the per-pass setting and opt-in call-site gate.

- **Inputs**: S4 bridge gate; patch number `NNNN` (gate+callsite) from
  `PATCH_ALLOCATION.md`; the stack-lock.
- **Outputs**: `spike/godot-rurix/patches/NNNN-rurix-accel-<pass>-pass-gate-
  and-callsite.patch` — default-`false`
  `rendering/rurix_accel/passes/<pass>/enabled`, `try_record_<pass>()` module
  gate (0002 pattern: setting off / session missing / non-OK → `false`, print
  one verbose fallback marker), default-`false` virtual
  `D3D12Hooks::try_record_<pass>()`, and the opt-in gate before the native call
  site (native path runs whenever the gate returns false — which is always by
  default).
- **Verify**: `py -3 ci/godot_rurix_patch_stack.py` (stacked applyability on a
  scratch copy). **Patch generated via `git diff --no-index` on a scratch copy
  with all prior patches applied — never hand-written.**
- **Red-leg**: patch must not apply cleanly when prior patches are missing; the
  applyability check drives the probe's patch state.
- **Parallelism**: **SERIAL** — requires the single patch-stack lock
  (`PATCH_ALLOCATION.md` §4).

## S6 — Standalone D3D12 dispatch smoke

Prove the offline package runs on a real device and matches the CPU reference.

- **Inputs**: S2 canonical DXIL/RTS0/descriptor layout; a real D3D12 adapter +
  signed DXC `dxil.dll` (for in-place container signing) + MSVC.
- **Tooling/Outputs**: `ci/grx<NNN>_<pass>_d3d12_dispatch_smoke.py` →
  `real_d3d12_dispatch_smoke.json` — real device/queue, root signature built
  directly from RTS0, compute PSO from the (signed) tracked DXIL, SRV `t0` /
  UAV `u0` / b0 bound strictly per the descriptor layout, one dispatch + fence
  + UAV readback, and the readback first texel compared to the S3 CPU
  reference within tolerance.
- **Verify**: run the smoke; readback matches CPU reference. Fill the GPU-
  observed side of `math_parity_evidence.json` here.
- **Red-leg**: no adapter / missing MSVC / missing signed DXC → honest `SKIP`
  that does NOT advance ready (fail under `RURIX_REQUIRE_REAL=1`); a
  readback/CPU mismatch is a hard FAIL.
- **Parallelism**: cross-pass parallel in authoring, but **GPU + MSVC
  execution is host-exclusive** (see S8).

## S7 — Patch B + C (runtime binding, recording + real-pass opt-in)

Wire real native handles and the opt-in real-pass arm.

- **Inputs**: S5 patch A applied; the next two reserved patch numbers; the
  stack-lock.
- **Outputs**:
  - Patch B `NNNN-rurix-accel-<pass>-runtime-resource-binding.patch` — resolve
    real `ID3D12Resource*` via `RenderingDevice::get_driver_resource(...)`;
    handle 0 / device unavailable → fallback (upgraded fallback marker).
  - Patch C `NNNN-rurix-accel-<pass>-recording-smoke-and-real-pass-optin.patch`
    — default-`false` `.../dispatch_real_pass`, `.../dispatch_recording_smoke`,
    `.../real_pass_force_capability_downgrade` opt-ins; set
    `RXGD_CAP_<PASS>_REAL_PASS` only when the opt-in is on;
    `RXGD_GODOT_RUNTIME_<PASS>_RECORD` marker; real dispatch path under
    `d3d12-recording-shim`. Result writeback is a SCAFFOLD (native path still
    re-renders every frame as continuation/backstop).
- **Verify**: `py -3 ci/godot_rurix_patch_stack.py` (both patches stacked).
  Patches generated by `git diff --no-index` on the scratch stack.
- **Red-leg**: default config (all opt-ins false) still yields
  `RXGD_STATUS_FALLBACK`; feature-off bridge fails closed with
  `real_dispatch_path_not_linked`.
- **Parallelism**: **SERIAL** — patch-stack lock.

## S8 — Scratch rebuild + enablement smoke

Measure the opt-in real pass in a real Godot build.

- **Inputs**: the full patch stack (0001..NNNN with this pass's triple)
  rebuilt into a scratch Godot (Windows D3D12 Forward+) with the
  `d3d12-recording-shim` bridge; real adapter + MSVC.
- **Tooling/Outputs**: `ci/grx<NNN>_<pass>_real_pass_enablement_smoke.py` →
  `real_pass_enablement_success_evidence.json` (+ `real_pass_enablement_
  telemetry.json`). Records a strict MEASURED success across **three legs**:
  `reference` (disabled) / `candidate` (opt-in real pass) / `forced_fallback`
  (forced capability downgrade red leg measuring `unsupported_device`), the
  in-engine LDR visual diff within pinned thresholds, GRX-008-valid telemetry,
  and full patch-stack / provenance / runtime-log audits. DLL fingerprint
  records `features=[d3d12-recording-shim]`.
- **Verify**: enablement smoke `status=success`; `RURIX_REQUIRE_REAL=1` makes a
  SKIP fail. A tampered/placeholder success artifact must be rejected by the
  strict audit (probe `..._real_pass_success_evidence_conflict`).
- **Red-leg**: the `forced_fallback` leg is mandatory; the strict audit rejects
  any tampered digest / SKIP-shaped success.
- **Parallelism**: **HOST-EXCLUSIVE** — one agent at a time holds the GPU+MSVC
  build/run resource (also covers S6 execution).

## S9 — Close-out (gate module + probe registration + flip + docs)

Finalize the pass and hand off to the next.

- **Inputs**: S8 measured success + the owner default-enable decision.
- **Outputs**:
  - `ci/grx_gates/grx<NNN>_<pass>.py` — a gate module exporting
    `evaluate() -> dict` per `ci/grx_gates/_common.py` (keys `gate_id`,
    `contract_ready`, `patch_applyability`, `dispatch_smoke_ready`,
    `enablement_ready`, `decision_ready`, `first_issue`, `next_action`).
  - Register the gate in `ci/godot_rurix_toolchain_probe.py::GRX_GATE_SEQUENCE`
    (append one entry `{"gate_id": "grx<NNN>", "module": "grx<NNN>_<pass>"}`).
    The probe walks the table fail-closed; a module load/interface/first_issue
    error records `grx_gate_module_error` and does NOT advance `next_action`.
    Its `next_action` (when fully ready) points at the NEXT pass's start action.
  - Flip `pass_manifest.json` (`implemented=true`, `real_gpu_pass=true`
    opt-in-measured, `real_d3d12_dispatch_recorded=true`,
    `runtime_state="fallback_only_by_default_real_pass_optin_measured"`;
    `default_enable_state` stays `disabled`) — accepted by the probe ONLY while
    the strict measured success is active.
  - `real_pass_default_enable_decision.{json,md}` — owner decision (typically
    `keep_default_disabled` pending per-pass FPS evidence).
  - Milestone bookkeeping (coordinator-serialized; do not edit `milestones/`
    without that lock).
- **Verify**: `py -3 ci/godot_rurix_toolchain_probe.py` (next_action advances to
  the next pass); `py -3 ci/godot_rurix_toolchain_probe_validation_failed_test.py`;
  `py -3 ci/check_guardrails.py`.
- **Red-leg**: a tampered success artifact drives a conflict and the probe
  falls back to the pre-close-out (false-valued) manifest shape; a broken gate
  module is reported as `grx_gate_module_error` without advancing.
- **Parallelism**: **SERIAL** — probe/`GRX_GATE_SEQUENCE` edits are a single
  shared file; register one pass at a time.

---

## Parallelism summary

| Stages | Parallelism |
| --- | --- |
| S1, S2, S3, S4, S6 (authoring) | cross-pass parallel (no lock) |
| S5, S7 | serial — patch-stack lock (one agent) |
| S6/S8 (execution) | host-exclusive — GPU + MSVC (one agent) |
| S9 | serial — probe / `GRX_GATE_SEQUENCE` registration (one agent) |

> `lib.rs` (S4) and `GRX_GATE_SEQUENCE` (S9) are shared files even when the
> surrounding stage is "parallel"; keep per-pass edits small and additive, and
> land the manifest flip + gate registration together with the pass's final
> patch (`PATCH_ALLOCATION.md` §4.3).
