<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C2 渲染器文档与示例） -->
# Rurix 渲染器性能调优指南

> 所属：G31+ 波 C Task C2（渲染器文档与示例，G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #49）。
> 口径纪律（先读 §1）：本文数字**全部**引在案 measured_local（RTX 4070 Ti + Vulkan 本机真跑，
> gpu_device_lock 串行，RURIX_VK_VALIDATION=1），禁编造新数字；你的机器上重测会得到不同绝对值——
> 用本文的**相对杠杆与测量协议**，不要用本文绝对值做你的 SLA。
> 姊妹篇：[integration_guide.md](integration_guide.md) · [feature_matrix.md](feature_matrix.md) ·
> [profiling_debugging.md](profiling_debugging.md)（C7 剖析/标注/捕获工具面——定位帧内热点入口）。

---

## 1. 口径纪律（勿混）

1. **measured_local**：一切数字 = 本机单卡真跑口径；证据等级注明，估算/预测不属于本文。
2. **双口径分离**：`real_render_fps`（真渲帧唯一构成）与 `presented_fps`（含 FG 生成帧）独立登记**永不混算**；presented 帧率冒充真实渲染帧率是契约级红线（G31/G32 out_of_scope 字面）。
3. **采样协议**：canonical = 160 帧 warmup 10（G14 Stage A 锚同口径）；门各自 frames+warmup 窗在各 evidence 登记；跨窗数字不直接比（交付跑 vs 复跑 fresh 值分列、不冒充同值——本文各处双值并列即此纪律）。
4. **frame_ms 双列**：`frame_ms_mean`（全帧墙钟）vs `frame_ms_production_mean`（生产口径，剔除非生产段）；真窗口车道另有 `real_render_frame_ms`（含 present 强制 BGRA8 回读段，`render_includes_forced_readback=true`）vs `present_frame_ms`（纯 present 腿）vs `encode_gpu_ms`（device 显示编码 GPU 段）。
5. **digest 零漂移优先于帧时抖动**：帧时机态抖动合法（候选源如实登记不冒充根因）；digest 漂移 = 渲染产出变化，是确定性事故（RD-045 观察面）。

## 2. 基线数字（在案 measured）

bistro-interior 1080p 直接光，canonical 160+10，bench 车道（`g14_3_pipeline_perf --bench`）：

| 配置 | frame_ms_mean | 来源 |
|---|---|---|
| t100 `tsr_device`（默认臂） | **2.29ms**（prod 1.79~1.93ms 窗） | G31_PLUS §0 定盘；B6 决策 JSON 双跑 |
| t100 `fsr_3_1_5` | **2.79ms** | G31_PLUS §0 |
| t100 `dlss_sr` | **4.01ms** | G31_PLUS §0 |

真窗口车道（`g31_window_present`，release，hidden 真窗口，orbit 64+10 组合窗；**口径注 = 全部为 `--quality off` 显式形态**——测量时点默认档尚为 off，G37 W4 翻转后复现本表须显式给 `--quality off`，单开臂写法同律）：

| 臂 | real_render frame_ms | present frame_ms | 来源 |
|---|---|---|---|
| C0 base | 4.114 / 3.871（双跑） | 0.959 / 0.970 | g31_waveb_combo_matrix.json |
| C1a `--textures on` | 3.772 / 3.909 | 0.960 / 0.987 | 同上 |
| C2 `--slab-table` | 3.738 / 3.801 | 0.958 / 0.968 | 同上 |
| C3 `--hzb on` | 18.667 / 18.386 | 0.991 / 0.974 | 同上 |
| demo 定版 `--textures on` 200+10 | 5.113 / 5.431（real fps 195.6 / 184.1） | 1.004 / 1.013（encode_gpu 0.21/0.23ms） | 同上 |

**G37 W4 默认档（`--quality full` 十九臂）帧时口径**：

- 生产预算锚 = **90fps ⇒ 11.11ms/帧**（DEFAULT_FLIP_PLAN §4 口径；产品口径见 [PRODUCT_NOTES.md](PRODUCT_NOTES.md) §3）。
- 在案 measured（RTX 4070 Ti，hidden 真窗口）：day_0829 **十六臂** soak 1955.4s 帧时 **9.54~10.70ms**（峰值 10.70 ≤ 11.11 预算全程达标，`day_0829_realism/final/F3_SUMMARY.json`）；F2 96f 窗 9.5~9.9ms；G37 W0 复验 full16 **9.63ms** / all-off **5.49ms**（`day_0830_delivery/w0_baseline/W0_BASELINE.json`）。
- **G37 十九臂（翻转后默认）终值 = W4 整批重收割在途，占位「见 W4_ANCHORS」**（`artifacts/day_0830_delivery/w4_flip/W4_ANCHORS.json`）——届时按修订程序追加，本表历史行不回写。

## 3. 调优杠杆（按收益/代价排序）

### 3.1 帧流水化 `--inflight <1|2|3>`（bench 车道 tsr_device 已接线）

submit/collect 分离、N 帧 in-flight 去每帧 fence 全同步：

| 臂 | p50 frame_ms | prod mean | 在案改善 |
|---|---|---|---|
| inflight=1 | 1.8344 | 2.0320 | 基线 |
| inflight=2 | 1.5185 | 1.9729 | **p50 −17.2%**（fresh 复跑；交付波登记 **−23.5%**，两臂不冒充同值） |
| inflight=3 | 1.5371 | 1.8315 | **p50 −16.2%**（fresh） |

约束：跨臂逐帧 digest 位级一致（确定性零破坏，机核在案）；`--warmup ≥ N−1`（填充段落 warmup）；**拒** `--dyn-demo/--skin-demo`（FIF 入口拒 tlas_update/blas_refit，fail-closed exit=1）；仅 `--bench --backend tsr_device` 已接线，其余臂 fail-closed。真窗口车道当前仍走当帧 fence（FIF 进窗口 = G31_PLUS #89 在案缺口）。

### 3.2 tier 选择（`--tier 50|67|100`）

内部分辨率相对输出 1080p 的比例。t100 = 内部 1920×1080（FG 闭集限 t100：MV 与 out_color 同栅格）。降 tier 是帧时最直接的杠杆（内部像素量 ∝ tier²），代价 = 重建输入信息量下降；画质面 TSR 契约 13 腿 device 在案（G8.5b），按你的内容验收。

### 3.3 超分臂选择（`--backend`）

- 帧时：tsr 2.29 < fsr 2.79 < dlss 4.01（§2 基线，t100）。
- DLSS 臂的 4.01ms 构成在案（波 C Task C9 NGX 分解，canonical 同窗）：NGX in-stream **1.84ms**（双边同硬件同 NGX 310.5.2 cubin 族，物理不可分离等量）+ 提交-同步税 **0.15ms**（逐帧孤立 submit+waitIdle 边界税）+ scene 段 **0.95ms** + mv 0.03ms + 宿主残差 **0.37ms**（含 pack GPU 0.154 + SL 簿记/录制/evaluate CPU 0.086）。
- 选型建议：无 vendor 依赖诉求 → tsr_device（默认 + 锚全家）；要 vendor 生态/特定画质 → fsr/dlls 按 §5 焦点格诚实红面知情决策。

### 3.4 帧生成 `--fg x2/x3`（真窗口车道）

- 收益 = **presented 口径**流畅度：x2 交付跑 presented **145.30fps** vs real 85.24fps；复跑 116.91 vs 65.83。
- 成本 = FG GPU 段 3.46~5.17ms（telemetry `stats.fg_gpu_ms` 单列）打进单提交墙钟——real_render 口径如实含此段（生成帧禁入计数，但墙钟同提交）。
- 适用面：确定性轨迹（`--auto-move` 闭集）+ t100 + 静态场景面（运动物体 MV 缺口在案）；与 `--hzb/--slab-table/--textures` 全互斥（feature_matrix §6）。
- **G37 W3 fg_combo 注（翻转后以此为准）**：fg 合法形态 = **两点式闭集** {all-off base} ∪ {`--quality full` 预设}——上列在案数字为 base 形态（当期默认 off），W4 翻转后复现须显式 `--quality off`；full 面 FG 插值 post-bloom 合成帧（comp parity 双缓冲，真实帧数值逐位不变），「与 --textures 互斥」字面 = 散臂口径维持、组合面经 full 预设字面开放；fg×{hzb, slab, svt, lut} 互斥维持（feature_matrix §8.2）。

### 3.5 HZB 遮挡剔除（`--hzb on`）

- **当前形态是正确性接线而非性能杠杆**：剔除像素中性（on vs ALL_VISIBLE digest_seq 位级——画面零变化）+ 剔除真实发生（8799 tested / 3549 occluded），但剔除闭环重渲工作量使 on ≈ **3×** off（静态对照 2.997×；orbit 动态相机 4.7×base）如实登记不设通过线（G6 无硬门纪律）。
- 适用面：剔除正确性验证、遮挡密集场景的将来优化基底；**不要在现窗把它当加速开关**。（G37 W4 翻转后 `--hzb on` 为诊断/互斥臂，须显式 `--quality off`。）

### 3.6 GI 档（`--gi on`，默认 off）

- 代价 measured：生产口径 **×3.64~3.93**（1.79/1.93 → 7.03ms；scene GPU ×6.16~6.39）；fps 434.8/407.7 → 133.2（1080p 仍 ≫60fps）。
- 收益 measured：+10.05% 平均 luma（97.46% 像素触及）；对 UE Lumen 在案诚实红未闭。
- 决策在案 = **maintain_default_off**（B6）；你的应用可按同窗数据自权衡，但重判默认档须走立项程序（两条件合取，feature_matrix §5）。

### 3.7 内容面成本预算（在案增量，bistro t100）

| 特性 | 帧耗增量 | 口径 |
|---|---|---|
| 纹理采样 on | **+6.29%** | B4 门 on/off measured |
| 蒙皮角色 | **+4.41ms** frame（6.637 vs 2.229） | B5 门 skin_on/off；骨骼逐帧更新 + BLAS refit |
| 动态实例 refit | prod 2.6722ms（vs 静态同窗 1.76 量级） | A4 门 |
| 动态实例 rebuild | prod 2.8251ms | A4 门（refit 优先、rebuild 回退策略） |
| slab 侧表 device 臂 | eval_ms 单列登记；组合窗 real 3.74~3.80ms | B3 门 + C2 臂 |

### 3.8 默认 full 档旋钮与 PSO 变体账本（G37）

**full 预设钉死档**（展开面字面，与 `--quality full` 同给 = fail-closed）：`--lamp-gain 4` / `--gi2-clamp 0.01` / `--soft-shadow-samples 1`（F1 帧时定档：2 样本 12.96ms 超 90fps 预算，1 样本 9.54ms 过线）。改这三档 = 弃预设走 `--quality off` + 全显式十九臂写法（语义变更即重锚，走重锚程序）。

**可随 full 组合的微调旋钮**（非展开面子参数，等价于显式十九臂 + 该子参数）：

| 旋钮 | 默认 | 杠杆方向 |
|---|---|---|
| `--gi2-ris-m` | 6（∈ [1,16]） | RIS 候选数 K 档：帧时 ↔ 反弹方差（EVAL_RESTIR K 阶梯口径——先方差源头收缩再谈降噪平台化，EVAL_DENOISE 第 0 级同律） |
| `--lamp-k` | 12 | 灯聚类代表点光数：帧时 ↔ 灯光保真 |
| `--lamp-contrib` | 0（全保留） | 贡献剔除阈值 |
| `--autoexp-key/rate/min/max` | 0.115 / 0.08 / 0.125 / 32.0 | AE 目标带与收敛速率（预期收敛行为见 §4 与 PRODUCT_NOTES §2） |
| `--tsrq-min-alpha` / `--tsrq-clamp` | 各自默认 | TSR 质量档 clamp 旋钮（EVAL_DENOISE 第 0 级 K 档阶梯 = 零代码定档路径，实测定档留窗） |
| `--rt-ao-radius/strength/samples` | 0.5m / 0.85 / 2 | 短程 AO 半径/强度/样本 |
| `--transp-alpha` | 0.85（∈ (0,1]） | 玻璃逐面透射率 |
| `--lut <off|neutral|warm|路径.cube>` | off | 第 4 级色彩分级（零新绑定；与 fg/hzb/svt/slab/cluster-lod/wp-hlod 互斥） |

**PSO 变体账本**（W2 落地，默认开零配置）：每次会话 stderr 单行 `[PSO] sessions=… unique_variants=… pso_runtime_creates=…`；`--pso-report <json>` = sidecar 落盘（不加 = 不落盘 0-byte）；`RURIX_G31_PSO_STRICT=1` = era≥1 运行期新变体 fail-closed（默认告警不断跑）。**验收口径 = `pso_runtime_creates == 0`**——窗口管线全部 session 构造期创建，运行期唯一重建点 = era 重建（resize），#113 天然满足；调优时以账本行确认你的组合没有引入运行期管线创建税。

## 4. 真窗口车道口径解读

- `real_render_frame_ms` 含 present 强制的 BGRA8 回读段（8.3MB/帧）——与 bench 车道 prod 口径**不可直接比**；soak 窗（10010 帧零崩在案）real_render=21.19ms 含逐帧 digest 强制回读税（digest_frame_ms 单列），不是渲染退化。
- `present_frame_ms` ≈ 1.0ms（acquire→copy→present→idle；组合窗 0.96~1.01）；`encode_gpu_ms` ≈ 0.10~0.23ms（device 显示编码第五 pass）。
- resize/alt-tab → swapchain era 重建不崩（A3 门在案）；最小化跳过渲染/present 不消费帧预算。
- **AE 预期收敛行为（G37 默认 full 内含 AE，协议内非缺陷）**：resize/era 重建 = AE 状态归零再适应，**~12 帧半衰**；EMA α=0.02 收敛 **~50 帧**——场景切换（进亮/暗区）后 **~1s 完全收敛**为设计内行为（DEFAULT_FLIP_PLAN §4 产品说明字面；产品口径与判缺陷界线见 [PRODUCT_NOTES.md](PRODUCT_NOTES.md) §2）。测量纪律：AE 是 EMA 跨帧反馈臂——画质 A/B 对照一律走无 AE 显式组合（day_0829 红修 #1 教训固化），帧时窗内含 AE 收敛段属预期。

## 5. 已知诚实红面（知情决策）

**G17-MD-F1 焦点格**（bistro-interior/t100/dlss_sr vs UE5 暖态 ue_median=3.4353ms）：

| 窗 | fresh frame_ms_prod | ratio | 终态 |
|---|---|---|---|
| G30 在案 | 3.5767ms | **0.960479** | 17/18 诚实红终判（物理不可达：NGX 宿主开销不可分离） |
| G31 波 A 锚检 | 3.5560ms | 0.966059 | 维持不恶化 |
| 波 C Task C9 同窗 | 3.5046ms | 0.980232 | 不恶化；UE 差 +0.069ms 全落宿主可分离段包络（主源 host_residual_separable） |
| G32 波 B 验收（5 样本中位） | 3.5929ms | **0.956162** | 较前次轨迹恶化**如实 RED 登记不冒充**；同格 digest 五跑全 == 在案锚（确定性零漂移）；帧时预算杠 ×2.0=7.1534ms 远未触及 |

轨迹 0.856 → 0.960 → 0.966 → 0.980(C9 同窗) / 0.956(波 B 中位)——机态漂移面如实登记；重判条件 = ratio ≥ 1.00 新证出现时只追加重判程序（RFC-0032 同源），在案行 0-byte 不回写。

## 6. 自助测量清单

1. 复现基线：`--bench --scene bistro-interior --tier 100 --backend tsr_device --frames 160 --warmup 10` → receipt `stats_post_warmup` + `last_frame_digest`（与 Stage A 锚位级比对）。
2. 环境纪律：`RURIX_REQUIRE_REAL=1 RURIX_VK_VALIDATION=1` + GPU 独占（`ci/gpu_device_lock.py` 串行）；validation 全程静默是绿件前提。
3. 双臂对照：同窗 A/B（如 inflight 1/2/3、gi off/on、textures off/on）→ 双跑 digest 位级 + frame_ms 对比；数字进你自己的 evidence，**勿与本文绝对值混窗比**。
4. DLSS 臂分解：`RURIX_VENDOR_TIMING=1`（dlss-ext 逐帧行）+ `RURIX_G31_NGX_TS=1` / `RURIX_G31_DLSS_EVAL_X2=1` 探针（默认关零行为变更；C9 报告 §分解面）。
5. 窗口默认档纪律（G37 W4 后）：不带 `--quality` = full 十九臂交付形态；复现本文 §2 真窗口表/§3 各单臂杠杆的在案数字一律显式 `--quality off`（+ 对应单臂旗标）；full 档自查以 `--pso-report` 账本行与 `--evidence` 多口径字段为准（§3.8）。

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 初版（G31+ 波 C Task C2）：口径纪律/在案基线/六杠杆（inflight/tier/超分臂/FG/HZB/GI+内容面成本）/真窗口口径/焦点格诚实红/自助测量 |
| v1.1 | 2026-08-30 | G37 商业化收官同步（W5）：§2 真窗口表标注 `--quality off` 口径 + 追加默认 full 档帧时口径（90fps 预算 11.11ms、day_0829 十六臂 9.54~10.70ms 在案、**十九臂终值占位 W4_ANCHORS**）；§3.4 FG 两点式闭集注；§3.5 HZB 显式 off 注；新增 §3.8（full 预设钉死档/可组合微调旋钮表/PSO 变体账本）；§4 追加 AE 预期收敛行为（DEFAULT_FLIP_PLAN §4 字面）；§6 追加窗口默认档测量纪律 |
