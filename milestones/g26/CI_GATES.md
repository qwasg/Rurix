<!-- Assisted-by: Cursor Agent(G26.1 治理波) -->
# G26 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（445–447）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 445 | g26.wave.1.acceptance_map | ci/g26_acceptance_map_check.py |
| 446 | g26.wave.1.candidate_decisions | ci/g26_candidate_decisions_check.py |
| 447 | g26.gov.implementation_interlock | ci/g26_interlock_check.py |

## 2. P0 五门（448–456，post-interlock 实测顺位）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 448 | g26.p0.m_a.framegen_device_kernel | ci/g26_framegen_device_kernel_smoke.py |
| 450 | g26.p0.m_b.framegen_device_bench_accounting | ci/g26_framegen_device_bench_accounting_smoke.py |
| 452 | g26.p0.m_c.rd045_backfill_rejudgment | ci/g26_rd045_backfill_rejudgment_smoke.py |
| 454 | g26.p0.m_d.g17_md_f1_rejudgment_window | ci/g26_g17_md_f1_rejudgment_window_smoke.py |
| 456 | g26.p0.m_e.closed_gate_no_regression | ci/g26_closed_gate_no_regression_smoke.py |

## 3. 波聚合门（449–457 奇数位）

参数化脚本 `ci/g26_wave_exit_check.py`，gate key `g26.wave.{2..6}.exit`。

## 4. 收口三门（458–460）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 458 | g26.wave.5a.decisions | ci/g26_p2_decisions_check.py |
| 459 | g26.wave.5a.soak | ci/g26_stabilization_soak.py |
| 460 | g26.wave.6b.closeout | ci/g26_closeout_check.py |
