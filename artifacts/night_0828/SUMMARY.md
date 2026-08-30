# 夜间自动巡航总结 — 渲染器画质优化（2026-08-28 00:18 → 08:00）

> 任务：优化 Rurix 渲染器生产环境实时渲染的出图效果（材质/光影/性能/游戏体感画质与帧率）。
> 纪律执行：先发现→出计划→修改验收→变差即回退→每方向最多 2 次；子 agent 积极协同；全部改动**加性默认 off、冻结面零漂移**。

## 一、产出总览（6 项改进全验收 + 2 项评估 + 回归全绿）

| 方向 | 问题 | 交付 | 验收证据 | 帧时代价 | 启用 |
|---|---|---|---|---|---|
| **平滑顶点法线** | 曲面/圆柱面片感、颗粒（材质） | 新 kernel `g18_smooth_nrm.rx` + glTF NORMAL 侧表链 + `--smooth-normals`（bench+窗口双车道） | Stage A 零漂移；颗粒墙面 p95 ↓24%；曲面消面片（图证）；~免费 | +0.00~0.03ms | `--smooth-normals on` |
| **GGX 高光** | 全 Lambert 粉笔感（材质最大差距） | tri_mr 2 f32/tri 侧表（glTF metal/rough 因子）+ 质量 kernel params[48] GGX 臂 + `--ggx`（bench+窗口双车道） | 默认/snrm/窗口全锚零漂移；地板釉面 sheen+柜台高光带（图证）；20.2% 像素结构化 specular | +16µs（+1.65%） | `--smooth-normals on --ggx on` |
| **半球环境光** | 死黑（光影） | 质量 kernel params[44..48) + env 门控 | 关臂零漂移；均值+38% 高光不炸；暗部可读（图证） | ~0 | `RURIX_G18_AMBIENT=0.004`（配 smooth-normals） |
| **Bloom 光晕** | 灯具生硬白点（图片效果） | 3 新 device kernel + 窗口车道 `--bloom`（九 pass 链） | 全绿 + 差异集中灯具区 + 视觉光晕（图证） | +0.21ms GPU | `--bloom on` |
| **TPDF 抖动** | 渐变色带（图片效果） | `g31_display_encode.rx` params[3] + `--dither` | off==改前零回归 + 色带 runlen 25px→2px | ~0 | `--dither on` |
| **GI 多反弹评估** | 死黑（真解） | 评估登记不启用 | 补光有效但 sin-hash 噪声 + ×4.4 帧时 → 非净收益 | ×4.4 | （opt-in `--gi on`，不推荐 60fps） |
| **纹理采样评估** | albedo 马赛克（颗粒次源） | 既有 `--textures on` 臂确认有效 | 2.32% 像素真实贴图细节 | +0.75ms | `--textures on`（top-12 材质） |

**回归**：Stage A **全 18 格零漂移**（bistro/cornell × t50/67/100 × tsr/dlss/fsr；12 vendor 格批内 MATCH + 6 tsr 格隔离 MATCH——批内 vendor→tsr 测序触发的设备态 rc=1 经隔离复跑+手工复现证伪为非本巡航回归）。含全部改动的二进制跑默认臂 == 在案锚位级一致。
**独立评审**：两轮均**可安全合入 ✅**（D1/D2/D3/D5 一轮 + GGX/窗口合流一轮；位级确定性含 ±0 角隅证明 / 治理 / 边界条件 / BRDF 数值面全 PASS；CONCERN 均已处置或如实登记）。
**稳定性 soak**：全特性栈两段累计 **66 迭代 / ~4909s（82 分钟）零失败**、digest 全程稳定（远超 ≥1800s 标准）+ 窗口风暴 resize 重建无崩溃。

## 二、生产实时路径现状（窗口车道 g31_window_present）

**全特性栈五臂可组合同开**（`--smooth-normals on --ggx on --bloom on --dither on` + env `RURIX_G18_AMBIENT`），组合双跑位级稳定、validation 静默：

```
scene(g18_smooth_nrm: 平滑法线 + GGX 高光 + 半球环境光)
  → mv → tsr_resample → tsr_resolve
  → bloom_bright → bloom_blur_h → bloom_blur_v → bloom_composite
  → display_encode(TPDF 抖动) → present
```

**终极前后对照**（`hero/ultimate_before_after.png`，窗口车道同 dolly 轨迹）：上=全关（暗/粉笔/平灯），下=五臂全开（暗部可读 + 地板釉面 sheen + 灯光光晕 + 无新缺陷）。

**帧率**（bistro 1080p tier100，RTX 4070 Ti，含 8.3MB 回读测量税）：基础 ~165fps；五臂全组合 ~135-180fps。纯渲染 GPU 链 ~1.7-2.1ms（生产口径，回读税另计——真实游戏循环不逐帧回读）。**60fps 预算（16.6ms）有大量余量**；各画质臂 GPU 增量：平滑法线 ~0 / 环境光 ~0 / GGX +16µs / bloom +0.21ms / 抖动 ~0 / 纹理 +0.75ms。

## 三、根因与关键认知（供后续）

1. **颗粒噪点主源不是蒙特卡洛噪声**（生产臂零 RNG），而是 **TSR EMA 驻态残差（≈0.23·σ_frame，min_alpha=0.04 地板）× 逐帧 jitter 的二值/高频信号**——最大源 = 44k 自发光灯片亚像素弹出（×16 曝光放大）+ 逐三角均值 albedo 马赛克 + 细几何。平滑法线治法线面片那一部分；albedo 马赛克要真纹理；emissive 弹出要 TSR 收敛/采样面（未动，留窗）。
2. **死黑根本因 = 44k 自发光灯片不投光**（无 emissive NEE），仅 4 点光照明。半球环境光是廉价近似（今晚交付）；真解 = GI/NEE（D4 评估：GI 臂 ×4.4 + sin-hash 噪声，不划算）。
3. **冻结面纪律**：g14_3_direct_gi（Stage A 默认臂）/g16_gi_multibounce（G16）/g18 母版（G18）等 kernel 被已收口里程碑 digest 锚冻结——任何默认路径改动必须走新加性臂。今晚全部改动遵守。

## 四、留窗与后续路线（按 ROI）

| 优先 | 方向 | 现状 | 价值/代价 |
|---|---|---|---|
| ~~高~~ ✅ | **GGX/高光材质** | **今晚已交付**（tri_mr 侧表 + params[48] 臂，bench+窗口双车道验证，~免费） | 窗口车道 --ggx 已接线（d6w2 验收全绿） |
| 高 | **emissive NEE / GI 降噪** | 灯片不投光；GI 臂 sin-hash 噪声+贵 | 死黑真解；GI 需换 RNG（新加性 kernel，G16 锚治理）+ 降噪 |
| 中 | **纹理全材质覆盖 + mip** | top-12 材质、mip0 双线性 | albedo 马赛克全消；内容管线扩面 |
| 中 | **TSR 锐化（CAS 类）** | 无锐化；TSR 软化 | 游戏体感；须先控颗粒否则放大噪声 |
| 中 | **自动曝光** | 仅 manual EV；ExposureState adapt 未接生产 | 游戏体感（明暗适应）；histogram 反馈链 |
| 中 | **平滑法线进 g34 全特性车道** | 仅 bench+窗口 Mega 车道 | 组合面统一 |
| 低 | GI RNG sin→R2 修复 | g16 kernel 冻结（G16 锚） | 仅 GI 臂受益；须新加性 kernel |
| 低 | 非均匀缩放法线逆置变换 | bistro（旋转+平移）正确 | 资产面扩展时 |

## 五、操作与回退

- **启用推荐画质组合**（窗口车道）：`g31_window_present --smooth-normals on --ggx on --bloom on --dither on` + 设 `RURIX_G18_AMBIENT=0.004`（bench 车道同臂：`g14_3_pipeline_perf --bench/--render ... --smooth-normals on --ggx on`）。
- **回退**：本巡航全部改动 = 5 tracked 文件（g31_display_encode.rx / aces13.rs / g31_window_present.rs / g14_3_lane_body.rs / g14_3_pipeline_perf.rs）+ 4 新 kernel（g18_smooth_nrm.rx / g31_bloom_{bright,blur,composite}.rx）。备份 = `night_changes_tracked.patch`。回退 = `git checkout -- <5 tracked>` + 删 4 新 kernel + 重建 SPV。**绝不碰** G36/G35 会话文件（00_MASTER_INDEX/11_ROADMAP/G35_CONTRACT/g34_full_lane/g35_particle_lane/check_schemas/milestones/g36/）。
- 详细逐轮证据：`NIGHT_LOG.md`；各臂证据 JSON：`d2_smooth_nrm/` `d3_bloom/` `d2_window/` `d1_default_spv/` `regression/`。
