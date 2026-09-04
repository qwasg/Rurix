# day_0903_water 结论件:HPWater 水面方案复现(G41)

> 入役 git HEAD `b276de60`(工作树叠加 G40 云 + G41 水面未提交面);本役 2026-09-03;
> **未 commit,入库归 owner**。全部 GPU 真跑 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`,
> VUID 0、rc=0。门 = `py -3 ci/g41_water_smoke.py --gate g41.water.surface`(11 facts 全绿)。

## 1. 做了什么

复现 [HPWater](https://github.com/AshenOneArt/HPWater)(Unity HDRP 水体渲染系统,MPL-2.0)
所刻画的**技术方案**,落成 Rurix 的一条**独立展示车道** `g41_water_present`:
解析泻湖场景 + 交互水波 + 水体 GBuffer + 屏幕空间折射 + Beer-Lambert 体积吸收散射 +
Fresnel/GGX + 环境反射 + 解析焦散 + 三波长色散 + 泡沫,真窗口实时呈现 + 飞行相机。

**clean-room**:不含 HPWater 仓库任何源码文本,只按公开算法族独立推导
(许可分析见 [`rfcs/0050`](../../rfcs/0050-water-surface-rendering.md) §7;
先例 = G40 对 HPVolumeCloud 的同构处理)。

## 2. 交付件

| 件 | 说明 |
|---|---|
| `src/rurix-render/src/world/water_surface.rs` | host 金标准(波方程 / 泻湖场景 / 水体参数 / 公式面);**24 单测** |
| `src/rurix-render/kernels/g41_water_wave.rx` | 波方程 256² 三缓冲 ping-pong(诺伊曼障碍 + 海绵层 + 高斯波源) |
| `src/rurix-render/kernels/g41_water_scene.rx` | 解析泻湖 ray march → 场景色 + **真视深** |
| `src/rurix-render/kernels/g41_water_blur.rx` | 2× box 降采样链(替代无硬件 mip 的散射模糊) |
| `src/rurix-render/kernels/g41_water_surface.rx` | 水面着色主体(折射 / 体积 / 焦散 / 反射 / 泡沫 / 色散) |
| `src/rurix-render/kernels/g41_water_encode.rx` | 曝光 → ACES → sRGB → BGRA8(含 `raw` 直通调试腿) |
| `src/rurix-render/src/bin/g41_water_present.rs` | 五 pass 持久 session + swapchain 真窗口 + 飞行相机 + 空格投水滴 |
| `src/rurix-render/src/bin/g41_water_probe.rs` | 波方程 device↔host 逐格对拍 + measured 冻结带 |
| `ci/g41_water_smoke.py` | 门(11 facts,含内建 RED 臂) |
| `rfcs/0050-water-surface-rendering.md` | Mini-RFC **Draft**(D-409 未评审) |
| `artifacts/day_0903_water/tools/fetch_env_hdri.py` | Poly Haven **CC0** 水景 HDRI → sky-view LUT |
| `artifacts/day_0903_water/tools/make_water_clip.py` | raw 帧序列 → PNG → mp4(`imageio_ffmpeg`,同 rain_night) |
| `previews/*.png`(留盘) | 出图 14 张(四天空档 / 八臂 A/B / 实拍 HDRI 环境 / 自定义波源);仓库 `*.png` 全局不入库,sha256 登记于 `DELIVERABLES.json` |
| `lagoon_orbit.mp4`(留盘)+ `lagoon_orbit.mp4.json`(入库) | 环绕短片 300 帧 @30 fps = 10.0 s,1280×720,libx264 crf 21;六滴脚本波源;登记件含 sha256 |

## 3. HPWater → Rurix 映射

| HPWater | 本役 | 关系 |
|---|---|---|
| `HPWaterWaveEquation.compute` | `g41_water_wave.rx` | 同一物理模型独立推导 |
| `HPWaterPassGbuffer.hlsl` 3-MRT | `g41_water_surface.rx` 内联 GBuffer | 合并入着色 pass(无 MRT 需求) |
| `HPWater.shader` `FragRefraction` | `g41_water_surface.rx` 折射段 | 指数步进 + IGN + 厚度阈值 + 边界衰减 + 水上回退,逐条对应 |
| `HPWaterBSDFLibary` + `HPWaterVolumetrics` | 同上 体积段 | Beer-Lambert + Rayleigh/HG 相位 + 6 采样指数步进 |
| `CalculateHPWaterMipLevel` | `g41_water_blur.rx` + 帐篷混合 | mip 链 → 显式 box 链(执行面无硬件 mip) |
| `HPWaterCausticCompute`(光子 + 原子 + À-trous) | 解析闭式 `1/(1 + D·k·∇²h)` | **刻意偏离**,见 §5 |
| `HPWaterVolumeDeferred`(半分辨率 + MV 重投影 + À-trous + JBU) | 全分辨率 6 采样解析式 | **刻意偏离**,见 §5 |

## 4. 验收(机器事实)

```
py -3 ci/g41_water_smoke.py --gate g41.water.surface   →  PASS(11/11)
```

| fact | 实测 |
|---|---|
| 五 kernel 编译 + `spirv-val` | 5/5 |
| host 金标准单测 | 24 passed / 0 failed |
| 波方程 device↔host 对拍 | `max_abs_diff = 1.2218952e-6`,在 measured 冻结带内 |
| 对拍 RED 臂(带收紧到 1e-9) | 如期红 |
| 七臂 A/B 可归因 | 8 组 present digest **两两互异** |
| 默认面双跑 | present digest 位级相等 |
| 门自身红绿 | 注入语法错 → `kernels_compile` FAIL、rc=1;复原 → rc=0 |

帧时(measured_local,非门):1600×900 present ≈ **2.3 ms/帧(≈ 430 fps)**,
远在 11.11 ms(90 fps)预算内;640×360 ≈ 0.7 ms。

出图统计(`--water off` vs 默认,1600×900,clear 档):
mean `(83.6, 76.1, 60.2)` → `(73.2, 89.3, 81.5)`——水面接入后绿蓝通道显著抬升、
红通道下降,与"水对长波吸收强"的物理预期同向。

## 5. 三处相对 HPWater 的**刻意偏离**(不冒充等价)

1. **焦散**:HPWater 走 compute 光子步进 + `InterlockedAdd` 累积 + À-trous 三 pass;
   本役取**解析闭式**——水面局部曲率决定折射光束面积压缩比 `≈ 1 + D·k·∇²h`,
   强度取其倒数。无原子、无额外 pass、逐像素确定。**代价 = 不含多次折射与全反射焦散**。
2. **体积光**:HPWater 有半分辨率累积 + 运动矢量时域重投影 + À-trous + 联合双边
   上采样的完整降噪管线;本役取全分辨率 **6 采样解析**单次散射(指数步进),
   无蒙特卡洛 ⇒ 无噪声 ⇒ 无需降噪。**代价 = 不含多次散射与体积阴影**。
3. **模糊**:HPWater 按散射密度选 mipmap 级;本执行面纹理**单 mip** 且 compute
   kernel **不能硬件采样**(typeck 限 fragment/vertex),故改 3 级显式 box 降采样链。

## 6. 诚实边界

1. **无解析礁石**。曾实现球求交 + 投影阴影,实测在本后端结果不可信:礁石像素上
   命中距离 `rock_t` 恒取 0(直接输出实测 = 0),而球轮廓门却正确成形。逐项排除
   动态下标 / 哨兵灾难性抵消 / 跨段浮点相等 / 多累加器锁存四类成因后症状不变;
   与在树已登记的 rurixc「`if` 包 `while` 深层嵌套」缺陷(`g31_realism.rx` 头注)
   同型——移除前该 kernel 在单个 `if` 内含**四个** while 循环,而可用的
   `g41_water_wave` 为一个、`g41_water_surface` 为两个。故移除礁石(循环降到两个),
   以沙纹项补偿底面结构。`LagoonScene::rocks` 与参数面槽位保留,device 侧不消费。
2. **场景是自持解析泻湖,不是外部资产**。屏幕空间折射需要真视深,而生产 Mega 车道的
   `U_SCENE_DEPTH` 是 clip.x/y quirk 域;自持场景同时让水深梯度完全可控。
   代价 = 未验证在 Bistro 等真实 glTF 场景上的表现。
3. **波对拍非位级相等**。高斯波源含 `exp`,Vulkan `OpExtInst Exp` 与 host libm 非
   同源,位级相等不可达;已注入 `NoContraction` 关 FMA 收缩,判据取 measured 冻结带。
   归因链:无装饰 1.4901161e-6 → 注入后 1.1920929e-6 → host 除法形式对齐 1.2218952e-6。
4. **`--env-lut` 丢方位结构**。实拍 HDRI 归并进 `(dot(dir,sun), dir.y)` 二维 LUT 时
   对同格所有方位取均值,只保留亮度/色度分布;适用于水面反射与环境光,
   **不可当全景背景**(出图 `lagoon_lakeside_hdri.png` 的地平线呈平白带即此故)。
5. **相机不支持下潜**。`Q` 键钳在水面上 0.35 m;水下渲染不在本役范围。
6. **浅化只收波幅**,不模拟破碎波形(无卷浪/白沫抛射)。远岸掠射角仍可见
   极细的水面边界锯齿。
7. **治理面**:本役只出 Mini-RFC **Draft**,未做 D-409 对抗评审、未立 milestone
   契约、未领 CI_step 号(门用符号键)。均归 owner。
8. **`spv_inject_no_contraction` 第四副本**:与共享体 / frame_cut 臂字面同式
   (后者已登记单源折叠留窗);本 bin 不 include 共享体故再持一份。

## 7. 展示场景(联网检索结论)

需求「联网搜索新的水面场景」的检索与选型:

| 来源 | 结论 |
|---|---|
| Khronos `glTF-Sample-Assets`(147 模型) | **无水景**(全库检索:无 lake/pool/water 场景,仅 `WaterBottle` 静物) |
| NVIDIA ORCA `Sun Temple` | **出局**:CC-BY-**NC-SA**,违反 `g10_asset_license_registry.json` 白名单 `["CC0-1.0","CC-BY-3.0","CC-BY-4.0"]` |
| Sketchfab CC-BY 水景(Bathhouse 158k / Pool in the Mountains 25k / LAKE 33k / Forgotten Sanctuary Lake 1.85M) | 许可合规但**下载 API 强制 OAuth**,需用户令牌;且生产装载面只吃 `.gltf + 外置 .bin + DDS(BC1/BC3)`,需另写 JPEG→DXT 转换链 |
| **Poly Haven HDRI(CC0-1.0)** | **采用**:白名单最宽松一档、公开 API 免鉴权、已有先例(`world::sky` 四档预设即标定自其 CC0「Pure Sky」)。水景候选 9 张:`lakeside_sunrise` / `lakeside_dawn` / `secluded_beach` / `fish_hoek_beach` / `small_harbour_sunset` / `small_harbour_morning` / `shudu_lake` / `radkow_lake` / `lakeside_night` |

本役实取 `lakeside_sunrise`(1k,`sha256:c38a2004…`),经
`tools/fetch_env_hdri.py` 烘焙为 128×128 sky-view LUT(`sha256:35b065ff…`,
日面方向实测 `(0.586, 0.095, 0.805)` ≈ 仰角 5.5°,与"日出"一致)。
**二进制留缓存根 `K:/rurix_g10_cache/polyhaven-env/` 不入 git**(仓库 `.gitignore` 拒 `*.hdr`)。

## 8. 复现命令

```powershell
# 1) 编 kernel(五件)
py -3 ci\g41_water_smoke.py --build-spv

# 2) 门(11 facts)
$env:RURIX_REQUIRE_REAL="1"; $env:RURIX_VK_VALIDATION="1"
py -3 ci\g41_water_smoke.py --gate g41.water.surface

# 3) 真窗口实时(WASD/QE 移动,方向键+鼠标转视角,空格投水滴,-/= 曝光,ESC 退出)
cargo build --release -p rurix-render --features vulkan --bin g41_water_present
target\release\g41_water_present.exe --preset golden

# 4) 出图(四天空档)
foreach ($p in @("noon","clear","golden","sunset")) {
  target\release\g41_water_present.exe --headless --frames 1 --warmup 70 `
    --width 1600 --height 900 --preset $p --digest `
    --dump artifacts\day_0903_water\previews\lagoon_$p.png
}

# 5) 实拍 CC0 水景环境(先下载烘焙,再出图)
py -3 artifacts\day_0903_water\tools\fetch_env_hdri.py --slug lakeside_sunrise --res 1k
target\release\g41_water_present.exe --headless --frames 1 --warmup 70 --preset golden `
  --env-lut K:/rurix_g10_cache/polyhaven-env/lakeside_sunrise/lakeside_sunrise_1k.skylut `
  --dump artifacts\day_0903_water\previews\lagoon_lakeside_hdri.png

# 6) 波方程 device↔host 对拍(--freeze 产 measured 带)
target\release\g41_water_probe.exe --frames 90

# 7) 环绕短片(300 帧 @30 fps;raw 帧 / PNG 帧 / mp4 均 .gitignore,登记件入库)
target\release\g41_water_present.exe --headless --frames 300 --warmup 60 --width 1280 --height 720 `
  --preset golden --cam-orbit `
  --drops "0:0.5,0.5,0.55,7; 45:0.34,0.62,0.40,5; 95:0.66,0.38,0.48,6; 150:0.45,0.55,0.42,6; 205:0.60,0.66,0.38,5; 260:0.36,0.42,0.45,6" `
  --dump-raw artifacts\day_0903_water\clip_orbit.raw --dump-raw-every 1
py -3 artifacts\day_0903_water\tools\make_water_clip.py --src artifacts\day_0903_water\clip_orbit.raw `
  --out artifacts\day_0903_water\lagoon_orbit.mp4 --warmup-skip 60 --fps 30
```

## 9. 帧时(measured_local,非门)

| 构型 | render_frame_ms | fps |
|---|---|---|
| 640×360 定帧 | 0.7–0.9 | ~1100–1400 |
| 1280×720 环绕(含逐帧 8.3→3.7 MB BGRA8 回读) | 1.7 | ~580 |
| 1600×900 定帧 | 2.2–2.5 | ~400–450 |

全部远在 11.11 ms(90 fps)预算内。逐帧回读在 `--dump-raw-every` 面下计入帧时,
与静态面不可直接比较(rain_night 同一口径登记)。
