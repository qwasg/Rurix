<!-- Assisted-by: Cursor Agent（G23.1 治理波） -->
# G23 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（397–399）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 397 | g23.wave.1.acceptance_map | ci/g23_acceptance_map_check.py |
| 398 | g23.wave.1.candidate_decisions | ci/g23_candidate_decisions_check.py |
| 399 | g23.gov.implementation_interlock | ci/g23_interlock_check.py |

## 2. P0 五门（400–408，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 400 | g23.p0.m_a.jolt_56_adoption_rejudgment | ci/g23_jolt_56_adoption_rejudgment_smoke.py |
| 402 | g23.p0.m_b.neural_deform_rejudgment | ci/g23_neural_deform_rejudgment_smoke.py |
| 404 | g23.p0.m_c.research_track_disposition | ci/g23_research_track_disposition_smoke.py |
| 406 | g23.p0.m_d.physics_p3_subitem_disposition | ci/g23_physics_p3_subitem_disposition_smoke.py |
| 408 | g23.p0.m_e.closed_gate_no_regression | ci/g23_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（401–409 奇数位）

参数化脚本 `ci/g23_wave_exit_check.py`，gate key `g23.wave.{2..6}.exit`。

## 4. 收口三门（410–412）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 410 | g23.wave.5a.decisions | ci/g23_p2_decisions_check.py |
| 411 | g23.wave.5a.soak | ci/g23_stabilization_soak.py |
| 412 | g23.wave.6b.closeout | ci/g23_closeout_check.py |
