<!-- Assisted-by: Cursor Agent(G26.1 治理波) -->
# G26 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（445–447）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 445 | g26.wave.1.acceptance_map | ci/g26_acceptance_map_check.py |
| 446 | g26.wave.1.candidate_decisions | ci/g26_candidate_decisions_check.py |
| 447 | g26.gov.implementation_interlock | ci/g26_interlock_check.py |

## 2. P0 五门（post-interlock 实测顺位领取后登记）

治理波零数字预占：G-G26-2 互锁绿后按 registry/number_ledger.json CI_step actual next_free 顺位领取数字步骤并回填本节；symbolic gate key 与稳定脚本以 G26_ACCEPTANCE_MAP §1 冻结字面为准（`g26.p0.m_<a~e>.<slug>` 五门）。

## 3. 波聚合门（post-interlock 实测顺位领取后登记）

参数化脚本 `ci/g26_wave_exit_check.py`，gate key `g26.wave.{2..6}.exit`；数字步骤 post-interlock 实测顺位领取后登记，治理波零数字预占。

## 4. 收口三门（post-interlock 实测顺位领取后登记）

gate key `g26.wave.5a.decisions`（`ci/g26_p2_decisions_check.py`）/ `g26.wave.5a.soak`（`ci/g26_stabilization_soak.py`）/ `g26.wave.6b.closeout`（`ci/g26_closeout_check.py`）；数字步骤 post-interlock 实测顺位领取后登记，治理波零数字预占。
