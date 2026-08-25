<!-- Assisted-by: Cursor Agent(G27.1 治理波) -->
# G27_CANDIDATE_DECISIONS — G27.1 候选决策表（v1.0 2026-08-25）

> **状态**：G27.1 治理波定稿。**候选闭集 8 行零空行** = §1 三行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G28+` / `strategic_override`。**G27 非收官期：defer 合法值 = defer-to-G28+**。

## 1. G25 交接登记表行 = 3 行逐行转引裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M61 | mesh shader 第三光栅路径重判窗 | G20.3 M-c maintain-no-go → G25 M-d 归档 → G27.1 go | cluster P4 差距闭集清零 + HZB device 化落地后只追加再判 → VS 光栅唯一 fallback 维持 | go | M-b 承载两半盘点(HZB device 半边本期 M-a 兑现) | milestones/g20/G20_P2_DECISIONS.md §1 M61 行 + rfcs/0034 重判表 | 重判条件 = M-b 盘点后条件全齐时重判程序启动;兜底 = maintain-no-go + VS 光栅唯一 fallback 维持 | 本表 §1 + G27_ACCEPTANCE_MAP M-b | go(M-b 承载) |
| M98-l4 | L4 Far Field 档重判窗 | G20.4 M-d maintain-no-go → G25 M-d 归档 → G27.1 go | HLOD proxy 追踪 device 腿落地 + L4 计数器接入选档 evidence → L1/L2/L3 三级链维持 | go | M-d 承载两半条件核验 | milestones/g20/G20_P2_DECISIONS.md §1 M98-l4 行 + src/rurix-render/src/gi/fallback_chain.rs L4 fail-closed 入口 | 重判条件 = 两半任一命中时重判程序启动;兜底 = 三级链维持 + L4 槽位恒零如实登记 | 本表 §1 + G27_ACCEPTANCE_MAP M-d | go(M-d 承载) |
| RD-039-mesh | HZB device 化 + cluster P4 + mesh 再判链(RD-039 分项) | G25 M-d 归档 rd_eight → G27.1 go | HZB device 化 + cluster P4 差距闭集(G20 落档)+ mesh shader 再判链 → 长线评估维持 open | go | M-a 承载 HZB device 化兑现 + M-c 承载 P4 四行重判 | milestones/g25/g25_campaign_handover_registry.json rd_eight RD-039 行 + milestones/g20/g20_cluster_streaming_p4_gap.json | 重判条件 = M-a/M-c 真跑后争议时只追加程序重判;兜底 = RD-039 维持 open + 差距表原文 0-byte | 本表 §1 + G27_ACCEPTANCE_MAP M-a/M-c | go(M-a/M-c 承载) |

## 2. open RD 逐条映射（八条口径）

| RD | title(摘要) | 条目级 status | G27.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open(G28+/G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(spirv-cross RT 消费路径/LLVM A 路解锁窗),geometry device 范围外 | 本表 §2 + registry/deferred.json RD-034 |
| RD-039 | 虚拟化几何 P3+ | open | M-a/M-c 承载(HZB device 化兑现 + P4 四行重判) | M-a/M-c | G25 归档锚在案(HZB device 化 + cluster P4 差距闭集 + mesh shader 再判链),本期 M-a 兑现 HZB device 半边 + M-c 四行 reeval | 本表 §2 + G27_ACCEPTANCE_MAP M-a/M-c + registry/deferred.json RD-039 history |
| RD-040 | 光照 P3+ | open | 维持 open(G28+/G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(五分项 reeval_anchor + ReSTIR device 化窗),geometry device 范围外 | 本表 §2 + registry/deferred.json RD-040 |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open(G28+/G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(SVT/KTX2/WG reeval_anchor + slab device 集成窗),geometry device 范围外 | 本表 §2 + registry/deferred.json RD-041 |
| RD-042 | 可微物理研究轨 | open | 维持 open(G28+/G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(可微仿真需求场景未出现),geometry device 范围外 | 本表 §2 + registry/deferred.json RD-042 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open(G28+/G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(out_of_scope 翻转程序 + wgrapier 成熟度证据未出现),geometry device 范围外 | 本表 §2 + registry/deferred.json RD-043 |
| RD-044 | 物理 P3+ | open | 维持 open(G28+/G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(三分项 reeval_anchor),geometry device 范围外 | 本表 §2 + registry/deferred.json RD-044 |
| RD-045 | digest 漂移修复 | open | 维持 open(G28+/G29+/G30 各期承接窗在案) | 无 | G26 M-c maintain-open 重判在案(新鲜窗 6/6 零漂移 + 三件盘点 0/3 只追加扩窗),geometry device 范围外 | 本表 §2 + registry/deferred.json RD-045 |

## 3. G27 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G27-N1 | HZB device kernel 实现车道 | G27.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G27_CONTRACT §4.2 M-a | 重判条件 = mips 逐级位级对拍或 rect 判定序列全等面争议时只追加程序重判;兜底 = host 参考臂 geometry/hzb.rs 0-byte 维持 + device 不可用 SKIP 如实登记 | G27_ACCEPTANCE_MAP M-a | go(M-a) |
| G27-N2 | M61 两半盘点面 | G27.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G27_CONTRACT §4.2 M-b | 重判条件 = 两半盘点或 searched-paths manifest 搜索面争议时只追加程序重判;兜底 = maintain-no-go 只追加再判记录 + VS 光栅唯一 fallback 维持 | G27_ACCEPTANCE_MAP M-b | go(M-b) |
| G27-N3 | cluster P4 四行 reeval 协议 | G27.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G27_CONTRACT §4.2 M-c | 重判条件 = 四行 reeval 或 P4-2 依赖面解除登记争议时只追加程序重判;兜底 = 维持 open 登记 g27_cluster_p4_rejudgment.json + g20 差距表原文 0-byte 不回写 | G27_ACCEPTANCE_MAP M-c | go(M-c) |
| G27-N4 | M98-l4 条件核验面 | G27.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G27_CONTRACT §4.2 M-d | 重判条件 = 两半树内实测面争议时只追加程序重判;兜底 = L1/L2/L3 三级链诚实登记维持 + L4 槽位恒零如实登记 | G27_ACCEPTANCE_MAP M-d | go(M-d) |
| G27-N5 | G26 链回归守护与 soak 探针轮换扩容至六车道 | G27.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据 + G27.5 soak 承载 | G27_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合或 soak 探针轮换集合争议时只追加程序扩表;兜底 = verify-latest 纪律维持 | G27_ACCEPTANCE_MAP M-e | go(M-e/soak 承载) |

## 4. 汇总

go §1 三行 + §3 五行 = 8 go 承载面；零 defer 行（G27 非收官期，defer 合法值 = defer-to-G28+，本波零消费）；§2 open RD 八条维持 open（RD-039 → M-a/M-c 承载）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 首版：8 行候选闭集。 |
