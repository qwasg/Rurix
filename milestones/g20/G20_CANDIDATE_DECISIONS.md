<!-- Assisted-by: Cursor Agent（G20.1 治理波） -->
# G20_CANDIDATE_DECISIONS — G20.1 候选决策表（v1.0 2026-08-24）

> **状态**：G20.1 治理波定稿。**候选闭集 14 行零空行** = §1 九行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G21+` / `strategic_override`。**G20 即本期：defer-to-G20+ 不再合法，defer 合法值 = defer-to-G21+（承接锚点名七期战役具体期别）**。

## 1. G19 defer-to-G20+ 承接 8 行 + M61 重判锚行 = 9 行逐行转引终态裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M61 | mesh shader 第三光栅路径 | G18.8 no-go 重判锚 → G20.1 go | G19+ HZB/cluster P4 触发条件齐备 → VS 光栅唯一 fallback 维持 | go | 重判条件半边命中（G20 M-a 兑现 HZB host 面）；M-c 承载重判程序（maintain-no-go/go 均合法） | milestones/g18/G18_P2_DECISIONS.md §1 M61 | 重判条件 = M-c 真跑后争议时只追加程序重判；兜底 = VS 光栅唯一 fallback 维持（字面 0-byte） | 本表 §1 + G20_ACCEPTANCE_MAP M-c | go（M-c 承载） |
| M98-l4 | M98 L4 Far Field 档 | G19.5 defer-to-G20+ → G20.1 go | G20 HLOD 接口面就绪 + L4 计数可测窗 → L1/L2/L3 三级链维持 | go | G20 即窗；M-d 承载重判程序（实现/维持三级链均合法） | milestones/g19/G19_P2_DECISIONS.md §1 M98-l4 | 重判条件 = M-d 真跑后争议时只追加程序重判；兜底 = L1/L2/L3 三级链维持 | 本表 §1 + G20_ACCEPTANCE_MAP M-d | go（M-d 承载） |
| M52 | SER / hit-object 重排 | G19.5 defer-to-G20+ → G20.1 窗重评 | G21 光照期高分歧 RT workload + rt.ser 设备面实测窗 → 语言层不加 SER 原语维持 | defer-to-G21+ | G20 几何主轨不交集；G21 光照 P3+ 深化期即窗 | milestones/g19/G19_P2_DECISIONS.md §1 M52 | 重判条件 = G21 rt.ser 设备面实测窗；兜底 = 语言层不加 SER 原语维持（字面 0-byte） | 本表 §1 行；registry/deferred.json RD-040 | open-defer（G21 窗） |
| SAFE-GPU | Safe GPU Operator Platform | G19.5 defer-to-G20+ → G20.1 窗结论 | G24 立项评估处置窗 → G9~G19 零交付维持 | defer-to-G21+ | G20 非独立期；G24 处置窗承接 | milestones/g19/G19_P2_DECISIONS.md §1 SAFE-GPU | 重判条件 = G24 立项评估处置窗；兜底 = G9~G20 零交付维持 | 本表 §1 行 | open-defer（G24 窗） |
| M127 | 神经变形研究子轨 | G19.5 defer-to-G20+ → G20.1 窗结论 | G23 corpus + 消费方条件实测窗 → 无主线门研究子轨维持 | defer-to-G21+ | 与 G20 主轨零依赖；G23 物理平台深化期窗承接 | milestones/g19/G19_P2_DECISIONS.md §1 M127 | 重判条件 = G23 corpus + PhysicsAsset residual 消费方实测窗；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | open-defer（G23 窗） |
| M114-strand | 毛发 strand 档精确 OIT | G19.5 defer-to-G20+ → G20.1 窗结论 | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | defer-to-G21+ | G24 呈现与尾门清理期窗承接 | milestones/g19/G19_P2_DECISIONS.md §1 M114-strand | 重判条件 = G24 M120 精确档 benchmark 裁决数据窗；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G24 窗） |
| M118-hdr-cal | HDR 设备标定层 | G19.5 defer-to-G20+ → G20.1 窗结论 | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | defer-to-G21+ | G24 呈现与尾门清理期窗承接 | milestones/g19/G19_P2_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G24 HDR 设备面实测窗；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | open-defer（G24 窗） |
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G19.5 defer-to-G20+ → G20.1 窗结论 | G23 Jolt 升级评估窗采纳臂重评 → 5.3 基线生产默认维持 | defer-to-G21+ | G23 物理平台深化期窗承接 | milestones/g19/G19_P2_DECISIONS.md §1 M125-adopt3 | 重判条件 = G23 Jolt 升级评估窗采纳臂重评；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | open-defer（G23 窗） |
| G10-N6 | BistroExterior 未入压测清单 | G19.5 defer-to-G20+ → G20.1 窗结论 | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | defer-to-G21+ | G24 呈现与尾门清理期窗承接 | milestones/g19/G19_P2_DECISIONS.md §1 G10-N6 | 重判条件 = G24 FBX2glTF 上游复查或替代转换臂窗；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | open-defer（G24 窗） |

## 2. open RD 逐条映射（八条口径）

| RD | title（摘要） | 条目级 status | G20.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open（blocked） | M61 | 上游未解锁；G21 上游复查窗承接 | 本表 §2；registry/deferred.json |
| RD-039 | 虚拟化几何 P3+ | open | 窗内评估（M-a/M-b/M-c/M-d） | M61/M98-l4 | G20 即 P4 评估与 HZB 兑现窗 | 本表 §2 + §1 |
| RD-040 | 光照 P3+ | open | 维持 open | M52 | G21 光照 P3+ 深化期承接 | 本表 §2 + §1 M52 |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open | 无 | SVT/KTX2 等分项 G22 承接 | 本表 §2 |
| RD-042 | 可微物理研究轨 | open | 维持 open-观察 | 无 | G23 承接 | 本表 §2 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open-观察 | 无 | G23 承接 | 本表 §2 |
| RD-044 | 物理 P3+ | open | 维持 open | 无 | G23 承接 | 本表 §2 |
| RD-045 | digest 漂移修复 | open | 维持 open（G19 观察窗后长窗存续） | 无 | G19.3 12/12 零漂移在档；G25 终审窗复核 | 本表 §2 |

## 3. G20 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G20-N1 | HZB host 参考臂实现 | G20.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G20_CONTRACT §4.2 M-a | 重判条件 = 保守性争议时只追加程序重判；兜底 = 既有两级剔除链维持 | G20_ACCEPTANCE_MAP M-a | go（M-a） |
| G20-N2 | cluster 流送 P4 评估两态 | G20.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G20_CONTRACT §4.2 M-b | 重判条件 = 差距闭集争议时只追加程序扩表；兜底 = 现 streaming 页式面维持 | G20_ACCEPTANCE_MAP M-b | go（M-b） |
| G20-N3 | M61 重判两态合法 | G20.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G20_CONTRACT §4.2 M-c | 重判条件 = maintain-no-go/go 均合法留档；兜底 = VS 光栅 fallback 维持 | G20_ACCEPTANCE_MAP M-c | go（M-c） |
| G20-N4 | M98-l4 重判两态合法 | G20.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G20_CONTRACT §4.2 M-d | 重判条件 = 实现/维持三级链均合法留档；兜底 = L1/L2/L3 三级链维持 | G20_ACCEPTANCE_MAP M-d | go（M-d） |
| G20-N5 | 旧门零降级闭集 | G20.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据承载 | G20_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G20_ACCEPTANCE_MAP M-e | go（M-e） |

## 4. 汇总

go §1 两行（M61/M98-l4）+ §3 五行 = 7 go 承载面；defer-to-G21+ §1 七行（承接锚点名 G21/G23/G24 窗）；§2 open RD 八条维持（RD-039 窗内评估）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：14 行候选闭集。 |
