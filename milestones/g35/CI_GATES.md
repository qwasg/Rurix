<!-- Assisted-by: cursor:claude-fable-5（G35-0 治理文档套件起草批） -->
# G35 CI_GATES — 里程碑冒烟门登记（GPU 粒子系统期）

> 事实源 = [G35_CONTRACT.md](G35_CONTRACT.md)。本表只登记门 key / 脚本 / 步骤号口径，不复述判据。

## 1. 九波门登记

symbolic gate key 与脚本名 = 本批冻结字面；**未占 CI 数字步骤**（pr-smoke.yml 无 g35 条目——G35 九门均为本地/device 真跑门，非 pr-smoke 秒级核验面，G31/G34 先例；[registry/number_ledger.json](../../registry/number_ledger.json) CI_step.next_free=525 零消费声明，收口验收批实测核验维持）。

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g35.wave1.primitives | ci/g35_primitives_smoke.py |
| 未占号 | g35.wave2.particle_core | ci/g35_particle_core_smoke.py |
| 未占号 | g35.wave3.render | ci/g35_render_wiring_smoke.py |
| 未占号 | g35.wave4.sort_oit | ci/g35_sort_oit_smoke.py |
| 未占号 | g35.wave5.collision | ci/g35_collision_smoke.py |
| 未占号 | g35.wave6.events | ci/g35_events_smoke.py |
| 未占号 | g35.wave7.fluids | ci/g35_fluids_smoke.py |
| 未占号 | g35.wave8.authoring | ci/g35_authoring_smoke.py |
| 未占号 | g35.wave9.replay | ci/g35_replay_smoke.py |

各门 facts 闭集逐字见 [G35_CONTRACT.md](G35_CONTRACT.md) front matter `acceptance_gates`（G-G35-1~G-G35-9）；判据字面 = 各门脚本 docstring（随波交付冻结）；实测数字待各波验收批填写，禁预支。九门三态同律：无 Vulkan loader/设备/场景资产/SPV → DEV_ENV_DEGRADE 退 0 如实登记不冒充 PASS；RURIX_REQUIRE_REAL=1 翻硬 FAIL。

## 2. 验收面（G-G35-10，登记面无新硬门不占号）

零降级回归锚三面复用既有门 [ci/g31_wave_a_anchor_check.py](../../ci/g31_wave_a_anchor_check.py)（Stage A digest 锚 18/18 + G16plus M-g 18/18 + G17-MD-F1 焦点格诚实红不恶化）——不新设脚本不占号；soak = 粒子车道 `--particles on` ≥5000 帧零崩 + validation 静默，close-out 只追加登记无新硬门（[../g34/CI_GATES.md](../g34/CI_GATES.md) §3 / [../g31/CI_GATES.md](../g31/CI_GATES.md) §2.6 登记面律）；守卫套件七条 + 九门新鲜复跑为程序面；RFC-0049 Agent Approved 在案为收口前置（D-409 对抗评审完成）。实测 facts 落盘 = [G35_CONTRACT.md](G35_CONTRACT.md) §8 close-out（收口验收批只追加）。

## 3. evidence schema 登记（milestones/g35/）

九 schema 文件名与产证脚本映射 = 本批冻结字面（schema 文件与 check_schemas 路由随各波实现批三处纯追加落地，不预放空 schema）：

| schema | 产证脚本 |
|---|---|
| g35_primitives_gate_evidence_schema.json | ci/g35_primitives_smoke.py（G35-1；门裁决件——harness 真跑件留 .tmp 工作区不注册，数字经门裁决件蒸馏，G31 波 B Task B1 律） |
| g35_particle_core_gate_evidence_schema.json | ci/g35_particle_core_smoke.py（G35-2；门裁决件） |
| g35_render_gate_evidence_schema.json | ci/g35_render_wiring_smoke.py（G35-3；门裁决件） |
| g35_sort_oit_gate_evidence_schema.json | ci/g35_sort_oit_smoke.py（G35-4；门裁决件） |
| g35_collision_gate_evidence_schema.json | ci/g35_collision_smoke.py（G35-5；门裁决件） |
| g35_events_gate_evidence_schema.json | ci/g35_events_smoke.py（G35-6；门裁决件） |
| g35_fluids_gate_evidence_schema.json | ci/g35_fluids_smoke.py（G35-7；门裁决件） |
| g35_authoring_gate_evidence_schema.json | ci/g35_authoring_smoke.py（G35-8；门裁决件） |
| g35_replay_gate_evidence_schema.json | ci/g35_replay_smoke.py（G35-9；门裁决件） |

**前缀分岔分析**：evidence 前缀闭集九支 = `g35_primitives_` / `g35_particle_core_` / `g35_render_` / `g35_sort_oit_` / `g35_collision_` / `g35_events_` / `g35_fluids_` / `g35_authoring_` / `g35_replay_`。族内互不包含：九支首段（primitives / particle_core / render / sort_oit / collision / events / fluids / authoring / replay）两两全串互不包含——其中 `g35_particle_core_` 与 `g35_primitives_` 在 `g35_p` 后分岔（a≠r）；`g35_render_` 与 `g35_replay_` 在 `g35_re` 后分岔（n≠p），均非彼此前缀；**注意 `g35_particle_core_` 与假想 `g35_particle_` 前缀关系——族内只有前者，`g35_particle_` 不作为任何注册前缀出现，无歧义无遮蔽**。与 g31_*/g34_* 全族互不包含（`g35_` 与 `g31_`/`g34_` 第 3 字符分岔 5≠1/4），与 gpu fallthrough 及其余既有全族互不包含，既有路由 0-byte。gate 长前缀纪律：本期九支均只注册 `*_gate_` 裁决件 schema（harness 真跑件留 .tmp 不注册），无「gate 长前缀先于短前缀」的匹配序问题；若后续波补注册 harness 件 schema，须按 G34 律 gate 长前缀先匹配并在本表修订行登记。

## 4. 编号纪律

- CI 数字步骤：零消费声明（九门均 symbolic gate key 未占号，pr-smoke.yml 无 g35 条目；**CI_step.next_free=525 维持**——收口验收批实测核验；后续若进 pr-smoke.yml 按 actual next_free 顺位领取，禁预占）。
- RFC 段：RFC-0049 单号（落盘前实测 namespaces.RFC next_free=49 顺位领取，v1.191 校准同批；Draft 待 D-409 对抗评审，Agent Approved 为 G-G35-10 收口前置）。
- RXS/RD/U/SG/MR/D/RX_error 共享段：零消费（零新 RXS 声明——渲染器是库不进语言 06 §8.3，G5/G6/G31/G34 先例，kernel 全用现有语言面；RX_error 零新码——库面违例走 typed Err 镜像 RX6029/6030 口径；RD 确需自 46 顺位，评估窗登记不预造——详见 [G35_CONTRACT.md](G35_CONTRACT.md) front matter rfc_required 与 [../../rfcs/0049-gpu-particle-system.md](../../rfcs/0049-gpu-particle-system.md) §5）。
