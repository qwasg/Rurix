<!-- Assisted-by: Cursor Agent(G28.1 治理波) -->
# G28_PLAN — 光照 device 化期执行计划

> 事实源 = [G28_CONTRACT.md](G28_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G28 = 「G26-G30 五期串行战役」第三期（光照 device 化期；用户战役指令字面「帮我一次性完成G26-G30」，2026-08-25）。上一期 G27 已 closed（tag g27-closed）。上游法定输入 = milestones/g25/g25_campaign_handover_registry.json M100-high/M52 行 + registry/deferred.json RD-034/RD-040 + milestones/g21/g21_rd040_subitem_registry.json(五分项 reeval_anchor)+ src/rurix-render/src/gi/restir_reservoir.rs(G21 host 参考臂,本期 0-byte 冻结面)。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G28.1 | 治理四件套 + RFC-0045 + 对抗评审 + baseline 快检 + 治理三门 | 477/478/479 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G28.2 | M-a ReSTIR device kernel 兑现 + M-b 空间重用加性臂 | post-interlock 实测顺位（零数字预占） |
| G28.3 | M-c M52/RD-040 workload 重判 + M-d RD-034 上游复查 | post-interlock 实测顺位（零数字预占） |
| G28.4 | M-e 旧门零降级（全量测试波） | post-interlock 实测顺位（零数字预占） |
| G28.5 | P2 穷举 + stabilization soak ≥1800s | post-interlock 实测顺位（零数字预占） |
| G28.6 | close-out 八 facts → status flip → tag g28-closed（第三期收口） | post-interlock 实测顺位（零数字预占） |

波聚合门 `g28.wave.{2..6}.exit` 数字步骤 post-interlock 实测顺位领取（零数字预占）。

## 3. 实现面设计

- ReSTIR device kernel 车道：kernels/g28_restir.rx（rurixc --target vulkan 产 SPV + spirv-val 通过）经 vk::run_compute 派发；WRS/RIS reservoir 更新链 device 化——候选流与均匀随机数由 host 单源预生成上传，device 不重生成 RNG（PCG32 u64 状态面留 host；逐 trial 单 invocation 顺序 WRS 链保浮点序）；device vs host 金标准（gi/restir_reservoir.rs estimate_ris）同输入逐 trial 对拍（p100 ≤ 标定容差，threshold = measured × 2.0 冻结 k 程序产禁手写；实测位级可达则登记零容差）+ 无偏 3σ 维持 + device 双跑位级一致 + kernel-bias RED 臂；host 参考臂 gi/restir_reservoir.rs 0-byte 冻结；device 环境不可用时 SKIP 如实登记。
- 空间重用加性臂：bin-local 多着色点网格邻域 reservoir 合并（Reservoir::merge 语义同构 m_cap 截断，时域/空间同律）；无偏 3σ 维持（空间合并不引入偏差，等验证预算 measured 对照）+ 空间合并方差再收益 measured 登记（程序产对照，收益值如实登记不设通过线）+ 双跑位级一致；M100 低档 MegaLights 生产默认面（gi/multi_light.rs）0-byte 机核。
- M52/RD-040 workload 重判：M52 两半盘点——capability 半边（G21 vulkaninfo 三 token available 取证只读盘点 + 新鲜 vulkaninfo 复测）+ workload 半边（RT pipeline/SBT 宿主车道树内检索，searched-paths manifest 必填）——两半全齐方改判，未全齐 maintain-defer 只追加；RD-040 五分项逐锚重判（g21_rd040_subitem_registry.json 五分项 reeval_anchor 树内实测逐项登记），全未命中维持 defer；RD-040 history 只追加；RT pipeline/SBT 宿主车道实现不入本期（重判窗只核验条件）。
- RD-034 上游复查：真跑 ci/meshrt_probe_smoke.py（spirv-cross 拒 raygen 探针新鲜——非零退出 = blocked 证据新鲜；意外成功翻红提醒复评）+ deferred.json RD-034 status/history 核验（G28.3 行只追加）；解锁/维持 blocked 均合法诚实终态零冒充。

## 4. 编号纪律

治理三门步骤 477/478/479 落盘前实测领取（registry/number_ledger.json CI_step.next_free=477 顺位领取）；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（零数字预占）；RFC-0045 实测领取（RFC next_free=45，文件名 rfcs/0045-lighting-device-realization.md）并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费。
