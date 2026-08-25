<!-- Assisted-by: Cursor Agent(G27.1 治理波) -->
# G27_PLAN — 几何 device 化期执行计划

> 事实源 = [G27_CONTRACT.md](G27_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G27 = 「G26-G30 五期串行战役」第二期（几何 device 化期；用户战役指令字面「帮我一次性完成G26-G30」，2026-08-25）。上一期 G26 已 closed（tag g26-closed）。上游法定输入 = milestones/g25/g25_campaign_handover_registry.json M61/M98-l4 行 + registry/deferred.json RD-039 + milestones/g20/g20_cluster_streaming_p4_gap.json(四行差距闭集,本期只读重判不回写)+ src/rurix-render/src/geometry/hzb.rs(G20 host 参考臂,本期 0-byte 冻结面)。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G27.1 | 治理四件套 + RFC-0044 + 对抗评审 + baseline 快检 + 治理三门 | 461/462/463 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G27.2 | M-a HZB device kernel 兑现 + M-b M61 重判窗 | post-interlock 实测顺位（零数字预占） |
| G27.3 | M-c cluster P4 差距闭集重判 + M-d M98-l4 重判窗 | post-interlock 实测顺位（零数字预占） |
| G27.4 | M-e 旧门零降级（全量测试波） | post-interlock 实测顺位（零数字预占） |
| G27.5 | P2 穷举 + stabilization soak ≥1800s | post-interlock 实测顺位（零数字预占） |
| G27.6 | close-out 八 facts → status flip → tag g27-closed（第二期收口） | post-interlock 实测顺位（零数字预占） |

波聚合门 `g27.wave.{2..6}.exit` 数字步骤 post-interlock 实测顺位领取（零数字预占）。

## 3. 实现面设计

- HZB device kernel 车道：kernels/g27_hzb_reduce.rx + g27_hzb_test.rx（rurixc --target vulkan 产 SPV + spirv-val 通过）经 vk::run_compute 派发；金字塔逐级 farther-of 归约 device 化（mips 与 host HzbPyramid::build 逐级位级相等）+ rect 测试 device 化（mip 选择/≤2×2 窗/is_farther 判定与 host test_rect 逐字同源，800 rect × 双约定判定序列与 host 全等）+ 零假阳性硬不变量（device 判 Occluded ⇒ exact_rect_occluded 同判）+ device 双跑位级一致 + 篡改 RED 臂；host 参考臂 geometry/hzb.rs 0-byte 冻结；device 环境不可用时 SKIP 如实登记。
- M61 重判窗：RFC-0034 只追加程序——HZB device 化半边（M-a 绿件只读盘点）+ cluster P4 差距闭集清零半边（g20_cluster_streaming_p4_gap.json 四行 open 状态实测）+ mesh shader HW 性能差 measured 证据树内搜索（searched-paths manifest 必填）；条件未全齐 maintain-no-go 只追加再判记录，全齐重判程序启动；mesh shader HW 管线实现不入本期（重判窗只核验条件）。
- cluster P4 差距重判：四行（P4-1~P4-4）逐行 reeval——P4-2 依赖面（HZB device 化）本期解除事实登记 + 各行现面零实现树内实测（streaming/ 模块 cluster 载荷面检索）；清零 closed-go，未清零维持 open 登记 milestones/g27/g27_cluster_p4_rejudgment.json（g20 差距表原文 0-byte 不回写）；RD-039 history 只追加。
- M98-l4 重判窗：两半树内实测——HLOD proxy 追踪 device 腿（src 检索零实现登记）+ L4 计数器接入（gi/fallback_chain.rs L4 槽位恒零/fail-closed 入口实测 + world/hlod.rs 接口面就绪盘点）；任一半命中重判程序启动，均未命中维持 L1/L2/L3 三级链诚实登记；HLOD proxy device 腿实现不入本期（重判窗只核验条件）。

## 4. 编号纪律

治理三门步骤 461/462/463 落盘前实测领取（registry/number_ledger.json CI_step.next_free=461 顺位领取）；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（零数字预占）；RFC-0044 实测领取（RFC next_free=44，文件名 rfcs/0044-geometry-device-realization.md）并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费。
