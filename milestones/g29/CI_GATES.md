<!-- Assisted-by: Cursor Agent(G29.1 治理波) -->
# G29 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（493–495）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 493 | g29.wave.1.acceptance_map | ci/g29_acceptance_map_check.py |
| 494 | g29.wave.1.candidate_decisions | ci/g29_candidate_decisions_check.py |
| 495 | g29.gov.implementation_interlock | ci/g29_interlock_check.py |

## 2. P0 五门（496–504，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 496 | g29.p0.m_a.slab_device_kernel | ci/g29_slab_device_kernel_smoke.py |
| 498 | g29.p0.m_b.slab_side_table_arm | ci/g29_slab_side_table_arm_smoke.py |
| 500 | g29.p0.m_c.svt_ktx2_gap_rejudgment | ci/g29_svt_ktx2_gap_rejudgment_smoke.py |
| 502 | g29.p0.m_d.wg_dgc_capability_recheck | ci/g29_wg_dgc_capability_recheck_smoke.py |
| 504 | g29.p0.m_e.closed_gate_no_regression | ci/g29_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（497–505 奇数位）

参数化脚本 `ci/g29_wave_exit_check.py`，gate key `g29.wave.{2..6}.exit`。

## 4. 收口三门（506–508）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 506 | g29.wave.5a.decisions | ci/g29_p2_decisions_check.py |
| 507 | g29.wave.5a.soak | ci/g29_stabilization_soak.py |
| 508 | g29.wave.6b.closeout | ci/g29_closeout_check.py |
