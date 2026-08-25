<!-- Assisted-by: Cursor Agent（G27.5 P2 穷举波） -->
# G27_P2_DECISIONS — G27.5 P2 穷举决策表（v1.0 2026-08-25）

> **状态**：G27.5 收口前置定稿。**穷举闭集 8 行零空行** = §1 三行 + §3 五行；§2 open RD 八条映射（登记面）。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `maintain-no-go` / `maintain-defer` / `maintain-open` / `defer-to-G28+` / `strategic_override`。
> **候选表 0-byte**：[G27_CANDIDATE_DECISIONS.md](G27_CANDIDATE_DECISIONS.md) 裁决字面不回写，本表为终态穷举。

## 1. 上游承接三行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M61 | mesh shader 第三光栅路径重判窗 | G27.1 go → G27.2 M-b 兑现 | cluster P4 差距闭集清零 + HZB device 化落地后只追加再判 → VS 光栅唯一 fallback 维持 | closed-go | 重判窗兑现：三项机器盘点 1/3（①HZB device 化半边本期命中〔M-a implemented〕+ ②P4 四行仍全 open + ③HW measured 证据零命中〔manifest 3 pattern〕）→ maintain-no-go 只追加（防冒充硬线：①命中不得单独启动），RFC-0034 重判表 G27.2 行在案 | evidence/g27_m_b_m61_mesh_shader_rejudgment_20260825T044918Z.json + rfcs/0034 重判表 | 重判条件 = 三项闭集全齐时重判程序启动；兜底 = maintain-no-go + VS 光栅唯一 fallback 维持字面 0-byte | 本表 §1 + rfcs/0034 重判表 | closed-go（重判窗兑现：maintain-no-go，三项 1/3） |
| M98-l4 | L4 Far Field 档重判窗 | G27.1 go → G27.3 M-d 兑现 | HLOD proxy 追踪 device 腿落地 + L4 计数器接入选档 evidence → L1/L2/L3 三级链维持 | closed-go | 重判窗兑现：两半树内实测 0/2（①device 腿零实现〔manifest 5 pattern〕+ ②三处 fail-closed 入口在位〔check_l4_trigger 恒 NotTriggered/l4_serve 恒 Err/L4 槽位恒零〕）→ 维持三级链；接口面就绪在案但不构成半命中；RXS-0396/0359 不混同检索排除 | evidence/g27_m_d_hlod_l4_counter_rejudgment_20260825T045037Z.json | 重判条件 = 两半全齐方改判（锚「+」合取）；兜底 = L1/L2/L3 三级链维持 + fail-closed 入口不动 | 本表 §1 + G27_ACCEPTANCE_MAP M-d | closed-go（重判窗兑现：maintain 三级链，两半 0/2） |
| RD-039-mesh | HZB device 化 + cluster P4 + mesh 再判链（RD-039 分项） | G27.1 go → G27.2/G27.3 M-a/M-c 兑现 | HZB device 化 + cluster P4 差距闭集（G20 落档）+ mesh shader 再判链 → 长线评估维持 open | closed-go | M-a HZB device kernel **implemented**（mips 9 级双臂位级全等 + 800×2 判定序列全等 + 零假阳性 + 双跑位级 + tamper 双臂检出 + 231/800 剔除与 host 一致）+ M-c 四行重判全维持 open（P4-2 依赖解除事实登记≠兑现）+ RD-039 history G27.3 只追加（断档口径注明） | evidence/g27_m_a_hzb_device_kernel_20260825T044714Z.json + milestones/g27/g27_cluster_p4_rejudgment.json + registry/deferred.json RD-039 history | 重判条件 = P4 各行现面兑现或反馈链出现时只追加重判；兜底 = RD-039 维持 open + g20 差距表原文 0-byte | 本表 §1 + registry/deferred.json RD-039 | closed-go（M-a implemented + M-c 四行维持 open） |

## 2. open RD 逐条映射（八条口径；登记面）

| RD | title（摘要） | 条目级 status | G27.5 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open（G28 重判窗在案） | 无 | G28 光照期承接 | 本表 §2 |
| RD-039 | 虚拟化几何 P3+ | open | M-a/M-c 承载兑现（HZB device 化分项 implemented + P4 四行重判维持 open + history G27.3 只追加） | M-a/M-c | 分项兑现不构成条目 close（其余分项 open） | 本表 §2 + registry/deferred.json |
| RD-040 | 光照 P3+ | open | 维持 open（G28 ReSTIR device 化窗在案） | 无 | G28 光照期承接 | 本表 §2 |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open（G29 slab device 集成窗在案） | 无 | G29 材质期承接 | 本表 §2 |
| RD-042 | 可微物理研究轨 | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-044 | 物理 P3+ | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-045 | digest 漂移修复 | open | 维持 open（G26.3 新鲜窗 6/6 零漂移 + 三件 0/3 在案；G30 终审窗复核） | 无 | 三件未齐不冒充 close | 本表 §2 |

## 3. G27 期内五行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G27-N1 | HZB device kernel 实现车道 | G27.1 新增 → G27.2 兑现 | M-a 真跑后争议时重判 | closed-go | M-a 七 facts 全绿（SPV/spirv-val + mips 位级 + 序列全等 + 零假阳性 + 双跑位级 + tamper 双臂 + geometry 冻结 0-byte） | evidence/g27_m_a_hzb_device_kernel_20260825T044714Z.json | 重判条件 = 对拍面争议时只追加程序重判；兜底 = 零容差协议纪律维持 | G27_ACCEPTANCE_MAP M-a | closed-go（M-a） |
| G27-N2 | M61 两半盘点面 | G27.1 新增 → G27.2 兑现 | M-b 真跑后争议时重判 | closed-go | M-b 六 facts 全绿（三项盘点 + manifest 必填 + RFC-0034 只追加 + fallback 字面维持） | evidence/g27_m_b_m61_mesh_shader_rejudgment_20260825T044918Z.json | 重判条件 = 盘点面争议时只追加程序重判；兜底 = maintain-no-go 诚实终态维持 | G27_ACCEPTANCE_MAP M-b | closed-go（M-b） |
| G27-N3 | cluster P4 四行 reeval 协议 | G27.1 新增 → G27.3 兑现 | M-c 真跑后争议时重判 | closed-go | M-c 六 facts 全绿（g20 表 0-byte + 依赖解除登记 + 四行 cluster 专属检索维持 open + history 只追加 + append-only 机核） | milestones/g27/g27_cluster_p4_rejudgment.json | 重判条件 = 检索面闭集争议时只追加扩面；兜底 = 四行维持 open 零冒充 | G27_ACCEPTANCE_MAP M-c | closed-go（M-c） |
| G27-N4 | M98-l4 条件核验面 | G27.1 新增 → G27.3 兑现 | M-d 真跑后争议时重判 | closed-go | M-d 七 facts 全绿（device 腿零实现 manifest + 三处 fail-closed 入口实测 + 接口面就绪盘点〔双代字段兼容判读〕+ 合取判定 + 边界不混同） | evidence/g27_m_d_hlod_l4_counter_rejudgment_20260825T045037Z.json | 重判条件 = 两半任一新证出现时只追加登记（改判须全齐）；兜底 = 三级链维持 | G27_ACCEPTANCE_MAP M-d | closed-go（M-d） |
| G27-N5 | G26 链回归守护与 soak 六车道扩容 | G27.1 新增 → G27.4/G27.5 兑现 | M-e 真跑后争议时重判 | closed-go | M-e 六 facts 全绿（G26 两门 verify-latest 全绿 + g27_ 前缀零抢占）；soak 六车道扩容（framegen/hzb 两 device 探针入轮换）soak 门 475 承载 | evidence/g27_m_e_closed_gate_no_regression_20260825T044926Z.json + ci/g27_stabilization_soak.py 六车道字面 | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G27_ACCEPTANCE_MAP M-e | closed-go（M-e） |

## 4. 汇总

closed-go §1 三行 + §3 五行 = 8 行穷举闭集零空行；零 defer 行（本期四面全兑现：HZB device kernel implemented + M61 maintain-no-go 重判 + P4 四行维持 open 重判 + M98-l4 maintain 三级链重判）；§2 open RD 八条维持 open（G28/G29/G30 各期承接窗在案）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G27.5 定稿：8 行穷举闭集。 |
