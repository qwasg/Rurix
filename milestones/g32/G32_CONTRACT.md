---
contract: G32
title: G32 画面完整期（波 B 批次：HZB 遮挡剔除/ReSTIR/slab 材质侧表/纹理采样/蒙皮骨骼动画生产接线 + GI 默认档决策 + OIT 评估窗 + 组合矩阵 + 游戏画面 demo 定版 + 波 B 验收门）
status: active
implementation_status: unlocked
active_scope: g32_wave_b_visual_completeness
version: v1.0
date: 2026-08-26
timebox: "波 B 批次 = g30-closed 后 G31+ 战役第二波（B1~B7 实现/评估 + B8 验收）；#11 BistroExterior 维持 G10-N6 锚挂起（FBX2glTF 上游修复或替代臂 + 源资产同窗齐备，后续期窗）；GI 默认档重判按 B6 re_trigger 两条件后续期窗"
rfc_required: "波 B 批次零新 RFC——实现全部消费既有语义面与冻结面（RFC-0030 管线结构性优化 / RFC-0035 FG 独立层 / RFC-0036 FG 实现语义 / RFC-0043 FG device kernel + G26 framegen/G27 hzb/G28 restir/G29 slab 四 device kernel 0-byte 冻结消费 + G11.3 DDS manifest + M92 蒙皮对拍门 + M120 OIT 测量面）；零新 RXS 条款、零共享编号段消费（落盘前实测：RFC next_free=48 / RXS next_free=408 / RD next_free=46 / CI_step next_free=525 全维持）"
upstream_docs:
  - "milestones/g30/g30_campaign_handover_registry.json（RFC-0047 §5.5：G31+ 唯一法定输入面）"
  - "G31_PLUS_COMMERCIAL_RENDERER_TODO.md §1.2 #6~#13 + §1.3 #10 + §6 波次线 2（G32 = 画面完整期定义）"
  - "registry/deferred.json（RD-041 蒙皮 WPO MV 触发面 / RD-045 确定性观察面）"
in_scope:
  - g32_wave_b_hzb_wiring
  - g32_wave_b_restir_wiring
  - g32_wave_b_slab_wiring
  - g32_wave_b_texture_wiring
  - g32_wave_b_skinning_wiring
  - g32_wave_b_gi_default_decision
  - g32_wave_b_oit_evaluation_window
  - g32_wave_b_combo_matrix_and_demo
  - g32_wave_b_acceptance_gate
out_of_scope:
  - bistro_exterior_scene_arm（#11 维持 G10-N6 锚挂起：工具链三根检索零命中 + 源资产未齐，双场景闭集维持）
  - hdr_display_chain_implementation（M118-hdr-cal maintain-SDR 字面维持）
  - new_optimization_or_feature_work_beyond_wave_b
  - rewriting_g13_g31_frozen_registries_or_anchors
  - presented_fps_masquerading_as_real_render_fps（双口径分离，生成帧禁入真实渲染帧率口径）
  - cross_harness_combo_masquerading_as_single_process（--skin-demo/--dyn-demo 在 pipeline_perf 车道闭集，--textures/--hzb/--slab-table/--fg 在 window_present 车道闭集；跨 harness 组合以双真跑同窗登记，不冒充单进程组合）
deferred_refs: [RD-041, RD-045]
deliverables:
  - id: D-G32-1
    name: 波 B 实现五硬门 + 两评估窗（B1~B5 harness + smoke + evidence schema 均 gate PASS 在案；B6/B7 决策 JSON 只追加）
  - id: D-G32-2
    name: 波 B 验收门（B8：组合矩阵登记 + 游戏画面 demo 定版 + 零降级三面 + 守卫套件 + RD-045 复核，登记面不设硬门不占号）
  - id: D-G32-3
    name: G32 四件套（PLAN/CONTRACT/CI_GATES/g32_budget.json，零 estimated 全 measured）
acceptance_gates:
  - id: G-G32-1
    check: "B1 HZB 门 g31.waveB.hzb PASS：剔除真实发生（tested=8799/occluded=3549）+ 剔除像素中性（on vs ALL_VISIBLE digest_seq 位级）+ mips 位级 + off 锚零漂移 + on≈3×off 帧耗如实登记"
  - id: G-G32-2
    check: "B2 ReSTIR 门 g31.waveB.restir PASS：y 锚 20000/20000 + p100 1.75e-9 ≪ 冻结容差 5.66e-6 + 方差降 15.8× + off 静态锚 4 跑零漂移"
  - id: G-G32-3
    check: "B3 slab 门 g31.waveB.slab PASS：238927 slab 三角 + device vs host bitexact 跨臂 0/2073600 + MaterialClosure 32B ABI 核验 + Stage A 锚 MATCH"
  - id: G-G32-4
    check: "B4 纹理门 g31.waveB.texture PASS：albedo/normal 70/70 + rough-metal 0/70 如实缺 + sampler max_lsb=1 + 12/12 槽 == G11.3 manifest + on/off +6.29% measured"
  - id: G-G32-5
    check: "B5 蒙皮门 g31.waveB.skinning PASS：蒙皮角色骨骼动画 20/20 位置核验 + MV 通道进 TSR + BLAS refit 桥 + skin≠静态 + Stage A 锚 MATCH"
  - id: G-G32-6
    check: "B8 波 B 验收门：组合矩阵可组合臂 5/5 真跑绿（双跑 digest 位级 + 帧率 measured）+ 互斥 12/12 fail-closed 拒跑 + demo 臂真窗口 ≥200 帧双口径 + Stage A digest 18/18 零漂移 + G16 M-g 18/18 VERDICT=PASS + 守卫套件五条 exit 0 + RD-045 窗 6/6 臂零漂移三件 0/3 维持；G17-MD-F1 焦点格 fresh 诚实红面 = 如实登记（达标/维持/恶化均合法终态，禁冒充）"
guardrails:
  - "诚实登记不冒充：所有数字来自真实命令输出；达标/维持/诚实红均合法终态"
  - "append-only：evidence/ 只增不删不改；deferred history 只追加；既有锚/注册表 0-byte 不回写"
  - "双口径分离：presented 帧率（含 FG 生成帧）与 real_render 帧率独立登记，禁混入"
  - "三态纪律：dev-env 降级 = SKIP 如实登记，禁冒充 PASS；RURIX_REQUIRE_REAL=1 翻硬 FAIL"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G32 契约 — 画面完整期（波 B 批次范围）

> 所属：[../11_ROADMAP.md](../11_ROADMAP.md) §3 / G31+ 期待办总表 [../G31_PLUS_COMMERCIAL_RENDERER_TODO.md](../G31_PLUS_COMMERCIAL_RENDERER_TODO.md) §6 波次线 2；契约机制见 14 §1。front matter 双状态机：`status` 与 `implementation_status` 严格分离。

---

## 1. 目标

波 B 批次结束时，项目获得：**bistro 级场景在真窗口内以"游戏画面"内容面实时渲染并呈现**——HZB 遮挡剔除/ReSTIR 采样/slab 材质侧表/纹理贴图采样/蒙皮骨骼动画五大特性接进生产车道（各自 0-byte 冻结面消费），GI 默认档经 measured 权衡决策（maintain_default_off），OIT/半透明经评估窗决策（not_triggered），特性组合矩阵机器核验（可组合臂真跑绿 + 互斥 fail-closed），"游戏画面"demo 定版真窗口 ≥200 帧双口径登记；且全部既有画质/性能/确定性锚零降级（机器核验在案，诚实红面如实登记不冒充）。

## 2. 范围

### 2.1 in-scope（波 B 批次）

| 项 | 说明 | 对应交付物/门 |
|---|---|---|
| g32_wave_b_hzb_wiring | HZB 遮挡剔除生产接线（§1.2 #6） | D-G32-1 / G-G32-1 |
| g32_wave_b_restir_wiring | ReSTIR 生产接线（§1.2 #7） | D-G32-1 / G-G32-2 |
| g32_wave_b_slab_wiring | slab 材质侧表生产接线（§1.2 #8） | D-G32-1 / G-G32-3 |
| g32_wave_b_texture_wiring | 纹理采样管线进生产场景（§1.2 #9） | D-G32-1 / G-G32-4 |
| g32_wave_b_skinning_wiring | 蒙皮/骨骼动画进生产帧（§1.3 #10，RD-041 兑现窗） | D-G32-1 / G-G32-5 |
| g32_wave_b_gi_default_decision | GI 默认档 measured 权衡窗（§1.2 #12） | D-G32-1（g31_gi_default_tier_decision.json） |
| g32_wave_b_oit_evaluation_window | OIT/半透明评估窗（§1.2 #13） | D-G32-1（g31_oit_evaluation_window.json） |
| g32_wave_b_combo_matrix_and_demo | 特性组合矩阵 + 游戏画面 demo 定版 | D-G32-2 / G-G32-6 |
| g32_wave_b_acceptance_gate | 波 B 验收门（守卫/零降级三面/RD-045 复核/四件套） | D-G32-2/D-G32-3 / G-G32-6 |

### 2.2 out-of-scope（显式排除）

- #11 BistroExterior 场景转换臂——维持 G10-N6 锚挂起（fbx2gltf/assimp/blender 三工具 PATH 全缺 + 源资产三根检索 0 命中；锚 = FBX2glTF 上游修复在树或替代臂 + 源资产同窗齐备），双场景闭集（BistroInterior+CornellBox）维持。
- HDR 显示链实现（M118-hdr-cal maintain-SDR 字面维持，锚不变）。
- 任何新优化/新特性工作、冻结面（G13~G31 注册表/锚/契约条款）改写——front matter out_of_scope 逐字。
- presented 帧率冒充真实渲染帧率（双口径分离硬线）。
- 跨 harness 特性组合冒充单进程组合（闭集面核验在案：--skin-demo/--dyn-demo 不进 window_present 参数闭集）。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-G32-1 | B1~B5 五硬门 + B6/B7 两评估窗 | ci/g31_{hzb,restir,slab,texture_sampling,skinning}_wiring_smoke.py + 七 evidence schema + 两决策 JSON | 五门 --gate 全 PASS（在案）+ 两窗决策只追加（在案） |
| D-G32-2 | B8 波 B 验收门 | milestones/g31/g31_waveb_combo_matrix.json + g31_waveb_rd045_observation_results.json + §8 close-out 实测 facts | 组合矩阵 5 臂绿 + 互斥 12 拒 + demo ≥200 帧 + 零降级三面 + 守卫五条 + RD-045 窗（§8.1 六面登记） |
| D-G32-3 | 四件套 | milestones/g32/{G32_PLAN.md,G32_CONTRACT.md,CI_GATES.md,g32_budget.json} | 在树 + budget_eval 全 PASS 零 estimated |

## 4. 验收门（完整版，YAML 头为可提取摘要）

G-G32-1~G-G32-6 逐字见 front matter `acceptance_gates`。性能/帧率面证据等级 = measured_local（RTX 4070 Ti + Vulkan，本机真跑，gpu_device_lock 串行，RURIX_VK_VALIDATION=1）；采样协议 = 各门脚本内登记（canonical 160 帧 warmup 10 / 门各自 frames+warmup 口径 / 组合矩阵 64+10 与 demo 200+10 窗）。

## 5. Guardrails（字节级，机器核对）

见 YAML 头 `guardrails` 字段。核对方式：`ci/check_guardrails.py`（agent 完全自主模式 ADVISORY 不阻断）+ `ci/check_schemas.py` / `ci/budget_eval.py` 硬门。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-041 | 蒙皮 WPO MV 在动态资产面出现时接通（三类速度设计） | B5 兑现：类 3 蒙皮 MV 经 g31_skin_mv 进 TSR 历史链（核验 20/20）；类 2 刚性实例缺口维持 A4 登记不冒充 |
| RD-045 | 间歇 digest 漂移 backfill 三件（定位/修复/Full RFC 评估），open/maintain-open | 本波观察窗复核 = 波 B 各臂 digest 锚 6/6 零漂移 + Stage A 18/18 ×2 跑——累计观察面只追加（三件 0/3 维持不冒充，F5 硬线同源） |

详情以 [../registry/deferred.json](../registry/deferred.json) 为唯一事实源，本表仅引用。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-26 | 初版契约固化（波 B 批次 = 画面完整期首波；G31_PLUS §6 波次线 2 承接；B1~B7 交付 + B8 验收 §8 close-out） |

---

## 8. Close-out 区（只追加 — 波 B 验收记录）

### §8.1 波 B 验收门（B8）验收记录（2026-08-26，验证+治理 agent 真跑产出）——六面：五面全绿 + 焦点格诚实红面恶化如实登记

**① 组合矩阵核验（milestones/g31/g31_waveb_combo_matrix.json；全部真跑，release + RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 + --hidden 真窗口）**：

*可组合臂 5/5 真跑绿（双跑 digest 位级一致 + 帧率 measured_local）*：

| 臂 | 命令面 | digest（双跑位级一致） | real_render/prod frame_ms | present frame_ms |
|---|---|---|---|---|
| C0 base orbit | g31_window_present --frames 64 --warmup 10 --auto-move orbit | sha256:060e69a81e26…（== B3/B4 门 off 在案锚，跨日零漂移） | 4.114 / 3.871 | 0.959 / 0.970 |
| C1a textures on + orbit | + --textures on（SPV 在树） | sha256:81d330150f60…（== B4 门 on 在案锚，跨日零漂移） | 3.772 / 3.909 | 0.960 / 0.987 |
| C1b skin-demo（跨 harness 面，如实不冒充单进程组合） | g14_3_pipeline_perf --bench --skin-demo --frames 64 --warmup 10 | sha256:d57645c9d57e…（位级一致） | prod 3.473 / 3.962（frame 6.043/6.628，fps 165.5/150.9；核验 7/7 帧双跑过） | —（bench 车道无 present 口径） |
| C2 slab-table + orbit | + --slab-table 资产 --slab-arm device | sha256:7359fd905e0d…（== B3 门 device 在案锚，跨日零漂移） | 3.738 / 3.801 | 0.958 / 0.968 |
| C3 hzb on + orbit | + --hzb on（五 SPV 在树） | sha256:6acc15081205…（新窗锚位级一致） | 18.667 / 18.386（on≈4.7×base，orbit 动态相机剔除闭环工作量如实登记不设通过线） | 0.991 / 0.974 |

*互斥组合 fail-closed 拒绝核验 12/12（全 exit=1 逐字拒跑，零冒充可组合）*：M1 --hzb on×--fg x2 / M2 --hzb on×--slab-table / M3 --slab-table 无 --auto-move / M4 --slab-table×--fg / M5 --fg 无 --auto-move / M6 --dyn-demo×--inflight 2 / M6b --skin-demo×--inflight 2 / M7 --textures on×--hzb on / M8 --textures on×--fg / M9 --textures on×--slab-table / M10 --skin-demo 不进 window_present 闭集（未知参数）/ M11 --dyn-demo 不进 window_present 闭集（未知参数）。

**② 游戏画面 demo 定版**：定版臂 = `g31_window_present --textures on --auto-move orbit`（真窗口车道最大可组合集：贴图材质 12 槽 697878 三角 + 生产 GI 直接光车道 + 确定性轨迹动态相机 + swapchain 逐帧 present；HZB 剔除/蒙皮角色/slab 侧表各臂同窗真跑绿件在案，蒙皮角色面跨 harness 登记不冒充单进程组合）。真窗口 **200+10 帧双跑真跑**：frames_completed=210 exit=frames_done 零崩零 resize era；digest 双跑位级一致 sha256:cf3532f442df…；**帧率双口径**：real_render_frame_ms = 5.1131 / 5.4308（real fps 195.6 / 184.1，含强制回读段如实登记）+ present_frame_ms = 1.0044 / 1.0126（present 口径独立登记）+ encode_gpu 0.210/0.231ms；evidence .tmp/g31_gates/waveB_matrix/demo200_{a,b}.json。

**③ 零降级回归三面（ci/g31_wave_a_anchor_check.py 范式 + ci/g16_absolute_quality_closure_smoke.py --gate）**：
- **Stage A digest 锚 18/18 零漂移**：canonical 160 帧 warmup 10 逐格重跑（RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 + GPU 独占窗），本日双跑（evidence/g31_wave_a_anchor_check_20260826T094943Z.json / 20260826T100312Z.json）18 格末帧 digest 与 milestones/g14/g14_3_stage_a_digest_anchor.json 在案锚逐格位级 MATCH（18/18 ×2 跑，零漂移）。
- **画质面 G16plus M-g 18/18 复跑不降级**：`py -3 ci/g16_absolute_quality_closure_smoke.py --gate g16.p0.m_g.absolute_quality_closure`（步骤 290 门）→ **VERDICT=PASS** 8/8 facts：met_count_18（18/18 verdict=达标）+ thresholds_program_produced（entries=4，k=2.0 程序产禁手写维持）+ m_c_history_honest_0_18 + g15_budget_0byte + ai_reading_bound + no_threshold_loosening + commercial_closure_pass；UE 参照臂按在案锚只读消费不重跑；evidence evidence/g16_m_g_absolute_quality_closure_20260826T100706Z.json。
- **性能面 G17-MD-F1 焦点格 fresh——诚实红较前次轨迹恶化，如实 RED 登记不冒充**：bistro-interior/t100/dlss_sr canonical 160 帧本日新鲜 5 样本 frame_ms_production_mean = 3.7076（锚检跑①，ratio 0.926562）/ 3.9232（锚检跑②，ratio 0.875645）/ 3.5674（独立跑①，ratio 0.962972）/ 3.5190（独立跑②，ratio 0.976229）/ 3.5929（独立跑③，ratio 0.956162）ms；**样本中位 3.5929ms → 中位 ratio 0.956162 < 在案 0.960479**（ue_median 在案锚 3.43535ms 只读消费）——锚检门两跑均判 RED（evidence 双件 append-only 留存不删），17/18 诚实红维持但较前次在案轨迹（0.856→0.960→0.966）恶化；**同格 digest 五跑全 == 在案锚（sha256:55ea0c2b…）——确定性面零漂移，恶化面 = 帧时机态抖动非渲染产出漂移**；机态上下文如实登记：本格历史基线 g14_budget 在案 4.195ms / g17 baseline rurix 3.7955ms，本日样本仍优于历史基线、劣于 G30.2/G31 波 A 新鲜最优（3.5767/3.5560ms）；nvidia-smi 实测 GPU 35℃ 非热饱和，方差源未逐字定位（候选 = 机态背景负载/DLSS vendor 初始化抖动，不冒充根因）；帧时预算杠（在案 ×2.0 = 7.1534ms）远未触及（g32.baseline.focus_cell.frame_ms_production_mean 条目 PASS）。

**④ 守卫套件五条全跑（仓库根目录，exit 全 0）**：
- `py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`，exit=0。
- `py -3 ci/check_schemas.py` → `[check_schemas] PASS`，exit=0（含本批 `g32_baseline_` 快检件跳过路由一处纯追加重放核验）。
- `py -3 ci/check_number_ledger.py` → PASS（ADVISORY 不阻断：off_tree_workflows[grx] branch/closeout_commit exists 两注；spec RXS 头 389 个零同号碰撞 + ledger 14 命名空间保留号被尊重 + red 自检已过），exit=0。
- `py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (389/389 clauses anchored, 875 test files scanned)`，exit=0。
- `py -3 ci/budget_eval.py` → `[budget_eval] PASS (307 pass, 0 skip, normal mode)`，exit=0（含 g18~g31 各期 baseline.stage_a_digest_guard 18/18 全绿在案 + 本批 g32 七条 measured_local 全绿）。

**⑤ RD-045 观察窗复核（milestones/g31/g31_waveb_rd045_observation_results.json + registry/deferred.json history 只追加）**：波 B 各臂 digest 锚全中零漂移——base（累计 4 跑位级）/ slab device（3 跑）/ textures（4 跑）三臂 == B3/B4 门在案锚跨日零漂移 + hzb/skin/demo200 三臂新窗双跑位级一致 = **6/6 臂零漂移**；Stage A 18/18 ×2 跑零漂移；**backfill 三件盘点机器实测 0/3 维持**（①根因确证记录指定落点缺——F5 防冒充硬线：观察窗零漂移不充①件；②①未齐修复无法确证，G14.10 结构性缓解事实维持登记 ≠ 确证修复；③rfcs/ 主题检索零命中维持）→ disposition = maintain-open-with-extended-zero-recurrence 只追加扩窗登记，不冒充 close。

**⑥ 编号纪律**：CI 数字步骤零消费（落盘前实测 registry/number_ledger.json CI_step.next_free=525 维持；波 B 七门 + B8 验收面均未占号，pr-smoke.yml 无 g31/g32 条目）；RFC/RXS/RD/U/SG/MR/D/RX_error 共享段零消费（RFC next_free=48 / RXS next_free=408 / RD next_free=46 维持实测——RD-045 history 只追加扩窗登记，RD 编号段零消费）。

**⑦ 波 B 验收总结论**：B1~B7 交付面全绿/如实在案 + B8 验收六面中五面全绿（组合矩阵/demo/Stage A digest/G16 M-g/守卫套件/RD-045 复核），**G17-MD-F1 焦点格 fresh 诚实红面 = 恶化如实登记**（中位 ratio 0.956162 < 在案 0.960479；digest 零漂移维持；帧时预算杠 ×2.0 远未触及）——按「诚实登记不冒充；达标/维持/诚实红均合法终态」纪律，波 B 验收门以 **五面绿 + 一面诚实红（性能轨迹面）** 终态登记，无任何面冒充。

**⑧ 签署**：`Assisted-by: TraeCode:Kimi-K3（G31+ 波 B 验收门 Task B8）`。
