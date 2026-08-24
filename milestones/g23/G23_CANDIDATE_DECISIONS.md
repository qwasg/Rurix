<!-- Assisted-by: Cursor Agent（G23.1 治理波） -->
# G23_CANDIDATE_DECISIONS — G23.1 候选决策表（v1.0 2026-08-24）

> **状态**：G23.1 治理波定稿。**候选闭集 11 行零空行** = §1 六行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G24+` / `strategic_override`。**G23 即本期：defer-to-G23+ 不再合法，defer 合法值 = defer-to-G24+（承接锚点名七期战役具体期别）**。

## 1. G22 defer-to-G23+ 承接 6 行逐行转引终态裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G22.5 defer-to-G23+ → G23.1 go | G23 Jolt 升级评估窗采纳臂重评 → 5.3 基线生产默认维持 | go | G23 即窗；M-a 承载机器取证重判（sys56 评估臂在树 + A/B 绿件盘点 + 构建新鲜真跑 + 采纳三件条件核验） | milestones/g22/G22_P2_DECISIONS.md §1 M125-adopt3 | 重判条件 = M-a 真跑后争议时只追加程序重判；兜底 = 5.3 基线生产默认（VENDOR.md pin 0-byte）维持 | 本表 §1 + G23_ACCEPTANCE_MAP M-a | go（M-a 承载） |
| M127 | 神经变形研究子轨 | G22.5 defer-to-G23+ → G23.1 go | G23 corpus + 消费方条件实测窗 → 无主线门研究子轨维持 | go | G23 即窗；M-b 承载两半实测重判 | milestones/g22/G22_P2_DECISIONS.md §1 M127 | 重判条件 = M-b 真跑后争议时只追加程序重判；兜底 = 无主线门研究子轨维持 | 本表 §1 + G23_ACCEPTANCE_MAP M-b | go（M-b 承载） |
| SAFE-GPU | Safe GPU Operator Platform | G22.5 defer-to-G23+ → G23.1 窗结论 | G24 立项评估处置窗 → G9~G22 零交付维持 | defer-to-G24+ | G23 非独立期；G24 即下期处置窗 | milestones/g22/G22_P2_DECISIONS.md §1 SAFE-GPU | 重判条件 = G24 立项评估处置窗；兜底 = G9~G23 零交付维持 | 本表 §1 行 | open-defer（G24 窗） |
| M114-strand | 毛发 strand 档精确 OIT | G22.5 defer-to-G23+ → G23.1 窗结论 | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | defer-to-G24+ | G24 呈现与尾门清理期即下期窗 | milestones/g22/G22_P2_DECISIONS.md §1 M114-strand | 重判条件 = G24 M120 精确档 benchmark 裁决数据窗；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G24 窗） |
| M118-hdr-cal | HDR 设备标定层 | G22.5 defer-to-G23+ → G23.1 窗结论 | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | defer-to-G24+ | G24 呈现与尾门清理期即下期窗 | milestones/g22/G22_P2_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G24 HDR 设备面实测窗；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | open-defer（G24 窗） |
| G10-N6 | BistroExterior 未入压测清单 | G22.5 defer-to-G23+ → G23.1 窗结论 | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | defer-to-G24+ | G24 呈现与尾门清理期即下期窗 | milestones/g22/G22_P2_DECISIONS.md §1 G10-N6 | 重判条件 = G24 FBX2glTF 上游复查或替代转换臂窗；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | open-defer（G24 窗） |

## 2. open RD 逐条映射（八条口径）

| RD | title（摘要） | 条目级 status | G23.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open | 无 | G21.3 复查在案 | 本表 §2；registry/deferred.json |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open | 无 | G20 HZB + P4 闭集在案 | 本表 §2 |
| RD-040 | 光照 P3+ | open | 维持 open | 无 | G21 五分项闭集在案 | 本表 §2 |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open | 无 | G22 四分项处置在案 | 本表 §2 |
| RD-042 | 可微物理研究轨 | open | 窗内处置（M-c） | 无 | G23 即观察轨处置窗 | 本表 §2 + G23_ACCEPTANCE_MAP M-c |
| RD-043 | wgrapier GPU 刚体 | open | 窗内处置（M-c） | 无 | G23 即观察轨处置窗 | 本表 §2 + G23_ACCEPTANCE_MAP M-c |
| RD-044 | 物理 P3+ | open | 窗内处置（M-d） | M125-adopt3 | G23 即分项处置窗 | 本表 §2 + G23_ACCEPTANCE_MAP M-d |
| RD-045 | digest 漂移修复 | open | 维持 open（长窗存续） | 无 | G25 终审窗复核 | 本表 §2 |

## 3. G23 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G23-N1 | Jolt 5.6 采纳臂机器取证重判 | G23.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G23_CONTRACT §4.2 M-a | 重判条件 = 采纳三件条件争议时只追加程序重判；兜底 = 5.3 生产默认维持 | G23_ACCEPTANCE_MAP M-a | go（M-a） |
| G23-N2 | 神经变形两半实测重判 | G23.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G23_CONTRACT §4.2 M-b | 重判条件 = 两半争议时只追加程序重判；兜底 = 无主线门研究子轨维持 | G23_ACCEPTANCE_MAP M-b | go（M-b） |
| G23-N3 | 研究轨处置闭集 | G23.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G23_CONTRACT §4.2 M-c | 重判条件 = 观察轨闭集争议时只追加程序扩表；兜底 = RD-042/043 open-观察维持 | G23_ACCEPTANCE_MAP M-c | go（M-c） |
| G23-N4 | 物理 P3+ 分项处置闭集 | G23.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G23_CONTRACT §4.2 M-d | 重判条件 = 分项闭集争议时只追加程序扩表；兜底 = RD-044 open 维持 | G23_ACCEPTANCE_MAP M-d | go（M-d） |
| G23-N5 | 旧门零降级闭集 | G23.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据承载 | G23_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G23_ACCEPTANCE_MAP M-e | go（M-e） |

## 4. 汇总

go §1 两行（M125-adopt3/M127）+ §3 五行 = 7 go 承载面；defer-to-G24+ §1 四行（承接锚点名 G24 窗）；§2 open RD 八条维持（RD-042/043/044 窗内处置）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：11 行候选闭集。 |
