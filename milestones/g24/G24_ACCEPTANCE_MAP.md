<!-- Assisted-by: Cursor Agent（G24.1 治理波） -->
# G24_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G24.1 治理交付物；事实源为 [G24_CONTRACT.md](G24_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 413/414/415。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g24.p0.m_a.hair_strand_oit_rejudgment` | `py -3 ci/g24_hair_strand_oit_rejudgment_smoke.py --gate g24.p0.m_a.hair_strand_oit_rejudgment` | `milestones/g24/g24_m_a_hair_strand_oit_rejudgment_evidence_schema.json` | M114-strand 重判兑现：M120 七算法 OIT benchmark 裁决数据只读盘点（measured 绿件在案性核验）+ strand 档生产需求面核验（压测闭集毛发资产存在性）；两半分别登记；maintain-card-mesh/go 均合法终态 | **G24.2** | post-interlock actual-next-free allocation |
| **M-b** | `g24.p0.m_b.hdr_calibration_rejudgment` | `py -3 ci/g24_hdr_calibration_rejudgment_smoke.py --gate g24.p0.m_b.hdr_calibration_rejudgment` | `milestones/g24/g24_m_b_hdr_calibration_rejudgment_evidence_schema.json` | M118-hdr-cal 重判兑现：HDR 设备面实测（vulkaninfo 表面色彩空间枚举取证落 g24_hdr_probe_results.json）+ HDR 资产/产品需求面核验；两半分别登记；maintain-SDR/go 均合法终态 | **G24.2** | post-interlock actual-next-free allocation |
| **M-c** | `g24.p0.m_c.bistro_exterior_conversion_rejudgment` | `py -3 ci/g24_bistro_exterior_conversion_rejudgment_smoke.py --gate g24.p0.m_c.bistro_exterior_conversion_rejudgment` | `milestones/g24/g24_m_c_bistro_exterior_conversion_rejudgment_evidence_schema.json` | G10-N6 重判兑现：FBX2glTF/替代转换臂工具链在树性实测 + BistroExterior 源资产在树性核验 + 场景闭集裁决登记 g24_bistro_exterior_recheck.json；maintain-双场景闭集/go 均合法终态 | **G24.3** | post-interlock actual-next-free allocation |
| **M-d** | `g24.p0.m_d.safe_gpu_and_legacy_rd_disposition` | `py -3 ci/g24_safe_gpu_and_legacy_rd_disposition_smoke.py --gate g24.p0.m_d.safe_gpu_and_legacy_rd_disposition` | `milestones/g24/g24_m_d_safe_gpu_and_legacy_rd_disposition_evidence_schema.json` | SAFE-GPU 立项评估处置 + 历史 open RD 清册逐条重判：RD-007/011/012/014/015/026/027/030/032/033/036 十一条逐条 disposition 闭集登记 g24_legacy_rd_registry.json + 逐条 history 只追加；maintain/close/inherit 均合法终态 | **G24.3** | post-interlock actual-next-free allocation |
| **M-e** | `g24.p0.m_e.closed_gate_no_regression` | `py -3 ci/g24_closed_gate_no_regression_smoke.py --gate g24.p0.m_e.closed_gate_no_regression` | `milestones/g24/g24_m_e_closed_gate_no_regression_evidence_schema.json` | G23 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g24_` 前缀不抢 latest | **G24.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G24.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G23 defer-to-G24+ 四行本波全部 go：M114-strand → M-a；M118-hdr-cal → M-b；G10-N6 → M-c；SAFE-GPU → M-d（承接池清零窗，见 G24_CANDIDATE_DECISIONS §1）。

---

## 4. 双向一致声明

本表 §1 五行与 G24_CONTRACT.md §4.2 逐字相等；key 命名空间 `g24.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G24.1 治理覆盖

```text
g24.wave.1.acceptance_map         步骤 413
  py -3 ci/g24_acceptance_map_check.py --gate g24.wave.1.acceptance_map
g24.wave.1.candidate_decisions    步骤 414
  py -3 ci/g24_candidate_decisions_check.py --gate g24.wave.1.candidate_decisions
g24.gov.implementation_interlock  步骤 415
  py -3 ci/g24_interlock_check.py --gate g24.gov.implementation_interlock
```

---

## 6. G24.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g24_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G19-G25」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g24-closed`。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G24.1 初版：五 P0 行冻结。 |
