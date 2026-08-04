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

---

## 6. 波次兑现追加(只增不改;§1~§5 基线审计口径 0-byte)

> 纪律:本节**只追加**各波次的真实 device 证据与分项状态迁移,不改写 §1 矩阵的 close 判据
> 与 §5 审计声明。RD-038 的 `status` 仍以 G-G7-9 的**逐字审计**为唯一裁判。

### 6.1 G7.4 W3c(2026-08-03;门 G-G7-6;CI 步骤 94)

**分项状态迁移**:§1 行 6「屏幕探针 GI」与行 7「RTAO 硬阴影」自 **无** → **已兑现**
(device kernel 本体在树 + 共用同一真实 TLAS 真跑 + host oracle 对拍全过)。

- **kernel 本体**:`apps/uc06-renderer/kernels/{gi_probe,rtao,hard_shadow}.rx`
  (§1 行 6/7「无 .rx kernel 本体」缺口关闭;`KERNEL_WAVE_ROUTES` 的 W3 路由 0-byte 不扩)。
- **共用同一真实 TLAS**(§1 行 7 close 判据「与 GI 共用同一真实 TLAS device 真跑」):
  一次 `VkAsManager::create_scene`(3 BLAS × 3 实例 = 冻结场景 764 三角形)+ 一条 command
  buffer + **单次提交**,三个 descriptor set 的 set 0 / binding 0 写入**同一** TLAS 句柄;
  evidence `shared_tlas.dispatch_tlas` 三项逐项等于 `shared_tlas.tlas_identity`。
- **几何语义对拍**(G-G7-6 逐字):hit/miss、`committed_instance_index`、
  `committed_primitive_index`、`committed_geometry_index` 四项 **零容差**(2304 探针光线 /
  1706 命中,mismatch 计数全 0);`committed_t` measured 1.43e-6(冻结 1e-5)、
  barycentric 两分量 measured 1.26e-5(冻结 1e-4)。
- **效果输出对拍**:GI 命中点辐射度(§1 行 6「GI 方向一致性对拍」)measured 1.19e-7
  (冻结 1e-5,对 `gi::tracer::RayTracedRadiance::trace` 逐光线);RTAO AO(§1 行 7
  「RTAO 同 TLAS 对拍」)measured **0.0 逐位一致**(冻结 1e-6,对 `rt::ref_tracer::
  rtao_reference`);硬阴影可见性 measured **0.0 零容差**(对
  `rt::ref_tracer::hard_shadow_reference`)。host oracle 数值语义 **0-byte**
  (未为过门改动;`cosine_sample_hemisphere` 仅**可见性**加性升 `pub`)。
- **validation 零错误**(§4.2-③ 未证实项的**部分补锚**):步骤 94 device 段以
  `RURIX_VK_VALIDATION=1` 真跑,`VK_EXT_debug_utils` messenger ERROR 级消息 fail-closed
  翻 `Err`;GREEN 路径零报错,RED 轴 `wrong-barrier` 经 VUID-02815 被拦截证 layer 生效。
  **限定**:本条只覆盖 W3c 三核执行路径;§4.2-③ 所指 W1/W2 五 kernel 的 `validation_clean`
  仍为环境开关记录,其补锚归后续波次。
- **RED 轴**:篡改 device 侧场景顶点 → 对拍必红(**数据流反证**)+ 注入式过期 TLAS
  fail-closed + 错误 barrier validation 拦截 + 编译面篡改 `.spv` → `spirv-val` 必拒。
- **evidence**:`evidence/renderer_w3_smoke_*.json`(schema
  `milestones/g7/renderer_w3_evidence_schema.json`);采集机 = NVIDIA GeForce RTX 4070 Ti
  (`G7_SCENE_FREEZE.md` §4.3 绑定口径:换机/换驱动须重采,不外推)。

**诚实边界(不充绿)**:
1. RTAO 采样方向为 **host 同源输入** buffer(与 oracle 取自同一次 `Pcg32` 生成),
   非 device 侧 RNG —— 冻结语义子集(RXS-0298)无 u64 `wrapping_mul`/`rotate_right` 供给面,
   device 实现 RNG 须扩语言面,按纪律不自行扩张;device 真做**遍历与遮蔽判定**。
   该输入纪律与 W1/W2 kernel 消费 host 预备输入同构,evidence `input_provenance` 字段化。
2. oracle 的**无效像素臂**(NaN/±inf 位置、零长法线/光方向)不在 device kernel 表达,
   由 host 单测覆盖;**miss 轴**(探针光线打空 / 阴影光线打空)照常 device 真实覆盖。
3. 本波兑现的是**孤立三核对拍**;并入连续真实设备帧(cull → VisBuffer → … → readback,
   provenance 逐 pass 资源 identity)归 G7.6 步骤 96,§1 行 1/2/4/8 的「帧链并入」余项不动。

**RD-038 status 结论**:**维持 open**。§1 八行中 HW 光栅(行 3)、VSM 深度(行 5)、
TSR(行 8 的 TSR 腿)仍无 device 证据,按 G-G7-7/G-G7-9「未覆盖任一项则 RD-038 保持 open,
禁止局部完成冒充全关」不得翻 closed;本波只把行 6/7 推入 **closed 候选**。

### 6.2 G7.5(2026-08-03;门 G-G7-7;CI 步骤 95)

**分项状态迁移**:§1 行 5「VSM 深度」自 **部分**(仅 page-mark)→ **已兑现**;
§1 行 8「TAA-TSR」的 **TSR 腿**自 **无** → **已兑现(空间超分核)**。
§1 行 3「HW 光栅」**维持无**,如实记 blocked-honest(见下「阻断点」)。

#### 已兑现:VSM 深度(行 5)

- **kernel 本体**:`apps/uc06-renderer/kernels/vsm_depth_raster.rx`(页内深度光栅)
  + `vsm_sample.rx`(阴影可见性采样);装配层 `apps/uc06-renderer/src/device_g75.rs`,
  CLI `--g75-residuals`。§1 行 5 缺口「页内深度的 device 渲染与采样对拍全缺」关闭。
- **device 真做的事**:深度腿 = 逐纹素 × 逐三角形边函数覆盖(`w0/w1/w2 >= 0`,边界含边)
  + 重心深度插值 + **min 归约**;采样腿 = 距离 → `select_level`(`ceil(log2(d/R0))` 钳制)
  → **逐级回退环** → `page_at` 窗口判定 → 槽位 `rem_euclid` → 页表项位解包
  (`phys`/`resident`/`dirty`)→ 最近邻纹素定位 → `dp <= stored + bias` 比较。
- **形态裁决**:深度腿取 **gather**(1 线程/(页, 纹素),线程内按 **host 同序**遍历三角形
  做 min)—— 每个输出纹素**单一写者**,由构造消除竞争,且与 host 单线程序**逐比较等价**;
  不引入原子,不建第二套光栅器。
- **对拍量**(RTX 4070 Ti / driver 620.02 实测):深度 **1 048 576** 纹素(64 页 × 128×128)
  × 764 三角形,measured max_abs = **3.576278687e-7** → 冻结 **1e-6**;覆盖纹素
  1 048 192(非退化)。采样 **764** 点(冻结场景逐三角形重心),measured = **0.0**、
  mismatch = **0**、遮蔽比 3.93% → **零容差**(0/1 二值,任何不一致都是级/页/纹素定位分歧)。

#### 已兑现:TAA-TSR 的 TSR 腿(行 8)

- **kernel 本体**:`apps/uc06-renderer/kernels/tsr_resample.rx` —— 逐字复刻
  `temporal::tsr::TsrUpscaler::resample_current_frame`:jitter 对齐的 **4×4
  Catmull-Rom**(Keys a=−0.5,超分时 `kernel_scale` 加宽)+ 邻域包络**抗振铃钳制** + 曝光;
  16 tap 以双层 `while` 保持 host 的 dy/dx 遍历序(浮点加法非结合,序即语义)。
- **对拍量**:内部 64×36 → 输出 128×72(冻结 `internal = out/2` 的 TSR 2× 契约),
  **27 648** 通道,measured max_abs = **1.490116119e-8** → 冻结 **1e-7**;
  抗振铃钳制真实生效 **1 921** 通道(证明覆盖了 Catmull-Rom 负瓣分支,判据不空转)。

#### 浮点残差归因(如实登记,非容差放宽借口)

device 与 host 的**表达式与求值序逐字一致**,残差唯一来源 = **浮点收缩**:SPIR-V 侧未加
`NoContraction` 装饰时驱动可把 `a*b − c*d`(边函数)与 `acc + w*p`(tap 累加)融合为 FMA,
而 Rust host 不自动收缩;实测量级正是 f32 单位舍入(ULP(1.0) = 1.19e-7)的数倍。
W2 VisBuffer 之所以能逐位相等,是因为其判据落在**量化后的 30 位整数域**,收缩差被量化吸收。
故按本波纪律 measured → 冻结,**未改 host oracle 数值语义**(`shadow::vsm` / `temporal::tsr`
本波 0-byte)。

#### 阻断点:HW 光栅(行 3)维持无 —— blocked-honest

- **判据未达成**:G-G7-7 字面「真实 graphics raster 输出对真实 W2 software raster 输出
  逐像素整数域 diff=0」的 **HW 侧不存在**,故 diff 无从谈起。**未放宽整数域容差,
  未以任何容差型替代物冒充**。
- **阻断依据(逐字)**:`spec/dxil_backend.md` **RXS-0171 L4**「最小 rvalue 白名单」——
  图形=B 路 body lowering **仅**支持 `Use`、f32/i32/u32 `Const`、标量/向量 f32/i32/u32
  **加减乘除** `BinaryOp`,与「声明的输出 I/O 聚合返回值」的机械分解;**控制流分支/循环、
  调用、借用/引用、cast、unary、资源/纹理/采样访问、非输出 I/O 聚合**一律 strict-only 拒。
  Vulkan 原生图形路**复用同一编码器**(`dxil_spirv::emit_spirv_body_vulkan`),故同一白名单
  在 `--target vulkan` 下以 **RX6026** 呈现。
- **实测证据(步骤 95 host 段第 5 项,机器可核)**:目标形态语料
  `conformance/vulkan/reject/vk_hw_raster_visbuffer_fs.rx` 必红 `RX6026` 且零 `.spv`;
  **五条逐轴隔离探针 + 一条绿对照臂**逐条落真实诊断,机器产出 `missing_toolchain_caps` 六项:

  | 缺失能力 | 隔离探针构造 | 实测诊断 |
  |---|---|---|
  | `graphics_vector_component_projection` | `inp.frag.0`(两层 Field) | 「最小切片仅支持单层 Field 投影」 |
  | `graphics_comparison_ops` | `v < 0.0` | 「最小切片仅支持 f32/i32/u32 加减乘除」 |
  | `graphics_control_flow_and_calls` | `.round()` | 「最小切片仅支持 straight-line Goto/Return」 |
  | `graphics_buffer_indexing` | `data[0]`(usize 常量) | 「最小切片仅支持 f32/i32/u32 常量」 |
  | `graphics_output_assembly` | 由标量装配 `vec4` 输出 | 「仅允许声明的输出 I/O 聚合返回值机械分解」 |
  | `graphics_ssbo_atomic_u64` | 编码器资源最小子集静态锚 | 「CBV/structured buffer SPIR-V 降级为后续扩展」 |

  绿对照臂(同形态单层直通)编译**绿**,证明红可归因于白名单而非探针写法。
- **覆盖规则分歧仍未裁定**:本波阻断发生在**语言面**,尚未触及 G7_PLAN §G7.5 风险表所列的
  「Vulkan top-left / edge coverage 与 software raster 覆盖规则规范差异」。该分歧**独立存在**
  且必须同期裁定 —— Vulkan 规范只保证 `subPixelPrecisionBits ≥ 4`(顶点先吸附到定点栅格),
  且共享边界像素归属为实现定义;software raster 侧是**精确 f32** 边函数 + top-left。二者
  在边界像素上不可能由规范推出逐位一致。故 RFC-0018 修订行须**同时**裁定「语言面加性扩展」
  与「覆盖规则的唯一权威定义(HW 侧是否须以 fragment 内复刻 SW 规则 + 保守光栅保证覆盖超集)」。
- **升级路径**:RFC-0018 修订行草案随本波报告提交;**在裁定前不自行扩语言面、不放宽容差**。

#### W1/W2 与既有波次零回归

五 kernel 对 `tests/vulkan/w1w2_spv_manifest.json` 的 sha256 + SPIR-V 版本 + capability
逐件零漂移(**不重 bless**),经步骤 93 / 94 / 95 三处独立复跑;步骤 66/67 恒绿。
`shadow::` / `temporal::` / `geometry::visbuffer` host oracle 全量恒绿。

- **evidence**:`evidence/renderer_raster_diff_smoke_*.json`(schema
  `milestones/g7/renderer_raster_diff_evidence_schema.json`);采集机 = NVIDIA GeForce
  RTX 4070 Ti / driver 620.02(`G7_SCENE_FREEZE.md` §4.3 绑定口径:换机/换驱动须重采)。

**诚实边界(不充绿)**:
1. VSM 灯空间三角形与逐页 `(origin, page_world, z_range)` 为 host 预备输入
   (**场景/配置装配面**,与 W1/W2 kernel 及 W3c 的 `sun_dir` 同纪律);device 真做光栅与
   采样判定。`page_mark` / `page_alloc`(屏幕反馈与池分配策略)本波仍在 host。
2. TSR 只兑现**空间超分核**;时域臂(history 双缓冲 / 闪烁 EMA 符号状态 / reproject +
   validity / 3×3 YCoCg AABB 混合)是**跨帧状态机**,与 `taa.rx` 已 device 化的时域面同构,
   并入连续真实帧归 G7.6 步骤 96。
3. 本波兑现的仍是**孤立对拍**;§1 行 1/2/4 的「帧链并入」余项不动。

**RD-038 status 结论**:**维持 open**。§1 八行中 **行 3「HW 光栅」仍无 device 证据**,
按 G-G7-7/G-G7-9「未覆盖任一项则 RD-038 保持 open,禁止局部完成冒充全关」不得翻 closed。
本波把行 5「VSM 深度」与行 8 的 TSR 腿推入 **closed 候选**(连同 G7.4 的行 6/7,
现 closed 候选 = 行 5/6/7/8;行 1/2/4 的帧链余项归 G7.6)。**本波不动
`registry/deferred.json`**;status 由 G7.7 逐字审计裁决。

### 6.3 G7.5b(2026-08-04;门 G-G7-7 轴一补全;CI 步骤 95 翻全绿)

**分项状态迁移**:§1 行 3「HW 光栅」自 **无** → **已兑现**。

- **语言面裁定留痕**:RFC-0018 修订行 **v1.1**(§E:HW 光栅 VisBuffer 对拍裁定;
  兑现 §6.2 承诺的「语言面加性扩展 + 覆盖规则唯一权威定义」双裁定)。spec 面 =
  `spec/vulkan_backend.md` **RXS-0301**(图形 body 扩展白名单)/ **RXS-0302**(SSBO +
  push constant + u64 原子)/ **RXS-0303**(保守光栅执行语义);台账 ledger v1.44
  (RXS on_tree_max 300→303)。DXIL 路 **RXS-0171 L4 一字不动**(FS dxil-target 必拒锁)。
- **kernel/管线本体**:
  - 语料:`conformance/vulkan/accept/vk_hw_raster_visbuffer_{vs,fs}.rx`(FS = SW
    判定段逐字同构);负面清单 reject ×4(`loop`/`devfn_call`/`cta_atomic`/`f64`)。
  - 装配:`apps/uc06-renderer/src/device_g75_hw.rs` + CLI `--g75-hw-raster`;
    `build.rs` 图形编译腿(VS/FS → `.spv`);rurix-rt `RasterPass.conservative=Some`
    + `VK_EXT_conservative_rasterization` DeviceCaps 探测/fail-closed。
- **diff=0 机器判据**:同场景同投影同 VisBuffer ABI(`depth30|cluster27|tri7` 冻结面
  0-byte),真实 graphics raster(保守光栅 OVERESTIMATE + FS 复刻 SW 判定)输出 vs
  真实 W2 SW compute 输出逐词整数域 **diff_pixels = 0**;覆盖
  `hw_covered_words == sw_covered_words = 7442`(非退化);evidence
  `hw_raster_diff.status` 自 `blocked-frozen-graphics-body-slice` 翻
  **`verified-diff-zero`**。
- **覆盖规则分歧处置**:RFC-0018 §E1/E2 裁定 —— 覆盖语义唯一权威 = SW 精确 f32
  边函数 + top-left;HW = 保守光栅超集派发 + FS 内逐字复刻过滤;**未放宽整数域容差**。
- **RED 反证轴**:`--g75-hw-red-tamper-varying`(交换 winner 三角形 flat vb/vc)与
  `--g75-hw-red-tamper-ids`(ids.cluster+1)均 `diff_pixels > 0`(数据流反证)。
- **W1/W2 零漂移**:`tests/vulkan/w1w2_spv_manifest.json` 五 kernel 逐字节零漂移
  (步骤 95 步骤 6 复跑;**不重 bless**)。
- **evidence**:`evidence/renderer_raster_diff_smoke_*.json`(本波新采;
  `hw_raster_diff.status=verified-diff-zero`,`diff_pixels=0`,
  `hw_side.pipeline=vk-graphics-conservative-raster`);采集机 = NVIDIA GeForce
  RTX 4070 Ti / driver 620.02(`G7_SCENE_FREEZE.md` §4.3 绑定口径)。

**诚实边界(不充绿)**:
1. 冻结场景重叠加盖下,host `VisBufferCpu` 与 device SW 的 packed 全屏逐位可因驱动
   FMA 收缩差数 ULP depth30 并改写 atomicMax 胜者(G7.5 残差归因同构);G-G7-7 本体
   = **同 GPU** 的 HW==SW(`diff_pixels=0`)。SW↔host packed 全屏逐位锚由 W2 合成
   80 三角形场景(`sw_baseline` / `device_w2_visbuffer_u64_bitexact_host`)承担;
   冻结场景侧 `oracle_bitexact` = **覆盖集合**对齐。
2. 降级臂(host 三边外扩几何膨胀)本波**未启用**;DeviceCaps 无保守光栅 → fail-closed。

**RD-038 status 结论**:**维持 open**(帧链并入余项行 1/2/4/8 归 G7.6;
本波把行 3 推入 closed 候选——至此 §1 八行中 closed 候选 = 行 3/5/6/7/8)。
本波不动 registry/deferred.json;status 由 G7.7 逐字审计裁决。

### 6.4 G7.6(2026-08-04;门 G-G7-8;CI 步骤 96 materialize + soak schema)

**分项状态迁移**:§1 行 1「两级剔除」/行 2「VisBuffer SW」/行 4「classify-
resolve」/行 8「TAA-TSR」的**帧链并入余项**全部关闭;§6.2 诚实边界 2 所列
TSR **时域臂**(history 双缓冲/reproject+validity/YCoCg AABB)兑现。

- **帧链 provenance 机验**:cull→VisBuffer→classify/resolve→VSM/lighting→
  TAA/TSR→readback 每一箭头的 input/output resource identity 逐 pass 记录,
  evidence 字段化;isolated nonzero 拼装的 RED 反证轴
  (`--frame-red-visbuffer` / `--frame-red-history` / `--frame-red-jitter` /
  `--frame-red-provenance`)。
- **步骤 96 materialize**:`ci/renderer_device_frame_smoke.py` + workflow 步骤
  96 + 两 schema + `check_schemas.py` 前缀路由;**ledger v1.45 校准**(CI_step
  `on_tree_max` 95→96 / `next_free` 96→97;v1.44 RXS-0301~0303 0-byte 保留)。
- **soak(close-out 专用取证,不占步骤号,CI_GATES §3)**:
  `--soak --frames 10000 --min-minutes 30` → `actual_frames ≥ 10000` 且
  `elapsed_minutes ≥ 30`;validation/device-loss/TDR/resource-leak 全 0;
  schema `renderer_soak_evidence_schema.json` 本波预置,真跑归 PR-4。
- **预算追加**:g7_budget.json 追加归 PR-4;本波 0-byte。
- **evidence**:`evidence/renderer_device_frame_smoke_<ts>.json`(本波);
  soak evidence 归后续。

**RD-038 status 结论**:八行全部推入 closed 候选;**本波仍不动
registry/deferred.json**;**RD-038 维持 open**,status flip 唯一归 G7.7 逐字终审。
