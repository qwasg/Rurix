<!-- Assisted-by: Cursor Agent（G20.1 治理波） -->
# G20_PLAN — 虚拟化几何 P4 期执行计划

> 事实源 = [G20_CONTRACT.md](G20_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G20 = 「G19-G25 七期串行战役」第二期。上游法定输入 = G19_P2_DECISIONS §1 defer-to-G20+ 八行 + G18_P2_DECISIONS §1 M61 no-go 重判锚（触发条件 = G19+ HZB/cluster P4 齐备——本期兑现 HZB 半边）+ RD-039。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G20.1 | 治理四件套 + RFC-0037 + 对抗评审 + baseline 快检 + 治理三门 | 349/350/351 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G20.2 | M-a HZB host 参考臂（geometry/hzb.rs + g20_hzb_probe）+ M-b cluster P4 评估 | 352/354（post-interlock 实测顺位） |
| G20.3 | M-c M61 重判（RFC-0034 只追加）+ M-d M98-l4 重判 | 356/358 |
| G20.4 | M-e 旧门零降级（全量测试波） | 360 |
| G20.5 | P2 穷举 + stabilization soak ≥1800s | 362/363 |
| G20.6 | close-out 八 facts → status flip → tag g20-closed | 364 |

波聚合门 `g20.wave.{2..6}.exit` 步骤 353/355/357/359/361。

## 3. 实现面设计（M-a）

`src/rurix-render/src/geometry/hzb.rs`：HZB 金字塔（每级纹素 = footprint 内最远深度，reverse-Z 取 min / standard-Z 取 max，非 2 幂 ceil 减半 + 越界 clamp）+ `test_rect` 保守遮挡测试（rect 跨度 ≤2 纹素选级 + ≤2×2 窗最远归约）+ `exact_rect_occluded` 逐像素精确真值（测试金标准）。probe bin `g20_hzb_probe`：确定性合成深度场 + 400 确定性 rect 夹具——零假阳性硬不变量 + 剔除率非零 + 双跑位级 digest。

## 4. 编号纪律

治理三门 349~351 落盘前实测领取；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（预期 352~364）；RFC-0037 实测领取并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费（RD-039/RFC-0034 只追加 history/重判记录）。
