<!-- Assisted-by: Cursor Agent（G25.1 治理波） -->
# G25_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G25.1 治理交付物；事实源为 [G25_CONTRACT.md](G25_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 429/430/431。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g25.p0.m_a.quality_final_state_verification` | `py -3 ci/g25_quality_final_state_verification_smoke.py --gate g25.p0.m_a.quality_final_state_verification` | `milestones/g25/g25_m_a_quality_final_state_verification_evidence_schema.json` | 画质终态维持核验：G18 M-d 商用画质终审达标绿件只读盘点 + 战役期画质表面 0-byte 机核（presentation/显示链/默认渲染臂 vs g18-closed git-diff 闭集）+ G19~G24 加性面零接线核验；维持达标终态/降级检出均如实登记 | **G25.2** | post-interlock actual-next-free allocation |
| **M-b** | `g25.p0.m_b.fps_parity_final_verdict` | `py -3 ci/g25_fps_parity_final_verdict_smoke.py --gate g25.p0.m_b.fps_parity_final_verdict` | `milestones/g25/g25_m_b_fps_parity_final_verdict_evidence_schema.json` | fps 18 格终判兑现：G14 M-d 最新 18 格 evidence 如实定盘 + 性能面 0-byte 机核（g14_3_pipeline_perf 全战役 0-byte）+ 焦点格新鲜单测真跑（bistro-interior/t100/dlss_sr canonical 160 帧 bench 一轮 ratio 登记）；≥1.00 → 18/18 或物理不可达维持 **17/18 诚实红终判**（两态均为战役合法收官态，G15 兜底同源） | **G25.2** | post-interlock actual-next-free allocation |
| **M-c** | `g25.p0.m_c.campaign_full_chain_no_regression` | `py -3 ci/g25_campaign_full_chain_no_regression_smoke.py --gate g25.p0.m_c.campaign_full_chain_no_regression` | `milestones/g25/g25_m_c_campaign_full_chain_no_regression_evidence_schema.json` | 战役全链零降级：G24 受影响门 `--verify-latest` 全绿（递归链自动涵盖 G13~G23）+ budget_eval --strict 全量零 skip 零 estimated；禁 `--gate` 旧脚本 | **G25.3** | post-interlock actual-next-free allocation |
| **M-d** | `g25.p0.m_d.campaign_handover_ledger` | `py -3 ci/g25_campaign_handover_ledger_smoke.py --gate g25.p0.m_d.campaign_handover_ledger` | `milestones/g25/g25_m_d_campaign_handover_ledger_evidence_schema.json` | 战役承接锚归档闭集：g25_campaign_handover_registry.json（七期 defer/maintain 行 + RD 八条 + 历史清册十一条 + SAFE-GPU 处置 + RD-045 累计观察复核）全量汇总闭集登记——G26+ 法定输入面；归档完整性机核 | **G25.3** | post-interlock actual-next-free allocation |
| **M-e** | `g25.p0.m_e.closed_gate_no_regression` | `py -3 ci/g25_closed_gate_no_regression_smoke.py --gate g25.p0.m_e.closed_gate_no_regression` | `milestones/g25/g25_m_e_closed_gate_no_regression_evidence_schema.json` | G24 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g25_` 前缀不抢 latest | **G25.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G25.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G24 defer-to-G25+ 行（SAFE-GPU → M-d 归档承载）+ fps 终判锚（G17-MD-F1 链 → M-b 终判承载）；战役承接池 G25 后清零，G26+ 法定输入 = M-d 归档闭集。

---

## 4. 双向一致声明

本表 §1 五行与 G25_CONTRACT.md §4.2 逐字相等；key 命名空间 `g25.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G25.1 治理覆盖

```text
g25.wave.1.acceptance_map         步骤 429
  py -3 ci/g25_acceptance_map_check.py --gate g25.wave.1.acceptance_map
g25.wave.1.candidate_decisions    步骤 430
  py -3 ci/g25_candidate_decisions_check.py --gate g25.wave.1.candidate_decisions
g25.gov.implementation_interlock  步骤 431
  py -3 ci/g25_interlock_check.py --gate g25.gov.implementation_interlock
```

---

## 6. G25.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g25_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G19-G25」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g25-closed`（战役收官）。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G25.1 初版：五 P0 行冻结。 |
