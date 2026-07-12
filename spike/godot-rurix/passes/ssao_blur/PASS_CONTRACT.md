# GRX-011 SSAO/SSIL Blur Pass — PASS CONTRACT

> **状态声明(2026-07-08,segment A:pass contract + offline kernel + bridge gate + standalone dispatch smoke)。**
> 本切片完全复用 GRX-010 tonemap 已打通的成熟模板(GRX-009 模板的最新精简版):pass 契约三件套、HLSL bridge 数学等价 kernel(DXC 编译 + DXV 验证 + Rurix-owned RTS0 合成,owner-approved `hlsl_bridge_workaround` provenance,政策 `../luminance_reduction/texture_artifact_provenance_policy.json` 适用于所有 texture compute pass)、bridge `SsaoBlurGate`(preflight → eligibility → binding-kind → math-parity → real-dispatch 链,默认恒 `RXGD_STATUS_FALLBACK`)、Godot patch 0012(per-pass 设置默认 `false` + call-site opt-in gate,原生 SSAO blur 路径始终保留 fallback)、standalone real D3D12 dispatch smoke。
> 本切片**不**做 Godot scratch 全栈重建与引擎内实测(4f/4h 级别留后续切片)、**不**做引擎内 visual diff、**不**启用默认 pass、**不**宣称任何 FPS / GPU timestamp / 性能提升。measured 上限为 standalone dispatch + CPU parity。
> pass 默认 `disabled`;任何 compile / validation / visual / perf 失败都走 Godot 原生 SSAO blur 路径。
> §3 对 `external/godot-master` 的调查只记录路径与函数名;Godot 侧改动只以 `spike/godot-rurix/patches/` 下的 patch 文件入库,不直接修改快照的 Godot 原生源文件。

## 1. Pass 标识

- `pass_id = ssao_blur`
- bridge pass id:`RXGD_PASS_SSAO_BLUR = 2`(`src/rurix-godot/src/lib.rs` 既有预留;`RXGD_PASS_SSIL_BLUR = 3` 同为预留但本切片不接线)
- Tier:Tier 1(低风险 pass 候选,GRX-009/GRX-010 之后第三个)
- 目标后端:`Godot 4.7-dev Windows D3D12 Forward+`
- 默认启用状态:`disabled`

## 2. 目标场景

- `mixed_forward_plus`
- `clustered_lights`

(对齐 GRX_PLAN GRX-011 任务行的 temporal/noise 稳定性注意事项:SSAO 属 screen-space 效果,先覆盖含几何/光照复杂度的两个场景;temporal 稳定性 evidence 属后续 visual diff 段。)

## 3. Godot 侧候选 hook / call site / 资源流调查结果

仅记录路径与函数,**不改 `external/godot-master`**。

### 3.1 Effect 类

- 头文件:`servers/rendering/renderer_rd/effects/ss_effects.h`
- 源文件:`servers/rendering/renderer_rd/effects/ss_effects.cpp`
- 关键函数 / 类型:
  - `RendererRD::SSEffects::generate_ssao(...)`(`ss_effects.cpp:1130`)——SSAO 全链路(downsample → gather → **Edge-Aware Blur** → interleave)compute 入口;blur 段位于 `L1320-1378`(`draw_command_begin_label("Edge-Aware Blur")` 在 `L1321`)。
  - `RendererRD::SSEffects::generate_ssil(...)` 的 "Edge Aware Blur" 段(`L929-996`)——SSIL 变体,本切片不接线(known gap)。
  - `SSEffects::SSAOBlurPushConstant`(`ss_effects.h:383-387`,16 bytes:`edge_sharpness`、`pad`、`half_screen_pixel_size[2]`)。
  - `SSEffects::ssao_set_quality(...)`(`ss_effects.cpp:1048`)——`ssao_blur_passes`(默认 2,`ss_effects.h:168`)与 quality 决定 blur 变体与 pass 数。
- pipeline 枚举:`SSAO_BLUR_PASS` / `SSAO_BLUR_PASS_SMART` / `SSAO_BLUR_PASS_WIDE`(`ss_effects.h:334-336`)。

### 3.2 Shader

- `servers/rendering/renderer_rd/shaders/effects/ssao_blur.glsl`(本 kernel 的数学对标源;Intel ASSAO 派生,compute,`local_size 8x8x1`):
  - `unpack_edges` L39-48:packed byte → 4×2-bit LRTB 边缘值 / 3.0,再 `clamp(edges + edge_sharpness, 0, 1)`。
  - `MODE_SMART` 的 `sample_blurred` L95-122:中心 `texelFetch` 取 packed edges,两次 `textureGather`(±half_pixel*0.5)取 center/L/R/T/B 值;`sum_weight` 起始 0.5,L/R/T/B 按边缘权重 `add_sample`(L50-55),结果 `sum / sum_weight`;输出 `vec2(blurred, packed_edges)`。
  - `MODE_WIDE` 的 `sample_blurred_wide` L58-91:±2 texel 十字,edge 权重双向相乘,`sum_weight` 起始 0.8(不支持,known gap)。
  - `MODE_NON_SMART` L129-143:4 次对角 `textureLod` 加权 0.2(不支持,known gap)。
  - `main()` L125-154:`imageStore(dest_image, ssC, vec4(sampled, 0.0, 0.0))`。
- `servers/rendering/renderer_rd/shaders/effects/ssil_blur.glsl`(SSIL 变体:`rgba16` 值 image + 独立 `r8` edges image,vec4 加权;不支持,known gap)。

### 3.3 调用 / 注入候选点

- `servers/rendering/renderer_rd/effects/ss_effects.cpp` `generate_ssao` 的 Edge-Aware Blur 段(`L1320-1378`):
  - push constants 组装 `L1322-1324`:`edge_sharpness = 1.0 - p_settings.sharpness`、`half_screen_pixel_size = 1.0 / buffer_{width,height}`。
  - `blur_passes = ssao_quality > VERY_LOW ? ssao_blur_passes : 1`(`L1326`);pass < blur_passes-2 用 WIDE,否则 SMART(VERY_LOW 用非 smart,`L1330-1338`)。
  - **注入点**:`for (int i = 0; i < 4; i++)` slice 循环体(`L1340-1373`)——每 slice 绑定 source sampler+texture(set 0)与 dest image(set 1),ping-pong(`pass % 2`,`L1347-1369`),`compute_list_dispatch_threads(buffer_width, buffer_height, 1)`(`L1372`)。patch 0012 在 blur 段整体前插入 opt-in gate:只有 `D3D12Hooks::get_singleton()->try_record_ssao_blur()` 返回 `true`(module 设置开启 且 bridge `rxgd_record_pass(RXGD_PASS_SSAO_BLUR)` 返回 `RXGD_STATUS_OK`)时才跳过原生 blur 循环;否则原生循环照常执行。SSIL blur 段(`L929-996`)不接线。
- 上游调用:`render_forward_clustered.cpp` `_process_ssao`(`L1402`)→ `ss_effects->generate_ssao(...)`(`L1424`)。

### 3.4 资源流(原生)

- 输入:`ao_deinterleaved_slices[i]` / `ao_pong_slices[i]`(ping-pong;`RB_SCOPE_SSAO` `RB_DEINTERLEAVED`/`RB_DEINTERLEAVED_PONG`,`R8G8_UNORM`,4 slices,`ss_effects.cpp:1123-1124`;x = ssao value,y = packed edges)。
- 尺寸:`buffer_width/height = (full_screen + 1) / 2`(half_size 时 `(full_screen + 3) / 4`,`L1104-1114`)——deinterleaved slice 分辨率。
- 采样器:`ss_effects.mirror_sampler`(VERY_LOW 用 default sampler,`L1347-1365`)。
- 输出:对侧 ping-pong slice(`UNIFORM_TYPE_IMAGE`,rg8 storage image)。
- 原生 pass 形态为 compute(与 bridge 模板同构);但原生一帧含 `blur_passes × 4 slices` 次 dispatch 的 ping-pong 链,本切片 kernel 只做**单次** SMART blur(1 slice、1 pass)的数学等价子集,链式调度与 rg8 unorm 存储量化属 known gaps。

## 4. 输入 / 输出资源(Rurix mapping)

- 输入:`src_ssao = Texture2D<float4>`,SRV `t0 space0`,`binding_kind = texture2d`(Godot deinterleaved slice 的 native `ID3D12Resource*`;x = ssao value,y = packed edges)。
- 输出:`dst_ssao = RWTexture2D<float4>`,UAV `u0 space0`,`binding_kind = rwtexture2d`(ping-pong 对侧 slice;dst 尺寸 == src 尺寸,1:1)。
- b0 root constants(28 bytes / 7 dwords,root_parameter_index 0,复用 GRX-009/GRX-010 canonical 打包形状 `[i64, i64, f32, f32, f32]`):
  - `source_width`(i64,dword 0-1)/ `source_height`(i64,dword 2-3)——deinterleaved slice 尺寸
  - `edge_sharpness`(f32,dword 4)= Godot `1.0 - p_settings.sharpness`
  - `half_screen_pixel_size_x`(f32,dword 5;与 Godot `SSAOBlurPushConstant` 形状对齐,本 Load 寻址 kernel 不消费)
  - `half_screen_pixel_size_y`(f32,dword 6;同上)
- tracked mapping:`resource_mapping.md`。

## 5. 支持范围与 gaps(起步口径)

- **支持**:`MODE_SMART` 单次 blur pass、单 slice——edge-aware 3x3 十字(center + L/R/T/B),`unpack_edges` 权重、`sum_weight` 起始 0.5、packed edges passthrough、z/w 写 0;边界用 clamp 寻址(interior texel 与 Godot gather 寻址逐 texel 等价)。
- **不支持(known gaps,manifest `known_gaps` 逐条入账)**:`MODE_WIDE`(±2 texel、双向 edge 乘、0.8 起始权重)、`MODE_NON_SMART`、多 pass ping-pong 链(`ssao_blur_passes` 默认 2)与 4-slice 循环调度、SSIL blur(rgba16 值 + 独立 r8 edges image,`RXGD_PASS_SSIL_BLUR` 不接线)、mirror-sampler 边界寻址(border texel 与 clamp 寻址可能不同)、rg8 unorm 存储量化(kernel 按 float 计算;原生 slice 为 `R8G8_UNORM`)、gather-vs-load 半像素寻址接缝(非 interior texel)、multiview、temporal/noise 稳定性 evidence。

## 6. Fallback

- fallback reason 枚举(对齐 GRX-008 五枚举):`compile_failed` / `validation_failed` / `unsupported_device` / `visual_diff_failed` / `manual_disabled`。
- 任一 compile / validation / visual / perf 失败 → 回退到 Godot 原生 SSAO blur 路径(`godot_native_ssao_blur`)。
- 默认 Godot config(per-pass 设置全部 `false`)与 shipping bridge 对 `RXGD_PASS_SSAO_BLUR` 恒返回 `RXGD_STATUS_FALLBACK`,原生 blur 循环始终执行。

## 7. Bridge gate(`src/rurix-godot/src/lib.rs` `SsaoBlurGate`)

模板复制 GRX-010 `TonemapGate` 的检查链,常量与 digest 指向 ssao_blur 产物:

1. **runtime binding preflight**:64-bit integer capability flag(b0 承载 i64 dims 的既有模板口径)、恰好 2 个 texture 资源(src 在前 dst 在后)、28-byte push constants、b0 中 source dims 非零且与 `src_ssao` 资源一致、`dst_ssao` 尺寸 == source 尺寸(1:1 ping-pong 形状)。
2. **dispatch eligibility**:opt-in capability flag `RXGD_CAP_SSAO_BLUR_REAL_PASS (1u << 5)`(复用 `RxGdCaps.flags` 位,**不改 C ABI struct layout,`RXGD_ABI_VERSION` 保持 1**;缺失 → `manual_disabled`)、64-bit integer capability、native D3D12 device/queue 非空、resource native handle 非空、compiled package layout/digest 与 offline evidence 三个 SHA-256 逐字节匹配。
3. **kernel-binding-kind conformance**:`SSAO_BLUR_KERNEL_RESOURCE_BINDING_KINDS = ["texture2d", "rwtexture2d"]` per-slot 校验;buffer 资源 fail closed。
4. **math parity gate**:`SSAO_BLUR_KERNEL_MATH_PARITY_STATUS = "smart_blur_cpu_reference_proven_pending_gpu_dispatch"`(`math_parity_evidence.json`:CPU reference 已证,GPU 侧观察 pending real dispatch)。
5. **real dispatch**:仅在 `d3d12-recording-shim` feature(默认关闭)下经参数化的通用 texture-pass 录制 shim(`shim/rxgd_luminance_record.cpp` 的 `rxgd_luminance_record_dispatch`,SRV t0 + UAV u0 + 28-byte b0 + `ceil(dims/8)` dispatch 形状对 ssao_blur 逐项吻合,view format 由真实资源 format 推导)录制;shipping feature-off bridge fail closed 为 `real_dispatch_path_not_linked`。
- 任一失败返回 `RXGD_STATUS_FALLBACK`、记录 fallback reason、每 session 一次打印机读诊断 `RXGD_SSAO_BLUR_REAL_PASS_BLOCKED first_missing_prerequisite=... fallback_reason=... kernel_binding=texture2d default_enable_state=disabled`(非 `ERROR:` 行、不含 `RXGD_DIAG`),不累计 estimated GPU/CPU time。
- **行为变更入账**:`RXGD_PASS_SSAO_BLUR` 在本段前走 bridge 的占位 estimated-timing 路径(record OK + 伪 estimated GPU time 120_000ns);自本段起改走 `SsaoBlurGate` fail-closed 链,默认恒 fallback、不再产生任何 estimated ssao_blur GPU time。`RXGD_PASS_SSIL_BLUR` 保持占位 estimated-timing 路径不变(本切片不接线)。

## 8. Godot patch 0012

- `spike/godot-rurix/patches/0012-rurix-accel-ssao-blur-pass-gate-and-callsite.patch`,栈式叠加在 0001..0011 之后(scratch copy `git apply --check` 校验,不污染 `external/godot-master`;本切片**不**做 scratch 全栈重建)。
- `modules/rurix_accel/register_types.cpp`:新增默认 `false` 的 `rendering/rurix_accel/passes/ssao_blur/enabled`。
- `modules/rurix_accel/rurix_accel.{h,cpp}`:`#define RXGD_PASS_SSAO_BLUR 2u`;`try_record_ssao_blur()`(0002 模式:设置关 / session 缺 / 非 OK 一律 `false`,首个 fallback 打印一次 `RurixAccel: ssao_blur fallback rc=` verbose marker)。
- `drivers/d3d12/d3d12_hooks.h`:基类新增默认返回 `false` 的 `virtual bool try_record_ssao_blur()`。
- `servers/rendering/renderer_rd/effects/ss_effects.cpp` `generate_ssao` Edge-Aware Blur 段:blur 循环前加 opt-in gate;gate 返回 `false`(实测恒 false:bridge 恒 fallback、设置默认 `false`)时原生 blur 循环照常执行。SSIL blur 段不接线。
- 0012 级 module 调用不带 resource binding(0002 级),bridge preflight 按构造以 `validation_failed` fallback;native handle / 资源绑定接线属后续段(对应 GRX-009 0005/0007 的位置)。

## 9. Evidence 要求

- **offline compile**(本切片 measured):`offline_compile_evidence.json`——DXC cs_6_0 编译、DXV 验证、Rurix-owned RTS0(`emit_grx011_ssao_blur_rts0` example 经 `rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}`)、descriptor layout(binding_kind per slot + 28-byte root constants)、三 artifact SHA-256 可追溯;`provenance=hlsl_bridge_workaround`、`rurix_owned=false`、`runtime_mappable=true`(owner provenance 政策)。
- **standalone real D3D12 dispatch smoke**(本切片 measured 上限):`ci/grx011_ssao_blur_d3d12_dispatch_smoke.py` → `real_d3d12_dispatch_smoke.json`——真实 device/queue、RTS0 直建 root signature、tracked DXIL 建 compute PSO(内存副本经签名 DXC `dxil.dll` in-place 签名)、SRV t0 / UAV u0 / b0 严格按 descriptor layout 绑定、一次 dispatch + fence + readback,且 readback 全部 texel 与 CPU reference(edge-aware smart blur)逐分量在容差内一致。SKIP(无 adapter / 缺 MSVC / 缺签名 DXC)不推进 ready。
- **math parity**:`math_parity_evidence.json`(`generate_math_parity_evidence.py`)——确定性合成输入的 CPU float32 reference(逐操作 binary32 舍入),`status=pending_gpu_dispatch`;GPU 侧观察由后续 real dispatch 段补齐。
- **引擎内 visual diff / measured fallback telemetry / real pass enablement**(4f/4g/4h 级别):留后续切片;本文件不宣称任何此类证据已取得。
- Perf:复用 GRX-006 baseline / perf gate;在产出实测证据前,不得声称任何性能提升。

## 10. 出口判据

- pass 默认 `disabled`;manifest `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`。
- 未通过 compile / validation / visual / perf 门禁前,保持 `disabled` 并 fallback 到 Godot 原生 SSAO blur 路径。
- 本切片(contract + offline kernel + bridge gate + patch 0012 + standalone dispatch smoke)**不**代表 pass 完成;后续段必须交付 Godot runtime 资源绑定(native handle)、runtime bridge dispatch recording、引擎内 visual diff + measured fallback telemetry、gated real-pass enablement 与 strict evidence(复用 GRX-009 4a..4h 段模板)。

## 11. 仍未完成项

- Godot runtime 资源绑定接线(0005/0007 级别:hook 传参 + native `ID3D12Resource*` 解析,含 4-slice/ping-pong 调度设计)。
- Godot runtime bridge dispatch recording smoke(4f 级别,需 full patch stack scratch 重建)。
- 引擎内 real visual diff + measured fallback telemetry gate(4g 级别;含 temporal/noise 稳定性 evidence)。
- gated real-pass enablement gate 与 measured success(4h 级别)。
- `MODE_WIDE` / `MODE_NON_SMART` 变体、多 pass ping-pong 链、4-slice 循环、SSIL blur(`RXGD_PASS_SSIL_BLUR`)、mirror-sampler 边界、rg8 unorm 量化。
- full baseline / per-pass FPS 对比数据;任何性能提升声明。

## 12. Close-out（GRX-011 stage-A5 对等）

> 本节为 close-out 追加段;§1–§11 的调查/契约/marker 保持不变(pass_id = ssao_blur、`RXGD_PASS_SSAO_BLUR`、`MODE_SMART`、`RXGD_CAP_SSAO_BLUR_REAL_PASS` 等契约字面不动),§5/§11 known gaps 不变。**patch 编号更正**:§8 起草时按占位写作 "patch 0012",实际入库栈号为 `0014`(pass-gate + call-site)/ `0015`(runtime resource binding)/ `0016`(recording smoke + real-pass opt-in),以 `spike/godot-rurix/patches/PATCH_ALLOCATION.md` 与 `pass_manifest.json` 为准;call-site gate、per-pass 设置默认 false、`try_record_ssao_blur()`、`RXGD_SSAO_BLUR_REAL_PASS_BLOCKED` 诊断等语义与 §7/§8 一致。

GRX-011 ssao_blur 已 close-out(复用 GRX-009/GRX-010 4a..4h 成熟模板)。在 §8 pass-gate patch 之后补齐两段栈式 patch:

- **runtime resource binding — patch 0015(`0015-rurix-accel-ssao-blur-runtime-resource-binding.patch`)**:把 Godot runtime SSAO blur call site 传给 bridge 的资源从 logical id 改成真实 `ID3D12Resource*` native handle(`ss_effects.cpp` `generate_ssao` Edge-Aware Blur 段经 `RenderingDevice::get_driver_resource(DRIVER_RESOURCE_TEXTURE, RID, 0)` 解析 deinterleaved slice source / ping-pong dest 真实句柄,句柄为 0 或 `RenderingDevice` 不可用时 fallback 到原生 blur 循环;fallback marker `RurixAccel: ssao_blur native resource handle mapping fallback rc=`)。
- **recording smoke + real-pass opt-in — patch 0016(`0016-rurix-accel-ssao-blur-recording-smoke-and-real-pass-optin.patch`)**:新增默认 `false` 的 `rendering/rurix_accel/passes/ssao_blur/{dispatch_real_pass,dispatch_recording_smoke,real_pass_force_capability_downgrade}` opt-in、`RXGD_CAP_SSAO_BLUR_REAL_PASS`(1u<<5,复用 `RxGdCaps.flags`,`RXGD_ABI_VERSION` 保持 1)、`RXGD_GODOT_RUNTIME_SSAO_BLUR_REAL_PASS` marker 与 `d3d12-recording-shim` 下的 real dispatch path;patch 0016 result writeback 为 **SCAFFOLD**(native Godot SSAO blur 仍作 continuation/backstop,每帧照常重跑完整 `blur_passes × 4 slices` ping-pong 链)。

**Enablement measured success 事实**:`ci/grx011_ssao_blur_real_pass_enablement_smoke.py` 在 0001..0016 scratch Godot(Windows D3D12 Forward+,NVIDIA GeForce RTX 4070 Ti)上记录 strict MEASURED success(`real_pass_enablement_success_evidence.json`,`status=success`、`strict_success=true`):opt-in real-pass 腿(`enabled`+`dispatch_real_pass`,均默认 false)真正执行且完成——candidate 腿 `real_pass_marker_observed=true` + `writeback_marker_observed=true`(`RXGD_GODOT_RUNTIME_SSAO_BLUR_REAL_PASS recorded=1`)入证,checks 全绿含 `native_continuation_writeback_scaffold`/`real_pass_dispatched_and_completed`,`forced_capability_downgrade` 红腿实测 `first_missing_prerequisite=runtime_binding_preflight_failed`/`fallback_reason=unsupported_device`,LDR visual gate `max_abs=0`/`mean_abs=0`(reference/candidate/forced 三帧在 pinned 阈值内),measured_local telemetry 通过 GRX-008 校验,`0001..0016` patch-stack/溯源/日志审计全绿,DLL 指纹记 `features=[d3d12-recording-shim]`。standalone dispatch smoke(`real_d3d12_dispatch_smoke.json`)CPU parity `max_abs_diff≈1.19e-07`(~1 ULP)、`mismatched=0`。manifest 顶层如实翻转 `implemented=true`、`real_gpu_pass=true`(opt-in 实测口径)、`real_d3d12_dispatch_recorded=true`、`runtime_state=fallback_only_by_default_real_pass_optin_measured`;`default_enable_state` 保持 `disabled`,新增 `implementation_status.real_pass_measured_success` block。

**Owner default-enable decision 引用**:`real_pass_default_enable_decision.json` / `real_pass_default_enable_decision.md` 记 `keep_default_disabled`,理由:①无 per-pass FPS 证据(契约要求 per-pass FPS >= 0.95x baseline);②patch 0016 writeback 仍 scaffold(native continuation 全量重跑,无净收益,candidate 图像 bit-exact);③仅 `MODE_SMART` 单遍单 slice 子集(`MODE_WIDE`/`MODE_NON_SMART`/多 pass ping-pong/4-slice 循环/SSIL 未覆盖);full baseline + per-pass benchmark 后由 owner 复评。

**Fail-closed 不变**:默认 Godot config 下 bridge 对 `RXGD_PASS_SSAO_BLUR` 仍返回 `RXGD_STATUS_FALLBACK`、native SSAO blur 接管;shipping feature-off bridge 仍 fail closed 为 `real_dispatch_path_not_linked`;`RXGD_PASS_SSIL_BLUR` 仍不接线。gate 模块 `ci/grx_gates/grx011_ssao_blur.py` decision/enablement 双 ready(决策文件顶层 `default_enable_decision` 非空 + `strict_success=true`)后 probe `next_action=start_grx012_taa_resolve_pass_contract`;任一缺失/被篡改即 `grx_gate_module_error`、`next_action` 保持不变。**§5/§11 known gaps 全部保留不变**。无 FPS、p95、GPU timestamp 或任何性能提升宣称。
