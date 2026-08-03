# G7 CI_GATES — Production Frame Closure 机器门

> 契约：[G7_CONTRACT.md](G7_CONTRACT.md) · 计划：[G7_PLAN.md](G7_PLAN.md)
> 通用纪律：host/reference 段恒跑；device 段 gate real（`RURIX_REQUIRE_REAL=1`）；缺 provisioning 的 SKIP 只表示 dev-env degrade，不能满足 G7 close-out；mock、host substitution、isolated nonzero 均不充绿。

---

## 1. 既有守卫

全程恒跑：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -3 ci/check_number_ledger.py
py -3 ci/check_schemas.py
py -3 ci/check_structure.py
py -3 ci/check_guardrails.py <g7-base-or-pr-base>
py -3 ci/check_contribution.py
py -3 ci/trace_matrix.py --check
py -3 ci/budget_eval.py
```

既有步骤 41~92 判据 0-byte 只增；步骤 69 的 RD-034 blocked probe 与步骤 70 永久 gap 维持。

## 2. 新步骤拟分配（步骤 93 起）

| 步骤（拟） | 脚本（拟） | host/compile 段（恒跑） | device 段（gate real） | 对应门 |
|---|---|---|---|---|
| 93 | `ci/ray_query_codegen_smoke.py` | RED/accept 语料、SPIR-V 1.4/capability/extension/golden、`spirv-val`、W1/W2 最低版本零回归 | 最小 hit/miss/属性查询 kernel 真跑 | G-G7-4 |
| 94 | `ci/renderer_w3_smoke.py` | host BVH/reference 与三效果 oracle；AS/lifetime 审计 | 同一真实 TLAS 驱动 GI/RTAO/硬阴影 `.rx` kernel，对拍与 validation | G-G7-5/6 |
| 95 | `ci/renderer_raster_diff_smoke.py` | 固定场景、覆盖规则与 RD-038 字面矩阵完整性 | VisBuffer SW/HW 整数域 diff=0；VSM depth/TSR 等余项 device 见证 | G-G7-7 |
| 96 | `ci/renderer_device_frame_smoke.py` | graph/resource provenance、禁止 host substitution/isolated 拼装审计 | 连续真实设备帧、readback、capability snapshot、GPU timestamps | G-G7-8 |

步骤号随真实脚本 materialize 时回填 ledger；本脚手架不在 workflow 预放空步骤，也不预占多余号。

## 3. Close-out 专用取证（不占 PR smoke 步骤号）

`ci/renderer_device_frame_smoke.py --soak --frames 10000 --min-minutes 30`（最终 CLI 由实现 PR 冻结）必须产：

- `actual_frames >= 10000` 且 `elapsed_minutes >= 30`；
- validation/device-loss/TDR/resource-leak 计数均为 0；
- 固定相机视觉摘要与输入场景 digest；
- frame GPU/CPU submit p50/p95/p99、peak VRAM、pass timestamps；
- 每个 pass 的 input/output resource identity，证明连续消费；
- 环境画像、capability snapshot、`RURIX_REQUIRE_REAL=1` 与 run URL。

## 4. Evidence schema（与 smoke 同 PR 落）

拟定：

- `ray_query_codegen_evidence_schema.json`
- `renderer_w3_evidence_schema.json`
- `renderer_raster_diff_evidence_schema.json`
- `renderer_device_frame_evidence_schema.json`
- `renderer_soak_evidence_schema.json`

schema 与 `ci/check_schemas.py` 路由必须和对应 smoke 同 PR 落，避免先有 YAML/JSON 壳后无真实执行。

## 5. 预算门

- G7.0：`g7_budget.json` 三组可为空，且仅表示“尚未测量”，不是通过性能验收。
- G7.1：目标 GPU baseline 完成后，首个语义实现 PR 前追加至少一项 `measured_local` 性能 entry 和一项 correctness counter，并同时实现 `budget_eval.py` evaluator；禁止未知 id、禁止 estimated。
- G7.2~G7.6：预算只追加或按 14 §3 合法收紧，不回改已有 measured 事实。
- G7.7：strict 模式必须非空、全 PASS、零 skip/estimated；空数组直接视为 G-G7-9 失败，即使通用 evaluator 返回 PASS。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-01 | G7.0 初版；拟分配步骤 93~96，不 materialize workflow/script/schema。 |
| v1.2 | 2026-08-03 | **步骤 94 materialize：G7.4 W3c 三效果核 device 真跑**（ledger v1.42 消费步骤 94，`on_tree_max` 93→94 / `next_free` 94→95；步骤 95/96 **维持拟分配**，不预占）。`ci/renderer_w3_smoke.py` host 段恒跑七项 = host BVH/reference 三效果 oracle 单测（`rurix-render` 的 `rt::` + `gi::` 全量，host oracle **数值语义 0-byte** 回归网）+ AS/lifetime 审计（`rt::as_manager` 策略单源 + `rurix-render` `#![forbid(unsafe_code)]` + unsafe-audit **U30** 边界登记，RFC-0018 §C1/C4）+ 三 kernel `{gi_probe,rtao,hard_shadow}.rx` 真实 `.rx`→`.spv`（SPIR-V 1.4 + `spirv-val` `vulkan1.2`/`spv1.4` 双口径退出码判定）+ 反汇编 golden（per-file 必含 AS/RayQuery 类型与 initialize/proceed/intersection-type，三核**并集**覆盖 committed 五查询族）+ 单 TLAS 纪律静态审计（RXS-0297：每 kernel `AccelStruct` 形参恰好一个）+ W1/W2 五 kernel 对 `tests/vulkan/w1w2_spv_manifest.json` 逐字节零漂移 + 篡改 `.spv` RED 反证。device 段 **gate real**（`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`）= `uc06-renderer --w3-effects`：三 kernel 在**一次** `VkAsManager` 建面（3 BLAS × 3 实例 = 冻结场景 764 三角形）+ 一条 command buffer + **单次提交**中依次 dispatch，**同一个 TLAS 句柄**写入三个 descriptor set（`dispatch_tlas` 逐项等于 `tlas_identity` = 「三核共用真实 TLAS」机器判据）；对拍 host oracle —— hit/miss 与 instance/primitive/geometryIndex **零容差**，`committed_t` / barycentric 两分量 / GI 辐射度 / RTAO AO / 硬阴影可见性按 **measured 后冻结**容差成对机验（measured ≤ tol，阈值只来自真实 GPU 输出）；RED 三轴 = 篡改 device 侧场景顶点的**数据流反证** + 注入式 stale-tlas + wrong-barrier（validation VUID 拦截）。device 段同时构成 **G-G7-6 效果门**的机器见证。**W3a 章 B 实现面回填留痕**（owner 2026-08-03 裁决路 A）：`vulkan_codegen::place_ptr` 增单层向量分量投影一支，兑现 RXS-0298 已冻结的 `committed_barycentric() -> vec2<f32>`（此前分量在语言面不可消费 → 该条款为死条款）；定性 = **纯实现侧回填、spec 面 0-byte、零新 RXS**，先例引用 = `spec/softraster.md` §4 与 `spec/stdlib.md` §5「聚合值类型 device codegen（后续扩展，**非禁区**）……其接通**不改本文件既有条款语义**（纯实现侧回填）」+ RX6026 原拒绝消息「属后续**分片**」字面。步骤 95/96 维持拟分配。 |
| v1.1 | 2026-08-03 | 步骤 93 全段 materialize：G7.2 W3a 落 host/compile 段六项（ledger v1.41 消费步骤 93）；G7.3 W3b 落 device 段真跑——`bin/vk_ray_query` 消费 rurixc 产 `.spv` 经**单所有者 `VkAsManager`**（自 `rt_body`/U30 等序提取，步骤 66/67 恒绿证零漂移）真实单三角形 TLAS 在 compute queue 执行：W3 七能力链 fail-closed 门禁 + hit(committed_t=1.0±1e-6)/miss(-1.0 哨兵)数据流红绿 + RED 四轴（missing-capability 注入拒绝 / stale-tlas fail-closed / wrong-barrier validation VUID-02815 拦截 / device-lost `VK_ERROR_DEVICE_LOST` 传播单测）；workflow 步骤 93 置 `RURIX_REQUIRE_REAL=1`（拟分配注「待 G7.3 落地后置 1」兑现）。device 段同时构成 **G-G7-5 执行门**的机器见证（AS descriptor/import 通道 + KernelWave::W3 缺一确定性拒绝 + validation 零错误）；步骤 94~96 维持拟分配。 |
