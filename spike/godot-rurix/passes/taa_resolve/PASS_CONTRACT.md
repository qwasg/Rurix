# GRX-012 TAA Resolve Pass — PASS CONTRACT

> **状态声明(2026-07-12,segment A:pass contract + offline kernel + math parity + bridge gate + standalone dispatch smoke,即 PASS_TEMPLATE 的 S1-S4+S6,不含 Godot patch/enablement/close-out)。**
> 本切片完全复用 GRX-011 ssao_blur 已打通的成熟模板:pass 契约三件套、HLSL bridge 数学等价 kernel(DXC 编译 + DXV 验证 + Rurix-owned RTS0 合成,owner-approved `hlsl_bridge_workaround` provenance,政策 `../luminance_reduction/texture_artifact_provenance_policy.json` 适用于所有 texture compute pass)、bridge `TaaResolveGate`(preflight → eligibility → binding-kind → math-parity → real-dispatch 链,默认恒 `RXGD_STATUS_FALLBACK`)、standalone real D3D12 dispatch smoke。
> 本切片**不**做 Godot patch(0017-0019 归后续串行切片)、**不**做 Godot scratch 全栈重建与引擎内实测、**不**做引擎内 visual diff、**不**启用默认 pass、**不**宣称任何 FPS / GPU timestamp / 性能提升。measured 上限为 standalone dispatch + CPU parity。
> pass 默认 `disabled`;任何 compile / validation / visual / perf 失败都走 Godot 原生 TAA resolve 路径。
> §3 对 `external/godot-master` 的调查只记录路径与函数名;Godot 侧改动只以 `spike/godot-rurix/patches/` 下的 patch 文件入库,不直接修改快照的 Godot 原生源文件。

## 1. Pass 标识

- `pass_id = taa_resolve`
- bridge pass id:`RXGD_PASS_TAA_RESOLVE = 6`(`src/rurix-godot/src/lib.rs` 既有预留)
- cap 位:`RXGD_CAP_TAA_RESOLVE_REAL_PASS = 1 << 6`(`PATCH_ALLOCATION.md` §3 预分配,复用 `RxGdCaps.flags`,不改 C ABI struct layout,`RXGD_ABI_VERSION` 保持 1)
- Tier:Tier 1(screen-space post 效果,GRX-009/010/011 之后第四个)
- 目标后端:`Godot 4.7-dev Windows D3D12 Forward+`
- 默认启用状态:`disabled`

## 2. 目标场景

- `mixed_forward_plus`
- `clustered_lights`

(TAA 是 temporal 累积效果,对齐 GRX_PLAN GRX-012 的 temporal/noise 稳定性注意事项:先覆盖含几何/光照复杂度的两个场景;temporal 稳定性 evidence 属后续 visual diff 段。)

## 3. Godot 侧候选 hook / call site / 资源流调查结果

仅记录路径与函数,**不改 `external/godot-master`**。

### 3.1 Effect 类

- 头文件:`servers/rendering/renderer_rd/effects/taa.h`
- 源文件:`servers/rendering/renderer_rd/effects/taa.cpp`
- 关键函数 / 类型:
  - `RendererRD::TAA::resolve(...)`(`taa.cpp:51-88`)——纯 compute resolve(`local_size 8x8x1`,`compute_list_dispatch_threads(p_resolution.x, p_resolution.y, 1)`,`L82`);push constant 组装 `L67-73`。
  - `RendererRD::TAA::process(...)`(`taa.cpp:90+`)——上层入口(resolve 到 temp → history 维护三次物理 `copy_to_rect`)。
  - `TAA::TAAResolvePushConstant`(`taa.h`,16 bytes:`resolution[2]`、`disocclusion_threshold`、`variance_dynamic`)。
- pipeline:单 `resolve` compute pipeline(无变体)。

### 3.2 Shader

- `servers/rendering/renderer_rd/shaders/effects/taa_resolve.glsl`(本 kernel 的数学对标源;Spartan Engine 派生,compute,`local_size 8x8x1`):
  - 绑定六件(`L46-51`):`color_buffer`(rgba16f image r,binding 0)/ `depth_buffer`(sampler2D,binding 1)/ `velocity_buffer`(rg16f image r,binding 2)/ `last_velocity_buffer`(rg16f image r,binding 3)/ `history_buffer`(sampler2D,binding 4)/ `output_buffer`(rgba16f image w,binding 5)。
  - push constant(`L53-58`):`vec2 resolution` / `float disocclusion_threshold`(= `0.1 / max(res.x, res.y)`)/ `float variance_dynamic`。
  - groupshared tile(`L92-93`):`tile_color[10][10]` / `tile_depth[10][10]`(8x8 group + 1 border),`populate_group_shared_memory`(`L121-139`)每帧 25 线程各载 4 texel,边界 clamp。
  - 数学核心:`get_closest_pixel_velocity_3x3`(`L155-171`,3x3 最近深度速度盘查)、`sample_catmull_rom_9`(`L177-228`,9-tap Catmull-Rom history 采样,textureLod 双线性)、`clip_aabb` + `clip_history_3x3`(`L235-292`,方差裁剪盒)、`get_factor_disocclusion`(`L306-312`,速度差 disocclusion 因子)、`temporal_antialiasing`(`L314-368`,Reinhard 域 blend + inverse Reinhard,base blend `RPC_16 = 1/16`,亮度差抑闪)。
  - `main()`(`L370-385`):`imageStore(output_buffer, gid.xy, vec4(result, 1.0))`。

### 3.3 调用 / 注入候选点

- `servers/rendering/renderer_rd/forward_clustered/render_forward_clustered.cpp` `_render_scene`(`L2512` 附近)→ `taa->process(...)`。
- **注入点**:`TAA::resolve` compute dispatch 前插入 opt-in gate:只有 `D3D12Hooks::get_singleton()->try_record_taa_resolve()` 返回 `true`(module 设置开启 且 bridge `rxgd_record_pass(RXGD_PASS_TAA_RESOLVE)` 返回 `RXGD_STATUS_OK`)时才跳过原生 resolve dispatch;否则原生 resolve 照常执行。history 三次物理 `copy_to_rect` 维护段属 native continuation,本切片不接线。

### 3.4 资源流(原生)

- 输入:`color`(当前帧 HDR,rgba16f)、`depth`(sampler)、`velocity`(rg16f)、`prev_velocity`(rg16f,上一帧)、`history`(sampler,上一帧 resolve 结果)。
- 输出:`temp`(rgba16f;随后 `resolve → temp → internal → history` 三次物理 copy)。
- 尺寸:全分辨率(`p_resolution`);resolve 是 1:1 pass。
- **一帧延迟物理约束(侦察 Q3 终裁)**:从自队列 dispatch 读到上一帧内容;TAA 对同帧 `color`/`velocity` 是语义硬依赖(见下 §5/§10)。本切片交付 **GRX-011 同段位**:opt-in measured real dispatch + native continuation(scaffold writeback,不替代图像);kernel 语义仍按 native 忠实实现(为将来 `draw_graph` 集成铺路),真替代需 `draw_graph` 集成(deferred)。

## 4. 输入 / 输出资源(Rurix mapping)

六资源布局(单一 descriptor table:SRV range t0..t4 先于 UAV range u0,对齐 rurixc `infer_root_signature` §9 Q-RootShape=B):

| slot | Rurix 名 | binding | kind | HLSL 类型 | Godot 源 |
| --- | --- | --- | --- | --- | --- |
| t0 | `color_buffer` | SRV t0 space0 | texture2d | `Texture2D<float4>` | 当前帧 HDR color(rgba16f) |
| t1 | `depth_buffer` | SRV t1 space0 | texture2d | `Texture2D<float>` | depth(sampler2D → Load) |
| t2 | `velocity_buffer` | SRV t2 space0 | texture2d | `Texture2D<float2>` | velocity(rg16f) |
| t3 | `last_velocity_buffer` | SRV t3 space0 | texture2d | `Texture2D<float2>` | 上一帧 velocity(rg16f) |
| t4 | `history_buffer` | SRV t4 space0 | texture2d | `Texture2D<float4>` | 上一帧 resolve history(sampler2D → 显式双线性 Load) |
| u0 | `output_buffer` | UAV u0 space0 | rwtexture2d | `RWTexture2D<float4>` | resolve 输出(rgba16f;dst 尺寸 == color 尺寸,1:1) |

- b0 root constants(28 bytes / 7 dwords,root_parameter_index 0,复用 GRX-009/010/011 canonical 打包形状 `[i64, i64, f32, f32, f32]`):
  - `source_width`(i64,dword 0-1)/ `source_height`(i64,dword 2-3)——resolution 尺寸;kernel `resolution = float2(source_width, source_height)`。
  - `disocclusion_threshold`(f32,dword 4)= Godot `0.1 / max(res.x, res.y)`。
  - `variance_dynamic`(f32,dword 5)= Godot `params.variance_dynamic`。
  - `reserved0`(f32,dword 6;补齐 canonical 7-dword 形状,kernel 不消费,runtime 写 0)。
- tracked mapping:`resource_mapping.md`。

## 5. 支持范围与 gaps(起步口径)

- **支持**:单次 resolve pass 全分辨率——groupshared 10x10 tile(color+depth,边界 clamp)、`get_closest_pixel_velocity_3x3` 3x3 最近深度速度盘查、9-tap Catmull-Rom history 采样、`clip_aabb` + variance clipping(动态盒)、`get_factor_disocclusion`、Reinhard 域 blend(base `1/16` + 亮度差抑闪)+ inverse Reinhard、out-of-screen reset。
- **不支持(known gaps,manifest `known_gaps` 逐条入账)**:
  - hardware 双线性(`textureLod`,linear+clamp sampler)以**显式 float 4-tap Load 双线性**复现——interior UV 逐 texel 等价,与真实 linear sampler 的 sub-texel fixed-point 舍入差异属 gap(镜像 ssao_blur 的 gather-vs-load seam)。
  - `rgba16f`/`rg16f` half 存储量化未建模(kernel 与 parity/smoke 用 float32;native buffer 为 half)。
  - history 物理维护链(`resolve → temp → internal → history` 三次 `copy_to_rect`)不接线——本切片只做单次 resolve dispatch;native continuation 每帧照常维护。
  - 一帧延迟(自队列 dispatch 读上一帧 `color`/`velocity`)——真替代需 `draw_graph` 集成,本切片声明为 deferred,只做 scaffold writeback。
  - `imageLoad` 越界语义:native Vulkan imageLoad 越界返回 0,kernel/parity 复现同语义(Load 越界返回 0)。
  - `get_closest_pixel_velocity_3x3` 的 border-offset 差(velocity 取 `group_top_left + min_pos` = `pos_screen - 1 + offset`,较 depth 查询偏移一 texel)——忠实复现 native 该 Spartan 派生行为。
  - multiview;temporal/noise 稳定性 evidence(4g-level,后续切片)。

## 6. Fallback

- fallback reason 枚举(对齐 GRX-008 五枚举):`compile_failed` / `validation_failed` / `unsupported_device` / `visual_diff_failed` / `manual_disabled`。
- 任一 compile / validation / visual / perf 失败 → 回退到 Godot 原生 TAA resolve 路径(`godot_native_taa_resolve`)。
- 默认 Godot config(per-pass 设置全部 `false`)与 shipping bridge 对 `RXGD_PASS_TAA_RESOLVE` 恒返回 `RXGD_STATUS_FALLBACK`,原生 resolve 始终执行。

## 7. Bridge gate(`src/rurix-godot/src/lib.rs` `TaaResolveGate`)

模板复制 GRX-011 `SsaoBlurGate` 的检查链,常量与 digest 指向 taa_resolve 产物:

1. **runtime binding preflight**:64-bit integer capability(b0 承载 i64 dims)、恰好 6 个 texture 资源(顺序 color/depth/velocity/last_velocity/history/output)、28-byte push constants、b0 中 source dims 非零且与 `color_buffer` 资源一致、`output_buffer` 尺寸 == color 尺寸(1:1 full-res)。
2. **dispatch eligibility**:opt-in capability flag `RXGD_CAP_TAA_RESOLVE_REAL_PASS (1u << 6)`(缺失 → `manual_disabled`)、64-bit integer capability、native D3D12 device/queue 非空、所有 6 个 resource native handle 非空、compiled package layout/digest 与 offline evidence 三个 SHA-256 逐字节匹配。
3. **kernel-binding-kind conformance**:`TAA_RESOLVE_KERNEL_RESOURCE_BINDING_KINDS = ["texture2d","texture2d","texture2d","texture2d","texture2d","rwtexture2d"]` per-slot 校验;buffer 资源 fail closed。
4. **math parity gate**:`TAA_RESOLVE_KERNEL_MATH_PARITY_STATUS = "taa_resolve_cpu_reference_proven_pending_gpu_dispatch"`(`math_parity_evidence.json`:CPU reference 已证,GPU 侧观察 pending real dispatch)。
5. **real dispatch**:仅在 `d3d12-recording-shim` feature(默认关闭)下经 6 资源 shim 入口(`rxgd_taa_resolve_record_dispatch`,SRV t0..t4 + UAV u0 + 28-byte b0 + `ceil(dims/8)` dispatch 形状,view format 由真实资源 format 推导)录制;shipping feature-off bridge fail closed 为 `real_dispatch_path_not_linked`。
- 任一失败返回 `RXGD_STATUS_FALLBACK`、记录 fallback reason、每 session 一次打印机读诊断 `RXGD_TAA_REAL_PASS_BLOCKED first_missing_prerequisite=... fallback_reason=... kernel_binding=texture2d default_enable_state=disabled`(非 `ERROR:` 行、不含 `RXGD_DIAG`),不累计 estimated GPU/CPU time。
- **行为变更入账**:`RXGD_PASS_TAA_RESOLVE` 在本段前走 bridge 的占位 estimated-timing 路径(record OK + 伪 estimated GPU time 160_000ns);自本段起改走 `TaaResolveGate` fail-closed 链,默认恒 fallback、不再产生任何 estimated taa_resolve GPU time。

## 8. Godot patch(deferred)

- patch `0017-0019`(gate+callsite / runtime binding / recording+real-pass opt-in)在 `PATCH_ALLOCATION.md` §2 预分配,归**后续串行切片**(需 patch-stack lock,S5/S7)。本切片(S1-S4+S6)**不**产出任何 patch。

## 9. Evidence 要求

- **offline compile**(本切片 measured):`offline_compile_evidence.json`——DXC cs_6_0 编译、DXV 验证、Rurix-owned RTS0(`emit_grx012_taa_resolve_rts0` example 经 `rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}`)、descriptor layout(6 slot binding_kind + 28-byte root constants)、三 artifact SHA-256 可追溯;`provenance=hlsl_bridge_workaround`、`rurix_owned=false`、`runtime_mappable=true`。
- **math parity**:`math_parity_evidence.json`(`generate_math_parity_evidence.py`)——确定性合成输入的 CPU float32 reference(逐操作 binary32 舍入),≥8 帧时序三用例(静止收敛 / 运动 disocclusion / 出屏 reset),`status=pending_gpu_dispatch`;GPU 侧观察由 standalone dispatch 段补齐。
- **standalone real D3D12 dispatch smoke**(本切片 measured 上限):`ci/grx012_taa_resolve_d3d12_dispatch_smoke.py` → `real_d3d12_dispatch_smoke.json`——真实 device/queue、RTS0 直建 root signature、tracked DXIL 建 compute PSO(内存副本经签名 DXC `dxil.dll` in-place 签名)、5 SRV(t0..t4)+ UAV u0 + b0 严格按 descriptor layout 绑定、一次 dispatch + fence + readback,且 readback 全部 output texel 与 CPU reference(单帧 TAA resolve)逐分量在容差内一致。SKIP(无 adapter / 缺 MSVC / 缺签名 DXC)不推进 ready。
- **引擎内 visual diff / measured fallback telemetry / real pass enablement**(4f/4g/4h 级别):留后续切片;本文件不宣称任何此类证据已取得。
- Perf:复用 GRX-006 baseline / perf gate;在产出实测证据前,不得声称任何性能提升。

## 10. 出口判据

- pass 默认 `disabled`;manifest `runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`。
- 未通过 compile / validation / visual / perf 门禁前,保持 `disabled` 并 fallback 到 Godot 原生 TAA resolve 路径。
- 本切片(contract + offline kernel + math parity + bridge gate + standalone dispatch smoke)**不**代表 pass 完成;后续段必须交付 Godot patch(0017-0019)、runtime 资源绑定(native handle)、runtime bridge dispatch recording、引擎内 visual diff + measured fallback telemetry、gated real-pass enablement 与 strict evidence(复用 GRX-009/011 4a..4h 段模板)。

## 11. 仍未完成项

- Godot patch 0017-0019(pass-gate + call-site / runtime native-handle binding / recording-smoke + real-pass opt-in;含 history 物理维护链与 `draw_graph` 真替代设计)。
- Godot runtime bridge dispatch recording smoke(4f 级别,需 full patch stack scratch 重建)。
- 引擎内 real visual diff + measured fallback telemetry gate(4g 级别;含 temporal/noise 稳定性 evidence)。
- gated real-pass enablement gate 与 measured success(4h 级别)。
- hardware-sampler 真替代(真 linear sampler 静态采样器 vs 显式 Load 双线性)、half 存储量化 parity、一帧延迟的 `draw_graph` 集成真替代。
- full baseline / per-pass FPS 对比数据;任何性能提升声明。

## 12. Close-out（GRX-011 stage-A5 对等）

> 本节为 close-out 追加段;§1–§11 的调查/契约/marker 保持不变(pass_id = taa_resolve、`RXGD_PASS_TAA_RESOLVE`、`RXGD_CAP_TAA_RESOLVE_REAL_PASS` 等契约字面不动),§5/§11 known gaps 不变。§10 出口判据与 §11「仍未完成项」原文保留;本节仅登记已落地事实。

GRX-012 taa_resolve 已 close-out(复用 GRX-009/010/011 4a..4h 成熟模板)。§8 pass-gate patch 之后补齐两段栈式 patch:

- **runtime resource binding — patch 0018(`0018-rurix-accel-taa-resolve-runtime-resource-binding.patch`)**:把 Godot runtime TAA resolve call site(`render_forward_clustered.cpp` `using_taa` 分支)传给 bridge 的资源从 logical id 改成真实 `ID3D12Resource*` native handle(经 `RenderingDevice::get_driver_resource(DRIVER_RESOURCE_TEXTURE, RID, 0)` 解析 color/depth/velocity/prev_velocity/history/temp 六个真实句柄,`rb->has_texture(taa, history)` 守卫 first-frame just-allocated,句柄为 0 或 `RenderingDevice` 不可用时 fallback 到原生 `taa->process`;fallback marker `RurixAccel: taa_resolve native resource handle mapping fallback rc=`)。
- **recording smoke + real-pass opt-in — patch 0019(`0019-rurix-accel-taa-resolve-recording-smoke-and-real-pass-optin.patch`)**:新增默认 `false` 的 `rendering/rurix_accel/passes/taa_resolve/{dispatch_real_pass,dispatch_recording_smoke,real_pass_force_capability_downgrade}` opt-in、`RXGD_CAP_TAA_RESOLVE_REAL_PASS`(1u<<6,复用 `RxGdCaps.flags`,`RXGD_ABI_VERSION` 保持 1)、`RXGD_GODOT_RUNTIME_TAA_RESOLVE_REAL_PASS` marker 与 `d3d12-recording-shim` 下的 real dispatch path;patch 0019 result writeback 为 **SCAFFOLD**(native Godot TAA resolve 仍作 continuation/backstop,每帧照常重跑整帧 resolve 并维护 `resolve->temp->internal->history` 物理 copy 链)。

**Enablement measured success 事实**:`ci/grx012_taa_resolve_real_pass_enablement_smoke.py` 在 0001..0022 scratch Godot(Windows D3D12 Forward+,NVIDIA GeForce RTX 4070 Ti)上记录 strict MEASURED success(`real_pass_enablement_success_evidence.json`,`status=success`、`strict_success=true`):opt-in real-pass 腿真正执行且完成——candidate 腿 `real_pass_marker_observed=true` + `writeback_marker_observed=true`(`RXGD_GODOT_RUNTIME_TAA_RESOLVE_REAL_PASS recorded=1`),`forced_capability_downgrade` 红腿实测 `first_missing_prerequisite=runtime_binding_preflight_failed`/`fallback_reason=unsupported_device`(`RXGD_TAA_REAL_PASS_BLOCKED`)。**temporal 硬约束(GRX_PLAN DoD)**:连续 8 帧序列(非单帧截图)逐帧对 reference 腿 diff——candidate/forced 全 8 帧 `max_abs=0`/`mean_abs=0`(逐帧 bit-exact),reference 序列帧间稳定性 `nonzero_delta_pairs=7/7`(携带真实运动,时序证据有效);measured_local telemetry 通过 GRX-008 校验,`0001..0022` patch-stack/溯源/日志审计全绿,DLL 指纹记 `features=[d3d12-recording-shim]`。standalone dispatch smoke(`real_d3d12_dispatch_smoke.json`)`real_d3d12_dispatch_recorded=true`、`cpu_reference_match=true`。manifest 顶层如实翻转 `implemented=true`、`real_gpu_pass=true`(opt-in 实测口径)、`real_d3d12_dispatch_recorded=true`、`runtime_state=fallback_only_by_default_real_pass_optin_measured`;`default_enable_state` 保持 `disabled`,新增 `implementation_status.real_pass_measured_success` block。

**shim 修复(W2-G 同类)**:`d3d12-recording-shim` 的 `typed_view_format` 映射扩充,把 Godot TAA depth buffer 的组合深度-模板 typeless 家族(`R32G8X24_TYPELESS`/`R24G8_TYPELESS`)映射到其 depth-plane SRV 读格式(`R32_FLOAT_X8X24_TYPELESS`/`R24_UNORM_X8_TYPELESS`),否则六资源 TAA record dispatch 会在 depth SRV 处 fail-closed 于 "unmapped typeless format"。

**Owner default-enable decision 引用**:`real_pass_default_enable_decision.json` / `.md` 记 `keep_default_disabled`,理由:①无 per-pass FPS 证据;②patch 0019 writeback 仍 scaffold(native continuation 每帧重跑 resolve 并维护 history,无净收益,candidate 序列逐帧 bit-exact);③仅单 resolve 子集 + 一帧延迟(history 双缓冲 copy-back、`draw_graph` 真替代、hardware-sampler、half 存储量化、multiview 未覆盖);full baseline + per-pass benchmark 后由 owner 复评。

**Fail-closed 不变**:默认 Godot config 下 bridge 对 `RXGD_PASS_TAA_RESOLVE` 仍返回 `RXGD_STATUS_FALLBACK`、native TAA resolve 接管;shipping feature-off bridge 仍 fail closed 为 `real_dispatch_path_not_linked`。gate 模块 `ci/grx_gates/grx012_taa_resolve.py` decision/enablement 双 ready(决策文件顶层 `default_enable_decision` 非空 + `strict_success=true`)后 probe `next_action=start_grx013_particles_copy_pass_contract`;任一缺失/被篡改即 `grx_gate_module_error`、`next_action` 保持不变。**§5/§11 known gaps 全部保留不变**。无 FPS、p95、GPU timestamp、时序稳定性或任何性能提升宣称。
