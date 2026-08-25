<!-- Assisted-by: Cursor Agent(G28.1 治理波) -->
# G28_CANDIDATE_DECISIONS — G28.1 候选决策表（v1.0 2026-08-25）

> **状态**：G28.1 治理波定稿。**候选闭集 8 行零空行** = §1 三行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G29+` / `strategic_override`。**G28 非收官期：defer 合法值 = defer-to-G29+**。

## 1. G25 交接登记表行 = 3 行逐行转引裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M100-high | ReSTIR 高档 reservoir device 化窗 | G21.2 M-a closed-go → G25 M-d 归档 → G28.1 go | device 化/空间重用/M100 车道集成窗（RFC-0038 out-of-scope 锚）→ 低档 MegaLights 默认档维持 | go | M-a/M-b 承载(device kernel + 空间重用两面兑现,M100 车道集成显式 out-of-scope) | milestones/g21/G21_P2_DECISIONS.md §1 M100-high 行 + milestones/g25/g25_campaign_handover_registry.json | 重判条件 = M-a/M-b 真跑后争议时只追加程序重判;兜底 = 低档 MegaLights 默认档维持(multi_light 0-byte)+ M100 车道集成归后续窗 | 本表 §1 + G28_ACCEPTANCE_MAP M-a/M-b | go(M-a/M-b 承载) |
| M52 | SER workload 重判窗 | G21.2 M-b maintain-defer → G25 M-d 归档 → G28.1 go | RT pipeline/SBT 宿主车道出现（RD-040 分项 RT-PIPELINE-SBT reeval_anchor）→ 语言层不加 SER 原语维持 | go | M-c 承载两半盘点(capability available 在案 + workload 检索) | milestones/g21/G21_P2_DECISIONS.md §1 M52 行 + milestones/g21/g21_ser_capability_probe_results.json | 重判条件 = 两半全齐方改判;兜底 = maintain-defer + 语言层不加 SER 原语字面 0-byte | 本表 §1 + G28_ACCEPTANCE_MAP M-c | go(M-c 承载) |
| RD-034 | spirv-cross RT 消费路径上游复查窗 | G21.3 M-d 维持 blocked → G25 M-d 归档 rd_eight → G28.1 go | spirv-cross SPV_KHR_ray_tracing 消费路径或 LLVM A 路解锁（G21.3 探针复查在案）→ 维持 blocked | go | M-d 承载探针新鲜复查 | registry/deferred.json RD-034 + ci/meshrt_probe_smoke.py | 重判条件 = 探针意外成功(上游获得消费能力)时复评启动;兜底 = 维持 blocked + 探针恒跑防静默腐烂 | 本表 §1 + G28_ACCEPTANCE_MAP M-d | go(M-d 承载) |

## 2. open RD 逐条映射（八条口径）

| RD | title(摘要) | 条目级 status | G28.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | M-d 承载(上游探针新鲜复查) | M-d | G25 归档锚在案(spirv-cross RT 消费路径/LLVM A 路解锁窗),本期 M-d 真跑探针新鲜复查 + history 只追加 | 本表 §2 + G28_ACCEPTANCE_MAP M-d + registry/deferred.json RD-034 history |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open(G29+/G30 各期承接窗在案) | 无 | G27 重判在案(M-a HZB device 化 implemented + M-c cluster P4 四行维持 open),lighting device 范围外 | 本表 §2 + registry/deferred.json RD-039 |
| RD-040 | 光照 P3+ | open | M-c 承载(M52 两半盘点 + 五分项逐锚重判) | M-c | G25 归档锚在案(五分项 reeval_anchor + ReSTIR device 化窗),本期 M-a/M-b 兑现 ReSTIR device 化窗 + M-c 五分项逐锚重判 history 只追加 | 本表 §2 + G28_ACCEPTANCE_MAP M-c + registry/deferred.json RD-040 history |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open(G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(SVT/KTX2/WG reeval_anchor + slab device 集成窗),lighting device 范围外 | 本表 §2 + registry/deferred.json RD-041 |
| RD-042 | 可微物理研究轨 | open | 维持 open(G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(可微仿真需求场景未出现),lighting device 范围外 | 本表 §2 + registry/deferred.json RD-042 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open(G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(out_of_scope 翻转程序 + wgrapier 成熟度证据未出现),lighting device 范围外 | 本表 §2 + registry/deferred.json RD-043 |
| RD-044 | 物理 P3+ | open | 维持 open(G29+/G30 各期承接窗在案) | 无 | G25 归档锚在案(三分项 reeval_anchor),lighting device 范围外 | 本表 §2 + registry/deferred.json RD-044 |
| RD-045 | digest 漂移修复 | open | 维持 open(G29+/G30 各期承接窗在案) | 无 | G26 M-c maintain-open 重判在案(新鲜窗 6/6 零漂移 + 三件盘点 0/3 只追加扩窗),lighting device 范围外 | 本表 §2 + registry/deferred.json RD-045 |

## 3. G28 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G28-N1 | ReSTIR device kernel 实现车道 | G28.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G28_CONTRACT §4.2 M-a | 重判条件 = 逐 trial 对拍容差面或无偏 3σ 面争议时只追加程序重判;兜底 = host 参考臂 gi/restir_reservoir.rs 0-byte 维持 + device 不可用 SKIP 如实登记 | G28_ACCEPTANCE_MAP M-a | go(M-a) |
| G28-N2 | 空间重用加性臂 | G28.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G28_CONTRACT §4.2 M-b | 重判条件 = 空间合并无偏 3σ 面或方差再收益 measured 对照面争议时只追加程序重判;兜底 = M100 低档 MegaLights 生产默认面(gi/multi_light.rs)0-byte 维持 + 收益值如实登记不设通过线 | G28_ACCEPTANCE_MAP M-b | go(M-b) |
| G28-N3 | M52/RD-040 盘点面 | G28.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G28_CONTRACT §4.2 M-c | 重判条件 = 两半盘点或五分项逐锚实测面争议时只追加程序重判;兜底 = maintain-defer 只追加 + 五分项维持 defer + RD-040 history 只追加 | G28_ACCEPTANCE_MAP M-c | go(M-c) |
| G28-N4 | RD-034 探针复查面 | G28.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G28_CONTRACT §4.2 M-d | 重判条件 = 探针意外成功或 history 核验面争议时只追加程序重判;兜底 = 维持 blocked 诚实登记 + 探针恒跑防静默腐烂 | G28_ACCEPTANCE_MAP M-d | go(M-d) |
| G28-N5 | G27 链回归守护与 soak 探针轮换扩容至七车道 | G28.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据 + G28.5 soak 承载 | G28_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合或 soak 探针轮换集合争议时只追加程序扩表;兜底 = verify-latest 纪律维持 | G28_ACCEPTANCE_MAP M-e | go(M-e/soak 承载) |

## 4. 汇总

go §1 三行 + §3 五行 = 8 go 承载面；零 defer 行（G28 非收官期，defer 合法值 = defer-to-G29+，本波零消费）；§2 open RD 八条维持 open（RD-040 → M-c 承载；RD-034 → M-d 承载）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 首版：8 行候选闭集。 |
