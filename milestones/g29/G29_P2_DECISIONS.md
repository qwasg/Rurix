<!-- Assisted-by: Cursor Agent（G29.5 P2 穷举波） -->
# G29_P2_DECISIONS — G29.5 P2 穷举决策表（v1.0 2026-08-25）

> **状态**：G29.5 收口前置定稿。**穷举闭集 7 行零空行** = §1 两行 + §3 五行；§2 open RD 八条映射（登记面）。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `maintain-no-go` / `maintain-defer` / `maintain-open` / `maintain-blocked` / `defer-to-G30+` / `strategic_override`。
> **候选表 0-byte**：[G29_CANDIDATE_DECISIONS.md](G29_CANDIDATE_DECISIONS.md) 裁决字面不回写，本表为终态穷举。

## 1. 上游承接两行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| RD-041-slab | slab device kernel/侧表集成波 | G29.1 go → G29.2 M-a/M-b 兑现 | device kernel/侧表集成波（RFC-0039 out-of-scope）→ closure 单层生产面 0-byte 维持 | closed-go | M-a implemented（kernels/g29_slab.rx 真跑对拍 p100=1.192e-7〔恰 f32@1.0 一 ULP〕≤ 冻结容差 2.384e-7 + 角点 rc=ab=1 device 位级 1.0〔修法 A 兑现〕+ 有限性一等断言 + 白炉行 dev=1.19e-7 如实登记 + 能量上界 + 双跑位级 + RED 臂 + material/ 整目录 0-byte）+ M-b 侧表 16 槽 implemented（逐槽 p100=3.68e-8 + 逐槽白炉互核双端登记 + MaterialClosure 32B 零触碰 + 生产侧表零挂接） | evidence/g29_m_a_slab_device_kernel_*.json + evidence/g29_m_b_slab_side_table_arm_*.json + evidence/g29_slab_side_table_arm.json | 重判条件 = 生产集成窗（closure/侧表转正需求）出现时只追加重判；兜底 = closure 单层生产面 0-byte + bin-local 侧表不落资产 | 本表 §1 + registry/deferred.json RD-041 history G29.3 行 | closed-go（M-a/M-b 兑现：device kernel + 侧表两件 implemented） |
| RD-041-svt-ktx2-wg | SVT/KTX2/WG 差距表重判窗 | G29.1 go → G29.3 M-c/M-d 兑现 | 各差距表 reeval_anchor 字面 → SVT 四行/KTX2 三行 defer + WG not-available 实测维持 | closed-go | 重判窗兑现：M-c 七行逐锚重判全维持 defer（常量 pattern 表 + 锚派生映射零实现实测，verdict=maintain-defer-seven-rows）+ M-d WG 新鲜复测 absent 维持 not-available〔与 G22 在案一致零漂移〕+ DGC 三扩展复测全 true 互核 + FSR 3.1.5 maintain 盘点（vendor_upscale 0-byte） | milestones/g29/g29_svt_ktx2_rejudgment.json + evidence/g29_m_c_svt_ktx2_gap_rejudgment_20260825T084220Z.json + evidence/g29_m_d_wg_dgc_capability_recheck_20260825T084223Z.json | 重判条件 = 各行 reeval_anchor 命中或 WG 扩展 present 翻转时启动；兜底 = 七行维持 defer + not-available 维持 + DDS 链维持 | 本表 §1 + registry/deferred.json RD-041 | closed-go（重判窗兑现：七行 maintain-defer + WG not-available 维持） |

## 2. open RD 逐条映射（八条口径；登记面）

| RD | title（摘要） | 条目级 status | G29.5 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open（G28 复查 maintain-blocked 在案；G30 尾锚窗） | 无 | 上游仍拒 | 本表 §2 |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open（G27 HZB 分项兑现在案；G30 尾锚窗） | 无 | G30 收官期承接 | 本表 §2 |
| RD-040 | 光照 P3+ | open | 维持 open（G28 M100-high 两件兑现在案；G30 尾锚窗） | 无 | G30 收官期承接 | 本表 §2 |
| RD-041 | 材质/流送/时域 P3+ | open | M-a/M-b/M-c/M-d 承载兑现（slab 分项 implemented + 七行重判维持 defer + WG 复测维持 + history G29.3 只追加） | M-a/M-b/M-c/M-d | 分项兑现不构成条目 close | 本表 §2 + registry/deferred.json |
| RD-042 | 可微物理研究轨 | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-044 | 物理 P3+ | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-045 | digest 漂移修复 | open | 维持 open（G26.3 新鲜窗在案；G30 终审窗复核） | 无 | 三件未齐不冒充 close | 本表 §2 |

## 3. G29 期内五行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G29-N1 | slab device kernel 实现车道 | G29.1 新增 → G29.2 兑现 | M-a 真跑后争议时重判 | closed-go | M-a 十一 facts 全绿（含有限性一等断言与角点位级还原） | evidence/g29_m_a_slab_device_kernel_*.json | 重判条件 = 对拍面争议时只追加程序重判；兜底 = 修法 A 分母安全化纪律维持 | G29_ACCEPTANCE_MAP M-a | closed-go（M-a） |
| G29-N2 | 侧表供参加性臂 | G29.1 新增 → G29.2 兑现 | M-b 真跑后争议时重判 | closed-go | M-b 六 facts 全绿（防混淆零挂接 + 冻结面双面机核） | evidence/g29_m_b_slab_side_table_arm_*.json | 重判条件 = 侧表面争议时只追加程序重判；兜底 = bin-local 不落资产纪律维持 | G29_ACCEPTANCE_MAP M-b | closed-go（M-b） |
| G29-N3 | SVT/KTX2 七行 reeval 协议 | G29.1 新增 → G29.3 兑现 | M-c 真跑后争议时重判 | closed-go | M-c 六 facts 全绿（F6 三件常量表纪律 + g22 两表 0-byte + append-only 机核） | milestones/g29/g29_svt_ktx2_rejudgment.json | 重判条件 = 检索面闭集争议时只追加扩面；兜底 = 七行维持 defer 零冒充 | G29_ACCEPTANCE_MAP M-c | closed-go（M-c） |
| G29-N4 | WG/DGC capability 复测面 | G29.1 新增 → G29.3 兑现 | M-d 真跑后争议时重判 | closed-go | M-d 六 facts 全绿（三态闭集 + 互核 + FSR 盘点 + probe 源 0-byte） | evidence/g29_m_d_wg_dgc_capability_recheck_20260825T084223Z.json | 重判条件 = WG present 翻转时复评启动；兜底 = not-available 维持诚实终态 | G29_ACCEPTANCE_MAP M-d | closed-go（M-d） |
| G29-N5 | G28 链回归守护与 soak 八车道扩容 | G29.1 新增 → G29.4/G29.5 兑现 | M-e 真跑后争议时重判 | closed-go | M-e 六 facts 全绿（G28 两门 verify-latest + g29_ 前缀零抢占）；soak 八车道扩容（slab device --probe 入轮换）soak 门 507 承载 | evidence/g29_m_e_closed_gate_no_regression_20260825T084226Z.json + ci/g29_stabilization_soak.py 八车道字面 | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G29_ACCEPTANCE_MAP M-e | closed-go（M-e） |

## 4. 汇总

closed-go §1 两行 + §3 五行 = 7 行穷举闭集零空行；零 defer 行（本期四面全兑现：slab device kernel implemented + 侧表 implemented + 七行 maintain-defer 重判 + WG not-available 维持复测）；§2 open RD 八条维持 open（G30 尾锚重判窗在案）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G29.5 定稿：7 行穷举闭集。 |
