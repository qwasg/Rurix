<!-- Assisted-by: Cursor Agent（G18.8 P2 穷举） -->
# G18_P2_DECISIONS — G18.8 P2 穷举决策表

> **闭集**：§1 十六行 + §3 九行 = 25 行零空行。

## 1. 候选表 16 行终态裁决

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M61 | mesh shader 第三光栅路径 | G17.1 defer-to-G18+ → G18.1 go | G18+ 重评窗内多厂商扩展行为收敛 + 性能差 measured 证据齐备 → VS fallback 维持 | no-go | RFC-0034 终态 no-go | evidence/g18_m_g_virtualized_geometry_p3_*.json | 重判条件 = G19+ HZB/cluster P4 触发条件齐备；兜底 = VS 光栅唯一 fallback 维持（字面 0-byte） | 本表 §1 + G18_ACCEPTANCE_MAP M-g | no-go |
| M52 | SER / hit-object 重排 | G17.1 defer-to-G18+ → G18.1 窗重评 | G18+ 高分歧 RT workload + rt.ser 设备面实测 → RFC 评估 | defer-to-G19+ | G18 窗未齐备 | milestones/g17/G17_P2_DECISIONS.md §1 M52 | 重判条件 = G19+ 高分歧 RT workload 真实集成需求 + capability rt.ser 设备面实测可用；兜底 = 语言层不加 SER 原语维持（字面 0-byte） | 本表 §1 行；registry/deferred.json RD-040 | defer-to-G19+ |
| M100-high | ReSTIR GI/DI 高档 reservoir | G17.1 defer-to-G18+ → G18.1 go | G18+ 低档 MegaLights 多灯场景 measured 对照齐备 → 兜底维持（G18 窗结论） | closed-go | G18 go 承载兑现 | evidence/g18_*.json | 重判条件 = G19+ 高档 reservoir 证据齐备；兜底 = 低档 MegaLights 默认档维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-a | closed-go |
| SAFE-GPU | Safe GPU Operator Platform | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ Safe GPU Operator Platform 独立期立项 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 窗未齐备 | milestones/g17/G17_P2_DECISIONS.md §1 SAFE-GPU | 重判条件 = G19+ Safe GPU Operator Platform 独立期立项；兜底 = G9~G18 零交付维持 | 本表 §1 行 | defer-to-G19+ |
| M127 | 神经变形研究子轨 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ corpus 语料 + PhysicsAsset residual 消费方出现 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 窗未齐备 | milestones/g17/G17_P2_DECISIONS.md §1 M127 | 重判条件 = G19+ 离线工具链 corpus + PhysicsAsset residual 消费方出现；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | defer-to-G19+ |
| M98-l4 | M98 L4 Far Field 档 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ HLOD 运行时接口面就绪 + L4 计数可测 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 窗未齐备 | milestones/g17/G17_P2_DECISIONS.md §1 M98-l4 | 重判条件 = G19+ HLOD 接口面就绪 + L4 计数可测；兜底 = L1/L2/L3 三级链维持 | 本表 §1 行 | defer-to-G19+ |
| M114-strand | 毛发 strand 档精确 OIT | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ M120 精确档 benchmark 数据落地 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 窗未齐备 | milestones/g17/G17_P2_DECISIONS.md §1 M114-strand | 重判条件 = G19+ M120 精确档 benchmark 裁决数据落地；兜底 = card/mesh 档维持 | 本表 §1 行 | defer-to-G19+ |
| M118-hdr-cal | HDR 设备标定层 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ HDR 显示设备资产/产品需求出现 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 窗未齐备 | milestones/g17/G17_P2_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G19+ HDR 显示设备资产/产品需求出现；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | defer-to-G19+ |
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ Jolt 升级评估窗采纳臂成立 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 窗未齐备 | milestones/g17/G17_P2_DECISIONS.md §1 M125-adopt3 | 重判条件 = G19+ Jolt 升级评估窗采纳臂成立；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | defer-to-G19+ |
| G10-N6 | BistroExterior 未入压测清单 | G17.1 defer-to-G18+ → G18.1 窗结论 | G18+ FBX2glTF 上游修复或替代转换臂落地 → 兜底维持（G18 窗结论） | defer-to-G19+ | G18 窗未齐备 | milestones/g17/G17_P2_DECISIONS.md §1 G10-N6 | 重判条件 = G19+ FBX2glTF 上游修复或替代臂落地；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | defer-to-G19+ |
| G10-N8 | `-renderoffscreen` UE 5.8 可用性 | G17.1 defer-to-G18+ → G18.1 go | G18+ 无头出图需求出现时实测可用性 → 兜底维持（G18 窗结论） | closed-go | G18 go 承载兑现 | evidence/g18_*.json | 重判条件 = 可用性争议时按只追加程序重判；兜底 = 窗口模式 MRQ 出图臂维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-c | closed-go |
| G10-N17 | M137 scalars.flip 演进位 | G17.1 defer-to-G18+ → G18.1 go | G18+ diff 报告消费 FLIP 标量面真实需求出现 → 兜底维持（G18 窗结论） | closed-go | G18 go 承载兑现 | evidence/g18_*.json | 重判条件 = 演进位争议时按 RXS-0388 L3 程序翻转；兜底 = null 演进位维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-d | closed-go |
| G11-N5 | 锁定度量暗帧稳健性 | G17.1 defer-to-G18+ → G18.1 go | G18+ SSIM/FLIP 暗帧稳健性 measured 对照数据集齐备 → 兜底维持（G18 窗结论） | closed-go | G18 go 承载兑现 | evidence/g18_*.json | 重判条件 = G19+ 度量口径修订评估窗数据集扩展齐备；兜底 = 现锁定度量口径维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-d | closed-go |
| G13-N7 | 帧生成 FG/MFG 独立层 | G17.1 defer-to-G18+ → G18.1 go | G18+ 帧生成独立层立项（真实渲染帧率口径） → 兜底维持（G18 窗结论） | defer-to-G19+ | RFC-0035 defer | evidence/g18_m_h_frame_generation_independent_layer_*.json | 重判条件 = RFC-0035 终态落档后按只追加程序重判；兜底 = FG/MFG 零实现维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-h | defer-to-G19+ |
| G17-MB-F1 | ngx SL 运行时升级换版 | G17.3 检出 → G18.1 go | G18+ Streamline 运行时升级评估窗落地后 M-b 同口径重评 → 兜底维持（G18 窗结论） | closed-go | M-e not-available | evidence/g18_m_e_sl_runtime_upgrade_disposition_*.json | 重判条件 = 换版程序面争议时按只追加程序重判；兜底 = 310.5.2 生产默认维持 | 本表 §1 + G18_ACCEPTANCE_MAP M-e | closed-go |
| G17-MD-F1 | fps_parity_deficit bistro/t100/dlss_sr | G17.5 定盘 → G18.1 go | G18+ 立项窗三面重评任一齐备即重评 → 兜底维持（G18 窗结论） | closed-go | M-f 诚实红 | evidence/g18_m_f_fps_parity_reeval_*.json | 重判条件 = ≥1.00 → 18/18；物理不可达 → 维持未达标登记不冒充；兜底 = G15 同源 | 本表 §1 + G18_ACCEPTANCE_MAP M-f | closed-go |

## 2. open RD 八条映射

| RD | status | G18.8 处置 |
|---|---|---|
| RD-034 | open | 维持 open |
| RD-039 | open | 维持 open |
| RD-040 | open | 维持 open |
| RD-041 | open | 维持 open |
| RD-042 | open | 维持 open |
| RD-043 | open | 维持 open |
| RD-044 | open | 维持 open |
| RD-045 | open | 维持 open |

## 3. G18 期内行 9 行终态

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G18-N1 | Rurix 光照纵深加性 profile | G18.1 新增 | M-a 真跑后争议时重判 | closed-go | M 门承载兑现 | evidence/g18_*.json | 重判条件 = 加性 profile 争议时只追加程序重判；兜底 = 默认臂 digest 零漂移维持 | G18_ACCEPTANCE_MAP M-a | closed-go |
| G18-N2 | Presentation 双 profile 出图协议 | G18.1 新增 | M-b 真跑后争议时重判 | closed-go | M 门承载兑现 | evidence/g18_*.json | 重判条件 = profile 契约争议时只追加程序重判；兜底 = G13 冻结契约 0-byte | G18_ACCEPTANCE_MAP M-b | closed-go |
| G18-N3 | UE 臂灯光修复与日景 variant | G18.1 新增 | M-c 真跑后争议时重判 | closed-go | M 门承载兑现 | evidence/g18_*.json | 重判条件 = UE 关卡争议时只追加程序重判；兜底 = G13 关卡 0-byte | G18_ACCEPTANCE_MAP M-c | closed-go |
| G18-N4 | 双端商业化画质终审两态 | G18.1 新增 | M-d 真跑后争议时重判 | closed-go | M 门承载兑现 | evidence/g18_*.json | 重判条件 = 判定口径争议时契约变更程序；兜底 = 达标/诚实红均合法 | G18_ACCEPTANCE_MAP M-d | closed-go |
| G18-N5 | SL 运行时升级 disposition 三态 | G18.1 新增 | M-e 真跑后争议时重判 | closed-go | M 门承载兑现 | evidence/g18_*.json | 重判条件 = disposition 争议时只追加程序重判；兜底 = 拒绝换版/ not-available 如实登记 | G18_ACCEPTANCE_MAP M-e | closed-go |
| G18-N6 | fps parity 重评两态合法 | G18.1 新增 | M-f 真跑后争议时重判 | closed-go | M 门承载兑现 | evidence/g18_*.json | 重判条件 = 物理不可达时维持未达标登记；兜底 = ×1.00 口径 0-byte | G18_ACCEPTANCE_MAP M-f | closed-go |
| G18-N7 | 虚拟化几何 P3 RFC-0034 终态 | G18.1 新增 | M-g 真跑后争议时重判 | closed-go | M 门承载兑现 | evidence/g18_*.json | 重判条件 = no-go/defer 留档均合法；兜底 = VS 光栅 fallback 维持 | G18_ACCEPTANCE_MAP M-g | closed-go |
| G18-N8 | 帧生成独立层 RFC-0035 终态 | G18.1 新增 | M-h 真跑后争议时重判 | closed-go | M 门承载兑现 | evidence/g18_*.json | 重判条件 = no-go/defer 留档均合法；兜底 = FG 零实现维持 | G18_ACCEPTANCE_MAP M-h | closed-go |
| G18-N9 | 旧门零降级闭集 | G18.1 新增 | M-i 真跑后争议时重判 | closed-go | M 门承载兑现 | evidence/g18_*.json | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G18_ACCEPTANCE_MAP M-i | closed-go |

## 4. 版本

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：§1 十六行 + §3 九行 = 25 行候选闭集零空行。 |
