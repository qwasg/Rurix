<!-- Assisted-by: Cursor Agent（G18.1 治理波） -->
# G18_CANDIDATE_DECISIONS — G18.1 候选决策表（v1.0 2026-08-24）

> **状态**：G18.1 治理波定稿。**候选闭集 25 行零空行** = §1 十六行 + §3 九行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G19+` / `strategic_override`。**G18 即本期：defer-to-G18+ 不再合法，defer 合法值 = defer-to-G19+**。

## 1. G17 defer-to-G18+ 承接 16 行逐行转引终态裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M61 | mesh shader 第三光栅路径 | G17.1 defer-to-G18+ → G18.1 go | G18+ 重评窗内多厂商扩展行为收敛 + 性能差 measured 证据齐备 → VS fallback 维持 | go | G18 全量方向立项 mesh shader P3（RFC-0034） | milestones/g17/G17_P2_DECISIONS.md §1 M61 | 重判条件 = G19+ HZB/cluster P4 触发条件齐备；兜底 = VS 光栅唯一 fallback 维持（字面 0-byte） | 本表 §1 + G18_ACCEPTANCE_MAP M-g | go（M-g 承载） |
| M52 | SER / hit-object 重排 | G17.1 defer-to-G18+ → G18.1 窗重评 | G18+ 高分歧 RT workload + rt.ser 设备面实测 → RFC 评估 | defer-to-G19+ | G18 性能/画质主轨不交集；SER 适用性窗内实测后 no-go 留档 | milestones/g17/G17_P2_DECISIONS.md §1 M52 | 重判条件 = G19+ 高分歧 RT workload 真实集成需求 + capability rt.ser 设备面实测可用；兜底 = 语言层不加 SER 原语维持（字面 0-byte） | 本表 §1 行；registry/deferred.json RD-040 | open-defer（G19+） |
| M100-high | ReSTIR GI/DI 高档 reservoir | G17.1 defer-to-G18+ → G18.1 go | G18+ 低档 MegaLights 多灯场景 measured 对照齐备 → 兜底维持（G18 窗结论） | go | G18 光照纵深轨 M-a 含低档 MegaLights 对照评估面 | milestones/g17/G17_P2_DECISIONS.md §1 M100-high | 重判条件 = G19+ 高档 reservoir 证据齐备；兜底 = 低档 MegaLights 默认档维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-a | go（M-a 子面） |
| SAFE-GPU | Safe GPU Operator Platform | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ Safe GPU Operator Platform 独立期立项 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 非独立期维持 | milestones/g17/G17_P2_DECISIONS.md §1 SAFE-GPU | 重判条件 = G19+ Safe GPU Operator Platform 独立期立项；兜底 = G9~G18 零交付维持 | 本表 §1 行 | open-defer（G19+） |
| M127 | 神经变形研究子轨 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ corpus 语料 + PhysicsAsset residual 消费方出现 → 兜底维持（G18 窗结论） | defer-to-G19+ | 与 G18 主轨零依赖 | milestones/g17/G17_P2_DECISIONS.md §1 M127 | 重判条件 = G19+ 离线工具链 corpus + PhysicsAsset residual 消费方出现；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | open-defer（G19+） |
| M98-l4 | M98 L4 Far Field 档 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ HLOD 运行时接口面就绪 + L4 计数可测 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 不扩 L4 | milestones/g17/G17_P2_DECISIONS.md §1 M98-l4 | 重判条件 = G19+ HLOD 接口面就绪 + L4 计数可测；兜底 = L1/L2/L3 三级链维持 | 本表 §1 行 | open-defer（G19+） |
| M114-strand | 毛发 strand 档精确 OIT | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ M120 精确档 benchmark 数据落地 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 产画质数据非 M120 精确档裁决数据 | milestones/g17/G17_P2_DECISIONS.md §1 M114-strand | 重判条件 = G19+ M120 精确档 benchmark 裁决数据落地；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G19+） |
| M118-hdr-cal | HDR 设备标定层 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ HDR 显示设备资产/产品需求出现 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 presentation 走 SDR 全量验证面 | milestones/g17/G17_P2_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G19+ HDR 显示设备资产/产品需求出现；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | open-defer（G19+） |
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ Jolt 升级评估窗采纳臂成立 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 物理面零交付 | milestones/g17/G17_P2_DECISIONS.md §1 M125-adopt3 | 重判条件 = G19+ Jolt 升级评估窗采纳臂成立；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | open-defer（G19+） |
| G10-N6 | BistroExterior 未入压测清单 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ FBX2glTF 上游修复或替代转换臂落地 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 场景闭集 0-byte | milestones/g17/G17_P2_DECISIONS.md §1 G10-N6 | 重判条件 = G19+ FBX2glTF 上游修复或替代臂落地；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | open-defer（G19+） |
| G10-N8 | `-renderoffscreen` UE 5.8 可用性 | G17.1 defer-to-G18+ → G18.1 go | G18+ 无头出图需求出现时实测可用性 → 兜底维持（G18 窗结论） | go | G18 商业化出图需求已出现——M-c 承载实测 | milestones/g17/G17_P2_DECISIONS.md §1 G10-N8 | 重判条件 = 可用性争议时按只追加程序重判；兜底 = 窗口模式 MRQ 出图臂维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-c | go（M-c 承载） |
| G10-N17 | M137 scalars.flip 演进位 | G17.1 defer-to-G18+ → G18.1 go | G18+ diff 报告消费 FLIP 标量面真实需求出现 → 兜底维持（G18 窗结论） | go | G18 M-d 商业化画质终审消费 FLIP 标量面 | milestones/g17/G17_P2_DECISIONS.md §1 G10-N17 | 重判条件 = 演进位争议时按 RXS-0388 L3 程序翻转；兜底 = null 演进位维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-d | go（M-d 子面） |
| G11-N5 | 锁定度量暗帧稳健性 | G17.1 defer-to-G18+ → G18.1 go | G18+ SSIM/FLIP 暗帧稳健性 measured 对照数据集齐备 → 兜底维持（G18 窗结论） | go | G18 M-d 建立暗帧对照数据集并评估 | milestones/g17/G17_P2_DECISIONS.md §1 G11-N5 | 重判条件 = G19+ 度量口径修订评估窗数据集扩展齐备；兜底 = 现锁定度量口径维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-d | go（M-d 子面） |
| G13-N7 | 帧生成 FG/MFG 独立层 | G17.1 defer-to-G18+ → G18.1 go | G18+ 帧生成独立层立项（真实渲染帧率口径） → 兜底维持（G18 窗结论） | go | G18 全量方向立项 RFC-0035 + M-h 承载 | milestones/g17/G17_P2_DECISIONS.md §1 G13-N7 | 重判条件 = RFC-0035 终态落档后按只追加程序重判；兜底 = FG/MFG 零实现维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-h | go（M-h 承载） |
| G17-MB-F1 | ngx SL 运行时升级换版 | G17.3 检出 → G18.1 go | G18+ Streamline 运行时升级评估窗落地后 M-b 同口径重评 → 兜底维持（G18 窗结论） | go | G18 M-e 承载 SL 升级 disposition | milestones/g17/G17_P2_DECISIONS.md §3 G17-MB-F1 | 重判条件 = 换版程序面争议时按只追加程序重判；兜底 = 310.5.2 生产默认维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-e | go（M-e 承载） |
| G17-MD-F1 | fps_parity_deficit bistro/t100/dlss_sr | G17.5 定盘 → G18.1 go | G18+ 立项窗三面重评任一齐备即重评 → 兜底维持（G18 窗结论） | go | G18 M-f 承载 18 格重评 | milestones/g17/G17_P2_DECISIONS.md §3 G17-MD-F1 | 重判条件 = ≥1.00 → 18/18；物理不可达 → 维持未达标登记不冒充；兜底 = G15 同源 | 本表 §1 + G18_ACCEPTANCE_MAP M-f | go（M-f 承载） |

## 2. open RD 逐条映射（G17 八条口径 + 全表 18 条 open 域内/域外分档）

| RD | title（摘要） | 条目级 status | G18.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open（blocked） | 无 | 上游未解锁；G18 Vulkan 主腿 | 本表 §2；registry/deferred.json |
| RD-039 | 虚拟化几何 P3+ | open | 窗内评估（M-g） | M61 | G18 立项 mesh shader P3 | 本表 §2 + §1 M61 |
| RD-040 | 光照 P3+ | open | 窗内评估（M-a） | M52/M100-high | GI 纵深 + SER 实测 | 本表 §2 + §1 |
| RD-041 | 材质/流送/时域 P3+ | open | 窗内评估（M-h FG 分项） | G13-N7 | 帧生成独立层 | 本表 §2 + §1 G13-N7 |
| RD-042 | 可微物理研究轨 | open | 维持 open-观察 | 无 | 与 G18 无关 | 本表 §2 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open-观察 | 无 | 与 G18 无关 | 本表 §2 |
| RD-044 | 物理 P3+ | open | 维持 open | 无 | M126 maintain_no_go | 本表 §2 |
| RD-045 | digest 漂移修复 | open | 维持 open（守护登记） | M-a~M-i | 检出即升级 | 本表 §2 |

## 3. G18 期新增候选 9 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G18-N1 | Rurix 光照纵深加性 profile | G18.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G18_CONTRACT §4.2 M-a | 重判条件 = 加性 profile 争议时只追加程序重判；兜底 = 默认臂 digest 零漂移维持 | G18_ACCEPTANCE_MAP M-a | go（M-a） |
| G18-N2 | Presentation 双 profile 出图协议 | G18.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G18_CONTRACT §4.2 M-b | 重判条件 = profile 契约争议时只追加程序重判；兜底 = G13 冻结契约 0-byte | G18_ACCEPTANCE_MAP M-b | go（M-b） |
| G18-N3 | UE 臂灯光修复与日景 variant | G18.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G18_CONTRACT §4.2 M-c | 重判条件 = UE 关卡争议时只追加程序重判；兜底 = G13 关卡 0-byte | G18_ACCEPTANCE_MAP M-c | go（M-c） |
| G18-N4 | 双端商业化画质终审两态 | G18.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G18_CONTRACT §4.2 M-d | 重判条件 = 判定口径争议时契约变更程序；兜底 = 达标/诚实红均合法 | G18_ACCEPTANCE_MAP M-d | go（M-d） |
| G18-N5 | SL 运行时升级 disposition 三态 | G18.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据承载 | G18_CONTRACT §4.2 M-e | 重判条件 = disposition 争议时只追加程序重判；兜底 = 拒绝换版/ not-available 如实登记 | G18_ACCEPTANCE_MAP M-e | go（M-e） |
| G18-N6 | fps parity 重评两态合法 | G18.1 新增 | M-f 真跑后争议时重判 | go | M-f 判据承载 | G18_CONTRACT §4.2 M-f | 重判条件 = 物理不可达时维持未达标登记；兜底 = ×1.00 口径 0-byte | G18_ACCEPTANCE_MAP M-f | go（M-f） |
| G18-N7 | 虚拟化几何 P3 RFC-0034 终态 | G18.1 新增 | M-g 真跑后争议时重判 | go | M-g 判据承载 | G18_CONTRACT §4.2 M-g | 重判条件 = no-go/defer 留档均合法；兜底 = VS 光栅 fallback 维持 | G18_ACCEPTANCE_MAP M-g | go（M-g） |
| G18-N8 | 帧生成独立层 RFC-0035 终态 | G18.1 新增 | M-h 真跑后争议时重判 | go | M-h 判据承载 | G18_CONTRACT §4.2 M-h | 重判条件 = no-go/defer 留档均合法；兜底 = FG 零实现维持 | G18_ACCEPTANCE_MAP M-h | go（M-h） |
| G18-N9 | 旧门零降级闭集 | G18.1 新增 | M-i 真跑后争议时重判 | go | M-i 判据承载 | G18_CONTRACT §4.2 M-i | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G18_ACCEPTANCE_MAP M-i | go（M-i） |

## 4. 汇总

go §1 八行 + §3 九行 = 17 go 承载面；defer-to-G19+ §1 八行；§2 open RD 八条维持。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：25 行候选闭集。 |
