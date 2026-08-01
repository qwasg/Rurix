# RD-038 字面矩阵(G7.1 基线审计;门 G-G7-3)

> 审计对象:[`registry/deferred.json`](../../registry/deferred.json) RD-038(status=open,owner_milestone=G5.3)。
> 审计依据:[G7_CONTRACT.md](G7_CONTRACT.md) G-G7-3(逐字审计 title/backfill_condition/history,形成 pass×host×device×evidence 矩阵)/ [G7_PLAN.md](G7_PLAN.md) G7.1 / [CI_GATES.md](CI_GATES.md) §2(步骤 93~96 拟分配)。
> 来源条款:[`rfcs/0016-native-renderer.md`](../../rfcs/0016-native-renderer.md) §4.E3(device 腿条件臂)与 §9.1 R-3(rayQueryEXT compute 编码通道未核实,不伪造 device 绿)。
> 审计日期:2026-08-01。审计方式:逐字比对 RD-038 原文与仓库现状;每条事实均带文件/行号或 evidence 锚,无法锚定的项如实标「未证实」。目标 smoke 与 evidence schema 均为 G7 CI_GATES §2/§4 拟分配名,随真实脚本 materialize 回填。

## 0. title 逐字

> 渲染器效果 kernel device 化——两级剔除/VisBuffer SW(u64 atomicMax)+HW 光栅/classify-resolve/VSM 深度/屏幕探针 GI/RTAO 硬阴影/TAA-TSR 的 GPU compute/raster kernel 化 + device 对拍(host 参考已全量锚定)

字面斜切段共 7 项;其中「VisBuffer SW(u64 atomicMax)+HW 光栅」按 `+` 两侧 device 现状不同拆为两行,矩阵合计 8 行。尾句「的 GPU compute/raster kernel 化 + device 对拍(host 参考已全量锚定)」为对全部分项的谓语要求,在 §2 单列核对。

## 1. 字面矩阵

| 分项 | host oracle 现状(文件+测试数) | 当前 device 现状 | 缺口 | 目标 smoke(拟) | evidence schema 名(拟) | close 判据(可机验) |
|---|---|---|---|---|---|---|
| 两级剔除 | `src/rurix-render/src/geometry/cull.rs` 7 单测(instance_cull/cluster_cull 两级)+ `geometry/gpu_scene.rs` 6;上游 `rurix-geom-build` 22 单测含 CPU 参照剔除(步骤 84 evidence `geom_build_count=22`);`geometry::` cargo 过滤合计 29 | **已兑现**(W1):`apps/uc06-renderer/kernels/cull.rx` + `src/device_kernels.rs::match_w1_cull`;`device_w1_cull_matches_host` 真跑绿,120 簇(3 实例×40)场景可见 72 簇集合与 host 排序后逐对相等(`evidence/uc06_renderer_smoke_20260731T164219.json`:`device_cull_visible_clusters=72`) | 无功能缺口;余项=自孤立对拍并入连续真实帧链(cull 输出须被下游真实消费,禁 isolated nonzero 拼装) | 96(帧链并入);既有步骤 84/87 判据 0-byte 只增维持 | `renderer_device_frame_evidence_schema.json`(回归面:既有 `renderer_visbuffer_smoke_evidence_schema.json` / `uc06_renderer_smoke_evidence_schema.json`) | 步骤 84 evidence `device_wave_tests` 含 `device_w1_cull_matches_host` 且 `device_section_rc=0` 恒绿;步骤 96 evidence 中 cull pass 的 output resource identity 被 VisBuffer pass input 真实消费的 provenance 记录存在 |
| VisBuffer SW(u64 atomicMax) | `geometry/visbuffer.rs` 7 单测(`VisBufferCpu` 逐位对拍金标准,含位格式与 `VISBUFFER_CLEAR`) | **已兑现**(W2):`kernels/visbuffer_sw_u64.rx`;u64 atomicMax 经 `Rvalue::Atomic`→`OP_ATOMIC_UMAX`,`CAP_INT64`/`CAP_INT64_ATOMICS` 按需声明(`src/rurixc/src/vulkan_codegen.rs:147-148,1387-1397,1937-1942`);128×72=9216 词 u64 与 host 逐位一致容差 0(`device_visbuffer_matched_words=9216`,`device_kernels.rs:312`) | 无功能缺口;余项=作为 SW 基准侧参与 SW/HW 整数域 diff + 帧链并入 | 95(diff 基准侧)+ 96(帧链);既有 84/87 维持 | `renderer_raster_diff_evidence_schema.json` | `device_w2_visbuffer_u64_bitexact_host` 恒绿;步骤 95 evidence 记录同场景同投影同 VisBuffer ABI 下 SW 侧输出与 HW 侧逐像素整数域 diff=0 |
| HW 光栅 | 复用上行 `geometry/visbuffer.rs` 7 单测同一金标准(HW 侧不另造 host 参考) | **无**:`apps/uc06-renderer/kernels/` 仅 5 件(cull/visbuffer_sw_u64/classify_resolve/vsm_page_mark/taa),无 HW 光栅件;`KERNEL_WAVE_ROUTES`(`src/rurix-rt/src/render_exec.rs:531-564`)无对应路由;RD-038 history(2026-07-30)明记「HW 光栅 diff 腿维持 blocked 存续」 | 真实 graphics raster 写 VisBuffer 的 kernel/管线全缺;SW/HW 对拍未建立;Vulkan top-left/edge coverage 与 SW 规则的规范差异未经 RFC 裁定(G7_PLAN G7.5 风险表) | 95 | `renderer_raster_diff_evidence_schema.json` | 步骤 95 device 段 `RURIX_REQUIRE_REAL=1` 真跑:真实 graphics raster VisBuffer 对真实 W2 SW VisBuffer 逐像素整数域 diff=0;覆盖规则差异只经 RFC 裁定,不放宽容差 |
| classify-resolve | `geometry/material_pass.rs` 5 单测(classify 桶聚 + resolve 逐像素材质) | **已兑现**(W1):`kernels/classify_resolve.rx`;9216 像素 resolve 逐值一致 + 8 材质桶计数一致(`device_classify_matched_pixels=9216`,`device_kernels.rs:366-372`) | 无功能缺口;帧链并入(须真实消费 VisBuffer pass 输出) | 96;既有 84/87 维持 | `renderer_device_frame_evidence_schema.json` | `device_w1_classify_resolve_matches_host` 恒绿;步骤 96 evidence provenance 记录其 input 为上游 VisBuffer pass 的真实输出资源 |
| VSM 深度 | `shadow/vsm.rs` 17 单测(+ `clipmap.rs` 4 / `page_table.rs` 3 / `pool.rs` 2;`shadow::` 过滤合计 26,步骤 85 evidence `shadow_count=26`) | **部分**:仅 `kernels/vsm_page_mark.rx`(W1;4 页标记位图与 host `Vsm::page_mark` 一致,`device_vsm_marked_pages=4`);title 所指「VSM 深度」(页内深度渲染/采样)无任何 device kernel | backfill「VSM device 深度对拍」未兑现:页内深度的 device 渲染与采样可见性对拍全缺 | 95(拟步骤 95 device 段明列「VSM depth/TSR 等余项 device 见证」,G7 CI_GATES §2) | `renderer_raster_diff_evidence_schema.json` | 步骤 95 evidence 含 VSM 深度 device 真跑条目:与 `shadow::vsm` host oracle 在 RFC 冻结容差内对拍;host 仅 oracle 不参与成功路径 |
| 屏幕探针 GI | `gi/` 20 单测(`pipeline.rs` 5 / `probe.rs` 3 / `sh.rs` 3 / `tracer.rs` 3 / `interpolate.rs` 2 / `filter.rs` 2 / `temporal.rs` 2;`gi::` 过滤合计 20,含能量守恒/方向一致性;步骤 85 evidence `gi_count=20`) | **无**:`gi_probe` 路由 `KernelWave::W3`(`render_exec.rs:552-555`)但无对应 .rx kernel;步骤 85 evidence blocked-honest:`device_blocked_w3="RD-038-W3"`,`missing_toolchain_caps=["ray_query_codegen","spirv_1_4"]`;设备侧 ray query 五件链全真(`device_ray_query=true` 等,非阻塞项) | SPIR-V 1.4 + RayQuery codegen(93 前置)、真实 TLAS compute descriptor(94 前置)、`gi_probe.rx` 本体与 host 对拍 | 93(编码通道)→ 94(效果对拍) | `ray_query_codegen_evidence_schema.json` + `renderer_w3_evidence_schema.json` | 93:`spirv-val` 通过 + golden 锚定 OpTypeRayQueryKHR/OpRayQueryInitializeKHR 等关键指令及 capability/extension;94:`gi_probe.rx` 共用真实 TLAS device 真跑,方向一致性/能量守恒与 `gi::` host oracle 在 RFC 冻结容差内一致,validation 零错误 |
| RTAO 硬阴影 | `rt/` 49 单测(`effects.rs` 11 / `ref_tracer.rs` 8 / `as_manager.rs` 11 / `bvh.rs` 11 / `denoise.rs` 8;`rt::` 过滤合计 49,步骤 85 evidence `rt_count=49`;`ref_tracer` = 同 TLAS 对拍金标准) | **无**:`rtao`/`hard_shadow` 路由 W3(`render_exec.rs:556-563`)无 kernel;同 GI blocked-honest(步骤 85 同一 evidence 记录) | 同 GI 三前置 + `rtao.rx`/`hard_shadow.rx` 本体;同 TLAS 对拍链未建立 | 93 → 94 | `ray_query_codegen_evidence_schema.json` + `renderer_w3_evidence_schema.json` | 94:`rtao.rx`/`hard_shadow.rx` 与 GI 共用同一真实 TLAS device 真跑;hit/miss/t/instance/primitive/barycentric 在 RFC 冻结容差内与 `rt::ref_tracer` 一致;静态序列帧间差趋零;validation 零错误 |
| TAA-TSR | `temporal/taa.rs` 3 + `tsr.rs` 13(底座 `common.rs` 14 / `ssim.rs` 6 / `image.rs` 3 / `upscale.rs` 5);`temporal::` 过滤合计 46(步骤 86 evidence `temporal_count=46`——cargo 子串过滤兼中 `gi::temporal` 2 件,本模块 `#[test]` 实计 44) | **部分**:`taa.rx` 已兑现(W1;`device_taa_max_err=1.2e-7` ≤ 断言阈值 1e-5,`device_kernels.rs:508`);TSR 无任何 device kernel(纯 host) | TSR device 化全缺;TAA 余项=帧链并入 | TSR→95(「TSR 等余项 device 见证」);TAA→96;既有 86/87 维持 | `renderer_raster_diff_evidence_schema.json`(TSR 见证)+ `renderer_device_frame_evidence_schema.json`(帧链) | 步骤 95 evidence 含 TSR device 真跑与 host `tsr.rs` 对拍条目(数值/SSIM 容差先 measured 后经 RFC 冻结);步骤 96 provenance 记录 TAA 在帧链中真实消费上游输出 |

## 2. 尾句谓语逐字核对

- 「的 GPU compute/raster kernel 化 + device 对拍」:compute 侧 8 分项中 5 件已有 .rx device kernel 并真跑对拍(cull/visbuffer_sw_u64/classify_resolve/vsm_page_mark/taa,见 §1);raster 侧(HW 光栅)为 0;W3 三件(gi_probe/rtao/hard_shadow)仅有 `KERNEL_WAVE_ROUTES` 路由声明,无 kernel 本体。未完成项不得以 host 参考冒充 device 绿(RFC-0016 §4.E3 / §9.1 R-3)。
- 「host 参考已全量锚定」:**属实**。`src/rurix-render/src/` 41 文件 `#[test]` 合计 239,与 RD-038 reason「239 单测」字面一致;crate 级 `#![forbid(unsafe_code)]` 在位(`src/rurix-render/src/lib.rs:19`);六面(剔除/VisBuffer/VSM/GI/RT/TAA-TSR)host 参考文件全部在位(见 §1 host 列)。

## 3. backfill_condition 逐字核对

原文:

> rurixc vulkan_codegen 效果 kernel 编码通道就位(ray query/u64 atomic/storage image 写等,条款按需自 RXS-0297 顺位)后逐效果兑现:GPU 剔除与 CPU 蛮力逐簇一致对拍/VisBuffer SW-HW 逐像素 diff 容差 0/VSM device 深度对拍/GI 方向一致性对拍/RTAO 同 TLAS 对拍——host 参考器(geometry::cull/visbuffer,shadow::vsm,gi::pipeline,rt::ref_tracer,temporal::taa/tsr)即金标准,判据已在 G5 CI_GATES §2 步骤 84~86 device 段 blocked 探针占位恒跑

| 子句 | 核对结论 |
|---|---|
| 编码通道就位:u64 atomic | **部分就位(本项已兑现)**:`Rvalue::Atomic`→`OP_ATOMIC_IADD/ISUB/SMIN/UMIN/SMAX/UMAX/AND/OR/XOR/EXCHANGE`(`vulkan_codegen.rs:1270,1387-1397`);`CAP_INT64`(=11)/`CAP_INT64_ATOMICS`(=12)按需声明(`:147-148,1937-1942`) |
| 编码通道就位:storage image 写 | **部分就位(本项已兑现)**:compute `TextureRw2D` format-qualified 存储图像 load/store(`vulkan_codegen.rs:261,1433-1492`) |
| 编码通道就位:ray query | **未就位**:步骤 85 evidence `missing_toolchain_caps=["ray_query_codegen","spirv_1_4"]`;即 G7.2 W3a 工作内容。「条款按需自 RXS-0297 顺位」的 spec 条款兑现情况:**未证实**(本次审计未逐条核 spec 编号) |
| GPU 剔除与 CPU 蛮力逐簇一致对拍 | **已兑现**:可见簇集合(72/120)与 host 两级剔除排序后逐对相等(§1 行 1) |
| VisBuffer SW-HW 逐像素 diff 容差 0 | **未兑现**:SW 侧已锚定(9216 词 u64 逐位一致容差 0),HW 腿不存在,SW-HW diff 无从谈起(§1 行 2/3) |
| VSM device 深度对拍 | **未兑现**:仅 page-mark 页位图 device 对拍;页内深度无 device kernel(§1 行 5) |
| GI 方向一致性对拍 | **未兑现**:W3 blocked-honest(§1 行 6) |
| RTAO 同 TLAS 对拍 | **未兑现**:W3 blocked-honest(§1 行 7) |
| host 参考器即金标准(geometry::cull/visbuffer, shadow::vsm, gi::pipeline, rt::ref_tracer, temporal::taa/tsr) | **属实**:六个文件/模块全部在位,单测计数见 §1 host 列 |
| 判据已在 G5 CI_GATES §2 步骤 84~86 device 段 blocked 探针占位恒跑 | **属实且已细化**:84 = W1/W2 gate-real 真跑(cull/visbuffer_u64/classify_resolve),85 = W1 gate-real(vsm_page_mark)+ W3 blocked-honest 留痕,86 = W1 gate-real(taa);三步 2026-07-30 evidence 全绿(`renderer_{visbuffer,lighting,temporal}_smoke_20260730T1213*.json`) |

## 4. history 逐字核对(2 条)

### 4.1 条目一(2026-07-29,G5.3 W2 交付登记)

「效果六面 host 参考全量落地…确定性单测…步骤 84~86 device 段以 blocked-honest 探针恒跑」——**属实**:六面 host 文件与 239 单测在位(§1 host 列);步骤 84~86 三脚本在位且 device 段为分波探针结构(`ci/renderer_visbuffer_smoke.py` / `ci/renderer_lighting_smoke.py` / `ci/renderer_temporal_smoke.py`)。

### 4.2 条目二(2026-07-30,W1+W2 分波部分兑现)

- ① 「32 位 SSBO 原子 + compute TextureRw2D + Int64/Int64Atomics(u64 算术+OpAtomicUMax,capability 按需声明,SPIR-V 维持 1.0)」——**属实**(行号锚见 §3 前三行)。
- ② 「DeviceCaps 扩 ray query 五件链探测 + KernelWave W1/W2/W3 fail-closed require_wave 路由」——**属实**:`W3_REQUIRED_CAPABILITIES` = synchronization2 + shader_buffer_int64_atomics + ray_query/acceleration_structure/buffer_device_address/descriptor_indexing/deferred_host_operations 五件链(`render_exec.rs:474-482`);`require_wave` fail-closed(`:508-519`);`KERNEL_WAVE_ROUTES` 8 条路由(`:531-564`)。
- ③ 「五效果内核 .rx→SPIR-V device 真跑对拍全绿(cull 72/120/VisBuffer 9216 词 u64 逐位/classify-resolve 一致/VSM 页位图一致/TAA 最大误差 1.2e-7;RTX 4070 Ti)」——**属实**:`evidence/uc06_renderer_smoke_20260730T120711.json` 与 `…20260731T164219.json` 逐项吻合(cull 72、visbuffer 9216、classify 9216、vsm 4、taa 1.2e-7、NVIDIA GeForce RTX 4070 Ti)。**但同句「validation 零报错」未在 evidence 中锚定**:两份 uc06 evidence 的 `device.validation_clean=false`,经核该字段仅是 `RURIX_VK_VALIDATION` 环境开关记录(`apps/uc06-renderer/src/pipeline.rs:1008`),并非「validation 已开启且零错误」的测量——此项标**未证实**,validation 零错误须由 G7 新证据以 validation 开启真跑锚定(G-G7-5/6/8 门要求)。
- ④ 「步骤 84~86 device 段细化为 W1/W2 gate-real 真跑 + W3 blocked-honest…W3 与 HW 光栅 diff 腿维持 blocked 存续,status 维持 open」——**属实**(三 smoke 脚本现状与 §1 行 3/6/7;RD-038 status 现仍为 open)。

## 5. 审计声明

1. 字面分项兑现统计(8 行):**已兑现 3**(两级剔除、VisBuffer SW(u64 atomicMax)、classify-resolve);**部分 2**(VSM 深度——仅 page-mark;TAA-TSR——仅 TAA);**无 3**(HW 光栅、屏幕探针 GI、RTAO 硬阴影)。
2. 按 G7 契约 §6(RD-038「按 title、backfill_condition、history 逐字验收;全部兑现才 closed」)与 G-G7-7/G-G7-9(未覆盖任一字面分项则 RD-038 保持 open,禁止局部完成冒充全关):**当前 RD-038 必须保持 open**;本矩阵为 G-G7-3 基线审计件,各 close 判据以步骤 93~96 脚本 materialize 后产出的真实 device evidence 为最终裁判。
3. 未证实项汇总:① history 条目二「validation 零报错」缺乏 evidence 锚(现有 `validation_clean` 仅为开关记录);② backfill_condition「条款按需自 RXS-0297 顺位」的 spec 条款兑现情况未逐条核;此两项不构成分项兑现,仅作留痕,待 G7.2/G7.5 波次补锚。
