<!-- Assisted-by: Cursor Agent（G24.1 治理波） -->
# G24_PLAN — 呈现与尾门清理期执行计划

> 事实源 = [G24_CONTRACT.md](G24_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G24 = 「G19-G25 七期串行战役」第六期。上游法定输入 = G23_P2_DECISIONS §1 defer-to-G24+ 四行（G18 承接池最后四行——本期清零）+ 历史 open RD 十一条清册。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G24.1 | 治理四件套 + RFC-0041 + 对抗评审 + baseline 快检 + 治理三门 | 413/414/415 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G24.2 | M-a 毛发 OIT 重判 + M-b HDR 重判 | 416/418（post-interlock 实测顺位） |
| G24.3 | M-c BistroExterior 复查 + M-d SAFE-GPU/历史 RD 清册 | 420/422 |
| G24.4 | M-e 旧门零降级（全量测试波） | 424 |
| G24.5 | P2 穷举 + stabilization soak ≥1800s | 426/427 |
| G24.6 | close-out 八 facts → status flip → tag g24-closed | 428 |

波聚合门 `g24.wave.{2..6}.exit` 步骤 417/419/421/423/425。

## 3. 实现面设计

- HDR 探针：`milestones/g24/harness/g24_hdr_probe.py` 真跑 vulkaninfo 落表面色彩空间枚举（HDR10_ST2084/BT2020 等 token 取证）。
- 毛发 OIT：M120 七算法 benchmark 绿件只读盘点（禁 --gate 重跑）+ 压测闭集毛发资产存在性核验。
- BistroExterior：FBX2glTF/替代臂工具链在树性 + 源资产在树性实测，登记 g24_bistro_exterior_recheck.json。
- 历史 RD 清册：十一条逐条重判闭集 g24_legacy_rd_registry.json + registry history 逐条只追加。

## 4. 编号纪律

治理三门 413~415 落盘前实测领取；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（预期 416~428）；RFC-0041 实测领取并登记 rfcs/README.md §5；共享 D/U/SG 段零消费（历史 RD 只追加 history）。
