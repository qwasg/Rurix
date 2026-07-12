# GRX-019 Fused Post Chain Pass — PASS CONTRACT

> **状态声明(2026-07-12,S1-S3 切片:pass contract 三件套 + offline HLSL bridge kernel(DXC/DXV/RTS0)+ CPU math parity,即 PASS_TEMPLATE 的 S1-S3。**
> 本切片完全复用 GRX-012 taa_resolve 已打通的多资源纹理型模板:pass 契约三件套、HLSL bridge 数学等价 kernel(DXC 编译 + DXV 验证 + Rurix-owned RTS0 合成,owner-approved `hlsl_bridge_workaround` provenance,政策 `../luminance_reduction/texture_artifact_provenance_policy.json` 适用于所有 texture compute pass)、CPU float32 parity reference。
> 本切片**不**做 bridge gate(S4 归后续切片)、**不**做 standalone D3D12 dispatch smoke(S6)、**不**做 Godot patch(0036-0038 归后续串行切片)、**不**做 Godot scratch 全栈重建与引擎内实测、**不**做引擎内 visual diff、**不**启用默认 pass、**不**宣称任何 FPS / GPU timestamp / dispatch/barrier/VRAM traffic 下降 / 性能提升。measured 上限为 offline compile + CPU parity。
> pass 默认 `disabled`;任何 compile / validation / visual / perf 失败都走两级回退链(§6)最终回到 Godot 原生 luminance_reduction + tonemap 路径。
> §3 对 `external/godot-master` 的调查只记录路径与函数名;Godot 侧改动只以 `spike/godot-rurix/patches/` 下的 patch 文件入库,不直接修改快照的 Godot 原生源文件。

## 1. Pass 标识

- `pass_id = fused_post_chain`
- bridge pass id:`RXGD_PASS_FUSED_POST_CHAIN = 10`(`src/rurix-godot/src/lib.rs` 既有预留)
- cap 位:`RXGD_CAP_FUSED_POST_CHAIN_REAL_PASS = 1 << 13`(`patches/PATCH_ALLOCATION.md` §3 预分配 bit 13,复用 `RxGdCaps.flags`,不改 C ABI struct layout,`RXGD_ABI_VERSION` 保持 1)
- Tier:Tier 3(GRX.6 结构性优化;GRX_PLAN GRX-019「post FX fusion — 仅融合相邻 full-screen pass」)
- 目标后端:`Godot 4.7-dev Windows D3D12 Forward+`
- 默认启用状态:`disabled`
- Godot patch 预留:`0036`(gate+callsite)/ `0037`(runtime binding)/ `0038`(recording+real-pass opt-in),本切片不产出任何 patch。

## 2. 目标场景与融合对象

- 场景:`mixed_forward_plus` / `clustered_lights`(与两成员 pass 一致)。
- **融合对象(两成员均已 measured strict success,融合前置满足)**:
  1. **luminance 末级**(GRX-009 `luminance_reduction` 的 WRITE_LUMINANCE 最终级:tile mean → clamp → EMA,输出 1x1 current luminance);
  2. **tonemap**(GRX-010 `tonemap` 的 LINEAR + sRGB 子集,读 `../tonemap/artifacts/hlsl_bridge/tonemap_apply.hlsl` 现有 kernel 数学)。
- 融合形态:**一次全屏 compute dispatch** 同时完成 luminance 末级 EMA 写出与全屏 tonemap 写出,省去两 pass 之间的一次全屏读写往返与一次 dispatch/barrier 边界。这是设计动机的结构性陈述,**不是性能宣称**——在 S8 级 measured 证据落地前,不得声称任何 dispatch/barrier/VRAM traffic 或 FPS 收益。
- GRX_PLAN「仅融合相邻 full-screen pass」约束的对应:融合 dispatch 本身是全屏形态(tonemap 段决定线程网格);luminance 末级(1x1 输出、≤8x8 源)作为每 thread group 的寄存器驻留前导段搭载,不改变全屏 dispatch 形状。两成员在原生 post 链中相邻(luminance_reduction → [glow,不融合] → tonemap,tonemap 消费 luminance current;见 §3.3)。

## 3. Godot 侧候选 hook / call site / 资源流调查结果

仅记录路径与函数,**不改 `external/godot-master`**。

### 3.1 成员 A:luminance 末级(WRITE_LUMINANCE)

- `servers/rendering/renderer_rd/effects/luminance.cpp` `Luminance::luminance_reduction(...)`(`L159-256`):compute 路径逐级 8x 缩减;**最终级**(`i == reduce.size()-1 && !p_set`,`L228-231`)用 `LUMINANCE_REDUCE_WRITE` 变体并绑 `current` 为 prev(set 2);链末 `SWAP(current, reduce[last])`(`L255`)。
- `servers/rendering/renderer_rd/shaders/effects/luminance_reduce.glsl` `WRITE_LUMINANCE`(`L76-79`):`prev_lum = texelFetch(prev_luminance, ivec2(0,0), 0).r; avg = clamp(prev_lum + (avg - prev_lum) * exposure_adjust, min_luminance, max_luminance)`——**原生序 = EMA 在 clamp 内**。
- GRX-009 tracked 成员 kernel(`../luminance_reduction/artifacts/hlsl_bridge/luminance_reduce_level.hlsl` `-D RX_WRITE_LUMINANCE`)为 **clamp 后 EMA**(`cur = clamp(avg,min,max); out = prev + (cur-prev)*exposure_adjust`)。本 pass 融合段 A 与 tracked 成员 kernel 保持逐操作等价;与原生序的瞬态差异作为继承 gap 入账(§5)。
- 首帧:原生 `p_set == true` 时最终级走**普通 reduce**(写 raw avg,无 clamp 无 EMA);call site `renderer_scene_render_rd.cpp` `set_immediate`(`L583` 附近)。

### 3.2 成员 B:tonemap(LINEAR + sRGB 子集)

- `servers/rendering/renderer_rd/effects/tone_mapper.cpp` `ToneMapper::tonemapper(...)`:`u_exposure_texture`(set 1 binding 0,`L183-187`)、`TONEMAP_FLAG_USE_AUTO_EXPOSURE` / `auto_exposure_scale` push constant(`L155/L158`)。
- `servers/rendering/renderer_rd/shaders/effects/tonemap.glsl`:
  - `L860`:`color.rgb *= params.luminance_multiplier;`
  - `L864-870`:`float exposure = params.exposure;` + **auto-exposure 腿(`L866-868`)**:`exposure *= 1.0 / (texelFetch(source_auto_exposure, ivec2(0,0), 0).r * params.luminance_multiplier / params.auto_exposure_scale);` + `color.rgb *= exposure;`
  - `L893`:`apply_tonemapping`(LINEAR = identity);`L942-943`:`FLAG_CONVERT_TO_SRGB` 腿 `linear_to_srgb`。

### 3.3 融合接缝与关键缺口(侦察结论,契约必写)

- **原生数据流**:`renderer_scene_render_rd.cpp` `_render_buffers_post_process_and_tonemap`(`L459` 起)——auto-exposure 分支先跑 `luminance->luminance_reduction(...)`(`L555-588` 区域,含 `set_immediate`/`step`);随后 tonemap 块 `tonemap.exposure_texture = luminance->get_current_luminance_buffer(rb)`(**`L697`**),`camera_attributes_uses_auto_exposure` 为真且 RID 有效时 `use_auto_exposure = true; auto_exposure_scale = ...`(`L698-700`),否则 exposure_texture 回落 `DEFAULT_RD_TEXTURE_WHITE`(`L702`)。
- **关键缺口**:原生 tonemap 经 `exposure_texture` 纹理采样读 luminance current(`renderer_scene_render_rd.cpp:697` + `tonemap.glsl L866-868`),而 **patch 0012 只向 bridge 转发标量 `exposure/white/luminance_multiplier`,未传 `exposure_texture` handle**(`try_record_tonemap(...)` 签名无纹理腿)——因此现有 Rurix tonemap pass 仅对**关闭 auto-exposure** 的场景语义正确。
- **融合 kernel 的语义修复**:本 pass 的 kernel 把 luminance current(段 A 在 kernel 内寄存器直传的 EMA 结果)作为 tonemap 曝光输入,按 `tonemap.glsl L866-868` 原式复现 `exposure_effective = exposure * (1.0 / (lum_current * luminance_multiplier / auto_exposure_scale))`,即融合覆盖 **auto-exposure 开启**的 tonemap 场景(原生只有 auto-exposure 开启时才跑 luminance_reduction)。tonemap 侧的 exposure_texture 消费在融合 dispatch 内闭环;glow 等其它 luminance 消费者不覆盖(§5)。
- 中间隔着 glow:原生序为 luminance_reduction → gaussian_glow(自身也消费 luminance_texture + auto_exposure_scale,`L619/L625`)→ tonemap(glow composite 在 tonemap 内)。本 pass 子集**不含 glow**(两成员 kernel 均不含),glow 开启场景不满足融合前置,归 known gaps。

### 3.4 资源流(原生)

- 成员 A 输入:`reduce[last-1]`(最后一个中间缩减级,r32f,≤8x8——最终级 dst 为 1x1、单级 8x 缩减决定)与 `current`(1x1 r32f,上一帧 EMA 结果,即 prev);输出:`reduce[last]`(1x1)随后 SWAP 为新 `current`。
- 成员 B 输入:`rb->get_internal_texture()`(全分辨率 HDR,rgba16f)+ exposure_texture(= luminance current);输出:LDR 目标(原生为全屏 fragment pass 写 framebuffer)。
- **一帧延迟物理约束(沿用 GRX-009/012 声明)**:自队列 dispatch 读到上一帧内容。luminance 段吃上一帧 source/prev——EMA 时域反馈语义可辩护(GRX-009 同款声明);**tonemap 段原生消费当帧 internal texture,自队列融合 dispatch 无法替代当帧真实 tonemap 输出**——真替代需 draw_graph 集成路线 B(侦察结论:`RenderingShaderContainerD3D12` 原生化为长期方向),本切片如实声明为 deferred,S7 级 writeback 只能是 scaffold。

## 4. 输入 / 输出资源(Rurix mapping)

五资源布局(单一 descriptor table:SRV range t0..t2 先于 UAV range u0..u1,对齐 rurixc `infer_root_signature` §9 Q-RootShape=B):

| slot | Rurix 名 | binding | kind | HLSL 类型 | Godot 源 |
| --- | --- | --- | --- | --- | --- |
| t0 | `src_color` | SRV t0 space0 | texture2d | `Texture2D<float4>` | 全分辨率 HDR internal texture(rgba16f) |
| t1 | `lum_source` | SRV t1 space0 | texture2d | `Texture2D<float>` | luminance 最后一个中间缩减级(r32f,**extent ≤ 8x8**) |
| t2 | `prev_luminance` | SRV t2 space0 | texture2d | `Texture2D<float>` | 上一帧 1x1 luminance current(EMA prev) |
| u0 | `dst_color` | UAV u0 space0 | rwtexture2d | `RWTexture2D<float4>` | LDR 输出(extent == src_color extent,1:1 全屏) |
| u1 | `dst_luminance` | UAV u1 space0 | rwtexture2d | `RWTexture2D<float>` | 本帧 1x1 luminance current(EMA 输出;须与 t2 为不同资源,双缓冲镜像原生 SWAP) |

- b0 root constants(**64 bytes / 16 dwords**,root_parameter_index 0;两成员 canonical b0 合并 + 融合控制位,维持「i64 dims 在前、f32 标量在后」的 canonical 打包纪律,打包形状 `[i64, i64, i64, i64, f32 x8]`;**有意偏离成员的 28-byte 形状,S4 gate 必须按 64-byte 校验**):
  - `source_width`(i64,dword 0-1)/ `source_height`(i64,dword 2-3)——HDR color extent(tonemap 段线程网格与边界)。
  - `lum_source_width`(i64,dword 4-5)/ `lum_source_height`(i64,dword 6-7)——luminance 末级源 extent(≤8,partial-tile 除数)。
  - `max_luminance`(f32,dword 8)/ `min_luminance`(f32,dword 9)/ `exposure_adjust`(f32,dword 10)——成员 A b0 原样(Godot `LuminanceReducePushConstant`)。
  - `exposure`(f32,dword 11)/ `white`(f32,dword 12,LINEAR 不消费、形状 parity)/ `luminance_multiplier`(f32,dword 13)——成员 B b0 原样(Godot `TonemapPushConstant` 子集)。
  - `first_frame`(f32,dword 14;0.0/1.0,镜像原生 `p_set`,§5 语义)。
  - `auto_exposure_scale`(f32,dword 15;`tonemap.glsl L867` 原式因子)。
- tracked mapping:`resource_mapping.md`。

## 5. 支持范围与 gaps(起步口径)

- **支持**:单次融合 dispatch =
  - **段 A(luminance 末级)**:对 `lum_source`(≤8x8)做与成员 kernel 逐操作等价的 partial-tile-correct tile mean → `cur = clamp(avg, min_luminance, max_luminance)` → `ema = prev + (cur - prev) * exposure_adjust`;`first_frame != 0` 时输出 `cur`(跳过 EMA);每 thread group 冗余重算并经 groupshared 广播,group (0,0) 线程 0 写 `dst_luminance[0,0]`;
  - **段 B(tonemap LINEAR+sRGB)**:`color.rgb *= luminance_multiplier` → `exposure_effective = exposure * (1.0 / (lum_current * luminance_multiplier / auto_exposure_scale))`(`tonemap.glsl L866-868` 原式,lum_current 为段 A 寄存器直传值)→ `color.rgb *= exposure_effective` → LINEAR identity → `linear_to_srgb`(`tonemap.glsl L230-233` 系数)→ alpha passthrough。
- **不支持(known gaps,manifest `known_gaps` 逐条入账)**:
  - **glow composite 未覆盖**(pre/post/softlight/mix 全部模式、glow map、bicubic upscale;且 gaussian_glow 自身的 luminance_texture + auto_exposure_scale 消费不在融合内)——glow 开启场景不满足融合前置。
  - **LINEAR 之外的 tonemapper 模式未覆盖**(Reinhard / FILMIC / ACES / AgX)。
  - **auto-exposure 纹理链其余消费者未覆盖**:融合只闭环 tonemap 侧的 exposure_texture 读;luminance reduce 金字塔上游各级(READ_TEXTURE 首级 + 中间级)仍须原生链产出 `lum_source`,glow 等其它 exposure_texture 消费者仍走原生。
  - 成员 kernel clamp 序继承差:段 A 镜像 GRX-009 tracked kernel(clamp 后 EMA);原生 `WRITE_LUMINANCE` 为 EMA 在 clamp 内(`luminance_reduce.glsl L78`)——瞬态响应有界差异,稳态一致,继承入账。
  - `first_frame=1` 语义差:本 kernel 输出 `cur = clamp(avg)`;原生 `p_set` 最终级走普通 reduce 写 **raw avg(无 clamp 无 EMA)**——差异以 clamp 为界,有据入账(不同于成员的 zero-prev `cur * exposure_adjust` 退化,differently bounded)。
  - `lum_current == 0`(min_luminance=0 且全黑)时 `exposure_effective` 除零得 inf——与原生 GLSL 除法语义一致地复现,不加额外守卫;parity fixture 全部取 `min_luminance > 0`。
  - `rgba16f` / `r32f` 存储量化未建模(kernel 与 parity 用 float32;native color buffer 为 half)。
  - 一帧延迟(§3.4):luminance 段 EMA 语义可辩护;tonemap 段吃当帧 source 的正确性约束如实入账——自队列融合 dispatch 不能替代当帧 tonemap 输出,真替代需 draw_graph 路线 B(`RenderingShaderContainerD3D12` 原生化为长期方向),本切片只到 offline+parity。
  - sRGB 输出不 clamp 到 [0,1](继承 tonemap 成员;原生靠 framebuffer 量化)。
  - raster-vs-compute 输出接缝:原生 tonemap 是全屏 fragment pass 写 framebuffer,本 kernel 写全分辨率 UAV(继承 tonemap 成员)。
  - `lum_source` extent > 8x8 不支持(单 tile 约束,镜像原生最终级 dst=1x1 形态);S4 gate 必须 fail-closed。
  - no FXAA / BCS / color correction / debanding / multiview / HDR 输出(convert_to_srgb=false)(继承 tonemap 成员)。
  - GPU 侧 math parity 观察 pending real dispatch(`math_parity_evidence.json`)。

## 6. Fallback(两级回退契约)

- fallback reason 枚举(对齐 GRX-008 五枚举):`compile_failed` / `validation_failed` / `unsupported_device` / `visual_diff_failed` / `manual_disabled`。
- **两级回退链(归 patch 切片 0036-0038,marker 需区分两级)**:
  1. **fusion 级**:`RXGD_PASS_FUSED_POST_CHAIN` 融合 gate 任一失败 → 回退到**逐成员单 pass gated 路径**(既有 `LuminanceReductionGate` / `TonemapGate` opt-in 腿;它们默认亦 fallback)。fusion 级 marker(patch 0036-0038 落地时)必须与成员级可区分(规划名 `RXGD_FUSED_POST_CHAIN_REAL_PASS_BLOCKED ...`,非 `ERROR:` 行、不含 `RXGD_DIAG`)。
  2. **成员级**:单 pass gate 再失败 → Godot 原生路径(`godot_native_luminance_reduction` + `godot_native_tonemap`),原生 post 链照常执行。
- 默认 Godot config(per-pass 设置全部 `false`)与 shipping bridge 对 `RXGD_PASS_FUSED_POST_CHAIN` 恒返回 `RXGD_STATUS_FALLBACK`,原生链始终执行。
- 本切片(S1-S3)未接线任何 bridge gate;上述为契约规划,S4/S5/S7 落地。

## 7. Bridge gate(规划,S4 切片落地)

模板复制 GRX-012 `TaaResolveGate` 检查链,常量与 digest 指向 fused_post_chain 产物:

1. **runtime binding preflight**:64-bit integer capability(b0 承载 i64 dims)、恰好 5 个 texture 资源(顺序 src_color/lum_source/prev_luminance/dst_color/dst_luminance)、**64-byte** push constants、b0 中 source dims 非零且与 `src_color` 资源一致、`dst_color` extent == `src_color` extent(1:1)、`dst_luminance`/`prev_luminance` extent == 1x1、`lum_source` extent ≤ 8x8 且与 b0 一致、`prev_luminance` 与 `dst_luminance` 为不同资源(反别名,双缓冲)。
2. **dispatch eligibility**:opt-in capability flag `RXGD_CAP_FUSED_POST_CHAIN_REAL_PASS (1u << 13)`(缺失 → `manual_disabled`)、native D3D12 device/queue 非空、5 个 resource native handle 非空、compiled package layout/digest 与 offline evidence 三个 SHA-256 逐字节匹配。
3. **kernel-binding-kind conformance**:`FUSED_POST_CHAIN_KERNEL_RESOURCE_BINDING_KINDS = ["texture2d","texture2d","texture2d","rwtexture2d","rwtexture2d"]` per-slot 校验;buffer 资源 fail closed。
4. **math parity gate**:`FUSED_POST_CHAIN_KERNEL_MATH_PARITY_STATUS = "fused_post_chain_cpu_reference_proven_pending_gpu_dispatch"`。
5. **real dispatch**:仅在 `d3d12-recording-shim` feature(默认关闭)下经 5 资源 shim 入口(SRV t0..t2 + UAV u0..u1 + 64-byte b0 + `ceil(source dims / 8)` dispatch 形状)录制;shipping feature-off bridge fail closed 为 `real_dispatch_path_not_linked`。

## 8. Godot patch(deferred)

- patch `0036-0038`(gate+callsite / runtime binding / recording+real-pass opt-in)在 `patches/PATCH_ALLOCATION.md` §2 预分配,归**后续串行切片**(需 patch-stack lock,S5/S7)。本切片(S1-S3)**不**产出任何 patch。
- patch 0036 call-site 规划:`_render_buffers_post_process_and_tonemap` 的 auto-exposure + tonemap 区域(§3.3)加 opt-in gate;两级回退 marker(§6)在 0036-0038 内落地;融合前置(auto-exposure on、glow off、LINEAR、SDR)不满足时直接走成员级/原生级。

## 9. Evidence 要求

- **offline compile**(本切片 measured):`offline_compile_evidence.json`——DXC cs_6_0 编译、DXV 验证、Rurix-owned RTS0(`emit_grx019_fused_post_chain_rts0` example 经 `rurixc::binding_layout::{infer_root_signature, pack_root_constants, serialize_rts0}`)、descriptor layout(5 slot binding_kind + 64-byte root constants)、三 artifact SHA-256 可追溯;`provenance=hlsl_bridge_workaround`、`rurix_owned=false`、`runtime_mappable=true`。
- **math parity**(本切片 measured):`math_parity_evidence.json`(`generate_math_parity_evidence.py`)——确定性合成输入的 CPU float32 reference(逐操作 binary32 舍入),覆盖 **EMA 序列 × tonemap(LINEAR+sRGB)复合**:≥8 帧时序 case ≥3 个(静态收敛 / 亮度阶跃自适应 / clamp 双边界)+ 首帧边界 case + 单帧 dispatch fixture(留待 S6 GPU 复核),`status=pending_gpu_dispatch`。
- **standalone real D3D12 dispatch smoke / bridge gate / patch / enablement / visual diff / telemetry**:全部留后续切片;本文件不宣称任何此类证据已取得。
- Perf:复用 GRX-006 baseline / perf gate;在产出实测证据前,不得声称任何 dispatch/barrier/VRAM traffic 或性能提升。

## 10. 出口判据

- pass 默认 `disabled`;manifest `implemented=false`、`runtime_state=fallback_only`、`real_gpu_pass=false`、`real_d3d12_dispatch_recorded=false`。
- 未通过 compile / validation / visual / perf 门禁前,保持 `disabled` 并按 §6 两级回退链回到 Godot 原生路径。
- 本切片(contract + offline kernel + math parity)**不**代表 pass 完成;后续段必须交付 bridge gate(S4)、Godot patch 0036-0038(S5/S7)、standalone dispatch smoke(S6)、scratch 重建 + enablement strict evidence(S8)、gate module + close-out(S9),全部复用 GRX-009..014 成熟模板。

## 11. 仍未完成项

- bridge `FusedPostChainGate`(S4;含 64-byte b0 preflight、lum_source ≤8x8 gate、prev/dst 反别名 gate、两级回退 marker 规划的 bridge 侧常量)。
- Godot patch 0036-0038(S5/S7;pass-gate + call-site / runtime native-handle binding / recording-smoke + real-pass opt-in;含两级回退 marker 与融合前置检查)。
- standalone real D3D12 dispatch smoke(S6;`ci/grx019_fused_post_chain_d3d12_dispatch_smoke.py`,回填 math parity 的 GPU 侧)。
- Godot runtime bridge dispatch recording smoke + 引擎内 real visual diff + measured fallback telemetry + gated real-pass enablement(4f/4g/4h 级别,S8)。
- draw_graph 路线 B 真替代设计(tonemap 段当帧正确性;`RenderingShaderContainerD3D12` 原生化长期方向)。
- full baseline / per-pass FPS / dispatch/barrier/VRAM traffic proxy 对比数据;任何性能提升声明。
