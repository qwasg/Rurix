<!-- Assisted-by: Cursor Agent（G21.5 P2 穷举） -->
# G21_P2_DECISIONS — G21.5 P2 穷举决策表

> **闭集**：§1 八行 + §3 五行 = 13 行零空行。

## 1. 候选表 8 行终态裁决

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M100-high | ReSTIR GI/DI 高档 reservoir | G21.1 go → G21.2 M-a 兑现 | G19+ 高档 reservoir 证据齐备 → 低档 MegaLights 默认档维持 | closed-go | M-a 全绿（无偏 3σ + 方差收益 15.955× + 时域再收益 7.27× + 双跑位级——高档 reservoir 证据齐备兑现） | evidence/g21_m_a_restir_high_reservoir_realization_*.json + evidence/g21_restir_probe_*.json | 重判条件 = device 化/空间重用/M100 车道集成窗（RFC-0038 out-of-scope 锚）；兜底 = 低档 MegaLights 默认档维持（multi_light 0-byte） | 本表 §1 + G21_ACCEPTANCE_MAP M-a | closed-go |
| M52 | SER / hit-object 重排 | G21.1 go → G21.2 M-b 重判兑现 | G21 光照期高分歧 RT workload + rt.ser 设备面实测窗 → 语言层不加 SER 原语维持 | closed-go | M-b 重判窗兑现完结：capability 半边实测 available（vulkaninfo 三 token 取证）+ workload 半边未命中（RT pipeline/SBT 车道零实现）→ 裁决 maintain-defer，兜底维持 | evidence/g21_m_b_ser_capability_disposition_*.json + milestones/g21/g21_ser_capability_probe_results.json | 重判条件 = RT pipeline/SBT 宿主车道出现（RD-040 分项 RT-PIPELINE-SBT reeval_anchor）；兜底 = 语言层不加 SER 原语维持（字面 0-byte） | 本表 §1 + G21_ACCEPTANCE_MAP M-b | closed-go（裁决 maintain-defer 留档） |
| SAFE-GPU | Safe GPU Operator Platform | G21.1 defer-to-G22+ | G24 立项评估处置窗 → G9~G21 零交付维持 | defer-to-G22+ | G24 处置窗承接 | milestones/g21/G21_CANDIDATE_DECISIONS.md §1 SAFE-GPU | 重判条件 = G24 立项评估处置窗；兜底 = G9~G21 零交付维持 | 本表 §1 行 | open-defer（G24 窗） |
| M127 | 神经变形研究子轨 | G21.1 defer-to-G22+ | G23 corpus + 消费方条件实测窗 → 无主线门研究子轨维持 | defer-to-G22+ | G23 物理平台深化期窗承接 | milestones/g21/G21_CANDIDATE_DECISIONS.md §1 M127 | 重判条件 = G23 corpus + PhysicsAsset residual 消费方实测窗；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | open-defer（G23 窗） |
| M114-strand | 毛发 strand 档精确 OIT | G21.1 defer-to-G22+ | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | defer-to-G22+ | G24 呈现与尾门清理期窗承接 | milestones/g21/G21_CANDIDATE_DECISIONS.md §1 M114-strand | 重判条件 = G24 M120 精确档 benchmark 裁决数据窗；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G24 窗） |
| M118-hdr-cal | HDR 设备标定层 | G21.1 defer-to-G22+ | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | defer-to-G22+ | G24 呈现与尾门清理期窗承接 | milestones/g21/G21_CANDIDATE_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G24 HDR 设备面实测窗；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | open-defer（G24 窗） |
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G21.1 defer-to-G22+ | G23 Jolt 升级评估窗采纳臂重评 → 5.3 基线生产默认维持 | defer-to-G22+ | G23 物理平台深化期窗承接 | milestones/g21/G21_CANDIDATE_DECISIONS.md §1 M125-adopt3 | 重判条件 = G23 Jolt 升级评估窗采纳臂重评；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | open-defer（G23 窗） |
| G10-N6 | BistroExterior 未入压测清单 | G21.1 defer-to-G22+ | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | defer-to-G22+ | G24 呈现与尾门清理期窗承接 | milestones/g21/G21_CANDIDATE_DECISIONS.md §1 G10-N6 | 重判条件 = G24 FBX2glTF 上游复查或替代转换臂窗；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | open-defer（G24 窗） |

## 2. open RD 八条映射

| RD | status | G21.5 处置 |
|---|---|---|
| RD-034 | open | 维持 open（G21.3 探针复查：上游仍拒 raygen，blocked 维持 history 只追加） |
| RD-039 | open | 维持 open |
| RD-040 | open | 维持 open（五分项处置闭集 + M100-high 兑现 + M52 重判 history 只追加） |
| RD-041 | open | 维持 open（G22 承接） |
| RD-042 | open | 维持 open（G23 承接） |
| RD-043 | open | 维持 open（G23 承接） |
| RD-044 | open | 维持 open（G23 承接） |
| RD-045 | open | 维持 open（长窗存续） |

## 3. G21 期内行 5 行终态

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G21-N1 | ReSTIR 高档 reservoir 参考臂 | G21.1 新增 | M-a 真跑后争议时重判 | closed-go | M-a 全绿（方差收益 15.955×/时域 7.27×/无偏 3σ/双跑位级） | evidence/g21_m_a_restir_high_reservoir_realization_*.json | 重判条件 = 判据争议时只追加程序重判；兜底 = 低档 MegaLights 默认档维持 | G21_ACCEPTANCE_MAP M-a | closed-go |
| G21-N2 | SER 两半实测重判 | G21.1 新增 | M-b 真跑后争议时重判 | closed-go | M-b 全绿（capability available + workload 未命中 → maintain-defer 留档） | evidence/g21_m_b_ser_capability_disposition_*.json | 重判条件 = 两半争议时只追加程序重判；兜底 = 语言层不加 SER 原语维持 | G21_ACCEPTANCE_MAP M-b | closed-go |
| G21-N3 | RD-040 五分项处置闭集 | G21.1 新增 | M-c 真跑后争议时重判 | closed-go | M-c 全绿（五分项全 defer 各附 basis+reeval_anchor + history 只追加） | evidence/g21_m_c_rd040_subitem_disposition_*.json | 重判条件 = 分项闭集争议时只追加程序扩表；兜底 = RD-040 open 维持 | G21_ACCEPTANCE_MAP M-c | closed-go |
| G21-N4 | RD-034 上游复查两态 | G21.1 新增 | M-d 真跑后争议时重判 | closed-go | M-d 全绿（探针真跑复查 = 维持 blocked + history 只追加） | evidence/g21_m_d_rd034_upstream_recheck_*.json | 重判条件 = 探针复查争议时只追加程序重判；兜底 = RD-034 blocked 维持 | G21_ACCEPTANCE_MAP M-d | closed-go |
| G21-N5 | 旧门零降级闭集 | G21.1 新增 | M-e 真跑后争议时重判 | closed-go | M-e 全绿（G20 链 verify-latest 零降级 + g21_ 前缀不抢 latest） | evidence/g21_m_e_closed_gate_no_regression_*.json | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G21_ACCEPTANCE_MAP M-e | closed-go |
