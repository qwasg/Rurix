<!-- Assisted-by: Cursor Agent（G24.1 治理波） -->
# G24_CANDIDATE_DECISIONS — G24.1 候选决策表（v1.0 2026-08-24）

> **状态**：G24.1 治理波定稿。**候选闭集 9 行零空行** = §1 四行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G25+` / `strategic_override`。**G24 即本期：defer-to-G24+ 不再合法，defer 合法值 = defer-to-G25+（承接锚点名期别）**。

## 1. G23 defer-to-G24+ 承接 4 行逐行转引终态裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M114-strand | 毛发 strand 档精确 OIT | G23.5 defer-to-G24+ → G24.1 go | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | go | G24 即窗；M-a 承载两半重判（M120 measured 绿件盘点 + strand 生产需求核验） | milestones/g23/G23_P2_DECISIONS.md §1 M114-strand | 重判条件 = M-a 真跑后争议时只追加程序重判；兜底 = card/mesh 档维持 | 本表 §1 + G24_ACCEPTANCE_MAP M-a | go（M-a 承载） |
| M118-hdr-cal | HDR 设备标定层 | G23.5 defer-to-G24+ → G24.1 go | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | go | G24 即窗；M-b 承载两半重判（设备色彩空间实测 + 需求面核验） | milestones/g23/G23_P2_DECISIONS.md §1 M118-hdr-cal | 重判条件 = M-b 真跑后争议时只追加程序重判；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 + G24_ACCEPTANCE_MAP M-b | go（M-b 承载） |
| G10-N6 | BistroExterior 未入压测清单 | G23.5 defer-to-G24+ → G24.1 go | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | go | G24 即窗；M-c 承载工具链/资产在树性实测复查 | milestones/g23/G23_P2_DECISIONS.md §1 G10-N6 | 重判条件 = M-c 真跑后争议时只追加程序重判；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 + G24_ACCEPTANCE_MAP M-c | go（M-c 承载） |
| SAFE-GPU | Safe GPU Operator Platform | G23.5 defer-to-G24+ → G24.1 go | G24 立项评估处置窗 → G9~G23 零交付维持 | go | G24 即窗；M-d 承载立项评估处置（独立期立项判据核验） | milestones/g23/G23_P2_DECISIONS.md §1 SAFE-GPU | 重判条件 = M-d 真跑后争议时只追加程序重判；兜底 = G9~G24 零交付维持 | 本表 §1 + G24_ACCEPTANCE_MAP M-d | go（M-d 承载） |

## 2. open RD 逐条映射（八条口径 + 历史清册联动）

| RD | title（摘要） | 条目级 status | G24.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 清册重判（M-d 域外转引 G21.3 复查在案） | 无 | 上游未解锁在案 | 本表 §2 |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open | 无 | G20 处置在案 | 本表 §2 |
| RD-040 | 光照 P3+ | open | 维持 open | 无 | G21 处置在案 | 本表 §2 |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open | 无 | G22 处置在案 | 本表 §2 |
| RD-042 | 可微物理研究轨 | open | 维持 open | 无 | G23 处置在案 | 本表 §2 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open | 无 | G23 处置在案 | 本表 §2 |
| RD-044 | 物理 P3+ | open | 维持 open | 无 | G23 处置在案 | 本表 §2 |
| RD-045 | digest 漂移修复 | open | 维持 open（长窗存续） | 无 | G25 终审窗复核 | 本表 §2 |

> 历史 open RD（RD-007 inherited/RD-011/012/014/015/026/027/030/032/033/036 十一条）= M-d 清册域（g24_legacy_rd_registry.json 闭集逐条重判），不重复计入本表 §2 八条口径。

## 3. G24 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G24-N1 | 毛发 OIT 两半重判 | G24.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G24_CONTRACT §4.2 M-a | 重判条件 = 两半争议时只追加程序重判；兜底 = card/mesh 档维持 | G24_ACCEPTANCE_MAP M-a | go（M-a） |
| G24-N2 | HDR 两半重判 | G24.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G24_CONTRACT §4.2 M-b | 重判条件 = 两半争议时只追加程序重判；兜底 = SDR 面维持 | G24_ACCEPTANCE_MAP M-b | go（M-b） |
| G24-N3 | BistroExterior 复查 | G24.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G24_CONTRACT §4.2 M-c | 重判条件 = 工具链/资产面变化时只追加程序重判；兜底 = 双场景闭集维持 | G24_ACCEPTANCE_MAP M-c | go（M-c） |
| G24-N4 | SAFE-GPU 处置 + 历史 RD 清册 | G24.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G24_CONTRACT §4.2 M-d | 重判条件 = 清册闭集争议时只追加程序扩表；兜底 = 各条 RD 现状维持 | G24_ACCEPTANCE_MAP M-d | go（M-d） |
| G24-N5 | 旧门零降级闭集 | G24.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据承载 | G24_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G24_ACCEPTANCE_MAP M-e | go（M-e） |

## 4. 汇总

go §1 四行（承接池清零窗）+ §3 五行 = 9 go 承载面；零 defer 行（G18 承接池本期全量消化——G25 收官期承接面 = 各期重判记录锚 + fps 终判窗）；§2 open RD 八条维持。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：9 行候选闭集。 |
