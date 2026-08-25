<!-- Assisted-by: Cursor Agent(G29.1 治理波) -->
# G29_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G29.1 治理交付物；事实源为 [G29_CONTRACT.md](G29_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 493/494/495。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据(逐字) | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g29.p0.m_a.slab_device_kernel` | `py -3 ci/g29_slab_device_kernel_smoke.py --gate g29.p0.m_a.slab_device_kernel` | `milestones/g29/g29_m_a_slab_device_kernel_evidence_schema.json` | slab device kernel 兑现:kernels/g29_slab.rx(rurixc --target vulkan 产 SPV + spirv-val 通过)经 vk::run_compute 派发——slab 能量守恒闭式 device 化(公式面与 host material/slab.rs 逐字同源:闭式反照率/白炉恒等/能量上界/lerp 连续)+ device vs host 同输入逐样本对拍(16641 样本网格同 g22_slab_probe 口径〔GRID=128 经 furnace_audit (grid+1)² 格点〕;p100 ≤ 标定容差 threshold = measured × 2.0 程序产禁手写,实测位级可达则登记零容差零条目)+ 白炉恒等 device 复现(dev 如实登记)+ device 双跑位级一致 + kernel-bias RED 臂检出;host 参考臂 material/slab.rs 0-byte;device 环境不可用时 SKIP 如实登记不冒充 | **G29.2** | post-interlock actual-next-free allocation |
| **M-b** | `g29.p0.m_b.slab_side_table_arm` | `py -3 ci/g29_slab_side_table_arm_smoke.py --gate g29.p0.m_b.slab_side_table_arm` | `milestones/g29/g29_m_b_slab_side_table_arm_evidence_schema.json` | 侧表供参加性臂兑现(bin-local,冻结面 0-byte):多材质槽 slab 参数侧表(bin 内合成独立 SSBO,MaterialClosure 32B 与 reserved 拓扑位零触碰)——device kernel 逐槽消费侧表求值 + 与 host 逐槽对拍(p100 同 M-a 容差协议)+ 逐槽白炉恒等维持 + 双跑位级一致 + graph/types.rs 0-byte 机核 | **G29.2** | post-interlock actual-next-free allocation |
| **M-c** | `g29.p0.m_c.svt_ktx2_gap_rejudgment` | `py -3 ci/g29_svt_ktx2_gap_rejudgment_smoke.py --gate g29.p0.m_c.svt_ktx2_gap_rejudgment` | `milestones/g29/g29_m_c_svt_ktx2_gap_rejudgment_evidence_schema.json` | SVT/KTX2 差距重判:SVT 四行(g22_svt_gap.json)+ KTX2 三行(g22_ktx2_disposition.json)逐行 reeval——各行现面实现痕迹树内实测(逐行检索清单 + 锚关键词映射入 evidence)——兑现 → 该行 closed-go;零实现 → 维持 defer 登记 milestones/g29/g29_svt_ktx2_rejudgment.json(g22 原表 0-byte 不回写);RD-041 history 只追加 | **G29.3** | post-interlock actual-next-free allocation |
| **M-d** | `g29.p0.m_d.wg_dgc_capability_recheck` | `py -3 ci/g29_wg_dgc_capability_recheck_smoke.py --gate g29.p0.m_d.wg_dgc_capability_recheck` | `milestones/g29/g29_m_d_wg_dgc_capability_recheck_evidence_schema.json` | Work Graphs/DGC capability 复测:VK_AMDX_shader_enqueue 新鲜 vulkaninfo 复测(三态闭集:absent 维持 not-available/present 翻转复评启动/SKIP 如实登记)+ DGC 三扩展 available 复测互核 + FSR 3.1.5 maintain 盘点(vendor_upscale 面 0-byte)——not-available 维持/复评启动均合法诚实终态零冒充 | **G29.3** | post-interlock actual-next-free allocation |
| **M-e** | `g29.p0.m_e.closed_gate_no_regression` | `py -3 ci/g29_closed_gate_no_regression_smoke.py --gate g29.p0.m_e.closed_gate_no_regression` | `milestones/g29/g29_m_e_closed_gate_no_regression_evidence_schema.json` | G28 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g29_` 前缀不抢 latest | **G29.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G29.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G29 承接两行（RD-041-slab → M-a/M-b 承载；RD-041-svt-ktx2-wg → M-c/M-d 承载）；上游法定输入 = milestones/g25/g25_campaign_handover_registry.json RD-041-slab/RD-041-svt-ktx2-wg 两行 + registry/deferred.json RD-041 + milestones/g22/g22_svt_gap.json(SVT 四行)+ milestones/g22/g22_ktx2_disposition.json(KTX2 三行)+ milestones/g22/g22_work_graphs_probe_results.json(WG absent/DGC available 实测)+ src/rurix-render/src/material/slab.rs(G22 host 参考臂,本期 0-byte 冻结面)。

---

## 4. 双向一致声明

本表 §1 五行与 G29_CONTRACT.md §4.2 逐字相等；key 命名空间 `g29.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G29.1 治理覆盖

```text
g29.wave.1.acceptance_map         步骤 493
  py -3 ci/g29_acceptance_map_check.py --gate g29.wave.1.acceptance_map
g29.wave.1.candidate_decisions    步骤 494
  py -3 ci/g29_candidate_decisions_check.py --gate g29.wave.1.candidate_decisions
g29.gov.implementation_interlock  步骤 495
  py -3 ci/g29_interlock_check.py --gate g29.gov.implementation_interlock
```

---

## 6. G29.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g29_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G26-G30」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g29-closed`（五期串行战役第四期收口）。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G29.1 初版：五 P0 行冻结。 |
