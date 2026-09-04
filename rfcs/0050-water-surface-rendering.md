# RFC-0050:水面渲染前端(G41)

- 编号:RFC-0050
- 档位:**Mini-RFC**(加性渲染前端;不动语言面、不动冻结契约面)
- 状态:**Draft**(D-409 对抗评审未进行;Agent Approved 为收口门前置)
- 提出日:2026-09-03
- 关联:`world::water_surface`、`kernels/g41_water_*.rx`、`bin/g41_water_present`、
  `bin/g41_water_probe`、战役目录 `artifacts/day_0903_water/`
- 先例:RFC-0049(G35 GPU 粒子,Full RFC)、G40 体积云(HPVolumeCloud 复现)
- Assisted-by: cursor:claude-sonnet-4.5(G41 水面渲染前端)

---

## 1. 动机

仓库已有的水体面 `world::water`(M113 / RXS-0366)是**语义层**:Tessendorf 大洋
谱资产、host DFT 参照、双管线几何路径互斥机核、浮力接口预留——全部 host-only,
**无任何 device 渲染腿**,且被 `milestones/g9/g9_m113_water_band.json` 冻结带
锚定。即:仓库能"描述"水,但画不出水。

同时,生产渲染面对**透明介质**的能力如实登记为:玻璃 = 直线透射着色
(`kernels/g31_realism.rx` 臂 ⑦,头注明写"工程直线透射,不折射"),OIT 只解决
排序不解决折射;焦散、水体体积散射全无。

本 RFC 提出补上这条缺口:一个**加性**的水面渲染前端 —— 交互波方程 + 水体
GBuffer + 屏幕空间折射 + Beer-Lambert 体积吸收/散射 + Fresnel/GGX + 环境反射 +
解析焦散,以 G40 体积云同构的**独立展示车道**形态落地。

## 2. Prior art(技术参照,非代码派生)

| 来源 | 参照到的构造 |
|---|---|
| **HPWater**(<https://github.com/AshenOneArt/HPWater>,Unity HDRP,**MPL-2.0**) | 五层分解(GBuffer / 光追折射 / 体积光 / 焦散 / 流体)、折射的"只保留法线扰动分量"构造、指数步进 `d(t) = (Fᵗ−1)/(F−1)` + IGN 抖动 + 厚度阈值 + 边界衰减、散射密度→模糊级、三波长色散、波动方程的诺伊曼障碍边界 + 海绵层 |
| Tessendorf, *Simulating Ocean Water*(2001) | 波谱/色散关系背景(本 RFC 未用,归 `world::water` M113) |
| Henyey & Greenstein(1941);Rayleigh 散射 | 相位函数 |
| Schlick(1994) | 菲涅尔近似 |
| Walter et al., *Microfacet Models*(2007) | GGX D·V |
| Poly Haven(CC0-1.0) | 展示用实拍水景 HDRI(`lakeside_sunrise` 等) |

**§7 许可分析约束下,本 RFC 的实现为 clean-room 重写:只按上表算法族重新推导,
不含 HPWater 仓库的任何源码文本。**

## 3. 设计

### 3.1 分层(逐层一个 device kernel)

```
g41_water_wave    波方程 256² 三缓冲 ping-pong(诺伊曼障碍 + 海绵层 + 高斯波源)
g41_water_scene   解析泻湖 ray march → scene-linear HDR 场景色 + **真视深**
g41_water_blur    2× box 降采样链 ×2(替代无硬件 mip 的散射模糊)
g41_water_surface 水体 GBuffer + 指数步进 SS 折射 + 体积 + 焦散 + 反射 + 泡沫
g41_water_encode  曝光 → ACES filmic → sRGB → BGRA8
```

host 金标准 `world::water_surface` 与五个 kernel **公式面逐字同源**;
`g41_water_probe` 对波方程做 device↔host 逐格对拍。

### 3.2 三处相对 HPWater 的**刻意偏离**(不冒充等价)

| 面 | HPWater | 本 RFC | 理由 |
|---|---|---|---|
| 焦散 | compute 光子步进 + `InterlockedAdd` 累积 + À-trous(三 pass) | **解析闭式**:面积压缩比 ≈ `1 + D·k·∇²h`,强度取其倒数 | 无原子、无额外 pass、逐像素确定;代价 = 不含多次折射与全反射焦散 |
| 体积光 | 半分辨率累积 + MV 时域重投影 + À-trous + 联合双边上采样 | 全分辨率 **6 采样解析**单次散射(指数步进) | 无蒙特卡洛 ⇒ 无噪声 ⇒ 无需降噪管线;代价 = 无多次散射与体积阴影 |
| 模糊 | 按散射密度选 mipmap 级 | 3 级显式 box 降采样链 + 帐篷权重 | 执行面纹理**单 mip**,且 compute kernel **不能硬件采样**(typeck 限 fragment/vertex) |

### 3.3 为什么自持场景

屏幕空间折射的前提是**真深度**。生产 Mega 车道的 `U_SCENE_DEPTH` 是
clip.x/clip.y quirk 域(沿视线为常量,只能做屏幕序判,见
`g35_render_splat.rx` ③ 段头注),不可用于折射步进。故 G41 自持一条解析场景腿
(碗盆海床 + 岸坡 + 沙丘 + 沙纹),既拿到真视深,又让**深水 → 浅滩 → 干岸**的
水深梯度完全可控 —— 这是水体渲染最有信息量的构图,且 host/device 对拍不依赖
任何外部资产。

### 3.4 与冻结面的关系(0-byte)

本 RFC 的全部实现对 `g31_window_present` / `g14_3_lane_body` / `display/` /
`world::water`(M113 冻结带)**零触碰**:不 include 共享体、不复用生产 kernel、
不动任何冻结 digest 锚。天空复用 G40 已入树的 `world::sky`。

## 4. 验收面

| 判据 | 手段 | 实测 |
|---|---|---|
| 五 kernel 编译 + `spirv-val` | `ci/g41_water_smoke.py --build-spv` | 5/5 通过 |
| host 金标准单测 | `cargo test -p rurix-render --lib world::water_surface` | 24/24 通过 |
| 波方程 device↔host 对拍 | `g41_water_probe --frames 90` | `max_abs_diff = 1.2218952e-6`,冻结带内 |
| 对拍 RED 臂 | 收紧带到 1e-9 | 如期红 |
| 七臂 A/B 可归因 | 逐臂 `--<arm> off` 出图 digest | 8 组 digest **两两不等** |
| 真设备 + 校验层 | `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1` | 全程 rc=0、VUID 0 |
| 帧时 | 1600×900 present | ≈ 2.3 ms/帧(≈ 430 fps),远在 11.11 ms 预算内 |

## 5. 位级/确定性纪律

- 逐像素独立顺序求值,禁 atomic;只读输入、直写输出;全 f32;无 RNG(抖动取
  IGN 解析式)。
- 分支一律算术门 `(x·big).min(1).max(0)`,无 `else`/`break`/`continue`;
  无 device fn 调用(本后端未接线),全手工内联。
- 波对拍注入 SPIR-V `NoContraction` 关 FMA 收缩。**位级相等不可达**:高斯
  波源含 `exp`,Vulkan `OpExtInst Exp` 与 host libm 非同源,故判据取 measured
  冻结带(`--freeze` 程序产,禁手写,P-09)。归因链:无装饰 1.4901161e-6 →
  注入后 1.1920929e-6 → host 除法形式对齐后 1.2218952e-6(帧序不同,同量级)。

## 6. 留窗(如实登记,不冒充完成)

| # | 项 | 现状 | 恢复条件 |
|---|---|---|---|
| W-1 | **解析礁石**(球求交 + 投影阴影) | **移除**。实测该段在本后端结果不可信:礁石像素上命中距离 `rock_t` 恒取 0(直接输出实测),而球轮廓门却正确成形。逐项排除动态下标 / 哨兵灾难性抵消 / 跨段浮点相等 / 多累加器锁存四类成因后症状不变;与在树已登记的 rurixc「`if` 包 `while` 深层嵌套」缺陷(`g31_realism.rx` 头注)同型——移除前该 kernel 在单个 `if` 内含**四个** while 循环,可用的 `g41_water_wave` 为一个、`g41_water_surface` 为两个。`LagoonScene::rocks` 与参数面 [96..160) 槽位保留(host 侧仍有打包与单测),device 侧不消费 | rurixc 该缺陷定位修复,或改走 TLAS ray query 路径 |
| W-2 | 体积光完整降噪管线(范围 B) | 未做(取 6 采样解析式,见 §3.2) | 需要多次散射/体积阴影时 |
| W-3 | 光子累积焦散(范围 C) | 未做(取解析闭式,见 §3.2) | 需要全反射/多次折射焦散时 |
| W-4 | 环境 LUT 方位结构 | `--env-lut` 把实拍 HDRI 归并进 `(dot(dir,sun), dir.y)` 二维 LUT,**丢失方位结构**(只保留亮度/色度分布) | 改绑 equirect 环境图 |
| W-5 | `spv_inject_no_contraction` 第四副本 | 与共享体/frame_cut 臂字面同式(后者已登记单源折叠留窗) | 单源折叠 |
| W-6 | 相机在水面以下 | 不支持(`--water` 车道按相机在水上构型实现,`Q` 键下潜钳在水面上 0.35 m) | 水下渲染另立 |
| W-7 | 破碎波形 | 浅化只收波幅,不模拟卷浪/白沫抛射 | — |
| W-8 | 正式 milestone 契约 / Full RFC 档 / CI_step 领号 | 本役只出 Mini-RFC Draft + 战役目录 | 归 owner |

## 7. 许可分析(为什么是 clean-room 而非移植)

- HPWater 为 **MPL-2.0**;本仓库为 `MIT OR Apache-2.0`(`Cargo.toml` L11),
  `CONTRIBUTING.md` L75 要求贡献按双许可授权。
- MPL-2.0 §1.10 的 "Modification" 覆盖"对被覆盖源码的任何增删改"——HLSL →
  `.rx` 的逐行翻译属之,产物须继续以 MPL-2.0 分发并附 Exhibit A 头,与本仓库
  授权面**直接冲突**。
- `milestones/g31/g31_vendor_license_matrix.json` 的 `owner_action_policy` 把
  零障碍集**枚举**为 OSI 的 MIT / Apache-2.0 / 双许可;MPL 虽属 OSI 但为文件级
  copyleft,**不在**该枚举内,亦无既有条目。
- 结论:**不搬代码,只复现技术**。本 RFC 与实现由公开算法族独立推导,Prior art
  见 §2。先例 = G40 对 HPVolumeCloud(MIT + 署名)的同构处理。
- 展示资产另循 RXS-0381 白名单:本役用 Poly Haven **CC0-1.0**(白名单最宽松
  一档,无署名义务),二进制留缓存根不入 git。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-09-03 | 初稿(Draft):五层设计 + 三处刻意偏离 + 八条留窗 + 许可分析 |
| v0.2 | 2026-09-04 | owner 判档:维持展示战役形态、不升格正式 milestone、RFC 状态维持 Draft(登记见 artifacts/day_0903_water/HANDOVER.md §D-1);本役入库随 owner 分层入库波,正文 §1-§7 字面 0-byte |
