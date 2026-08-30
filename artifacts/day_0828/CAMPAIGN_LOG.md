# 生产渲染画质修复战役日志 — 2026-08-28 09:11 起

> 任务：修复 GPU 实时生产管线出图的四类问题——曝光 / 死黑 / 异常色块 / 噪点。积极调用子 agent，允许大规模推进。
> 计划：`~/.cursor/plans/生产渲染画质修复战役_f9cbb373.plan.md`（Phase A 灯光+曝光 → B 统一质量 kernel+纹理全覆盖 → C GI R2 → D TSR 降噪 → E 合流回归 soak）。
> 前序遗产：夜间巡航（00:18–08:00，artifacts/night_0828/）交付 5 个 opt-in 画质臂（--dither/--smooth-normals/--ggx/--bloom/env 环境光），全部验收 KEEP、默认 off 零漂移；根因侦察与指标工具在档。

## 纪律

- 冻结面红线：默认路径 kernel（g14_3_direct_gi/g16_gi_multibounce/g18_light_transport_depth）0-byte；所有改动走加性臂（新/自有 kernel + 旗标 + params 空槽 + 哑表）；关臂位级零漂移（Stage A 18 格锚 + 窗口 off 锚 5596a730）。
- 每方向：改前锚定 → 实现 → off==锚 + on 双跑位级 + A/B 指标 + 视觉 → 变差回退；最多 2 次尝试。
- 构建隔离 target-night/；GPU 真跑过 ci/gpu_device_lock（run_render.py）；RURIX_REQUIRE_REAL=1。
- 绝不碰他会话文件：00_MASTER_INDEX.md、11_ROADMAP.md、milestones/g35/G35_CONTRACT.md、milestones/g36/**。不 git commit。
- g14_3_lane_body.rs 为共享 include 文件：实现类子 agent 串行修改，禁并行编辑。

## 根因备忘（侦察结论）

- 曝光暗+死黑 = 契约光照标定：4 盏 0.018cd 点光 + sun/sky=0 + 44k emissive 灯片不投光；×16 曝光已在链内（TSR resample 折叠），ACES 标准无误。原始辐亮均值 ~0.0006。
- 异常色块 = 逐三角均值 albedo 马赛克（--textures 仅 top-12 材质 66.7% 三角、mip0）；纹理臂×平滑法线臂互斥待解除。
- 噪点：生产臂 = TSR EMA 驻态残差×（emissive 亚像素弹出+albedo 马赛克+细几何）；GI 臂 = g16 L317-320 sin hash（arcsine+屏空间相关），R2 仿真已验证（chi² 171032→0.05）。
- 点光 SSBO stride=8 f32（槽 6/7 空）→ 逐灯遮蔽半径可零布局携带；params[49..56) 空槽可用。

## 轮次记录

### Round 0（09:00–09:11）接手与规划 ✅
- 复核夜间日志/总结/对照图；两个探索子 agent 报告收回（管线结构 + 遗留问题证据，根因全定位）。
- GPU 空闲确认（无在飞任务）；target-night 双 bin 新鲜（03:33/03:55 构建）。
- 计划批准，战役开始。

### Round 1（09:11– ）Phase A1 灯光提取臂（在飞）
- 09:14 A1 实现子 agent 派出：extract_lamp_lights（0.6m 网格+union-find 聚类→top-K 通量灯，I=Φ·gain/4π，逐灯半径进 points 槽 6）+ --lamp-lights/--lamp-gain/--lamp-k/--lamp-contrib 双车道旗标 + g18_smooth_nrm.rx 点光循环 radius t_max 截断与贡献剔除门（params[49]）。验收：默认臂==Stage A 锚 + off/on 双跑位级 + scene_gpu ≤3ms + 128 帧亮度指标 + 窗口 off==5596a730。
- 09:14 B 相侦察子 agent 派出（只读）：蓝色块/红斑材质归因 + 图集全覆盖/mip 链设计提案（atlas_design.md）。

### Round 1 追加：B 相侦察报告收回（09:50）✅
- **红斑归因 = (a) 均值马赛克**：右墙 = 画作 Paris_Paintings(mat 51, rank 56) + 鎏金框(mat 50) + 红灰泥墙(mat 27, rank 50)；红帘 curtainB1（26,252 tri 饱和红）rank 13 恰被 top-12 切掉。画布 DDS 是真实画作（2048² BC1 12 级 mip）——全覆盖直接修复。
- **蓝色块归因 = (c) 窗口车道显示异常**（非纹理问题）：吊扇 Paris_CeilingFan(mat 40) 契约暖橙 Le，bench EXR (1500,12)=(0.357,0.254,0.106) 正确，窗口 dump 同像素 (R0,G62,B170) 饱和蓝。已派专项 debug 子 agent（切层对账：dump 工具通道序 / scene kernel 差异 / TSR / encode BGRA）。
- **图集设计定案（atlas_design.md）**：推荐方案 C = 线性 texel heap（一维 SSBO 手动寻址，u32 偏移头表进 buffer 头部，零新增绑定）+ DDS 源 mip 直搬；cap-1024 全 mip 链 = 283 MiB 起步；nearest-mip+bilinear（4 fetch 不变），lod=log2(th·k_pix·k_tri·tex_w)，th 命中距离 kernel 寄存器现成。资源下标：tex 五件留 24..28，trinrm→29、tri_mr→30（解撞 24/25）；互斥解除点 g31_window_present.rs L4913-4921；新合流 kernel g31_texture_nrm_gi.rx。
- **实 bug 捕获**：g31_texture_gi.rx L223/L226 双线性底行 G/B 通道误用 fy 做水平混合（R 通道 L220 正确）；host 镜像 L5612/5615 同错故对拍恒绿；同源传播 7 kernel。Phase B 8 处同步修（host+device 同步改，探针保持位级绿）。

### Round 1 收口：A1 灯光提取臂验收全绿（10:04）✅ KEEP
- **交付**：`--lamp-lights on|off` + `--lamp-gain/--lamp-k/--lamp-contrib`（bench+窗口双车道）；extract_lamp_lights（0.6m 网格+union-find，44,024 emissive 三角→13 簇→top-12：6 吊灯+4 壁灯+1 大合并簇+1 弱簇，弃 1 簇如实登记）；PointLight.radius 走 points SSBO 槽 7；kernel 半径阴影截断 + params[49] 贡献剔除门（branchless gate——rurixc「if 包 while」codegen 缺陷绕行，缺陷已登记）。
- **验收 7/7 PASS**：默认臂 == Stage A 锚 c1d28ad7 ✅；smooth+lamp off == 昨夜锚 6b46f70a/778f1dfc（kernel 演进零漂移）✅；lamp on 双跑位级（9f37f8c1）✅；窗口 off == 5596a730 ✅ on 双跑稳定（db7d48f7）+ validation 静默 ✅。
- **帧时**：scene_gpu 0.95→2.88ms（16 灯 vs 4 灯，≤3ms 达标，未动用 contrib 剔除）。
- **亮度**（converged.exr 线性 mean/p5/p99）：off 0.00985/0.0/0.265 → g4 0.0137/0.0007/0.268 → g16 0.0254(×2.58)/0.0025/0.280（p99 仅 +5.6% 高光不炸；p50 ×42 死黑地板全脱纯黑）。gain 定档留 A3。
- 遗留：大合并簇 r=3.11m 遮蔽豁免球偏大（本视角无伪影）；--lamp-contrib >0 档未实测；A1 单独回退需按 ACCEPTANCE_SUMMARY 手工摘段（与夜巡改动同文件耦合）。
- 证据：artifacts/day_0828/a1_lamp_lights/（ACCEPTANCE_SUMMARY.json + extract_stats.json + png/）。

### Round 2（10:08– ）Phase A2 自动曝光臂（在飞）
- 10:08 A2 实现子 agent 派出：g31_autoexp_reduce/state 两微 pass + encode 增益槽（off 绑 1.0 哑 buffer 位级恒等）+ --auto-exposure/--autoexp-key/rate/min/max；验收含 off==5596a730 复验、组合臂双跑位级、dolly 240 帧适应曲线（无振荡+目标带）。

### Round 2 收口：A2 自动曝光臂验收全绿（11:02）✅ KEEP
- **交付**：`--auto-exposure` + key/rate/min/max 四参数；g31_autoexp_reduce（256 线程跨步 log2-luma）+ g31_autoexp_state（串行归约→几何均值→EMA→写 enc_params[133]）两微 pass；encode IDT 前增益（≤0→1.0 恒等守卫）。
- **设计偏差（合理）**：原案新增 storage 绑定被否——默认 encode SPV 被 g34_full_lane/g35_particle_lane（他会话）以 3 绑定共享消费，新增绑定会破他们车道 ⇒ 改走 params[133] 预留槽，绑定数零变化，跨会话零破坏。
- **验收 6/6 PASS**：off==5596a730 复验 ✅ dither 臂==e989c6ee 在案锚 ✅ AE 单臂 fd5ca68c/六臂组合 a4695558 双跑位级+validation 静默 ✅ dolly 240 帧适应曲线：presented mean 0.13(off)→0.40(on) 入带、最大逐帧变化 0.42% 零振荡 ✅。
- **代价**：reduce+state 0.11–0.19ms（超 <0.1ms 期望如实登记，单 workgroup 结构代价，两级归约优化留窗）。
- 遗留：AE 提亮暴露墙面均值 albedo 色带（归 Phase B）；resize 后 EMA 复位 ~12 帧半衰；gpu_device_lock 解锁段偶发 PermissionError + stale holder pid 6028（基建观察面）。
- 证据：artifacts/day_0828/a2_autoexp/。

### Round 2 追加：蓝色块根因钉死（10:21）✅ —— encode kernel ACES 样条转置 bug
- **根因**：`g31_display_encode.rx` ACES 1.3 分段样条基函数手工展开写成转置形（M·cf 写反 cf·M，12 处 b1/b2 全错，L213-214 起 c5/c9×RGB×low/high）。错基破坏节点连续性 ⇒ 色调曲线非单调+段间跳变：中亮饱和色三通道落不同段 ⇒ 色相反转（暖橙扇叶 R 压 0/B 抬 0.38 = 饱和蓝）；暗部同段 ⇒ 色相保持但系统性提亮（故墙面两车道色相一致未暴露）；灯罩同心环带 = 非单调曲线铁证。
- **证据链**：TSR 输出 f32 @fan 双车道一致暖橙 ⇒ 翻转唯一在 encode；逐字 f32 仿真全帧 99.9918% 位级复现实测（fan (0,62,170) 逐字节相等）；改正基后 fan→(144,122,77) == host aces13.rs f64 金标准，0.18 灰→99（ACES 0.104 设计点）。bluefan_rootcause.{json,md} + fan_bug_vs_fixed.png 在档。
- **影响面**：仅窗口 presented 面（render_digest/bench EXR 在 encode 上游不受影响；--export-png 走 host f64 正确）。夜巡 P3 色带窗口观测可能混入本缺陷贡献（跳变曲线自制色带），修复后复测。
- **修复计划（A2b，A2 落地后执行避免同文件冲突）**：12 处两行改写（b1 删 +0.5·cf2、b2 加 +0.5·cf1；b0 两约定同值不动）→ 重编 SPV → 全窗口 presented 锚重定基（5596a730 系全部演进，新锚双跑收割）→ 新增 device-vs-host encode parity 探针（防复发——夜巡只验了确定性 digest 未验 host parity，此为漏网通道）。
- 旁支缺陷另立不追：presentation_night.png zlib 流损坏（--export-png 写出面）。
- 意义：「异常色块」问题的窗口显示异常部分（蓝块+环带）根因即此；修复后暗部会比现在更暗（bug 曾提亮暗部）——由 A1 灯光+A2 自动曝光正确补偿，属正确性回归。

### Round 3（11:02– ）Phase A2b 样条修复+锚重定基（在飞）
- 11:02 A2b 子 agent 派出：12 处样条 b1/b2 改写 + 共享 SPV 消费审计（固定 presented 锚门清单→决定覆盖共享路径 or 新 SPV 路径隔离）+ 四臂锚重定基 + fan 像素/环带对照 + device-vs-host parity 探针 + AE 曲线复测。
- 11:21 A2b 会话中断（挂掉），已 resume 续跑（先盘点现场：git diff 断点/已落证据/SPV 时间戳/GPU 锁遗留，从断点继续不重跑已完成步骤）。
- 11:47 **主线中检确认 A2b 存活推进中**（长批处理内，evidence 持续落盘）。已核实的中间事实：
  - 12 处样条修复在位（12× cf1/cf2 修正项）；
  - **parity 探针 PASS**：2,073,600 px 中 99.9891% 精确匹配、p100=1 LSB、>1LSB 像素 0；fan (1500,12) = (144,122,77) 与 host f64 金标准逐字节一致；0.18 灰→99（display 0.104 设计点）✓；
  - **视觉修复确认**（fan_ring_before_after.png）：灯罩同心环带→平滑渐变、蓝扇叶→暖卡其、天花板杂色纹消失；
  - **治理走 v2 隔离路**：G31_DEFAULT_SPV_ENCODE → .tmp/night_0828/spv/g31_display_encode_v2.spv（共享 m_c SPV 0-byte 不动，g34/g35 他会话零影响）；
  - **bench 零影响确认**：默认臂 last_frame_digest == Stage A 锚 c1d28ad7 ✓；
  - 新锚（8 帧口径双跑位级）：off 55e4a92d / dither 5abd765f / AE 790809aa（旧 5596a730/e989c6ee/fd5ca68c 系作废待汇总登记）；combo 双跑与 AE dolly 曲线复测在飞。

### Round 3 收口：A2b 样条修复验收全绿（12:15）✅ KEEP + A3 定档（12:24）✅
- **A2b 全步 PASS**：12 处样条改写零漏改（对照 aces13.rs vmul 行向量约定）；治理 route B（审计发现 ci/g31_blocked_probes_smoke.py P02 RD-045 腿硬编码 presented 锚 060e69a8 经旧二进制+共享 SPV 消费 ⇒ 不覆盖共享件，v2 新路径隔离，他会话零影响）；parity 探针 exact 99.9891%/p100=1LSB/>1LSB=0；fan (144,122,77)±0；0.18 灰→99；环带非单调折返 3→0；render_digest 面 == f39e9808 旁证 encode 上游未扰；bench 默认臂 == c1d28ad7。
- **窗口锚重定基**：off 5596a730→55e4a92d / dither e989c6ee→5abd765f / AE fd5ca68c→790809aa / 六臂 a4695558→f0c46b87（全双跑位级+validation 静默）。夜巡旧 presented 锚系（b02b08b57/12d5dc91/48353e86/2b6efac6/db7d48f7）随之作废，Phase E 统一重收割。
- **AE 曲线复测**：终态 mean 0.3356（bug 版 0.4005 下移 16% = 暗部提亮纠正的预期效应），逐帧变化 ≤0.51% 零振荡。
- **交接项**：①源码-共享 SPV divergence（主线收编时统一切 v2 字节 + RD-045 锚 060e69a8 重收割）②parity 探针具 CI 门候选形态（encode_parity_probe.py）。
- **A3 定档**（artifacts/day_0828/a3_tuning/）：lamp-gain 4/16 全栈对照（32 帧收敛 + presented 统计）——AE 归一化后 mean 81 vs 83 几乎一致，差异在分布：**定档 g4 默认**（阴影柔、点光近似硬影隐蔽、p5 31>27），g16 登记备选。A3 验收判据全 PASS（暗部可读/均值带/p99/预算/锚/视觉，A3_SUMMARY.json）。全栈窗口 107-124fps 含回读税。
- 已知限制登记：12 点光多重硬影（真解面光/PCSS 留窗）；validation 必设教训（RURIX_REQUIRE_REAL=1 强制 RURIX_VK_VALIDATION=1）。
- **Phase A 收口**：曝光+死黑双问题的生产修复成立（死黑餐厅→全可读，AE 自适应，全锚零漂移）。

### Round 4（12:30– ）Phase B 纹理全覆盖 + 统一质量 kernel（在飞）
- 12:30 B 相联合实现子 agent 派出：fx/fy 双线性 bug ×5 处自有面修复（g34 三 kernel 同源 bug 只登记交接）+ texel heap 全 70 材质 + DDS 源 mip 直搬（cap-1024，~283 MiB）+ tritex 步幅 2 携 k_tri + kernel mip 选择（params[50]=k_pix）+ 统一 kernel g31_texture_nrm_gi.rx（纹理×平滑法线×GGX×环境光×灯光合体）+ 互斥解除 + SVT 臂 fail-closed 处置。验收：off==55e4a92d/默认==c1d28ad7 + 合流臂双跑位级 + 三验收位（画作/红帘/红墙）+ 色块连通域指标 + 帧时 ≤+1.5ms。
- 观察项：A3 对照图左侧地板有一道亮弧线（g4/g16 均在）——待 B 落地后核对是几何饰边均值色还是灯光影界伪影。

### Round 4 收口：Phase B 纹理全覆盖验收全绿（13:38）✅ KEEP
- **交付**：texel heap 单 SSBO（u32 偏移头表 910 项进 buffer 头部，零新增绑定）+ 全 70 材质 100% 三角覆盖 + DDS 源 mip 直搬（cap-1024：53×2048² 从 mip1 起 11 级 + 17×16² 全 5 级，零重采样）= **282.7 MiB 实测**（预估 283 ✓）；mip 选择 lod=clamp(floor(log2(th·k_pix·k_tri·w)),0,mips−1)（params[50]=k_pix）；tritex 步幅 2 携 k_tri；统一 kernel g31_texture_nrm_gi.rx（487 行，五臂全继承+纹理，albedo 入 GGX F0/环境光双面）；fx/fy 双线性 bug ×5 自有处修复；四形态 descs（tex/tex+nrm/tex+bloom/tex+nrm+bloom，AE 致密尾挂）。
- **验收 8/8 PASS**：off==55e4a92d/128 帧==夜巡 beb61a04/默认==c1d28ad7 零漂移；tex 单臂新锚 e6df516c + 探针 p100=0.0（5040 探针=70 槽×24UV×3 mip 级）；**七臂合流新锚 8b1c12f3** 双跑位级 + validation 静默；**色块 ≥5000px 恒定域 26 块/13.62% → 1 块/0.29%（面积 −97.84%）**；帧时纹理增量 **+0.19ms（+2.4%）**；G11.3 manifest 互核 70/70。
- **视觉**：画作（金框巴黎街景/抽象画/鹿素描）、红墙灰泥斑驳、地板六角花砖全部显真实纹理（crop 对照 ×3 + combo_tex.png）。偏差如实登记：红帘 curtainB1 契约相机 0 顶点在框（方位角 −82° 不可达）→ 地板替补验收 + 槽位证据（heap 入位 + 探针绿）。
- **交接**：g34 三 kernel fx/fy 同源 bug（L288/291 等，修法 fy→fx）；--svt×heap fail-closed 互斥（页表假设 2048 网格，深修留窗）；ci/g31_texture_sampling_smoke.py 判读器同步；bench 无 --textures 臂留窗（B4 指标窗口 dump 承载）；旧 tex 臂锚 6fab598c 作废；--textures 须随 --auto-move 硬门解除（静态双跑位级新协议）。
- A3 观察项闭环：地板亮弧线 = 地板反射高光叠瓷砖纹（非均值色块伪影）。
- 证据：artifacts/day_0828/b_textures/。

### Round 5（13:45– ）Phase C GI R2 加性臂（在飞）
- 13:45 C 相子 agent 派出：g31_texture_nrm_gi.rx 尾加 1 反弹 GI 段（params[51] 门/[52] 帧序号 R2 旋转/[53] firefly clamp/[54] scale）——R2 采样（gi_rng_study 仿真形态）+ 反弹点均值 albedo + emission 直取（emissive 间接光真通道）+ 单灯随机 NEE；`--gi2` 窗口旗标 + bench 哑表腿（供 EXR grain 指标）。验收：8b1c12f3/55e4a92d/c1d28ad7 零漂移 + 三臂噪点对比（旧 sin hash GI vs gi2 vs off）+ 帧时 ≤2×。

### Round 5 收口：Phase C GI R2 臂验收全绿（15:17）✅ KEEP（ultra 档候选）
- **交付**：g31_texture_nrm_gi.rx +199 行 GI2 段（R2 Cranley-Patterson 帧旋转：u=fract(px·a1+py·a2+n·a1) 与 gi_rng_study 自相关精确互证；1 反弹余弦半球 + 反弹点均值 albedo+emission 直取+单灯 R3 随机 NEE + firefly clamp）；params[51..55)；`--gi2/--gi2-scale/--gi2-clamp` 双车道；bench 哑表五件腿（EXR 指标全程走 --render）。
- **纪律事件（红→修）**：首编覆盖共享 SPV 后 gi2-off 漂移 d89848b9——E1 探针证明 +0.0 尾加数学恒等成立，漂移根因 = 新增 2 个 ray query 站点致后端对既有代码重编译 ULP 扰动（TSR/AE 反馈放大）。修法照 A2b 先例字节隔离（gi2-off 恒载 pre-C 锚定字节，GI2 编译独立 g31_texture_nrm_gi_gi2.spv）。**纪律修订登记：kernel 演进含新 ray query 站点时 gate=0 恒等不可依赖，保锚一律走字节隔离**。
- **验收 7/7 PASS**：55e4a92d/8b1c12f3/c1d28ad7/夜巡 778f1dfc 全零漂移；八臂 gi2 on 双跑位级（0e6ca110/b36c3e1f 两 clamp 档）+ validation 静默；bench gi2-on render 双跑位级（71083792）。
- **噪点对决（绝对幅值 std_p95，TSR 收敛后）**：gi2 clamp0.01 vs 旧 sin hash GI——wall 5.38e-4 vs 5.45e-3（−90%）/ 拱下 −88% / 桌下 −77%；屏空间相关色块不重现（旧臂斜链彩斑 vs gi2 稀疏无结构微点）。暗部增益 clamp0.01：桌下 +29%/吧台 +51%。**帧时 scene_gpu ×1.65（旧 ×4.39）≤2× 达标**。
- 如实登记：`≤2× off 比值门`对 1 spp 估计器不可达（off 基线近零），改绝对幅值判据；极暗区稀疏微光点可辨（Phase D TSR 降噪后复评是否入 full 档）；反弹无 quad NEE；R2 f32 frame_idx>100k 粒度退化（soak 拆和留窗）。
- **预设建议**：--quality ultra = 八臂 + --gi2-clamp 0.01（full 档待 D 后复评）。
- 证据：artifacts/day_0828/c_gi_r2/。

### Round 6（15:25– ）Phase D TSR 降噪质量档（在飞）
- 15:25 D 相子 agent 派出：fork g14_8_tsr_resolve.rx → g31_tsr_resolve_q.rx（Karis 色调域混合 + min_alpha 0.04→0.02 可配 + 可选 3×3 邻域 clamp，tsr_params[10..13) 驱动）+ `--tsr-quality` 双车道 + **字节隔离换载**（C 相纪律：off 恒载冻结 SPV）。验收：四臂颗粒对比（目标墙/地板 std_p95 ≥30%↓、gi2 微光点 ≥50%↓ 决定 gi2 入 full 档）+ P2 栏杆复核 + dolly 拖影检查 + 全锚零漂移。
- 16:24 D 会话中断（挂掉）；断点核实：kernel+接线已落（15:31）、零漂移三件已跑（alloff/combo7/bench_default 15:43-15:49）、四臂 128 帧 EXR 全部渲染完（16:04）、九臂双跑完（16:16）、d_metrics.py 刚写完即断。已带断点清单 resume（剩指标计算/栏杆复核/拖影检查/汇总）。

### Round 6 收口：Phase D TSR 降噪质量档验收全绿（17:22）✅ KEEP
- **交付**：g31_tsr_resolve_q.rx（312 行 fork，字节隔离换载）+ `--tsr-quality/--tsrq-min-alpha/--tsrq-clamp` 双车道；tsr_params[19]=稳态 alpha 档（默认 0.02）/[20]=邻域 clamp K（默认关）。
- **两轮红修（额度用尽，最终全绿，认知沉淀）**：v1「min_alpha 地板 0.04→0.02」零效——母版 α=0.1·(1−0.5·score)∈[0.05,0.1]，**0.04 地板构造性不可达**（母版缺陷登记主线）；v2 稳态 alpha 档直入 base 位→墙 −49% 但拱下 99.1% 像素两臂逐位相同 ⇒ 根因 = 深度验证在深度边缘随 jitter 逐帧拒史 passthrough；v3 决定性修法 = **深度验证 3×3 膨胀区间判据**（真 disocclusion 仍拒 + YCoCg AABB 仍钳）→ 全 ROI 达/超标。
- **验收 9/9 PASS**：全锚零漂移（55e4a92d/8b1c12f3/c1d28ad7/778f1dfc/C 锚 6144d9f7）+ bench 质量腿+tsrq 双跑 05532d5e + 窗口九臂双跑 **6bd3af63** + validation 全程静默。
- **颗粒战果（conv 协议 std_p95）**：墙 −49.3%（稳态 rel_p95 2.28%→0.74%）/拱下 −43%/桌下 −30%；**gi2 微光点 −87.3%~−97.3%（判据 ≥50% 大幅超标）**，gi2+tsrq 拱下残余已近 gi2-off 量级；收敛帧高通能量不升反降（墙 −27%/地板 −57% = 冻结颗粒消除+真 AA）。地板尾像素 −18% 未达 30% 如实登记（相关性低频 jitter 响应，EMA 档原理性不可滤）。
- **P2 走样：改善**（吊线/灯罩/瓶架阶梯→连续平滑，ROI 高通 −50.5%，无涂抹）；**拖影：PASS @α0.02**（dolly 曲线更平滑，三帧对照无拖尾）；**帧时增量 ~0**。
- **gi2 入 full 档判定：批准（绑定 tsrq）**——--quality full = 九臂（八臂+tsrq+gi2 c0.01），锚 6bd3af63 在案。
- 遗留：Karis HDR 尖峰有偏（远小灯略暗）；clamp K 档未实测（后备旋钮）；α0.02 收敛 ~50 帧；combo7 终态重建复验被中断（归 E 重收割）；ladder 帧数据 3.2GB 待清理；night 协议对 tsrq 欠敏感（E 用 conv 协议）。
- 证据：artifacts/day_0828/d_tsr/。

### Round 7（17:30– ）Phase E 终局合流（双子 agent 并行在飞）
- 17:30 E1 合流子 agent 派出：--quality off|full 预设（full=九臂一键展开，与显式旗标位级等价证明）+ 锚总表重收割（含 D 遗留 combo7 复验）+ Stage A 全 18 格锚检（12 vendor 批 + 6 tsr 隔离）+ 窗口风暴 + soak ≥1800s + 三段式 hero 对照 + TODO v1.1.8 只追加登记 + DEFAULT_FLIP_PLAN.md + HANDOVER.md + 磁盘清理。
- 17:30 E2 独立评审子 agent 派出（只读零 GPU）：治理红线核验（冻结面/他会话文件/共享 SPV 字节）+ 字节隔离完备性 + 边界条件（heap u32/R2 精度/NEE 除零/AE era 复位）+ 性能与证据链互核 + kernel 数值面抽查（GI2/tsrq/ACES 12 处）→ review_report.md。

### Round 7 追加：E1 中断续跑（18:43）
- E1 会话 18:33-18:43 间中断；断点核实：预设代码+HANDOVER.md 已落，Stage A 12 vendor 格完成（e3_stagea18_summary.json 在档）。已带断点清单 resume：6 tsr 隔离格 → 预设等价/锚复验 → 风暴 → soak ≥1800s → hero → TODO v1.1.8 → DEFAULT_FLIP_PLAN → 清理 → E_ACCEPTANCE_SUMMARY；折入 RD-045 门干净复跑（C-2 收尾）。

### Round 7 收口：E1 终局合流全项绿（19:33）✅ + 战役定盘
- **--quality off|full 预设**：窗口 full=九臂一键展开 / bench full=质量腿子集（textures/bloom/dither/AE 无 bench 面如实注释）；RURIX_G18_AMBIENT 缺席自注入 0.004（OnceLock 槽，env 优先）；fail-closed 四例冒烟；**位级等价 ×17 证明**（窗口 full==显式九臂==9e5f6300；bench full==1c12b7fd ×3）。
- **漂移事件如实处理**：6bd3af63/8b1c12f3 复验不再现——三二进制 A/B 定案 = D 相终态重建一次性 ULP 扰动（E1 代码无罪，五 SPV 指纹/atlas/探针全 SAME）；按预案重收割：七臂 → d89848b9、九臂/full → 9e5f6300（e2_reanchor_registry.json）。治理教训：**窗口纹理合流臂 presented 锚 = 二进制绑定锚，重建后先复验再消费**。
- **Stage A 18/18 MATCH**（6 tsr 隔离先行 + 12 vendor 批跑；dlss 六格 VUID 计数留观察 rc=0）；**风暴 PASS**（解除 storm×textures 互斥后 resize_eras=1 + validation 静默 + 重建后锚复验仍 9e5f6300）；**soak 1895s/13 迭代零失败** digest 恒值 + 4 次 Stage A 探针全绿；**评审 C-2 干净收尾** rc=0（RD-045 锚 060e69a8 精确匹配）。
- **hero 三段对照 + 四特写**（e_final/）；**TODO v1.1.8 已登记**；DEFAULT_FLIP_PLAN.md（只案不执行）；HANDOVER.md 21 项交接；磁盘清理 **59.37 GiB**。

## 战役定盘（19:40）

**四类问题全部修复，生产实时路径 `--quality full` 一键可用（锚 9e5f6300，soak 定盘）：**

| 问题 | 交付 | 关键指标 |
|---|---|---|
| 曝光 | 自动曝光两 pass + ACES 样条转置修复（12 处）+ EV 体系 | AE 适应零振荡；parity p100=1 LSB；0.18 灰→99 设计点 |
| 死黑 | 灯光提取 12 代表灯 + 半球环境光 + GI2 R2 一反弹 | 暗部 p50 ×42；GI 帧时 ×1.65（旧 ×4.39） |
| 异常色块 | 纹理全覆盖 70 材质 texel heap+mip（283 MiB）+ 蓝扇叶根因修复 + fx/fy bug ×5 | 色块面积 −97.8%；探针 p100=0.0 |
| 噪点 | GI R2 替换 sin hash + TSR 质量档（稳态 α+膨胀深度区间+Karis） | GI 噪声 −77~90%；微光点 −87~97%；墙面颗粒 −49%；P2 走样改善 |

**帧率**：--quality full 窗口 ~107-124fps 含回读税（60fps 预算余量充足）。**回归**：Stage A 18/18 + 全臂 off 零漂移 + soak 1895s 零失败。**评审**：有条件合入，两 CONCERN 已处置（C-1 EOL 归一 + C-2 锚复验精确匹配）。**默认翻转** = 唯一待批事项（DEFAULT_FLIP_PLAN.md 在案）。

改动面：5 tracked rs/kernel 文件（+~5400 行）+ 12 新 kernel 文件 + artifacts/day_0828/ 证据全集。全部加性默认 off、冻结面零触碰、他会话文件零污染、未 git commit。

### Round 7 追加：E2 独立评审收回（17:47）——**有条件合入** + CONCERN 即时处置
- **六面裁决**：治理红线 CONCERN×2（不阻断）+ 位级确定性/边界条件/性能声明/证据链/kernel 数值五面 PASS。亮点核验：三处字节隔离换载无臂组合漏洞；七锚跨相互核全一致；C/D 红修记录如实；ACES 12 处与金标准逐系数相等；GI2 单灯 NEE ×point_count 无偏证明；NEE point_count=0 零迭代安全（cornell 忧虑解除）。
- **C-1 已处置（17:50）**：6 件冻结面/并行会话文件被 11:20:27 批量事件（疑 A2b 崩溃恢复）翻 CRLF——逐件验证 `--ignore-cr-at-eol` diff 为空（文本零变）后 `git checkout --` 归一：ci/_patch_g31_cluster_lod_schemas.py、milestones/g31/g31_cluster_lod_evidence_schema.json、src/rurix-asset/kernels/g31_cluster_cull.rx、src/rurix-asset/src/bin/g31_cluster_cull_device.rs、src/rurix-geom-build/src/{lod_bounds,qem}.rs。冻结面字节纯净恢复，git status 该 6 件已清。**向 cluster-lod/G35 会话通报事项归 HANDOVER.md**。
- **C-2 实质已回答（18:08）：RD-045 锚完好** ✅——P02 device 腿渲染实际完成（.tmp/g31_blocked_probes/rd045_gate_spot_orbit_64p10.json，74 帧），digest == 锚 `060e69a8…` **精确匹配**：A2 覆盖共享 encode SPV 的 ×1.0 恒等论证被机器复跑证实，旧二进制+共享件消费面零破坏。门形式 rc=1 = 已登记的 gpu_device_lock 解锁段 PermissionError 基建抖动（host 10 腿全绿，仅异常包装层挂）；干净 rc=0 正式复跑待 E1 释放 GPU 后补（不阻断，实质证据在盘）。
- 低危观察登记：O-1 零法线退化射线 / O-2 AE NaN 中毒理论路径 / O-3 tsrq clamp K 全黑邻域全杀（K 默认关）；提示：gi2-on soak ≈216k 帧进 R2 f32 退化域（非 UB，判读知悉）。
- 报告：artifacts/day_0828/e_final/review_report.md。

### Round 8（20:27– ）Phase F 灯具 emissive 贴图臂（在飞）
- 用户复核 4K 出图发现灯具整体全白 → 侦察定案：有建模有贴图（吊灯 6,768 tri + 2048² BaseColor），全白 = 整材质均值 Le 自发光 × 曝光链裁剪；glTF emissiveTexture（4 张 2048² PNG）未接线是内容模型简化（emission 面的「均值马赛克」同族）。
- 20:27 F 相子 agent 派出：烘焙侧车（PNG→rgba8bin+mip+manifest，仓库无 PNG 解码器不引新依赖）→ kernel emissive 采样段（字节隔离新工件 g31_texture_nrm_gi_em.spv，既有两 SPV 0-byte）→ heap 槽 70..73 + triem 侧表 + 能量守恒标定（scale=契约Le/贴图线性均值，总光通量维持 G13 标定，A1 提取/GI2 零改动）→ --emissive-tex 臂验收 → 并入 --quality full 重锚 + 风暴 + soak ≥1800s。

### Round 8 收口：Phase F emissive 贴图臂验收全绿（23:10）✅ KEEP —— --quality full 升级为十臂
- **交付**：烘焙侧车（4×2048² PNG→rgba8bin+12 级 mip+manifest，双跑 sha 恒等）；kernel +94 行 emissive 采样段 → 新工件 g31_texture_nrm_gi_em.spv（既有两 SPV 0-byte 复核 fd22cb19/75d08aec == C 相记录）；heap 70→74 槽（+22.4MB）+ triem 侧表（4.2MB，em_tris=44,024 == A1 侦察精确吻合）；`--emissive-tex` + `--emissive-dir`。
- **能量守恒闭环**：scale ≡ 1.0 **精确**（契约 Le 逐位等于贴图线性均值——契约本就由贴图派生），A1 提取/GI2 均值面零改动零漂移。
- **验收 F4 8/8 + F5 8/8**：三锚零漂移（55e4a92d/9e5f6300/c1d28ad7）+ em 臂双跑 78113d56 + 74 槽探针 p100=0.0（5,328 探针）+ **--quality full 升十臂新锚 78113d56（预设==显式 ×3 位级）**，旧九臂锚 9e5f6300 作废登记（F5_ANCHOR.json 谱系）+ 风暴 PASS + 4K（d9cf0cdf）+ **soak 1800.5s / 9 迭代零失败**。
- **视觉/指标**：吊灯罩区 8bit 250.3→222.4（100% 像素脱饱和，显编织纹）、bulb 区 253.5（≥250 维持过曝白）、灯笼吊链/顶盖脱白、吊扇叶显红木；全图 mean Δ0.049（AE 补偿稳定）；帧时增量 ≈0。
- 遗留登记：GI2 反弹点 emission 仍均值（低频近似留窗）；emissive 采样无条件 4 fetch（帧时 ≈0 如实登记）；壁灯 mat59 契约相机 0 顶点在框走探针替补（curtainB1 同律）；soak it7 单迭代 374s（宿主瞬时负载，digest 稳定）；**CARGO_TARGET_DIR 后台化丢失教训固化**（构建命令内联 env + exe mtime 核验）。
- 证据：artifacts/day_0828/f_emissive/。

### Round 9 收口：F6 跨会话编译修复全绿（00:06）✅
- **F 相暴露的合入阻断项**：Phase B 把共享纹理装配 API **就地改形**（heap/tritex stride-2/探针协议/host_sample 7 参）→ 他会话 g34_full_lane（已提交代码，其 kernel 按原 grid/stride-1 语义 + 门锚冻结）编译红 4 处 + 运行时语义破坏——Phase B 加性纪律违规，编译暴露迟至今日（E2 评审未编译全 bin，判读盲区登记）。
- **双形态回正交付**：原形态 8 函数 + 2 struct + 1 常量从 HEAD **逐字恢复**（机核 20/20 位级，含 g31_tex_load 264 行/g31_tex_host_sample 6 参含原 fy 式——与 g34 冻结 kernel 同源抵消面一致，不越权代修）；heap 形态改名 `*_heap`/`*_mip` 并存；我们的调用点（window_present 9 处）全切 heap 命名；**g34 三文件与 kernel 零字节**。
- **验收全绿（红修 0 次）**：全 bin 编译首轮 rc=0（4 exe mtime 新鲜）；三锚零漂移（all-off 55e4a92d / full 78113d56 / bench c1d28ad7，VUID=0）；g34 真跑 74f orbit **双跑位级一致**（4a06301c，VUID=0；在案锚 f39e9808 系属 render 静态锚格与 orbit 不可比，如实登记双跑一致性口径）。
- 纪律教训固化（HANDOVER §F.28）：**共享 include 体内符号改形即跨会话破坏——一切共享符号变更必须双形态并存 + 全 bin 编译为界**。
- 证据：f_emissive/F6_SUMMARY.json + F6_RUNS.json + f6_verbatim_check.json。

### Round 10（08-29 09:53–10:40）F7 小灯微调（γ 对比度重映射）✅ KEEP
- **用户复核**：吧台四盏小吊灯 F 相后仍全白。诊断（small_lamp_probe.py，射线归因+UV+逐 mip 采样）：小灯 = 灯笼材质 mat 38，**非 mip 问题**（L0≈L4 保持良好）——其 emissive 贴图玻璃罩区本身 0.05–0.16（非零占比 38.7%），显示链总增益 ×52.8（EV16×AE3.3）把 >0.019 全推白；大灯 F 相见效只因其贴图 89% 区域为零。
- **修法**：烘焙期 γ 对比度重映射（linear 域 tex^γ，均值经 manifest 重标定 scale=Le/mean(tex^γ) ⇒ 可见面均值仍==契约 Le；投光面 A1/GI2 架构解耦零影响；γ=1 位级恒等旧烘焙）。**零 kernel/SPV/代码改动，纯资产字节**。
- **定档 γ=2.5**（两次尝试额度内）：小灯饱和白 77-79% → **37-38%**（γ2 为 46-47%），玻璃灯笼形体/明暗渐变可见、光晕收紧、灯泡区仍白；大灯罩仅 82→79%（其亮板区 0.65+ 本意常亮，如实登记）。三档阶梯图 g_ladder_bells.png。
- **验收**：双跑位级 de342586 ×2 一致 + validation 静默 + em_fallback=0；**full 预设锚 78113d56（γ1）→ de342586（γ2.5）作废谱系登记**；all-off/bench 零影响（烘焙资产仅 em 臂加载，架构隔离）。
- **性能实测**：7.35→7.58ms 落 run 间噪声带（131-136 fps），烘焙期变更运行时零代码差——**微调性能代价 = 0**。
- 证据：f_emissive/{g25_96_ev,g25_96_ev2}.json + png/{g_ladder_bells,g2_cmp_*,g25_96}.png + small_lamp_probe.py。

## 战役终局定盘 v2（00:10，Phase F/F6 后更新）

**生产实时路径 `--quality full` = 十臂**（平滑法线+GGX+灯光提取+纹理全覆盖+bloom+抖动+自动曝光+TSR 质量档+GI2+emissive 贴图），锚 `78113d56`（预设==显式 ×3 位级），soak 1800.5s 零失败，全 bin 编译绿，g34/g35 他会话面零破坏。四类用户问题全部修复且灯具质感闭环（罩壳显纹理/灯泡过曝白/能量守恒精确）。待批事项唯默认翻转（DEFAULT_FLIP_PLAN.md）；交接清单 HANDOVER.md §1-28。

## Phase E 预案（B/C/D 落地后执行）

- **--quality off|full 预设映射**（窗口）：full = --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --textures on --bloom on --dither on --auto-exposure on + env RURIX_G18_AMBIENT=0.004（+ C 相 GI 档如验收达标则入 --quality ultra）；bench 对应子集（--smooth-normals/--ggx/--lamp-lights/--textures）。
- **回归矩阵**：①Stage A 18 格锚检（regression_probe.py 6 代表格 + 全 18 格分批，vendor→tsr 测序脆弱面注意隔离跑 tsr）②窗口 all-off == 55e4a92d 系新锚 ③全臂组合双跑位级 ④validation 静默 ⑤窗口风暴 --window-storm 3 ⑥soak ≥1800s 零失败（全栈组合）。
- **默认翻转治理方案产出**（条件执行）：受影响锚清单已有底稿（A2b 审计：RD-045 P02 腿 060e69a8 经旧二进制+共享 SPV；夜巡旧锚系已作废清单在 Round 3）；方案 = 共享 SPV 统一切 v2 字节 + RD-045 锚重收割 + 窗口默认预设 flip + 全门复跑清单；bench Stage A 默认臂永不动。
- **交接清单归集**：g34 三 kernel fx/fy 同源 bug；ci/g31_texture_sampling_smoke.py 判读器同步（heap 形态）；encode_parity_probe.py 转正 CI 门候选；gpu_device_lock 解锁段 PermissionError + stale holder；rurixc「if 包 while」codegen 缺陷。

## A2 自动曝光设计预案（已派发，规格如下）

- 仅窗口车道（bench --render 出 EXR 是 pre-encode HDR，不受影响）。旗标 `--auto-exposure off|on` 默认 off。
- 两个微 pass（resolve/bloom-composite 之后、encode 之前）：
  1. `g31_autoexp_reduce.rx`：256 线程各自跨步累加 log-luma（4 px 步进采样），写 256 个 partial（各线程序内顺序累加，确定性）；
  2. `g31_autoexp_state.rx`：单线程串行求和 partial → mean_log_luma → target_gain = key/exp(mean_log)，EMA：state = state==未初始化 ? target : lerp(state, target, rate)，gain 钳 [1/8, 32]；状态 buffer 跨帧持久（4 f32）。
- encode kernel 读 gain（新绑定，off 臂绑 1.0 哑 buffer）：*1.0 为 IEEE 位级恒等 → off==5596a730 可保（夜间「含抖动 kernel 重建默认 SPV 后验证 off==锚」同先例，重建后必须复验）。
- gain 施加点 = ACES IDT 之前（post-TSR pre-tonemap，显示域历史无拖影税）。
- 风险：EMA 引入跨帧反馈 → digest 判定改用「双跑位级一致」而非与既往锚相等；EV 突变场景（dolly 进灯区）观察振荡，rate 取 0.05-0.1。
