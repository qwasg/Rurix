===== 3fc9e896 =====
# Rurix 生产管线画质面测绘报告

## 0. 关键文件清单（全部为绝对路径）

- 车道 harness：`H:\rurix\src\rurix-render\src\bin\g14_3_pipeline_perf.rs`（入口/参数闭集）+ `H:\rurix\src\rurix-render\src\bin\g14_3_lane\g14_3_lane_body.rs`（共享实现，~57.7 万字符）
- 窗口呈现：`H:\rurix\src\rurix-render\src\bin\g31_window_present.rs`
- 生产 kernel 目录：`H:\rurix\src\rurix-render\kernels\`（76 个 .rx；生产链消费 `g14_3_*.rx`、`g14_mv.rx`、`g14_8_tsr_*.rx`、`g16_gi_multibounce.rx`、`g18_light_transport_depth.rx`、`g31_display_encode.rx`、`g31_texture_gi.rx`、`g26_framegen.rx`、`g31_mv_negate.rx`）
- 材质：`H:\rurix\src\rurix-render\src\material\{mod.rs, closure.rs, slab.rs, side_table.rs}`
- 阴影：`H:\rurix\src\rurix-render\src\shadow\vsm.rs`
- GI：`H:\rurix\src\rurix-render\src\gi\{multi_light.rs, restir_reservoir.rs, fallback_chain.rs, if_tier.rs}`
- 呈现底座：`H:\rurix\src\rurix-rt\src\vk_g31_present.rs`
- 待办总表：`H:\rurix\G31_PLUS_COMMERCIAL_RENDERER_TODO.md`

---

## 1. 生产帧的精确 pass 序列

**内部/输出分辨率**：`g14_3_lane_body.rs:9367-9369` — 内部分辨率 = `floor(输出 × tier/100)`，tier ∈ {50, 67, 100}（cornell 输出 512²，bistro 输出 1920×1080）。窗口路径默认 `--tier 100`（`g31_window_present.rs:3945`），即 1:1 不放大。

**TSR 臂（`tsr_device`，生产默认）= 统一单 session 四 pass**（`g14_3_lane_body.rs:5471-5472`、10877 行登记）：

| pass | kernel | 分辨率 | 输出格式 |
|---|---|---|---|
| pass0 scene | `kernels/g14_3_direct_gi.rx`（RayQuery compute，`#[numthreads(8,8,1)]`） | 内部 res | color 3×f32/px scene-linear HDR + depth 1×f32/px ZO NDC |
| pass1 mv | `kernels/g14_mv.rx` | 内部 res | 2×f32/px UV 偏移（**仅相机运动**，深度重投影） |
| pass2 tsr resample | `kernels/g14_8_tsr_resample.rx`（32×4） | 输出 res | cur_rgb 3×f32（×exposure 后"显示域"）+ luma + 最近邻深度 |
| pass3 tsr resolve | `kernels/g14_8_tsr_resolve.rx`（32×4） | 输出 res | 3×f32 时域累积输出 |

**cornell 拆散车道**（G14.10b，仅 `quad_count==1 && point_count==0 && 非 GI` 时启用，`g14_3_lane_body.rs:10690-10692`）：scene pass 拆成三 pass —— `g14_3_primary.rx`（主射线最近命中 → t+prim）→ `g14_3_shadow_scatter.rx`（y 维打包 16 层，每 invocation 1 条 first-hit 阴影射线 → blk∈{0,1}）→ `g14_3_shade_reduce.rx`（固定 0..15 序累加），加 mv+TSR 双 pass 共六 pass。bistro（点光场景）走 Mega 单 kernel。

**DLSS 臂**：scene → mv → 手编 pack SPV（RGBA32F/R32F/RG32F exportable image）→ OPAQUE_WIN32 导入 → DLSS `upscale_resident_external`（`g14_3_lane_body.rs:11033`）。**FSR 臂**：scene → mv → fsr_pack（color f16/depth f32/mv RG32F 写 D3D12 SHARED staging）→ ffx `dispatch_resident`（:11215）。

**窗口呈现路径**（`g31_window_present.rs:45-57`）= 生产五 pass：Mega 四 pass + **pass5 `kernels/g31_display_encode.rx`**（ACES 1.3 RRT+ODT.Rec709_100nits_dim → BT.1886 γ2.4 逆 EOTF → 8-bit 量化 → BGRA8 打包 u32/px SSBO）→ host 回读 8.3MB BGRA8 → `vk::ExternalImagePresent::present_rgba8`（staging buffer → `vkCmdCopyBufferToImage` 纯 transfer 上屏，`vk_g31_present.rs:249` 注释、:524）。swapchain = **B8G8R8A8_UNORM/R8G8B8A8_UNORM + FIFO（vsync）**（`vk_g31_present.rs:504-506`、:828、:1398）——UNORM 选择正确（kernel 已做 γ2.4，无双重 gamma）。`--fg x2/x3` 时扩为 8/10 pass（+`g31_mv_negate` + `g26_framegen` + 生成帧 encode，限 tier 100）。

---

## 2. 生产 kernel 的材质着色模型

**BRDF = 纯双面 Lambert 漫反射，零高光**。`g14_3_direct_gi.rx:310-312`：

```text
out_color[i*3] = hit_f * (mats[mb+3] + al_r * inv_pi * dir_r)  // emission + albedo/π · E_direct
```

- **每三角材质仅 8 f32 = [albedo.rgb, emission.rgb, 0, 0]**（kernel 头 :38 行）。**无 roughness/metalness/F0/不透明度消费槽**。
- **法线 = 逐三角平面几何法线**（边叉积，`g14_3_direct_gi.rx:129-146`），朝相机翻转（双面）。**无顶点法线插值、无法线贴图** —— 曲面必然呈 faceted 外观。
- 直接光几何项：quad 面光 `g = cos_s·cos_l/d²·(area/16)`（:238），点光 `g = cos_s/d²`（:295）。
- **纹理**：默认策略 `texture_mean_albedo=true`（bistro 契约行，`milestones/g13/g13_ue_upscale_parity_contract.json:118`）——DDS BC1/BC3 mip0 真实解码 → sRGB→linear → **整图塌缩成单一均值色**（`g14_3_lane_body.rs:1255-1258` `dds_mean_linear_rgb`）。`--textures on` 臂（`kernels/g31_texture_gi.rx`）= SSBO 图集（u32 RGBA8）+ 256 项 sRGB→linear LUT + 手动双线性 × mod（factor×(1−metallic)），但**仅 top-12 材质映射**（`G31_TEX_N_MAPPED = 12`，`g14_3_lane_body.rs:3951`）、仅 mip0、无 mip/aniso、**法线贴图不消费**（glTF 零 TANGENT，登记后续）、rough-metal 贴图 0/70 且无消费槽（`g31_window_present.rs:6243` 登记原文）。
- `material/closure.rs` 的 32B `MaterialClosure`（albedo/F0/rough/metal/AO/oct 法线/RGBE 自发光，:36-55）是**另一条（光栅/资产）线的格式，生产 RT kernel 不消费**。`material/slab.rs` = Substrate 双层 slab 的 **host 纯解析参考臂，"0-byte 不接线"**（:23-24）；`--slab-table` 臂只是把 slab 总反照率 R_slot **预乘进 albedo** 写回既有 mats SSBO（`g31_window_present.rs:115-122`），kernel 仍是 Lambert。`side_table.rs`（Burley 皮肤/Marschner 毛发参数）同为资产化 host 通道，不进生产着色。

---

## 3. 阴影

**生产 = 光追布尔硬阴影，无 PCF/PCSS/过滤**：

- 逐灯逐样本一条 **first-hit** 阴影射线（`ray_query_initialize_first_hit`，`g14_3_direct_gi.rx:221-235`），`vis = 1 − gate_far·blk` ∈ {0,1}。origin = p + wl·eps，t_max = d−2eps；eps = `clamp(场景包围盒extent×1e-4, 1e-3, 0.5)`（`g14_3_lane_body.rs:1888-1899`）。
- **quad 面光：固定 4×4=16 分层确定性采样**（`u=(sx+0.5)/4` 逐字，:185-186）→ 软阴影来自面积积分的 16 个二值样本。**采样图案逐像素逐帧完全静止**（无旋转/抖动）→ 半影是结构化 16 级量化条带，TSR 时域累积也洗不掉（每帧同一图案）。
- **点光：单射线 delta → 纯硬阴影**，边缘锯齿由 TSR 抗。bistro 生产 = 4 盏点光（契约 :63-88）+ 0 quad；cornell = 1 quad + 0 点光。
- `shadow/vsm.rs` = **"虚拟阴影贴图"（SVSM 式 clipmap 页，非方差阴影贴图）的 host 金标准**，`sample_shadow`（:687-717）= 选级 → 页表查询 → **最近邻单纹素硬 0/1 深度比较**，常量 `depth_bias=1e-3`（:53），未驻留/脏页保守返回 lit=1.0（漏影不漏黑）。**不进生产**（"device 接线属 W3" :20；TODO #104 登记 `vsm_page_mark_project` 曾"编进 SPV 无人 dispatch"）。
- `g18_light_transport_depth.rx`（--presentation-profile 臂）头注释宣称"point 灯 2×2 分层采样软阴影，半径 params[43]"（:3-4），**但 params[43] 在 kernel 体内零消费**（grep 证实）——文档/代码不符，实际只实现了 miss 天光（params[42]，:311-315）。

---

## 4. 直接光 + GI

- **灯模型**：仅 quad 面光 + point 点光。**方向光（太阳）不消费** —— 契约 `sun_intensity_lux=0.0/sky_intensity=0.0`；G10 语料里 300 lux 的 directional 只做一致性对拍后"delta 如实登记不消费"（`g31_window_present.rs:880-912`）。**无天空光、无环境项、无 AO**（生产 kernel 群 grep `ambient|occlusion|ssao|rtao` 零命中；miss → color=(0,0,0)）。
- **每像素遍历全部灯**，无 light culling/clustering（O(灯数)/像素；TODO #107/#108 登记缺失）。
- **自发光三角只在主命中时可见**（Lo 初值=emission），**不作为光源照亮他面**（无对 emissive 三角的 NEE）——bistro 的灯笼/灯具直接看会亮，但不投光（GI 臂里次级命中能拾到 emission，见下）。
- **GI 默认关**（`--gi off` 为默认 0-byte 锚，`g14_3_pipeline_perf.rs:54-58`；TODO #12"GI 默认档评估"未做）。`--gi on` = `kernels/g16_gi_multibounce.rx`：主射线直接光同式 + **固定 2 次余弦半球反弹**（Lambert ⇒ 吞吐 ×= albedo），次级命中点做 NEE（quad 仍 4×4 + point delta），**反弹辐射 clamp 到 16.0**（:524-526），反弹 miss = 能量丢弃（无天光）。无 RR、无 MIS、无降噪。
- AO：生产零。`apps/uc06-renderer` 的 `rtao` 标记只存在于另一（G7 帧链）应用，不进 Mega。

---

## 5. 明显画质弱点（按严重度排序）

1. **无高光/无 GGX/无 Fresnel** —— 全 Lambert，一切材质呈粉笔感；金属/粗糙度概念不存在于生产 kernel。
2. **平面法线 + 无法线贴图 + 无平滑着色** —— 曲面 faceted；bistro 70/70 法线贴图在树但零消费（无 TANGENT，登记后续）。
3. **默认纹理 = 整图均值单色**（texture_mean_albedo）——bistro 贴图细节全部丢失；`--textures on` 也只覆盖 top-12 材质、双线性、mip0。
4. **阴影**：点光 1 射线硬阴影锯齿；quad 灯 16 样本**静止分层图案** → 半影 16 级结构化条带（无逐像素/逐帧抖动，时域无法收敛掉）。
5. **GI 臂 RNG 是劣质 sin 哈希**：`g16_gi_multibounce.rx:318-320` —— `r1 = sin(px·12.9898 + py·78.233 + bnc·17.13 + jx)·0.5+0.5`，**无 fract/无乘大常数，输出呈 arcsine 分布非均匀**，且屏空间强相关（固定空间频率图案），余弦采样的 `r2` 非均匀 ⇒ GI 采样有偏 + 可见结构性噪声。帧间唯一变化是共享 jitter 加到相位上。
6. **无 GI（默认）+ 无 AO + 无天光/环境** —— 阴影区纯黑、miss 纯黑；emissive 不投光，bistro 实际只靠 4 盏点光照明。
7. **8-bit 量化零抖动**：`g31_display_encode.rx:441-443` `floor(v·255+0.5)` —— 暗部渐变必出色带；全 crate grep `dither|blue.noise|bayer` **零命中**。
8. **TSR 历史验证仅深度**（法线通道填均匀常量，验证 = 深度相对差 + 出屏，`g14_8_tsr_resolve.rx:150-155` 注释自承），`depth_rel_tol=0.1`（10% 相对容差偏松 → 细几何漏检/拖影）；MV **仅相机**（物体运动 MV 缺口 = A4 已登记项，`g31_window_present.rs:23-24`；蒙皮 MV 仅在 MegaSkin 臂）；disocclusion 直接回退当前帧（锯齿回归）；**无锐化**（RFC-0016 明文归 tonemap 后 pass，窗口链未实现）。
9. **曝光只支持 manual**（契约强制 `exposure.mode=="manual"`，`g14_3_lane_body.rs:531`），无自动曝光/histogram（post_chain 五级仅为 host 骨架，TODO #79 升 P0′）。exposure=2^(−ev100) 在 TSR resample 里乘（:10666）——**TSR 在 post-exposure 域累积历史**，EV 突变（窗口 `-`/`=` 键 ±0.25）时新旧曝光历史混blend，会有短暂亮度拖影。
10. **ACES f32 移植偏差已登记**（`g31_display_encode.rx:17-33`）：log10=log2×0.30103 截断、Rajan atan 逼近（≤0.086° hue 误差）等，ULP~1e-3 量级。bench/EXR 出图臂**无 tone mapping**（scene-linear 直出），只有窗口路径过 ACES。
11. **文档漂移两处**：g18 头宣称点光 2×2 软阴影（params[43]）但代码未实现；g16 头/注释说"次级 NEE 2×2"（:302）但代码跑 4×4（:421-424）。
12. FG 臂：仅相机 MV + 静态场景 + tier 100 限定；无 UI-aware 插帧（TODO #84）。HDR 输出未支持（maintain-SDR，TODO #17；swapchain 仅 SDR UNORM + FIFO）。

---

## 6. TODO / 留窗 / 已知限制登记（原文位置）

- `G31_PLUS_COMMERCIAL_RENDERER_TODO.md`：#9 纹理管线（部分兑现=top-12 臂）、#12 GI 默认档评估窗、#17 HDR、#27 SMRT 软阴影、#29 NRD 降噪、#79 后处理五级链 device 接线（P0′，"出货立刻缺的三件：自动曝光/mip bloom/ACES·AgX 进真窗口"）、#80 运动模糊/景深、#81 粒子/蒙皮写速度、#104 VSM 页管线生产接线、#105 PCSS、#106 缓存阴影、#107/#108 clustered 光照 + GPU light culling、#109 DDGI/probe volume、#110 bindless 贴图表。
- `g31_window_present.rs:6243`（缺面如实登记原文）：① sampler 对象不进 compute 生产车道（阶段矩阵限制，生产采样 = SSBO 图集 + 手动双线性）；② normal 贴图 70/70 但零 TANGENT ⇒ 法线贴图着色登记后续；③ rough-metal 贴图 0/70 且生产 Lambert 无消费槽。:6339：各向异性跨瓦片 N/A 登记、sampler feedback 未接（TODO #85）。
- `g14_3_direct_gi.rx:18-22`：无 GI/天光显式登记（"不冒充 GI 帧"）；GI kernel 面内容模型不同构不复用的评估登记。
- `g14_3_pipeline_perf.rs:77-81`：未消费登记 —— `compute_camera_mv` 留 host 的评估（后被 G14.10 GPU 化消费）、vendor evaluate 同步面不可消。
- `temporal/tsr.rs:36-42`：收敛加速（Resurrection）/拒绝抗锯齿质量档"归后续波次"；:13 深度感知 MV 上采样"归 P3 质量攻坚"。
- `g14_3_lane_body.rs:3927-3946`：B4 纹理资产盘点结论（albedo 70/70、normal 70/70 无 TANGENT、rough-metal 0/70）。
- 帧率侧（非画质但相关）：G17-MD-F1 bistro/t100/dlss_sr 对 UE5 ratio=0.960 诚实红（TODO #14）。

**一句话总结**：生产链是"确定性/位级对拍优先"的直接光 Lambert 光追器 —— 高光、法线细节、真实纹理、软阴影过滤、GI（默认）、AO、天光、自动曝光、抖动量化全部缺失或仅 host 参考臂在树；最大的低成本画质收益点 = ①quad 分层图案逐帧/逐像素抖动（TSR 可直接收敛）、②8-bit 前加三角抖动、③平滑顶点法线、④emissive 三角 NEE、⑤GI 臂换掉 sin 哈希。

===== 4298767c =====
# Rurix 显示/后处理/色调映射链精确测绘报告

## 1. 五级 post chain：定义、真实度、生产车道实际消费面

### 1.1 链定义（host 骨架，M119)

五级显式排序在 `src/rurix-render/src/display/post_chain.rs:166-197` 冻结为闭集：

```166:197:src/rurix-render/src/display/post_chain.rs
pub enum Stage {
    Exposure = 0,
    Bloom = 1,
    Tonemap = 2,
    ColorGrading = 3,
    OutputTransform = 4,
}
```

执行体 `PostProcessChain::process`(`post_chain.rs:316-367`）五级依次调用，HDR 域级（exposure/bloom）经 `HdrProbe::check_for_implicit_clamp`(`post_chain.rs:146-158`）检验隐式 SDR clamp。

### 1.2 各级真实度（host 骨架 vs device 兑现）

| 级 | host 骨架实现 | device 兑现 | 生产车道是否跑 |
|---|---|---|---|
| 1 exposure | `apply_exposure` = 标量乘 `2^EV`(`post_chain.rs:205-208`) | **折叠进 TSR resample kernel**(`kernels/g13_tsr_resample.rx:147-149` `o0 = (v0 * exposure).max(0.0)`) | **跑**（固定手动 EV，非 histogram) |
| 2 bloom | 3×3 box blur + 0.5 权重加性叠加（`post_chain.rs:212-233`)，注释自承"完整 mip 链在 device 面" | **不存在**（全仓 grep `bloom` 仅命中 post_chain.rs 与 harness) | **不跑** |
| 3 tonemap | 经 `ViewTransform` 插件（`post_chain.rs:338-341`) | `kernels/g31_display_encode.rx` = ACES 1.3 RRT+ODT 全式 f32 移植 | **跑**（仅 ACES 1.3，烧死） |
| 4 color grading LUT | 逐通道 slope/offset + `.max(0.0)`(`post_chain.rs:237-243`)，注释自承"完整 3D LUT 在 device 面" | **不存在** | **不跑** |
| 5 output transform | `encode_display_linear`(`view_transform.rs:125-154`)：SDR BT.1886 γ2.4 / scRGB / PQ 三闭集 | 同一 kernel 内 BT.1886 γ2.4 逆 EOTF + 8-bit 量化 + BGRA8 打包（`g31_display_encode.rx:440-451`) | **跑**（仅 SDR BT.1886 路径） |

### 1.3 生产车道（g14_3 / g31 window）每帧实际执行序列

**g31 真窗口车道**(`src/rurix-render/src/bin/g31_window_present.rs:5159-5161`):

```
hzb_primary→hzb_shade→mv→resample→resolve→display_encode   (HZB on)
scene→mv→resample→resolve→display_encode                    (HZB off)
```

- 第五 pass `g31_display_encode` 直读 TSR 输出 `U_OUT_COLOR[parity]`(`g31_window_present.rs:1002-1011`;kernel 头 `g31_display_encode.rx:49` "in_color: 3 f32/px(TSR out_color[parity] 驻留直读，零 host 往返）")。
- 编码参数由 host `aces13::aces13_device_encode_params(ew, eh, bgra)` 现算上传（`g31_window_present.rs:4901`;544B f32 块，布局见 `aces13.rs:425-485`)。
- 曝光：`exposure = 2^(-ev100)`(`g31_window_present.rs:5311`)，键盘 `-`/`=` 边沿触发 ±0.25、钳 [-8,8](`g31_window_present.rs:3786-3796`)，经 `pack_tsr_params` 128B 逐帧 uniform 进 TSR resample。**`ExposureState` 的 adapt 状态机（上 1.0/下 0.5 速率，`post_chain.rs:262-293`）在生产车道不被消费**——它只在 harness/出图臂存活。

**g14_3 性能/契约车道**(`src/rurix-render/src/bin/g14_3_pipeline_perf.rs` + `g14_3_lane/g14_3_lane_body.rs`)：四 pass 统一车道 = scene→mv→tsr resample→tsr resolve(`g14_3_lane_body.rs:5470-5473`),**链内无 tonemap**——契约强制 `rendering_policy.tonemap == "off"`(`g14_3_lane_body.rs:702-705`)，输出 HDR EXR/digest。唯一走完整 host 五级链的是 `--export-png` 出图臂（G18 M-b):`export_presentation_png`(`g14_3_lane_body.rs:8979-9027`）构造 `PostProcessChain { plugin: &Aces13, lut_slope: [1+warm_lift, 1, 1-warm_lift/2], ... }`，是离线 PNG 导出，不在实时帧环。

**结论：生产实时路径每帧跑的是 3 级（exposure→[TSR]→ACES tonemap→BT.1886 encode),bloom 与 LUT 两级在实时面完全缺位；完整五级链只在 M119 harness 与 g14_3 离线出图臂存在。**

## 2. 色调映射曲线与自动曝光

### 2.1 四内置插件（`view_transform.rs:170-181` 注册表并列）

| 插件 | 文件 | 参考公式 | 关键参数 |
|---|---|---|---|
| `aces13` | `display/aces13.rs` | AMPAS aces-dev v1.3 CTL 逐字：RRT.a1.0.3(glow 0.05/0.08 → red modifier 0.82/0.03/135° → AP0→AP1 → desat 0.96 → c5 样条）+ ODT.Rec709_100nits_dim(c9 样条 ODT_48nits → dim surround γ0.9811 → desat 0.93 → D60→D65 CAT) | 常量 `aces13.rs:25-38`;0.18 灰 → 显示线性 ≈0.104（测试锚 `aces13.rs:514-522`) |
| `aces20` | `display/aces20.rs` | aces-core `Lib.Academy.OutputTransform.a2.v1` 逐字：Hellwig2022 JMh + MM tonescale + chroma compress(reach M 表）+ gamut compress(cusp/upper-hull gamma 表，360 色相表构建期现算） | preset Rec709-D65 100nit BT1886(`aces20.rs:1286-1293`);CTL 怪癖逐字保留并注记（`aces20.rs:14-19`) |
| `agx` | `display/agx.rs` | Troy Sobotka iolite minimal 逐字：inset 矩阵 → log2 编码（min_ev=-12.47393/max_ev=4.026069)→ 6 阶 sigmoid → look → outset | look **资产化**(`AgxLook`,`agx.rs:47-69`),canonical = Punchy(power=1.35, sat=1.4)；禁硬编码有篡改探针（`agx.rs:184-193`) |
| `neutral` | `display/neutral.rs` | Khronos PBR Neutral 逐字 | 起始压缩点 0.76、desat 0.15(`neutral.rs:9-10`) |

**生产默认 = ACES 1.3，且是 device 面唯一兑现**：`g31_display_encode.rx` 只移植了 ACES 1.3;device 面没有插件选择机制（参数 SSBO 布局即 ACES c5/c9 样条专用）。AgX/ACES 2.0/neutral 只在 host 面存在。

### 2.2 自动曝光：非真

- 场景契约只许手动：`exposure.mode 仅 manual`(`g14_3_lane_body.rs:531-532`)。
- host 骨架注释自承："histogram 曝光（简化为标量乘 EV 偏移；**完整 histogram 计数在 device 面**,host 骨架用确定性 EV 映射维持 golden 等价性）"(`post_chain.rs:203-204`)——但 device 面没有任何 histogram kernel（全仓 grep `histogram` 在渲染面零命中，仅蒙皮统计埋点同名）。
- `ExposureState` 有帧间持久 + 上/下异速 adapt(up=1.0/down=0.5 硬编码，`post_chain.rs:269-270`)，但 `ev_target` 由调用方外给，无测光来源；生产车道不消费此状态机。
- TODO 已登记：`G31_PLUS_COMMERCIAL_RENDERER_TODO.md` #79 行（P0′)——"#3 只保证 EV 能进 uniform，不是 histogram 自适应；出货立刻缺的三件：自动曝光 / mip bloom / ACES·AgX 进真窗口"。

## 3. Bloom：生产路径无真 mip-chain bloom

- 全仓唯一的 bloom 代码 = host 骨架 `apply_bloom`(`post_chain.rs:212-233`):3×3 box blur,`out = px + blur/9 × 0.5`。**无阈值/无软膝**（对全图含深阴影模糊后加性叠加 50%，若真用会洗掉对比度）、**无 mip 链**、权重 0.5 硬编码。
- device 面零实现；g31 五 pass 序列无 bloom pass。
- TODO #61(P2）把 "bloom mip" 列为未来 async compute 候选；#79(P0′）列为出货缺口。

## 4. TSR / DLSS / FSR 细节

### 4.1 jitter

- 序列 = Halton(2,3) 居中到 [-0.5, 0.5)(`temporal/common.rs:36-40` `jitter_sequence`)；生产 g31 逐帧 `halton(jitter_base+fi+1, 2)-0.5`(`g31_window_present.rs:5312-5315`)，经 `jittered_vp` 叠加投影矩阵；g14_3 契约钉死 `jitter = "halton_static"`(`g14_3_lane_body.rs:710-711`)。

### 4.2 重采样（resample 腿）

- jitter 对齐 Catmull-Rom(a=-0.5 Keys)4×4 窗；核参缩放 `ratio>1 时 ×0.75` 规避 t=1 零点相位坍缩（`temporal/tsr.rs:184-193`)；抗振铃钳入 4×4 采集邻域 min/max(`tsr.rs:231`)；然后 `×exposure` 转显示域 + `max(0)`(`tsr.rs:232`)。
- device 兑现：`kernels/g13_tsr_resample.rx`(1D 调度）与 `kernels/g14_8_tsr_resample.rx`(2D 32×4 调度变体，数学逐字不变）,min/max 算术门无分支。

### 4.3 历史拒绝/裁剪（resolve 腿）

- 历史输出分辨率常驻、双缓冲；验证 = **深度相对差（容差 0.1)+ 出屏检测**;**法线判据恒过**——冻结接口无法线输入，填均匀常量（`tsr.rs:16-19` 文档 d 字面；device `g13_tsr_resolve.rx:176-182` 同语义）。
- YCoCg 3×3 邻域 AABB 裁剪（`common.rs:388-413`)；闪烁时域分析 = 亮度差分符号翻转 EMA（窗 16 帧，死区 abs 0.02/rel 0.1,tighten 0.5)，高闪烁区**松弛**邻域钳 + 收紧 alpha;reactive mask 优先（alpha→1)。`base_alpha=0.1 / min_alpha=0.04`(`tsr.rs:75-87` 默认即验收口径）。
- device 参数面 = 32 f32 块（`pack_tsr_params`,`g14_3_lane_body.rs:5433-5467` / `g13_tsr_device.rs:206-242`)，全部取 `TsrParams::default()` 硬编码。

### 4.4 锐化：无

- `tsr.rs:36-37` 明文："**不做锐化**(RFC-0016 §4.H2；锐化归 tonemapper 后可选 pass,Fortnite `r.Tonemapper.Sharpen=0.5` 先例）"——该 tonemap 后锐化 pass 不存在。全仓 grep `rcas` 零命中。
- 注意区分：`temporal/cas.rs` 的 `CasUpscaler` 是 **EASU 级空间超分 backend**（边缘自适应 3×3 + 深度不连续 sharpen 上限 0.65,`cas.rs:102`),M25 副 backend，与 `apps/uc06-renderer/kernels/cas_upscale.rx` 对拍，**未接入 g31 生产窗口车道**。

### 4.5 DLSS/FSR 集成点

- 冻结接口 = `temporal/upscale.rs` 的 `UpscaleBackend` trait(RFC-0016 §4.0-3;color/depth/mv/reactive/exposure/jitter/output_size/reset 八槽；历史内置双缓冲）。
- `src/rurix-render/src/bin/g13_vendor_upscale.rs`:`DlssBackend`(Streamline SDK 2.10.3,Vulkan interop,`"dlss_sr"`）与 `FsrBackend`(FSR 3.1.5,FFX SDK 2.0.0 DX12,`"fsr_3_1_5"`);g14_3 的 `--backend tsr_device|dlss_sr|fsr_3_1_5` 三臂。DLSS 已迁驻留统一车道（DlssResidentLane);FSR 臂维持 scene session + host mv + vendor host pack。
- 已登记语义修正（G14.10f):vendor 臂输出 `rgb × exposure` 转显示域，与 TSR resample `o=v·exposure` 同律（`g14_3_lane_body.rs:7276-7287`、`8024-8028`);FSR 3.1.5 LDR 路径不消费 pre_exposure（原 zero-exposure RED 注入无效已废止，`g13_vendor_upscale.rs:37-38`)。

## 5. 画质弱点清单（优化机会）

1. **8-bit 量化无 dither**:`g31_display_encode.rx:441-443` `floor(v^(1/2.4)·255+0.5)` 直出，无三角抖动/蓝噪声 → 暗部与天空渐变必带 banding。全仓 display/kernels 面 grep `dither|grain` 零命中。
2. **SDR 编码 = 纯 BT.1886 γ2.4 逆 EOTF，非 sRGB 分段曲线**(`view_transform.rs:127-134` + kernel 退化形 `v^(1/2.4)`)。若 OS/swapchain 把 BGRA8 缓冲当 sRGB 解释，暗部系统性偏暗（γ2.4 vs sRGB≈γ2.2 分段）——值得实测确认 present 腿（D-130 C++ shim）的色彩空间声明。
3. **曝光变化时 TSR 历史域不一致**：历史在**显示域**（后曝光）常驻（`upscale.rs:45-47` 明文 FSR2/TSR 后曝光口径）,resample 处 `×exposure` 后累积；手动 ±0.25 调曝光时历史与新帧处于不同曝光域，无曝光除法补偿 → 调曝光瞬间出现拖影/脉冲，需若干帧收敛。
4. **bloom 骨架即便接线也有质缺**：无阈值（深阴影也发光）、3×3 单尺度（无 mip 链宽辉光）、加性 0.5 非能量保守设计。
5. **LUT 级非真 3D LUT**：逐通道 slope/offset(`post_chain.rs:237-243`）无法做色相旋转/通道串扰；`.max(0.0)` 会压掉 tonemap 后轻微负值（AgX outset 可产负值）。
6. **device 面 tonemap 烧死 ACES 1.3**:AgX/ACES 2.0 无 device 兑现；AgX Punchy look 已资产化（好先例）但生产用不上。
7. **无 HDR ODT**:ACES 1.3 ODT 钉死 Rec709_100nits_dim(c9 = ODT_48nits 影院色调尺度）;ACES 2.0 preset 钉死 100 nit;PQ 路径虽在 host 编码面存在（`view_transform.rs:143-152`)，但插件输出语义 1.0=100nits，无 1000/4000-nit ODT 变体 → HDR 输出管线整体留窗（TODO #17，本机显示链 HDR10 token 全 absent)。
8. **硬编码参数面**:TSR 全部旋钮取 `TsrParams::default()`;adapt 速率 1.0/0.5、bloom 权重 0.5、AgX Punchy、ACES 常量全部编译期冻结，无运行时画质档。
9. **无胶片颗粒（grain)**：全链无 grain pass。
10. **f32 移植偏差已登记**(`g31_display_encode.rx:17-33`):log10=log2×0.30103、Rajan atan 逼近（max 0.086°) 等 ULP~1e-3 量级差——位级确定性不受影响，但与 host f64 golden 不互等（digest 语义分列，G31BGRA-1 vs A1 host 域不冒充同值，`g31_window_present.rs:6859`)。
11. **ACES 1.3 固有 hue-skew / ACES 1.3↔2.0 版本差**:D4 R-D4-5 已知差异记录，实测入 golden 带不作 bug 返工（`aces13.rs:15-16`、`g9_m118_display_pipeline.rs:190-241`)。
12. **ACES 2.0 逐字保留 CTL 怪癖**(`aces20.rs:14-19`):`min_index=1` 字面、previous 恒 {0,0}、last_idx 零初始化口径——保真 CTL 但继承了其怪癖面。

## 6. 已知限制 / 留窗登记（原文位置）

- `display/swapchain.rs:125-133`:HDR 能力查询面 unwired(D-130 C++ shim 0-byte),`HdrCalibrationStatus::NotTriggered` 显式登记不充绿；强制消费 → typed Err。
- `display/post_chain.rs:26`：与 TAA/TSR 时域链的显式排序只落接口面，"时域底座消费 M24 字面 0-byte"。
- `display/post_chain.rs:203-204 / 210-211 / 235-236`:histogram 计数、mip 链 bloom、3D LUT 三处均注记"在 device 面"——当前波 device 面无对应实现。
- `display/hair.rs:22-27、457-481`：毛发 strand 档强制精确 OIT 分项 not-triggered 登记（M120 测量冻结带数据可得性不足，不充绿）。
- `G31_PLUS_COMMERCIAL_RENDERER_TODO.md`:
  - **#79(P0′)**：后处理五级链 device 接线 [遗漏]——"实时窗口未恒走完整五级"；缺自动曝光/mip bloom/ACES·AgX 进真窗口。
  - **#61(P2)**：跨帧 async post(bloom/tonemap 走 compute 队列与下帧几何重叠）未接线，依赖 #57+#79。
  - **#17**:HDR 输出管线现 maintain-SDR（本机 HDR10_ST2084/BT2020_LINEAR/HDR10_HLG 全 absent)。
  - **#3**：游戏循环最小面（逐帧 jitter/曝光/灯光 uniform）已部分兑现（g31 的 ±0.25 手动 EV 即此面）。
- `g13_vendor_upscale.rs:37-38`:FSR 3.1.5 LDR 路径不消费 pre_exposure（留痕防回归）。
- 附注：`display/skin.rs`(M115 Burley 屏单 pass separable SSS,16 tap,32×32 pre-integrated LUT 回退档）与 `display/hair.rs`(M114 Marschner R/TT/TRT 三瓣）属 G9.5 display 波次下的**着色模型**面，不是 post chain 级；两者均为 host 语义面 + RED 臂，device 对拍归后续波。

**一句话总结**：实时生产链 = `TSR(Halton jitter + CR 重采样 + ×固定EV)→ ACES 1.3(device f32)→ BT.1886 γ2.4 + 8bit 无量化抖动`；五级链的 exposure-adapt/bloom/LUT 三级在实时面缺位，是 TODO #79 登记的 P0′ 缺口；最立竿见影的画质优化点 = 量化前加 dither、确认 swapchain sRGB/BT.1886 色彩空间匹配、曝光变化时 TSR 历史域补偿、bloom 阈值+mip 链 device 兑现、tonemap 后锐化位（RCAS 或同等）。

Now write the full report in Chinese.

===== 84d58325 =====
# Rurix 工作区构建 / Benchmark / 验证 全链路测绘报告

## 1. 构建（Build）

### 工作区布局
- 根 `H:\rurix\Cargo.toml:5` — 31 个 member crate，关键者：`src/rurixc`（编译器）、`src/rurix-rt`（Vulkan 执行底座）、`src/rurix-render`（渲染器 + 全部 bench harness bin）、`src/image-io`（EXR）。edition 2024（`:8`），workspace 级 `unsafe_code = "deny"`（`:14`）。

### 生产 bench 二进制
- `g14_3_pipeline_perf` 由 crate **rurix-render** 构建：`src/rurix-render/Cargo.toml:301-304`，`required-features = ["vendor-upscale"]`。feature 定义在同文件 `:22-29`（`default=[]`；`vulkan`；`vendor-upscale = vulkan + rurix-rt/vendor-upscale`）。
- **rurix-render 没有 build.rs**（全仓库 9 个 build.rs 均不属于它），无 codegen 前置步骤。

### .rx kernel 编译链（重点：**SPV 不入 git**）
- kernel 源在 `src/rurix-render/kernels/*.rx`（如 `g14_3_direct_gi.rx`、`g14_mv.rx`、`g14_8_tsr_{resample,resolve}.rx`、`g14_3_{primary,shadow_scatter,shade_reduce}.rx`）。
- 全仓库 `*.spv` 仅存在于 JoltPhysics vendor 目录；`.tmp/` 被 `.gitignore:48` 忽略 ⇒ **kernel 不是预编译提交**的。
- 编译方式 = 现场用 rurixc：`rurixc <src.rx> --target vulkan -o <out.spv>`，见 `ci/g14_rurix_pipeline_perf_smoke.py:247-253`；rurixc 本身用 `cargo build -p rurixc --features vulkan-backend --bin rurixc` 构建（`:225-231`，feature 定义 `src/rurixc/Cargo.toml:38`）。
- 默认 SPV 路径常量在 `src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs:60-76`（`.tmp/g14_gates/m_c/*.spv`）。CI 门的 `_ensure_spv()`（`ci/g14_rurix_pipeline_perf_smoke.py:256-272`）会对缺失 SPV 自动补编译——**所以跑 CI 门时 cargo build + 门脚本即可；手工直跑二进制则需先保证 SPV 存在**。
- 结论：`cargo build` 单独可产出二进制；**运行** bench 还需 ① SPV ② 场景资产 ③ vendor DLL。

### 外部运行依赖
- 场景资产（硬编码默认，`g14_3_lane_body.rs:9113-9121`）：bistro = `K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf`；契约 = `milestones/g13/g13_ue_upscale_parity_contract.json`（`:58`）。
- DLSS：Streamline 2.10.3（`sl.interposer.dll`/`sl.common.dll`/`sl.dlss.dll`/`nvngx_dlss.dll`），目录取 env `RURIX_STREAMLINE_SDK_DIR` 或默认 `external/streamline-2.10.3`（`src/rurix-rt/src/vendor_upscale.rs:80-81, 1797-1818, 7594`）。
- FSR 3.1.5：FFX SDK 2.0.0 的 DX12 DLL（FSR 2.x 已移除 Vulkan 通道），env `RURIX_FSR_SDK_DIR`（`vendor_upscale.rs:5-13, 83`）。

## 2. Benchmark

### CLI（参数解析：`src/rurix-render/src/bin/g14_3_pipeline_perf.rs:203-334`）
- 子模式闭集：`--dump-scene` / `--contract-digest` / `--selftest-digest` / `--render` / `--bench`（`:119-124`）。
- `--bench` 必填 `--scene <cornell-box|bistro-interior> --tier <50|67|100> --backend <tsr_device|dlss_sr|fsr_3_1_5>`（`:335-337`）；默认 `--frames 160`（`:521`）、`--warmup 10`（`:209`）、`--inflight 1`（`:210`）。tier = 内部分辨率百分比，输出 1920×1080。
- `--inflight` 只接受 1|2|3，且 **仅 `--bench --backend tsr_device` 已接线**（`:349-358`），要求 `warmup ≥ N−1`（`:359-363`）。
- 用法块见 `:90-111`；`--gi on`、`--dyn-demo`、`--skin-demo`、`--cluster-lod`、`--wp-hlod`、`--profile-json` 各有 fail-closed 闭集校验。

### 输出位置
- stdout 汇总行：`BENCH PASS scene=… frame_ms_mean=… cv=… fps=… scene_gpu_ms_mean=… prod_ms_mean=… tail_ms_mean=…`（`g14_3_lane_body.rs:13743`）。
- receipt JSON：`<out-root>/<scene>/tier<N>/<backend>/bench_receipt.json`（`:13702`），schema `rurix.g14.pipeline_perf_bench_receipt.v1`，含逐帧数组 `frame_ms/scene_gpu_ns/cpu_record_ns/cpu_submit_ns/cpu_fence_wait_ns/tail_ms` + `stats_post_warmup.frame_ms_production_mean` + `last_frame_digest`（`:13651`）。默认 out-root = `K:/rurix-ext/g14-frames/rurix_prod`（`:59`），可用 `--out-root` 改。
- `--render` 产 32 帧 Halton 收敛序列 + `converged.exr` + `render_receipt.json`（`converged_digest`）。

### CI 驱动脚本
- `ci/g14_rurix_pipeline_perf_smoke.py` — M-c 门：`run_bench()`（`:114-150`）以 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1` 跑 160 帧/warmup 10；全门 = 2 场景×3 档×3 后端×3 轮 = 54 次 bench（`:309-361`）+ 双跑位级 + RED 三臂。
- `ci/g31_wave_a_anchor_check.py:81-105` — canonical 单格 bench 面（gpu_device_lock 串行）。
- `ci/g31_frame_pipelining_smoke.py` — `--inflight 1/2/3` A/B 门（`:54-59`）。
- `g31_window_present` 有 `--headless-smoke`（`src/rurix-render/src/bin/g31_window_present.rs:85, 4103`）——无窗口退化仅供自检，**不计真门**（`:127-128`）；`--frames N` 自动退出。

### 实测运行时长（evidence 在案）
- 单格 160 帧 bench ≈ 30 s 量级；锚检门 18 格 `wall_s_total` = 578~622 s（`evidence/g31_wave_a_anchor_check_*.json:222`，多次在 578.4/591.5/594.4/603.1/618.4/622.4 s）。完整 g14 M-c 门（54 bench + render + RED）显著更长，脚本内单命令 timeout 7200 s。

## 3. 验证（Verification）

### 确定性 digest（Stage A 锚）
- 黄金锚文件：`milestones/g14/g14_3_stage_a_digest_anchor.json` — 18 格 `last_frame_digest`（如焦点格 `bistro-interior_t100_dlss_sr` = `sha256:55ea0c2b…`，`:55-57`）；2026-08-22 双跑位级重收割（`:62-67`）。
- 核验门：`py -3 ci/g31_wave_a_anchor_check.py --gate` — 18 格逐格重跑对拍，要求 18/18 位级一致（`:176-198`）；焦点格 fresh ratio ≥ 在案 0.960479（`:59-61, 200-213`）。
- 单格自查可用二进制的 `--expect-digest <sha256:…>`（`g14_3_pipeline_perf.rs:279`）。

### 容差/预算门
- 预算文件：`milestones/g*/g*_budget.json` 共 36 个（含 `g14_budget.json`、`g35_budget.json`），统一由 `ci/budget_eval.py` 评判（门内调用见 `ci/g14_rurix_pipeline_perf_smoke.py:500-502`）。
- 阈值口径：帧时守护阈 = 实测 ×1.5（`:467`）；焦点格条目 `g14.pipeline_perf.frame_ms.bistro-interior_t100_dlss_sr`：measured 4.195032 ms / threshold 6.292548 ms（`milestones/g14/g14_budget.json:346-355`）；验收预算杠 = 在案 3.5767 ms ×2.0 = 7.1534 ms（`milestones/g33/g33_budget.json:31`）。

### EXR diff 工具链
- Rust 报告器：`src/rurix-render/src/bin/g10_m137_diff_report.rs` — 逐像素误差缓冲 + 16×16 区域统计 `err_max/err_mean/err_p95`（`:244-292, 478-480`）+ 误差 EXR/PPM 热区图 + evidence JSON（`:541-542`）。
- Python 侧：`ci/g10_exr_lib.py`（`decode_exr_file` `:329`，`nearest_rank_p95` `:352`）、`ci/g10_ssim_psnr_lib.py`（Wang2004 SSIM）、`ci/g10_pixel_diff_report_smoke.py`（字段闭集 `:68-71`）。口径规范：`spec/visual_comparison.md`。
- 画质锚：G14.3 车道 vs G13.4 converged.exr 的 SSIM deficit ≤ 锚定带（`ci/g14_rurix_pipeline_perf_smoke.py:405-440`）。

## 4. 性能已知面（Performance knowns）

### G17-MD-F1 红cell
- 格 = `bistro-interior/t100/dlss_sr`；在案锚 frame_ms = **3.5767 ms**（G30.2 M-b）、UE 中位 = **3.43535 ms**、在案 ratio = 0.960479（`ci/g31_wave_a_anchor_check.py:59-61`）。轨迹 0.856→0.960→0.966→0.957894→0.921836（G34 收口日恶化如实登记，`milestones/g34/G34_CONTRACT.md:129`）。
- 根因分解在案：`milestones/g31/g31_ngx_decomposition_report.md` — NGX in-stream 1.837 ms 不可分离等量；Δ=+0.1521 ms 全落宿主可分离段包络 ≈0.707 ms。

### 帧流水化状态
- **已实现**：`submit_with_frame_update`（`src/rurix-rt/src/render_exec.rs:1632`）/ `collect`（`:1728`）分离，per-slot cmd/timestamp/staging；消费面仅限 `g14_3_pipeline_perf --bench --backend tsr_device --inflight 2|3`（`g14_3_pipeline_perf.rs:354-357`），车道侧 `submit_frame`/`collect_frame` 在 `g14_3_lane_body.rs:7157-7212`。FIF 入口 fail-closed 拒 `tlas_update`/`blas_refit`（`render_exec.rs:1651-1666`）。
- **未消费**：`g31_window_present` 仍走当帧 fence 全同步（`G31_PLUS_COMMERCIAL_RENDERER_TODO.md:231`，#89 项）；host 侧 `compute_camera_mv` ~5.5 ms@bistro t67 未移植（`g14_3_pipeline_perf.rs:77-81`）。

### GPU timestamp profiling
- `DeviceFrameTelemetry`（`render_exec.rs:933-962`）：逐 pass `PassGpuTiming` + `cpu_record_ns/cpu_submit_ns/cpu_fence_wait_ns`；`vkCmdWriteTimestamp2` 注入点 `:5851, :5996`；`timestampPeriod` 实采 `:4721`。C7 profiler 输出面 `--profile-json`（仅 tsr_device 静态 inflight=1，`g14_3_pipeline_perf.rs:286, 436-446`）。

### 生产路径显式同步点
- 顺序入口每帧 `vkWaitForFences(frame completion)`（`render_exec.rs:9339`）；slot 复用等待 `:9027, :9852`。
- vendor 面：`queue_wait_idle` ×5（`vendor_upscale.rs:3303, 3485, 3592, 4288, 4748`）+ `device_wait_idle`（`:5139`）——DLSS/FSR evaluate 固有同步（实测 FSR ~18-27 ms、DLSS ~18 ms@1080p 输出档，`g14_3_pipeline_perf.rs:80-81` 登记为「不可消」）。

## 5. Windows PowerShell 命令清单

```powershell
# ① 一次性:构建 rurixc 并编译 7 件 SPV(CI 门会自动补,手工直跑需要先产)
cargo build -p rurixc --features vulkan-backend --bin rurixc
.\target\debug\rurixc.exe src\rurix-render\kernels\g14_3_direct_gi.rx --target vulkan -o .tmp\g14_gates\m_c\g14_3_direct_gi.spv
# 其余: g14_mv.rx / g14_8_tsr_resample.rx / g14_8_tsr_resolve.rx /
#       g14_3_primary.rx / g14_3_shadow_scatter.rx / g14_3_shade_reduce.rx (同式)

# ② release 构建(生产 bench 二进制)
cargo build --release -p rurix-render --bin g14_3_pipeline_perf --features vendor-upscale

# ③ TSR bench(bistro-interior 1080p t100;加 --inflight 2 测流水臂)
$env:RURIX_REQUIRE_REAL="1"; $env:RURIX_VK_VALIDATION="1"
.\target\release\g14_3_pipeline_perf.exe --bench --scene bistro-interior --tier 100 --backend tsr_device --frames 160 --warmup 10

# ④ FSR / DLSS bench
.\target\release\g14_3_pipeline_perf.exe --bench --scene bistro-interior --tier 100 --backend fsr_3_1_5 --frames 160 --warmup 10
.\target\release\g14_3_pipeline_perf.exe --bench --scene bistro-interior --tier 100 --backend dlss_sr --frames 160 --warmup 10

# ⑤ digest 检查(18 格 Stage A 锚 + 焦点格 ratio;约 10 分钟)
py -3 ci\g31_wave_a_anchor_check.py --gate
# 单格快速 digest 对拍:
.\target\release\g14_3_pipeline_perf.exe --bench --scene bistro-interior --tier 100 --backend dlss_sr --frames 160 --warmup 10 --expect-digest sha256:55ea0c2ba68011727b4136ecb32c627e36d539bb38a2aadad617bb17cb578d4a
```

预期耗时：单格 160 帧 ≈ 30 s；锚检门全程 ≈ 10 min（在案 578-622 s）；完整 g14 M-c 门（54 bench + 双跑 + RED 三臂）为小时级。注意缺设备/资产/DLL 时二进制走 `SKIP DEV_ENV_DEGRADE` 退 0，`RURIX_REQUIRE_REAL=1` 下缺真实面即 FAIL 退 1（`g14_3_pipeline_perf.rs:83-88`）。