# G9 CI_GATES — UE5 级渲染器与物理引擎正式建造期机器门

> 契约：[G9_CONTRACT.md](G9_CONTRACT.md) · 计划：[G9_PLAN.md](G9_PLAN.md) v1.1 · 能力矩阵：[G9_CAPABILITY_MATRIX.md](G9_CAPABILITY_MATRIX.md) v1.0。
> 当前状态（v1.0，2026-08-09）：**G9.1 governance-only，G9.2+ blocked**（`implementation_status: blocked`）。本文 §4 的 15 个 P0 key 当前全部未 materialize——脚本、schema、workflow 步骤一件未落；任何「G9.2 开工」叙述都不得当作 PASS。治理 validator 落地后必须诚实输出 `BLOCKED`，直到 §5 互锁六条件同时为真。

---

## 1. 互锁与编号纪律

### 1.1 实现互锁

稳定治理 validator 名为 `ci/check_g9_implementation_interlock.py`，属于 `check_*` 类未编号守卫。其实现后必须读取事实源并逐项输出：

1. `milestones/g8/G8_CONTRACT.md` §8.26 的有效 status 是否为 `closed`（flip commit `b4189e79`）；G9.0 不可变 ref `1d9460a1` 是否已登记；
2. RFC-0022/0023/0024 是否均 Agent Approved；
3. `G9_CANDIDATE_DECISIONS.md` 是否无空行，M52→M108 / M61→M109 的 strategic_override 是否已登记 `registry/deferred.json` history（只追加）；
4. 用户 G9.2 开工指令是否留痕；workflow 与 ledger 的实际末号/`next_free` 是否一致。

全假或任一为假时 `BLOCKED` 是唯一正确结论；禁止把 `--expect-blocked` 一类测试模式当成互锁 PASS——它只能证明 validator 能识别阻断。G9.2 起每个实现 PR 必须把 `--require-ready` 作为前置 required check。互锁全绿后才允许 `src/`/`spec/`/`conformance/` 改动，且 spec 条款 PR 先于实现 PR（G9_PLAN §5 G9.2 实现门）。

### 1.2 数字步骤延迟分配

- G9 的稳定身份是本文件中的 `symbolic_gate_key` 与 `script`。所有未来编号栏统一写 **`post-interlock actual-next-free allocation`**。
- 只有 §5 互锁全绿后，才可同时读取 `.github/workflows/pr-smoke.yml` 与 `registry/number_ledger.json`，按合入时实际 `next_free` 给即将 materialize 的脚本分配数字步骤，并在同一 PR 追加 ledger 校准。
- **禁止沿用 `design/` 草案的建议编号区间**（如 D3 §⑨ RXS-0322 起与 G8 M50 实际消费段 RXS-0322~0327 冲突，R-G9-7）；一切编号（RXS/RD/U/RX/CI step/RFC）以领取时实测 `next_free` 为准。RFC-0022/0023/0024 已按 2026-08-09 实测 `namespaces.RFC next_free=22` 顺位领取。
- 不创建“预留” workflow step、空 YAML job、空脚本、永远 PASS 的 schema 壳或注释占位。脚本 + RED/GREEN 自检 + schema + workflow 真步骤 + ledger 校准同一实现 PR 落。

## 2. 既有守卫与 0-byte 边界

G9.1 可运行且不得改弱的既有守卫：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -3 ci/check_number_ledger.py
py -3 ci/check_schemas.py
py -3 ci/check_structure.py
py -3 ci/check_guardrails.py <g8-close-ref-or-pr-base>
py -3 ci/check_contribution.py
py -3 ci/trace_matrix.py --check
py -3 ci/budget_eval.py
```

G8 已 materialize 的全部数字 CI 步骤判据 0-byte 只增；G8 四件套/决策表/evidence schema/budget 0-byte，G8 closed 判据不回写。G9.1 不改 `.github/workflows/pr-smoke.yml`，也不以文档表格冒充 workflow 接线；spec/conformance/registry 在 G9.1 期 0-byte（registry 登记/翻转/history 追加归立项治理动作，与本文件无关）。

## 3. G9.1 governance-only 机器门

| Symbolic gate key | 稳定脚本/检查 | 数字步骤 | 判据 |
|---|---|---|---|
| `g9.gov.structure` | 既有 `ci/check_structure.py` + `ci/check_schemas.py` | 不编号（`check_*`） | CONTRACT/CI/decision/map/RFC 结构一致；map 中预定 schema 名唯一，实际 schema 只与对应脚本同 PR 落，不预建空壳 |
| `g9.gov.number_isolation` | 既有 `ci/check_number_ledger.py` | 不编号（`check_*`） | RFC-0022~0024 claim 与既有命名空间隔离；RXS/RD/U/RX/数字 CI 零推测 claim、零草案建议号沿用 |
| `g9.gov.implementation_interlock` | `ci/check_g9_implementation_interlock.py` | 不编号（`check_*`） | 当前应诚实报 `BLOCKED`；仅 §5 互锁六条件全绿时才输出 READY receipt |
| `g9.gov.acceptance_coverage` | `ci/check_g9_acceptance_map.py` | 不编号（`check_*`） | 15 个 P0 key/script/schema/check 双向全覆盖；MAP §2 / CONTRACT 验收章 / 本文 §4 三向逐字一致；候选决策表无缺行 |
| `g9.gov.measured_baseline` | 既有 `ci/budget_eval.py` + `ci/check_g9_budget_baseline.py` | 不编号（`check_*`） | `g9_budget.json` 非空 measured_local、零 estimated（P-09），counter 与 evaluator 同步；当前不得声称实现性能通过 |

这些 validator 可以在 G9.1 落地，但不得带数字“步骤 NN”，也不得把 G9.2 目标脚本接进 workflow。

## 4. 15 个 P0 独立机器断言

下表的 key 与脚本名冻结，与 [G9_ACCEPTANCE_MAP.md](G9_ACCEPTANCE_MAP.md) §2 逐字一致，由 `ci/check_g9_acceptance_map.py` 三向比对强制。每一行均须独立 evidence subject 和独立结果；同一 workflow 进程可以顺序调用多个脚本，但任一行 `FAIL`、`SKIP` 或 `DEV_ENV_DEGRADE` 都必须保持可见，聚合结果不得 PASS。`numeric_step` 一律为 `post-interlock actual-next-free allocation`。

| symbolic_gate_key | M## | 最晚波次 | script | 判据摘要 |
|---|---:|---|---|---|
| `g9.p0.m90.cluster_dag_deepening` | M90 | G9.2 | `ci/g9_cluster_dag_deepening_smoke.py` | DAG 误差单调逐边核验 + 双构建字节一致 + 破坏单调性 fixture 被拒 |
| `g9.p0.m91.page_format_v2_abi` | M91 | G9.2 | `ci/g9_page_format_v2_abi_smoke.py` | 页格式 v2 编解码往返无损 golden + M04 v1 0-byte 兼容 + 篡改 digest 页被拒 |
| `g9.p0.m102.dgc_abstraction` | M102 | G9.2 | `ci/g9_dgc_abstraction_smoke.py` | DGC token 限制装配期 fail-closed + layout 违规声明被拒 + 缺 capability 禁模拟 |
| `g9.p0.m103.descriptor_global_table` | M103 | G9.2 | `ci/g9_descriptor_global_table_smoke.py` | 全局 descriptor 索引与 shader 实际索引双向精确相等 + ≥65536 条目出图正确 |
| `g9.p0.m104.accesskind_indirect_edge` | M104 | G9.2 | `ci/g9_accesskind_indirect_edge_smoke.py` | 新 AccessKind 边 barrier 推导 golden + 漏声明 indirect 读边装配期 strict 拒 |
| `g9.p0.m121.physics_particle_view` | M121 | G9.2 + G9.6 | `ci/g9_physics_particle_view_smoke.py` | 五域 adapter 全实现 + 写路径仅 impulse/force + M68 journal 迁移无损（双 phase） |
| `g9.p0.m122.gameplay_field` | M122 | G9.2 + G9.6 | `ci/g9_gameplay_field_smoke.py` | 过滤默认空匹配零影响断言 + persistent 全 journal replay hash 一致（双 phase） |
| `g9.p0.m93.visible_cluster_set` | M93 | G9.3 | `ci/g9_visible_cluster_set_smoke.py` | selection cut 无重叠无空洞 + 未驻留页父簇兜底 + 空洞注入 RED |
| `g9.p0.m94.clas_rt_convergence` | M94 | G9.3 | `ci/g9_clas_rt_convergence_smoke.py` | CLAS 腿与回退腿逐命中一致 + 可见集/BLAS 错开一簇即 RED + 静态帧零 AS 构建 |
| `g9.p0.m95.single_source_truth` | M95 | G9.3 | `ci/g9_single_source_truth_smoke.py` | 蒙皮簇 VisBuffer SW/HW diff=0 + 旁路单源真相 variant provenance RED |
| `g9.p0.m96.path_tracer_reference` | M96 | G9.4 | `ci/g9_path_tracer_reference_smoke.py` | 固定 seed 位级一致 + pbrt-v4 收敛曲线容差带 + 改 seed/跳 RR/关 MIS 三臂 RED；门序前置 |
| `g9.p0.m97.surface_cache` | M97 | G9.4 | `ci/g9_surface_cache_smoke.py` | Card 空洞漏光检测臂 RED 有效 + 只丢能量不漏光断言 + 按匹配深度对 M96 golden |
| `g9.p0.m98.tracing_fallback_chain` | M98 | G9.4 | `ci/g9_tracing_fallback_chain_smoke.py` | 四级命中率/耗时计数非空 + 逐级强关回归可检测 + 禁静默回退 |
| `g9.p0.m110.world_partition` | M110 | G9.5 | `ci/g9_world_partition_smoke.py` | 预算违约注入必排队降级 + hitch p99 soak + cell 事件序列逐字 golden |
| `g9.p0.m118.display_pipeline_view_transform` | M118 | G9.5 | `ci/g9_display_pipeline_view_transform_smoke.py` | 四插件逐一 golden + 非 HDR 交换链携带 PQ 输出即 RED；设备标定未触发 SKIP 不充绿 |

> **单一命名空间**：本文件、`G9_CONTRACT.md` 验收章、`G9_ACCEPTANCE_MAP.md` §2/§3 与 RFC-0022~0024 必须引用同一份 key/脚本；`g9.p{0,1}.m##.<slug>` + `ci/g9_<slug>_smoke.py` 为唯一合法形态，由 `ci/check_g9_acceptance_map.py` 三向比对强制。

## 4A. 已 go P1 独立机器断言（G9.3 波四行 + G9.4 波三行）

G9.3 波 P1 全进裁决（[G9_CONTRACT.md](G9_CONTRACT.md) §8.1 裁决①）只追加登记：下表四行与 [G9_ACCEPTANCE_MAP.md](G9_ACCEPTANCE_MAP.md) §3 逐字一致，由 `ci/check_g9_acceptance_map.py` 双向比对强制（§4 P0 三向比对 0-byte 不改弱）。`numeric_step` 一律为 `post-interlock actual-next-free allocation`，待各门脚本/schema/workflow 步骤 materialize 时按 §1.2 纪律落盘实测回填；本节不预建空脚本、空 schema 壳或占位 workflow 步骤。G9.4 波 P1 全进裁决（同一裁决①）第二批只追加登记 M99/M100/M101 三行（2026-08-12，与 MAP §3 同一双向比对强制；M99 世界级 clipmap 未 measured 举证、M100 高档 ReSTIR workload 证据不足，两分项 not-triggered 不充绿，RD-040 校准见 G9_CANDIDATE_DECISIONS v1.3 注）。

| symbolic_gate_key | M## | 最晚波次 | script | 判据摘要 |
|---|---:|---|---|---|
| `g9.p1.m92.gpu_skinning_lod_update` | M92 | G9.3 | `ci/g9_gpu_skinning_lod_update_smoke.py` | GPU 蒙皮输出与 host 参照逐顶点一致 + 保守包围体含全部蒙皮顶点 + 距离分级更新率档位切换确定性 + 蒙皮簇 AS 更新计数非空 |
| `g9.p1.m105.command_build_node` | M105 | G9.3 | `ci/g9_command_build_node_smoke.py` | command build node 全链路零 CPU 回读（readback_counter=0 断言）+ 构建产物与 host 参照逐字节一致 |
| `g9.p1.m106.execution_set_pso` | M106 | G9.3 | `ci/g9_execution_set_pso_smoke.py` | Execution Set 与 PSO 衔接出图正确 + 失效重建确定性 + capability 缺失 fail-closed |
| `g9.p1.m107.shader_library_ir_link` | M107 | G9.3 | `ci/g9_shader_library_ir_link_smoke.py` | IR 链接 interface hash 双构建相等 + 变体预算超限硬失败 RED 臂 |
| `g9.p1.m99.spg_radiance_cache` | M99 | G9.4 | `ci/g9_spg_radiance_cache_smoke.py` | 屏幕级 SPG 自适应细分 + Radiance Cache 双级产物 digest golden + 关 product IS 方差回归可检测 RED；世界级 clipmap 未举证 not-triggered 不充绿 |
| `g9.p1.m100.multi_light_low` | M100 | G9.4 | `ci/g9_multi_light_low_smoke.py` | 低档多灯直接光默认档 golden + 验证射线零跳过偏置门 RED + 高档 ReSTIR 证据不足 not-triggered 不充绿 |
| `g9.p1.m101.if_tier_ladder` | M101 | G9.4 | `ci/g9_if_tier_ladder_smoke.py` | IF L0~L3 共享 probe 着色/八面体编码内核只换空间索引 + 每档 AS 更新预算行消费 AsStats + 超预算强制降档 RED |

## 5. G9.2 互锁

`G9.GOV.G9_2.ENTRY_INTERLOCK` 六条件与判据字面见 [G9_ACCEPTANCE_MAP.md](G9_ACCEPTANCE_MAP.md) §5（G8 closed + G9.0 不可变 ref 登记 + 三 RFC Agent Approved + 决策表无空行且 M52/M61 override 已登记 deferred history + 数字步骤按互锁后 actual next_free 分配 + 用户 G9.2 开工指令留痕）。互锁未输出 READY 前：禁止合入 G9.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动；禁止 claim 任何数字 CI step；spec-first + RED 先行自互锁通过后才启动。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-09 | G9.1 初版：冻结治理/实现双门、15 个 P0 独立 key 与脚本（与 G9_ACCEPTANCE_MAP §2 逐字一致）；`g9.gov.*` 五个 governance-only 机器门全不编号；全部 numeric_step 延迟为 `post-interlock actual-next-free allocation`；禁沿用 design/ 草案建议编号区间；零 workflow/script/schema 预放，当前实现门诚实 blocked。 |
| v1.1 | 2026-08-09 | G9.2 实现波 agent D（M121+M122 骨架期）：`g9.p0.m121.physics_particle_view` → 步骤 **136**（`ci/g9_physics_particle_view_smoke.py --gate … --phase g9.2`，host 门）、`g9.p0.m122.gameplay_field` → 步骤 **137**（`ci/g9_gameplay_field_smoke.py --gate … --phase g9.2`，host 门）按 §1.2 互锁后 actual-next-free 纪律落盘时实测领取（ledger next_free=131，领 136/137，与蜂群预分配计划一致；A/B/C 同波领号以各自落盘实测为准，撞号顺位记 revision_log）。同 PR 落 evidence schema 双件 + check_schemas 三处纯追加 + 新建 apps/g9-physics-gates harness（G8 门 0-byte：g8-physics-gates 字面不动）+ conformance/physics/particle_view/、conformance/physics/field/ 语料。双 phase 纪律：骨架期 `phase_g9_6_pass` 恒 false 不充绿，`--phase g9.6` 调用诚实非零退出。§4 表 M121/M122 行 key/脚本字面不动。 |
| v1.2 | 2026-08-10 | G9.2 波聚合门: 数字步骤按互锁后实测  顺位 materialize 为 **步骤 138**( G9.2 wave2.exit aggregate;脚本  薄壳 + ,evidence schema ;七 P0 subject 只读汇总 + RFC-0022/0023/0024 Approved + RD-039/040 维持 open;M121/M122 骨架期 phase_g9_6_pass=false 不充绿); 由  转为实测 138(落盘前实测 CI_step.next_free=138 顺位;agent A 132/133、B 131、C 134/135、D 136/137 已落满 131~137)。 |
| v1.2 | 2026-08-09 | G9.2 M102 实现波：`g9.p0.m102.dgc_abstraction` 数字步骤按互锁后实测 `next_free` 顺位 materialize 为 **步骤 131**（`pr-smoke.yml` G9.2 M102 DGC abstraction smoke，device 门 env 双置 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`；脚本 `ci/g9_dgc_abstraction_smoke.py`，evidence schema `milestones/g9/g9_m102_dgc_abstraction_evidence_schema.json`，13 checks 双闭集）；`numeric_step` 由 `post-interlock actual-next-free allocation` 转为实测 131(与 agent D 的 136/137 合并;M102 实领 131)；实现面 = `src/rurix-rt/src/dgc.rs`（IndirectCmdLayout token 闭集装配期核验 + DgcBuffer 无 host 读接口结构性断言 + 三后端映射 + capability snapshot 阻塞性前置）+ `src/rurix-rt/src/vk.rs` DGC FFI 段（U54，`VK_EXT_device_generated_commands`）+ `src/rurixc/src/capability_check.rs` 闭集加性两位实位（RXS-0349）+ `src/rurix-rt/src/bin/vk_dgc.rs` device 最小链路。 |
| v1.3 | 2026-08-10 | G9.2 实现 agent A 双 P0 门落地（只追加）：步骤 132（`g9.p0.m90.cluster_dag_deepening`，host 纯 host 门 `ci/g9_cluster_dag_deepening_smoke.py`）+ 步骤 133（`g9.p0.m91.page_format_v2_abi`，host+device 门 `ci/g9_page_format_v2_abi_smoke.py`，env 双置 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`）插入 `.github/workflows/pr-smoke.yml` 步骤 131 块后、步骤 134 块前；evidence schema 双闭集（`milestones/g9/g9_m90_cluster_dag_deepening_evidence_schema.json` device_section_state=not_applicable / `g9_m91_page_format_v2_abi_evidence_schema.json` enum 含 executed）与 `ci/check_schemas.py` 前缀路由（`g9_m90_cluster_dag_deepening_` / `g9_m91_page_format_v2_abi_`）同 PR 落；ledger CI_step on_tree_max 137→133 / next_free 138→134（v1.48，落盘前实测 next_free=138 撞号顺位领取 132/133）。§4 表体冻结不动，本行只追加。 |
| v1.4 | 2026-08-11 | G9.3 波 P1 全进裁决只追加登记（G9_CONTRACT §8.1 裁决①）：新增 **§4A 已 go P1 独立机器断言表**四行——`g9.p1.m92.gpu_skinning_lod_update` / `g9.p1.m105.command_build_node` / `g9.p1.m106.execution_set_pso` / `g9.p1.m107.shader_library_ir_link`（最晚波次均 G9.3，与 G9_ACCEPTANCE_MAP §3 逐字一致，`ci/check_g9_acceptance_map.py` MAP §3 ↔ 本文 §4A 双向比对强制）；numeric_step 一律 `post-interlock actual-next-free allocation` 待 materialize 实测回填，零脚本/schema/workflow 预放。§4 表体 0-byte；G5~G8 closed 判据 0-byte。 |
| v1.5 | 2026-08-12 | G9.3 实现波七 CI 门 materialize（只追加）：数字步骤按 §1.2 互锁后 actual-next-free 纪律落盘前实测 `CI_step.next_free=139` 顺位领取 **139~145**——§4 P0 三行 `g9.p0.m93.visible_cluster_set` → 步骤 **139**（`ci/g9_visible_cluster_set_smoke.py`，host 纯 host 门）、`g9.p0.m94.clas_rt_convergence` → 步骤 **140**（`ci/g9_clas_rt_convergence_smoke.py`，host+device 门，env 双置 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`）、`g9.p0.m95.single_source_truth` → 步骤 **141**（`ci/g9_single_source_truth_smoke.py`，host+device 门，env 双置）；§4A P1 四行 `g9.p1.m92.gpu_skinning_lod_update` → 步骤 **142**（`ci/g9_gpu_skinning_lod_update_smoke.py`，host+device 门，env 双置）、`g9.p1.m105.command_build_node` → 步骤 **143**（`ci/g9_command_build_node_smoke.py`，host+device 门，env 双置）、`g9.p1.m106.execution_set_pso` → 步骤 **144**（`ci/g9_execution_set_pso_smoke.py`，host+device 门，env 双置）、`g9.p1.m107.shader_library_ir_link` → 步骤 **145**（`ci/g9_shader_library_ir_link_smoke.py`，host 纯 host 门）。同批落：七 evidence schema 双闭集（`milestones/g9/g9_m{93,94,95,92,105,106,107}_*_evidence_schema.json`，沿 M102 体例 const 钉死；M93/M107 `device_section_state=not_applicable`）+ `ci/check_schemas.py` 三处纯追加（load/validator/前缀路由，与既有 g9_* 全族互不包含）+ `.github/workflows/pr-smoke.yml` 步骤 138 块后追加七步骤（注释块含「落盘前实测 CI_step.next_free=139 顺位领取」留痕）。numeric_step 由 `post-interlock actual-next-free allocation` 转为实测真号（ledger v1.85：CI_step on_tree_max 138→145 / next_free 139→146）。§4/§4A 表体 0-byte；G5~G8 closed 判据与 G9.2 门脚本 0-byte；零新 RXS/RD/U/SG/RX_error/MR/D/RFC。 |
| v1.6 | 2026-08-12 | G9.3 波聚合门 `g9.wave.3.exit` materialize（只追加）：数字步骤按 §1.2 纪律落盘前实测 `CI_step.next_free=146` 顺位领取 **步骤 146**（`ci/g9_wave3_exit_check.py --gate g9.wave.3.exit`，薄壳聚合 + `ci/g9_wave_exit_lib.py` 复用同构；evidence schema `milestones/g9/g9_wave3_exit_evidence_schema.json`，required_gates minItems=maxItems=7、aggregate_read_only const true、numeric_step const 146）——七门最新 evidence 只读汇总（§4 三 P0：`g9.p0.m93.visible_cluster_set` 步骤 139 / `g9.p0.m94.clas_rt_convergence` 步骤 140 / `g9.p0.m95.single_source_truth` 步骤 141；§4A 四 P1：`g9.p1.m92.gpu_skinning_lod_update` 步骤 142 / `g9.p1.m105.command_build_node` 步骤 143 / `g9.p1.m106.execution_set_pso` 步骤 144 / `g9.p1.m107.shader_library_ir_link` 步骤 145）+ RFC-0022/0023 Agent Approved 字面维持 + RXS-0350~0356 条款头在树（spec/virtual_geometry.md 0350~0353 + spec/gpu_driven_submit.md 0354~0356）+ U56/U57 unsafe 登记在树（unsafe-audit/rurix-rt.md）；聚合不代绿、不重跑 smoke、不设 `RURIX_REQUIRE_REAL`，聚合 PASS 不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE。同 PR 落 `ci/check_schemas.py` 三处纯追加（load/validator/前缀路由 `g9_wave3_exit_`，与 `g9_wave2_exit_` 及 g9_* 全族互不包含，既有路由 0-byte）+ `pr-smoke.yml` 步骤 145 块后追加步骤 146（注释块留痕体例沿步骤 138/145）。ledger v1.86：CI_step on_tree_max 145→146 / next_free 146→147。§4/§4A/§5 表体 0-byte；G5~G8 closed 判据 0-byte；零新 RXS/RD/U/SG/RX_error/MR/D/RFC。 |
| v1.7 | 2026-08-12 | G9.4 波 P1 全进裁决只追加登记（G9_CONTRACT §8.1 裁决①）：§4A 追加三行——`g9.p1.m99.spg_radiance_cache` / `g9.p1.m100.multi_light_low` / `g9.p1.m101.if_tier_ladder`（最晚波次均 G9.4，与 G9_ACCEPTANCE_MAP §3 逐字一致，`ci/check_g9_acceptance_map.py` MAP §3 ↔ 本文 §4A 双向比对强制）；M99 仅屏幕级判 go、M100 仅低档默认判 go（RD-040 条件分项触发举证校准：世界级 clipmap 未 measured 举证、高档 ReSTIR workload 证据不足，两分项 not-triggered 不充绿，G9_CANDIDATE_DECISIONS v1.3 注）、M101 全档判 go；numeric_step 一律 `post-interlock actual-next-free allocation` 待 materialize 实测回填，零脚本/schema/workflow 预放。§4 P0 表体与 §4A 既有四行 0-byte；G5~G8 closed 判据 0-byte；零新 RXS/RD/U/SG/RX_error/MR/D/RFC/CI_step。 |
