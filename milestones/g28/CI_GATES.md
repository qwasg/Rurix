<!-- Assisted-by: Cursor Agent(G28.1 治理波) -->
# G28 CI_GATES — 里程碑冒烟门登记

## 1. 治理三门（477–479）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 477 | g28.wave.1.acceptance_map | ci/g28_acceptance_map_check.py |
| 478 | g28.wave.1.candidate_decisions | ci/g28_candidate_decisions_check.py |
| 479 | g28.gov.implementation_interlock | ci/g28_interlock_check.py |

## 2. P0 五门（post-interlock 实测顺位领取后登记）

<!-- 占位：M-a~M-e 五 P0 门（gate key = g28.p0.m_<a~e>.<slug>，脚本 = ci/g28_<slug>_smoke.py）数字步骤 post-interlock 实测顺位领取后登记（零数字预占）。 -->

## 3. 波聚合门（post-interlock 实测顺位领取后登记）

<!-- 占位：参数化脚本 ci/g28_wave_exit_check.py，gate key = g28.wave.{2..6}.exit；数字步骤 post-interlock 实测顺位领取后登记（零数字预占）。 -->

## 4. 收口三门（post-interlock 实测顺位领取后登记）

<!-- 占位：g28.wave.5a.decisions / g28.wave.5a.soak / g28.wave.6b.closeout；数字步骤 post-interlock 实测顺位领取后登记（零数字预占）。 -->
