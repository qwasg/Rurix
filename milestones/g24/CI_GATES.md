<!-- Assisted-by: Cursor Agent（G24.1 治理波） -->
# G24 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（413–415）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 413 | g24.wave.1.acceptance_map | ci/g24_acceptance_map_check.py |
| 414 | g24.wave.1.candidate_decisions | ci/g24_candidate_decisions_check.py |
| 415 | g24.gov.implementation_interlock | ci/g24_interlock_check.py |

## 2. P0 五门（416–424，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 416 | g24.p0.m_a.hair_strand_oit_rejudgment | ci/g24_hair_strand_oit_rejudgment_smoke.py |
| 418 | g24.p0.m_b.hdr_calibration_rejudgment | ci/g24_hdr_calibration_rejudgment_smoke.py |
| 420 | g24.p0.m_c.bistro_exterior_conversion_rejudgment | ci/g24_bistro_exterior_conversion_rejudgment_smoke.py |
| 422 | g24.p0.m_d.safe_gpu_and_legacy_rd_disposition | ci/g24_safe_gpu_and_legacy_rd_disposition_smoke.py |
| 424 | g24.p0.m_e.closed_gate_no_regression | ci/g24_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（417–425 奇数位）

参数化脚本 `ci/g24_wave_exit_check.py`，gate key `g24.wave.{2..6}.exit`。

## 4. 收口三门（426–428）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 426 | g24.wave.5a.decisions | ci/g24_p2_decisions_check.py |
| 427 | g24.wave.5a.soak | ci/g24_stabilization_soak.py |
| 428 | g24.wave.6b.closeout | ci/g24_closeout_check.py |
