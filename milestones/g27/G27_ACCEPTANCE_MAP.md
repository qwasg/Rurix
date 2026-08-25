<!-- Assisted-by: Cursor Agent(G27.1 治理波) -->
# G27_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G27.1 治理交付物；事实源为 [G27_CONTRACT.md](G27_CONTRACT.md) v1.0。
> **编号纪律**：P0 行 numeric_step = `post-interlock actual-next-free allocation`；治理三门步骤 461/462/463。
> **证据纪律**：表内脚本与 schema 为实现 PR 强制目标路径。

---

## 1. P0 硬门（精确 5 行）

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据(逐字) | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g27.p0.m_a.hzb_device_kernel` | `py -3 ci/g27_hzb_device_kernel_smoke.py --gate g27.p0.m_a.hzb_device_kernel` | `milestones/g27/g27_m_a_hzb_device_kernel_evidence_schema.json` | HZB device 化兑现:kernels/g27_hzb_reduce.rx + g27_hzb_test.rx(rurixc --target vulkan 产 SPV + spirv-val 通过)经 vk::run_compute 派发——金字塔逐级 farther-of 归约 device 化(mips 与 host HzbPyramid::build 逐级位级相等)+ rect 测试 device 化(mip 选择/≤2×2 窗/is_farther 判定与 host test_rect 逐字同源,800 rect × 双约定判定序列与 host 全等)+ 零假阳性硬不变量(device 判 Occluded ⇒ exact_rect_occluded 同判)+ device 双跑位级一致 + 篡改 RED 臂检出;host 参考臂 geometry/hzb.rs 0-byte;device 环境不可用时 SKIP 如实登记不冒充 | **G27.2** | post-interlock actual-next-free allocation |
| **M-b** | `g27.p0.m_b.m61_mesh_shader_rejudgment` | `py -3 ci/g27_m61_mesh_shader_rejudgment_smoke.py --gate g27.p0.m_b.m61_mesh_shader_rejudgment` | `milestones/g27/g27_m_b_m61_mesh_shader_rejudgment_evidence_schema.json` | M61 重判窗兑现(RFC-0034 只追加程序):重判条件两半机器盘点——HZB device 化半边(M-a 绿件只读盘点)+ cluster P4 差距闭集清零半边(g20_cluster_streaming_p4_gap.json 四行 open 状态实测)+ mesh shader HW 性能差 measured 证据树内搜索(searched-paths manifest 必填)——条件未全齐 → maintain-no-go 只追加再判记录(RFC-0034 重判表 + 本期 evidence);全齐 → 重判程序启动;零冒充 | **G27.2** | post-interlock actual-next-free allocation |
| **M-c** | `g27.p0.m_c.cluster_p4_gap_rejudgment` | `py -3 ci/g27_cluster_p4_gap_rejudgment_smoke.py --gate g27.p0.m_c.cluster_p4_gap_rejudgment` | `milestones/g27/g27_m_c_cluster_p4_gap_rejudgment_evidence_schema.json` | cluster P4 差距闭集重判:四行(P4-1~P4-4)逐行 reeval——P4-2 依赖面(HZB device 化)本期解除事实登记 + 各行现面零实现树内实测(streaming/ 模块 cluster 载荷面检索)——清零 → closed-go;未清零 → 维持 open 登记 milestones/g27/g27_cluster_p4_rejudgment.json(g20 差距表原文 0-byte 不回写);RD-039 history 只追加 | **G27.3** | post-interlock actual-next-free allocation |
| **M-d** | `g27.p0.m_d.hlod_l4_counter_rejudgment` | `py -3 ci/g27_hlod_l4_counter_rejudgment_smoke.py --gate g27.p0.m_d.hlod_l4_counter_rejudgment` | `milestones/g27/g27_m_d_hlod_l4_counter_rejudgment_evidence_schema.json` | M98-l4 重判窗条件核验:重判条件两半树内实测——HLOD proxy 追踪 device 腿(src 检索零实现登记)+ L4 计数器接入(gi/fallback_chain.rs L4 槽位恒零/fail-closed 入口实测 + world/hlod.rs 接口面就绪盘点)——任一半命中 → 重判程序启动;均未命中 → 维持 L1/L2/L3 三级链诚实登记,承接锚只追加 | **G27.3** | post-interlock actual-next-free allocation |
| **M-e** | `g27.p0.m_e.closed_gate_no_regression` | `py -3 ci/g27_closed_gate_no_regression_smoke.py --gate g27.p0.m_e.closed_gate_no_regression` | `milestones/g27/g27_m_e_closed_gate_no_regression_evidence_schema.json` | G26 受影响门 `--verify-latest` 全绿零降级;禁 `--gate` 旧脚本;`g27_` 前缀不抢 latest | **G27.4** | post-interlock actual-next-free allocation |

---

## 2. 已 go P1 硬门（零行）

G27.1 无 go 的 P1 行——候选决策实现门全为 P0。

---

## 3. 条件型登记面

G27 承接三行（M61 → M-b 承载；M98-l4 → M-d 承载；RD-039-mesh → M-a/M-c 承载）；上游法定输入 = milestones/g25/g25_campaign_handover_registry.json M61/M98-l4 行 + registry/deferred.json RD-039 + milestones/g20/g20_cluster_streaming_p4_gap.json(四行差距闭集,本期只读重判不回写)+ src/rurix-render/src/geometry/hzb.rs(G20 host 参考臂,本期 0-byte 冻结面)。

---

## 4. 双向一致声明

本表 §1 五行与 G27_CONTRACT.md §4.2 逐字相等；key 命名空间 `g27.p0.m_<a~e>.<slug>` 唯一。

---

## 5. G27.1 治理覆盖

```text
g27.wave.1.acceptance_map         步骤 461
  py -3 ci/g27_acceptance_map_check.py --gate g27.wave.1.acceptance_map
g27.wave.1.candidate_decisions    步骤 462
  py -3 ci/g27_candidate_decisions_check.py --gate g27.wave.1.candidate_decisions
g27.gov.implementation_interlock  步骤 463
  py -3 ci/g27_interlock_check.py --gate g27.gov.implementation_interlock
```

---

## 6. G27.2 硬互锁

`implementation_status: blocked` 解锁须：治理两门 PASS + `ci/g27_interlock_check.py --require-ready` READY + 用户战役指令「帮我一次性完成G26-G30」留痕。

---

## 7. Close-out 审计

M-a~M-e 五 P0 + P2 穷举 + soak ≥1800s + close-out 八 facts READY → tag `g27-closed`（五期串行战役第二期收口）。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G27.1 初版：五 P0 行冻结。 |
