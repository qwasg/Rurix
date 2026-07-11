# GRX-009 Texture Artifact Provenance Policy

## Scope

This document records the owner-approved provenance policy for promoting DXC HLSL bridge texture artifacts as the **temporary runtime-mappable canonical** luminance package while patched `llc` lacks `llvm.dx.resource.load/store.texture.2d` support. This policy slice does not by itself set `runtime_mappable=true`, enable `real_gpu_pass`, or claim visual/performance success.

## Owner Decision

**Approved by:** project owner (local test machine)  
**Decision:** Approve `provenance=hlsl_bridge_workaround` with `rurix_owned=false` as a temporary canonical path when all policy gates below are satisfied.  
**Effective:** upon `texture_artifact_provenance_policy.json` recording `policy_ready=true`.

## Exception to Canonical Switch Conditions

[`dxc_texture_artifact_bridge.md`](dxc_texture_artifact_bridge.md) §Canonical Switch Conditions item 6 requires `provenance.rurix_owned=true`. This policy adds an **owner-approved exception**:

- HLSL bridge DXIL may become the canonical DXIL container when `provenance=hlsl_bridge_workaround` and `rurix_owned=false`, provided:
  - RTS0 bytes are **Rurix-synthesized** from the package descriptor layout (`rurixc::binding_layout`).
  - Descriptor layout records `binding_kind=texture2d` / `rwtexture2d` for `src_luminance` / `dst_luminance`.
  - DXV validation records `status=pass` with full tool/version/hash evidence.
  - Descriptor/RTS0 cross-check evidence shows byte-for-byte RTS0 match.
  - HLSL source SHA-256 and DXC version are recorded in offline compile evidence.
  - `root_constants` layout matches the luminance pass contract (28-byte b0 block when constants are required).

## Revert / Re-cut Conditions

Switch back to Rurix-owned canonical when **all** are true:

1. Patched `llc` supports texture load/store intrinsics and target-ext `dx.Texture2D<float>` / `dx.RWTexture2D<float>`.
2. `src/lib_texture.rx` offline compile records `status=success` with `runtime_mappable=true`.
3. DXV accepts the Rurix-emitted container; provenance flips to `rurix_owned=true`.
4. Bridge tracked digests update to the new Rurix-owned artifact hashes.

Until revert, historical raw-buffer bytes remain at `artifacts/raw_buffer_historical/` for regression continuity.

## Fail-Closed Invariants (unchanged by this policy)

- Default pass enable stays `disabled`; `runtime_state` stays `fallback_only` until a separate real-pass slice succeeds with measured evidence.
- No visual, GPU timestamp, FPS, or performance claims from the policy slice alone.
- Fallback path is never removed.
- `real_pass_enablement_success_evidence.json` must not exist until segment 4h strict measured success.

## Next Slice

When `texture_artifact_provenance_policy.json` is ready, the probe advances to `provide_grx009_runtime_mappable_luminance_kernel_artifact`: supply a math-parity HLSL kernel package (pyramid/EMA/root constants) compiled via DXC, with updated descriptor layout and bridge tracked digests.

## Errata (2026-07-11): toolchain unblocked, achieved form differs from policy spelling

This policy (and `texture_intrinsic_toolchain_blocker.json`) were written against the *self-invented* target-ext spelling `target("dx.Texture2D<float>", 0, 0)` / `target("dx.RWTexture2D<float>", 0, 0)` and the intrinsics `llvm.dx.resource.load.texture.2d` / `store.texture.2d`. That spelling is **abandoned**: no llc ever defined it. The texture toolchain was instead unblocked with the **upstream** DirectX form:

- SRV: `target("dx.Texture", float, 0, 0, 0, 2)`; UAV: `target("dx.Texture", float, 1, 0, 0, 2)`.
- Load: `llvm.dx.resource.load.level` (merged upstream, PR #193343) -> `dx.op.textureLoad(66)`.
- Store: `llvm.dx.resource.store.texture` (local llc patch, `H:/llvm-clean-82c5bce5-src` commit `2afad69a7`, tracking issue #194930; `registry/deferred.json` RD-025) -> `dx.op.textureStore(67)`.

**Revert / Re-cut condition status (append-only; history above unchanged):**

1. Patched `llc` supports texture load/store intrinsics + target-ext types — **SATISFIED** (via the upstream `dx.Texture` form above; read "the intrinsics rurixc emits" as this upstream form, not the abandoned `dx.Texture2D<float>` spelling).
2. `src/lib_texture.rx` offline compile `status=success` with `runtime_mappable=true` — **NOT satisfied**. Compile is `status=success` and DXV-validates a real 2D texture container, but `runtime_mappable` stays **false**: the kernel is single-level only (`math_parity_status=single_level_only`; pyramid/EMA/prev-luminance/WRITE_LUMINANCE deferred), and `runtime_mappable=true` additionally needs a real D3D12 dispatch smoke.
3. DXV accepts the Rurix-emitted container; provenance flips to `rurix_owned=true` — DXV **accepts** (`Validation succeeded.`, `dx.op.textureLoad(66)`/`textureStore(67)`), but provenance is **NOT** flipped because (2) is unmet.
4. Bridge tracked digests update — **NOT done** (no switch).

**Net:** condition 1 satisfied; conditions 2-4 not. Per "switch back only when **all** are true", the canonical switch is **not** performed. Fail-closed is preserved (canonical artifacts, `pass_manifest`, and bridge digests stay on the raw-buffer/hlsl_bridge fallback). Remaining work for the switch: a math-parity full-pyramid kernel + real D3D12 dispatch proof. See `texture_intrinsic_toolchain_blocker.json` `resolution`.
