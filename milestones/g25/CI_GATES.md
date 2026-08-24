<!-- Assisted-by: Cursor Agent（G25.1 治理波） -->
# G25 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（429–431）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 429 | g25.wave.1.acceptance_map | ci/g25_acceptance_map_check.py |
| 430 | g25.wave.1.candidate_decisions | ci/g25_candidate_decisions_check.py |
| 431 | g25.gov.implementation_interlock | ci/g25_interlock_check.py |

## 2. P0 五门（432–440，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 432 | g25.p0.m_a.quality_final_state_verification | ci/g25_quality_final_state_verification_smoke.py |
| 434 | g25.p0.m_b.fps_parity_final_verdict | ci/g25_fps_parity_final_verdict_smoke.py |
| 436 | g25.p0.m_c.campaign_full_chain_no_regression | ci/g25_campaign_full_chain_no_regression_smoke.py |
| 438 | g25.p0.m_d.campaign_handover_ledger | ci/g25_campaign_handover_ledger_smoke.py |
| 440 | g25.p0.m_e.closed_gate_no_regression | ci/g25_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（433–441 奇数位）

参数化脚本 `ci/g25_wave_exit_check.py`，gate key `g25.wave.{2..6}.exit`。

## 4. 收口三门（442–444）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 442 | g25.wave.5a.decisions | ci/g25_p2_decisions_check.py |
| 443 | g25.wave.5a.soak | ci/g25_stabilization_soak.py |
| 444 | g25.wave.6b.closeout | ci/g25_closeout_check.py |
