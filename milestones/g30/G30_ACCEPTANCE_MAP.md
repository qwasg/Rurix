<!-- Assisted-by: Cursor Agent(G30.1 治理波) -->
# G30_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G30.1 治理交付物；事实源为 [G30_CONTRACT.md](G30_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 509/510/511。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据(逐字) | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g30.p0.m_a.tail_anchor_rejudgment_closure` | `py -3 ci/g30_tail_anchor_rejudgment_closure_smoke.py --gate g30.p0.m_a.tail_anchor_rejudgment_closure` | `milestones/g30/g30_m_a_tail_anchor_rejudgment_closure_evidence_schema.json` | 尾锚重判闭集:六件外部条件类尾锚机器取证重判——M125-adopt3(Jolt 5.6 需求证据三类树内实测 + sys56 评估臂 cargo check 新鲜)+ M127(corpus 目录 + PhysicsAsset residual 消费方检索)+ M114-strand(毛发资产入压测闭集检索)+ M118-hdr-cal(vulkaninfo HDR token 新鲜探针)+ G10-N6(fbx2gltf/assimp/blender 三工具 PATH 实测 + 源资产检索)+ SAFE-GPU(独立期资源窗 + 平台需求方文档检索);RD-042/043/044 三条 G30 尾锚窗同批逐锚重判;各件 searched-paths manifest 必填,全未命中 → 逐件维持诚实终态零冒充;deferred history 只追加 | **G30.2** | post-interlock actual-next-free allocation |
| **M-b** | `g30.p0.m_b.commercial_final_review` | `py -3 ci/g30_commercial_final_review_smoke.py --gate g30.p0.m_b.commercial_final_review` | `milestones/g30/g30_m_b_commercial_final_review_evidence_schema.json` | 三面商用终审:画质面——画质表面闭集 0-byte 机核(vs g25-closed git-diff)+ 战役期加性面(四 kernel/四 device bin)零接线核验 + G18 M-d 达标绿件只读盘点;性能面——G14 M-d 最新 18 格 evidence 如实定盘 + 性能面 0-byte 机核 + 焦点格新鲜单测真跑(bistro-interior/t100/dlss_sr canonical 160 帧 ratio 登记,G17-MD-F1 终判法定义务:≥1.00 → 18/18 或物理不可达维持 17/18 诚实红终判,两态均为战役合法收官态);确定性面——Stage A 18 格 digest 锚在档 + 战役期四 device kernel 双跑位级绿件盘点;三面终态如实定盘零冒充 | **G30.2** | post-interlock actual-next-free allocation |
| **M-c** | `g30.p0.m_c.campaign_full_chain_no_regression` | `py -3 ci/g30_campaign_full_chain_no_regression_smoke.py --gate g30.p0.m_c.campaign_full_chain_no_regression` | `milestones/g30/g30_m_c_campaign_full_chain_no_regression_evidence_schema.json` | 战役全链零降级:G29 受影响门 `--verify-latest` 全绿(递归链自动涵盖 G26~G28 及更早)+ budget_eval --strict 全量零 skip 零 estimated;禁 `--gate` 旧脚本 | **G30.3** | post-interlock actual-next-free allocation |
| **M-d** | `g30.p0.m_d.campaign_handover_ledger` | `py -3 ci/g30_campaign_handover_ledger_smoke.py --gate g30.p0.m_d.campaign_handover_ledger` | `milestones/g30/g30_m_d_campaign_handover_ledger_evidence_schema.json` | 战役承接锚归档闭集:g30_campaign_handover_registry.json(五期 defer/maintain 行 + RD 八条 G31+ 锚 + 历史清册引用 + 尾锚六件重判终态)全量汇总闭集登记——G31+ 唯一法定输入面;归档完整性机核 | **G30.3** | post-interlock actual-next-free allocation |
| **M-e** | `g30.p0.m_e.closed_gate_no_regression` | `py -3 ci/g30_closed_gate_no_regression_smoke.py --gate g30.p0.m_e.closed_gate_no_regression` | `milestones/g30/g30_m_e_closed_gate_no_regression_evidence_schema.json` | G29 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g30_` 前缀不抢 latest | **G30.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G30.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G30 承接七行（M125-adopt3/M127/M114-strand/M118-hdr-cal/G10-N6/SAFE-GPU → M-a 尾锚重判承载；G17-MD-F1 → M-b 终判承载）+ RD-042/043/044 三条 → M-a 尾锚窗同批逐锚重判 + RD-045 → M-b 确定性面复核承载；上游法定输入 = milestones/g25/g25_campaign_handover_registry.json（七行）+ registry/deferred.json（八条）+ milestones/g24/g24_legacy_rd_registry.json（历史清册）+ G26~G29 四期 P2 表（战役期承接锚）。**战役承接池 G30 后清零，G31+ 法定输入 = M-d 归档闭集**。

---

## 4. 双向一致声明

本表 §1 五行与 G30_CONTRACT.md §4.2 逐字相等；key 命名空间 `g30.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G30.1 治理覆盖

```text
g30.wave.1.acceptance_map         步骤 509
  py -3 ci/g30_acceptance_map_check.py --gate g30.wave.1.acceptance_map
g30.wave.1.candidate_decisions    步骤 510
  py -3 ci/g30_candidate_decisions_check.py --gate g30.wave.1.candidate_decisions
g30.gov.implementation_interlock  步骤 511
  py -3 ci/g30_interlock_check.py --gate g30.gov.implementation_interlock
```

---

## 6. G30.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g30_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G26-G30」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g30-closed`（战役收官）。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G30.1 初版：五 P0 行冻结。 |
