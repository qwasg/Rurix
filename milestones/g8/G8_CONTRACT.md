---
contract: G8
title: G8 UE5 级渲染器与物理引擎前置能力完成期
status: active
implementation_status: unblocked
active_scope: G8.2_implementation
version: v1.0
date: 2026-08-02
timebox: "G8.1 治理波可与 G7 active 并行；G8.2~G8.8 严格波次，工期在实现互锁开放后由 measured baseline 校准"
rfc_required: "RFC-0019 rendering-platform；RFC-0020 asset-pipeline；RFC-0021 physics-platform（三份 Full RFC，D-409 独立 provenance 对抗性评审后 Approved）"
upstream_docs:
  - "milestones/g8/G8_PLAN.md v1.2（治理门/实现门解耦、波次与退出门的契约上游事实源）"
  - "milestones/g8/G8_CAPABILITY_MATRIX.md v1.1 §11（18 个 P0 与十条成功判据）"
  - "milestones/g7/G7_CONTRACT.md（当前 status=active；G-G7-9 与 RD-038 字面收口边界）"
  - "registry/deferred.json RD-037~044（尤其 RD-038 status/history）"
  - "registry/number_ledger.json reserved_in_flight[G7]/[G8]（共享编号隔离；CI 数字延迟分配）"
  - "04 P-01/P-04/P-07/P-09/P-12/P-13；10 §3/§7/§9.5；14 §1/§3/§4/§5"
implementation_unlock:
  required_all:
    - "G8.1 治理门全部完成且有真实验证记录"
    - "G7_CONTRACT status=closed；G7 active 时不存在实现 override 出口"
    - "RD-038 status=closed，或 G7 closed 后六行接入表终态全填并在 RD-038 history 追加一条独立 override"
    - "共享编号按互锁开放时 actual next_free 重新校准；数字 CI 步骤不得沿用推测号"
in_scope:
  - g8_1_governance_only
  - rfc_0019_rendering_platform
  - rfc_0020_asset_pipeline
  - rfc_0021_physics_platform
  - candidate_decisions_and_rd_mapping
  - p0_acceptance_mapping
  - measured_4070ti_baseline
  - g8_2_shader_rhi_rt_and_rd037
  - g8_3_asset_pipeline_and_page_abi
  - g8_4_multiqueue_and_streaming
  - g8_5_rendering_completion
  - g8_6_physics_platform
  - g8_7_p2_exhaustive_decisions
  - g8_8_stabilization_and_closeout
out_of_scope:
  - g8_2_plus_while_implementation_interlock_is_red
  - g8_1_src_spec_conformance_semantic_implementation
  - g8_1_numbered_workflow_steps_or_stub_scripts
  - formal_ue5_renderer_or_physics_engine_buildout
  - gpu_rigid_body_mainline
  - dxil_rt_forcing_while_rd034_blocked
  - editor_gui_audio_network_engine_multi_gpu_webgpu
  - safe_gpu_operator_platform
  - unmeasured_performance_claims
deferred_refs: [RD-034, RD-037, RD-038, RD-039, RD-040, RD-041, RD-042, RD-043, RD-044]
deliverables:
  - id: D-G8-1
    name: "G8.1 治理四件套：G8_PLAN v1.2、G8_CONTRACT、CI_GATES、非空 measured g8_budget；status=active 且 implementation_status=blocked"
  - id: D-G8-2
    name: "G8.1 完整候选决策表：RD-037~044 每个分项逐行映射 M##/backfill/证据/go|no-go|strategic_override/波次/最终状态"
  - id: D-G8-3
    name: "G8.1 验收映射：18 个 P0 各有独立 symbolic gate key、稳定脚本名、evidence schema 与判据；已 go 的 P1 同步覆盖"
  - id: D-G8-4
    name: "RFC-0019/0020/0021 三份 Full RFC Approved；0019 含 RT 增量、RD-037、多队列、M28 与时域语义，0020 含页格式 ABI，0021 含 replay-first 物理平台"
  - id: D-G8-5
    name: "G8.1 RTX 4070 Ti measured baseline 与非空 g8_budget；零 estimated；互锁 validator 当前诚实报告 blocked"
  - id: D-G8-6
    name: "G8.2 编译器/RHI/RT 增量 + RD-037 单源 gfx submit + 必要 RD-038 接入项"
  - id: D-G8-7
    name: "G8.3 资产闭环、DDC、glTF、M01/M04 版本化页格式 ABI 与 golden"
  - id: D-G8-8
    name: "G8.4 磁盘到 GPU 流送、多队列或诚实单队列 fallback、VT 与几何页独立门"
  - id: D-G8-9
    name: "G8.5a 几何/阴影与 G8.5b 材质/GI/时域/显示闭环"
  - id: D-G8-10
    name: "G8.6a~d replay-first 物理平台、网络/角色、破坏、布料/载具"
  - id: D-G8-11
    name: "G8.7 P2 穷举决策 + G8.8a soak + G8.8b close-out"
acceptance_gates:
  - id: G-G8-1
    check: "治理激活门：用户 2026-08-02 指令与 G8_PLAN v1.2 解耦裁决留痕；G8.0 基线可追溯；G8/G7 提交和 claim 隔离；仅 governance-only 范围 active"
  - id: G-G8-2
    check: "G8.1 完成门：D-G8-1~5 齐备并通过结构/schema/ledger/guardrail/预算核验；18 个 P0 映射无缺行；无 src/spec/conformance 语义实现、无数字 workflow 空步骤；本门通过不自动开放实现"
  - id: G-G8-3
    check: "实现互锁门：G7 closed 且 RD-038 closed，或 G7 closed 后六行接入表终态全填 + RD-038 history 独立 override；再以 actual next_free 分配共享编号。任一条件不满足均保持 implementation_status=blocked"
  - id: G-G8-4
    check: "G8.2 退出门：M50/M89/M29/M30/M31/M32/M85 七个 P0 独立断言全绿；RT 必须是 RXS-0248 之外增量；RD-037 零 Rust 宿主像素断言；接入本波的 RD-038 分项逐字兑现"
  - id: G-G8-5
    check: "G8.3 退出门：M79/M80/M81/M01/M04 五个 P0 独立断言全绿；同输入双构建 hash 相等；DDC 内容寻址；严格 glTF；页格式版本/磁盘内存分离/解码 ABI/golden 冻结后才供 G8.4 消费"
  - id: G-G8-6
    check: "G8.4 退出门：M37 独立断言全绿；磁盘 I/O→解压→上传真实链与迟到页降级有 evidence；GeomPage 必过且不被 VT 替代；VT 按决策 go 真跑或登记 not-triggered；多队列须 RFC-0019，否则只准单队列 fallback"
  - id: G-G8-7A
    check: "G8.5a 退出门：M19 独立断言全绿；VSM 跨帧页缓存/失效/clipmap scroll/local light/non-virtual caster device 对拍；所有 go 的几何项与 RD-038 raster/VSM 接入项各自有证据"
  - id: G-G8-7B
    check: "G8.5b 退出门：M24 独立断言全绿；TSR history resurrection/pixel animation/thin geometry/dynamic resolution/transparent velocity device 序列对拍；所有 go 的材质/GI/显示项及 RD-038 GI/TSR/真帧接入项各自有证据"
  - id: G-G8-8A
    check: "G8.6a 退出门：M66 独立断言全绿；先在 Jolt 5.3 建 replay corpus、状态哈希与首 divergence 定位，再做 5.3↔5.6 A/B；升级失败钉 5.3 不得伪绿升级"
  - id: G-G8-8B
    check: "G8.6b 退出门：M67 独立断言全绿；预测/权威修正/rollback-resimulation/事件去重/平滑全链；CharacterVirtual 与 PhysicsAsset/ragdoll/physical animation authoring 闭环"
  - id: G-G8-8C
    check: "G8.6c 退出门：M68 独立断言全绿；fracture cook→层级 cluster→strain 断键→cache→VFX 事件全链"
  - id: G-G8-8D
    check: "G8.6d 退出门：布料 schema+DCC 导入+碰撞+LOD+独立求解 timeline 与载具产品层各自闭环；GPU 主刚体禁止线不变"
  - id: G-G8-9
    check: "G8.7 决策门：G8_PLAN §2.7 所列 P2 全部逐行 go/no-go/defer-to-G9+，零空行；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述"
  - id: G-G8-10
    check: "G8.8a 稳定门：18 个 P0 与所有 go 的 P1 全量回归；既有步骤 41~92 与 G7 最终 materialize 判据 0-byte；RURIX_REQUIRE_REAL=1 零 mock/host substitution；soak 不低于 30 分钟且 10000 帧；strict budget 非空、零 estimated/skip；新绿不得当日 close"
  - id: G-G8-11
    check: "G8.8b 收口门：验收映射、候选决策、RD 最终状态逐字一致；所有 P0 独立断言均 PASS；evidence/schema/预算终审；§8 只追加后 status active→closed"
guardrails:
  - "双状态不可混同：status=active 仅表示 G8.1 governance-only 已立项；在 G-G8-3 真实通过前 implementation_status=blocked，任何治理完成叙述不得冒充 G8.2 开工"
  - "G7 active 没有实现 override 出口；RD-038 override 只能在 G7 closed 后、六行接入表终态全填后独立追加，且与 M50 等 strategic_override 互不替代"
  - "G8.1 允许 milestones/g8、RFC-0019~0021、G8 专属 claim、只追加 RD history、未编号 validator 与 measured baseline；src/spec/conformance 和编号 workflow 步骤 0-byte"
  - "G8 CI 只冻结 symbolic gate key 与脚本名；numeric_step 一律写 post-G7 actual-next-free allocation。不得假定 G7 只消费步骤 93~96，不得预放空 workflow、空脚本或空 schema"
  - "18 个 P0 必须 18 个独立布尔断言与独立 evidence subject；可共享一次进程执行，但聚合 PASS 不能遮蔽任一子断言 FAIL/SKIP"
  - "缺硬件/工具链仅可 dev_env_degrade 或 SKIP=not-triggered；两者均不充 P0 绿。host oracle、mock、isolated nonzero、现有 RXS-0248 最小 RT 见证均不能替代目标 device 门"
  - "条件型 RD 逐分项 go/no-go/strategic_override；证据与 override 均只追加。未触发项维持 open，不得通过删除验收行获得全绿"
  - "M01/M04 页格式 ABI 必须在 G8.3 冻结；G8.4/M44 只消费不重定。多队列触碰 G5 Barrier EB 三轴须先经 RFC-0019 明示修订"
  - "RFC-0017 物理五纪律、G5 冻结面、G6 GPU 主刚体否决线、milestones/m0~g7 既有契约条款与 CI 判据 0-byte；close-out 证据只追加"
  - "g8_budget 首个实现 PR 前必须非空 measured_local 且有 evaluator；全程零 estimated；性能数字不替代 correctness gate"
  - "新 unsafe 仅在实现互锁开放后按 actual next_free 登记并附 SAFETY；rurix-render 维持 forbid(unsafe_code)"
  - "新文件 LF + 尾换行；本契约合入后正文冻结，激活/验收/收口只追加 §8，除最终 status flip 外不回写既有事实"
---

# G8 契约 — UE5 级前置能力完成期

> 计划：[G8_PLAN.md](G8_PLAN.md) v1.2 · 能力事实源：[G8_CAPABILITY_MATRIX.md](G8_CAPABILITY_MATRIX.md) v1.1 · 机器门：[CI_GATES.md](CI_GATES.md)。
> 当前裁决：**G8.1 governance-only active；G8.2~G8.8 implementation blocked**。`active` 不是实现门绿灯。

---

## 1. 目标与双门状态

G8 的目标是在 G9+ 正式建造前，补齐 UE5 级渲染器和物理引擎所需的平台、资产、流送、渲染与物理生产化前置。G8 不用“目标宏大”替代可验证事实：全部 18 个 P0 必须独立过门，条件型能力按逐分项决策执行，未触发项保持 open。

本契约拆分两种状态：

| 状态 | 当前值 | 含义 |
|---|---|---|
| `status` | `active` | G8.1 治理波已获授权，可落治理资产、RFC、决策/映射、validator、G8 专属 claim 与 measured baseline |
| `implementation_status` | `blocked` | G8.2+ 尚未获准；当前不得改 `src/`、`spec/`、`conformance/`，不得 materialize 数字 CI 步骤 |

G-G8-3 是唯一实现入口：必须先有 **G7 closed**，再满足 **RD-038 closed**，或在 G7 closed 后把六行接入表填成实测终态并为 RD-038 单独追加 override。G7 active 时没有任何 override 可以打开 G8.2。

## 2. 范围与严格波次

### 2.1 G8.1 governance-only

G8.1 只做 D-G8-1~5。允许治理文档、三份 RFC、候选决策、验收映射、G8 专属无冲突 claim、互锁 validator、RTX 4070 Ti baseline 与非空 budget；禁止语义实现和编号 workflow。validator 在当前事实下应明确返回 `blocked`，这正是正确结果，不是失败需要被绕开。

### 2.2 G8.2~G8.8 implementation

实现互锁开放后按以下顺序推进，波次内可并行，波次间不得越级：

```text
G8.2 shader/RHI/RT
  → G8.3 asset/page ABI
  → G8.4 multiqueue/streaming
  → G8.5a geometry/shadow → G8.5b material/GI/temporal/display
  → G8.6a replay → G8.6b network/character → G8.6c destruction → G8.6d cloth/vehicle
  → G8.7 exhaustive decisions → G8.8a soak → G8.8b close-out
```

每波退出门见 YAML `acceptance_gates`；任一上游门未绿，下游 evidence 即使局部成功也不能宣称波次完成。

## 3. G8.1 交付冻结

| ID | 交付 | 退出判据 |
|---|---|---|
| D-G8-1 | 契约四件套与双状态 | PLAN v1.2、CONTRACT、CI_GATES、非空 measured budget 一致；`status=active`、`implementation_status=blocked` |
| D-G8-2 | 候选决策与 RD 总映射 | RD-037~044 每个字面分项一行；原 backfill、证据、裁决、波次、退出门、最终状态无空项；RD-038 启动快照不冒充终态 |
| D-G8-3 | 验收映射 | 18 个 P0 全部有独立 key/script/schema/check；go 的 P1 同步入表；不存在“由邻项代绿” |
| D-G8-4 | RFC-0019~0021 | 三份 Full RFC 均经 D-409 独立 provenance 评审后 Approved；编号登记与 README/ledger 一致 |
| D-G8-5 | baseline、budget、互锁 validator | RTX 4070 Ti measured 数据非空、零 estimated；validator 对当前 G7/RD-038 状态诚实报 blocked；无空 workflow |

G8.1 完成仅关闭治理准备，不改变 G-G8-3 的机器事实。

## 4. 验收门与 18 个 P0 独立断言

### 4.1 波次验收门

G-G8-1~11 以 YAML 头为可提取摘要。[CI_GATES.md](CI_GATES.md) 冻结脚本与 evidence 形态。条件型分项的 `SKIP=not-triggered` 只表示决策已记录，不是成功；设备门的 `dev_env_degrade` 只表示环境缺失，也不是成功。

### 4.2 P0 独立断言

以下 18 行是 close-out 不可合并、不可删减的独立布尔断言。一次 smoke 可以共享启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。

| Symbolic gate key | M## | 最晚波次 | 稳定脚本名 | 独立硬判据 |
|---|---:|---|---|---|
| `g8.p0.m50.rt_pipeline_incremental` | M50 | G8.2 | `ci/g8_rt_pipeline_incremental_smoke.py` | 多 hit group/材质记录、SBT 用户数据、stack sizing、pipeline library 端到端 device 真跑；RFC-0019 子集的 any-hit/intersection/callable 有 RED-GREEN；现有 RXS-0248 最小见证不得代绿 |
| `g8.p0.m89.single_source_gfx_submit` | M89 | G8.2 | `ci/g8_single_source_gfx_smoke.py` | `.rx` gfx 图在零 Rust 宿主出图路径真实 submit，readback 像素断言满足 RD-037 backfill |
| `g8.p0.m29.shader_permutation` | M29 | G8.2 | `ci/g8_shader_permutation_smoke.py` | permutation 域/key、静态裁剪与预算报告可复现；超预算 RED、合法集合 GREEN |
| `g8.p0.m30.pso_cache` | M30 | G8.2 | `ci/g8_pso_cache_smoke.py` | precache→cache/binary→warm hit 闭环；compile-stall 计数器存在且 cold/warm 行为可核 |
| `g8.p0.m31.reflection_hash` | M31 | G8.2 | `ci/g8_reflection_hash_smoke.py` | reflection schema 可序列化；同接口 hash 稳定，ABI 改动必改 hash，hash 进入 DDC 键 |
| `g8.p0.m32.capability_profile` | M32 | G8.2 | `ci/g8_capability_profile_smoke.py` | capability/profile 进入类型检查；受支持用例 GREEN，不支持组合确定性 RED，禁止运行时静默 fallback |
| `g8.p0.m85.shader_manifest_ddc` | M85 | G8.2/3 | `ci/g8_shader_manifest_ddc_smoke.py` | shader/PSO manifest canonical 合并去重并完成 DDC 往返；篡改/缺项 RED |
| `g8.p0.m79.asset_determinism` | M79 | G8.3 | `ci/g8_asset_determinism_smoke.py` | SourceAsset/ImportRecipe/DerivedArtifact/CookProfile canonical schema；同输入独立双构建 artifact hash 逐字节相等 |
| `g8.p0.m80.ddc_content_address` | M80 | G8.3 | `ci/g8_ddc_content_address_smoke.py` | DDC key 覆盖源、依赖、工具版本与 profile；hit/miss/tamper 路径均可核，错误内容不能命中 |
| `g8.p0.m81.gltf_import` | M81 | G8.3 | `ci/g8_gltf_import_smoke.py` | 锁定扩展集的 glTF 2.0 严格导入与 validator GREEN；非法 schema/越界引用 RED |
| `g8.p0.m01.meshlet_page_builder` | M01 | G8.3 | `ci/g8_meshlet_page_builder_smoke.py` | DAG/meshlet builder 产物确定、格式版本在位、golden 稳定；版本变化显式迁移而非静默重解释 |
| `g8.p0.m04.page_format_abi` | M04 | G8.3 | `ci/g8_page_format_abi_smoke.py` | 磁盘/内存页格式分离，压缩编码与 device 解码 ABI golden 往返；未知版本 fail-closed；G8.4 只消费 |
| `g8.p0.m37.streaming_io` | M37 | G8.4 | `ci/g8_streaming_io_smoke.py` | 真实磁盘异步 I/O→解压→上传→GPU 消费链，timeline/provenance 可核；迟到页走可见降级，禁止 host substitution |
| `g8.p0.m19.vsm_page_cache` | M19 | G8.5a | `ci/g8_vsm_page_cache_smoke.py` | VSM 跨帧 cache、物理页池/age、失效分类、clipmap scroll、local light、非虚拟几何 caster 全面 device 对拍 |
| `g8.p0.m24.tsr_contract` | M24 | G8.5b | `ci/g8_tsr_contract_smoke.py` | history resurrection、pixel animation、thin geometry、动态分辨率、透明 velocity 的序列 device 对拍与降级边界 |
| `g8.p0.m66.physics_replay` | M66 | G8.6a | `ci/g8_physics_replay_smoke.py` | Jolt 5.3 固定步世界 capture 完整重演；状态 hash 一致；故障注入可定位首个 divergence；先于 5.6 A/B |
| `g8.p0.m67.network_physics` | M67 | G8.6b | `ci/g8_network_physics_smoke.py` | input/state history、prediction、权威修正、rollback/resimulation、事件去重与平滑全链见证 |
| `g8.p0.m68.fracture_pipeline` | M68 | G8.6c | `ci/g8_fracture_pipeline_smoke.py` | 预破碎 cook→connection/hierarchical cluster→strain 断键→cache→VFX 事件全链见证 |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G8.8b。

## 5. Guardrails

见 YAML `guardrails`。特别强调三点：

1. 治理 active 不等于实现 active。
2. 数字 CI 步骤只能在 G7 close 后读取实际末号再分配；文档中的稳定身份是 symbolic gate key 和脚本名。
3. `closed/open/override/PASS` 都必须来自事实源与追加记录，不从 `partial`、`unresolved`、`SKIP` 推导。

## 6. Deferred 处置

| Deferred | G8 处置 |
|---|---|
| RD-037 | G8.2/M89 P0 正式承接，按 backfill 字面验收 |
| RD-038 | 实现互锁；closed 或 G7 closed 后六行终态接入 + 独立 override，启动快照不改变 status |
| RD-039/040/041/044 | 逐分项 go/no-go/strategic_override；history 只追加，未触发项维持 open |
| RD-034 | DXIL RT 上游 blocked，维持 open，不阻 Vulkan 主腿 |
| RD-042/043 | 研究/观察与 GPU 主刚体否决线维持，不进 G8 主硬门 |

详情始终以 `registry/deferred.json` 为唯一事实源；本表只冻结承接纪律。

## 7. 修订记录与开工裁决

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-02 | 初版契约：按 G8_PLAN v1.2 显式拆分 governance 与 implementation；G8.1 active、G8.2+ blocked；冻结波次门和 18 个 P0 独立断言；CI 数字延迟到 post-G7 actual-next-free allocation。 |

**开工裁决留痕**：用户 2026-08-02 指令“帮我把G8.1前置堵塞修掉”。该指令授权消除治理死锁，不授权伪造 G7/RD-038 完成事实。依 10 §7、P-13 与 D-406，G8.1 governance-only 可与 G7 active 并行；G8.2+ 继续由 G-G8-3 硬阻断。G8.0 不可变基线为 `eb519560`；G7 RFC-0018 台账校准为独立提交 `e599c69a`；G8 的 RFC-0019~0021 claim 见 `registry/number_ledger.json`，RXS/RD/U/RX/数字 CI 均延迟到实现互锁开放后按 actual next_free 领取。

---

## 8. Implementation activation / Close-out（只追加区）

<!-- 首条未来记录只能是 G-G8-3 互锁实测与 implementation_status 解锁凭据；其后追加逐波验收与 close-out。当前不得写 PASS、不得预填 run URL。 -->

### 8.1 G-G8-3 实现互锁实测与 implementation_status 解锁（2026-08-05）

**触发**：用户指令「帮我完成G8.2」。依 10 §7 / P-13 / D-406，agent 自主执行 G8.2 入场，但**不以指令替代机器事实**——本节的解锁凭据全部来自命令输出。

**事实源实测**：

| 事实 | 实测值 | 来源 |
|---|---|---|
| G7 收口 | `G7_CONTRACT.md` front matter `status: closed`；close-out 提交 `5269f96a`，annotated tag `g7-closed`（tag 对象 `41d937f6`） | `git rev-parse g7-closed` / 契约 front matter |
| RD-038 | `registry/deferred.json` `RD-038.status = closed`（G7.7 close-out 逐字审计路径，非六行 override 路径） | 互锁 validator 事实门 ② |
| 六行接入表 | 终态列仍为 `unresolved` ×6 —— **诚实登记**：RD-038 走 `closed` 路径，六行 override 路径未被使用，故终态列不回填、不伪造 | `G8_PLAN` §1.0 |
| G8.1 治理交付 | 七件齐备（PLAN/CONTRACT/CI_GATES/budget/CANDIDATE_DECISIONS/ACCEPTANCE_MAP/CAPABILITY_MATRIX） | 互锁 validator 事实门 ④ |
| RFC-0019~0021 | 三份均 Agent Approved 且独立评审 provenance `Kiro:claude-opus-5 rfc-review-session` ≠ 起草 `Codex:gpt-5` | 互锁 validator 事实门 ⑤ |
| 入场基线 | `6c80dcf0`（本 PR base commit） | `git rev-parse HEAD` |

**验证命令与逐字输出**（`py -3 ci/check_g8_implementation_interlock.py --require-ready`，exit 0）：

```text
[check_g8_implementation_interlock] 事实门（当前可为红）：
  PASS ① G7_CONTRACT status = 'closed'（要求 closed）
  PASS ② RD-038 status = 'closed'；六行接入表终态 = ['unresolved', 'unresolved', 'unresolved', 'unresolved', 'unresolved', 'unresolved']；history 独立 override = False（要求 closed，或 G7 closed 后终态全填 + override）
  PASS ③ G8_PLAN §1.0 接入表行数 = 6（要求 6 行逐行可判）
  PASS ④ G8.1 治理交付齐备（缺 无）
  PASS ⑤ rfcs/0019-rendering-platform.md：Agent Approved；独立评审 provenance ['Kiro:claude-opus-5 rfc-review-session']
  PASS ⑤ rfcs/0020-asset-pipeline.md：Agent Approved；独立评审 provenance ['Kiro:claude-opus-5 rfc-review-session']
  PASS ⑤ rfcs/0021-physics-platform.md：Agent Approved；独立评审 provenance ['Kiro:claude-opus-5 rfc-review-session']
[check_g8_implementation_interlock] 一致性门（红即脚本失败）：
  PASS C1 ledger RFC on_tree_max/next_free = 21/22；rfcs/ 实际末号 = 21（要求台账随 materialize 校准，v1.13/v1.28/v1.29/v1.38 先例）
  PASS C2 ledger reserved_in_flight[G8] claim 在位
  PASS C3 G8_CONTRACT implementation_status = 'blocked'；事实门全绿 = True（事实未全绿时必须保持 blocked，禁止治理完成冒充实现开工）
[check_g8_implementation_interlock] VERDICT = READY
```

**裁决**：G-G8-3 = **PASS**。front matter 由 `implementation_status: blocked` → `unblocked`、`active_scope: G8.1_governance_only` → `G8.2_implementation`。C3 断言在翻转后仍为绿（事实门全绿时允许非 blocked）。§1 状态表与 §2.1/§3 的 G8.1 期叙述是**当时快照**，按只追加纪律不回写；当前有效状态以本节与 front matter 为准。

**本次解锁**不改任何 P0/P1 判据、不改波次结构、不动 G7 车道、不预分配任何数字 CI 步骤或 RXS 号。共享编号的 post-G7 实测基线（`registry/number_ledger.json` v1.46 校准）：`RXS.next_free = 304`、`CI_step.next_free = 97`、`RD.next_free = 45`、`U.next_free = 44`、`RX_error.next_free = 7023`。按 CI_GATES §1.2，数字步骤与 RXS 条款号只能在各自 materialize 的那个 PR 里按当时实测 `next_free` 领取；本节记录的基线不构成预占。

**G8.2 交付顺序（spec-first + RED 先行，G8_PLAN §3）**：spec 条款 PR 先行 → RED 语料 → 实现 + 脚本 + evidence schema + workflow 真步骤 + ledger 校准同 PR。七个 P0（M50/M89/M29/M30/M31/M32/M85）各自独立断言，聚合门 `g8.wave.2.exit` 只汇总不代绿。

`Assisted-by: Kiro:claude-opus-5 g82-entry-session`（影响范围：G8 契约双状态 front matter + 本节 + PLAN/CI_GATES/README 状态镜像 + ledger revision_log；验证方式：本节逐字输出 + §8.1 末列回归命令）

### 8.2 M31 reflection_hash materialize（2026-08-05）

**触发**：G8.2 七个 P0 之一 M31 `g8.p0.m31.reflection_hash` 实现 PR。

**交付物**（同 PR 落，spec-first + RED 先行）：

- **spec 条款**：`spec/rendering_platform.md` RXS-0304~0307（reflection v1 字段闭集 / canonical 序列化 / hash 计算 / 装配期核验）+ `spec/README.md` §4 行与修订行。
- **实现**：`src/rurixc/src/reflection.rs`（reflection v1 模块：canonical bytes + SHA-256 interface hash + JSON 产物 + 装配期核验原语）；`src/rurixc/src/iface_extract.rs`（自 `mir_build::dxil_io` 机械搬迁,I/O 签名 / 资源句柄 / mesh_meta 提取,reflection 与 device MIR 附着同一提取律）；`driver.rs` `--emit=reflection` 接线；`rurixc` CLI `--emit=reflection` 支持。
- **SHA-256**：复用 `rurix-pkg::sha256`（零依赖手写,无循环依赖,不造第三份实现）。
- **RED 语料**：`conformance/reflection/accept/{basic_reflection,mesh_reflection,compute_only,empty_entries}.rx`（4 件）+ `conformance/reflection/reject/{unbounded_sampler_table,duplicate_entry_name,compute_struct_param}.rx`（3 件,头部 `//@ expect-error: RX####` 声明）。
- **CI 脚本**：`ci/g8_reflection_hash_smoke.py`（`--gate` / `--selftest` / 六腿判据 + 语料批跑 + evidence 落盘）。
- **evidence schema**：`milestones/g8/g8_m31_reflection_hash_evidence_schema.json`（Draft-07,16 个 `checks.*` 独立断言,`device_section_state` enum `["not_applicable"]`）。
- **check_schemas.py 路由**：`g8_m31_reflection_hash_` 前缀 → 新 schema。
- **pr-smoke.yml 步骤 97**：`py -3 ci/g8_reflection_hash_smoke.py --gate g8.p0.m31.reflection_hash`。
- **CI_GATES.md §4 M31 行**：`numeric_step` 由 `post-G7 actual-next-free allocation` 回填为 `97`。
- **g8_budget.json**：`g8.counter.reflection_hash_legs` counter + `budget_eval.py` evaluator 分支。
- **number_ledger.json v1.47**：`CI_step.on_tree_max` 96→97、`next_free` 97→98。

**判据六腿**（RFC-0019 §4.4 逐字 + RXS-0304~0307）：

1. 双次构建 canonical bytes 与 digest 逐字节相等（确定性）。
2. 声明序置换 / 语义无关路径扰动 → canonical 与 hash 不变。
3. 仅改函数体 → interface_hash 不变、source_digest 必变。
4. ABI 四轴（binding / resource kind / stage visibility / value type）任一改变 → interface_hash 必变。
5. 空/未实现字段（M29/M32/M50）确定性空编码 + 同名 entry 跨 mod fail-closed + 无界非-SRV 纹理表 fail-closed + compute 形参超闭集 fail-closed。
6. JSON 产物确定性 + 不含路径/文件名/时间戳 + 装配期核验 fail-closed。

**device 段**：`not_applicable`（host/compile 纯 host 门,CI_GATES §6 host-only 行）。

**验收命令**：
```
cargo test -p rurixc --lib reflection
py -3 ci/g8_reflection_hash_smoke.py --gate g8.p0.m31.reflection_hash
py -3 ci/g8_reflection_hash_smoke.py --selftest
py -3 ci/check_schemas.py
py -3 ci/check_g8_acceptance_map.py
```

`Assisted-by: Devin g8-m31-reflection-hash`（影响范围：本节 + CI_GATES §4 M31 行 + ledger v1.47 + g8_budget.json + pr-smoke.yml 步骤 97 + check_schemas.py 路由；验证方式：上述验收命令全绿）

### 8.3 M29 shader_permutation materialize（2026-08-06）

**触发**：G8.2 七个 P0 之一 M29 `g8.p0.m29.shader_permutation` 实现 PR（host 轨第一门，G8.2 设计案 §7 PR-1）。

**交付物**（spec-first 两 commit：条款 commit `c53a3c2c` 先行,实现+治理接线 commit 随后,硬规则 7）：

- **spec 条款**：`spec/rendering_platform.md` v1.1 RXS-0308~0310（permutation 域声明闭集 / canonical key 与 domain digest / 裁剪·预算·报告·选择律）+ **RXS-0304 加性修订**（`permutation_domain_digest`/`variant_key` 真值化,空域路径 0 字节漂移）+ `spec/README.md` v1.71 行。**ledger v1.48 同 commit 校准 M31 滞后**（RXS 303/304→307/308）并领号 0308~0310（→310/311）。
- **实现**：`src/rurixc/src/permutation.rs`（域模型/合法性校验/求解 canonical key/domain digest,纯 host safe）；`#[permutation(axis/forbid/budget)]` attr 解析（parser 加性扩展嵌套 attr 实参）；`driver.rs` `--emit=permutations` + `--permutation-budget=N` + `--permutation-select=KEY`；`reflection.rs` 二字段真值化（空域恒既有常量）。
- **错误码**：RX3019（typeck `shader.permutation_domain_invalid`,RXS-0308/0310）+ RX7023（工具段 `toolchain.permutation_budget_exceeded`,RXS-0310）,error_codes.json + en/zh messages 成对,`bilingual_coverage.py` 113/113 对齐。
- **RED 语料**：`conformance/permutation/accept/{basic_domain,axis_order_permuted,int_axis,empty_domain_entry}.rx`（4 件）+ `reject/{duplicate_axis,empty_value_domain,forbid_unknown_axis}.rx`（RX3019）+ `reject/budget_exceeded.rx`（RX7023）+ `golden/basic_domain_keys.json`。
- **CI 脚本**：`ci/g8_shader_permutation_smoke.py`（`--gate`/`--selftest`/13 项 checks + 语料批跑 + evidence 落盘）。
- **evidence schema**：`milestones/g8/g8_m29_shader_permutation_evidence_schema.json`（Draft-07,13 个 `checks.*` 独立断言,`device_section_state` enum `["not_applicable"]`）。
- **check_schemas.py 路由**：`g8_m29_shader_permutation_` 前缀 → 新 schema。
- **pr-smoke.yml 步骤 98**：`py -3 ci/g8_shader_permutation_smoke.py --gate g8.p0.m29.shader_permutation`。
- **CI_GATES.md §4 M29 行**：`numeric_step` 回填 `98`（v1.4 行）。
- **g8_budget.json v1.2**：`g8.counter.shader_permutation_legs`（≥13）+ `budget_eval.py` evaluator 分支。
- **number_ledger.json v1.49**：`CI_step.on_tree_max` 97→98、`next_free` 98→99；`RX_error.on_tree_max` 7022→7023、`next_free` 7023→7024（RX7023 消费;RX3019 < on_tree_max 小号不 bump,v1.37 先例）。

**判据 13 项**（G8_ACCEPTANCE_MAP §2 M29 行逐字 + RXS-0308~0310）：双次 key 逐字节相等 / 合法集==golden 全等 / 静态不可能组合全部裁剪 / 声明序不变 / 预算 `limit==legal_count` GREEN / `limit==legal_count-1` RED / `enumerated==pruned+emitted` 恒等式 / 超限 axis contribution report / select 合法填 variant_key 且 pipeline_key 分裂 / select 非法确定性错误禁最接近回退 / 空域 reflection 0 漂移 / accept 语料绿 / reject 语料红+码。**不以 M30/M31/M32/M85 任一结果代替**。

**device 段**：`not_applicable`（host/compile 纯 host 门,CI_GATES §6 host-only 行）。

**验收命令**（实测全绿,2026-08-06）：
```
cargo test -p rurixc --lib permutation      # 14 passed
cargo test -p rurixc --lib reflection       # 15 passed(M31 零回归)
cargo test -p rurixc                        # 全套件 0 failed(lib 433)
py -3 ci/g8_shader_permutation_smoke.py --gate g8.p0.m29.shader_permutation   # PASS 13/13
py -3 ci/g8_shader_permutation_smoke.py --selftest                            # PASS
py -3 ci/g8_reflection_hash_smoke.py --gate g8.p0.m31.reflection_hash         # PASS(M31 smoke 零回归)
py -3 ci/bilingual_coverage.py              # 113/113
py -3 ci/check_schemas.py / check_g8_acceptance_map.py / check_number_ledger.py / budget_eval.py
```

`Assisted-by: kimi-k3 subagent g8-m29-shader-permutation`（实现/语料/smoke/schema/错误码登记；中断后由主 agent 复核接手治理接线。影响范围：本节 + CI_GATES §4 M29 行 v1.4 + ledger v1.48/v1.49 + g8_budget v1.2 + pr-smoke.yml 步骤 98 + check_schemas.py 路由 + budget_eval.py 分支；验证方式：上述验收命令全绿）

### 8.4 M32 capability_profile materialize（2026-08-06）

**触发**：G8.2 七个 P0 之一 M32 `g8.p0.m32.capability_profile` 实现 PR（host 轨第二门，G8.2 设计案 §7 PR-2）。

**交付物**（spec-first 三 commit：条款 `bec06980` + 兼容判定精确化 `138897c0` 先行,实现+治理接线随后,硬规则 7）：

- **spec 条款**：`spec/shader_stages.md` v1.7 RXS-0311（`#[requires]` 声明/ID 闭集 v1 十项/隐式推导映射表/调用图并集律/第五 key `capability.unknown_id`）+ `spec/rendering_platform.md` v1.2 RXS-0312（profile 闭集/构建期选择律/fallback,`--emit=capabilities` selection manifest）与 RXS-0313（运行期 snapshot 核验 fail-closed,`verify_profile_snapshot` host 原语）+ v1.3（兼容判定精确化：resources 允许缺失 capability 对应资源类〔accel↔{rt.ray_query,rt.pipeline}〕在 fallback 缺席——修复 v1.2 从严口径与 fallback 腿判据不可同时满足的条款矛盾）+ RXS-0304 修订（capability 二字段真值化）。ledger v1.50（RXS 310/311→313/314）。
- **实现**：`src/rurixc/src/capability_check.rs`（ID 闭集/attr 解析/隐式推导/DefId 级调用图并集/profile 装载与 digest/选择律四分支/兼容判定/`verify_profile_snapshot`,15 单测）；`driver.rs` `--profile`/`--emit=capabilities`；`mir_build.rs` 收集根按 selection 过滤（fallback 选中主 variant 不发射）；`reflection.rs` 二字段真值化（空路径与 M31/M29 基线逐字节 0 漂移,常量实测不动）。
- **错误码**：RX3020~RX3023（typeck 段四枚,en/zh 成对,bilingual 117/117）；`capability.runtime_snapshot_mismatch` 库层 typed Err 不占 RX 码。
- **RED 语料**：`conformance/capability/accept/{requires_supported,implicit_propagation,fallback_low_profile}.rx` + `reject/{missing_required,forbidden_used,fallback_incompatible,unknown_capability_id}.rx`（RX3020~3023）+ `profiles/{high,low,low_with_fallback}.json`。
- **CI 脚本**：`ci/g8_capability_profile_smoke.py`（`--gate`/`--selftest`/14 项 checks,三腿缺一 FAIL）。
- **evidence schema**：`milestones/g8/g8_m32_capability_profile_evidence_schema.json`（Draft-07,14 checks,device `not_applicable`）。
- **治理接线**：check_schemas 路由 + pr-smoke.yml 步骤 99 + CI_GATES v1.5 回填 99 + g8_budget v1.3 `g8.counter.capability_profile_legs`（≥14）+ budget_eval 分支 + ledger v1.51（CI_step 98→99/100;RX3020~3023 < RX_error on_tree_max 小号不 bump）。

**判据三腿**（G8_ACCEPTANCE_MAP §2 M32 行逐字）：accept 腿——支持 profile 的 fixture 类型检查 0 诊断；reject 腿——同一 fixture 移除必需 capability 后以 RFC-0019 冻结 symbolic key 确定性拒录（`capability.missing_required` 消息携带缺失 ID + 首个引入 callee,实测 `requires_supported.rx` high 绿/low 红）；fallback 腿——低 profile 只生成允许的 specialization,选中产物 OpRayQuery* 指令数与 SPV_KHR_ray_query 扩展计数 == 0（高 profile 正对照指令在场,spirv-val 双腿 accept）。三腿缺一即 FAIL。

**device 段**：`not_applicable`（host/compile 纯 host 门,CI_GATES §6 host-only 行）。

**验收命令**（实测全绿,2026-08-06;主 agent 独立复核时发现并行跑多 smoke 会因 cargo 二进制互覆盖假红,串行复跑 PASS,假红 evidence 已删）：
```
cargo test -p rurixc --lib capability       # 15 passed
cargo test -p rurixc                        # 全套件 0 failed
py -3 ci/g8_capability_profile_smoke.py --gate g8.p0.m32.capability_profile   # PASS 14/14
py -3 ci/g8_capability_profile_smoke.py --selftest                            # PASS
py -3 ci/g8_shader_permutation_smoke.py --gate ...   # M29 零回归 PASS
py -3 ci/g8_reflection_hash_smoke.py --gate ...      # M31 零回归 PASS
py -3 ci/bilingual_coverage.py              # 117/117
```

**诚实边界**（实现模块头注释同文）：① 调用图为 DefId 级直调路径调用;固有 impl 方法接收者类型解析在 TBIR 期,不在 AST 调用事实面（v1 加性演进面）。② 四个 RT intrinsic（trace_ray/report_intersection/execute_callable/ignore_intersection）当前未注册可调 lang item（RT 体 lowering 归 M50）,识别 = 单段名 + 「解析到用户 DefId 则不推导」守卫,用户遮蔽零误判;`ray_query_initialize` 为 DefId 级识别。

`Assisted-by: kimi-k3 subagent g8-m32-capability-profile (47c8905d)`（实现/语料/smoke/schema/错误码；主 agent 独立复核 + 治理接线。影响范围：本节 + CI_GATES §4 M32 行 v1.5 + ledger v1.50/v1.51 + g8_budget v1.3 + pr-smoke.yml 步骤 99 + check_schemas.py 路由 + budget_eval.py 分支；验证方式：上述验收命令全绿）

### 8.5 M30 pso_cache materialize（2026-08-06）

**触发**：G8.2 七个 P0 之一 M30 `g8.p0.m30.pso_cache` 实现 PR（cache 轨，G8.2 设计案 §7 PR-3；硬依赖 M31 interface_hash 已在树，与 M29/M32 无硬序）。

**交付物**（spec-first：条款 `988dcefe` 先行，实现+治理接线随后，硬规则 7）：

- **spec 条款**：`spec/vulkan_backend.md` v1.23 RXS-0314（PSO key preimage 七段闭集/collector JSON 字段位）+ RXS-0315（`rurix_pso_cache.bin` 磁盘格式/核验序 fail-closed/rebuild_reason）+ RXS-0316（cold/warm/stall/binary 强制律/VkPipelineCache fallback）。ledger v1.52（RXS 313/314→316/317）。
- **实现**：`src/rurix-rt/src/pso_cache.rs`（key/collector/store/manager；host ≥17 单测）+ `bin/vk_pso_cache`（`--collector-only`/`--cold`/`--warm [--drop-key N]`/`--tamper <schema|version|driver_uuid|keyset>`）+ `vk.rs` FFI append（PipelineCache + `VK_KHR_pipeline_binary` + flags2；unsafe 归 U27/U31 扩注，0 新 U）+ `tests/pso/pso_keys.golden.json`。
- **CI 脚本**：`ci/g8_pso_cache_smoke.py`（`--gate`/`--selftest`/14 项 checks；host collector + device 串行腿，各腿独立 temp dir）。
- **evidence schema**：`milestones/g8/g8_m30_pso_cache_evidence_schema.json`（Draft-07，14 checks，`device_section_state` 含 `executed`/`skipped_dev_env`）。
- **治理接线**：check_schemas 路由 + pr-smoke.yml 步骤 100（`RURIX_REQUIRE_REAL=1`）+ CI_GATES v1.6 回填 100 + g8_budget v1.4 `g8.counter.pso_cache_legs`（device 见证 ≥1）+ budget_eval 分支 + ledger v1.53（CI_step 99→100/101；RXS/RX_error/U 字段 0-byte）。

**判据**（G8_ACCEPTANCE_MAP §2 M30 行逐字 + 设计案 14 checks）：collector key 集 == golden；cold 构建数 == keyset；全新进程 warm `runtime_compile_stalls==0` 且全 key 命中已持久化；篡改 schema/version/driver identity/keyset fail-closed 重建且无误命中；删单 blob 能红（stalls≥1）；扩展在位必走 binary，否则 capability=false + VkPipelineCache fallback；validation=0。

**device 段**：gate real（`RURIX_REQUIRE_REAL=1` 翻硬红；缺 provisioning SKIP=dev-env-degrade 不充绿）。

**验收命令**（实测全绿，2026-08-06，本机 RTX 4070 Ti；branch=binary，warm_stalls=0，drop_stalls=1，validation_errors=0）：
```
cargo test -p rurix-rt --features vulkan --lib pso   # 17 passed
py -3 ci/g8_pso_cache_smoke.py --gate g8.p0.m30.pso_cache   # PASS 14/14
py -3 ci/g8_pso_cache_smoke.py --selftest                    # PASS
```

**诚实边界**：① binary 缺 blob 时驱动磁盘缓存仍可能让 FAIL_ON 返回 SUCCESS——记本 store miss/stall，不得记 hit（RXS-0316 能红字面）。② vendor blob 非 stable，不入 golden。③ pipelineCreationCacheControl 缺位 = fail-closed dev-env degrade，不降级判据。

`Assisted-by: cursor-grok-4.5-high-fast`（实现初稿 + smoke/schema；主 agent 复核 stall 语义修复 + 治理接线。影响范围：本节 + CI_GATES §4 M30 行 v1.6 + ledger v1.52/v1.53 + g8_budget v1.4 + pr-smoke.yml 步骤 100 + check_schemas.py 路由 + budget_eval.py 分支 + unsafe-audit U27 扩注；验证方式：上述验收命令全绿）

### 8.6 M85 shader_manifest_ddc `--phase g8.2` materialize（2026-08-06）

**触发**：G8.2/3 共担 P0 M85 `g8.p0.m85.shader_manifest_ddc` 的 G8.2 腿（host 轨收尾；硬依赖 M29/M30，软依赖 M32）。

**交付物**（spec-first：条款 `0905a8b6` 先行，实现+治理接线随后）：

- **spec**：`spec/rendering_platform.md` v1.4 RXS-0317（manifest v1 字段闭集/canonical digest）+ RXS-0318（merge/dedup/冲突/coverage/phase 纪律）。ledger v1.54（RXS 316/317→318/319）。
- **实现**：`src/rurixc/src/manifest.rs`（from_parts/merge/coverage/digest；9 单测）+ `rurixc --merge-manifests`/`--assemble-manifest` + `conformance/manifest/{fixtures,golden}`。
- **CI**：`ci/g8_shader_manifest_ddc_smoke.py --phase g8.2`（10 checks）+ evidence schema（双 phase 字段位；`phase_g8_3_pass` 恒 false）。
- **治理**：check_schemas 路由 + pr-smoke 步骤 101 + CI_GATES v1.7 回填 101 + g8_budget v1.5 `g8.counter.shader_manifest_phase_g82_legs` + budget_eval + ledger v1.55（CI_step 100→101/102）。

**判据**：merged key 集 == golden；重复恰好去重；冲突 fail-closed；coverage 无缺口 + 缺口 RED；输入乱序 digest 不变；双跑确定性；改 interface_hash/pso_key digest 必变；pipeline_key 字段源自 reflection；`phase_g8_2_pass=true` 且不代 `phase_g8_3_pass`。

**验收**（2026-08-06）：
```
cargo test -p rurixc --lib manifest::   # 9 passed
py -3 ci/g8_shader_manifest_ddc_smoke.py --gate g8.p0.m85.shader_manifest_ddc --phase g8.2  # PASS
py -3 ci/g8_shader_manifest_ddc_smoke.py --selftest
```
实测 merged digest=`8eb511bf7357f6f2de895edb052d8a48ce991b63454c5b7ca5daee0a3dc8d32a`。

`Assisted-by: cursor-grok-4.5-high-fast`（实现/fixtures/smoke/schema；主 agent 治理接线。影响范围：本节 + CI_GATES v1.7 + ledger v1.54/v1.55 + g8_budget v1.5 + pr-smoke 步骤 101）

### 8.7 M89 single_source_gfx_submit materialize（2026-08-06）

**触发**：G8.2 P0 M89 `g8.p0.m89.single_source_gfx_submit`（device 门；正式兑现 RD-037 三件套）。

**交付物**（spec-first：条款 `acaa31e3` 先行，实现+治理接线随后）：

- **spec**：`spec/rhi.md` v1.8 RXS-0319（VB/IB 声明算子与布局推导）+ RXS-0320（装配核验与越界拒）+ RXS-0321（gfx 派发臂与 readback provenance）。ledger v1.56（RXS 318/319→321/322）。
- **实现**：语言面 `vertex_data`/`index_data`/`draw`/`draw_indexed` + cabi `rxrt_rhi_vb_create`/`ib_create`/`gfx_draw`/`gfx_vs_layout` + `rhi_submit_vk` gfx 派发臂 + vk `run_rhi_graphics_offscreen_v2`（IB + `DrawIndexed`，`INDEX_TYPE_UINT32=1`）+ `conformance/gfx_submit/{accept,reject}` + `tests/gfx/m89_golden.rgba8`。
- **CI**：`ci/g8_single_source_gfx_smoke.py`（11 checks）+ evidence schema（`numeric_step=102`）。
- **治理**：check_schemas 路由 + pr-smoke 步骤 102（`RURIX_REQUIRE_REAL=1`）+ CI_GATES v1.8 回填 102 + g8_budget v1.6 `g8.counter.single_source_gfx_checks` + budget_eval + ledger v1.57（CI_step 101→102/103）+ RD-037 `open→closed`（deferred v1.75）+ U31 扩注。

**判据 11 项**（G8_ACCEPTANCE_MAP §2 M89 行逐字 + RXS-0319~0321）：EXE 像素自断言 / dump==checked-in golden / fixture+启动链零 `.rs` / 无 host 填像素替身 / artifacts v2 真消费 / cabi VB/IB 绑定 / reject 装配腿红 / seal 双向 / 仅编译不算 PASS / validation=0 / accept 语料绿。**不以 M50/M29/M30/M31/M32/M85 任一结果代替**。

**验收**（2026-08-06，本机 RTX 4070 Ti）：
```
$env:RURIX_REQUIRE_REAL=1
py -3 ci/g8_single_source_gfx_smoke.py --gate g8.p0.m89.single_source_gfx_submit
# PASS；evidence/g8_m89_single_source_gfx_submit_20260806T062455Z.json
# checks.* 11/11 true；device_section_state=pass
```

`Assisted-by: cursor-grok-4.5-high-fast`（实现/语料/smoke/schema；主 agent 治理接线与 RD-037 关闭。影响范围：本节 + CI_GATES v1.8 + ledger v1.57 + g8_budget v1.6 + pr-smoke 步骤 102 + deferred RD-037）
