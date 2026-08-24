<!-- Assisted-by: Cursor Agent（G19.1 治理波） -->
# G19 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（333–335）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 333 | g19.wave.1.acceptance_map | ci/g19_acceptance_map_check.py |
| 334 | g19.wave.1.candidate_decisions | ci/g19_candidate_decisions_check.py |
| 335 | g19.gov.implementation_interlock | ci/g19_interlock_check.py |

## 2. P0 五门（336–344，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 336 | g19.p0.m_a.frame_generation_host_realization | ci/g19_frame_generation_host_realization_smoke.py |
| 338 | g19.p0.m_b.frame_generation_vendor_disposition | ci/g19_frame_generation_vendor_disposition_smoke.py |
| 340 | g19.p0.m_c.rd045_drift_observation_window | ci/g19_rd045_drift_observation_window_smoke.py |
| 342 | g19.p0.m_d.fps_parity_window_registration | ci/g19_fps_parity_window_registration_smoke.py |
| 344 | g19.p0.m_e.closed_gate_no_regression | ci/g19_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（337–345 奇数位）

参数化脚本 `ci/g19_wave_exit_check.py`，gate key `g19.wave.{2..6}.exit`。

## 4. 收口三门（346–348）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 346 | g19.wave.5a.decisions | ci/g19_p2_decisions_check.py |
| 347 | g19.wave.5a.soak | ci/g19_stabilization_soak.py |
| 348 | g19.wave.6b.closeout | ci/g19_closeout_check.py |
