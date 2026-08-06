# G8 CI_GATES — UE5 级前置能力完成期机器门

> 契约：[G8_CONTRACT.md](G8_CONTRACT.md) · 计划：[G8_PLAN.md](G8_PLAN.md) v1.2 · 能力矩阵：[G8_CAPABILITY_MATRIX.md](G8_CAPABILITY_MATRIX.md) v1.1。
> 当前状态（v1.2，2026-08-05 更新）：**G8.2 implementation 门已开**（`implementation_status: unblocked`，凭据 [G8_CONTRACT §8.1](G8_CONTRACT.md#81-g-g8-3-实现互锁实测与-implementation_status-解锁2026-08-05)）。上一行 v1.0/v1.1 的「G8.2+ blocked」是当时快照，不回写。**门已开 ≠ 门已绿**：本文 §4/§4.0/§5 的 21 个 P0/P1 key 与聚合门当前仍全部未 materialize，脚本、schema、workflow 步骤一件未落；任何「G8.2 开工」叙述都不得当作 PASS。

---

## 1. 互锁与编号纪律

### 1.1 实现互锁

稳定治理 validator 名为 `ci/check_g8_implementation_interlock.py`，属于 `check_*` 类未编号守卫。其实现后必须读取事实源并逐项输出：

1. `milestones/g7/G7_CONTRACT.md` 的有效 status 是否为 `closed`；
2. `registry/deferred.json` 的 RD-038 是否 `closed`；若否，是否在 **G7 closed 之后**完成 G8_PLAN §1.0 六行终态并向 RD-038 history 追加独立 override；
3. G8.1 的三 RFC、候选决策、验收映射、非空 measured budget 是否齐备；
4. workflow 与 ledger 的实际末号/`next_free` 是否一致。

当前（2026-08-05）G7 `closed`、RD-038 `closed`，validator 输出 `VERDICT = READY`（`--require-ready` exit 0），凭据逐字见 G8_CONTRACT §8.1。在此之前 G7 active / RD-038 open 时 `BLOCKED` 是唯一正确结论——该结论不因 READY 而追溯改写。禁止把 `--expect-blocked` 一类测试模式当成 G-G8-3 PASS；它只能证明 validator 能识别阻断。G8.2 起每个实现 PR 必须把 `--require-ready` 作为前置 required check。

### 1.2 数字步骤延迟分配

- G7 的 `reserved_in_flight[G7].CI_step` 从步骤 93 起，真实数量尚未 materialize；G8 不假定其止于 96，也不预占 97。
- G8 的稳定身份是本文件中的 `symbolic_gate_key` 与 `script`。所有未来编号栏统一写 **`post-G7 actual-next-free allocation`**。
- 只有 G-G8-3 全绿后，才可同时读取 `.github/workflows/pr-smoke.yml` 与 `registry/number_ledger.json`，按合入时实际 `next_free` 给即将 materialize 的脚本分配数字步骤，并在同一 PR 追加 ledger 校准。
- 不创建“预留” workflow step、空 YAML job、空脚本、永远 PASS 的 schema 壳或注释占位。脚本 + RED/GREEN 自检 + schema + workflow 真步骤 + ledger 校准同一实现 PR 落。

## 2. 既有守卫与 0-byte 边界

G8.1 可运行且不得改弱的既有守卫：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -3 ci/check_number_ledger.py
py -3 ci/check_schemas.py
py -3 ci/check_structure.py
py -3 ci/check_guardrails.py <g7-close-ref-or-pr-base>
py -3 ci/check_contribution.py
py -3 ci/trace_matrix.py --check
py -3 ci/budget_eval.py
```

步骤 41~92 与 G7 最终 materialize 的全部步骤判据 0-byte 只增；步骤 70 永久 gap、步骤 69 RD-034 blocked probe 均维持。G8.1 不改 `.github/workflows/pr-smoke.yml`，也不以文档表格冒充 workflow 接线。

## 3. G8.1 governance-only 机器门

| Symbolic gate key | 稳定脚本/检查 | 数字步骤 | 判据 |
|---|---|---|---|
| `g8.gov.structure` | 既有 `ci/check_structure.py` + `ci/check_schemas.py` | 不编号（既有 `check_*`） | CONTRACT/CI/budget/decision/map/RFC 结构一致；map 中预定 schema 名唯一，实际 schema 只与对应脚本同 PR 落，不预建空壳 |
| `g8.gov.number_isolation` | 既有 `ci/check_number_ledger.py` | 不编号（既有 `check_*`） | RFC-0019~0021 claim 与 G7 隔离；RXS/RD/U/RX/数字 CI 零推测 claim |
| `g8.gov.implementation_interlock` | `ci/check_g8_implementation_interlock.py` | 不编号（新增 `check_*` validator） | 当前应诚实报 `BLOCKED`；仅事实互锁全绿时才输出可开工 receipt |
| `g8.gov.acceptance_coverage` | `ci/check_g8_acceptance_map.py` | 不编号（新增 `check_*` validator） | 18 个 P0 key/script/schema/check 双向全覆盖；候选决策表和 RD 分项无缺行 |
| `g8.gov.measured_baseline` | 既有 `ci/budget_eval.py` + `ci/check_g8_budget_baseline.py` | 不编号（新增 `check_*` validator） | `g8_budget.json` 非空 measured_local、零 estimated，counter 与 evaluator 同步；当前不得声称实现性能通过 |

这些 validator 可以在 G8.1 落地，但不得带数字“步骤 NN”，也不得把 G8.2 目标脚本接进 workflow。

## 4. 18 个 P0 独立机器断言

下表的 key 与脚本名冻结。每一行均须独立 evidence subject 和独立结果；同一 workflow 进程可以顺序调用多个脚本，但任一行 `FAIL`、`SKIP` 或 `DEV_ENV_DEGRADE` 都必须保持可见，聚合结果不得 PASS。

| symbolic_gate_key | M## | 波次 | script | numeric_step |
|---|---:|---|---|---|
| `g8.p0.m50.rt_pipeline_incremental` | M50 | G8.2 | `ci/g8_rt_pipeline_incremental_smoke.py` | 103 |
| `g8.p0.m89.single_source_gfx_submit` | M89 | G8.2 | `ci/g8_single_source_gfx_smoke.py` | 102 |
| `g8.p0.m29.shader_permutation` | M29 | G8.2 | `ci/g8_shader_permutation_smoke.py` | 98 |
| `g8.p0.m30.pso_cache` | M30 | G8.2 | `ci/g8_pso_cache_smoke.py` | 100 |
| `g8.p0.m31.reflection_hash` | M31 | G8.2 | `ci/g8_reflection_hash_smoke.py` | 97 |
| `g8.p0.m32.capability_profile` | M32 | G8.2 | `ci/g8_capability_profile_smoke.py` | 99 |
| `g8.p0.m85.shader_manifest_ddc` | M85 | G8.2/3 | `ci/g8_shader_manifest_ddc_smoke.py` | 101 |
| `g8.p0.m79.asset_determinism` | M79 | G8.3 | `ci/g8_asset_determinism_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p0.m80.ddc_content_address` | M80 | G8.3 | `ci/g8_ddc_content_address_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p0.m81.gltf_import` | M81 | G8.3 | `ci/g8_gltf_import_smoke.py` | 106 |
| `g8.p0.m01.meshlet_page_builder` | M01 | G8.3 | `ci/g8_meshlet_page_builder_smoke.py` | 105 |
| `g8.p0.m04.page_format_abi` | M04 | G8.3 | `ci/g8_page_format_abi_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p0.m37.streaming_io` | M37 | G8.4 | `ci/g8_streaming_io_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p0.m19.vsm_page_cache` | M19 | G8.5a | `ci/g8_vsm_page_cache_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p0.m24.tsr_contract` | M24 | G8.5b | `ci/g8_tsr_contract_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p0.m66.physics_replay` | M66 | G8.6a | `ci/g8_physics_replay_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p0.m67.network_physics` | M67 | G8.6b | `ci/g8_network_physics_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p0.m68.fracture_pipeline` | M68 | G8.6c | `ci/g8_fracture_pipeline_smoke.py` | `post-G7 actual-next-free allocation` |

### 4.0 已 go 的 P1 独立断言（3 行，v1.1 补齐）

与 §4 P0 同纪律：独立 key、独立脚本、独立 evidence subject；`no-go` 项不入本表，改判 go 须先修 `G8_ACCEPTANCE_MAP.md` §1 覆盖集合与 §6 流程。

| symbolic_gate_key | M## | 波次 | script | numeric_step |
|---|---:|---|---|---|
| `g8.p1.m25.upscaler_input_abi` | M25 | G8.5b | `ci/g8_upscaler_input_abi_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p1.m72.cloth_product_chain` | M72 | G8.6d | `ci/g8_cloth_product_chain_smoke.py` | `post-G7 actual-next-free allocation` |
| `g8.p1.m83.texture_transcode` | M83 | G8.3 | `ci/g8_texture_transcode_smoke.py` | 107 |

> **单一命名空间（v1.1）**：本文件、`G8_CONTRACT.md` §4.2、`G8_ACCEPTANCE_MAP.md` §2/§3 与 RFC-0019~0021 必须引用同一份 key/脚本；`g8.p{0,1}.m##.<slug>` + `ci/g8_<slug>_smoke.py` 为唯一合法形态，由 `ci/check_g8_acceptance_map.py` 三向比对强制。

### 4.1 独立判据与 RED/GREEN

| Key 后缀 | 恒跑/RED 判据 | GREEN 判据 | 设备纪律 |
|---|---|---|---|
| `m50.rt_pipeline_incremental` | 现有最小单 hit-group 见证必须被判“不足”；非法 SBT record/stack/subset RED | 多 hit group + SBT 用户数据 + stack sizing + pipeline library 真跑；RFC 子集阶段 RED-GREEN | `RURIX_REQUIRE_REAL=1`；device 必需 |
| `m89.single_source_gfx_submit` | 发现 Rust 宿主出图/host substitution 即 RED | `.rx` 单源 gfx submit + readback 像素断言 | device 必需 |
| `m29.shader_permutation` | 超预算与非法 key RED | 域/key/静态裁剪/预算报告确定 | host/compile 硬门 |
| `m30.pso_cache` | cache tamper/miss 协议 RED | precache、binary/cache roundtrip、warm hit、stall counter | driver/device 路径必需 |
| `m31.reflection_hash` | ABI 改动而 hash 不变 RED | canonical schema 与稳定 hash，进入 DDC key | host/compile 硬门 |
| `m32.capability_profile` | 不支持组合必须编译期 RED | 支持 profile GREEN，fallback specialization 明示 | host/compile 硬门；禁静默运行时降级 |
| `m85.shader_manifest_ddc` | 缺项/重复冲突/tamper RED | canonical merge/dedup + manifest↔DDC 往返 | host 硬门 |
| `m79.asset_determinism` | 非 canonical 输入漂移 RED | 独立双构建 artifact hash 逐字节相等 | host 硬门 |
| `m80.ddc_content_address` | 依赖/工具/profile 改变仍命中或内容篡改即 RED | key 全覆盖且 hit/miss/tamper 行为正确 | host 硬门 |
| `m81.gltf_import` | 非法 schema、越界索引、未锁扩展 RED | 锁定 glTF 2.0 corpus 导入并过 validator | host 硬门 |
| `m01.meshlet_page_builder` | 未知/迁移错误版本 RED | builder 确定性、版本字段、DAG/page golden | host 硬门 |
| `m04.page_format_abi` | 未知版本/损坏页/ABI 不匹配 RED | 磁盘↔内存格式、压缩↔device 解码 golden 往返 | device 解码腿必需 |
| `m37.streaming_io` | 迟到/损坏页与缺 queue 能力必须显式分支 | 真实磁盘→解压→上传→GPU provenance；迟到页可见降级 | device 必需；MQ 或单队列事实可见 |
| `m19.vsm_page_cache` | stale/错误失效/local-light 缺页 RED | 跨帧 cache、失效、scroll、local light、caster 对拍 | device 必需 |
| `m24.tsr_contract` | history/velocity/dynamic-resolution 破坏序列 RED | 五类生产时域场景序列对拍 | device 必需 |
| `m66.physics_replay` | 注入 divergence 必须定位首个差异 | Jolt 5.3 capture/replay hash 一致 | CPU/host 硬门；5.6 A/B 后置 |
| `m67.network_physics` | correction/丢包/重复事件故障注入 RED | prediction→correction→rollback/resim→dedup→smooth 全链 | CPU/host 硬门 |
| `m68.fracture_pipeline` | 缺 cook/断键/cache/VFX 任一段 RED | fracture 全链事件与状态证据 | CPU/host + 渲染事件桥 |

## 5. G8.2~G8.8 波次聚合门

聚合门只汇总独立事实，不可替代 §4 任一 P0。脚本名同样稳定，数字步骤仍延迟分配。

| Symbolic gate key | script | numeric_step | 聚合条件 |
|---|---|---|---|
| `g8.wave.2.exit` | `ci/g8_wave2_exit_check.py` | 104 | 七个 G8.2 P0 全 PASS；RFC-0019 Approved；RD-037 与本波 RD-038 接入逐字通过 |
| `g8.wave.3.exit` | `ci/g8_wave3_exit_check.py` | `post-G7 actual-next-free allocation` | 五个 G8.3 P0 全 PASS；M01/M04 ABI 已冻结；资产/纹理/打包 go 项独立绿 |
| `g8.wave.4.exit` | `ci/g8_wave4_exit_check.py` | `post-G7 actual-next-free allocation` | M37 PASS；GeomPage 必过；VT go 时独立过、no-go 时 not-triggered；MQ 三断言或单队列 fallback 事实 |
| `g8.wave.5a.exit` | `ci/g8_wave5a_exit_check.py` | `post-G7 actual-next-free allocation` | M19 PASS；go 的几何/阴影项与 RD-038 raster/VSM 接入各自有 PASS evidence |
| `g8.wave.5b.exit` | `ci/g8_wave5b_exit_check.py` | `post-G7 actual-next-free allocation` | M24 PASS；go 的材质/GI/显示项与 RD-038 GI/TSR/真帧接入各自有 PASS evidence |
| `g8.wave.6a.exit` | `ci/g8_wave6a_exit_check.py` | `post-G7 actual-next-free allocation` | M66 PASS；Jolt 5.3 corpus 先完成；5.6 A/B 结果诚实判档 |
| `g8.wave.6b.exit` | `ci/g8_wave6b_exit_check.py` | `post-G7 actual-next-free allocation` | M67 PASS；网络全链、CharacterVirtual、PhysicsAsset/ragdoll/physical animation 闭环 |
| `g8.wave.6c.exit` | `ci/g8_wave6c_exit_check.py` | `post-G7 actual-next-free allocation` | M68 PASS；破坏全链闭环 |
| `g8.wave.6d.exit` | `ci/g8_wave6d_exit_check.py` | `post-G7 actual-next-free allocation` | 布料 schema/import/collision/LOD/timeline 与载具产品层独立闭环 |
| `g8.wave.7.decisions` | `ci/g8_p2_decisions_check.py` | `post-G7 actual-next-free allocation` | G8_PLAN §2.7 全部 P2 有 go/no-go/defer-to-G9+，零空行；非 go 不冒充 PASS |
| `g8.wave.8a.soak` | `ci/g8_stabilization_soak.py` | `post-G7 actual-next-free allocation` | 18 P0 + go P1 回归；≥30 分钟且 ≥10000 帧；strict budget 非空零 estimated/skip；零 validation/device-loss/TDR/leak |
| `g8.wave.8b.closeout` | `ci/g8_closeout_check.py` | `post-G7 actual-next-free allocation` | map/decision/RD/evidence 全等；最后一个新增或修复硬门 PASS 与 8b 不得同日，且 8a 完整先行；status flip 前全部硬门 PASS |

## 6. Evidence schema

每个 §4/§4.0 脚本在其 materialize PR 同步落 `milestones/g8/g8_m<##>_<slug>_evidence_schema.json`（slug 与 symbolic key 末段同字面，见 `G8_ACCEPTANCE_MAP.md` §2/§3 的目标路径列），至少包含：

```text
schema_version
subject
symbolic_gate_key
matrix_row
wave
numeric_step
source_ref
host_section_pass
device_section_state = pass|fail|not_applicable|dev_env_degrade
checks.<symbolic_assertion> = true|false
evidence_level
run_url
timestamp
environment
```

约束：

- `subject` 与 `symbolic_gate_key` 一一对应，18 个 P0 不得共用一个不可拆 subject；
- `numeric_step` 在脚本 materialize 后必须是当时 ledger 实际分配值，不能保留本文件的延迟分配文字；
- `run_url`、GPU 型号、驱动、帧数、耗时与预算数字只能来自真实运行，禁止预填；
- device 必需行只有 `device_section_state=pass` 可绿；`dev_env_degrade`/`not_applicable`/`skip` 均不绿；
- host-only 行的 device 可为 `not_applicable`，但必须由本表明确指定，不能由脚本自行降级；
- schema 路由、budget counter 与 evaluator 必须和脚本同 PR 落；未知 counter 强制 FAIL。

## 7. 无假绿规则

1. **现有见证不可代绿新门**：RXS-0248 最小 RT、isolated kernel 非零、host oracle、SW raster、TAA-only 都不能分别替代 M50、连续 device frame、HW/SW diff、VSM、TSR。
2. **条件项不污染全绿叙述**：`no-go`/`defer`/`not-triggered` 是诚实决策，不是 PASS；报告必须分开列出 implemented 与 retained-open。
3. **VT 与 GeomPage 不互代**：GeomPage 永远独立必过；VT 仅按决策判 go 或 not-triggered。
4. **多队列不抢跑**：RFC-0019 多队列章未 Approved 时只准单队列；单队列 evidence 不能声称跨队列 timeline 通过。
5. **物理升级不倒置**：M66 replay corpus 未绿前不得跑 Jolt 5.6 升级结论；升级失败时钉 5.3 是诚实判档，不是 5.6 PASS。
6. **soak 不省略**：条件实现刚绿不得当日 close；8a 必须先完成规定时长/帧数与 strict budget，再进入 8b。

## 8. 预算门

- G8.1：RTX 4070 Ti baseline 必须写入非空 `g8_budget.json`，`evidence_level=measured_local`；零 `estimated`。baseline 只证明测量已建立，不证明未来实现达标。
- G8.2~G8.7：counter 与 evaluator 同实现 PR 落；budget 只追加或按 14 §3 合法收紧，不改写历史 measured 事实。
- G8.8a：`py -3 ci/budget_eval.py --strict` 必须非空、全 PASS、零 skip/estimated；空数组或未知 id 直接视为失败。

## 9. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-02 | G8.1 初版：冻结治理/实现双门、18 个 P0 独立 key 与脚本、G8.2~8.8 聚合门；全部 numeric_step 延迟为 `post-G7 actual-next-free allocation`；零 workflow/script/schema 预放，当前实现门诚实 blocked。 |
| v1.12 | 2026-08-06 | **M01+M81+M83 materialize**：§4 M01→`105`、M81→`106`；§4.0 M83→`107`。`rurix-geom-pages`/`rurix-asset`/`rurix-basis-sys` + RXS-0328~0334 + smokes + schemas 同波落。host 门。零新 RX 码；M83 U44~U46。 |
| v1.10 | 2026-08-06 | **wave2.exit materialize**：§5 `g8.wave.2.exit` 行 `numeric_step` 由 `post-G7 actual-next-free allocation` 回填为 `104`（ledger next_free=104 实际分配）；`ci/g8_wave_exit_lib.py` 共享库首落 + `ci/g8_wave2_exit_check.py` 薄壳 + `milestones/g8/g8_wave2_exit_evidence_schema.json` + `pr-smoke.yml` 步骤 104（host 聚合，**不加** `RURIX_REQUIRE_REAL`）同 PR 落。只读汇总七 P0 + RFC-0019 Approved + RD-037 closed + 本波 RD-038 接入空集；RD-040 总体维持 open。零新 RXS/RX/U/budget counter。其余聚合门行 0-byte。 |
| v1.9 | 2026-08-06 | **M50 materialize**：§4 M50 行 `numeric_step` 由 `post-G7 actual-next-free allocation` 回填为 `103`（ledger next_free=103 实际分配）；`ci/g8_rt_pipeline_incremental_smoke.py` + `milestones/g8/g8_m50_rt_pipeline_incremental_evidence_schema.json` + rt_pipeline/rt_incremental/vk_m50_rt_body + `pr-smoke.yml` 步骤 103（`RURIX_REQUIRE_REAL=1`）同 PR 落。device 门；RD-040 总体维持 open、M50 分项 history 关闭留痕。spec-first：RXS-0322~0327 先行（commit 5d2ba225，ledger v1.58）。零新 RX 码；unsafe 归 U30 扩注（0 新 U）。其余 14 个 P0/P1 行 0-byte。 |
| v1.8 | 2026-08-06 | **M89 materialize**：§4 M89 行 `numeric_step` 由 `post-G7 actual-next-free allocation` 回填为 `102`（ledger next_free=102 实际分配）；`ci/g8_single_source_gfx_smoke.py` + `milestones/g8/g8_m89_single_source_gfx_submit_evidence_schema.json` + cabi VB/IB/draw + vk gfx 派发臂 + `pr-smoke.yml` 步骤 102（`RURIX_REQUIRE_REAL=1`）同 PR 落。device 门；RD-037 三件套同 commit 关闭。spec-first：RXS-0319~0321 先行（commit acaa31e3，ledger v1.56）。零新 RX 码；unsafe 归 U31 扩注（0 新 U）。其余 15 个 P0/P1 行 0-byte。 |
| v1.7 | 2026-08-06 | **M85 `--phase g8.2` materialize**：§4 M85 行 `numeric_step` 由 `post-G7 actual-next-free allocation` 回填为 `101`（ledger next_free=101 实际分配）；`ci/g8_shader_manifest_ddc_smoke.py --phase g8.2` + `milestones/g8/g8_m85_shader_manifest_ddc_evidence_schema.json` + `src/rurixc/src/manifest.rs` + fixtures/golden + `pr-smoke.yml` 步骤 101 同 PR 落。host 门，`phase_g8_3_pass` 诚实 false。spec-first：RXS-0317~0318 先行（commit 0905a8b6，ledger v1.54）。零新 RX 码。其余 16 个 P0/P1 行 0-byte（G8.3 DDC 腿仍待）。 |
| v1.6 | 2026-08-06 | **M30 materialize**：§4 M30 行 `numeric_step` 由 `post-G7 actual-next-free allocation` 回填为 `100`（ledger next_free=100 实际分配）；`ci/g8_pso_cache_smoke.py` + `milestones/g8/g8_m30_pso_cache_evidence_schema.json` + `src/rurix-rt` PSO cache 实现 + `bin/vk_pso_cache` + `pr-smoke.yml` 步骤 100 同 PR 落。driver/device 门，`RURIX_REQUIRE_REAL=1` 翻硬红。spec-first：RXS-0314~0316 先行（commit 988dcefe，ledger v1.52）。零新 RX 码；unsafe 归 U27/U31 扩注（0 新 U）。其余 17 个 P0/P1 行 0-byte。 |
| v1.5 | 2026-08-06 | **M32 materialize**：§4 M32 行 `numeric_step` 由 `post-G7 actual-next-free allocation` 回填为 `99`（ledger next_free=99 实际分配）；`ci/g8_capability_profile_smoke.py` + `milestones/g8/g8_m32_capability_profile_evidence_schema.json` + `conformance/capability/{accept,reject,profiles}` 语料 + `pr-smoke.yml` 步骤 99 同 PR 落。host/compile 纯 host 门，device 段 `not_applicable`。spec-first：RXS-0311~0313 + RXS-0304 修订先行（commits bec06980 + 138897c0 兼容判定 v1.3 精确化，ledger v1.50）。新错误码 RX3020~RX3023（typeck 段四枚：missing_required/forbidden_used/fallback_incompatible/unknown_id）；`capability.runtime_snapshot_mismatch` 为库层 typed Err 不占 RX 码。其余 18 个 P0/P1 行 0-byte。 |
| v1.4 | 2026-08-06 | **M29 materialize**：§4 M29 行 `numeric_step` 由 `post-G7 actual-next-free allocation` 回填为 `98`（ledger next_free=98 实际分配）；`ci/g8_shader_permutation_smoke.py` + `milestones/g8/g8_m29_shader_permutation_evidence_schema.json` + `conformance/permutation/{accept,reject,golden}` 语料 + `pr-smoke.yml` 步骤 98 同 PR 落。host/compile 纯 host 门，device 段 `not_applicable`。spec-first：RXS-0308~0310 + RXS-0304 修订已先行（commit c53a3c2c，ledger v1.48 含 M31 滞后校准 303/304→307/308）。新错误码 RX3019（typeck `shader.permutation_domain_invalid`）+ RX7023（工具段 `toolchain.permutation_budget_exceeded`）按实现 commit 实测顺位领取。其余 19 个 P0/P1 行 0-byte。 |
| v1.3 | 2026-08-05 | **M31 materialize**：§4 M31 行 `numeric_step` 由 `post-G7 actual-next-free allocation` 回填为 `97`（ledger next_free=97 实际分配）；`ci/g8_reflection_hash_smoke.py` + `milestones/g8/g8_m31_reflection_hash_evidence_schema.json` + `conformance/reflection/{accept,reject}` 语料 + `pr-smoke.yml` 步骤 97 同 PR 落。host/compile 纯 host 门，device 段 `not_applicable`。其余 20 个 P0/P1 行 0-byte。 |
| v1.2 | 2026-08-05 | **实现门状态镜像（判据 0-byte）**：G-G8-3 互锁实测 READY（G7 closed + RD-038 closed），头部与 §1.1 由「blocked / 正确结论是 BLOCKED」更新为「门已开、当前仍全部未 materialize」；显式登记「门已开 ≠ 门已绿」与「每个实现 PR 前置 `--require-ready`」。§4/§4.0/§5 的 21 个 key、脚本名、判据、`post-G7 actual-next-free allocation` 文字与 §6~§8 纪律全部 0-byte；本次零 workflow 步骤、零脚本、零 schema。 |
| v1.1 | 2026-08-02 | **命名空间与 P1 勘误**：补 §4.0 三行已 go P1 独立断言（M25/M72/M83）；§6 evidence schema 命名规则由 `<script-stem>_` 改为与 `G8_ACCEPTANCE_MAP.md` 一致的 `g8_m##_<slug>_`；显式登记 key/脚本单一命名空间并交由 `ci/check_g8_acceptance_map.py` 三向比对。§3 三个治理 validator 已 materialize（含 `--selftest` 负样本自检），实现互锁当前诚实输出 `BLOCKED`。既有 18 行 P0 key/脚本/判据 0-byte。 |
