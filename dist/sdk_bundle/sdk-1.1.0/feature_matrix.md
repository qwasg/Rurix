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

> **G37 W4 注**：上表为 all-off 骨架（翻转后经显式 `--quality off` 可达）。窗口车道**缺省已 = `--quality full` 十九臂**：同一骨架上 scene pass 换载画质 kernel 链（`g31_realism` 链，含透明穿透/RIS/NEE 链位），并追加 bloom 四 pass 与 AE 两微 pass 等；`--quality full × --fg` 组合面 pass 链（x2 十四 pass / x3 十六 pass，comp parity 双缓冲插值 post-bloom）见 §8.2。默认档与锚谱系见 §8.1。

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
| **G37 W3 fg×full 组合** | fg 合法形态 = **两点式闭集**：{全画质 off base} ∪ {`--quality full` 预设}（W4 翻转后缺省即第二点）；full 面 FG 插值 post-bloom 合成帧（comp parity 双缓冲，真实帧数值逐位不变），AE 增益经 enc_params[133] 生成帧同读继承；散臂微调混搭 fail-closed（下标族爆炸防线）；fg×{hzb, slab, svt, lut, storm/fault} 互斥维持——详见 §8.2 |

## 4. 内容面特性（G32 波 B 生产接线五大件 + 动态场景）

| 特性 | 开关 / harness | 状态 | 在案核验与 measured | 门 |
|---|---|---|---|---|
| **纹理采样** | `--textures on`（真窗口车道） | **生产**（B4 验收） | albedo/normal 70/70 核验；rough-metal 0/70 如实缺（不冒充）；sampler max_lsb=1；12/12 槽 == G11.3 DDS manifest；on/off **+6.29%** 帧耗 measured；组合窗 mapped=12 tex_tris=697878 probes=288 | g31.waveB.texture |
| **slab 材质侧表** | `--slab-table <asset.json> --slab-arm <device|host>`（真窗口车道） | **生产**（B3 验收） | 238927 slab 三角；device vs host bitexact 跨臂 **0/2073600** 差；MaterialClosure 32B ABI 核验；parity_p100=3.68e-8；Stage A 锚 MATCH | g31.waveB.slab |
| **HZB 遮挡剔除** | `--hzb on`（真窗口车道，五 SPV） | **生产**（B1 验收；成本如实登记不设通过线） | 剔除真实发生 tested=8799 / occluded=3549；剔除像素中性（on vs ALL_VISIBLE digest_seq 位级）；mips 位级；静态对照 on/off=**2.997×**；orbit 动态相机 on≈4.7×base（18.39~18.67ms vs 3.87~4.11ms）如实登记 | g31.waveB.hzb |
| **蒙皮/骨骼动画** | `--skin-demo`（bench 车道 MegaSkin） | **生产**（B5 验收；跨 harness 面不冒充单进程组合） | 蒙皮角色骨骼动画 20/20 位置核验；MV 通道进 TSR 历史链（RD-041 类 3 兑现）；BLAS refit 桥；帧成本 **+4.41ms**（skin_on 6.637 vs static 2.229 frame_ms_mean）；组合窗 prod 3.47/3.96ms、fps 165.5/150.9 | g31.waveB.skinning |
| **动态场景** | `--dyn-demo <refit|rebuild>`（bench 车道 MegaDyn） | **生产**（A4 验收） | refit/rebuild 双臂逐帧 64B 实例增量位置核验 60/60；跨臂 digest 位级一致；静态回归锚 == g14 Stage A 锚；refit prod 2.6722 / rebuild prod 2.8251ms | g31.waveA.dynscene |
| **ReSTIR 高档** | `g31_restir_wiring --restir <off|on> [--spatial] [--compare]` | **生产**（B2 验收；默认档 = 低档 MegaLights 语义镜像 0-byte） | y 整数锚 20000/20000 全等；estimate p100=1.75e-9 ≪ 冻结容差 5.66e-6；无偏 3σ 维持；方差降 **15.8×**（on vs off，方向硬门）；off 静态锚 4 跑零漂移 | g31.waveB.restir |

> **G37 W4 注**：本表各「真窗口车道」单开臂写法（`--textures on`/`--hzb on`/`--slab-table` 等）在默认翻转后**须显式 `--quality off`**（单臂显式写法闭集，§8.1）；表内在案数字与门语义不变（各门 CI 调用面已按 `w4_flip/QUALITY_OFF_SWEEP.md` 对账补扫）。

## 5. GI 档（`--gi`，默认 off 决策在案）

| 档 | 状态 | 说明 |
|---|---|---|
| `--gi off`（**默认**） | **生产**（G13/G14/G15 位级锚 0-byte 面） | 直接光唯一内容模型；Stage A digest 锚 18/18 + G16 M-g 18/18 + soak 三面全部锚定本臂 |
| `--gi on` | **评估窗已决策：maintain_default_off**（B6，`milestones/g31/g31_gi_default_tier_decision.json`） | 加性多反弹车道（RFC-0031，`kernels/g16_gi_multibounce.rx`）；管线健康 GREEN（bench/render 双腿真跑零缺陷）但默认不开启 |

B6 measured 权衡（bistro t100，canonical 160+10）：off 生产口径 1.79~1.93ms vs on 7.03ms = **×3.64~3.93**（scene GPU ×6.16~6.39）；画质 +10.05% 平均 luma（97.46% 像素触及）真实但温和，且对 UE Lumen 参照在案诚实红未闭（energy Δ79.94× / ssim 0.0065 / flip 0.967）；off 臂 fresh digest == Stage A 锚位级 MATCH。重判条件（两条件合取）：GI 臂对 UE Lumen parity 差距闭环 + 帧时代价落入在案预算格（如 ReSTIR GI 路径替代暴力 2 反弹 NEE）。

**OIT/半透明**：B7 评估窗决策 **not_triggered**（`milestones/g31/g31_oit_evaluation_window.json`——压测闭集机核全 OPAQUE；`oit/` 维持 M120 测量 harness 态；strand 档锚未命中维持）。**G37 注**：真窗口车道玻璃类透明材质已由 `--transparency` 臂（ray 穿透真解，入 full 默认）收口（§8.2/§8.3）——B7 决策面（bench 压测闭集 OPAQUE 判定）不受影响、字面维持。

> **G37 注（勿混两 GI 面）**：本节 `--gi` 为 **bench 车道**多反弹评估臂（RFC-0031），决策 maintain_default_off 维持、Stage A 锚面永不动。真窗口车道默认 full 内含的 **`--gi2`**（反弹裁剪 clamp 0.01 + `--gi2-tex` 贴图反弹 + `--gi2-ris`/`--gi2-nee` 方差收缩）是窗口画质臂族，与本节 `--gi` 分属两车道两语义，见 §8.1/§8.2。

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

> **G37 W4 翻转后以 §8 为准（本表历史字面不回写）**：波 B 闭集在 G37 有三处终态演进——① M1/M4/M5/M8 的 fg 互斥面收窄为**两点式闭集**（fg 合法 = all-off base ∪ `--quality full` 预设；M8 textures×fg 经 full 预设字面开放，散臂混搭维持拒跑）；② 默认翻转后本表各互斥/诊断臂命令形态一律须显式 `--quality off`；③ `--transparency/--lut/--gi2-ris/--gi2-nee/--visbuffer/--cluster-per-frame-cut` 等 G37 新臂的组合约束见 §8.2。

## 7. 状态词表

| 状态词 | 含义 |
|---|---|
| **生产** | 验收门 PASS 在案（gate 真跑 + evidence + schema 核验）；消费面冻结或有拒跑护栏 |
| **评估窗** | measured 权衡/触发评估已落决策 JSON（只追加登记面，不设硬门）；如 B6 GI 默认档、B7 OIT |
| **挂起** | 锚未命中维持（如 BistroExterior = G10-N6 锚：FBX2glTF 上游修复 + 源资产齐备） |
| **阻塞** | 上游依赖未解（如 DXIL RT 腿 = RD-034 spirv-cross 拒 raygen；Work Graphs = 驱动扩展 absent） |
| **诚实红** | 真跑 measured 未达参照、如实登记不冒充（如 G17-MD-F1 焦点格 ratio < 1.00） |

## 8. G37 商业化收官终态（默认档翻转 + 新臂 + 登记面）

> 来源：`artifacts/day_0830_delivery/`（CAMPAIGN_LOG + 各波 REPORT）与 `g31_window_present.rs` 解析层字面；本节为 G37 终态事实面，历史各节字面不回写。

### 8.1 默认档翻转（W4，DEFAULT_FLIP_PLAN 获批执行）

- **`--quality` 缺省 = `full`**（`g31_window_present` 交付默认）：解析层一键展开**十九臂**——smooth-normals / ggx / lamp-lights（gain 4）/ textures / bloom / dither / auto-exposure / tsr-quality / gi2（clamp 0.01）/ emissive-tex / metal-f0 / rt-ao / soft-shadows（**1 样本**，F1 定档：2 样本 12.96ms 超 90fps 预算）/ rt-reflect / gi2-tex / normal-maps / **transparency** / **gi2-ris** / **gi2-nee**；`RURIX_G18_AMBIENT` 缺席时注入战役终态档 0.004。
- **`--quality off` = 显式回退档**（中性字面零展开，all-off 锚面）。展开面 22 旗标（十九臂 + `--lamp-gain/--gi2-clamp/--soft-shadow-samples` 三预设子参数）与 full 同给 = **dup fail-closed**；⇒ **单臂显式写法与诊断/互斥臂**（fg base 点 / hzb / slab / svt / storm / fault / cluster-lod / wp-hlod）**须显式 `--quality off`**。CI 调用面已全量对账补扫（A 类 18 调用点，`w4_flip/QUALITY_OFF_SWEEP.md`）。
- **bench 面不动**：`g14_3_pipeline_perf` `--quality` 默认维持 off；Stage A 18 格锚（`c1d28ad7…` 系）跨里程碑回归事实源永不翻转。
- **锚谱系**：all-off `55e4a92d…`（跨重建稳定，W4 s01 复验 MATCH）；十六臂 `5db2e7d7…`（day_0829，十九臂并入后作废）；**十九臂默认新锚 + RD-045 P02 锚替换值 = W4 整批重收割，占位「见 W4_ANCHORS」**（`artifacts/day_0830_delivery/w4_flip/W4_ANCHORS.json`）。presented 锚 = 二进制绑定锚（重建后整批重收割，E1 治理律）。

### 8.2 G37 新臂（W2 接线 + W3 判档合入）

| 臂 | 开关 / 组合约束 | 状态 | 语义与在案要点 |
|---|---|---|---|
| **透明材质（玻璃 ray 穿透）** | `--transparency <off|on>`（**入 full**）；`--transp-alpha`（默认 **0.85** ∈ (0,1]，可随 full 微调） | **生产**（W2 臂⑦，`g31_realism` 第 7 链位） | 主射线 ≤8 层穿透 + 点光阴影透射衰减；判定 = **alphaMode BLEND ∨ baseColorFactor.a<1**（bistro 唯一命中 TransparentGlass 130,792 tris；名字启发式弃用登记）；tint 取 tri_base 未衰减 baseColorFactor；GI2/AO/反射 NEE 视玻璃不透明如实留窗 |
| **GI2 反弹 RIS 选灯 + 面光 NEE** | `--gi2-ris` + `--gi2-nee`（**入 full**）；`--gi2-ris-m`（默认 **6** ∈ [1,16]，可随 full 微调） | **生产**（W2 臂⑧，`g31_realism` 第 8 链位） | 反弹命中点 RIS 蓄水池选灯 + 44k 灯片 CDF 面光 NEE（EVAL_RESTIR §9.3 方差源头收缩路径）；能量口径 = nee on 时灯片 emission 直取置零 + 12 代表灯反弹让位（不双计） |
| **LUT 色彩分级（后处理五级链第 4 级）** | `--lut <off|neutral|warm|路径.cube>`（默认 off；**可与默认 full 组合**）；与 fg/hzb/svt/slab/cluster-lod/wp-hlod 及显式 `--spv-encode` 互斥 | **生产**（W2 post_chain 缺级收口，#79） | LUT 表体内嵌 enc_params 尾部（[134] 门/[135] dim/[136..) 表体）——**零新绑定/零下标族**；换载「默认字面才换」+ off 恒载锚定字节 |
| **PSO 变体账本** | **默认开**（零配置，stderr 单行 `[PSO] sessions=… unique_variants=… pso_runtime_creates=…`）；`--pso-report <json>` sidecar 落盘；`RURIX_G31_PSO_STRICT=1` 升 fail-closed | **生产**（W2，#82/#113） | 窗口管线全部 session 构造期创建，运行期唯一重建点 = era 重建（resize）——era0 = precache 面，era≥1 新变体 = 告警（strict 下 fail）；**验收口径 = `pso_runtime_creates == 0`** |
| **VisBuffer 证据臂** | `--visbuffer <off|on>` + `--visbuffer-out|samples|res`；须随 `--cluster-lod leaf|on` ⇒ **须 `--quality off`** | **生产证据臂**（W2 判档档 2，#74/#111） | 窗口会话消费真轨迹相机 × 真 RXCP 簇 DAG device 真跑机制链（cut→32px 分箱→SW u64 原子软光栅→oracle 全等 + 双跑位级→classify/resolve）；sidecar evidence，presented 面 0-byte |
| **逐帧 cut→AS 证据臂** | `--cluster-per-frame-cut <off|on>` + `--frame-cut-out|every|res|blocks-limit`（every 1 = 逐帧，>1 = 惰性节拍降档）；须随 `--cluster-lod` ⇒ **须 `--quality off`** | **生产证据臂**（W3 判档，#77×#89） | **BLAS 顶点 refit 竞技场**：全簇固定槽位拓扑 ≈72MB，cut 以顶点内容切换（进 cut 真几何/出 cut 零面积折叠 = UPDATE 合法域），TLAS 恒不动；帧 0 全量上传堵死 AABB 陈旧假漏命中；命中槽陈旧零容忍机核 |
| **FG × full 组合** | `--fg <x2|x3>` 合法形态 = **两点式闭集**：all-off base ∪ `--quality full` 预设（翻转后缺省即第二点） | **生产**（W3 fg_combo 判档 + 合入） | 19/20 臂正交零适配；唯一耦合 = bloom 单缓冲 → comp parity 双缓冲适配（零新 kernel，真实帧数值逐位不变）；AE 增益 enc_params[133] 生成帧同读继承；散臂混搭 fail-closed（下标族爆炸 = AE 红修 #2 事故几何）；fg×{hzb, slab, svt, lut, storm/fault} 互斥维持 |

### 8.3 修复登记（W1/W2，已修在案）

| 件 | 修复 | 佐证 |
|---|---|---|
| 玻璃隔断雾状楔形（交互预览可见，day_0829 §H 登记「消融证明与画质臂无关」） | `--transparency` 臂收口（资产级透明管线缺失的本体修复；ray 穿透真解） | `w2_wiring/transparency/REPORT.md` |
| slot14 法线源件损坏（`Paris_Table_cloth_01_Normal.dds` 常值非法法线） | **v2 烘焙**：`baked_normals_bin_v2/`（slot14 替平坦 (127,127) 全 12 级 mip；其余 69 张与 v1 逐字节相等；L1 范数检测律唯一命中 slot14，校验 11/11） | `w1_fixes/slot14_normal/` |
| em+AE override 绑定错位（day_0828 Phase F 遗留，旧十臂组合 AE 近似恒等） | `set_autoexp` 补 `_EM` 两分支 + `g31_apply_autoexp` 连号断言 debug→assert 升级（语义变更重锚归 W4） | CAMPAIGN_LOG W1 |
| ACES encode 源码-字节 divergence（A2b 样条转置修复后共享件未切） | 共享 m_c 件收编 **v2**（`43b0c255`→`e7291c79`，spirv-val 绿）+ 防复发硬门 `ci/g31_encode_parity_smoke.py`（门 `g31.g37w1.encode_parity`，GATE PASS 在案） | `w1_fixes/encode_and_gates/REPORT.md` |
| rurixc「if 包 while」codegen（OpSelectionMerge 指向臂内块） | `structured_merge` 交汇计算排除 latch→header 回边（vulkan_codegen + dxil_spirv 双面）；98 生产 kernel pre/post 90/90 位级全同、冻结 SPV 零触碰 | `w1_fixes/rurixc_if_while/REPORT.md` |

### 8.4 判档登记（评估窗决策，append-only）

| 件 | 判档 | 依据与重启锚 |
|---|---|---|
| **VSM 页管线接线**（#104/#106） | **no-go / maintain-defer** | 生产车道阴影 = RayQuery 逐像素射线，无 shadow map 生成成本可摊销（强接 = 无消费面空转 dispatch）；判档件 `g31_vsm_device_probe` 三腿 GPU 真跑 PASS（golden dirty_depth 轴首个 device 消费腿在案）；**重启锚 = 光栅阴影档/VSM 采样车道出现（#105 PCSS / #27 SMRT 立项即触发）** |
| **异步双队列三件套**（M59/#88/#59） | **维持 no-go + 新鲜 measured** | digest 等价硬前置全过（single == dual == CPU 参照逐字节，双跑位级）——机制正确性已证；**重叠率中位 48.54% < 50% 阈** ⇒ 判据未达（中位改善 ≥3% ∧ ≥0.15ms ∧ 重叠率 ≥50%）；RFC-0019 修订草案留档不落地、RXS-0239 字面维持；重启锚 = 更重异步负载/专用 compute family 形态 |
| **FIF × 动态场景**（G36 留窗 #90） | **opt-in 判档件在案** | 拒绝面钉死（`submit_with_frame_update` 拒 tlas_update/blas_refit = host 写↔在飞 GPU 读共享面）；加性入口 `submit_with_frame_update_slot_as`（每槽 AS 副本组，槽纪律三判据 fail-closed）+ 三臂判档件逐帧 digest ≡ 基线；**RFC-0030 修订行草案（L2a opt-in 子行）留档** `w3_deep/fif_dyn/RFC_DRAFT_RFC0030_amendment.md`，落地归主线 RFC 程序 |

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 初版（G31+ 波 C Task C2）：五 pass 结构/超分三臂/FG/内容面五大件+动态场景/GI 档与 OIT 决策/互斥 12 闭集/状态词表 |
| v1.1 | 2026-08-30 | G37 商业化收官同步（W5）：新增 §8（默认档翻转十九臂 + 锚谱系 W4_ANCHORS 占位 / 新臂七行：transparency·gi2-ris·gi2-nee·lut·PSO 账本·visbuffer·cluster-per-frame-cut·fg×full 两点式 / 修复登记五件 / 判档登记三件：VSM no-go·异步双队列 no-go·FIF×动态 opt-in）；§1/§4/§5/§6 各追加 G37 注（历史字面不回写，翻转后语义以 §8 为准）；§3 FG 表追加 fg×full 组合行 |
