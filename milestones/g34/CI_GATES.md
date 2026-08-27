<!-- Assisted-by: Cursor:Claude（G34 全特性合流收口批） -->
# G34 CI_GATES — 里程碑冒烟门登记（全特性合流期）

> 事实源 = [G34_CONTRACT.md](G34_CONTRACT.md)。本表只登记门 key / 脚本 / 步骤号口径，不复述判据。

## 1. G34-1 合流地基门（已验收）

symbolic gate key 与脚本名 = 交付即冻结字面；**未占 CI 数字步骤**（pr-smoke.yml 无 g34 条目——G34 三门均为本地/device 真跑门，非 pr-smoke 秒级核验面；[registry/number_ledger.json](../../registry/number_ledger.json) CI_step.next_free=525 零消费声明，收口验收批实测核验维持）。

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g34.wave1.unified | [ci/g34_unified_lane_smoke.py](../../ci/g34_unified_lane_smoke.py) |

八 facts 闭集（kernels_spv_valid / default_faces_bitexact_anchor / merged_semantics_host_parity / determinism_double_run / dyn_position_verified / per_feature_digest_discrimination / stage_a_anchor_replay / frame_ms_measured）；PASS 证据 = [evidence/g34_unified_lane_gate_20260827T041754Z.json](../../evidence/g34_unified_lane_gate_20260827T041754Z.json)（八 facts 全绿：缺省面 == Stage A 锚 sha256:c1d28ad73783cc3c… 位级 MATCH、host 对拍 p100=3.968658857047558e-04 ≤ 容差 7.937317714095116e-04〔[g34_budget.json](g34_budget.json) `g34.unified_lane.host_parity_tol` 程序读〕、双跑 74 帧位级、baseline=6.9565ms / full=9.0033ms measured_local）。

## 2. G34-2/G34-3 两门（本批收口）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g34.wave2.hzb | ci/g34_hzb_unified_smoke.py |
| 未占号 | g34.wave2.skin | [ci/g34_skin_unified_smoke.py](../../ci/g34_skin_unified_smoke.py) |

- **g34.wave2.hzb**（G34-2 HZB 接统一车道）：六 facts 闭集（kernels_spv_valid / culling_pixel_neutral / hzb_host_parity / determinism_double_run / culling_effective_measured / stage_a_anchor_replay）——判据字面 = 门脚本 docstring；架构字面 = [g34_2_hzb.rs](../../src/rurix-render/src/bin/g34_full_lane/g34_2_hzb.rs) 头注释；实测数字待收口验收批填写。
- **g34.wave2.skin**（G34-3 蒙皮进统一车道）：九面判据闭集（kernels_spv_valid / skin_vertex_bitexact / skin_position_verified / skin_mv_wired / rigid_mv_wired / determinism_double_run / per_feature_digest_discrimination / stage_a_anchor_replay / frame_ms_measured）——判据字面 = 门脚本 docstring；架构字面 = [g34_skin_section.rs](../../src/rurix-render/src/bin/g34_full_lane/g34_skin_section.rs) 头注释；实测数字待收口验收批填写。

两门三态同律：无 Vulkan loader/设备/场景资产/SPV → DEV_ENV_DEGRADE 退 0 如实登记不冒充 PASS；RURIX_REQUIRE_REAL=1 翻硬 FAIL。

## 3. 验收面（G-G34-4，登记面无新硬门不占号）

零降级回归锚三面复用既有门 [ci/g31_wave_a_anchor_check.py](../../ci/g31_wave_a_anchor_check.py)（Stage A digest 锚 18/18 零漂移事实 + G16plus M-g 18/18 独立门复跑 + G17-MD-F1 焦点格新鲜多样本中位如实登记——诚实红维持/恶化均合法终态禁冒充，G-G33-4 字面同律；锚检门焦点格轨迹面严判 verdict 如实留档）——不新设脚本不占号；soak = `g34_full_lane --full` ≥5000 帧零崩 + validation 静默，close-out 只追加登记无新硬门（同 [../g31/CI_GATES.md](../g31/CI_GATES.md) §2.6 B6/B7 登记面律）；守卫套件七条 + 三门新鲜复跑为程序面。实测 facts 落盘 = [G34_CONTRACT.md](G34_CONTRACT.md) §8 close-out（收口验收批只追加）。

## 4. evidence schema 登记（milestones/g34/）

| schema | 产证脚本 |
|---|---|
| g34_unified_lane_evidence_schema.json | ci/g34_unified_lane_smoke.py（G34-1；harness 真跑件——g34_full_lane 各腿归档） |
| g34_unified_lane_gate_evidence_schema.json | ci/g34_unified_lane_smoke.py（G34-1；门裁决件） |
| g34_skin_unified_evidence_schema.json | ci/g34_skin_unified_smoke.py（G34-3 本批；harness 真跑件 --skin on 腿归档） |
| g34_skin_unified_gate_evidence_schema.json | ci/g34_skin_unified_smoke.py（G34-3 本批；门裁决件） |
| g34_hzb_unified_gate_evidence_schema.json | ci/g34_hzb_unified_smoke.py（G34-2 本批；门裁决件——HZB harness 真跑件留 .tmp 工作区不注册，数字经门裁决件蒸馏，同 G31 波 B Task B1 律） |

ci/check_schemas.py 路由：g34_unified_lane 两 schema = 已注册路由在案（[ci/_patch_g34_unified_lane_schemas.py](../../ci/_patch_g34_unified_lane_schemas.py) 三处纯追加重放幂等；gate 长前缀 `g34_unified_lane_gate_` 先于 `g34_unified_lane_` 匹配）。g34_skin_unified 两 schema 路由 = 本批 ci/_patch_g34_skin_schemas.py 三处纯追加（gate 长前缀 `g34_skin_unified_gate_` 先于 `g34_skin_unified_` 匹配）。g34_hzb_unified gate schema 一件路由 = 本批 ci/_patch_g34_hzb_schemas.py 三处纯追加（HZB harness 真跑件留 .tmp 工作区无注册 schema）。**前缀分岔分析**：g34_ 族三支 `g34_unified_lane_` / `g34_skin_unified_` / `g34_hzb_unified_` 首段 u/s/h 互不包含（全串互不包含），各支内 gate 长前缀先匹配；skin 门对照腿 baseline/full_noskin 归档复用 `g34_unified_lane_g34skin_` 前缀走既有 unified_lane 路由（s/u 首段分岔全串互不包含）；与 g31_* 全族及 gpu fallthrough 互不包含，既有路由 0-byte。

## 5. 编号纪律

- CI 数字步骤：零消费声明（三门均 symbolic gate key 未占号，pr-smoke.yml 无 g34 条目；CI_step.next_free=525 维持——收口验收批实测核验；后续若进 pr-smoke.yml 按 actual next_free 顺位领取，禁预占）。
- RFC/RXS/RD/U/SG/MR/D/RX_error 共享段：零消费（统一 kernel 全族引用既有条款 RXS-0405，零新条款零新 RFC——详见 [G34_CONTRACT.md](G34_CONTRACT.md) front matter rfc_required）。
