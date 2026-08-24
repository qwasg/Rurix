<!-- Assisted-by: Cursor Agent（G24.5 P2 穷举） -->
# G24_P2_DECISIONS — G24.5 P2 穷举决策表

> **闭集**：§1 四行 + §3 五行 = 9 行零空行。

## 1. 候选表 4 行终态裁决

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M114-strand | 毛发 strand 档精确 OIT | G24.1 go → G24.2 M-a 重判兑现 | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | closed-go | M-a 重判窗兑现完结：数据半命中（M120 measured 绿件在案）+ 需求半未命中（压测闭集毛发资产 NONE）→ 裁决 maintain card/mesh | evidence/g24_m_a_hair_strand_oit_rejudgment_*.json | 重判条件 = 需求半命中（毛发资产入压测闭集）；兜底 = card/mesh 档维持 | 本表 §1 + G24_ACCEPTANCE_MAP M-a | closed-go（裁决 maintain 留档） |
| M118-hdr-cal | HDR 设备标定层 | G24.1 go → G24.2 M-b 重判兑现 | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | closed-go | M-b 重判窗兑现完结：设备半实测 not-available（HDR 色彩空间 token 全 absent）+ 需求半未命中 → 裁决 maintain-SDR | evidence/g24_m_b_hdr_calibration_rejudgment_*.json + milestones/g24/g24_hdr_probe_results.json | 重判条件 = 显示链变化 + HDR 资产需求成立；兜底 = SDR 面维持 | 本表 §1 + G24_ACCEPTANCE_MAP M-b | closed-go（裁决 maintain-SDR 留档） |
| G10-N6 | BistroExterior 未入压测清单 | G24.1 go → G24.3 M-c 复查兑现 | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | closed-go | M-c 复查窗兑现完结：工具链三缺实测（fbx2gltf/assimp/blender PATH 全缺）+ 独立源资产缺 → 裁决维持双场景闭集 | evidence/g24_m_c_bistro_exterior_conversion_rejudgment_*.json + milestones/g24/g24_bistro_exterior_recheck.json | 重判条件 = 上游修复在树或替代臂+源资产同窗齐备；兜底 = 双场景闭集维持 | 本表 §1 + G24_ACCEPTANCE_MAP M-c | closed-go（裁决维持留档） |
| SAFE-GPU | Safe GPU Operator Platform | G24.1 go → G24.3 M-d 处置兑现 | G24 立项评估处置窗 → G9~G23 零交付维持 | defer-to-G25+ | M-d 处置窗兑现：独立期立项判据未成立（专属资源窗不存在 + 平台需求方零出现）→ defer-to-G25+ 归档窗点名 | evidence/g24_m_d_safe_gpu_and_legacy_rd_disposition_*.json + milestones/g24/g24_legacy_rd_registry.json | 重判条件 = G25 M-d 归档闭集（G26+ 锚化）；兜底 = G9~G24 零交付维持 | 本表 §1 + G24_ACCEPTANCE_MAP M-d | open-defer（G25 归档窗） |

## 2. open RD 八条映射

| RD | status | G24.5 处置 |
|---|---|---|
| RD-034 | open | 维持 open（清册 G24.3 重判在案） |
| RD-039 | open | 维持 open |
| RD-040 | open | 维持 open |
| RD-041 | open | 维持 open |
| RD-042 | open | 维持 open |
| RD-043 | open | 维持 open |
| RD-044 | open | 维持 open |
| RD-045 | open | 维持 open（G25 终审窗复核） |

> 历史清册十一条（RD-007/011/012/014/015/026/027/030/032/033/036）= G24.3 M-d 逐条重判零 close（history 只追加在案），不重复计入本表八条口径。

## 3. G24 期内行 5 行终态

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G24-N1 | 毛发 OIT 两半重判 | G24.1 新增 | M-a 真跑后争议时重判 | closed-go | M-a 全绿（数据半命中 + 需求半未命中 + maintain） | evidence/g24_m_a_hair_strand_oit_rejudgment_*.json | 重判条件 = 两半争议时只追加程序重判；兜底 = card/mesh 档维持 | G24_ACCEPTANCE_MAP M-a | closed-go |
| G24-N2 | HDR 两半重判 | G24.1 新增 | M-b 真跑后争议时重判 | closed-go | M-b 全绿（设备半 not-available 实测 + 需求半未命中 + maintain-SDR） | evidence/g24_m_b_hdr_calibration_rejudgment_*.json | 重判条件 = 两半争议时只追加程序重判；兜底 = SDR 面维持 | G24_ACCEPTANCE_MAP M-b | closed-go |
| G24-N3 | BistroExterior 复查 | G24.1 新增 | M-c 真跑后争议时重判 | closed-go | M-c 全绿（工具链三缺 + 源资产缺 + 维持双场景闭集） | evidence/g24_m_c_bistro_exterior_conversion_rejudgment_*.json | 重判条件 = 工具链/资产面变化时只追加程序重判；兜底 = 双场景闭集维持 | G24_ACCEPTANCE_MAP M-c | closed-go |
| G24-N4 | SAFE-GPU 处置 + 历史 RD 清册 | G24.1 新增 | M-d 真跑后争议时重判 | closed-go | M-d 全绿（十二行清册闭集 + 十一条 history 只追加 + 零 close 诚实 + SAFE-GPU defer-to-G25+） | evidence/g24_m_d_safe_gpu_and_legacy_rd_disposition_*.json | 重判条件 = 清册闭集争议时只追加程序扩表；兜底 = 各条 RD 现状维持 | G24_ACCEPTANCE_MAP M-d | closed-go |
| G24-N5 | 旧门零降级闭集 | G24.1 新增 | M-e 真跑后争议时重判 | closed-go | M-e 全绿（G23 链 verify-latest 零降级 + g24_ 前缀不抢 latest） | evidence/g24_m_e_closed_gate_no_regression_*.json | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G24_ACCEPTANCE_MAP M-e | closed-go |
