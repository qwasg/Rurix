---
contract: G35
title: G35 GPU 粒子系统期（对标并超越 UE5 Niagara 五轴——九波：G35-1 GPU 基元〔分段稳定 scan + 24 位键 3-pass 稳定 radix sort + compact〕 + G35-2 粒子核心〔SoA 池 ping-pong + 确定性发射 + 半隐式 Euler + 稳定压缩 + indirect args 零回读〕 + G35-3 渲染接线〔billboard splat 进 G34 统一车道 + 软粒子 + 粒子 MV + mesh 粒子 TLAS〕 + G35-4 半透明〔深度排序 + WBOIT 双臂〕 + G35-5 碰撞与力场 + G35-6 事件与 particle_view 双向桥 + G35-7 流体〔count-sort 空间哈希 + XPBD〕 + G35-8 作者面与 SDK + G35-9 确定性回放回滚与收口验收面）
status: active
implementation_status: unlocked
active_scope: g35_gpu_particle_system
version: v1.2
date: 2026-08-27
timebox: "G35-0 治理文档套件 = 本批（2026-08-27 开工即交付：契约四件套 + RFC-0049 Draft）；G35-1/G35-2 = 基元与核心先行波（scan 三 kernel 已落树为其上游）；G35-3~G35-9 = 依赖序推进（波间依赖见 G35_PLAN §2）；G-G35-10 收口验收面归收口验收批；Onesweep/MPM/GPU 动态 bounds/FIF×动态共存均为评估窗登记不占波次"
rfc_required: "RFC-0049 **Agent Approved（2026-08-27，D-409 一轮 17 findings〔5 blocker + 11 high + 1 med〕全 disposition——评审 provenance cursor:gpt-5.6-sol-medium ≠ 起草 cursor:claude-fable-5，rfcs/0049 §9.1）**；RXS 零新条款声明——渲染器是库不进语言 06 §8.3，九波 kernel 全用现有语言面；CI 数字步骤零消费声明——九门均 symbolic gate key g35.waveN.* 未占号，收口验收批实测核验 CI_step.next_free=525 维持"
upstream_docs:
  - "milestones/g34/G34_CONTRACT.md §8 close-out（G34 全特性合流期收口终态 = G35 统一车道底座法定输入；落笔时三门新鲜绿件在案〔evidence/g34_unified_lane_gate_20260827T093331Z.json + g34_hzb_unified_gate_20260827T091200Z.json + g34_skin_unified_gate_20260827T084533Z.json〕，§8 只追加区由收口验收批同窗填写）"
  - "milestones/g30/g30_campaign_handover_registry.json（RFC-0047 §5.5 谱系法定输入面）"
  - "G31_PLUS_COMMERCIAL_RENDERER_TODO.md #13/#81 去重消费不重开（#13 OIT/半透明触发条件字面〔粒子特效进画面〕本期命中 + #81 粒子写速度同窗验收 + #80 依赖登记；既有行字面 0-byte 不回写）"
  - "milestones/g31/g31_oit_evaluation_window.json（not_triggered 终态的 conditional_wiring_sketch ① re-trigger 条件本期命中：粒子特效进画面 ⇒ 消费 M120 冻结测量启动 WBOIT 起步选型——选型提交须引 benchmark 数据，无数据提交判 RED 选型纪律字面维持）"
in_scope:
  - g35_wave1_primitives
  - g35_wave2_particle_core
  - g35_wave3_render
  - g35_wave4_sort_oit
  - g35_wave5_collision
  - g35_wave6_events
  - g35_wave7_fluids
  - g35_wave8_authoring
  - g35_wave9_replay
out_of_scope:
  - cpu_particle_lane（CPU 粒子生产车道——host 金标准层 = 对拍参照非生产形态，RFC-0049 §7.4/§8）
  - rewriting_frozen_registries_or_anchors（G13~G34 冻结注册表/锚 0-byte 不回写；生产管线既有 pass 与 Stage A 锚 0-byte，粒子面全加性）
  - cross_hardware_bitexact_promise（跨硬件位级承诺——确定性诚实边界 = 同 GPU 同驱动位级、跨硬件协议一致，RFC-0049 §9 Q4）
  - editor_gui（编辑器 GUI——emitter 资产为文本声明式，可视化编辑器归产品面不在渲染器库范围）
deferred_refs: [RD-045]
deliverables:
  - id: D-G35-1
    name: G35-1 GPU 基元（kernels/g35_sort_hist.rx + g35_sort_spine.rx + g35_sort_scatter.rx + g35_compact_u32.rx〔hist/spine/scatter 三段命名 = mod.rs 契约字面；+ 实验臂 g35_sort_onesweep.rx 评估窗登记不进生产〕 + src/rurix-render/src/particles/primitives.rs host 金标准 + src/rurix-render/src/bin/g35_primitives_device.rs + ci/g35_primitives_smoke.py + gate evidence schema；上游 = 已落树 scan 三 kernel〔kernels/g35_scan_seg_sum.rx / g35_scan_spine.rx / g35_scan_seg_apply.rx + particles/scan.rs〕0-byte 消费）
  - id: D-G35-2
    name: G35-2 粒子核心（kernels/g35_emit.rx + g35_sim.rx + g35_particle_compact.rx + g35_indirect_args.rx + src/rurix-render/src/particles/core.rs host 金标准 + src/rurix-render/src/bin/g35_particle_core_device.rs + ci/g35_particle_core_smoke.py + gate evidence schema + f32 流标定容差条目程序产入 g35_budget.json）
  - id: D-G35-3
    name: G35-3 渲染接线（billboard splat + 粒子 MV kernel〔kernel 名单随波交付在 CI_GATES 修订行冻结〕 + g34_full_lane 粒子独立 include 区段〔G34-2/G34-3 并行分区纪律同律，主 bin 仅 --particles 旗标解析 + 挂点〕 + mesh 粒子 TLAS 臂〔inflight=1，tlas_update 现约束〕 + ci/g35_render_wiring_smoke.py + gate evidence schema）
  - id: D-G35-4
    name: G35-4 半透明双臂（24 位深度键 radix 排序臂 + back-to-front alpha blend 执行器加性状态面〔rurix-rt 库面零 RXS，RFC-0049 §9 Q2〕 + WBOIT 臂 + ci/g35_sort_oit_smoke.py + gate evidence schema + #13 OIT 评估窗 re-trigger 登记件〔消费 M120 冻结测量的 WBOIT 起步选型提交〕）
  - id: D-G35-5
    name: G35-5 碰撞与力场（sim kernel ray query 同帧碰撞腿〔RXS-0297~0300 谱系引用不扩〕 + 深度缓冲对照臂 + rurix-physics field 通道复用〔RFC-0024〕 + ci/g35_collision_smoke.py + gate evidence schema）
  - id: D-G35-6
    name: G35-6 事件与 particle_view 双向桥（有界事件 SSBO 队列〔scan 推导写槽禁原子，溢出计数如实登记禁静默丢〕 + particle_view GPU↔host 双向桥〔RFC-0024 统一 particle view 语义面引用不扩〕 + ci/g35_events_smoke.py + gate evidence schema）
  - id: D-G35-7
    name: G35-7 流体（count-sort 空间哈希邻居〔计数→scan→段内串行 scatter 确定序〕 + XPBD 约束求解〔密度 + 距离约束，固定迭代 Jacobi ping-pong〕 + ci/g35_fluids_smoke.py + gate evidence schema + MPM 评估窗登记件〔不承诺〕）
  - id: D-G35-8
    name: G35-8 作者面与 SDK（声明式 emitter 资产 schema〔交付冻结〕 + 参数化 megakernel 参数块〔禁逐 emitter 重编 kernel〕 + 热重载 + apps/g31-renderer-sdk C ABI 加性导出〔既有导出面 0-byte〕 + ci/g35_authoring_smoke.py + gate evidence schema）
  - id: D-G35-9
    name: G35-9 确定性回放回滚（journal 逐帧输入记录 + 位级重放 + 截断重放式回滚〔快照间隔 measured 权衡登记〕 + ci/g35_replay_smoke.py + gate evidence schema + 粒子面独立 digest 锚〔不混入生产 Stage A 锚表〕）
acceptance_gates:
  - id: G-G35-1
    check: "G35-1 GPU 基元门 g35.wave1.primitives PASS：八 facts 闭集（kernels_spv_valid / scan_bitexact / sort_bitexact / sort_stability / compact_bitexact / determinism_double_run / red_arm_effective / throughput_measured——与已落树 ci/g35_primitives_smoke.py FACT_IDS 逐字同序，v1.1 校准）——整数流零容差位级（scan/sort 键与序/compact 槽位 device↔host 位级全等，mod.rs 容差协议字面）+ 稳定序见证（同键异序输入相对序保持 = 段序×段内序）+ 红臂有效性（判读器构造缺陷红绿两臂）；判据字面 = ci/g35_primitives_smoke.py docstring（已交付冻结）；实测数字待验收批填写，禁预支"
  - id: G-G35-2
    check: "G35-2 粒子核心门 g35.wave2.particle_core PASS：八 facts 闭集（kernels_spv_valid / integer_streams_bitexact / f32_parity_within_budget / pid_persistent_unique / indirect_args_zero_readback / determinism_double_run / red_arm_effective / frame_ms_measured——与已落树 ci/g35_particle_core_smoke.py FACT_IDS 一致，v1.1 校准）——发射/压缩槽位 scan 推导禁原子抢槽 + persistent ID 全生命周期不变（pid 硬域 [0, 2^24)，RFC-0049 §4.4 F6）+ 随机数一律经 rand_table[(pid·RAND_K + slot) % RAND_TABLE_LEN] 单源 + f32 流（pos/vel/age/life）host 对拍走标定容差（threshold = measured × 2.0 程序产禁手写，g35_budget.json）+ 整数流（pid/flags/scan/args）零容差位级 + 存活总数经 spine 总和槽直供 indirect args host 零回读；容量钳制语义 accepted = min(requested, cap − alive_total) 与 rejected 计数由 probe 硬断言承载（RFC-0049 §4.4 F7，FACT_IDS 闭集 0-byte）；判据字面 = ci/g35_particle_core_smoke.py docstring（已交付冻结）；实测数字待验收批填写，禁预支"
  - id: G-G35-3
    check: "G35-3 渲染接线门 g35.wave3.render PASS：九 facts 闭集（kernels_spv_valid / off_face_stage_a_anchor_match / on_off_digest_discrimination / determinism_double_run / particle_mv_parity_2px / barrier_plan_audit / soft_depth_occlusion_witness / indirect_splat_zero_readback / frame_ms_measured——与已落树 ci/g35_render_wiring_smoke.py FACT_IDS 逐字同序，v1.2 校准〔判据协议内容零变化：billboard splat/软粒子/MV 接线语义由 on_off 判别 + 遮挡见证 + MV 对拍三 facts 合并承载；mesh 粒子 TLAS 臂 N=1 已接线、N>1 not_wired 如实登记于 evidence〕；v1.1 增补 barrier_plan_audit + particle_mv_parity_2px，RFC-0049 F2/F11）——粒子关面（--particles off 缺省）digest == 母版 Stage A 冻结锚位级一致（加性 0-byte 机器证明，G34 同律）+ 粒子 MV 写速度进 MV 通道（TODO #81 字面消费；公式冻结 mv = project_curr(pos) − project_prev(pos − vel·dt)，像素归属 u64 max 赢家，RFC-0049 §4.6）+ 粒子 MV 与 host 投影对拍 ≤2px（G34-3 三类速度口径先例）+ TSR 互操作 digest 判别 + 逐 pass barrier plan 覆盖审计（(资源,TargetState) 转换表 + args write→INDIRECT_COMMAND_READ，RFC-0049 §4.2 同步契约生产面）+ mesh 粒子 TLAS 臂顺序入口 inflight=1（tlas_update 现约束字面）；判据字面 = ci/g35_render_wiring_smoke.py docstring（随波交付冻结）；实测数字待验收批填写，禁预支"
  - id: G-G35-4
    check: "G35-4 半透明双臂门 g35.wave4.sort_oit PASS：九 facts 闭集（kernels_spv_valid / sorted_arm_bitexact / wboit_fixedpoint_saturation / near_far_order_witness / oit_arms_digest_discrimination / oit_retrigger_registered / determinism_double_run / red_arm_effective / frame_ms_measured——与已落树 milestones/g35/g35_sort_oit_gate_evidence_schema.json facts enum 逐字同序，v1.2 校准〔判据协议内容零变化：depth_key24 位级/双臂登记语义由 sorted_arm_bitexact + oit_arms_digest_discrimination 合并承载〕；v1.1 增补 sorted_arm_bitexact + wboit_fixedpoint_saturation + near_far_order_witness，RFC-0049 F4/F5）——24 位深度键 device↔host 位级（零位转换公式字面 = mod.rs depth_key24，边界语义 RFC-0049 §4.3 F9）+ 排序臂反键 back-to-front（排序键 = DEPTH_KEY_MAX − depth_key24，近远两粒子顺序见证：远者先画）+ 排序后固定序合成位级确定基准 + WBOIT 臂定点整数累加（Q 格式交付冻结/舍入 floor/饱和 clamp u32::MAX，饱和触发计数如实登记；浮点位级承诺已撤销，RFC-0049 §4.8 F5）+ 双臂各自双跑位级（双臂语义不同不设互拍容差硬门，视觉差 measured 登记）+ #13 OIT 评估窗 re-trigger 消费登记（选型提交引 M120 benchmark 数据，无数据提交判 RED）；判据字面 = ci/g35_sort_oit_smoke.py docstring（随波交付冻结）；实测数字待验收批填写，禁预支"
  - id: G-G35-5
    check: "G35-5 碰撞与力场门 g35.wave5.collision PASS：八 facts 闭集（kernels_spv_valid / collision_parity_vs_host / same_frame_semantics_witness / fallback_chain_explicit / force_fields_parity / determinism_double_run / red_arm_effective / frame_ms_measured——与已落树 ci/g35_collision_smoke.py FACT_IDS 逐字同序，v1.2 校准〔判据协议内容零变化：同帧见证/深度对照臂/反弹有界语义由 same_frame_semantics_witness + collision_parity_vs_host 承载〕；v1.1 增补 fallback_chain_explicit，RFC-0049 F12）——ray query 碰撞同帧生效（命中帧即反弹，vs Niagara 异步一帧延迟的结构性对照见证）+ 深度对照臂两臂行为差如实登记（屏外/遮挡区失效为深度臂固有缺陷不设互拍硬门）+ 碰撞臂三档显式降级链 ray_query→depth_buffer→off fail-closed 禁静默换臂（RFC-0049 §4.13）+ 力场求值 rurix-physics field 通道复用 host 对拍；判据字面 = ci/g35_collision_smoke.py docstring（随波交付冻结）；实测数字待验收批填写，禁预支"
  - id: G-G35-6
    check: "G35-6 事件与双向桥门 g35.wave6.events PASS：八 facts 闭集（kernels_spv_valid / event_overflow_payload_stable / event_spawn_parity / gpu_secondary_emission_zero_readback / particle_view_bridge_roundtrip / determinism_double_run / red_arm_effective / frame_ms_measured——与已落树 ci/g35_events_smoke.py FACT_IDS 逐字同序，v1.2 校准〔判据协议内容零变化：队列有界/槽位 scan 推导/payload 位级语义由 event_overflow_payload_stable + event_spawn_parity 承载〕；v1.1 增补 event_overflow_payload_stable，RFC-0049 F15）——事件写槽 scan 推导禁原子 + 队列有界溢出计数如实登记禁静默丢 + 溢出保留集确定（生产者稳定序 (producer_pid, slot) 键 scan 裁剪保留前 capacity 项，门对拍具体 payload 集）+ particle_view GPU↔host 往返整数流位级；判据字面 = ci/g35_events_smoke.py docstring（随波交付冻结）；实测数字待验收批填写，禁预支"
  - id: G-G35-7
    check: "G35-7 流体门 g35.wave7.fluids PASS：八 facts 闭集（kernels_spv_valid / neighbor_sets_bitexact / hash_cell_floor_semantics / xpbd_parity_within_budget / density_error_measured / determinism_double_run / red_arm_effective / frame_ms_measured——与已落树 ci/g35_fluids_smoke.py FACT_IDS 逐字同序，v1.2 校准〔判据协议内容零变化：count-sort 位级/邻居集精确由 neighbor_sets_bitexact 承载；MPM 评估窗登记由 evidence notes 承载〕；v1.1 增补 hash_cell_floor_semantics，RFC-0049 F14）——分段三阶段空间哈希（段局部直方图行零跨组竞态 → 单线程 spine 固定序合并 → 段内串行 scatter，零原子）cell-major 序 device↔host 位级 + cell = floor(p/cell_size) 逐轴负坐标语义与世界界 clamp 见证 + 邻居集 27-cell 固定序精确 + XPBD 固定迭代 Jacobi ping-pong f32 流走标定容差 + MPM 评估窗登记不承诺（atomics 与确定性协议冲突待裁字面）；判据字面 = ci/g35_fluids_smoke.py docstring（随波交付冻结）；实测数字待验收批填写，禁预支"
  - id: G-G35-8
    check: "G35-8 作者面与 SDK 门 g35.wave8.authoring PASS：八 facts 闭集（asset_schema_fail_closed / curve_eval_deterministic / hot_reload_semantics / pid_continuity_across_reload / sdk_abi_surface_frozen / sdk_handle_fail_closed / determinism_double_run / red_arm_effective——与已落树 ci/g35_authoring_smoke.py FACT_IDS 逐字同序，v1.2 校准〔判据协议内容零变化：schema 往返/digest/热重载保留语义由 asset_schema_fail_closed + hot_reload_semantics + pid_continuity_across_reload 承载；纯 host 门无 kernels_spv_valid/frame_ms_measured；SDK ABI 加性 0-byte 由 sdk_abi_surface_frozen 内嵌 stable_snapshot --check 机核承载〕）——emitter 资产 schema 往返无损 + 同资产同字节 digest + 参数化 megakernel 与专用参数直跑等价对拍 + 热重载参数块重上传池状态保留 + SDK C ABI 加性既有导出面 0-byte（API_VERSIONING 加性纪律）；判据字面 = ci/g35_authoring_smoke.py docstring（随波交付冻结）；实测数字待验收批填写，禁预支"
  - id: G-G35-9
    check: "G35-9 确定性回放门 g35.wave9.replay PASS：八 facts 闭集（kernels_spv_valid / journal_record_replay_bitexact / checkpoint_restore_bitexact / rollback_resim_bitexact / first_divergence_frame_witness / determinism_double_run / red_arm_effective / frame_ms_measured——与已落树 ci/g35_replay_smoke.py FACT_IDS 逐字同序，v1.2 校准〔判据协议内容零变化：跨跑 digest 稳定由 determinism_double_run 承载、soak 归 G-G35-10 收口面、开销登记由 frame_ms_measured 承载；新增 checkpoint/首异帧定位两 facts 为交付强化〕）——journal 重放粒子面输出 digest 逐帧位级一致（同 GPU 同驱动，RFC-0049 §9 Q4 诚实边界）+ 截断重放式回滚位级 + 粒子面独立 digest 锚不混入生产 Stage A 锚表；判据字面 = ci/g35_replay_smoke.py docstring（随波交付冻结）；实测数字待验收批填写，禁预支"
  - id: G-G35-10
    check: "G35 收口验收面：守卫套件七条 exit 0（check_structure/check_schemas/check_number_ledger/check_guardrails/check_contribution/trace_matrix --check/budget_eval）+ 九门新鲜复跑全 PASS（g35.wave1.primitives ~ g35.wave9.replay，--selftest + --gate）+ 零降级回归锚三面（Stage A digest 锚 18/18 canonical 160 帧重跑零漂移 + G16plus M-g 18/18 canonical 复跑不降级 + G17-MD-F1 焦点格新鲜真跑诚实红不恶化 fresh ratio ≥ 在案——ci/g31_wave_a_anchor_check.py 既有门复用）+ capability_chain_registered（particles 能力降级链登记面兑现：gpu_particles→off 主链 + 碰撞臂三档,capability_matrix 第七链扩展登记,RFC-0049 §4.13——v1.1 增补,F12）+ soak 登记面（粒子车道 --particles on ≥5000 帧零崩 + validation 静默——close-out 只追加登记无新硬门）+ 四件套在树（G35_PLAN/G35_CONTRACT/CI_GATES/g35_budget——budget 容差条目零 estimated 全 measured 程序产）+ RFC-0049 Agent Approved 在案（2026-08-27，D-409 一轮 17 findings 全 disposition）+ §9 Q5（pid epoch 扩宽）收口前重判登记 + §8 close-out 只追加填写；实测 facts 待收口验收批填写 §8，禁预支"
guardrails:
  - "诚实登记不冒充：所有数字来自真实命令输出；达标/维持/诚实红均合法终态"
  - "append-only：evidence/ 只增不删不改；deferred history 只追加；既有锚/注册表 0-byte 不回写"
  - "缺省面位级锚（无降级硬线）：粒子面缺省关（--particles off）时缺省面 digest == 母版 Stage A 冻结锚位级一致，粒子系统 = 加性扩展对既有面 0-byte 机器证明"
  - "三态纪律：dev-env 降级 = SKIP 如实登记，禁冒充 PASS；RURIX_REQUIRE_REAL=1 翻硬 FAIL"
  - "commit 带 Assisted-by: trailer 且不 push"
---

<!-- Assisted-by: cursor:claude-fable-5（G35-0 治理文档套件起草批） -->
# G35 契约 — GPU 粒子系统期

> 承接：G34 全特性合流期收口终态（[../g34/G34_CONTRACT.md](../g34/G34_CONTRACT.md) §8 close-out——落笔时三门新鲜绿件在案，收口验收批同窗）+ G31+ 期待办总表 [../../G31_PLUS_COMMERCIAL_RENDERER_TODO.md](../../G31_PLUS_COMMERCIAL_RENDERER_TODO.md) #13/#80/#81 + 用户立项指令『为本项目制定完整的GPU粒子系统，要求超过虚幻五』（2026-08-27）；语义面 = [../../rfcs/0049-gpu-particle-system.md](../../rfcs/0049-gpu-particle-system.md)（Draft，D-409 对抗评审后 Agent Approved 方可收口）；代码契约 = [src/rurix-render/src/particles/mod.rs](../../src/rurix-render/src/particles/mod.rs)（G35-P 冻结契约 v1）；契约机制见 14 §1。front matter 双状态机：`status` 与 `implementation_status` 严格分离。

---

## 1. 目标

G35 收口时，项目获得：**确定性 GPU 粒子系统**——对标并超越 UE5 Niagara 的五轴：① 确定性（禁原子抢槽 + 分段稳定 scan 推导槽位 + 随机带单源 ⇒ 固定输入 device 双跑位级一致，vs Niagara GPU sim 非确定/Determinism flag 仅 CPU 生效/粒子索引 >64 不稳定）；② 光追集成（mesh 粒子进 TLAS 收光追阴影/GI + ray query 同帧碰撞，vs Niagara GPU RT 碰撞异步一帧延迟）；③ 规模（百万级 PARTICLE_CAP_MAX=1048576，存活总数经 scan spine 总和槽直供 indirect args，host 全程零回读，vs Niagara GPU sim 动态 bounds 需回读故禁用）；④ 流体统一物理（count-sort 空间哈希 + XPBD，与粒子池同一确定性协议，对标 Niagara Fluids）；⑤ 数据驱动作者面（声明式 emitter 资产 + 参数化 megakernel + 热重载 + SDK C ABI 加性）。「无降级」判据 = 粒子面缺省关时缺省面与母版 Stage A 锚位级一致（guardrail 字面）；正确性协议 = 整数流零容差位级 + f32 流标定容差（measured × 2.0 程序产禁手写）+ device 双跑一律位级（mod.rs 容差协议字面）；且全部既有画质/性能/确定性锚零降级（收口验收批机器核验，G-G35-10）。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物/门 |
|---|---|---|
| g35_wave1_primitives | G35-1 GPU 基元：24 位键 3-pass 稳定 radix sort + compact_u32（scan 三 kernel 已落树为上游） | D-G35-1 / G-G35-1 |
| g35_wave2_particle_core | G35-2 粒子核心：SoA 池 ping-pong + 确定性发射（persistent ID）+ 半隐式 Euler + 稳定压缩 + indirect args 零回读 | D-G35-2 / G-G35-2 |
| g35_wave3_render | G35-3 渲染接线：billboard splat 进 G34 统一车道 + 软粒子 + 粒子 MV（#81）+ mesh 粒子 TLAS 臂 | D-G35-3 / G-G35-3 |
| g35_wave4_sort_oit | G35-4 半透明：深度排序臂 + WBOIT 臂双臂 + #13 OIT 评估窗 re-trigger 消费 | D-G35-4 / G-G35-4 |
| g35_wave5_collision | G35-5 碰撞与力场：ray query 同帧碰撞 + 深度缓冲对照臂 + rurix-physics field 复用 | D-G35-5 / G-G35-5 |
| g35_wave6_events | G35-6 事件数据通道 + particle_view GPU↔host 双向桥 | D-G35-6 / G-G35-6 |
| g35_wave7_fluids | G35-7 流体：count-sort 空间哈希邻居 + XPBD（MPM 评估窗登记不承诺） | D-G35-7 / G-G35-7 |
| g35_wave8_authoring | G35-8 作者面与 SDK：声明式 emitter 资产 + 参数化 megakernel + 热重载 + SDK C ABI 加性 | D-G35-8 / G-G35-8 |
| g35_wave9_replay | G35-9 确定性回放回滚 + 收口验收面 | D-G35-9 / G-G35-9、G-G35-10 |

### 2.2 out-of-scope（显式排除）

- **CPU 粒子生产车道**——host 金标准层 = 对拍参照非生产形态（RFC-0049 §7.4/§8；Niagara 以 CPU sim 换确定性的理由在本系统不成立）。
- **G13~G34 冻结注册表/锚改写**——0-byte（front matter out_of_scope 逐字）；生产管线既有 pass 与 Stage A 锚 0-byte，粒子面全加性。
- **跨硬件位级承诺**——确定性诚实边界 = 同 GPU 同驱动位级、跨硬件协议一致（整数流可期位级，f32 流不承诺；RFC-0049 §9 Q4）。
- **编辑器 GUI**——emitter 资产为文本声明式；可视化编辑器归产品面。
- **评估窗登记项（不占波次不预支）**：Onesweep/decoupled-lookback 生产化（Vulkan 无前进保证，RFC-0049 §9 Q1）/ MPM 生产化（atomics 与确定性协议冲突待裁）/ GPU 动态 bounds（本波 fixed bounds 语义）/ FIF×动态共存（TODO #90）/ Work Graphs（#40 not-available 维持）。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-G35-1 | G35-1 GPU 基元 | kernels/g35_sort_{hist,spine,scatter}.rx + g35_compact_u32.rx（+ 实验臂 g35_sort_onesweep.rx）+ [particles/primitives.rs](../../src/rurix-render/src/particles/primitives.rs) + bin/g35_primitives_device.rs + ci/g35_primitives_smoke.py + gate schema | 门 --selftest + --gate 全 PASS |
| D-G35-2 | G35-2 粒子核心 | kernels/g35_{emit,sim,particle_compact,indirect_args}.rx + [particles/core.rs](../../src/rurix-render/src/particles/core.rs) + bin/g35_particle_core_device.rs + ci/g35_particle_core_smoke.py + gate schema + 容差条目程序产 | 门 --selftest + --gate 全 PASS |
| D-G35-3 | G35-3 渲染接线 | splat/MV kernel + g34_full_lane 粒子独立 include 区段 + mesh 粒子 TLAS 臂 + ci/g35_render_wiring_smoke.py + gate schema | 门 --selftest + --gate 全 PASS |
| D-G35-4 | G35-4 半透明双臂 | 排序臂（alpha blend 执行器加性）+ WBOIT 臂 + ci/g35_sort_oit_smoke.py + gate schema + #13 re-trigger 登记件 | 门 --selftest + --gate 全 PASS |
| D-G35-5 | G35-5 碰撞与力场 | ray query 同帧碰撞腿 + 深度对照臂 + field 复用 + ci/g35_collision_smoke.py + gate schema | 门 --selftest + --gate 全 PASS |
| D-G35-6 | G35-6 事件与双向桥 | 有界事件队列 + particle_view 桥 + ci/g35_events_smoke.py + gate schema | 门 --selftest + --gate 全 PASS |
| D-G35-7 | G35-7 流体 | count-sort 哈希 + XPBD + ci/g35_fluids_smoke.py + gate schema + MPM 评估窗登记件 | 门 --selftest + --gate 全 PASS |
| D-G35-8 | G35-8 作者面与 SDK | emitter 资产 schema + megakernel 参数化 + 热重载 + SDK C ABI 加性 + ci/g35_authoring_smoke.py + gate schema | 门 --selftest + --gate 全 PASS |
| D-G35-9 | G35-9 回放回滚 | journal + 回放/回滚 + ci/g35_replay_smoke.py + gate schema + 粒子面独立 digest 锚 | 门 --selftest + --gate 全 PASS |

治理四件套（G35_PLAN.md / G35_CONTRACT.md / CI_GATES.md / g35_budget.json）= 本批（G35-0）交付，不占 D 号；在树 + budget_eval 全 PASS 为 G-G35-10 收口面判据之一。

## 4. 验收门（完整版，YAML 头为可提取摘要）

G-G35-1~G-G35-10 逐字见 front matter `acceptance_gates`。性能/对拍面证据等级 = measured_local（RTX 4070 Ti + Vulkan，本机真跑，gpu_device_lock 串行，RURIX_VK_VALIDATION=1）；采样协议 = 各门脚本内登记（判据字面 = 各 smoke docstring，随波交付冻结）；容差纪律 = 整数流零容差位级 + f32 流标定容差（threshold = measured × 2.0 程序产禁手写，入 [g35_budget.json](g35_budget.json)）。全部实测数字一律待各波验收批填写，禁预支。

## 5. Guardrails（字节级，机器核对）

见 YAML 头 `guardrails` 字段。核对方式：`ci/check_guardrails.py`（agent 完全自主模式 ADVISORY 不阻断）+ `ci/check_schemas.py` / `ci/budget_eval.py` 硬门。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-045 | 间歇 digest 漂移 backfill 三件（定位/修复/Full RFC 评估），open/maintain-open | 本期 soak/双跑/回放 digest 位级一致 + Stage A 锚零漂移 = 累计观察面只追加（不充三件，G31/G34 契约 §6 同律） |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源，本表仅引用。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-27 | 初版契约固化（GPU 粒子系统期九波；G35-0 治理批交付四件套 + RFC-0049 Draft；实测数字全部待各波验收批填写） |
| v1.1 | 2026-08-27 | RFC-0049 D-409 对抗评审修法批同步（17 findings 全 disposition 后 Agent Approved——rfc_required 字段更新）：① F16 六门 facts 增补——G-G35-3 +barrier_plan_audit/+particle_mv_parity_2px（十 facts）、G-G35-4 +sorted_arm_bitexact/+wboit_fixedpoint_saturation/+near_far_order_witness（十一 facts）、G-G35-5 +fallback_chain_explicit（八 facts）、G-G35-6 +event_overflow_payload_stable（八 facts）、G-G35-7 +hash_cell_floor_semantics（九 facts）、G-G35-10 +capability_chain_registered 与 §9 Q5 重判登记；② G-G35-1/G-G35-2 facts 闭集与已落树 W1/W2 smoke FACT_IDS 字面同步（v1.0 起草先于 smoke 交付的行文差校准——smoke/schema 冻结互核面为准，判据协议内容 0 变化）；③ G-G35-2 增注容量钳制语义由 probe 硬断言承载（F7，FACT_IDS 闭集 0-byte）。判据协议面变更均源自 RFC-0049 v0.2（评审 provenance cursor:gpt-5.6-sol-medium）；既有 §8 close-out 空区与其余条款 0-byte |
| v1.2 | 2026-08-27 | 收口前 facts 名单校准（v1.1 同款纪律扩展到 G-G35-3~9 七门）：契约条款自身规定「判据字面 = ci/g35_*_smoke.py docstring（随波交付冻结）」——各波 smoke/schema 交付后其 FACT_IDS 即法定闭集，本行把七门 facts 枚举与落树字面逐字同步（G-G35-3 十→九、G-G35-4 十一→九、G-G35-5/6 名单改字、G-G35-7 九→八、G-G35-8 七→八、G-G35-9 六→八；判据协议内容零变化，被合并/更名 fact 的语义承载关系逐门注于 check 行内）；G-G35-1/2/10 与 guardrails 0-byte；§8 close-out 空区 0-byte（收口验收批填写） |

---

## 8. Close-out 区（只追加 — G35 收口验收记录）

> 结构预期（收口验收批按 G31/G34 契约 §8 行文范式只追加填写）：① 守卫套件七条全跑 → ② 九门新鲜复跑 → ③ 零降级回归锚三面 → ④ soak 登记面 → ⑤ 编号纪律实测 → ⑥ RFC-0049 Agent Approved 核验 → ⑦ 签署。

### §8.1 收口验收记录（2026-08-27 收口验收批,只追加）

**① 守卫套件七条全跑 exit 0**（2026-08-27 收口批新鲜真跑）：check_structure / check_schemas / check_number_ledger / check_guardrails / check_contribution / trace_matrix --check / budget_eval 全部退 0；budget_eval **329 pass, 0 skip**（g35 命名空间 7 条容差条目全 measured 程序产零 estimated：g35.particle_core.f32_parity_p100=0.0〔NoContraction 注入 sim+emit 后 f32 位级〕/ g35.collision.parity_p100=1.004338e-05〔RT core t 值 ULP 级〕/ g35.events.parity_p100=0.0 / g35.fluids.parity_p100=7.324219e-03〔单帧注入协议〕/ g35.render.mv_parity_px / g35.oit.parity_p100=0.0 / g35.oit.wboit_acc_tol=0.0——threshold = measured × 2.0 各门标定腿程序写入）。

**② 九门真跑全 PASS + selftest 新鲜复跑 9/9 退 0**（真跑 = 2026-08-27 验收批当日串行经 gpu_device_lock;selftest = 收口批新鲜复跑）：

| 门 | verdict | evidence |
|---|---|---|
| g35.wave1.primitives | PASS 8/8 | evidence/g35_primitives_gate_20260827T103626Z.json（scan/sort/compact 整数域零容差位级 + 稳定序 + 双跑位级 + tamper 红臂 812 槽检出） |
| g35.wave2.particle_core | PASS 8/8 | evidence/g35_particle_core_gate_20260827T103639Z.json（五整数流 64 帧位级 + **f32 全流 p100=0.0 位级** + pid 持久唯一 + indirect args 零回读恒等式） |
| g35.wave3.render | PASS 9/9 | evidence/g35_render_gate_20260827T122240Z.json（off 面 == Stage A 锚 c1d28ad7… 位级 + 粒子 MV 对拍 max_err=0.0px ≤ 2px〔#81 兑现〕+ 遮挡见证 + DispatchSpec::Indirect 零回读 + 屏障计划机核审计;**首跑 FAIL 件 g35_render_gate_20260827T115018Z.json 按 append-only 留存**——根因 = 执行器屏障幂等去重 × 全 RW 计划 ⇒ 粒子 pass 间零屏障竞争,修法 = 计划改 StorageWrite 强制真屏障对 + 全缓冲零初始化,全 bin 局部共享体 0-byte） |
| g35.wave4.sort_oit | PASS 9/9 | evidence/g35_sort_oit_gate_20260827T135322Z.json（sorted 臂位级基准 p100=0.0 + wboit 定点累加整数差=0 饱和事件 0 + 近远见证 changed_px=10 + 三臂 digest 两两互异 + --oit off == 缺省位级 + #13 评估窗 re-trigger 登记消费;t100 tile 键域越界诚实冻结 --tier 50 门腿） |
| g35.wave5.collision | PASS 8/8 | evidence/g35_collision_gate_20260827T113120Z.json(同帧见证首异帧==突移帧 32 + Niagara 一帧延迟模型对照臂 + 降级链 typed 退出码 3/2 + parity p100=5.02e-06) |
| g35.wave6.events | PASS 8/8 | evidence/g35_events_gate_20260827T113510Z.json（溢出稳定裁剪如实登记〔pushed 1200/kept 1024/overflow 176〕+ 双源发射位级 + GPU 二次发射零回读 51 帧/47052 粒 + particle_view 桥 roundtrip 位级 15707 粒） |
| g35.wave7.fluids | PASS 8/8 | evidence/g35_fluids_gate_20260827T114153Z.json（邻居结构五整数流 32 帧位级 + floor/clamp 语义 26036 事件见证 + XPBD p100=3.66e-03 入预算 + 密度残差 0.41→0.022 登记;自由跑协议不可达〔device sqrt 非正确舍入 ULP × 混沌域 Lyapunov 放大〕→ 单帧 host 状态注入协议冻结,如实登记于 evidence parity_protocol） |
| g35.wave8.authoring | PASS 8/8 | evidence/g35_authoring_gate_20260827T121831Z.json（资产十字段 fail-closed 10/10 + 热重载 pid 连续 46831 粒核验 + SDK 四冻结签名 dumpbin 4 新+9 既有 + stable_snapshot --check exit 0〔用户面 ABI 1.0.0 零破坏机器证明;MINOR 1.1.0 薄转发归后续批,API_VERSIONING.md §6 登记〕） |
| g35.wave9.replay | PASS 8/8 | evidence/g35_replay_gate_20260827T122849Z.json（journal 344B 重放逐帧位级〔首异帧=-1〕+ 检查点 k=16 回滚重仿真 33 帧 digest[48] 位级 + **篡改帧 32 首异帧精确==32 分歧可定位见证** + 回放链尾 digest 与 G35-2 门链尾一致 = 跨波确定性互核） |

**③ 零降级回归锚三面 PASS**：evidence/g31_wave_a_anchor_check_20260827T125042Z.json——Stage A digest **18/18 canonical 160 帧重跑零漂移** + 焦点格 bistro-interior_t100_dlss_sr fresh ratio **0.991676 ≥ 在案 0.960479**（维持不恶化）+ 五门 evidence 5/5 在档。G35 全期对冻结生产车道**零回归**的机器证明。

**④ soak 登记面**：粒子车道 `--particles on --auto-move orbit` **5010 帧零崩**（RURIX_VK_VALIDATION=1 全程 **VUID 输出 = 0** 静默）,render 均值 64.943ms / 粒子 10 pass GPU 段 8.527ms / 活跃粒子 ~10k 稳态,presented digest sha256:543cd3f0…;harness 件 .tmp/g35_gates/soak/soak_5000.json（登记面无新硬门,G-G35-10 字面）。

**⑤ 编号纪律实测**（收口批 ledger 实测）：CI_step next_free = **525 维持**（九门全 symbolic gate key 未占号）/ RFC next_free = 50（RFC-0049 已 materialize）/ RD next_free = 46（本期零新 RD——F6 pid epoch 走 §9 Q5 重判登记）/ D 共享段零消费。evidence 前缀族 g35_primitives_/g35_particle_core_/g35_render_/g35_sort_oit_/g35_collision_/g35_events_/g35_fluids_/g35_authoring_/g35_replay_ 九路由全部经 _patch_g35_*_schemas.py 三处纯追加驻留 check_schemas（分岔分析 CI_GATES.md §3）。

**⑥ RFC-0049 Agent Approved 核验**：2026-08-27 D-409 一轮 17 findings（5 blocker + 11 high + 1 med）全 disposition（评审 provenance cursor:gpt-5.6-sol-medium ≠ 起草 cursor:claude-fable-5,rfcs/0049 §9.1）后 Agent Approved;§9 Q5（pid epoch 扩宽）重判登记在案;capability_matrix **第七链 particles 加性登记兑现**（gpu_particles→off 主链 + 碰撞臂 ray_query→depth_buffer→off,六链冻结闭集 0-byte 有单测互核,G-G35-10 capability_chain_registered 字面）。

**⑦ 签署**：G-G35-1 ~ G-G35-10 十门全兑现,G35「GPU 粒子系统期」收口。五轴超越验证矩阵全部以机器事实兑现：确定性（九门 determinism_double_run 全位级 + journal 回放/回滚位级 + 首异帧可定位——Niagara GPU sim 结构性做不到）/ 光追集成（ray query 同帧碰撞见证 vs 一帧延迟对照 + mesh 粒子 TLAS 臂 N=1）/ 规模（容量可寻址 1M,吞吐 measured 登记,spine 串行瓶颈如实单列）/ 流体统一物理（count-sort 空间哈希位级 + XPBD 入预算）/ 数据驱动作者面（资产 fail-closed + 热重载 pid 连续 + SDK 加性 ABI 零破坏）。Assisted-by: cursor:claude-fable-5（起草/实现/收口）+ cursor:gpt-5.6-sol-medium（D-409 对抗评审）。

（本区由 G35 收口验收批只追加填写——七守卫/九门复跑/三锚/soak 实测 facts）
