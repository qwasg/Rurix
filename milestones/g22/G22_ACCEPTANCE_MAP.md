<!-- Assisted-by: Cursor Agent（G22.1 治理波） -->
# G22_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G22.1 治理交付物；事实源为 [G22_CONTRACT.md](G22_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 381/382/383。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g22.p0.m_a.slab_material_host_realization` | `py -3 ci/g22_slab_material_host_realization_smoke.py --gate g22.p0.m_a.slab_material_host_realization` | `milestones/g22/g22_m_a_slab_material_host_realization_evidence_schema.json` | Substrate 类双层 slab 能量守恒闭合 host 参考臂实现（无穷弹跳解析闭式 + farther 级数对拍）；白炉能量守恒硬不变量（白炉恒等 + 全参数域 R_total ≤ 1 + 对 base 反照率单调 + 闭式↔级数+尾和恒等式 1e-9）；层参数 lerp 连续性；双跑位级确定性；既有 material/closure 单层面 0-byte | **G22.2** | post-interlock actual-next-free allocation |
| **M-b** | `g22.p0.m_b.svt_disposition` | `py -3 ci/g22_svt_disposition_smoke.py --gate g22.p0.m_b.svt_disposition` | `milestones/g22/g22_m_b_svt_disposition_evidence_schema.json` | RD-041 SVT 分项评估 disposition：streaming/ 页式现面 vs 虚拟纹理页表差距闭集登记 g22_svt_gap.json；go/no-go/defer 均合法终态 | **G22.2** | post-interlock actual-next-free allocation |
| **M-c** | `g22.p0.m_c.ktx2_basisu_disposition` | `py -3 ci/g22_ktx2_basisu_disposition_smoke.py --gate g22.p0.m_c.ktx2_basisu_disposition` | `milestones/g22/g22_m_c_ktx2_basisu_disposition_evidence_schema.json` | RD-041 KTX2-BasisU 分项评估 disposition：G11.3 DDS 转码链现面盘点 + 转码器差距/收益登记 g22_ktx2_disposition.json；go/no-go/defer 均合法终态 | **G22.3** | post-interlock actual-next-free allocation |
| **M-d** | `g22.p0.m_d.work_graphs_fsr_reeval_disposition` | `py -3 ci/g22_work_graphs_fsr_reeval_disposition_smoke.py --gate g22.p0.m_d.work_graphs_fsr_reeval_disposition` | `milestones/g22/g22_m_d_work_graphs_fsr_reeval_disposition_evidence_schema.json` | RD-041 Work Graphs + FSR 分项重评：Work Graphs Vulkan 车道设备实测（vulkaninfo AMDX/DGC 扩展枚举取证落 g22_work_graphs_probe_results.json）+ DGC 现面盘点（dgc.rs M102）+ FSR 3.1.5 第二超分臂重评维持登记；not-available/maintain 均合法终态 | **G22.3** | post-interlock actual-next-free allocation |
| **M-e** | `g22.p0.m_e.closed_gate_no_regression` | `py -3 ci/g22_closed_gate_no_regression_smoke.py --gate g22.p0.m_e.closed_gate_no_regression` | `milestones/g22/g22_m_e_closed_gate_no_regression_evidence_schema.json` | G21 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g22_` 前缀不抢 latest | **G22.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G22.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G21 defer-to-G22+ 六行本波重评：SAFE-GPU/M127/M114-strand/M118-hdr-cal/M125-adopt3/G10-N6 → defer-to-G23+（战役排程点名期别，见 G22_CANDIDATE_DECISIONS §1）；RD-041 四分项 = 本期 M-b/M-c/M-d 窗内处置。

---

## 4. 双向一致声明

本表 §1 五行与 G22_CONTRACT.md §4.2 逐字相等；key 命名空间 `g22.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G22.1 治理覆盖

```text
g22.wave.1.acceptance_map         步骤 381
  py -3 ci/g22_acceptance_map_check.py --gate g22.wave.1.acceptance_map
g22.wave.1.candidate_decisions    步骤 382
  py -3 ci/g22_candidate_decisions_check.py --gate g22.wave.1.candidate_decisions
g22.gov.implementation_interlock  步骤 383
  py -3 ci/g22_interlock_check.py --gate g22.gov.implementation_interlock
```

---

## 6. G22.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g22_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G19-G25」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g22-closed`。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G22.1 初版：五 P0 行冻结。 |
