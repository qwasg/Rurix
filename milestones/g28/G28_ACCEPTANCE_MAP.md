<!-- Assisted-by: Cursor Agent(G28.1 治理波) -->
# G28_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G28.1 治理交付物；事实源为 [G28_CONTRACT.md](G28_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 477/478/479。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据(逐字) | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g28.p0.m_a.restir_device_kernel` | `py -3 ci/g28_restir_device_kernel_smoke.py --gate g28.p0.m_a.restir_device_kernel` | `milestones/g28/g28_m_a_restir_device_kernel_evidence_schema.json` | ReSTIR device kernel 兑现:kernels/g28_restir.rx(rurixc --target vulkan 产 SPV + spirv-val 通过)经 vk::run_compute 派发——WRS/RIS reservoir 更新链 device 化(候选流与均匀随机数由 host 单源预生成上传,device 不重生成 RNG——PCG32 u64 状态面留 host;逐 trial 单 invocation 顺序 WRS 链保浮点序)+ device vs host 金标准(gi/restir_reservoir.rs estimate_ris)同输入逐 trial 对拍(p100 ≤ 标定容差,threshold = measured × 2.0 冻结 k 程序产禁手写;实测位级可达则登记零容差)+ 无偏 3σ 维持 + device 双跑位级一致 + kernel-bias RED 臂检出;host 参考臂 0-byte;device 环境不可用时 SKIP 如实登记不冒充 | **G28.2** | post-interlock actual-next-free allocation |
| **M-b** | `g28.p0.m_b.restir_spatial_reuse_arm` | `py -3 ci/g28_restir_spatial_reuse_arm_smoke.py --gate g28.p0.m_b.restir_spatial_reuse_arm` | `milestones/g28/g28_m_b_restir_spatial_reuse_arm_evidence_schema.json` | 空间重用加性臂兑现(bin-local,host 参考臂 0-byte):多着色点网格邻域 reservoir 合并(Reservoir::merge 语义同构 m_cap 截断,时域/空间同律)——无偏 3σ 维持(空间合并不引入偏差,等验证预算 measured 对照)+ 空间合并方差再收益 measured 登记(程序产对照,收益值如实登记不设通过线)+ 双跑位级一致 + M100 低档 MegaLights 生产默认面 0-byte 机核 | **G28.2** | post-interlock actual-next-free allocation |
| **M-c** | `g28.p0.m_c.m52_rd040_workload_rejudgment` | `py -3 ci/g28_m52_rd040_workload_rejudgment_smoke.py --gate g28.p0.m_c.m52_rd040_workload_rejudgment` | `milestones/g28/g28_m_c_m52_rd040_workload_rejudgment_evidence_schema.json` | M52/RD-040 workload 重判:M52 两半盘点——capability 半边(G21 vulkaninfo 三 token available 取证只读盘点 + 新鲜 vulkaninfo 复测)+ workload 半边(RT pipeline/SBT 宿主车道树内检索,searched-paths manifest 必填)——两半全齐方改判;未全齐 → maintain-defer 只追加;RD-040 五分项逐锚重判(五分项 reeval_anchor 树内实测逐项登记)——全未命中 → 维持 defer;RD-040 history 只追加 | **G28.3** | post-interlock actual-next-free allocation |
| **M-d** | `g28.p0.m_d.rd034_upstream_recheck` | `py -3 ci/g28_rd034_upstream_recheck_smoke.py --gate g28.p0.m_d.rd034_upstream_recheck` | `milestones/g28/g28_m_d_rd034_upstream_recheck_evidence_schema.json` | RD-034 上游复查:真跑 ci/meshrt_probe_smoke.py(spirv-cross 拒 raygen 探针新鲜——非零退出 = blocked 证据新鲜;意外成功翻红提醒复评)+ deferred.json RD-034 status/history 核验(G28.3 行只追加)——解锁/维持 blocked 均合法诚实终态零冒充 | **G28.3** | post-interlock actual-next-free allocation |
| **M-e** | `g28.p0.m_e.closed_gate_no_regression` | `py -3 ci/g28_closed_gate_no_regression_smoke.py --gate g28.p0.m_e.closed_gate_no_regression` | `milestones/g28/g28_m_e_closed_gate_no_regression_evidence_schema.json` | G27 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g28_` 前缀不抢 latest | **G28.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G28.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G28 承接三行（M100-high → M-a/M-b 承载；M52 → M-c 承载；RD-034 → M-d 承载）；上游法定输入 = milestones/g25/g25_campaign_handover_registry.json M100-high/M52 行 + registry/deferred.json RD-034/RD-040 + milestones/g21/g21_rd040_subitem_registry.json(五分项 reeval_anchor)+ src/rurix-render/src/gi/restir_reservoir.rs(G21 host 参考臂,本期 0-byte 冻结面)。

---

## 4. 双向一致声明

本表 §1 五行与 G28_CONTRACT.md §4.2 逐字相等；key 命名空间 `g28.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G28.1 治理覆盖

```text
g28.wave.1.acceptance_map         步骤 477
  py -3 ci/g28_acceptance_map_check.py --gate g28.wave.1.acceptance_map
g28.wave.1.candidate_decisions    步骤 478
  py -3 ci/g28_candidate_decisions_check.py --gate g28.wave.1.candidate_decisions
g28.gov.implementation_interlock  步骤 479
  py -3 ci/g28_interlock_check.py --gate g28.gov.implementation_interlock
```

---

## 6. G28.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g28_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G26-G30」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g28-closed`（五期串行战役第三期收口）。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G28.1 初版：五 P0 行冻结。 |
