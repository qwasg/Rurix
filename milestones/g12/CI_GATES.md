<!-- Assisted-by: Kimi-K3（G12.1 治理波起草） -->
# G12 CI_GATES — 路径追踪生产化期机器门

> 契约：[G12_CONTRACT.md](G12_CONTRACT.md) v1.0 · 计划：[G12_PLAN.md](G12_PLAN.md) v1.0 · 候选决策：[G12_CANDIDATE_DECISIONS.md](G12_CANDIDATE_DECISIONS.md) v1.0 · 验收映射：[G12_ACCEPTANCE_MAP.md](G12_ACCEPTANCE_MAP.md) v1.0。
> 当前状态（v1.0，2026-08-17）：**G12.1 governance-only，G12.2+ blocked**（`implementation_status: blocked`）。本文 §4 的 8 个 P0 key 与 §4A 的 1 个 P1 key 当前全部未 materialize——脚本、schema、workflow 步骤一件未落；任何「G12.2 开工」叙述都不得当作 PASS。治理 validator 落地后必须诚实输出 `BLOCKED`，直到 §6 互锁条件同时为真。

---

## 1. 互锁与编号纪律

### 1.1 实现互锁

稳定治理 validator 名为 `ci/check_g12_implementation_interlock.py`，属于 `check_*` 类未编号守卫。其实现后必须读取事实源并逐项输出：

1. `milestones/g11/G11_CONTRACT.md` §8.8 的有效 status 是否为 `closed`（2026-08-17，flip commit `51279d45` + 回归刷新批 `5ae83aa7`）；G12.0 不可变 ref `5ae83aa7` 是否已登记；
2. Full RFC-0029（G12 路径追踪生产化伞形）是否经 D-409 独立 provenance 对抗性评审后 Agent Approved（编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free=29` 领取）；
3. `G12_CANDIDATE_DECISIONS.md` 是否无空行，`registry/deferred.json` history 是否只追加、无静默改判，`G12_ACCEPTANCE_MAP.md` §1/§2 是否无缺行；
4. 用户 G12.2 开工指令是否留痕（2026-08-15 指令全期授权面——「支持 dlss、超分采样、路径追踪等前沿技术」字面）；workflow 与 ledger 的实际末号/`next_free` 是否一致。

全假或任一为假时 `BLOCKED` 是唯一正确结论；禁止把 `--expect-blocked` 一类测试模式当成互锁 PASS——它只能证明 validator 能识别阻断。G12.2 起每个实现 PR 必须把 `--require-ready` 作为前置 required check。互锁全绿后才允许 `src/`、`spec/`、`conformance/` 改动，且 spec 条款 PR 先于实现 PR（G12_PLAN §2 spec-first + RED 先行）。

### 1.2 数字步骤延迟分配

- G12 的稳定身份是本文件中的 `symbolic_gate_key` 与 `script`。所有未来编号栏统一写 **`post-interlock actual-next-free allocation`**。
- 只有 §6 互锁全绿后，才可同时读取 `.github/workflows/pr-smoke.yml` 与 `registry/number_ledger.json`，按合入时实际 `next_free` 给即将 materialize 的脚本分配数字步骤，并在同一 PR 追加 ledger 校准。**当前实测 `CI_step.next_free=217`（G11 已消费至 216，[G11 CI_GATES](../g11/CI_GATES.md) v1.9 / ledger v1.123）；G12 编号自互锁后实测 `next_free` 顺位领取，禁预占、禁沿用任何草案建议值**。
- 不创建"预留" workflow step、空 YAML job、空脚本、永远 PASS 的 schema 壳或注释占位。脚本 + RED/GREEN 自检 + schema + workflow 真步骤 + ledger 校准同一实现 PR 落。

### 1.3 三层 CI 口径

- **PR Smoke**（`.github/workflows/pr-smoke.yml`）：G12 各 P0/P1 门与波聚合门的常驻承载层；数字步骤按 §1.2 纪律 post-interlock materialize；device 门 env 双置 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`（沿 G9/G10/G11 体例）。
- **Nightly / full-run**：G12.7a soak full-run 与生产化链路连续复跑的承载层（本地 / `workflow_dispatch` 产 evidence，pr-smoke 侧 `--verify-latest` 秒级核最新 full-run evidence，沿 G9.8a/G10.8a/G11.7a 体例）；soak 量级沿 G11.7a 继承（≥1800s）或 measured 证明更短足够，阈值 G12.1 裁决 measured 标定。
- **Release**：不新增 G12 专属 release 门；Release 面只消费 G12.7b close-out 终审 evidence 与终审锁定的生产化差距清单。
- 三层接线形态一律 post-interlock materialize；本文只冻结口径与 symbolic 身份，不以文档表格冒充 workflow 接线。

## 2. 既有守卫与 0-byte 边界

G12.1 可运行且不得改弱的既有守卫：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -3 ci/check_number_ledger.py
py -3 ci/check_schemas.py
py -3 ci/check_structure.py
py -3 ci/check_guardrails.py <g7-closed-or-pr-base>
py -3 ci/check_contribution.py
py -3 ci/trace_matrix.py --check
py -3 ci/budget_eval.py
```

G5~G11 已 materialize 的全部数字 CI 步骤判据 0-byte 只增；G5~G11 四件套/决策表/evidence schema/budget/复测清单 0-byte，closed 判据不回写；M96 参照器既有判据（RXS-0357 起步范围/确定性协议/pbrt-v4 容差带/golden 门序 D2-Q7）与 `g9_m96_pbrt_tolerance_band.json` 冻结带 0-byte 只消费不回写。G12.1 不改 `.github/workflows/pr-smoke.yml`，也不以文档表格冒充 workflow 接线；spec/conformance 在 G12.1 期 0-byte（registry 登记/翻转/history 追加归立项治理动作，与本文件无关）。UE 源码仅外部参照只读（PathTracing.cpp 只读可参照；零 vendoring、零片段复制）；压测资产二进制不入 git（外部缓存 K: 盘 + 仓库内元数据登记）；temporal 底座 0-byte 不接线；异己会话 src/ 未提交面严禁消费/混入。

## 3. G12.1 governance-only 机器门

| Symbolic gate key | 稳定脚本/检查 | 数字步骤 | 判据 |
|---|---|---|---|
| `g12.gov.structure` | 既有 `ci/check_structure.py` + `ci/check_schemas.py` | 不编号（`check_*`） | CONTRACT/CI/decision/map/RFC 结构一致；map 中预定 schema 名唯一，实际 schema 只与对应脚本同 PR 落，不预建空壳 |
| `g12.gov.number_isolation` | 既有 `ci/check_number_ledger.py` | 不编号（`check_*`） | RFC-0029 claim 与既有命名空间隔离；RXS/RD/U/RX/数字 CI 零推测 claim、零草案建议号沿用 |
| `g12.gov.implementation_interlock` | `ci/check_g12_implementation_interlock.py` | 不编号（`check_*`） | 当前应诚实报 `BLOCKED`；仅 §6 互锁条件全绿时才输出 READY receipt |
| `g12.gov.acceptance_coverage` | `ci/check_g12_acceptance_map.py` | 不编号（`check_*`） | 8 个 P0 + 1 个已 go P1 key/script/schema/check 双向全覆盖；MAP §1 / CONTRACT §4.2 / 本文 §4 三向逐字一致（P1 行 MAP §2 ↔ 本文 §4A 双向比对）；候选决策表无缺行 |
| `g12.gov.measured_baseline` | 既有 `ci/budget_eval.py` | 不编号（`check_*`） | `g12_budget.json` 非空 measured_local、零 estimated（P-09），counter 与 evaluator 同步；当前不得声称实现性能通过 |

这些 validator 可以在 G12.1 落地，但不得带数字"步骤 NN"，也不得把 G12.2 目标脚本接进 workflow。

## 4. 8 个 P0 独立机器断言

下表的 key 与脚本名冻结，与 [G12_ACCEPTANCE_MAP.md](G12_ACCEPTANCE_MAP.md) §1 逐字一致，由 `ci/check_g12_acceptance_map.py` 三向比对强制。每一行均须独立 evidence subject 和独立结果；同一 workflow 进程可以顺序调用多个脚本，但任一行 `FAIL`、`SKIP` 或 `DEV_ENV_DEGRADE` 都必须保持可见，聚合结果不得 PASS。`numeric_step` 一律为 `post-interlock actual-next-free allocation`。Evidence schema 只冻结目标路径不预建文件；schema 形态见 §7。**生产化判据统一形态**（M158~M162 五行共用）：生产化落盘（只消费 M96 冻结面 + 候选决策表对应行）+ 正确性锚 0-byte（M96 既有判据/固定 seed 确定性协议/golden 门序 D2-Q7）+ 收敛/方差/噪声面 measured 不劣于参照器基线锚（容差由 G12.2 标定程序 measured 产出禁手写；或演进位显式登记即 RED 评审面）+ 不降级既有 62 门绿面；**G12 不设绝对 UE PT 画质通过线**（契约 §1/§5 字面）。

| symbolic_gate_key | M## | 最晚波次 | script | evidence schema（目标路径） | 判据摘要 |
|---|---:|---|---|---|---|
| `g12.p0.m158.mis_full_surface` | M158 | G12.2 | `ci/g12_mis_full_surface_smoke.py` | `milestones/g12/g12_m158_mis_full_surface_evidence_schema.json` | MIS 完整面：NEE × BSDF 采样 MIS 权重全路径覆盖 + 能量守恒（白炉 + 逐级能量增量单调不增）+ 同 spp 收敛曲线不劣于参照器基线锚 + 确定性协议继承 + M96 既有判据 0-byte；权重缺失/能量偏置/收敛劣化/协议漂移即 RED |
| `g12.p0.m159.russian_roulette_prod` | M159 | G12.2 | `ci/g12_russian_roulette_prod_smoke.py` | `milestones/g12/g12_m159_russian_roulette_prod_evidence_schema.json` | RR 生产化：吞吐自适应 + 无偏补偿闭式 + 最小反弹保障 + 终止率/补偿计数非空 + 收敛不劣于基线锚；早杀偏置/补偿缺失/跳 RR 未检出即 RED |
| `g12.p0.m160.sampling_lds_upgrade` | M160 | G12.2 | `ci/g12_sampling_lds_upgrade_smoke.py` | `milestones/g12/g12_m160_sampling_lds_upgrade_evidence_schema.json` | 采样策略升级 + 低差异序列：序列索引确定性 + 固定 seed 位级一致维持 + RNG 流布局 provenance + 收敛不劣于独立 PCG 流锚；序列非确定/位级破坏未登记/收敛劣化即 RED |
| `g12.p0.m161.convergence_criterion_prod` | M161 | G12.2 | `ci/g12_convergence_criterion_prod_smoke.py` | `milestones/g12/g12_m161_convergence_criterion_prod_evidence_schema.json` | 收敛判据生产化：逐像素方差驱动自适应 spp + 收敛报告（spp 分布/方差/未收敛像素计数非空）+ 误判率 ≤ 标定阈 + 全 spp golden 对拍不偏离冻结带；早停冒充/缺报/golden 偏离即 RED |
| `g12.p0.m162.denoise_pipeline_tsr` | M162 | G12.3 | `ci/g12_denoise_pipeline_tsr_smoke.py` | `milestones/g12/g12_m162_denoise_pipeline_tsr_evidence_schema.json` | 降噪管线 + TSR 联动：噪声谱高频能量下降 measured + 帧均值能量守恒容差内 + temporal 底座 0-byte + NRD 评估报告落盘（评估不接线）+ golden 对拍不降级；系统性偏置/底座接线/评估冒充接入/噪声底未降即 RED |
| `g12.p0.m163.ue_pt_parity` | M163 | G12.4 | `ci/g12_ue_pt_parity_smoke.py` | `milestones/g12/g12_m163_ue_pt_parity_evidence_schema.json` | UE PT 对标：同场景同 spp 双端出图（UE build digest == M128 登记机核；契约 digest 独立冻结不等仍出报告即 RED）+ 收敛曲线逐段/噪声谱/能量守恒 measured 对拍 + UE PathTracing 模块归属差距登记表；不设绝对通过线；超容差静默/差距静默混入/单端缺帧聚合 PASS 即 RED |
| `g12.p0.m164.regression_guard` | M164 | G12.4 | `ci/g12_regression_guard_smoke.py` | `milestones/g12/g12_m164_regression_guard_evidence_schema.json` | 生产化回归门：既有 62 门（G9 34 + G10 14 + G11 14）最新 evidence 全绿只读汇总 + 触改面既有门重跑零降级（M96 golden 门序面真跑抽检）；降级/聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED |
| `g12.p0.m165.pt_throughput_baseline` | M165 | G12.5 | `ci/g12_pt_throughput_baseline_smoke.py` | `milestones/g12/g12_m165_pt_throughput_baseline_evidence_schema.json` | PT 吞吐基线：rays/sec + 帧时 measured（50×3 trimmed mean）入 g12_budget provenance 齐备 + 不设通过线登记 + 正确性锚（digest 0-byte 或演进位登记）；冒充帧率对标/digest 漂移未登记/estimated 冒充即 RED |

> **单一命名空间**：本文件、`G12_CONTRACT.md` §4.2、`G12_ACCEPTANCE_MAP.md` §1/§2 必须引用同一份 key/脚本；`g12.p{0,1}.m###.<slug>` + `ci/g12_<slug>_smoke.py` 为唯一合法形态，由 `ci/check_g12_acceptance_map.py` 三向比对强制。**G12 不设绝对 UE PT 画质通过线**——生产化判据 = 正确性锚 0-byte + measured 不劣于参照器基线锚（契约 §1/§5 / 立项裁决 3）。

## 4A. 已 go P1 独立机器断言（一行：M166）

契约 §4.2 末段「M166（PT 生产化标定）为 P1，入验收映射随主门核验」只追加登记：下行与 [G12_ACCEPTANCE_MAP.md](G12_ACCEPTANCE_MAP.md) §2 逐字一致，由 `ci/check_g12_acceptance_map.py` 双向比对强制（§4 P0 三向比对 0-byte 不改弱）。`numeric_step` 一律为 `post-interlock actual-next-free allocation`，待门脚本/schema/workflow 步骤 materialize 时按 §1.2 纪律落盘实测回填；本节不预建空脚本、空 schema 壳或占位 workflow 步骤。

| symbolic_gate_key | M## | 最晚波次 | script | evidence schema（目标路径） | 判据摘要 |
|---|---:|---|---|---|---|
| `g12.p1.m166.pt_production_calibration` | M166 | G12.2 | `ci/g12_pt_production_calibration_smoke.py` | `milestones/g12/g12_m166_pt_production_calibration_evidence_schema.json` | 生产化闭门槛值标定集（方差削减比/收敛误判率/噪声底——样本集下界 + digest 入 evidence）+ 标定程序可复跑两跑逐位一致 + 标定值 p100×k measured 入 `g12_budget.json` provenance 齐备（P-09）；手写阈值/estimated 冒充/不可复跑/低于下界冒充即 RED |

## 5. 波聚合门与收口机器门清单

以下门脚本名/ symbolic key 一并冻结（同 §1.2：数字步骤一律 `post-interlock actual-next-free allocation`，不预占）。波聚合门为薄壳只读聚合（沿 `ci/g11_wave{N}_exit_check.py` + `ci/g11_wave_exit_lib.py` 同构体例，G12 共享库 = `ci/g12_wave_exit_lib.py`）：聚合不代绿、不重跑 smoke、不设 `RURIX_REQUIRE_REAL`，聚合 PASS 不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE；`required_gates` 闭集与 `aggregate_read_only const true` 进各自 evidence schema。G12.6 决策门/G12.7a soak/G12.7b closeout 沿 `ci/g11_p2_decisions_check.py` / `ci/g11_stabilization_soak.py` / `ci/g11_closeout_check.py` 同构体例（G12.6 决策门脚本 = `ci/g12_p2_decisions_check.py`，G12.1 落骨架：候选全集行级机核与横向对账面，G12.6 时按候选全集扩闭集 materialize——同 G10/G11 先例「骨架期行级机核 → materialize 期扩闭集」）。

| symbolic_gate_key | 波次 | script | evidence schema（目标路径） | 判据摘要 | numeric_step |
|---|---|---|---|---|---|
| `g12.wave.2.exit` | G12.2 | `ci/g12_wave2_exit_check.py` | `milestones/g12/g12_wave2_exit_evidence_schema.json` | M158/M159/M160/M161/M166 汇总：四面生产化落盘 + 正确性锚 0-byte + 收敛/方差面 measured 不劣于基线锚 + 标定值入 g12_budget provenance 齐备（P-09） | post-interlock actual-next-free allocation |
| `g12.wave.3.exit` | G12.3 | `ci/g12_wave3_exit_check.py` | `milestones/g12/g12_wave3_exit_evidence_schema.json` | M162 汇总：降噪管线落盘 + 噪声底回归 measured + 均值能量守恒 + temporal 底座 0-byte + NRD 评估报告落盘（不接线） | post-interlock actual-next-free allocation |
| `g12.wave.4.exit` | G12.4 | `ci/g12_wave4_exit_check.py` | `milestones/g12/g12_wave4_exit_evidence_schema.json` | M163/M164 汇总：对标报告 + 差距登记表落盘（逐段对拍 + 模块归属）+ 契约 digest 门序留痕 + 既有 62 门零降级 | post-interlock actual-next-free allocation |
| `g12.wave.5.exit` | G12.5 | `ci/g12_wave5_exit_check.py` | `milestones/g12/g12_wave5_exit_evidence_schema.json` | M165 汇总：吞吐基线入 budget provenance + 不设通过线登记 + 正确性锚断言 | post-interlock actual-next-free allocation |
| `g12.wave.6.decisions` | G12.6 | `ci/g12_p2_decisions_check.py` | `milestones/g12/g12_p2_decisions_evidence_schema.json` | G12 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G13+ 零空行；defer 必有承接锚（机核同构 `ci/g11_p2_decisions_check.py`）；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述 | post-interlock actual-next-free allocation |
| `g12.wave.7a.soak` | G12.7a | `ci/g12_stabilization_soak.py` | `milestones/g12/g12_stabilization_soak_evidence_schema.json` | 全部 P0 与 go 的 P1 全量回归；G5~G11 既有判据 0-byte；生产化链路（PT 出图/降噪/对标装配）连续复跑 soak（量级沿 G11.7a 继承〔≥1800s〕或 measured 证明更短足够）；`budget_eval --strict` 非空、零 estimated/skip | post-interlock actual-next-free allocation |
| `g12.wave.7b.closeout` | G12.7b | `ci/g12_closeout_check.py` | `milestones/g12/g12_wave7b_closeout_evidence_schema.json` | 验收映射、候选决策、RD 最终状态逐字一致；全部 P0 独立断言均 PASS；evidence/schema/预算终审；生产化差距清单终审锁定（残余差距/未闭环行如实登记）；§8 只追加后 status active→closed 前置 | post-interlock actual-next-free allocation |

## 6. G12.2 互锁

`G12.GOV.G12_2.ENTRY_INTERLOCK` 条件与判据字面见 [G12_ACCEPTANCE_MAP.md](G12_ACCEPTANCE_MAP.md) §6（G11 closed + G12.0 不可变 ref `5ae83aa7` 登记 + Full RFC-0029 经 D-409 评审 Agent Approved + 决策表/验收映射无缺行且 deferred history 只追加 + acceptance_coverage 与 measured_baseline 双 PASS + 数字步骤按互锁后 actual next_free 分配 + 用户 G12.2 开工指令留痕〔2026-08-15 指令全期授权面——「支持 dlss、超分采样、路径追踪等前沿技术」字面〕）。互锁未输出 READY 前：禁止合入 G12.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动；禁止 claim 任何数字 CI step；spec-first + RED 先行自互锁通过后才启动。`check_g12_implementation_interlock --require-ready` 输出 READY 是机器事实，不以叙述替代（契约 G-G12-3）。

## 7. Evidence 形态（沿 G9/G10/G11 schema 范式）

- 每门 evidence 顶层至少含：`schema_version` / `subject` / `milestone`（`G12`）/ `wave` / `assertion_id`（必须等于对应 `symbolic_gate_key`）/ `status`（`pass|fail`；`skip|estimated|advisory` 不充绿）/ `commands` / `environment` / `base_commit` / `run_url` / `timestamp`（UTC）。
- 治理与聚合门形态：`symbolic_gate_key`（const 钉死）/ `host_section_pass`（boolean）/ `device_section_state`（enum：`not_applicable|executed|dev_env_degrade`）/ `checks`（键集闭集全 boolean，逐条打印不以总 `all_pass` 掩盖）/ 聚合门加 `required_gates`（闭集 minItems=maxItems）与 `aggregate_read_only const true`；`numeric_step` materialize 时 const 钉死实测真号。
- 生产化门（M158~M162）evidence 增生产化节字段闭集（materialize 时硬化）：`production` = { `correctness_anchor_unchanged`（boolean，M96 既有判据/digest/门序 0-byte）/ `baseline_anchor_id`（g12_budget 锚条目 id 字面）/ `measured_value`（当次实测）/ `not_worse_than_anchor`（boolean，不劣于判定）/ `threshold_provenance`（容差标定程序来源，禁手写）/ `evolution_register`（演进位显式登记面——无演进登记须为 null 字面，有则非空字符串）}——容差由标定程序 measured 产，evidence 登记溯源。
- 对标门（M163）evidence 增对标节字段闭集（materialize 时硬化）：`parity` = { `contract_digest`（独立冻结 digest）/ `ue_build_id`（== M128 登记值机核）/ `curve_segments`（spp 逐段对拍数组非空）/ `noise_spectrum_delta`（measured）/ `energy_conservation_delta`（measured）/ `gap_registry_file`（UE PathTracing 模块归属差距登记表路径，行集对账）/ `residual_caliber_note`（残余口径差显式登记面——无残余须为 null 字面）}。
- evidence 落盘 `evidence/g12_<slug>_<UTC>.json` 新文件不覆盖既有件（只增不删不改）；文件名 UTC stamp 机核新鲜度。
- 条件语义：`SKIP=not-triggered` 只表示决策已记录，`DEV_ENV_DEGRADE` 只表示环境缺失，两者均不充 P0 绿、不反向否决其他可在当前环境全量验证的面（MAP §3）。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-17 | G12.1 初版：冻结治理/实现双门、8 个 P0 独立 key 与脚本（与 G12_CONTRACT §4.2 / G12_ACCEPTANCE_MAP §1 逐字一致——生产化核心 M158~M161〔G12.2〕+ 降噪 M162〔G12.3〕+ 对标与回归 M163/M164〔G12.4〕+ 性能基线 M165〔G12.5〕）+ 1 个已 go P1 key（§4A M166 pt_production_calibration，与 MAP §2 逐字一致）；生产化判据统一形态字面冻结（§4 头注：生产化落盘 + 正确性锚 0-byte + measured 不劣于基线锚 + 不降级 62 门；不设绝对 UE PT 通过线）；§5 波聚合门与收口门清单（wave2~wave5 exit + wave6 p2 决策 + wave7a soak + wave7b closeout）脚本名冻结；`g12.gov.*` 五个 governance-only 机器门全不编号；三层 CI（PR Smoke/Nightly/Release）口径冻结；§7 evidence 形态沿 G9/G10/G11 schema 范式 + 生产化节/对标节字段闭集（materialize 时硬化）；全部 numeric_step 延迟为 `post-interlock actual-next-free allocation`（当前实测 CI_step next_free=217，G11 已消费至 216）；零 workflow/script/schema 预放，当前实现门诚实 blocked。 |
| v1.1 | 2026-08-17 | G12.2 生产化核心波六 CI 门 materialize 校准（落盘前实测 CI_step.next_free=217 顺位领取 217~222；判据事实源 = G12_CONTRACT §4.2 M158~M161 行 + G-G12-4 + G12_ACCEPTANCE_MAP §1/§2）：步骤 217 = `g12.p1.m166.pt_production_calibration`（ci/g12_pt_production_calibration_smoke.py，纯 host 门——标定两跑逐字节一致 + 7 条 g12.pt.* 标定条目 measured_local 入 g12_budget + 选型 artifact + budget_eval 全 PASS + RED 四臂）/ 218 = `g12.p0.m158.mis_full_surface`（ci/g12_mis_full_surface_smoke.py，host+device 门，env 双置 RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1）/ 219 = `g12.p0.m159.russian_roulette_prod`（ci/g12_russian_roulette_prod_smoke.py，env 双置）/ 220 = `g12.p0.m160.sampling_lds_upgrade`（ci/g12_sampling_lds_upgrade_smoke.py，env 双置）/ 221 = `g12.p0.m161.convergence_criterion_prod`（ci/g12_convergence_criterion_prod_smoke.py，env 双置）/ 222 = `g12.wave.2.exit`（ci/g12_wave2_exit_check.py，host 只读聚合门，G12 共享库 ci/g12_wave_exit_lib.py 落盘）。同批：六门 evidence schema + M166 标定条目共享 schema 落 §7 形态（numeric_step const 钉死真号）+ ci/check_schemas.py 三处纯追加（既有路由 0-byte）+ pr-smoke.yml 步骤 217~222（步骤 216 块后追加）+ registry/number_ledger.json CI_step on_tree_max 216→222、next_free 217→223 + revision_log v1.126。§4/§4A/§5 表体 0-byte——numeric_step 经本行校准回填，不回写表体。 |
