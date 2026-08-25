<!-- Assisted-by: Cursor Agent(G27.1 治理波) -->
# G27 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（461–463）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 461 | g27.wave.1.acceptance_map | ci/g27_acceptance_map_check.py |
| 462 | g27.wave.1.candidate_decisions | ci/g27_candidate_decisions_check.py |
| 463 | g27.gov.implementation_interlock | ci/g27_interlock_check.py |

## 2. P0 五门（post-interlock 实测顺位领取后登记）

数字步骤零预占：五 P0 门 gate key `g27.p0.m_<a~e>.<slug>` 与脚本 `ci/g27_<slug>_smoke.py`（见 G27_ACCEPTANCE_MAP §1）待互锁解锁后 post-interlock 实测顺位领取后登记。

## 3. 波聚合门（post-interlock 实测顺位领取后登记）

参数化脚本 `ci/g27_wave_exit_check.py`，gate key `g27.wave.{2..6}.exit`；数字步骤 post-interlock 实测顺位领取后登记。

## 4. 收口三门（post-interlock 实测顺位领取后登记）

`g27.wave.5a.decisions`（ci/g27_p2_decisions_check.py）+ `g27.wave.5a.soak`（ci/g27_stabilization_soak.py）+ `g27.wave.6b.closeout`（ci/g27_closeout_check.py）；数字步骤 post-interlock 实测顺位领取后登记。
