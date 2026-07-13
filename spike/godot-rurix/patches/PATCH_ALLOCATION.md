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
| 0027 | `0027-rurix-accel-gpu-culling-pass-gate-and-callsite.patch` | gpu_culling (GRX-015) — pass gate + additive collect call-site (no native dispatch to wrap; collects `INSTANCE_DATA_FLAG_MULTIMESH_INDIRECT` bases in `render_forward_clustered.cpp` after `_fill_instance_data(RENDER_LIST_ALPHA)`) |
| 0028 | `0028-rurix-accel-gpu-culling-runtime-resource-binding.patch` | gpu_culling — runtime resource binding (3 structured buffers src_transforms/dst_commands/dst_visibility + 144-byte Rurix b0 with frustum planes normal-negated `n_rurix=-n_godot, d_rurix=plane.d`; Rurix-owned visibility bitmask cache) |
| 0029 | `0029-rurix-accel-gpu-culling-recording-smoke-and-real-pass-optin.patch` | gpu_culling — recording smoke + pre-dispatch zeroing (count dwords `(s*5+1)*4` + bitmask) + real-pass opt-in (cap bit 9) |
| 0036 | `0036-rurix-accel-fused-post-chain-pass-gate-and-callsite.patch` | fused_post_chain (GRX-019) — pass gate + fusion-first call-site (stacks on the 0026 tip; 0030-0035 reserved for GRX-016/018) |
| 0037 | `0037-rurix-accel-fused-post-chain-runtime-resource-binding.patch` | fused_post_chain — runtime resource binding (5 texture native handles + 64-byte b0) |
| 0038 | `0038-rurix-accel-fused-post-chain-recording-smoke-and-real-pass-optin.patch` | fused_post_chain — recording smoke + real-pass opt-in |
| 0040 | `0040-rurix-accel-tonemap-rd-native-inframe-replacement.patch` | tonemap **Route B rd_native** (first non-scaffold real replacement) — single slice: new `try_record_tonemap_rd_native(RID,RID,Size2i,Size2i,f32,f32,f32)` virtual + three-state `passes/tonemap/backend` selector (0=disabled/1=shim/2=rd_native) + `passes/tonemap/rd_container_path` + module RD-native pipeline (lazy `shader_create_from_bytecode`→`compute_pipeline_create`, `UniformSetCacheRD` bind, 28-byte b0, in-frame `compute_list` dispatch). Bridge-independent (no rxgd session, no `RxGdCaps.flags` bit). Stacks on the **culling tail 0001-0029** (branch HEAD). |
| 0041 | `0041-rurix-accel-ssao-blur-rd-native-inframe-replacement.patch` | ssao_blur **Route B rd_native** (second non-scaffold real replacement) — new `try_record_ssao_blur_rd_native(int64_t compute_list,RID,RID,Size2i,f32,f32,f32)` virtual + three-state `passes/ssao_blur/backend` selector + `passes/ssao_blur/rd_container_path` + module RD-native pipeline. Records onto the ALREADY-OPEN `generate_ssao` compute list (does NOT begin/end its own — the SSAO list is opened once around gather/blur/interleave); only the SMART blur pipeline slices route through rd_native (`blur_pipeline == SSAO_BLUR_PASS_SMART`). t0 SRV / u0 UAV, 28-byte b0 [i64 slice_width, i64 slice_height, f32 edge_sharpness, f32 hspx, f32 hspy]. Bridge-independent (no cap bit). Stacks on **0001-0029 + 0040**. Enablement strict MEASURED success (LDR max_abs=0). |
| 0042 | `0042-rurix-accel-taa-resolve-rd-native-inframe-replacement.patch` | taa_resolve **Route B rd_native** (third non-scaffold real replacement) — new `try_record_taa_resolve_rd_native(RID color,RID depth,RID velocity,RID prev_velocity,RID history,RID temp,Size2i,f32,f32)` virtual + three-state `passes/taa_resolve/backend` selector + `passes/taa_resolve/rd_container_path` + module RD-native pipeline (six resources t0..t4 SRV / u0 UAV binding 5, 28-byte b0). Injects INSIDE `TAA::process` (taa.cpp), replacing ONLY the `resolve()` compute dispatch; the native history-maintenance copies (temp→internal, internal→history, velocity→prev_velocity) still run so the temporal feedback loop is preserved. Own `compute_list` (resolve() is a standalone list). Bridge-independent (no cap bit). Stacks on **0001-0029 + 0040 + 0041**. Enablement strict MEASURED success (8-frame temporal, worst max_abs=1). |
| 0043 | `0043-rurix-accel-particles-copy-rd-native-inframe-replacement.patch` | particles_copy **Route B rd_native** (fourth real replacement; FIRST buffer-path rd_native) — new `try_record_particles_copy_rd_native(int64_t compute_list,RID src_particles,RID dst_instances,uint32_t total_particles,const uint8_t*,uint32_t)` virtual + three-state `passes/particles_copy/backend` selector + `passes/particles_copy/rd_container_path` + module RD-native pipeline. Records onto the ALREADY-OPEN `particles_set_view_axis` fill-instances compute list (does NOT begin/end its own). Two STRUCTURED buffers (src_particles=SRV t0, dst_instances=UAV u0) bound as `UNIFORM_TYPE_STORAGE_BUFFER` (RAW≡structured, buffer probe zero-tolerance), 128-byte CopyPushConstant b0, `dispatch_threads(total_particles,1,1)`. Only the plain 3D no-userdata fill-instances subset (`!do_sort && copy_mode_2d==0 && userdata_count==0`) routes; sort/2D/userdata stay native. No usage-bits preflight (buffers have no `texture_get_format`). Bridge-independent (no cap bit). Stacks on **0001-0029 + 0040 + 0041 + 0042**. **Revised in place（号不变，§4 rule 2）during enablement bring-up**: 加 `userdata_count == 0` 子集门——实测根因 = kernel 的「no userdata」缺口是 STRIDE 边界（userdata_count>0 → ParticleData stride 112→112+16u；标准 `ParticleProcessMaterial` 恒用 USERDATA1 → stride 128），未门控时 kernel 逐字节错位读出 1/8 对齐稀疏图（首轮 max_abs=191），local-RD stride probe 以 CPU 错读模型 0/1280 逐字节复现 GPU 输出定罪。修订后 enablement strict MEASURED success（no-userdata 自定义 particles shader 场景，LDR max_abs=0 逐字节一致，真跳过 native fill-instances dispatch）。sha256 `fcb74373…bef0d1ca`. |
| 0044 | `0044-rurix-accel-cluster-store-rd-native-inframe-replacement.patch` | cluster_store **Route B rd_native** (fifth real replacement; **GRX-014 scaffold→real**) — new `try_record_cluster_store_rd_native(int64_t compute_list,RID cluster_render,RID render_elements,RID cluster_store,Size2i screen_size,const uint8_t*,uint32_t)` virtual + three-state `passes/cluster_store/backend` selector + `passes/cluster_store/rd_container_path` + module RD-native pipeline. Records onto the ALREADY-OPEN `bake_cluster` store compute list (naturally same-list, does NOT begin/end its own — cleanest of the batch). Three STRUCTURED buffers in Rurix binding order (cluster_render=t0, render_elements=t1, cluster_store=u0; Godot native set-0 numbers 1/3/2 differ) as `UNIFORM_TYPE_STORAGE_BUFFER`, 32-byte ClusterStore::PushConstant b0, `dispatch_threads(screen.x,screen.y,1)`. Only the compute merge (store) segment routes; the raster segment, buffer clears, and count==0 early-out stay native. Turns GRX-014 from the 0025 scaffold into a genuine replacement (native store dispatch skipped). Bridge-independent (no cap bit). Stacks on **0001-0029 + 0040..0043**. sha256 `a6417953…c98f60c4e`. |
| 0046 | `0046-rurix-accel-gpu-culling-rd-native-inframe-replacement.patch` | gpu_culling **Route B rd_native** (seventh slice; FIRST rd_native pass that itself issues `RD::buffer_clear`) — new `try_record_gpu_culling_rd_native(RID transforms,RID commands,RID visibility,const uint8_t*frustum_planes,uint32_t,const uint8_t*b0,uint32_t,uint32_t instance_count,uint32_t surface_count,uint32_t command_stride_dwords,uint32_t instance_count_dword_index,uint32_t visibility_bytes)` virtual + three-state `passes/gpu_culling/backend` selector (0=disabled/1=shim/2=rd_native, INDEPENDENT of the shim 0027-0029 `enabled` gate which is byte-unchanged) + `passes/gpu_culling/rd_container_path` + module RD-native pipeline. The module `buffer_update`s a self-owned 96-byte frustum-planes SSBO (the t1 `StructuredBuffer<float4>`), `buffer_clear`s each surface's instance-count dword `(s*stride+index)*4` and the whole visibility bitmask **BEFORE** opening its OWN `compute_list` (buffer_clear is hard-forbidden while a compute list is active), binds four structured buffers (src_transforms SRV t0 / frustum_planes SRV t1 / dst_commands UAV u0 / dst_visibility UAV u1 as `UNIFORM_TYPE_STORAGE_BUFFER`), packs the 48-byte b0, and dispatches `ceil(instance_count/64)` on the main RenderingDevice. Additive pass (no native compute replaced); a failed record is a no-op (CPU count survives). **Container line (b0 144B→48B, off the CBV dead path)**: the 6 frustum planes move from b0 root constants into the t1 `StructuredBuffer<float4>` so b0 fits the RD 128-byte window (the shim 144B b0 is rejected by generate_rd_container.py as `push_constant_too_large` and produces no container). New HLSL `passes/gpu_culling/artifacts/hlsl_bridge/gpu_culling_rd_native.hlsl` (DXC cs_6_0 + DXV pass) + Rurix-owned RTS0 via `src/rurixc/examples/emit_grx015_gpu_culling_rd_native_rts0.rs` + descriptor layout `gpu_culling_rd_native_descriptor_layout.json` + generate_rd_container.py PASS_REGISTRY entry `gpu_culling_rd_native` (`artifacts_pass`/`layout_name` override → co-located artifacts) + `out/gpu_culling_rd_native.rd_container.bin` (verify_container 59/59). New evidence face: `ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py` (multi-leg matrix + garbage-container fail-closed red leg + **no-device-removal judgement** of the clear→dispatch→indirect-draw chain + picture-preservation zero-tolerance visual). Bridge-independent (no cap bit). Stacks on **0001-0029 + 0040..0045** (needs 0045→0048 in the build). **Revised in place (§4 rule 2, no number change) to R1**: the first shipped 0046 (buffer_clear+compute-UAV on the LIVE `multimesh->command_buffer`) REMOVED THE DEVICE (`0x887A0005`, same-frame dual-role GPU-side fault, convicted empirically + silent under GBV). R1 drives a caller-owned SCRATCH indirect buffer instead — module `buffer_copy`s the live template into the scratch, `buffer_clear`s+dispatches ONLY the scratch, and the draw site retargets `draw_list_draw_indirect` to the scratch; the live buffer is copy-read only. Hook signature gains `RID p_scratch_commands`+`uint32_t p_command_bytes`; adds `render_forward_clustered.h` (2 cache maps). No container/b0/RTS0/kernel change. **R1 is a CANDIDATE** — device-removal-free enablement smoke on rb4 is the final verdict; `real_gpu_pass=false`/default-disabled until then. See the Wave 6 revision #2 note. sha256 `925840a0be2456ec…`. |
| 0047 | `0047-rurix-accel-particles-view-axis-render-timestamp.patch` | particles telemetry micro-patch — adds `RENDER_TIMESTAMP("Particles View-Axis Copy")` after the two early-out checks of `ParticlesStorage::particles_set_view_axis()` and the Godot `"< "` region-end close at the function tail. That call site (distinct from the `_particles_process` "Update GPUParticles" path) had no timestamp, so its view-axis fill-instances/sort dispatches were invisible to the RENDER_TIMESTAMP per-pass timing. Pure telemetry: no control-flow change, no new dispatch, no cap bit. Stacks on **0001-0029 + 0040..0046**. |
| 0048 | `0048-rurix-accel-luminance-readonly-getters.patch` | Luminance read-only getters — FIRST patch to touch `servers/rendering/renderer_rd/effects/luminance.{h,cpp}`. Two pure accessors mirroring `get_current_luminance_buffer`: `get_previous_luminance_buffer` (returns `reduce[size-1]`, which after the SWAP in `luminance_reduction` holds last frame's final — the patch 0010 mirror semantics) and `get_luminance_reduce_penultimate_buffer` (returns `reduce[size-2]`, a safe read window). Both return `RID()` when the buffers / required reduce levels are absent (fail-closed at the caller's is_valid). Unlocks the fused Design 1 shadow-recompute (patch 0045 revision) by exposing distinct luminance-final buffers. Zero behavior change: `luminance_reduction`/SWAP/`configure` untouched, no new dispatch/resource, no cap bit. Stacks on **0001-0029 + 0040..0047**. |
| 0045 | `0045-rurix-accel-fused-post-chain-rd-native-inframe-replacement.patch` | fused_post_chain **Route B rd_native** (sixth slice; FUSION-FIRST leg) — new `try_record_fused_post_chain_rd_native(RID src_color,RID lum_source,RID prev_luminance,RID dst_color,RID dst_luminance,Size2i src,Size2i dst,f32 exposure,f32 white,f32 lum_mult)` virtual + three-state `passes/fused_post_chain/backend` selector + `passes/fused_post_chain/rd_container_path` + module RD-native pipeline with its OWN `RD_NATIVE_FUSED_POST_CHAIN_*` 64-byte/5-resource constants (distinct from the fused-tail `FUSED_POST_CHAIN_RESOURCE_MAPPING_*` so the two tails never collide). **SELF-CONTAINED on the culling tail**: the fused shim scaffold 0036-0038 is on the mutually-exclusive fused tail and is NOT in this stack, so 0045 cascades into the patch **0040 tonemap rd_native** as the single-pass fallback (two-level: fused rd_native → tonemap rd_native → native). Five textures (t0..t2 SRV / u0/u1 UAV), own `compute_list`. **FAILS CLOSED (honest measured boundary)** when `lum_source==prev_luminance` or `dst_color==dst_luminance`: the distinct double-buffered luminance-final targets are unexposed by the public Luminance API at the tonemap call site (deferred Luminance-API extension, PASS_CONTRACT 3.4/5), so on the culling tail it cascades to tonemap rd_native today and lifts automatically once distinct targets are supplied. Bridge-independent (no cap bit). Stacks on **0001-0029 + 0040..0044**. **Revised in place a 2nd time (§4 rule 2, no number change): real auto-exposure parameter pipeline** — the five placeholder AE b0 scalars are replaced by the real per-frame values via a +5-float hook-signature extension (call site hoists the reduce params); no b0/DXIL/container/kernel change; closes the measured `max_abs=85/mean_abs=66` gap. See the Wave 6 revision #2 note. sha256 `418406b2…` → `a783c2f752fbd1c7…`. |

> **Route B rd_native lineage / double-tail note (0040).** Patch 0040 opens the
> Route B rd_native series. It stacks on the **gpu_culling culling tail
> (0001-0029)**, the branch-HEAD lineage. The two existing tails —
> gpu_culling (0027-0029) and fused_post_chain (0036-0038) — were BOTH authored
> against the 0026 tip and insert at the SAME module anchors
> (`d3d12_hooks.h` virtual list, `register_types.cpp` settings block,
> `rurix_accel.{h,cpp}` pass-id/member/method decls), so they are **mutually
> exclusive under strict `git apply`**: neither `0001-0029,0036-0038` nor
> `0001-0026,0036-0038,0027-0029` assembles (whichever tail is second fails on
> the shared anchors). This is a pre-existing condition, not introduced by 0040.
> The combined stack `0001-0029+0036-0038+0040` the plan targeted is therefore
> **not applyable with the frozen patches**; 0040 is validated on the maximal
> FEASIBLE stack `0001-0029+0040` (culling tail). 0040 does not apply on the
> fused tail either, as its hunks anchor on culling-tail (gpu_culling) context.

> **Double-tail decision (0041/0042 batch, 2026-07-13): KEEP the double tail;
> do NOT rebase fused onto the culling tail.** The Route B first-batch task
> considered unifying to a single linear stack by rebasing 0036-0038 onto the
> 0029 tail. Decision after evaluation: keep the two tails independent and land
> the rd_native series (0040/0041/0042) on the branch-HEAD **culling tail
> (0001-0029)**, which is the maximal feasible Route-B stack. Rationale: (1) the
> double tail is a pre-existing, already-documented condition, not introduced by
> this batch; (2) 0040 already validated on the culling tail and 0041/0042
> extend it additively; (3) rebasing 0036-0038 would change their sha256 and
> invalidate the frozen fused enablement success evidence (`grx019` pins those
> shas), forcing a full GPU re-run of the fused enablement — a cost
> disproportionate to this copy-stage batch and outside its scope; (4) the
> fused tail's own rd_native/benchmark work can proceed on its tail
> independently. The single shared Route B scratch build for this batch is
> `0001-0029 + 0040 + 0041 + 0042` (32 patches; `check-only` stacked-applyable,
> and a real incremental SCons build linked cleanly). A future full-stack
> benchmark that needs fused + rd_native in ONE linear stack is the trigger to
> revisit the rebase (owner-scoped), at which point the fused enablement must be
> re-signed.

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

> **Wave 6 rd_native revision note (0040 + 0045 revised in place, no number
> change — §4 rule 2).** Two Route B slices were revised in place while landing
> the Wave 6 batch (0046-0048):
> - **0040 (tonemap rd_native): config-signature fail-closed mode guard.** The
>   backend==2 tonemap gate now ANDs a shared
>   `rurix_tonemap_linear_subset_compatible` predicate (read off the assembled
>   `tonemap` settings: LINEAR mode + SDR `convert_to_srgb` + no glow/FXAA/BCS/
>   color-correction/debanding/multiview) plus `!use_auto_exposure`, mirroring the
>   bridge-side GRX-010 whitelist (`src/rurix-godot/src/lib.rs`
>   `TONEMAP_KERNEL_MATH_PARITY_STATUS`). This is a **correctness fix**: a FILMIC
>   (or any non-LINEAR / effect-enabled / HDR) scene previously would have engaged
>   rd_native and written LINEAR pixels; it now fails closed to the native
>   tonemapper. The shared predicate deliberately excludes auto-exposure so patch
>   0045's fused gate (which folds auto-exposure in) can reuse it.
> - **0045 (fused_post_chain rd_native): Design 1 shadow-recompute unlocked by
>   0048.** The fused gate now binds THREE distinct luminance buffers via the new
>   patch 0048 getters (lum_source = current, prev_luminance = previous, and a
>   distinct penultimate handle for dst_luminance) and the module redirects the
>   fused luminance-final WRITE (u1) to a self-owned 1x1 R32F STORAGE scratch that
>   is never read back. The fused gate also ANDs the shared
>   `rurix_tonemap_linear_subset_compatible` predicate. Prior text
>   (`measured_blocked_by_design`, the aliasing fail-closed cascade) is superseded:
>   the fused gate now genuinely records. **Honest boundary (contract-normative):**
>   this is a shadow-recompute — the native `luminance_reduction` still runs in
>   FULL, the net dispatch saving is ZERO, and NO structural fusion / dispatch
>   saving is claimed (`engaged / shadow-luminance-write / dispatch-savings-not-
>   claimed`). Only the LINEAR + convert_to_srgb tonemap leg (t0→u0) is the real
>   replacement. A true fusion (Design 2: glow-off gate + skip the native final
>   level + external SWAP) is a later batch.
>
> Both revisions changed those two patches' bytes (and hence their sha256), so the
> **frozen rd_native strict-success evidence that pins them is invalidated and must
> be re-signed by a scratch rebuild + strict enablement re-run** (patch numbers
> unchanged). Affected frozen evidence (do NOT edit the success files by hand —
> re-sign on rebuild): the tonemap rd_native success (0040) and the fused_post_chain
> rd_native success (0045). See the per-pass `rd_native_enablement_success_evidence.json`
> under `spike/godot-rurix/passes/tonemap/` and `.../fused_post_chain/`, plus any
> `grx_rb_*` gate whose `PATCH_ORDINALS` includes 0040/0045 (`grx_rb_cluster_store_*`
> pins 0040 via the two-level cascade; `grx_rb_fused_post_chain_*` pins 0045). The
> new `grx_rb_gpu_culling_rd_native` gate (0046) and the Wave 6 rebuild will re-sign
> all of them in one pass.

> **Wave 6 revision #2 note (0045 + 0046 revised in place, no number change).**
> After the gpu_culling rd_native device-removal root cause was convicted (see
> `passes/gpu_culling/rd_native_device_removal_diagnosis.md`) and the fused
> rd_native AE parity gap was diagnosed (`passes/fused_post_chain/fused_ae_
> parameter_pipeline_design.md`), 0045 and 0046 were each revised in place under
> §4 rule 2 (number-preserving in-place revision):
> - **0045 (fused_post_chain rd_native): real auto-exposure parameter pipeline
>   (revision #2).** The measured `rd_native` parity gap `max_abs=85/mean_abs=66`
>   was root-caused to the five PLACEHOLDER AE b0 scalars (max=1/min=0/adjust=1/
>   first_frame=0/scale=1) — a ~2.5x uniform exposure error. Revision #2 extends
>   `try_record_fused_post_chain_rd_native(...)` with five trailing floats
>   (`p_min_luminance, p_max_luminance, p_exposure_adjust, p_first_frame,
>   p_auto_exposure_scale`) and packs the REAL per-frame values (the exact
>   `camera_attributes_get_auto_exposure_*` + `set_immediate` arguments the native
>   `luminance_reduction`+tonemap consume) into b0 dwords 8-15. The call site
>   (`renderer_scene_render_rd.cpp`) hoists four function-scope copies of the reduce
>   parameters next to the existing `auto_exposure_scale` and assigns them inside
>   the auto-exposure block. **No b0 layout / DXIL / RTS0 / container / kernel
>   change** — dwords 8-15 already carried these fields; only the packed runtime
>   values change. HONEST BOUNDARY (shadow-recompute, dispatch-savings-not-claimed)
>   is unchanged. sha256 `418406b2…` → `a783c2f7…`.
> - **0046 (gpu_culling rd_native): R1 Rurix-owned scratch indirect buffer.** The
>   first-shipped 0046 removed the D3D12 device (`0x887A0005`) because it
>   `buffer_clear`+compute-UAV-wrote Godot's LIVE `multimesh->command_buffer` while
>   the same frame's native `draw_list_draw_indirect` read it as an indirect
>   argument (a same-frame dual-role GPU-side execution fault; convicted empirically
>   — an empty kernel + 1-instance scene both still removed the device, and it is
>   SILENT under both the debug layer and GPU-Based Validation). R1 decouples the
>   dual role onto a caller-owned SCRATCH indirect-args buffer
>   (`rurix_cull_rdn_indirect_args_buffers`, `storage_buffer_create` with
>   `DISPATCH_INDIRECT`, sized like the live buffer): the module `buffer_copy`s the
>   live command template into the scratch (a READ of the live buffer), then
>   `buffer_clear`s + compute-writes ONLY the scratch, and the draw site
>   (`_render_list_template`) retargets `draw_list_draw_indirect` to the scratch for
>   bases whose cull recorded this frame (per-frame `rurix_cull_rdn_active_scratch`
>   map). The live command buffer is never compute/clear-touched. The hook signature
>   gains `RID p_scratch_commands` + `uint32_t p_command_bytes`; a new file
>   (`render_forward_clustered.h`) gains the two cache maps. **No container / b0 /
>   RTS0 / kernel change.** CAVEAT: R1 is a CANDIDATE — if the fault is a general RDG
>   gap for any compute-written draw-indirect buffer consumed the same frame, a
>   fully-Rurix-owned buffer hits the same wall; R1 ships only after the
>   `grx_rb_gpu_culling_rd_native` enablement smoke shows NO device removal on a
>   rebuilt exe. Until then `real_gpu_pass=false`, default-disabled. sha256 (first
>   shipped) → `925840a0…`.
>
> Both revisions changed those two patches' bytes (and hence their sha256), so ALL
> SEVEN rd_native passes' evidence `patch_stack_identity` is now mismatched (0045
> and 0046 sit inside every rd_native gate's `0001-0029 + 0040-0048` stack). The
> frozen strict-success evidence they pin is invalidated and must be re-signed by a
> **single rb4 scratch rebuild + strict enablement re-run across all seven passes**
> (patch numbers unchanged; do NOT edit the success files by hand). This is the same
> re-sign pass the Wave 6 note above already schedules; it now also carries the
> 0046 R1 device-removal verdict (the FINAL check of whether R1's premise holds).

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
| 0040-0049 | Route B rd_native | GRX Route B | RD-native in-frame compute replacement series. tonemap rd_native = **0040**, ssao_blur rd_native = **0041**, taa_resolve rd_native = **0042** (texture passes, all strict MEASURED success). Second batch (buffer/fused passes): particles_copy rd_native = **0043** (strict MEASURED success after the in-place userdata-stride-gate revision; LDR max_abs=0), cluster_store rd_native = **0044** (strict MEASURED success; **GRX-014 scaffold→real confirmed on hardware**, LDR max_abs=0), fused_post_chain rd_native = **0045** (fusion-first; revised in place to the Design 1 shadow-recompute — see the Wave 6 rd_native revision note — genuinely records with a scratch-redirected luminance write; dispatch-savings-not-claimed). Wave 6 batch: gpu_culling rd_native = **0046** (FIRST rd_native pass that itself issues `RD::buffer_clear`; b0 144B→48B with the frustum planes moved to a t1 `StructuredBuffer<float4>`; container verify 59/59; device-removal-free clear→dispatch→indirect-draw is the headline evidence), particles view-axis `RENDER_TIMESTAMP` = **0047** (telemetry micro-patch, not a Route B rd_native slice), Luminance read-only getters = **0048** (not a Route B slice; unlocks the 0045 Design 1). All landed, §1. **0049 free** for a later rd_native slice. Claimed atomically with the consuming patch per §4 rule 3. |
| 0050+ | bindless | GRX-022 | reserve pool start (**BUMPED from 0040+** so Route B rd_native can own 0040-0049; bindless is not started). Allocate concrete numbers only AFTER the bindless RFC is adjudicated. |

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

> **Route B rd_native consumes no cap bit.** The rd_native series (0040+) drives
> the main `RenderingDevice` directly (`shader_create_from_bytecode` →
> `compute_pipeline_create` → `compute_list_*`) and does NOT go through the rxgd
> bridge (`rxgd_create_d3d12_session` / `rxgd_record_pass`). It therefore neither
> reads nor sets any `RxGdCaps.flags` bit and allocates nothing from §3. The
> existing `RXGD_CAP_TONEMAP_REAL_PASS` (bit 4) remains the SHIM-path tonemap
> arm; rd_native (backend == 2) is a parallel, bridge-independent path selected
> by the `passes/tonemap/backend` project setting. `RXGD_ABI_VERSION` stays `1`.

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
