---
contract: G31
title: G31 实时呈现期（波 A：生产管线真窗口呈现 + 帧流水化 + 游戏循环 + 动态场景 + 帧生成接线 + 波 A 验收门）
status: active
implementation_status: unlocked
active_scope: g31_wave_a_realtime_present
version: v1.0
date: 2026-08-25
timebox: "波 A = g30-closed 后即时接续波（A1~A5 实现 + A6 验收）；波 B+ 内容归 G32 后续期立项程序（G31_PLUS_COMMERCIAL_RENDERER_TODO §6 波次线 2/3）"
rfc_required: "波 A 零新 RFC——实现全部消费既有语义面（RFC-0030 管线结构性优化 / RFC-0035 FG 独立层 / RFC-0036 FG 实现语义 / RFC-0043 FG device kernel + G26~G30 战役既定锚与 g30_campaign_handover_registry G13-N7 行生产接线窗锚）；零新 RXS 条款、零共享编号段消费（落盘前实测：RFC next_free=48 / RXS next_free=408 / CI_step next_free=525 全维持）"
upstream_docs:
  - "milestones/g30/g30_campaign_handover_registry.json（RFC-0047 §5.5：G31+ 唯一法定输入面；G13-N7 行 = FG 生产接线窗锚）"
  - "G31_PLUS_COMMERCIAL_RENDERER_TODO.md §1.1 #1~#4 + §1.2 #5 + §6 波次线 1（G31 = 实时呈现期定义）"
  - "registry/deferred.json（RD-045 确定性观察面）"
in_scope:
  - g31_wave_a_window_present
  - g31_wave_a_frame_pipelining
  - g31_wave_a_game_loop
  - g31_wave_a_dynamic_scene
  - g31_wave_a_framegen_wiring
  - g31_wave_a_acceptance_gate
out_of_scope:
  - wave_b_content（HZB/ReSTIR/slab 生产接线、纹理采样管线、蒙皮动画、GI 默认档、BistroExterior——G32 画面完整期候选，G31_PLUS §6 波次线 2）
  - hdr_display_chain_implementation（M118-hdr-cal maintain-SDR 字面维持）
  - new_optimization_or_feature_work_beyond_wave_a
  - rewriting_g13_g30_frozen_registries_or_anchors
  - presented_fps_masquerading_as_real_render_fps（双口径分离，生成帧禁入真实渲染帧率口径）
deferred_refs: [RD-045]
deliverables:
  - id: D-G31-1
    name: 波 A 实现五门（A1~A5 harness + smoke + evidence schema，均 gate PASS 在案）
  - id: D-G31-2
    name: 波 A 验收两门（anchor_check + soak 脚本与 schema，check_schemas 三处纯追加接线）
  - id: D-G31-3
    name: G31 四件套（PLAN/CONTRACT/CI_GATES/g31_budget.json，零 estimated 全 measured）
acceptance_gates:
  - id: G-G31-1
    check: "A1 真窗口呈现门 g31.waveA.present PASS：bistro-interior 1080p 真 swapchain present 逐帧成功 + evidence 口径三分离（real_render/present/encode）核验（measured_local，device 真跑 RURIX_VK_VALIDATION=1）"
  - id: G-G31-2
    check: "A2 帧流水化门 g31.waveA.pipelining PASS：in-flight 1/2/3 臂 A/B 帧时 measured + 跨臂逐帧 digest 位级一致 + 帧序严格 FIFO（确定性协议零破坏）"
  - id: G-G31-3
    check: "A3 游戏循环门 g31.waveA.gameloop PASS：--auto-move orbit 双跑 digest_seq 位级一致 + dolly 异轨迹区分 + ev100-ramp 异曝光区分 + resize/alt-tab swapchain era 重建不崩"
  - id: G-G31-4
    check: "A4 动态场景门 g31.waveA.dynscene PASS：refit/rebuild 双臂逐帧 64B 实例增量位置核验全中 + 跨臂 digest 位级一致 + 静态回归锚 == g14 Stage A 锚同格"
  - id: G-G31-5
    check: "A5 FG 接线门 g31.waveA.framegen PASS：--fg x2/x3 presented/real 双口径恒等式 + x2 双跑 digest_seq 位级一致 + G26 对拍门接线态复跑 pass（p100 ≤ 冻结容差 7.152557e-07）"
  - id: G-G31-6
    check: "A6 波 A 验收门：守卫套件七条 exit 0 + 五门复跑全 PASS + Stage A digest 锚 18/18 canonical 160 帧重跑零漂移 + G16plus M-g 18/18 canonical 复跑不降级（UE 臂按在案锚不重跑）+ G17-MD-F1 焦点格新鲜真跑诚实红不恶化（fresh ratio ≥ 在案 0.960479）+ soak ≥10000 帧（或 ≥30min 墙钟取先达）零崩 + validation 静默 + leak 账本零 + digest_seq 确定性抽查双跑位级一致"
guardrails:
  - "诚实登记不冒充：所有数字来自真实命令输出；达标/维持/诚实红均合法终态"
  - "append-only：evidence/ 只增不删不改；deferred history 只追加；既有锚/注册表 0-byte 不回写"
  - "双口径分离：presented 帧率（含 FG 生成帧）与 real_render 帧率独立登记，禁混入"
  - "三态纪律：dev-env 降级 = SKIP 如实登记，禁冒充 PASS；RURIX_REQUIRE_REAL=1 翻硬 FAIL"
  - "commit 带 Assisted-by: trailer 且不 push"
---

# G31 契约 — 实时呈现期（波 A 范围）

> 所属：[../11_ROADMAP.md](../11_ROADMAP.md) §3 / G31+ 期待办总表 [../G31_PLUS_COMMERCIAL_RENDERER_TODO.md](../G31_PLUS_COMMERCIAL_RENDERER_TODO.md) §6 波次线 1；契约机制见 14 §1。front matter 双状态机：`status` 与 `implementation_status` 严格分离。

---

## 1. 目标

波 A 结束时，项目获得：**bistro 级场景在真窗口内以生产管线实时渲染并呈现**——五 pass 生产车道（g14_3_lane_body 逐字共享）输出经 device 侧显示编码接 win32 真 swapchain 逐帧 present，帧流水化（submit/collect 分离）就位，确定性轨迹游戏循环（相机/曝光逐帧 uniform + resize/alt-tab 健壮）就位，动态场景 refit/rebuild 通路就位，G26 帧生成 device kernel 生产接线（--fg off/x2/x3，presented/real 双口径分离）就位；且全部既有画质/性能/确定性锚零降级（机器核验在案）。

## 2. 范围

### 2.1 in-scope（波 A）

| 项 | 说明 | 对应交付物/门 |
|---|---|---|
| g31_wave_a_window_present | 生产管线 swapchain 真窗口呈现（G31_PLUS §1.1 #1） | D-G31-1 / G-G31-1 |
| g31_wave_a_frame_pipelining | 帧流水化 N in-flight（§1.1 #2） | D-G31-1 / G-G31-2 |
| g31_wave_a_game_loop | 游戏循环最小面（§1.1 #3） | D-G31-1 / G-G31-3 |
| g31_wave_a_dynamic_scene | 动态场景更新通路（§1.1 #4） | D-G31-1 / G-G31-4 |
| g31_wave_a_framegen_wiring | FG/MFG 生产接线（§1.2 #5，G13-N7 锚） | D-G31-1 / G-G31-5 |
| g31_wave_a_acceptance_gate | 波 A 验收门（守卫/复跑/三锚/soak/四件套） | D-G31-2/D-G31-3 / G-G31-6 |

### 2.2 out-of-scope（显式排除）

- 波 B 内容面（HZB #6 / ReSTIR #7 / slab #8 生产接线、纹理管线 #9、蒙皮动画 #10、GI 默认档 #12、BistroExterior #11）——归 G32 后续期立项程序，本波不预开。
- HDR 显示链实现（M118-hdr-cal maintain-SDR 字面维持，锚不变）。
- 任何新优化/新特性工作、冻结面（G13~G30 注册表/锚/契约条款）改写——front matter out_of_scope 逐字。
- presented 帧率冒充真实渲染帧率（双口径分离硬线）。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-G31-1 | A1~A5 五门 | src/rurix-render/src/bin/g31_window_present.rs（统一 harness）+ ci/g31_{window_present,frame_pipelining,game_loop,dynamic_scene,framegen_present}_smoke.py + 五 evidence schema | 五门 --selftest + --gate 全 PASS（本波复跑在案） |
| D-G31-2 | A6 验收两门 | ci/g31_wave_a_anchor_check.py + ci/g31_wave_a_soak.py + 两 evidence schema + check_schemas 三处纯追加 | 两门 --selftest + --gate 全 PASS |
| D-G31-3 | 四件套 | milestones/g31/{G31_PLAN.md,G31_CONTRACT.md,CI_GATES.md,g31_budget.json} | 在树 + budget_eval 全 PASS 零 estimated |

## 4. 验收门（完整版，YAML 头为可提取摘要）

G-G31-1~G-G31-6 逐字见 front matter `acceptance_gates`。性能/帧率面证据等级 = measured_local（RTX 4070 Ti + Vulkan，本机真跑，gpu_device_lock 串行，RURIX_VK_VALIDATION=1）；采样协议 = 各门脚本内登记（canonical 160 帧 warmup 10 / 门各自 frames+warmup 口径）。

## 5. Guardrails（字节级，机器核对）

见 YAML 头 `guardrails` 字段。核对方式：`ci/check_guardrails.py`（agent 完全自主模式 ADVISORY 不阻断）+ `ci/check_schemas.py` / `ci/budget_eval.py` 硬门。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-045 | 间歇 digest 漂移 backfill 三件（定位/修复/Full RFC 评估），open/maintain-open | 本波 soak digest_seq 双跑位级一致 + Stage A 锚 18/18 零漂移 = 累计观察面只追加（不充三件，F5 硬线同源） |

详情以 [../registry/deferred.json](../registry/deferred.json) 为唯一事实源，本表仅引用。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 初版契约固化（波 A 范围 = 实时呈现期；G31_PLUS §6 波次线 1 承接） |

---

## 8. Close-out 区（只追加 — 波 A 验收记录）

### §8.1 波 A 验收门（A6）验收记录（2026-08-25，验收+治理 agent 真跑产出）——G-G31-6 六面全绿

**① 守卫套件七条全跑（仓库根目录，exit 全 0）**：
- `py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`，exit=0。
- `py -3 ci/check_schemas.py` → `[check_schemas] PASS`，exit=0（含本波两新 schema 路由重放核验）。
- `py -3 ci/check_number_ledger.py` → PASS（ADVISORY 不阻断：off_tree_workflows[grx] branch/closeout_commit exists 两注；spec RXS 头 389 个零同号碰撞 + ledger 14 命名空间保留号被尊重 + red 自检已过），exit=0。
- `py -3 ci/check_guardrails.py` → exit=0，ADVISORY 清单（base=g7-closed 不阻断：规划文档 M 改动 / spec 修订记录行 / evidence 既有文件修改注记——agent 完全自主模式建议项，10 §7 v2.0；**预存项，非本波引入**：本波未 commit，工作树改动为 A1~A6 交付面本身）。
- `py -3 ci/check_contribution.py` → exit=0，ADVISORY 清单（历史 commit provenance/验证标记/对抗性评审注记——**预存项**，agent 完全自主模式建议项不阻断）。
- `py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (389/389 clauses anchored, 875 test files scanned)`，exit=0。
- `py -3 ci/budget_eval.py` → `[budget_eval] PASS (301 pass, 0 skip, normal mode)`，exit=0（含 g18~g30 各期 baseline.stage_a_digest_guard 18/18 全绿在案）。

**② 五门复跑（--selftest + --gate 全 PASS，2026-08-25 真跑）**：
- g31.waveA.present：selftest PASS（1 GREEN + 6 RED + schema 互核）；gate PASS——真跑口径 real_render=60.052ms present=1.100ms overhead=1.100ms encode=0.000ms digest=sha256:94a2cfc63fa34453…（debug 构建腿，3+1 帧逐帧 present 成功 + 口径恒等式 + 转引 consistency=pass）。
- g31.waveA.pipelining：selftest PASS（红绿臂全如预期）；gate PASS——arm inflight=1 mean=2.8309 p50=1.8344 prod=2.0320 / inflight=2 mean=2.8305 p50=1.5185 prod=1.9729 / inflight=3 mean=2.5940 p50=1.5371 prod=1.8315（**fresh p50：inflight2 −17.2% / inflight3 −16.2% vs inflight1**，三臂 digest 位级一致 sha256:ac30d022d382b93d… + trace 侧跑帧序严格 FIFO=True 逐帧 digest 位级一致=True）；evidence evidence/g31_frame_pipelining_20260825T211934Z.json。**诚实登记：交付波登记值 p50 −23.5% 为彼次真跑口径，本次复跑 fresh 值如上，两臂数字不冒充同值。**
- g31.waveA.gameloop：selftest PASS（2 GREEN + 7 RED + 比较器 4 象限 + schema 互核）；gate PASS——orbit 双跑 digest_seq 位级一致 + dolly 异轨迹不同 + orbit+ramp 异曝光不同；真跑口径 real_render=4.418ms present=0.986ms encode_gpu=0.105ms；evidence evidence/g31_game_loop_20260826T014058Z.json。
- g31.waveA.dynscene：selftest PASS（红绿臂全如预期）；gate PASS——refit arm mean=4.0549 p50=1.9026 prod=2.6722 verify=30/30 / rebuild arm mean=4.2803 p50=1.9559 prod=2.8251 verify=30/30（**位置核验 60/60**），跨臂 digest 位级一致 sha256:652f7084c84a9258…，静态回归锚 sha256:c1d28ad73783cc3c… == g14 锚=True 且 动≠静=True；evidence evidence/g31_dynamic_scene_20260825T213448Z.json。
- g31.waveA.framegen：selftest PASS（2 GREEN + 10 RED + 比较器 4 象限 + schema 互核）；gate PASS——x2 双跑 digest_seq 位级一致 + fg off 不污染 + x3 一致 + dolly 异轨迹 + **G26 对拍门接线态复跑 pass**（x2 p100=2.980e-07 / x3 p100=3.576e-07 / x4 p100=2.980e-07 全 ≤ 冻结容差 7.152557e-07，SSIM 全帧严格胜 frame-hold，双跑位级）；x2 真跑口径 real_render=15.190ms real_fps=65.83 presented_fps=116.91 fg_gpu=3.457ms wired_p100=9.678e-04（presented/real 双口径分离登记）。

**③ 零降级回归锚三面（A6 程序产，evidence/g31_wave_a_anchor_check_20260825T215125Z.json）**：
- **Stage A digest 锚 18/18 零漂移**：canonical 160 帧 warmup 10 逐格重跑（g14_3_pipeline_perf 既有口径，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 + GPU 独占窗），18 格末帧 digest 与 milestones/g14/g14_3_stage_a_digest_anchor.json 在案锚逐格位级 MATCH（18/18，零漂移）。
- **画质面 G16plus M-g 18/18 复跑不降级**：canonical 复跑入口 `py -3 ci/g16_absolute_quality_closure_smoke.py --gate g16.p0.m_g.absolute_quality_closure`（步骤 290 门）→ VERDICT=PASS 8/8 facts：met_count_18（18/18 verdict=达标）+ thresholds_program_produced（entries=4，双 seed 方差底 p100×2.0 程序产禁手写维持）+ m_c_history_honest_0_18（G15 历史 0/18 未改写）+ g15_budget_0byte + ai_reading_bound + no_threshold_loosening；Rurix 侧产出复跑（RXS-0357 位级确定性复用 ≡ 重跑：receipt 全要素合法 + converged digest 重算一致，度量面全新重算），UE 参照臂按在案锚只读消费不重跑；evidence evidence/g16_m_g_absolute_quality_closure_20260825T215544Z.json。
- **性能面 G17-MD-F1 焦点格新鲜真跑不恶化**：bistro-interior/t100/dlss_sr canonical 160 帧新鲜 frame_ms_production_mean=**3.5560ms**（在案 3.5767ms，G30.2 M-b 20260825T102813Z）→ fresh ratio = ue_median 3.43535ms / 3.5560ms = **0.966059 ≥ 在案 0.960479**——17/18 诚实红维持且不恶化（ratio  trajectory 0.856→0.960→0.966）；其余 17 格 fresh frame_ms 逐格对照 g14_budget 在案 measured 登记于 anchor_check evidence cells[]（全格 fresh ≤ 在案，无恶化格）。

**④ soak（g31.waveA.soak，ci/g31_wave_a_soak.py 产证）**：门 PASS——主腿 `g31_window_present --frames 10000 --warmup 10 --auto-move orbit --hidden`（release，RURIX_VK_VALIDATION=1 + GPU 独占窗）**10010 帧 present 零崩**（frames_presented=10000+10 计数恒等，exit_reason=frames_done），wall=556.928s（≥10000 帧先达口径，A6 任务书「≥10000 帧 或 ≥30min 墙钟取先达」），real_render=21.191ms / present=3.013ms 双口径登记（含逐帧 digest 强制回读口径，render_includes_forced_readback=true）；**validation 全程静默**（零 "Validation Error"/"VUID-"，harness 逐帧 validation_error_count/leak 账本硬门零触发 = **资源无泄漏机核绿**）；**digest 序列确定性抽查**：同轨迹 64+4 帧双跑 digest_seq 68 项逐帧位级一致（首漂移帧=-1，末帧 sha256:1df8f75714e08abb…）；evidence evidence/g31_wave_a_soak_20260825T223024Z.json。**过程诚实登记**：首跑因验收脚本侧两处口径误判（frames_completed/digest_seq 未含 warmup 迭代数）判 FAIL——evidence/g31_wave_a_soak_20260825T220747Z.json 如实留存不删（append-only），判据修正（harness 口径 = frames+warmup 迭代总数）后本跑全绿；harness 本身双腿零异常。

**⑤ 编号纪律**：CI 数字步骤零消费（落盘前实测 registry/number_ledger.json CI_step.next_free=525 维持；波 A 七门均未占号，pr-smoke.yml 无 g31 条目）；RFC/RXS/RD/U/SG/MR/D/RX_error 共享段零消费（RFC next_free=48 / RXS next_free=408 / RD next_free=46 维持实测）。

**⑥ 签署**：`Assisted-by: TraeCode:Kimi-K3（G31+ 波 A 验收门 Task A6）`。
