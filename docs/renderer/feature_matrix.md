<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C2 渲染器文档与示例） -->
# Rurix 渲染器 pass / 特性矩阵

> 所属：G31+ 波 C Task C2（渲染器文档与示例，G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #49）。
> 口径：特性状态以里程碑验收记录在案为准（生产 / 评估窗 / 挂起 / 阻塞，§7 词表）；性能数字一律
> measured_local（RTX 4070 Ti + Vulkan，canonical 160 帧 warmup 10 或各门登记窗），来源随表标注。
> 姊妹篇：[integration_guide.md](integration_guide.md) · [performance_tuning.md](performance_tuning.md)。

---

## 1. 生产五 pass 结构（真窗口车道）

`g31_window_present` 生产车道（与 `g14_3_pipeline_perf` 逐字共享 `g14_3_lane_body.rs`，bench/契约锚面 0-byte）：

| # | pass | kernel | 职责 |
|---|---|---|---|
| 1 | scene | `kernels/g14_3_direct_gi.rx` | 直接光 GI 主 pass（RayQuery compute 单 kernel 车道，bistro 契约场景） |
| 2 | mv | `kernels/g14_mv.rx` | 相机运动向量场（`m(x) = prev_uv(x) − x`；NoContraction 注入保 IEEE 位级） |
| 3 | tsr_resample | `kernels/g14_8_tsr_resample.rx` | TSR 时域重采样 |
| 4 | tsr_resolve | `kernels/g14_8_tsr_resolve.rx` | TSR 解析出帧（输出分辨率） |
| 5 | display_encode | `kernels/g31_display_encode.rx` | device 侧显示编码（ACES 1.3 RRT+ODT + BT.1886 γ2.4 + BGRA8 打包，链内直写 SSBO；G31 波 A Task A3） |

- bench/契约车道 = 前 4 pass（无 display_encode；出图走末帧一次性 f32 回读）。
- GPU 链内零 host 往返（RFC-0030 §4.5 L2 + §4.3 L3：`DeviceFrameSession` AS 常驻 + 逐帧参数上传 + 逐帧 fence）。
- `--fg x2` 时车道扩为 8 pass（+ `g31_mv_negate` 取反 glue + `g26_framegen` + 复用 display_encode 编生成帧），`--fg x3` 为 10 pass；生产五 pass 语义 0-byte（真实渲染帧 digest 与 fg off 位级一致为机核门）。

## 2. 超分三臂（`--backend` 闭集）

| 臂 | backend 字 | 状态 | 在案 measured（bistro-interior t100 1080p 直接光，frame_ms_mean） | 备注 |
|---|---|---|---|---|
| TSR（自研时域超分） | `tsr_device` | **生产**（默认臂；Stage A digest 锚 18/18 在案） | **2.29ms**（G31_PLUS §0 定盘） | G13 TSR 契约 13 腿 device；历史链跨帧 |
| FSR 3.1.5 | `fsr_3_1_5` | **生产**（vendor-upscale feature） | **2.79ms**（同上） | D3D12 共享驻留车道；LUID/布局 fail-closed 核验 |
| DLSS（NGX） | `dlss_sr` | **生产**（vendor-upscale feature；G17-MD-F1 焦点格诚实红面见 §5 注） | **4.01ms**（同上） | NGX 310.5.2；宿主税分解在案（波 C Task C9：NGX in-stream 1.84ms + 提交-同步税 0.15ms + scene 0.95ms + 宿主残差 0.37ms） |

tier 闭集 = `50 | 67 | 100`（内部分辨率相对输出 1080p 的比例；t100 = 内部 1920×1080）。

## 3. 帧生成 FG（`--fg`，G26 kernel 生产接线）

| 项 | 内容 |
|---|---|
| 状态 | **生产**（G31 波 A Task A5 门 g31.waveA.framegen 验收在案；G30 承接锚 G13-N7 行兑现） |
| 档位 | `off / x2 / x3`（真窗口车道；x2 插 1 帧、x3 插 2 帧） |
| kernel | `kernels/g26_framegen.rx`（0-byte 冻结消费）+ `kernels/g31_mv_negate.rx`（MV 取反 glue，零数值误差） |
| 闭集约束 | 须随 `--auto-move`（确定性轨迹登记面）+ `--tier 100`（MV 与 out_color 同栅格）+ frames+warmup ≥ 2 |
| 双口径 | `real_render_fps`（真渲帧唯一构成）与 `presented_fps`（含生成帧）独立登记永不混算；`caliber_identities` 恒等式组 schema 层钉死 |
| 在案 measured | x2 交付跑：real 85.24 fps / presented **145.30 fps**（real_render 11.73ms，fg_gpu 5.17ms）；波 A 验收复跑：real 65.83 / presented 116.91（15.19ms，fg_gpu 3.46ms）——两臂不冒充同值 |
| 对拍 | 接线态 host 金标准对拍 p100 结构界内（excess 恒 0）；G26 合成 GT 门复跑：x2 p100=2.980e-07 / x3 p100=3.576e-07 ≤ 冻结容差 7.152557e-07，SSIM 全帧严格胜 frame-hold |
| 已知缺口 | MV 仅含相机运动 + 静态场景深度重投影；运动物体 MV 缺口在案（bistro 静态面；dyn 实例场景 FG 不接） |

## 4. 内容面特性（G32 波 B 生产接线五大件 + 动态场景）

| 特性 | 开关 / harness | 状态 | 在案核验与 measured | 门 |
|---|---|---|---|---|
| **纹理采样** | `--textures on`（真窗口车道） | **生产**（B4 验收） | albedo/normal 70/70 核验；rough-metal 0/70 如实缺（不冒充）；sampler max_lsb=1；12/12 槽 == G11.3 DDS manifest；on/off **+6.29%** 帧耗 measured；组合窗 mapped=12 tex_tris=697878 probes=288 | g31.waveB.texture |
| **slab 材质侧表** | `--slab-table <asset.json> --slab-arm <device|host>`（真窗口车道） | **生产**（B3 验收） | 238927 slab 三角；device vs host bitexact 跨臂 **0/2073600** 差；MaterialClosure 32B ABI 核验；parity_p100=3.68e-8；Stage A 锚 MATCH | g31.waveB.slab |
| **HZB 遮挡剔除** | `--hzb on`（真窗口车道，五 SPV） | **生产**（B1 验收；成本如实登记不设通过线） | 剔除真实发生 tested=8799 / occluded=3549；剔除像素中性（on vs ALL_VISIBLE digest_seq 位级）；mips 位级；静态对照 on/off=**2.997×**；orbit 动态相机 on≈4.7×base（18.39~18.67ms vs 3.87~4.11ms）如实登记 | g31.waveB.hzb |
| **蒙皮/骨骼动画** | `--skin-demo`（bench 车道 MegaSkin） | **生产**（B5 验收；跨 harness 面不冒充单进程组合） | 蒙皮角色骨骼动画 20/20 位置核验；MV 通道进 TSR 历史链（RD-041 类 3 兑现）；BLAS refit 桥；帧成本 **+4.41ms**（skin_on 6.637 vs static 2.229 frame_ms_mean）；组合窗 prod 3.47/3.96ms、fps 165.5/150.9 | g31.waveB.skinning |
| **动态场景** | `--dyn-demo <refit|rebuild>`（bench 车道 MegaDyn） | **生产**（A4 验收） | refit/rebuild 双臂逐帧 64B 实例增量位置核验 60/60；跨臂 digest 位级一致；静态回归锚 == g14 Stage A 锚；refit prod 2.6722 / rebuild prod 2.8251ms | g31.waveA.dynscene |
| **ReSTIR 高档** | `g31_restir_wiring --restir <off|on> [--spatial] [--compare]` | **生产**（B2 验收；默认档 = 低档 MegaLights 语义镜像 0-byte） | y 整数锚 20000/20000 全等；estimate p100=1.75e-9 ≪ 冻结容差 5.66e-6；无偏 3σ 维持；方差降 **15.8×**（on vs off，方向硬门）；off 静态锚 4 跑零漂移 | g31.waveB.restir |

## 5. GI 档（`--gi`，默认 off 决策在案）

| 档 | 状态 | 说明 |
|---|---|---|
| `--gi off`（**默认**） | **生产**（G13/G14/G15 位级锚 0-byte 面） | 直接光唯一内容模型；Stage A digest 锚 18/18 + G16 M-g 18/18 + soak 三面全部锚定本臂 |
| `--gi on` | **评估窗已决策：maintain_default_off**（B6，`milestones/g31/g31_gi_default_tier_decision.json`） | 加性多反弹车道（RFC-0031，`kernels/g16_gi_multibounce.rx`）；管线健康 GREEN（bench/render 双腿真跑零缺陷）但默认不开启 |

B6 measured 权衡（bistro t100，canonical 160+10）：off 生产口径 1.79~1.93ms vs on 7.03ms = **×3.64~3.93**（scene GPU ×6.16~6.39）；画质 +10.05% 平均 luma（97.46% 像素触及）真实但温和，且对 UE Lumen 参照在案诚实红未闭（energy Δ79.94× / ssim 0.0065 / flip 0.967）；off 臂 fresh digest == Stage A 锚位级 MATCH。重判条件（两条件合取）：GI 臂对 UE Lumen parity 差距闭环 + 帧时代价落入在案预算格（如 ReSTIR GI 路径替代暴力 2 反弹 NEE）。

**OIT/半透明**：B7 评估窗决策 **not_triggered**（`milestones/g31/g31_oit_evaluation_window.json`——压测闭集机核全 OPAQUE；`oit/` 维持 M120 测量 harness 态；strand 档锚未命中维持）。

**焦点格诚实红注**（§2 DLSS 行）：G17-MD-F1 bistro/t100/dlss_sr 对 UE 暖态 ratio 在案 **0.960479** < 1.00（17/18 终判）；波 B 验收 fresh 中位 0.956162 较前次轨迹恶化**如实 RED 登记不冒充**（同格 digest 五跑全 == 在案锚，确定性面零漂移；详见 performance_tuning.md §5）。

## 6. 组合互斥表（波 B 在案闭集，fail-closed exit=1 逐字拒跑）

来源：`milestones/g31/g31_waveb_combo_matrix.json`（12/12 拒跑核验在案，零冒充可组合）。可组合臂 5/5 真跑绿（双跑 digest 位级）= C0 base orbit / C1a textures+orbit / C1b skin-demo（跨 harness）/ C2 slab-table+orbit / C3 hzb+orbit。

| # | 组合 | 拒跑文案要点 |
|---|---|---|
| M1 | `--hzb on` × `--fg` | 互斥（B1 接线面 = 生产五 pass 现状车道；FG 组合面非本任务口径） |
| M2 | `--hzb on` × `--slab-table` | 互斥（B1/B3 组合面非本任务口径） |
| M3 | `--slab-table` 无 `--auto-move` | slab 须随确定性轨迹（B3 登记面 = 轨迹 digest_seq） |
| M4 | `--slab-table` × `--fg` | 互斥（B3 接线面 = 生产五 pass 现状车道） |
| M5 | `--fg` 无 `--auto-move` | FG 须随确定性轨迹（A5 登记面；交互面 FG 非本任务口径） |
| M6 | `--dyn-demo` × `--inflight 2` | dyn 要求 inflight 1（A2 约束：FIF 流水入口拒 tlas_update） |
| M6b | `--skin-demo` × `--inflight 2` | skin 要求 inflight 1（同律：FIF 入口拒 blas_refit） |
| M7 | `--textures on` × `--hzb on` | 互斥（B4/B1 组合面非本任务口径） |
| M8 | `--textures on` × `--fg` | 互斥（B4 接线面 = 生产五 pass 现状车道） |
| M9 | `--textures on` × `--slab-table` | 互斥（B4/B3 组合面非本任务口径） |
| M10 | `g31_window_present --skin-demo` | 未知参数（蒙皮 demo 在 pipeline_perf 车道闭集，不进真窗口车道） |
| M11 | `g31_window_present --dyn-demo` | 未知参数（动态实例 demo 在 pipeline_perf 车道闭集） |

跨 harness 纪律：`--skin-demo/--dyn-demo` 与 `--textures/--hzb/--slab-table/--fg` 分属两车道参数闭集；跨 harness「组合」以双真跑同窗登记，**不冒充单进程组合**（G32 契约 out_of_scope 字面）。

## 7. 状态词表

| 状态词 | 含义 |
|---|---|
| **生产** | 验收门 PASS 在案（gate 真跑 + evidence + schema 核验）；消费面冻结或有拒跑护栏 |
| **评估窗** | measured 权衡/触发评估已落决策 JSON（只追加登记面，不设硬门）；如 B6 GI 默认档、B7 OIT |
| **挂起** | 锚未命中维持（如 BistroExterior = G10-N6 锚：FBX2glTF 上游修复 + 源资产齐备） |
| **阻塞** | 上游依赖未解（如 DXIL RT 腿 = RD-034 spirv-cross 拒 raygen；Work Graphs = 驱动扩展 absent） |
| **诚实红** | 真跑 measured 未达参照、如实登记不冒充（如 G17-MD-F1 焦点格 ratio < 1.00） |

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 初版（G31+ 波 C Task C2）：五 pass 结构/超分三臂/FG/内容面五大件+动态场景/GI 档与 OIT 决策/互斥 12 闭集/状态词表 |
