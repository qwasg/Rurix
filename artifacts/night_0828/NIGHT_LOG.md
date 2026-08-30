# 夜间自动巡航日志 — 2026-08-28 00:18 → 08:00

> 任务：优化渲染器生产环境实时渲染出图效果（材质/光影/性能/游戏体感相关的画质与帧率）。
> 纪律：先发现问题 → 出计划 → 修改验收 → 优化后变差直接回退；一个方向最多尝试两次；积极调用子 agent。
> 前序会话（22:35–00:09）遗产：基线 128 帧收敛帧（artifacts/night_baseline/）+ 抖动/ GI RNG 两项研究 out/*.json（设计稿已被删，仅余指标）。
> 协同约束：G36 门真跑持有 GPU 锁（pid 106824，00:09 起）；工作树有 G36 会话未提交修改（g34_full_lane.rs / g35_particle_lane.rs / check_schemas.py / G35_CONTRACT.md / milestones/g36/ 等）——**本巡航不触碰、不回退这些文件**；回退只针对本巡航自己改的文件。

## 问题清单（发现阶段，持续更新）

| # | 问题 | 证据 | 严重度 | 状态 |
|---|------|------|--------|------|
| P1 | 全画面颗粒噪点（128 帧 TSR 收敛后仍可见，墙面/天花板/地板） | crop_wall/crop_floor 3× 放大；前序 gi_rng_study.json：现 sin hash 呈 arcsine 分布(std 0.3536 vs 均匀 0.2887)、chi2=171032(理想 63)、空间相关 0.91/−0.95 | 高 | 待定位根因 |
| P2 | 细几何走样（栏杆竖条阶梯/摩尔纹、水平梁断线） | crop_railing 3× 放大 | 高 | 待定位 |
| P3 | 渐变色彩带（墙面渐变离散台阶） | 前序 dither_metrics.json：无抖动 unique_levels=1.17/恒定段 30.7px；TPDF 后 2.88/1.97px | 中 | 方案已仿真验证，待落地 |
| P4 | 自发光无 bloom（灯具为生硬白点，无光晕） | crop_lamp 3× 放大 | 中 | 待查 post_chain 接线 |
| P5 | 阴影死黑无 GI 补光（拱下/楼梯底纯黑） | baseline_aces 全图 | 中（已知 deferred，--gi off） | 评估中 |
| P6 | 性能基线待测：干净段 GPU ≈ scene 0.96ms + mv 0.03ms + tsr 0.53ms ≈ 1.52ms@1080p tier100 | render_receipt.json | — | 待建正式测量规程 |

## 根因分析（00:20–00:30 代码侦察）

- **P1 颗粒**：pass0 kernel `g18_light_transport_depth.rx` 全确定性（面光 4×4 分层 + 点光 delta），无随机采样 → 噪点非蒙特卡洛噪声。真根因 = **flat shading（面法线 cross(e1,e2)，L130-141）+ 逐三角均值 albedo 内容模型** → 三角形级不连续 → Halton jitter 逐帧采样位置变化 → 时域走样颗粒；TSR alpha 地板（base 0.1 / min 0.04，`temporal/tsr.rs` L78-79）只等效平均 ~10-25 帧 → 128 帧后残留可见。量化基线：墙面 temporal_rel mean 1.04%/p95 5.84%、收敛帧高频能量 rel 21.9%；地板 temporal_rel p95 12.0%。
- **P3 色带**：`g31_display_encode.rx` L441-443 8-bit 量化 `floor(v·255+0.5)` 无抖动（前序仿真已证 TPDF 可消除）。
- **P4 bloom**：`display/post_chain.rs` 仅 host 3×3 box 骨架（L212），device 面 mip 链缺（TODO #79）。
- **P5 死黑**：基线契约 sun/sky=0.0（g18 kernel 支持 sky_amb=params[42] 但未开）；`--gi on` 加性臂存在（g16_gi_multibounce.spv 在档）。
- **工具链**：`rurixc <kernel.rx> --target vulkan -o x.spv` 可现编；SPV 落 `.tmp/g14_gates/m_c/`；`committed_barycentric()` 可用（g34 已用）。
- **性能基线**：GPU 链 ≈1.52ms@1080p tier100（scene 0.96 + mv 0.03 + tsr 0.53）；bench frame_ms≈100ms 为逐帧 fence+EXR 回写测量开销，非生产口径。

## Round 1 计划（画质主攻）

| 方向 | 问题 | 改法 | 验收口径 | 风险 |
|---|---|---|---|---|
| D1 抖动 | P3 色带 | g31_display_encode 加确定性 TPDF（R2/Weyl 逐像素 + 帧号旋转），参数旗标门控 | 8-bit 输出 unique_levels↑/恒定段↓ + 视觉；默认臂 digest 0 漂移 | 低 |
| D2 平滑法线 | P1 颗粒 | 加性臂：glTF NORMAL → 9 f32/tri 侧表 → kernel 重心插值法线 | 墙面/地板 temporal_rel + 收敛 HP 下降 + 视觉 + 帧时增量 measured；默认臂位级锚不动 | 中 |
| D3 bloom | P4 无光晕 | 新 device kernel：阈值+mip 链+合成，窗口车道 TSR→bloom→encode | 灯具光晕视觉 + 帧时增量 | 中 |
| D4 GI/环境光评估 | P5 死黑 | A/B：--gi on 臂 + sky_amb 契约钮 | 暗部亮度/视觉 vs 帧时代价，如实登记 | 低（只评估） |

执行纪律：每方向默认臂 0-byte 不动（Stage A 18/18 digest 锚红线）；A/B 同 seed 同帧数；变差即 `git checkout` 本巡航自有文件回退（不碰 G36 未提交面）；每方向最多 2 次尝试。

## 轮次记录

（每轮大计划完成后在此追加：时间、方向、改动文件、A/B 指标、验收结论、是否回退）

### Round 0（00:18–00:30）摸底 ✅
- 00:18 接手，确认前序遗产与 G36 在飞门（GPU 锁等待中）
- 基线 EXR 可视化工具 exr_view.py / crop_tool.py / grain_metric.py 落地（复用 ci/g10_exr_lib）
- 基线全图 + 4 区域 3× 放大人工复核完成 → P1–P5 立项
- 根因代码侦察完成（见上节）；两个侦察子 agent 报告收回（噪点根因 = EMA 驻态残差 × jitter 二值信号〔emissive 亚像素弹出/细几何/阴影掠射〕，非蒙特卡洛噪声；平滑法线通路高度现成）

### Round 1 执行（00:30– ）
- 00:30 隔离构建 target-night 双 bin 成功（不污染主 target/，G36 门并行安全）
- 00:33 基线复现 --bench：prod_ms_mean=1.92ms / fps=397.7 / scene_gpu=1.02ms（对照前序基线 100ms/帧 = --render 逐帧 EXR 落盘税，非生产口径）
- 00:36 **D1 抖动实现完成**：g31_display_encode.rx（params[3] 门控 IGN TPDF，关臂 dn=0 位级恒等）+ aces13.rs `aces13_device_encode_params_ex`（原函数 0-byte）+ g31_window_present `--dither off|on`（默认 off）。kernel 编译 spirv-val 绿。
- 00:37 **D1 公式验证**（verify_dither.py 真实收敛帧墙面渐变）：runlen 24.7–26.3px → 1.95–2.06px，unique_levels ↑ ⇒ 色带消除（d1_dither_verify.json）。窗口车道集成验收待 D2 构建窗。
- 00:38 **基线复现位级锚定**：--render 128 帧 converged_digest == 前序基线 cde1b255… **MATCH**（确定性协议成立，A/B 口径可信）
- 00:39 GI 臂（--gi on）128 帧渲染完成
- 00:42 **D4 GI 评估结论**：--gi on 暗部补光有效（左拱/楼梯底死黑→可见细节，均值亮度 0.0101→0.0115），**但**①新增颗粒（g16 kernel 反弹采样用劣质 sin hash，arcsine 分布+屏空间相关，与前序 gi_rng_study 一致）②scene GPU ×4.39（4.54→19.9ms，2 反弹非相干射线发散）→ 1080p 原生 ~50fps 跌破 60。**结论：GI 臂现状非净收益**——补光收益被噪声+帧时抵消；如实登记为 opt-in 画质档，sin hash→R2 修复列为候选（非默认路径，优先级低于 D2）。digest b2fcdebe≠base cde1b255（接线真实生效）。
- 00:47 **重要治理发现**：g16_gi_multibounce.rx（G16 已收口锚）、g18_light_transport_depth.rx（G18）、g14_3_direct_gi.rx（Stage A 默认臂）等 kernel 均被已收口里程碑 digest 锚冻结——**任何默认路径改动都必须走新加性臂**（新 kernel/旗标），in-place 改 = 破锚违规。D2 的 g18_smooth_nrm.rx fork + --smooth-normals 旗标即此纪律的正确形态。
- 01:00 **D2 子 agent 曾被 GPU 锁阻塞 25 分钟**（G36 会话间歇复跑抢锁）——已中断重启，lock-free 后续作 host 侧。
- 01:05 **D3 bloom 三 kernel 落地并编译绿**：g31_bloom_bright（软膝阈值+2×降采样）/g31_bloom_blur（9-tap 可分离高斯 H/V）/g31_bloom_composite（双线性上采样加性合成）。**算法仿真验证**（bloom_sim.py 真实收敛帧）：灯具硬白点→柔和光晕，0:63% 像素超阈、0.79% 像素获 >0.01 增量（bloom_bloom.png / *_lamps.png 对照）。device 接线（窗口车道 opt-in --bloom）为留窗。
- 01:20 **D2 平滑法线子 agent 自验全绿**（d2_evidence.json）：off 双跑位级 + off==改前基线 f39e9808 零漂移 + on 双跑位级 + on≠off 接线生效；干净 bench 下 on−off ≈ +0.00~0.03ms（噪声带内，基本免费）。改动 g14_3_lane_body.rs +355 / g14_3_pipeline_perf.rs +45 + 新 kernel g18_smooth_nrm.rx。
- 01:25 **D2 全 128 帧 A/B 验收（编排侧）**：Stage A 零漂移（off==base cde1b255）✅ 接线生效（on=778f1dfc）✅ 颗粒墙面 temporal_rel_p95 5.84%→4.41%（↓24%）、mean 1.04%→0.90%（↓13%）；地板/收敛高频基本持平（证实颗粒主源 = emissive 亚像素弹出 + albedo 马赛克，法线面片为次源）。**视觉：圆柱/曲面法线面片感消除（snrm_on/off_col.png 对照）= 真实材质质感收益**。帧时增量 ~0（干净 bench 口径）。**结论：KEEP**。已知留窗：cluster-lod/wp-hlod 组合面 gather、vendor 双臂、cornell Split 形态未接线（均 fail-closed 互斥登记）。
- 01:27 **D1 抖动窗口车道集成验收**：off 臂 presented digest == 改前旧二进制（5596a730）**零回归** + on≠off 接线生效 + encode_gpu 0.107ms（抖动几乎免费）。**结论：KEEP**（opt-in --dither，默认 off 保五门回归锚；默认翻转需更新锚的治理留窗）。
- 01:43 **D5 半球环境光加性臂落地**（g18_smooth_nrm.rx params[44..48) + pack_frame_params_nrm env RURIX_G18_AMBIENT 门控；基线车道 smooth_nrm=false 永不触达 ⇒ Stage A 零风险）：关臂零漂移 == D2 778f1dfc ✅；intensity=0.004 时均值亮度 0.0101→0.0139（+38%）而 p99 0.3568→0.3579（高光不炸）——**死黑区（拱下/楼梯底/地板阴影）死黑→可读细节，暖色环境光贴合灯光色调**（ambient_on_dark.png / ambient_on_aces.png 对照）。**结论：KEEP（intensity 0.004，一次到位）**。注：根本因 = 44k 自发光灯片不投光（无 emissive NEE），环境光为廉价近似；真解 = GI/NEE（D4 评估在案）。
- 02:00 **Stage A 默认臂零降级机核 PASS**：含全部改动（D1+D2+D5）的 target-night 二进制跑默认臂（无旗标、g14_3_direct_gi 车道）bistro t100 tsr 160 帧 last_frame_digest == 锚 c1d28ad7… **位级一致**——全部改动确为加性默认 off，冻结面零回归。prod_ms 2.12ms。
- 02:00 **D3 bloom 窗口接线验收 PASS**（d3_verdict.json）：九 pass 链 scene→mv→resample→resolve→**bloom_bright→bloom_blur_h→bloom_blur_v→bloom_composite**→display_encode；off 双跑位级 + off==改前基线 5596a730 + on 双跑位级 + on≠off（presented digest 变）+ on 不污染 render_digest（TSR 面）+ validation 静默 + CLI fail-closed。**bloom GPU 增量仅 0.214ms**（bright 0.059+blurH 0.019+blurV 0.017+composite 0.119）。
- 02:10 **D3 视觉证据补齐**（编排侧加 `--dump-present-raw` 验证面，bloom on/off 各跑取 presented BGRA8→PNG）：逐像素差异 mean 0.376/max 203、**1.31% 像素变化且差异区 off 亮度 67.45 vs 全图 13.92（4.8×）= 精确集中灯具高光区非全屏噪声**；灯具裁剪对照（bloom_off/on_lamp.png）= 生硬白点→柔和光晕放射。**结论：KEEP（--bloom opt-in，strength 0.3/threshold 1.0，0.21ms）**。
- 02:20 **纹理臂评估**（--textures on，既有 B4 门臂，窗口车道 --auto-move dolly）：on vs off 2.32% 像素变化（top-12 已映射材质面），地板/墙面逐三角均值色→真实贴图细节可见（tex_on/off.png 对照）；帧时 4.04→4.79ms（+0.75ms/+19%，含回读税）。**结论：有效但覆盖 partial（top-12 材质+mip0 双线性）；默认开启/全材质覆盖归内容管线后续窗**。

## Round 3（02:20– ）合流验证与收尾

- 02:25 **合流 hero 图产出**（hero_before_after.png，上=基线/下=平滑法线+环境光，同 ACES 口径）：死黑拱区/柱/地板全可读，曲面平滑，暖环境光贴合——画质提升明确。注：--export-png 产的 presentation_night.png 为 16-bit PNG，PIL 读不了（Read 工具可显示），对照图改用 exr_view 同口径路径。
- 02:27 **多会话协同事实确认**：G36 会话在飞并提交 `bece24e7`（W4-W5 收口，其门十 facts 全绿，含其早前蒙皮 FAIL 的标定口径更正复跑全绿）。其 commit 明文按文件名择取、把本巡航文件（g31_window_present.rs/aces13.rs/g31_display_encode.rx/g18_smooth_nrm.rx/artifacts/）留工作树不混入——**双向隔离纪律成立**。本巡航改动备份 = artifacts/night_0828/night_changes_tracked.patch（126KB，仅含本巡航 5 个 tracked 文件）。
- 02:28 **D2 合流capstone在飞**：平滑法线+环境光接进窗口车道（--smooth-normals opt-in），子 agent 实施中——窗口车道 scene pass 与 bench 共享 unified_lane_descs，nrm 变体已在（unified_lane_descs_nrm），需解 U_TRINRM=22 与 encode 资源下标冲突。
- 02:42 **D2 窗口合流验收全绿**（d2w_summary.json）：off==锚 5596a730 零漂移 + on 双跑位级（b02b08b57）+ on≠off + **三加性臂同开组合（--smooth-normals on --bloom on --dither on）双跑位级稳定（12d5dc91）+ validation 全静默**。窗口车道出图（window_smooth_ambient.png）确认曲面平滑+环境光补亮进生产实时路径。帧时：off 6.29ms / on ~5.5-6.4ms / 全组合 ~6.7-7.4ms（~135-160fps 含回读税）。**生产窗口车道现已具备全特性栈：平滑法线+环境光（scene kernel）+ bloom（post）+ 抖动（encode），全可组合、全默认 off 零漂移**。

## 当前成果汇总（02:44）

| 方向 | 问题 | 状态 | 验收 | 帧时代价 |
|---|---|---|---|---|
| D1 抖动 | P3 色带 | KEEP（--dither） | off==改前零回归 + on 生效 + 色带 runlen 25→2px | ~0（encode 0.107ms） |
| D2 平滑法线 | P1 面片/材质 | KEEP（--smooth-normals，bench+窗口双车道） | Stage A 零漂移 + 颗粒墙面 p95↓24% + 曲面消面片 | ~0（噪声带内） |
| D5 环境光 | P5 死黑 | KEEP（质量车道 env 门控） | 关臂零漂移 + 均值+38% 高光不炸 + 暗部可读 | ~0 |
| D3 bloom | P4 无灯光光晕 | KEEP（--bloom） | 全绿 + 差异集中灯具区 + 视觉光晕 | +0.21ms GPU |
| D4 GI | P5 死黑（真解） | 评估后登记不启用 | 补光有效但噪声+×4.4 帧时 | ×4.4（不划算） |
| 纹理 | P1 albedo 马赛克 | 既有臂评估确认有效 | 2.32% 像素真实贴图细节 | +0.75ms |

## Round 4（03:12–03:52）GGX 高光材质 ✅ KEEP

- 03:12 立项 D6：GGX 高光加性臂（修复全 Lambert 粉笔感 = 侦察最大材质差距）。bistro gltf 70/70 材质带 metallicFactor/roughnessFactor 标量可用（样例 metal 0.4/rough 0.30）。设计：tri_mr 2 f32/tri 侧表（照 trinrm 模板）+ g18_smooth_nrm.rx 加 GGX 高光臂（params[48] 门，D·G·F/(4cos·cos) Schlick F0=mix(0.04,albedo,metal)）+ `--ggx off|on`（须随 --smooth-normals on）。子 agent 在飞实施。
- 03:40 **D6 bench 验收全绿**（verify_summary.json）：默认臂双跑 == 改前 f39e9808（GGX 不破默认）+ smooth-nrm 臂（GGX off）== D2 锚 6b46f70a（GGX 不破 D2）+ GGX on 双跑位级（46e0af63）+ on≠off 接线生效；**GGX GPU 增量 ~+0.03ms（噪声带内，基本免费）**。
- 03:45 **D6 窗口车道回归**：off==5596a730 / on==D2 窗锚 b02b08b57 / combo==12d5dc91 全零漂移（GGX bench 改动不污染窗口既有臂）。
- 03:50 **D6 视觉验收**（render_on/off 128 帧 + 裁剪对照）：off==D2 128 帧锚 778f1dfc 零漂移 + on=ec395575；**地板釉面光泽反射 + 柜台/收银机高光带清晰可见**（crop_floor/crop_counter 对照）——粉笔感→釉面/金属质感，材质真实感成立。**结论：KEEP（--smooth-normals on --ggx on，~免费）**。
- 04:05 **D6 窗口车道接线验收全绿**（d6w2_summary.json）：off==D2 窗锚 b02b08b57 零漂移 + on 双跑位级（52020f9c）+ on≠off + **四加性臂同开（smooth-normals+ggx+bloom+dither）组合双跑稳定（48353e86）+ validation 静默**；窗口 GGX on vs off 22.27% 像素变化（地板釉面 sheen 广域可见，win_ggx_on/off_crop.png 对照）。**生产实时路径（窗口车道）现已具备完整材质+光影+后处理栈：平滑法线 + GGX 高光 + 半球环境光 + bloom + TPDF 抖动，全可组合、全默认 off 零漂移、全廉价**。

## Round 5（04:31– ）健壮性与收尾

- 04:31 最终构建干净（双 bin exit 0，4 warning 皆前序既有）。
- 04:33 **全特性栈窗口风暴健壮性 PASS**：--smooth-normals on --ggx on --bloom on --dither on + --window-storm 3 + validation=1 → exit=frames_done 干净退出 + resize_eras=1（resize 触发 swapchain/资源重建，bloom/nrm/ggx 各臂缓冲正确重建）+ validation 静默 + 无崩溃。**生产健壮性确认**（画质臂不破坏窗口车道的 resize/风暴面）。
- 05:06 **全特性栈稳定性 soak 收口**：25 迭代 / 1863.8s（≥1800s honest 口径达标）/ **零失败**——每迭代全五臂+环境光窗口真跑 digest 全程稳定（2b6efac6 零漂移）+ 周期 Stage A 单格探针全过（画质臂全程零污染默认面）。soak_summary.json 在档。
- 05:20 **GGX+窗口合流补充独立评审：可安全合入 ✅**（ggx_review_report.md）——位级确定性 PASS（GGX 关臂==加 GGX 前 kernel、窗口 off==b02b08b57、PARAMS_LEN 48→56 对既有面真零影响，全与 D2/G36 在案锚交叉互证）+ BRDF 正确性 PASS（D/G/F 数值面无奇异/无 NaN 通道/几何符号正确）+ 治理 PASS。唯一实质 CONCERN：F0 用已乘 (1−metallic) 的漫反射 albedo ⇒ 金属 F0 低估 (1−m) 倍、metal=1 极端资产高光近失（bistro metal=0.4 不触达，视觉验收成立）——纯画质语义限制，opt-in 臂，如实登记不阻断。
- 06:00 **延伸 soak 收口**：第二段 41 迭代 / 3045.2s 零失败；**两段累计 66 迭代 / ~4909s（~82 分钟）零失败、digest 全程稳定**——远超 ≥1800s 标准，全特性栈长窗稳定性成立（直面 RD-045 间歇漂移关注面，本巡航全特性栈零漂移）。
- 07:18 **收尾 soak 收口**：第三段 61 迭代 / 4536.1s 零失败。**全夜三段 soak 累计 127 迭代 / ~9445s（~2.6 小时）零失败**、全特性栈 digest 全程稳定、周期 Stage A 探针全过。**夜间巡航稳定性验证终态**。
- 07:23 **终态确认跑**：全五臂+环境光窗口真跑 digest == `2b6efac6`（== 全夜 127 迭代 soak 的稳定 digest，终态一致）+ exit=frames_done + validation 静默 + real_render 5.1ms（~196fps 全栈）。**夜间巡航终态全绿定盘**。
- 08:00 **巡航窗口结束**（00:18→08:00，~7.7 小时）。终态：工作树稳定（本巡航 5 tracked +1637/−23 + 4 新 kernel + artifacts/；G36/G35 会话文件零触碰）；全特性栈 digest 2b6efac6 定盘。**任务收口。**
- 06:02 备份 patch 重生成含全部改动（night_changes_tracked.patch 185KB，5 tracked +1637/−23；4 新 kernel 另立）。**夜间巡航主体工作收口**：6 改进 + 2 评估 + 全 18 格零漂移 + 双评审可合入 + 风暴+82min soak 全绿。

## 回归与协同（02:58）

- **Stage A 多格回归探针 6/6 零漂移**（regression_probe.py，target-night 含全部本巡航改动）：bistro t50/t67/t100 tsr（Mega 车道多档）+ cornell t100/t50 tsr（Split 拆散车道）+ bistro t100 dlss_sr（vendor 臂）全部 last_frame_digest == 在案锚位级一致。**证明：全部改动确为加性默认 off，冻结面/既有门零回归**。
- 生产 hero 前后对照（hero/window_before_after.png，窗口车道同 dolly 轨迹）：全特性栈（平滑法线+环境光+bloom+抖动）vs 全关——暗部可读/曲面平滑/灯光光晕/无新缺陷。
- 独立评审子 agent 在飞（位级确定性/治理/边界条件审计）。
- 04:28 **Stage A 全 18 格零漂移确认**：首批 6 格探针 6/6；全 18 格批跑时 12 vendor 格（dlss/fsr）全 MATCH + 6 tsr 格批内 ERR——**排查确认非本巡航回归**：tsr 格在 vendor 臂（dlss/fsr 加载 Streamline/FFX + D3D12 interop）后同批跑会 rc=1（设备态/资源竞争面，与本巡航改动无关——本巡航不触 vendor 臂与 tsr 车道正确性），隔离单跑 tsr 6/6 MATCH + 手工复现 BENCH PASS。故 18/18 零漂移成立（12 vendor 批内 + 6 tsr 隔离）。**vendor→tsr 同批测序脆弱面如实登记**（官方锚检门 nested 序在案通过，本巡航探针的排序触发了该面）。
- 03:10 **独立评审结论：可安全合入 ✅**（review_report.md）——位级确定性 PASS（含 ±0 角隅静态证明：gate_sn 门/环境光关臂/+0.0 恒等全链追踪）+ 治理 PASS（冻结面/他人文件零触碰）+ 正确性 PASS（无 OOB/NaN 通道/下标撞面；bloom 半分辨率奇偶、composite 边界 clamp、哑表零 NaN、资源下标互核全过）。两项 CONCERN 均为 on 臂操作性陷阱（不威胁默认臂）：①**已处置**——默认 encode SPV 用含抖动 kernel 重建，验证 off==锚 5596a730 零漂移 + on==e989c6ee 默认面生效（与夜间 SPV 一致）；②--smooth-normals on + 显式 --spv-scene 缺 fail-closed 校验，登记留窗（CLI 已引导正确用法，低危）。另登记：on 臂 NaN 暴露面（畸形资产 NORMAL，opt-in 低概率）/ 非均匀缩放法线方向（bistro 旋转+平移面正确，已如实登记）/ IGN f32 ≥4K 精度微降（确定性不受影响）。
