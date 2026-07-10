# GRX-009 Dxc Texture Artifact Bridge Contract

## Scope

This document defines the design contract for turning the segment 4k DXC/DXV Texture2D feasibility result into a future Rurix-owned, runtime-mappable texture artifact package. This slice is design-only. It does not replace the canonical luminance artifact, does not make `offline_compile_evidence.json` successful, does not set `runtime_mappable=true`, does not enable `dispatch_real_pass`, does not make `real_gpu_pass=true`, and does not claim visual, GPU timestamp, FPS, or performance success.

The segment 4k feasibility artifact proves only that a minimal HLSL compute shader using `Texture2D<float>` and `RWTexture2D<float>` can be compiled by DXC and accepted by DXV. That HLSL bridge output is not yet the canonical Rurix luminance package because it lacks a Rurix-owned root signature artifact, a Rurix-owned descriptor layout contract, and Rurix source provenance.

## Root Signature Strategy

The future artifact package must produce Rurix-owned `RTS0` root signature bytes before it can become runtime-mappable. The scaffold may use either DXC container reflection/extraction or Rurix descriptor-layout synthesis as an input, but the accepted package contract is Rurix-owned output plus explicit cross-checks.

The preferred implementation path is Rurix descriptor-layout synthesis: derive the root signature from the package descriptor layout, emit `RTS0` bytes owned by the Rurix artifact pipeline, and record the hash in package evidence. If DXC container reflection or root signature extraction is available, it is a cross-check input only. Reflection/extraction cannot by itself make the artifact Rurix-owned.

Root signature consistency checks must compare root parameter count and order, descriptor range type, register, space, descriptor count, shader visibility, root constant parameter index, root constant byte count, root constant DWORD count, and root constant DWORD offsets. Any mismatch is `validation_failed` and must fail closed before PSO creation or runtime binding.

The minimal segment 4k feasibility shader currently has no Rurix-owned `RTS0` artifact. Its evidence correctly records `root_signature_expectation.rurix_owned_rts0_available=false`, so it can feed this design but cannot become the canonical luminance artifact.

## Descriptor Layout Synthesis

The texture artifact package descriptor layout for the first scaffolded luminance level is fixed as follows:

| Resource | Register | Space | Class | Binding kind | Runtime resource |
| --- | --- | --- | --- | --- | --- |
| `src_luminance` | `t0` | `space0` | SRV | `texture2d` | Godot source `Texture2D` `ID3D12Resource*` |
| `dst_luminance` | `u0` | `space0` | UAV | `rwtexture2d` | Godot destination `RWTexture2D` `ID3D12Resource*` |

If the texture artifact package carries luminance constants, they must use the existing `b0 space0` root constant layout:

| Field | Type | DWORD offset | DWORD size |
| --- | --- | --- | --- |
| `source_width` | `i64` | 0 | 2 |
| `source_height` | `i64` | 2 | 2 |
| `max_luminance` | `f32` | 4 | 1 |
| `min_luminance` | `f32` | 5 | 1 |
| `exposure_adjust` | `f32` | 6 | 1 |

The total root constant size is 7 DWORDs / 28 bytes. Register, space, packing, order, and byte size must match `resource_mapping.md` and the artifact descriptor layout exactly.

If a minimal DXC feasibility shader has no constants, package metadata must explicitly record `root_constants=none`. Such a no-constant shader may be used only as feasibility evidence or a scaffold input; it must not be promoted as the canonical luminance artifact because it does not satisfy the luminance pass contract.

## Binding Kind Mapping

Godot `Texture2D` `ID3D12Resource*` handles must map to texture resources. They must never be rebound to a raw-buffer shader declaration.

The bridge mapping remains:

| Runtime resource type | Binding kind |
| --- | --- |
| `RXGD_RESOURCE_TEXTURE` | `texture2d` for SRV usage or `rwtexture2d` for UAV usage according to descriptor slot |
| `RXGD_RESOURCE_BUFFER` | `raw_buffer_view` |

The current canonical tracked package is still the fail-closed raw-buffer fallback: both canonical descriptor resources record `binding_kind="raw_buffer_view"`. That package cannot bind Godot Texture2D handles as a real pass. Any design-ready check must reject a silent canonical descriptor change from `raw_buffer_view` to texture binding kinds during this design slice.

## DXIL Validation Metadata

Every future texture artifact package must record enough DXIL validation metadata to reproduce and audit the container:

- `dxc.exe` path and version output.
- `dxv.exe` path and version/help output.
- Compile argv, exit code, stdout path or bounded summary, and stderr path or bounded summary.
- Validation argv, exit code, stdout path or bounded summary, and stderr path or bounded summary.
- DXIL container path, size, SHA-256, artifact kind, and whether it was produced by the current package run.
- Target profile and entry point.
- Source path and source SHA-256 for HLSL bridge inputs or Rurix source inputs.
- Root signature hash and descriptor layout hash when those artifacts exist.

DXV failure, missing tool metadata, missing container hash, stdout/stderr evidence drift, or a container hash mismatch must fail closed.

## Rurix Provenance

Only an artifact generated from Rurix source through the Rurix compiler/offline artifact pipeline may set `provenance.rurix_owned=true`.

If the package still uses the segment 4k HLSL bridge workaround, it must state that explicitly with `provenance="hlsl_bridge_workaround"`, `rurix_owned=false`, and either `runtime_mappable=false` or `design_only=true`. A DXC/DXV-pass HLSL bridge artifact may inform the scaffold but is not the final Rurix-owned artifact.

The future scaffold may temporarily carry HLSL bridge output as a non-final package input only when the evidence says it is not canonical, not runtime-mappable, and not a real GPU pass.

## Canonical Switch Conditions

Switching the canonical artifact from raw-buffer fallback bytes to a texture artifact package is allowed only after all conditions below are true in the same evidence chain:

- `offline_compile_evidence.json` is backed by a real texture package and records `status=success`.
- `runtime_mappable=true` is supported by an actual DXIL container, Rurix-owned `RTS0` root signature bytes, and descriptor layout artifacts.
- DXV validation records `validation.status=pass`, the validated container SHA-256, profile, entry point, tool paths, tool versions, and stdout/stderr evidence.
- Descriptor resources record `binding_kind` values `texture2d` for `src_luminance` and `rwtexture2d` for `dst_luminance`.
- Root signature synthesis and any DXC reflection/extraction cross-check agree on parameter order, descriptor ranges, register/space, visibility, and root constant packing.
- `provenance.rurix_owned=true` and the source hash points to Rurix source or an accepted Rurix-owned bridge source policy.
- Visual/fallback evidence required by the real-pass gate exists and stays green; missing visual/fallback evidence prevents runtime promotion.
- Probe red/green regression proves no success file, manifest runtime state, or canonical binding kind was silently advanced by the design slice.
- Runtime remains `fallback_only` until a separate real-pass enablement slice explicitly changes it with measured evidence.

Until every condition is satisfied, the canonical `artifacts/luminance_reduction.dxil`, `artifacts/luminance_reduction.rts0.bin`, and `artifacts/luminance_reduction_descriptor_layout.json` paths must remain the current raw-buffer historical fallback bytes.

### Owner-Approved Exception (Segment 4l Provenance Policy)

[`texture_artifact_provenance_policy.md`](texture_artifact_provenance_policy.md) records an owner-approved exception to the `provenance.rurix_owned=true` condition above: while patched `llc` lacks `llvm.dx.resource.load/store.texture.2d` support, an HLSL bridge DXIL container with `provenance=hlsl_bridge_workaround` and `rurix_owned=false` may become the **temporary runtime-mappable canonical** package, provided the RTS0 bytes stay Rurix-synthesized from the package descriptor layout, the descriptor records `binding_kind=texture2d`/`rwtexture2d`, DXV validation passes with full tool/version/hash evidence, the descriptor/RTS0 cross-check stays byte-for-byte green, and the HLSL source SHA-256 plus DXC version are recorded. The exception is tracked by `texture_artifact_provenance_policy.json` (`policy_ready=true`) and reverts to the Rurix-owned canonical path under the revert conditions listed in the policy document. Every other Canonical Switch Condition and all Fail Closed Conditions remain in force; the policy alone never sets `runtime_mappable=true`, never enables `real_gpu_pass`, and never replaces the canonical artifact by itself.

## Fail Closed Conditions

The bridge contract must fail closed for all of the following:

- Root signature mismatch between synthesized Rurix RTS0 and reflected/extracted DXC container data.
- Descriptor layout mismatch, including register, space, descriptor kind, resource order, or root constant packing.
- DXIL validation failed, skipped when required, or missing validation metadata.
- Non-Rurix provenance for a package claiming to be final or runtime-mappable.
- Missing visual/fallback evidence for any runtime promotion.
- Missing or mismatched container, root signature, descriptor layout, source, stdout, or stderr hashes.
- Canonical descriptor binding kinds changed from raw-buffer fallback during this design slice.
- `offline_compile_evidence.json` changed to success during this design slice.
- `runtime_mappable=true`, `real_gpu_pass=true`, or `runtime_state` changed away from `fallback_only` during this design slice.
- `real_pass_enablement_success_evidence.json` exists before the separate real-pass enablement slice succeeds.

The fail-closed result must preserve the native Godot luminance path, keep the pass default disabled, and avoid all visual, GPU timestamp, FPS, and performance claims.

## Implementation Scaffold Entry Criteria

The next action may become `implement_grx009_dxc_texture_artifact_bridge_scaffold` only when the design document, design evidence JSON, manifest design marker, and probe design-ready gate all agree that this is design-ready and runtime-closed. The scaffold implementation must start from this contract and still keep the canonical artifact raw-buffer until a later package evidence round satisfies the canonical switch conditions.
