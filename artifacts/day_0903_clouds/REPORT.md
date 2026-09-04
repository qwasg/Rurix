# day_0903_clouds 结论件:HanPi Volume Cloud 体积云方案复现(G40)

> 入役 git HEAD `b276de60`(工作树叠加 G40 云 + G41 水面未提交面);本役 2026-09-03,
> 战役文档与门补齐于 2026-09-04;**未 commit,入库归 owner**。
> 门 = `py -3 ci/g40_cloud_smoke.py --gate g40.clouds.present`(7 facts);
> 门本身为本次补齐件,**尚未真跑**,`DELIVERABLES.json` 的 `gate_status` 如实登记为
> `pending`——不冒充 PASS。

## 1. 做了什么

复现 [HPVolumeCloud](https://github.com/AshenOneArt/HPVolumeCloud)(MIT + 署名要求;
其自身派生自 Unity HDRP 体积云)所刻画的**技术方案**,落成 Rurix 的一条**独立展示
车道** `g40_cloud_present`:程序化物理天空 + 体积云 ray march + device 侧编码 +
swapchain 真窗口实时呈现 + 飞行相机自由观察。

**clean-room**:不含参考仓库任何源码文本,只按公开算法族独立推导。先例 = G41 对
HPWater 的同构处理(见 [`day_0903_water/REPORT.md`](../day_0903_water/REPORT.md) §1)。

## 2. 交付件

| 件 | 说明 |
|---|---|
| `src/rurix-render/src/world/sky.rs` | Rayleigh + Mie + 臭氧单次散射解析天空;四档命名预设 |
| `src/rurix-render/src/world/clouds.rs` | host 金标准:Schneider 密度模型 / 自适应主步进 / 锥形光步 / 双叶 HG + Hillaire 多次散射 / phi_fwd 各向同性漫射场;`CloudFrontend` |
| `src/rurix-render/kernels/g40_volumetric_cloud.rx` | 逐像素 slab ray march(主步进 + 光步 + 相位 + phi_fwd) |
| `src/rurix-render/kernels/g40_cloud_encode.rx` | 曝光 → ACES filmic → sRGB → BGRA8 打包(device 侧) |
| `src/rurix-render/src/bin/g40_cloud_present.rs` | 两 pass 持久 `DeviceFrameSession` + `ExternalImagePresent` 真窗口 + 飞行相机 |
| `ci/g40_cloud_smoke.py` | 门(7 facts,含内建 RED 臂 + 真调 `spirv-val` + `gpu_device_lock`) |
| `previews/*.png`(留盘) | 出图 13 张(四天空档 / phi_fwd 开关对照 / phi 强度扫参 / 高清定帧 / 冒烟帧);仓库 `*.png` 全局不入库,sha256 登记于 `DELIVERABLES.json` |

## 3. 五层复现(host 金标准 `world::clouds`)

| 层 | 内容 |
|---|---|
| 密度模型 | Schneider 式:球壳 slab + weather map + 烘焙 Perlin-Worley 基频 + Worley 侵蚀 + Cu/Tcu/Cb 三型高度廓形 |
| 主步进 | 自适应:空域大步、密度域细步,近端/远端步长 cap 随 slab 厚度与视距缩放 |
| 光步 | 锥形几何级数步进(首步窄、逐步展开的锥内采样) |
| 相位与多次散射 | 双叶 HG + Hillaire 2020 三倍频多次散射近似 |
| phi_fwd | 招牌的各向同性漫射场:五因子一维 Green 函数,`κ = σ_t·√(3(1−ω₀))`,规避逐步 sqrt |

`CloudFrontend` 兑现了 `atmosphere.rs` 里长期空悬的**「两个前端」契约**(M112:
云与雾共用同一 froxel 基础设施、两个前端)——云前端与雾前端写同一 `FroxelVolume`、
同签名同错误面。

## 4. 天空(`world::sky`)

Rayleigh + Mie + 臭氧单次散射解析天空,四档命名预设 `noon` / `clear` / `golden` /
`sunset`,太阳高度角、方位角、浊度标定自 Poly Haven **CC0**「Pure Sky」实拍天空。
**只取标定数值,零二进制资产入库**(与 G41 取 Poly Haven CC0 HDRI 同一许可面,
但本役连缓存件都不需要)。

## 5. GPU 腿与 device 侧编码

```text
g40_volumetric_cloud  (逐像素 slab ray march + 锥形光步 + phi_fwd)
  → out_color(3 f32/px scene-linear HDR,驻留 device)
g40_cloud_encode      (曝光 → ACES filmic → sRGB → BGRA8 打包)
  → out_bgra(1 u32/px)→ 回读 → ExternalImagePresent
```

device 侧编码是**刻意**的:回读量从 `w·h·12B` 降到 `w·h·4B`,1080p 下
**24.9MB → 8.3MB**,present 腿不成为带宽瓶颈。

实测:**1280×720 真窗口实时 ≈ 110 fps**(含回读 + present)。战役当时全工作区
测试套 **609 tests 全绿**。

## 6. 三处值得记录的发现

### 6.1 rurixc 的两条硬限(探针证得,非猜测)

1. **device `fn` 调用未接线**。`src/rurixc/src/vulkan_codegen.rs:2584` 直接返回
   `VulkanCodegenError::unsupported("Vulkan compute device fn 调用(内联)属后续分片")`。
   后果:三线性采样 / HG / smoothstep 全部**手工内联**——同一份密度求值在主步进
   里出现一次、在光步里再出现一次(两份字面同源,靠 host 金标准对齐而非靠共享
   代码)。
2. **`while` 条件里的 `&&` 破 SPIR-V 块支配序**。故提前退出改写成「计数器直接跳到
   上限」的形态。这条也顺带解释了一个可机器核对的现象:树内 **98 个 `.rx` kernel
   没有任何一个**在 `while` 条件里用 `&&`。

### 6.2 噪声必须烘焙

逐采样求值 Perlin-Worley 约需 **350 次 hash**,逐像素约 **7 万次**——即使在 GPU 上
也不可承受。改为烘焙进 **128³ / 32³** 体积。host 与 device **共享同一份字节**,
这同时把噪声项彻底移出了对拍容差(噪声不再是误差源,只有步进与相位是)。

### 6.3 phi_fwd 强度与参考实现差两个数量级(量纲差,非公式差)

参考实现建议区间 `0.1–2.0`;本实现的可用带是 **20–60,默认 30**。原因是 φ 的量纲由
`σ_s·Δs·σ_tr·(1/r)` 决定,而这一串跟随本实现的 σ_t 标定与光步的度量尺度,比参考实现
**小约两个数量级**。这是**归一化常数**的差,不是公式的差——Green 函数的五个因子逐项
同源。该结论写在 `CloudParams::phi_fwd_intensity` 的文档上,并配一条 **5% 阈值单测**:
强度一旦掉回参考实现建议带即变红。

## 7. 一处**刻意偏离**原始需求(不冒充等价)

原始需求是「给 `g31_window_present.rs` 加一条 `--clouds` 臂」,实际实现为**独立真窗口
车道**。

**理由**(两条,都指向同一结论):

1. 该车道渲染的是 bistro-**interior**,云只会落在寥寥几个 sky-miss 像素上——加了也
   看不见。
2. 需求的后半句明确是「**一个新的天空场景**」,而室内场景不是天空场景。

独立窗口同时满足两条,且避免了重新收割已冻结的 digest 锚。

**仍然敞开的替代路线**(如实登记):在 scene 与 mv 之间插一个 pass,**就地读改写**
`U_OUT_COLOR`——逐像素独立、下游绑定零变更。代价是要为十来条组合臂补描述符下标常量,
并重新收割 `full19` / `RD-045` 锚。

> **Owner 决策 2026-09-04:维持独立窗口;`--clouds` 臂作为留窗登记。**

## 8. 诚实边界

1. **门尚未真跑**。`ci/g40_cloud_smoke.py` 是本次补齐件,7 facts 的判据已成文并静态
   校验通过(`ast.parse` 绿),但**未在设备上执行过**。`DELIVERABLES.json`
   `gate_status = "pending"`。§9 给出待跑命令。
2. **无真实 glTF 场景**。本车道是**纯天空**的:没有地形、没有建筑、没有任何外部资产。
   与 G41 自持解析泻湖同型,但更彻底。
3. **froxel 未合流**。`CloudFrontend` 与 `FogFrontend` 写同一个 `FroxelVolume`,但
   **生产车道两个都不消费**——契约兑现了,消费面还没有。
4. **密度求值双份字面**。§6.1 的手工内联导致主步进与光步各持一份;rurixc 补齐 device
   `fn` 调用后可折叠。
5. **噪声体每次启动现烘**,未落盘缓存。启动有固定烘焙开销(bin 以 `bake_ms` 如实打印)。
6. **治理面**:本役无 Mini-RFC、未立 milestone 契约、未领 CI_step 号(门用符号键)、
   未 commit。均归 owner。

## 9. 复现命令

```powershell
# 1) 编两件 kernel(门内建腿,亦可单独跑)
py -3 ci\g40_cloud_smoke.py --build-spv

# 2) 门(7 facts;device 腿自持 gpu_device_lock)
$env:RURIX_REQUIRE_REAL="1"; $env:RURIX_VK_VALIDATION="1"
py -3 ci\g40_cloud_smoke.py --gate g40.clouds.present

# 3) 只跑 host 面(kernel 编译 + spirv-val + RED 臂 + 金标准单测;零 GPU 出图)
py -3 ci\g40_cloud_smoke.py --selftest

# 4) 真窗口实时(WASD 平移 / QE 升降 / 方向键 + 鼠标转视角 / -,= 曝光 ±0.25 EV / ESC 退出)
cargo build --release -p rurix-render --features vulkan --bin g40_cloud_present
target\release\g40_cloud_present.exe --preset golden

# 5) 出图(四天空档)
foreach ($p in @("noon","clear","golden","sunset")) {
  target\release\g40_cloud_present.exe --headless --frames 1 `
    --width 1280 --height 720 --preset $p --digest `
    --spv-cloud .tmp\g40\spv\g40_volumetric_cloud.spv `
    --spv-encode .tmp\g40\spv\g40_cloud_encode.spv `
    --dump artifacts\day_0903_clouds\previews\preset_$p.png
}
```

两条 rurixc 编译行的展开形(门内即此形):

```powershell
target\debug\rurixc.exe src\rurix-render\kernels\g40_volumetric_cloud.rx `
  --target vulkan -o .tmp\g40\spv\g40_volumetric_cloud.spv
target\debug\rurixc.exe src\rurix-render\kernels\g40_cloud_encode.rx `
  --target vulkan -o .tmp\g40\spv\g40_cloud_encode.spv
```

## 10. 帧时(measured_local,非门)

| 构型 | 实测 |
|---|---|
| 1280×720 真窗口实时(含回读 + present) | **≈ 110 fps** |

回读量口径见 §5(device 侧编码后 4B/px)。
