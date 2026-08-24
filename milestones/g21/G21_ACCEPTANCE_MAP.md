<!-- Assisted-by: Cursor Agent（G21.1 治理波） -->
# G21_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G21.1 治理交付物；事实源为 [G21_CONTRACT.md](G21_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 365/366/367。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g21.p0.m_a.restir_high_reservoir_realization` | `py -3 ci/g21_restir_high_reservoir_realization_smoke.py --gate g21.p0.m_a.restir_high_reservoir_realization` | `milestones/g21/g21_m_a_restir_high_reservoir_realization_evidence_schema.json` | ReSTIR DI 高档 reservoir host 参考臂实现（WRS/RIS 无偏估计 + 时域 reservoir 合并 M-cap）；程序产判据（无偏 3σ 检验 + 等验证预算方差收益 var(uniform)/var(RIS) > 2 + 时域再收益 > 1.2，均 measured 禁手写）；双跑位级确定性；M100 低档生产默认面 0-byte（multi_light.rs 与其 fail-closed 登记面不接线） | **G21.2** | post-interlock actual-next-free allocation |
| **M-b** | `g21.p0.m_b.ser_capability_disposition` | `py -3 ci/g21_ser_capability_disposition_smoke.py --gate g21.p0.m_b.ser_capability_disposition` | `milestones/g21/g21_m_b_ser_capability_disposition_evidence_schema.json` | M52 SER 重判兑现：rt.ser 设备 capability 实测（vulkaninfo 扩展枚举取证落 g21_ser_capability_probe_results.json）+ 高分歧 RT workload 宿主车道核验（RT pipeline/SBT 车道存在性）；capability/workload 两半分别登记；maintain-defer/go 均合法终态 | **G21.2** | post-interlock actual-next-free allocation |
| **M-c** | `g21.p0.m_c.rd040_subitem_disposition` | `py -3 ci/g21_rd040_subitem_disposition_smoke.py --gate g21.p0.m_c.rd040_subitem_disposition` | `milestones/g21/g21_m_c_rd040_subitem_disposition_evidence_schema.json` | RD-040 分项处置闭集：SMRT/世界辐射缓存演进/NRD 降噪/OMM/RT pipeline+SBT 五分项 disposition 登记 g21_rd040_subitem_registry.json + RD-040 history 只追加；go/no-go/defer 均合法终态 | **G21.3** | post-interlock actual-next-free allocation |
| **M-d** | `g21.p0.m_d.rd034_upstream_recheck` | `py -3 ci/g21_rd034_upstream_recheck_smoke.py --gate g21.p0.m_d.rd034_upstream_recheck` | `milestones/g21/g21_m_d_rd034_upstream_recheck_evidence_schema.json` | RD-034 上游复查兑现：blocked 恒跑探针真跑（ci/meshrt_probe_smoke.py --verify-latest 或 --gate）+ 复查结论 RD-034 history 只追加；解锁/维持 blocked 均合法诚实终态 | **G21.3** | post-interlock actual-next-free allocation |
| **M-e** | `g21.p0.m_e.closed_gate_no_regression` | `py -3 ci/g21_closed_gate_no_regression_smoke.py --gate g21.p0.m_e.closed_gate_no_regression` | `milestones/g21/g21_m_e_closed_gate_no_regression_evidence_schema.json` | G20 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g21_` 前缀不抢 latest | **G21.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G21.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G20 defer-to-G21+ 七行 + M100-high 重判锚行本波重评：M100-high → go（M-a 承载）；M52 → go（M-b 重判承载）；SAFE-GPU/M127/M114-strand/M118-hdr-cal/M125-adopt3/G10-N6 → defer-to-G22+（战役排程点名期别，见 G21_CANDIDATE_DECISIONS §1）。

---

## 4. 双向一致声明

本表 §1 五行与 G21_CONTRACT.md §4.2 逐字相等；key 命名空间 `g21.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G21.1 治理覆盖

```text
g21.wave.1.acceptance_map         步骤 365
  py -3 ci/g21_acceptance_map_check.py --gate g21.wave.1.acceptance_map
g21.wave.1.candidate_decisions    步骤 366
  py -3 ci/g21_candidate_decisions_check.py --gate g21.wave.1.candidate_decisions
g21.gov.implementation_interlock  步骤 367
  py -3 ci/g21_interlock_check.py --gate g21.gov.implementation_interlock
```

---

## 6. G21.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g21_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G19-G25」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g21-closed`。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G21.1 初版：五 P0 行冻结。 |
