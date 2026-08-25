<!-- Assisted-by: Cursor Agent(G29.1 治理波) -->
# G29_CANDIDATE_DECISIONS — G29.1 候选决策表（v1.0 2026-08-25）

> **状态**：G29.1 治理波定稿。**候选闭集 7 行零空行** = §1 两行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G30+` / `strategic_override`。**G29 非收官期：defer 合法值 = defer-to-G30+**。

## 1. G25 交接登记表行 = 2 行逐行转引裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-041-slab | slab device kernel/侧表集成波 | G22.2 M-a closed-go → G25 M-d 归档 → G29.1 go | device kernel/侧表集成波(RFC-0039 out-of-scope)→ closure 单层生产面 0-byte 维持 | go | M-a/M-b 承载(device kernel + 侧表供参两面兑现,生产集成显式 out-of-scope) | milestones/g25/g25_campaign_handover_registry.json RD-041-slab 行 + milestones/g22/G22_P2_DECISIONS.md §3 | 重判条件 = M-a/M-b 真跑后争议时只追加程序重判;兜底 = closure 单层生产面 0-byte + 生产集成归后续窗 | 本表 §1 + G29_ACCEPTANCE_MAP M-a/M-b | go(M-a/M-b 承载) |
| RD-041-svt-ktx2-wg | SVT/KTX2/WG 差距表重判窗 | G22.2~22.3 defer → G25 M-d 归档 → G29.1 go | 各差距表 reeval_anchor 字面 → SVT 四行/KTX2 三行 defer + WG not-available 实测维持 | go | M-c 承载七行逐锚重判 + M-d 承载 WG/DGC capability 复测 | milestones/g22/g22_svt_gap.json + g22_ktx2_disposition.json + g22_work_graphs_probe_results.json | 重判条件 = 各行 reeval_anchor 命中或 WG 扩展 present 翻转时启动;兜底 = 全行维持 defer + not-available 维持 + DDS 链维持 | 本表 §1 + G29_ACCEPTANCE_MAP M-c/M-d | go(M-c/M-d 承载) |

## 2. open RD 逐条映射（八条口径）

| RD | title(摘要) | 条目级 status | G29.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open(G30 尾锚重判窗在案) | 无 | G28 M-d 复查在案(maintain-blocked 探针新鲜),material device 范围外 | 本表 §2 + registry/deferred.json RD-034 |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open(G30 尾锚重判窗在案) | 无 | G27 重判在案(M-a HZB device 化 implemented + M-c cluster P4 四行维持 open),material device 范围外 | 本表 §2 + registry/deferred.json RD-039 |
| RD-040 | 光照 P3+ | open | 维持 open(G30 尾锚重判窗在案) | 无 | G28 重判在案(M52 maintain-defer 两半 1/2 + 五分项维持 defer),material device 范围外 | 本表 §2 + registry/deferred.json RD-040 |
| RD-041 | 材质/流送/时域 P3+ | open | M-a/M-b/M-c/M-d 承载(slab 分项兑现 + 差距七行重判 + WG 复测) | M-a/M-b/M-c/M-d | G25 归档锚在案(SVT/KTX2/WG reeval_anchor + slab device 集成窗),本期 M-a/M-b 兑现 slab device 集成窗 + M-c 七行逐锚重判 + M-d WG/DGC capability 复测,RD-041 history 只追加 | 本表 §2 + G29_ACCEPTANCE_MAP M-a/M-b/M-c/M-d + registry/deferred.json RD-041 history |
| RD-042 | 可微物理研究轨 | open | 维持 open(G30 尾锚重判窗在案) | 无 | G25 归档锚在案(可微仿真需求场景未出现),material device 范围外 | 本表 §2 + registry/deferred.json RD-042 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open(G30 尾锚重判窗在案) | 无 | G25 归档锚在案(out_of_scope 翻转程序 + wgrapier 成熟度证据未出现),material device 范围外 | 本表 §2 + registry/deferred.json RD-043 |
| RD-044 | 物理 P3+ | open | 维持 open(G30 尾锚重判窗在案) | 无 | G25 归档锚在案(三分项 reeval_anchor),material device 范围外 | 本表 §2 + registry/deferred.json RD-044 |
| RD-045 | digest 漂移修复 | open | 维持 open(G30 尾锚重判窗在案) | 无 | G26 M-c maintain-open 重判在案(新鲜窗 6/6 零漂移 + 三件盘点 0/3 只追加扩窗),material device 范围外 | 本表 §2 + registry/deferred.json RD-045 |

## 3. G29 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G29-N1 | slab device kernel 实现车道 | G29.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G29_CONTRACT §4.2 M-a | 重判条件 = 逐样本对拍容差面或白炉恒等 device 复现面争议时只追加程序重判;兜底 = host 参考臂 material/slab.rs 0-byte 维持 + device 不可用 SKIP 如实登记 | G29_ACCEPTANCE_MAP M-a | go(M-a) |
| G29-N2 | 侧表供参加性臂 | G29.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G29_CONTRACT §4.2 M-b | 重判条件 = 逐槽对拍面或逐槽白炉恒等面争议时只追加程序重判;兜底 = MaterialClosure 32B 冻结面与 reserved 拓扑位(graph/types.rs)0-byte 维持 + 侧表臂 bin-local 独立 SSBO 不触碰冻结面 | G29_ACCEPTANCE_MAP M-b | go(M-b) |
| G29-N3 | SVT/KTX2 七行 reeval 协议 | G29.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G29_CONTRACT §4.2 M-c | 重判条件 = 七行逐锚树内实测面争议时只追加程序重判;兜底 = 全行维持 defer 登记 g29_svt_ktx2_rejudgment.json + g22 原表 0-byte 不回写 + RD-041 history 只追加 | G29_ACCEPTANCE_MAP M-c | go(M-c) |
| G29-N4 | WG/DGC capability 复测面 | G29.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G29_CONTRACT §4.2 M-d | 重判条件 = WG 扩展 present 翻转或 DGC 三扩展复测互核面争议时复评启动只追加程序重判;兜底 = not-available 维持 + FSR 3.1.5 maintain(vendor_upscale 面 0-byte) | G29_ACCEPTANCE_MAP M-d | go(M-d) |
| G29-N5 | G28 链回归守护与 soak 探针轮换扩容至八车道 | G29.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据 + G29.5 soak 承载 | G29_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合或 soak 探针轮换集合争议时只追加程序扩表;兜底 = verify-latest 纪律维持 | G29_ACCEPTANCE_MAP M-e | go(M-e/soak 承载) |

## 4. 汇总

go §1 两行 + §3 五行 = 7 go 承载面；零 defer 行（G29 非收官期，defer 合法值 = defer-to-G30+，本波零消费）；§2 open RD 八条维持 open（RD-041 → M-a/M-b/M-c/M-d 承载）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 首版：7 行候选闭集。 |
