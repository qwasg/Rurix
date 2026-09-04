# day_0903_clouds 战役日志(G40 体积云展示车道)

> 任务:「参考 HanPi Volume Cloud 项目中的体积云实现方案,并复现到本项目中。
> 然后给渲染器加一个新的天空场景,以便展示体积云效果」。
> 编排形态(如实):主 agent 串行承担侦察 / 实施 / 验收三层。**未 commit,入库归 owner**。
> 许可面:参考实现 MIT + 署名要求,其自身派生自 Unity HDRP 体积云;本役走
> **clean-room**——只取技术方案,不含参考仓库任何源码文本。先例 = G41 对 HPWater 的
> 同构处理。

## 波次

| 波 | 内容 | 终态 |
|---|---|---|
| W0 | 侦察:HPVolumeCloud 全源通读 + 许可分析,定 clean-room 路线 | 完成 |
| W1 | host 金标准 `world::sky`(Rayleigh + Mie + 臭氧解析天空,四档预设)+ `world::clouds`(五层复现 + `CloudFrontend`) | 单测绿 |
| W2 | 两 kernel(`g40_volumetric_cloud` / `g40_cloud_encode`)+ 展示车道 `g40_cloud_present` + swapchain 真窗口 | 1280×720 ≈ 110 fps |
| W3 | 视觉定标与 phi_fwd 标定(`--phi-intensity` 扫参定可用带 20–60,默认 30) | 见「关键缺陷与修法」3 |
| W4 | 出图十三张(四天空档 / phi_fwd 开关对照 / phi 强度扫参 / 高清定帧 / 冒烟帧) | `previews/` 13 图 |
| W5 | 门 `ci/g40_cloud_smoke.py`(7 facts)+ 战役文档 + `.gitignore` 战役块 | 门待真跑(`pending`) |

## 关键缺陷与修法(施工实录,供后来者少走弯路)

本役在 device 侧撞上的三件事,两条是 rurixc 的硬限、一条是性能定性判断。三条都是
**探针证得**,不是猜的。

1. **device `fn` 调用未接线**。`src/rurixc/src/vulkan_codegen.rs:2584` 直接返回
   `VulkanCodegenError::unsupported("Vulkan compute device fn 调用(内联)属后续分片")`。
   后果:三线性采样 / HG / smoothstep 一律**手工内联**——同一份密度求值在主步进里
   出现一次、在光步里再出现一次。**修法**:接受双份字面,靠 host 金标准把两份对齐
   (而不是靠共享代码);rurixc 补齐后可折叠(留窗 W-6)。

2. **`while` 条件里的 `&&` 破 SPIR-V 块支配序**。提前退出写成
   `while (i < n && t > eps)` 形态时,生成的块支配序不合法。**修法**:改成
   「计数器直接跳到上限」——需要退出时把循环变量一次性赋成上限值。
   这条有一个可机器核对的旁证:树内 **98 个 `.rx` kernel 没有任何一个**在 `while`
   条件里用 `&&`——不是巧合,是同一条限制在全仓的投影。

3. **噪声必须烘焙**(性能定性,非缺陷)。逐采样求值 Perlin-Worley 约 **350 次 hash**,
   逐像素约 **7 万次**,即使在 GPU 上也不可承受。**修法**:烘焙进 **128³ / 32³** 体积,
   host 与 device **共享同一份字节**。附带收益:噪声项彻底移出对拍容差——误差只剩
   步进与相位两个来源。代价 = 启动有固定烘焙开销、且未落盘缓存(留窗 W-5)。

另有一处**量纲口径**的澄清(REPORT §6.3 展开):phi_fwd 强度参考实现建议 `0.1–2.0`,
本实现可用带是 **20–60、默认 30**。φ 的量纲由 `σ_s·Δs·σ_tr·(1/r)` 决定,跟随本实现的
σ_t 标定与光步度量尺度,比参考实现小约两个数量级。这是**归一化常数**差而非公式差
(Green 函数五因子逐项同源),已配 5% 阈值单测锁住:强度掉回参考带即红。

## 需求偏离实录

原始需求后半句是「给 `g31_window_present.rs` 加一条 `--clouds` 臂」。实际落成的是
**独立真窗口车道**。理由与仍然敞开的替代路线见 REPORT §7;
**owner 决策 2026-09-04:维持独立窗口,`--clouds` 臂作为留窗 W-1 挂起**。

## 出图清单(`previews/`,留盘)

四天空档 `preset_{noon,clear,golden,sunset}.png` + phi_fwd 开关对照
`phifwd_{on,off}.png` / `cmp_phifwd_{on,off}.png` + phi 强度扫参
`phi_{5,20,60}.png`(标定 W3 的直接依据)+ 高清定帧 `clear_hq.png` +
冒烟帧 `smoke_clear.png`,共 **13 张**。仓库 `*.png` 全局不入库,
sha256 登记于 [`DELIVERABLES.json`](DELIVERABLES.json)
`groups.media_on_disk_not_tracked`。
