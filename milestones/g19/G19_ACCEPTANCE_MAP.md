<!-- Assisted-by: Cursor Agent（G19.1 治理波） -->
# G19_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G19.1 治理交付物；事实源为 [G19_CONTRACT.md](G19_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 333/334/335。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g19.p0.m_a.frame_generation_host_realization` | `py -3 ci/g19_frame_generation_host_realization_smoke.py --gate g19.p0.m_a.frame_generation_host_realization` | `milestones/g19/g19_m_a_frame_generation_host_realization_evidence_schema.json` | FG/MFG 独立层 host 参考臂实现（mv 后向双向 warp + 遮挡感知混合 + MFG ×2/×3/×4 档）；插帧质量程序产对照阈（interp SSIM > frame-hold SSIM 逐帧，禁手写阈）；双跑位级确定性；真实渲染帧率口径 0-byte（presented 口径独立登记，禁混入 upscale/FG ratio）；默认臂 Stage A digest 锚 18 格零漂移（g14_3_pipeline_perf 本期 0-byte） | **G19.2** | post-interlock actual-next-free allocation |
| **M-b** | `g19.p0.m_b.frame_generation_vendor_disposition` | `py -3 ci/g19_frame_generation_vendor_disposition_smoke.py --gate g19.p0.m_b.frame_generation_vendor_disposition` | `milestones/g19/g19_m_b_frame_generation_vendor_disposition_evidence_schema.json` | RFC-0035 重判兑现：FSR3-FG / DLSS-G / SL-310.6.0 三 vendor 臂 disposition（integrated/rejected/not-available 均合法终态）；g19_vendor_sdk_registry.json provenance 登记；310.5.2 生产默认维持或换版程序面留痕 | **G19.2** | post-interlock actual-next-free allocation |
| **M-c** | `g19.p0.m_c.rd045_drift_observation_window` | `py -3 ci/g19_rd045_drift_observation_window_smoke.py --gate g19.p0.m_c.rd045_drift_observation_window` | `milestones/g19/g19_m_c_rd045_drift_observation_window_evidence_schema.json` | RD-045 长窗观察兑现：bistro-interior/t50/tsr_device 连续 ≥12 轮 --expect-digest 锚对拍零漂移取证 + registry history 只追加登记；close/maintain-open 均合法诚实终态 | **G19.3** | post-interlock actual-next-free allocation |
| **M-d** | `g19.p0.m_d.fps_parity_window_registration` | `py -3 ci/g19_fps_parity_window_registration_smoke.py --gate g19.p0.m_d.fps_parity_window_registration` | `milestones/g19/g19_m_d_fps_parity_window_registration_evidence_schema.json` | G17-MD-F1 重评窗登记：G14 M-d 最新 18 格 evidence 如实登记（met 计数 + 焦点格 ratio）；FG 生成帧禁计入真实渲染帧率；达标判定归 G25 终判窗 | **G19.4** | post-interlock actual-next-free allocation |
| **M-e** | `g19.p0.m_e.closed_gate_no_regression` | `py -3 ci/g19_closed_gate_no_regression_smoke.py --gate g19.p0.m_e.closed_gate_no_regression` | `milestones/g19/g19_m_e_closed_gate_no_regression_evidence_schema.json` | G18 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g19_` 前缀不抢 latest | **G19.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G19.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G18 defer-to-G19+ 九行本波重评：G13-N7 → go（M-a/M-b 承载）；M52/SAFE-GPU/M127/M98-l4/M114-strand/M118-hdr-cal/M125-adopt3/G10-N6 → defer-to-G20+（七期战役排程承接锚点名具体期别，见 G19_CANDIDATE_DECISIONS §1）。

---

## 4. 双向一致声明

本表 §1 五行与 G19_CONTRACT.md §4.2 逐字相等；key 命名空间 `g19.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G19.1 治理覆盖

```text
g19.wave.1.acceptance_map         步骤 333
  py -3 ci/g19_acceptance_map_check.py --gate g19.wave.1.acceptance_map
g19.wave.1.candidate_decisions    步骤 334
  py -3 ci/g19_candidate_decisions_check.py --gate g19.wave.1.candidate_decisions
g19.gov.implementation_interlock  步骤 335
  py -3 ci/g19_interlock_check.py --gate g19.gov.implementation_interlock
```

---

## 6. G19.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g19_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G19-G25」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g19-closed`。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G19.1 初版：五 P0 行冻结。 |
