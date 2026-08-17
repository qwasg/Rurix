# G11.5b bistro LDR 域残差分解诊断（设计/取证文档）

> 波次：G11.5b 追加子波（主会话裁决：先诊断修复后评 metric，禁改判据充绿）。
> 输入帧锚：G11.5 复测帧区 `K:\rurix-ext\g11-frames\g11_5`（双端 HDR/LDR，只读消费）；
> G11.5b 诊断帧区 `K:\rurix-ext\g11-frames\g11_5b\diag|ue_diag|preview`。
> 全部数字来自命令输出（驱动：`milestones/g11/harness/g11_5b_ldr_diag.py`，产物
> `g11_5b/diag/diag_out.json`；bin 诊断面 `--diag-ldr-stages/--diag-aces13-sweep/--diag-sky-vis`；
> UE 诊断面 `ue_python/g11_5b_diag_scenes.py` + MRQ 两臂）。
> R1 锁定基线：ssim@bistro-interior(ldr) delta = **0.8328980787837229**（ssim 0.16710192121627712）；
> G11.5 首跑复测 ssim = **0.010847362392386794**（delta 0.9891526376076132，反向增大，整波 FAIL 停线）。

---

## 0. 结论摘要（主因排序，实测）

| # | 残差成分 | 实测份额 | 处置 |
|---|---|---|---|
| 主因 | **天光漫反射全向 IBL 在 Rurix 侧缺失**（UE SkyLight 实质无遮蔽全向投递 vs Rurix 仅 GI 探针真可见性采样） | UE 帧均值的 **95.4%** 由 SkyLight 单独承载（sky0 臂）；双端 HDR 中位差 **147.7×** | **本波修复面**（`--sky-ibl`，RXS-0397） |
| 次因 | 太阳穿玻璃小区域（UE CSM 不遮蔽半透明玻璃；Rurix TLAS 全遮蔽） | 仅高光尾（UE max 57.29 vs Rurix 4.77；p90 以下无感） | 登记 G11.6 P2 候选（不展开） |
| 残余 | UE SkyLight 镜面 IBL（c1_ue_specular_ibl 登记项） | 实测 **≤0.03%**（nospec 臂帧均值份额 0.000306）——本配置下可忽略 | 维持 G15 画质量级收口面候选登记（本次附实测上界） |
| 排除 | view transform / sRGB 派生链 | 双端同一 host 派生链逐位同构（stage3 双端 == G11.5 LDR 帧逐位一致） | 非发散段，不修 |
| 排除 | 玻璃窗遮挡（诊断前主假设之一） | 玻璃材质半球遮挡份额仅 **1.43%** | 假设证伪（非主因） |

修复预览（preview 帧实测，正式复测面 = g11_5b 复跑驱动）：bistro **SSIM 0.3344040417570749 > 0.16710192121627712（R1 收敛阈）**；R3/R4/U2 残余 delta 同号续降；cornell 全行维持。

---

## 1. ① HDR→LDR 逐段分解（发散段定位）

派生链 = HDR × exposure（双端 ×1.0，C2 对齐面维持）→ aces13 view transform（RFC-0026 F3 双端同一字面）→ 共享 sRGB 编码器。`--diag-ldr-stages` 对 G11.5 四张 HDR 帧逐段落盘（stage1 曝光后 / stage2 view transform 后显示线性 / stage3 sRGB 后），双端逐段亮度统计（命令：`g10_5_scene_render.exe --diag-ldr-stages …`，产物 `g11_5b/diag/*_stage{1,2,3}_*.exr`，统计驱动 `g11_5b_ldr_diag.py`）：

### bistro-interior（亮度统计 = Rec.709 亮度，median/p90/mean/max）

| 段 | Rurix median | UE median | 中位比 UE/Rurix | 均值比 UE/Rurix |
|---|---|---|---|---|
| stage1 曝光后（= HDR 输入，×1.0 恒等） | 0.003273 | 0.483505 | **147.75×** | 113.94× |
| stage2 view transform 后 | 9.4e-05 | 0.354965 | **3787.74×** | 453.43× |
| stage3 sRGB 后（= LDR） | 0.001211 | 0.625142 | **516.31×** | 79.09× |

stage1 全量：Rurix p10=0.000432 / p90=0.011101 / mean=0.005670 / max=4.768794 / nonzero=0.994367；UE p10=0.119240 / p90=1.228093 / mean=0.646051 / max=57.285356 / nonzero=1.0。

**判读**：发散在 **stage1 输入段已全额存在**（147.75×）；aces13 趾部把深阴影非线性压垮（stage2 比值放大到 3787.74×——sweep 实测 0.001→0、0.003→4.44e-05），sRGB 段把阴影抬回（516×）。**LDR 域塌陷是 HDR 域能量赤字经 ACES 趾部放大的结果——派生链本身零缺陷**：stage3 与 G11.5 已派生 LDR 双端**逐位一致**（`stage3_vs_g11_5_ldr_bitexact = true ×2`，命令输出）。

### cornell-box（对照，已收敛场景）

| 段 | 中位比 UE/Rurix | 中位 delta |
|---|---|---|
| stage1 | 1.5793× | 0.036303 |
| stage2 | 1.9690× | 0.020522 |
| stage3 | 1.4936× | 0.074604 |

对照含义：HDR 输入接近（1.58×）时同一派生链产出接近的 LDR（SSIM 0.5827）——再次坐实发散段 = HDR 输入，非链路段。

## 2. ② tone mapping 曲线对拍（UE 侧实际应用曲线实测取证）

**链路事实（三证）**：
1. 双端 LDR 由同一 host 派生链产出（`g10_5_scene_render --derive-ldr`，双端 ×1.0，同一 `Aces13` + 同一 `srgb_encode` 代码路径）——G11.5 复跑报告 derive 段四件命令输出在档；
2. UE MRQ 配置 `disable_tone_curve = True`（`g10_5_build_scenes.py` 源码字面，5.8 源树 MoviePipelineEXROutput.cpp bDisableToneCurve → SCS_FinalColorHDR 实证面）⇒ UE EXR = tonemap 前 scene-linear；**实测佐证**：UE bistro HDR max = 57.285356 > 1.0（display-referred 帧不可能超 1）；
3. 曲线单源实测：`--diag-aces13-sweep`（与派生链同一代码路径）中性锚点：0.18 → display 0.104064 / sRGB 0.355954；1.0 → 0.624316 / 0.811971；3.0 → 0.863517 / 0.937427；30 → 1/1（钳）。

**双端真实帧经验映射分桶对拍**（HDR 亮度分桶 → LDR sRGB 中位；命令输出摘）：

| HDR 桶 | Rurix hdr→ldr | UE hdr→ldr | sweep 中性参照 |
|---|---|---|---|
| (0.03,0.1] | 0.04257→0.10622 | 0.07590→0.18160 | 0.03→0.071909 / 0.09→0.209206 |
| (0.1,0.3] | 0.12259→0.26664 | 0.18410→0.36122 | 0.09→0.209206 / 0.18→0.355954 |
| (0.3,1] | 0.66990→0.72768 | 0.48652→0.62690 | 0.36→0.550276 / 1→0.811971 |
| (1,3] | 1.09397→0.82726 | 1.17317→0.83838 | 1→0.811971 / 3→0.937427 |

双端同桶映射均落在同一 sweep 曲线上（桶内色度分布差致小偏差）——**双端实际应用的 tone 曲线同构实测成立，曲线面零残差**。

## 3. ③ 天光/镜面 IBL 贡献量分离实测（UE MRQ 诊断两臂）

诊断臂（`g11_5b_diag_scenes.py` + `g11_5b_ue_render.py`；契约地图/序列/后处理逐字同构，仅开关目标维度；输出 `g11_5b/ue_diag/`）：

- **V_sky0**（SkyLight intensity=0，诊断地图 G11_DiagBistroSky0 = .umap 磁盘字节复制 + 单属性改写）；
- **V_nospec**（MRQ config 追加 `r.Lumen.Reflections.Allow=0` + `r.SSR.Quality=0`，复用契约地图/序列）。

分解（对 G11.5 UE 基帧 .0000.exr 逐像素差分；基帧 digest `sha256:92ed5dff…` 与 G11.5 报告登记逐位一致）：

| 量 | mean | median | p90 | 占基帧均值份额 |
|---|---|---|---|---|
| 基帧 base | 0.646051 | 0.483505 | 1.228093 | 100% |
| sky0 残帧 | 0.029850 | 3.54e-05 | 0.000175 | 4.6% |
| **sky_total = base − sky0（SkyLight 全贡献）** | **0.616236** | **0.476768** | 1.217753 | **95.39%** |
| spec_path = base − nospec（SSR/Lumen 反射路径） | 0.000198 | 0.0 | 0.0 | **0.031%** |
| sky_diffuse = nospec − sky0（天光漫反射分量） | 0.618063 | 0.478987 | 1.218460 | 95.67% |
| sky_specular = sky_total − sky_diffuse | 0.000198（≤ 交互相噪声量级） | 0.0 | 0.0 | ≤0.031% |

**判读**：
- UE 帧能量 **95.4% 由 SkyLight 单独承载**，且其中**漫反射 ≥99.9%、镜面 ≤0.03%**——已登记残余 `c1_ue_specular_ibl`（UE SkyLight 镜面 IBL 结构差）在本配置下实测可忽略，维持 G15 候选登记（本次附实测上界 0.031%）。
- sky0 残帧中位 3.54e-05 **比 Rurix 当前帧中位 0.0033 还低约 100×**——UE 关掉天光后比 Rurix 现状更暗（UE 此配置 GI 全关，无多反弹），反证 Rurix 的 GI 链在搬运能量、但搬运量被真可见性（§4 审计 = 3%）卡死。

**UE 侧机制取证（探针读回，`g11_5b/diag/ue_probe_bistro.json`，命令输出）**：
- SkyLight 组件：`source_type = SLS_SPECIFIED_CUBEMAP`、`intensity = 5.0`、cubemap = `/Game/G10/white_2x1`（TextureCube）、`cast_shadows = true`、**`lower_hemisphere_is_black = true`**、movable、real_time_capture=false；
- 管线 cvar 实测：`r.DynamicGlobalIlluminationMethod = 0`（**Lumen GI 关**）、`r.ReflectionMethod = 2`（**SSR**）、`r.GenerateMeshDistanceFields = 0`（**距离场关 ⇒ DFAO/Lumen 场景均不存在**）、`r.Lumen.ScreenProbeGather.Allow = 0`、`r.SSR.Quality = 3`。

**机制结论**：该 UE 配置下 movable SkyLight（指定 cubemap）**无任何遮蔽机制可消费**（Lumen 关、距离场关）⇒ 天光按**全向无遮蔽 IBL** 投递（下半球黑），SSR 只管镜面且 SSR 看不见 cubemap 天空 ⇒ 镜面天光 ≈0。这就是 C1 登记「UE SkyLight 指定 cubemap 全向 IBL」字面的运行时实证。

## 4. ④ diff 热区空间分布 + Rurix 天空/太阳可见性审计

**UE 亮度分位三区残差**（bistro LDR；p10=0.260793 / p90=0.845354 分界）：

| 区 | px | UE 均值 | Rurix 均值 | Rurix/UE |
|---|---|---|---|---|
| 阴影（<p10） | 207361 | 0.175887 | 0.003985 | 2.27% |
| 中间调 | 1658880 | 0.627694 | 0.006688 | **1.07%** |
| 高光（>p90） | 207359 | 0.863916 | 0.019150 | 2.22% |

32×18 块区 log10(ue/rurix) 中位网格（`diag_out.json` diff_heatmap.grid + PNG `bistro_ldr_log10ratio.png`）：**全幅均匀 100~1000× 赤字**（墙面/地板/家具/天花板一致），非窗口局部集中——排除「穿窗天光局部缺失」假设，坐实「全向环境项整体缺失」。

**Rurix 侧天空可见性审计**（`--diag-sky-vis`，stride 8 × 32 余弦半球射线/点，契约 seed 确定性；bistro 32399 覆盖点）：
- 天空可见率：mean **0.0302** / median 0.03125 / p90 0.0625 / **43.7% 的点 <1%**——真实路径追踪口径下室内天光逃逸率极低；
- 半球遮挡者直方图 top：MASTER_Interior_01_Plaster 212566 / Plaster_Red 169996 / Paris_Table_03 150606 / Floor_Tile 148364 / Wood 102786 射线——**遮挡主体 = 室内壳体/家具**；**玻璃材质遮挡份额仅 1.43%**（玻璃窗非主因，诊断前假设证伪）；
- 太阳验证射线：**可见 0 / 被挡 11879**（遮挡 top = Plaster 7672 / Table_03 2094 / Wood 553）——Rurix direct_mean≈2.9e-05 是几何真相；UE 的 57.29 max 高光尾只能来自 CSM 不遮蔽半透明玻璃的穿玻璃太阳（小区域，次因登记 G11.6 P2 候选）。
- cornell 对照：天空可见率 mean 0.2727（开放面）、太阳可见 5469/7843——双端同向 ⇒ cornell 收敛与机制一致。

## 5. 修复面与修法（实测优先级定）

**修法 = Rurix 侧补天光漫反射全向 IBL 直接消费**（旗标 `--sky-ibl`，spec-first 条款 RXS-0397，RFC-0028 §4.5 伞形「GI/天光遮蔽语义面」下）：

1. 主射线命中点直接项 += `albedo × L_sky × (1+n·up)/2`（半球混合解析式 = 下半球黑口径的全向漫反射 IBL 精确闭式；`L_sky = sky_intensity` 常量，up=+Y；法线 = 着色法线〔法线贴图扰动后〕；确定性零采样面）；
2. **GI 双重计数排除**：`--gi-multibounce` 世界缓存构建/渲染的 miss 射线整零（天光首反弹 = 直接项单计数），沉积/探针点直接项同期 += 同式天光项（天空二反弹及以上经缓存链接正常进入）；`--sky-ibl` 关 = 旧口径逐字节 parity（实测：默认面双场景帧 digest == G10.5 锁定值 `c2000ebf…`/`8519cc67…` 逐位一致——cornell 默认面已实测 `c2000ebfbe90359d55e668f8af3b7df24d64c3f72e637904f614821b7ad0d727`）；
3. **末级兜底修订行口径**（RXS-0396 L4 字面 0-byte）：旗标开时天光末级兜底由直接项承接，GI 零值 = 有效零间接，`last_resort_px` 计数显式登记维持；
4. 镜面天光项**不消费**（实测份额 ≤0.03% + 防高光尾过冲），维持 `c1_ue_specular_ibl` G15 候选登记。

**预览实测**（preview 帧 + G11.5 UE 帧，`g11_5b_preview_metrics.py` 命令输出）：

| 行 | 度量 | G11.5 复测 delta | G11.5b 预览 delta | 判定 |
|---|---|---|---|---|
| R1 | ssim@bistro(ldr)（ssim 值） | 0.98915（ssim 0.01085） | **0.66560（ssim 0.33440）** | 收敛（>0.16710 阈） |
| R3 | HDR 中位@bistro | 0.48023 | **0.39304** | 同号续降 |
| R4 | HDR p90@bistro | 1.21699 | **1.03977** | 同号续降 |
| U2 | LDR 中位@bistro | 0.62393 | **0.42715** | 同号续降 |
| R2/U1 | nonzero 覆盖@cornell | +0.00378418 | +0.00378418 | 不变维持 |
| C1 cornell 腿 | HDR p90@cornell | 0.23203 | **0.14638** | 同号续降 |
| — | ssim@cornell(ldr)（非门行） | 0.58273 | 0.69688 | 改善 |
| — | FLIP/PSNR@bistro | 0.91423 / 3.8652 | 0.81715 / 7.4203 | 改善 |

**如实登记的风险/边界**：
- cornell HDR 中位 delta = **−0.00447**（Rurix 0.10344 微超 UE 0.09897）与 HDR max 1.0654 > UE 0.5866——cornell 开放面墙受全向天光后的尾部过冲；**无门行消费 cornell HDR 中位/max**（R2/U1 = 覆盖比、C1 cornell 腿 = p90 仍同号 0.14638 ≥0），如实登记不遮蔽；
- bistro 高光尾（UE sun 穿玻璃区）Rurix 仍缺（max 4.88 vs 57.29）——次因登记 G11.6 P2 候选（承接锚 = 本文档 §0 次因行）；
- `--sky-ibl` 消费面 = 与 `--gi-multibounce` 组合（G11.5b 复测消费面）；单反弹 GI 组合不在本条款消费面（登记边界）。

## 6. 判档与法定输入面

- 修复范围法定来源：R1 行（ssim@bistro-interior LDR 残余闭环面，G11.5 整波 FAIL 停线行）+ C1 登记残余面（c1_ue_specular_ibl 本次附实测上界维持登记）——G10.8b 锁定清单 11 行 + 承接锚字面内，未无锚新立修复项；新发现差距（太阳穿玻璃）登记 G11.6 P2 候选不展开。
- 判档：GI/天光遮蔽语义面已由 Full RFC-0028（Agent Approved，§4.5 C1 口径对齐伞形 + §5 映射表）覆盖 ⇒ 本波 spec-first 新条款承接（RXS-0397，actual next_free 顺位），不新立 Full RFC；RXS-0357~0396 既有字面 0-byte；G5~G10 closed 判据 0-byte；未触高敏面（UB/内存模型/FFI ABI/安全包络）。
- 度量口径 0-byte：收敛判据/阈值/基线字面不改（RXS-0393 面；R1 阈 = `g11.fix.r1_ssim_shrink_tol` 标定条目 0.0 维持）——本波不改任何判据充绿。

---

`Assisted-by: Kimi-K3（G11.5b 波）`
