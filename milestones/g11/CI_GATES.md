<!-- Assisted-by: Kimi-K3（G11.1 治理波起草） -->
# G11 CI_GATES — 画质修复期机器门

> 契约：[G11_CONTRACT.md](G11_CONTRACT.md) v1.0 · 计划：[G11_PLAN.md](G11_PLAN.md) v1.0 · 候选决策：[G11_CANDIDATE_DECISIONS.md](G11_CANDIDATE_DECISIONS.md) v1.0 · 验收映射：[G11_ACCEPTANCE_MAP.md](G11_ACCEPTANCE_MAP.md) v1.0。
> 当前状态（v1.0，2026-08-16）：**G11.1 governance-only，G11.2+ blocked**（`implementation_status: blocked`）。本文 §4 的 13 个 P0 key 与 §4A 的 1 个 P1 key 当前全部未 materialize——脚本、schema、workflow 步骤一件未落；任何「G11.2 开工」叙述都不得当作 PASS。治理 validator 落地后必须诚实输出 `BLOCKED`，直到 §6 互锁条件同时为真。

---

## 1. 互锁与编号纪律

### 1.1 实现互锁

稳定治理 validator 名为 `ci/check_g11_implementation_interlock.py`，属于 `check_*` 类未编号守卫。其实现后必须读取事实源并逐项输出：

1. `milestones/g10/G10_CONTRACT.md` §8.10 的有效 status 是否为 `closed`（2026-08-16，flip commit `27e3b07c` + 幂等复跑批 `53eb3a28`）；G11.0 不可变 ref `53eb3a28` 是否已登记；
2. Full RFC-0028（G11 GI 与光照画质闭环伞形）是否经 D-409 独立 provenance 对抗性评审后 Agent Approved（编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free=28` 领取）；
3. `G11_CANDIDATE_DECISIONS.md` 是否无空行，`registry/deferred.json` history 是否只追加、无静默改判，`G11_ACCEPTANCE_MAP.md` §1/§2 是否无缺行；
4. 用户 G11.2 开工指令是否留痕（2026-08-15 指令全期授权面）；workflow 与 ledger 的实际末号/`next_free` 是否一致。

全假或任一为假时 `BLOCKED` 是唯一正确结论；禁止把 `--expect-blocked` 一类测试模式当成互锁 PASS——它只能证明 validator 能识别阻断。G11.2 起每个实现 PR 必须把 `--require-ready` 作为前置 required check。互锁全绿后才允许 `src/`、`spec/`、`conformance/` 改动，且 spec 条款 PR 先于实现 PR（G11_PLAN §2 spec-first + RED 先行）。

### 1.2 数字步骤延迟分配

- G11 的稳定身份是本文件中的 `symbolic_gate_key` 与 `script`。所有未来编号栏统一写 **`post-interlock actual-next-free allocation`**。
- 只有 §6 互锁全绿后，才可同时读取 `.github/workflows/pr-smoke.yml` 与 `registry/number_ledger.json`，按合入时实际 `next_free` 给即将 materialize 的脚本分配数字步骤，并在同一 PR 追加 ledger 校准。**当前实测 `CI_step.next_free=196`（G10 已消费至 195，[G10 CI_GATES](../g10/CI_GATES.md) v1.10 / ledger v1.112）；G11 编号自互锁后实测 `next_free` 顺位领取，禁预占、禁沿用任何草案建议值**。
- 不创建"预留" workflow step、空 YAML job、空脚本、永远 PASS 的 schema 壳或注释占位。脚本 + RED/GREEN 自检 + schema + workflow 真步骤 + ledger 校准同一实现 PR 落。

### 1.3 三层 CI 口径

- **PR Smoke**（`.github/workflows/pr-smoke.yml`）：G11 各 P0/P1 门与波聚合门的常驻承载层；数字步骤按 §1.2 纪律 post-interlock materialize；device 门 env 双置 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`（沿 G9/G10 体例）。
- **Nightly / full-run**：G11.7a soak full-run 与修复链路连续复跑的承载层（本地 / `workflow_dispatch` 产 evidence，pr-smoke 侧 `--verify-latest` 秒级核最新 full-run evidence，沿 G9.8a/G10.8a 体例）；soak 量级沿 G10.8a 继承（≥1800s）或 measured 证明更短足够，阈值 G11.1 裁决 measured 标定。
- **Release**：不新增 G11 专属 release 门；Release 面只消费 G11.7b close-out 终审 evidence 与终审锁定的复测差距清单。
- 三层接线形态一律 post-interlock materialize；本文只冻结口径与 symbolic 身份，不以文档表格冒充 workflow 接线。

## 2. 既有守卫与 0-byte 边界

G11.1 可运行且不得改弱的既有守卫：

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

G5~G10 已 materialize 的全部数字 CI 步骤判据 0-byte 只增；G5~G10 四件套/决策表/evidence schema/budget/差距清单 0-byte，closed 判据不回写。G11.1 不改 `.github/workflows/pr-smoke.yml`，也不以文档表格冒充 workflow 接线；spec/conformance 在 G11.1 期 0-byte（registry 登记/翻转/history 追加归立项治理动作，与本文件无关）。UE 源码仅外部参照只读（零 vendoring、零片段复制）；压测资产二进制不入 git（外部缓存 K: 盘 + 仓库内元数据登记）。

## 3. G11.1 governance-only 机器门

| Symbolic gate key | 稳定脚本/检查 | 数字步骤 | 判据 |
|---|---|---|---|
| `g11.gov.structure` | 既有 `ci/check_structure.py` + `ci/check_schemas.py` | 不编号（`check_*`） | CONTRACT/CI/decision/map/RFC 结构一致；map 中预定 schema 名唯一，实际 schema 只与对应脚本同 PR 落，不预建空壳 |
| `g11.gov.number_isolation` | 既有 `ci/check_number_ledger.py` | 不编号（`check_*`） | RFC-0028 claim 与既有命名空间隔离；RXS/RD/U/RX/数字 CI 零推测 claim、零草案建议号沿用 |
| `g11.gov.implementation_interlock` | `ci/check_g11_implementation_interlock.py` | 不编号（`check_*`） | 当前应诚实报 `BLOCKED`；仅 §6 互锁条件全绿时才输出 READY receipt |
| `g11.gov.acceptance_coverage` | `ci/check_g11_acceptance_map.py` | 不编号（`check_*`） | 13 个 P0 + 1 个已 go P1 key/script/schema/check 双向全覆盖；MAP §1 / CONTRACT §4.2 / 本文 §4 三向逐字一致（P1 行 MAP §2 ↔ 本文 §4A 双向比对）；候选决策表无缺行 |
| `g11.gov.measured_baseline` | 既有 `ci/budget_eval.py` | 不编号（`check_*`） | `g11_budget.json` 非空 measured_local、零 estimated（P-09），counter 与 evaluator 同步；当前不得声称实现性能通过 |

这些 validator 可以在 G11.1 落地，但不得带数字"步骤 NN"，也不得把 G11.2 目标脚本接进 workflow。

## 4. 13 个 P0 独立机器断言

下表的 key 与脚本名冻结，与 [G11_ACCEPTANCE_MAP.md](G11_ACCEPTANCE_MAP.md) §1 逐字一致，由 `ci/check_g11_acceptance_map.py` 三向比对强制。每一行均须独立 evidence subject 和独立结果；同一 workflow 进程可以顺序调用多个脚本，但任一行 `FAIL`、`SKIP` 或 `DEV_ENV_DEGRADE` 都必须保持可见，聚合结果不得 PASS。`numeric_step` 一律为 `post-interlock actual-next-free allocation`。Evidence schema 只冻结目标路径不预建文件；schema 形态见 §7。**修复闭环判据统一形态**（M144~M154 十一行共用）：修复落盘（只消费 G10.8b 锁定清单对应行 + 承接锚字面）+ 修复前后度量 delta 收敛 measured（收敛阈值由 G11.2/G11.5 标定程序 measured 产出，禁手写）+ 契约参数 digest 0-byte + 不降级既有 48 门绿面；**G11 不设绝对画质通过线**（契约 §1/§5 字面）。

| symbolic_gate_key | M## | 最晚波次 | script | evidence schema（目标路径） | 判据摘要 |
|---|---:|---|---|---|---|
| `g11.p0.m144.caliber_c1_indoor_luminance` | M144 | G11.2 | `ci/g11_caliber_c1_indoor_luminance_smoke.py` | `milestones/g11/g11_m144_caliber_c1_indoor_luminance_evidence_schema.json` | GI/天光遮蔽口径差 + 太阳 lux→辐射度链差逐行对齐（残余口径差显式登记）+ 口径参数 provenance 齐备；未对齐口径消费复测 delta/拟合冒充对齐/残余未登记即 RED |
| `g11.p0.m145.caliber_c2_exposure_chain` | M145 | G11.2 | `ci/g11_caliber_c2_exposure_chain_smoke.py` | `milestones/g11/g11_m145_caliber_c2_exposure_chain_evidence_schema.json` | 双端 EV100 同字面下派生尺度对齐（统一或显式互证登记）+ 派生链元数据互证回归；未对齐出 LDR 度量/互证链断裂即 RED |
| `g11.p0.m146.caliber_c3_exr_bit_depth` | M146 | G11.2 | `ci/g11_caliber_c3_exr_bit_depth_smoke.py` | `milestones/g11/g11_m146_caliber_c3_exr_bit_depth_evidence_schema.json` | UE fp16→f32 提升口径（RXS-0385 strip-and-log）与 Rurix 原生 f32 度量域对齐登记 + 位深元数据闭集回归；位深截断/元数据缺字段即 RED |
| `g11.p0.m147.fix_r1_material_subset` | M147 | G11.3 | `ci/g11_fix_r1_material_subset_smoke.py` | `milestones/g11/g11_m147_fix_r1_material_subset_evidence_schema.json` | R1 修复闭环：材质子集采样接入 + LDR 臂 delta 收敛 measured（锁定基线 0.8328980787837229）+ 契约 digest 0-byte；未采样冒充/未收敛冒充/契约漂移即 RED |
| `g11.p0.m148.fix_r2_geometry_normals` | M148 | G11.3 | `ci/g11_fix_r2_geometry_normals_smoke.py` | `milestones/g11/g11_m148_fix_r2_geometry_normals_evidence_schema.json` | R2 修复闭环：winding 朝向 + 双面翻转消费 + cornell HDR 覆盖 delta 收敛 measured（锁定基线 −0.7451210021972656）+ 与 U1 对账；未消费冒充/未收敛冒充即 RED |
| `g11.p0.m149.fix_r5_json_u64_seed` | M149 | G11.3 | `ci/g11_fix_r5_json_u64_seed_smoke.py` | `milestones/g11/g11_m149_fix_r5_json_u64_seed_evidence_schema.json` | R5 修复闭环：u64 顶格 seed 合法消费 + 既有 seed=42 契约 digest 不变回归 + u64 边界语料锚定；仍拒绝/digest 漂移即 RED |
| `g11.p0.m150.fix_u1_cornell_shell_radiance` | M150 | G11.3 | `ci/g11_fix_u1_cornell_shell_radiance_smoke.py` | `milestones/g11/g11_m150_fix_u1_cornell_shell_radiance_evidence_schema.json` | U1 修复闭环：壳体零辐射修复（M133 只追加修订程序或口径对齐面）+ UE 帧覆盖收敛 measured（锁定基线 18.39% vs 92.90%）+ Rurix 侧不降级；静默改写/未收敛冒充/降级即 RED |
| `g11.p0.m151.fix_u2_bistro_texture_dds` | M151 | G11.3 | `ci/g11_fix_u2_bistro_texture_dds_smoke.py` | `milestones/g11/g11_m151_fix_u2_bistro_texture_dds_evidence_schema.json` | U2 修复闭环：DDS 解码面落地（G10-N7 承接锚兑现）+ texture_parameter_values 非空回归 + LDR 臂 delta 收敛 measured（锁定基线 0.7698879749655723）；仍全缺冒充/未登记混入/未收敛冒充即 RED |
| `g11.p0.m152.fix_u3_bistro_animation` | M152 | G11.3 | `ci/g11_fix_u3_bistro_animation_smoke.py` | `milestones/g11/g11_m152_fix_u3_bistro_animation_evidence_schema.json` | U3 修复闭环：动画通道消费或显式静态契约登记闭环 + 通道计数对账（0 vs 2）+ 相机位姿契约 0-byte；静默丢弃冒充/相机契约漂移即 RED |
| `g11.p0.m153.fix_r3_light_subset` | M153 | G11.4 | `ci/g11_fix_r3_light_subset_smoke.py` | `milestones/g11/g11_m153_fix_r3_light_subset_evidence_schema.json` | R3 修复闭环：点/面光源 + glTF emissive 表达（4+ 盏实测消费）+ HDR 亮度中位 delta 收敛 measured（锁定基线 2.664779790997505）+ cornell sun+sky 契约灯面 0-byte；未表达冒充/未收敛冒充/灯面漂移即 RED |
| `g11.p0.m154.fix_r4_gi_multibounce_world_cache` | M154 | G11.4 | `ci/g11_fix_r4_gi_multibounce_world_cache_smoke.py` | `milestones/g11/g11_m154_fix_r4_gi_multibounce_world_cache_evidence_schema.json` | R4 + M99-clipmap 修复闭环：世界辐射缓存世界级 clipmap 级落地（RFC-0028 语义面 spec-first，RXS-0360 世界级登记翻转修订行）+ HDR 亮度 p90 delta 收敛 measured（锁定基线 4.697253086805343）；世界级未落地冒充/屏幕级冒充世界级/未收敛冒充即 RED |
| `g11.p0.m155.ab_retest_closure` | M155 | G11.5 | `ci/g11_ab_retest_closure_smoke.py` | `milestones/g11/g11_m155_ab_retest_closure_evidence_schema.json` | A/B 复测闭环：同契约双端复跑（契约 digest == G10.5 锁定值，不等仍出报告即 RED）+ 复测度量报告 + 复测差距清单 11 行闭集逐项闭环机核；缺行/新项静默混入/单端缺帧聚合 PASS 即 RED |
| `g11.p0.m156.regression_guard` | M156 | G11.5 | `ci/g11_regression_guard_smoke.py` | `milestones/g11/g11_m156_regression_guard_evidence_schema.json` | 修复回归门：既有 48 门（G9 34 + G10 14）最新 evidence 全绿只读汇总 + 触改面既有门重跑零降级；降级/聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED |

> **单一命名空间**：本文件、`G11_CONTRACT.md` §4.2、`G11_ACCEPTANCE_MAP.md` §1/§2 必须引用同一份 key/脚本；`g11.p{0,1}.m###.<slug>` + `ci/g11_<slug>_smoke.py` 为唯一合法形态，由 `ci/check_g11_acceptance_map.py` 三向比对强制。**G11 不设绝对画质通过线**——修复闭环判据 = 修复前后度量 delta 收敛 measured（契约 §1/§5 / 立项裁决 3）。

## 4A. 已 go P1 独立机器断言（一行：M157）

契约 §4.2 末段「M157（HDR-FLIP 独立标定）为 P1，入验收映射随主门核验」只追加登记：下行与 [G11_ACCEPTANCE_MAP.md](G11_ACCEPTANCE_MAP.md) §2 逐字一致，由 `ci/check_g11_acceptance_map.py` 双向比对强制（§4 P0 三向比对 0-byte 不改弱）。`numeric_step` 一律为 `post-interlock actual-next-free allocation`，待门脚本/schema/workflow 步骤 materialize 时按 §1.2 纪律落盘实测回填；本节不预建空脚本、空 schema 壳或占位 workflow 步骤。

| symbolic_gate_key | M## | 最晚波次 | script | evidence schema（目标路径） | 判据摘要 |
|---|---:|---|---|---|---|
| `g11.p1.m157.hdr_flip_calibration` | M157 | G11.2 | `ci/g11_hdr_flip_calibration_smoke.py` | `milestones/g11/g11_m157_hdr_flip_calibration_evidence_schema.json` | HDR 域正式对拍样本集（下界 + digest 入 evidence）+ 标定程序可复跑两跑逐位一致 + 标定值 p100×k measured 入 `g11_budget.json` provenance 齐备（P-09；G10-N10 承接锚兑现）；手写阈值/estimated 冒充/不可复跑/低于下界冒充即 RED |

## 5. 波聚合门与收口机器门清单

以下门脚本名/ symbolic key 一并冻结（同 §1.2：数字步骤一律 `post-interlock actual-next-free allocation`，不预占）。波聚合门为薄壳只读聚合（沿 `ci/g10_wave{N}_exit_check.py` + `ci/g10_wave_exit_lib.py` 同构体例，G11 共享库 = `ci/g11_wave_exit_lib.py`）：聚合不代绿、不重跑 smoke、不设 `RURIX_REQUIRE_REAL`，聚合 PASS 不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE；`required_gates` 闭集与 `aggregate_read_only const true` 进各自 evidence schema。G11.6 决策门/G11.7a soak/G11.7b closeout 沿 `ci/g10_p2_decisions_check.py` / `ci/g10_stabilization_soak.py` / `ci/g10_closeout_check.py` 同构体例（G11.6 决策门脚本 = `ci/g11_p2_decisions_check.py`，G11.1 落骨架：候选全集行级机核与横向对账面，G11.6 时按候选全集扩闭集 materialize——同 G10 先例「骨架期行级机核 → materialize 期扩闭集」）。G11 无 defer 重评窗波次（G10 重评窗是 G9 十锚的承接窗；G11 法定输入 = G10.8b 锁定清单直消费 + G10 defer 18 行处置归 G11.1 候选决策表与 G11.6 穷举，不设独立重评窗门——如实登记不设门）。

| symbolic_gate_key | 波次 | script | evidence schema（目标路径） | 判据摘要 | numeric_step |
|---|---|---|---|---|---|
| `g11.wave.2.exit` | G11.2 | `ci/g11_wave2_exit_check.py` | `milestones/g11/g11_wave2_exit_evidence_schema.json` | M144/M145/M146/M157 汇总：三行口径差逐行对齐闭环（残余口径差显式登记）+ HDR-FLIP 标定值入 g11_budget 且 provenance 齐备（P-09） | post-interlock actual-next-free allocation |
| `g11.wave.3.exit` | G11.3 | `ci/g11_wave3_exit_check.py` | `milestones/g11/g11_wave3_exit_evidence_schema.json` | M147~M152 汇总：六行修复落盘 + 局部度量 delta 收敛 + 契约参数 digest 0-byte + 语料修订只追加程序留痕 | post-interlock actual-next-free allocation |
| `g11.wave.4.exit` | G11.4 | `ci/g11_wave4_exit_check.py` | `milestones/g11/g11_wave4_exit_evidence_schema.json` | M153/M154 汇总：RFC-0028 语义面 spec-first 条款落地（RXS-0360 世界级登记翻转修订行在树）+ HDR 域 delta 收敛 + 屏幕级不冒充世界级 | post-interlock actual-next-free allocation |
| `g11.wave.5.exit` | G11.5 | `ci/g11_wave5_exit_check.py` | `milestones/g11/g11_wave5_exit_evidence_schema.json` | M155/M156 汇总：复测差距清单 11 行闭集逐项闭环（行集逐字对账）+ 契约 digest 门序留痕 + 既有 48 门零降级 | post-interlock actual-next-free allocation |
| `g11.wave.6.decisions` | G11.6 | `ci/g11_p2_decisions_check.py` | `milestones/g11/g11_p2_decisions_evidence_schema.json` | G11 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G12+ 零空行；defer 必有承接锚（机核同构 `ci/g10_p2_decisions_check.py`）；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述 | post-interlock actual-next-free allocation |
| `g11.wave.7a.soak` | G11.7a | `ci/g11_stabilization_soak.py` | `milestones/g11/g11_stabilization_soak_evidence_schema.json` | 全部 P0 与 go 的 P1 全量回归；G5~G10 既有判据 0-byte；修复链路（复测出图/度量/差距清单装配）连续复跑 soak（量级沿 G10.8a 继承〔≥1800s〕或 measured 证明更短足够）；`budget_eval --strict` 非空、零 estimated/skip | post-interlock actual-next-free allocation |
| `g11.wave.7b.closeout` | G11.7b | `ci/g11_closeout_check.py` | `milestones/g11/g11_wave7b_closeout_evidence_schema.json` | 验收映射、候选决策、RD 最终状态逐字一致；全部 P0 独立断言均 PASS；evidence/schema/预算终审；复测差距清单终审锁定（残余差距/未闭环行如实登记）；§8 只追加后 status active→closed 前置 | post-interlock actual-next-free allocation |

## 6. G11.2 互锁

`G11.GOV.G11_2.ENTRY_INTERLOCK` 条件与判据字面见 [G11_ACCEPTANCE_MAP.md](G11_ACCEPTANCE_MAP.md) §6（G10 closed + G11.0 不可变 ref `53eb3a28` 登记 + Full RFC-0028 经 D-409 评审 Agent Approved + 决策表/验收映射无缺行且 deferred history 只追加 + acceptance_coverage 与 measured_baseline 双 PASS + 数字步骤按互锁后 actual next_free 分配 + 用户 G11.2 开工指令留痕〔2026-08-15 指令全期授权面〕）。互锁未输出 READY 前：禁止合入 G11.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动；禁止 claim 任何数字 CI step；spec-first + RED 先行自互锁通过后才启动。`check_g11_implementation_interlock --require-ready` 输出 READY 是机器事实，不以叙述替代（契约 G-G11-3）。

## 7. Evidence 形态（沿 G9/G10 schema 范式）

- 每门 evidence 顶层至少含：`schema_version` / `subject` / `milestone`（`G11`）/ `wave` / `assertion_id`（必须等于对应 `symbolic_gate_key`）/ `status`（`pass|fail`；`skip|estimated|advisory` 不充绿）/ `commands` / `environment` / `base_commit` / `run_url` / `timestamp`（UTC）。
- 治理与聚合门形态：`symbolic_gate_key`（const 钉死）/ `host_section_pass`（boolean）/ `device_section_state`（enum：`not_applicable|executed|dev_env_degrade`）/ `checks`（键集闭集全 boolean，逐条打印不以总 `all_pass` 掩盖）/ 聚合门加 `required_gates`（闭集 minItems=maxItems）与 `aggregate_read_only const true`；`numeric_step` materialize 时 const 钉死实测真号。
- 修复闭环门（M144~M154）evidence 增闭环节字段闭集（materialize 时硬化）：`closure` = { `gap_row_id`（锁定清单行 id 字面）/ `baseline_delta`（锁定基线 delta，转引自 g10_gap_registry.json 0-byte）/ `retest_delta`（复测实测）/ `converged`（boolean，收敛判定）/ `threshold_provenance`（收敛阈值标定程序来源，禁手写）/ `contract_digest_unchanged`（boolean）}——收敛阈由标定程序 measured 产，evidence 登记溯源。
- evidence 落盘 `evidence/g11_<slug>_<UTC>.json` 新文件不覆盖既有件（只增不删不改）；文件名 UTC stamp 机核新鲜度。
- 条件语义：`SKIP=not-triggered` 只表示决策已记录，`DEV_ENV_DEGRADE` 只表示环境缺失，两者均不充 P0 绿、不反向否决其他可在当前环境全量验证的面（MAP §3）。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-16 | G11.1 初版：冻结治理/实现双门、13 个 P0 独立 key 与脚本（与 G11_CONTRACT §4.2 / G11_ACCEPTANCE_MAP §1 逐字一致——口径对齐闭环 M144~M146〔G11.2〕+ 资产场景修复闭环 M147~M152〔G11.3〕+ 光照 GI 修复闭环 M153/M154〔G11.4〕+ 复测与回归 M155/M156〔G11.5〕）+ 1 个已 go P1 key（§4A M157 hdr_flip_calibration，与 MAP §2 逐字一致）；修复闭环判据统一形态字面冻结（§4 头注：修复落盘 + delta 收敛 measured + 契约 digest 0-byte + 不降级 48 门；不设绝对画质通过线）；§5 波聚合门与收口门清单（wave2~wave5 exit + wave6 p2 决策 + wave7a soak + wave7b closeout）脚本名冻结——G11 无 defer 重评窗门如实登记（法定输入直消费，无独立重评窗波次）；`g11.gov.*` 五个 governance-only 机器门全不编号；三层 CI（PR Smoke/Nightly/Release）口径冻结；§7 evidence 形态沿 G9/G10 schema 范式 + 修复闭环节字段闭集（materialize 时硬化）；全部 numeric_step 延迟为 `post-interlock actual-next-free allocation`（当前实测 CI_step next_free=196，G10 已消费至 195）；零 workflow/script/schema 预放，当前实现门诚实 blocked。 |
