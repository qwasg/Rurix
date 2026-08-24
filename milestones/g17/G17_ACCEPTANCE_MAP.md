<!-- Assisted-by: Cursor Claude Fable 5（G17.1 治理波） -->
# G17_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G17.1 治理交付物（governance-only）；事实源为 [G17_CONTRACT.md](G17_CONTRACT.md) v1.0 front matter acceptance_gates（G-G17-1~9）与 §4.2 五行 P0 独立断言表、G17 法定输入面（[G15_P2_DECISIONS.md](../g15/G15_P2_DECISIONS.md) §4 G15-MD-F1 行 + [G16_CONTRACT.md](../g16/G16_CONTRACT.md) §7 立项裁决 3 + [G15_CONTRACT.md](../g15/G15_CONTRACT.md) §8.5~§8.7）、[G17_CANDIDATE_DECISIONS.md](G17_CANDIDATE_DECISIONS.md) v1.0。
> **编号纪律**：本表 P0 行只冻结 symbolic CI gate key 与稳定脚本名，不 claim 数字 CI step。数字步骤必须等 G-G17-2 实现互锁（§6）通过后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配——**P0 行 numeric_step 一律写 `post-interlock actual-next-free allocation`**；禁止沿用推测号与任何草案建议值。**例外面**：G17.1 治理三门（§5）按契约 §4.3/§7 立项裁决 4 明令本波即落盘真脚本真步骤——步骤 293/294/295 = 落盘前实测 `CI_step.next_free=293` 顺位领取，ledger 校准同批。
> **M 行号纪律**：M-a~M-e 字母行号为治理期稳定身份；本表不预占 M 数字。
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS；本波不预建空脚本、空 schema 壳或占位 workflow 步骤（只冻结路径——治理三门为例外，见上）。

---

## 1. P0 硬门（精确 5 行）

- P0 精确集合（5 行）：`{M-a, M-b, M-c, M-d, M-e}`。每一行的 symbolic key 同时是该能力唯一的 `assertion_id`，与 [G17_CONTRACT.md](G17_CONTRACT.md) §4.2 **逐字一致**。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论。
- **单一 key/脚本命名空间**：symbolic key 一律小写点分 `g17.p0.m_<a~e>.<slug>`，脚本一律 `ci/g17_<slug>_smoke.py`，evidence schema 一律 `milestones/g17/g17_m_<a~e>_<slug>_evidence_schema.json`（slug 与 key 末段同字面，只冻结路径不预建文件）。

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g17.p0.m_a.dual_end_retest_warm_recalib` | `py -3 ci/g17_dual_end_retest_warm_recalib_smoke.py --gate g17.p0.m_a.dual_end_retest_warm_recalib` | `milestones/g17/g17_m_a_dual_end_retest_warm_recalib_evidence_schema.json` | G14 M-d 门同口径协议双端复测（复测窗内同会话四轮全协议复跑，三轮进程级独立 50×3 trimmed mean 跨轮中位数零缩短，Stage A digest 锚守护）+ UE 参照臂暖态基线程序产重标定（复测窗 UE 逐格帧时包络程序产入 `g17_budget` 新条目，禁手写 P-09；`g14/g15/g16_budget` 既有条目 0-byte）+ 新旧环境差异如实分解（UE 侧暖态事件与 Rurix 侧变化分列登记，禁混淆归因） | **G17.2** | post-interlock actual-next-free allocation |
| **M-b** | `g17.p0.m_b.ngx_evolution_alignment` | `py -3 ci/g17_ngx_evolution_alignment_smoke.py --gate g17.p0.m_b.ngx_evolution_alignment` | `milestones/g17/g17_m_b_ngx_evolution_alignment_evidence_schema.json` | NGX 版本演进面对齐评估：nvngx_dlss.dll 310.5.2→310.6.0+ 换版评估走新缓存目录 + G17 新 provenance 登记面 `milestones/g17/g17_vendor_sdk_registry.json`（g13 登记表 0-byte）+ PaddedWindowNetwork 实例化形态核验（SL verbose 日志逐字）+ in-stream/提交税源 X2 边际探针重测分解（对照 1.90+0.10ms 基线，新鲜命令输出）+ 画质守护双门禁（Stage A digest 锚零漂移 + 画质锚带复核带内，超带即拒绝换版）+ A/B measured 结论如实登记（采纳/拒绝/零收益均合法） | **G17.3** | post-interlock actual-next-free allocation |
| **M-c** | `g17.p0.m_c.d3d12_host_lane_disposition` | `py -3 ci/g17_d3d12_host_lane_disposition_smoke.py --gate g17.p0.m_c.d3d12_host_lane_disposition` | `milestones/g17/g17_m_c_d3d12_host_lane_disposition_evidence_schema.json` | RFC-0032（D3D12 宿主 NGX 车道：跨 device 同步面/单 device 化评估）终态兑现——经 D-409 对抗评审（评审 provenance ≠ 起草 provenance，findings 逐条 disposition）后 approved/no-go/defer 三态均合法终态；approved → 实现（unsafe 纪律：`// SAFETY:` + unsafe-audit 注册条目 + 单块单操作）；no-go/defer → 可机器核验评估证据留档 + 兜底字面维持（RFC 终态字面入 evidence） | **G17.4** | post-interlock actual-next-free allocation |
| **M-d** | `g17.p0.m_d.t100_final_verdict` | `py -3 ci/g17_t100_final_verdict_smoke.py --gate g17.p0.m_d.t100_final_verdict` | `milestones/g17/g17_m_d_t100_final_verdict_evidence_schema.json` | t100 档优化与终判复测：scene 面有界优化（L0 位级探针漂移即弃，禁碰 NGX 税源物理地板冒充收益）+ 终判双端 18 格全协议复测（G14 M-d 同口径，ratio 终值必须来自 evidence JSON 命令输出）+ 终判判定如实登记（达标 18/18 或维持未达标登记不冒充，二者均合法收口，兜底字面与 G15 同源） | **G17.5** | post-interlock actual-next-free allocation |
| **M-e** | `g17.p0.m_e.closed_gate_no_regression` | `py -3 ci/g17_closed_gate_no_regression_smoke.py --gate g17.p0.m_e.closed_gate_no_regression` | `milestones/g17/g17_m_e_closed_gate_no_regression_evidence_schema.json` | G13/G14/G15/G16 受影响门 `--verify-latest` 全绿零降级；`g17_` 前缀不抢旧门 latest；禁对旧脚本发 `--gate` | **G17.6** | post-interlock actual-next-free allocation |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断后续收口。M-d 门内「协议完整性/证据链」与「ratio 达标判定」双断言分离（契约立项裁决 5）：后者红时如实登记不遮蔽，close-out 消费「终判证据链完整 + 判定如实登记」面（两种结局均合法收口）。

---

## 2. 已 go P1 硬门（零行）

G17.1 无 go 的 P1 行——候选决策实现门（M-a~M-e）全为 P0（契约 §4.2 字面）。后续波次若治理程序将新 P1 判为 go，须先按治理程序修订本表及覆盖集合（只追加进 §2）再开对应实现；不得把它静默并入现有 key。本节的机核面 = §2 零行声明与 `ci/g17_acceptance_map_check.py` 的 P1 空集断言（§5）。

---

## 3. 条件型 / not-triggered 登记面

### 3.1 G16 defer 行 G17 重评窗登记面

G16 十四行 defer-to-G17+（M61/M52/M100-high/SAFE-GPU/M127/M98-l4/M114-strand/M118-hdr-cal/M125-adopt3/G10-N6/G10-N8/G10-N17/G11-N5/G13-N7）本波重评窗结论 = **defer-to-G18+**——本波范围 = DLSS 性能缺口收口，与各行触发条件不交集。窗结论逐字以候选决策表 §1 登记为准。

### 3.2 G15-MD-F1 本波 go 面

G15-MD-F1（fps_parity_deficit@bistro-interior/t100/dlss_sr）= 本波唯一 go 承接，承接锚三件套字面兑现：① 双端同协议复测与暖态重标定由 M-a 承载；② NGX 版本演进面对齐由 M-b 承载；③ 车道架构面 D3D12 宿主 NGX 由 M-c 承载（RFC-0032）；终判由 M-d 承载；旧门零降级由 M-e 承载。

### 3.3 终判两态与 G18 方向分列

终判达标 18/18 或维持未达标如实登记均为合法收口（不冒充）。G18 方向（GI P2~P4、虚拟化几何、帧生成独立层等）本窗只重评触发条件，零实现。

---

## 4. 双向一致与互斥面（key 命名空间机器可核声明）

1. **双方逐字一致**：本表 §1 五行与 [G17_CONTRACT.md](G17_CONTRACT.md) §4.2 五行对同一 P0 M 行给出的独立硬判据与波次**必须逐字相等**，由 `ci/g17_acceptance_map_check.py` 双向比对机器强制。
2. **唯一命名空间**：`g17.p0.m_<a~e>.<slug>` + `ci/g17_<slug>_smoke.py` + `milestones/g17/g17_m_<a~e>_<slug>_evidence_schema.json` 为唯一合法形态。
3. **互斥**：M 行与 key 一对一；`no-go`/`defer` 项不产生 key、不入本表。**本表 §1 五 key 与 [G17_CANDIDATE_DECISIONS.md](G17_CANDIDATE_DECISIONS.md) 19 行候选行 ID 命名空间互斥**。

---

## 5. G17.1 治理覆盖与空行门

G17.1 治理三门（本波 materialize，步骤按契约 §4.3/§7 立项裁决 4 落盘前实测 `CI_step.next_free=293` 顺位领取；**白名单声明**：本波除下列治理三门外，零 workflow 步骤、零脚本、零 schema 壳预放）：

```text
g17.wave.1.acceptance_map         步骤 293（落盘前实测 CI_step.next_free=293 顺位领取）
  py -3 ci/g17_acceptance_map_check.py --gate g17.wave.1.acceptance_map

g17.wave.1.candidate_decisions    步骤 294（同批顺位领取）
  py -3 ci/g17_candidate_decisions_check.py --gate g17.wave.1.candidate_decisions

g17.gov.implementation_interlock  步骤 295（同批顺位领取）
  py -3 ci/g17_interlock_check.py --gate g17.gov.implementation_interlock
```

`ci/g17_acceptance_map_check.py` 的 PASS 判据：P0 行集合与 §1 的 5 项集合全等 + §2 P1 空集 + key/脚本/schema 单一命名空间 + numeric_step 字面零预占 + MAP §1 ↔ CONTRACT §4.2 判据/波次双向逐字一致。

`ci/g17_candidate_decisions_check.py` 的 PASS 判据：19 行闭集全等（§1 十五行 + §3 四行；§2 RD 八条映射行不重复计入）+ 裁决枚举合法（G17 即本期，defer-to-G17+ 不再合法，defer 合法值 = defer-to-G18+）+ 零空行 + 承接锚纪律 + defer 行 G18+ 重评窗 + go 行验收映射锚 + §2 RD 八条 open。

`ci/g17_interlock_check.py` 的 PASS 判据：逐项读取事实源输出各条件真值；诚实输出 BLOCKED/READY。

---

## 6. G17.2 硬互锁

`implementation_status: blocked` 解锁须以下条件**同时**为真：

1. G17.1 治理门全部完成且有真实验证记录——§5 的 `g17.wave.1.acceptance_map` 与 `g17.wave.1.candidate_decisions` 独立 PASS。
2. `ci/g17_interlock_check.py --require-ready` 输出 **READY**。
3. 用户 G17.2 开工指令已留痕——「**帮我一次性完成G17**」字面。
4. 共享编号按互锁开放时 **actual next_free 重新校准**。

---

## 7. Close-out 审计

完成条件 = M-a~M-e 五 P0 齐备（M-d 终判两态均合法）+ P2 穷举零空行 + soak ≥1800s 零失败 + close-out 八 facts VERDICT=READY → status flip + tag `g17-closed`。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G17.1 初版：冻结 5 个 P0 的 symbolic gate、目标脚本/schema、独立判据（契约 §4.2 逐字 0-byte 转引）与最晚波次。P0 行数字 CI 步骤全部 `post-interlock actual-next-free allocation`。治理三门步骤 293/294/295。 |
