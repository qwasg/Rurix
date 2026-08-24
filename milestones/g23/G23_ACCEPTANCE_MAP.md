<!-- Assisted-by: Cursor Agent（G23.1 治理波） -->
# G23_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G23.1 治理交付物；事实源为 [G23_CONTRACT.md](G23_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 397/398/399。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g23.p0.m_a.jolt_56_adoption_rejudgment` | `py -3 ci/g23_jolt_56_adoption_rejudgment_smoke.py --gate g23.p0.m_a.jolt_56_adoption_rejudgment` | `milestones/g23/g23_m_a_jolt_56_adoption_rejudgment_evidence_schema.json` | M125-adopt3 重判兑现：5.6 评估臂在树核验（rurix-physics-sys56 + VENDOR56.md）+ g9_m125 A/B 最新绿件只读盘点 + 评估臂构建新鲜真跑（cargo check -p rurix-physics-sys56）+ 采纳三件成立条件核验（生产切换需求证据面）；maintain-5.3/adopt 均合法终态，登记 g23_jolt_adoption_registry.json | **G23.2** | post-interlock actual-next-free allocation |
| **M-b** | `g23.p0.m_b.neural_deform_rejudgment` | `py -3 ci/g23_neural_deform_rejudgment_smoke.py --gate g23.p0.m_b.neural_deform_rejudgment` | `milestones/g23/g23_m_b_neural_deform_rejudgment_evidence_schema.json` | M127 重判兑现：离线工具链 corpus 语料在树性实测 + PhysicsAsset residual 消费方存在性核验（两半分别登记）；maintain-研究子轨/go 均合法终态 | **G23.2** | post-interlock actual-next-free allocation |
| **M-c** | `g23.p0.m_c.research_track_disposition` | `py -3 ci/g23_research_track_disposition_smoke.py --gate g23.p0.m_c.research_track_disposition` | `milestones/g23/g23_m_c_research_track_disposition_evidence_schema.json` | RD-042/RD-043 观察轨处置闭集：Newton/Genesis/MuJoCo-Warp/wgrapier 逐轨 disposition 登记 g23_research_track_registry.json + 两条 RD history 只追加；观察存续/关闭均合法终态 | **G23.3** | post-interlock actual-next-free allocation |
| **M-d** | `g23.p0.m_d.physics_p3_subitem_disposition` | `py -3 ci/g23_physics_p3_subitem_disposition_smoke.py --gate g23.p0.m_d.physics_p3_subitem_disposition` | `milestones/g23/g23_m_d_physics_p3_subitem_disposition_evidence_schema.json` | RD-044 分项处置闭集：Jolt 软体/布料/流体、Taichi MPM、Rapier 快路径（M126 maintain-no-go 在案转引）三分项 disposition 登记 g23_rd044_subitem_registry.json + RD-044 history 只追加；go/no-go/defer 均合法终态 | **G23.3** | post-interlock actual-next-free allocation |
| **M-e** | `g23.p0.m_e.closed_gate_no_regression` | `py -3 ci/g23_closed_gate_no_regression_smoke.py --gate g23.p0.m_e.closed_gate_no_regression` | `milestones/g23/g23_m_e_closed_gate_no_regression_evidence_schema.json` | G22 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g23_` 前缀不抢 latest | **G23.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G23.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G22 defer-to-G23+ 六行本波重评：M125-adopt3 → go（M-a 重判承载）；M127 → go（M-b 重判承载）；SAFE-GPU/M114-strand/M118-hdr-cal/G10-N6 → defer-to-G24+（下期即窗，见 G23_CANDIDATE_DECISIONS §1）。

---

## 4. 双向一致声明

本表 §1 五行与 G23_CONTRACT.md §4.2 逐字相等；key 命名空间 `g23.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G23.1 治理覆盖

```text
g23.wave.1.acceptance_map         步骤 397
  py -3 ci/g23_acceptance_map_check.py --gate g23.wave.1.acceptance_map
g23.wave.1.candidate_decisions    步骤 398
  py -3 ci/g23_candidate_decisions_check.py --gate g23.wave.1.candidate_decisions
g23.gov.implementation_interlock  步骤 399
  py -3 ci/g23_interlock_check.py --gate g23.gov.implementation_interlock
```

---

## 6. G23.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g23_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G19-G25」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g23-closed`。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G23.1 初版：五 P0 行冻结。 |
