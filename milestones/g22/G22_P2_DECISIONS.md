<!-- Assisted-by: Cursor Agent（G22.5 P2 穷举） -->
# G22_P2_DECISIONS — G22.5 P2 穷举决策表

> **闭集**：§1 六行 + §3 五行 = 11 行零空行。

## 1. 候选表 6 行终态裁决

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| SAFE-GPU | Safe GPU Operator Platform | G22.1 defer-to-G23+ | G24 立项评估处置窗 → G9~G22 零交付维持 | defer-to-G23+ | G24 处置窗承接 | milestones/g22/G22_CANDIDATE_DECISIONS.md §1 SAFE-GPU | 重判条件 = G24 立项评估处置窗；兜底 = G9~G22 零交付维持 | 本表 §1 行 | open-defer（G24 窗） |
| M127 | 神经变形研究子轨 | G22.1 defer-to-G23+ | G23 corpus + 消费方条件实测窗 → 无主线门研究子轨维持 | defer-to-G23+ | G23 即下期窗（M-b 承载重判） | milestones/g22/G22_CANDIDATE_DECISIONS.md §1 M127 | 重判条件 = G23 M-b 两半实测重判；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | open-defer（G23 窗） |
| M114-strand | 毛发 strand 档精确 OIT | G22.1 defer-to-G23+ | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | defer-to-G23+ | G24 呈现与尾门清理期窗承接 | milestones/g22/G22_CANDIDATE_DECISIONS.md §1 M114-strand | 重判条件 = G24 M120 精确档 benchmark 裁决数据窗；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G24 窗） |
| M118-hdr-cal | HDR 设备标定层 | G22.1 defer-to-G23+ | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | defer-to-G23+ | G24 呈现与尾门清理期窗承接 | milestones/g22/G22_CANDIDATE_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G24 HDR 设备面实测窗；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | open-defer（G24 窗） |
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G22.1 defer-to-G23+ | G23 Jolt 升级评估窗采纳臂重评 → 5.3 基线生产默认维持 | defer-to-G23+ | G23 即下期窗（M-a 承载重判） | milestones/g22/G22_CANDIDATE_DECISIONS.md §1 M125-adopt3 | 重判条件 = G23 M-a 采纳臂机器取证重判；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | open-defer（G23 窗） |
| G10-N6 | BistroExterior 未入压测清单 | G22.1 defer-to-G23+ | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | defer-to-G23+ | G24 呈现与尾门清理期窗承接 | milestones/g22/G22_CANDIDATE_DECISIONS.md §1 G10-N6 | 重判条件 = G24 FBX2glTF 上游复查或替代转换臂窗；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | open-defer（G24 窗） |

## 2. open RD 八条映射

| RD | status | G22.5 处置 |
|---|---|---|
| RD-034 | open | 维持 open |
| RD-039 | open | 维持 open |
| RD-040 | open | 维持 open |
| RD-041 | open | 维持 open（slab 分项兑现 + SVT/KTX2/WG/FSR 四分项处置 history 只追加） |
| RD-042 | open | 维持 open（G23 承接） |
| RD-043 | open | 维持 open（G23 承接） |
| RD-044 | open | 维持 open（G23 承接） |
| RD-045 | open | 维持 open（长窗存续） |

## 3. G22 期内行 5 行终态

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G22-N1 | slab 材质能量守恒参考臂 | G22.1 新增 | M-a 真跑后争议时重判 | closed-go | M-a 全绿（白炉恒等 dev=0 + 能量上界 + 单调 + 恒等式 1e-15 + lerp 连续 + 双跑位级） | evidence/g22_m_a_slab_material_host_realization_*.json + evidence/g22_slab_probe_*.json | 重判条件 = 白炉判据争议时只追加程序重判；兜底 = closure 单层生产面维持 | G22_ACCEPTANCE_MAP M-a | closed-go |
| G22-N2 | SVT 差距闭集处置 | G22.1 新增 | M-b 真跑后争议时重判 | closed-go | M-b 全绿（四行差距闭集 disposition=defer 如实登记） | evidence/g22_m_b_svt_disposition_*.json | 重判条件 = 差距闭集争议时只追加程序扩表；兜底 = 现 streaming 页式面维持 | G22_ACCEPTANCE_MAP M-b | closed-go |
| G22-N3 | KTX2-BasisU 转码链处置 | G22.1 新增 | M-c 真跑后争议时重判 | closed-go | M-c 全绿（三行差距闭集 disposition=defer + DDS 链维持） | evidence/g22_m_c_ktx2_basisu_disposition_*.json | 重判条件 = 转码收益争议时只追加程序重判；兜底 = G11.3 DDS 转码链维持 | G22_ACCEPTANCE_MAP M-c | closed-go |
| G22-N4 | Work Graphs/FSR 分项重评 | G22.1 新增 | M-d 真跑后争议时重判 | closed-go | M-d 全绿（WG not-available 实测〔AMDX absent〕+ DGC available 实测 + dgc.rs 现面 + FSR maintain） | evidence/g22_m_d_work_graphs_fsr_reeval_disposition_*.json + milestones/g22/g22_work_graphs_probe_results.json | 重判条件 = 设备扩展面变化时只追加程序重判；兜底 = DGC 现面 + FSR 3.1.5 维持 | G22_ACCEPTANCE_MAP M-d | closed-go |
| G22-N5 | 旧门零降级闭集 | G22.1 新增 | M-e 真跑后争议时重判 | closed-go | M-e 全绿（G21 链 verify-latest 零降级 + g22_ 前缀不抢 latest） | evidence/g22_m_e_closed_gate_no_regression_*.json | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G22_ACCEPTANCE_MAP M-e | closed-go |
