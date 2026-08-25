<!-- Assisted-by: Cursor Agent(G27.1 治理波) -->
# G27 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（461–463）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 461 | g27.wave.1.acceptance_map | ci/g27_acceptance_map_check.py |
| 462 | g27.wave.1.candidate_decisions | ci/g27_candidate_decisions_check.py |
| 463 | g27.gov.implementation_interlock | ci/g27_interlock_check.py |

## 2. P0 五门（464–472，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 464 | g27.p0.m_a.hzb_device_kernel | ci/g27_hzb_device_kernel_smoke.py |
| 466 | g27.p0.m_b.m61_mesh_shader_rejudgment | ci/g27_m61_mesh_shader_rejudgment_smoke.py |
| 468 | g27.p0.m_c.cluster_p4_gap_rejudgment | ci/g27_cluster_p4_gap_rejudgment_smoke.py |
| 470 | g27.p0.m_d.hlod_l4_counter_rejudgment | ci/g27_hlod_l4_counter_rejudgment_smoke.py |
| 472 | g27.p0.m_e.closed_gate_no_regression | ci/g27_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（465–473 奇数位）

参数化脚本 `ci/g27_wave_exit_check.py`，gate key `g27.wave.{2..6}.exit`。

## 4. 收口三门（474–476）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 474 | g27.wave.5a.decisions | ci/g27_p2_decisions_check.py |
| 475 | g27.wave.5a.soak | ci/g27_stabilization_soak.py |
| 476 | g27.wave.6b.closeout | ci/g27_closeout_check.py |
