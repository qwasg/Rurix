<!-- Assisted-by: Cursor Agent（G23.1 治理波） -->
# G23_PLAN — 物理平台深化期执行计划

> 事实源 = [G23_CONTRACT.md](G23_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G23 = 「G19-G25 七期串行战役」第五期。上游法定输入 = G22_P2_DECISIONS §1 defer-to-G23+ 六行 + RD-042/043/044。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G23.1 | 治理四件套 + RFC-0040 + 对抗评审 + baseline 快检 + 治理三门 | 397/398/399 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G23.2 | M-a Jolt 5.6 采纳臂重判 + M-b 神经变形重判 | 400/402（post-interlock 实测顺位） |
| G23.3 | M-c 研究轨处置 + M-d 物理 P3+ 分项处置 | 404/406 |
| G23.4 | M-e 旧门零降级（全量测试波） | 408 |
| G23.5 | P2 穷举 + stabilization soak ≥1800s | 410/411 |
| G23.6 | close-out 八 facts → status flip → tag g23-closed | 412 |

波聚合门 `g23.wave.{2..6}.exit` 步骤 401/403/405/407/409。

## 3. 实现面设计（M-a/M-b）

- Jolt 采纳臂重判机器取证：rurix-physics-sys56 crate 在树核验 + VENDOR56.md provenance + g9_m125 A/B 最新绿件只读盘点（禁 --gate 重跑旧门）+ `cargo check -p rurix-physics-sys56` 构建新鲜真跑 + 采纳三件（升格裁决/生产切换/5.3 退役程序）成立条件核验——生产切换需求证据缺 ⇒ maintain-5.3；登记 g23_jolt_adoption_registry.json。
- 神经变形两半实测：corpus 语料树内搜索（离线工具链语料面）+ PhysicsAsset residual 消费方存在性核验（代码面搜索）——两半未命中 ⇒ maintain 研究子轨。

## 4. 编号纪律

治理三门 397~399 落盘前实测领取；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（预期 400~412）；RFC-0040 实测领取并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费（RD-042/043/044 只追加 history）。
