<!-- Assisted-by: Cursor Agent（G23.5 P2 穷举） -->
# G23_P2_DECISIONS — G23.5 P2 穷举决策表

> **闭集**：§1 六行 + §3 五行 = 11 行零空行。

## 1. 候选表 6 行终态裁决

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G23.1 go → G23.2 M-a 重判兑现 | G23 Jolt 升级评估窗采纳臂重评 → 5.3 基线生产默认维持 | closed-go | M-a 重判窗兑现完结：三件条件 1/3（证据新鲜面命中〔sys56 cargo check 绿 + A/B 绿件〕，生产切换需求证据三类全空）→ 裁决 maintain-5.3，兜底维持 | evidence/g23_m_a_jolt_56_adoption_rejudgment_*.json + milestones/g23/g23_jolt_adoption_registry.json | 重判条件 = 需求证据三类任一命中（5.6 独有 API 引用/5.3 缺陷命中/A/B 超带）；兜底 = 5.3 生产默认（VENDOR.md pin 0-byte）维持 | 本表 §1 + G23_ACCEPTANCE_MAP M-a | closed-go（裁决 maintain-5.3 留档） |
| M127 | 神经变形研究子轨 | G23.1 go → G23.2 M-b 重判兑现 | G23 corpus + 消费方条件实测窗 → 无主线门研究子轨维持 | closed-go | M-b 重判窗兑现完结：两半实测未命中（corpus 目录 NONE + residual 消费方 NONE）→ 裁决 maintain 研究子轨 | evidence/g23_m_b_neural_deform_rejudgment_*.json | 重判条件 = 两半任一命中（搜索面闭集只追加扩面）；兜底 = 无主线门研究子轨维持 | 本表 §1 + G23_ACCEPTANCE_MAP M-b | closed-go（裁决 maintain 留档） |
| SAFE-GPU | Safe GPU Operator Platform | G23.1 defer-to-G24+ | G24 立项评估处置窗 → G9~G23 零交付维持 | defer-to-G24+ | G24 即下期处置窗（M-d 承载） | milestones/g23/G23_CANDIDATE_DECISIONS.md §1 SAFE-GPU | 重判条件 = G24 M-d 立项评估处置；兜底 = G9~G23 零交付维持 | 本表 §1 行 | open-defer（G24 窗） |
| M114-strand | 毛发 strand 档精确 OIT | G23.1 defer-to-G24+ | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | defer-to-G24+ | G24 即下期窗（M-a 承载） | milestones/g23/G23_CANDIDATE_DECISIONS.md §1 M114-strand | 重判条件 = G24 M-a 两半重判；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G24 窗） |
| M118-hdr-cal | HDR 设备标定层 | G23.1 defer-to-G24+ | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | defer-to-G24+ | G24 即下期窗（M-b 承载） | milestones/g23/G23_CANDIDATE_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G24 M-b 两半重判；兜底 = SDR 面维持 | 本表 §1 行 | open-defer（G24 窗） |
| G10-N6 | BistroExterior 未入压测清单 | G23.1 defer-to-G24+ | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | defer-to-G24+ | G24 即下期窗（M-c 承载） | milestones/g23/G23_CANDIDATE_DECISIONS.md §1 G10-N6 | 重判条件 = G24 M-c 工具链/资产实测复查；兜底 = 双场景闭集维持 | 本表 §1 行 | open-defer（G24 窗） |

## 2. open RD 八条映射

| RD | status | G23.5 处置 |
|---|---|---|
| RD-034 | open | 维持 open |
| RD-039 | open | 维持 open |
| RD-040 | open | 维持 open |
| RD-041 | open | 维持 open |
| RD-042 | open | 维持 open（四轨 maintain-observe history 只追加） |
| RD-043 | open | 维持 open（wgrapier 轨 maintain-observe history 只追加） |
| RD-044 | open | 维持 open（三分项处置闭集 history 只追加） |
| RD-045 | open | 维持 open（长窗存续） |

## 3. G23 期内行 5 行终态

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G23-N1 | Jolt 5.6 采纳臂机器取证重判 | G23.1 新增 | M-a 真跑后争议时重判 | closed-go | M-a 全绿（sys56 在树 + VENDOR56 + A/B 只读盘点 + cargo check 新鲜 + 三件登记 + maintain-5.3） | evidence/g23_m_a_jolt_56_adoption_rejudgment_*.json | 重判条件 = 三件条件争议时只追加程序重判；兜底 = 5.3 生产默认维持 | G23_ACCEPTANCE_MAP M-a | closed-go |
| G23-N2 | 神经变形两半实测重判 | G23.1 新增 | M-b 真跑后争议时重判 | closed-go | M-b 全绿（两半未命中 + 搜索面闭集登记） | evidence/g23_m_b_neural_deform_rejudgment_*.json | 重判条件 = 两半争议时只追加扩面重判；兜底 = 研究子轨维持 | G23_ACCEPTANCE_MAP M-b | closed-go |
| G23-N3 | 研究轨处置闭集 | G23.1 新增 | M-c 真跑后争议时重判 | closed-go | M-c 全绿（四轨 maintain-observe + 两 RD history 只追加） | evidence/g23_m_c_research_track_disposition_*.json | 重判条件 = 闭集争议时只追加扩表；兜底 = open-观察维持 | G23_ACCEPTANCE_MAP M-c | closed-go |
| G23-N4 | 物理 P3+ 分项处置闭集 | G23.1 新增 | M-d 真跑后争议时重判 | closed-go | M-d 全绿（三分项 defer 2 + maintain-no-go 1 + RD-044 history 只追加） | evidence/g23_m_d_physics_p3_subitem_disposition_*.json | 重判条件 = 闭集争议时只追加扩表；兜底 = RD-044 open 维持 | G23_ACCEPTANCE_MAP M-d | closed-go |
| G23-N5 | 旧门零降级闭集 | G23.1 新增 | M-e 真跑后争议时重判 | closed-go | M-e 全绿（G22 链 verify-latest 零降级 + g23_ 前缀不抢 latest） | evidence/g23_m_e_closed_gate_no_regression_*.json | 重判条件 = 受影响门集合争议时只追加扩表；兜底 = verify-latest 纪律维持 | G23_ACCEPTANCE_MAP M-e | closed-go |
