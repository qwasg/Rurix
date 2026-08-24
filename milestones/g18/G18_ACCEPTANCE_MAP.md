<!-- Assisted-by: Cursor Agent（G18.1 治理波） -->
# G18_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G18.1 治理交付物；事实源为 [G18_CONTRACT.md](G18_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 309/310/311。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 9 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g18.p0.m_a.rurix_light_transport_depth` | `py -3 ci/g18_rurix_light_transport_depth_smoke.py --gate g18.p0.m_a.rurix_light_transport_depth` | `milestones/g18/g18_m_a_rurix_light_transport_depth_evidence_schema.json` | 天光/IBL + 镜面反射 + 软阴影 + 降噪 + GI 纵深加性 profile 实现；默认臂 `--gi off` Stage A digest 锚 18 格零漂移；加性 profile 走 `--presentation-profile` 或 `--gi on` 独立登记面 | **G18.2** | post-interlock actual-next-free allocation |
| **M-b** | `g18.p0.m_b.presentation_pipeline_dual_profile` | `py -3 ci/g18_presentation_pipeline_dual_profile_smoke.py --gate g18.p0.m_b.presentation_pipeline_dual_profile` | `milestones/g18/g18_m_b_presentation_pipeline_dual_profile_evidence_schema.json` | 后处理链（exposure/bloom/tonemap）接入 + PNG 出图 + `g18_presentation_contract.json` 夜/日双 profile；收敛帧 ≥128；G13 冻结契约 0-byte | **G18.2** | post-interlock actual-next-free allocation |
| **M-c** | `g18.p0.m_c.ue_arm_lighting_repair_and_render` | `py -3 ci/g18_ue_arm_lighting_repair_and_render_smoke.py --gate g18.p0.m_c.ue_arm_lighting_repair_and_render` | `milestones/g18/g18_m_c_ue_arm_lighting_repair_and_render_evidence_schema.json` | UE 臂 bistro 灯光/曝光校准 + 日景关卡 variant（DirectionalLight+SkyLight）+ MRQ presentation 出图（夜/日 × 两场景）+ `-renderoffscreen` UE 5.8 可用性实测 | **G18.3** | post-interlock actual-next-free allocation |
| **M-d** | `g18.p0.m_d.dual_end_commercial_quality_verdict` | `py -3 ci/g18_dual_end_commercial_quality_verdict_smoke.py --gate g18.p0.m_d.dual_end_commercial_quality_verdict` | `milestones/g18/g18_m_d_dual_end_commercial_quality_verdict_evidence_schema.json` | 双端商业化画质终审：AI 读图逐格 + SSIM/FLIP 程序产阈（p100×2.0 禁手写）；达标/诚实红均合法；G10-N17 FLIP 演进位 + G11-N5 暗帧数据集顺带兑现 | **G18.7** | post-interlock actual-next-free allocation |
| **M-e** | `g18.p0.m_e.sl_runtime_upgrade_disposition` | `py -3 ci/g18_sl_runtime_upgrade_disposition_smoke.py --gate g18.p0.m_e.sl_runtime_upgrade_disposition` | `milestones/g18/g18_m_e_sl_runtime_upgrade_disposition_evidence_schema.json` | G17-MB-F1 兑现：新版 Streamline 换版/拒绝换版/not-available 均合法终态；provenance 登记 + 画质守护双门禁 | **G18.4** | post-interlock actual-next-free allocation |
| **M-f** | `g18.p0.m_f.fps_parity_reeval` | `py -3 ci/g18_fps_parity_reeval_smoke.py --gate g18.p0.m_f.fps_parity_reeval` | `milestones/g18/g18_m_f_fps_parity_reeval_evidence_schema.json` | G17-MD-F1 兑现：G14 M-d 同口径 18 格重评；≥1.00 → 18/18；物理不可达 → 维持未达标登记不冒充 | **G18.4** | post-interlock actual-next-free allocation |
| **M-g** | `g18.p0.m_g.virtualized_geometry_p3` | `py -3 ci/g18_virtualized_geometry_p3_smoke.py --gate g18.p0.m_g.virtualized_geometry_p3` | `milestones/g18/g18_m_g_virtualized_geometry_p3_evidence_schema.json` | RFC-0034 终态兑现：mesh shader VisBuffer 第三光栅路径实现 / no-go / defer 均合法；像素零差判据或评估证据留档 | **G18.5** | post-interlock actual-next-free allocation |
| **M-h** | `g18.p0.m_h.frame_generation_independent_layer` | `py -3 ci/g18_frame_generation_independent_layer_smoke.py --gate g18.p0.m_h.frame_generation_independent_layer` | `milestones/g18/g18_m_h_frame_generation_independent_layer_evidence_schema.json` | RFC-0035 终态兑现：FG/MFG 独立层（真实渲染帧率口径，禁混入 upscale ratio）；实现 / no-go / defer 均合法 | **G18.6** | post-interlock actual-next-free allocation |
| **M-i** | `g18.p0.m_i.closed_gate_no_regression` | `py -3 ci/g18_closed_gate_no_regression_smoke.py --gate g18.p0.m_i.closed_gate_no_regression` | `milestones/g18/g18_m_i_closed_gate_no_regression_evidence_schema.json` | G13~G17 受影响门 `--verify-latest` 全绿零降级；禁 `--gate`；`g18_` 前缀不抢 latest | **G18.7** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G18.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G17 defer-to-G18+ 十六行本波重评：G17-MB-F1/G17-MD-F1/G13-N7/M61/M100-high/G10-N8/G10-N17/G11-N5 → go；其余 → defer-to-G19+（触发条件不齐备）。

---

## 4. 双向一致声明

本表 §1 九行与 G18_CONTRACT.md §4.2 逐字相等；key 命名空间 `g18.p0.m_<a~i>.<slug>` 唯一。

---

## 5. G18.1 治理覆盖

```text
g18.wave.1.acceptance_map         步骤 309
  py -3 ci/g18_acceptance_map_check.py --gate g18.wave.1.acceptance_map
g18.wave.1.candidate_decisions    步骤 310
  py -3 ci/g18_candidate_decisions_check.py --gate g18.wave.1.candidate_decisions
g18.gov.implementation_interlock  步骤 311
  py -3 ci/g18_interlock_check.py --gate g18.gov.implementation_interlock
```

---

## 6. G18.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g18_interlock_check.py --require-ready` READY + 用户 U-59 指令留痕。

---

## 7. Close-out 审计

M-a~M-i 九 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g18-closed`。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G18.1 初版：九 P0 行冻结。 |
