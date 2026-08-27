<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 B 验收门 Task B8） -->
# G32 CI_GATES — 里程碑冒烟门登记（画面完整期 · 波 B 批次）

> 事实源 = [G32_CONTRACT.md](G32_CONTRACT.md)。本表只登记门 key / 脚本 / 步骤号口径，不复述判据。

## 1. 波 B 实现五硬门（B1~B5，G31+ 波 B 批次执行，gate key = 交付冻结字面 `g31.waveB.*`）

symbolic gate key 与脚本名 = 波 B 交付即冻结字面；**未占 CI 数字步骤**（pr-smoke.yml 无 g31/g32 条目——波 B 门为本地/device 真跑门，非 pr-smoke 秒级核验面；落盘前实测 registry/number_ledger.json CI_step.next_free=525 维持零消费）。

| 步骤 | gate key | 脚本 | 状态 |
|---|---|---|---|
| 未占号 | g31.waveB.hzb | ci/g31_hzb_wiring_smoke.py | PASS（evidence/g31_hzb_wiring_20260826T062758Z.json） |
| 未占号 | g31.waveB.restir | ci/g31_restir_wiring_smoke.py | PASS（evidence/g31_restir_wiring_20260826T002545Z.json） |
| 未占号 | g31.waveB.slab | ci/g31_slab_wiring_smoke.py | PASS（evidence/g31_slab_wiring_gate_20260826T001204Z.json） |
| 未占号 | g31.waveB.texture | ci/g31_texture_sampling_smoke.py | PASS（evidence/g31_texture_sampling_gate_20260826T082640Z.json） |
| 未占号 | g31.waveB.skinning | ci/g31_skinning_wiring_smoke.py | PASS（evidence/g31_skinning_wiring_20260826T041617Z.json） |

## 2. 波 B 评估窗（B6/B7，登记面无硬门）

B6 GI 默认档 measured 权衡窗与 B7 OIT/半透明评估窗 = 评估登记面（决策只追加），**不设硬门不占号**：无 ci 脚本、无 evidence schema、check_schemas 零消费。结论落盘 = milestones/g31/ 下两份只追加 JSON（measured 数字全部来自真实命令输出，既有锚/bench 默认面 0-byte）。

| 窗 | 结论件 | 结论 |
|---|---|---|
| B6 GI 默认档 | milestones/g31/g31_gi_default_tier_decision.json | maintain_default_off（off 1.79~1.93ms vs on 7.03ms 生产口径 ×3.64~3.93；画质 +10.05% luma 但对 UE Lumen 在案诚实红未闭） |
| B7 OIT/半透明 | milestones/g31/g31_oit_evaluation_window.json | not_triggered（压测闭集机核全 OPAQUE；oit/ 维持 M120 测量 harness 态；strand 档锚未命中维持） |

## 3. 波 B 验收门（B8，本波 materialize——登记面不设硬门不占号）

B8 = 验收登记面（同 B6/B7 律：无 ci 脚本、无 evidence schema、check_schemas 零消费），六面判据与实测 facts 见 G32_CONTRACT §8：

| 面 | 结论件 | 结论 |
|---|---|---|
| 组合矩阵 | milestones/g31/g31_waveb_combo_matrix.json | 可组合臂 5/5 真跑绿（双跑 digest 位级一致）+ 互斥 12/12 fail-closed 拒跑 exit=1 |
| 游戏画面 demo 定版 | 同上 demo_selection 节 | --textures on + orbit 真窗口 200+10 帧双跑位级，real_render 5.113/5.431ms + present 1.004/1.013ms 双口径 |
| 零降级三面 | evidence/g31_wave_a_anchor_check_20260826T*.json + g16_m_g_absolute_quality_closure_20260826T*.json | Stage A digest 18/18 零漂移 + G16 M-g 18/18 VERDICT=PASS + 焦点格 fresh ratio ≥ 在案 0.960479 |
| RD-045 复核 | milestones/g31/g31_waveb_rd045_observation_results.json | 波 B 各臂 digest 锚零漂移；三件 0/3 维持 maintain-open 不冒充 |
| 守卫套件 | — | check_structure/check_schemas/check_number_ledger/trace_matrix --check/budget_eval 全 exit 0 |

## 4. evidence schema 登记（milestones/g31/，B1~B5 各批在案）

| schema | 产证脚本 |
|---|---|
| g31_hzb_wiring_evidence_schema.json | ci/g31_hzb_wiring_smoke.py（B1；PASS-only 闭集 schema） |
| g31_restir_wiring_evidence_schema.json | ci/g31_restir_wiring_smoke.py（B2） |
| g31_slab_wiring_evidence_schema.json | ci/g31_slab_wiring_smoke.py（B3；harness 真跑件） |
| g31_slab_wiring_gate_evidence_schema.json | ci/g31_slab_wiring_smoke.py（B3；门裁决件） |
| g31_texture_sampling_evidence_schema.json | ci/g31_texture_sampling_smoke.py（B4；harness 真跑件 --textures on 腿） |
| g31_texture_sampling_gate_evidence_schema.json | ci/g31_texture_sampling_smoke.py（B4；门裁决件） |
| g31_skinning_wiring_evidence_schema.json | ci/g31_skinning_wiring_smoke.py（B5） |

ci/check_schemas.py 路由 = B1~B5 各批三处纯追加在案（前缀路由 `g31_hzb_wiring_` / `g31_restir_wiring_` / `g31_slab_wiring_gate_` 先于 `g31_slab_wiring_` / `g31_texture_sampling_gate_` 先于 `g31_texture_sampling_` / `g31_skinning_wiring_`——前缀包含长前缀先匹配；与全族前缀及 gpu fallthrough 互不包含）。B8 验收批同律一处纯追加：`g32_baseline_` 快检件跳过路由（同 `g31_baseline_` 律——budget_eval eval_entry 通用路 results.trimmed_mean 消费，无映射前缀跳过）。

## 5. 编号纪律

- CI 数字步骤：本期零消费（next_free=525 实测维持；后续波若进 pr-smoke.yml 按 actual next_free 顺位领取，禁预占）。
- RFC/RXS/RD/U/SG/MR/D/RX_error 共享段：本期零消费（波 B = 既有语义面与 G26~G29 冻结 kernel 0-byte 消费，零新条款零新 RFC——详见 G32_CONTRACT front matter rfc_required；RD-045 history 只追加扩窗登记，RD 编号段零消费）。
