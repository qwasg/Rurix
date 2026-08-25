<!-- Assisted-by: Cursor Agent(G29.1 治理波) -->
# G29_PLAN — 材质 device 集成期执行计划

> 事实源 = [G29_CONTRACT.md](G29_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G29 = 「G26-G30 五期串行战役」第四期（材质 device 集成期；用户战役指令字面「帮我一次性完成G26-G30」，2026-08-25）。上一期 G28 已 closed（tag g28-closed）。上游法定输入 = milestones/g25/g25_campaign_handover_registry.json RD-041-slab/RD-041-svt-ktx2-wg 两行 + registry/deferred.json RD-041 + milestones/g22/g22_svt_gap.json(SVT 四行)+ milestones/g22/g22_ktx2_disposition.json(KTX2 三行)+ milestones/g22/g22_work_graphs_probe_results.json(WG absent/DGC available 实测)+ src/rurix-render/src/material/slab.rs(G22 host 参考臂,本期 0-byte 冻结面)。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G29.1 | 治理四件套 + RFC-0046 + 对抗评审 + baseline 快检 + 治理三门 | 493/494/495 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G29.2 | M-a slab device kernel 兑现 + M-b 侧表供参加性臂 | post-interlock 实测顺位（零数字预占） |
| G29.3 | M-c SVT/KTX2 差距重判 + M-d WG/DGC capability 复测 | post-interlock 实测顺位（零数字预占） |
| G29.4 | M-e 旧门零降级（全量测试波） | post-interlock 实测顺位（零数字预占） |
| G29.5 | P2 穷举 + stabilization soak ≥1800s | post-interlock 实测顺位（零数字预占） |
| G29.6 | close-out 八 facts → status flip → tag g29-closed（第四期收口） | post-interlock 实测顺位（零数字预占） |

波聚合门 `g29.wave.{2..6}.exit` 数字步骤 post-interlock 实测顺位领取（零数字预占）。

## 3. 实现面设计

- slab device kernel 车道：kernels/g29_slab.rx（rurixc --target vulkan 产 SPV + spirv-val 通过）经 vk::run_compute 派发；slab 能量守恒闭式 device 化——公式面与 host material/slab.rs 逐字同源（闭式反照率/白炉恒等/能量上界/lerp 连续）；device vs host 同输入逐样本对拍（16641 样本网格同 host 单测口径；p100 ≤ 标定容差，threshold = measured × 2.0 程序产禁手写；实测位级可达则登记零容差零条目）+ 白炉恒等 device 复现（dev 如实登记）+ device 双跑位级一致 + kernel-bias RED 臂；host 参考臂 material/slab.rs 0-byte 冻结；device 环境不可用时 SKIP 如实登记。
- 侧表供参加性臂：bin-local 多材质槽 slab 参数侧表（bin 内合成独立 SSBO，MaterialClosure 32B 与 reserved 拓扑位零触碰）；device kernel 逐槽消费侧表求值 + 与 host 逐槽对拍（p100 同 M-a 容差协议）+ 逐槽白炉恒等维持 + 双跑位级一致；graph/types.rs 0-byte 机核；单层 closure 生产面与生产集成显式 out-of-scope。
- SVT/KTX2 差距重判：SVT 四行（g22_svt_gap.json）+ KTX2 三行（g22_ktx2_disposition.json）逐行 reeval——各行现面实现痕迹树内实测（逐行检索清单 + 锚关键词映射入 evidence）；兑现 → 该行 closed-go；零实现 → 维持 defer 登记 milestones/g29/g29_svt_ktx2_rejudgment.json（g22 原表 0-byte 不回写）；RD-041 history 只追加；SVT/KTX2/WG 各行实现不入本期（重判窗只核验条件）。
- WG/DGC capability 复测：VK_AMDX_shader_enqueue 新鲜 vulkaninfo 复测（三态闭集：absent 维持 not-available/present 翻转复评启动/SKIP 如实登记）+ DGC 三扩展 available 复测互核 + FSR 3.1.5 maintain 盘点（vendor_upscale 面 0-byte）；not-available 维持/复评启动均合法诚实终态零冒充。

## 4. 编号纪律

治理三门步骤 493/494/495 落盘前实测领取（registry/number_ledger.json CI_step.next_free=493 顺位领取）；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（零数字预占）；RFC-0046 实测领取（RFC next_free=46，文件名 rfcs/0046-material-device-integration.md）并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费。
