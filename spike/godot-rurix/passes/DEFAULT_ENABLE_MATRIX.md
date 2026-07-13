# GRX-025 default-enable matrix (2026-07-13, measured_local, no performance claim)

> 所属:[milestones/grx/GRX_CONTRACT.md](../../../milestones/grx/GRX_CONTRACT.md) · GRX-025
> 输入证据:GRX-025 per-pass 二分 `spike/godot-rurix/bench/grx025_default_enable_20260713/`(≥0.95 门)、GRX-024 视觉 parity `spike/godot-rurix/bench/grx024_visual_20260713/`、rd_native 终局天花板 `spike/godot-rurix/bench/rd_native_final_20260713/`。
> **零性能宣称**:本矩阵是**默认启用/禁用决策**的汇总,不含 FPS、p95、GPU timestamp 或任何加速宣称。GRX-025 的 ≥0.95 门量的是「开启的**成本**」(cost),不是收益(benefit)。

## 0. 决策口径与图例

GRX-025 只问一个 gate-shaped 问题:在每个 rd_native pass **engage** 的场景上,它的单-pass avg_fps ratio(对 v2.3 baseline median)是否 **≥ 0.95x**(即开启该 pass 的成本 < 5%)。这是**默认启用的输入**,不是提速。

本矩阵对**所有涉足过的 pass**逐行给出:决策 token / 门槛证据链接(≥0.95 数字 + parity 证据 + engagement 子集)/ 子集边界 / decision 文档链接。

| 决策 token | 含义 |
|---|---|
| `eligible_for_default_enable_gate_met` | GRX-025 ≥0.95 门 + GRX-024 视觉 parity 两者实测达成;但当前 default **仍 disabled**——见下方「为何门达成仍 disabled」。 |
| `keep_disabled` | 门未达成或结构性原因(净零 / 参数缺口 / 场景不可达),默认禁用并留证。 |
| `mechanism_blocked` | 机制层 blocked(引擎/驱动缺口),default disabled 并留 upstream 诊断。 |
| `not_a_render_pass` | 非 Godot render pass(shim/bridge 内部件),无「默认启用」可言。 |

### 为何门达成仍 disabled(rd_native 启用 = opt-in by design)

rd_native backend(`passes/<pass>/backend`)默认值为 `0`(native)。启用一个 rd_native pass 需要集成方**逐项目**:①把 Rurix 容器 staged 到 `target/grx/rd_containers/<pass>.rd_container.bin`;②在项目设置 `rendering/rurix_accel/passes/<pass>/rd_container_path` 指向它;③把 `backend` 置 `2`。**没有全局默认可言**——无容器则 fail-closed 回落 native。因此五个过门 pass 的诚实决策是:**门槛条件已全部实测达成,启用为集成方按需 opt-in**,而非仓库层翻一个全局开关。这与既有 bridge-era `real_pass_default_enable_decision`(`keep_default_disabled`)口径一致但成因不同:bridge-era 是「缺 per-pass FPS 证据 + writeback scaffold」,rd_native 是「门达成 + 启用天然是 per-project staging opt-in」。

## 1. 主矩阵(全部涉足 pass)

| pass | slice | 决策 token | 当前 default | 门槛/状态证据 | decision 文档 |
|---|---|---|---|---|---|
| tonemap | GRX-010 | `eligible_for_default_enable_gate_met` | disabled | GRX-025 engaged-geomean **0.9959** ≥0.95;GRX-024 byte-exact(clustered/material) | [tonemap/real_pass_default_enable_decision.md](tonemap/real_pass_default_enable_decision.md) §rd_native |
| ssao_blur | GRX-011 | `eligible_for_default_enable_gate_met` | disabled | GRX-025 **0.9950** ≥0.95;GRX-024 byte-exact(post_fx) | [ssao_blur/real_pass_default_enable_decision.md](ssao_blur/real_pass_default_enable_decision.md) §rd_native |
| taa_resolve | GRX-012 | `eligible_for_default_enable_gate_met` | disabled | GRX-025 **0.9906** ≥0.95;GRX-024 ±1 LSB(many_mesh,deterministic floor 0) | [taa_resolve/real_pass_default_enable_decision.md](taa_resolve/real_pass_default_enable_decision.md) §rd_native |
| particles_copy | GRX-013 | `eligible_for_default_enable_gate_met` | disabled | GRX-025 **0.9945** ≥0.95;GRX-024 floor-limited + container ~1-ULP | [particles_copy/real_pass_default_enable_decision.md](particles_copy/real_pass_default_enable_decision.md) §rd_native |
| cluster_store | GRX-014 | `eligible_for_default_enable_gate_met` | disabled | GRX-025 **0.9993** ≥0.95;GRX-024 byte-exact(clustered/post_fx/volumetric) | [cluster_store/real_pass_default_enable_decision.md](cluster_store/real_pass_default_enable_decision.md) §rd_native |
| fused_post_chain | GRX-019 | `keep_disabled` | disabled | rd_native engage 但 AE parity out-of-tolerance(max_abs=85/mean=66)+ 结构净零(shadow-recompute);bench 7 场景 LINEAR∩AE=∅ 从不 engage | [fused_post_chain/real_pass_default_enable_decision.md](fused_post_chain/real_pass_default_enable_decision.md) §rd_native |
| gpu_culling | GRX-015 | `mechanism_blocked` | disabled | rd_native R1 终裁 device-removal(`0x887A0005`),RDG compute→INDIRECT 同帧 sync 缺口 | [gpu_culling/rd_native_default_enable_decision.md](gpu_culling/rd_native_default_enable_decision.md) |
| instance_compaction | GRX-016 | `mechanism_blocked` | disabled | offline kernel + math parity 落地;运行时依赖 gpu_culling indirect 机制,同 RDG 缺口 blocked | (pass_manifest `grx016_offline_kernel_and_math_parity_default_disabled`) |
| indirect_args | GRX-018 | `mechanism_blocked` | disabled | offline kernel + math parity 落地;compute-written draw-indirect 同 RDG 缺口 blocked | (pass_manifest `grx018_offline_kernel_and_math_parity_default_disabled`) |
| luminance_reduction | GRX-009 | `keep_default_disabled`(bridge-era) | disabled | bridge real-pass strict success 但 writeback scaffold(native continuation 重渲染)、无净收益、math parity level-0-only | [luminance_reduction/real_pass_default_enable_decision.md](luminance_reduction/real_pass_default_enable_decision.md) |
| material_sorting_telemetry | GRX-017 | `keep_disabled`(telemetry-only) | disabled | 仅遥测切片,无 kernel/hook/bridge;是否再做排序切片待 full baseline 判断 | (passes/material_sorting_telemetry/README.md) |
| descriptor_cache | GRX-020 | `not_a_render_pass` | n/a | shim descriptor-heap ring-fence 硬化,feature 内 always-on,非 render pass | [descriptor_cache/descriptor_cache_decision.json](descriptor_cache/descriptor_cache_decision.json) |
| pso_prewarm | GRX-021 | `not_a_render_pass` | n/a | bridge 内 session 启动预热,`rxgd_create_d3d12_session` 自动触发,patch 0039 not_needed | [pso_prewarm/pso_prewarm_decision.json](pso_prewarm/pso_prewarm_decision.json) |

(luminance_reduction 的 bridge-era decision 早于 rd_native 复制阶段;luminance 无独立 rd_native pass,其复制被 fused_post_chain 的 luminance-final leg 吸收,见 fused honest boundary。GRX-022 bindless 从未开工,收官下归档 frozen,见 GRX_PLAN。)

## 2. 五个过门 pass 的门槛明细(GRX-025 §4 + GRX-024 + rd_native_final §4)

| pass | GRX-025 legs | engaged# | **engaged geomean** | worst engaged scene / ratio / noise | engagement 子集(engage 的场景) | rd_native_final per-pass µs |
|---|---|---|---|---|---|---|
| tonemap | 2(median-of-2) | 4 | **0.9959** | many_mesh_instances / 0.9888 / 2.17% | 4/7 **LINEAR-only**(clustered, many_mesh, material, particles;FILMIC 三场景 0040 mode-guard fail-closed) | +13~31 µs(~1.8-1.9x 其 native bucket) |
| ssao_blur | 1 | 2 | **0.9950** | mixed_forward_plus / 0.9904 / 0.69% | 2/7 SSAO on + **SMART** blur 单-slice 子集(post_fx, mixed) | ≈0 delta(gather-dominated bucket 内小片) |
| taa_resolve | 2(median-of-2) | 2 | **0.9906** | mixed_forward_plus / 0.9890 / 0.69% | 2/7 `use_taa`(many_mesh, mixed);单 resolve dispatch 子集 | +100~114 µs(~1.6-1.8x native resolve;门内 <1.1% avg_fps) |
| particles_copy | 1 | 1 | **0.9945** | particles / 0.9945 / 1.03% | 1/7 **no-userdata** emitter 子集(`userdata_count==0`/stride 112);标准 ParticleProcessMaterial 出子集 | ≈0 delta(bandwidth-bound copy,3/15 emitter 替换) |
| cluster_store | 1 | 4 | **0.9993** | mixed_forward_plus / 0.9887 / 0.69% | 4/7 clustered omni/spot 灯的场景;仅 bake_cluster 的 compute merge(store)段 | +4~10 µs(~6-11%);光栅段/clears/count==0 early-out 留 native |

判据(`analyze_grx025.py:verdict_for`):**pass** = engaged geomean ≥ 0.95 且 worst engaged scene ≥ 0.95 或其 shortfall 在该场景 noise 带内。五个 pass 的 engaged geomean 全在 **[0.9906, 0.9993]**,最低单元格 mixed taa 0.9890,减去场景 noise 后仍 comfortably 过 0.95。**没有 pass 在门边缘。**

### 视觉 parity(GRX-024,default-enable-safe 的 pixel 证据)

| pass | 确定性场景 parity | verdict |
|---|---|---|
| tonemap | byte-exact(clustered, material) | pixel-identical |
| ssao_blur | byte-exact(post_fx) | pixel-identical |
| cluster_store | byte-exact(clustered, post_fx, volumetric) | pixel-identical |
| taa_resolve | ±1 LSB,floor 0(many_mesh) | near-exact(≤1 LSB) |
| particles_copy | 无确定性场景(仅 particles 非确定);floor-limited + container ~1-ULP + GRX-025 leg 交叉印证 | floor-limited,cross-referenced |

## 3. fused_post_chain(GRX-019)诚实边界 — 双重 keep_disabled

fused 的诚实决策有两条**独立**成因,任一足以 keep_disabled:

1. **bench 场景从不 engage**:fused gate 需 LINEAR-tonemap 子集 **AND** auto-exposure 产出的独立 current/previous luminance buffer;7 个 bench 场景里两者互斥(4 个 LINEAR 场景无 AE,2 个 AE 场景是 FILMIC),交集 ∅。rd_native_final §2 记 fused 0/7 non-engagement,all5_fused leg 作为 A/A 控制,fused kernel 在 bench 上 perf-unmeasured。
2. **专属 AE 场景上 engage 但两条 honest boundary**:在 CameraAttributesPractical 驱动 AE 的专属 enablement 场景(`ci/grx_rb_fused_post_chain_rd_native_enablement_smoke.py`)上,fused rd_native **genuinely engage**(`RXGD_RD_NATIVE_FUSED_POST_CHAIN active` 仅现于 candidate 腿),但:
   - **parity out-of-tolerance**:fused AE/EMA + tonemap 数学对 native auto-exposure 的 LDR 输出发散 **max_abs=85 / mean_abs=66**(阈值 max≤4/mean≤1.0),因 fused kernel 的 b0 auto-exposure 标量仍是占位(max_luminance=1/min=0/exposure_adjust=1/…),待 Luminance-API 扩展供真参数;`pass_manifest.rd_native.enablement.status=measured_engaged_parity_out_of_tolerance`、`real_gpu_pass=false`、`first_missing_prerequisite=fused_tonemap_parity_out_of_tolerance`,**不写 success evidence、不推进 gate**。
   - **结构净零**:即便 parity 修好,fused 的 luminance-final 写是 shadow-recompute(写自有 scratch UAV,永不读回),native luminance_reduction pyramid 仍每帧全跑,`net_dispatch_saving=0`;`honest_boundary.status_tokens=[engaged, shadow-luminance-write, dispatch-savings-not-claimed]`,`structural_fusion_claimed=false`。真结构收益要 Design 2(glow-off gate + 跳过 native final reduce + luminance 双缓冲外部 SWAP),属未来批次。
   - cascade 实测:AE-off + tonemap backend=2 腿上 fused gate 在 invalid-lum-RID 关口 fail-closed → 级联到 `RXGD_RD_NATIVE_TONEMAP active`,帧与非 AE reference 逐字节一致(**fused→tonemap→native 两级回退真机打通**)。

历史勘误(manifest `historical_misattribution_note`):早前「fused 在 aliasing guard blocked、candidate byte-identical」的读数是误判(专属 smoke 场景曾对 Environment 设不受支持的 `auto_exposure_enabled` → 每次 GDScript SCRIPT ERROR、AE 从未 engage、luminance buffer 从未分配、module 在 invalid-lum-RID 关口 fail-closed);runtime 审计只扫 `ERROR:` 前缀漏了 `SCRIPT ERROR`。现 smoke 经 CameraAttributesPractical 驱动 AE 且对 SCRIPT ERROR 失败,故上述 parity 读数是**首次诚实的 engage+parity 测量**。

## 4. mechanism-blocked 三 pass(GRX-015/016/018)

gpu_culling(GRX-015)rd_native R1 已实现到位(patch 0046 修订:Rurix-owned scratch indirect buffer、`buffer_copy` live→scratch、clear/dispatch 限于 scratch、`draw_list_draw_indirect` retarget scratch),交付其预注册终裁 `ci/grx_rb_gpu_culling_rd_native_enablement_smoke.py`(rb4 exe)。**终裁:candidate 腿仍 device-remove(`0x887A0005`)**,定罪 GENERAL 假设:此 Godot D3D12 后端上「compute 写 `DISPATCH_INDIRECT` buffer 被同帧 indirect draw 消费」的模式,无论涉及哪个 buffer、kernel 写什么(空 kernel 复现)、instance 数(1 复现)都移除设备;debug 层与 GPU-Based Validation 全静默(CPU 侧 barrier/state 链验证干净,故障在 GPU 时间线)。R2(用上帧 cull 结果 + 帧边界 copy)被拒:1 帧陈旧 visibility 在相机运动下违背 pass 的保守画面保持不变式。**R3 终态**:`pass_manifest.json` 翻转到 `grx015_rd_native_r1_final_verdict_mechanism_blocked_rdg_gap`;GRX-015/016/018 在此 engine/driver 组合上均 blocked;default 保持 disabled;无性能宣称。这是 upstream Godot bug-report 候选(RDG / D3D12 driver:一帧 frame-graph submission 内缺 UAV→INDIRECT_ARGUMENT 的 GPU-timeline sync)。诊断全文:[gpu_culling/rd_native_device_removal_diagnosis.md](gpu_culling/rd_native_device_removal_diagnosis.md) §6。

GRX-016 instance_compaction / GRX-018 indirect_args:offline kernel + math parity 已落地(default disabled),运行时都依赖同一 compute→indirect 机制,故同 RDG 缺口下 mechanism-blocked,收官归档为 blocked 而非 keep_disabled-by-choice。

## 5. Fallback 政策(disabled pass 有原因和证据)

- **默认全禁**:所有 pass 的 `backend`/opt-in 默认 `0`/`false`;缺容器、preflight 失败、dispatch-eligibility 失败、visual diff 超阈值均 fail-closed 回落 native Godot 路径。每个禁用 pass 在上表都有原因 + evidence 链接(合规 GRX_PLAN GRX-025「disabled pass 有原因和证据」)。
- **五个过门 pass**:门达成 ≠ 自动全局开;启用是 per-project 容器 staging + `rd_container_path` opt-in(§0)。
- **fused**:双重 keep_disabled(§3)。
- **gpu_culling/016/018**:mechanism-blocked,留 upstream 诊断(§4)。
- **luminance / material_sorting**:bridge-era / telemetry-only keep_disabled,各自 decision 文档。
- **descriptor_cache / pso_prewarm**:非 render pass,无默认启用面。

## 6. 证据索引

- ≥0.95 门:`spike/godot-rurix/bench/grx025_default_enable_20260713/grx025_default_enable_report.md`(+ `matrices/`、`analyze_grx025.py`)
- 视觉 parity:`spike/godot-rurix/bench/grx024_visual_20260713/grx024_visual_report.md`(+ `captures/`、`grx024_visual_summary.json`)
- 天花板 / per-pass µs:`spike/godot-rurix/bench/rd_native_final_20260713/rd_native_final_report.md`(Amdahl 零成本 geomean ceiling **1.0669x**,measured all5 0.9942)
- per-pass rd_native enablement:`spike/godot-rurix/passes/<pass>/rd_native_enablement_success_evidence.json`(tonemap/ssao_blur/taa_resolve/particles_copy/cluster_store strict success;fused=measured_engaged_parity_out_of_tolerance;gpu_culling=mechanism_blocked)
- per-pass decision:各行「decision 文档」列。

## 7. CR self-check

本文件为新建 close-out 文档,写为 LF、`CR=0`;引用的 bench/pass evidence 均为既有归档,本任务不改其结论(append-only)。无 FPS/p95/GPU-timestamp/性能提升宣称。
