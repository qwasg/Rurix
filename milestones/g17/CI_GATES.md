<!-- Assisted-by: Cursor Claude Fable 5（G17.1 治理波） -->
# G17 CI_GATES — 门清单汇总视图

> **性质**：G17.1 治理交付物（契约四件套之一）；本文件是**汇总视图**，门冻结面 = [G17_CONTRACT.md](G17_CONTRACT.md) §4.2 + [G17_ACCEPTANCE_MAP.md](G17_ACCEPTANCE_MAP.md) §1/§2（双向逐字一致由 `ci/g17_acceptance_map_check.py` 机器强制；本文件不作为第三比对源，冲突时以契约为准）。
> **编号纪律**：治理三门步骤 293/294/295 = 落盘前实测 `CI_step.next_free=293` 顺位领取（契约立项裁决 4）；P0 实现门与波门 numeric_step 一律 post-interlock actual-next-free allocation。

## 1. 治理三门（G17.1 本波 materialize，真脚本真步骤）

| 门 | symbolic gate key | 命令 | 步骤 |
|---|---|---|---|
| 验收映射核验 | `g17.wave.1.acceptance_map` | `py -3 ci/g17_acceptance_map_check.py --gate g17.wave.1.acceptance_map` | 293 |
| 候选决策核验 | `g17.wave.1.candidate_decisions` | `py -3 ci/g17_candidate_decisions_check.py --gate g17.wave.1.candidate_decisions` | 294 |
| 实现互锁 | `g17.gov.implementation_interlock` | `py -3 ci/g17_interlock_check.py --gate g17.gov.implementation_interlock`（`--require-ready` 供 G17.2 开工前置） | 295 |

## 2. P0 实现门（五行，numeric_step post-interlock 领取）

| M 行 | symbolic gate key | 命令 | 波次 |
|---|---|---|---|
| M-a | `g17.p0.m_a.dual_end_retest_warm_recalib` | `py -3 ci/g17_dual_end_retest_warm_recalib_smoke.py --gate g17.p0.m_a.dual_end_retest_warm_recalib` | G17.2 |
| M-b | `g17.p0.m_b.ngx_evolution_alignment` | `py -3 ci/g17_ngx_evolution_alignment_smoke.py --gate g17.p0.m_b.ngx_evolution_alignment` | G17.3 |
| M-c | `g17.p0.m_c.d3d12_host_lane_disposition` | `py -3 ci/g17_d3d12_host_lane_disposition_smoke.py --gate g17.p0.m_c.d3d12_host_lane_disposition` | G17.4 |
| M-d | `g17.p0.m_d.t100_final_verdict` | `py -3 ci/g17_t100_final_verdict_smoke.py --gate g17.p0.m_d.t100_final_verdict` | G17.5 |
| M-e | `g17.p0.m_e.closed_gate_no_regression` | `py -3 ci/g17_closed_gate_no_regression_smoke.py --gate g17.p0.m_e.closed_gate_no_regression` | G17.6 |

判据逐字见契约 §4.2（唯一事实源）。每门独立 evidence subject / 独立布尔断言；`--selftest` 红绿自检（构造缺陷→红→复原→绿）；`--verify-latest` 供旧门复核。

## 3. 波门 / 决策门 / soak / close-out（numeric_step post-interlock 领取）

| 门 | symbolic gate key | 命令 |
|---|---|---|
| 波聚合（每波） | `g17.wave.{N}.exit` | `py -3 ci/g17_wave{N}_exit_check.py --gate g17.wave.{N}.exit`（只读汇总不代绿，红树下聚合不充绿） |
| P2 穷举决策 | `g17.wave.7a.decisions` | `py -3 ci/g17_p2_decisions_check.py --gate g17.wave.7a.decisions` |
| 稳定 soak | `g17.wave.7a.soak` | `py -3 ci/g17_stabilization_soak.py --gate g17.wave.7a.soak`（≥1800s 零失败） |
| close-out 终审 | `g17.wave.7b.closeout` | `py -3 ci/g17_closeout_check.py --gate g17.wave.7b.closeout`（八 facts VERDICT=READY） |

## 4. 守卫套件（每波验收全跑，全部仓库根目录）

```bash
py -3 ci/check_structure.py          # 目录结构（blocking）
py -3 ci/check_schemas.py            # 注册表/预算/证据 schema（blocking）
py -3 ci/check_number_ledger.py      # 编号台账（blocking）
py -3 ci/check_guardrails.py         # 字节级只追加核对（advisory）
py -3 ci/budget_eval.py              # 预算门禁（close-out 加 --strict）
py -3 -m pytest tests/ -q            # harness 统计单测
```

## 5. 旧门零降级面（M-e 消费，禁 `--gate` 只许 `--verify-latest`）

G13/G14/G15/G16 受影响门（closeout / wave exit / M-d 帧率门 / M-e 回归门等）`--verify-latest` 全绿；`g17_` 前缀 evidence 不抢旧门 latest。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版（G17.1 治理波定稿；治理三门 293/294/295 + P0 五门 + 波门/决策/soak/close-out 汇总视图）。 |
