# GRX Godot Patch Allocation Registry

Industrialized ledger for the `spike/godot-rurix/patches/` stack. GRX-011..022
are developed by multiple agents in parallel off the shared per-pass template
(`spike/godot-rurix/passes/PASS_TEMPLATE.md`). Every patch number and every
`RxGdCaps.flags` capability bit is pre-allocated here so parallel agents never
collide. This file is the single source of truth for patch-number and cap-bit
ownership; keep it in sync (see §4 rules).

- Patch stack home: `spike/godot-rurix/patches/NNNN-rurix-accel-*.patch`
- Applyability checker: `ci/godot_rurix_patch_stack.py` (stacked scratch-copy
  `git apply --check`; the ignored `external/godot-master` snapshot is never
  mutated).
- The Godot side is only ever changed through these patch files; the tracked
  Godot snapshot source is never edited directly.

## 1. Allocated patches (0001-0026, in use)

| Patch | File | Pass / milestone |
| --- | --- | --- |
| 0001 | `0001-rurix-accel-module-scaffold.patch` | luminance_reduction (GRX-009) — module scaffold |
| 0002 | `0002-rurix-accel-luminance-pass-gate.patch` | luminance_reduction — pass gate |
| 0003 | `0003-rurix-accel-luminance-core-callsite-wiring.patch` | luminance_reduction — core call-site wiring |
| 0004 | `0004-rurix-accel-luminance-resource-mapping-scaffold.patch` | luminance_reduction — resource-mapping scaffold |
| 0005 | `0005-rurix-accel-luminance-runtime-binding-preflight.patch` | luminance_reduction — runtime binding preflight |
| 0006 | `0006-rurix-accel-luminance-gated-dispatch-bringup.patch` | luminance_reduction — gated dispatch bring-up |
| 0007 | `0007-rurix-accel-luminance-native-resource-handle-mapping.patch` | luminance_reduction — native resource-handle mapping |
| 0008 | `0008-rurix-accel-luminance-godot-runtime-bridge-recording-smoke.patch` | luminance_reduction — runtime bridge recording smoke |
| 0009 | `0009-rurix-accel-luminance-real-pass-optin.patch` | luminance_reduction — real-pass opt-in |
| 0010 | `0010-rurix-accel-luminance-real-pass-result-writeback.patch` | luminance_reduction — real multi-level pyramid writeback (GRX-009 Wave 2) |
| 0011 | `0011-rurix-accel-tonemap-pass-gate-and-callsite.patch` | tonemap (GRX-010) — pass gate + call-site |
| 0012 | `0012-rurix-accel-tonemap-runtime-resource-binding.patch` | tonemap — runtime resource binding |
| 0013 | `0013-rurix-accel-tonemap-recording-smoke-and-real-pass-optin.patch` | tonemap — recording smoke + real-pass opt-in |
| 0014 | `0014-rurix-accel-ssao-blur-pass-gate-and-callsite.patch` | ssao_blur (GRX-011) — pass gate + call-site |
| 0015 | `0015-rurix-accel-ssao-blur-runtime-resource-binding.patch` | ssao_blur — runtime resource binding |
| 0016 | `0016-rurix-accel-ssao-blur-recording-smoke-and-real-pass-optin.patch` | ssao_blur — recording smoke + real-pass opt-in |
| 0017 | `0017-rurix-accel-taa-resolve-pass-gate-and-callsite.patch` | taa_resolve (GRX-012) — pass gate + call-site |
| 0018 | `0018-rurix-accel-taa-resolve-runtime-resource-binding.patch` | taa_resolve — runtime resource binding |
| 0019 | `0019-rurix-accel-taa-resolve-recording-smoke-and-real-pass-optin.patch` | taa_resolve — recording smoke + real-pass opt-in |
| 0020 | `0020-rurix-accel-particles-copy-pass-gate-and-callsite.patch` | particles_copy (GRX-013) — pass gate + call-site |
| 0021 | `0021-rurix-accel-particles-copy-runtime-resource-binding.patch` | particles_copy — runtime resource binding |
| 0022 | `0022-rurix-accel-particles-copy-recording-smoke-and-real-pass-optin.patch` | particles_copy — recording smoke + real-pass opt-in |
| 0023 | `0023-rurix-accel-cluster-store-pass-gate-and-callsite.patch` | cluster_store (GRX-014) — pass gate + call-site |
| 0024 | `0024-rurix-accel-cluster-store-runtime-resource-binding.patch` | cluster_store — runtime resource binding |
| 0025 | `0025-rurix-accel-cluster-store-recording-smoke-and-real-pass-optin.patch` | cluster_store — recording smoke + real-pass opt-in |
| 0026 | `0026-rurix-accel-material-sorting-telemetry.patch` | material_sorting (GRX-017) — single telemetry-only slice (no D3D12Hooks virtual, no bridge call, no kernel) |
| 0036 | `0036-rurix-accel-fused-post-chain-pass-gate-and-callsite.patch` | fused_post_chain (GRX-019) — pass gate + fusion-first call-site (stacks on the 0026 tip; 0027-0035 reserved for GRX-015/016/018) |
| 0037 | `0037-rurix-accel-fused-post-chain-runtime-resource-binding.patch` | fused_post_chain — runtime resource binding (5 texture native handles + 64-byte b0) |
| 0038 | `0038-rurix-accel-fused-post-chain-recording-smoke-and-real-pass-optin.patch` | fused_post_chain — recording smoke + real-pass opt-in |

> **Wave 4 print-gating revision note (0009/0010/0013/0016/0019/0022 revised
> in place, no number change — §4 rule 2).** The per-dispatch module-side
> `RXGD_GODOT_RUNTIME_<PASS>_REAL_PASS` markers and the call-site
> `RXGD_GODOT_RUNTIME_<PASS>_REAL_PASS_WRITEBACK` scaffold markers are now
> printed ONLY under each pass's harness-only `dispatch_recording_smoke`
> opt-in, so the production `dispatch_real_pass` opt-in path emits zero
> per-dispatch stdout (pass engagement is read from the shim engagement
> counter file / `RXGD_SUMMARY` instead). Semantics are otherwise unchanged.
> The revisions changed those six files' bytes (and hence their sha256), so
> every frozen enablement success evidence that pinned them (GRX-009 segment
> 4h, GRX-010, GRX-011, GRX-012, GRX-013) was invalidated and regenerated by
> Wave 4 scratch rebuilds + strict enablement re-runs; patch numbers are
> unchanged.

> **0010 revision note (GRX-009 Wave 2, no number change).** Patch 0010 was
> revised in place from the level-0 result-writeback *scaffold* to the real
> multi-level luminance *pyramid* writeback defined by
> `spike/godot-rurix/passes/luminance_reduction/hook_contract_v2.md`. It adds a
> NEW default-false `D3D12Hooks::try_record_luminance_pyramid()` virtual carrying
> the ordered `[source, reduce[0..L-1], current, prev]` native-handle array; the
> existing 9-argument `try_record_luminance_reduction()` signature (patch
> 0005/0007) is byte-unchanged. The revision changed the file's bytes (and hence
> its sha256), so the frozen segment-4h success evidence must be regenerated by a
> scratch rebuild; the patch number is unchanged (§4 rule 2).

## 2. Pre-allocated patches (0027-0040+, reserved)

> ssao_blur (GRX-011) 0014-0016, taa_resolve (GRX-012) 0017-0019,
> particles_copy (GRX-013) 0020-0022, cluster_store (GRX-014) 0023-0025, the
> material_sorting (GRX-017) telemetry slice 0026 and fused_post_chain
> (GRX-019) 0036-0038 have landed and moved to §1 (in use). fused_post_chain
> stacks directly on the 0026 tip: its reserved block 0036-0038 is authored
> ahead of the 0027-0035 blocks (gpu_culling / instance_compaction /
> indirect_args, still reserved), which is a legal monotonic hole (§4 rule 2).

Each pass reserves a small contiguous block (typically three: gate+callsite →
runtime binding → recording+real-pass, mirroring the GRX-010 0011/0012/0013
triple). Numbers are reserved even if a pass ends up using fewer; unused
reserved numbers become holes (monotonic, holes allowed — §4).

| Patches | Pass | Milestone | Notes |
| --- | --- | --- | --- |
| 0027-0029 | gpu_culling | GRX-015 | 0027 gate+callsite / 0028 runtime binding / 0029 recording+real-pass opt-in |
| 0030-0032 | instance_compaction | GRX-016 | 0030 gate+callsite / 0031 runtime binding / 0032 recording+real-pass opt-in |
| 0033-0035 | indirect_args | GRX-018 | 0033 gate+callsite / 0034 runtime binding / 0035 recording+real-pass opt-in |
| 0039 | pso_prewarm | GRX-021 | NOT NEEDED — permanent hole. GRX-021 auto-triggers the kernel prewarm from `rxgd_create_d3d12_session` (the bridge session-creation path patch 0001 already routes through), so no Godot-side call site is required. See `spike/godot-rurix/passes/pso_prewarm/pso_prewarm_decision.json` (`patch_0039_status=not_needed`). A future slice may claim 0039 for a Godot-visible prewarm toggle/telemetry surface. |
| 0040+ | bindless | GRX-022 | reserve pool start; allocate concrete numbers only AFTER the bindless RFC is adjudicated |

> Milestone ordering note: the patch blocks are grouped by pass, not strictly by
> GRX number (GRX-017 `material_sorting` is the single 0026 telemetry slice
> placed between the GRX-014 and GRX-015 blocks, now in §1). Follow the
> milestone column, not numeric adjacency.

## 3. `RxGdCaps.flags` capability-bit allocation

Cap bits live in `src/rurix-godot/src/lib.rs` (carried in `RxGdCaps.flags`,
ABI v1 — reusing `flags` bits never changes the C ABI struct layout, so
`RXGD_ABI_VERSION` stays `1`). Bits 0-5 are already defined; bits 6-14 are
pre-allocated for the parallel passes in the order the milestone plan lists
them. A pass's real-pass opt-in patch (its `...16`/`...19`/... slice) is what
first makes the Godot side set its bit; the default Godot config never sets any
of these, and setting a bit never by itself makes the bridge return
`RXGD_STATUS_OK`.

| Bit | Value | Constant (`RXGD_CAP_*`) | Pass | Status |
| --- | --- | --- | --- | --- |
| 0 | `1 << 0` | `SHADER_INT64` | (device capability) | defined |
| 1 | `1 << 1` | `LUMINANCE_DISPATCH_BRINGUP` | luminance_reduction | defined |
| 2 | `1 << 2` | `LUMINANCE_DISPATCH_RECORD` | luminance_reduction | defined |
| 3 | `1 << 3` | `LUMINANCE_REAL_PASS` | luminance_reduction | defined |
| 4 | `1 << 4` | `TONEMAP_REAL_PASS` | tonemap | defined |
| 5 | `1 << 5` | `SSAO_BLUR_REAL_PASS` | ssao_blur | defined |
| 6 | `1 << 6` | `TAA_RESOLVE_REAL_PASS` | taa_resolve (GRX-012) | defined |
| 7 | `1 << 7` | `PARTICLES_COPY_REAL_PASS` | particles_copy (GRX-013) | defined |
| 8 | `1 << 8` | `CLUSTER_STORE_REAL_PASS` | cluster_store (GRX-014) | defined |
| 9 | `1 << 9` | `GPU_CULLING_REAL_PASS` | gpu_culling (GRX-015) | reserved |
| 10 | `1 << 10` | `INSTANCE_COMPACTION_REAL_PASS` | instance_compaction (GRX-016) | reserved |
| 11 | `1 << 11` | `MATERIAL_SORTING_REAL_PASS` | material_sorting (GRX-017) | reserved |
| 12 | `1 << 12` | `INDIRECT_ARGS_REAL_PASS` | indirect_args (GRX-018) | reserved |
| 13 | `1 << 13` | `FUSED_POST_CHAIN_REAL_PASS` | fused_post_chain (GRX-019) | defined |
| 14 | `1 << 14` | `PSO_PREWARM_REAL_PASS` | pso_prewarm (GRX-021) | reserved |
| 15+ | `1 << 15`+ | (reserve pool) | bindless (GRX-022) / future | reserve pool |

> The `RXGD_PASS_*` per-pass id enum in `src/rurix-godot/src/lib.rs` is a
> separate namespace (`CLUSTER_STORE=1`, `SSAO_BLUR=2`, `SSIL_BLUR=3`,
> `LUMINANCE_REDUCTION=4`, `TONEMAP=5`, `TAA_RESOLVE=6`, `PARTICLES_COPY=7`,
> `GPU_CULLING=8`, `INDIRECT_ARGS=9`, `FUSED_POST_CHAIN=10`, ...). Do not confuse
> a pass id with its cap bit; allocate any new `RXGD_PASS_*` id in that enum, not
> here.

## 4. Rules (normative)

1. **Single stack-lock holder.** The right to append to the patch stack (add or
   modify any `NNNN-*.patch`) is held by exactly ONE agent at a time. Acquire
   the stack-lock before generating patches; release it when your slice lands.
   Stages S1-S4 and S6 of `PASS_TEMPLATE.md` are cross-pass parallel and need no
   lock; the patch-authoring stages S5 and S7 are serialized by this lock.
2. **Monotonic numbering, holes allowed.** Patch numbers only ever increase.
   Never renumber or reuse a number. A pass that uses fewer patches than its
   reserved block leaves the unused numbers as permanent holes (e.g. GRX-021 may
   use only 0039 and leave nothing else).
3. **Overflow uses the reserve pool, atomically.** If a pass needs more patches
   than its reserved block, take the next free number(s) from the reserve pool
   (§2, `0040+` / cap bit 15+). Any change to THIS registry (claiming a reserve
   number or cap bit) MUST land in the SAME commit as the patch that consumes
   it — the ledger and the stack never diverge.
4. **Patches are generated, never hand-written.** Every patch MUST be produced
   by `git diff --no-index` (or an equivalent generated diff) against a scratch
   copy of the Godot snapshot with ALL prior patches in the stack already
   applied. Do not hand-edit hunks. Verify with
   `py -3 ci/godot_rurix_patch_stack.py` (stacked applyability on a temporary
   scratch copy; the real `external/godot-master` snapshot is never touched).
5. **Cap bits are append-only.** Claim the next free `RxGdCaps.flags` bit from
   §3 in milestone order; never reuse or renumber a bit (reusing a bit is an ABI
   hazard even though the struct layout is unchanged). Reusing a `flags` bit
   keeps `RXGD_ABI_VERSION = 1`; a real struct-layout change would require an ABI
   bump and is out of scope for these passes.
