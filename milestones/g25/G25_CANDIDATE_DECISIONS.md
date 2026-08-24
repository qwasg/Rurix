<!-- Assisted-by: Cursor Agent（G25.1 治理波） -->
# G25_CANDIDATE_DECISIONS — G25.1 候选决策表（v1.0 2026-08-24）

> **状态**：G25.1 治理波定稿。**候选闭集 7 行零空行** = §1 两行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G26+` / `strategic_override`。**G25 即本期（战役收官）：defer-to-G25+ 不再合法，defer 合法值 = defer-to-G26+（归档锚承接）**。

## 1. G24 defer-to-G25+ 行 + fps 终判锚行 = 2 行逐行转引终态裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| SAFE-GPU | Safe GPU Operator Platform | G24.5 defer-to-G25+ → G25.1 go | G25 战役终审窗归档处置 → G9~G24 零交付维持 | go | G25 即归档窗；M-d 承载（战役承接锚归档闭集行——独立期立项判据核验后归档 G26+ 锚） | milestones/g24/G24_P2_DECISIONS.md §1 SAFE-GPU | 重判条件 = M-d 归档闭集争议时只追加程序重判；兜底 = G9~G25 零交付维持 + G26+ 归档锚 | 本表 §1 + G25_ACCEPTANCE_MAP M-d | go（M-d 承载） |
| G17-MD-F1 | fps_parity_deficit bistro/t100/dlss_sr 终判链 | G19.4 M-d「终判归 G25」字面 → G25.1 go | ≥1.00 → 18/18；物理不可达 → 维持未达标登记不冒充；兜底 = G15 同源 | go | G25 即终判窗；M-b 承载（最新 18 格定盘 + 性能面 0-byte 机核 + 焦点格新鲜单测） | milestones/g19/G19_P2_DECISIONS.md §3 G19-N4 + milestones/g18/G18_P2_DECISIONS.md §1 G17-MD-F1 | 重判条件 = M-b 真跑后争议时只追加程序重判；兜底 = 17/18 诚实红终判合法（G15 物理不可达定论同源） | 本表 §1 + G25_ACCEPTANCE_MAP M-b | go（M-b 承载） |

## 2. open RD 逐条映射（八条口径 + 历史清册归档联动）

| RD | title（摘要） | 条目级 status | G25.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 归档 G26+ 锚（M-d） | 无 | G21.3 复查在案 | 本表 §2 + M-d 归档表 |
| RD-039 | 虚拟化几何 P3+ | open | 归档 G26+ 锚（M-d） | 无 | G20 处置在案 | 本表 §2 + M-d 归档表 |
| RD-040 | 光照 P3+ | open | 归档 G26+ 锚（M-d） | 无 | G21 处置在案 | 本表 §2 + M-d 归档表 |
| RD-041 | 材质/流送/时域 P3+ | open | 归档 G26+ 锚（M-d） | 无 | G22 处置在案 | 本表 §2 + M-d 归档表 |
| RD-042 | 可微物理研究轨 | open | 归档 G26+ 锚（M-d） | 无 | G23 处置在案 | 本表 §2 + M-d 归档表 |
| RD-043 | wgrapier GPU 刚体 | open | 归档 G26+ 锚（M-d） | 无 | G23 处置在案 | 本表 §2 + M-d 归档表 |
| RD-044 | 物理 P3+ | open | 归档 G26+ 锚（M-d） | 无 | G23 处置在案 | 本表 §2 + M-d 归档表 |
| RD-045 | digest 漂移修复 | open | 终审窗复核（M-d：累计观察汇总登记） | 无 | G19.3 12/12 + 五期 soak 零漂移累计 | 本表 §2 + M-d 归档表 |

## 3. G25 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G25-N1 | 画质终态维持核验 | G25.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G25_CONTRACT §4.2 M-a | 重判条件 = 表面 0-byte 机核争议时只追加程序重判；兜底 = G18 M-d 达标终态维持 | G25_ACCEPTANCE_MAP M-a | go（M-a） |
| G25-N2 | fps 终判两态合法 | G25.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G25_CONTRACT §4.2 M-b | 重判条件 = 物理不可达时维持未达标登记；兜底 = ×1.00 口径 0-byte | G25_ACCEPTANCE_MAP M-b | go（M-b） |
| G25-N3 | 战役全链零降级 | G25.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G25_CONTRACT §4.2 M-c | 重判条件 = 链集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G25_ACCEPTANCE_MAP M-c | go（M-c） |
| G25-N4 | 战役承接锚归档闭集 | G25.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G25_CONTRACT §4.2 M-d | 重判条件 = 归档完整性争议时只追加程序扩表；兜底 = 各期 P2 表原始锚维持 | G25_ACCEPTANCE_MAP M-d | go（M-d） |
| G25-N5 | 旧门零降级闭集 | G25.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据承载 | G25_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G25_ACCEPTANCE_MAP M-e | go（M-e） |

## 4. 汇总

go §1 两行 + §3 五行 = 7 go 承载面；零 defer 行（战役收官——G26+ 法定输入 = M-d 归档闭集）；§2 open RD 八条归档锚化。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：7 行候选闭集。 |
