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
| 95 实况 | 同上（v1.4 翻全绿） | 七项恒跑（HW capability 正向机验） | 余项两轴 + **HW 光栅 diff=0** 已兑现；`hw_raster_diff.status=verified-diff-zero` | G-G7-7 **全绿** |
| 96 | `ci/renderer_device_frame_smoke.py` | graph/resource provenance、禁止 host substitution/isolated 拼装审计 | 连续真实设备帧、readback、capability snapshot、GPU timestamps | G-G7-8 **已上线** |

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
| v1.6 | 2026-08-05 | **G7.7 close-out**:门终审表 G-G7-1~9 全过落契约 §8.1;RD-038 逐字终审 **closed**(deferred.json **v1.74**);G7_CONTRACT front-matter status active→**closed**(洁净独行,禁行内注释);number_ledger **v1.46** 纯留痕(各 namespace 字段 0-byte);`check_guardrails.resolve_base` ea1-closed→**g7-closed**;G8 互锁事实门预期 READY(READY ≠ G8.2 已开工);soak 采自 `ff44030c`/`evidence/renderer_soak_20260805T135929.json`(10000 帧/268.173643 min,health0)+ 步骤 96 锚 `evidence/renderer_device_frame_smoke_20260805T140247.json`+ HW diff=0 锚 `evidence/renderer_raster_diff_smoke_20260804T170945.json`;全量回归冻结真实输出骨架落 §8.1(**待 C2 回填**)。§1/§2/§3/§5 正文 **0-byte**。 |
| v1.5.2 | 2026-08-05 | **PR-4 soak 阈值回填(契约 status 0-byte)**:完整 soak evidence 实名 = `evidence/renderer_soak_20260805T135929.json`(`ok=true`,10000 帧 / 268.173643 min,health 全 0;锚点 `evidence/soak_anchors/1785893478/`);`g7_budget.json` 升 **v1.2.1**——`g7.bench.uc06_device_frame_soak_1080p` estimated→`measured_local`,thresholds={frame_gpu_p95_ms:2209.59744,cpu_submit_p95_ms:0.13455,peak_vram_mb:548.027343}(=实测×1.5),measured={1473.06496,0.0897,365.351562}。失败短跑 soak 四份只增不删。步骤 96 / G-G7-8 / RD-038 / `G7_CONTRACT.md` status **不改**。 |
| v1.5.1 | 2026-08-04 | **PR-4 预算结构注(契约 status 0-byte)**:`g7_budget.json` 升 **v1.2**——追加 `g7.bench.uc06_device_frame_soak_1080p`(多阈值结构;`eval_device_frame_soak` 已挂接)与 counters `g7.counter.uc06_device_frame_chain` / `g7.counter.uc06_soak_passed`。soak bench 阈值**待**完整 `evidence/renderer_soak_*.json`(ok:true)实测×1.5 回填(本波 estimated 占位,禁止短跑伪造);§3 soak evidence 实名占位 = `evidence/renderer_soak_<ts>.json` + 锚点 PPM `evidence/soak_anchors/<run>/`。步骤 96 / G-G7-8 / RD-038 status **不改**。 |
| v1.5 | 2026-08-04 | **步骤 96 materialize:G7.6 One True Device Frame**(ledger **v1.45** 消费步骤 96,`on_tree_max` 95→96 / `next_free` 96→97;v1.44 RXS-0301~0303 0-byte 保留)。`ci/renderer_device_frame_smoke.py` host 段恒跑六项 = 两 evidence schema Draft7 自检 + `G7_SCENE_FREEZE` 锚(含 **960×540→1920×1080**)+ RD-038 行 1/2/4/8 字面与 §6.4 帧链并入留痕 + host oracle 过滤(`geometry::`+`shadow::`+`temporal::`+`rt::`+`gi::`,数值语义 0-byte)+ 既有 kernel 对 `w1w2_spv_manifest.json` 零漂移 + 6 glue kernel 排放(SPIR-V 1.0 + `spirv-val vulkan1.0` + 同源 ×2 确定性)+ `device_frame.rs` 静态 provenance 审计(禁 `execute_frame(` 单发入口)。device 段 **gate real**(`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`)= `uc06-renderer --features device-frame --device-frame --frames 8 --json`:15-pass 阶段转移对拍 + 非退化 + provenance 边表 + telemetry;RED 四轴 = `--frame-red-visbuffer` / `--frame-red-history` / `--frame-red-jitter` / `--frame-red-provenance`。§3 soak CLI 冻结 + 两 schema(`renderer_device_frame_evidence_schema.json` / `renderer_soak_evidence_schema.json`)落 §4 拟定名;soak **不进** workflow。G-G7-8 **全绿**。RD-038 **维持 open**(status 归 G7.7)。 |
| v1.4 | 2026-08-04 | **步骤 95 翻全绿:G7.5b HW 光栅 diff=0**（ledger 字段 0-byte——步骤 95 已由 v1.43 消费,本波只翻判据终态）。`hw_raster_diff.status` = `verified-diff-zero`（diff_pixels=0 + hw_side 在位,schema if/then 由 v1.3 预置;加性字段 `conservative_props`/`spirv_caps`/`red_axes`/`capability_probes`）。RFC-0018 修订行 v1.1 §E 双裁定（语言面加性扩展 RXS-0301~0303 + 覆盖规则唯一权威 = SW 精确 f32 + top-left;HW = 保守光栅 OVERESTIMATE 超集 + FS 复刻）落点;v1.3 的 blocked-honest 六项 `missing_toolchain_caps` 自本波起翻空集如实记录。device 段加性:`uc06-renderer --g75-hw-raster`（真实 graphics pipeline,`pipeline=vk-graphics-conservative-raster`）+ 两 RED 轴 `--g75-hw-red-tamper-varying` / `--g75-hw-red-tamper-ids`（篡改 HW 顶点流 → diff>0）。W1/W2 五 kernel 零漂移维持。G-G7-7 **全绿**。RD-038 维持 open（帧链归 G7.6）。 |
| v1.3 | 2026-08-03 | **步骤 95 materialize：G7.5 光栅 diff 与 RD-038 余项**（ledger v1.43 消费步骤 95，`on_tree_max` 94→95 / `next_free` 95→96；步骤 96 **维持拟分配**，不预占）。`ci/renderer_raster_diff_smoke.py` host 段恒跑七项 = RD-038 **八行**字面矩阵 + `G7_SCENE_FREEZE` 场景/相机冻结锚在位（防矩阵/场景漂移后判据自动放水）+ host oracle 单测（`shadow::` + `temporal::` + `geometry::visbuffer`，**数值语义 0-byte** 回归网）+ **VisBuffer 位格式冻结面**（`depth30 \| cluster27 \| tri7` 与 clear 值在 host 冻结契约与 W2 SW kernel 源内同一套常量 —— SW/HW 同 ABI 的前提，G5 冻结面不得为迁就 HW 漂移）+ 余项三核 `{vsm_depth_raster,vsm_sample,tsr_resample}.rx` 真实 `.rx`→`.spv`（SPIR-V 维持 **1.0** 不误升 1.4 + `spirv-val --target-env vulkan1.0` + 同源 ×2 字节全等 + 零 ray query 误声明）+ **HW 光栅 blocked-honest 机验**（目标形态语料 `conformance/vulkan/reject/vk_hw_raster_visbuffer_fs.rx` 必红 `RX6026` 且零 `.spv`；**五条逐轴隔离探针 + 一条绿对照臂**取真实诊断，机器产出 `missing_toolchain_caps` 六项）+ W1/W2 五 kernel 零漂移（**不重 bless**）+ 篡改 `.spv` RED 反证。device 段 **gate real**（`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`）= `uc06-renderer --g75-residuals`：VSM 页内深度光栅（**gather 形态**，1 线程/(页,纹素)，线程内按 host 同序做 min 归约 ⇒ 单一写者、零原子）+ VSM 阴影采样（0/1 二值，**零容差**）+ TSR 空间超分核（jitter 对齐 4×4 Catmull-Rom + 抗振铃钳制），measured 与**冻结**容差成对机验并带非退化统计（覆盖纹素 / 遮蔽比 / 钳制通道数）；同段复跑 **SW/HW diff 的 SW 基准侧**（`device_w2_visbuffer_u64_bitexact_host`，9216 词 u64 逐位相等）；RED 两轴 = `--g75-red-vsm`（篡改 device 侧灯空间三角形 → 深度对拍必红的**数据流反证**）+ `--g75-red-tsr`（篡改 device jitter → 相位错位必红）。**G-G7-7 轴一（SW/HW 整数域 diff=0）本波未兑现**：阻断 = `spec/dxil_backend.md` **RXS-0171 L4** 冻结的图形=B body「最小 rvalue 白名单」（仅 `Use` / f32-i32-u32 `Const` / 加减乘除 `BinaryOp` / 输出 I/O 聚合机械分解；控制流、调用、cast、资源访问、非输出聚合一律 strict-only 拒），Vulkan 原生图形路复用同一编码器 `dxil_spirv::emit_spirv_body_vulkan`。**未放宽整数域容差、未以容差型替代物冒充 diff=0**；schema `renderer_raster_diff_evidence_schema.json` 对 `hw_raster_diff` 施 if/then（`verified-diff-zero` 须 `diff_pixels == 0` 且 `hw_side` 在位；`blocked-*` 须 `missing_toolchain_caps` 非空 + 逐轴探针 + spec 锚 + 升级路径），by-construction 封死「以容差替代 diff=0」与「blocked 只写一句话」。RD-038 **维持 open**。**恒跑清单加固（本波起执行）**：`ci/vulkan_codegen_smoke.py` 入每波必跑清单 —— 该恒跑步自 `2fa12759`（2026-07-30）起持续红、跨 G7.2/G7.3/G7.4 三波未被发现（三波均未触碰相关面，属**既有红**），说明前波「全绿自检」漏跑了恒跑步；G7.5.0 已按路 A 清红（详见 ledger v1.43 §③）。 |
| v1.2 | 2026-08-03 | **步骤 94 materialize：G7.4 W3c 三效果核 device 真跑**（ledger v1.42 消费步骤 94，`on_tree_max` 93→94 / `next_free` 94→95；步骤 95/96 **维持拟分配**，不预占）。`ci/renderer_w3_smoke.py` host 段恒跑七项 = host BVH/reference 三效果 oracle 单测（`rurix-render` 的 `rt::` + `gi::` 全量，host oracle **数值语义 0-byte** 回归网）+ AS/lifetime 审计（`rt::as_manager` 策略单源 + `rurix-render` `#![forbid(unsafe_code)]` + unsafe-audit **U30** 边界登记，RFC-0018 §C1/C4）+ 三 kernel `{gi_probe,rtao,hard_shadow}.rx` 真实 `.rx`→`.spv`（SPIR-V 1.4 + `spirv-val` `vulkan1.2`/`spv1.4` 双口径退出码判定）+ 反汇编 golden（per-file 必含 AS/RayQuery 类型与 initialize/proceed/intersection-type，三核**并集**覆盖 committed 五查询族）+ 单 TLAS 纪律静态审计（RXS-0297：每 kernel `AccelStruct` 形参恰好一个）+ W1/W2 五 kernel 对 `tests/vulkan/w1w2_spv_manifest.json` 逐字节零漂移 + 篡改 `.spv` RED 反证。device 段 **gate real**（`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`）= `uc06-renderer --w3-effects`：三 kernel 在**一次** `VkAsManager` 建面（3 BLAS × 3 实例 = 冻结场景 764 三角形）+ 一条 command buffer + **单次提交**中依次 dispatch，**同一个 TLAS 句柄**写入三个 descriptor set（`dispatch_tlas` 逐项等于 `tlas_identity` = 「三核共用真实 TLAS」机器判据）；对拍 host oracle —— hit/miss 与 instance/primitive/geometryIndex **零容差**，`committed_t` / barycentric 两分量 / GI 辐射度 / RTAO AO / 硬阴影可见性按 **measured 后冻结**容差成对机验（measured ≤ tol，阈值只来自真实 GPU 输出）；RED 三轴 = 篡改 device 侧场景顶点的**数据流反证** + 注入式 stale-tlas + wrong-barrier（validation VUID 拦截）。device 段同时构成 **G-G7-6 效果门**的机器见证。**W3a 章 B 实现面回填留痕**（owner 2026-08-03 裁决路 A）：`vulkan_codegen::place_ptr` 增单层向量分量投影一支，兑现 RXS-0298 已冻结的 `committed_barycentric() -> vec2<f32>`（此前分量在语言面不可消费 → 该条款为死条款）；定性 = **纯实现侧回填、spec 面 0-byte、零新 RXS**，先例引用 = `spec/softraster.md` §4 与 `spec/stdlib.md` §5「聚合值类型 device codegen（后续扩展，**非禁区**）……其接通**不改本文件既有条款语义**（纯实现侧回填）」+ RX6026 原拒绝消息「属后续**分片**」字面。步骤 95/96 维持拟分配。 |
| v1.1 | 2026-08-03 | 步骤 93 全段 materialize：G7.2 W3a 落 host/compile 段六项（ledger v1.41 消费步骤 93）；G7.3 W3b 落 device 段真跑——`bin/vk_ray_query` 消费 rurixc 产 `.spv` 经**单所有者 `VkAsManager`**（自 `rt_body`/U30 等序提取，步骤 66/67 恒绿证零漂移）真实单三角形 TLAS 在 compute queue 执行：W3 七能力链 fail-closed 门禁 + hit(committed_t=1.0±1e-6)/miss(-1.0 哨兵)数据流红绿 + RED 四轴（missing-capability 注入拒绝 / stale-tlas fail-closed / wrong-barrier validation VUID-02815 拦截 / device-lost `VK_ERROR_DEVICE_LOST` 传播单测）；workflow 步骤 93 置 `RURIX_REQUIRE_REAL=1`（拟分配注「待 G7.3 落地后置 1」兑现）。device 段同时构成 **G-G7-5 执行门**的机器见证（AS descriptor/import 通道 + KernelWave::W3 缺一确定性拒绝 + validation 零错误）；步骤 94~96 维持拟分配。 |
| v1.0 | 2026-08-01 | G7.0 初版；拟分配步骤 93~96，不 materialize workflow/script/schema。 |
