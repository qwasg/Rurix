<!-- Assisted-by: Cursor Agent（G20.1 治理波） -->
# G20 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（349–351）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 349 | g20.wave.1.acceptance_map | ci/g20_acceptance_map_check.py |
| 350 | g20.wave.1.candidate_decisions | ci/g20_candidate_decisions_check.py |
| 351 | g20.gov.implementation_interlock | ci/g20_interlock_check.py |

## 2. P0 五门（352–360，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 352 | g20.p0.m_a.hzb_occlusion_host_realization | ci/g20_hzb_occlusion_host_realization_smoke.py |
| 354 | g20.p0.m_b.cluster_streaming_p4_disposition | ci/g20_cluster_streaming_p4_disposition_smoke.py |
| 356 | g20.p0.m_c.mesh_shader_rejudgment | ci/g20_mesh_shader_rejudgment_smoke.py |
| 358 | g20.p0.m_d.far_field_l4_disposition | ci/g20_far_field_l4_disposition_smoke.py |
| 360 | g20.p0.m_e.closed_gate_no_regression | ci/g20_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（353–361 奇数位）

参数化脚本 `ci/g20_wave_exit_check.py`，gate key `g20.wave.{2..6}.exit`。

## 4. 收口三门（362–364）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 362 | g20.wave.5a.decisions | ci/g20_p2_decisions_check.py |
| 363 | g20.wave.5a.soak | ci/g20_stabilization_soak.py |
| 364 | g20.wave.6b.closeout | ci/g20_closeout_check.py |
