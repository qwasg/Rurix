<!-- Assisted-by: Cursor Agent（G19.1 治理波） -->
# G19_CANDIDATE_DECISIONS — G19.1 候选决策表（v1.0 2026-08-24）

> **状态**：G19.1 治理波定稿。**候选闭集 14 行零空行** = §1 九行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G20+` / `strategic_override`。**G19 即本期：defer-to-G19+ 不再合法，defer 合法值 = defer-to-G20+（承接锚点名七期战役具体期别）**。

## 1. G18 defer-to-G19+ 承接 9 行逐行转引终态裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G13-N7 | 帧生成 FG/MFG 独立层 | G18.1 go → G18.8 defer-to-G19+ | RFC-0035 终态落档后按只追加程序重判 → FG/MFG 零实现维持 | go | 重判条件命中（RFC-0035 defer 终态已落档）；G19 主轨立项 RFC-0036 host 参考臂 + vendor disposition | milestones/g18/G18_P2_DECISIONS.md §1 G13-N7 | 重判条件 = M-a/M-b 真跑后争议时只追加程序重判；兜底 = FG/MFG 零实现维持 | 本表 §1 + G19_ACCEPTANCE_MAP M-a/M-b | go（M-a/M-b 承载） |
| M52 | SER / hit-object 重排 | G18.8 defer-to-G19+ → G19.1 窗重评 | G19+ 高分歧 RT workload 真实集成需求 + capability rt.ser 设备面实测可用 → 语言层不加 SER 原语维持 | defer-to-G20+ | G19 帧生成主轨与 SER 不交集；战役排程归 G21 光照 P3+ 深化期 rt.ser 设备实测窗 | milestones/g18/G18_P2_DECISIONS.md §1 M52 | 重判条件 = G21 光照期高分歧 RT workload + rt.ser 设备面实测窗；兜底 = 语言层不加 SER 原语维持（字面 0-byte） | 本表 §1 行；registry/deferred.json RD-040 | open-defer（G21 窗） |
| SAFE-GPU | Safe GPU Operator Platform | G18.8 defer-to-G19+ → G19.1 窗结论 | G19+ Safe GPU Operator Platform 独立期立项 → G9~G18 零交付维持 | defer-to-G20+ | G19 非独立期；战役排程归 G24 呈现与尾门清理期立项评估处置窗 | milestones/g18/G18_P2_DECISIONS.md §1 SAFE-GPU | 重判条件 = G24 立项评估处置窗；兜底 = G9~G19 零交付维持 | 本表 §1 行 | open-defer（G24 窗） |
| M127 | 神经变形研究子轨 | G18.8 defer-to-G19+ → G19.1 窗结论 | G19+ 离线工具链 corpus + PhysicsAsset residual 消费方出现 → 无主线门研究子轨维持 | defer-to-G20+ | 与 G19 主轨零依赖；战役排程归 G23 物理平台深化期重判窗 | milestones/g18/G18_P2_DECISIONS.md §1 M127 | 重判条件 = G23 corpus + 消费方条件实测窗；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | open-defer（G23 窗） |
| M98-l4 | M98 L4 Far Field 档 | G18.8 defer-to-G19+ → G19.1 窗结论 | G19+ HLOD 接口面就绪 + L4 计数可测 → L1/L2/L3 三级链维持 | defer-to-G20+ | G19 不扩几何面；战役排程归 G20 虚拟化几何 P4 期 HLOD 接口窗 | milestones/g18/G18_P2_DECISIONS.md §1 M98-l4 | 重判条件 = G20 HLOD 接口面就绪 + L4 计数可测窗；兜底 = L1/L2/L3 三级链维持 | 本表 §1 行 | open-defer（G20 窗） |
| M114-strand | 毛发 strand 档精确 OIT | G18.8 defer-to-G19+ → G19.1 窗结论 | G19+ M120 精确档 benchmark 裁决数据落地 → card/mesh 档维持 | defer-to-G20+ | G19 非呈现资产期；战役排程归 G24 呈现与尾门清理期 | milestones/g18/G18_P2_DECISIONS.md §1 M114-strand | 重判条件 = G24 M120 精确档 benchmark 裁决数据窗；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G24 窗） |
| M118-hdr-cal | HDR 设备标定层 | G18.8 defer-to-G19+ → G19.1 窗结论 | G19+ HDR 显示设备资产/产品需求出现 → g9.p0.m118 门绿 SDR 面维持 | defer-to-G20+ | G19 presentation 面零触碰；战役排程归 G24 HDR 标定重判窗 | milestones/g18/G18_P2_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G24 HDR 设备面实测窗；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | open-defer（G24 窗） |
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G18.8 defer-to-G19+ → G19.1 窗结论 | G19+ Jolt 升级评估窗采纳臂成立 → 5.3 基线生产默认维持 | defer-to-G20+ | G19 物理面零交付；战役排程归 G23 物理平台深化期 Jolt 升级评估窗 | milestones/g18/G18_P2_DECISIONS.md §1 M125-adopt3 | 重判条件 = G23 Jolt 升级评估窗采纳臂重评；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | open-defer（G23 窗） |
| G10-N6 | BistroExterior 未入压测清单 | G18.8 defer-to-G19+ → G19.1 窗结论 | G19+ FBX2glTF 上游修复或替代臂落地 → BistroInterior + CornellBox 维持 | defer-to-G20+ | G19 场景闭集 0-byte；战役排程归 G24 转换臂重试窗 | milestones/g18/G18_P2_DECISIONS.md §1 G10-N6 | 重判条件 = G24 FBX2glTF 上游复查或替代转换臂窗；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | open-defer（G24 窗） |

## 2. open RD 逐条映射（G18 八条口径）

| RD | title（摘要） | 条目级 status | G19.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open（blocked） | 无 | 上游未解锁；G21 上游复查窗承接 | 本表 §2；registry/deferred.json |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open | M98-l4 | G20 虚拟化几何 P4 期承接 | 本表 §2 |
| RD-040 | 光照 P3+ | open | 维持 open | M52 | G21 光照 P3+ 深化期承接 | 本表 §2 + §1 M52 |
| RD-041 | 材质/流送/时域 P3+ | open | 窗内评估（M-a/M-b FG 分项） | G13-N7 | G19 帧生成分项兑现窗；其余分项 G22 承接 | 本表 §2 + §1 G13-N7 |
| RD-042 | 可微物理研究轨 | open | 维持 open-观察 | 无 | G23 物理平台深化期承接 | 本表 §2 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open-观察 | 无 | G23 物理平台深化期承接 | 本表 §2 |
| RD-044 | 物理 P3+ | open | 维持 open | 无 | G23 物理平台深化期承接 | 本表 §2 |
| RD-045 | digest 漂移修复 | open | 窗内观察（M-c） | M-c | G19 长窗观察兑现 backfill 字面动作；close/maintain-open 均合法 | 本表 §2 + G19_ACCEPTANCE_MAP M-c |

## 3. G19 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G19-N1 | FG/MFG host 参考臂实现 | G19.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G19_CONTRACT §4.2 M-a | 重判条件 = 参考臂质量争议时只追加程序重判；兜底 = 默认臂 digest 零漂移维持 | G19_ACCEPTANCE_MAP M-a | go（M-a） |
| G19-N2 | vendor 三臂 disposition 三态 | G19.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G19_CONTRACT §4.2 M-b | 重判条件 = disposition 争议时只追加程序重判；兜底 = 310.5.2 生产默认 + FG vendor 零集成如实登记 | G19_ACCEPTANCE_MAP M-b | go（M-b） |
| G19-N3 | RD-045 长窗观察两态 | G19.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G19_CONTRACT §4.2 M-c | 重判条件 = 观察窗争议时只追加程序重判；兜底 = RD-045 maintain-open 诚实登记 | G19_ACCEPTANCE_MAP M-c | go（M-c） |
| G19-N4 | fps 重评窗登记两态合法 | G19.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G19_CONTRACT §4.2 M-d | 重判条件 = 物理不可达时维持未达标登记；兜底 = ×1.00 口径 0-byte，终判归 G25 | G19_ACCEPTANCE_MAP M-d | go（M-d） |
| G19-N5 | 旧门零降级闭集 | G19.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据承载 | G19_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G19_ACCEPTANCE_MAP M-e | go（M-e） |

## 4. 汇总

go §1 一行（G13-N7）+ §3 五行 = 6 go 承载面；defer-to-G20+ §1 八行（承接锚点名 G20/G21/G23/G24 窗）；§2 open RD 八条维持（RD-045 窗内观察）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：14 行候选闭集。 |
