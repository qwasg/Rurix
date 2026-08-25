<!-- Assisted-by: Cursor Agent(G30.1 治理波) -->
# G30 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（509–511）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 509 | g30.wave.1.acceptance_map | ci/g30_acceptance_map_check.py |
| 510 | g30.wave.1.candidate_decisions | ci/g30_candidate_decisions_check.py |
| 511 | g30.gov.implementation_interlock | ci/g30_interlock_check.py |

## 2. P0 五门（占位——post-interlock 实测顺位，零数字预占）

symbolic gate key 与脚本名已由 G30_ACCEPTANCE_MAP §1 冻结；数字步骤 G30.2 互锁绿后按 actual next_free 顺位领取回填。

| 步骤 | gate key | 脚本 |
|---|---|---|
| 占位 | g30.p0.m_a.tail_anchor_rejudgment_closure | ci/g30_tail_anchor_rejudgment_closure_smoke.py |
| 占位 | g30.p0.m_b.commercial_final_review | ci/g30_commercial_final_review_smoke.py |
| 占位 | g30.p0.m_c.campaign_full_chain_no_regression | ci/g30_campaign_full_chain_no_regression_smoke.py |
| 占位 | g30.p0.m_d.campaign_handover_ledger | ci/g30_campaign_handover_ledger_smoke.py |
| 占位 | g30.p0.m_e.closed_gate_no_regression | ci/g30_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（占位——post-interlock 实测顺位，零数字预占）

参数化脚本 `ci/g30_wave_exit_check.py`，gate key `g30.wave.{2..6}.exit`；数字步骤随各波退出实测顺位回填。

## 4. 收口三门（占位——post-interlock 实测顺位，零数字预占）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 占位 | g30.wave.5a.decisions | ci/g30_p2_decisions_check.py |
| 占位 | g30.wave.5a.soak | ci/g30_stabilization_soak.py |
| 占位 | g30.wave.6b.closeout | ci/g30_closeout_check.py |
