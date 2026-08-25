<!-- Assisted-by: Cursor Agent(G26.1 治理波) -->
# G26_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G26.1 治理交付物；事实源为 [G26_CONTRACT.md](G26_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 445/446/447。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据(逐字) | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g26.p0.m_a.framegen_device_kernel` | `py -3 ci/g26_framegen_device_kernel_smoke.py --gate g26.p0.m_a.framegen_device_kernel` | `milestones/g26/g26_m_a_framegen_device_kernel_evidence_schema.json` | FG/MFG device kernel 兑现:kernels/g26_framegen.rx(rurixc --target vulkan 产 SPV + spirv-val 通过)经 vk::run_compute 派发 + device vs host 金标准(temporal/framegen.rs)同输入逐帧对拍——×2/×3/×4 三档合成运动场景逐帧逐像素最大绝对差 p100 ≤ 标定容差(threshold = measured × 2.0 冻结 k,标定腿两跑位级一致程序产,禁手写)+ SSIM(interp)>SSIM(frame-hold) 程序产对照继承 + device 双跑位级一致 + kernel-bias RED 臂检出;host 参考臂 temporal/framegen.rs 0-byte;device 环境不可用时 SKIP 如实登记不冒充 | **G26.2** | post-interlock actual-next-free allocation |
| **M-b** | `g26.p0.m_b.framegen_device_bench_accounting` | `py -3 ci/g26_framegen_device_bench_accounting_smoke.py --gate g26.p0.m_b.framegen_device_bench_accounting` | `milestones/g26/g26_m_b_framegen_device_bench_accounting_evidence_schema.json` | FG device 车道帧时 measured 登记 + 口径纪律回验:device 全链路(打包+dispatch+回读)warmup+timed 逐帧墙钟登记(回归守护语义,不构成帧率对标通过线,生成帧禁计入真实渲染帧率)+ FgAccounting 真渲/presented 两口径类型面分离核验 + 性能面 g14_3_pipeline_perf 0-byte 机核 vs g25-closed | **G26.2** | post-interlock actual-next-free allocation |
| **M-c** | `g26.p0.m_c.rd045_backfill_rejudgment` | `py -3 ci/g26_rd045_backfill_rejudgment_smoke.py --gate g26.p0.m_c.rd045_backfill_rejudgment` | `milestones/g26/g26_m_c_rd045_backfill_rejudgment_evidence_schema.json` | RD-045 backfill 三件重判:新鲜观察窗真跑(RD-045 焦点车道 canonical 双跑 digest 轨迹多轮零漂移登记)+ 三件条件逐项机器盘点(根因定位/生产化修复/Full RFC 评估——树内证据闭集实测)——全齐 → close;未齐 → maintain-open 只追加扩窗零冒充;deferred history 只追加 | **G26.3** | post-interlock actual-next-free allocation |
| **M-d** | `g26.p0.m_d.g17_md_f1_rejudgment_window` | `py -3 ci/g26_g17_md_f1_rejudgment_window_smoke.py --gate g26.p0.m_d.g17_md_f1_rejudgment_window` | `milestones/g26/g26_m_d_g17_md_f1_rejudgment_window_evidence_schema.json` | G17-MD-F1 重判窗条件核验:NGX 分解 profiling 证据与 UE 侧插桩证据两半树内闭集搜索实测(evidence/ 检索面登记)——任一命中 → 重判程序启动;两半均未命中 → 维持 17/18 诚实红 carry(终判归 G30 商用终审),搜索面闭集只追加登记零冒充 | **G26.3** | post-interlock actual-next-free allocation |
| **M-e** | `g26.p0.m_e.closed_gate_no_regression` | `py -3 ci/g26_closed_gate_no_regression_smoke.py --gate g26.p0.m_e.closed_gate_no_regression` | `milestones/g26/g26_m_e_closed_gate_no_regression_evidence_schema.json` | G25 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g26_` 前缀不抢 latest | **G26.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G26.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G25 交接登记表三行（G13-N7 → M-a/M-b 承载；RD-045-window → M-c 承载；G17-MD-F1 → M-d 承载）；上游法定输入 = milestones/g25/g25_campaign_handover_registry.json(G26+ 唯一法定输入面)+ registry/deferred.json RD-045 条目 + src/rurix-render/src/temporal/framegen.rs(G19 host 参考臂,本期 0-byte 冻结面)。

---

## 4. 双向一致声明

本表 §1 五行与 G26_CONTRACT.md §4.2 逐字相等；key 命名空间 `g26.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G26.1 治理覆盖

```text
g26.wave.1.acceptance_map         步骤 445
  py -3 ci/g26_acceptance_map_check.py --gate g26.wave.1.acceptance_map
g26.wave.1.candidate_decisions    步骤 446
  py -3 ci/g26_candidate_decisions_check.py --gate g26.wave.1.candidate_decisions
g26.gov.implementation_interlock  步骤 447
  py -3 ci/g26_interlock_check.py --gate g26.gov.implementation_interlock
```

---

## 6. G26.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g26_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G26-G30」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g26-closed`（五期串行战役第一期收口）。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G26.1 初版：五 P0 行冻结。 |
