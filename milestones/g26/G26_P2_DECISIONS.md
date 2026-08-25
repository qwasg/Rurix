<!-- Assisted-by: Cursor Agent（G26.5 P2 穷举波） -->
# G26_P2_DECISIONS — G26.5 P2 穷举决策表（v1.0 2026-08-25）

> **状态**：G26.5 收口前置定稿。**穷举闭集 8 行零空行** = §1 三行 + §3 五行；§2 open RD 八条映射（不进机核，登记面）。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `maintain-no-go` / `maintain-defer` / `maintain-open` / `defer-to-G27+` / `strategic_override`。
> **候选表 0-byte**：[G26_CANDIDATE_DECISIONS.md](G26_CANDIDATE_DECISIONS.md) 裁决字面不回写，本表为终态穷举。

## 1. 上游承接三行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G13-N7 | FG/MFG device kernel 车道 | G26.1 go → G26.2 M-a/M-b 兑现 | device kernel 车道重判（RFC-0036 §1.5）→ host 参考臂 0-byte 维持 | closed-go | M-a implemented：kernels/g26_framegen.rx 真跑对拍 p100=3.576e-7 ≤ 冻结容差 7.153e-7 + SSIM 全帧严格胜 frame-hold + device 双跑位级 + 双 RED 臂检出 + spirv-val；M-b 三档帧时 measured 登记（60.64/60.71/60.22ms）+ FgAccounting 双恒等式 + 性能面 0-byte | evidence/g26_m_a_framegen_device_kernel_20260825T030005Z.json + evidence/g26_m_b_framegen_device_bench_accounting_20260825T030047Z.json | 重判条件 = device 车道生产集成窗出现时只追加程序重判；兜底 = bin-local adapter 加性面维持 + host 参考臂 0-byte + 生成帧禁入真渲帧率口径 0-byte | 本表 §1 + G26_CONTRACT §8 | closed-go（M-a/M-b 兑现，implemented） |
| RD-045-window | RD-045 backfill 三件重判窗 | G26.1 go → G26.3 M-c 兑现 | backfill 三件（定位+修复+Full RFC 评估）→ 累计观察零漂移维持 | closed-go | 重判窗兑现：新鲜观察窗 6/6 轮 digest 锚全中零漂移 + 三件盘点机器实测 0/3（①确证记录缺〔F5 硬线：观察窗零漂移不充①件〕②修复无法确证③Full RFC 缺）→ maintain-open 终态，不冒充 close | milestones/g26/g26_rd045_fresh_window_results.json + evidence/g26_m_c_rd045_backfill_rejudgment_20260825T030358Z.json + registry/deferred.json RD-045 history G26.3 行 | 重判条件 = 三件任一新证出现（确证记录落点/修复记录/主题 Full RFC）时只追加重判；兜底 = maintain-open-with-extended-zero-recurrence + 累计观察面扩窗（G19.3 12/12 + 本窗 6/6 + 七期 soak 零漂移） | 本表 §1 + registry/deferred.json RD-045 | closed-go（重判窗兑现：maintain-open，三件 0/3） |
| G17-MD-F1 | fps 17/18 诚实红重判窗 | G26.1 go → G26.3 M-d 兑现 | NGX 分解 profiling 或 UE 侧插桩（宿主差可分离 measured 证据）→ 未命中维持 | closed-go | 重判窗兑现：两半证据树内闭集搜索实测 0+0 命中（searched-paths manifest 6 条 pattern 逐条登记，F6 非空清单硬线）→ 维持 17/18 诚实红 carry，终判归 G30 商用终审 | evidence/g26_m_d_g17_md_f1_rejudgment_window_20260825T030400Z.json + evidence/g25_m_b_fps_parity_final_verdict_20260824T194143Z.json（ratio 0.856326 锚） | 重判条件 = 两半任一命中时重判程序启动；兜底 = 17/18 诚实红 carry（G15 物理不可达兜底同源）终判归 G30 | 本表 §1 + G26_ACCEPTANCE_MAP M-d | closed-go（重判窗兑现：maintain 17/18 诚实红 carry） |

## 2. open RD 逐条映射（八条口径；登记面）

| RD | title（摘要） | 条目级 status | G26.5 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open（G28 重判窗在案） | 无 | G21.3 复查在案，G28 光照期承接 | 本表 §2 |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open（G27 HZB device 化窗在案） | 无 | G27 几何期承接 | 本表 §2 |
| RD-040 | 光照 P3+ | open | 维持 open（G28 ReSTIR device 化窗在案） | 无 | G28 光照期承接 | 本表 §2 |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open（G29 slab device 集成窗在案）；FG/MFG 分项本期 M-a/M-b 兑现 history 联动 | M-a/M-b | G29 材质期承接 | 本表 §2 |
| RD-042 | 可微物理研究轨 | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-043 | wgrapier GPU 刚体 | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-044 | 物理 P3+ | open | 维持 open（G30 尾锚重判窗在案） | 无 | G30 收官期承接 | 本表 §2 |
| RD-045 | digest 漂移修复 | open | M-c 重判窗兑现（新鲜窗 6/6 零漂移 + 三件 0/3 → maintain-open 扩窗登记） | M-c | 三件未齐不冒充 close，history 只追加在案 | 本表 §2 + registry/deferred.json |

## 3. G26 期内五行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G26-N1 | framegen device kernel 实现车道 | G26.1 新增 → G26.2 兑现 | M-a 真跑后争议时重判 | closed-go | M-a 九 facts 全绿（对拍/位级/RED 臂/spirv-val/temporal 0-byte） | evidence/g26_m_a_framegen_device_kernel_20260825T030005Z.json | 重判条件 = 对拍面争议时只追加程序重判；兜底 = 标定容差程序产纪律维持 | G26_ACCEPTANCE_MAP M-a | closed-go（M-a） |
| G26-N2 | device 帧时与口径登记面 | G26.1 新增 → G26.2 兑现 | M-b 真跑后争议时重判 | closed-go | M-b 六 facts 全绿（三档 bench 程序产入 budget + 双恒等式 + 性能面 0-byte） | evidence/g26_m_b_framegen_device_bench_accounting_20260825T030047Z.json | 重判条件 = 帧时回归守护带超限时只追加重判；兜底 = 不构成通过线语义 0-byte | G26_ACCEPTANCE_MAP M-b | closed-go（M-b） |
| G26-N3 | RD-045 新鲜观察窗协议 | G26.1 新增 → G26.3 兑现 | M-c 真跑后争议时重判 | closed-go | M-c 八 facts 全绿（6 轮真跑 + 三件盘点 + F5 输入面隔离 + history 只追加） | milestones/g26/g26_rd045_fresh_window_results.json | 重判条件 = 窗长/口径争议时只追加程序扩窗；兜底 = maintain-open 诚实终态维持 | G26_ACCEPTANCE_MAP M-c | closed-go（M-c） |
| G26-N4 | G17-MD-F1 证据搜索面闭集 | G26.1 新增 → G26.3 兑现 | M-d 真跑后争议时重判 | closed-go | M-d 六 facts 全绿（manifest 6 条 pattern 非空 + 两半 0+0 实测 + G25 锚在档） | evidence/g26_m_d_g17_md_f1_rejudgment_window_20260825T030400Z.json | 重判条件 = 搜索面闭集争议时只追加扩面；兜底 = maintain carry 终判归 G30 | G26_ACCEPTANCE_MAP M-d | closed-go（M-d） |
| G26-N5 | G25 链回归守护与 soak 探针轮换扩容 | G26.1 新增 → G26.4/G26.5 兑现 | M-e 真跑后争议时重判 | closed-go | M-e 六 facts 全绿（G25 两门 verify-latest 全绿 + g26_ 前缀零抢占）；soak 五车道扩容（framegen device --probe 快车道入轮换）soak 门 459 承载 | evidence/g26_m_e_closed_gate_no_regression_20260825T030403Z.json + ci/g26_stabilization_soak.py 五车道字面 | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G26_ACCEPTANCE_MAP M-e | closed-go（M-e） |

## 4. 汇总

closed-go §1 三行 + §3 五行 = 8 行穷举闭集零空行；零 defer 行（本期四面全兑现：device kernel implemented + 帧时登记 + RD-045 maintain-open 重判 + G17-MD-F1 maintain carry）；§2 open RD 八条维持 open（G27/G28/G29/G30 各期承接窗在案）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G26.5 定稿：8 行穷举闭集。 |
