<!-- Assisted-by: Cursor Agent(G26.1 治理波) -->
# G26_CANDIDATE_DECISIONS — G26.1 候选决策表（v1.0 2026-08-25）

> **状态**：G26.1 治理波定稿。**候选闭集 8 行零空行** = §1 三行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G27+` / `strategic_override`。**G26 非收官期：defer 合法值 = defer-to-G27+**。

## 1. G25 交接登记表行 = 3 行逐行转引裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G13-N7 | FG/MFG device kernel 车道 | G19.2 M-a closed-go → G25 M-d 归档 G26 锚 → G26.1 go | device kernel 车道重判(RFC-0036 §1.5)→ host 参考臂 0-byte 维持 | go | G26 即 device 化兑现窗,M-a/M-b 承载 | milestones/g25/g25_campaign_handover_registry.json G13-N7 行 | 重判条件 = M-a 真跑后争议时只追加程序重判;兜底 = host 参考臂维持 + defer-to-G27+ 归档锚 | 本表 §1 + G26_ACCEPTANCE_MAP M-a | go(M-a/M-b 承载) |
| RD-045-window | RD-045 backfill 三件重判窗 | G19.3 M-c maintain-open → G25 M-d 归档 → G26.1 go | backfill 三件(定位+修复+Full RFC 评估)→ 累计观察零漂移维持 | go | M-c 承载新鲜观察窗与三件盘点 | registry/deferred.json RD-045 + milestones/g25/g25_campaign_handover_registry.json | 重判条件 = 新鲜观察窗真跑后三件逐项盘点;兜底 = maintain-open 只追加扩窗零冒充 | 本表 §1 + G26_ACCEPTANCE_MAP M-c | go(M-c 承载) |
| G17-MD-F1 | fps 17/18 诚实红重判窗 | G25.2 M-b 终判 17/18 → G25 M-d 归档 → G26.1 go | NGX 分解 profiling 或 UE 侧插桩(宿主差可分离 measured 证据)→ 未命中维持 | go | M-d 承载两半条件核验 | milestones/g25/g25_campaign_handover_registry.json G17-MD-F1 行 | 重判条件 = 两半任一命中时重判程序启动;兜底 = 17/18 诚实红 carry 终判归 G30 | 本表 §1 + G26_ACCEPTANCE_MAP M-d | go(M-d 承载) |

## 2. open RD 逐条映射（八条口径）

| RD | title(摘要) | 条目级 status | G26.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open(G27+/G28+/G29+ 各期承接窗在案) | 无 | G25 归档锚在案(spirv-cross RT 消费路径/LLVM A 路解锁窗),framegen device 范围外 | 本表 §2 + registry/deferred.json RD-034 |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open(G27+/G28+/G29+ 各期承接窗在案) | 无 | G25 归档锚在案(HZB device 化 + cluster P4 差距闭集 + mesh shader 再判链),framegen device 范围外 | 本表 §2 + registry/deferred.json RD-039 |
| RD-040 | 光照 P3+ | open | 维持 open(G27+/G28+/G29+ 各期承接窗在案) | 无 | G25 归档锚在案(五分项 reeval_anchor + ReSTIR device 化窗),framegen device 范围外 | 本表 §2 + registry/deferred.json RD-040 |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open(G27+/G28+/G29+ 各期承接窗在案) | 无 | G25 归档锚在案(SVT/KTX2/WG reeval_anchor + slab device 集成窗),framegen device 范围外 | 本表 §2 + registry/deferred.json RD-041 |
| RD-042 | 可微物理研究轨 | open | 维持 open(G27+/G28+/G29+ 各期承接窗在案) | 无 | G25 归档锚在案(可微仿真需求场景未出现),framegen device 范围外 | 本表 §2 + registry/deferred.json RD-042 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open(G27+/G28+/G29+ 各期承接窗在案) | 无 | G25 归档锚在案(out_of_scope 翻转程序 + wgrapier 成熟度证据未出现),framegen device 范围外 | 本表 §2 + registry/deferred.json RD-043 |
| RD-044 | 物理 P3+ | open | 维持 open(G27+/G28+/G29+ 各期承接窗在案) | 无 | G25 归档锚在案(三分项 reeval_anchor),framegen device 范围外 | 本表 §2 + registry/deferred.json RD-044 |
| RD-045 | digest 漂移修复 | open | M-c 重判窗承载(新鲜观察窗 + 三件盘点) | 无 | G19.3 12/12 + 六期 soak 零漂移累计,三件条件待 M-c 树内实测盘点 | 本表 §2 + G26_ACCEPTANCE_MAP M-c + registry/deferred.json RD-045 history |

## 3. G26 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G26-N1 | framegen device kernel 实现车道 | G26.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G26_CONTRACT §4.2 M-a | 重判条件 = device vs host 对拍或容差标定争议时只追加程序重判;兜底 = host 参考臂 temporal/framegen.rs 0-byte 维持 + device 不可用 SKIP 如实登记 | G26_ACCEPTANCE_MAP M-a | go(M-a) |
| G26-N2 | device 帧时与口径登记面 | G26.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G26_CONTRACT §4.2 M-b | 重判条件 = 帧时口径或 FgAccounting 分离面争议时只追加程序重判;兜底 = 回归守护语义维持(不构成帧率对标通过线)+ 性能面 0-byte 机核 | G26_ACCEPTANCE_MAP M-b | go(M-b) |
| G26-N3 | RD-045 新鲜观察窗协议 | G26.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G26_CONTRACT §4.2 M-c | 重判条件 = 新鲜观察窗真跑后三件盘点争议时只追加程序重判;兜底 = maintain-open 只追加扩窗零冒充 | G26_ACCEPTANCE_MAP M-c | go(M-c) |
| G26-N4 | G17-MD-F1 证据搜索面闭集 | G26.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G26_CONTRACT §4.2 M-d | 重判条件 = 两半证据搜索面争议时只追加程序扩面;兜底 = 17/18 诚实红 carry 维持(终判归 G30) | G26_ACCEPTANCE_MAP M-d | go(M-d) |
| G26-N5 | G25 链回归守护与 soak 探针轮换扩容 | G26.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据 + G26.5 soak 承载 | G26_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合或 soak 探针轮换集合争议时只追加程序扩表;兜底 = verify-latest 纪律维持 | G26_ACCEPTANCE_MAP M-e | go(M-e/soak 承载) |

## 4. 汇总

go §1 三行 + §3 五行 = 8 go 承载面；零 defer 行（G26 非收官期，defer 合法值 = defer-to-G27+，本波零消费）；§2 open RD 八条维持 open（RD-045 → M-c 重判窗承载）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 首版：8 行候选闭集。 |
