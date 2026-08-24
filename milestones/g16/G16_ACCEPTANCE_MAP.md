<!-- Assisted-by: Cursor Grok 4.6（G16.1 治理波） -->
# G16_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G16.1 治理交付物（governance-only）；事实源为 [G16_CONTRACT.md](G16_CONTRACT.md) v1.0 front matter acceptance_gates（G-G16-1~6）与 §4.2 四行 P0 独立断言表、G16 法定输入面（[G15_CONTRACT.md](../g15/G15_CONTRACT.md) §8.9 G16 承接锚 + [G15_P2_DECISIONS.md](../g15/G15_P2_DECISIONS.md) G15-MC-F1 / G15-MD-F1 + 十四行 defer-to-G16+）、[G16_CANDIDATE_DECISIONS.md](G16_CANDIDATE_DECISIONS.md) v1.0。
> **编号纪律**：本表 P0 行只冻结 symbolic CI gate key 与稳定脚本名，不 claim 数字 CI step。数字步骤必须等 G-G16-2 实现互锁（§6）通过后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配——**P0 行 numeric_step 一律写 `post-interlock actual-next-free allocation`**；禁止沿用推测号与任何草案建议值。**例外面**：G16.1 治理三门（§5）按契约 §4.3/§7 立项裁决 4 明令本波即落盘真脚本真步骤——步骤 281/282/283 = 落盘前实测 `CI_step.next_free=281` 顺位领取，ledger 校准同批。
> **M 行号纪律**：M-a~M-d 字母行号为治理期稳定身份；本表不预占 M 数字。
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS；本波不预建空脚本、空 schema 壳或占位 workflow 步骤（只冻结路径——治理三门为例外，见上）。

---

## 1. P0 硬门（精确 4 行）

- P0 精确集合（4 行）：`{M-a, M-b, M-c, M-d}`。每一行的 symbolic key 同时是该能力唯一的 `assertion_id`，与 [G16_CONTRACT.md](G16_CONTRACT.md) §4.2 **逐字一致**。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论。
- **单一 key/脚本命名空间**：symbolic key 一律小写点分 `g16.p0.m_<a~d>.<slug>`，脚本一律 `ci/g16_<slug>_smoke.py`，evidence schema 一律 `milestones/g16/g16_m_<a~d>_<slug>_evidence_schema.json`（slug 与 key 末段同字面，只冻结路径不预建文件）。

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g16.p0.m_a.ue_reference_arm_repair` | `py -3 ci/g16_ue_reference_arm_repair_smoke.py --gate g16.p0.m_a.ue_reference_arm_repair` | `milestones/g16/g16_m_a_ue_reference_arm_repair_evidence_schema.json` | 探针定因 + harness 补丁 + 只重建 cornell + 重采 5 job + 内容有效性（五份末帧 HDR luma max > 1e-3、非全黑读图可见盒体/红绿墙/双箱、bistro 旁证不退化） | **G16.2** | post-interlock actual-next-free allocation |
| **M-b** | `g16.p0.m_b.dual_end_reharvest` | `py -3 ci/g16_dual_end_reharvest_smoke.py --gate g16.p0.m_b.dual_end_reharvest` | `milestones/g16/g16_m_b_dual_end_reharvest_evidence_schema.json` | 同口径重算 G13 M-c/M-d 度量（UE 新帧 + 既有/按需新鲜 Rurix 臂），fresh measured_delta 入 G16 处置表 `milestones/g16/g16_quality_gap_disposition.json`；不写 G13 两张登记表（git 机核 0-byte） | **G16.3** | post-interlock actual-next-free allocation |
| **M-c** | `g16.p0.m_c.absolute_quality_rereview` | `py -3 ci/g16_absolute_quality_rereview_smoke.py --gate g16.p0.m_c.absolute_quality_rereview` | `milestones/g16/g16_m_c_absolute_quality_rereview_evidence_schema.json` | 18 格生产管线 vs 新 UE 参照；双 seed 重标定（新 `g16_budget` 条目，不改 `g15_budget`）；AI 读图；商用收口 x/18 如实（cornell 九格应不再 `ue_reference_degenerate`；bistro 九格预期仍超阈，不冒充达标） | **G16.4** | post-interlock actual-next-free allocation |
| **M-d** | `g16.p0.m_d.closed_gate_no_regression` | `py -3 ci/g16_closed_gate_no_regression_smoke.py --gate g16.p0.m_d.closed_gate_no_regression` | `milestones/g16/g16_m_d_closed_gate_no_regression_evidence_schema.json` | G13/G15 closeout 与 wave exit、G15 M-e / G14 M-e `--verify-latest` 仍 PASS；84 门绿面不因新 `g16_`* 件被抢 latest；禁对旧脚本发 `--gate` | **G16.5** | post-interlock actual-next-free allocation |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断后续收口。本波不做 G16 收口。

---

## 2. 已 go P1 硬门（零行）

G16.1 无 go 的 P1 行——候选决策 4 行实现门（M-a~M-d）全为 P0（契约 §4.2 字面）。后续波次若治理程序将新 P1 判为 go，须先按治理程序修订本表及覆盖集合（只追加进 §2）再开对应实现；不得把它静默并入现有 key。本节的机核面 = §2 零行声明与 `ci/g16_acceptance_map_check.py` 的 P1 空集断言（§5）。

---

## 3. 条件型 / not-triggered 登记面

### 3.1 G15 defer 行 G16 重评窗登记面

G15 十四行 defer-to-G16+（M61/M52/M100-high/SAFE-GPU/M127/M98-l4/M114-strand/M118-hdr-cal/M125-adopt3/G10-N6/G10-N8/G10-N17/G11-N5/G13-N7）本波重评窗结论 = **defer-to-G17+**——本波范围 = 参照臂修复，与各行触发条件不交集。G15-MD-F1 同判 defer-to-G17+。窗结论逐字以候选决策表 §1 登记为准。

### 3.2 G15-MC-F1 本波 go 面

G15-MC-F1（UE cornell 参照臂死黑）= 本波唯一 go 承接，由 M-a 承载诊断/补丁/重采/内容有效性，M-b/M-c 承载重测。不改坐标尺度。

### 3.3 坐标尺度与 Lumen 分列

禁止改坐标尺度。Lumen on/off 作为旁证确认不再死黑，不宣称 GI 达标，不立项 GI 表达 RFC。

---

## 4. 双向一致与互斥面（key 命名空间机器可核声明）

1. **双方逐字一致**：本表 §1 四行与 [G16_CONTRACT.md](G16_CONTRACT.md) §4.2 四行对同一 P0 M 行给出的独立硬判据与波次**必须逐字相等**，由 `ci/g16_acceptance_map_check.py` 双向比对机器强制。
2. **唯一命名空间**：`g16.p0.m_<a~d>.<slug>` + `ci/g16_<slug>_smoke.py` + `milestones/g16/g16_m_<a~d>_<slug>_evidence_schema.json` 为唯一合法形态。
3. **互斥**：M 行与 key 一对一；`no-go`/`defer` 项不产生 key、不入本表。**本表 §1 四 key 与 [G16_CANDIDATE_DECISIONS.md](G16_CANDIDATE_DECISIONS.md) 20 行候选行 ID 命名空间互斥**。

---

## 5. G16.1 治理覆盖与空行门

G16.1 治理三门（本波 materialize，步骤按契约 §4.3/§7 立项裁决 4 落盘前实测 `CI_step.next_free=281` 顺位领取；**白名单声明**：本波除下列治理三门外，零 workflow 步骤、零脚本、零 schema 壳预放）：

```text
g16.wave.1.acceptance_map         步骤 281（落盘前实测 CI_step.next_free=281 顺位领取）
  py -3 ci/g16_acceptance_map_check.py --gate g16.wave.1.acceptance_map

g16.wave.1.candidate_decisions    步骤 282（同批顺位领取）
  py -3 ci/g16_candidate_decisions_check.py --gate g16.wave.1.candidate_decisions

g16.gov.implementation_interlock  步骤 283（同批顺位领取）
  py -3 ci/g16_interlock_check.py --gate g16.gov.implementation_interlock
```

`ci/g16_acceptance_map_check.py` 的 PASS 判据：P0 行集合与 §1 的 4 项集合全等 + §2 P1 空集 + key/脚本/schema 单一命名空间 + numeric_step 字面零预占 + MAP §1 ↔ CONTRACT §4.2 判据/波次双向逐字一致。

`ci/g16_candidate_decisions_check.py` 的 PASS 判据：20 行闭集全等 + 裁决枚举合法 + 零空行 + 承接锚纪律 + defer 行 G17+ 重评窗 + go 行验收映射锚 + §2 RD 八条 open。

`ci/g16_interlock_check.py` 的 PASS 判据：逐项读取事实源输出各条件真值；诚实输出 BLOCKED/READY。

---

## 6. G16.2 硬互锁

`implementation_status: blocked` 解锁须以下条件**同时**为真：

1. G16.1 治理门全部完成且有真实验证记录——§5 的 `g16.wave.1.acceptance_map` 与 `g16.wave.1.candidate_decisions` 独立 PASS。
2. `ci/g16_interlock_check.py --require-ready` 输出 **READY**。
3. 用户 G16.2 开工指令已留痕——「**修复UE5参考臂全黑的问题**」字面。
4. 共享编号按互锁开放时 **actual next_free 重新校准**。

---

## 7. Close-out 审计

G16.1~G16.5 本波不做 G16 收口（历史字面）。G16plus 另立治理程序：附录 A + RFC-0031；完成条件 = M-g `met_count==18` + soak/close-out。四旧 P0 全绿仍为前置。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G16.1 初版：冻结 4 个 P0 的 symbolic gate、目标脚本/schema、独立判据（契约 §4.2 逐字 0-byte 转引）与最晚波次。P0 行数字 CI 步骤全部 `post-interlock actual-next-free allocation`。治理三门步骤 281/282/283。 |
| v1.1 | 2026-08-24 | G16plus：附录 A 只追加 M-e~M-h。§1 四行闭集 0-byte。 |

## 附录 A. G16plus 延续波门（只追加——§1 冻结 4 行闭集 0-byte 不动，非数字节首不参与 §1/§2 行集机核）

| M 行 | symbolic gate key / 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|
| **M-e** | `g16.p0.m_e.gi_expression`<br>`py -3 ci/g16_gi_expression_smoke.py --gate g16.p0.m_e.gi_expression` | `milestones/g16/g16_m_e_gi_expression_evidence_schema.json` | RFC-0031 落地 + `--gi on` 加性车道 + cornell 间接光能量非近零且读图色bleed 机核；`--gi off` 默认臂位级不漂移 | **G16.8** | 288 |
| **M-f** | `g16.p0.m_f.lumen_reharvest`<br>`py -3 ci/g16_lumen_reharvest_smoke.py --gate g16.p0.m_f.lumen_reharvest` | `milestones/g16/g16_m_f_lumen_reharvest_evidence_schema.json` | G13 M-d 同口径重算入 G16 处置表；`indirect_ssim` / `gi_energy_rel` 可溯源；不写 G13 两张登记表（git 机核 0-byte） | **G16.9** | 289 |
| **M-g** | `g16.p0.m_g.absolute_quality_closure`<br>`py -3 ci/g16_absolute_quality_closure_smoke.py --gate g16.p0.m_g.absolute_quality_closure` | `milestones/g16/g16_m_g_absolute_quality_closure_evidence_schema.json` | 生产管线 GI on vs 新 UE 参照；双 seed 重标定（新 `g16_budget` 条目）；AI 读图；`met_count==18` 且阈为程序产 p100×2.0；不改 M-c 历史 0/18 门 | **G16.10** | 290 |
| **M-h** | `g16.p0.m_h.continuation_closeout`<br>`py -3 ci/g16_closeout_check.py --gate g16.wave.6b.closeout` | `milestones/g16/g16_wave6b_closeout_evidence_schema.json` | 仅当 M-g 已绿：soak≥1800s 零失败 + 八 facts VERDICT=READY；四旧 P0 仍绿（M-c 保持诚实 0/18 历史门）+ RFC-0031 Approved + RD 八条 open + 商用 18/18 | **G16plus** | 292 |

- 附录 A 行纪律：与 §1 同构（独立 symbolic key + 独立 evidence subject + 独立布尔断言）；§1 冻结 4 行集合机核面 0-byte 不扩。soak 门 `g16.wave.6a.soak` 步骤 291 为稳定波聚合，不进 §1。
- 步骤 288~292 = 落盘前实测 `CI_step.next_free=288` 顺位领取。
