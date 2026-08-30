===== b1d9293a =====
# 渲染器画质优化侦察报告（只读，未修改任何文件）

基线配置已由 receipt 实锤：`artifacts/night_baseline/bistro-interior/tier100/tsr_device/render_receipt.json:17` 显示 pass0 SPV = `g18_light_transport_depth.spv`（--presentation-profile night 触发，`g14_3_pipeline_perf.rs:346-348`），`--gi off`，exposure=16（bistro ev100=-4 → 2⁴，`g14_3_lane_body.rs:11538`），tier100 内部=输出=1920×1080，128 帧。bistro 灯面：**0 个 quad 面光 + 4 个 point 灯 + 4 个 emissive 材质**（`milestones/g13/g13_ue_upscale_parity_contract.json:60-89`），emissive_tri_count=44024。

---

## 1. 噪点根因：pass0 的随机数生成（结论：生产臂零 RNG）

**`kernels/g14_3_direct_gi.rx` 与 `g18_light_transport_depth.rx` 全部确定性，无 sin hash、无 per-pixel hash、无帧序号种子。** 唯二采样结构：

- quad 面光 **4×4=16 分层确定性采样**（无随机项）：

```185:186:src/rurix-render/kernels/g14_3_direct_gi.rx
                    let u = (sxq as f32 + 0.5) * 0.25;
                    let v = (syq as f32 + 0.5) * 0.25;
```

- 阴影射线：每分层样本 1 条 first-hit 阴影 ray，origin 沿光线方向偏 eps（**不是沿法线**），t_max = d−2eps：

```207:209:src/rurix-render/kernels/g14_3_direct_gi.rx
                    let t_sh = d - 2.0 * ray_eps;
                    let gate_far = (t_sh * big).min(1.0).max(0.0);
                    let t_fire = t_sh.max(ray_eps);
```

- point 灯：单 delta 样本 + 1 条阴影 ray（`g14_3_direct_gi.rx:253-301`）。**注意 `g18_light_transport_depth.rx:3-4` 头注释声称「point 灯 2×2 分层采样，半径 params[43]」，但代码中 params[43] 从未出现**——point 臂与 g14_3 逐字相同（单样本），头注释是过期/愿景性描述；g18 相对 g14_3 的实际增量只有 miss 天光 `params[42]`（`g18_light_transport_depth.rx:311-315`）。
- `g14_3_shadow_scatter.rx`（Split 拆散车道，仅 cornell 单 quad 启用）：layer = syq·4+sxq 确定性映射（`:100-101, 118-119`），同样零 RNG。
- 逐帧唯一变化量 = Halton jitter（`jx/jy`，仅进主射线，`g14_3_direct_gi.rx:75-76`），host 侧 `halton(jitter_base+i+1, 2)-0.5`（`g14_3_lane_body.rs:11597-11598`，窗口模数 `JITTER_WINDOW_MOD=65521`，`:78`）。

**spp 常量**：1 主射线/px；bistro = 1+4（4 点灯各 1 阴影 ray）= **5 ray/px**；cornell = 1+16 = 17 ray/px。emissive 三角**不参与 NEE**——只有 quad/point 被采样，emissive 仅主命中时无条件加 emission：

```310:312:src/rurix-render/kernels/g14_3_direct_gi.rx
        out_color[i * 3] = hit_f * (mats[mb + 3] + al_r * inv_pi * dir_r);
        out_color[i * 3 + 1] = hit_f * (mats[mb + 4] + al_g * inv_pi * dir_g);
        out_color[i * 3 + 2] = hit_f * (mats[mb + 5] + al_b * inv_pi * dir_b);
```

**全仓库唯一的 sin hash** 在未接线的 GI kernel（见 §5）：

```318:320:src/rurix-render/kernels/g16_gi_multibounce.rx
            let ph = (px as f32) * 12.9898 + (py as f32) * 78.233 + (bnc as f32) * 17.13 + jx;
            let r1 = ph.sin() * 0.5 + 0.5;
            let r2 = ((px as f32) * 26.716 + (py as f32) * 41.329 + (bnc as f32) * 9.71 + jy).sin() * 0.5 + 0.5;
```

种子 = 纯像素坐标 + bounce 序号 + **全帧共享的** Halton jitter（非帧序号）——这正是「per-pixel 静态 hash」形态，但它不在基线链路里。

## 2. TSR 收敛行为参数

host 默认值（`src/rurix-render/src/temporal/tsr.rs:75-87`）：`base_alpha=0.1`、`min_alpha=0.04`、`flicker_window_frames=16`（→ ema_k=2/17≈0.1176）、`flicker_tighten=0.5`、`flicker_deadzone_abs=0.02`、`flicker_deadzone_rel=0.1`、`depth_rel_tol=0.1`。打包于 `g14_3_lane_body.rs:6282-6316`（`pack_tsr_params`），**reactive 恒关**（调用点 `:7536` 硬编码 `false`）。

- **resample**（`g14_8_tsr_resample.rx`）：Catmull-Rom 4×4（a=-0.5），ratio>1 时核 ×0.75（`:64-69`）；抗振铃 = 逐通道 RGB 钳入 4×4 采集邻域 min/max（`:122-136`）；随后 ×exposure 转显示域（`:137-139`）；深度最近邻上采样（`:146-148`）。
- **resolve**（`g14_8_tsr_resolve.rx`）：闪烁检测 = 亮度帧间差分 + 死区符号翻转 EMA（`:84-98`）；MV 最近邻上采样（`:100-105`）；历史双线性重投影（`:109-149`）；历史验证 = 深度相对差 `depth_tol·max(dc,dp) − |dc−dp|` + 出屏（`:152-156`，法线恒过）；**无 variance clipping**——历史色 YCoCg 化后钳入当前帧 3×3 邻域 AABB，闪烁松弛 `relax = min(score·(1−reactive),1)`（`:207-214`）；alpha 调制：

```216:219:src/rurix-render/kernels/g14_8_tsr_resolve.rx
            let alpha = (base_alpha * (1.0 - tighten * score * (1.0 - reactive)))
                .max(reactive)
                .min(1.0)
                .max(min_alpha);
```

valid=0 时直通当前帧（`:232-234`）。混合在 YCoCg 域（`:224-226`）。

## 3. 输出量化路径

- **EXR（bench/render 车道）**：`write_exr`（`g14_3_lane_body.rs:9761-9767`）→ fp32 直写，元数据 `ExrTransfer::Linear / Float32 / SceneLinearHdr`（`:9697-9710`）。**EXR 前无 tonemap**；契约强制 `tonemap=off`（`:702-705`）。exposure（×16）在 TSR resample kernel 内乘入（`g14_8_tsr_resample.rx:137-139`），即 TSR 全程在 ×16 显示域运算。converged.exr = 末帧 TSR 输出（`:12350-12357`）。
- **窗口 present**：`g31_window_present.rs` 第五 pass = `kernels/g31_display_encode.rx`：TSR 输出 → ACES 1.3 RRT+ODT（Rec.709 100nits dim）→ **BT.1886 γ2.4 逆 EOTF**（不是 sRGB EOTF）→ 8-bit 量化，**无 dither**：

```441:443:src/rurix-render/kernels/g31_display_encode.rx
        let qr8 = (fr.powf(0.41666666) * 255.0 + 0.5).floor() as u32;
        let qg8 = (fg.powf(0.41666666) * 255.0 + 0.5).floor() as u32;
        let qb8 = (fb.powf(0.41666666) * 255.0 + 0.5).floor() as u32;
```

  BGRA8 SSBO → host 回读 8.3MB → `present_rgba8` staging → copy→present（`src/rurix-rt/src/vk_g31_present.rs:521-537`）。swapchain = `B8G8R8A8_UNORM` + `SRGB_NONLINEAR` color space（`src/rurix-rt/src/vk.rs:6836, 7000-7015`）——UNORM 格式意味着无硬件 sRGB 编码，encode kernel 的 γ2.4 就是全部 transfer function。**全仓库 grep `dither` 零命中**（src/rurix-render 全树）。
- `display/swapchain.rs` = host 侧骨架/oracle（逐像素 f64 `plugin.transform`，SdrBt1886），非 device 生产路径。
- **post_chain.rs 五级链 = 纯 host 骨架**（f64 逐像素）：exposure=EV 标量乘（`:205-208`）、bloom=3×3 box 近似（见 §4）、tonemap=ViewTransform 插件（`:338-341`）、LUT=逐通道 slope/offset（`:237-243`）、output transform=`encode_display_linear`（`:358-361`）。**仅被两处调用**：`export_presentation_png`（`g14_3_lane_body.rs:9828-9876`，需 `--export-png` + `--presentation-profile`）和 harness `g9_m119_post_chain.rs`。**任何生产 render/bench/window 车道都不调用它**；且 PNG 出口同样是 clamp(0,1)→8-bit 无 dither（`:9860-9865`）。

## 4. bloom 现状：基本不存在

- `kernels/` 下 **零 bloom kernel**（glob 76 个 .rx + grep 双重确认）。
- 唯一实现 = `post_chain.rs:212-233` 的 host 3×3 box blur 骨架：`out = px + blur/9 × 0.5`——**无阈值、无 mip 链、强度 0.5 硬编码**。注释声称「完整 mip 链在 device 面」但 device 面不存在该代码。
- G18 契约的 `bloom_strength`（night=0.15 / day=0.08，`milestones/g18/g18_presentation_contract.json`）**没有任何 Rust 代码消费**（全仓库 grep 零命中）——`export_presentation_png` 只读 ev_offset/ev100_delta/warm_lift（`g14_3_lane_body.rs:9841-9852`）。

## 5. GI 状态

- 接线点：`g14_3_pipeline_perf.rs:343-345`——`--gi on` 且 spv_scene 为默认时，切换到 `DEFAULT_SPV_GI = .tmp/g14_gates/m_c/g16_gi_multibounce.spv`（`g14_3_lane_body.rs:61`）。Split 形态对 g16 显式禁用（`:11562-11564` 与 `:13731` 的 `!spv_scene.contains("g16_gi_multibounce")`），即 GI 臂只能走 Mega 四 pass。receipt 如实登记 `gi_arm=additive_on`（`:12400-12402`）。
- 「G14.3 不接线 / fail-closed not-triggered」的措辞（kernel 头注释 `:19-22`、receipt 字符串 `:12403`）指的是 **M-c 门的评估结论**（g9_m98/g9_m99 GI kernel 内容模型与 G13.4 锚不同构，不得替换默认臂）；代码现状是 `--gi on` 作为加性 opt-in 臂可跑。基线确认为 `--gi off`。
- g16 kernel 内容：直接光与 g14_3 同式 + **2 次余弦半球反弹**（`while bnc < 2`，`:317`）+ 次级 NEE（quad 16 分层 / point delta）+ 次级辐射 clamp 16.0（`:524-526`）；反弹方向用 §1 的 sin hash——**若未来接线 GI，这个静态 hash 会成为真正的逐像素噪点源**（1 spp/反弹、种子不含帧序号、逐帧仅靠共享 jitter 扰动，TSR 无法有效收敛空间相关的 hash 图案）。

## 6. 性能关键常量与 bench 开销分离

- **dispatch/线程组**：pass0 `#[numthreads(8,8,1)]`（`g14_3_direct_gi.rx:43`），dispatch = ceil(iw/8)×ceil(ih/8)×1，host 自 SPV LocalSize 派生（`g14_3_lane_body.rs:6256` `spv_local_size`、`:6560`、`:6593`）——1080p 下 240×135 组。TSR resample/resolve = `#[numthreads(32,4,1)]`（`g14_8_tsr_resample.rx:34`、`g14_8_tsr_resolve.rx:28`）→ 60×270 组。mv = 8×8（`g14_mv.rx:36`）。
- **射线数**：见 §1（bistro 5 ray/px）。`ray_eps = clamp(场景包围盒最长边×1e-4, 1e-3, 0.5)`（`g14_3_lane_body.rs:1892-1903`），`RAY_TMAX=1e30`（`:80`）。
- **bench 逐帧 fence/readback**：`lane.frame`（`:7731-7757`）→ `execute_with_frame_update`（`src/rurix-rt/src/render_exec.rs:1487`）→ `execute_persistent_frame` 内 submit 后**当有界 `wait_fences` 全同步**（`render_exec.rs:9324-9331`，`cpu_fence_wait_ns` 计量于 `:9331`）——inflight=1 下每帧 CPU-GPU 全串行。测量循环 `readback_subset=Some([])` 零回读（`:7590-7614`；`readback_out = flip_trace || i+1==total`，`:13909`），tail（字节→f32 转换 + is_finite 校验 + digest）仅末帧/trace 帧非零，`frame_ms_production = frame_ms − tail`（`:13951-13968`，口径注释 `:14004`）。FIF 流水（inflight 2/3）走 `submit_with_frame_update`→票据→延迟 `collect`（`:8007-8055`；`render_exec.rs:1632, 1728`）。**结论：bench 开销与生产帧时分离是真实落地的**——测量帧无回读税，但每帧 fence 全同步是生产固有口径（present 车道同样逐帧 fence，`g31_window_present.rs:45-48`）。

---

## 7. 噪点根因推断（基于代码证据）

**基线链路（g18 pass0 + TSR）逐帧完全确定性，不存在任何 hash 噪声源。** 「TSR 收敛后仍有全画面颗粒噪点」的机制不是 Monte Carlo 采样噪声，而是**逐 jitter 二值/高频信号 × TSR 驻态 EMA 残差**：

1. **EMA 永不到达不动点**：base_alpha=0.1 意味着「收敛」后是平稳随机过程而非定点，驻态残差 std ≈ σ_frame·√(α/(2−α)) ≈ **0.23·σ_frame**。只要逐帧（随 Halton jitter 变化的）信号存在大幅度二值起伏，残差即表现为静态颗粒。
2. **最大 σ_frame 源 = 亚像素 emissive 三角弹出**：bistro 有 44024 个 emissive 三角，主命中即无条件加 emission（`g14_3_direct_gi.rx:310-312`），jitter 使亚像素灯片逐帧进/出覆盖 → 0/Le 二值信号，且 TSR 在 ×16 exposure 域运算（`g14_8_tsr_resample.rx:137-139`）把幅度放大 16 倍 → 全画面颗粒。这与「疑似静态 hash 噪声」的观感一致（空间上去相关、时间上收敛后冻结成颗粒）。
3. **栏杆细几何走样（症状②）同源**：jitter 主射线在细几何上 hit/miss 或命中不同三角 → 逐帧二值 albedo/visibility → 同一 EMA 残差机制；TSR 的历史验证只有深度相对差（`g14_8_tsr_resolve.rx:152-156`），对同深度的细几何抖动不拒史。
4. **阴影边界/接触区颗粒**：阴影 origin 沿光线方向偏移 eps（非沿法线，`g14_3_direct_gi.rx:207-209`），eps = 场景尺度×1e-4（bistro 为毫米级），掠射/薄遮挡处可见性随 jitter 逐帧翻转 → 第三类二值信号。
5. **TSR 的防护对此类噪声无效**：resample 抗振铃钳和 resolve YCoCg AABB 都是**邻域相对**钳制——邻域内含 firefly 时钳位范围随之放大，无法抑制；闪烁死区（0.02 abs / 0.1 rel，×16 域）对小振幅振荡不触发收紧。
6. **色带（③）**：唯一 8-bit 出口 `g31_display_encode.rx:441-443` 为 `floor(v·255+0.5)` 无 dither（全仓库无 dither 实现），EXR/PNG 路径同样无 dither——渐变区量化台阶即色带。
7. **无 bloom（④）**：device 无 kernel、host 骨架未接线、契约 `bloom_strength` 无人消费——症状④是「从未实现」而非「参数问题」。

**修复方向暗示**（供决策，不在本次侦察范围）：颗粒噪点的治理点不在「换 hash」（生产臂无 hash 可换），而在 ①emissive 主命中的 jitter 方差（如 emissive 重要性采样/主射线过滤）②阴影 origin 改沿法线偏移 ③降低驻态 alpha 或收敛后冻结历史 ④8-bit 出口加 dither。

Now write the full report.

===== 422990fd =====
# 平滑顶点法线加性臂 — 只读侦察报告

## 1. glTF 加载链：NORMAL 从未被读取

生产车道的 glTF 加载是 **bin-local 实现**，全部在 `src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs`（该文件被 `g14_3_pipeline_perf.rs` 与 `g31_window_present.rs` 两 bin 经 `include!` 逐字共享，L1-6）。链路：

- `load_gltf` L1089 — 读 glTF JSON + 外部 bin buffer，算 `gltf_sha256`（L1091）
- `impl Gltf` L1111-1245 — accessor 读取器三件套：`positions` L1182（float VEC3）、`texcoords` L1206（float VEC2）、`indices` L1227。**没有 `normals` 方法——NORMAL accessor 从未被读取，这就是"丢弃点"（更准确说是"从未接入点"）**
- 装配循环 `assemble_scene_ex` L1475，primitive 循环 L1625-1704：

```1657:1681:src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs
            let pos_acc = prim
                .get("attributes")
                .and_then(|a| a.get("POSITION"))
                .and_then(|v| v.as_u64())
                .ok_or_else(|| cerr("primitive 缺 POSITION"))?;
            let pos = gltf.positions(pos_acc as usize)?;
            // ...
            // Task B4：TEXCOORD_0 读取（uv_out 消费面;off 面不读不算,0-byte）。
            let uvs: Option<Vec<[f32; 2]>> = if uv_out.is_some() {
                let uv_acc = prim
                    .get("attributes")
                    .and_then(|a| a.get("TEXCOORD_0"))
```

`SceneData` L1381-1396 只有 positions/indices/albedo/emission/tri_mat/quads/points/camera——无法线字段。三角汤扁平化 `pack_tris` L1912（9 f32/tri）、`pack_mats` L1926（8 f32/tri [albedo3, emission3, 0, 0]）。

用户提到的「P0 不引入法线/UV」确切记在**离线构建器**（非本车道）：`src/rurix-geom-build/src/mesh.rs` L3（`TriMesh` = positions + indices，L12-15），属 cluster-lod 离线链，与 g14_3 生产加载器正交。

**资产面实证**：`K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf`（路径解析 `default_gltf` L9962-9970）2062 个 primitive **全部带 NORMAL**（float VEC3，componentType 5126），源数据完备。

## 2. UV 先例：6 f32/tri 旁路 sink，通路完全可承载法线

UV 通路（G31+ 波 B Task B4，--textures on 面）：

- **装配侧**：`assemble_scene_uv` L1460 → `assemble_scene_ex(..., Some(uv_out))` L1466。契约注释 L1473-1474：「sink 布局 = 6 f32/tri〔uv0,uv1,uv2 顶点序与 tris 同源〕」。写入点：

```1694:1698:src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs
                if let (Some(sink), Some(uv)) = (uv_out.as_deref_mut(), uvs.as_ref()) {
                    for &vi in t3 {
                        sink.push(uv[vi as usize][0]);
                        sink.push(uv[vi as usize][1]);
                    }
                }
```

quad 灯面尾段 UV 恒 0（L1759-1762）。关键纪律：「SceneData 各字段与 off 面逐位同值（UV = 旁路 sink 纯记录）」L1458——**侧表不进 SceneData，装配产物 0-byte**。

- **消费侧**（`g31_window_present.rs`）：L4514-4519 `assemble_scene_uv(..., &mut tri_uv)`；资源下标 24 = 逐三角 UV 6 f32/tri（L222 注释），`d.resources.push(init(&tex.texuv_bytes)) // G31_U_TEX_UV` L1202；kernel 变体 = `g31_texture_gi.rx`。
- **G34 车道**：`G34TexSideTable`（g14_3_lane_body.rs L6870-6900，含 `texuv_bytes`/`tritex_bytes` + textures off 缺省哑件 `default_face` L6882——tritex 全 −1 ⇒ kernel 行为 == 母版位级）。
- **重排工具**：`gather_tri_uv` L4348（cluster-lod cut 后按 TriProvenance 重排侧表，恒等排列 ⇒ 逐位一致）——法线侧表若与 cluster-lod 组合需同律 gather。

**结论：这条通路能原样承载法线**——把 `texcoords`（VEC2）换成 VEC3 normals 读取、sink 从 6 f32/tri 变 9 f32/tri、再加一个 SSBO 绑定即可，所有纪律（off 面 0-byte、quad 尾段恒 0、provenance gather）都有现成模板。

## 3. kernel 侧消费点：barycentric 插值先例三处

`committed_barycentric()` 先例（`g34_unified_primary.rx` L106-108）：

```106:108:src/rurix-render/kernels/g34_unified_primary.rx
            let bary = rq.committed_barycentric();
            bu = bary.0;
            bv = bary.1;
```

**megakernel 内**（与 g18 同构，不拆 pass）的先例 = `g31_texture_gi.rx`（g14_3_direct_gi 逐字 fork + 5 个侧表 view，签名 L45-59：多出 `texuv/texmeta/tritex/atlas/linlut`），L114-119 在 `has_committed` 臂内取 bary，插值式 L175-179：

```175:179:src/rurix-render/kernels/g34_unified_shade.rx
        let uu0 = bw_ * texuv[ub] + bu * texuv[ub + 2] + bv * texuv[ub + 4];
        let vv0 = bw_ * texuv[ub + 1] + bu * texuv[ub + 3] + bv * texuv[ub + 5];
```

（`bw_ = 1.0 - bu - bv`、`ub = pg * 6` 在 L176-177；`g31_svt_gi.rx` L124-126/L180-183 同式。）

注意：`g34_unified_shade.rx` 的 shading 法线仍是 flat 几何法线（cross + 归一 + 双面翻转，L142-156），barycentric 只用于 albedo 采样——**平滑法线 shading 尚无先例**，但插值式直接推广为 `n[c] = bw*n0[c] + bu*n1[c] + bv*n2[c]`（9 f32/tri 侧表，`nb = prim * 9`），随后接既有归一化（g18 L133-141 的倒数乘口径）与双面翻转（L143-147）即可。

g18 当前法线计算（要替换/门控的点）：

```130:141:src/rurix-render/kernels/g18_light_transport_depth.rx
        let ngx = e1y * e2z - e1z * e2y;
        let ngy = e1z * e2x - e1x * e2z;
        let ngz = e1x * e2y - e1y * e2x;
        let nl = (ngx * ngx + ngy * ngy + ngz * ngz).sqrt();
        // ...
        let inv_nl = 1.0 / nl_safe;
        let hgx = gate_nl * (ngx * inv_nl);
```

参数面有富余：`PARAMS_LEN = 48`（L82），`pack_frame_params` L6188-6229 只写到 [42]（sky，env `RURIX_G18_SKY_INTENSITY`），**[43..48) 恒 0 保留**——平滑法线开关可放 params[43]（gate 门化，0 = flat 母版字面）。

## 4. host 金标准对拍面

host 参考在 `src/rurix-render/src/bin/g13_4_ue_upscale_parity_render.rs`：`render_frame` L2089、`shade_pixel` L1998（`n_in` 参数传入）、`visible` L1984、`unproject` L1975。法线来源 L2154 `let mut n = hit.normal;` → `TriBvh` 的 flat 面法线 `face_normal = (b-a).cross(c-a).normalize()`（`src/rurix-render/src/rt/bvh.rs` L383，赋值 L678）。g18 kernel 头 L6-7 明示「与 G13.4 host `render_frame`/`shade_pixel` 逐字同模——M-d 画质守护可比性锚」。

**关键事实：host 没有逐像素纹理采样臂**——只有 `texture_mean_albedo`（DDS 均值 → 常量 albedo，g13_4 L1675-1695）。--textures 加性臂的 host 对拍**没有走全帧 host render**，而是探针制：

- `g31_tex_host_sample` L5068（host f32 位级参考）/ `g31_tex_probe_evaluate` L5618（device 双跑 + host digest 互等，p100==0.0 硬门）
- CI 门 `ci/g31_texture_sampling_smoke.py`：off 双跑 digest_seq 位级一致（回归锚）+ on 双跑位级一致（确定性）+ **on≠off**（接线真实生效门，L309）+ on/off frame_ms 对照

平滑法线臂可照此办理：host 侧加一个 `smooth_normal` 探针参考函数（纯 f32 插值 + 同式归一，易位级），或更轻量——复用 g13_4 host 加 `n_in` 替换点（shade_pixel 已吃参数化法线，L2002/L2008），加性 arm 函数即可。

## 5. 风险面：冻结门/digest 锚清单

| 锚 | 位置 | 钉住什么 |
|---|---|---|
| 文件级语义冻结 | g14_3_lane_body.rs L5-6「digest 锚定逻辑禁动」 | 本文件改动同时影响两 bin |
| 契约 digest | `FROZEN_CONTRACT_DIGEST` L52-53；门序 `prelude` L10190-10196（不等即拒出图） | 契约 JSON 内容 |
| selftest digest | `SELFTEST_TINY_DIGEST` L55-56 | canonical digest 算法 |
| **Stage A 18 格锚** | `milestones/g14/g14_3_stage_a_digest_anchor.json`（cornell/bistro × t50/67/100 × tsr/dlss/fsr 末帧 digest）；门 `ci/g14_production_caliber_stage_a_smoke.py` L66/L78 `stage_a_bitexact_probe` | **默认臂（g14_3_direct_gi SPV）输出位级** |
| G18 门纪律 | `ci/g18_rurix_light_transport_depth_smoke.py` L54-58：「默认 --gi off + 无 --presentation-profile 走 g14_3_direct_gi SPV（digest 锚红线 0-byte）」 | g18 SPV 只在加性 profile 下启用 |
| SPV sha256 登记 | `unified_provenance_json` L9986 起（逐 SPV 路径+sha256 入 receipt） | kernel 文件面 |

**避让规则**（既有加性臂纪律，--textures/--presentation-profile 已示范）：默认 off 时 ① 不读 NORMAL、不产 sink（`uv_out.is_some()` 门同律）；② 不换 SPV（`g14_3_pipeline_perf.rs` L346-347 的 `spv_scene` 替换先例）；③ SceneData/pack_tris/pack_mats/params 前 43 槽逐字不动；④ 新 kernel fork 新文件，母版 `g18_light_transport_depth.rx`/`g14_3_direct_gi.rx` 0-byte。

## 可行性结论 + 最小改动面清单

**结论：可行，且通路高度现成。** UV 侧表链（装配 sink → SSBO → kernel fork → 探针对拍 → on/off 门）每一环都有逐字模板；bistro 源数据 2062/2062 primitive 带 NORMAL；唯一全新内容是"重心插值法线用于 shading"这一数学面本身（插值式已有先例，归一化/双面翻转复用 g18 同式）。

最小改动面（全部加性、默认 off）：

1. **`g14_3_lane_body.rs`**：
   - `impl Gltf` 加 `normals()` 读取器（镜像 `texcoords` L1206，VEC3 版，~20 行）
   - `assemble_scene_ex` 加 `nrm_out: Option<&mut Vec<f32>>` 参数（镜像 uv_out；**注意世界变换**：法线需随节点世界矩阵旋转——`xform` 是带平移的点变换，法线要用旋转部分另变，quad 灯面尾段恒 0 同律）；或加 `assemble_scene_nrm` 包装（镜像 L1460）
   - 新 `G18NrmAssets`/侧表字节面 + 资源下标追加（镜像 L6663-6686 绑定表，加 `U_TRINRM`）
   - `pack_frame_params` 门控写 params[43]（CLI `--smooth-normals on` 时 = 1.0；默认 0 ⇒ kernel 走 flat 字面）
2. **新 kernel** `kernels/g18_smooth_nrm.rx`（或 g14_3 系新变体）：g18 逐字 fork + 第 9 个 view `trinrm: View<global, f32>`（9 f32/tri）+ `committed_barycentric()` + `n = bw·n0 + bu·n1 + bv·n2` → 既有归一/翻转式；`params[43]==0` 门走母版字面（缺省哑表保底读，镜像 `G34TexSideTable::default_face` L6882 纪律）
3. **SPV 选择**：`g14_3_pipeline_perf.rs` 加 `--smooth-normals` 闭集档 + spv_scene 替换（镜像 L346-347），与 gi/presentation 互斥登记
4. **对拍门**：新 `ci/g*_smooth_nrm_smoke.py` 照 g31_texture_sampling_smoke 八面判据瘦身——off 双跑 digest_seq 位级（= Stage A 锚零漂移机核）+ on 双跑位级 + on≠off + host 探针/全帧参考位级对拍
5. **禁动**：`g18_light_transport_depth.rx`、`g14_3_direct_gi.rx`、`pack_tris`/`pack_mats`、params[0..43]、Stage A 锚 JSON、FROZEN_CONTRACT_DIGEST 门序逻辑

**已知留窗**（需登记不冒充）：① cluster-lod/wp-hlod 组合面——cut 重排三角汤会破坏侧表序（UV 已有 `gather_tri_uv` L4348 模板，法线需同律 gather + 代理三角回退 flat）；② 法线世界变换的位级口径（旋转矩阵 f32 提取 vs 顶点烘焙的 f64→f32 路径）需与 host 参考逐 op 对齐；③ g34 拆散车道若要同特性，primary 腿已输出 bary（`out_bary`），shade 腿加侧表即可，改动比 mega 车道更小。