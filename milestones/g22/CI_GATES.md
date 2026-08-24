<!-- Assisted-by: Cursor Agent（G22.1 治理波） -->
# G22 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（381–383）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 381 | g22.wave.1.acceptance_map | ci/g22_acceptance_map_check.py |
| 382 | g22.wave.1.candidate_decisions | ci/g22_candidate_decisions_check.py |
| 383 | g22.gov.implementation_interlock | ci/g22_interlock_check.py |

## 2. P0 五门（384–392，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 384 | g22.p0.m_a.slab_material_host_realization | ci/g22_slab_material_host_realization_smoke.py |
| 386 | g22.p0.m_b.svt_disposition | ci/g22_svt_disposition_smoke.py |
| 388 | g22.p0.m_c.ktx2_basisu_disposition | ci/g22_ktx2_basisu_disposition_smoke.py |
| 390 | g22.p0.m_d.work_graphs_fsr_reeval_disposition | ci/g22_work_graphs_fsr_reeval_disposition_smoke.py |
| 392 | g22.p0.m_e.closed_gate_no_regression | ci/g22_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（385–393 奇数位）

参数化脚本 `ci/g22_wave_exit_check.py`，gate key `g22.wave.{2..6}.exit`。

## 4. 收口三门（394–396）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 394 | g22.wave.5a.decisions | ci/g22_p2_decisions_check.py |
| 395 | g22.wave.5a.soak | ci/g22_stabilization_soak.py |
| 396 | g22.wave.6b.closeout | ci/g22_closeout_check.py |
