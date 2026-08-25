<!-- Assisted-by: Cursor Agent(G26.1 治理波) -->
# G26_PLAN — 时域/帧生成 device 化期执行计划

> 事实源 = [G26_CONTRACT.md](G26_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G26 = 「G26-G30 五期串行战役」第一期（时域/帧生成 device 化期；用户战役指令字面「帮我一次性完成G26-G30」，2026-08-25）。上一期 G25 已 closed（tag g25-closed）。上游法定输入 = milestones/g25/g25_campaign_handover_registry.json(G26+ 唯一法定输入面)+ registry/deferred.json RD-045 条目 + src/rurix-render/src/temporal/framegen.rs(G19 host 参考臂,本期 0-byte 冻结面)。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G26.1 | 治理四件套 + RFC-0043 + 对抗评审 + baseline 快检 + 治理三门 | 445/446/447 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G26.2 | M-a FG/MFG device kernel 兑现 + M-b device 帧时与口径登记 | post-interlock 实测顺位（零数字预占） |
| G26.3 | M-c RD-045 backfill 重判 + M-d G17-MD-F1 重判窗 | post-interlock 实测顺位（零数字预占） |
| G26.4 | M-e 旧门零降级（全量测试波） | post-interlock 实测顺位（零数字预占） |
| G26.5 | P2 穷举 + stabilization soak ≥1800s | post-interlock 实测顺位（零数字预占） |
| G26.6 | close-out 八 facts → status flip → tag g26-closed（第一期收口） | post-interlock 实测顺位（零数字预占） |

波聚合门 `g26.wave.{2..6}.exit` 数字步骤 post-interlock 实测顺位领取（零数字预占）。

## 3. 实现面设计

- device kernel 车道：kernels/g26_framegen.rx（rurixc --target vulkan 产 SPV + spirv-val 通过）经 vk::run_compute 派发；device vs host 金标准（temporal/framegen.rs）同输入逐帧对拍，×2/×3/×4 三档合成运动场景；标定容差程序产（threshold = measured × 2.0 冻结 k，标定腿两跑位级一致，禁手写）+ SSIM(interp)>SSIM(frame-hold) 程序产对照继承 + device 双跑位级一致 + kernel-bias RED 臂；device 环境不可用时 SKIP 如实登记。
- device 帧时口径：device 全链路（打包+dispatch+回读）warmup+timed 逐帧墙钟登记（回归守护语义，不构成帧率对标通过线，生成帧禁计入真实渲染帧率）+ FgAccounting 真渲/presented 两口径类型面分离核验 + 性能面 g14_3_pipeline_perf 0-byte 机核 vs g25-closed。
- RD-045 重判：新鲜观察窗真跑（焦点车道 canonical 双跑 digest 轨迹多轮零漂移登记）+ backfill 三件逐项机器盘点（根因定位/生产化修复/Full RFC 评估——树内证据闭集实测）——全齐 close/未齐 maintain-open 只追加扩窗；deferred history 只追加。
- G17-MD-F1 重判窗：NGX 分解 profiling 与 UE 侧插桩两半树内闭集搜索实测（evidence/ 检索面登记）——任一命中启动重判程序，两半均未命中维持 17/18 诚实红 carry（终判归 G30 商用终审）。

## 4. 编号纪律

治理三门步骤 445/446/447 落盘前实测领取（registry/number_ledger.json CI_step.next_free=445 顺位领取）；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（零数字预占）；RFC-0043 实测领取（RFC next_free=43，文件名 rfcs/0043-framegen-device-kernel-realization.md）并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费。
