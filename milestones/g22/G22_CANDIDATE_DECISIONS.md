<!-- Assisted-by: Cursor Agent（G22.1 治理波） -->
# G22_CANDIDATE_DECISIONS — G22.1 候选决策表（v1.0 2026-08-24）

> **状态**：G22.1 治理波定稿。**候选闭集 11 行零空行** = §1 六行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G23+` / `strategic_override`。**G22 即本期：defer-to-G22+ 不再合法，defer 合法值 = defer-to-G23+（承接锚点名七期战役具体期别）**。

## 1. G21 defer-to-G22+ 承接 6 行逐行转引终态裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| SAFE-GPU | Safe GPU Operator Platform | G21.5 defer-to-G22+ → G22.1 窗结论 | G24 立项评估处置窗 → G9~G21 零交付维持 | defer-to-G23+ | G22 非独立期；G24 处置窗承接 | milestones/g21/G21_P2_DECISIONS.md §1 SAFE-GPU | 重判条件 = G24 立项评估处置窗；兜底 = G9~G22 零交付维持 | 本表 §1 行 | open-defer（G24 窗） |
| M127 | 神经变形研究子轨 | G21.5 defer-to-G22+ → G22.1 窗结论 | G23 corpus + 消费方条件实测窗 → 无主线门研究子轨维持 | defer-to-G23+ | 与 G22 主轨零依赖；G23 物理平台深化期即下期窗 | milestones/g21/G21_P2_DECISIONS.md §1 M127 | 重判条件 = G23 corpus + PhysicsAsset residual 消费方实测窗；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | open-defer（G23 窗） |
| M114-strand | 毛发 strand 档精确 OIT | G21.5 defer-to-G22+ → G22.1 窗结论 | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | defer-to-G23+ | G24 呈现与尾门清理期窗承接 | milestones/g21/G21_P2_DECISIONS.md §1 M114-strand | 重判条件 = G24 M120 精确档 benchmark 裁决数据窗；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G24 窗） |
| M118-hdr-cal | HDR 设备标定层 | G21.5 defer-to-G22+ → G22.1 窗结论 | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | defer-to-G23+ | G24 呈现与尾门清理期窗承接 | milestones/g21/G21_P2_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G24 HDR 设备面实测窗；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | open-defer（G24 窗） |
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G21.5 defer-to-G22+ → G22.1 窗结论 | G23 Jolt 升级评估窗采纳臂重评 → 5.3 基线生产默认维持 | defer-to-G23+ | G23 物理平台深化期即下期窗 | milestones/g21/G21_P2_DECISIONS.md §1 M125-adopt3 | 重判条件 = G23 Jolt 升级评估窗采纳臂重评；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | open-defer（G23 窗） |
| G10-N6 | BistroExterior 未入压测清单 | G21.5 defer-to-G22+ → G22.1 窗结论 | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | defer-to-G23+ | G24 呈现与尾门清理期窗承接 | milestones/g21/G21_P2_DECISIONS.md §1 G10-N6 | 重判条件 = G24 FBX2glTF 上游复查或替代转换臂窗；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | open-defer（G24 窗） |

## 2. open RD 逐条映射（八条口径）

| RD | title（摘要） | 条目级 status | G22.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open（G21.3 复查在案） | 无 | 上游未解锁 | 本表 §2；registry/deferred.json |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open | 无 | G20 已落 HZB + P4 差距闭集 | 本表 §2 |
| RD-040 | 光照 P3+ | open | 维持 open（G21 五分项闭集在案） | 无 | G21 已处置 | 本表 §2 |
| RD-041 | 材质/流送/时域 P3+ | open | 窗内评估（M-a/M-b/M-c/M-d） | 无 | G22 即材质/流送/时域深化窗 | 本表 §2 + G22_ACCEPTANCE_MAP |
| RD-042 | 可微物理研究轨 | open | 维持 open-观察 | 无 | G23 承接 | 本表 §2 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open-观察 | 无 | G23 承接 | 本表 §2 |
| RD-044 | 物理 P3+ | open | 维持 open | 无 | G23 承接 | 本表 §2 |
| RD-045 | digest 漂移修复 | open | 维持 open（长窗存续） | 无 | G25 终审窗复核 | 本表 §2 |

## 3. G22 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G22-N1 | slab 材质能量守恒参考臂 | G22.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G22_CONTRACT §4.2 M-a | 重判条件 = 白炉判据争议时只追加程序重判；兜底 = closure 单层生产面维持 | G22_ACCEPTANCE_MAP M-a | go（M-a） |
| G22-N2 | SVT 差距闭集处置 | G22.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G22_CONTRACT §4.2 M-b | 重判条件 = 差距闭集争议时只追加程序扩表；兜底 = 现 streaming 页式面维持 | G22_ACCEPTANCE_MAP M-b | go（M-b） |
| G22-N3 | KTX2-BasisU 转码链处置 | G22.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G22_CONTRACT §4.2 M-c | 重判条件 = 转码收益争议时只追加程序重判；兜底 = G11.3 DDS 转码链维持 | G22_ACCEPTANCE_MAP M-c | go（M-c） |
| G22-N4 | Work Graphs/FSR 分项重评 | G22.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G22_CONTRACT §4.2 M-d | 重判条件 = 设备扩展面变化时只追加程序重判；兜底 = DGC 现面（dgc.rs M102）+ FSR 3.1.5 维持 | G22_ACCEPTANCE_MAP M-d | go（M-d） |
| G22-N5 | 旧门零降级闭集 | G22.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据承载 | G22_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G22_ACCEPTANCE_MAP M-e | go（M-e） |

## 4. 汇总

go §3 五行 = 5 go 承载面；defer-to-G23+ §1 六行（承接锚点名 G23/G24 窗）；§2 open RD 八条维持（RD-041 窗内评估）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：11 行候选闭集。 |
