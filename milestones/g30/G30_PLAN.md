<!-- Assisted-by: Cursor Agent(G30.1 治理波) -->
# G30_PLAN — 商用终审收官期执行计划

> 事实源 = [G30_CONTRACT.md](G30_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G30 = 「G26-G30 五期串行战役」收官期（商用终审收官期，G25 收官期同构；用户战役指令字面「帮我一次性完成G26-G30」+ 原指令「要求完成商业化使用标准收尾」字面，2026-08-25）。上一期 G29 已 closed（tag g29-closed）。上游法定输入 = milestones/g25/g25_campaign_handover_registry.json（M125-adopt3/M127/M114-strand/M118-hdr-cal/G10-N6/SAFE-GPU/G17-MD-F1 七行）+ registry/deferred.json 八条（RD-034/039/040/041/042/043/044/045）+ milestones/g24/g24_legacy_rd_registry.json（历史清册）+ G26~G29 四期 P2 表（战役期承接锚）。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G30.1 | 治理四件套 + RFC-0047 + 对抗评审 + baseline 快检 + 治理三门 | 509/510/511 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G30.2 | M-a 尾锚重判闭集 + M-b 三面商用终审 | post-interlock 实测顺位（零数字预占） |
| G30.3 | M-c 战役全链零降级 + M-d 战役承接锚归档 | post-interlock 实测顺位（零数字预占） |
| G30.4 | M-e 旧门零降级（全量测试波） | post-interlock 实测顺位（零数字预占） |
| G30.5 | P2 穷举 + stabilization soak ≥1800s | post-interlock 实测顺位（零数字预占） |
| G30.6 | close-out 八 facts → status flip → tag g30-closed（战役收官） | post-interlock 实测顺位（零数字预占） |

波聚合门 `g30.wave.{2..6}.exit` 数字步骤 post-interlock 实测顺位领取（零数字预占）。

## 3. 实现面设计

- 尾锚重判闭集（M-a，纯核验）：六件外部条件类尾锚机器取证重判——M125-adopt3（Jolt 5.6 需求证据三类树内实测：5.6 独有 API 引用/5.3 缺陷命中/A/B 超带 + sys56 评估臂 cargo check 新鲜）/M127（corpus 目录 + PhysicsAsset residual 消费方检索）/M114-strand（毛发资产入压测闭集检索）/M118-hdr-cal（vulkaninfo HDR token 新鲜探针）/G10-N6（fbx2gltf/assimp/blender 三工具 PATH 实测 + 源资产检索）/SAFE-GPU（独立期资源窗 + 平台需求方文档检索）；RD-042/043/044 三条 G30 尾锚窗同批逐锚重判；各件 searched-paths manifest 必填，全未命中 → 逐件维持诚实终态（maintain-5.3/maintain 研究子轨/maintain card-mesh/maintain-SDR/双场景闭集维持/零交付维持）零冒充；deferred history 只追加；采纳切换实现（jolt_56_switch_implementation）与 HDR 显示链实现（hdr_display_chain_implementation）显式 out-of-scope——重判只核验条件。
- 三面商用终审（M-b，纯核验 + 焦点格新鲜单测）：画质面——画质表面闭集 0-byte 机核（vs g25-closed git-diff）+ 战役期加性面（framegen/hzb/restir/slab 四 kernel 与四 device bin）零接线核验 + G18 M-d 达标绿件只读盘点；性能面——G14 M-d 最新 18 格 evidence 如实定盘 + 性能面 0-byte 机核 + 焦点格新鲜单测真跑（bistro-interior/t100/dlss_sr canonical 160 帧 ratio 登记，G17-MD-F1 终判法定义务两态：≥1.00 → 18/18 或物理不可达维持 17/18 诚实红终判）；确定性面——Stage A 18 格 digest 锚在档 + 战役期四 device kernel 双跑位级绿件盘点（RD-045 累计观察复核承载）；三面终态如实定盘零冒充。
- 全链零降级 + 承接锚归档（M-c/M-d）：G29 受影响门 verify-latest 全绿（递归链自动涵盖 G26~G28 及更早）+ budget_eval --strict 全量零 skip 零 estimated；g30_campaign_handover_registry.json 全量汇总闭集登记（五期 defer/maintain 行 + RD 八条 G31+ 锚 + 历史清册引用 + 尾锚六件重判终态）——G31+ 唯一法定输入面，战役承接池 G30 后清零。
- 旧门零降级与收官（M-e/G30.6）：G29 受影响门 verify-latest 全绿零降级 + `g30_` 前缀不抢 latest；close-out 八 facts → status flip → tag g30-closed（五期串行战役收官）+ 战役总结报告归 close-out 签署块。

## 4. 编号纪律

治理三门步骤 509/510/511 落盘前实测领取（registry/number_ledger.json CI_step.next_free=509 顺位领取）；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（零数字预占）；RFC-0047 实测领取（RFC next_free=47，文件名 rfcs/0047-campaign-final-review.md）并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费。
