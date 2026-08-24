<!-- Assisted-by: Cursor Agent（G20.1 治理波） -->
# G20_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G20.1 治理交付物；事实源为 [G20_CONTRACT.md](G20_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 349/350/351。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g20.p0.m_a.hzb_occlusion_host_realization` | `py -3 ci/g20_hzb_occlusion_host_realization_smoke.py --gate g20.p0.m_a.hzb_occlusion_host_realization` | `milestones/g20/g20_m_a_hzb_occlusion_host_realization_evidence_schema.json` | HZB 层级深度金字塔遮挡剔除 host 参考臂实现（farther-of 归约金字塔 + ≤2×2 纹素窗保守测试 + reverse-Z/standard-Z 双约定）；保守零假阳性硬不变量（确定性 rect 夹具 vs 逐像素精确真值零假阳性 + 剔除率非零）；双跑位级确定性；既有 cull/visbuffer 面 0-byte | **G20.2** | post-interlock actual-next-free allocation |
| **M-b** | `g20.p0.m_b.cluster_streaming_p4_disposition` | `py -3 ci/g20_cluster_streaming_p4_disposition_smoke.py --gate g20.p0.m_b.cluster_streaming_p4_disposition` | `milestones/g20/g20_m_b_cluster_streaming_p4_disposition_evidence_schema.json` | RD-039 cluster 流送 P4 分项评估 disposition：streaming/ 现面盘点 + P4 差距闭集登记 g20_cluster_streaming_p4_gap.json；go/no-go/defer 均合法终态 | **G20.2** | post-interlock actual-next-free allocation |
| **M-c** | `g20.p0.m_c.mesh_shader_rejudgment` | `py -3 ci/g20_mesh_shader_rejudgment_smoke.py --gate g20.p0.m_c.mesh_shader_rejudgment` | `milestones/g20/g20_m_c_mesh_shader_rejudgment_evidence_schema.json` | M61 重判兑现：RFC-0034 只追加重判记录（HZB host 面兑现事实 + mesh shader 性能差 measured 证据面核验 + VS fallback 维持裁决）；maintain-no-go/go 均合法终态 | **G20.3** | post-interlock actual-next-free allocation |
| **M-d** | `g20.p0.m_d.far_field_l4_disposition` | `py -3 ci/g20_far_field_l4_disposition_smoke.py --gate g20.p0.m_d.far_field_l4_disposition` | `milestones/g20/g20_m_d_far_field_l4_disposition_evidence_schema.json` | M98-l4 重判兑现：HLOD 运行时接口面就绪核验（world/hlod.rs + g9_m111 门绿件）+ L4 计数可测性评估 + disposition 登记；实现/维持 L1/L2/L3 三级链均合法终态 | **G20.3** | post-interlock actual-next-free allocation |
| **M-e** | `g20.p0.m_e.closed_gate_no_regression` | `py -3 ci/g20_closed_gate_no_regression_smoke.py --gate g20.p0.m_e.closed_gate_no_regression` | `milestones/g20/g20_m_e_closed_gate_no_regression_evidence_schema.json` | G19 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g20_` 前缀不抢 latest | **G20.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G20.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G19 defer-to-G20+ 八行 + M61 重判锚行本波重评：M98-l4 → go（M-d 承载）；M61 → go（M-c 重判承载）；M52/SAFE-GPU/M127/M114-strand/M118-hdr-cal/M125-adopt3/G10-N6 → defer-to-G21+（战役排程点名期别，见 G20_CANDIDATE_DECISIONS §1）。

---

## 4. 双向一致声明

本表 §1 五行与 G20_CONTRACT.md §4.2 逐字相等；key 命名空间 `g20.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G20.1 治理覆盖

```text
g20.wave.1.acceptance_map         步骤 349
  py -3 ci/g20_acceptance_map_check.py --gate g20.wave.1.acceptance_map
g20.wave.1.candidate_decisions    步骤 350
  py -3 ci/g20_candidate_decisions_check.py --gate g20.wave.1.candidate_decisions
g20.gov.implementation_interlock  步骤 351
  py -3 ci/g20_interlock_check.py --gate g20.gov.implementation_interlock
```

---

## 6. G20.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g20_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G19-G25」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g20-closed`。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G20.1 初版：五 P0 行冻结。 |
