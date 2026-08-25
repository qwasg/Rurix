<!-- Assisted-by: Cursor Agent(G28.1 治理波) -->
# G28 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（477–479）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 477 | g28.wave.1.acceptance_map | ci/g28_acceptance_map_check.py |
| 478 | g28.wave.1.candidate_decisions | ci/g28_candidate_decisions_check.py |
| 479 | g28.gov.implementation_interlock | ci/g28_interlock_check.py |

## 2. P0 五门（480–488，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 480 | g28.p0.m_a.restir_device_kernel | ci/g28_restir_device_kernel_smoke.py |
| 482 | g28.p0.m_b.restir_spatial_reuse_arm | ci/g28_restir_spatial_reuse_arm_smoke.py |
| 484 | g28.p0.m_c.m52_rd040_workload_rejudgment | ci/g28_m52_rd040_workload_rejudgment_smoke.py |
| 486 | g28.p0.m_d.rd034_upstream_recheck | ci/g28_rd034_upstream_recheck_smoke.py |
| 488 | g28.p0.m_e.closed_gate_no_regression | ci/g28_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（481–489 奇数位）

参数化脚本 `ci/g28_wave_exit_check.py`，gate key `g28.wave.{2..6}.exit`。

## 4. 收口三门（490–492）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 490 | g28.wave.5a.decisions | ci/g28_p2_decisions_check.py |
| 491 | g28.wave.5a.soak | ci/g28_stabilization_soak.py |
| 492 | g28.wave.6b.closeout | ci/g28_closeout_check.py |
