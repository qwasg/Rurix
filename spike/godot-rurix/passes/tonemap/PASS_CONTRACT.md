# GRX-010 Tonemap Pass — PASS CONTRACT

> **状态声明（2026-07-08，segment A：pass contract + offline kernel + bridge gate + standalone dispatch smoke）。**
> 本切片完全复用 GRX-009 luminance_reduction 已打通的成熟模板：pass 契约三件套、HLSL bridge 数学等价 kernel（DXC 编译 + DXV 验证 + Rurix-owned RTS0 合成，owner-approved `hlsl_bridge_workaround` provenance，政策 `../luminance_reduction/texture_artifact_provenance_policy.json` 适用于所有 texture compute pass）、bridge `TonemapGate`（preflight → eligibility → binding-kind → math-parity → real-dispatch 链，默认恒 `RXGD_STATUS_FALLBACK`）、Godot patch 0011（per-pass 设置默认 `false` + call-site opt-in gate，原生 tonemap 路径始终保留 fallback）、standalone real D3D12 dispatch smoke。
> 本切片**不**做 Godot scratch 全栈重建与引擎内实测（4f/4h 级别留后续切片）、**不**做引擎内 visual diff、**不**启用默认 pass、**不**宣称任何 FPS / GPU timestamp / 性能提升。measured 上限为 standalone dispatch + CPU parity。
> pass 默认 `disabled`；任何 compile / validation / visual / perf 失败都走 Godot 原生 tonemap 路径。
> §3 对 `external/godot-master` 的调查只记录路径与函数名；Godot 侧改动只以 `spike/godot-rurix/patches/` 下的 patch 文件入库，不直接修改快照的 Godot 原生源文件。

## 1. Pass 标识

- `pass_id = tonemap`
- bridge pass id：`RXGD_PASS_TONEMAP = 5`（`src/rurix-godot/src/lib.rs` 既有预留）
- Tier：Tier 1（低风险 pass 候选，GRX-009 之后第二个）
- 目标后端：`Godot 4.7-dev Windows D3D12 Forward+`
- 默认启用状态：`disabled`

## 2. 目标场景

- `post_fx_chain`
- `mixed_forward_plus`

（对齐 GRX_PLAN GRX-010 任务行：先覆盖 post_fx_chain 和 mixed_forward_plus。）

## 3. Godot 侧候选 hook / call site / 资源流调查结果

仅记录路径与函数，**不改 `external/godot-master`**。

### 3.1 Effect 类

- 头文件：`servers/rendering/renderer_rd/effects/tone_mapper.h`
- 源文件：`servers/rendering/renderer_rd/effects/tone_mapper.cpp`
- 关键函数 / 类型：
  - `RendererRD::ToneMapper::tonemapper(RID p_source_color, RID p_dst_framebuffer, const TonemapSettings &p_settings)`（`tone_mapper.cpp:117`）——Forward+（`can_use_storage`）路径入口，**raster fullscreen fragment pass**（写 framebuffer，非 compute dispatch）。
  - `RendererRD::ToneMapper::tonemapper_mobile(...)` / `tonemapper_subpass(...)`——mobile / subpass 变体，本 pass 不覆盖。
  - `ToneMapper::TonemapSettings`（`tone_mapper.h:166`）——全部 tonemap 参数：`tonemap_mode`、`exposure`、`white`、`max_value`、`luminance_multiplier`、auto exposure、glow、BCS、color correction、FXAA、debanding、`convert_to_srgb` 等。
  - `ToneMapper::TonemapPushConstant`（`tone_mapper.h:104`，112 bytes）——原生 push constant 布局；本切片的 Rurix b0 只承载其 `exposure` / `white` / `luminance_multiplier` 子集。

### 3.2 Shader

- `servers/rendering/renderer_rd/shaders/effects/tonemap.glsl`（Forward+ raster fragment 路径；本 kernel 的数学对标源）
  - tonemapper 模式：`TONEMAPPER_LINEAR 0` / `REINHARD 1` / `FILMIC 2` / `ACES 3` / `AGX 4`（L240-244）；`apply_tonemapping` L246-264（LINEAR = identity）。
  - `linear_to_srgb` L230-233。
  - fragment `main()` L854-955：`luminance_multiplier`（L860）→ exposure（L864/870，可选 auto exposure L866-868）→ FXAA → pre-tonemap glow → `apply_tonemapping`（L893）→ post-tonemap softlight glow → BCS/color correction → `FLAG_CONVERT_TO_SRGB` 的 `linear_to_srgb`（L942-943）→ debanding。
- `servers/rendering/renderer_rd/shaders/effects/tonemap_mobile.glsl`（mobile 变体，不覆盖）。

### 3.3 调用 / 注入候选点

- `servers/rendering/renderer_rd/renderer_scene_render_rd.cpp` 的 `_render_buffers_post_process_and_tonemap`（`L459`）"Tonemap" 段：
  - `RENDER_TIMESTAMP("Tonemap")` + `draw_command_begin_label("Tonemap")`（`L690-691`）。
  - `TonemapSettings tonemap` 组装 `L693-823`（exposure texture / glow / tonemap_mode / white / exposure / BCS / color correction / luminance_multiplier / dest framebuffer / debanding）。
  - **注入点**：`tone_mapper->tonemapper(color_texture, dest_fb, tonemap)`（`L826`，`can_use_storage` 腿）。patch 0011 在该调用前插入 opt-in gate：只有 `D3D12Hooks::get_singleton()->try_record_tonemap()` 返回 `true`（module 设置开启 且 bridge `rxgd_record_pass(RXGD_PASS_TONEMAP)` 返回 `RXGD_STATUS_OK`）时才跳过原生 tonemapper；否则必须执行原生调用。`tonemapper_mobile` 腿（`L828`）不接线。
  - source：`color_texture = use_upscaled_texture ? rb->get_upscaled_texture() : rb->get_internal_texture()`（`L492`）。
  - dest：`dest_fb`（render target framebuffer 或 spatial upscaler/SMAA 的中间 texture framebuffer，`L786-807`）。

### 3.4 资源流（原生）

- 输入：HDR linear scene color（`color_texture`，`R16G16B16A16_(TYPELESS/FLOAT)` 家族或 RGB10A2）。
- 辅助输入（本切片不映射）：`source_auto_exposure`（1x1 luminance，GRX-009 的输出）、glow mips、color correction LUT、glow map。
- 输出：LDR framebuffer（`dest_fb`；SDR 时 `convert_to_srgb=true`）。
- 原生 pass 形态为 **raster fullscreen triangle fragment**；Rurix bridge 模板为 compute。本切片的 kernel 以 full-res `RWTexture2D<float4>` UAV 输出等价数学结果，raster-vs-compute 输出接缝（真实替换需要 UAV→render target 交接或 compute 写 swapchain-compatible target）记录为 known gap，属于后续 runtime 段。

## 4. 输入 / 输出资源（Rurix mapping）

- 输入：`src_color = Texture2D<float4>`，SRV `t0 space0`，`binding_kind = texture2d`（Godot `color_texture` 的 native `ID3D12Resource*`）。
- 输出：`dst_color = RWTexture2D<float4>`，UAV `u0 space0`，`binding_kind = rwtexture2d`（full-res LDR 输出；dst 尺寸 == src 尺寸，1:1）。
- b0 root constants（28 bytes / 7 dwords，root_parameter_index 0，复用 GRX-009 canonical 打包形状 `[i64, i64, f32, f32, f32]`）：
  - `source_width`（i64，dword 0-1）/ `source_height`（i64，dword 2-3）
  - `exposure`（f32，dword 4）
  - `white`（f32，dword 5；LINEAR 模式不消费，为与 Godot `TonemapPushConstant` 字段对齐而保留）
  - `luminance_multiplier`（f32，dword 6）
- tracked mapping：`resource_mapping.md`。

## 5. 支持范围与 gaps（起步口径）

- **支持**：`TONEMAPPER_LINEAR`（Godot 默认 `ENV_TONE_MAPPER_LINEAR`）+ `FLAG_CONVERT_TO_SRGB`（SDR 默认路径）+ `luminance_multiplier` / `exposure` 标量核心，alpha 透传。
- **不支持（known gaps，manifest `known_gaps` 逐条入账）**：Reinhard / Filmic / ACES / AgX tonemappers、auto exposure（`source_auto_exposure` 纹理腿）、glow（pre/post/softlight/bicubic）、FXAA、BCS、color correction（1D/3D LUT）、debanding、multiview、HDR 输出（`convert_to_srgb=false`）、8-bit 量化 clamp（原生由 framebuffer 量化完成；本 kernel 输出未 clamp 的 float sRGB 值）、raster-vs-compute 输出接缝。

## 6. Fallback

- fallback reason 枚举（对齐 GRX-008 五枚举）：`compile_failed` / `validation_failed` / `unsupported_device` / `visual_diff_failed` / `manual_disabled`。
- 任一 compile / validation / visual / perf 失败 → 回退到 Godot 原生 tonemap 路径（`godot_native_tonemap`）。
- 默认 Godot config（per-pass 设置全部 `false`）与 shipping bridge 对 `RXGD_PASS_TONEMAP` 恒返回 `RXGD_STATUS_FALLBACK`，原生 tonemapper 始终执行。

## 7. Bridge gate（`src/rurix-godot/src/lib.rs` `TonemapGate`）

模板复制 GRX-009 `LuminanceReductionGate` 的检查链，常量与 digest 指向 tonemap 产物：

1. **runtime binding preflight**：64-bit integer capability flag（b0 承载 i64 dims 的既有模板口径）、恰好 2 个 texture 资源（src 在前 dst 在后）、28-byte push constants、b0 中 source dims 非零且与 `src_color` 资源一致、`dst_color` 尺寸 == source 尺寸（1:1 full-res 形状）。
2. **dispatch eligibility**：opt-in capability flag `RXGD_CAP_TONEMAP_REAL_PASS (1u << 4)`（复用 `RxGdCaps.flags` 位，**不改 C ABI struct layout，`RXGD_ABI_VERSION` 保持 1**；缺失 → `manual_disabled`）、64-bit integer capability、native D3D12 device/queue 非空、resource native handle 非空、compiled package layout/digest 与 offline evidence 三个 SHA-256 逐字节匹配。
3. **kernel-binding-kind conformance**：`TONEMAP_KERNEL_RESOURCE_BINDING_KINDS = ["texture2d", "rwtexture2d"]` per-slot 校验；buffer 资源 fail closed。
4. **math parity gate**：`TONEMAP_KERNEL_MATH_PARITY_STATUS = "linear_srgb_cpu_reference_proven_pending_gpu_dispatch"`（`math_parity_evidence.json`：CPU reference 已证，GPU 侧观察 pending real dispatch）。
5. **real dispatch**：仅在 `d3d12-recording-shim` feature（默认关闭）下经参数化的通用 texture-pass 录制 shim（`shim/rxgd_luminance_record.cpp` 的 `rxgd_luminance_record_dispatch`，SRV t0 + UAV u0 + 28-byte b0 + `ceil(dims/8)` dispatch 形状对 tonemap 逐项吻合，view format 由真实资源 format 推导）录制；shipping feature-off bridge fail closed 为 `real_dispatch_path_not_linked`。
- 任一失败返回 `RXGD_STATUS_FALLBACK`、记录 fallback reason、每 session 一次打印机读诊断 `RXGD_TONEMAP_REAL_PASS_BLOCKED first_missing_prerequisite=... fallback_reason=... kernel_binding=texture2d default_enable_state=disabled`（非 `ERROR:` 行、不含 `RXGD_DIAG`），不累计 estimated GPU/CPU time。
- **行为变更入账**：`RXGD_PASS_TONEMAP` 在本段前走 bridge 的占位 estimated-timing 路径（record OK + 伪 estimated GPU time）；自本段起改走 `TonemapGate` fail-closed 链，默认恒 fallback、不再产生任何 estimated tonemap GPU time。

## 8. Godot patch 0011

- `spike/godot-rurix/patches/0011-rurix-accel-tonemap-pass-gate-and-callsite.patch`，栈式叠加在 0001..0010 之后（scratch copy `git apply --check` 校验，不污染 `external/godot-master`；本切片**不**做 scratch 全栈重建）。
- `modules/rurix_accel/register_types.cpp`：新增默认 `false` 的 `rendering/rurix_accel/passes/tonemap/enabled`。
- `modules/rurix_accel/rurix_accel.{h,cpp}`：`#define RXGD_PASS_TONEMAP 5u`；`try_record_tonemap()`（0002 模式：设置关 / session 缺 / 非 OK 一律 `false`，首个 fallback 打印一次 `RurixAccel: tonemap fallback rc=` verbose marker）。
- `drivers/d3d12/d3d12_hooks.h`：基类新增默认返回 `false` 的 `virtual bool try_record_tonemap()`。
- `servers/rendering/renderer_rd/renderer_scene_render_rd.cpp` Tonemap 段：`can_use_storage` 腿在原生 `tone_mapper->tonemapper(...)` 前加 opt-in gate；gate 返回 `false`（实测恒 false：bridge 恒 fallback、设置默认 `false`）时原生调用照常执行。
- 0011 级 module 调用不带 resource binding（0002 级），bridge preflight 按构造以 `validation_failed` fallback；native handle / 资源绑定接线属后续段（对应 GRX-009 0005/0007 的位置）。

## 9. Evidence 要求

- **offline compile**（本切片 measured）：`offline_compile_evidence.json`——DXC cs_6_0 编译、DXV 验证、Rurix-owned RTS0（`emit_grx010_tonemap_rts0` example 经 `rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}`）、descriptor layout（binding_kind per slot + 28-byte root constants）、三 artifact SHA-256 可追溯；`provenance=hlsl_bridge_workaround`、`rurix_owned=false`、`runtime_mappable=true`（owner provenance 政策）。
- **standalone real D3D12 dispatch smoke**（本切片 measured 上限）：`ci/grx010_tonemap_d3d12_dispatch_smoke.py` → `real_d3d12_dispatch_smoke.json`——真实 device/queue、RTS0 直建 root signature、tracked DXIL 建 compute PSO（内存副本经签名 DXC `dxil.dll` in-place 签名）、SRV t0 / UAV u0 / b0 严格按 descriptor layout 绑定、一次 dispatch + fence + readback，且 readback 首像素与 CPU reference（`linear_to_srgb(src * luminance_multiplier * exposure)`）在容差内一致。SKIP（无 adapter / 缺 MSVC / 缺签名 DXC）不推进 ready。
- **math parity**：`math_parity_evidence.json`（`generate_math_parity_evidence.py`）——确定性合成输入的 CPU float32 reference（逐操作 binary32 舍入），`status=pending_gpu_dispatch`；GPU 侧观察由后续 real dispatch 段补齐。
- **引擎内 visual diff / measured fallback telemetry / real pass enablement**（4f/4g/4h 级别）：留后续切片；本文件不宣称任何此类证据已取得。
- Perf：复用 GRX-006 baseline / perf gate；在产出实测证据前，不得声称任何性能提升。

## 10. 出口判据

- pass 默认 `disabled`；manifest `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`。
- 未通过 compile / validation / visual / perf 门禁前，保持 `disabled` 并 fallback 到 Godot 原生 tonemap 路径。
- 本切片（contract + offline kernel + bridge gate + patch 0011 + standalone dispatch smoke）**不**代表 pass 完成；后续段必须交付 Godot runtime 资源绑定（native handle）、runtime bridge dispatch recording、引擎内 visual diff + measured fallback telemetry、gated real-pass enablement 与 strict evidence（复用 GRX-009 4a..4h 段模板）。

## 11. 仍未完成项

- Godot runtime 资源绑定接线（0005/0007 级别：hook 传参 + native `ID3D12Resource*` 解析）。
- Godot runtime bridge dispatch recording smoke（4f 级别，需 full patch stack scratch 重建）。
- 引擎内 real visual diff + measured fallback telemetry gate（4g 级别）。
- gated real-pass enablement gate 与 measured success（4h 级别）。
- 其余 tonemapper 模式（Reinhard/Filmic/ACES/AgX）、auto exposure、glow、FXAA、BCS、color correction、debanding、HDR 输出、raster-vs-compute 输出接缝。
- full baseline / per-pass FPS 对比数据；任何性能提升声明。

## 12. Close-out（GRX-010 stage-A5 对等）

> 本节为 close-out 追加段;§1–§11 的调查/契约/marker 保持不变(pass_id = tonemap、RXGD_PASS_TONEMAP、TONEMAPPER_LINEAR、RXGD_CAP_TONEMAP_REAL_PASS 等契约字面不动),§11 known gaps 不变。

GRX-010 tonemap 已 close-out(复用 GRX-009 4a..4h 成熟模板)。在 §8 patch 0011 之后补齐两段栈式 patch:

- **segment B — patch 0012(runtime resource binding)**:`0012-rurix-accel-tonemap-runtime-resource-binding.patch`(栈式叠 0001..0011,scratch copy `git apply --check` 通过)把 Godot runtime tonemap call site 传给 bridge 的资源从 logical id 改成真实 `ID3D12Resource*` native handle——`renderer_scene_render_rd.cpp` Tonemap 段用 `RenderingDevice::get_driver_resource(DRIVER_RESOURCE_TEXTURE, RID, 0)` 解析 source/dest 真实句柄,句柄为 0 或 `RenderingDevice` 不可用时 fallback 到原生 tonemapper;fallback marker 升级为 `RurixAccel: tonemap native resource handle mapping fallback rc=`。
- **segment C — patch 0013(recording smoke + real-pass opt-in)**:`0013-rurix-accel-tonemap-recording-smoke-and-real-pass-optin.patch`(栈式叠 0001..0012)新增默认 `false` 的 `rendering/rurix_accel/passes/tonemap/{dispatch_real_pass,dispatch_recording_smoke,real_pass_force_capability_downgrade}` opt-in、`RXGD_CAP_TONEMAP_REAL_PASS`(1u<<4,复用 `RxGdCaps.flags`,`RXGD_ABI_VERSION` 保持 1)、`RXGD_GODOT_RUNTIME_TONEMAP_RECORD` marker 与 `d3d12-recording-shim` 下的 real dispatch path;patch 0013 result writeback 为 **SCAFFOLD**(native Godot tonemapper 仍作 continuation/backstop 重渲染每帧)。

**Enablement measured success 事实**:`ci/grx010_tonemap_real_pass_enablement_smoke.py` 在 0001..0013 scratch Godot(Windows D3D12 Forward+)上记录 strict MEASURED success(`real_pass_enablement_success_evidence.json`,`status=success`):opt-in real-pass 腿(`enabled`+`dispatch_real_pass`,均默认 false)真正执行且完成——`RXGD_GODOT_RUNTIME_TONEMAP_REAL_PASS` marker + patch 0013 writeback scaffold marker 入证,22 checks 全绿含 `forced_capability_downgrade` 红腿实测 `unsupported_device`,LDR visual gate `max_abs=0`/`mean_abs=0`(reference/candidate/forced 三帧在 pinned 阈值内),measured_local telemetry 通过 GRX-008 校验,`0001..0013` patch-stack/溯源/日志审计全绿,DLL 指纹记 `features=[d3d12-recording-shim]`(唯一带 linked real dispatch path 的构建)。manifest 顶层如实翻转 `implemented=true`、`real_gpu_pass=true`(opt-in 实测口径)、`real_d3d12_dispatch_recorded=true`、`runtime_state=fallback_only_by_default_real_pass_optin_measured`;`default_enable_state` 保持 `disabled`。

**Owner default-enable decision 引用**:`real_pass_default_enable_decision.json` / `real_pass_default_enable_decision.md` 记 `keep_default_disabled`,理由:无 per-pass FPS 证据(契约要求 per-pass FPS >= 0.95x baseline)、仅 `TONEMAPPER_LINEAR` + sRGB 子集、patch 0013 writeback 仍 scaffold + raster-vs-compute output seam 未设计;full baseline + per-pass benchmark 后由 owner 复评。

**Fail-closed 不变**:默认 Godot config 下 bridge 对 `RXGD_PASS_TONEMAP` 仍返回 `RXGD_STATUS_FALLBACK`、native tonemapper 接管;shipping feature-off bridge 仍 fail closed 为 `real_dispatch_path_not_linked`。probe manifest 检查 fail-closed 放宽:仅当 strict success 存在且全量审计通过(`grx010_real_pass_measured_success_active`)才接受翻转后的新值,placeholder/篡改 success 文档报 `grx010_real_pass_success_evidence_conflict` 回落旧值。**§11 known gaps 全部保留不变**(其余 tonemapper 模式 / auto exposure / glow / HDR / raster-vs-compute seam / per-pass FPS 门)。probe enablement + 决策双 ready 后 `next_action=start_grx011_ssao_blur_godot_patch_0014`。无 FPS、p95、GPU timestamp 或任何性能提升宣称。
