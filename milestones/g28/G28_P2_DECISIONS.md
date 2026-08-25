<!-- Assisted-by: Cursor Agent（G28.5 P2 穷举波） -->
# G28_P2_DECISIONS — G28.5 P2 穷举决策表（v1.0 2026-08-25）

> **状态**：G28.5 收口前置定稿。**穷举闭集 8 行零空行** = §1 三行 + §3 五行；§2 open RD 八条映射（登记面）。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `maintain-no-go` / `maintain-defer` / `maintain-open` / `maintain-blocked` / `defer-to-G29+` / `strategic_override`。
> **候选表 0-byte**：[G28_CANDIDATE_DECISIONS.md](G28_CANDIDATE_DECISIONS.md) 裁决字面不回写，本表为终态穷举。

## 1. 上游承接三行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M100-high | ReSTIR 高档 reservoir device 化窗 | G28.1 go → G28.2 M-a/M-b 兑现 | device 化/空间重用/M100 车道集成窗（RFC-0038 out-of-scope 锚）→ 低档 MegaLights 默认档维持 | closed-go | M-a implemented（kernels/g28_restir.rx 真跑对拍 p100=2.831e-6 ≤ 冻结容差 5.66e-6 + y 整数锚 20000/20000 全等 + 无偏 3σ dev=3.04e-3 + 双跑位级 + RED 臂）+ M-b 空间重用加性臂（受点重评零复刻直调冻结 merge + 聚合 3σ dev=3.54e-3 + 逐点 5σ 全过 + 方差再收益 0.899/2.063/2.733 如实登记）——锚三件中前两件命中事实登记，第三件 M100 车道集成窗不出现 | evidence/g28_m_a_restir_device_kernel_*.json + evidence/g28_m_b_restir_spatial_reuse_arm_*.json + evidence/g28_restir_spatial_arm.json | 重判条件 = M100 车道集成窗（生产 GI 车道消费需求）出现时只追加重判；兜底 = 低档 MegaLights 默认档维持（multi_light 0-byte）+ 登记≠车道锚整体兑现 | 本表 §1 + registry/deferred.json RD-040 history G28.3 行 | closed-go（M-a/M-b 兑现：device 化 + 空间重用两件 implemented） |
| M52 | SER workload 重判窗 | G28.1 go → G28.3 M-c 兑现 | RT pipeline/SBT 宿主车道出现（RD-040 分项 reeval_anchor）→ 语言层不加 SER 原语维持 | closed-go | 重判窗兑现：两半盘点——capability 现势 available（G21 三 token 在案 + 新鲜 vulkaninfo 复测 available 零漂移）+ workload 零实现（manifest 5 条 + M50 库面底座不混同）→ maintain-defer（单半命中不得改判，G21 终判先例） | milestones/g28/g28_m52_rd040_workload_rejudgment.json + evidence/g28_m_c_m52_rd040_workload_rejudgment_20260825T063849Z.json | 重判条件 = 两半全齐方改判（SER 语言原语须独立 Full RFC）；兜底 = maintain-defer + 语言层不加 SER 原语字面 0-byte | 本表 §1 + G28_ACCEPTANCE_MAP M-c | closed-go（重判窗兑现：maintain-defer，两半 1/2） |
| RD-034 | spirv-cross RT 消费路径上游复查窗 | G28.1 go → G28.3 M-d 兑现 | spirv-cross SPV_KHR_ray_tracing 消费路径或 LLVM A 路解锁（G21.3 探针复查在案）→ 维持 blocked | closed-go | 复查窗兑现：探针真跑退出码判定 rc=0（spirv-cross 仍拒 raygen，HLSL builtin 5319 谱系）→ maintain-blocked 分支（blocked 证据新鲜）；②分支零检测声明在档 | evidence/g28_m_d_rd034_upstream_recheck_20260825T063852Z.json + registry/deferred.json RD-034 history G28.3 行 | 重判条件 = 探针意外成功（exit 0 翻 1 语义反转）时复评启动；兜底 = 维持 blocked + 探针恒跑防静默腐烂 | 本表 §1 + registry/deferred.json RD-034 | closed-go（复查窗兑现：maintain-blocked 新鲜） |

## 2. open RD 逐条映射（八条口径；登记面）

| RD | title（摘要） | 条目级 status | G28.5 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | M-d 承载兑现（探针新鲜 maintain-blocked + history G28.3 只追加） | M-d | 上游仍拒，维持 blocked 诚实 | 本表 §2 + registry/deferred.json |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open（G27 HZB 分项兑现在案；其余分项 G30 尾锚窗） | 无 | G30 收官期承接 | 本表 §2 |
| RD-040 | 光照 P3+ | open | M-c 承载兑现（M100-high 分项两件兑现 + M52 maintain-defer + 五分项全维持 defer + history G28.3 只追加） | M-a/M-b/M-c | 分项兑现不构成条目 close | 本表 §2 + registry/deferred.json |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open（G29 slab device 集成窗在案） | 无 | G29 材质期承接 | 本表 §2 |
| RD-042 | 可微物理研究轨 | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-044 | 物理 P3+ | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-045 | digest 漂移修复 | open | 维持 open（G26.3 新鲜窗在案；G30 终审窗复核） | 无 | 三件未齐不冒充 close | 本表 §2 |

## 3. G28 期内五行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G28-N1 | ReSTIR device kernel 实现车道 | G28.1 新增 → G28.2 兑现 | M-a 真跑后争议时重判 | closed-go | M-a 十 facts 全绿（SPV/标定位级/budget 程序产/tol 上界/y 锚/对拍/3σ/双跑/RED 臂/冻结 0-byte） | evidence/g28_m_a_restir_device_kernel_*.json | 重判条件 = 对拍面争议时只追加程序重判；兜底 = 随机带单源 + 容差程序产纪律维持 | G28_ACCEPTANCE_MAP M-a | closed-go（M-a） |
| G28-N2 | 空间重用加性臂 | G28.1 新增 → G28.2 兑现 | M-b 真跑后争议时重判 | closed-go | M-b 六 facts 全绿（聚合 3σ + 逐点 5σ + 诊断表 64 行 + 方差收益登记 + 双跑 + 冻结 0-byte） | evidence/g28_m_b_restir_spatial_reuse_arm_*.json | 重判条件 = 无偏面争议时只追加程序重判；兜底 = 受点重评零复刻纪律维持 | G28_ACCEPTANCE_MAP M-b | closed-go（M-b） |
| G28-N3 | M52/RD-040 盘点面 | G28.1 新增 → G28.3 兑现 | M-c 真跑后争议时重判 | closed-go | M-c 七 facts 全绿（在案盘点 + 新鲜复测三态 + workload manifest + 合取判定 + 五分项映射表 + history + append-only 机核） | milestones/g28/g28_m52_rd040_workload_rejudgment.json | 重判条件 = 盘点面争议时只追加扩面；兜底 = maintain-defer 诚实终态维持 | G28_ACCEPTANCE_MAP M-c | closed-go（M-c） |
| G28-N4 | RD-034 探针复查面 | G28.1 新增 → G28.3 兑现 | M-d 真跑后争议时重判 | closed-go | M-d 六 facts 全绿（探针真跑 + 门态映射分支 + status/history + append-only + ②零检测声明） | evidence/g28_m_d_rd034_upstream_recheck_20260825T063852Z.json | 重判条件 = 探针语义反转时复评启动；兜底 = maintain-blocked 诚实终态维持 | G28_ACCEPTANCE_MAP M-d | closed-go（M-d） |
| G28-N5 | G27 链回归守护与 soak 七车道扩容 | G28.1 新增 → G28.4/G28.5 兑现 | M-e 真跑后争议时重判 | closed-go | M-e 六 facts 全绿（G27 两门 verify-latest + g28_ 前缀零抢占）；soak 七车道扩容（restir device --probe 快车道入轮换）soak 门 491 承载 | evidence/g28_m_e_closed_gate_no_regression_20260825T063854Z.json + ci/g28_stabilization_soak.py 七车道字面 | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G28_ACCEPTANCE_MAP M-e | closed-go（M-e） |

## 4. 汇总

closed-go §1 三行 + §3 五行 = 8 行穷举闭集零空行；零 defer 行（本期四面全兑现：ReSTIR device kernel implemented + 空间重用 implemented + M52 maintain-defer 重判 + RD-034 maintain-blocked 复查）；§2 open RD 八条维持 open（G29/G30 各期承接窗在案）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G28.5 定稿：8 行穷举闭集。 |
