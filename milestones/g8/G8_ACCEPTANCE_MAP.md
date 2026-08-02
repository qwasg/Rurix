# G8_ACCEPTANCE_MAP — P0 / 已 go P1 验收映射

> **性质**：G8.1 治理交付物；事实源为 [G8_PLAN.md](G8_PLAN.md) §2、§2.9 与 [G8_CAPABILITY_MATRIX.md](G8_CAPABILITY_MATRIX.md) §11.3。  
> **编号纪律**：本表只冻结 symbolic CI gate key，不 claim 数字 CI step。数字步骤必须等 post-G7 interlock 通过、G7 的在途步骤全部 materialize/校准后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配；**G7 已保留的 93+ 段不是 G8 号源，G8 不预抢其中任何号码**。  
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS。

---

## 1. 覆盖集合与独立绿灯纪律

- P0 精确集合（18 行）：`{M50,M89,M29,M30,M31,M32,M85,M79,M80,M81,M01,M04,M37,M19,M24,M66,M67,M68}`。
- G8.1 已确定 go 的 P1 精确集合（3 行）：`{M25,M72,M83}`。`M04` 已属 P0，不重复记入 P1。
- 每一行的 symbolic key 同时是该能力唯一的 `assertion_id`。实现后的 evidence 顶层必须至少含：`schema_version`、`subject`、`milestone`、`wave`、`assertion_id`、`status`、`commands`、`environment`、`base_commit`、`run_url`、`timestamp`；其中 `assertion_id` 必须等于本表 key，`status` 必须为 `pass|fail`，不得以 `skip|estimated|advisory` 充绿。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论；任一 P0 assertion 缺失或失败，只能使该 P0 为红，**不得用同脚本内另一 assertion 的绿色代替**。
- **单一 key/脚本命名空间（v1.1 勘误）**：symbolic key 一律小写点分 `g8.p{0,1}.m<##>.<slug>`，脚本一律 `ci/g8_<slug>_smoke.py`，evidence schema 一律 `milestones/g8/g8_m<##>_<slug>_evidence_schema.json`。本表、[G8_CONTRACT.md](G8_CONTRACT.md) §4.2、[CI_GATES.md](CI_GATES.md) §4 与 RFC-0019~0021 引用同一份 key/脚本，由 `ci/check_g8_acceptance_map.py` 三向比对强制一致；此前本表的大写 `G8.P0.M##.*` 与分组脚本名（`g8_rt_platform_smoke.py`/`g8_shader_platform_smoke.py`/`g8_asset_pipeline_smoke.py` 等）为已废弃写法，不得复用。
- device 判据统一要求真实路径、`RURIX_REQUIRE_REAL=1`、validation error 为 0；mock、stub、host substitution、仅检查非零输出、缺 provisioning 的 SKIP 均不满足能力门。

---

## 2. P0 硬门（精确 18 行）

| M 行 | Symbolic CI gate key / 脚本 | Evidence schema（目标路径） | 精确 PASS 判据（本行独立 assertion） | 最晚波次 |
|---|---|---|---|---|
| **M50** | `g8.p0.m50.rt_pipeline_incremental`<br>`py -3 ci/g8_rt_pipeline_incremental_smoke.py --gate g8.p0.m50.rt_pipeline_incremental` | `milestones/g8/g8_m50_rt_pipeline_incremental_evidence_schema.json` | 真实 device 同一场景一次运行同时证明：① 至少两个 hit group / 材质记录产生各自 golden hit-id；② SBT 用户数据由 host 写入并在 shader readback 逐字节相等；③ stack size 由 pipeline 的 required stack 查询结果显式设置且 evidence 记录 `configured >= required`；④ pipeline library 分库创建、链接后输出与单体 golden 相等；⑤ RFC-0019 冻结的 any-hit / intersection / callable 子集逐项有 accept GREEN 与非法/缺能力 RED；⑥ validation error=0。evidence 必须列 `incremental_features={multi_hit_group,sbt_user_data,stack_sizing,pipeline_library,...}` 全真；现有 RXS-0248 单 hit-group 最小三角见证即使为绿也**不能**满足本门。 | **G8.2** |
| **M89** | `g8.p0.m89.single_source_gfx_submit`<br>`py -3 ci/g8_single_source_gfx_smoke.py --gate g8.p0.m89.single_source_gfx_submit` | `milestones/g8/g8_m89_single_source_gfx_submit_evidence_schema.json` | 固定 `.rx` gfx 图经 rurixc 生成 artifacts-v2 VS/FS，走 C ABI 的 VB/IB 绑定与 `rxrt_rhi_submit` gfx 派发，在真实 device readback 得到 checked-in RGBA8 golden 且 validation error=0；workload fixture、生成物和专用启动链的 Rust host 源清单必须为空（`rust_host_source_count == 0`，不得有 workload-specific `.rs` 画图/填像素替身）。编译成功或仅出现非零像素不算 PASS。 | **G8.2** |
| **M29** | `g8.p0.m29.shader_permutation`<br>`py -3 ci/g8_shader_permutation_smoke.py --gate g8.p0.m29.shader_permutation` | `milestones/g8/g8_m29_shader_permutation_evidence_schema.json` | 对固定 domain golden：canonical key 两次生成逐字节相等；合法组合集合与 golden 集合全等；静态不可能组合全部被裁剪；预算 `limit == legal_count` 为 GREEN、`limit == legal_count - 1` 为 RED；报告中的 `enumerated/pruned/emitted` 满足 `enumerated == pruned + emitted`。不得以 M30/M31/M32/M85 任一结果代替。 | **G8.2** |
| **M30** | `g8.p0.m30.pso_cache`<br>`py -3 ci/g8_pso_cache_smoke.py --gate g8.p0.m30.pso_cache` | `milestones/g8/g8_m30_pso_cache_evidence_schema.json` | 固定场景 collector 的 PSO key 集合与 checked-in golden 全等；冷 precache 构建数等于该集合大小；全新进程 warm run 中 `runtime_compile_stalls == 0` 且每个所需 key 均命中已持久化 cache/binary；篡改 schema/version/driver identity 的 artifact 必须 fail-closed 并重建，不能误命中。设备支持 pipeline binary 时必须走 binary 分支；否则 evidence 明记 capability=false 并走 `VkPipelineCache` 冻结 fallback。 | **G8.2** |
| **M31** | `g8.p0.m31.reflection_hash`<br>`py -3 ci/g8_reflection_hash_smoke.py --gate g8.p0.m31.reflection_hash` | `milestones/g8/g8_m31_reflection_hash_evidence_schema.json` | 同一 shader 的两次 canonical reflection 序列化字节与 interface hash 完全相等；仅改变声明次序/无语义路径后仍相等；改变任一 ABI 字段（binding、resource kind、stage visibility 或 value type）后 hash 必须改变；产物通过冻结 schema 校验，hash 被记录为后续 DDC key 的组成项。 | **G8.2** |
| **M32** | `g8.p0.m32.capability_profile`<br>`py -3 ci/g8_capability_profile_smoke.py --gate g8.p0.m32.capability_profile` | `milestones/g8/g8_m32_capability_profile_evidence_schema.json` | 支持 profile 的 fixture 类型检查 0 诊断；同一 fixture 移除一项必需 capability 后以 RFC-0019 冻结的 symbolic diagnostic key 确定性拒录；声明 fallback 的 fixture 在低 profile 只生成允许的 specialization，禁止能力对应指令/扩展计数为 0。三腿（accept、reject、fallback）缺一即 FAIL。 | **G8.2** |
| **M85** | `g8.p0.m85.shader_manifest_ddc`<br>`py -3 ci/g8_shader_manifest_ddc_smoke.py --gate g8.p0.m85.shader_manifest_ddc --phase g8.2`<br>`py -3 ci/g8_shader_manifest_ddc_smoke.py --gate g8.p0.m85.shader_manifest_ddc --phase g8.3` | `milestones/g8/g8_m85_shader_manifest_ddc_evidence_schema.json` | G8.2 生成的 shader/PSO manifest canonical merge 后 key 集合与 fixture golden 全等、重复项恰好去重、coverage 无缺口；G8.3 将 manifest digest 纳入 DDC key 后 put/get 字节相等，任一 shader interface hash 或 PSO key 改变均产生新 DDC key，旧 artifact 不得误命中。schema 同时要求 `phase_g8_2_pass=true` 与 `phase_g8_3_pass=true`；任一阶段绿色不能替另一阶段充绿。 | **G8.2 + G8.3** |
| **M79** | `g8.p0.m79.asset_determinism`<br>`py -3 ci/g8_asset_determinism_smoke.py --gate g8.p0.m79.asset_determinism` | `milestones/g8/g8_m79_asset_determinism_evidence_schema.json` | 同一 SourceAsset/ImportRecipe/CookProfile/tool-version 集合在两个隔离、空缓存输出根各构建一次，canonical DAG、每个 DerivedArtifact 字节及顶层 digest 全等；对依赖、recipe、profile、tool version 各做一次单变量变更时对应 artifact key 必须改变。复用同一输出目录或暖缓存的“双构建”无效。 | **G8.3** |
| **M80** | `g8.p0.m80.ddc_content_address`<br>`py -3 ci/g8_ddc_content_address_smoke.py --gate g8.p0.m80.ddc_content_address` | `milestones/g8/g8_m80_ddc_content_address_evidence_schema.json` | DDC key 的已冻结 preimage 必须完整含源 digest、依赖 digest 集合、工具版本与 CookProfile；相同 preimage 得同 key且 put/get 字节相等；四类输入分别单变量变更均得不同 key；artifact 位翻转后 checksum 校验必须拒绝，不能以损坏对象命中。 | **G8.3** |
| **M81** | `g8.p0.m81.gltf_import`<br>`py -3 ci/g8_gltf_import_smoke.py --gate g8.p0.m81.gltf_import` | `milestones/g8/g8_m81_gltf_import_evidence_schema.json` | 锁定扩展集内的 glTF 2.0 fixtures 全部导入且 canonical scene/node/mesh/primitive/material/texture 数量与 digest 逐项等于 golden；越界扩展、非法 accessor 范围和缺失必需 buffer 三个 reject fixture 均 fail-closed；不得静默丢字段或只验证 JSON 可解析。 | **G8.3** |
| **M01** | `g8.p0.m01.meshlet_page_builder`<br>`py -3 ci/g8_meshlet_page_builder_smoke.py --gate g8.p0.m01.meshlet_page_builder` | `milestones/g8/g8_m01_meshlet_page_builder_evidence_schema.json` | 固定 mesh 两次 builder 输出字节相等；页 header 的 magic/version/schema digest 精确等于冻结 golden；解码后 cluster/meshlet DAG 的节点、边、bounds、LOD parent 关系与 CPU reference 全等；未知 version fixture 必须在消费前拒绝。M04 codec 通过不能替本 builder/version assertion。 | **G8.3** |
| **M04** | `g8.p0.m04.page_format_abi`<br>`py -3 ci/g8_page_format_abi_smoke.py --gate g8.p0.m04.page_format_abi` | `milestones/g8/g8_m04_page_format_abi_evidence_schema.json` | 磁盘页与内存页具有不同且冻结的 ABI id/version；checked-in fixtures 经 encode→decode 后 canonical quantized records 与 golden 逐字节相等，压缩流两次生成逐字节相等；截断、checksum 损坏、未知 codec/version 四类输入均 fail-closed；device 解码 ABI 对同一页的 readback digest 等于 CPU 解码 digest。该门在 G8.3 冻结，G8.4 只消费、不得重定格式。 | **G8.3** |
| **M37** | `g8.p0.m37.streaming_io`<br>`py -3 ci/g8_streaming_io_smoke.py --gate g8.p0.m37.streaming_io` | `milestones/g8/g8_m37_streaming_io_evidence_schema.json` | 从真实临时文件发起 async read（不得预载内存替代），evidence 中 read→decompress→upload→GPU consume 的完成值严格单调且最终 device digest 等于页 golden；强制迟到页时至少一帧命中确定性 fallback，页到达后转为正确内容；validation error=0。若 `queue_mode=multi`，还须 RFC-0019 多队列章已 Approved 且 ownership/barrier 断言全真；否则必须登记 `queue_mode=single` 且不得声称跨队列过门。 | **G8.4** |
| **M19** | `g8.p0.m19.vsm_page_cache`<br>`py -3 ci/g8_vsm_page_cache_smoke.py --gate g8.p0.m19.vsm_page_cache` | `milestones/g8/g8_m19_vsm_page_cache_evidence_schema.json` | 固定多帧、多视图场景的物理页分配/命中/淘汰事件序列与 golden 全等，并分别命中：跨帧 cache hit、每种冻结失效原因、clipmap scroll、local-light page、非虚拟几何 caster、multi-view batch；页表/readback depth digest 与 device golden 相等且 validation error=0。只有 page-mark 或单帧 depth 不满足本门。 | **G8.5a** |
| **M24** | `g8.p0.m24.tsr_contract`<br>`py -3 ci/g8_tsr_contract_smoke.py --gate g8.p0.m24.tsr_contract` | `milestones/g8/g8_m24_tsr_contract_evidence_schema.json` | 真实 device 序列回归的 case 集合必须**恰好覆盖且全部 PASS**：`history_resurrection`、`pixel_animation_velocity`、`thin_geometry`、`dynamic_resolution`、`transparent_velocity`；每例输出 digest/误差均满足 RFC-0019 冻结 golden 与 tolerance，错误 history identity 必须被拒绝，validation error=0。既有 TAA 或单帧 TSR 输出不能充绿。 | **G8.5b** |
| **M66** | `g8.p0.m66.physics_replay`<br>`py -3 ci/g8_physics_replay_smoke.py --gate g8.p0.m66.physics_replay` | `milestones/g8/g8_m66_physics_replay_evidence_schema.json` | 先以 Jolt 5.3 建成冻结 corpus；原始固定步世界与 capture replay 在每一 tick 的 canonical state hash 全等，body create/destroy journal 全消费且无剩余；在 fixture 指定 tick 注入单 bit 状态变更后，定位器报告的 `first_divergence_tick` 必须精确等于注入 tick。只做 N=100 决定性重跑、没有 capture/replay，不满足本门。 | **G8.6a** |
| **M67** | `g8.p0.m67.network_physics`<br>`py -3 ci/g8_network_physics_smoke.py --gate g8.p0.m67.network_physics` | `milestones/g8/g8_m67_network_physics_evidence_schema.json` | 固定丢包/延迟 trace 中，客户端先产生预测偏差，再在 golden physics-frame-id 收到权威修正；rollback 起点、重演 input 序列与 expected trace 全等，resimulation 后最终 canonical state hash 等于服务端；同一接触事件跨 rollback 多次产生时对外只提交一次；平滑输出逐帧满足 RFC-0021 冻结 bound。五项均须独立字段为真。 | **G8.6b** |
| **M68** | `g8.p0.m68.fracture_pipeline`<br>`py -3 ci/g8_fracture_pipeline_smoke.py --gate g8.p0.m68.fracture_pipeline` | `milestones/g8/g8_m68_fracture_pipeline_evidence_schema.json` | checked-in 预破碎资产经 fracture cook 后，chunk/connection-graph/interior-face/anchor 数量与 digest 全等 golden；阈值下不断键，阈值上在指定 tick 断开指定 edge 并激活指定层级 cluster；cache roundtrip 后事件序列与状态 hash 不变，VFX bridge 对每个 fracture event 恰好发一次。任一链段缺失即 FAIL。 | **G8.6c** |

---

## 3. 已 go P1 硬门（精确 3 行）

这些行由 G8.1 候选决策表确定为 go 后进入对应波次硬门；未过门不得在 G8.8a 的“已 go P1 回归”中记绿。

| M 行 | Symbolic CI gate key / 脚本 | Evidence schema（目标路径） | 精确 PASS 判据（本行独立 assertion） | 最晚波次 |
|---|---|---|---|---|
| **M25** | `g8.p1.m25.upscaler_input_abi`<br>`py -3 ci/g8_upscaler_input_abi_smoke.py --gate g8.p1.m25.upscaler_input_abi` | `milestones/g8/g8_m25_upscaler_input_abi_evidence_schema.json` | 冻结 ABI 对 color/depth/motion/exposure/jitter/render extent/output extent/reset/reactive/transparent 输入逐项给出 layout/hash；真实 device 上至少一个审计过的非 no-op backend 消费全部必需 resource identity 并输出正确 extent、有限值及冻结序列 golden；缺任一必需输入或 ABI hash 不匹配必须 fail-closed，backend 切换不得改变调用侧 ABI。stub/no-op/reference-only 不能满足本门。 | **G8.5b** |
| **M72** | `g8.p1.m72.cloth_product_chain`<br>`py -3 ci/g8_cloth_product_chain_smoke.py --gate g8.p1.m72.cloth_product_chain` | `milestones/g8/g8_m72_cloth_product_chain_evidence_schema.json` | 固定 garment fixture 的开放 panel/seam/fabric schema 经过 DCC import→canonical roundtrip 后字节稳定；模拟中缝约束不断裂、碰撞穿透不超过 RFC-0021 冻结 bound；指定帧 LOD 切换后拓扑/状态映射等于 golden；cloth solver timeline 与 rigid-body timeline 为不同 identity 且依赖次序满足契约。schema、导入、碰撞、LOD、独立时间线五项缺一即 FAIL。 | **G8.6d** |
| **M83** | `g8.p1.m83.texture_transcode`<br>`py -3 ci/g8_texture_transcode_smoke.py --gate g8.p1.m83.texture_transcode` | `milestones/g8/g8_m83_texture_transcode_evidence_schema.json` | checked-in source 经真实 codec（非占位）生成 KTX2/Basis 与目标 profile 的 BCn/ASTC artifact；两次 cook 字节相等，容器/层/mip 数与 golden 全等；解码后颜色误差、normal length 与 alpha-coverage 分别落在冻结 tolerance，profile 选择得到预期目标格式；codec/vendor license 与 SBOM 条目存在。任一格式腿 SKIP、伪转码或仅改扩展名均 FAIL。 | **G8.3** |

---

## 4. G8.1 治理覆盖与空行门

G8.1 必须提供不占 numeric CI step 的 guardrail（脚本名与 [CI_GATES.md](CI_GATES.md) §3 同一份，属既有 `check_*` 未编号守卫）：

```text
g8.gov.acceptance_coverage
  py -3 ci/check_g8_acceptance_map.py

g8.gov.implementation_interlock
  py -3 ci/check_g8_implementation_interlock.py

g8.gov.measured_baseline
  py -3 ci/check_g8_budget_baseline.py
```

`ci/check_g8_acceptance_map.py` 的 PASS 判据（coverage + no-empty 两组断言分别独立报告）：

1. P0 行集合与 §1 的 18 项**集合全等**，无遗漏、无额外 P0、无重复；P1-go 行集合与 `{M25,M72,M83}` 集合全等，且 M04 只出现一次并标 P0。
2. 21 个 symbolic key 全局唯一，均匹配 `g8\.p[01]\.m\d{2}\.[a-z0-9_]+`；每行只有一个 canonical `assertion_id`，没有两个 M 行共享 key。
3. 每一行均有脚本命令、evidence schema、可机器求值的 PASS 判据、最晚波次；共享脚本必须使用不同的 `--gate` 参数。
4. **三向一致**：本表、`G8_CONTRACT.md` §4.2 与 `CI_GATES.md` §4 对同一 M 行给出的 key 与脚本必须逐字相等；任一处漂移即 FAIL（v1.1 勘误引入的机器锁）。

no-empty 组的 PASS 判据：

- 逐单元格拒绝空串、空白、`TBD`、`TODO`、`待定`、`待补`、`—`；表中 21 行的五个必填列均非空。
- 所有 schema 路径必须唯一落到对应 M 行，且文件名含同一 `m##`；所有波次属于 `G8.2|G8.3|G8.4|G8.5a|G8.5b|G8.6a|G8.6b|G8.6c|G8.6d` 的非空集合（M85 允许 `G8.2 + G8.3`）。
- G8.1 只核映射完整性，不把尚未 materialize 的脚本/schema 误判为实现绿色；对应能力实现 PR 合入前或同 PR，`ci/check_schemas.py` 必须能校验该 schema 与实际 evidence。

三个 validator 均把每组断言逐条打印，不以一个总 `all_pass=true` 掩盖具体缺行；`--selftest` 用受控负样本证明它们能红。治理 evidence schema 与实现期 evidence 同 PR 落，不预建空壳。

---

## 5. G8.2 硬互锁

`G8.GOV.G8_2.ENTRY_INTERLOCK` 是 G8.2 的前置 required check；它属于 `check_*` 治理守卫，不占 numeric CI step。以下条件必须**同时**为真：

1. `G7_CONTRACT.md` 的 status 已为 `closed`，且 post-G7 ledger 校准已完成；RD-038 满足 G8_PLAN §1.0 的 `closed` 路径，或“遗留接入表填满 + owner 书面 override history”路径，禁止以 `in-flight` 占位过门。
2. G8 立项指令有仓内不可变引用，`G8_CONTRACT.md` 已激活；RFC-0019/0020/0021 均达到契约要求的 Agent Approved 状态（§9.1 独立 provenance 对抗性评审完成且 findings 全部 disposition），RTX 4070 Ti measured baseline 与非空 `g8_budget.json` 已通过治理门。
3. `G8_CANDIDATE_DECISIONS.md` 的 RD-037~044 分项映射无空行；M50 的 go/strategic_override 依据已真正登记，M25/M72/M83 均为 go，且决策表、deferred history 与本表逐字一致。
4. §4 的 `G8.GOV.ACCEPTANCE.COVERAGE` 与 `G8.GOV.ACCEPTANCE.NO_EMPTY` 均独立 PASS。
5. G8 的 numeric CI step claim 发生在上述 post-G7 时点，并以 claim 当下 ledger 实测 `CI_step.next_free` 为起点；每个 symbolic key 一对一分配，未复用、未覆盖、未预抢 G7 93+。

任一条件为假时，互锁必须返回非零；此时禁止合入 G8.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动。不能用 owner override 绕过“G7 仍 active”，也不能用本表存在本身当作 G8.2 开工许可。

---

## 6. Close-out 审计

- G8.8a 必须重跑全部 18 个 P0 与 3 个已 go P1 的**各自 assertion**；evidence 的 `base_commit` 必须落在同一候选 close-out 基线，零 skip/estimated。
- G8.8b 只有在 18 个 P0 key 全 PASS、3 个已 go P1 key 全 PASS、决策表最终状态无漂移时才可 status flip。
- 后续若 owner 将新的 P1 判为 go，须先按治理流程修订本表及覆盖集合，再开对应实现；不得把它静默并入现有 key。P0 集合变更属于契约变更，不得以勘误处理。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-02 | G8.1 初版：冻结 18 个 P0 与已 go 的 M25/M72/M83 三个 P1 的 symbolic gate、目标脚本/schema、独立判据与波次；加入 post-G7 numeric interlock、覆盖/空行治理门及 G8.2 硬互锁。 |
| v1.1 | 2026-08-02 | **命名空间勘误（判据字面 0-byte）**：本表此前的大写 key `G8.P#.M##.*` 与分组脚本名同 `G8_CONTRACT.md` §4.2 / `CI_GATES.md` §4 的小写 key + 单脚本口径冲突，且 RFC-0019 §6.1 另有第三套；统一为 `g8.p{0,1}.m##.<slug>` + `ci/g8_<slug>_smoke.py` + `g8_m##_<slug>_evidence_schema.json`，新增 §1 单一命名空间条与 §4 三向一致断言，由 `ci/check_g8_acceptance_map.py` 强制。占位代号 RFC-α/γ 全部替换为实际 RFC-0019/0021。21 行的判据文字与波次逐字未改。 |
