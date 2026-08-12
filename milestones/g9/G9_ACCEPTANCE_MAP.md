# G9_ACCEPTANCE_MAP — P0 / 已 go P1 验收映射

> **性质**：G9.1 治理交付物（governance-only）；事实源为 [G9_PLAN.md](G9_PLAN.md) §2、§2.9 与 [G9_CAPABILITY_MATRIX.md](G9_CAPABILITY_MATRIX.md) §6.1、§6.4。  
> **编号纪律**：本表只冻结 symbolic CI gate key，不 claim 数字 CI step。数字步骤必须等 G9.2 硬互锁（§5）通过后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配；**`design/` 草案的建议编号区间（如 D3 §⑨ RXS-0322 起）与 G8 M50 实际消费段冲突，一律不得沿用**（R-G9-7）。  
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS；本波不预建空脚本、空 schema 壳或占位 workflow 步骤。

---

## 1. 覆盖集合与独立绿灯纪律

- P0 精确集合（15 行）：`{M90,M91,M102,M103,M104,M121,M122,M93,M94,M95,M96,M97,M98,M110,M118}`。
- 已 go 的 P1 精确集合：`{M92,M105,M106,M107}`（G9.3 波 P1 全进裁决，[G9_CONTRACT.md](G9_CONTRACT.md) §8.1 裁决①；2026-08-11 只追加登记，见 §3）。后续在对应波次开工前判 go 的 P1，须经治理流程**只追加**修订本表（§1 覆盖集合 + §3 行），不得静默并入现有 key；P0 集合变更属于契约变更，不得以勘误处理。
- 每一行的 symbolic key 同时是该能力唯一的 `assertion_id`。实现后的 evidence 顶层必须至少含：`schema_version`、`subject`、`milestone`、`wave`、`assertion_id`、`status`、`commands`、`environment`、`base_commit`、`run_url`、`timestamp`；其中 `assertion_id` 必须等于本表 key，`status` 必须为 `pass|fail`，不得以 `skip|estimated|advisory` 充绿（条件未触发只能登记 `not-triggered`，见 M98/M118 行）。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论；任一 P0 assertion 缺失或失败，只能使该 P0 为红，**不得用同脚本内另一 assertion 的绿色代替**。
- **单一 key/脚本命名空间**：symbolic key 一律小写点分 `g9.p{0,1}.m<##>.<slug>`，脚本一律 `ci/g9_<slug>_smoke.py`，evidence schema 一律 `milestones/g9/g9_m<##>_<slug>_evidence_schema.json`（slug 与 key 末段同字面）。本表、[G9_CONTRACT.md](G9_CONTRACT.md) 验收章、[CI_GATES.md](CI_GATES.md) §4 与 RFC-0022~0024 引用同一份 key/脚本，由 `ci/check_g9_acceptance_map.py` 三向比对强制一致。
- device 判据统一要求真实路径、`RURIX_REQUIRE_REAL=1`、validation error 为 0；mock、stub、host substitution、仅检查非零输出、缺 provisioning 的 SKIP 均不满足能力门。

---

## 2. P0 硬门（精确 15 行）

| M 行 | Symbolic CI gate key / 脚本 | Evidence schema（目标路径） | 精确 PASS 判据（本行独立 assertion） | 最晚波次 |
|---|---|---|---|---|
| **M90** | `g9.p0.m90.cluster_dag_deepening`<br>`py -3 ci/g9_cluster_dag_deepening_smoke.py --gate g9.p0.m90.cluster_dag_deepening` | `milestones/g9/g9_m90_cluster_dag_deepening_evidence_schema.json` | 固定 mesh 语料的 cluster DAG 两次独立构建 canonical 字节相等；DAG 每条 parent→child 边误差度量单调不增逐边机器核验；注入破坏单调性的 fixture 必须构建期 fail-closed 拒绝（RED 臂）；蒙皮元数据与 CLAS 离线烘焙输入字段按冻结 schema 完整 roundtrip。仅 G8 M01 静态 DAG 输出为绿不能满足本门。 | **G9.2** |
| **M91** | `g9.p0.m91.page_format_v2_abi`<br>`py -3 ci/g9_page_format_v2_abi_smoke.py --gate g9.p0.m91.page_format_v2_abi` | `milestones/g9/g9_m91_page_format_v2_abi_evidence_schema.json` | RXPL 新 major 页格式 v2 的 ABI id/version 与 v1 不同且冻结；checked-in fixtures 经 encode→decode 往返无损，canonical records 与 golden 逐字节相等；M04 v1 页 ABI 0-byte 兼容（v1 消费路径回归 digest 不变）；篡改 digest 的页必须 fail-closed（RED 臂）；device 解码 digest 等于 CPU 解码 digest。该门在 G9.2 冻结，后续波次只消费、不得重定格式。 | **G9.2** |
| **M102** | `g9.p0.m102.dgc_abstraction`<br>`py -3 ci/g9_dgc_abstraction_smoke.py --gate g9.p0.m102.dgc_abstraction` | `milestones/g9/g9_m102_dgc_abstraction_evidence_schema.json` | IndirectCmdLayout/DgcBuffer 类型层无 host 读接口（结构性断言 + 装配期静态核验全真）；token 集合取三后端最小公倍数，超出 token 限制的声明必须装配期 fail-closed（RED 臂）；layout 违规声明被拒；目标硬件 capability snapshot 实测确认为阻塞性前置——缺 capability 必须 fail-closed，禁止静默模拟（P-01）。 | **G9.2** |
| **M103** | `g9.p0.m103.descriptor_global_table`<br>`py -3 ci/g9_descriptor_global_table_smoke.py --gate g9.p0.m103.descriptor_global_table` | `milestones/g9/g9_m103_descriptor_global_table_evidence_schema.json` | reflection/manifest 中「资源→全局 descriptor 索引」与 shader 实际消费索引**双向**精确相等（双向对拍，不接受单向抽查）；≥65536 条目的 fixture 出图与 golden 相等；set/binding 旧路径加性并存、回归 digest 不变；索引分配律/回收进 spec，回收重用不得产生悬空索引。 | **G9.2** |
| **M104** | `g9.p0.m104.accesskind_indirect_edge`<br>`py -3 ci/g9_accesskind_indirect_edge_smoke.py --gate g9.p0.m104.accesskind_indirect_edge` | `milestones/g9/g9_m104_accesskind_indirect_edge_evidence_schema.json` | 新 AccessKind 边 `StorageWrite→IndirectCommandRead` 的 barrier 推导输出与冻结 golden 全等；漏声明 indirect 读边的 fixture 在装配期 strict 模式必须拒绝（RED 臂）；触 G5 Barrier EB 三轴冻结面须经 RFC-0023 显式修订行登记；RXS-0239 单 queue 全序字面不动。command build node 零 CPU 回读由结构性断言 + 回读计数器=0 机器核验。 | **G9.2** |
| **M121** | `g9.p0.m121.physics_particle_view`<br>`py -3 ci/g9_physics_particle_view_smoke.py --gate g9.p0.m121.physics_particle_view --phase g9.2`<br>`py -3 ci/g9_physics_particle_view_smoke.py --gate g9.p0.m121.physics_particle_view --phase g9.6` | `milestones/g9/g9_m121_physics_particle_view_evidence_schema.json` | 五域 `ParticleAdapter` 全部实现；写路径仅 impulse/force，旁路写注入即 RED；`PhysicsParticleRef` 名义类型编译期隔离断言全真；M68 damage journal 迁移为首个 consumer 后，迁移前后逐 tick digest 与 golden 一致、journal 全消费无损；单向事实源纪律 0-byte。schema 同时要求 `phase_g9_2_pass=true` 与 `phase_g9_6_pass=true`；骨架期绿色不能替完整期充绿。 | **G9.2 + G9.6** |
| **M122** | `g9.p0.m122.gameplay_field`<br>`py -3 ci/g9_gameplay_field_smoke.py --gate g9.p0.m122.gameplay_field --phase g9.2`<br>`py -3 ci/g9_gameplay_field_smoke.py --gate g9.p0.m122.gameplay_field --phase g9.6` | `milestones/g9/g9_m122_gameplay_field_evidence_schema.json` | 三层解耦 schema 冻结；首期 `FieldPhysicsType` 八枚举逐项 accept GREEN 与非法枚举 RED；过滤默认空匹配 = 零影响显式断言（field 注册但零匹配时世界状态 hash 与无 field 基线逐位一致）；persistent 注册/注销/变更全 journal 化且 replay 逐 tick hash 一致；World-Field 唯一出口 = GpuScene 只读 buffer、渲染侧零回写断言全真。schema 同 M121 双 phase 要求；任一阶段绿色不能替另一阶段充绿。 | **G9.2 + G9.6** |
| **M93** | `g9.p0.m93.visible_cluster_set`<br>`py -3 ci/g9_visible_cluster_set_smoke.py --gate g9.p0.m93.visible_cluster_set` | `milestones/g9/g9_m93_visible_cluster_set_evidence_schema.json` | 固定多视图场景 `VisibleClusterSet` 的屏幕空间误差 selection cut 逐帧无重叠无空洞（覆盖性机器核验）；强制未驻留页时命中父簇兜底、页到达后转为正确内容（沿 G8.4 迟到页降级语义不重定）；空洞注入负例 RED 臂独立有效；输出 digest 与 golden 全等。静态 LOD cut 无运行时误差驱动的旧输出不能充绿。 | **G9.3** |
| **M94** | `g9.p0.m94.clas_rt_convergence`<br>`py -3 ci/g9_clas_rt_convergence_smoke.py --gate g9.p0.m94.clas_rt_convergence` | `milestones/g9/g9_m94_clas_rt_convergence_evidence_schema.json` | 同一场景 CLAS 主腿（NV）与传统 BLAS 回退腿 ray query **逐命中一致**；可见集与 BLAS 内容错开一簇的注入必须判 RED；静态帧零 AS 构建（构建计数非零即 RED）；Cluster Template 实例化与当帧 multi-indirect 拼装 digest 等于 golden；validation error=0。回退腿为正确性基线，两条腿各自独立 evidence。 | **G9.3** |
| **M95** | `g9.p0.m95.single_source_truth`<br>`py -3 ci/g9_single_source_truth_smoke.py --gate g9.p0.m95.single_source_truth` | `milestones/g9/g9_m95_single_source_truth_evidence_schema.json` | `VisibleClusterSet` 一份三喂光栅/RT/VSM 的 provenance 链完整可机核；蒙皮簇 VisBuffer SW/HW diff=0 维持；旁路单源真相的 variant 必须被 provenance 校验判 RED（负例臂为硬门，R-G9-8）；动画分级作用于 AS 更新、静态帧零 AS 构建；帧末一致性校验断言全真、validation error=0。光栅/RT 各自独立计算可见性的双世界结构即使出图相似也判 FAIL。 | **G9.3** |
| **M96** | `g9.p0.m96.path_tracer_reference`<br>`py -3 ci/g9_path_tracer_reference_smoke.py --gate g9.p0.m96.path_tracer_reference` | `milestones/g9/g9_m96_path_tracer_reference_evidence_schema.json` | 固定 seed 两次运行位级一致；pbrt-v4 对照收敛曲线落入冻结容差带；改 seed（期望不同）、跳过 RR、关闭 MIS 三臂 RED 独立有效；megakernel 起步范围冻结（焦散/体积/specular 链明确 out）。**门序硬约束（D2-Q7）**：本门未绿前 M97~M101 任何画质门不得验收。仅 host `ref_tracer` 非 RT pipeline 级输出不能充绿。 | **G9.4** |
| **M97** | `g9.p0.m97.surface_cache`<br>`py -3 ci/g9_surface_cache_smoke.py --gate g9.p0.m97.surface_cache` | `milestones/g9/g9_m97_surface_cache_evidence_schema.json` | 离线 Card 参数化（≤12/mesh 可配）与运行时辐射度缓存产物 digest 等于 golden；Card 空洞漏光检测负例 RED 臂独立有效；缺失覆盖**只丢能量不漏光**断言（能量差 measured 记录、漏光像素计数=0）；Card 图集页复用页格式 ABI 不私定格式；按匹配深度（1/2/full bounce）对 M96 golden 验收。 | **G9.4** |
| **M98** | `g9.p0.m98.tracing_fallback_chain`<br>`py -3 ci/g9_tracing_fallback_chain_smoke.py --gate g9.p0.m98.tracing_fallback_chain` | `milestones/g9/g9_m98_tracing_fallback_chain_evidence_schema.json` | L1 Screen Trace → L2 SWRT → L3 HWRT（含 hit lighting 档）→ L4 Far Field 四级命中率/耗时计数非空且逐帧 evidence；逐级强制关闭后回归差异必须可检测（强关后输出仍同 golden 即 RED）；实际使用级别必须显式记录，**禁静默回退**；L4 Far Field 依赖 HLOD 接口未就绪时登记 SKIP=not-triggered 不充绿；各档按匹配深度对 M96 golden。 | **G9.4** |
| **M110** | `g9.p0.m110.world_partition`<br>`py -3 ci/g9_world_partition_smoke.py --gate g9.p0.m110.world_partition` | `milestones/g9/g9_m110_world_partition_evidence_schema.json` | 单一持久世界 schema + 2D cell 冻结；三项预算契约逐帧 evidence 非空；预算违约注入必须排队降级、不得静默超帧（RED 臂）；代表性大世界 soak hitch p99 ≤ measured 阈值（阈值来自 `g9_budget.json` 实测标定，禁手写）；cell 四事件（load/unload/activate/deactivate 类）序列与 golden 逐字相等；Data Layer 掩码位只预留不接线。 | **G9.5** |
| **M118** | `g9.p0.m118.display_pipeline_view_transform`<br>`py -3 ci/g9_display_pipeline_view_transform_smoke.py --gate g9.p0.m118.display_pipeline_view_transform` | `milestones/g9/g9_m118_display_pipeline_view_transform_evidence_schema.json` | SDR/scRGB/PQ 三交换链路径运行时切换证据齐备；ACES 1.3/2.0/AgX/中性四内置插件**逐一**对冻结 golden（含 AgX/ACES 已知差异记录）；非 HDR 交换链携带 PQ 输出即 RED；HDR 设备标定层条件未触发时登记 SKIP=not-triggered 或 open-留痕、**不假绿**，且标定层未触发不得反向否决 SDR 上可全量验证的管线/插件面。 | **G9.5** |

---

## 3. 已 go P1 硬门（G9.3 波四行）

G9.3 波 P1 全进裁决（[G9_CONTRACT.md](G9_CONTRACT.md) §8.1 裁决①：P1 全进，逐波经治理流程只追加进本节，不静默并入既有 key）首批登记 M92/M105/M106/M107 四行（2026-08-11，只追加）。M## 承接面按 G9.3 执行波口径（G9_PLAN §2 G9.3 D3 链路行）：M105 = command build node 全链路零 CPU 回读（RFC-0023 §4.4 语义面，在 M104 P0 已冻结的 AccessKind 新边与结构性零回读之上）、M106 = Execution Set 与 PSO 衔接（RFC-0023 §4.2，`submit.execution_set` 预留位转正）、M107 = shader library IR 链接 + 变体预算合并门（RFC-0023 §4.5/§4.6「同波不延后」字面）。后续在对应波次开工前经治理流程判 go 的 P1，按 §1 覆盖集合条**只追加**进入本节（独立 key、独立脚本、独立 schema、独立判据、独立波次），并同步修订 [CI_GATES.md](CI_GATES.md) §4A 追加段；`no-go`/`defer` 项不入本表，不得冒充 PASS。

| M 行 | Symbolic CI gate key / 脚本 | Evidence schema（目标路径） | 精确 PASS 判据（本行独立 assertion） | 最晚波次 |
|---|---|---|---|---|
| **M92** | `g9.p1.m92.gpu_skinning_lod_update`<br>`py -3 ci/g9_gpu_skinning_lod_update_smoke.py --gate g9.p1.m92.gpu_skinning_lod_update` | `milestones/g9/g9_m92_gpu_skinning_lod_update_evidence_schema.json` | GPU 蒙皮 kernel 输出与 host Kerbl 参照逐顶点一致（定点化输入域容差 0；浮点输入域容差须 spec 明示冻结，禁手写掩盖）；bound_inflation 应用后的保守包围体必须含全部蒙皮后顶点（任意姿态序列 100% 包含，法向锥覆盖真实法向）；距离分级更新率档位表为规范闭集（全速/1/2/1/3/1/4，10m 内全速）、档位切换对同输入确定（双运行逐位一致）；蒙皮簇 AS 更新经 AsStats 计数非空可机核；静态帧（无蒙皮输入变化）零 AS 构建。G9.3 波 P1 判 go（G9_CONTRACT §8.1 裁决①）只追加登记。 | **G9.3** |
| **M105** | `g9.p1.m105.command_build_node`<br>`py -3 ci/g9_command_build_node_smoke.py --gate g9.p1.m105.command_build_node` | `milestones/g9/g9_m105_command_build_node_evidence_schema.json` | command build node（compute pre-pass 产 DgcBuffer → indirect pass 消费）全链路零 CPU 回读：DgcBuffer host 读接口不存在结构性断言 + readback_counter=0 机器核验（任何隐式回读含调试路径必须经计数器显式记账，非零即 RED）；构建产物与 host 参照逐字节一致（同输入双构建 digest 相等）。G9.3 波 P1 判 go（G9_CONTRACT §8.1 裁决①）只追加登记。 | **G9.3** |
| **M106** | `g9.p1.m106.execution_set_pso`<br>`py -3 ci/g9_execution_set_pso_smoke.py --gate g9.p1.m106.execution_set_pso` | `milestones/g9/g9_m106_execution_set_pso_evidence_schema.json` | Execution Set 与 PSO 衔接出图正确（同状态仅换 shader 的管线数组 GPU 侧索引切换，材质变体为消费方）；`submit.execution_set` capability 由 RXS-0349 预留位转正（RXS-0311 加性修订行纪律，profile 选择律裁定 fallback）；失效重建对同输入确定；capability 缺失 fail-closed（D3D12 诚实降级 CPU 侧 PSO 切换并显式登记不可表达，禁静默模拟）。G9.3 波 P1 判 go（G9_CONTRACT §8.1 裁决①）只追加登记。 | **G9.3** |
| **M107** | `g9.p1.m107.shader_library_ir_link`<br>`py -3 ci/g9_shader_library_ir_link_smoke.py --gate g9.p1.m107.shader_library_ir_link` | `milestones/g9/g9_m107_shader_library_ir_link_evidence_schema.json` | shader library IR 函数级组合链接 interface hash 确定性（同输入双构建相等；链接拓扑进 manifest 可回放，拓扑 → 产物 digest 重算相等）；符号缺失/类型契约失配/接口失配/循环链接编译期 fail-closed；变体工程级总预算超限装配期硬失败 RED 臂独立有效；死变体检测报告（只报告不自动删）。G9.3 波 P1 判 go（G9_CONTRACT §8.1 裁决①）只追加登记。 | **G9.3** |

---

## 4. G9.1 治理覆盖与空行门

G9.1 必须提供不占 numeric CI step 的 guardrail（脚本名与 [CI_GATES.md](CI_GATES.md) §3 同一份，属 `check_*` 未编号守卫）：

```text
g9.gov.acceptance_coverage
  py -3 ci/check_g9_acceptance_map.py

g9.gov.implementation_interlock
  py -3 ci/check_g9_implementation_interlock.py

g9.gov.measured_baseline
  py -3 ci/budget_eval.py
  py -3 ci/check_g9_budget_baseline.py
```

`ci/check_g9_acceptance_map.py` 的 PASS 判据（coverage + no-empty 两组断言分别独立报告）：

1. P0 行集合与 §1 的 15 项**集合全等**，无遗漏、无额外 P0、无重复；已 go P1 行集合与 §1 声明集合全等（G9.3 波起 = `{M92,M105,M106,M107}`）。
2. 全部 symbolic key 全局唯一，均匹配 `g9\.p[01]\.m\d{2,3}\.[a-z0-9_]+`；每行只有一个 canonical `assertion_id`，没有两个 M 行共享 key。
3. 每一行均有脚本命令、evidence schema、可机器求值的 PASS 判据、最晚波次；共享脚本必须使用不同的 `--gate`（及 `--phase`）参数。
4. **三向一致**：本表 §2、`G9_CONTRACT.md` 验收章与 `CI_GATES.md` §4 对同一 P0 M 行给出的 key 与脚本必须逐字相等；任一处漂移即 FAIL。已 go P1 行做本表 §3 与 `CI_GATES.md` §4A **双向**逐字比对（CONTRACT 验收章为 15 P0 独立断言表，不载 P1 行）。

no-empty 组的 PASS 判据：

- 逐单元格拒绝空串、空白、`TBD`、`TODO`、`待定`、`待补`、`—`；表中全部行的五个必填列均非空。
- 所有 schema 路径必须唯一落到对应 M 行，且文件名含同一 `m##` 与同一 slug；所有波次属于 `G9.2|G9.3|G9.4|G9.5|G9.6` 的非空集合（M121/M122 允许 `G9.2 + G9.6`）。
- G9.1 只核映射完整性，不把尚未 materialize 的脚本/schema 误判为实现绿色；对应能力实现 PR 合入前或同 PR，`ci/check_schemas.py` 必须能校验该 schema 与实际 evidence。

`ci/check_g9_implementation_interlock.py` 的 PASS 判据：逐项读取事实源输出 §5 各条件真值；G9.1 期间必须诚实输出 `BLOCKED`（`--expect-blocked` 只证明 validator 能识别阻断，不算互锁 PASS）；仅全部条件为真时才输出 `READY`（`--require-ready` exit 0）。`ci/check_g9_budget_baseline.py` 的 PASS 判据：`g9_budget.json` 非空、`evidence_level=measured_local`、零 `estimated`，counter 与 evaluator 同步；baseline 只证明测量已建立，不得声称实现性能通过。

三个 validator 均把每组断言逐条打印，不以一个总 `all_pass=true` 掩盖具体缺行；`--selftest` 用受控负样本证明它们能红。治理 evidence schema 与实现期 evidence 同 PR 落，不预建空壳。

---

## 5. G9.2 硬互锁

`G9.GOV.G9_2.ENTRY_INTERLOCK` 是 G9.2 的前置 required check；它属于 `check_*` 治理守卫，不占 numeric CI step。以下条件必须**同时**为真：

1. G8 已 closed（`milestones/g8/G8_CONTRACT.md` §8.26 `status: closed`，2026-08-06，flip commit `b4189e79`），且 G9.0 文档集不可变 ref `1d9460a1` 已登记；G8 遗留 staged 工作树集合按立项裁决「带未提交项立项」保持 staged 待独立提交，未混入 G9.1 提交。
2. RFC-0022（虚拟几何与 GI 语义）/ RFC-0023（GPU-driven 提交与着色系统）/ RFC-0024（物理平台修订，RFC-0021 修订）均达到契约要求的 Agent Approved 状态（D-409 对抗性评审完成且 findings 全部 disposition）。
3. `G9_CANDIDATE_DECISIONS.md` 分项映射无空行；M52 SER → M108 与 M61 mesh shader → M109 的 strategic_override 已真正登记，`registry/deferred.json` history 只追加 override 行、无静默改判；Safe GPU Operator Platform defer 至 G10+（承接锚「G10+ Safe GPU Operator Platform 独立期」）、神经变形维持 rfcs/0021:122 无归属留痕（M127 研究子轨）、G9 规模五模块全进、G8.8b 同日放行先例继承，六项裁决与决策表逐字一致。
4. §4 的 `g9.gov.acceptance_coverage`（coverage + no-empty 两组）独立 PASS。
5. G9 的 numeric CI step claim 发生在上述互锁通过之后，并以 claim 当下 ledger 实测 `CI_step.next_free` 为起点；每个 symbolic key 一对一分配，未复用、未覆盖、未沿用任何草案建议号。
6. 用户 G9.2 开工指令已留痕（仓内可引用记录）。

任一条件为假时，互锁必须返回非零；此时禁止合入 G9.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动。不能用 owner override 绕过任何条件，也不能用本表存在本身当作 G9.2 开工许可。

---

## 6. Close-out 审计

- G9.8a 必须重跑全部 15 个 P0（M121/M122 含 `--phase g9.6` 完整腿）与全部已 go P1 的**各自 assertion**；evidence 的 `base_commit` 必须落在同一候选 close-out 基线，零 skip/estimated；soak 阈值 ≥ G7 量级（≥30min/≥10000 帧），`budget_eval --strict` 非空全 PASS。
- G9.8b 只有在 15 个 P0 key 全 PASS、全部已 go P1 key 全 PASS、决策表最终状态无漂移时才可 status flip；任一 P0 无独立硬门则禁止 flip（G9_PLAN §2.9）。
- 同日放行先例继承：8a full-run 先行完成后允许同日进 8b close-out；条件实现刚绿不得跳过 8a 直接 close。
- 后续若 owner 将新的 P1 判为 go，须先按治理流程修订本表及覆盖集合，再开对应实现；不得把它静默并入现有 key。P0 集合变更属于契约变更，不得以勘误处理。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-09 | G9.1 初版：冻结 15 个 P0 的 symbolic gate、目标脚本/schema、独立判据（自 G9_PLAN §2.9 判据草案 + 矩阵 §6.4 展开，含负例 RED 臂与防假绿句）与最晚波次；已 go P1 为空集并立只追加流程；单一命名空间 `g9.p{0,1}.m##.<slug>` + `ci/g9_<slug>_smoke.py` + `g9_m##_<slug>_evidence_schema.json` 由 `ci/check_g9_acceptance_map.py` 三向比对强制；加入 G9.2 硬互锁（六条件）、覆盖/空行治理门与 Close-out 审计。数字 CI 步骤全部延迟分配，零 workflow/script/schema 预放。 |
| v1.1 | 2026-08-11 | **G9.3 波 P1 全进裁决只追加登记**（[G9_CONTRACT.md](G9_CONTRACT.md) §8.1 裁决①）：§1 已 go P1 集合空集 → `{M92,M105,M106,M107}`；§3 追加四行（M92 `g9.p1.m92.gpu_skinning_lod_update` / M105 `g9.p1.m105.command_build_node` / M106 `g9.p1.m106.execution_set_pso` / M107 `g9.p1.m107.shader_library_ir_link`，脚本 `ci/g9_<slug>_smoke.py` 同 slug，最晚波次均 G9.3，numeric CI step 一律 `post-interlock actual-next-free allocation` 待 materialize 回填）；M105/M106/M107 承接面按 G9.3 执行波口径登记（command build node 全链路零 CPU 回读 / Execution Set 与 PSO 衔接 / IR 链接+变体预算合并门，语义面 RFC-0023 §4.4/§4.2/§4.5/§4.6）；§4 validator 判据描述同步（P1 行 MAP §3 ↔ CI_GATES §4A 双向比对）。`ci/check_g9_acceptance_map.py` 同 PR 扩展 §3 P1 覆盖（`EXPECTED_P1` + 节内作用域解析 + 双向比对组；§2 P0 十五行 coverage/no-empty/三向比对 0-byte 不改弱；selftest 7 RED → 10 RED + 1 GREEN）。**§2 P0 15 行精确集合 0-byte**；G5~G8 closed 判据 0-byte；零脚本/schema/workflow 预放。 |
