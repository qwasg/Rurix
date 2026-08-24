<!-- Assisted-by: Cursor Agent（G21.1 治理波） -->
# G21_PLAN — 光照 P3+ 深化期执行计划

> 事实源 = [G21_CONTRACT.md](G21_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G21 = 「G19-G25 七期串行战役」第三期。上游法定输入 = G20_P2_DECISIONS §1 defer-to-G21+ 七行 + G18 M100-high closed-go 重判锚 + RD-040/RD-034。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G21.1 | 治理四件套 + RFC-0038 + 对抗评审 + baseline 快检 + 治理三门 | 365/366/367 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G21.2 | M-a ReSTIR 高档 reservoir（gi/restir_reservoir.rs + g21_restir_probe）+ M-b SER 两半实测重判 | 368/370（post-interlock 实测顺位） |
| G21.3 | M-c RD-040 五分项处置 + M-d RD-034 探针复查 | 372/374 |
| G21.4 | M-e 旧门零降级（全量测试波） | 376 |
| G21.5 | P2 穷举 + stabilization soak ≥1800s | 378/379 |
| G21.6 | close-out 八 facts → status flip → tag g21-closed | 380 |

波聚合门 `g21.wave.{2..6}.exit` 步骤 369/371/373/375/377。

## 3. 实现面设计（M-a/M-b）

- `src/rurix-render/src/gi/restir_reservoir.rs`：WRS 流式蓄水池（无偏权 W_y = w_sum/(p̂(y)·m)）+ 时域 merge（M-cap 截断）+ 64 灯环形夹具 + 方差对照实验（等验证预算：uniform 1 灯 MC vs RIS-16 候选 vs 时域 8 帧链）。probe bin `g21_restir_probe`：20k trial 无偏 3σ + 方差收益 measured + 双跑位级 digest。
- SER capability probe：`milestones/g21/harness/g21_ser_capability_probe.py` 真跑 vulkaninfo 落扩展枚举取证（VK_NV/EXT_ray_tracing_invocation_reorder + ReorderingHint）；workload 半边 = RT pipeline/SBT 宿主车道存在性核验（RD-040 分项 open ⇒ 未命中如实）。

## 4. 编号纪律

治理三门 365~367 落盘前实测领取；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（预期 368~380）；RFC-0038 实测领取并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费（RD-040/RD-034 只追加 history）。
