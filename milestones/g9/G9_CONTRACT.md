---
contract: G9
title: G9 UE5 级渲染器与物理引擎正式建造期
status: active
implementation_status: unblocked
active_scope: g9_1_governance_only + g9_2_plus_implementation_waves
version: v1.0
date: 2026-08-09
timebox: "G9.1 治理波即刻执行（G8 已 closed）；G9.2~G9.8b 严格波次，工期在实现互锁开放后由 measured baseline 校准"
rfc_required: "RFC-0022 虚拟几何与 GI 语义；RFC-0023 GPU-driven 提交与着色系统；RFC-0024 物理平台修订（RFC-0021 修订）（三份 Full RFC，D-409 独立 provenance 对抗性评审后 Approved；编号按 2026-08-09 实测 number_ledger next_free=22 领取）——RFC-0022/0023/0024（须 D-409 评审后 Agent Approved 方为语义冻结；未 Approved 前本契约对应条款为引用占位，不构成语义冻结）"
upstream_docs:
  - "milestones/g9/G9_PLAN.md v1.1（治理门/实现门解耦、五轨道波次与退出门判据草案的契约上游事实源）"
  - "milestones/g9/G9_CAPABILITY_MATRIX.md（M90~M127 能力缺口矩阵、15 个 P0 建议值与十条成功判据草案）"
  - "milestones/g8/G8_CONTRACT.md §8.26（G8 closed 终态，2026-08-06，flip commit b4189e79）"
  - "milestones/g8/G8_P2_DECISIONS.md（十条 defer-to-G9+ 承接锚，法定输入）"
  - "registry/deferred.json RD-039~044（条件型 RD 字面与 history；只追加禁静默改判）"
  - "registry/number_ledger.json reserved_in_flight[G9]（G9 RFC claim 与共享编号隔离；数字 CI 延迟分配）"
  - "04 P-01/P-04/P-07/P-09/P-12/P-13；10 §3/§7/§9.5；14 §1/§3/§4/§5（同 G8 口径）"
implementation_unlock:
  required_all:
    - "G9.1 治理门全部完成且有真实验证记录"
    - "check_g9_implementation_interlock --require-ready 输出 READY（互锁 validator 机器事实，不以叙述替代）"
    - "共享编号按互锁开放时 actual next_free 重新校准；数字 CI 步骤不得沿用推测号与 design/ 草案建议值"
in_scope:
  - g9_1_governance_only
  - rfc_0022_virtual_geometry_and_gi_semantics
  - rfc_0023_gpu_driven_submission_and_shading
  - rfc_0024_physics_platform_revision
  - candidate_decisions_and_rd_mapping
  - p0_acceptance_mapping
  - measured_4070ti_baseline
  - g9_2_foundation_wave
  - g9_3_geometry_rt_convergence_wave
  - g9_4_gi_wave
  - g9_5_world_and_specialty_wave
  - g9_6_physics_wave
  - g9_7_p2_exhaustive_decisions
  - g9_8_stabilization_and_closeout
out_of_scope:
  - g9_2_plus_while_implementation_interlock_is_red
  - g9_1_src_spec_conformance_semantic_implementation
  - g9_1_numbered_workflow_steps_or_stub_scripts
  - dmm_displacement_micromap_permanently_forbidden
  - work_graphs_async_compute_second_leg_task_shader_open
  - gpu_rigid_body_mainline_differentiable_physics_true_bidirectional_fluid_coupling_generic_softbody_mpm_flip
  - nrc_neural_radiance_cache_fg_mfg_cooperative_vector
  - rewriting_g5_to_g8_closed_contracts_and_00_14
  - unmeasured_performance_hard_gates
  - speculative_number_consumption
  - safe_gpu_operator_platform_deferred_to_g10_plus
  - editor_gui_audio_network_engine_multi_gpu_webgpu
deferred_refs: [RD-034, RD-039, RD-040, RD-041, RD-042, RD-043, RD-044]
deliverables:
  - id: D-G9-1
    name: "G9.1 治理四件套：G9_PLAN v1.1（升格契约上游事实源）、G9_CONTRACT、CI_GATES、非空 measured g9_budget；status=active 且 implementation_status=blocked"
  - id: D-G9-2
    name: "G9.1 完整候选决策表：G8 十锚 + 全部追加输入 + 存续 open RD 分项逐行映射 M##/backfill/证据/go|no-go|strategic_override|defer/波次/承接锚/最终状态；含 M52→M108、M61→M109 两条 strategic_override 登记；缺行阻断 G9.2"
  - id: D-G9-3
    name: "G9.1 验收映射：15 个 P0 各有独立 symbolic gate key、稳定脚本名、evidence schema 目标路径与判据；已 go 的 P1 同步覆盖"
  - id: D-G9-4
    name: "RFC-0022/0023/0024 三份 Full RFC 经 D-409 独立 provenance 对抗性评审后 Approved；0022 含 cluster DAG/页格式 v2/CLAS/Surface Cache/probe 编码/材质时域，0023 含 DGC/Execution Set/descriptor 全局索引/SER 原语/mesh shader 可选路径与 G5 Barrier EB 冻结面修订行，0024 含 Field 系统/双通道 tick/浮力/Jolt 5.6 升级路径/神经变形研究轨边界"
  - id: D-G9-5
    name: "G9.1 RTX 4070 Ti measured baseline 与非空 g9_budget（零 estimated）；G9 validator 五件套落盘——implementation interlock 当前诚实报告 BLOCKED，acceptance map 三向比对/budget baseline/决策表承接锚机核在当前治理事实下诚实报告 PASS"
  - id: D-G9-6
    name: "G9.2 地基波：cluster DAG 深化与页格式 v2 ABI 冻结、descriptor 全局表/DGC 抽象/AccessKind 新边、统一 particle view 与 Field 骨架"
  - id: D-G9-7
    name: "G9.3 几何×RT 合流波：GPU 蒙皮/LOD/VisibleClusterSet/CLAS 当帧拼装与回退腿/单源真相集成 + command build node/Execution Set/shader library IR 链接/变体预算"
  - id: D-G9-8
    name: "G9.4 GI 波：M96 M17 参照器先行（golden 前置）→ M97 Surface Cache → M98 追踪降级链 → SPG/Radiance Cache → IF 档位 → 多灯"
  - id: D-G9-9
    name: "G9.5 大世界×专项波：分区骨架/OIT benchmark → 大气/地形/贴花 → 云/水体/皮肤/HDR → AVBOIT/毛发；M110/M118 两个 P0 在本波"
  - id: D-G9-10
    name: "G9.6 物理波：Field 完整语义/浮力/双通道 tick 判档/Jolt 5.6 A/B/Rapier 基准；M121/M122 完整语义收尾"
  - id: D-G9-11
    name: "G9.7 P2 穷举决策 + G9.8a soak + G9.8b close-out"
acceptance_gates:
  - id: G-G9-1
    check: "治理激活门：用户 2026-08-09 立项指令（G9.1 治理包七项交付清单）留痕；agent 依 10 §7/P-13/D-406 v2.0 完全自主签署立项裁决留痕；六项立项裁决全部落定；G9.0 不可变 ref=1d9460a1 登记；仅 governance-only 范围 active"
  - id: G-G9-2
    check: "G9.1 完成门：D-G9-1~5 齐备并通过结构/schema/ledger/guardrail/预算核验；15 个 P0 映射无缺行；无 src/spec/conformance 语义实现、无数字 workflow 空步骤；本门通过不自动开放实现"
  - id: G-G9-3
    check: "实现互锁门：check_g9_implementation_interlock --require-ready 输出 READY + 用户 G9.2 开工指令留痕 + 共享编号按 actual next_free 重新校准。任一条件不满足均保持 implementation_status=blocked"
  - id: G-G9-4
    check: "G9.2 退出门：M90/M91/M102/M103/M104/M121/M122 七个 P0 独立断言全绿（M121/M122 为骨架段）；页格式 v2 编解码 golden + 篡改 digest 页被拒；DGC/DgcBuffer 类型层无 host 读接口断言 + 装配期限制核验 RED 臂；M68 journal 迁移前后 digest 一致"
  - id: G-G9-5
    check: "G9.3 退出门：M93/M94/M95 三个 P0 独立断言全绿；CLAS 腿与回退腿 ray query 逐命中一致；静态帧零 AS 构建（非零即 RED）；单源真相负例 RED 臂有效；M108/M109 仅按立项裁决 go 后在硬约束顺序（meshlet 页格式 v2 与 GPU-driven 剔除之后）内排波"
  - id: G-G9-6
    check: "G9.4 退出门：M96/M97/M98 三个 P0 独立断言全绿；M96 M17 golden 门未绿则 M97~M101 任何画质门不得验收（门序硬约束）；漏光负例 RED 臂独立有效；验证射线零跳过统计性偏置门"
  - id: G-G9-7
    check: "G9.5 退出门：M110/M118 两个 P0 独立断言全绿；大世界 soak hitch p99 ≤ measured 阈值 + 预算违约注入必降级不静默超帧；HLOD 双构建 hash 相等 + 运行时零合并断言；OIT 默认档选型必须引 benchmark 数据；HDR 设备标定未触发 SKIP=not-triggered 不充绿"
  - id: G-G9-8
    check: "G9.6 退出门：M121/M122 完整语义独立断言全绿；persistent field 注册/注销/变更全 journal 化且 replay 逐 tick hash 一致；浮力旁路 API 注入即 RED（必须走 Field 通道）；双通道未经 Jolt 单线程成本 measured 判档则登记 no-go 不充绿；Jolt A/B 两臂（采纳三件事/失败钉 5.3）诚实登记"
  - id: G-G9-9
    check: "G9.7 决策门：全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G10+，零空行；defer 必有承接锚（机核同构 ci/g8_p2_decisions_check.py）；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述"
  - id: G-G9-10
    check: "G9.8a 稳定门：15 个 P0 与所有 go 的 P1 全量回归；G5~G8 既有判据 0-byte；soak 不低于 30 分钟且 10000 帧（≥G7 量级）；strict budget 非空、零 estimated/skip；继承 G8.8b 同日放行先例——8a full-run 先行完成后允许同日进入 8b close-out"
  - id: G-G9-11
    check: "G9.8b 收口门：验收映射、候选决策、RD 最终状态逐字一致；15 个 P0 独立断言均 PASS；evidence/schema/预算终审；§8 只追加后 status active→closed"
guardrails:
  - "双状态不可混同：status=active 仅表示 G9.1 governance-only 已立项；在 G-G9-3 真实通过前 implementation_status=blocked，任何治理完成叙述不得冒充 G9.2 开工"
  - "G9.1 允许 milestones/g9、RFC-0022~0024、G9 专属 claim、deferred history 只追加、未编号 validator 与 measured baseline；src/spec/conformance 和编号 workflow 步骤 0-byte"
  - "G9 CI 只冻结 symbolic gate key 与脚本名；numeric_step 一律写 post-interlock actual-next-free allocation。不得沿用推测号与 design/ 草案建议值（D3 草案 §⑨ 建议区间与 M50 已消费段冲突先例），不得预放空 workflow、空脚本或空 schema 壳"
  - "15 个 P0 必须 15 个独立布尔断言与独立 evidence subject；可共享一次进程执行，但聚合 PASS 不能遮蔽任一子断言 FAIL/SKIP"
  - "缺硬件/工具链仅可 dev_env_degrade 或 SKIP=not-triggered；两者均不充 P0 绿。host oracle、mock、isolated nonzero、既有最小见证均不能替代目标 device 门"
  - "条件型 RD（RD-039/040/041/044）逐分项 go/no-go/strategic_override；证据与 override 均只追加。未触发项维持 open，不得通过删除验收行获得全绿；任何分项的触发条件不得被「UE5 目标」静默改写 backfill 字面"
  - "触 G5/G6 冻结面（AccessKind 新边 M104、MaterialClosure 32B 扩展 M115/M114、World-Field buffer M122）必须 RFC 显式修订行，禁静默扩；G5~G8 closed 契约与 00-14 0-byte，close-out 证据只追加"
  - "M91 页格式 v2（RXPL 新 major）必须在 G9.2 spec-first 冻结；M04 v1 ABI 0-byte 共存，下游波次只消费不重定"
  - "g9_budget 首个实现 PR 前必须非空 measured_local 且有 evaluator；全程零 estimated；性能数字不替代 correctness gate；D4 阈值全部实测标定禁手写"
  - "新 unsafe 仅在实现互锁开放后按 actual next_free 登记并附 SAFETY；rurix-render 维持 forbid(unsafe_code)"
  - "G9.8a 为 G9.8b 前置硬门；同日放行仅按继承先例字面（8a full-run 先行完成后允许同日进 8b），不得扩展解释为跳过 soak"
  - "新文件 LF + 尾换行；本契约合入后正文冻结，激活/验收/收口只追加 §8，除最终 status flip 外不回写既有事实"
---

# G9 契约 — UE5 级渲染器与物理引擎正式建造期

> 计划：[G9_PLAN.md](G9_PLAN.md) v1.1 · 能力事实源：[G9_CAPABILITY_MATRIX.md](G9_CAPABILITY_MATRIX.md) · 机器门：[CI_GATES.md](CI_GATES.md)。
> 当前裁决：**G9.1 governance-only active；G9.2~G9.8b implementation blocked**。`active` 不是实现门绿灯。

---

## 1. 目标与双门状态

G9 是 UE5 级渲染器与物理引擎的**正式建造期**：在 G8 冻结底座上建造五模块——D1 虚拟化几何×RT 合流、D2 全局光照、D3 GPU-driven 提交与着色系统、D4 大世界×专项渲染器×显示管线、D5 物理。「UE5 级」可核对基线沿用 G8 口径 = UE 5.8；验收五层级沿用 G8：核心等价、功能闭环、可降级、可生产化、Vulkan 主线。G9 不用“目标宏大”替代可验证事实：全部 15 个 P0 必须独立过门，条件型能力按逐分项决策执行，未触发项保持 open。

本契约拆分两种状态：

| 状态 | 当前值 | 含义 |
|---|---|---|
| `status` | `active` | G9.1 治理波已获授权，可落治理资产、三份 RFC、候选决策/验收映射、G9 专属 claim、互锁 validator、RTX 4070 Ti measured baseline 与非空 budget |
| `implementation_status` | `blocked` | G9.2+ 尚未获准；当前不得改 `src/`、`spec/`、`conformance/`，不得 materialize 数字 CI 步骤 |

G-G9-3 是唯一实现入口：互锁 validator（`check_g9_implementation_interlock --require-ready`）输出 READY + 用户 G9.2 开工指令留痕 + 共享编号按 actual `next_free` 重新校准，三者齐备方可解锁；任一缺失均保持 `blocked`。

## 2. 范围与严格波次

### 2.1 G9.1 governance-only

G9.1 只做 D-G9-1~5。允许治理文档、RFC-0022/0023/0024（须 D-409 评审后 Agent Approved 方为语义冻结；未 Approved 前本契约对应条款为引用占位，不构成语义冻结）、候选决策表、验收映射、G9 专属无冲突 claim、互锁 validator、RTX 4070 Ti baseline 与非空 budget；禁止语义实现和编号 workflow。interlock validator 在当前事实下应明确返回 `BLOCKED`，这正是正确结果，不是失败需要被绕开。

### 2.2 G9.2~G9.8b implementation

实现互锁开放后按以下顺序推进，波次内可蜂群并行，波次间不得越级；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑：

```text
G9.2 地基波（D1 cluster 数据格式/页格式 v2 冻结 + D3 descriptor 全局表/DGC 抽象 + D5 Field 骨架与统一 particle view）
  → G9.3 几何×RT 合流波（D1 GPU 蒙皮/LOD/CLAS + D3 command build node/Execution Set）
  → G9.4 GI 波（M17 参照器先行（golden 前置）→ Surface Cache → SPG/Radiance Cache → IF 档位 → 多灯）
  → G9.5 大世界×专项波（分区骨架/OIT benchmark → 大气/地形/贴花 → 云/水体/皮肤/HDR → AVBOIT/毛发）
  → G9.6 物理波（Field 完整语义/浮力/双通道 tick/Jolt 5.6 A/B/Rapier 基准）
  → G9.7 P2 穷举决策 → G9.8a stabilization/soak → G9.8b close-out
```

每波退出门见 YAML `acceptance_gates`（G-G9-4~8，判据按 G9_PLAN §2 各波退出门草案硬化）；任一上游门未绿，下游 evidence 即使局部成功也不能宣称波次完成。

## 3. G9.1 交付冻结

| ID | 交付 | 退出判据 |
|---|---|---|
| D-G9-1 | 契约四件套与双状态 | PLAN v1.1、CONTRACT、CI_GATES、非空 measured budget 一致；`status=active`、`implementation_status=blocked` |
| D-G9-2 | 候选决策与 RD 总映射 | 十锚 + 追加输入 + 存续 open RD 分项每个字面分项一行；原 backfill、证据、裁决、波次、退出门、承接锚、最终状态无空项；M52→M108/M61→M109 两条 strategic_override 如实登记；defer 出 G9 的分项必带承接锚 |
| D-G9-3 | 验收映射 | 15 个 P0 全部有独立 key/script/schema 目标路径/check；go 的 P1 同步入表；不存在“由邻项代绿”；缺行阻断 G9.2 |
| D-G9-4 | RFC-0022/0023/0024 | 三份 Full RFC 均经 D-409 独立 provenance 评审后 Approved（未 Approved 前本契约对应条款为引用占位，不构成语义冻结）；编号登记与 README/ledger 一致 |
| D-G9-5 | baseline、budget、互锁 validator | RTX 4070 Ti measured 数据非空、零 estimated；interlock validator 对当前状态诚实报 BLOCKED；无空 workflow、无空 schema 壳 |

G9.1 完成仅关闭治理准备，不改变 G-G9-3 的机器事实。

## 4. 验收门与 15 个 P0 独立断言

### 4.1 波次验收门

G-G9-1~11 以 YAML 头为可提取摘要。[CI_GATES.md](CI_GATES.md) 冻结脚本与 evidence 形态。条件型分项的 `SKIP=not-triggered` 只表示决策已记录，不是成功；设备门的 `dev_env_degrade` 只表示环境缺失，也不是成功。

### 4.2 P0 独立断言

以下 15 行是 close-out 不可合并、不可删减的独立布尔断言（key 命名空间三方逐字一致，冻结）。一次 smoke 可以共享启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。evidence schema 目标路径统一为 `milestones/g9/g9_m<##>_<slug>_evidence_schema.json`——本契约只冻结路径，不预建文件。硬判据由 G9_PLAN §2.9 与矩阵 §6.4 十条判据草案展开为可机器求值形式，负例 RED 臂要求逐行写明。

| Symbolic gate key | M## | 最晚波次 | 稳定脚本名 | 独立硬判据 |
|---|---:|---|---|---|
| `g9.p0.m90.cluster_dag_deepening` | M90 | G9.2 | `ci/g9_cluster_dag_deepening_smoke.py` | DAG 误差度量 monotonic 逐边机器核验成立；同输入双构建产物字节一致；破坏单调性 fixture 为独立 RED 臂必须被拒 |
| `g9.p0.m91.page_format_v2_abi` | M91 | G9.2 | `ci/g9_page_format_v2_abi_smoke.py` | 页格式 v2（RXPL 新 major）编解码往返无损 golden；M04 v1 ABI 0-byte 共存；篡改 digest 的页 fail-closed 被拒（RED 臂）；版本变化显式迁移而非静默重解释 |
| `g9.p0.m102.dgc_abstraction` | M102 | G9.2 | `ci/g9_dgc_abstraction_smoke.py` | DGC token 跨 API 最小公倍数限制装配期 fail-closed；layout 违规声明被拒（RED 臂）；DgcBuffer 类型层无 host 读接口结构性断言；目标硬件 capability snapshot 实测确认为阻塞性前置 |
| `g9.p0.m103.descriptor_global_table` | M103 | G9.2 | `ci/g9_descriptor_global_table_smoke.py` | 全局 descriptor 索引与 shader 实际索引双向精确相等（reflection/manifest 可核）；≥65536 条目真机出图正确；索引分配律/回收进 spec |
| `g9.p0.m104.accesskind_indirect_edge` | M104 | G9.2 | `ci/g9_accesskind_indirect_edge_smoke.py` | 新 AccessKind 边（`StorageWrite→IndirectCommandRead`）barrier 推导 golden；漏声明 indirect 读边装配期 strict 拒（RED 臂）；G5 Barrier EB 三轴冻结面修订以 RFC-0023 显式修订行为前置 |
| `g9.p0.m121.physics_particle_view` | M121 | G9.2 + G9.6 | `ci/g9_physics_particle_view_smoke.py` | 五域 `ParticleAdapter` 全实现；写路径仅 impulse/force 结构性断言；M68 damage journal 迁移为首个 consumer 且迁移前后 digest 一致 + golden |
| `g9.p0.m122.gameplay_field` | M122 | G9.2 + G9.6 | `ci/g9_gameplay_field_smoke.py` | 过滤默认空匹配 = 零影响显式断言；persistent field 注册/注销/变更全 journal 化且 replay 逐 tick hash 一致；World-Field 唯一出口 = GpuScene 只读 buffer、渲染侧零回写 |
| `g9.p0.m93.visible_cluster_set` | M93 | G9.3 | `ci/g9_visible_cluster_set_smoke.py` | 屏幕空间误差 selection cut 无重叠无空洞机器核验；未驻留页父簇兜底路径有 evidence；空洞注入 RED 臂独立有效 |
| `g9.p0.m94.clas_rt_convergence` | M94 | G9.3 | `ci/g9_clas_rt_convergence_smoke.py` | CLAS 腿与传统 BLAS 回退腿对同场景 ray query 逐命中一致；可见集/BLAS 错开一簇即 RED；静态帧零 AS 构建（非零即 RED） |
| `g9.p0.m95.single_source_truth` | M95 | G9.3 | `ci/g9_single_source_truth_smoke.py` | `VisibleClusterSet` 一份三喂光栅/RT/VSM；蒙皮簇 VisBuffer SW/HW diff=0 维持；旁路单源真相的 variant provenance 校验 RED 臂为硬门；帧末一致性校验进 CI |
| `g9.p0.m96.path_tracer_reference` | M96 | G9.4 | `ci/g9_path_tracer_reference_smoke.py` | 固定 seed 确定性协议位级一致；pbrt-v4 收敛曲线在容差带内；改 seed/跳 RR/关 MIS 三臂 RED 独立有效；本门未绿则 M97~M101 任何画质门不得验收（门序硬约束） |
| `g9.p0.m97.surface_cache` | M97 | G9.4 | `ci/g9_surface_cache_smoke.py` | Card 空洞漏光检测负例 RED 臂独立有效；缺失覆盖只丢能量不漏光断言；Card 图集页格式复用 M04 ABI 不私定 |
| `g9.p0.m98.tracing_fallback_chain` | M98 | G9.4 | `ci/g9_tracing_fallback_chain_smoke.py` | L1 Screen Trace/L2 SWRT/L3 HWRT/L4 Far Field 四级命中率/耗时计数非空；逐级强关回归可检测；禁静默回退（无计数降级即 RED）；L4 依赖 HLOD 接口未就绪时登记 SKIP=not-triggered 不充绿 |
| `g9.p0.m110.world_partition` | M110 | G9.5 | `ci/g9_world_partition_smoke.py` | 三项预算契约字段逐帧 evidence；预算违约注入必排队降级不静默超帧（RED 臂）；代表性大世界 soak hitch p99 ≤ measured 阈值；cell 四事件序列逐字 golden |
| `g9.p0.m118.display_pipeline_view_transform` | M118 | G9.5 | `ci/g9_display_pipeline_view_transform_smoke.py` | ACES 1.3/ACES 2.0/AgX/中性四内置插件逐一 golden（含已知差异记录）；非 HDR 交换链携带 PQ 输出即 RED；HDR 设备标定条件未触发登记 SKIP=not-triggered 不充绿 |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G9.8b。

## 5. Guardrails

见 YAML `guardrails`。特别强调三点：

1. 治理 active 不等于实现 active；G-G9-3 的机器事实（validator READY + 用户 G9.2 开工指令 + actual `next_free` 重校）不可替代。
2. 数字 CI 步骤只能在实现互锁开放后读取 actual `next_free` 再分配；文档中的稳定身份是 symbolic gate key 和脚本名；禁止沿用 `design/` 草案建议值（D3 草案 §⑨ 区间撞 M50 已消费段是已定性的冲突先例）。
3. `closed/open/override/PASS` 都必须来自事实源与追加记录，不从 `partial`、`unresolved`、`SKIP` 推导；deferred history 只追加，禁止静默改判。

## 6. Deferred 处置

| Deferred | G9 处置 |
|---|---|
| RD-039 | 总体维持 open 为法定输入；M06/M09 两分项以 G9 正式立项书为触发证据，history 只追加登记 open-defer → G9 承接（不得改写 G8.7 决策表原文）；M61 mesh shader 分项 strategic_override 接受（→M109 可选 geometry pipeline，顺序硬约束 = 排在 meshlet 页格式 v2 与 GPU-driven 剔除之后），history 只追加 override；其余分项未触发维持 open |
| RD-040 | 逐分项判档：M99 世界 clipmap 级须 measured 触发举证，未举证只做屏幕级；M100 多灯高档须附 workload 证据，不足则只做低档、M15 维持 open-留档；禁止以 UE5 目标静默改写 backfill 字面 |
| RD-041 | M52 SER 分项 strategic_override 接受（→M108 语言层原语 + capability 可选），history 只追加 override；Work Graphs（M56）/async compute 第二腿（M59）/task shader（M62）no-go 维持，RXS-0239/RXS-0270 字面不动，`reserved_` 前缀预留字段不接线 |
| RD-044 | M65b Rapier 深造条件制维持：对标基准先行（M126），「快路径被真实 workload 采用时」字面不变；M49a GPU 粒子 VFX / M49b present pacing 不进 G9 |
| RD-034 | DXIL RT/mesh 上游 blocked 维持 open；D1~D3 仅 Vulkan 主腿，不阻主线 |
| RD-042/043 | 可微物理观察维持不进 D5；GPU 主刚体否决线维持（含「预算隔离的可选副求解器」），Jolt 5.6 GPU compute 只评估不接权威 |

详情始终以 `registry/deferred.json` 为唯一事实源；本表只冻结承接纪律。

## 7. 修订记录与开工裁决

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-09 | 初版契约：按 G9_PLAN v1.1 显式拆分 governance 与 implementation；G9.1 active、G9.2+ blocked；冻结波次门（G-G9-1~11）与 15 个 P0 独立断言（key 命名空间三方逐字一致）；CI 数字延迟到 post-interlock actual-next-free allocation；六项立项裁决逐字登记；§8 只追加区启用。 |

**开工裁决留痕**：

- **用户立项指令**：2026-08-09 会话下达 G9.1 治理包七项交付清单（概述：①立项裁决与 G9.0 不可变 ref 登记、工作树处置；②本契约；③候选决策表；④验收映射；⑤三份伞形 Full RFC 起草并送 D-409 独立对抗性评审；⑥RTX 4070 Ti measured baseline、非空 g9_budget 与 validator 五件套；⑦README 状态镜像与索引 errata——指令原文以会话留痕为准）。该指令授权 G9.1 governance-only 开工，不授权任何 `src/`/`spec/`/`conformance/` 语义实现或编号 CI 步骤 materialize。
- **agent 立项裁决**：依 10 §7、P-13 与 D-406 v2.0，agent 完全自主签署立项裁决；G9.1 治理波即刻 active，G9.2+ 继续由 G-G9-3 硬阻断。
- **不可变基线**：G9.0 文档集不可变 ref = `1d9460a1`。
- **六项立项裁决（逐字登记）**：
  1. 现在立项；G9.0 不可变 ref=`1d9460a1`；G8 遗留 staged 工作树集合「带未提交项立项」，保持 staged 待独立提交，不混入 G9.1 提交。
  2. Safe GPU Operator Platform = defer 至 G10+，承接锚「G10+ Safe GPU Operator Platform 独立期」。
  3. M52 SER / M61 mesh shader 改判**接受**：各记 strategic_override（M52→M108 语言层原语+capability 可选；M61→M109 可选 geometry pipeline，顺序硬约束=排在 meshlet 页格式 v2 与 GPU-driven 剔除之后），deferred.json history 只追加 override，禁静默改判。
  4. G9 规模 = 五模块全进（不分包）。
  5. 神经变形 = 维持 rfcs/0021:122 无归属留痕，不新设 RD；M127 研究子轨，无主线门；边界由 RFC-0024 冻结。
  6. G8.8b 同日放行先例 = 继承（8a full-run 先行完成后允许同日进 8b close-out）。
- **G8 遗留 staged 工作树集合处置**：带未提交项立项；保持 staged 待独立提交；不混入 G9.1 提交。
- RFC-0022/0023/0024（须 D-409 评审后 Agent Approved 方为语义冻结；未 Approved 前本契约对应条款为引用占位，不构成语义冻结）；编号按 2026-08-09 实测 `registry/number_ledger.json` namespaces.RFC `next_free=22` 领取，登记由立项治理统一落。RXS/RD/U/RX/数字 CI 均延迟到实现互锁开放后按 actual `next_free` 领取。

---

## 8. Implementation activation / Close-out（只追加区）

<!-- 首条未来记录只能是 G-G9-3 互锁实测与 implementation_status 解锁凭据；其后追加逐波验收与 close-out。当前不得写 PASS、不得预填 run URL。 -->

### §8.1 G-G9-3 implementation_status 解锁记录（2026-08-09）

- **用户 G9.2 开工指令**：2026-08-09 主会话下达「帮我一次性完成G9，积极委派子agent和workflow」（G9.2~G9.8b 全实现波授权：15 个 P0 + 已 go P1 + P2/留档穷举 + soak + close-out；指令原文以会话留痕为准）。同会话三项执行裁决：①P1 全进（逐波经治理流程只追加进 ACCEPTANCE_MAP §3，不静默并入既有 key）；②CI 验证按波走 PR（每波 spec PR + 实现 PR，self-hosted runner 真跑全部数字步骤）；③蜂群本地资源策略 = 本机+worktree 蜂群（实现 agent 独立 worktree 写码，device 真跑腿回主 checkout 持 `ci/gpu_device_lock.py` 串行，cargo 同时只一个 agent）。
- **互锁 validator 实测**：`py -3 ci/check_g9_implementation_interlock.py --require-ready` → 事实门①~⑥全绿、一致性门 C1~C3 全绿，VERDICT=READY，exit=0（本小节落盘后实测；`--selftest` 5 RED + 1 GREEN + 1 TREE 全过）。
- **共享编号重校准（actual next_free，本 commit 落地时 `registry/number_ledger.json` 实测）**：CI_step `next_free=131`（on_tree_max=130）/ RXS `next_free=344`（on_tree_max=343）/ RD `next_free=45` / U `next_free=54` / RX_error `next_free=7024` / MR `next_free=12` / RFC `next_free=25` / D `next_free=410`。数字 CI 步骤自 131 起按波次实测顺位领取；禁沿用 design/ 草案建议号段（R-G9-7）。
- **front matter 双状态翻转**：`implementation_status: blocked → unblocked`；`active_scope` 追加 `g9_2_plus_implementation_waves`（`status` 维持 `active`，close-out 才 flip）。
- **蜂群基设**（治理面，不占数字 CI 步骤，本 commit 同落）：`ci/gpu_device_lock.py`（本机 GPU/构建互斥锁，2 RED + 1 GREEN selftest 实证互斥有效）、`ci/g9_wave_exit_lib.py`（波次聚合门共享库，同构 g8_wave_exit_lib，DEVICE_FAIL_STATES 原样保留）、`ci/g9_p2_decisions_check.py` 骨架（G9.7 门；当前决策表未落盘，`--gate` 诚实红、`--selftest` 全绿）；`ci/check_g9_implementation_interlock.py` selftest 的 TREE 臂放行为「登记未落盘=BLOCKED／已落盘=READY 两态均正确」（原实现假设永远登记前，解锁后必自相矛盾 FAIL；5 RED + 1 GREEN 断言语义 0-byte）。
- **签署**：白栀（依 10 §7 / P-13 / D-406 v2.0 agent 完全自主签署，G8 §8.1 同模）。`Assisted-by: Claude Code:claude-fable-5`（影响范围：G9_CONTRACT §8.1 与 front matter、ci/ 蜂群基设三件套与 interlock selftest 放行；验证方式：interlock `--require-ready`/`--selftest`、gpu_device_lock `--selftest`、g9_p2_decisions_check `--selftest`/`--gate` 实测，输出如上）。

### §8.2 G9.2 波验收（G-G9-4,2026-08-10）

- **七 P0 独立断言全绿**（M121/M122 骨架段 `--phase g9.2`）：M90 cluster DAG 深化（破坏单调性 fail-closed typed Err 拒录 + 双构建字节相等 + 蒙皮元数据/CLAS 烘焙输入字段 schema roundtrip，步骤 132，纯 host）· M91 RXPL major=2 v2 ABI（新 schema preimage + v1 0-byte 共存 + 篡改 digest fail-closed + device 解码 digest==CPU 7840 字 `2158adbc…`，步骤 133,host+device）· M102 DGC 抽象（IndirectCmdLayout token 装配期 fail-closed + DgcBuffer 无 host 读接口 + capability 阻塞性前置 + device 最小链路哨兵字 golden,步骤 131,U54）· M103 descriptor 全局表（≥65536 条目 device 出图与 host 种子 golden 逐字节相等 + 双向精确相等 + validation=0,步骤 134,U55）· M104 AccessKind 新边（StorageWrite→IndirectCommandRead barrier golden + strict 漏声明拒 + cabi 不可表达诊断 + RFC-0023 §4.4.3 🔒 修订行逐字一致，步骤 135，纯 host）· M121 五域 ParticleAdapter + 写路径仅 impulse/force + M68 journal 迁移 digest 一致（步骤 136，骨架段）· M122 Field 三层解耦 + 八枚举 + 过滤默认空匹配零影响 + persistent journal replay hash（步骤 137，骨架段）。
- **波聚合门**：`ci/g9_wave2_exit_check.py --gate g9.wave.2.exit` VERDICT=PASS（步骤 138，只读汇总七门最新 evidence + RFC-0022/0023/0024 Approved + RD-039/040 维持 open;phase_g9_6_pass=false 不充绿；聚合不遮蔽子断言）。evidence `evidence/g9_wave2_exit_20260810T054348Z.json`。
- **验收命令（实测全绿）**：七门 `--gate` 全 PASS（M90 6/6、M91 8/8、M102 13/13、M103 7 腿、M104 8 腿、M121/M122 骨架全绿）+ 全门 `--selftest` 红绿 + 本地门禁十件套 + G9 三 validator + cargo fmt/clippy/test 全绿。
- **签署**：白栀（D-406 v2.0 agent 完全自主签署）。`Assisted-by: Claude Code:claude-fable-5`（影响范围：G9.2 波 7 P0 实现 + 步骤 131~138 + evidence/schema/check_schemas/ledger v1.75~v1.80;验证方式：七门 --gate/--selftest + wave2.exit 聚合 + 本地门禁全绿实测）。

### §8.3 G9.3 波验收（2026-08-12）

- **七门独立断言全绿**（步骤 139~145，各门取最新 UTC evidence 一份）：`g9.p0.m93.visible_cluster_set`（步骤 139，纯 host；`evidence/g9_m93_visible_cluster_set_20260812T102613Z.json`）· `g9.p0.m94.clas_rt_convergence`（步骤 140，host+device；`evidence/g9_m94_clas_rt_convergence_20260812T104020Z.json`）· `g9.p0.m95.single_source_truth`（步骤 141，host+device；`evidence/g9_m95_single_source_truth_20260812T105747Z.json`）· `g9.p1.m92.gpu_skinning_lod_update`（步骤 142，host+device；`evidence/g9_m92_gpu_skinning_lod_update_20260812T105925Z.json`）· `g9.p1.m105.command_build_node`（步骤 143，host+device；`evidence/g9_m105_command_build_node_20260812T110549Z.json`）· `g9.p1.m106.execution_set_pso`（步骤 144，host+device；`evidence/g9_m106_execution_set_pso_20260812T110854Z.json`）· `g9.p1.m107.shader_library_ir_link`（步骤 145，纯 host；`evidence/g9_m107_shader_library_ir_link_20260812T102528Z.json`）。
- **波聚合门**：`ci/g9_wave3_exit_check.py --gate g9.wave.3.exit` VERDICT=PASS（步骤 146；只读汇总七门最新 evidence + RFC-0022/0023 Agent Approved 字面维持 + RXS-0350~0356 条款头在树〔spec/virtual_geometry.md 0350~0353 + spec/gpu_driven_submit.md 0354~0356〕+ U56/U57 unsafe 登记在树〔unsafe-audit/rurix-rt.md〕；聚合不代绿、不重跑 smoke、不设 RURIX_REQUIRE_REAL、不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE）。evidence `evidence/g9_wave3_exit_20260812T113534Z.json`。
- **验收命令（实测全绿）**：聚合门 `--gate` PASS + `--selftest` 红绿双全（空 evidence 目录负样本必红 / 真树七门正样本绿）+ `py -3 ci/check_schemas.py` / `py -3 ci/check_g9_acceptance_map.py` / `py -3 ci/check_number_ledger.py` / `py -3 ci/trace_matrix.py --check` 四守卫全 PASS。
- **签署**：白栀（D-406 v2.0 agent 完全自主签署）。`Assisted-by: Kimi-K3`（影响范围：G9.3 波聚合门步骤 146 五件套——`ci/g9_wave3_exit_check.py` + `milestones/g9/g9_wave3_exit_evidence_schema.json` + `ci/check_schemas.py` 三处纯追加 + `pr-smoke.yml` 步骤 146 + CI_GATES v1.6 / ledger v1.86 / 本小节留痕；验证方式：wave3.exit 聚合 --gate/--selftest 实测 + 四守卫全绿，输出如上）。

### §8.4 G9.4 波验收（2026-08-12）

- **六门独立断言全绿**（步骤 147~152，各门取最新 UTC evidence 一份）：`g9.p0.m96.path_tracer_reference`（步骤 147，host+device，12/12，device=executed；`evidence/g9_m96_path_tracer_reference_20260813T055651Z.json`）· `g9.p0.m97.surface_cache`（步骤 148，host+device，12/12，device=executed；`evidence/g9_m97_surface_cache_20260813T055714Z.json`）· `g9.p0.m98.tracing_fallback_chain`（步骤 149，host+device，14/14，device=executed；`evidence/g9_m98_tracing_fallback_chain_20260813T055741Z.json`）· `g9.p1.m99.spg_radiance_cache`（步骤 150，host+device，13/13，device=executed；`evidence/g9_m99_spg_radiance_cache_20260813T060302Z.json`）· `g9.p1.m100.multi_light_low`（步骤 151，host+device，13/13，device=executed；`evidence/g9_m100_multi_light_low_20260813T060329Z.json`）· `g9.p1.m101.if_tier_ladder`（步骤 152，host+device，14/14，device=executed；`evidence/g9_m101_if_tier_ladder_20260813T060509Z.json`）。六 device 门 env 双置 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1` 且 `ci/gpu_device_lock.py` 持锁串行（`CARGO_TARGET_DIR=target-g94g` 隔离）。
- **门序机器阻断实测**（D2-Q7 硬约束）：M96 门未绿时 M97 门 `--gate` 前置机器核验失败 FAIL 退 1 留痕（harness 直出件不充绿——`evidence/g9_m96_path_tracer_reference_*.json` 须 `status=="pass"` 且 `assertion_id=="g9.p0.m96.path_tracer_reference"`）；M96 门绿后 M97~M101 五门逐门打印「M96 门最新 evidence … status=pass(门序前置满足)」放行走查通过；聚合门事实项 `gate_order_interlock_enforced=PASS`（五门最新 evidence 均含 `checks.gate_order_m96_passed=true`）。
- **波聚合门**：`ci/g9_wave4_exit_check.py --gate g9.wave.4.exit` VERDICT=PASS（步骤 153；只读汇总六门最新 evidence + RFC-0022 Agent Approved 字面维持 + RXS-0357~0362 条款头在树〔spec/global_illumination.md〕+ 门序机器阻断留痕 + 六冻结带在树；聚合不代绿、不重跑 smoke、不设 RURIX_REQUIRE_REAL、不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE）。evidence `evidence/g9_wave4_exit_20260813T061351Z.json`。
- **验收命令（实测全绿）**：六门 `--gate` 全 PASS + 各门 `--selftest` 红绿双全（含「M96 evidence 缺失必红」臂）+ 聚合门 `--gate` PASS + `--selftest` 红绿双全 + `py -3 ci/check_schemas.py` / `py -3 ci/check_g9_acceptance_map.py` / `py -3 ci/check_number_ledger.py` / `py -3 ci/trace_matrix.py --check` 守卫全 PASS。
- **签署**：白栀（D-406 v2.0 agent 完全自主签署）。`Assisted-by: Kimi-K3`（影响范围：G9.4 波六门 + 聚合门步骤 147~153——`ci/g9_{path_tracer_reference,surface_cache,tracing_fallback_chain,spg_radiance_cache,multi_light_low,if_tier_ladder}_smoke.py` + `ci/g9_gi_interlock.py` + `ci/g9_wave4_exit_check.py` + 七 evidence schema + `ci/check_schemas.py` 三处纯追加 + `pr-smoke.yml` 步骤 147~153 + CI_GATES v1.8/v1.9 / ledger v1.88/v1.89 / 本小节留痕；验证方式：六门与 wave4.exit 聚合 --gate/--selftest 实测 + 守卫全绿，输出如上）。

### §8.5 G9.5 波验收（2026-08-14）

- **十一门独立断言全绿**（步骤 154~164，各门取最新 UTC evidence 一份；全 host 纯 host 确定性门，`device_section_state=not_applicable`）：`g9.p0.m110.world_partition`（步骤 154，6/6；`evidence/g9_m110_world_partition_20260814T020901Z.json`）· `g9.p0.m118.display_pipeline_view_transform`（步骤 155，7/7；`evidence/g9_m118_display_pipeline_view_transform_20260814T021021Z.json`）· `g9.p1.m111.hlod_baking`（步骤 156，6/6；`evidence/g9_m111_hlod_baking_20260814T021050Z.json`）· `g9.p1.m112.atmosphere_froxel`（步骤 157，6/6；`evidence/g9_m112_atmosphere_froxel_20260814T021054Z.json`）· `g9.p1.m113.water_dual_pipeline`（步骤 158，6/6；`evidence/g9_m113_water_dual_pipeline_20260814T021057Z.json`）· `g9.p1.m114.hair_marschner`（步骤 159，7/7；`evidence/g9_m114_hair_marschner_20260814T021124Z.json`）· `g9.p1.m115.skin_burley_diffusion`（步骤 160，6/6；`evidence/g9_m115_skin_burley_diffusion_20260814T021127Z.json`）· `g9.p1.m116.terrain_chunk_cell`（步骤 161，6/6；`evidence/g9_m116_terrain_chunk_cell_20260814T021130Z.json`）· `g9.p1.m117.decal_dbuffer`（步骤 162，6/6；`evidence/g9_m117_decal_dbuffer_20260814T021201Z.json`）· `g9.p1.m119.post_processing_skeleton`（步骤 163，6/6；`evidence/g9_m119_post_processing_skeleton_20260814T021204Z.json`）· `g9.p1.m120.oit_benchmark_harness`（步骤 164，7/7；`evidence/g9_m120_oit_benchmark_harness_20260814T021309Z.json`）。门 key 按 G9_ACCEPTANCE_MAP §2/§3 实记；M111/M119 命名差（harness 直出件 `assertion_id=g9.p1.m111.hlod_runtime`/`g9.p1.m119.post_chain` vs 门 key `hlod_baking`/`post_processing_skeleton`）已于 v1.11 如实登记，门按 harness 实记字面核验不改写。
- **门序/not-triggered 登记面摘要**：G9.5 波无门序机器阻断（D2-Q7 门序硬约束为 G9.4 GI 波专属，本波十一门无此前置）；not-triggered/不定档三处登记面全部机器核验留痕且不充绿——M118 HDR 设备标定 `hdr_calibration.status==not-triggered` 且 `counts_as_green=false`（不假绿、不反向否决 SDR 面全量验证）；M114 strand 档强制精确 OIT 分项 `strand_tier.status==not-triggered`（依赖 M120 精确档 benchmark 裁决数据不足不充绿，消费 M120 测量带并如实记录可用性，承接锚「M120 精确档数据落地后重判，兜底 G9.7 穷举」）；M120 仅测量不定档 `tier_selection.committed==false`（select_default_tier 一律 fail-closed NotMeasuredYet，默认档选型必须引 benchmark 数据）。
- **波聚合门**：`ci/g9_wave5_exit_check.py --gate g9.wave.5.exit` VERDICT=PASS（步骤 165；只读汇总十一门最新 evidence + RFC-0025 Agent Approved 字面维持 + RXS-0363~0373 条款头在树〔spec/world_partition.md 0363~0368 + spec/display_pipeline.md 0369~0373，共 11 枚〕+ M115 MaterialClosure 32B 布局 digest 冻结面核验〔`g9_m115_skin_band.json` 冻结值 ↔ spec RXS-0373 修订行断言字面 ↔ harness 直出件 `material_closure_32b` 机核面三面逐字一致〕+ M114 strand 档 not-triggered 登记字面 + M120 仅测量不定档字面；聚合不代绿、不重跑 smoke、不设 `RURIX_REQUIRE_REAL`、不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE）。evidence `evidence/g9_wave5_exit_20260814T023622Z.json`。
- **验收命令（实测全绿）**：十一门 `--gate` 全 PASS + 聚合门 `--gate` PASS + `--selftest` 红绿双全（空 evidence 目录负样本必红 / 真树十一门正样本绿）+ `py -3 ci/check_schemas.py` / `py -3 ci/check_g9_acceptance_map.py` / `py -3 ci/check_number_ledger.py` / `py -3 ci/trace_matrix.py --check` / `py -3 ci/stable_snapshot.py --check` 守卫全 PASS。
- **签署**：白栀（D-406 v2.0 agent 完全自主签署）。`Assisted-by: Kimi-K3`（影响范围：G9.5 波聚合门步骤 165 五件套——`ci/g9_wave5_exit_check.py` + `milestones/g9/g9_wave5_exit_evidence_schema.json` + `ci/check_schemas.py` 三处纯追加 + `pr-smoke.yml` 步骤 165 + CI_GATES v1.12 / ledger v1.92 / 本小节留痕；验证方式：wave5.exit 聚合 --gate/--selftest 实测 + 守卫全绿，输出如上）。

### §8.6 G9.6 波验收（2026-08-14）

- **五门独立断言全绿**（M121/M122 步骤 136/137 同 step 双 phase 完整期腿 + M124/M126/M125 步骤 166/167/168，各门取最新 UTC evidence 一份；全 host 纯 host 确定性门，`device_section_state=not_applicable`）：`g9.p0.m121.physics_particle_view`（步骤 136 完整期腿 `--phase g9.6`，13/13，`phase_g9_2_pass=true` 且 `phase_g9_6_pass=true`；`evidence/g9_m121_physics_particle_view_20260814T060148Z.json`）· `g9.p0.m122.gameplay_field`（步骤 137 完整期腿 `--phase g9.6`，10/10，双 phase 同真；`evidence/g9_m122_gameplay_field_20260814T060151Z.json`）· `g9.p1.m124.buoyancy_field_channel`（步骤 166，7/7；`evidence/g9_m124_buoyancy_field_channel_20260814T110714Z.json`）· `g9.p1.m126.rapier_benchmark_ab`（步骤 167，8/8；`evidence/g9_m126_rapier_benchmark_ab_20260814T110640Z.json`）· `g9.p1.m125.jolt_56_ab_evaluation`（步骤 168，11/11；`evidence/g9_m125_jolt_56_ab_evaluation_20260814T110511Z.json`）。门 key 按 G9_ACCEPTANCE_MAP §2/§3 实记。
- **M123 no-go 与 not-triggered 登记面摘要**：**M123 双通道判档 = no-go 不充绿**（判档硬前置 Jolt 单线程成本 measured 未满足——树内零 measured artifact；维持 M75 no-go 留档，`physics-async-decorative`/`DecorativePhysicsTickId` 维持「仅判档 go 时生效」字面不启用；承接锚 G9.7 穷举）——登记三面机器核验一致：spec/physics.md RXS-0379 L1「证据非空但 `counts_as_green=false`」字面 + G9_CANDIDATE_DECISIONS v1.5 校准注 + MAP M123 no-go 登记句，且 no-go 不入 MAP §3（零 m123 gate key 机器核验）；M125 采纳臂⑦三件 not-triggered 登记不升格（corpus 迁移/replay 门重跑/判据字面修订均未触发，verdict=`maintain_5_3_default`，「Jolt 5.3」字面钉住处 0-byte，5.3 基线 vendor pin 字面不动）；M126 RD-044 verdict=`maintain_no_go` 诚实登记（condition_literal_unchanged=true，字面不变维持 open-留档，不升格深造、不作验收依赖与生产默认）。
- **门序 interlock 句**：RXS-0375 门序硬约束机器阻断留痕——`ci/g9_physics_interlock.py` 核验 M121 完整期最新 evidence `status=="pass"` 且 `phase_g9_6_pass==true`（骨架期件/harness 直出件/他门件不充绿），M122 完整期门最新 evidence `checks.gate_order_m121_full_passed==true`；M121 完整期未绿前 M122 完整期不得验收。
- **波聚合门**：`ci/g9_wave6_exit_check.py --gate g9.wave.6.exit` VERDICT=PASS（步骤 169；只读汇总五门最新 evidence——M121/M122 完整期聚合裁定核验最新件 `status=="pass"` 且 `phase_g9_6_pass==true`，骨架期绿不替完整期充绿+ spec/physics.md 在树且 RXS-0374~0379 条款头齐〔共 6 枚，RXS-0376 条款头「解析浮力走 Field 通道」字面〕+ RFC-0024 v1.1 章 F1/F2 字面 + M123 no-go `counts_as_green=false` 登记三面一致 + M125 verdict 与 5.3 基线 0-byte 事实 + M126 RD-044 verdict 登记 + 门序 interlock 留痕；聚合不代绿、不重跑 smoke、不设 `RURIX_REQUIRE_REAL`、不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE）。evidence `evidence/g9_wave6_exit_20260814T113336Z.json`。
- **验收命令（实测全绿）**：聚合门 `--gate` PASS + `--selftest` 红绿双全（空 evidence 目录负样本必红 / 真树五门正样本绿）+ `py -3 ci/check_schemas.py` / `py -3 ci/check_g9_acceptance_map.py` / `py -3 ci/check_number_ledger.py` / `py -3 ci/trace_matrix.py --check` / `py -3 ci/stable_snapshot.py --check` 守卫全 PASS + 既有门回归抽检（wave5.exit 聚合 / M124 / M125 / M126 / M121+M122 完整期腿）全 PASS。
- **签署**：白栀（D-406 v2.0 agent 完全自主签署）。`Assisted-by: Kimi-K3`（影响范围：G9.6 波聚合门步骤 169 五件套——`ci/g9_wave6_exit_check.py` + `milestones/g9/g9_wave6_exit_evidence_schema.json` + `ci/check_schemas.py` 三处纯追加 + `pr-smoke.yml` 步骤 169 + CI_GATES v1.18 / ledger v1.98 / 本小节留痕；验证方式：wave6.exit 聚合 --gate/--selftest 实测 + 守卫全绿 + 既有门回归抽检，输出如上）。

### §8.7 G9.7 波验收（G-G9-9，2026-08-14）

- **决策门 `g9.wave.7.decisions` VERDICT=PASS**（步骤 170，普通检查门非聚合门，required_gates 空）：`G9_P2_DECISIONS.md` v1.0 **33 行闭集穷举零空行**（G9_CANDIDATE_DECISIONS 47 行实记全集未进 34 key 验收面者 + G9.3~G9.6 新增 not-triggered/no-go 登记面去重）——**no-go 23 行 + defer-to-G10+ 10 行 + go 0 行**；每行承接锚「重判条件 + 兜底」齐备，defer 行全含 G10+ 重评窗字面；裁决枚举合法、FROZEN_IDS 33 行闭集全等（候选全集基数对账）；横向机核①MAP 34 key（15 P0 + 19 已 go P1 实解）互斥零命中②deferred.json history 对账（G9.7 P2 defer 登记恰好 RD-039 +1〔M61〕/ RD-040 +3〔M52/M99-clipmap/M100-high〕，零新 RD max=RD-044，RD-039/040 条目级 status open 维持 0-byte）。no-go/defer 如实保持 open/留档，**不写进全绿叙述、不阻塞 G9.8a**（G-G9-9 字面）。evidence `evidence/g9_p2_decisions_20260814T122539Z.json`（full-run 回归腿复跑刷新件 `20260815T030523Z` 同 PASS）。
- **验收命令（实测全绿）**：门 `--gate` PASS + `--selftest` 红绿双全（真表绿/合成全表绿 + 缺行/缺 G10+ 锚/非法枚举/互斥违例/空单元格/deferred 缺登记六臂必红）+ `py -3 ci/check_schemas.py` / `py -3 ci/check_g9_acceptance_map.py`（34 key 不动）/ `py -3 ci/check_number_ledger.py` / `py -3 ci/trace_matrix.py --check`（361/361）/ `py -3 ci/stable_snapshot.py --check` 守卫全 PASS。
- **签署**：白栀（D-406 v2.0 agent 完全自主签署）。`Assisted-by: Kimi-K3`（影响范围：G9.7 决策门步骤 170 六件套——`milestones/g9/G9_P2_DECISIONS.md` v1.0 + `ci/g9_p2_decisions_check.py` + `milestones/g9/g9_p2_decisions_evidence_schema.json` + `ci/check_schemas.py` 三处纯追加 + `pr-smoke.yml` 步骤 170 + `registry/deferred.json` history 只追加四条与 CI_GATES v1.19 / ledger v1.99 / 本小节留痕；验证方式：决策门 --gate/--selftest 实测 + 守卫全绿，输出如上）。

### §8.8 G9.8a 波验收（G-G9-10，2026-08-15）

- **稳定门 `g9.wave.8a.soak` full-run VERDICT=PASS**（步骤 171，evidence `evidence/g9_stabilization_soak_20260815T033526Z.json`，`host_section_pass=true`、checks 六键全真、exit=0）。四腿全绿：**①全量回归**（15 P0 + 19 go P1 逐门真跑 `--gate`——M121/M122 走 `--phase g9.6` 完整期腿双 phase 真，M121 先于 M122〔RXS-0375 门序〕、M96 先于 M97~M101〔D2-Q7 门序〕；M90/M91 按 G9.2 既定无顶层 status/base_commit 形态 wel 口径核验并闭集断言恰 {M90,M91}；携带 base_commit 的 **32 门 evidence 同值且=HEAD `1d298017`**，40 门 evidence UTC stamp ≥ run 起点新鲜度机核，MAP §6 同一候选 close-out 基线；波聚合门 wave2~wave6 exit + p2_decisions 真跑核验，required_gates 恰 40 行）；**②M110 大世界流送长 soak**（`g9_m110_world_partition --long-soak`：frames=**286324** ≥ 10000、seconds=**1800.003** ≥ 1800 双阈值同满；`sleep_seconds=0.0` 恒零、`active_frame_seconds=1799.961` ≈ soak_seconds、gate 外测墙钟 1802.236s 交叉核验无谎报；hitch p99=9.215ms、total_events=53984316、total_cells_streamed=13496100 计数非空；host-soak 无 device 零错字面量门）；**③budget_eval --strict**（exit 0，**131 pass / 0 skip**，非空零 estimated/skip）；**④纪律日期锚**（utc_date=20260815）。`--verify-latest` PASS（pr-smoke 步骤 171 同模式）+ `--selftest` 5 红 + 1 绿全过。
- **full-run 三跑如实登记**（防假绿纪律留痕）：首跑 `20260814T141928Z` **FAIL**（初版口径误拒 M90/M91 无顶层 status 字段 + wave5.exit smoke exit=1 + base_commit 不统一；口径修正为 M90/M91 缺字段闭集豁免后 40 门于同基线重跑全绿）；次跑 `20260815T025643Z` **FAIL**（40 门回归 + soak honest + budget 三腿已绿，**base_commit_uniform FAIL**——根因实解：`ci/g9_descriptor_global_table_smoke.py` 与 `ci/g9_accesskind_indirect_edge_smoke.py` 写 evidence 时 `git rev-parse HEAD` 输出被 `[:12]` 截断为短哈希，与其余 30 门完整 40 位形态不一致；`fix(g9.2)` 提交 `1d298017` 去除截断统一完整哈希，判据语义 0-byte）；三跑 `20260815T033526Z` **PASS**（四腿全绿）。另：`fix(g9.8a)` 提交 `01c04d5e` 修 `--verify-latest` 对 evidence 平铺 soak 块 hitch 核验兼容（`hitch_p99_ms` 平铺键回退，嵌套优先；--gate 判定路径不受影响）。
- **同日放行先例援引**：立项裁决 6（G8.8b 同日放行先例继承——8a full-run 先行完成后允许同日进 8b close-out）；本 full-run 于 2026-08-15 完成，8b close-out 同日推进，先例字面不扩展解释（soak 双阈值实测满足，未跳过）。
- **验收命令（实测全绿）**：`py -3 ci/g9_stabilization_soak.py --gate g9.wave.8a.soak` exit=0 VERDICT=PASS + `--verify-latest` PASS + `--selftest` 5 红 + 1 绿 + `py -3 ci/check_schemas.py` / `py -3 ci/check_g9_acceptance_map.py` / `py -3 ci/check_number_ledger.py` 守卫全 PASS。
- **签署**：白栀（D-406 v2.0 agent 完全自主签署）。`Assisted-by: Kimi-K3`（影响范围：G9.8a 波收口——full-run 三跑与两笔 fix（`1d298017` M103/M104 base_commit 完整哈希统一、`01c04d5e` verify-latest 平铺 hitch 兼容）+ 本小节留痕；验证方式：soak 门 --gate/--verify-latest/--selftest 实测 + 守卫全绿，输出如上）。
