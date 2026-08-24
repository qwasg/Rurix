<!-- Assisted-by: Cursor Agent（G22.1 治理波） -->
# G22_PLAN — 材质/流送/时域期执行计划

> 事实源 = [G22_CONTRACT.md](G22_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G22 = 「G19-G25 七期串行战役」第四期。上游法定输入 = G21_P2_DECISIONS §1 defer-to-G22+ 六行 + RD-041。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G22.1 | 治理四件套 + RFC-0039 + 对抗评审 + baseline 快检 + 治理三门 | 381/382/383 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G22.2 | M-a slab 材质参考臂（material/slab.rs + g22_slab_probe）+ M-b SVT 处置 | 384/386（post-interlock 实测顺位） |
| G22.3 | M-c KTX2 处置 + M-d Work Graphs/FSR 重评 | 388/390 |
| G22.4 | M-e 旧门零降级（全量测试波） | 392 |
| G22.5 | P2 穷举 + stabilization soak ≥1800s | 394/395 |
| G22.6 | close-out 八 facts → status flip → tag g22-closed | 396 |

波聚合门 `g22.wave.{2..6}.exit` 步骤 385/387/389/391/393。

## 3. 实现面设计（M-a/M-d）

- `src/rurix-render/src/material/slab.rs`：双层 slab 解析闭式 `R = r_c + t_c²·a_b/(1−r_c·a_b)` + 级数+尾和恒等式 + 白炉审计网格。probe bin `g22_slab_probe`：128×128 参数网格白炉审计 + 双跑位级。
- Work Graphs 探针：`milestones/g22/harness/g22_work_graphs_probe.py` 真跑 vulkaninfo 落 AMDX shader_enqueue（预期 absent）与 DGC 三扩展（预期 present）取证。

## 4. 编号纪律

治理三门 381~383 落盘前实测领取；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（预期 384~396）；RFC-0039 实测领取并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费（RD-041 只追加 history）。
