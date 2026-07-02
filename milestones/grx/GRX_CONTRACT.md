---
contract: GRX
title: Godot 4.7-dev D3D12 Forward+ Rurix opt-in 加速
status: active
version: v1.10
date: 2026-07-02
timebox: "integration spike -> measured acceleration close-out; task grain = 1-2 days / PR"
rfc_required: none
upstream_docs:
  - "spike/godot-rurix/README.md (tracked patch/bench assets for ignored Godot snapshot)"
  - "src/rurix-godot/README.md (Rurix Godot C ABI bridge scaffold)"
  - "spike/godot-rurix/bench/bench_manifest.json (Godot 4.7-dev Windows D3D12 Forward+ benchmark target)"
  - "14_ENGINEERING_DISCIPLINE.md (evidence discipline, guardrails, measured performance claims)"
in_scope:
  - godot_ignored_snapshot
  - godot_patch_queue
  - rurix_godot_c_abi
  - d3d12_forward_plus_opt_in_module
  - benchmark_visual_perf_evidence
  - tier0_baseline
  - tier1_low_risk_passes
  - tier2_forward_plus_high_gain_passes
  - tier3_structural_optimizations
out_of_scope:
  - full_godot_renderer_replacement
  - compatibility_opengl_mobile_vulkan_metal
  - vendoring_godot_source_into_rurix_git
  - unmeasured_performance_claims
  - disabling_godot_fallback_path
deferred_refs: []
deliverables:
  - id: D-GRX-1
    name: Godot ignored snapshot + patch queue + C ABI bridge kept reproducible and smoke-tested
  - id: D-GRX-2
    name: Godot D3D12 Forward+ opt-in module builds and loads rurix_godot.dll without breaking fallback
  - id: D-GRX-3
    name: Tier 0 benchmark, visual capture, and telemetry evidence chain
  - id: D-GRX-4
    name: Tier 1 low-risk pass acceleration with fallback and visual diff gates
  - id: D-GRX-5
    name: Tier 2 Forward+ high-gain pass acceleration with scene-level performance evidence
  - id: D-GRX-6
    name: Tier 3 structural optimizations and close-out strict performance gate
acceptance_gates:
  - id: G-GRX-1
    check: "external/godot-master remains ignored and untracked; Godot modifications are represented only by tracked patch files under spike/godot-rurix/patches; ci/godot_rurix_bridge_smoke.py passes."
  - id: G-GRX-2
    check: "rurix_godot.dll is produced from src/rurix-godot and Godot module_rurix_accel_enabled=yes builds locally with D3D12 enabled when SCons/toolchain are present; missing DLL or unsupported device must keep Godot fallback active."
  - id: G-GRX-3
    check: "Tier 0 benchmark emits measured_local baseline evidence for all seven scenes: clustered_lights, many_mesh_instances, material_variants, post_fx_chain, volumetric_fog, particles, mixed_forward_plus."
  - id: G-GRX-4
    check: "Every accelerated pass has per-pass enable/disable state, fallback telemetry, visual diff evidence, and at least one real red/green validation path."
  - id: G-GRX-5
    check: "Close-out strict gate passes: geomean FPS ratio >= 1.5, mean p95 frame time reduction >= 0.30, and no scene FPS ratio < 0.95, using py -3 spike/godot-rurix/bench/perf_gate.py <results.json>."
guardrails:
  - "The external/ directory is permanently ignored; no Godot source file from external/godot-master may be tracked."
  - "Godot-side edits must be managed as patch files under spike/godot-rurix/patches; the ignored external tree may be patched locally but is not the source of record."
  - "Performance claims must cite measured_local evidence JSON; estimated numbers may appear only as planning placeholders and cannot satisfy close-out."
  - "Any C ABI shape change must bump RXGD_ABI_VERSION and update src/rurix-godot/include/rurix_godot.h, src/rurix-godot/src/lib.rs, and ci/godot_rurix_bridge_smoke.py in the same PR."
  - "Any Rurix pass compile failure, validation failure, unsupported device, or visual diff breach must preserve Godot fallback and record telemetry."
---

# GRX 契约 - Godot 4.7-dev D3D12 Forward+ Rurix opt-in 加速

> 所属:Godot/Rurix 工程集成里程碑。本文是执行契约,不修改既有 closed milestone,不改 00-14 规划文档。
> **当前状态(2026-07-02):GRX-009 第一段 gated scaffold 已交付,第二段 core call-site fallback wiring 已通过 patch 0003 接线;`segment 3a` 离线 compile evidence 已尝试但处于 blocked,不得进入 resource mapping。pass 本体仍未实现,默认仍保持 disabled/fallback。** `ci/godot_rurix_toolchain_probe.py` 现保留 `grx009_prep_ready` 作为准备产物/manifest path 校验,并新增分段 gate:`grx009_segment1_ready` 用于确认第一段历史交付态,`grx009_segment2_ready` 用于确认第二段接线态(segment=2、`real_gpu_pass=false`、`godot_core_call_site_wired=true`、0002/0003 patch 与 wired-disabled 样例齐备,且 wired sample 必须通过 `fallback_telemetry.py --validate-only`,patch stack 必须真实处于 `0001+0002+0003`)。`grx009_segment3a_compile_ready=false`:latest evidence 为 `compile_failed/body_lowering_missing`;current artifacts 只描述 latest compile attempt 产物,任何 `artifact_kind=dxil_ir_text`、`semantic_status=entry_shell_only` 的 LLVM IR 只能作为 debug/non-ready evidence,不是真实 DXIL container;非平凡 luminance compute body 仍无真实 lowering。manifest 必须保持 segment 2,`next_action` 指向修复真实 DXIL container/body lowering blocker,而不是 resource mapping。`external/godot-master` 仍作为 ignored Godot 4.7-dev 快照存在,Godot 侧修改继续只以 `spike/godot-rurix/patches/0001/0002/0003` 栈式 patch 管理。第二段 patch 0003 仅把 Auto Exposure `luminance_reduction` 调用点接到 `D3D12Hooks::try_record_luminance_reduction()` 的 opt-in gate;当前 bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 恒返回 `RXGD_STATUS_FALLBACK`,所以实测语义仍是 Godot 原生 luminance 接管。GRX-009 目前仍无真实 DXIL container、无真实 GPU pass、无真实 visual diff evidence、无 measured fallback telemetry、无 full baseline / Rurix 对比数据,因此不能宣称任何性能提升。

---

## 1. 目标

GRX 结束时,Godot 4.7-dev Windows D3D12 Forward+ 可通过 opt-in `modules/rurix_accel` 加载 `rurix_godot.dll`,在不删除 Godot 原始渲染路径的前提下,用 Rurix 替换经过验证的高收益 GPU pass。最终性能声明必须来自同画质、同分辨率、同场景、同 D3D12 后端的 measured_local evidence。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| godot_ignored_snapshot | `external/godot-master` 作为本地源码快照,永远 ignored | D-GRX-1 |
| godot_patch_queue | Godot module 和后续 Godot 侧变更只以 patch 文件入库 | D-GRX-1, D-GRX-2 |
| rurix_godot_c_abi | `rurix_godot.dll` C ABI: session/resource/pass/stats/fallback | D-GRX-1, D-GRX-2 |
| d3d12_forward_plus_opt_in_module | 仅 `--rendering-driver d3d12 --rendering-method forward_plus` + opt-in setting 启用 | D-GRX-2 |
| benchmark_visual_perf_evidence | 7 场景 benchmark、GPU timestamp/frame stats、视觉 diff、telemetry | D-GRX-3 |
| tier1_low_risk_passes | luminance reduction、tonemap、SSAO/SSIL blur、TAA resolve、particles copy | D-GRX-4 |
| tier2_forward_plus_high_gain_passes | clustered light binning、GPU culling、visible instance compaction、material sorting、indirect draw args | D-GRX-5 |
| tier3_structural_optimizations | post FX fusion、descriptor/root signature cache、PSO prewarm、bindless/resource-array 扩展 | D-GRX-6 |

### 2.2 out-of-scope

- 不整体重写或替换 Godot renderer。
- 不覆盖 Compatibility/OpenGL、Mobile、Vulkan、Metal。
- 不把 `external/godot-master` 或其它 Godot 源文件纳入 Rurix Git。
- 不在没有 evidence JSON 的情况下宣称 1.5x 或任何性能提升。
- 不移除 Godot 原始 fallback path。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-GRX-1 | scaffold 可复现 | ignored tree + patch queue + smoke | G-GRX-1 |
| D-GRX-2 | Godot module build/load | SCons build log + DLL load/fallback smoke | G-GRX-2 |
| D-GRX-3 | Tier 0 evidence chain | benchmark generator/runner + raw frame samples + baseline JSON + visual capture | G-GRX-3 |
| D-GRX-4 | Tier 1 passes | 每 pass 一个 PR + visual/perf/fallback evidence | G-GRX-4 |
| D-GRX-5 | Tier 2 passes | 每 pass 一个 PR + scene-level perf evidence | G-GRX-4, G-GRX-5 |
| D-GRX-6 | Tier 3 + close-out | structural optimization evidence + strict perf gate | G-GRX-5 |

## 4. 验收门

1. **G-GRX-1(scaffold/ignore)**:`external/godot-master` 必须由 `.gitignore` 命中且 `git status --porcelain -- external/godot-master` 为空;`py -3 ci/godot_rurix_bridge_smoke.py` 通过。
2. **G-GRX-2(Godot build/load)**:存在可归档 Godot build log;命令为 `scons platform=windows target=template_debug d3d12=yes module_rurix_accel_enabled=yes disable_path_overrides=no`;无 `rurix_godot.dll`、ABI mismatch、非 D3D12/Forward+ 时不崩溃并记录 fallback。
3. **G-GRX-3(Tier 0 baseline)**:`GRX-005` 建设期先交付 7 场景 measured_local raw frame sample JSON,采样参数来自 `bench_manifest.json`:warmup 300、sample 2000、vsync false、1920x1080,且 `gpu_timestamps_available=false` 必须显式记录;baseline evidence JSON、strict perf claim 与任何性能提升声明仍属于后续 `GRX-006+`。
4. **G-GRX-4(pass safety)**:每个新增 pass 必须有 enable/disable、fallback telemetry、视觉 diff、pass 输出错误或视觉 diff 超阈值的真实红绿验证。
5. **G-GRX-5(strict perf close-out)**:`py -3 spike/godot-rurix/bench/perf_gate.py <results.json>` 通过;`geomean_fps_ratio_min=1.5`,`p95_frame_time_reduction_min=0.3`,`single_scene_fps_ratio_min=0.95`;即 geomean FPS ratio >= 1.5、p95 frame time reduction >= 30%、no single scene FPS ratio < 0.95;所有数据为 measured_local。

## 5. Guardrails

见 YAML 头 `guardrails` 字段。补充说明:

- `RXGD_ABI_VERSION` 是 C ABI 兼容边界。任何 `RxGdCaps`、`RxGdResource`、`RxGdFrameStats`、status/pass id、导出函数签名变化都必须同 PR 更新 header、Rust 实现、smoke。
- fallback 是稳定性硬边界。失败 pass 默认禁用;不得因追求性能破坏 Godot 原路径。
- `measured_local` 是 close-out 唯一性能证据等级;建设期 SKIP 必须写明缺工具链、缺 benchmark、缺 scene 或缺 visual capture 的具体原因。

## 6. Deferred 引用

本契约不预注册 RD 条目。执行期若发现必须跨里程碑延期的能力,按项目规则追加 `registry/deferred.json` 并在相应代码/文档中双侧标注。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-01 | 初版契约固化。建立 GRX 作为 Godot/Rurix 工程集成里程碑;记录当前 scaffold 状态;锁定 ignored Godot tree、patch queue、C ABI、fallback、measured_local 性能证据和 1.5x / p95 -30% strict close-out 门。 |
| v1.1 | 2026-07-01 | 收紧 GRX.0 文档基线口径。明确当前仅完成文档与状态基线,不得把性能结果写成已达成;把 strict gate 的 p95 阈值表述统一为 `p95_frame_time_reduction_min=0.3` 对应 `p95 frame time reduction >= 30%`。 |
| v1.2 | 2026-07-01 | 统一阶段边界口径:明确 `GRX.0 = 文档与状态基线`,`GRX.1 = detector/build/load`;当前仍未完成 Godot SCons 构建与 DLL load/fallback smoke。 |
| v1.3 | 2026-07-01 | 根据现有 evidence 修正当前状态:明确 `GRX-001/002/003` 已本地通过并产出 detector/build/load/fallback summary;本次只做 `GRX.1` 收口硬化与接续修正,`GRX.2 benchmark`、visual diff、实际加速 pass 仍未完成,不得宣称任何性能提升已达成。 |
| v1.4 | 2026-07-01 | 以 fresh path-overrides rebuild + fresh load smoke 重新落地 `GRX.1` close-out:确认 build summary 已记录 `disable_path_overrides=no`、artifact evidence 与 `path_overrides_ready`,fresh present/missing DLL smoke 均通过且 `external/godot-master/bin` 不再残留 smoke 项目文件;probe 现已推进到 `GRX-004`,但 benchmark、visual diff、实际加速 pass 仍未开始。 |
| v1.5 | 2026-07-01 | 收口 `GRX-004` / 接续 `GRX-005`:明确 7 场景 benchmark project 已以 fresh per-scene smoke 通过,下一步只实现固定 warmup/sample/vsync/resolution 的 tracked runner 与 raw frame sample JSON;baseline schema/perf gate、visual diff、实际加速 pass 与性能提升声明仍未完成。 |
| v1.6 | 2026-07-01 | 收口 `GRX-005` 硬化 / 交付 `GRX-006` schema 与 perf gate 输入格式:`run_benchmark_scenes.py` 增加 Godot 日志 failure marker 扫描(对齐 `bench_project_smoke.py`,allowlist global script cache warning),`per_scene_results` 记录 `failure_markers` / `warnings`,summary 增加 `warning_count`;新增 `spike/godot-rurix/bench/schemas/` 两套 draft-07 schema、扩展 `perf_gate.py`(`--kind`/`--strict`/`--validate-only`,strict 拒绝 SKIP/estimated/quick_smoke/缺 scene/缺 raw path)、新增 `spike/godot-rurix/bench/samples/` 两个样例;`godot_rurix_toolchain_probe.py` 在 build/load/scenes/runner evidence 齐备时把 `next_action` 指向 `start_grx006_baseline_schema_perf_gate`。GRX-006 仅交付格式/门禁基础设施,full baseline 实测、加速 pass、visual diff 与性能提升声明仍未完成。 |
| v1.7 | 2026-07-01 | 收口 `GRX-006` hardening / 交付 `GRX-007` scaffold:`perf_gate.py` 修复三处漏判——strict forbidden marker 由前缀匹配改为词边界正则(命中 `SKIP: missing`/`skip-reason`/`status=SKIP`/`estimated:true`/`estimated local`,不误伤 `spike`/普通路径)、baseline reader 补齐 `sample_count` 正整数且 `== sample_frames` 校验、strict `thresholds` 三项固定值(1.5/0.3/0.95)防篡改校验;新增两个红测样例(`perf_gate_forbidden_skip_example.json`、`baseline_missing_sample_count_example.json`);`godot_rurix_toolchain_probe.py` 新增 `grx006_schema_ready` 检测并把 `next_action` 推进到 `start_grx007_visual_diff_scaffold`。GRX-007 仅交付 scaffold(`capture_reference_frames.py`、`visual_diff.py`、`visual_diff_evidence.schema.json`、`visual_diff_placeholder.json`),7 场景全部 SKIP;full baseline 实测、Rurix 加速 pass、真实 visual diff pass 与性能提升声明仍未完成。 |
| v1.8 | 2026-07-01 | 收口 `GRX-007` hardening / 交付 `GRX-008` scaffold hardening / 接续 `GRX-009` 准备:`visual_diff.py` 与 schema 现禁止 `status=skip` 携带伪造 diff/帧路径(`reference/candidate path`、`ldr/hdr/temporal diff` 必须 null 或缺省),新增红测 `visual_diff_skip_with_fake_ldr_example.json`(skip 带 ldr_diff 必 FORMAT FAIL),保持既有 pass 缺 ldr_diff FORMAT FAIL / diff 不一致 DIFF FAIL / diff 一致 PASS / `--write-output` 生成 evidence;`fallback_telemetry.py` 与 schema 区分 scaffold 与 full(scaffold 允许 timestamp/frame=null 但必须 disabled + `godot_fallback_active=true`;full/measured_local 要求 timestamp 非空、frame 非负整数;measured_local 禁止 `placeholder_` pass_id),新增两个红测样例并保持 placeholder FORMAT PASS;`godot_rurix_toolchain_probe.py` 新增 `grx007_visual_ready`/`grx008_telemetry_ready`(跑红绿样例),GRX-008 ready 后 `next_action=start_grx009_luminance_reduction_pass_prep`。本轮不实现任何实际 Rurix 加速 pass,不宣称视觉验证、fallback 真接入或性能提升已完成。 |
| v1.9 | 2026-07-02 | 收口 GRX-009 准备 / 交付 gated implementation 第一段:修正当前状态口径(准备已完成,下一步为 gated implementation 而非准备);`grx009_prep_ready` 加强为校验 manifest 记录的 Godot source/header/shader/call-site 文件存在于 `external/godot-master`(只读、不改快照);`src/rurix-godot` 新增 `LuminanceReductionGate`(ABI v1 不变,默认 disabled,`request_enable` 恒 `compile_failed`,`rxgd_record_pass` 对 luminance 恒 `RXGD_STATUS_FALLBACK` 并移除其占位 estimated GPU time,新增两条单测);新增栈式 0002 module patch(per-pass 设置默认 false,`try_record_luminance_reduction()` 非 OK 即走 Godot 原生 luminance 路径,仅 `modules/rurix_accel/*`)、disabled telemetry 样例与 bridge smoke patch 栈三态检查。GRX-009 pass 本体仍未实现:core call-site 未接线、无真实 GPU pass、无真实 visual diff、无 measured telemetry,不宣称任何性能提升。 |
| v1.10 | 2026-07-02 | 收口 GRX-009 第二段 core call-site fallback wiring:当前状态改为“第一段 gated scaffold 已交付、第二段通过 patch `0003-rurix-accel-luminance-core-callsite-wiring.patch` 完成 Godot core Auto Exposure call-site 接线”;`ci/godot_rurix_toolchain_probe.py` 新增 `grx009_segment1_ready` / `grx009_segment2_ready` 分段 gate,并把 segment 2 完成态的 `next_action` 推进到 `start_grx009_luminance_reduction_real_gpu_pass`;Godot 侧 patch 栈扩为 `0001/0002/0003`,bridge smoke 改为四态检查。默认语义仍保持 disabled/fallback:当前 bridge 对 `RXGD_PASS_LUMINANCE_REDUCTION` 恒返回 `RXGD_STATUS_FALLBACK`,所以虽已接线,仍由 Godot 原生 luminance 路径接管;真实 GPU pass、真实 visual diff、measured fallback telemetry 与任何性能提升声明仍未完成。 |
| v1.11 | 2026-07-02 | 收口 GRX-009 review findings 并接续 `segment 3a` 离线 compile evidence:把当前状态明确为 segment 2 已完成、下一步是离线 kernel/package 编译取证;说明 `grx009_segment1_ready` 只表示历史交付点,`grx009_segment2_ready` 需要 callsite-wired sample 的 `fallback_telemetry.py --validate-only` 与真实 patch 栈 `0001+0002+0003`;同时规定只有真实 `DXIL + root signature + descriptor layout` artifact 三者齐备时 manifest 才能进入 segment 3,compile_failed 仅是 blocker evidence complete,不改变 runtime fallback 默认语义。 |
| v1.12 | 2026-07-02 | 修复 GRX-009 segment 3a artifact gate 口径:LLVM IR 文本与 `ret void` entry shell 不再视为真实 DXIL container 或 real pass compile success;`offline_compile_evidence.json` 记录 `compile_failed/body_lowering_missing`,current artifacts 只描述 latest compile attempt,manifest 保持 segment 2,`grx009_segment3a_compile_ready=false`,next_action 指向修复真实 DXIL container/body lowering blocker,不进入 resource mapping。 |

---

## 8. Close-out(只追加区 - 开工时为空)

<!-- 追加 Godot build log、benchmark results JSON、visual diff evidence、pass enable matrix、fallback telemetry、perf_gate 输出、strict close-out 判定。上方条款 0-byte 修改。 -->
