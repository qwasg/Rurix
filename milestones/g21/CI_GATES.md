<!-- Assisted-by: Cursor Agent（G21.1 治理波） -->
# G21 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（365–367）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 365 | g21.wave.1.acceptance_map | ci/g21_acceptance_map_check.py |
| 366 | g21.wave.1.candidate_decisions | ci/g21_candidate_decisions_check.py |
| 367 | g21.gov.implementation_interlock | ci/g21_interlock_check.py |

## 2. P0 五门（368–376，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 368 | g21.p0.m_a.restir_high_reservoir_realization | ci/g21_restir_high_reservoir_realization_smoke.py |
| 370 | g21.p0.m_b.ser_capability_disposition | ci/g21_ser_capability_disposition_smoke.py |
| 372 | g21.p0.m_c.rd040_subitem_disposition | ci/g21_rd040_subitem_disposition_smoke.py |
| 374 | g21.p0.m_d.rd034_upstream_recheck | ci/g21_rd034_upstream_recheck_smoke.py |
| 376 | g21.p0.m_e.closed_gate_no_regression | ci/g21_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（369–377 奇数位）

参数化脚本 `ci/g21_wave_exit_check.py`，gate key `g21.wave.{2..6}.exit`。

## 4. 收口三门（378–380）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 378 | g21.wave.5a.decisions | ci/g21_p2_decisions_check.py |
| 379 | g21.wave.5a.soak | ci/g21_stabilization_soak.py |
| 380 | g21.wave.6b.closeout | ci/g21_closeout_check.py |
