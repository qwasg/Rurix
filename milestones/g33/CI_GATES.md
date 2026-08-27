<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C 验收门 Task C18） -->
# G33 CI_GATES — 里程碑冒烟门登记（商业化期 · 波 C 批次）

> 事实源 = [G33_CONTRACT.md](G33_CONTRACT.md)。本表只登记门 key / 脚本 / 步骤号口径，不复述判据。波 C 门 key 沿用交付冻结字面 `g31.waveC.*`（G31+ 战役门族同波 A `g31.waveA.*` / 波 B `g31.waveB.*` 律）。

## 1. 波 C 交付十八门（C1~C17；未占 CI 数字步骤）

symbolic gate key 与脚本名 = 波 C 交付即冻结字面；**未占 CI 数字步骤**（pr-smoke.yml 无 g31/g33 条目——波 C 门为本地/device 真跑门，非 pr-smoke 秒级核验面；落盘前实测 registry/number_ledger.json CI_step.next_free=525 维持零消费）。

| 步骤 | gate key | 脚本 | C18 验收复跑状态（2026-08-26 新鲜） |
|---|---|---|---|
| 未占号 | g31.waveC.sdk | ci/g31_renderer_sdk_smoke.py | PASS（evidence/g31_renderer_sdk_20260826T205708Z.json；digest==Stage A 锚 c1d28ad7… + 帧时 mean=2.1572ms） |
| 未占号 | g31.waveC.docs | ci/g31_renderer_docs_smoke.py | PASS（evidence/g31_renderer_docs_20260826T205521Z.json） |
| 未占号 | g31.waveC.capability | ci/g31_capability_fallback_smoke.py | PASS（evidence/g31_capability_fallback_20260826T210628Z.json） |
| 未占号 | g31.waveC.robustness | ci/g31_robustness_smoke.py | PASS（evidence/g31_robustness_20260826T212406Z.json；窗口风暴 121 零崩 + soak 故障臂新鲜 1000 帧） |
| 未占号 | g31.waveC.dist | ci/g31_sdk_dist_smoke.py | PASS（evidence/g31_sdk_dist_20260826T205842Z.json；离线可建 digest==锚 + EA1 回归绿） |
| 未占号 | g31.waveC.license | ci/g31_vendor_license_smoke.py | PASS（evidence/g31_vendor_license_20260826T205856Z.json；cleared 15/conditional 1，GAP-01~03 维持 open） |
| 未占号 | g31.waveC.profiling | ci/g31_profiling_smoke.py | PASS（evidence/g31_profiling_20260826T212626Z.json） |
| 未占号 | g31.waveC.support | ci/g31_support_policy_smoke.py | PASS（evidence/g31_support_policy_20260826T210000Z.json） |
| 未占号 | g31.waveC.ngx_decomp | ci/g31_ngx_decomposition_smoke.py | **FAIL 诚实红**（诊断件 .tmp/g31_gates/ngx_decomp/gate_fail_20260826T210732Z.json：6/7 facts PASS，canonical_ratio_not_worsened 红 fresh ratio=0.957606<0.960479；digest 零漂移维持——性能轨迹面诚实红终态合法不冒充） |
| 未占号 | g31.waveC.rd027 | ci/g31_rd027_poison_guard.py（--gate 无 key 参数） | PASS（evidence/g31_rd027_poison_guard_20260826T211300Z.json；毒确认腿 hang_timeout 维持） |
| 未占号 | g31.waveC.p4stream | ci/g31_p4_streaming_smoke.py | PASS（evidence/g31_p4_streaming_20260826T213056Z.json） |
| 未占号 | g31.waveC.hlodl4 | ci/g31_hlod_l4_smoke.py | PASS（evidence/g31_hlod_l4_20260826T213134Z.json；rejudged-four-tier-chain 登记维持） |
| 未占号 | g31.waveC.svt | ci/g31_svt_smoke.py | PASS（evidence/g31_svt_gate_20260826T213146Z.json） |
| 未占号 | g31.waveC.ktx2 | ci/g31_ktx2_smoke.py | PASS（evidence/g31_ktx2_gate_20260826T214234Z.json；M83 门复跑绿） |
| 未占号 | g31.waveC.rtpipeline | ci/g31_rt_pipeline_smoke.py | PASS（evidence/g31_rt_pipeline_20260826T214507Z.json；SER 新鲜 ratio=0.519489） |
| 未占号 | g31.waveC.meshbench | ci/g31_mesh_vs_raster_bench.py | PASS（evidence/g31_mesh_vs_raster_bench_20260826T214521Z.json） |
| 未占号 | g31.waveC.rejudgment | ci/g31_rejudgment_smoke.py | PASS（evidence/g31_rejudgment_windows_20260826T211131Z.json） |
| 未占号 | g31.waveC.blockedprobes | ci/g31_blocked_probes_smoke.py | PASS（evidence/g31_blocked_probes_20260826T211436Z.json；12 探针零冒充 + 活体 10 腿 + device ok） |

## 2. 波 C 验收门（C18，本波 materialize——登记面不设硬门不占号）

C18 = 验收登记面（同波 B B8 律：无 ci 脚本、无 evidence schema 新族、check_schemas 仅 `g33_baseline_` 一处纯追加），六面判据与实测 facts 见 G33_CONTRACT §8：

| 面 | 结论件 | 结论 |
|---|---|---|
| 终验三面复跑 | 上表 sdk/dist/docs 三行新鲜 evidence | 3/3 PASS（双链 digest==Stage A 锚 c1d28ad7…） |
| 发布件核验 | license 行新鲜 evidence + milestones/g31/g31_vendor_license_matrix.json + docs/renderer/release_checklist.md | PASS + GAP-01~03 维持 open（「附带义务未闭前不以对应形态发布」口径在案） |
| 全量回归 | 守卫五条 + 波 A 5/5 + 波 B 4/5 + 波 C 17/18 + 三面锚 | 26 PASS + 2 诚实红如实登记；budget_eval --strict 321 pass 0 skip 零 estimated |
| 三面锚 | evidence/g31_wave_a_anchor_check_20260826T230806Z.json + g16_m_g_absolute_quality_closure_20260826T231716Z.json | digest 18/18 零漂移 + M-g 18/18 VERDICT=PASS + 焦点格中位 ratio 0.957894 诚实红维持 |
| soak 汇总 | 波 A 10010 在案 + C4 故障臂在案 1010 + 本日故障臂 1000 + SDK 面 1010 | 四面零崩零泄漏在案/新鲜 |
| 战役总登记 | milestones/g31_plus_campaign_record.md | 56 项逐项终态映射（兑现门 evidence 指针 / 维持 open 锚 / 诚实红项） |

## 3. evidence schema 登记

波 C 各门 evidence schema 全族 = milestones/g31/ 下二十四件（g31_renderer_sdk_/g31_renderer_docs_/g31_capability_fallback_/g31_robustness_/g31_sdk_dist_/g31_vendor_license_/g31_profiling_/g31_support_policy_/g31_ngx_decomposition_/g31_rd027_poison_guard_/g31_p4_streaming_/g31_hlod_l4_/g31_svt_(gate|harness)/g31_ktx2_(gate|ab)/g31_rt_pipeline_/g31_ser_gain_estimate_/g31_mesh_vs_raster_bench_/g31_rejudgment_windows_/g31_blocked_probes_ 各 schema + 登记件），check_schemas 路由 = 各批三处纯追加在案（详见 milestones/g31/CI_GATES.md §3 族谱）。**C18 验收批同律一处纯追加**：`g33_baseline_` 快检件跳过路由（同 `g31_baseline_`/`g32_baseline_` 律——budget_eval eval_entry 通用路 results.trimmed_mean 消费，无映射前缀跳过；2026-08-26 重放核验 check_schemas PASS）。

## 4. 编号纪律

- CI 数字步骤：本波零消费（next_free=525 实测维持；后续波若进 pr-smoke.yml 按 actual next_free 顺位领取，禁预占）。
- RFC/U 段各一件消费在案（RFC-0048 C15 / U-59 C1）；RXS/RD/SG/MR/D/RX_error 共享段零消费（详见 G33_CONTRACT front matter rfc_required 实测面）。
