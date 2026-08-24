<!-- Assisted-by: Cursor Agent（G25.5 P2 穷举） -->
# G25_P2_DECISIONS — G25.5 P2 穷举决策表

> **闭集**：§1 两行 + §3 五行 = 7 行零空行。

## 1. 候选表 2 行终态裁决

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| SAFE-GPU | Safe GPU Operator Platform | G25.1 go → G25.3 M-d 归档兑现 | G25 战役终审窗归档处置 → G9~G24 零交付维持 | closed-go | M-d 归档窗兑现完结：独立期立项判据未成立核验在案（G24 清册行）+ G26+ 归档锚落 g25_campaign_handover_registry.json | evidence/g25_m_d_campaign_handover_ledger_*.json + milestones/g25/g25_campaign_handover_registry.json | 重判条件 = 独立期资源窗 + 平台需求方出现时立项评估（G26+ 归档锚字面）；兜底 = G9~G25 零交付维持 | 本表 §1 + G25_ACCEPTANCE_MAP M-d | closed-go（defer-to-G26+ 归档锚留档） |
| G17-MD-F1 | fps_parity_deficit bistro/t100/dlss_sr 终判链 | G25.1 go → G25.2 M-b 终判兑现 | ≥1.00 → 18/18；物理不可达 → 维持未达标登记不冒充；兜底 = G15 同源 | closed-go | M-b 终判窗兑现完结：**17/18 诚实红终判**（焦点格 ratio 0.856326 + 性能面全战役 0-byte 机核 + 焦点格新鲜单测 3.5520ms 真跑登记）——战役合法收官态 | evidence/g25_m_b_fps_parity_final_verdict_*.json | 重判条件 = NGX 分解 profiling / UE 侧插桩（宿主差可分离 measured 证据）；兜底 = 17/18 诚实红终判维持（非关闭性定论） | 本表 §1 + G25_ACCEPTANCE_MAP M-b | closed-go（17/18 诚实红终判留档） |

## 2. open RD 八条映射

| RD | status | G25.5 处置 |
|---|---|---|
| RD-034 | open | 归档 G26+ 锚（M-d 归档表行） |
| RD-039 | open | 归档 G26+ 锚（M-d 归档表行） |
| RD-040 | open | 归档 G26+ 锚（M-d 归档表行） |
| RD-041 | open | 归档 G26+ 锚（M-d 归档表行） |
| RD-042 | open | 归档 G26+ 锚（M-d 归档表行） |
| RD-043 | open | 归档 G26+ 锚（M-d 归档表行） |
| RD-044 | open | 归档 G26+ 锚（M-d 归档表行） |
| RD-045 | open | 归档 G26+ 锚 + 累计观察复核（G19.3 12/12 + 六期 soak 零漂移；maintain-open 不冒充） |

## 3. G25 期内行 5 行终态

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G25-N1 | 画质终态维持核验 | G25.1 新增 | M-a 真跑后争议时重判 | closed-go | M-a 全绿（画质表面闭集 10 项 0-byte 机核 + 加性四模块零接线 + G18 达标绿件盘点） | evidence/g25_m_a_quality_final_state_verification_*.json | 重判条件 = 表面变化证据出现时重测程序；兜底 = G18 M-d 达标终态维持 | G25_ACCEPTANCE_MAP M-a | closed-go |
| G25-N2 | fps 终判两态合法 | G25.1 新增 | M-b 真跑后争议时重判 | closed-go | M-b 全绿（17/18 诚实红终判 + 焦点格新鲜单测真跑） | evidence/g25_m_b_fps_parity_final_verdict_*.json | 重判条件 = 物理不可达时维持未达标登记；兜底 = ×1.00 口径 0-byte | G25_ACCEPTANCE_MAP M-b | closed-go |
| G25-N3 | 战役全链零降级 | G25.1 新增 | M-c 真跑后争议时重判 | closed-go | M-c 全绿（G24 链递归 verify-latest + 守卫三件 + strict 预算全量） | evidence/g25_m_c_campaign_full_chain_no_regression_*.json | 重判条件 = 链集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G25_ACCEPTANCE_MAP M-c | closed-go |
| G25-N4 | 战役承接锚归档闭集 | G25.1 新增 | M-d 真跑后争议时重判 | closed-go | M-d 全绿（七期 15 行 + RD 八条 + 清册十二行引用完整性机核） | evidence/g25_m_d_campaign_handover_ledger_*.json | 重判条件 = 归档完整性争议时只追加程序扩表；兜底 = 各期 P2 表原始锚维持 | G25_ACCEPTANCE_MAP M-d | closed-go |
| G25-N5 | 旧门零降级闭集 | G25.1 新增 | M-e 真跑后争议时重判 | closed-go | M-e 全绿（G24 链 verify-latest 零降级 + g25_ 前缀不抢 latest） | evidence/g25_m_e_closed_gate_no_regression_*.json | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G25_ACCEPTANCE_MAP M-e | closed-go |
