// Assisted-by: Kimi-K3（G31+ 波 A Task A1/A3/A5）
//! G31+ 波 A 生产管线 swapchain 真窗口呈现 + 游戏循环最小面（A1 门 `g31.waveA.present`;
//! A3 门 `g31.waveA.gameloop`——Task A3 = device 侧显示编码 + 输入→相机逐帧 uniform +
//! 窗口事件健壮性 + `--auto-move` 确定性门）+ FG/MFG 帧生成生产接线（A5 门
//! `g31.waveA.framegen`——Task A5 = `--fg <off|x2|x3>`:G26 device kernel
//! `kernels/g26_framegen.rx` 链接入呈现车道,真帧/生成帧序列 present + 双口径分离
//! 登记 + 接线态对拍门维持;G30 承接锚 G13-N7 行兑现）。
//!
//! ## A5 FG 接线面（G26 kernel 本体与 host 金标准面 0-byte）
//!
//! 1. **车道内链接**：fg on 时车道由五 pass 扩为 8/10 pass——生产五 pass（0-byte
//!    语义,真实渲染帧 digest 与 fg off 位级一致为机核门）后追加 `g31_mv_negate`
//!    （MV 取反 glue)+ `g26_framegen`（读 prev/cur TSR 输出 parity 双缓冲 +
//!    取反后 MV）+ 复用 `g31_display_encode` 将生成帧编码 BGRA8;present 序 =
//!    生成帧（t 升序）→ 真帧。
//! 2. **MV 约定换算（取反 glue 直通,禁改 G26/G14 kernel）**：g14_mv 输出
//!    `m(x) = prev_uv(x) − x`(temporal::common::compute_camera_mv 同式);host
//!    金标准 `framegen::interpolate` 约定 `prev_uv = cur_uv − mv` 即需 `-m`。
//!    新增 glue kernel `kernels/g31_mv_negate.rx` 对 MV 场逐元素 IEEE 取反
//!    （零数值误差）,g26_framegen 以 `(prev, cur, −m, t)` **直通馈入——与 host
//!    逐字同语义**（含 t=0.5 near 选臂 tie-break;prev/cur 对调 swap 馈入的
//!    tie 翻转缺陷由直通形消除,t 任意值无 1ULP wobble）。
//!    MV 仅含相机运动 + 静态场景深度重投影——**运动物体 MV 缺口为 A4 已登记项,
//!    如实登记不冒充**（bistro 静态场景面;dyn 实例场景 FG 不接）。
//! 3. **栅格约束**：kernel 要求 prev/cur/mv 同栅格;MV 产出于 internal 分辨率,
//!    TSR 输出在输出分辨率——`--fg` 闭集限 `--tier 100`（两栅格相等同律）+ 须随
//!    `--auto-move`（FG 登记面 = 确定性轨迹）。
//! 4. **双口径分离（G13-N7 字面纪律）**：`real_render_frame_ms/real_render_fps`
//!    只由真渲帧构成（生成帧禁入计数;单提交墙钟含 FG GPU 段——telemetry 分列
//!    `stats.render5_gpu_ms`/`stats.fg_gpu_ms` 如实登记）;`presented_fps` =
//!    presented 帧 ÷（渲染 + present 墙钟）独立新口径,与真实渲染帧率并列输出
//!    永不混算;`caliber_identities` 恒等式组（presented = real + generated、
//!    real fps 重算、real fps 对 generated 扰动隔离、presented fps 重算）schema
//!    层钉 const true。
//! 5. **接线态对拍门**：probe 帧（post-warmup 首生成帧）回读 prev/cur f32 + MV +
//!    生成帧 f32,host 金标准 `interpolate(prev, cur, −mv, t)` 复算对拍——
//!    p100 ≤ G26 冻结容差（`milestones/g26/g26_budget.json`
//!    `g26.framegen_device.host_device_maxdiff_tol` threshold 程序读,禁手写）
//!    + SSIM(device, hostref) > SSIM(frame-hold, hostref),结果进 evidence
//!    `wired_parity`;G26 合成 GT 对拍门（p100 + SSIM + 双跑位级）由
//!    `ci/g31_framegen_present_smoke.py` 接线态复跑维持。
//!
//! ## 职责闭集
//!
//! 1. **生产管线真跑**：以 bistro-interior 契约跑 G14.3 统一四 pass TSR 车道（Mega：
//!    scene `g14_3_direct_gi` → mv `g14_mv` → TSR `g14_8_tsr_{resample,resolve}`，
//!    `DeviceFrameSession` AS 常驻 + 逐帧 192B/160B/128B 参数上传 + 逐帧 fence 全
//!    同步）——实现与 `g14_3_pipeline_perf` 逐字共享
//!    （`include!("g14_3_lane/g14_3_lane_body.rs")`，digest 锚定逻辑 0-byte 不动;
//!    共享体 0-byte——A3 的第五 pass/车道面全部落本文件,bench/契约锚面零触碰）。
//! 2. **A3 device 侧显示编码**（A1 host 编码瓶颈消除位）：TSR 输出**驻留 device**
//!    （零逐帧 f32 回读），第五 pass `g31_display_encode`（kernels/
//!    g31_display_encode.rx——ACES 1.3 RRT+ODT f32 移植 + BT.1886 γ2.4 编码 +
//!    8-bit 量化 + BGRA/RGBA 打包,矩阵/样条系数由 host
//!    `aces13::aces13_device_encode_params` 同一 f64 参考实现现算上传）链内直写
//!    BGRA8 SSBO;host 仅回读 8.3MB BGRA8（A1 = 24.9MB f32 + 逐像素 f64 编码）
//!    供 present 拷贝/digest。
//! 3. **契约链**：生产契约（digest 门 == FROZEN）+ G10 语料三件套转引一致性核验
//!    （逐字段相等,不等即 RED）;sun/sky/ev100 delta 如实登记不消费。
//! 4. **游戏循环最小面**：win32 输入（WASD/QE 平移 + mouse/方向键视角 + `-`/`=`
//!    曝光 ±0.25 ev）逐帧更新相机/曝光 → 经既有 192B 帧参数 + 128B TSR 参数
//!    uniform 通路进生产车道;WM_SIZE resize → swapchain 重建 + 渲染 extent 联动
//!    （车道按新 extent 重建,TSR 历史 reset）;最小化跳过渲染/present 不消费帧
//!    预算;ESC/WM_CLOSE 干净退出（资源逆序拆除,validation 静默）。
//! 5. **`--auto-move <orbit|dolly>`**（非交互 CI 面）：确定性脚本轨迹逐帧驱动
//!    相机（f64 参数化,帧号唯一事实源;`--ev100-ramp a b` 可加确定性曝光坡）,
//!    逐帧 BGRA8 digest 序列进 evidence（schema
//!    `milestones/g31/g31_game_loop_evidence_schema.json`）——同轨迹双跑位级
//!    一致（确定性门）,异轨迹 digest 序列必须不同（防"确定性的坏内容"）。
//! 6. **present 口径独立登记**：`real_render_frame_ms`（生产管线五 pass 渲染耗时,
//!    **不含 present**;含 present 强制的 BGRA8 回读段——如实登记
//!    `render_includes_forced_readback=true`）/ `present_frame_ms` /
//!    `present_overhead_ms`（= encode(host≈0,device 编码 GPU 耗时分列
//!    `stats.encode_gpu_ms`) + present 腿）/ `digest_frame_ms`（auto-move 逐帧
//!    sha256 税,单列不混渲染口径）多口径分离,**真实渲染帧率口径禁混 present
//!    开销**。
//!
//! ## 用法
//!
//! ```text
//! g31_window_present [--frames 120] [--warmup 10] [--tier 100]
//!     [--contract <c.json>] [--g10-dir milestones/g10/corpus] [--gltf <scene.gltf>]
//!     [--spv-scene <a.spv>] [--spv-mv <b.spv>] [--spv-resample <c.spv>] [--spv-resolve <d.spv>]
//!     [--spv-encode <e.spv>] [--evidence <path>] [--expect-digest <sha256:…>]
//!     [--hidden] [--headless-smoke] [--auto-move <orbit|dolly> [--ev100-ramp <a> <b>]]
//!     [--fg <off|x2|x3> [--spv-framegen <f.spv>] [--fg-tol <F>]]（可与 --quality full 组合,两点式闭集——G37 W3 fg_combo）
//!     [--slab-table <asset.json> [--slab-arm <device|host>] [--spv-slab <s.spv>]
//!      [--dump-last-frame <raw.bin>]]
//!     [--hzb <off|on> [--spv-hzb-primary <a>] [--spv-hzb-shade <b>]
//!      [--spv-hzb-pack <c>] [--spv-hzb-reduce <r.spv>] [--spv-hzb-test <t.spv>]]
//!     [--dither <off|on>]
//!     [--smooth-normals <off|on>]
//!     [--bloom <off|on> [--bloom-strength <s>] [--bloom-threshold <t>]
//!      [--spv-bloom-bright <a>] [--spv-bloom-blur <b>] [--spv-bloom-composite <c>]]
//!     [--auto-exposure <off|on> [--autoexp-key <k>] [--autoexp-rate <r>]
//!      [--autoexp-min <lo>] [--autoexp-max <hi>]
//!      [--spv-autoexp-reduce <a>] [--spv-autoexp-state <b>]]
//!     [--present-luma-out <json>] [--dump-present-every <n>]
//!     [--fault-probe <device-lost-acquire|device-lost-submit|device-lost-present|tdr|budget>]
//!     [--window-storm <n>] [--storm-soak <period>]
//!     [--quality <off|full>]（G37 W4 默认翻转:缺省 = full 十九臂画质终态;
//!      off = 显式回退档——诊断/互斥/单臂显式写法须显式给 off）
//! ```
//!
//! C4（G31+ 波 C Task C4 运行时健壮性 + 故障注入,门 `g31.waveC.robustness`）:
//! `--fault-probe` = 注入观察臂（机制面 env 双层门控:`RURIX_G31_FAULT_DEVICE_LOST=
//! <point>@<idx>` present 会话三点 DEVICE_LOST 覆写 → poisoned 锁存 + 级联确定性;
//! `RURIX_G31_FAULT_FENCE_TIMEOUT=<n>` 持久帧 fence 有界等待第 n 次覆写 VK_TIMEOUT
//! → TDR-suspected 确定性 Err 不挂死;`RURIX_G31_FAULT_BUDGET_BYTES=<n>` heap budget
//! 钳制 → OOM-suspected 确定性 Err fail-closed）,命中打印 G31_FAULT_PROBE 单行退 0,
//! 全程未触发 fail-closed 判红;`--window-storm <n>` = 爆发 resize 臂（n 次程序化
//! 半↔原 extent 真 swapchain/staging 重建）;`--storm-soak <period>` = 周期故障臂
//! （每 period 帧 resize toggle,每 period×8 帧最小化/恢复 WM_SIZE 同通路注入）。
//! 全臂与 --fg/--hzb/--slab-table/--svt 互斥（登记面 = 生产五 pass 现状车道;
//! day_0828 Phase E1 解除与 --textures 互斥——era 重建走完整变体描述组重建,
//! --quality full × 风暴验收在案 e_final/e4_storm_summary.json）,
//! 默认关零行为变更。
//!
//! `--hzb on`（G31+ 波 B Task B1 HZB 遮挡剔除生产接线,门 `g31.waveB.hzb`）:
//! bistro 逐 mesh 节点 BLAS 分解 + 双 TLAS（初剔/全量阴影）+ g27_hzb_reduce/
//! g27_hzb_test 两 kernel 0-byte 进剔除链（帧内金字塔轮换:上帧金字塔初剔 →
//! 本帧重建重测）+ 误剔/出新闭环重渲（剔除零假阳性 ⇒ 画面与 hzb off 位级一致,
//! digest_seq on/off 逐帧对拍为门）;evidence schema
//! `rurix.g31.hzb_wiring_evidence.v1`。闭集约束:与 --fg/--slab-table 互斥,
//! 须 --tier 100;--spv-hzb-* 须随 --hzb on。
//!
//! `--slab-table`（G31+ 波 B Task B3 slab 材质侧表生产接线,门 `g31.waveB.slab`）:
//! 资产文件驱动的 16 槽 slab 侧表（G29 M-b ABI 升级面）加载 → kernels/g29_slab.rx
//! （0-byte 冻结）device 逐槽求值 vs material/slab.rs host 金标准对拍（parity_p100
//! 登记,有限性一等断言先于聚合）→ 映射材质逐三角 albedo × R_slot 预调制进既有
//! mats SSBO 面（生产 kernel/管线 0-byte;非映射材质走既有单层面 0-byte）;evidence
//! schema `rurix.g31.slab_wiring_evidence.v1`。闭集约束:须随 --auto-move（确定性
//! 轨迹登记面）,与 --fg 互斥;--slab-arm/--spv-slab/--dump-last-frame 须随
//! --slab-table;--slab-arm host = host 参考臂渲染（跨臂像素对拍由 smoke 裁决）。
//!
//! `--fg`（A5）闭集约束：须随 `--auto-move` + `--tier 100` + frames+warmup ≥ 2;
//! `--fg-tol` 缺省时程序读 milestones/g26/g26_budget.json 冻结标定条目（fail-closed）。
//! G37 W3 fg_combo 合入：fg 合法形态 = {全画质 off base} ∪ {--quality full 预设
//! 字面} 两点式闭集——full 面 FG 插值 post-bloom 合成帧（comp parity 双缓冲:
//! composite 写 comp[p]/encode 读 comp[p]/AE reduce 读 comp[p]/FG 读
//! (comp[1−p], comp[p]),FULL 下标族 48..=56 按 TEXNRM_BLOOM_RIS+AE 终态定死,
//! 真实帧数值逐位不变 ⇒ digest_seq 不污染门跨 fg on/off 维持）,AE 增益经
//! enc_params[133] 生成帧同读继承;散臂微调混搭维持 fail-closed（下标族爆炸 =
//! 红修 #2 事故几何）;fg×{hzb,slab,svt,lut,storm/fault} 互斥维持。
//!
//! `--bloom`（夜间巡航 D3 HDR bloom 加性臂）闭集约束：默认 off = 既有五 pass
//! 车道/digest 锚 0-byte;on = resolve 后插 bright（软膝阈值+2×降采样）→
//! blur H→blur V（半分辨率 9-tap 可分离高斯）→ composite（双线性上采样×
//! strength 加性合成回全分辨率 HDR）四 pass,display_encode 的 in_color 改读
//! 合成缓冲;与 --fg/--hzb/--svt/--slab-table/--cluster-lod/
//! --wp-hlod fail-closed 互斥（组合面未接线;day_0828 Phase B 解除与
//! --textures 互斥——组合面 = g31_lane_descs_tex_bloom/_tex_nrm_bloom）;
//! --bloom-strength/--bloom-threshold/--spv-bloom-* 须随 --bloom on。
//!
//! `--smooth-normals`（夜间巡航 D2 平滑顶点法线加性臂）闭集约束：默认 off =
//! 既有五 pass 车道/digest 锚 0-byte;on = scene pass 换 kernels/
//! g18_smooth_nrm.rx（g18 逐字 fork + params[43] 门 committed 重心插值顶点
//! 法线;半球环境光 params[44..48) 经 RURIX_G18_AMBIENT env 门控,缺省关臂
//! 位级）+ trinrm 9 f32/tri 侧表 SSBO（挂既有面尾部——单臂面下标 24,与
//! --bloom on 组合面下标 32,encode 22/23 与 bloom 24..=31 下标 0-byte
//! 不动）+ D6 tri_mr 侧表（单臂面 25/组合面 33;--ggx off = 8B 零哑表,
//! kernel params[48]=0 门不读）;与 --fg/--hzb/--svt/
//! --slab-table/--cluster-lod/--wp-hlod fail-closed 互斥（组合面未接线;
//! day_0828 Phase B 解除与 --textures 互斥——合流臂换载
//! kernels/g31_texture_nrm_gi.rx 合体 kernel,trinrm/tri_mr 让位 29/30
//! 〔×bloom 组合 37/38〕）;与 --bloom/--dither 可组合（scene 上游/post
//! 下游正交）。
//!
//! `--ggx`（夜间巡航 D6 GGX 高光材质加性臂）闭集约束：默认 off = 既有面
//! 0-byte/digest 锚零漂移（哑表 + params[48]=0）;on = tri_mr 2 f32/tri
//! 真表（glTF pbrMetallicRoughness metallicFactor/roughnessFactor 逐三角,
//! assemble_scene_nrm_mr 与 trinrm 同窗装配）替换哑表绑定 + 逐帧参数
//! params[48]=1.0（pack_frame_params_ggx,与 bench 车道同口径）→ kernel
//! GGX 高光臂（D=Trowbridge-Reitz/G=Smith Schlick-GGX/F=Schlick,F0=
//! mix(0.04,albedo,metallic)）。须随 --smooth-normals on（fail-closed）;
//! 互斥集与 --smooth-normals 同;与 --bloom/--dither 可组合。
//!
//! 画质战役 A1（`--lamp-lights off|on`,默认 off）：灯光提取加性臂——
//! bistro 44k 自发光灯片三角不投光（死黑+欠曝根因）,on = host 确定性聚类
//! （0.6m 网格 + 26 邻域 union-find）→ ≤K 代表点光 append 进 points 面
//! （I_c = Φ_c·gain/(4π),radius = 簇最大顶点距+0.02m 进 pack 槽 7——
//! g18_smooth_nrm kernel 阴影 t_sh 提前截断消灯罩自遮蔽）+ params[49]
//! 贡献剔除阈值（--lamp-contrib,默认 0 全保留）。off = 既有面 0-byte/
//! digest 锚零漂移;须随 --smooth-normals on（fail-closed）,互斥集同。
//! 可调面 --lamp-gain（默认 1.0）/--lamp-k（默认 12）。
//!
//! 画质战役 A2（`--auto-exposure off|on`,默认 off）：自动曝光加性臂——
//! presented 亮度自适应（场景线性均值 ~0.01-0.03 自动增益到目标带,相机
//! 进亮/暗区平滑适应）。on = encode 前插两微 pass:g31_autoexp_reduce
//! （256 线程单 workgroup 跨步 log-luma 归约）→ g31_autoexp_state（单线程
//! 求和→几何均值→target=key/avg 钳 [min,max]→EMA 跨帧状态→增益写 encode
//! 参数 reserved 槽 [133]）;增益施加 = encode ACES IDT 前（post-TSR
//! pre-tonemap,TSR 显示域历史零 EV 拖影税;bench --render pre-encode EXR
//! 不受影响）。**encode kernel 绑定面零新增**（g34/g35 等 3 绑定既有消费面
//! 共享默认 SPV——增益走 params[133] + kernel ≤0→1.0 守卫,off 臂 host 打包
//! 恒 0 位级零漂移）。EMA 跨帧反馈 ⇒ on 臂验收口径 = 双跑位级一致;resize
//! era 重建 = 状态归零再适应。off = 既有面 0-byte/digest 锚零漂移;互斥集
//! 与 --bloom 同（fg/hzb/textures/svt/slab/cluster/wp fail-closed）;与
//! --dither/--smooth-normals/--ggx/--lamp-lights/--bloom 全可组合。可调面
//! --autoexp-key（默认 0.115）/--autoexp-rate（0.08）/--autoexp-min（0.125）
//! /--autoexp-max（32.0）。验证面（默认关 = 0-byte）：--present-luma-out =
//! 逐帧 presented 亮度序列 sidecar JSON;--dump-present-every <n> = 每 n 帧
//! presented raw dump（基路径 = --dump-present-raw 派生 `.f<帧号>`）。
//!
//! 画质战役 Phase E1（`--quality off|full`;E1 期默认 off,**G37 W4 默认翻转
//! 后缺省 = full**——本段"默认 off"字面为历史镜像不回写,以 W4 翻转注释为准）：
//! 画质预设一键展开——
//! 解析层将 full 展开为九臂战役终态组合（--smooth-normals on --ggx on
//! --lamp-lights on --lamp-gain 4 --textures on --bloom on --dither on
//! --auto-exposure on --tsr-quality on --gi2 on --gi2-clamp 0.01）,展开
//! 先于全部臂校验/SPV 换载 ⇒ 下游与显式九臂写法走完全相同路径 = 位级等价
//! （锚 6bd3af63 双跑在案）。RURIX_G18_AMBIENT env 缺席时预设注入 0.004
//! （战役终态档,进程内 OnceLock 槽——forbid(unsafe_code) 下不可 set_var;
//! 显式 env 一律优先,含非法字面的既有静默关臂语义）。展开面 11 旗标字面与
//! --quality full 同给 = fail-closed 报错（双重指定即语义歧义,微调请弃
//! 预设走全显式写法）;非展开面子参数（--lamp-k/--autoexp-* 等）可随预设
//! 组合（等价于显式九臂 + 该子参数）。off = 中性字面零展开零行为。
//!
//! `--headless-smoke` = 无窗口退化路径（仅供自检逻辑用,**不计真门**;evidence
//! `headless=true`,present 口径 null）。三态：无 Vulkan/设备/场景资产/窗口创建失败
//! → `skipped_dev_env`（退 0 非 fake pass;`RURIX_REQUIRE_REAL=1` 翻 FAIL 退 1）。
#![forbid(unsafe_code)]
// 共享体含本 bin 未消费面（render/bench 腿、dlss/fsr 双臂、EXR/PNG 出图、G16+ GI 臂等）
// ——dead_code 豁免如实登记;本 bin 消费面 = 契约解析/scene 装配/统一四 pass TSR 车道/
// 帧参数/jitter/digest/JSON 解析。
#![allow(dead_code)]

include!("g14_3_lane/g14_3_lane_body.rs");
// G37 W2 合入:--lut 色彩分级 host 资产模块(M119 五级链第 4 级;TODO #79)。
include!("g37_w2/g31_lut_assets.rs");
// G37 W2 ris_nee 合入:臂⑧ GI2 反弹 RIS 选灯/灯片 CDF 面光 NEE 的灯片表 +
// 功率 CDF 装配模块(--gi2-ris|--gi2-nee on 面消费;单源 = g37_w2/g31_ris_lamps.rs)。
include!("g37_w2/g31_ris_lamps.rs");
// G37 W2 合入:PSO precache/warmup 变体账本胶水(#82/#113;era0 = precache 面,
// era≥1 = 守护面,验收 pso_runtime_creates == 0)。
include!("g37_w2/g31_pso_warmup.rs");
// G37 W2 visbuffer:#74/#111 窗口生产证据臂共享体（加性 include;lane body 0-byte）。
include!("g14_3_lane/g31_visbuffer_arm.rs");
// G37 W3 frame_cut 合入:#77×#89 合流窗判档臂共享体（加性 include;lane body 0-byte）。
include!("g14_3_lane/g31_frame_cut_arm.rs");

use rurix_render::display::aces13::aces13_device_encode_params_ex;
use rurix_render::geometry::cull::Frustum;
use rurix_render::geometry::hzb::{DepthConvention, HzbPyramid, Occlusion, exact_rect_occluded};
use rurix_render::temporal::framegen::{FrameGenParams, interpolate};
use rurix_render::temporal::ssim::ssim;

const GTAG: &str = "[g31_window_present]";
/// A1 门键（默认面 evidence `gate` 字段字面）。
const G31_GATE: &str = "g31.waveA.present";
/// A3 游戏循环门键（`--auto-move` 面 evidence `gate` 字段字面）。
const G31_GAMELOOP_GATE: &str = "g31.waveA.gameloop";
/// A5 FG 接线门键（`--fg x2|x3` 面 evidence `gate` 字段字面）。
const G31_FRAMEGEN_GATE: &str = "g31.waveA.framegen";
/// G10 语料目录默认（contract_params_bistro_interior.json + camera/lighting 三件套）。
const G31_DEFAULT_G10_DIR: &str = "milestones/g10/corpus";
/// A1 evidence schema 字面（milestones/g31/g31_window_present_evidence_schema.json 同字面）。
const G31_SCHEMA: &str = "rurix.g31.window_present_evidence.v1";
/// A3 游戏循环 evidence schema 字面（milestones/g31/g31_game_loop_evidence_schema.json 同字面）。
const G31_GAMELOOP_SCHEMA: &str = "rurix.g31.game_loop_evidence.v1";
/// A5 FG 接线 evidence schema 字面（milestones/g31/g31_framegen_present_evidence_schema.json 同字面）。
const G31_FRAMEGEN_SCHEMA: &str = "rurix.g31.framegen_present_evidence.v1";
/// A3 device 编码 kernel 默认 SPV（源 = kernels/g31_display_encode.rx;`.tmp` 构建产物）。
/// A2b ACES 样条转置修复后指向新路径 v2：旧共享路径 `.tmp/g14_gates/m_c/
/// g31_display_encode.spv` 被 g34_full_lane/g35_particle_lane 运行时共享消费且被
/// ci/g31_blocked_probes_smoke.py P02 RD-045 固定 presented 锚（060e69a8…）经
/// target/release 旧二进制消费——修复改 presented 字节,覆盖共享件即破他会话/已收口
/// 锚,故共享旧 SPV 0-byte 不动;源码（kernels/*.rx）单一事实源已修,共享路径字节与
/// 源码 divergence 如实登记为交接项（见 artifacts/day_0828/a2b_aces_fix/）。
const G31_DEFAULT_SPV_ENCODE: &str = ".tmp/night_0828/spv/g31_display_encode_v2.spv";
/// G37 W2 合入:--lut 色彩分级 encode kernel（源 = kernels/g31_display_encode_lut.rx
/// ——母版 ACES 已修版逐字 fork,唯一新增 LUT 段〔ODT 色域钳后/BT.1886 前,显示
/// 线性域 trilinear〕;`.tmp` 构建产物）。**字节隔离**（C 相纪律）:--lut off 恒载
/// 上行 v2 锚定字节（不载本件,55e4a92d/5db2e7d7 等锚零漂移）,--lut 非 off 且
/// --spv-encode 未显式给出才换载本件（「默认字面才换」tsrq 同律;显式 SPV 无
/// LUT 段即静默失效冒充,fail-closed 拒组合）。
const G31_DEFAULT_SPV_ENCODE_LUT: &str = ".tmp/night_0830/spv/g31_display_encode_lut.spv";
/// A5 FG kernel 默认 SPV（源 = kernels/g26_framegen.rx——G26 kernel 本体 0-byte;
/// `.tmp` 构建产物,CI 门脚本保障编译）。
const G31_DEFAULT_SPV_FRAMEGEN: &str = ".tmp/g14_gates/m_c/g26_framegen.spv";
/// A5 MV 取反 glue kernel 默认 SPV（源 = kernels/g31_mv_negate.rx;`.tmp` 构建
/// 产物,CI 门脚本保障编译）。
const G31_DEFAULT_SPV_MVN: &str = ".tmp/g14_gates/m_c/g31_mv_negate.spv";
/// A5 冻结容差事实源（G26 标定 budget;`--fg-tol` 缺省时程序读,fail-closed）。
const G31_G26_BUDGET: &str = "milestones/g26/g26_budget.json";
/// A5 冻结容差条目标识（threshold = measured × 2.0 程序产;条目字面 G26 钉死）。
const G31_FG_TOL_ENTRY: &str = "g26.framegen_device.host_device_maxdiff_tol";
/// B3 slab 接线门键（--slab-table 面 evidence `gate` 字段字面）。
const G31_SLAB_GATE: &str = "g31.waveB.slab";
/// B3 slab 接线 evidence schema 字面（milestones/g31/g31_slab_wiring_evidence_schema.json 同字面）。
const G31_SLAB_SCHEMA: &str = "rurix.g31.slab_wiring_evidence.v1";
/// B3 slab device 求值 kernel 默认 SPV（源 = kernels/g29_slab.rx——G29 M-a 本体
/// 0-byte 冻结消费;`.tmp` 构建产物,CI 门脚本保障编译）。
const G31_DEFAULT_SPV_SLAB: &str = ".tmp/g14_gates/m_c/g29_slab.spv";
/// B4 纹理采样接线门键（--textures on 面 evidence `gate` 字段字面）。
const G31_TEXTURE_GATE: &str = "g31.waveB.texture";
/// B4 纹理采样接线 evidence schema 字面（milestones/g31/
/// g31_texture_sampling_evidence_schema.json 同字面）。
const G31_TEXTURE_SCHEMA: &str = "rurix.g31.texture_sampling_evidence.v1";
/// B4 生产场景 kernel 纹理变体默认 SPV（源 = kernels/g31_texture_gi.rx——
/// g14_3_direct_gi.rx 逐字 fork + 贴图采样 albedo 面;母版 kernel/SPV 0-byte,
/// off 面 = 回归锚）。day_0828 Phase B 指向 v2 隔离路（A2b 治理先例）：
/// heap+mip+fx/fy 修复后 SPV 与旧 host 数据布局不兼容——旧共享路
/// `.tmp/g31_gates/texture/g31_texture_gi.spv` 0-byte 不动（target/release
/// 旧二进制 + ci/g31_texture_sampling_smoke.py 现编面消费旧形态）,新形态
/// 走 night_0828 spv 目录;源码（kernels/*.rx）单一事实源已改,共享路径
/// 字节与源码 divergence 如实登记为交接项。
const G31_DEFAULT_SPV_TEXTURE: &str = ".tmp/night_0828/spv/g31_texture_gi_v2.spv";
/// B4 探针 kernel 默认 SPV（源 = kernels/g31_texture_probe.rx——生产采样块
/// 隔离对拍面;v2 隔离路同上,探针步幅 3→4 heap 化）。
const G31_DEFAULT_SPV_TEXTURE_PROBE: &str = ".tmp/night_0828/spv/g31_texture_probe_v2.spv";
/// day_0828 Phase B 统一质量 kernel 默认 SPV（源 =
/// kernels/g31_texture_nrm_gi.rx——g18_smooth_nrm 逐字 fork + texel heap
/// 贴图采样合体;(--smooth-normals on && --textures on) 合流臂换载）。
const G31_DEFAULT_SPV_TEXTURE_NRM: &str = ".tmp/night_0828/spv/g31_texture_nrm_gi.spv";
/// Phase C GI2 变体 SPV（同源 kernels/g31_texture_nrm_gi.rx 现编译产物，含
/// GI2 段；仅 --gi2 on 换载——**路线隔离**（A2b v2 同律）：gi2-off 合流臂
/// 恒载上行锚定字节（8b1c12f3 锚承载文件 0-byte 不动）。根因登记：GI2 段
/// （新增 2 ray query 站点 + sin/cos + 动态循环）令驱动后端对既有代码重编译
/// 产 ULP 级扰动经 TSR/AE 反馈放大（E1 探针证明纯 ALU +0.0 尾加恒等成立，
/// ev/e1_tailadd.json == 8b1c12f3）——字节隔离为强于数学恒等的保锚形态。
const G31_DEFAULT_SPV_TEXTURE_NRM_GI2: &str = ".tmp/night_0828/spv/g31_texture_nrm_gi_gi2.spv";
/// day_0828 Phase F emissive 贴图变体 SPV（同源 kernels/g31_texture_nrm_gi.rx
/// 现编译产物，含 triem 绑定 + 逐像素 emissive 采样段；仅 --emissive-tex on
/// 换载——gi2 on/off 都用本工件（GI2 段 params[51] 门控在内）。**字节隔离**
/// （C 相纪律）：em-off 各臂恒载既有锚定字节（g31_texture_nrm_gi.spv /
/// *_gi2.spv 两件 0-byte 不动）——新增采样站点令既有臂 gate=0 恒等不可依赖，
/// 不试图恒等直接隔离。
const G31_DEFAULT_SPV_TEXTURE_NRM_EM: &str = ".tmp/night_0828/spv/g31_texture_nrm_gi_em.spv";
/// day_0829 真实感战役 realism 链 SPV（源 = kernels/g31_realism.rx——
/// g31_texture_nrm_gi.rx Phase F 后源码逐字 fork 演进链;臂① --metal-f0 =
/// 金属 F0 修伤,签名 +tri_base 未衰减 baseColor 侧表〔15 路〕）。**字节隔离**
/// （C 相纪律）：realism 全臂 off 恒载 night_0828 三既有锚定字节（nrm/gi2/em
/// 0-byte 不动），任一 realism 臂 on 独载本链工件（链式超集：SPV_k 含臂 1..k
/// 的 params 门,换载取 on 集最高臂）。
const G31_DEFAULT_SPV_REALISM_F0: &str = ".tmp/night_0829/spv/g31_realism_f0.spv";
/// day_0829 臂② --rt-ao 链工件（f0 超集 + 短程 AO 遮蔽射线段;params[56..60)
/// 门控在内——链式换载取 on 集最高臂,f0 on/off 都可用本工件〔[55] 门控〕）。
const G31_DEFAULT_SPV_REALISM_AO: &str = ".tmp/night_0829/spv/g31_realism_ao.spv";
/// day_0829 臂⑤ --soft-shadows 链工件（ao 超集 + 点光圆盘采样软阴影段;
/// params[60..62) 门控在内——TODO #27 SMRT 方向简化形）。
const G31_DEFAULT_SPV_REALISM_SOFT: &str = ".tmp/night_0829/spv/g31_realism_soft.spv";
/// day_0829 臂③ --rt-reflect 链工件（soft 超集 + GGX 重要性采样反射射线段;
/// params[62..65) 门控在内——命中点 GI2 形着色 + Fresnel 权重加性合成,有偏
/// 近似如实登记〔单样本无 pdf 归一,能量由 clamp+w 控〕）。
const G31_DEFAULT_SPV_REALISM_REFL: &str = ".tmp/night_0829/spv/g31_realism_refl.spv";
/// day_0829 臂⑥ --gi2-tex 链工件（refl 超集 + GI2 反弹点贴图 albedo/逐像素
/// emission 采样段;params[67] 门控在内——反弹命中重心 UV + 主命中同 lod
/// 公式〔距离 = 反弹程〕,mats 均值面 while 计数门回退）。
const G31_DEFAULT_SPV_REALISM_GITEX: &str = ".tmp/night_0829/spv/g31_realism_gitex.spv";
/// day_0829 臂④ --normal-maps 链工件（gitex 超集 + 法线贴图 TBN 扰动段;
/// params[65..67) 门控在内——签名 +trinm/tri_tan 两路〔17 buffer,最高链位〕;
/// BC5 法线进 heap 新槽 74..143,切线 = 装配期 UV 导数法〔glTF 无 TANGENT〕）。
const G31_DEFAULT_SPV_REALISM_NRM: &str = ".tmp/night_0829/spv/g31_realism_nrm.spv";
/// G37 W2 臂⑦ --transparency 链工件（nrm 超集 + 玻璃透射段;params[68] 门控
/// 在内——签名 +tri_transp 1 f32/tri 透射率侧表〔18 路 View,新最高链位〕;
/// 主射线命中透明三角沿原方向推进重投累积 tint〔透射率 × tri_base 未衰减
/// baseColorFactor——mats 均值面 ×(1−metal)×灰贴图均值双重衰减全黑不可用,
/// 故 transp on 时 tri_base 恒真表〕,点光阴影 first_hit 判遮命中透明三角时
/// closest-hit 重走衰减〔玻璃灰影,纯透射率不带色调〕;transp on 而
/// --normal-maps off 时 trinm 绑 -1 回退真表/tri_tan 绑 16B 零哑表保持
/// 签名序〔triem 回退表同律〕）。
const G31_DEFAULT_SPV_REALISM_TRANSP: &str = ".tmp/night_0829/spv/g31_realism_transp.spv";
/// G37 W2 臂⑧ --gi2-ris/--gi2-nee 链工件(transp 超集 + GI2 反弹 RIS 选灯/
/// 灯片 CDF 面光 NEE 段;params[69..72) 门控在内——签名 +lamp_tbl 灯片表
/// 〔19 路 View,新最高链位〕;能量口径 = nee on 时反弹直击灯片 emission
/// 置零 + 聚类代表灯让位灯片真域,详 w2_wiring/ris_nee/REPORT.md)。
const G31_DEFAULT_SPV_REALISM_RIS: &str = ".tmp/night_0830/spv/g31_realism_ris.spv";
/// day_0829 realism params 扩面长度（[55..72)：[55] metal-f0 门 [56..60)
/// rt-ao [60..62) soft-shadows [62..65) rt-reflect [65..67) normal-maps
/// [67] gi2-tex,[68] G37 W2 transparency 门,[69] gi2-ris 门 [70] ris_m
/// [71] gi2-nee 门(G37 W2 ris_nee)——base 56 槽
/// 布局 0-byte 不动,realism 任一臂 on 时 params buffer/逐帧上传扩本长度,
/// off 恒 PARAMS_LEN 既有面）。
const G31_REAL_PARAMS_LEN: usize = 72;
/// C13 SVT 接线门键（--svt on 面 evidence `gate` 字段字面）。
const G31_SVT_GATE: &str = "g31.waveC.svt";
/// C13 SVT 接线 evidence schema 字面（milestones/g31/g31_svt_evidence_schema.json 同字面）。
const G31_SVT_SCHEMA: &str = "rurix.g31.svt_evidence.v1";
/// C13 SVT 生产 kernel 默认 SPV（源 = kernels/g31_svt_gi.rx——g31_texture_gi.rx
/// 逐字 fork + 页表间接采样/miss 记录/fallback;`.tmp` 构建产物,CI 门脚本保障编译）。
const G31_DEFAULT_SPV_SVT: &str = ".tmp/g31_gates/svt/g31_svt_gi.spv";
/// C13 SVT 探针 kernel 默认 SPV（源 = kernels/g31_svt_probe.rx;同上）。
const G31_DEFAULT_SPV_SVT_PROBE: &str = ".tmp/g31_gates/svt/g31_svt_probe.spv";

/// A3 车道追加资源下标（Mega 22 资源 0..=21 之后;Split 形态的 U_HIT_* 占用
/// 22..=24 与本面互斥——本 bin 恒 Mega,bistro quads=0）。
const G31_U_ENC_PARAMS: u32 = 22;
const G31_U_ENC_OUT: u32 = 23;
const G31_U_RESOURCE_COUNT: usize = 24;

/// A3 encode pass 屏障计划（保守超集逐字声明同律：读 TSR out_color 双 parity
/// 并集 + 编码参数 + BGRA8 输出;readback 触达由执行器隐式超集覆盖）。
const G31_U_PLAN_ENCODE: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
    (G31_U_ENC_OUT, TargetState::StorageReadWrite),
];

// ---------------------------------------------------------------------------
// B4 纹理采样面：资源下标（textures off 时车道 24 资源 0-byte 现状——on 追加
// 24..=28 五件 SSBO 侧表;与 fg/hzb/slab 各面互斥,下标无撞面）、屏障计划、
// 纹理变体描述组。
// ---------------------------------------------------------------------------

/// B4 追加资源下标（textures on 才存在;24=逐三角 UV〔6 f32/tri〕,25=texmeta
/// 头+槽表,26=逐三角槽索引〔−1 = 常量面〕,27=u32 打包 RGBA8 图集,28=256
/// 项 srgb→linear LUT）。
const G31_U_TEX_UV: u32 = 24;
const G31_U_TEX_META: u32 = 25;
const G31_U_TEX_TRITEX: u32 = 26;
const G31_U_TEX_ATLAS: u32 = 27;
const G31_U_TEX_LINLUT: u32 = 28;
/// B4 纹理车道资源数（24 既有 + 5 追加）。
const G31_U_RESOURCE_COUNT_TEX: usize = 29;

// ---------------------------------------------------------------------------
// C13 SVT 面：资源下标（svt off 时车道 0-byte 现状——on 在 B4 五件后再追加
// 29..=33 五件;须随 --textures on,与 fg/hzb/slab 闭集互斥同律）、屏障计划、
// readback 下标。
// ---------------------------------------------------------------------------

/// C13 追加资源下标（svt on 才存在;29=页表〔1024² u32〕,30=物理瓦片池
/// 〔pool_tiles×130² u32〕,31=miss 请求缓冲〔1 f32/px〕,32=svtmeta〔8 f32〕,
/// 33=fallback 表〔槽数×4 f32〕）。
const G31_U_SVT_PAGETABLE: u32 = 29;
const G31_U_SVT_POOL: u32 = 30;
const G31_U_SVT_REQ: u32 = 31;
const G31_U_SVT_META: u32 = 32;
const G31_U_SVT_FALLBACK: u32 = 33;
/// C13 SVT 车道资源数（B4 29 + 5 追加）。
const G31_U_RESOURCE_COUNT_SVT: usize = 34;
/// C13 readback 下标（svt on 面;0..=4 与 textures 面逐字同源,5 = miss 请求缓冲）。
const G31_RB_SVT_REQ: u32 = 5;

/// C13 SVT 变体 scene pass 屏障计划（G31_U_PLAN_SCENE_TEX 触达超集 + C13 五件——
/// 保守超集同律）。
const G31_U_PLAN_SCENE_SVT: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV, TargetState::StorageReadWrite),
    (G31_U_TEX_META, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (G31_U_SVT_PAGETABLE, TargetState::StorageReadWrite),
    (G31_U_SVT_POOL, TargetState::StorageReadWrite),
    (G31_U_SVT_REQ, TargetState::StorageReadWrite),
    (G31_U_SVT_META, TargetState::StorageReadWrite),
    (G31_U_SVT_FALLBACK, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];

/// B4 纹理变体 scene pass 屏障计划（U_PLAN_SCENE 触达超集 + B4 五件——
/// 保守超集同律;读侧 SSBO 与写侧 out 同域 StorageReadWrite）。
const G31_U_PLAN_SCENE_TEX: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV, TargetState::StorageReadWrite),
    (G31_U_TEX_META, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];

// ---------------------------------------------------------------------------
// day_0828 Phase B 合流臂资源下标（--textures × --smooth-normals 互斥解除 +
// --textures × --bloom/--auto-exposure 组合）。下标纪律 = A2 致密 Vec 尾挂
// 同律（四形态互斥占用,builder 内断言资源表连号）：
// ① tex+nrm（bloom off）：tex 五件留 24..28 不动（atlas_design.md §5——SVT
//    29..33 栈序依赖），trinrm=29、tri_mr=30，count=31；AE 尾挂 31..=33。
// ② tex+bloom（smooth off）：bloom 八件留 24..=31 不动（g31_lane_descs_bloom
//    产物 + prepare_update parity override 面 0-byte 复用），tex 五件尾挂
//    32..36，count=37；AE 尾挂 37..=39。
// ③ tex+nrm+bloom：② 基础上 trinrm=37、tri_mr=38，count=39；AE 尾挂 39..=41。
// ④ tex 单臂（既有）：count=29；AE 尾挂 29..=31。
// ---------------------------------------------------------------------------

/// ① tex+nrm 合流面（trinrm/tri_mr 从单臂 24/25 让位 29/30——解撞 texuv/
/// texmeta;未来 tex+nrm+svt 三合臂 trinrm 再让位 34/35,本波不动）。
const G31_U_TRINRM_TEX: u32 = 29;
const G31_U_TRI_MR_TEX: u32 = 30;
const G31_U_RESOURCE_COUNT_TEXNRM: usize = 31;
/// ② tex+bloom 组合面（tex 五件尾挂 bloom 八件之后）。
const G31_U_TEX_UV_BLOOM: u32 = 32;
const G31_U_TEX_META_BLOOM: u32 = 33;
const G31_U_TEX_TRITEX_BLOOM: u32 = 34;
const G31_U_TEX_ATLAS_BLOOM: u32 = 35;
const G31_U_TEX_LINLUT_BLOOM: u32 = 36;
const G31_U_RESOURCE_COUNT_TEX_BLOOM: usize = 37;
/// ③ tex+nrm+bloom 合流面。
const G31_U_TRINRM_TEX_BLOOM: u32 = 37;
const G31_U_TRI_MR_TEX_BLOOM: u32 = 38;
const G31_U_RESOURCE_COUNT_TEXNRM_BLOOM: usize = 39;
/// day_0828 Phase F emissive 变体面（em on 才存在;triem 逐三角 emissive 槽
/// 号侧表〔1 f32/tri〕尾挂 tri_mr 之后——CLI 约束 em ⇒ (textures &&
/// smooth-normals) ⇒ 仅 tex_nrm / tex_nrm_bloom 两形态可达,其余形态不接）。
const G31_U_TRIEM_TEXNRM: u32 = 31;
const G31_U_RESOURCE_COUNT_TEXNRM_EM: usize = 32;
const G31_U_TRIEM_TEXNRM_BLOOM: u32 = 39;
const G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_EM: usize = 40;
// day_0829 真实感战役：tri_base 未衰减 baseColor 侧表下标（realism 任一臂 on
// 面;kernel 绑定序 = triem 之后〔g31_realism.rx 签名序〕——em off 时 triem
// 绑 tri_count×(-1.0) 回退真表保持签名序,tri_base off 臂零触达 0-byte）。
const G31_U_TRIBASE_TEXNRM: u32 = 32;
const G31_U_RESOURCE_COUNT_TEXNRM_REAL: usize = 33;
const G31_U_TRIBASE_TEXNRM_BLOOM: u32 = 40;
const G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_REAL: usize = 41;
// day_0829 臂④：trinm/tri_tan 侧表下标（--normal-maps on 面;kernel 绑定序
// = tri_base 之后〔g31_realism.rx 17 buffer 签名〕;off 不尾挂——SPV 链下位
// 工件 15 buffer,多余绑定即 layout 失配 fail）。
const G31_U_TRINM_TEXNRM: u32 = 33;
const G31_U_TRITAN_TEXNRM: u32 = 34;
const G31_U_RESOURCE_COUNT_TEXNRM_NM: usize = 35;
const G31_U_TRINM_TEXNRM_BLOOM: u32 = 41;
const G31_U_TRITAN_TEXNRM_BLOOM: u32 = 42;
const G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_NM: usize = 43;
// G37 W2 transparency：tri_transp 透射率侧表下标（--transparency on 面;
// kernel 签名序 = tri_tan 之后新最高链位——transp on 而 nm off 时 trinm/
// tri_tan 绑回退表/哑表恒占 33/34〔41/42〕位,tri_transp 恒 35〔43〕）。
const G31_U_TRITRANSP_TEXNRM: u32 = 35;
const G31_U_RESOURCE_COUNT_TEXNRM_TRANSP: usize = 36;
const G31_U_TRITRANSP_TEXNRM_BLOOM: u32 = 43;
const G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_TRANSP: usize = 44;
// G37 W2 ris_nee:lamp_tbl 灯片表下标(--gi2-ris|--gi2-nee on 面;kernel
// 签名序 = tri_transp 之后新最高链位——ris|nee on 而 transp off 时
// tri_transp 绑 tri_count×0.0 零表恒占 35〔43〕位,lamp_tbl 恒 36〔44〕)。
const G31_U_LAMPTBL_TEXNRM: u32 = 36;
const G31_U_RESOURCE_COUNT_TEXNRM_RIS: usize = 37;
const G31_U_LAMPTBL_TEXNRM_BLOOM: u32 = 44;
const G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_RIS: usize = 45;

/// ① tex+nrm scene pass 屏障计划（U_PLAN_SCENE_TEX 超集 + trinrm/tri_mr）。
const G31_U_PLAN_SCENE_TEXNRM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV, TargetState::StorageReadWrite),
    (G31_U_TEX_META, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// Phase F ①+em tex+nrm+emissive scene pass 屏障计划（TEXNRM 超集 + triem）。
const G31_U_PLAN_SCENE_TEXNRM_EM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV, TargetState::StorageReadWrite),
    (G31_U_TEX_META, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// day_0829 ①+realism tex+nrm(+em 回退)+tri_base scene pass 屏障计划
/// （TEXNRM_EM 超集 + tri_base——realism 任一臂 on 面,triem 恒绑〔真表或
/// -1 回退表〕保持 kernel 签名序）。
const G31_U_PLAN_SCENE_TEXNRM_REAL: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV, TargetState::StorageReadWrite),
    (G31_U_TEX_META, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRIBASE_TEXNRM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// day_0829 臂④+realism tex+nrm+tri_base+trinm+tri_tan scene pass 屏障计划
/// （REAL 超集 + 法线两路——--normal-maps on 面）。
const G31_U_PLAN_SCENE_TEXNRM_NM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV, TargetState::StorageReadWrite),
    (G31_U_TEX_META, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRIBASE_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRINM_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRITAN_TEXNRM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// G37 W2 transparency scene pass 屏障计划（NM 超集 + tri_transp——
/// --transparency on 面,nm off 时 trinm/tri_tan 为回退表/哑表恒绑）。
const G31_U_PLAN_SCENE_TEXNRM_TRANSP: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV, TargetState::StorageReadWrite),
    (G31_U_TEX_META, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRIBASE_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRINM_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRITAN_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRITRANSP_TEXNRM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// G37 W2 ris_nee scene pass 屏障计划（TRANSP 超集 + lamp_tbl）。
const G31_U_PLAN_SCENE_TEXNRM_RIS: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV, TargetState::StorageReadWrite),
    (G31_U_TEX_META, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRIBASE_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRINM_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRITAN_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_TRITRANSP_TEXNRM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (G31_U_LAMPTBL_TEXNRM, TargetState::StorageReadWrite),
];
/// ② tex+bloom scene pass 屏障计划（tex 五件 = bloom 组合下标）。
const G31_U_PLAN_SCENE_TEX_BLOOM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_META_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT_BLOOM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// ③ tex+nrm+bloom scene pass 屏障计划。
const G31_U_PLAN_SCENE_TEXNRM_BLOOM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_META_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX_BLOOM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// Phase F ③+em tex+nrm+bloom+emissive scene pass 屏障计划。
const G31_U_PLAN_SCENE_TEXNRM_BLOOM_EM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_META_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// day_0829 ③+realism tex+nrm+bloom(+em 回退)+tri_base scene pass 屏障计划
/// （TEXNRM_BLOOM_EM 超集 + tri_base）。
const G31_U_PLAN_SCENE_TEXNRM_BLOOM_REAL: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_META_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRIBASE_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// day_0829 臂④+realism tex+nrm+bloom+tri_base+trinm+tri_tan scene pass
/// 屏障计划（BLOOM_REAL 超集 + 法线两路）。
const G31_U_PLAN_SCENE_TEXNRM_BLOOM_NM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_META_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRIBASE_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRINM_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRITAN_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// G37 W2 transparency ×bloom scene pass 屏障计划（BLOOM_NM 超集 + tri_transp）。
const G31_U_PLAN_SCENE_TEXNRM_BLOOM_TRANSP: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_META_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRIBASE_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRINM_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRITAN_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRITRANSP_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// G37 W2 ris_nee ×bloom scene pass 屏障计划（BLOOM_TRANSP 超集 + lamp_tbl）。
const G31_U_PLAN_SCENE_TEXNRM_BLOOM_RIS: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TEX_UV_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_META_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_TRITEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_ATLAS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TEX_LINLUT_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRINRM_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRIEM_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRIBASE_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRINM_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRITAN_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRITRANSP_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (G31_U_LAMPTBL_TEXNRM_BLOOM, TargetState::StorageReadWrite),
];

// ---------------------------------------------------------------------------
// A5 FG 接线面：资源下标（fg off 时车道 24 资源 0-byte;fg x2 追加 24..=28,
// x3 再追加 29..=31）、屏障计划、档位闭集、冻结容差读取、kernel 参数打包。
// ---------------------------------------------------------------------------

/// A5 FG 追加资源下标（fg on 才存在;24/25 = MV 取反 glue 参数/输出,26..=28 =
/// FG1 参数/输出 f32/BGRA8,29..=31 = FG2 同构 x3）。
const G31_U_MVN_PARAMS: u32 = 24;
const G31_U_MVN: u32 = 25;
const G31_U_FG1_PARAMS: u32 = 26;
const G31_U_FG1_OUT: u32 = 27;
const G31_U_FG1_BGRA: u32 = 28;
const G31_U_FG2_PARAMS: u32 = 29;
const G31_U_FG2_OUT: u32 = 30;
const G31_U_FG2_BGRA: u32 = 31;

/// A5 readback 下标（fg on 面;0..=4 与 fg off 逐字同源：0/1=OUT_COLOR f32,
/// 2=MV,3=DEPTH,4=cur BGRA8;x2: 5=FG1_BGRA,6=FG1_OUT f32;x3: 5=FG1_BGRA,
/// 6=FG2_BGRA,7=FG1_OUT,8=FG2_OUT）。
const G31_RB_BGRA: u32 = 4;

/// A5 MV 取反 pass 屏障计划（保守超集同律：读 MV_OUT + 参数,写 MVN）。
const G31_U_PLAN_MVN: &[(u32, TargetState)] = &[
    (U_MV_OUT, TargetState::StorageReadWrite),
    (G31_U_MVN_PARAMS, TargetState::StorageReadWrite),
    (G31_U_MVN, TargetState::StorageReadWrite),
];
/// A5 fg1 pass 屏障计划（kernel 读 prev/cur out_color 双 parity 并集 + 取反
/// MV + 参数,写 FG1_OUT）。
const G31_U_PLAN_FG1: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_MVN, TargetState::StorageReadWrite),
    (G31_U_FG1_PARAMS, TargetState::StorageReadWrite),
    (G31_U_FG1_OUT, TargetState::StorageReadWrite),
];
/// A5 fg1 编码 pass 屏障计划（读 FG1_OUT + 编码参数,写 FG1_BGRA）。
const G31_U_PLAN_ENC_FG1: &[(u32, TargetState)] = &[
    (G31_U_FG1_OUT, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
    (G31_U_FG1_BGRA, TargetState::StorageReadWrite),
];
/// A5 fg2 pass 屏障计划（x3）。
const G31_U_PLAN_FG2: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_MVN, TargetState::StorageReadWrite),
    (G31_U_FG2_PARAMS, TargetState::StorageReadWrite),
    (G31_U_FG2_OUT, TargetState::StorageReadWrite),
];
/// A5 fg2 编码 pass 屏障计划（x3）。
const G31_U_PLAN_ENC_FG2: &[(u32, TargetState)] = &[
    (G31_U_FG2_OUT, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
    (G31_U_FG2_BGRA, TargetState::StorageReadWrite),
];

/// A5 FG 档闭集（off = 车道 0-byte 现状;x2 = 每帧对插 1 帧;x3 = 插 2 帧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum G31Fg {
    Off,
    X2,
    X3,
}

impl G31Fg {
    /// presented/real 倍率（off=1, x2=2, x3=3）。
    fn factor(self) -> u32 {
        match self {
            G31Fg::Off => 1,
            G31Fg::X2 => 2,
            G31Fg::X3 => 3,
        }
    }
    /// 每对真渲帧插入帧数（x2=1, x3=2;host `mfg_inserted_frames` 同值）。
    fn inserted(self) -> u32 {
        self.factor() - 1
    }
    fn name(self) -> &'static str {
        match self {
            G31Fg::Off => "off",
            G31Fg::X2 => "x2",
            G31Fg::X3 => "x3",
        }
    }
}

/// A5 FG readback 布局（x2/x3 下标差异面;lane 创建期定,逐帧子集选路用）。
#[derive(Debug, Clone, Copy)]
struct G31FgLayout {
    rb_fg1_bgra: u32,
    rb_fg2_bgra: u32,
    rb_fg1_out: u32,
    rb_fg2_out: u32,
    /// MVN 取反结果回读（probe 帧 device 侧 MV 内容直比对面）。
    rb_mvn: u32,
    /// G37 W3 fg_combo：comp parity 对回读（fg×full 面 probe 帧 host
    /// interpolate 复算的 prev/cur 换 comp 对;base/off 面 MAX 不消费）。
    rb_comp0: u32,
    rb_comp1: u32,
}

impl G31FgLayout {
    fn of(fg: G31Fg) -> Self {
        match fg {
            G31Fg::X2 => Self {
                rb_fg1_bgra: 5,
                rb_fg2_bgra: u32::MAX,
                rb_fg1_out: 6,
                rb_fg2_out: u32::MAX,
                rb_mvn: 7,
                rb_comp0: u32::MAX,
                rb_comp1: u32::MAX,
            },
            G31Fg::X3 => Self {
                rb_fg1_bgra: 5,
                rb_fg2_bgra: 6,
                rb_fg1_out: 7,
                rb_fg2_out: 8,
                rb_mvn: 9,
                rb_comp0: u32::MAX,
                rb_comp1: u32::MAX,
            },
            G31Fg::Off => Self {
                rb_fg1_bgra: u32::MAX,
                rb_fg2_bgra: u32::MAX,
                rb_fg1_out: u32::MAX,
                rb_fg2_out: u32::MAX,
                rb_mvn: u32::MAX,
                rb_comp0: u32::MAX,
                rb_comp1: u32::MAX,
            },
        }
    }

    /// G37 W3 fg_combo：fg×full 面布局（base 5 件后 comp 对 5/6 先入列——
    /// g31_apply_fg_full readback 入列序同源;full 变体 tex/nrm/bloom/AE 均
    /// 不加逐帧 readback,基座 0..=4 与 base 逐字同序）。
    fn of_full(fg: G31Fg) -> Self {
        match fg {
            G31Fg::X2 => Self {
                rb_fg1_bgra: 7,
                rb_fg2_bgra: u32::MAX,
                rb_fg1_out: 8,
                rb_fg2_out: u32::MAX,
                rb_mvn: 9,
                rb_comp0: 5,
                rb_comp1: 6,
            },
            G31Fg::X3 => Self {
                rb_fg1_bgra: 7,
                rb_fg2_bgra: 8,
                rb_fg1_out: 9,
                rb_fg2_out: 10,
                rb_mvn: 11,
                rb_comp0: 5,
                rb_comp1: 6,
            },
            G31Fg::Off => Self::of(G31Fg::Off),
        }
    }
}

/// A5 冻结容差程序读（`--fg-tol` 缺省面;milestones/g26/g26_budget.json 标定
/// 条目 threshold + measured_value,fail-closed 禁手写阈）。返回
/// (threshold, measured_value, tol_source 登记串)。
fn g31_fg_frozen_tol(budget_path: &str) -> Result<(f64, f64, String), String> {
    let doc = json_parse(
        &std::fs::read_to_string(budget_path).map_err(|e| format!("读 {budget_path}: {e}"))?,
    )?;
    let entries = doc
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("{budget_path} 缺 entries 数组"))?;
    for e in entries {
        if e.get("id").and_then(|v| v.as_str()) == Some(G31_FG_TOL_ENTRY) {
            let thr = e
                .get("threshold")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("{G31_FG_TOL_ENTRY} 缺 threshold"))?;
            let meas = e
                .get("measured_value")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| format!("{G31_FG_TOL_ENTRY} 缺 measured_value"))?;
            return Ok((
                thr,
                meas,
                format!("{}#{}", budget_path.replace('\\', "/"), G31_FG_TOL_ENTRY),
            ));
        }
    }
    Err(format!(
        "{budget_path} 缺标定条目 {G31_FG_TOL_ENTRY}（G26 标定腿未落档,fail-closed）"
    ))
}

/// A5 FG kernel 参数面打包（与 g26_framegen.rx 参数面逐字同源;16 f32 位级
/// 编码——pixel_count/width/height/t/inv_sigma2/red_bias=0/reserved 恒 0）。
/// `t` = t_temporal = i/(n+1)（取反 glue 直通面,host mfg_between 同式位级）。
fn g31_fg_pack_params(pixel_count: u32, w: u32, h: u32, t: f32, inv_sigma2: f32) -> Vec<f32> {
    let mut v = vec![
        pixel_count as f32,
        w as f32,
        h as f32,
        t,
        inv_sigma2,
        0.0,
    ];
    v.resize(16, 0.0);
    v
}

/// A5 MV 取反 glue 参数面打包（与 g31_mv_negate.rx 参数面逐字同源;16 f32——
/// element_count = 2·pixel_count,reserved 恒 0）。
fn g31_mvn_pack_params(element_count: u32) -> Vec<f32> {
    let mut v = vec![element_count as f32];
    v.resize(16, 0.0);
    v
}

// ---------------------------------------------------------------------------
// 夜间巡航 D3 bloom 加性面（--bloom <off|on>,默认 off = 车道 24 资源/五 pass
// 0-byte 现状;on 在 resolve 后/display_encode 前插入四 pass:bright(软膝阈值+
// 2×降采样)→blur H→blur V(9-tap 可分离高斯)→composite(双线性上采样×strength
// 加性合成回全分辨率 HDR),display_encode 的 in_color 绑定从 TSR out_color
// [parity] 换成合成缓冲。与 --fg/--hzb/--textures/--svt/--slab-table/
// --cluster-lod/--wp-hlod fail-closed 互斥（组合面未接线;下标 24..=31 与
// FG/纹理面互斥占用,同既有闭集纪律）。
// ---------------------------------------------------------------------------

/// D3 追加资源下标（bloom on 才存在;24=bright 参数,25=bright 半分辨率亮部,
/// 26=blur H 参数,27=ping 半分辨率,28=blur V 参数,29=pong 半分辨率,
/// 30=composite 参数,31=composite 全分辨率合成输出）。
const G31_U_BLOOM_BRIGHT_PARAMS: u32 = 24;
const G31_U_BLOOM_BRIGHT: u32 = 25;
const G31_U_BLOOM_BLUR_H_PARAMS: u32 = 26;
const G31_U_BLOOM_PING: u32 = 27;
const G31_U_BLOOM_BLUR_V_PARAMS: u32 = 28;
const G31_U_BLOOM_PONG: u32 = 29;
const G31_U_BLOOM_COMP_PARAMS: u32 = 30;
const G31_U_BLOOM_COMP_OUT: u32 = 31;
/// D3 bloom 车道资源数（24 既有 + 8 追加）。
const G31_U_RESOURCE_COUNT_BLOOM: usize = 32;

/// D3 bloom 三 kernel 默认 SPV（源 = kernels/g31_bloom_{bright,blur,composite}.rx;
/// `.tmp` 构建产物,spirv-val 绿）。
const G31_DEFAULT_SPV_BLOOM_BRIGHT: &str = ".tmp/night_0828/spv/g31_bloom_bright.spv";
const G31_DEFAULT_SPV_BLOOM_BLUR: &str = ".tmp/night_0828/spv/g31_bloom_blur.spv";
const G31_DEFAULT_SPV_BLOOM_COMPOSITE: &str = ".tmp/night_0828/spv/g31_bloom_composite.spv";
/// D3 软膝宽度默认（kernel 头注释膝 = 0.5 固定斜率过渡带同值;host 单一事实源）。
const G31_BLOOM_KNEE: f32 = 0.5;
/// D3 默认加性强度（scene-linear 曝光域;--bloom-strength 可调）。
const G31_BLOOM_DEFAULT_STRENGTH: f32 = 0.3;
/// D3 默认 luma 阈（scene-linear 曝光域;--bloom-threshold 可调）。
const G31_BLOOM_DEFAULT_THRESHOLD: f32 = 1.0;

/// D3 bright pass 屏障计划（保守超集同律:读 TSR out_color 双 parity 并集 +
/// 参数,写半分辨率亮部）。
const G31_U_PLAN_BLOOM_BRIGHT: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_BLOOM_BRIGHT_PARAMS, TargetState::StorageReadWrite),
    (G31_U_BLOOM_BRIGHT, TargetState::StorageReadWrite),
];
/// D3 blur H pass 屏障计划（读亮部 + 参数,写 ping）。
const G31_U_PLAN_BLOOM_BLUR_H: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_BRIGHT, TargetState::StorageReadWrite),
    (G31_U_BLOOM_BLUR_H_PARAMS, TargetState::StorageReadWrite),
    (G31_U_BLOOM_PING, TargetState::StorageReadWrite),
];
/// D3 blur V pass 屏障计划（读 ping + 参数,写 pong）。
const G31_U_PLAN_BLOOM_BLUR_V: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_PING, TargetState::StorageReadWrite),
    (G31_U_BLOOM_BLUR_V_PARAMS, TargetState::StorageReadWrite),
    (G31_U_BLOOM_PONG, TargetState::StorageReadWrite),
];
/// D3 composite pass 屏障计划（读 TSR out_color 双 parity 并集 + pong + 参数,
/// 写全分辨率合成输出）。
const G31_U_PLAN_BLOOM_COMPOSITE: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_BLOOM_PONG, TargetState::StorageReadWrite),
    (G31_U_BLOOM_COMP_PARAMS, TargetState::StorageReadWrite),
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
];
/// D3 bloom on 面 encode pass 屏障计划（读合成输出 + 编码参数,写 BGRA8）。
const G31_U_PLAN_ENCODE_BLOOM: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
    (G31_U_ENC_OUT, TargetState::StorageReadWrite),
];

/// D3 bloom bright 参数面打包（与 g31_bloom_bright.rx 参数面逐字同源;8 f32——
/// in_w/in_h/threshold/knee,reserved 恒 0）。
fn g31_bloom_pack_bright_params(in_w: u32, in_h: u32, threshold: f32, knee: f32) -> Vec<f32> {
    let mut v = vec![in_w as f32, in_h as f32, threshold, knee];
    v.resize(8, 0.0);
    v
}

/// D3 bloom blur 参数面打包（与 g31_bloom_blur.rx 参数面逐字同源;8 f32——
/// w/h/dir(0=H,1=V),reserved 恒 0）。
fn g31_bloom_pack_blur_params(w: u32, h: u32, dir: f32) -> Vec<f32> {
    let mut v = vec![w as f32, h as f32, dir];
    v.resize(8, 0.0);
    v
}

/// D3 bloom composite 参数面打包（与 g31_bloom_composite.rx 参数面逐字同源;
/// 8 f32——out_w/out_h/strength/bloom_w/bloom_h,reserved 恒 0）。
fn g31_bloom_pack_composite_params(
    out_w: u32,
    out_h: u32,
    strength: f32,
    bloom_w: u32,
    bloom_h: u32,
) -> Vec<f32> {
    let mut v = vec![out_w as f32, out_h as f32, strength, bloom_w as f32, bloom_h as f32];
    v.resize(8, 0.0);
    v
}

// ---------------------------------------------------------------------------
// 画质战役 A2 自动曝光加性面（--auto-exposure <off|on>,默认 off = 车道/资源/
// pass 图 0-byte 现状 + presented digest 锚零漂移;on = encode 前插两微 pass:
// reduce（256 线程单 workgroup 跨步 log-luma 归约 → 512 f32 partials）→
// state（单线程串行求和 → 几何均值 avg_luma → target = key/avg 钳 [min,max]
// → EMA 跨帧状态 → 增益写 encode 参数 reserved 槽 [133]）。
// 绑定面纪律：encode kernel **零新增绑定**——默认 encode SPV 由
// g34_full_lane/g35_particle_lane 等他会话 3 绑定 pass 声明共享消费,新增
// storage 绑定会破他会话面（本 bin 又禁触其文件）;故增益走 params[133]
// reserved 槽（host aces13 打包面 0-byte 恒 0）+ kernel ≤0→1.0 恒等守卫,
// off 臂 ×1.0 = IEEE 全域位级恒等。下标纪律（D2 trinrm 变体尾挂同律,四形态
// 互斥占用）：base 24..=26 / nrm 26..=28 / bloom 32..=34 / nrm_bloom
// 34..=36（序 = state, params, partials）。EMA 跨帧反馈 ⇒ on 臂验收口径 =
// 双跑位级一致（非与既往锚相等）;resize era 重建 = 状态归零再适应（如实）。
// 与 --fg/--hzb/--textures/--svt/--slab-table/--cluster-lod/--wp-hlod
// fail-closed 互斥（组合面未接线）;与 --dither/--smooth-normals/--ggx/
// --lamp-lights/--bloom 全可组合。
// ---------------------------------------------------------------------------

/// A2 两 kernel 默认 SPV（源 = kernels/g31_autoexp_{reduce,state}.rx;
/// `.tmp` 构建产物,spirv-val 绿）。
const G31_DEFAULT_SPV_AE_REDUCE: &str = ".tmp/night_0828/spv/g31_autoexp_reduce.spv";
const G31_DEFAULT_SPV_AE_STATE: &str = ".tmp/night_0828/spv/g31_autoexp_state.spv";
/// A2 默认参数（key = 目标几何均值 luma;rate = EMA 逐帧步进;min/max = 增益钳）。
const G31_AE_DEFAULT_KEY: f32 = 0.115;
const G31_AE_DEFAULT_RATE: f32 = 0.08;
const G31_AE_DEFAULT_MIN: f32 = 0.125;
const G31_AE_DEFAULT_MAX: f32 = 32.0;
/// A2 state buffer 初值（[gain, initialized, avg_luma_debug, frame_count] 全 0
/// ——initialized<0.5 ⇒ 首帧直取 target;16B device 跨帧持久不清,era 重建重置）。
const G31_AE_STATE_INIT: &[u8] = &[0u8; 16];
/// A2 追加资源下标（base 变体 = 既有 24 资源尾部 24..=26）。
const G31_U_AE_STATE: u32 = 24;
const G31_U_AE_PARAMS: u32 = 25;
const G31_U_AE_PARTIALS: u32 = 26;
/// A2 追加资源下标（nrm 变体 = 26 资源尾部 26..=28）。
const G31_U_AE_STATE_NRM: u32 = 26;
const G31_U_AE_PARAMS_NRM: u32 = 27;
const G31_U_AE_PARTIALS_NRM: u32 = 28;
/// A2 追加资源下标（bloom 变体 = 32 资源尾部 32..=34）。
const G31_U_AE_STATE_BLOOM: u32 = 32;
const G31_U_AE_PARAMS_BLOOM: u32 = 33;
const G31_U_AE_PARTIALS_BLOOM: u32 = 34;
/// A2 追加资源下标（nrm×bloom 组合变体 = 34 资源尾部 34..=36）。
const G31_U_AE_STATE_NRM_BLOOM: u32 = 34;
const G31_U_AE_PARAMS_NRM_BLOOM: u32 = 35;
const G31_U_AE_PARTIALS_NRM_BLOOM: u32 = 36;

/// A2 reduce pass 屏障计划（保守超集同律:读 TSR out_color 双 parity 并集 +
/// 参数,写 partials——base 变体）。
const G31_U_PLAN_AE_REDUCE: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS, TargetState::StorageReadWrite),
];
/// A2 state pass 屏障计划（读 partials + 参数,写 state + encode 参数增益槽
/// ——base 变体;encode 侧计划已含 ENC_PARAMS 同域,写读依赖双向声明）。
const G31_U_PLAN_AE_STATE: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_AE_STATE, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// A2 reduce/state 屏障计划（nrm 变体）。
const G31_U_PLAN_AE_REDUCE_NRM: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_NRM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_NRM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_NRM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_NRM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_NRM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_NRM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// A2 reduce/state 屏障计划（bloom 变体——reduce 读 composite 合成输出,
/// encode in_color 同源静态绑定）。
const G31_U_PLAN_AE_REDUCE_BLOOM: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_BLOOM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_BLOOM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_BLOOM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// A2 reduce/state 屏障计划（nrm×bloom 组合变体）。
const G31_U_PLAN_AE_REDUCE_NRM_BLOOM: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_NRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_NRM_BLOOM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_NRM_BLOOM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_NRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_NRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_NRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// day_0828 Phase B：A2 追加资源下标（tex 变体族四形态——tex 29..=31 /
/// tex+nrm 31..=33 / tex+bloom 37..=39 / tex+nrm+bloom 39..=41;致密尾挂
/// 同律,序 = state, params, partials）。
const G31_U_AE_STATE_TEX: u32 = 29;
const G31_U_AE_PARAMS_TEX: u32 = 30;
const G31_U_AE_PARTIALS_TEX: u32 = 31;
const G31_U_AE_STATE_TEXNRM: u32 = 31;
const G31_U_AE_PARAMS_TEXNRM: u32 = 32;
const G31_U_AE_PARTIALS_TEXNRM: u32 = 33;
const G31_U_AE_STATE_TEX_BLOOM: u32 = 37;
const G31_U_AE_PARAMS_TEX_BLOOM: u32 = 38;
const G31_U_AE_PARTIALS_TEX_BLOOM: u32 = 39;
const G31_U_AE_STATE_TEXNRM_BLOOM: u32 = 39;
const G31_U_AE_PARAMS_TEXNRM_BLOOM: u32 = 40;
const G31_U_AE_PARTIALS_TEXNRM_BLOOM: u32 = 41;
/// day_0828 Phase F：A2 追加资源下标（emissive 变体两形态——triem 尾挂后
/// AE 三件顺延 +1：tex+nrm+em 32..=34 / tex+nrm+bloom+em 40..=42）。
const G31_U_AE_STATE_TEXNRM_EM: u32 = 32;
const G31_U_AE_PARAMS_TEXNRM_EM: u32 = 33;
const G31_U_AE_PARTIALS_TEXNRM_EM: u32 = 34;
const G31_U_AE_STATE_TEXNRM_BLOOM_EM: u32 = 40;
const G31_U_AE_PARAMS_TEXNRM_BLOOM_EM: u32 = 41;
const G31_U_AE_PARTIALS_TEXNRM_BLOOM_EM: u32 = 42;
/// day_0829 realism：A2 追加资源下标（tri_base 尾挂后 AE 三件再顺延 +1：
/// tex+nrm(+em 回退)+real 33..=35 / tex+nrm+bloom(+em 回退)+real 41..=43
/// ——红修 #2 根因登记:首版漏加本族,realism on + AE on 时 g31_apply_autoexp
/// 以 _EM 下标(32..=34)追加/绑定,release 下 debug_assert 不生效 ⇒ AE 三件
/// 实际落 33..=35 而绑定引用 32..=34 = tri_base 被 reduce 当 state 写、真
/// params 被当 partials 越界写——digest 确定性错乱(a1 full+f0 与无 AE 组合
/// digest 位级相同即本症状)。同域观察:既有 em+AE 组合 set_autoexp 选择块
/// 无 _EM 分支(override 传 TEXNRM 32/33 = triem/真 params)——day_0828
/// Phase F 遗留缺口,在 de342586 锚内冻结,day_0829 役不修如实登记 HANDOVER;
/// **G37 W1 已修复**(选择块补 _EM 两分支,de342586 谱系作废,重锚归 W4)。
const G31_U_AE_STATE_TEXNRM_REAL: u32 = 33;
const G31_U_AE_PARAMS_TEXNRM_REAL: u32 = 34;
const G31_U_AE_PARTIALS_TEXNRM_REAL: u32 = 35;
const G31_U_AE_STATE_TEXNRM_BLOOM_REAL: u32 = 41;
const G31_U_AE_PARAMS_TEXNRM_BLOOM_REAL: u32 = 42;
const G31_U_AE_PARTIALS_TEXNRM_BLOOM_REAL: u32 = 43;
/// day_0829 臂④：A2 追加资源下标（trinm/tri_tan 尾挂后 AE 三件再顺延 +2：
/// tex+nrm+real+nm 35..=37 / tex+nrm+bloom+real+nm 43..=45）。
const G31_U_AE_STATE_TEXNRM_NM: u32 = 35;
const G31_U_AE_PARAMS_TEXNRM_NM: u32 = 36;
const G31_U_AE_PARTIALS_TEXNRM_NM: u32 = 37;
const G31_U_AE_STATE_TEXNRM_BLOOM_NM: u32 = 43;
const G31_U_AE_PARAMS_TEXNRM_BLOOM_NM: u32 = 44;
const G31_U_AE_PARTIALS_TEXNRM_BLOOM_NM: u32 = 45;
/// G37 W2 transparency：A2 追加资源下标（tri_transp 尾挂后 AE 三件再顺延
/// +1：tex+nrm+real+nm(回退)+transp 36..=38 / ×bloom 44..=46——红修 #2 律:
/// 新侧表尾挂必新 AE 下标族,set_autoexp 选择块与 g31_apply_autoexp 调用点
/// 双接线,transp guard 先于 nm/realism〔transp 挂载序最尾〕）。
const G31_U_AE_STATE_TEXNRM_TRANSP: u32 = 36;
const G31_U_AE_PARAMS_TEXNRM_TRANSP: u32 = 37;
const G31_U_AE_PARTIALS_TEXNRM_TRANSP: u32 = 38;
const G31_U_AE_STATE_TEXNRM_BLOOM_TRANSP: u32 = 44;
const G31_U_AE_PARAMS_TEXNRM_BLOOM_TRANSP: u32 = 45;
const G31_U_AE_PARTIALS_TEXNRM_BLOOM_TRANSP: u32 = 46;
/// G37 W2 ris_nee:A2 追加资源下标(lamp_tbl 尾挂后 AE 三件再顺延 +1:
/// tex+nrm+…+transp(占位)+ris 37..=39 / ×bloom 45..=47——红修 #2 律:
/// 新侧表尾挂必新 AE 下标族,双接线 + assert 连号,ris guard 先于 transp)。
const G31_U_AE_STATE_TEXNRM_RIS: u32 = 37;
const G31_U_AE_PARAMS_TEXNRM_RIS: u32 = 38;
const G31_U_AE_PARTIALS_TEXNRM_RIS: u32 = 39;
const G31_U_AE_STATE_TEXNRM_BLOOM_RIS: u32 = 45;
const G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS: u32 = 46;
const G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS: u32 = 47;
/// A2 reduce/state 屏障计划（tex 单臂变体）。
const G31_U_PLAN_AE_REDUCE_TEX: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEX, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEX, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEX: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEX, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEX, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEX, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// A2 reduce/state 屏障计划（tex+nrm 合流变体）。
const G31_U_PLAN_AE_REDUCE_TEXNRM: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// A2 reduce/state 屏障计划（tex×bloom 组合变体——reduce 读 composite 合成
/// 输出,encode in_color 同源静态绑定）。
const G31_U_PLAN_AE_REDUCE_TEX_BLOOM: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEX_BLOOM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEX_BLOOM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEX_BLOOM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// A2 reduce/state 屏障计划（tex×nrm×bloom 合流变体）。
const G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_BLOOM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// Phase F：A2 reduce/state 屏障计划（tex+nrm+emissive 合流变体）。
const G31_U_PLAN_AE_REDUCE_TEXNRM_EM: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_EM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_EM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_EM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_EM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_EM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_EM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// Phase F：A2 reduce/state 屏障计划（tex+nrm+bloom+emissive 合流变体）。
const G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_EM: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_EM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_EM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_EM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_EM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_EM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_BLOOM_EM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// day_0829 realism：A2 reduce/state 屏障计划（tex+nrm+tri_base 合流变体）。
const G31_U_PLAN_AE_REDUCE_TEXNRM_REAL: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_REAL, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_REAL, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_REAL: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_REAL, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_REAL, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_REAL, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// day_0829 realism：A2 reduce/state 屏障计划（tex+nrm+bloom+tri_base 合流变体）。
const G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_REAL: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_REAL, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_REAL, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_REAL: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_REAL, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_REAL, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_BLOOM_REAL, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// day_0829 臂④：A2 reduce/state 屏障计划（+trinm/tri_tan 形态）。
const G31_U_PLAN_AE_REDUCE_TEXNRM_NM: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_NM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_NM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_NM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_NM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_NM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_NM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_NM: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_NM, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_NM, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_NM: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_NM, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_NM, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_BLOOM_NM, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// G37 W2 transparency：A2 reduce/state 屏障计划（transp 两形态）。
const G31_U_PLAN_AE_REDUCE_TEXNRM_TRANSP: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_TRANSP, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_TRANSP, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_TRANSP: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_TRANSP, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_TRANSP, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_TRANSP, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_TRANSP: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_TRANSP, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_TRANSP, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_TRANSP: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_TRANSP, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_TRANSP, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_BLOOM_TRANSP, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
/// G37 W2 ris_nee：A2 reduce/state 屏障计划（ris 两形态）。
const G31_U_PLAN_AE_REDUCE_TEXNRM_RIS: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_RIS, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_RIS: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_RIS, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_RIS: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
];
const G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_RIS: &[(u32, TargetState)] = &[
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_STATE_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
];

// ---------------------------------------------------------------------------
// G37 W3 fg_combo 合入：FG × --quality full 组合面静态下标族（两点式闭集第二
// 点;fg_combo/REPORT.md §3.2——RIS/NEE 合入后按现文件真实终态重推：full 终态
// 变体 = TEXNRM_BLOOM_RIS（45 资源,lamp_tbl 尾挂 44）+ AE 三件 45..=47 ⇒
// FG_FULL 族从 48 起连号〔报告制作时终态为 44+AE=47 起,已漂移登记〕,
// g31_apply_fg_full 施加期 assert 资源计数钉死〔AE 红修 #2 律〕）。
// comp parity 对 = (G31_U_BLOOM_COMP_OUT=31, G31_U_BLOOM_COMP_HIST_FULL=48)
// 逐帧轮换：composite 写 comp[p]、encode 读 comp[p]、AE reduce 读 comp[p]、
// FG 读 (comp[1−p], comp[p])——真实帧同 kernel 同输入仅输出缓冲对象轮换 ⇒
// 真渲帧 BGRA 与 fg-off（单 comp 静态）位级一致,digest_seq 不污染门结构上
// 维持。fg-on × full 面才构造,fg off / base fg 生产路径 0-byte。
// ---------------------------------------------------------------------------
const G31_U_BLOOM_COMP_HIST_FULL: u32 = 48;
const G31_U_MVN_PARAMS_FULL: u32 = 49;
const G31_U_MVN_FULL: u32 = 50;
const G31_U_FG1_PARAMS_FULL: u32 = 51;
const G31_U_FG1_OUT_FULL: u32 = 52;
const G31_U_FG1_BGRA_FULL: u32 = 53;
const G31_U_FG2_PARAMS_FULL: u32 = 54;
const G31_U_FG2_OUT_FULL: u32 = 55;
const G31_U_FG2_BGRA_FULL: u32 = 56;

/// G37 W3 fg_combo：composite 屏障计划超集（既有 COMPOSITE 计划 + comp 伙伴
/// 槽——composite 写 comp[p] 逐帧轮换,&'static 计划须覆双 parity 并集,
/// U_PLAN_FG1 双 OUT_COLOR 同律）。
const G31_U_PLAN_BLOOM_COMPOSITE_FULL_FG: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G31_U_BLOOM_PONG, TargetState::StorageReadWrite),
    (G31_U_BLOOM_COMP_PARAMS, TargetState::StorageReadWrite),
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_BLOOM_COMP_HIST_FULL, TargetState::StorageReadWrite),
];
/// G37 W3 fg_combo：encode 屏障计划超集（ENCODE_BLOOM + comp 伙伴槽——encode
/// 读 comp[p] 逐帧轮换;报告 §3.2 计划族六件之外按同一双 parity 并集律补齐,
/// 偏差登记 MERGE_REPORT）。
const G31_U_PLAN_ENCODE_BLOOM_FULL_FG: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_BLOOM_COMP_HIST_FULL, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
    (G31_U_ENC_OUT, TargetState::StorageReadWrite),
];
/// G37 W3 fg_combo：AE reduce 屏障计划超集（_TEXNRM_BLOOM_RIS 计划 + comp
/// 伙伴槽——reduce 读 comp[p] 逐帧轮换,同上补齐律）。
const G31_U_PLAN_AE_REDUCE_FULL_FG: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_BLOOM_COMP_HIST_FULL, TargetState::StorageReadWrite),
    (G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
    (G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS, TargetState::StorageReadWrite),
];
/// G37 W3 fg_combo：MV 取反 pass 屏障计划（FULL 族;G31_U_PLAN_MVN 同构）。
const G31_U_PLAN_MVN_FULL: &[(u32, TargetState)] = &[
    (U_MV_OUT, TargetState::StorageReadWrite),
    (G31_U_MVN_PARAMS_FULL, TargetState::StorageReadWrite),
    (G31_U_MVN_FULL, TargetState::StorageReadWrite),
];
/// G37 W3 fg_combo：fg1 pass 屏障计划（读 comp parity 对而非 U_OUT_COLOR 对
/// ——FG 插值 post-bloom 合成帧,与 DLSS-G/FSR-FG「对最终合成帧插值」同形态）。
const G31_U_PLAN_FG1_FULL: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_BLOOM_COMP_HIST_FULL, TargetState::StorageReadWrite),
    (G31_U_MVN_FULL, TargetState::StorageReadWrite),
    (G31_U_FG1_PARAMS_FULL, TargetState::StorageReadWrite),
    (G31_U_FG1_OUT_FULL, TargetState::StorageReadWrite),
];
/// G37 W3 fg_combo：fg1 编码 pass 屏障计划（FULL 族;ENC_FG1 同构）。
const G31_U_PLAN_ENC_FG1_FULL: &[(u32, TargetState)] = &[
    (G31_U_FG1_OUT_FULL, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
    (G31_U_FG1_BGRA_FULL, TargetState::StorageReadWrite),
];
/// G37 W3 fg_combo：fg2 pass 屏障计划（x3;comp parity 对同律）。
const G31_U_PLAN_FG2_FULL: &[(u32, TargetState)] = &[
    (G31_U_BLOOM_COMP_OUT, TargetState::StorageReadWrite),
    (G31_U_BLOOM_COMP_HIST_FULL, TargetState::StorageReadWrite),
    (G31_U_MVN_FULL, TargetState::StorageReadWrite),
    (G31_U_FG2_PARAMS_FULL, TargetState::StorageReadWrite),
    (G31_U_FG2_OUT_FULL, TargetState::StorageReadWrite),
];
/// G37 W3 fg_combo：fg2 编码 pass 屏障计划（x3;FULL 族）。
const G31_U_PLAN_ENC_FG2_FULL: &[(u32, TargetState)] = &[
    (G31_U_FG2_OUT_FULL, TargetState::StorageReadWrite),
    (G31_U_ENC_PARAMS, TargetState::StorageReadWrite),
    (G31_U_FG2_BGRA_FULL, TargetState::StorageReadWrite),
];

/// A2 参数面打包（与 g31_autoexp_{reduce,state}.rx 参数面逐字同源;8 f32——
/// [0]=pixel_count [1]=key [2]=rate [3]=min_gain [4]=max_gain,reserved 恒 0）。
fn g31_ae_pack_params(pixel_count: u32, key: f32, rate: f32, gmin: f32, gmax: f32) -> Vec<f32> {
    let mut v = vec![pixel_count as f32, key, rate, gmin, gmax];
    v.resize(8, 0.0);
    v
}

/// A2 描述组输入面（era 常量,extent 联动 resize 随车道重建;off 时不构造）。
struct G31AutoExpAssets<'x> {
    spv_reduce: &'x [u8],
    spv_state: &'x [u8],
    params_bytes: &'x [u8],
}

/// A2 描述组变换（D3 bloom「encode 摘出重挂」同模式,施加于变体既有产物）：
/// 尾部追加 state/params/partials 三资源（下标 = 变体族静态常量,调用方
/// 给定并断言连号）→ encode pass/屏障摘出 → 插 reduce（读 encode 同源
/// in_color——非 bloom 面初始绑定 parity 0 逐帧 override,bloom 面 comp_out
/// 静态）→ 插 state（串行归约 + EMA + 增益写 enc_params[133],绑定全静态）
/// → encode 重挂（绑定面 0-byte——增益经 params[133] 消费,零新增绑定）。
/// 既有资源/pass/屏障/readback 面 0-byte;off 面不调用。
#[allow(clippy::too_many_arguments)]
fn g31_apply_autoexp<'x>(
    d: &mut G31Descs<'x>,
    ae: &G31AutoExpAssets<'x>,
    idx_state: u32,
    idx_params: u32,
    idx_partials: u32,
    reduce_in: u32,
    plan_reduce: &'static [(u32, TargetState)],
    plan_state: &'static [(u32, TargetState)],
) {
    // G37 W1:debug_assert 升 assert(day_0829 HANDOVER §G.2 兑现)——红修 #2
    // (AE 下标族错位)release 下未被拦截的根因;车道创建期一次性,常数代价。
    assert_eq!(
        d.resources.len(),
        idx_state as usize,
        "g31_apply_autoexp: AE state 下标须 == 当前资源数(变体族下标错配即红修 #2 症状)"
    );
    assert_eq!(idx_params, idx_state + 1, "g31_apply_autoexp: AE 三件须连号");
    assert_eq!(idx_partials, idx_state + 2, "g31_apply_autoexp: AE 三件须连号");
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    // state（16B 初值全 0;device 跨帧持久——era 重建重置再适应）。
    d.resources.push(ResourceDesc::Buffer(BufferDesc {
        size: G31_AE_STATE_INIT.len() as u64,
        usage: storage,
        data: Some(G31_AE_STATE_INIT),
        device_local: true,
    }));
    // params（8 f32 era 常量）。
    d.resources.push(ResourceDesc::Buffer(BufferDesc {
        size: ae.params_bytes.len() as u64,
        usage: storage,
        data: Some(ae.params_bytes),
        device_local: true,
    }));
    // partials（512 f32;reduce 每帧全 512 槽覆写,无需初值）。
    d.resources.push(ResourceDesc::Buffer(BufferDesc {
        size: 512 * 4,
        usage: storage,
        data: None,
        device_local: true,
    }));
    // encode 摘出（变体既有末位 pass 恒 encode;fg/hzb 组合 CLI 已裁不达——
    // 防御性复核名字面,重挂模式前提破坏即红）。
    let enc_pass = d
        .passes
        .pop()
        .unwrap_or_else(|| fail("A2: 变体 descs 空 pass 面"));
    let enc_barrier = d
        .barriers
        .pop()
        .unwrap_or_else(|| fail("A2: 变体 descs 空屏障面"));
    if let Pass::Compute(cp) = &enc_pass {
        if cp.name != "g31_display_encode" {
            fail(&format!(
                "A2: 变体末位 pass {} ≠ g31_display_encode（encode 摘出重挂前提破坏）",
                cp.name
            ));
        }
    }
    d.passes.push(Pass::Compute(ComputePass {
        name: "g31_autoexp_reduce",
        spirv: ae.spv_reduce,
        entry: None,
        dispatch: DispatchSpec::Direct([1, 1, 1]),
        bindings: Bindings {
            storage_buffers: vec![reduce_in, idx_params, idx_partials],
            ..Bindings::default()
        },
    }));
    d.barriers.push(plan_reduce);
    d.passes.push(Pass::Compute(ComputePass {
        name: "g31_autoexp_state",
        spirv: ae.spv_state,
        entry: None,
        dispatch: DispatchSpec::Direct([1, 1, 1]),
        bindings: Bindings {
            storage_buffers: vec![idx_partials, idx_params, idx_state, G31_U_ENC_PARAMS],
            ..Bindings::default()
        },
    }));
    d.barriers.push(plan_state);
    d.passes.push(enc_pass);
    d.barriers.push(enc_barrier);
}

/// G37 W3 fg_combo 合入：FG × --quality full 组合变换（A2「摘出重挂」同模式
/// 家族,施加于 full 终态变体〔TEXNRM_BLOOM_RIS + AE〕产物;**必在
/// g31_apply_autoexp 之后**——AE 摘出断言前提「变体末位 pass = encode」由
/// 施加序保证,本函数为纯尾挂零摘出）：
/// ① 追加 comp parity 伙伴缓冲 comp[1]（48,opc×12——composite/encode/AE
///   reduce/FG 四处 prepare_update 逐帧 override 消费;真实帧同 kernel 同输入
///   仅输出缓冲对象轮换 ⇒ fg on/off 真渲帧 digest_seq 位级一致不污染门结构上
///   维持）+ mvn/FG1 五件（x3 再挂 FG2 三件,FULL 族 48..=56 连号）;
/// ② comp 触碰面三 pass（composite/AE reduce/encode）屏障计划换双 parity 槽
///   超集（绑定逐帧 override,&'static 静态计划须覆并集——按 pass 名定位防
///   下标漂移）;
/// ③ 尾挂 mvn/fg1/enc_fg1（x3 + fg2/enc_fg2）pass——FG 初始绑定 parity 0 形
///   （prev=comp[1], cur=comp[0]）,enc_fg 复用主 encode SPV +
///   `G31_U_ENC_PARAMS` 同 buffer（AE 增益经 params[133] 生成帧同读继承——
///   AE 慢收敛 EMA,帧间增益差可忽略,如实登记不适配）;
/// ④ readback 追加 comp[0]/comp[1] + FG_BGRA/FG_OUT/MVN（布局 =
///   `G31FgLayout::of_full`,comp 对仅 probe 帧入子集）。
/// 零新 kernel;既有资源/pass/readback 面 0-byte;fg off / base fg 面不调用。
fn g31_apply_fg_full<'x>(
    d: &mut G31Descs<'x>,
    fga: &G31FgAssets<'x>,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    opc: u64,
) {
    // W1 律：施加期 assert 钉死（红修 #2〔AE 下标族错位〕同保护网——FULL 族
    // 按 TEXNRM_BLOOM_RIS(45) + AE 三件(45..=47) 终态推得 48 起）。
    assert_eq!(
        d.resources.len(),
        G31_U_BLOOM_COMP_HIST_FULL as usize,
        "g31_apply_fg_full: FG_FULL 下标族须 == TEXNRM_BLOOM_RIS+AE 终态资源数（错位即红修 #2 症状）"
    );
    // 防御性复核：末位 pass 恒 encode（AE 变换后不变量;尾挂序前提破坏即红）。
    match d.passes.last() {
        Some(Pass::Compute(cp)) if cp.name == "g31_display_encode" => {}
        _ => fail("g31_apply_fg_full: 变体末位 pass ≠ g31_display_encode（施加序前提破坏——须在 g31_apply_autoexp 之后）"),
    }
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: true,
        })
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    // 48 = comp parity 伙伴缓冲（opc×12;composite 奇 parity 帧写,首帧未定义
    // 内容零消费——gen_active 首帧 false,与 base 面 U_OUT_COLOR 首帧同律）。
    d.resources.push(buf(opc * 12));
    // 49/50 = MV 取反 glue 参数（静态）/ 取反 MV 输出（2 f32/px）。
    d.resources.push(init(fga.mvn_params_bytes));
    d.resources.push(buf(opc * 8));
    // 51..=53 = FG1 参数（静态）/ FG1 输出 f32 / FG1 BGRA8。
    d.resources.push(init(fga.params1_bytes));
    d.resources.push(buf(opc * 12));
    d.resources.push(buf(opc * 4));
    // comp parity 触碰面三 pass 屏障计划换双槽超集（full+AE 十一 pass 图
    // composite=7/reduce=8/encode=10;按名定位不按下标,防并行合入 pass 图漂移）。
    for (k, p) in d.passes.iter().enumerate() {
        if let Pass::Compute(cp) = p {
            match cp.name {
                "g31_bloom_composite" => d.barriers[k] = G31_U_PLAN_BLOOM_COMPOSITE_FULL_FG,
                "g31_autoexp_reduce" => d.barriers[k] = G31_U_PLAN_AE_REDUCE_FULL_FG,
                "g31_display_encode" => d.barriers[k] = G31_U_PLAN_ENCODE_BLOOM_FULL_FG,
                _ => {}
            }
        }
    }
    d.passes.push(Pass::Compute(ComputePass {
        name: "g31_mv_negate",
        spirv: fga.mvn_spv_bytes,
        entry: None,
        dispatch: DispatchSpec::Direct(fga.mvn_dispatch),
        bindings: Bindings {
            storage_buffers: vec![U_MV_OUT, G31_U_MVN_PARAMS_FULL, G31_U_MVN_FULL],
            ..Bindings::default()
        },
    }));
    d.barriers.push(G31_U_PLAN_MVN_FULL);
    // fg1 初始绑定 parity 0 形（prev=comp[1], cur=comp[0]——base 面
    // U_OUT_COLOR[1]/[0] 同律;逐帧 override 换 (comp[1−p], comp[p])）。
    d.passes.push(Pass::Compute(ComputePass {
        name: "g26_framegen_fg1",
        spirv: fga.spv_bytes,
        entry: None,
        dispatch: DispatchSpec::Direct(fga.dispatch),
        bindings: Bindings {
            storage_buffers: vec![
                G31_U_BLOOM_COMP_HIST_FULL,
                G31_U_BLOOM_COMP_OUT,
                G31_U_MVN_FULL,
                G31_U_FG1_PARAMS_FULL,
                G31_U_FG1_OUT_FULL,
            ],
            ..Bindings::default()
        },
    }));
    d.barriers.push(G31_U_PLAN_FG1_FULL);
    d.passes.push(Pass::Compute(ComputePass {
        name: "g31_display_encode_fg1",
        spirv: enc_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(enc_dispatch),
        bindings: Bindings {
            storage_buffers: vec![G31_U_FG1_OUT_FULL, G31_U_ENC_PARAMS, G31_U_FG1_BGRA_FULL],
            ..Bindings::default()
        },
    }));
    d.barriers.push(G31_U_PLAN_ENC_FG1_FULL);
    if fga.mode == G31Fg::X3 {
        // 54..=56 = FG2 参数/输出 f32/BGRA8（x3 第二插入帧）。
        d.resources.push(init(fga.params2_bytes));
        d.resources.push(buf(opc * 12));
        d.resources.push(buf(opc * 4));
        d.passes.push(Pass::Compute(ComputePass {
            name: "g26_framegen_fg2",
            spirv: fga.spv_bytes,
            entry: None,
            dispatch: DispatchSpec::Direct(fga.dispatch),
            bindings: Bindings {
                storage_buffers: vec![
                    G31_U_BLOOM_COMP_HIST_FULL,
                    G31_U_BLOOM_COMP_OUT,
                    G31_U_MVN_FULL,
                    G31_U_FG2_PARAMS_FULL,
                    G31_U_FG2_OUT_FULL,
                ],
                ..Bindings::default()
            },
        }));
        d.barriers.push(G31_U_PLAN_FG2_FULL);
        d.passes.push(Pass::Compute(ComputePass {
            name: "g31_display_encode_fg2",
            spirv: enc_spv,
            entry: None,
            dispatch: DispatchSpec::Direct(enc_dispatch),
            bindings: Bindings {
                storage_buffers: vec![G31_U_FG2_OUT_FULL, G31_U_ENC_PARAMS, G31_U_FG2_BGRA_FULL],
                ..Bindings::default()
            },
        }));
        d.barriers.push(G31_U_PLAN_ENC_FG2_FULL);
    }
    // readback 入列序（G31FgLayout::of_full 下标同源）：comp[0]/comp[1]（probe
    // 帧才入子集——host interpolate 复算的 prev/cur 换 comp 对回读）→ FG_BGRA
    // 全部 → FG_OUT 全部 → MVN。
    d.readbacks.push(Readback::Buffer {
        res: G31_U_BLOOM_COMP_OUT,
        offset: 0,
        size: opc * 12,
    });
    d.readbacks.push(Readback::Buffer {
        res: G31_U_BLOOM_COMP_HIST_FULL,
        offset: 0,
        size: opc * 12,
    });
    d.readbacks.push(Readback::Buffer {
        res: G31_U_FG1_BGRA_FULL,
        offset: 0,
        size: opc * 4,
    });
    if fga.mode == G31Fg::X3 {
        d.readbacks.push(Readback::Buffer {
            res: G31_U_FG2_BGRA_FULL,
            offset: 0,
            size: opc * 4,
        });
    }
    d.readbacks.push(Readback::Buffer {
        res: G31_U_FG1_OUT_FULL,
        offset: 0,
        size: opc * 12,
    });
    if fga.mode == G31Fg::X3 {
        d.readbacks.push(Readback::Buffer {
            res: G31_U_FG2_OUT_FULL,
            offset: 0,
            size: opc * 12,
        });
    }
    d.readbacks.push(Readback::Buffer {
        res: G31_U_MVN_FULL,
        offset: 0,
        size: opc * 8,
    });
}

/// G31 三态 skip（g14 `dev_env_or_fail` 同语义;schema 字面独立——G31 口径不混 G14）。
fn g31_dev_env_or_fail(what: &str, err: &str) -> ! {
    if require_real() {
        fail(&format!(
            "{what} 不可用（RURIX_REQUIRE_REAL=1，禁 mock 充真跑）: {err}"
        ));
    }
    println!(
        "{{\"schema\":\"rurix.g31.window_present.skip.v1\",\"state\":\"skipped_dev_env\",\"what\":{},\"reason\":{}}}",
        jstr(what),
        jstr(err)
    );
    std::process::exit(0)
}

/// post-warmup 测量面稳态统计（mean/sd/cv/min/max;程序产禁手写阈,G14.3 同式）。
fn g31_stats(v: &[f64]) -> (f64, f64, f64, f64, f64) {
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / n;
    let sd = var.sqrt();
    let min = v.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (mean, sd, sd / mean, min, max)
}

/// C7:percentile（升序 + 线性插值;n=1 直返,调用方保证非空）。
fn g31_pct(sorted: &[f64], q: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = q / 100.0 * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// C7:段统计组（mean/p50/p99/min/max,均 ms;--profile-json 各段共用）。
fn g31_seg_stats(v: &[f64]) -> (f64, f64, f64, f64, f64) {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mean = s.iter().sum::<f64>() / s.len() as f64;
    (
        mean,
        g31_pct(&s, 50.0),
        g31_pct(&s, 99.0),
        s[0],
        s[s.len() - 1],
    )
}

/// C7:--profile-json 组装（机器可读逐 pass 分解 + CPU 段 + 帧统计 mean/p50/p99 +
/// 恒等式字段 + debug label 态;恒等式容差字面与 docs/renderer/profiling_debugging.md
/// 及 ci/g31_profiling_smoke.py 同一事实源——改动三面同步）。返回 JSON 文本。
#[allow(clippy::too_many_arguments)]
fn g31_profile_json(
    frames_rec: &[G31ProfileFrame],
    scene_id: &str,
    tier: u32,
    out_w: u32,
    out_h: u32,
    in_w: u32,
    in_h: u32,
    warmup: u32,
    headless: bool,
    debug_labels_active: bool,
    render_digest: &str,
    t_asm: std::time::Instant,
) -> Result<String, String> {
    if frames_rec.is_empty() {
        return Err("--profile-json: post-warmup 测量帧为空（--frames ≥ 1 才产 profile）".into());
    }
    // 逐帧 pass 名序一致（同车道同图;漂移 = 内部不一致 fail-closed 不冒充）。
    let canon: Vec<String> = frames_rec[0].passes.iter().map(|p| p.0.clone()).collect();
    for (k, f) in frames_rec.iter().enumerate() {
        let same = f.passes.len() == canon.len()
            && f
                .passes
                .iter()
                .zip(canon.iter())
                .all(|((n, _), c)| n == c);
        if !same {
            return Err(format!(
                "--profile-json: 帧 {k} pass 名序漂移（车道内部不一致）"
            ));
        }
    }
    let seg_json = |name: &str, unit: &str, series: &[f64]| -> String {
        let (mean, p50, p99, mn, mx) = g31_seg_stats(series);
        format!(
            "{{\"name\":{},\"unit\":{},\"mean_ms\":{mean:.6},\"p50_ms\":{p50:.6},\"p99_ms\":{p99:.6},\"min_ms\":{mn:.6},\"max_ms\":{mx:.6}}}",
            jstr(name),
            jstr(unit)
        )
    };
    let series = |pick: &dyn Fn(&G31ProfileFrame) -> f64| -> Vec<f64> {
        frames_rec.iter().map(|f| pick(f)).collect()
    };
    let mut pj = String::new();
    pj.push('{');
    pj.push_str("\"schema\":\"rurix.g31.profile_output.v1\",");
    pj.push_str("\"bin\":\"g31_window_present\",");
    pj.push_str(&format!(
        "\"scene\":{},\"tier\":{tier},\"backend\":\"tsr_device\",",
        jstr(scene_id)
    ));
    pj.push_str(&format!(
        "\"frames_measured\":{},\"warmup\":{warmup},",
        frames_rec.len()
    ));
    pj.push_str(&format!(
        "\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},\"internal_resolution\":{{\"w\":{in_w},\"h\":{in_h}}},\"headless\":{headless},"
    ));
    pj.push_str(&format!("\"render_digest\":{},", jstr(render_digest)));
    // ── 逐 pass GPU 段（telemetry 声明序）──
    pj.push_str("\"gpu_passes\":[");
    for (i, name) in canon.iter().enumerate() {
        if i > 0 {
            pj.push(',');
        }
        let s: Vec<f64> = frames_rec.iter().map(|f| f.passes[i].1).collect();
        pj.push_str(&seg_json(name, "gpu_timestamp_ms", &s));
    }
    pj.push_str("],");
    // ── CPU 段（telemetry 三分项 + host 回读转换）──
    pj.push_str("\"cpu_segments\":[");
    pj.push_str(&seg_json(
        "cpu_record",
        "host_wall_ms",
        &series(&|f| f.cpu_record_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json(
        "cpu_submit",
        "host_wall_ms",
        &series(&|f| f.cpu_submit_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json(
        "cpu_fence_wait",
        "host_wall_ms",
        &series(&|f| f.cpu_fence_wait_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json(
        "readback_convert",
        "host_wall_ms",
        &series(&|f| f.readback_convert_ms),
    ));
    pj.push_str("],");
    // ── 帧段（host 墙钟;render 含 BGRA8 强制回读,present headless 恒 0）──
    pj.push_str("\"frame_segments\":[");
    pj.push_str(&seg_json(
        "render_wall",
        "host_wall_ms",
        &series(&|f| f.render_wall_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json(
        "present_wall",
        "host_wall_ms",
        &series(&|f| f.present_wall_ms),
    ));
    pj.push(',');
    pj.push_str(&seg_json("digest", "host_wall_ms", &series(&|f| f.digest_ms)));
    pj.push_str("],");
    // ── 恒等式字段（分解和≈帧墙钟;容差字面 = 门/文档同一事实源）──
    let gpu_sum = series(&|f| f.passes.iter().map(|p| p.1).sum::<f64>());
    let cpu_sum = series(&|f| {
        f.cpu_record_ms + f.cpu_submit_ms + f.cpu_fence_wait_ms + f.readback_convert_ms
    });
    let rw = series(&|f| f.render_wall_ms);
    let residual: Vec<f64> = frames_rec
        .iter()
        .map(|f| {
            f.render_wall_ms
                - (f.cpu_record_ms + f.cpu_submit_ms + f.cpu_fence_wait_ms + f.readback_convert_ms)
        })
        .collect();
    let (gs_mean, _, gs_p99, _, _) = g31_seg_stats(&gpu_sum);
    let (cs_mean, _, _, _, _) = g31_seg_stats(&cpu_sum);
    let (rw_mean, _, _, _, _) = g31_seg_stats(&rw);
    let (res_mean, _, res_p99, res_min, res_max) = g31_seg_stats(&residual);
    pj.push_str(&format!(
        "\"identity\":{{\"gpu_sum_mean_ms\":{gs_mean:.6},\"gpu_sum_p99_ms\":{gs_p99:.6},\"render_wall_mean_ms\":{rw_mean:.6},\"cpu_seg_sum_mean_ms\":{cs_mean:.6},\"host_residual_mean_ms\":{res_mean:.6},\"host_residual_p99_ms\":{res_p99:.6},\"host_residual_min_ms\":{res_min:.6},\"host_residual_max_ms\":{res_max:.6},\"gpu_sum_le_render_wall_tol_ms\":0.10,\"host_residual_tol_ms\":2.00,\"rule\":\"gpu_sum_mean<=render_wall_mean+0.10 && -0.10<=host_residual_mean<=2.00\"}},"
    ));
    // ── debug label 态（VK_EXT_debug_utils 逐 pass 标注面;absent = 零开销跳过）──
    pj.push_str(&format!(
        "\"debug_labels\":{{\"active\":{debug_labels_active},\"annotated_pass_count\":{},\"extension\":\"VK_EXT_debug_utils\",\"note\":{}}},",
        if debug_labels_active { canon.len() } else { 0 },
        jstr("vkCmdBegin/EndDebugUtilsLabelEXT 逐 pass 标注（pass 名）;扩展 absent = 零开销跳过 fail-silent")
    ));
    // profiler 开销如实登记（组装段实测——本行前的全部统计/拼装;写盘段在其后）。
    let asm_ms = t_asm.elapsed().as_secs_f64() * 1000.0;
    pj.push_str(&format!("\"profiler_overhead\":{{\"assembly_ms\":{asm_ms:.6},\"note\":{}}},", jstr("profiler 开销 = host 簿记（逐帧 Vec 推送）+ 本 JSON 组装段（assembly_ms 实测;写盘段在其后）;渲染语义零变更——digest 锚 on/off 位级一致由 ci/g31_profiling_smoke.py 门检")));
    pj.push_str(&format!("\"notes\":{}", jstr("gpu_passes = DeviceFrameTelemetry 逐 pass GPU timestamp（声明序;×timestampPeriod 驱动实采）;cpu_segments = telemetry cpu_record/submit/fence_wait 三分项 + host readback_convert;frame_segments = host 墙钟（render_wall 含 BGRA8 8.3MB 强制回读税,present_wall headless 恒 0,digest 税单列不入渲染口径）;identity = 分解和≈帧墙钟恒等式（gpu_sum 为逐 pass GPU 合计,host_residual = render_wall − cpu 四段和;容差字段同 ci/g31_profiling_smoke.py）;默认关,开启零渲染语义变更")));
    pj.push('}');
    Ok(pj)
}

/// 文件 sha256(`sha256:` 前缀;provenance 登记面)。
fn g31_file_sha(path: &str) -> Result<String, String> {
    let b = std::fs::read(path).map_err(|e| format!("读 {path}: {e}"))?;
    Ok(format!("sha256:{}", sha256_hex(&b)))
}

/// JSON 数值字段精确相等（f64 位级;转引核验面——同字面十进制的 f64 解析位级同值）。
fn g31_json_num_eq(name: &str, a: &Json, b: &Json) -> Result<(), String> {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) if x == y => Ok(()),
        (Some(x), Some(y)) => Err(format!("转引不等 {name}: {x} ≠ {y}")),
        _ => Err(format!("转引字段 {name} 非数值")),
    }
}

/// 数值数组逐元素精确相等。
fn g31_json_vec_eq(name: &str, a: &Json, b: &Json, n: usize) -> Result<(), String> {
    let (Some(av), Some(bv)) = (a.as_array(), b.as_array()) else {
        return Err(format!("转引字段 {name} 非数组"));
    };
    if av.len() != n || bv.len() != n {
        return Err(format!(
            "转引数组 {name} 长度 {} / {} ≠ {n}",
            av.len(),
            bv.len()
        ));
    }
    for (i, (x, y)) in av.iter().zip(bv.iter()).enumerate() {
        g31_json_num_eq(&format!("{name}[{i}]"), x, y)?;
    }
    Ok(())
}

/// G10 语料三件套 ↔ 生产契约 bistro-interior 场景行**转引一致性核验**（相机/点光/
/// emissive 逐字段精确相等;不等即 Err 翻红）。返回登记用 JSON 片段（paths + sha256 +
/// delta note）。
fn g31_g10_corpus_gate(scene_row: &Json, g10_dir: &str) -> Result<String, String> {
    let contract_path = format!("{g10_dir}/contract_params_bistro_interior.json");
    let camera_path = format!("{g10_dir}/camera_bistro_interior.json");
    let lighting_path = format!("{g10_dir}/lighting_bistro_interior.json");
    let contract_sha = g31_file_sha(&contract_path)?;
    let camera_sha = g31_file_sha(&camera_path)?;
    let lighting_sha = g31_file_sha(&lighting_path)?;
    let g10_contract = json_parse(
        &std::fs::read_to_string(&contract_path).map_err(|e| format!("读 {contract_path}: {e}"))?,
    )?;
    let g10_camera = json_parse(
        &std::fs::read_to_string(&camera_path).map_err(|e| format!("读 {camera_path}: {e}"))?,
    )?;
    let g10_lighting = json_parse(
        &std::fs::read_to_string(&lighting_path).map_err(|e| format!("读 {lighting_path}: {e}"))?,
    )?;

    let row_cam = scene_row.get("camera").ok_or("场景行缺 camera")?;
    let row_lig = scene_row.get("lighting").ok_or("场景行缺 lighting")?;
    let c_cam = g10_contract.get("camera").ok_or("g10 契约缺 camera")?;

    // ── 相机:g10 契约 ↔ 生产行(position/quat/fov/near/far/resolution 逐字段)──
    g31_json_vec_eq(
        "camera.position",
        c_cam.get("position").unwrap(),
        row_cam.get("position").unwrap(),
        3,
    )?;
    g31_json_vec_eq(
        "camera.orientation_quat",
        c_cam.get("orientation_quat").unwrap(),
        row_cam.get("orientation_quat").unwrap(),
        4,
    )?;
    for k in ["fov_y_deg", "near", "far"] {
        g31_json_num_eq(
            &format!("camera.{k}"),
            c_cam.get(k).unwrap(),
            row_cam.get(k).unwrap(),
        )?;
    }
    g31_json_num_eq(
        "camera.resolution.w",
        c_cam
            .get("resolution")
            .and_then(|r| r.get("w"))
            .ok_or("g10 契约缺 resolution.w")?,
        row_cam
            .get("resolution")
            .and_then(|r| r.get("w"))
            .ok_or("生产行缺 resolution.w")?,
    )?;
    g31_json_num_eq(
        "camera.resolution.h",
        c_cam
            .get("resolution")
            .and_then(|r| r.get("h"))
            .ok_or("g10 契约缺 resolution.h")?,
        row_cam
            .get("resolution")
            .and_then(|r| r.get("h"))
            .ok_or("生产行缺 resolution.h")?,
    )?;
    // camera_*.json(eye/target/up 形式):eye == position、fov/resolution 一致。
    g31_json_vec_eq(
        "camera_file.eye",
        g10_camera.get("eye").ok_or("camera 文件缺 eye")?,
        row_cam.get("position").unwrap(),
        3,
    )?;
    g31_json_num_eq(
        "camera_file.fov_y_deg",
        g10_camera
            .get("fov_y_deg")
            .ok_or("camera 文件缺 fov_y_deg")?,
        row_cam.get("fov_y_deg").unwrap(),
    )?;
    let cam_res = g10_camera
        .get("resolution")
        .and_then(|v| v.as_array())
        .ok_or("camera 文件缺 resolution")?;
    g31_json_num_eq(
        "camera_file.resolution[0]",
        cam_res.first().ok_or("camera resolution 空")?,
        row_cam.get("resolution").and_then(|r| r.get("w")).unwrap(),
    )?;
    g31_json_num_eq(
        "camera_file.resolution[1]",
        cam_res.get(1).ok_or("camera resolution 缺 h")?,
        row_cam.get("resolution").and_then(|r| r.get("h")).unwrap(),
    )?;

    // ── 点光:g10 lighting point_lights ↔ 生产行 point_lights(逐灯 position/color/
    //    intensity_cd 精确相等,顺序一致)──
    let g_points = g10_lighting
        .get("point_lights")
        .and_then(|v| v.as_array())
        .ok_or("g10 lighting 缺 point_lights")?;
    let r_points = row_lig
        .get("point_lights")
        .and_then(|v| v.as_array())
        .ok_or("生产行缺 point_lights")?;
    if g_points.len() != r_points.len() {
        return Err(format!(
            "点光数不等:g10 {} ≠ 生产行 {}",
            g_points.len(),
            r_points.len()
        ));
    }
    for (i, (g, r)) in g_points.iter().zip(r_points.iter()).enumerate() {
        g31_json_vec_eq(
            &format!("point_lights[{i}].position"),
            g.get("position").unwrap(),
            r.get("position").unwrap(),
            3,
        )?;
        g31_json_vec_eq(
            &format!("point_lights[{i}].color_linear_rgb"),
            g.get("color_linear_rgb").unwrap(),
            r.get("color_linear_rgb").unwrap(),
            3,
        )?;
        g31_json_num_eq(
            &format!("point_lights[{i}].intensity_cd"),
            g.get("intensity_cd").unwrap(),
            r.get("intensity_cd").unwrap(),
        )?;
    }

    // ── emissive:g10 emissive_surfaces ↔ 生产行 emissive_materials(按 material_index
    //    配对,le_linear_rgb / area_m2 精确相等)──
    let g_em = g10_lighting
        .get("emissive_surfaces")
        .and_then(|v| v.as_array())
        .ok_or("g10 lighting 缺 emissive_surfaces")?;
    let r_em = row_lig
        .get("emissive_materials")
        .and_then(|v| v.as_array())
        .ok_or("生产行缺 emissive_materials")?;
    if g_em.len() != r_em.len() {
        return Err(format!(
            "emissive 数不等:g10 {} ≠ 生产行 {}",
            g_em.len(),
            r_em.len()
        ));
    }
    for (i, g) in g_em.iter().enumerate() {
        let mi = g
            .get("material_index")
            .and_then(|v| v.as_u64())
            .ok_or("g10 emissive 缺 material_index")?;
        let r = r_em
            .iter()
            .find(|r| r.get("material_index").and_then(|v| v.as_u64()) == Some(mi))
            .ok_or_else(|| format!("生产行缺 material_index={mi} 的 emissive"))?;
        g31_json_vec_eq(
            &format!("emissive[{mi}].le_linear_rgb"),
            g.get("le_linear_rgb").unwrap(),
            r.get("le_linear_rgb").unwrap(),
            3,
        )?;
        g31_json_num_eq(
            &format!("emissive[{mi}].area_m2"),
            g.get("area_m2").ok_or("g10 emissive 缺 area_m2")?,
            r.get("area_m2").ok_or("生产行 emissive 缺 area_m2")?,
        )?;
        let _ = i;
    }

    // ── delta 如实登记(G10 契约 sun/sky/ev100 ≠ 生产行;生产行为消费面,delta 不消费)──
    let g_sun = g10_contract
        .get("lighting")
        .and_then(|l| l.get("sun"))
        .and_then(|s| s.get("intensity_lux"))
        .and_then(|v| v.as_f64())
        .ok_or("g10 契约缺 sun.intensity_lux")?;
    let g_sky = g10_contract
        .get("lighting")
        .and_then(|l| l.get("sky"))
        .and_then(|s| s.get("intensity"))
        .and_then(|v| v.as_f64())
        .ok_or("g10 契约缺 sky.intensity")?;
    let g_ev = g10_contract
        .get("lighting")
        .and_then(|l| l.get("exposure"))
        .and_then(|s| s.get("ev100"))
        .and_then(|v| v.as_f64())
        .ok_or("g10 契约缺 exposure.ev100")?;
    let r_sun = row_lig
        .get("sun_intensity_lux")
        .and_then(|v| v.as_f64())
        .ok_or("生产行缺 sun_intensity_lux")?;
    let r_sky = row_lig
        .get("sky_intensity")
        .and_then(|v| v.as_f64())
        .ok_or("生产行缺 sky_intensity")?;
    let r_ev = scene_row
        .get("exposure")
        .and_then(|e| e.get("ev100"))
        .and_then(|v| v.as_f64())
        .ok_or("生产行缺 exposure.ev100")?;

    Ok(format!(
        "\"g10_contract\":{{\"path\":{},\"sha256\":{}}},\"g10_camera\":{{\"path\":{},\"sha256\":{}}},\"g10_lighting\":{{\"path\":{},\"sha256\":{}}},\"consistency\":\"pass\",\"delta_note\":{}",
        jstr(&contract_path.replace('\\', "/")),
        jstr(&contract_sha),
        jstr(&camera_path.replace('\\', "/")),
        jstr(&camera_sha),
        jstr(&lighting_path.replace('\\', "/")),
        jstr(&lighting_sha),
        jstr(&format!(
            "G10 契约 sun_intensity_lux={g_sun}/sky_intensity={g_sky}/ev100={g_ev} 与生产行 sun/sky={r_sun}/{r_sky}、ev100={r_ev} 差异如实登记不消费——生产内容模型锚(直接光 quad/point/emissive + ev100 标定)为消费面,差异面 = G10.5a 取景校准登记值"
        )),
    ))
}

// ---------------------------------------------------------------------------
// A3：五 pass 车道（统一四 pass + device 显示编码）描述组与车道状态机
// （共享体 0-byte——本区全部落本 bin;资源 0..=21/pass 0..=3/屏障/readback 0..=3
// 与 unified_lane_descs 逐字同源,追加资源 22/23 + pass 4 + 屏障计划 + readback 4）。
// ---------------------------------------------------------------------------

/// A3 五 pass 描述组（Vec 面——session 切片消费;`unified_lane_descs` 产物逐
/// 项克隆追加,既有项 0-byte）。A5:fg on 时追加 FG 资源/pass/屏障/readback
/// （fg off = 24 资源 + 5 pass + 5 readback 现状 0-byte）。
struct G31Descs<'x> {
    resources: Vec<ResourceDesc<'x>>,
    passes: Vec<Pass<'x>>,
    barriers: Vec<&'static [(u32, TargetState)]>,
    readbacks: Vec<Readback>,
}

/// A5 FG 描述组输入面（spv/参数字节 era 常量——t 逐 gen 不同故分两静态参数
/// 缓冲;fg off 时本面不构造）。
struct G31FgAssets<'x> {
    mode: G31Fg,
    spv_bytes: &'x [u8],
    mvn_spv_bytes: &'x [u8],
    dispatch: [u32; 3],
    mvn_dispatch: [u32; 3],
    mvn_params_bytes: &'x [u8],
    params1_bytes: &'x [u8],
    params2_bytes: &'x [u8],
}

/// A3 五 pass 描述组装配：统一四 pass（Mega）+ encode（pass4 读
/// `U_OUT_COLOR[parity]`——逐帧 binding_overrides 换 parity,初始绑定 = parity 0;
/// dispatch 自 encode SPV LocalSize 派生,SPV 单一事实源同律）。A5:fg on 追加
/// pass5 mvn（g31_mv_negate,MV 逐元素取反 glue,绑定静态）+ pass6 fg1
/// （g26_framegen 直通馈入 prev/cur/−mv/t,初始绑定 parity 0,逐帧 override
/// 换 parity）+ pass7 enc_fg1（绑定静态,复用 encode SPV/参数）+（x3）pass8
/// fg2 + pass9 enc_fg2;readback 追加 FG BGRA8 + FG f32（f32 路仅 probe 帧
/// 子集消费）。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
    fg: Option<G31FgAssets<'x>>,
) -> G31Descs<'x> {
    let (resources, passes, barriers, readbacks) = unified_lane_descs(assets, bits, iw, ih, ow, oh);
    let opc = (ow * oh) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let mut resources = resources.to_vec();
    debug_assert_eq!(resources.len(), U_RESOURCE_COUNT);
    // 22 = 编码参数（ACES 矩阵/样条 f32 块,创建期一次上传;逐帧曝光走 TSR
    // 参数面不经本 buffer——本面静态,resize 随车道重建）。
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: enc_params_bytes.len() as u64,
        usage: storage,
        data: Some(enc_params_bytes),
        device_local: true,
    }));
    // 23 = BGRA8 打包输出（1 u32/px;present 拷贝/digest 唯一消费面）。
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: opc * 4,
        usage: storage,
        data: None,
        device_local: true,
    }));
    debug_assert_eq!(resources.len(), G31_U_RESOURCE_COUNT);
    let mut passes = passes.to_vec();
    passes.push(Pass::Compute(ComputePass {
        name: "g31_display_encode",
        spirv: enc_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(enc_dispatch),
        bindings: Bindings {
            storage_buffers: vec![U_OUT_COLOR[0], G31_U_ENC_PARAMS, G31_U_ENC_OUT],
            ..Bindings::default()
        },
    }));
    let mut barriers = barriers.to_vec();
    barriers.push(G31_U_PLAN_ENCODE);
    let mut readbacks = readbacks.to_vec();
    readbacks.push(Readback::Buffer {
        res: G31_U_ENC_OUT,
        offset: 0,
        size: opc * 4,
    });
    if let Some(fga) = fg {
        debug_assert_ne!(fga.mode, G31Fg::Off);
        let buf = |size: u64| {
            ResourceDesc::Buffer(BufferDesc {
                size,
                usage: storage,
                data: None,
                device_local: true,
            })
        };
        let init = |bytes: &'x [u8]| {
            ResourceDesc::Buffer(BufferDesc {
                size: bytes.len() as u64,
                usage: storage,
                data: Some(bytes),
                device_local: true,
            })
        };
        // 24/25 = MV 取反 glue 参数（静态）/ 取反 MV 输出（2 f32/px）。
        resources.push(init(fga.mvn_params_bytes));
        resources.push(buf(opc * 8));
        // 26..=28 = FG1 参数（静态）/ FG1 输出 f32 / FG1 BGRA8。
        resources.push(init(fga.params1_bytes));
        resources.push(buf(opc * 12));
        resources.push(buf(opc * 4));
        passes.push(Pass::Compute(ComputePass {
            name: "g31_mv_negate",
            spirv: fga.mvn_spv_bytes,
            entry: None,
            dispatch: DispatchSpec::Direct(fga.mvn_dispatch),
            bindings: Bindings {
                storage_buffers: vec![U_MV_OUT, G31_U_MVN_PARAMS, G31_U_MVN],
                ..Bindings::default()
            },
        }));
        passes.push(Pass::Compute(ComputePass {
            name: "g26_framegen_fg1",
            spirv: fga.spv_bytes,
            entry: None,
            dispatch: DispatchSpec::Direct(fga.dispatch),
            bindings: Bindings {
                storage_buffers: vec![
                    U_OUT_COLOR[1],
                    U_OUT_COLOR[0],
                    G31_U_MVN,
                    G31_U_FG1_PARAMS,
                    G31_U_FG1_OUT,
                ],
                ..Bindings::default()
            },
        }));
        passes.push(Pass::Compute(ComputePass {
            name: "g31_display_encode_fg1",
            spirv: enc_spv,
            entry: None,
            dispatch: DispatchSpec::Direct(enc_dispatch),
            bindings: Bindings {
                storage_buffers: vec![G31_U_FG1_OUT, G31_U_ENC_PARAMS, G31_U_FG1_BGRA],
                ..Bindings::default()
            },
        }));
        barriers.push(G31_U_PLAN_MVN);
        barriers.push(G31_U_PLAN_FG1);
        barriers.push(G31_U_PLAN_ENC_FG1);
        // readback 入列序（G31FgLayout 下标同源）：FG_BGRA 全部 → FG_OUT 全部
        // → MVN;fg1 块只入 FG1_BGRA,FG1_OUT 待 x3 块后统一入列。
        readbacks.push(Readback::Buffer {
            res: G31_U_FG1_BGRA,
            offset: 0,
            size: opc * 4,
        });
        if fga.mode == G31Fg::X3 {
            // 29..=31 = FG2 参数/输出 f32/BGRA8（x3 第二插入帧）。
            resources.push(init(fga.params2_bytes));
            resources.push(buf(opc * 12));
            resources.push(buf(opc * 4));
            passes.push(Pass::Compute(ComputePass {
                name: "g26_framegen_fg2",
                spirv: fga.spv_bytes,
                entry: None,
                dispatch: DispatchSpec::Direct(fga.dispatch),
                bindings: Bindings {
                    storage_buffers: vec![
                        U_OUT_COLOR[1],
                        U_OUT_COLOR[0],
                        G31_U_MVN,
                        G31_U_FG2_PARAMS,
                        G31_U_FG2_OUT,
                    ],
                    ..Bindings::default()
                },
            }));
            passes.push(Pass::Compute(ComputePass {
                name: "g31_display_encode_fg2",
                spirv: enc_spv,
                entry: None,
                dispatch: DispatchSpec::Direct(enc_dispatch),
                bindings: Bindings {
                    storage_buffers: vec![G31_U_FG2_OUT, G31_U_ENC_PARAMS, G31_U_FG2_BGRA],
                    ..Bindings::default()
                },
            }));
            barriers.push(G31_U_PLAN_FG2);
            barriers.push(G31_U_PLAN_ENC_FG2);
            readbacks.push(Readback::Buffer {
                res: G31_U_FG2_BGRA,
                offset: 0,
                size: opc * 4,
            });
        }
        // FG_OUT f32 全部（probe 帧子集消费;序 = fg1,fg2）。
        readbacks.push(Readback::Buffer {
            res: G31_U_FG1_OUT,
            offset: 0,
            size: opc * 12,
        });
        if fga.mode == G31Fg::X3 {
            readbacks.push(Readback::Buffer {
                res: G31_U_FG2_OUT,
                offset: 0,
                size: opc * 12,
            });
        }
        // 末位 = MVN 取反结果回读（probe 帧 device 侧 MV 内容直比对面;布局
        // 下标见 G31FgLayout::of）。
        readbacks.push(Readback::Buffer {
            res: G31_U_MVN,
            offset: 0,
            size: opc * 8,
        });
    }
    G31Descs {
        resources,
        passes,
        barriers,
        readbacks,
    }
}

/// D3 bloom 描述组输入面（spv/dispatch/参数字节 era 常量——extent 联动,resize
/// 随车道重建;bloom off 时本面不构造）。
struct G31BloomAssets<'x> {
    spv_bright: &'x [u8],
    spv_blur: &'x [u8],
    spv_composite: &'x [u8],
    dispatch_bright: [u32; 3],
    dispatch_blur: [u32; 3],
    dispatch_composite: [u32; 3],
    bright_params_bytes: &'x [u8],
    blur_h_params_bytes: &'x [u8],
    blur_v_params_bytes: &'x [u8],
    comp_params_bytes: &'x [u8],
}

/// D3 bloom 变体描述组（--bloom on 面;`g31_lane_descs` fg=None 形态产物 +
/// encode pass/屏障摘出 → resolve 后插入 bright→blurH→blurV→composite 四 pass
/// → encode 重挂（in_color = 合成全分辨率缓冲,初始绑定即终态不轮换）+ 资源
/// 追加 24..=31——既有资源/readback 面 0-byte,off 面不构造;逐帧 parity 轮换
/// 由 prepare_update 对 bright(4)/composite(7)/encode(8) 三 pass override
/// 承载,blur 两 pass 绑定静态）。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs_bloom<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    blm: &'x G31BloomAssets<'x>,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> G31Descs<'x> {
    let mut d = g31_lane_descs(
        assets,
        bits,
        enc_spv,
        enc_dispatch,
        enc_params_bytes,
        iw,
        ih,
        ow,
        oh,
        None,
    );
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: true,
        })
    };
    // 半分辨率 = ceil(全分辨率/2)（与 g31_bloom_bright.rx out_w=(in_w+1)/2 同式）。
    let half_bytes = u64::from(ow.div_ceil(2)) * u64::from(oh.div_ceil(2)) * 12;
    let full_bytes = u64::from(ow) * u64::from(oh) * 12;
    d.resources.push(init(blm.bright_params_bytes)); // G31_U_BLOOM_BRIGHT_PARAMS
    d.resources.push(buf(half_bytes)); // G31_U_BLOOM_BRIGHT
    d.resources.push(init(blm.blur_h_params_bytes)); // G31_U_BLOOM_BLUR_H_PARAMS
    d.resources.push(buf(half_bytes)); // G31_U_BLOOM_PING
    d.resources.push(init(blm.blur_v_params_bytes)); // G31_U_BLOOM_BLUR_V_PARAMS
    d.resources.push(buf(half_bytes)); // G31_U_BLOOM_PONG
    d.resources.push(init(blm.comp_params_bytes)); // G31_U_BLOOM_COMP_PARAMS
    d.resources.push(buf(full_bytes)); // G31_U_BLOOM_COMP_OUT
    debug_assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_BLOOM);
    // 摘出 encode pass/屏障（五 pass 面末位;readback 面 0-byte 不动）。
    let _ = d.passes.pop();
    let _ = d.barriers.pop();
    debug_assert_eq!(d.passes.len(), 4);
    // pass 4 = bright（初始绑定 parity 0,逐帧 override 换 parity）。
    d.passes.push(Pass::Compute(ComputePass {
        name: "g31_bloom_bright",
        spirv: blm.spv_bright,
        entry: None,
        dispatch: DispatchSpec::Direct(blm.dispatch_bright),
        bindings: Bindings {
            storage_buffers: vec![
                U_OUT_COLOR[0],
                G31_U_BLOOM_BRIGHT_PARAMS,
                G31_U_BLOOM_BRIGHT,
            ],
            ..Bindings::default()
        },
    }));
    d.barriers.push(G31_U_PLAN_BLOOM_BRIGHT);
    // pass 5 = blur H（bright → ping,绑定静态）。
    d.passes.push(Pass::Compute(ComputePass {
        name: "g31_bloom_blur_h",
        spirv: blm.spv_blur,
        entry: None,
        dispatch: DispatchSpec::Direct(blm.dispatch_blur),
        bindings: Bindings {
            storage_buffers: vec![
                G31_U_BLOOM_BRIGHT,
                G31_U_BLOOM_BLUR_H_PARAMS,
                G31_U_BLOOM_PING,
            ],
            ..Bindings::default()
        },
    }));
    d.barriers.push(G31_U_PLAN_BLOOM_BLUR_H);
    // pass 6 = blur V（ping → pong,绑定静态）。
    d.passes.push(Pass::Compute(ComputePass {
        name: "g31_bloom_blur_v",
        spirv: blm.spv_blur,
        entry: None,
        dispatch: DispatchSpec::Direct(blm.dispatch_blur),
        bindings: Bindings {
            storage_buffers: vec![
                G31_U_BLOOM_PING,
                G31_U_BLOOM_BLUR_V_PARAMS,
                G31_U_BLOOM_PONG,
            ],
            ..Bindings::default()
        },
    }));
    d.barriers.push(G31_U_PLAN_BLOOM_BLUR_V);
    // pass 7 = composite（初始绑定 parity 0,逐帧 override 换 parity）。
    d.passes.push(Pass::Compute(ComputePass {
        name: "g31_bloom_composite",
        spirv: blm.spv_composite,
        entry: None,
        dispatch: DispatchSpec::Direct(blm.dispatch_composite),
        bindings: Bindings {
            storage_buffers: vec![
                U_OUT_COLOR[0],
                G31_U_BLOOM_PONG,
                G31_U_BLOOM_COMP_PARAMS,
                G31_U_BLOOM_COMP_OUT,
            ],
            ..Bindings::default()
        },
    }));
    d.barriers.push(G31_U_PLAN_BLOOM_COMPOSITE);
    // pass 8 = encode（in_color = 合成输出;绑定静态不轮换）。
    d.passes.push(Pass::Compute(ComputePass {
        name: "g31_display_encode",
        spirv: enc_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(enc_dispatch),
        bindings: Bindings {
            storage_buffers: vec![G31_U_BLOOM_COMP_OUT, G31_U_ENC_PARAMS, G31_U_ENC_OUT],
            ..Bindings::default()
        },
    }));
    d.barriers.push(G31_U_PLAN_ENCODE_BLOOM);
    d
}

// ---------------------------------------------------------------------------
// 夜间巡航 D2 平滑顶点法线窗口车道面（--smooth-normals <off|on>,默认 off =
// 既有五 pass/24 资源 0-byte 现状 + presented digest 锚零漂移;on = scene pass
// 换 kernels/g18_smooth_nrm.rx（g18 逐字 fork + params[43] 门重心插值顶点
// 法线;params[44..48) 半球环境光经 RURIX_G18_AMBIENT env 门控,host 面 =
// pack_frame_params_nrm 统一打包）+ trinrm 侧表 SSBO 追加）。下标纪律：
// trinrm 挂在既有面尾部（单臂面 = 24;与 --bloom on 组合面 = bloom 八件
// 24..=31 之后 32）——encode 22/23 与 bloom 24..=31 下标 0-byte 不动,
// parity override/回读/呈现链零改动。与 --fg/--hzb/--textures/--svt/
// --slab-table/--cluster-lod/--wp-hlod fail-closed 互斥（组合面未接线）;
// 与 --bloom/--dither 可组合（scene 上游/post 下游正交）。
// ---------------------------------------------------------------------------

/// D2 追加资源下标（单臂面 = 24〔既有 24 资源尾部〕;组合面 = 32〔bloom 八件
/// 之后〕——两形态互斥占用,同既有闭集纪律）。
const G31_U_TRINRM: u32 = 24;
const G31_U_TRINRM_BLOOM: u32 = 32;
/// D6 追加资源下标（tri_mr 2 f32/tri [metallic, roughness] 侧表——
/// g18_smooth_nrm kernel D6 扩签名第 9 路 storage view 必须全绑;--ggx off
/// 面绑 8B 零哑表（kernel 对 tri_mr 的唯一读取经 params[48]>0.5 均匀分支
/// 包裹,off 车道 params[48] 恒 0 ⇒ 哑表永不读,OOB 不可能）,on 面绑真表）。
/// 单臂面 = 25〔trinrm 后〕;组合面 = 33。
const G31_U_TRI_MR: u32 = 25;
const G31_U_TRI_MR_BLOOM: u32 = 33;
/// D6 哑表字节面（8B 零;--ggx off 面两 nrm 变体共享）。
const G31_TRI_MR_DUMMY: &[u8] = &[0u8; 8];
/// D2 单臂车道资源数（24 既有 + trinrm + tri_mr 哑表两件）。
const G31_U_RESOURCE_COUNT_NRM: usize = 26;
/// D2×D3 组合车道资源数（32 bloom + trinrm + tri_mr 哑表两件）。
const G31_U_RESOURCE_COUNT_NRM_BLOOM: usize = 34;

/// D2 单臂面 scene pass 屏障计划（U_PLAN_SCENE 触达超集 + trinrm——保守
/// 超集同律;D6 += tri_mr 哑表同域）。
const G31_U_PLAN_SCENE_NRM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TRINRM, TargetState::StorageReadWrite),
    (G31_U_TRI_MR, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];
/// D2×D3 组合面 scene pass 屏障计划（trinrm = 32;tri_mr 哑表 = 33）。
const G31_U_PLAN_SCENE_NRM_BLOOM: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G31_U_TRINRM_BLOOM, TargetState::StorageReadWrite),
    (G31_U_TRI_MR_BLOOM, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
];

/// D2 单臂变体描述组（--smooth-normals on 且 --bloom off 面;
/// `g31_lane_descs` fg=None 形态产物 + trinrm SSBO 追加下标 24 + scene pass
/// 换 g18_smooth_nrm kernel〔绑定面 = 既有 5 路 + trinrm + out 双路,声明序
/// 与 kernels/g18_smooth_nrm.rx 签名逐字同源〕+ scene 屏障换
/// G31_U_PLAN_SCENE_NRM——既有资源/pass/屏障/readback 各面 0-byte,off 面
/// 不构造）。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs_nrm<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    trinrm_bytes: &'x [u8],
    tri_mr_bytes: &'x [u8],
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> G31Descs<'x> {
    let mut d = g31_lane_descs(
        assets,
        bits,
        enc_spv,
        enc_dispatch,
        enc_params_bytes,
        iw,
        ih,
        ow,
        oh,
        None,
    );
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    d.resources.push(ResourceDesc::Buffer(BufferDesc {
        size: trinrm_bytes.len() as u64,
        usage: storage,
        data: Some(trinrm_bytes),
        device_local: true,
    })); // G31_U_TRINRM
    // D6：tri_mr 侧表（--ggx on = 真表〔2 f32/tri〕,off = 8B 零哑表——
    // kernel params[48]=0 门不读;绑定满足 D6 扩签名第 9 路 storage view
    // 全绑要求）。
    d.resources.push(ResourceDesc::Buffer(BufferDesc {
        size: tri_mr_bytes.len() as u64,
        usage: storage,
        data: Some(tri_mr_bytes),
        device_local: true,
    })); // G31_U_TRI_MR
    debug_assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_NRM);
    d.passes[0] = Pass::Compute(ComputePass {
        name: "g18_smooth_nrm",
        spirv: &bits.spv_scene,
        entry: None,
        dispatch: DispatchSpec::Direct(bits.scene_dispatch),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: vec![
                U_TRIS,
                U_MATS,
                U_QUADS,
                U_POINTS,
                U_SCENE_PARAMS,
                G31_U_TRINRM,
                G31_U_TRI_MR,
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
            ],
            ..Bindings::default()
        },
    });
    d.barriers[0] = G31_U_PLAN_SCENE_NRM;
    d
}

/// D2×D3 组合变体描述组（--smooth-normals on --bloom on 面;
/// `g31_lane_descs_bloom` 产物——bloom 八件 24..=31/九 pass 图/encode 改读
/// 合成缓冲逐字不动——+ trinrm SSBO 追加下标 32 + scene pass 换
/// g18_smooth_nrm + scene 屏障换 G31_U_PLAN_SCENE_NRM_BLOOM;bloom 下标面与
/// 逐帧 parity override 面 0-byte,off 面不构造）。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs_nrm_bloom<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    trinrm_bytes: &'x [u8],
    tri_mr_bytes: &'x [u8],
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    blm: &'x G31BloomAssets<'x>,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> G31Descs<'x> {
    let mut d = g31_lane_descs_bloom(
        assets,
        bits,
        enc_spv,
        enc_dispatch,
        enc_params_bytes,
        blm,
        iw,
        ih,
        ow,
        oh,
    );
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    d.resources.push(ResourceDesc::Buffer(BufferDesc {
        size: trinrm_bytes.len() as u64,
        usage: storage,
        data: Some(trinrm_bytes),
        device_local: true,
    })); // G31_U_TRINRM_BLOOM
    // D6：tri_mr 侧表（单臂面同律——on = 真表,off = 8B 零哑表,
    // params[48]=0 门不读,仅满足 kernel D6 扩签名第 9 路全绑）。
    d.resources.push(ResourceDesc::Buffer(BufferDesc {
        size: tri_mr_bytes.len() as u64,
        usage: storage,
        data: Some(tri_mr_bytes),
        device_local: true,
    })); // G31_U_TRI_MR_BLOOM
    debug_assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_NRM_BLOOM);
    d.passes[0] = Pass::Compute(ComputePass {
        name: "g18_smooth_nrm",
        spirv: &bits.spv_scene,
        entry: None,
        dispatch: DispatchSpec::Direct(bits.scene_dispatch),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: vec![
                U_TRIS,
                U_MATS,
                U_QUADS,
                U_POINTS,
                U_SCENE_PARAMS,
                G31_U_TRINRM_BLOOM,
                G31_U_TRI_MR_BLOOM,
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
            ],
            ..Bindings::default()
        },
    });
    d.barriers[0] = G31_U_PLAN_SCENE_NRM_BLOOM;
    d
}

/// B4 纹理变体描述组（--textures on 面;`g31_lane_descs` fg=None 形态产物逐项
/// 克隆 + scene pass 换 g31_texture_gi 变体〔绑定面 = 既有 7 路 + B4 五件
/// SSBO,声明序与 kernels/g31_texture_gi.rx 签名逐字同源〕+ 资源追加 24..=28
/// + scene 屏障换 G31_U_PLAN_SCENE_TEX——既有资源/pass/屏障/readback 各面
/// 0-byte,off 面不构造）。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs_tex<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    tex_spv: &'x [u8],
    tex: &'x G31TexAssetsHeap,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> G31Descs<'x> {
    let mut d = g31_lane_descs(
        assets,
        bits,
        enc_spv,
        enc_dispatch,
        enc_params_bytes,
        iw,
        ih,
        ow,
        oh,
        None,
    );
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    d.resources.push(init(&tex.texuv_bytes)); // G31_U_TEX_UV
    d.resources.push(init(&tex.texmeta_bytes)); // G31_U_TEX_META
    d.resources.push(init(&tex.tritex_bytes)); // G31_U_TEX_TRITEX
    d.resources.push(init(&tex.atlas_bytes)); // G31_U_TEX_ATLAS
    d.resources.push(init(&tex.linlut_bytes)); // G31_U_TEX_LINLUT
    debug_assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEX);
    d.passes[0] = Pass::Compute(ComputePass {
        name: "g31_texture_gi",
        spirv: tex_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(bits.scene_dispatch),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: vec![
                U_TRIS,
                U_MATS,
                U_QUADS,
                U_POINTS,
                U_SCENE_PARAMS,
                G31_U_TEX_UV,
                G31_U_TEX_META,
                G31_U_TEX_TRITEX,
                G31_U_TEX_ATLAS,
                G31_U_TEX_LINLUT,
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
            ],
            ..Bindings::default()
        },
    });
    d.barriers[0] = G31_U_PLAN_SCENE_TEX;
    d
}

/// day_0828 Phase B ① tex+nrm 合流变体描述组（--textures on &&
/// --smooth-normals on && --bloom off 面;`g31_lane_descs` fg=None 形态产物 +
/// tex 五件追加 24..=28（B4 下标不动）+ trinrm=29/tri_mr=30 + scene pass 换
/// g31_texture_nrm_gi 合体 kernel〔绑定序与 kernels/g31_texture_nrm_gi.rx
/// 签名逐字同源〕+ scene 屏障换 G31_U_PLAN_SCENE_TEXNRM——既有资源/pass/
/// 屏障/readback 各面 0-byte）。
/// day_0829 臂① tri_base 未衰减 baseColor 侧表装配（--metal-f0 on 面独消费;
/// off 面不调用 0-byte）。3 f32/tri：
///   - 有 heap 槽三角（tritex[i×2] ≥ 0）= baseColorFactor（kernel 内乘原始
///     采样 raw_*——F0 = 逐像素未衰减贴图色）；
///   - 常量面三角（tritex < 0）= tex_mean·factor（主装配 albedo 同源均值,
///     **不乘 (1−metallic)**——正是修伤面）；
///   - 无材质/灯面（tri_mat = SLAB_TRI_NONE）= [1,1,1]（主装配 albedo None
///     分支同律;灯面 metal=0 ⇒ F0 无 tri_base 消费,值无关紧要如实登记）。
/// 材质表/tex_mean = assemble_scene_ex_nrm L1632-1692 重放（gltf 二次解析,
/// 装配期一次性成本;共享体 0-byte 纪律 ⇒ 不改共享装配签名,本 fn 窗口 bin
/// 自有）。
fn g31_assemble_tri_base(
    contract: &Json,
    scene_id: &str,
    gltf_path: &Path,
    tri_mat: &[u32],
    tritex_bytes: &[u8],
) -> Result<Vec<f32>, String> {
    let srow = contract_scene_row(contract, scene_id)?;
    let texture_mean = srow
        .get("material_policy")
        .and_then(|p| p.get("texture_mean_albedo"))
        .and_then(|v| v.as_bool())
        .ok_or("契约场景行缺 material_policy.texture_mean_albedo（臂① fail-closed）")?;
    let (gltf, _sha) = load_gltf(gltf_path)?;
    let base = gltf_path.parent().unwrap_or_else(|| Path::new("."));
    // 材质表重放（factor/base_img——metallic 不读:未衰减面正是不乘 k）。
    let mut factors: Vec<[f32; 3]> = Vec::new();
    let mut base_imgs: Vec<Option<usize>> = Vec::new();
    for m in gltf
        .root
        .get("materials")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let pbr = m.get("pbrMetallicRoughness");
        let alb4 = pbr
            .and_then(|p| p.get("baseColorFactor"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|x| x.as_f64().unwrap_or(1.0) as f32).collect::<Vec<_>>());
        factors.push(match alb4 {
            Some(v) if v.len() == 4 => [v[0], v[1], v[2]],
            _ => [1.0, 1.0, 1.0],
        });
        base_imgs.push(
            pbr.and_then(|p| p.get("baseColorTexture"))
                .and_then(|t| t.get("index"))
                .and_then(|v| v.as_u64())
                .and_then(|ti| gltf.root.get("textures")?.as_array()?.get(ti as usize))
                .and_then(|tex| tex.get("source"))
                .and_then(|v| v.as_u64())
                .map(|x| x as usize),
        );
    }
    // tex_mean 重放（主装配同源:DDS BC1/BC3 真解码 sRGB→线性均值;仅常量面
    // 三角消费——bistro 70/70 有槽,本表通常零消费,语义完备保留）。
    let mut tex_mean: Vec<Option<[f32; 3]>> = Vec::new();
    if let Some(imgs) = gltf.root.get("images").and_then(|v| v.as_array()) {
        let consumed: std::collections::BTreeSet<usize> =
            base_imgs.iter().filter_map(|m| *m).collect();
        for (ii, im) in imgs.iter().enumerate() {
            let mut mean = None;
            if texture_mean && consumed.contains(&ii) {
                let uri = im
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .ok_or("image 缺 uri（臂① tri_base 面,内嵌不消费）")?;
                let raw = std::fs::read(base.join(uri))
                    .map_err(|e| format!("纹理 {uri} 读取失败: {e}"))?;
                mean = Some(
                    dds_mean_linear_rgb(&raw)
                        .map_err(|e| format!("纹理 {uri} DDS 解码失败: {e}"))?,
                );
            }
            tex_mean.push(mean);
        }
    }
    if tritex_bytes.len() != tri_mat.len() * 2 * 4 {
        return Err(format!(
            "臂① tritex 长度 {} ≠ tri_count×2×4 = {}（互核 fail-closed）",
            tritex_bytes.len(),
            tri_mat.len() * 2 * 4
        ));
    }
    let mut out = Vec::with_capacity(tri_mat.len() * 3);
    for (i, &mi) in tri_mat.iter().enumerate() {
        let slotf = f32::from_le_bytes([
            tritex_bytes[i * 8],
            tritex_bytes[i * 8 + 1],
            tritex_bytes[i * 8 + 2],
            tritex_bytes[i * 8 + 3],
        ]);
        let tb = if mi == SLAB_TRI_NONE {
            [1.0, 1.0, 1.0]
        } else {
            let f = factors
                .get(mi as usize)
                .copied()
                .ok_or_else(|| format!("臂① tri_mat[{i}]={mi} 越材质表（fail-closed）"))?;
            if slotf >= 0.0 {
                f
            } else {
                match base_imgs
                    .get(mi as usize)
                    .copied()
                    .flatten()
                    .and_then(|ii| tex_mean.get(ii).copied().flatten())
                {
                    Some(tm) => [tm[0] * f[0], tm[1] * f[1], tm[2] * f[2]],
                    None => f,
                }
            }
        };
        out.extend_from_slice(&tb);
    }
    Ok(out)
}

/// G37 W2 transparency：tri_transp 透射率侧表装配（--transparency on 面独
/// 消费;off 面不调用 0-byte）。1 f32/tri：透明材质三角 = alpha_v（--transp-
/// alpha,默认 0.85）,其余 = 0.0（不透明;SLAB_TRI_NONE 灯面同 0）。
/// **判定规则**（装配期 glTF 二次解析,g31_assemble_tri_base 同律窗口 bin
/// 自有）：材质 alphaMode == "BLEND" **或** pbrMetallicRoughness.
/// baseColorFactor[3]（alpha）< 1.0——bistro 资产 alphaMode 全 OPAQUE,唯一
/// alpha<1 材质 = TransparentGlass.DoubleSided（a=0.2,130,792 tris）,精确
/// 命中缺陷本体零误伤;名字启发式（含 "Glass" 的 9 材质中 8 个为实心/涂层
/// 玻璃形态——酒瓶/画框/外窗）误伤面大,不进判定如实登记。资产 alpha 值
/// 语义为 coverage 非透射率,透射率统一取 --transp-alpha 工程值登记。
fn g31_assemble_tri_transp(
    gltf_path: &Path,
    tri_mat: &[u32],
    alpha_v: f32,
) -> Result<(Vec<f32>, Vec<(u32, String, u32)>), String> {
    let (gltf, _sha) = load_gltf(gltf_path)?;
    let mats = gltf
        .root
        .get("materials")
        .and_then(|v| v.as_array())
        .ok_or("glTF 缺 materials（transparency 判定面,fail-closed）")?;
    let mut transp_mat: Vec<bool> = Vec::with_capacity(mats.len());
    let mut names: Vec<String> = Vec::with_capacity(mats.len());
    for m in mats {
        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("").to_owned();
        let blend = m
            .get("alphaMode")
            .and_then(|v| v.as_str())
            .map(|s| s == "BLEND")
            .unwrap_or(false);
        let alpha_lt1 = m
            .get("pbrMetallicRoughness")
            .and_then(|p| p.get("baseColorFactor"))
            .and_then(|v| v.as_array())
            .and_then(|a| a.get(3))
            .and_then(|x| x.as_f64())
            .map(|a| a < 1.0)
            .unwrap_or(false);
        transp_mat.push(blend || alpha_lt1);
        names.push(name);
    }
    let mut out = Vec::with_capacity(tri_mat.len());
    let mut hit_tris: Vec<u32> = vec![0; mats.len()];
    for (i, &mi) in tri_mat.iter().enumerate() {
        let t = if mi == SLAB_TRI_NONE {
            0.0
        } else {
            let is_t = *transp_mat
                .get(mi as usize)
                .ok_or_else(|| format!("transparency: tri_mat[{i}]={mi} 越材质表（fail-closed）"))?;
            if is_t {
                hit_tris[mi as usize] += 1;
                alpha_v
            } else {
                0.0
            }
        };
        out.push(t);
    }
    let hits: Vec<(u32, String, u32)> = transp_mat
        .iter()
        .enumerate()
        .filter(|&(_, &t)| t)
        .map(|(mi, _)| (mi as u32, names[mi].clone(), hit_tris[mi]))
        .collect();
    if hits.is_empty() {
        return Err("transparency: 判定规则零命中（alphaMode==BLEND || baseColor.a<1 无一材质满足——臂无消费面,fail-closed）".into());
    }
    Ok((out, hits))
}

/// day_0829 臂④ 法线烘焙容器装配（--normal-maps on 面独消费;fail-closed
/// 闭集:manifest_bin 缺件/字段缺失/sha256 失配/容器破/槽号越界任一破即 Err）。
/// 追加语义 = g31_emissive_append 同律:heap 头表 slots×13 → (slots+70)×13
/// 全重排布 + 70 槽 texel 段尾接（cap-1024 起级——2048² 12 级链从 mip1 起 =
/// 1024² 11 级,零重采样直搬）;texmeta mod 位 = [1,1,1]（法线无 scale 语义）;
/// 产 trinm 侧表 1 f32/tri（材质有法线槽 → 槽号,SLAB_TRI_NONE/缺行 → −1）。
fn g31_normals_append(
    tex: &mut G31TexAssetsHeap,
    tri_mat: &[u32],
    dir: &str,
) -> Result<(Vec<f32>, usize, usize), String> {
    let manifest_path = format!("{dir}/manifest_bin.json");
    let mtext = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("法线烘焙 manifest 缺件 {manifest_path}: {e}（先跑 pack_normals_bin.py,fail-closed）"))?;
    let mdoc = json_parse(&mtext).map_err(|e| format!("manifest_bin JSON: {e}"))?;
    let entries = mdoc
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or("manifest_bin 缺 entries 数组")?;
    if entries.is_empty() {
        return Err("manifest_bin entries 空（臂④无消费面,fail-closed）".into());
    }
    struct NmPending {
        material_index: u32,
        stored: Vec<(u32, u32, Vec<u8>)>,
    }
    let mut pend: Vec<NmPending> = Vec::with_capacity(entries.len());
    for e in entries {
        let mi = e
            .get("material_index")
            .and_then(|v| v.as_u64())
            .ok_or("manifest_bin 行缺 material_index")? as u32;
        let file = e
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or("manifest_bin 行缺 file")?;
        let out_sha = e
            .get("output_sha256")
            .and_then(|v| v.as_str())
            .ok_or("manifest_bin 行缺 output_sha256")?;
        let path = Path::new(dir).join(file);
        let blob = std::fs::read(&path)
            .map_err(|e2| format!("法线烘焙件缺件 {}: {e2}（fail-closed）", path.display()))?;
        let got = format!("sha256:{}", sha256_hex(&blob));
        if got != out_sha {
            return Err(format!(
                "{} sha256 {got} ≠ manifest {out_sha}（烘焙件漂移,fail-closed）",
                path.display()
            ));
        }
        let (_w, _h, levels) = g31_rgba8bin_read(&path)?;
        let start = levels
            .iter()
            .position(|(lw, lh, _)| *lw <= G31_TEX_CAP && *lh <= G31_TEX_CAP)
            .ok_or_else(|| format!("{} 全链无 ≤{}² 级", path.display(), G31_TEX_CAP))?;
        let stored: Vec<(u32, u32, Vec<u8>)> = levels[start..].to_vec();
        if stored.len() > G31_TEX_MIP_SLOTS {
            return Err(format!("{} 存储级数越头表槽位", path.display()));
        }
        pend.push(NmPending { material_index: mi, stored });
    }
    // heap 全重排布(em append 逐字同形)。
    let old_slots = tex.slots.len();
    let old_hdr = tex.heap_header_entries;
    if old_hdr != old_slots * G31_TEX_MIP_SLOTS {
        return Err(format!(
            "heap 头表项数 {old_hdr} ≠ slots×13 = {}（前置形态破坏,fail-closed）",
            old_slots * G31_TEX_MIP_SLOTS
        ));
    }
    let new_slots_n = old_slots + pend.len();
    let new_hdr = new_slots_n * G31_TEX_MIP_SLOTS;
    let shift = (new_hdr - old_hdr) as u32;
    let body_len = tex.atlas.len() - old_hdr;
    let append_texels: usize = pend
        .iter()
        .map(|p| p.stored.iter().map(|(lw, lh, _)| (*lw as usize) * (*lh as usize)).sum::<usize>())
        .sum();
    let new_total = new_hdr + body_len + append_texels;
    if (new_total as u64) * 4 > G31_TEX_HEAP_MAX_BYTES {
        return Err(format!(
            "法线扩容后 heap {}B 越保守界 {G31_TEX_HEAP_MAX_BYTES}B（fail-closed）",
            new_total * 4
        ));
    }
    let mut new_atlas: Vec<u32> = Vec::with_capacity(new_total);
    new_atlas.extend(tex.atlas[..old_hdr].iter().map(|v| v + shift));
    let mut cur = new_hdr + body_len;
    for p in &pend {
        let base_entry = new_atlas.len();
        for (lw, lh, _) in &p.stored {
            new_atlas.push(cur as u32);
            cur += (*lw as usize) * (*lh as usize);
        }
        let last = new_atlas[base_entry + p.stored.len() - 1];
        for _ in p.stored.len()..G31_TEX_MIP_SLOTS {
            new_atlas.push(last);
        }
    }
    debug_assert_eq!(new_atlas.len(), new_hdr);
    new_atlas.extend_from_slice(&tex.atlas[old_hdr..]);
    for p in &pend {
        for (_, _, px) in &p.stored {
            for chunk in px.chunks_exact(4) {
                new_atlas.push(
                    u32::from(chunk[0])
                        | (u32::from(chunk[1]) << 8)
                        | (u32::from(chunk[2]) << 16)
                        | (u32::from(chunk[3]) << 24),
                );
            }
        }
    }
    debug_assert_eq!(new_atlas.len(), new_total);
    tex.texmeta[0] = new_hdr as f32;
    tex.texmeta[2] = new_slots_n as f32;
    let mut slot_of_mi: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for (k, p) in pend.iter().enumerate() {
        let slot = old_slots + k;
        slot_of_mi.insert(p.material_index, slot);
        tex.texmeta.extend_from_slice(&[
            0.0,
            0.0,
            p.stored[0].0 as f32,
            p.stored[0].1 as f32,
            1.0,
            1.0,
            1.0,
            p.stored.len() as f32,
        ]);
        let mip_digests: Vec<String> = p
            .stored
            .iter()
            .map(|(_, _, px)| format!("sha256:{}", sha256_hex(px)))
            .collect();
        tex.slots.push(G31TexSlotHeap {
            material_index: p.material_index,
            material_name: format!("normal_mat{}", p.material_index),
            tris: 0,
            texture_uri: format!("slot{:02}.rgba8bin", slot - old_slots),
            width: p.stored[0].0,
            height: p.stored[0].1,
            src_width: p.stored[0].0,
            src_height: p.stored[0].1,
            dds_format: "bc5-rg-normal-baked".to_owned(),
            manifest_source_digest: None,
            rgba8_digest: format!("sha256:{}", sha256_hex(&p.stored[0].2)),
            manifest_rgba8_digest: None,
            mip_count: p.stored.len() as u32,
            mip_digests,
            mip_truncated: false,
            origin_x: 0,
            origin_y: 0,
            mod_rgb: [1.0, 1.0, 1.0],
        });
        tex.slots_rgba8.push(p.stored[0].2.clone());
    }
    tex.heap_header_entries = new_hdr;
    tex.heap_texels = new_total;
    tex.atlas = new_atlas;
    tex.atlas_bytes = tex.atlas.iter().flat_map(|v| v.to_le_bytes()).collect();
    tex.atlas_digest = format!("sha256:{}", sha256_hex(&tex.atlas_bytes));
    tex.texmeta_bytes = bytes_f32(&tex.texmeta);
    // trinm 侧表(1 f32/tri:材质有法线槽 → 槽号,否则 −1)。
    let mut trinm: Vec<f32> = Vec::with_capacity(tri_mat.len());
    let mut nm_tris = 0usize;
    for &mi in tri_mat {
        let s = if mi == SLAB_TRI_NONE {
            -1.0
        } else {
            slot_of_mi.get(&mi).map(|s| *s as f32).unwrap_or(-1.0)
        };
        if s >= 0.0 {
            nm_tris += 1;
        }
        trinm.push(s);
    }
    Ok((trinm, nm_tris, append_texels))
}

/// day_0829 臂④ 逐三角切线装配（UV 导数法;glTF 无 TANGENT ⇒ 烘焙期生成——
/// 参考实现 artifacts/day_0829_realism/a4_normalmap/tangent_ref.py）。
/// 4 f32/tri = [T.xyz(未逐像素正交化,kernel 内 Gram-Schmidt), 手性 w]:
///   T = (dP1·dv2 − dP2·dv1)/det, B0 = (dP2·du1 − dP1·du2)/det,
///   det = du1·dv2 − du2·dv1;|det| ≤ 1e-12 退化 ⇒ [0,0,0,1]（kernel
///   tan_gl 门保原法线）;w = sign(dot(cross(Ng,T), B0))（≥0 → 1,<0 → −1）。
fn g31_assemble_tri_tan(scene: &SceneData, tri_uv: &[f32]) -> Result<Vec<f32>, String> {
    let n = scene.indices.len();
    if tri_uv.len() != n * 6 {
        return Err(format!(
            "臂④ tri_uv 长度 {} ≠ tri_count×6 = {}（互核 fail-closed）",
            tri_uv.len(),
            n * 6
        ));
    }
    let mut out = Vec::with_capacity(n * 4);
    for (i, idx) in scene.indices.iter().enumerate() {
        let p0 = scene.positions[idx[0] as usize];
        let p1 = scene.positions[idx[1] as usize];
        let p2 = scene.positions[idx[2] as usize];
        let ub = i * 6;
        let (u0, v0) = (tri_uv[ub], tri_uv[ub + 1]);
        let (u1, v1) = (tri_uv[ub + 2], tri_uv[ub + 3]);
        let (u2, v2) = (tri_uv[ub + 4], tri_uv[ub + 5]);
        let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let du1 = u1 - u0;
        let dv1 = v1 - v0;
        let du2 = u2 - u0;
        let dv2 = v2 - v0;
        let det = du1 * dv2 - du2 * dv1;
        if det.abs() <= 1e-12 {
            out.extend_from_slice(&[0.0, 0.0, 0.0, 1.0]);
            continue;
        }
        let inv = 1.0 / det;
        let t = [
            (e1[0] * dv2 - e2[0] * dv1) * inv,
            (e1[1] * dv2 - e2[1] * dv1) * inv,
            (e1[2] * dv2 - e2[2] * dv1) * inv,
        ];
        let b0 = [
            (e2[0] * du1 - e1[0] * du2) * inv,
            (e2[1] * du1 - e1[1] * du2) * inv,
            (e2[2] * du1 - e1[2] * du2) * inv,
        ];
        let tl = (t[0] * t[0] + t[1] * t[1] + t[2] * t[2]).sqrt();
        if !(tl.is_finite() && tl > 1e-12) {
            out.extend_from_slice(&[0.0, 0.0, 0.0, 1.0]);
            continue;
        }
        let tn = [t[0] / tl, t[1] / tl, t[2] / tl];
        let ng = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let c = [
            ng[1] * tn[2] - ng[2] * tn[1],
            ng[2] * tn[0] - ng[0] * tn[2],
            ng[0] * tn[1] - ng[1] * tn[0],
        ];
        let w = if c[0] * b0[0] + c[1] * b0[1] + c[2] * b0[2] >= 0.0 {
            1.0
        } else {
            -1.0
        };
        out.extend_from_slice(&[tn[0], tn[1], tn[2], w]);
    }
    Ok(out)
}

/// day_0828 Phase F：`triem_bytes` = --emissive-tex on 面（Some = triem 尾挂
/// 31 + 绑定插 linlut 之后〔kernel 签名序〕+ 屏障换 _EM 计划 + kernel 名换
/// em 变体;None = 本函数产物与 Phase F 前逐字同构,0-byte）。
/// day_0829 realism：`tri_base_bytes` = realism 任一臂 on 面（Some = tri_base
/// 尾挂 32〔调用面保证 triem 同 Some——em off 传 -1 回退真表保持 kernel 签名
/// 序〕+ 屏障换 _REAL 计划;None = 本函数产物与 day_0829 前逐字同构,0-byte）。
/// day_0829 臂④：`nm_bytes` = --normal-maps on 面（Some = (trinm, tri_tan)
/// 尾挂 33/34〔tri_base 必 Some〕+ 屏障换 _NM 计划;None = 上述面逐字）。
/// G37 W2 transparency：`tri_transp_bytes` = --transparency on 面（Some =
/// tri_transp 尾挂 35〔nm_bytes 必 Some——nm off 时调用点传 trinm 回退真表 +
/// tri_tan 哑表保持 kernel 签名序〕+ 屏障换 _TRANSP 计划;None = 上述面逐字）。
/// G37 W2 ris_nee:lamp_tbl_bytes = --gi2-ris|--gi2-nee on 面(尾挂 36
/// 〔44〕;transp off 时调用点传 tri_transp 零表保持签名序)。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs_tex_nrm<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    tex_spv: &'x [u8],
    tex: &'x G31TexAssetsHeap,
    trinrm_bytes: &'x [u8],
    tri_mr_bytes: &'x [u8],
    triem_bytes: Option<&'x [u8]>,
    tri_base_bytes: Option<&'x [u8]>,
    nm_bytes: Option<(&'x [u8], &'x [u8])>,
    tri_transp_bytes: Option<&'x [u8]>,
    lamp_tbl_bytes: Option<&'x [u8]>,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> G31Descs<'x> {
    let mut d = g31_lane_descs(
        assets,
        bits,
        enc_spv,
        enc_dispatch,
        enc_params_bytes,
        iw,
        ih,
        ow,
        oh,
        None,
    );
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    d.resources.push(init(&tex.texuv_bytes)); // G31_U_TEX_UV
    d.resources.push(init(&tex.texmeta_bytes)); // G31_U_TEX_META
    d.resources.push(init(&tex.tritex_bytes)); // G31_U_TEX_TRITEX
    d.resources.push(init(&tex.atlas_bytes)); // G31_U_TEX_ATLAS
    d.resources.push(init(&tex.linlut_bytes)); // G31_U_TEX_LINLUT
    d.resources.push(init(trinrm_bytes)); // G31_U_TRINRM_TEX
    d.resources.push(init(tri_mr_bytes)); // G31_U_TRI_MR_TEX
    let mut sb = vec![
        U_TRIS,
        U_MATS,
        U_QUADS,
        U_POINTS,
        U_SCENE_PARAMS,
        G31_U_TRINRM_TEX,
        G31_U_TRI_MR_TEX,
        G31_U_TEX_UV,
        G31_U_TEX_META,
        G31_U_TEX_TRITEX,
        G31_U_TEX_ATLAS,
        G31_U_TEX_LINLUT,
    ];
    if let Some(tb) = triem_bytes {
        d.resources.push(init(tb)); // G31_U_TRIEM_TEXNRM
        sb.push(G31_U_TRIEM_TEXNRM);
        if tri_base_bytes.is_none() {
            // G37 W1:本域计数断言 debug→assert 升级(红修 #2 直接拦截点族)。
            assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_EM);
        }
    } else {
        assert!(
            tri_base_bytes.is_none(),
            "day_0829 realism: tri_base Some 须 triem 同 Some（em off 面调用点传 -1 回退真表——kernel 签名序 fail-closed）"
        );
        assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM);
    }
    // day_0829 realism：tri_base 尾挂 32（kernel 签名序 = triem 之后;None =
    // 既有面逐字 0-byte）。
    if let Some(bb) = tri_base_bytes {
        d.resources.push(init(bb)); // G31_U_TRIBASE_TEXNRM
        sb.push(G31_U_TRIBASE_TEXNRM);
        if nm_bytes.is_none() {
            assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_REAL);
        }
    } else {
        assert!(
            nm_bytes.is_none(),
            "day_0829 臂④: nm Some 须 tri_base 同 Some（realism 链绑定序 fail-closed）"
        );
    }
    // day_0829 臂④：trinm/tri_tan 尾挂 33/34（--normal-maps on 面;None =
    // 上述面逐字 0-byte。G37 W2:transp on 而 nm off 时调用点传回退表/哑表
    // 恒 Some——本断言面不变）。
    if let Some((nmb, tanb)) = nm_bytes {
        d.resources.push(init(nmb)); // G31_U_TRINM_TEXNRM
        sb.push(G31_U_TRINM_TEXNRM);
        d.resources.push(init(tanb)); // G31_U_TRITAN_TEXNRM
        sb.push(G31_U_TRITAN_TEXNRM);
        if tri_transp_bytes.is_none() {
            assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_NM);
        }
    } else {
        assert!(
            tri_transp_bytes.is_none(),
            "G37 W2 transparency: tri_transp Some 须 nm 同 Some（nm off 面调用点传 trinm 回退真表 + tri_tan 哑表——kernel 签名序 fail-closed）"
        );
    }
    // G37 W2 transparency：tri_transp 尾挂 35（kernel 签名序 = tri_tan 之后
    // 新最高链位;None = 上述面逐字 0-byte）。
    if let Some(tp) = tri_transp_bytes {
        d.resources.push(init(tp)); // G31_U_TRITRANSP_TEXNRM
        sb.push(G31_U_TRITRANSP_TEXNRM);
        if lamp_tbl_bytes.is_none() {
            assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_TRANSP);
        }
    } else {
        assert!(
            lamp_tbl_bytes.is_none(),
            "G37 W2 ris_nee: lamp_tbl Some 须 tri_transp 同 Some(transp off 面调用点传 tri_count×0.0 零表——kernel 签名序 fail-closed)"
        );
    }
    // G37 W2 ris_nee:lamp_tbl 尾挂 36(kernel 签名序 = tri_transp 之后新
    // 最高链位;None = 上述面逐字 0-byte)。
    if let Some(lt) = lamp_tbl_bytes {
        d.resources.push(init(lt)); // G31_U_LAMPTBL_TEXNRM
        sb.push(G31_U_LAMPTBL_TEXNRM);
        assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_RIS);
    }
    sb.push(U_SCENE_COLOR);
    sb.push(U_SCENE_DEPTH);
    d.passes[0] = Pass::Compute(ComputePass {
        name: if tri_base_bytes.is_some() {
            "g31_realism"
        } else {
            "g31_texture_nrm_gi"
        },
        spirv: tex_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(bits.scene_dispatch),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: sb,
            ..Bindings::default()
        },
    });
    // G37 W2 ris_nee:lamp_tbl on 面屏障计划最先(TRANSP 超集)。
    d.barriers[0] = if lamp_tbl_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_RIS
    } else if tri_transp_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_TRANSP
    } else if nm_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_NM
    } else if tri_base_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_REAL
    } else if triem_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_EM
    } else {
        G31_U_PLAN_SCENE_TEXNRM
    };
    d
}

/// day_0828 Phase B ② tex×bloom 组合变体描述组（--textures on && --bloom on
/// && --smooth-normals off 面;`g31_lane_descs_bloom` 产物——bloom 八件
/// 24..=31/九 pass 图/encode 读合成缓冲/parity override 面全 0-byte 复用——
/// + tex 五件尾挂 32..36 + scene pass 换 g31_texture_gi）。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs_tex_bloom<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    tex_spv: &'x [u8],
    tex: &'x G31TexAssetsHeap,
    blm: &'x G31BloomAssets<'x>,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> G31Descs<'x> {
    let mut d = g31_lane_descs_bloom(
        assets,
        bits,
        enc_spv,
        enc_dispatch,
        enc_params_bytes,
        blm,
        iw,
        ih,
        ow,
        oh,
    );
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    d.resources.push(init(&tex.texuv_bytes)); // G31_U_TEX_UV_BLOOM
    d.resources.push(init(&tex.texmeta_bytes)); // G31_U_TEX_META_BLOOM
    d.resources.push(init(&tex.tritex_bytes)); // G31_U_TEX_TRITEX_BLOOM
    d.resources.push(init(&tex.atlas_bytes)); // G31_U_TEX_ATLAS_BLOOM
    d.resources.push(init(&tex.linlut_bytes)); // G31_U_TEX_LINLUT_BLOOM
    debug_assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEX_BLOOM);
    d.passes[0] = Pass::Compute(ComputePass {
        name: "g31_texture_gi",
        spirv: tex_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(bits.scene_dispatch),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: vec![
                U_TRIS,
                U_MATS,
                U_QUADS,
                U_POINTS,
                U_SCENE_PARAMS,
                G31_U_TEX_UV_BLOOM,
                G31_U_TEX_META_BLOOM,
                G31_U_TEX_TRITEX_BLOOM,
                G31_U_TEX_ATLAS_BLOOM,
                G31_U_TEX_LINLUT_BLOOM,
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
            ],
            ..Bindings::default()
        },
    });
    d.barriers[0] = G31_U_PLAN_SCENE_TEX_BLOOM;
    d
}

/// day_0828 Phase B ③ tex×nrm×bloom 合流变体描述组（--textures on &&
/// --smooth-normals on && --bloom on 面;`g31_lane_descs_bloom` 产物 + tex
/// 五件尾挂 32..36 + trinrm=37/tri_mr=38 + scene pass 换 g31_texture_nrm_gi
/// 合体 kernel——bloom 下标/parity override 面 0-byte 复用）。
/// day_0828 Phase F：`triem_bytes` = --emissive-tex on 面（Some = triem 尾挂
/// 39 + 绑定插 linlut 之后 + 屏障换 _EM 计划;None = Phase F 前逐字同构）。
/// day_0829 realism：`tri_base_bytes` = realism 任一臂 on 面（Some = tri_base
/// 尾挂 40〔triem 同 Some 律同 tex_nrm 形态〕+ 屏障换 _BLOOM_REAL 计划;
/// None = 既有面逐字同构,0-byte）。
/// day_0829 臂④：`nm_bytes` = --normal-maps on 面（(trinm, tri_tan) 尾挂
/// 41/42 + 屏障换 _BLOOM_NM 计划;None = 上述面逐字）。
/// G37 W2 transparency：`tri_transp_bytes` = --transparency on 面（尾挂 43
/// + 屏障换 _BLOOM_TRANSP 计划;nm_bytes 必 Some 律同 tex_nrm 形态）。
/// G37 W2 ris_nee:lamp_tbl_bytes = --gi2-ris|--gi2-nee on 面(尾挂 36
/// 〔44〕;transp off 时调用点传 tri_transp 零表保持签名序)。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs_tex_nrm_bloom<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    tex_spv: &'x [u8],
    tex: &'x G31TexAssetsHeap,
    trinrm_bytes: &'x [u8],
    tri_mr_bytes: &'x [u8],
    triem_bytes: Option<&'x [u8]>,
    tri_base_bytes: Option<&'x [u8]>,
    nm_bytes: Option<(&'x [u8], &'x [u8])>,
    tri_transp_bytes: Option<&'x [u8]>,
    lamp_tbl_bytes: Option<&'x [u8]>,
    blm: &'x G31BloomAssets<'x>,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> G31Descs<'x> {
    let mut d = g31_lane_descs_bloom(
        assets,
        bits,
        enc_spv,
        enc_dispatch,
        enc_params_bytes,
        blm,
        iw,
        ih,
        ow,
        oh,
    );
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    d.resources.push(init(&tex.texuv_bytes)); // G31_U_TEX_UV_BLOOM
    d.resources.push(init(&tex.texmeta_bytes)); // G31_U_TEX_META_BLOOM
    d.resources.push(init(&tex.tritex_bytes)); // G31_U_TEX_TRITEX_BLOOM
    d.resources.push(init(&tex.atlas_bytes)); // G31_U_TEX_ATLAS_BLOOM
    d.resources.push(init(&tex.linlut_bytes)); // G31_U_TEX_LINLUT_BLOOM
    d.resources.push(init(trinrm_bytes)); // G31_U_TRINRM_TEX_BLOOM
    d.resources.push(init(tri_mr_bytes)); // G31_U_TRI_MR_TEX_BLOOM
    let mut sb = vec![
        U_TRIS,
        U_MATS,
        U_QUADS,
        U_POINTS,
        U_SCENE_PARAMS,
        G31_U_TRINRM_TEX_BLOOM,
        G31_U_TRI_MR_TEX_BLOOM,
        G31_U_TEX_UV_BLOOM,
        G31_U_TEX_META_BLOOM,
        G31_U_TEX_TRITEX_BLOOM,
        G31_U_TEX_ATLAS_BLOOM,
        G31_U_TEX_LINLUT_BLOOM,
    ];
    if let Some(tb) = triem_bytes {
        d.resources.push(init(tb)); // G31_U_TRIEM_TEXNRM_BLOOM
        sb.push(G31_U_TRIEM_TEXNRM_BLOOM);
        if tri_base_bytes.is_none() {
            assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_EM);
        }
    } else {
        assert!(
            tri_base_bytes.is_none(),
            "day_0829 realism: tri_base Some 须 triem 同 Some（em off 面调用点传 -1 回退真表——kernel 签名序 fail-closed）"
        );
        assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_BLOOM);
    }
    // day_0829 realism：tri_base 尾挂 40（kernel 签名序 = triem 之后;None =
    // 既有面逐字 0-byte）。
    if let Some(bb) = tri_base_bytes {
        d.resources.push(init(bb)); // G31_U_TRIBASE_TEXNRM_BLOOM
        sb.push(G31_U_TRIBASE_TEXNRM_BLOOM);
        if nm_bytes.is_none() {
            assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_REAL);
        }
    } else {
        assert!(
            nm_bytes.is_none(),
            "day_0829 臂④: nm Some 须 tri_base 同 Some（realism 链绑定序 fail-closed）"
        );
    }
    // day_0829 臂④：trinm/tri_tan 尾挂 41/42（--normal-maps on 面。G37 W2:
    // transp on 而 nm off 时调用点传回退表/哑表恒 Some——断言面不变）。
    if let Some((nmb, tanb)) = nm_bytes {
        d.resources.push(init(nmb)); // G31_U_TRINM_TEXNRM_BLOOM
        sb.push(G31_U_TRINM_TEXNRM_BLOOM);
        d.resources.push(init(tanb)); // G31_U_TRITAN_TEXNRM_BLOOM
        sb.push(G31_U_TRITAN_TEXNRM_BLOOM);
        if tri_transp_bytes.is_none() {
            assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_NM);
        }
    } else {
        assert!(
            tri_transp_bytes.is_none(),
            "G37 W2 transparency: tri_transp Some 须 nm 同 Some（nm off 面调用点传 trinm 回退真表 + tri_tan 哑表——kernel 签名序 fail-closed）"
        );
    }
    // G37 W2 transparency：tri_transp 尾挂 43（新最高链位）。
    if let Some(tp) = tri_transp_bytes {
        d.resources.push(init(tp)); // G31_U_TRITRANSP_TEXNRM_BLOOM
        sb.push(G31_U_TRITRANSP_TEXNRM_BLOOM);
        if lamp_tbl_bytes.is_none() {
            assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_TRANSP);
        }
    } else {
        assert!(
            lamp_tbl_bytes.is_none(),
            "G37 W2 ris_nee: lamp_tbl Some 须 tri_transp 同 Some(transp off 面调用点传 tri_count×0.0 零表——kernel 签名序 fail-closed)"
        );
    }
    // G37 W2 ris_nee:lamp_tbl 尾挂 44(kernel 签名序 = tri_transp 之后新
    // 最高链位;None = 上述面逐字 0-byte)。
    if let Some(lt) = lamp_tbl_bytes {
        d.resources.push(init(lt)); // G31_U_LAMPTBL_TEXNRM_BLOOM
        sb.push(G31_U_LAMPTBL_TEXNRM_BLOOM);
        assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_RIS);
    }
    sb.push(U_SCENE_COLOR);
    sb.push(U_SCENE_DEPTH);
    d.passes[0] = Pass::Compute(ComputePass {
        name: if tri_base_bytes.is_some() {
            "g31_realism"
        } else {
            "g31_texture_nrm_gi"
        },
        spirv: tex_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(bits.scene_dispatch),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: sb,
            ..Bindings::default()
        },
    });
    // G37 W2 ris_nee:lamp_tbl on 面屏障计划最先(BLOOM_TRANSP 超集)。
    d.barriers[0] = if lamp_tbl_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_BLOOM_RIS
    } else if tri_transp_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_BLOOM_TRANSP
    } else if nm_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_BLOOM_NM
    } else if tri_base_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_BLOOM_REAL
    } else if triem_bytes.is_some() {
        G31_U_PLAN_SCENE_TEXNRM_BLOOM_EM
    } else {
        G31_U_PLAN_SCENE_TEXNRM_BLOOM
    };
    d
}

/// C13 SVT 变体描述组（--svt on 面;`g31_lane_descs_tex` 产物逐项克隆 +
/// scene pass 换 g31_svt_gi 变体〔绑定面 = B4 纹理面减 atlas 直绑 + C13 五件,
/// 声明序与 kernels/g31_svt_gi.rx 签名逐字同源〕+ 资源追加 29..=33 + scene
/// 屏障换 G31_U_PLAN_SCENE_SVT + readback 追加 miss 请求缓冲（下标 5）——
/// 既有资源/pass/屏障/readback 各面 0-byte,svt off 面不构造）。
///
/// pagetable/pool 二件 = host-visible（G14.10d 判定规则:逐帧
/// `FrameUpdate.buffer_uploads` 目标 ⇒ device_local:false）;pagetable 初态
/// 字节 = 流送状态当前页表影（era 重建再同步面）,pool 初态字节 = host 池
/// 影（全驻留臂 = 全瓦片集,冷臂 = 全零——页表全空 ⇒ 池永不被未驻留读）。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs_svt<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    svt_spv: &'x [u8],
    tex: &'x G31TexAssetsHeap,
    svt: &'x G31SvtAssets,
    pagetable_bytes: &'x [u8],
    pool_bytes: &'x [u8],
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> G31Descs<'x> {
    let mut d = g31_lane_descs_tex(
        assets,
        bits,
        enc_spv,
        enc_dispatch,
        enc_params_bytes,
        svt_spv,
        tex,
        iw,
        ih,
        ow,
        oh,
    );
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let host_init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: false,
        })
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let ipc = (iw as u64) * (ih as u64);
    d.resources.push(host_init(pagetable_bytes)); // G31_U_SVT_PAGETABLE（逐帧上传面）
    d.resources.push(host_init(pool_bytes)); // G31_U_SVT_POOL（逐帧上传面）
    d.resources.push(ResourceDesc::Buffer(BufferDesc {
        size: ipc * 4,
        usage: storage,
        data: None,
        device_local: true,
    })); // G31_U_SVT_REQ（kernel 全屏直写,帧间零状态）
    d.resources.push(init(&svt.svtmeta_bytes)); // G31_U_SVT_META
    d.resources.push(init(&svt.fallback_bytes)); // G31_U_SVT_FALLBACK
    debug_assert_eq!(d.resources.len(), G31_U_RESOURCE_COUNT_SVT);
    d.passes[0] = Pass::Compute(ComputePass {
        name: "g31_svt_gi",
        spirv: svt_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(bits.scene_dispatch),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: vec![
                U_TRIS,
                U_MATS,
                U_QUADS,
                U_POINTS,
                U_SCENE_PARAMS,
                G31_U_TEX_UV,
                G31_U_TEX_META,
                G31_U_TEX_TRITEX,
                G31_U_TEX_LINLUT,
                G31_U_SVT_META,
                G31_U_SVT_FALLBACK,
                G31_U_SVT_PAGETABLE,
                G31_U_SVT_POOL,
                G31_U_SVT_REQ,
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
            ],
            ..Bindings::default()
        },
    });
    d.barriers[0] = G31_U_PLAN_SCENE_SVT;
    d.readbacks.push(Readback::Buffer {
        res: G31_U_SVT_REQ,
        offset: 0,
        size: ipc * 4,
    });
    debug_assert_eq!(d.readbacks.len() as u32, G31_RB_SVT_REQ + 1);
    d
}

/// A3 逐帧回读模式（常态 = BGRA8 8.3MB;末帧追加 f32 out_color 供
/// render_digest——与 bench 末帧回读同律;None = 零回读,headless 中间帧面）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum G31Readback {
    None,
    Bgra,
    BgraAndColor,
}

/// A3 一帧产物（五 pass GPU 分段 + BGRA8/f32 回读 + 校验计数）。A5:fg on 追加
/// FG pass GPU 分段 + 生成帧 BGRA8（时序升序）+ probe 帧 f32 三路回读。
struct G31FrameRec {
    scene_gpu_ns: f64,
    mv_gpu_ns: f64,
    resample_gpu_ns: f64,
    resolve_gpu_ns: f64,
    encode_gpu_ns: f64,
    /// A5:FG pass GPU 合计（fg1/fg2 kernel + 各自 encode;fg off 恒 0）。
    fg_gpu_ns: f64,
    cpu_record_ns: u64,
    cpu_submit_ns: u64,
    cpu_fence_wait_ns: u64,
    validation_error_count: u64,
    /// 所有权账本外 object/allocation 数（成功帧必须为 0——资源无泄漏机核面,
    /// 逐帧断言不累积到退出才查）。
    leaked_object_count: u64,
    leaked_allocation_count: u64,
    bgra8: Option<Vec<u8>>,
    /// A5:本帧生成帧 BGRA8（时序升序;gen 非活跃帧 = 空）。
    gen_bgra8: Vec<Vec<u8>>,
    out_color: Option<Vec<f32>>,
    /// A5 probe 帧三路：prev f32 / MV f32 / 生成帧 f32（时序升序;非 probe = 空）。
    probe_prev_color: Option<Vec<f32>>,
    probe_mv: Option<Vec<f32>>,
    probe_gen_out: Vec<Vec<f32>>,
    /// A5 probe 帧 MVN 取反结果（device 侧 MV 内容直比对面;非 probe = None）。
    probe_mvn: Option<Vec<f32>>,
    readback_convert_ms: f64,
    /// B1 HZB 决策/调度块（--hzb on 面;off = None,既有面 0-byte）。
    hzb: Option<G31HzbFrameRec>,
    /// C13 SVT miss 请求缓冲回读（--svt on 面 = iw·ih f32;off = None,既有面 0-byte）。
    svt_requests: Option<Vec<u8>>,
    /// C7 profiler 面：本帧全量逐 pass GPU 计时（telemetry 声明序;(pass 名, ns)）。
    /// 五段提取面 0-byte——既有字段全部同值维持,本列为 --profile-json 唯一消费面。
    pass_gpu_ns: Vec<(String, f64)>,
}

/// C7 profiler 逐帧记录（--profile-json 收集面;post-warmup 测量窗与 render_ms 同口径）。
struct G31ProfileFrame {
    /// 全量逐 pass GPU 毫秒（telemetry 声明序）。
    passes: Vec<(String, f64)>,
    cpu_record_ms: f64,
    cpu_submit_ms: f64,
    cpu_fence_wait_ms: f64,
    readback_convert_ms: f64,
    render_wall_ms: f64,
    present_wall_ms: f64,
    digest_ms: f64,
}

/// C13 SVT 逐帧状态（--svt on 面;车道创建期挂载,逐帧流送闭环消费面）。
struct G31SvtLaneState {
    /// 次帧上传段（页表写段 + 瓦片上传段;主循环 consume 产出,prepare_update 消费）。
    pending: Vec<(StableResourceId, u64, Vec<u8>)>,
    /// miss 请求缓冲字节数（iw·ih·4;rec_from_output 校验面）。
    req_bytes: usize,
}

/// C13 SVT 逐帧流送统计（evidence 面;svt on 才消费,off = 全零默认）。
#[derive(Default)]
struct G31SvtStats {
    /// 逐帧 miss 像素数（= fallback 像素数;请求缓冲非零项计数）。
    miss_px: Vec<u32>,
    /// 逐帧去重 miss 页数。
    unique_pages: Vec<u32>,
    /// 逐帧新入池瓦片数。
    loaded: Vec<u32>,
    /// 逐帧驱逐瓦片数。
    evicted: Vec<u32>,
    miss_px_total: u64,
    requested_pages_total: u64,
    tiles_loaded_total: u64,
    tiles_evicted_total: u64,
    io_bytes_total: u64,
    /// fallback 像素 >0 的帧数。
    fallback_frames: u32,
}

/// A3 五 pass 车道状态机（parity/历史门/prev_vp_j 与 UnifiedTsrLane 逐字同律;
/// 差异 = 五 pass 描述组 + encode binding parity 轮换 + BGRA8 回读面）。A5:
/// fg 档 + readback 布局入状态机（fg pass binding parity 轮换同律追加）。C13:
/// svt 档 + 逐帧上传段/miss 请求回读路入状态机。
struct G31TsrLane<'a> {
    session: DeviceFrameSession<'a>,
    parity: usize,
    has_history_state: bool,
    prev_vp_j: Option<Mat4>,
    fg: G31Fg,
    fg_layout: G31FgLayout,
    /// G37 W3 fg_combo：fg × --quality full 组合面（comp parity 双缓冲——
    /// composite/encode/AE reduce/FG 逐帧同 parity 槽 override;false = base
    /// U_OUT_COLOR 对现状 0-byte）。
    fg_full: bool,
    /// G37 W3 fg_combo：FG pass 下标（变体感知——base 图 fg1=6/fg2=8,full 图
    /// 12/14;创建期按 pass 名派生防写死漂移,fg off = MAX 不消费）。
    fg_pass_fg1: u32,
    fg_pass_fg2: u32,
    /// scene pass telemetry 名（B4:textures on = "g31_texture_gi" 变体,off =
    /// "g14_3_direct_gi"——descs 声明面直取,telemetry 按名提取同律）。
    scene_pass_name: &'a str,
    /// C13 SVT 状态（svt on = Some;off = None,既有面 0-byte）。
    svt: Option<G31SvtLaneState>,
    /// D3 bloom 加性臂（bloom on = true,prepare_update 对 bright/composite/
    /// encode 三 pass 按 9 pass 图 override;off = false,既有五 pass 面 0-byte）。
    bloom: bool,
    /// D2 平滑顶点法线臂（--smooth-normals on = true → prepare_update 经
    /// pack_frame_params_nrm 置 params[43]=1.0（+RURIX_G18_AMBIENT 门控的
    /// params[44..48) 半球环境光同面）;off = false,参数面 0-byte）。
    smooth_nrm: bool,
    /// D6 GGX 高光臂（--ggx on = true → prepare_update 经
    /// pack_frame_params_ggx 置 params[48]=1.0〔须 smooth_nrm 同 on,pack 面
    /// 第二重保险;CLI 已裁〕;off = false,参数面 0-byte）。
    ggx: bool,
    /// A1 灯贡献剔除阈值（--lamp-lights on 车道创建后经 set_lamp_contrib
    /// 一次性挂载 → prepare_update 置 params[49];off = 恒 0.0 与零填充
    /// 逐位同值,参数面 0-byte）。
    lamp_contrib: f32,
    /// day_0828 Phase B 纹理 mip 锥角（--textures on 车道创建后经
    /// set_tex_kpix 一次性挂载 → prepare_update 置 params[50];off = 恒 0.0
    /// 与零填充逐位同值,参数面 0-byte）。
    tex_kpix: f32,
    /// A2 自动曝光臂（--auto-exposure on 车道创建后经 set_autoexp 挂载
    /// (params_idx, partials_idx) 变体族下标 → prepare_update 对 reduce pass
    /// 做 parity override〔非 bloom 面,读 encode 同源 U_OUT_COLOR[p]〕并把
    /// encode override 下标右移 2〔reduce/state 两 pass 插入〕;off = None,
    /// 既有 override 面 0-byte）。
    autoexp: Option<(u32, u32)>,
    /// Phase C GI2 臂（--gi2 on 车道创建后经 set_gi2 一次性挂载 scale/clamp
    /// + 逐帧 set_gi2_frame 挂载帧序号〔R2 时域旋转〕→ prepare_update 置
    /// params[51..55);off = false ⇒ 四槽不写与零填充逐位同值,参数面 0-byte）。
    gi2: bool,
    gi2_scale: f32,
    gi2_clamp: f32,
    gi2_frame: f32,
    /// day_0829 臂① 金属 F0 修伤（--metal-f0 on 车道创建后经 set_metal_f0
    /// 一次性挂载 → prepare_update 置 params[55]=1.0 并扩 params 到
    /// G31_REAL_PARAMS_LEN;off = false ⇒ 不扩不写,参数面 0-byte）。
    metal_f0: bool,
    /// day_0829 臂② 短程 RT AO（--rt-ao on 车道创建后经 set_rt_ao 一次性
    /// 挂载 → prepare_update 置 params[56..60) + [52]=frame_idx〔R2 时域旋转,
    /// gi2 off 时由本臂补写〕;off = false ⇒ 四槽不写,参数面 0-byte）。
    rt_ao: bool,
    rt_ao_radius: f32,
    rt_ao_strength: f32,
    rt_ao_samples: f32,
    /// day_0829 臂⑤ 点光软阴影（--soft-shadows on 车道创建后经
    /// set_soft_shadows 一次性挂载 → prepare_update 置 params[60..62) +
    /// [52]=frame_idx 补写;off = false ⇒ 两槽不写,参数面 0-byte）。
    soft_shadows: bool,
    soft_shadow_samples: f32,
    /// day_0829 臂③ 光追反射（--rt-reflect on 车道创建后经 set_rt_reflect
    /// 一次性挂载 → prepare_update 置 params[62..65) + [52] 补写;off =
    /// false ⇒ 三槽不写,参数面 0-byte）。
    rt_reflect: bool,
    rt_reflect_rough_max: f32,
    rt_reflect_clamp: f32,
    /// day_0829 臂⑥ GI2 贴图反弹（--gi2-tex on 车道创建后经 set_gi2_tex
    /// 一次性挂载 → prepare_update 置 params[67]=1.0;off = false ⇒ 不写,
    /// 参数面 0-byte）。
    gi2_tex: bool,
    /// day_0829 臂④ 法线贴图（--normal-maps on 车道创建后经 set_normal_maps
    /// 一次性挂载 → prepare_update 置 params[65..67);off = false ⇒ 两槽
    /// 不写,参数面 0-byte）。
    normal_maps: bool,
    normal_strength: f32,
    /// G37 W2 臂⑦ 玻璃透射（--transparency on 车道创建后经 set_transparency
    /// 一次性挂载 → prepare_update 置 params[68]=1.0 并入 realism 扩面;off =
    /// false ⇒ 不写,参数面 0-byte）。
    transparency: bool,
    /// G37 W2 臂⑧ GI2 反弹 RIS/NEE(--gi2-ris/--gi2-nee on 车道创建后经
    /// set_gi2_ris 一次性挂载 → prepare_update 置 params[69..72);off =
    /// false ⇒ 三槽不写,参数面 0-byte)。
    gi2_ris: bool,
    gi2_ris_m: f32,
    gi2_nee: bool,
    /// Phase D TSR 降噪质量档（--tsr-quality on 车道创建后经 set_tsrq 一次性
    /// 挂载 min_alpha/clamp → prepare_update 置 tsr_params[19..21);off =
    /// false ⇒ 两槽不写与零填充逐位同值,参数面 0-byte——resolve SPV 换载在
    /// CLI 面完成,字节隔离）。
    tsrq: bool,
    tsrq_min_alpha: f32,
    tsrq_clamp: f32,
}

impl<'a> G31TsrLane<'a> {
    fn create(
        descs: &'a G31Descs<'a>,
        accel_structs: &[AccelStructDesc<'a>],
        fg: G31Fg,
        bloom: bool,
        smooth_nrm: bool,
        ggx: bool,
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let scene_pass_name = match descs.passes.first() {
            Some(Pass::Compute(cp)) => cp.name,
            _ => return Err("descs 首 pass 非 compute（scene pass 门面）".into()),
        };
        // G37 W3 fg_combo：FG pass 下标按名派生 + fg_full 判定（两点式闭集下
        // fg on × bloom on ⇔ full 组合面——CLI 卫兵已裁散臂混搭）。
        let mut fg_pass_fg1 = u32::MAX;
        let mut fg_pass_fg2 = u32::MAX;
        for (k, p) in descs.passes.iter().enumerate() {
            if let Pass::Compute(cp) = p {
                match cp.name {
                    "g26_framegen_fg1" => fg_pass_fg1 = k as u32,
                    "g26_framegen_fg2" => fg_pass_fg2 = k as u32,
                    _ => {}
                }
            }
        }
        if fg != G31Fg::Off && fg_pass_fg1 == u32::MAX {
            return Err("fg on 但 descs 缺 g26_framegen_fg1 pass（FG 接线面不完整）".into());
        }
        if fg == G31Fg::X3 && fg_pass_fg2 == u32::MAX {
            return Err("fg x3 但 descs 缺 g26_framegen_fg2 pass（FG 接线面不完整）".into());
        }
        let fg_full = fg != G31Fg::Off && bloom;
        // frame_slots=2（与 UnifiedTsrLane inflight=1 创建面逐字同——顺序全同步
        // 口径;FIF 流水化非本任务面,g31_frame_pipelining 门覆盖）。
        let session = DeviceFrameSession::new_with_accel_structs(
            &descs.resources,
            &descs.passes,
            &descs.barriers,
            &descs.readbacks,
            2,
            accel_structs,
        )?;
        Ok(Self {
            session,
            parity: 0,
            has_history_state: false,
            prev_vp_j: None,
            fg,
            fg_layout: if fg_full {
                G31FgLayout::of_full(fg)
            } else {
                G31FgLayout::of(fg)
            },
            fg_full,
            fg_pass_fg1,
            fg_pass_fg2,
            scene_pass_name,
            svt: None,
            bloom,
            smooth_nrm,
            ggx,
            lamp_contrib: 0.0,
            tex_kpix: 0.0,
            autoexp: None,
            gi2: false,
            gi2_scale: 0.0,
            gi2_clamp: 0.0,
            gi2_frame: 0.0,
            metal_f0: false,
            rt_ao: false,
            rt_ao_radius: 0.0,
            rt_ao_strength: 0.0,
            rt_ao_samples: 0.0,
            soft_shadows: false,
            soft_shadow_samples: 0.0,
            rt_reflect: false,
            rt_reflect_rough_max: 0.0,
            rt_reflect_clamp: 0.0,
            gi2_tex: false,
            normal_maps: false,
            normal_strength: 0.0,
            transparency: false,
            gi2_ris: false,
            gi2_ris_m: 6.0,
            gi2_nee: false,
            tsrq: false,
            tsrq_min_alpha: 0.0,
            tsrq_clamp: 0.0,
        })
    }

    /// A1 灯贡献剔除阈值挂载（--lamp-lights on 车道创建后一次性;off 车道
    /// 不调用 ⇒ 恒 0.0 参数面 0-byte）。
    fn set_lamp_contrib(&mut self, contrib: f32) {
        self.lamp_contrib = contrib;
    }

    /// day_0828 Phase B 纹理 mip 锥角挂载（--textures on 车道创建后一次性;
    /// off 车道不调用 ⇒ 恒 0.0 参数面 0-byte）。
    fn set_tex_kpix(&mut self, kpix: f32) {
        self.tex_kpix = kpix;
    }

    /// Phase C GI2 臂挂载（--gi2 on 车道创建后一次性 scale/clamp;off 车道
    /// 不调用 ⇒ gi2=false 四槽不写参数面 0-byte）。
    fn set_gi2(&mut self, scale: f32, clamp: f32) {
        self.gi2 = true;
        self.gi2_scale = scale;
        self.gi2_clamp = clamp;
    }

    /// Phase C GI2 帧序号逐帧挂载（params[52]=frame_idx——R2 时域旋转,TSR
    /// 收敛面;双跑同帧序 ⇒ 位级一致口径不破。off 车道不调用零消费）。
    fn set_gi2_frame(&mut self, frame_idx: f32) {
        self.gi2_frame = frame_idx;
    }

    /// day_0829 臂① 金属 F0 修伤挂载（--metal-f0 on 车道创建后一次性;off
    /// 车道不调用 ⇒ params 不扩不写,参数面 0-byte）。
    fn set_metal_f0(&mut self) {
        self.metal_f0 = true;
    }

    /// day_0829 臂② 短程 RT AO 挂载（--rt-ao on 车道创建后一次性;off 车道
    /// 不调用 ⇒ 四槽不写,参数面 0-byte）。
    fn set_rt_ao(&mut self, radius: f32, strength: f32, samples: f32) {
        self.rt_ao = true;
        self.rt_ao_radius = radius;
        self.rt_ao_strength = strength;
        self.rt_ao_samples = samples;
    }

    /// day_0829 臂⑤ 点光软阴影挂载（--soft-shadows on 车道创建后一次性;off
    /// 车道不调用 ⇒ 两槽不写,参数面 0-byte）。
    fn set_soft_shadows(&mut self, samples: f32) {
        self.soft_shadows = true;
        self.soft_shadow_samples = samples;
    }

    /// day_0829 臂③ 光追反射挂载（--rt-reflect on 车道创建后一次性;off
    /// 车道不调用 ⇒ 三槽不写,参数面 0-byte）。
    fn set_rt_reflect(&mut self, rough_max: f32, clamp: f32) {
        self.rt_reflect = true;
        self.rt_reflect_rough_max = rough_max;
        self.rt_reflect_clamp = clamp;
    }

    /// day_0829 臂⑥ GI2 贴图反弹挂载（--gi2-tex on 车道创建后一次性;off
    /// 车道不调用 ⇒ 不写,参数面 0-byte）。
    fn set_gi2_tex(&mut self) {
        self.gi2_tex = true;
    }

    /// day_0829 臂④ 法线贴图挂载（--normal-maps on 车道创建后一次性;off
    /// 车道不调用 ⇒ 两槽不写,参数面 0-byte）。
    fn set_normal_maps(&mut self, strength: f32) {
        self.normal_maps = true;
        self.normal_strength = strength;
    }

    /// G37 W2 臂⑦ 玻璃透射挂载（--transparency on 车道创建后一次性;off
    /// 车道不调用 ⇒ 不写,参数面 0-byte。透射率在 tri_transp 侧表逐三角,
    /// params 只带门）。
    fn set_transparency(&mut self) {
        self.transparency = true;
    }

    /// G37 W2 臂⑧ GI2 反弹 RIS/NEE 挂载(任一 on 车道创建后一次性;off
    /// 车道不调用 ⇒ 不写,参数面 0-byte)。
    fn set_gi2_ris(&mut self, ris: bool, ris_m: f32, nee: bool) {
        self.gi2_ris = ris;
        self.gi2_ris_m = ris_m;
        self.gi2_nee = nee;
    }

    /// Phase D TSR 降噪质量档挂载（--tsr-quality on 车道创建后一次性
    /// min_alpha/clamp → tsr_params[19..21);off 车道不调用 ⇒ 两槽不写
    /// 参数面 0-byte——SPV 换载归 CLI 面,本挂载仅参数槽）。
    fn set_tsrq(&mut self, min_alpha: f32, clamp: f32) {
        self.tsrq = true;
        self.tsrq_min_alpha = min_alpha;
        self.tsrq_clamp = clamp;
    }

    /// A2 自动曝光挂载（--auto-exposure on 车道创建后一次性;(params_idx,
    /// partials_idx) = 变体族 A2 下标——off 车道不调用 ⇒ override 面 0-byte）。
    fn set_autoexp(&mut self, params_idx: u32, partials_idx: u32) {
        self.autoexp = Some((params_idx, partials_idx));
    }

    /// C13 SVT 状态挂载（svt on 车道创建后一次性;req_bytes = iw·ih·4）。
    fn set_svt(&mut self, req_bytes: usize) {
        self.svt = Some(G31SvtLaneState {
            pending: Vec::new(),
            req_bytes,
        });
    }

    /// C13 次帧上传段写入（主循环 consume 产出;prepare_update 全量消费并清空）。
    fn set_svt_pending(&mut self, pending: Vec<(StableResourceId, u64, Vec<u8>)>) {
        if let Some(s) = self.svt.as_mut() {
            s.pending = pending;
        }
    }

    /// A5 本帧生成是否活跃（fg on 且有 prev 真渲帧对——首帧/resize era 首帧
    /// 无 prev,跳过生成不消费;FG pass 仍随固定图执行但输出面不读不 present,
    /// 真实渲染帧内容零影响）。
    fn gen_active(&self, reset: bool) -> bool {
        self.fg != G31Fg::Off && !reset && self.has_history_state
    }

    /// 本帧 FrameUpdate + provenance 组装（三小件参数打包 + parity 轮换
    /// resample/resolve/encode 三 pass binding_overrides + readback 子集;
    /// 与 UnifiedTsrLane::prepare_update 同律,追加 pass4 一项）。A5:fg on
    /// 追加 fg pass parity override（prev/cur 双缓冲绑定轮换）+ readback 子集
    /// 组合（cur BGRA8 → 生成帧 BGRA8 → cur f32 → probe 三路）。
    #[allow(clippy::too_many_arguments)]
    fn prepare_update(
        &self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        eps: f32,
        quad_count: usize,
        point_count: usize,
        inv_vp: &Mat4,
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback: G31Readback,
        probe: bool,
    ) -> Result<(SubmissionProvenance, FrameUpdate), String> {
        // D2：smooth_nrm 车道经 pack_frame_params_nrm 置 params[43]=1.0
        //（+RURIX_G18_AMBIENT 门控 params[44..48) 半球环境光）;off 车道
        // smooth_nrm=false ⇒ 参数面与既有 pack_frame_params 逐位同值 0-byte。
        // D6：ggx=true（--ggx on 车道）且 smooth_nrm 同 on → params[48]=1.0;
        // ggx=false 面产物与 D6 前逐位同值（0-byte）。
        // A1：lamp_contrib（--lamp-lights on 车道挂载）→ params[49];默认
        // 0.0 与零填充逐位同值（0-byte）。
        // day_0828 Phase B：tex_kpix（--textures on 车道挂载）→ params[50];
        // 默认 0.0 同 0-byte 律。
        // Phase C：gi2（--gi2 on 车道 set_gi2/set_gi2_frame 挂载）→
        // params[51..55);false 面四槽不写与零填充逐位同值（0-byte）。
        let mut scene_params = pack_frame_params_gi2(
            iw,
            ih,
            jitter,
            eps,
            quad_count,
            point_count,
            inv_vp,
            vp,
            self.smooth_nrm,
            self.ggx,
            self.lamp_contrib,
            self.tex_kpix,
            self.gi2,
            self.gi2_frame,
            self.gi2_clamp,
            self.gi2_scale,
        );
        // day_0829 realism params 扩面（任一 realism 臂 on 才扩到
        // G31_REAL_PARAMS_LEN 并写各自门槽;全 off = 不扩不写,产物与既有
        // pack_frame_params_gi2 逐位同值 0-byte——params buffer 大小同门扩容
        // 见 lane_assets 后置块）。
        if self.metal_f0
            || self.rt_ao
            || self.soft_shadows
            || self.rt_reflect
            || self.gi2_tex
            || self.normal_maps
            // G37 W2 transparency:臂⑦并入 realism 扩面门。
            || self.transparency
            // G37 W2 ris_nee:臂⑧并入 realism 扩面门。
            || self.gi2_ris
            || self.gi2_nee
        {
            scene_params.resize(G31_REAL_PARAMS_LEN, 0.0);
            if self.metal_f0 {
                scene_params[55] = 1.0;
            }
            if self.rt_ao {
                scene_params[56] = 1.0;
                scene_params[57] = self.rt_ao_radius;
                scene_params[58] = self.rt_ao_strength;
                scene_params[59] = self.rt_ao_samples;
            }
            if self.soft_shadows {
                scene_params[60] = 1.0;
                scene_params[61] = self.soft_shadow_samples;
            }
            if self.rt_reflect {
                scene_params[62] = 1.0;
                scene_params[63] = self.rt_reflect_rough_max;
                scene_params[64] = self.rt_reflect_clamp;
            }
            if self.gi2_tex {
                scene_params[67] = 1.0;
            }
            if self.normal_maps {
                scene_params[65] = 1.0;
                scene_params[66] = self.normal_strength;
            }
            // G37 W2 transparency:params[68] 门(透射率在 tri_transp 侧表)。
            if self.transparency {
                scene_params[68] = 1.0;
            }
            // G37 W2 ris_nee:params[69..72)(RIS 门/候选数/NEE 门;[52]
            // 帧旋转由 gi2 pack 已写——CLI 已裁须随 --gi2 on)。
            if self.gi2_ris {
                scene_params[69] = 1.0;
                scene_params[70] = self.gi2_ris_m;
            }
            if self.gi2_nee {
                scene_params[71] = 1.0;
            }
            // 时序采样帧旋转（gi2 off 时 pack 未写 [52],时序臂补写——gi2
            // on 时 pack 已写同值,幂等）。
            if self.rt_ao || self.soft_shadows || self.rt_reflect {
                scene_params[52] = self.gi2_frame;
            }
        }
        // mv 参数面:inv_cur = vp_j 逆(host Mat4::inverse 伴随法);prev = 上帧
        // vp_j;首帧 has_prev=0,kernel 门直写零——与统一车道逐字同律。
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆（mv 参数面）")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        let has_history = !reset && self.has_history_state;
        // Phase D：self.tsrq（--tsr-quality on 车道 set_tsrq 挂载）→
        // tsr_params[19..21)〔[19]=稳态 alpha 档/[20]=邻域 clamp K〕；false
        // 面两槽不写与零填充逐位同值（0-byte——冻结 resolve kernel 不读
        // [19..21)，仅 g31_tsr_resolve_q 变体消费）。
        let mut tsr_params = pack_tsr_params(iw, ih, ow, oh, jitter, exposure, has_history, false);
        if self.tsrq {
            tsr_params[19] = self.tsrq_min_alpha;
            tsr_params[20] = self.tsrq_clamp;
        }
        let p = self.parity;
        // G37 W3 fg_combo：fg_full 面本帧 comp parity 槽（composite 写/encode
        // 读/AE reduce 读/FG cur 同源;fg off / base fg 面恒 COMP_OUT 静态
        // 现状 0-byte 不轮换）。
        let comp_p = if self.fg_full && p == 1 {
            G31_U_BLOOM_COMP_HIST_FULL
        } else {
            G31_U_BLOOM_COMP_OUT
        };
        let mut uploads: Vec<(StableResourceId, u64, Vec<u8>)> = vec![
            (
                StableResourceId(u64::from(U_SCENE_PARAMS) + 1),
                0,
                bytes_f32(&scene_params),
            ),
            (
                StableResourceId(u64::from(U_MV_PARAMS) + 1),
                0,
                bytes_f32(&mv_params),
            ),
            (
                StableResourceId(u64::from(U_TSR_PARAMS) + 1),
                0,
                bytes_f32(&tsr_params),
            ),
        ];
        // C13 SVT:次帧上传段合入（页表写段 + 瓦片上传段;主循环 consume 产出）。
        if let Some(s) = self.svt.as_ref() {
            uploads.extend(s.pending.iter().cloned());
        }
        let bindings_resample = Bindings {
            storage_buffers: vec![
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
                U_TSR_PARAMS,
                U_CUR_RGB,
                U_LUMA[p],
                U_DEPTH_HI[p],
            ],
            ..Bindings::default()
        };
        let bindings_resolve = Bindings {
            storage_buffers: vec![
                U_CUR_RGB,
                U_LUMA[p],
                U_DEPTH_HI[p],
                U_MV_OUT,
                U_REACTIVE,
                U_OUT_COLOR[1 - p],
                U_DEPTH_HI[1 - p],
                U_LUMA[1 - p],
                U_OUT_SIGN[1 - p],
                U_OUT_SCORE[1 - p],
                U_TSR_PARAMS,
                U_OUT_COLOR[p],
                U_OUT_SIGN[p],
                U_OUT_SCORE[p],
            ],
            ..Bindings::default()
        };
        // encode 读本帧 resolve 写出的 U_OUT_COLOR[p](parity 轮换同律)。D3:
        // bloom on 时 encode 改读 composite 合成输出（静态绑定）,bright/
        // composite 读 U_OUT_COLOR[p]（parity 轮换同律）。G37 W3 fg_combo：
        // fg_full 面 encode 改读 comp[p]（comp parity 轮换;fg off 面 comp_p
        // 恒 COMP_OUT = 既有静态语义 0-byte）。
        let bindings_encode = Bindings {
            storage_buffers: if self.bloom {
                vec![comp_p, G31_U_ENC_PARAMS, G31_U_ENC_OUT]
            } else {
                vec![U_OUT_COLOR[p], G31_U_ENC_PARAMS, G31_U_ENC_OUT]
            },
            ..Bindings::default()
        };
        let mut binding_overrides = vec![
            (2, bindings_resample),
            (3, bindings_resolve),
        ];
        if self.bloom {
            // D3 九 pass 图:4=bright/5=blurH/6=blurV/7=composite/8=encode
            // （blur 两 pass 绑定静态,不随 parity 轮换）。A2 on = 十一 pass 图:
            // 8=reduce（读 comp_out 静态,不 override）/9=state（静态）/
            // 10=encode——encode override 下标右移 2。G37 W3 fg_combo：fg_full
            // 面 composite 出口/encode 入口换 comp[p],reduce 新增 override 读
            // comp[p]（fg off 面 comp_p 恒 COMP_OUT ⇒ composite/encode 与既有
            // 静态语义逐字同值,reduce 不 override——三处 0-byte）。
            binding_overrides.push((
                4,
                Bindings {
                    storage_buffers: vec![
                        U_OUT_COLOR[p],
                        G31_U_BLOOM_BRIGHT_PARAMS,
                        G31_U_BLOOM_BRIGHT,
                    ],
                    ..Bindings::default()
                },
            ));
            binding_overrides.push((
                7,
                Bindings {
                    storage_buffers: vec![
                        U_OUT_COLOR[p],
                        G31_U_BLOOM_PONG,
                        G31_U_BLOOM_COMP_PARAMS,
                        comp_p,
                    ],
                    ..Bindings::default()
                },
            ));
            if self.fg_full {
                if let Some((ae_params, ae_partials)) = self.autoexp {
                    binding_overrides.push((
                        8,
                        Bindings {
                            storage_buffers: vec![comp_p, ae_params, ae_partials],
                            ..Bindings::default()
                        },
                    ));
                }
            }
            binding_overrides.push((
                if self.autoexp.is_some() { 10 } else { 8 },
                bindings_encode,
            ));
        } else if let Some((ae_params, ae_partials)) = self.autoexp {
            // A2 七 pass 图:4=reduce（读本帧 resolve 写出的 U_OUT_COLOR[p],
            // encode in_color 同源 parity 轮换）/5=state（绑定静态）/6=encode。
            binding_overrides.push((
                4,
                Bindings {
                    storage_buffers: vec![U_OUT_COLOR[p], ae_params, ae_partials],
                    ..Bindings::default()
                },
            ));
            binding_overrides.push((6, bindings_encode));
        } else {
            binding_overrides.push((4, bindings_encode));
        }
        // A5:fg pass parity override——取反 glue 直通馈入:kernel(prev:=
        // U_OUT_COLOR[1−p](上帧 prev),cur:=U_OUT_COLOR[p](本帧 cur),mv:=
        // G31_U_MVN(g14 相机 MV 取反后 = host 约定形),t:=t_temporal（参数创建
        // 期定)）≡ host interpolate(prev, cur, −mv_g14, t) 逐字同语义（文件头
        // A5 §2;encode_fg/mvn 绑定静态不轮换）。
        if self.fg != G31Fg::Off {
            // G37 W3 fg_combo：pass 下标变体感知（base 图 6/8,full 图 12/14——
            // 创建期按名派生）;fg_full 面 prev/cur = comp 对（FG 插值
            // post-bloom 合成帧,base 面维持 U_OUT_COLOR 对 0-byte）。
            let (fg_prev, fg_cur, fg_mvn, fg1_params, fg1_out) = if self.fg_full {
                (
                    if p == 1 {
                        G31_U_BLOOM_COMP_OUT
                    } else {
                        G31_U_BLOOM_COMP_HIST_FULL
                    },
                    comp_p,
                    G31_U_MVN_FULL,
                    G31_U_FG1_PARAMS_FULL,
                    G31_U_FG1_OUT_FULL,
                )
            } else {
                (
                    U_OUT_COLOR[1 - p],
                    U_OUT_COLOR[p],
                    G31_U_MVN,
                    G31_U_FG1_PARAMS,
                    G31_U_FG1_OUT,
                )
            };
            binding_overrides.push((
                self.fg_pass_fg1,
                Bindings {
                    storage_buffers: vec![fg_prev, fg_cur, fg_mvn, fg1_params, fg1_out],
                    ..Bindings::default()
                },
            ));
            if self.fg == G31Fg::X3 {
                let (fg2_params, fg2_out) = if self.fg_full {
                    (G31_U_FG2_PARAMS_FULL, G31_U_FG2_OUT_FULL)
                } else {
                    (G31_U_FG2_PARAMS, G31_U_FG2_OUT)
                };
                binding_overrides.push((
                    self.fg_pass_fg2,
                    Bindings {
                        storage_buffers: vec![fg_prev, fg_cur, fg_mvn, fg2_params, fg2_out],
                        ..Bindings::default()
                    },
                ));
            }
        }
        // A5 readback 子集组合（序即解析序）：cur BGRA8 → 生成帧 BGRA8（gen 活跃
        // 才回读）→ cur f32（末帧/probe）→ probe 三路（prev f32 + MV + 生成 f32）。
        let gen_active = self.gen_active(reset);
        let mut subset: Vec<u32> = Vec::new();
        if readback != G31Readback::None {
            subset.push(G31_RB_BGRA);
            // C13 SVT:miss 请求缓冲逐帧回读（消费序 = BGRA 之后,probe/末帧路之前）。
            if self.svt.is_some() {
                subset.push(G31_RB_SVT_REQ);
            }
            if gen_active {
                subset.push(self.fg_layout.rb_fg1_bgra);
                if self.fg == G31Fg::X3 {
                    subset.push(self.fg_layout.rb_fg2_bgra);
                }
            }
            if readback == G31Readback::BgraAndColor || probe {
                // G37 W3 fg_combo：fg_full probe 帧 cur 换 comp[p] 回读（host
                // interpolate 复算的 cur = FG kernel 真实输入契约;回读尺寸与
                // out_color 同 opc×12 ⇒ rec_from_output 解析面 0-byte）。非
                // probe 末帧维持 U_OUT_COLOR[p]（raw dump 语义 = pre-bloom
                // TSR 输出,bloom 车道既有口径;probe 帧恰为末帧的角例登记
                // MERGE_REPORT）。
                if probe && self.fg_full {
                    subset.push(if p == 0 {
                        self.fg_layout.rb_comp0
                    } else {
                        self.fg_layout.rb_comp1
                    });
                } else {
                    subset.push(p as u32);
                }
            }
            if probe {
                // G37 W3 fg_combo：fg_full 面 prev 同律换 comp[1−p] 回读。
                if self.fg_full {
                    subset.push(if p == 0 {
                        self.fg_layout.rb_comp1
                    } else {
                        self.fg_layout.rb_comp0
                    });
                } else {
                    subset.push(1 - p as u32);
                }
                subset.push(2);
                subset.push(self.fg_layout.rb_fg1_out);
                if self.fg == G31Fg::X3 {
                    subset.push(self.fg_layout.rb_fg2_out);
                }
                subset.push(self.fg_layout.rb_mvn);
            }
        }
        let update = FrameUpdate {
            tlas_update: None,
            buffer_uploads: uploads,
            binding_overrides,
            push_constant_overrides: vec![],
            readback_subset: Some(subset),
            blas_refit: None, // G31+ 波 B Task B5 字段面:本车道无 BLAS refit(0-byte 默认)
        };
        let prov = self.session.next_provenance_with_update(&update)?;
        Ok((prov, update))
    }

    /// 一帧产物组装（telemetry 五 pass 提取 + BGRA8/f32 回读 + 尺寸校验）。A5:
    /// 回读按 prepare_update 子集同序解析;FG pass telemetry 按名提取合计。
    fn rec_from_output(
        &self,
        mut out: DeviceFrameOutput,
        readback: G31Readback,
        gen_active: bool,
        probe: bool,
        ow: u32,
        oh: u32,
    ) -> Result<G31FrameRec, String> {
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        let scene_gpu_ns = gpu(self.scene_pass_name)?;
        let mv_gpu_ns = gpu("g14_mv")?;
        let resample_gpu_ns = gpu("g14_8_tsr_resample")?;
        let resolve_gpu_ns = gpu("g14_8_tsr_resolve")?;
        let encode_gpu_ns = gpu("g31_display_encode")?;
        let mut fg_gpu_ns = 0.0;
        if self.fg != G31Fg::Off {
            fg_gpu_ns +=
                gpu("g31_mv_negate")? + gpu("g26_framegen_fg1")? + gpu("g31_display_encode_fg1")?;
            if self.fg == G31Fg::X3 {
                fg_gpu_ns += gpu("g26_framegen_fg2")? + gpu("g31_display_encode_fg2")?;
            }
        }
        let t_convert = std::time::Instant::now();
        let bgra_px = (ow * oh * 4) as usize;
        let f32_px = (ow * oh * 3) as usize;
        let mv_px = (ow * oh * 2) as usize;
        let mut idx = 0usize;
        let take_rb = |out: &mut DeviceFrameOutput, idx: &mut usize| -> Result<Vec<u8>, String> {
            if *idx >= out.readbacks.len() {
                return Err(format!(
                    "A5 回读路数 {} 少于子集消费序 {idx}",
                    out.readbacks.len()
                ));
            }
            let b = std::mem::take(&mut out.readbacks[*idx]);
            *idx += 1;
            Ok(b)
        };
        let (
            bgra8,
            gen_bgra8,
            out_color,
            probe_prev_color,
            probe_mv,
            probe_gen_out,
            probe_mvn,
            svt_requests,
        ) = if readback == G31Readback::None {
            if !out.readbacks.is_empty() {
                return Err(format!(
                    "A3 零回读面回读路数 {} ≠ 0",
                    out.readbacks.len()
                ));
            }
            (None, Vec::new(), None, None, None, Vec::new(), None, None)
        } else {
            let b = take_rb(&mut out, &mut idx)?;
            if b.len() != bgra_px {
                return Err(format!(
                    "A3 BGRA8 回读字节 {} ≠ {}x{}x4",
                    b.len(),
                    ow,
                    oh
                ));
            }
            // C13 SVT:miss 请求缓冲（消费序 = BGRA 之后;svt on 才有此路）。
            let svt_requests = if let Some(s) = self.svt.as_ref() {
                let r = take_rb(&mut out, &mut idx)?;
                if r.len() != s.req_bytes {
                    return Err(format!(
                        "C13 SVT 请求缓冲回读字节 {} ≠ {}",
                        r.len(),
                        s.req_bytes
                    ));
                }
                Some(r)
            } else {
                None
            };
            let mut gen_bgra8: Vec<Vec<u8>> = Vec::new();
            if gen_active {
                let want_gen = self.fg.inserted() as usize;
                for _ in 0..want_gen {
                    let g = take_rb(&mut out, &mut idx)?;
                    if g.len() != bgra_px {
                        return Err(format!(
                            "A5 生成帧 BGRA8 回读字节 {} ≠ {}x{}x4",
                            g.len(),
                            ow,
                            oh
                        ));
                    }
                    gen_bgra8.push(g);
                }
            }
            let out_color = if readback == G31Readback::BgraAndColor || probe {
                let data = read_f32(&take_rb(&mut out, &mut idx)?);
                if data.len() != f32_px {
                    return Err("A3 f32 回读字节数与输出分辨率不符".into());
                }
                Some(data)
            } else {
                None
            };
            let (probe_prev_color, probe_mv, probe_gen_out, probe_mvn) = if probe {
                let prev = read_f32(&take_rb(&mut out, &mut idx)?);
                if prev.len() != f32_px {
                    return Err("A5 probe prev f32 回读字节数与输出分辨率不符".into());
                }
                let mv = read_f32(&take_rb(&mut out, &mut idx)?);
                if mv.len() != mv_px {
                    return Err("A5 probe MV 回读字节数与输出分辨率不符".into());
                }
                let mut gens = Vec::new();
                for _ in 0..self.fg.inserted() {
                    let g = read_f32(&take_rb(&mut out, &mut idx)?);
                    if g.len() != f32_px {
                        return Err("A5 probe 生成帧 f32 回读字节数与输出分辨率不符".into());
                    }
                    gens.push(g);
                }
                let mvn = read_f32(&take_rb(&mut out, &mut idx)?);
                if mvn.len() != mv_px {
                    return Err("A5 probe MVN 回读字节数与输出分辨率不符".into());
                }
                (Some(prev), Some(mv), gens, Some(mvn))
            } else {
                (None, None, Vec::new(), None)
            };
            if idx != out.readbacks.len() {
                return Err(format!(
                    "A5 回读消费序 {idx} ≠ 实到路数 {}",
                    out.readbacks.len()
                ));
            }
            (
                Some(b),
                gen_bgra8,
                out_color,
                probe_prev_color,
                probe_mv,
                probe_gen_out,
                probe_mvn,
                svt_requests,
            )
        };
        let readback_convert_ms = t_convert.elapsed().as_secs_f64() * 1000.0;
        // C7 profiler 面:全量逐 pass GPU 计时（telemetry 声明序直拷）。
        let pass_gpu_ns: Vec<(String, f64)> = out
            .telemetry
            .passes
            .iter()
            .map(|pp| (pp.name.clone(), pp.gpu_ns))
            .collect();
        Ok(G31FrameRec {
            scene_gpu_ns,
            mv_gpu_ns,
            resample_gpu_ns,
            resolve_gpu_ns,
            encode_gpu_ns,
            fg_gpu_ns,
            cpu_record_ns: out.telemetry.cpu_record_ns,
            cpu_submit_ns: out.telemetry.cpu_submit_ns,
            cpu_fence_wait_ns: out.telemetry.cpu_fence_wait_ns,
            validation_error_count: out.telemetry.validation_error_count,
            leaked_object_count: out.telemetry.leaked_object_count,
            leaked_allocation_count: out.telemetry.leaked_allocation_count,
            bgra8,
            gen_bgra8,
            out_color,
            probe_prev_color,
            probe_mv,
            probe_gen_out,
            probe_mvn,
            readback_convert_ms,
            hzb: None,
            svt_requests,
            pass_gpu_ns,
        })
    }

    /// 一帧：三小件参数上传 → 五 pass GPU 链内执行（TSR 输出驻留 device,
    /// encode 链内直写 BGRA8;A5 fg on 追加 FG pass 链内生成 + 编码）→ 可选
    /// BGRA8(/f32/probe 三路）回读。
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        eps: f32,
        quad_count: usize,
        point_count: usize,
        inv_vp: &Mat4,
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback: G31Readback,
        probe: bool,
    ) -> Result<G31FrameRec, String> {
        let gen_active = self.gen_active(reset);
        let (prov, update) = self.prepare_update(
            iw, ih, ow, oh, jitter, eps, quad_count, point_count, inv_vp, vp, vp_j, exposure,
            reset, readback, probe,
        )?;
        let out = self.session.execute_with_frame_update(&prov, &update)?;
        let rec = self.rec_from_output(out, readback, gen_active, probe, ow, oh)?;
        self.prev_vp_j = Some(*vp_j);
        self.has_history_state = true;
        self.parity = 1 - self.parity;
        Ok(rec)
    }
}

// ---------------------------------------------------------------------------
// G31+ 波 B Task B1 HZB 遮挡剔除生产接线面（--hzb <off|on>;G30 承接锚 G27 行
// 「生产接线窗」+ RFC-0044 §5.8 两阶段第二段（F10 补项）兑现;G31_PLUS
// COMMERCIAL_RENDERER_TODO §1.2 #6 行）。
//
// ## 架构（单 TLAS 签名纪律〔RXS-0297〕下的双 TLAS 拆散分工）
// - **剔除对象粒度 = TLAS 实例**（bistro 逐 mesh 节点 BLAS 分解,仅 --hzb on
//   面;tris/mats SSBO 与单 BLAS 生产面位级同 buffer——节点段为装配序连续段,
//   g31_hzb_primary 经 inst_base 前缀和表把 (inst, prim) 映回全局下标,着色
//   数学与 Mega 面逐位同式）。
// - **消费点 = 主射线 pass 的 TLAS 实例 mask**（被剔实例 mask=0x00 ⇒ ray query
//   零遍历其 BLAS = 「跳过 primary pass 对应射线工作」的 RT 车道兑现形;
//   kernels/g31_hzb_primary.rx 相机射线走初剔后 TLAS;kernels/g31_hzb_shade.rx
//   阴影射线走全量 TLAS——被剔实例仍投阴影,遮挡物阴影正确性面）。
// - **金字塔构建进剔除链**（kernels/g27_hzb_reduce.rx 0-byte 冻结消费）:
//   本帧**真深度**（depth_hz 专用面 = g31_hzb_shade ④b 段由 vp 行 2/3 另算的
//   真 ZO NDC——U_SCENE_DEPTH 沿用 g14_3_shade_reduce 参数行 25..32 生产字面
//   供 MV/TSR,两路并存互不染指;剔除链语义对真实深度成立,近面内几何
//   z_ndc<0 合法入塔）逐级 dispatch 归约 + g31_hzb_pack.rx glue 平铺进单
//   SSBO;**帧间金字塔轮换** = 每帧先初剔后重建（test_p1 读上帧平铺,
//   reduce/pack 覆写为本帧,test_p2 读本帧）。
// - **两阶段闭环第二段**（RFC-0044 §5.8 字面「上帧金字塔初剔 + 本帧重建重测」):
//   逐帧单提交内 pass 序 [primary→shade→mv→tsr×2→encode→test_p1(全实例 rect
//   vs 上帧金字塔)→reduce×(L−1)+pack×L(本帧重建)→test_p2(上帧被剔集 vs 本帧
//   金字塔)];collect 后 host 结算本帧应见集 = p1 可见 ∪ p2 翻回——应见集中
//   有本帧未渲染者 ⇒ **闭环重渲**（同帧参数 + 掩码并集二次提交,迭代 ≤4 仍
//   未收敛 ⇒ 全掩码兜底重渲=精确收敛;漏剔合法零害、误剔必被重测翻回并
//   补渲——剔除零假阳性 ⇒ 闭环后画面与分解车道全集渲染位级一致,由
//   RURIX_HZB_ALL_VISIBLE 登记实验臂 digest_seq 逐帧对拍承载）。
// - **host 金标准面对拍**（geometry/{hzb,cull}.rs 只读消费 0-byte）:
//   生产路径消费 cull::Frustum 视锥面（离屏拒绝）+ hzb::HzbPyramid::build /
//   test_rect / exact_rect_occluded（probe 帧接线态对拍:车道金字塔 vs host
//   逐级位级全等 + p1 判定序列逐字节全等 + 零假阳性独立复核）。
// ---------------------------------------------------------------------------

/// B1 HZB 接线门键（--hzb on 面 evidence `gate` 字段字面）。
const G31_HZB_GATE: &str = "g31.waveB.hzb";
/// B1 HZB 接线 evidence schema 字面（milestones/g31/g31_hzb_wiring_evidence_schema.json 同字面）。
const G31_HZB_SCHEMA: &str = "rurix.g31.hzb_wiring_evidence.v1";
/// B1 主射线 kernel 默认 SPV（源 = kernels/g31_hzb_primary.rx;`.tmp` 构建产物,CI 门脚本保障编译）。
const G31_DEFAULT_SPV_HZB_PRIMARY: &str = ".tmp/g14_gates/m_c/g31_hzb_primary.spv";
/// B1 着色 kernel 默认 SPV（源 = kernels/g31_hzb_shade.rx）。
const G31_DEFAULT_SPV_HZB_SHADE: &str = ".tmp/g14_gates/m_c/g31_hzb_shade.spv";
/// B1 平铺打包 glue kernel 默认 SPV（源 = kernels/g31_hzb_pack.rx）。
const G31_DEFAULT_SPV_HZB_PACK: &str = ".tmp/g14_gates/m_c/g31_hzb_pack.spv";
/// B1 金字塔归约 kernel 默认 SPV（源 = kernels/g27_hzb_reduce.rx——G27 M-a 本体 0-byte 冻结消费）。
const G31_DEFAULT_SPV_HZB_REDUCE: &str = ".tmp/g14_gates/m_c/g27_hzb_reduce.spv";
/// B1 遮挡测试 kernel 默认 SPV（源 = kernels/g27_hzb_test.rx——G27 M-a 本体 0-byte 冻结消费）。
const G31_DEFAULT_SPV_HZB_TEST: &str = ".tmp/g14_gates/m_c/g27_hzb_test.spv";
/// B1 闭环重渲迭代上限（未收敛 ⇒ 全掩码兜底重渲 = 精确收敛;如实登记）。
const G31_HZB_CLOSURE_MAX: u32 = 4;
/// B1 深度约定（车道深度 = ZO NDC 小值近/miss=1.0 远 ⇒ standard-Z;
/// g27 kernel 约定位 conv=1.0,host `DepthConvention::StandardZ` 同律）。
const G31_HZB_CONV_FLAG: f32 = 1.0;

/// B1 HZB 档闭集（off = 车道 0-byte 现状;on = 两阶段遮挡剔除接线）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum G31Hzb {
    Off,
    On,
}

/// B1 逐实例初剔分类（host;生产剔除链第一关 = cull::Frustum 视锥面只读消费,
/// 第二关 = HZB 遮挡测试进 device test 流）。
enum G31HzbClass {
    /// 视锥外（离屏/相机后）——像素中性直接剔,不进 test 流。
    Offscreen,
    /// 在屏：rect（uv 闭区间,±半像素 jitter 保守裕量外扩）+ 最近深度。
    Rect {
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        nearest: f32,
    },
}

/// B1 逐实例初剔分类器（逐节点 AABB;host 确定性 f32）。
/// - 视锥面 = `cull::Frustum::from_view_proj` + `intersects_aabb`（冻结金标准
///   面只读消费;主射线采样域恒在视口锥内 ⇒ 离屏剔除像素中性,结构依据:
///   采样点 sx∈[px,px+1) 恒落视口 ⇒ ndc 恒 ∈[−1,1],w≤0 角点 = 近面穿越/
///   相机后 ⇒ 全屏 rect + nearest=0 超保守可见处理——近面骑跨实例永不误剔）。
/// - rect 像素域 = g27_hzb_test.rx 域前提字面（0 ≤ u_min < u_max ≤ 1）保障:
///   外扩后 clamp;退化（u_max ≤ u_min）⇒ 全屏 + nearest=−∞ 超保守可见。
/// - nearest = 8 角点 z_ndc 最小值（z_ndc 对视深单调 ⇒ min 在角点取得——保守
///   最近深度严格成立）。**只钳上界 1.0（远平面外 ⇒ 天空 1.0 不致误剔）,不钳
///   下界**：车道深度 = 光线命中 z_ndc,近面内几何（z_ndc<0,枝形吊灯链/天窗梁
///   实测 −0.98 量级）合法存在于金字塔;若把 nearest 钳 0 ⇒ 实例自身金字塔纹素
///   (−0.98) 严格小于钳后 nearest ⇒ standard-Z「nearest>farthest ⇒ 剔」自我
///   遮挡误剔（实测 14767 像素黑洞）;保负值 ⇒ 自身纹素 ≥ nearest 恒成立,严格
///   不等式自遮挡结构上不可达,g27 冻结 kernel 纯 f32 选择/比较语义域外安全。
fn g31_hzb_classify(
    vp: &Mat4,
    iw: u32,
    ih: u32,
    groups: &[SceneNodeGroup],
) -> Vec<G31HzbClass> {
    // 登记实验臂（ci/g31_hzb_wiring_smoke.py 剔除像素中性门消费）:
    // RURIX_HZB_ALL_VISIBLE=1 ⇒ 全实例恒可见(无视锥/无剔除 ⇒ 掩码恒全 0xFF)
    // ——同一分解车道渲染全集;--hzb on 常态臂 vs 本臂 digest_seq 逐帧位级
    // 一致 ⇒ 「剔除不改变可见像素」机核门成立(可见集一致性结构判据)。
    if std::env::var("RURIX_HZB_ALL_VISIBLE").ok().as_deref() == Some("1") {
        return groups
            .iter()
            .map(|_| G31HzbClass::Rect {
                uv_min: [0.0, 0.0],
                uv_max: [1.0, 1.0],
                nearest: f32::NEG_INFINITY,
            })
            .collect();
    }
    let frustum = Frustum::from_view_proj(&vp.m);
    let (w0, h0) = (iw as f32, ih as f32);
    let (du, dv) = (0.5 / w0, 0.5 / h0);
    let mut out = Vec::with_capacity(groups.len());
    for g in groups {
        // 相机面骑跨预审（先于视锥面）：w ≤ 0 角点存在 ⇒ 平面法视锥判定失真
        // （相机后角点投影镜像,可把一个部分在锥内的 AABB 误判全外——bistro
        // 近相机薄板实例实测误剔即此类）⇒ 不信任该 verdict：全部 w ≤ 0 ⇒
        // 相机后,像素中性直接剔;部分 w > 0 ⇒ 骑跨 ⇒ 超保守恒可见（全屏 rect +
        // nearest 0）。全部 w > 0 ⇒ 投影处处良定义,视锥面判定可信。
        let mut cs = [[0.0f32; 4]; 8];
        let (mut any_back, mut any_front) = (false, false);
        let mut k = 0usize;
        for &x in &[g.aabb_min[0], g.aabb_max[0]] {
            for &y in &[g.aabb_min[1], g.aabb_max[1]] {
                for &z in &[g.aabb_min[2], g.aabb_max[2]] {
                    let c = vp.transform_vec4([x, y, z, 1.0]);
                    if c[3] <= 1e-6 {
                        any_back = true;
                    } else {
                        any_front = true;
                    }
                    cs[k] = c;
                    k += 1;
                }
            }
        }
        if any_back {
            if any_front {
                // 相机面骑跨：视锥/投影均不可信 ⇒ 超保守恒可见（全屏 + −∞;
                // −∞ ⇒ standard-Z「nearest>farthest」恒假,无天空帧亦永不误剔）。
                out.push(G31HzbClass::Rect {
                    uv_min: [0.0, 0.0],
                    uv_max: [1.0, 1.0],
                    nearest: f32::NEG_INFINITY,
                });
            } else {
                // 整体相机后：前向主射线永不可达 ⇒ 像素中性剔。
                out.push(G31HzbClass::Offscreen);
            }
            continue;
        }
        if !frustum.intersects_aabb(g.aabb_min, g.aabb_max) {
            out.push(G31HzbClass::Offscreen);
            continue;
        }
        let (mut u_min, mut v_min, mut u_max, mut v_max, mut nearest) =
            (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY);
        for c in &cs {
            let inv_w = 1.0 / c[3];
            let u = (c[0] * inv_w + 1.0) * 0.5;
            let v = (1.0 - c[1] * inv_w) * 0.5;
            let zz = (c[2] * inv_w).min(1.0);
            u_min = u_min.min(u);
            u_max = u_max.max(u);
            v_min = v_min.min(v);
            v_max = v_max.max(v);
            nearest = nearest.min(zz);
        }
        let umin = (u_min - du).clamp(0.0, 1.0);
        let umax = (u_max + du).clamp(0.0, 1.0);
        let vmin = (v_min - dv).clamp(0.0, 1.0);
        let vmax = (v_max + dv).clamp(0.0, 1.0);
        if umax <= umin || vmax <= vmin {
            // 退化 rect（视锥相交但投影域塌缩;结构罕见）⇒ 超保守恒可见（−∞）。
            out.push(G31HzbClass::Rect {
                uv_min: [0.0, 0.0],
                uv_max: [1.0, 1.0],
                nearest: f32::NEG_INFINITY,
            });
            continue;
        }
        out.push(G31HzbClass::Rect {
            uv_min: [umin, vmin],
            uv_max: [umax, vmax],
            nearest,
        });
    }
    out
}

/// B1 车道 SPV/常量字节所有者（desc 数组借用源;借用纪律 = bits → descs →
/// session 声明序 drop 逆序,车道面同模）。
struct G31HzbBits {
    spv_primary: Vec<u8>,
    spv_shade: Vec<u8>,
    spv_reduce: Vec<u8>,
    spv_test: Vec<u8>,
    spv_pack: Vec<u8>,
    /// pass 名（telemetry 逐 pass 唯一键;kernel 身份由 SPV provenance 登记——
    /// reduce/test = g27 本体 0-byte,pack/primary/shade = g31 加性件）。
    name_primary: String,
    name_shade: String,
    name_test_p1: String,
    name_test_p2: String,
    reduce_names: Vec<String>,
    pack_names: Vec<String>,
    primary_dispatch: [u32; 3],
    shade_dispatch: [u32; 3],
    test_dispatch: [u32; 3],
    reduce_dispatch: Vec<[u32; 3]>,
    pack_dispatch: Vec<[u32; 3]>,
    /// mip 逐級 (w,h)（mip0 = 内部分辨率;直至 1×1）。
    levels: Vec<(u32, u32)>,
    /// 平铺金字塔逐級纹素偏移（前缀和;g27_hzb_test mip_table offset 段同源）。
    flat_offsets: Vec<u32>,
    flat_texels: usize,
    mip_table_bytes: Vec<u8>,
    reduce_params_bytes: Vec<Vec<u8>>,
    pack_params_bytes: Vec<Vec<u8>>,
    /// 平铺金字塔初值 = 全 1.0f32（standard-Z 最远 ⇒ 首帧前全 Visible 保守初值,
    /// 空金字塔假阳性构造性不可达）。
    flat_init_bytes: Vec<u8>,
    /// 逐实例全局三角形下标基底（前缀和;g31_hzb_primary inst_base 面）。
    inst_base_bytes: Vec<u8>,
}

impl G31HzbBits {
    fn load(
        spv_primary: &str,
        spv_shade: &str,
        spv_reduce: &str,
        spv_test: &str,
        spv_pack: &str,
        iw: u32,
        ih: u32,
        groups: &[SceneNodeGroup],
    ) -> Self {
        let to_bytes = |words: &[u32]| -> Vec<u8> {
            words.iter().flat_map(|w| w.to_le_bytes()).collect()
        };
        let pw = load_spv(spv_primary);
        let sw = load_spv(spv_shade);
        // HZB 两 kernel（g27 本体 0-byte）注入 NoContraction（mv kernel 同律 bin
        // 侧后处理,SPV 文件 0-byte 不动）——G27 零容差协议「conv 乘法门保位级」
        // 的语义域 = [0,1] 正值闭集;生产车道深度可含负值（近平面内侧几何
        // z_ndc<0）,驱动乘加收缩面在负值域产生 1-ULP 门差〔lerp 两步舍入〕,
        // NoContraction 禁驱动 FMA 收缩/重关联,保门形逐 op IEEE 位级。
        let rw = spv_inject_no_contraction(&load_spv(spv_reduce));
        let tw = spv_inject_no_contraction(&load_spv(spv_test));
        let kw = load_spv(spv_pack);
        let (px, py, _) = spv_local_size(&pw);
        let (sx, sy, _) = spv_local_size(&sw);
        // mip 拓扑 = host `HzbPyramid::build` 逐字（非 2 幂 ceil 减半 max 1,
        // 直至 1×1）。
        let mut levels: Vec<(u32, u32)> = vec![(iw, ih)];
        while levels.last().unwrap().0 > 1 || levels.last().unwrap().1 > 1 {
            let (w, h) = *levels.last().unwrap();
            levels.push((w.div_ceil(2).max(1), h.div_ceil(2).max(1)));
        }
        let mut flat_offsets = Vec::with_capacity(levels.len());
        let mut acc = 0u32;
        for &(w, h) in &levels {
            flat_offsets.push(acc);
            acc += w * h;
        }
        let flat_texels = acc as usize;
        // mip 表（3 f32/級 [offset,w,h];g27_hzb_test 参数面逐字同源）。
        let mut mip_table: Vec<f32> = Vec::with_capacity(levels.len() * 3);
        for (k, &(w, h)) in levels.iter().enumerate() {
            mip_table.push(flat_offsets[k] as f32);
            mip_table.push(w as f32);
            mip_table.push(h as f32);
        }
        // reduce 参数（級 k=1..L−1:g27_hzb_reduce 8 f32 参数面逐字同源;
        // conv = standard-Z 1.0——车道深度 ZO NDC 小值近）。
        let mut reduce_params_bytes = Vec::with_capacity(levels.len() - 1);
        let mut reduce_dispatch = Vec::with_capacity(levels.len() - 1);
        for k in 1..levels.len() {
            let (nw, nh) = levels[k];
            let (pw2, ph2) = levels[k - 1];
            let p = [
                (nw * nh) as f32,
                nw as f32,
                nh as f32,
                pw2 as f32,
                ph2 as f32,
                G31_HZB_CONV_FLAG,
                0.0,
                0.0,
            ];
            reduce_params_bytes.push(bytes_f32(&p));
            reduce_dispatch.push([nw * nh, 1, 1]);
        }
        // pack 参数（級 k=0..L−1:[count, dst_offset, 0..]）。
        let mut pack_params_bytes = Vec::with_capacity(levels.len());
        let mut pack_dispatch = Vec::with_capacity(levels.len());
        for (k, &(w, h)) in levels.iter().enumerate() {
            let p = [
                (w * h) as f32,
                flat_offsets[k] as f32,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ];
            pack_params_bytes.push(bytes_f32(&p));
            pack_dispatch.push([w * h, 1, 1]);
        }
        let flat_init: Vec<f32> = vec![1.0f32; flat_texels];
        // inst_base = 逐实例三角形段前缀和（< 2^24 f32 精确域;bistro 总量 ≪）。
        let mut inst_base: Vec<f32> = Vec::with_capacity(groups.len());
        for g in groups {
            inst_base.push(g.tri_offset as f32);
        }
        let n_inst = groups.len().max(1) as u32;
        Self {
            spv_primary: to_bytes(&pw),
            spv_shade: to_bytes(&sw),
            spv_reduce: to_bytes(&rw),
            spv_test: to_bytes(&tw),
            spv_pack: to_bytes(&kw),
            name_primary: "g31_hzb_primary".to_owned(),
            name_shade: "g31_hzb_shade".to_owned(),
            name_test_p1: "g27_hzb_test_p1".to_owned(),
            name_test_p2: "g27_hzb_test_p2".to_owned(),
            reduce_names: (1..levels.len())
                .map(|k| format!("g27_hzb_reduce_l{k}"))
                .collect(),
            pack_names: (0..levels.len())
                .map(|k| format!("g31_hzb_pack_l{k}"))
                .collect(),
            primary_dispatch: [iw.div_ceil(px), ih.div_ceil(py), 1],
            shade_dispatch: [iw.div_ceil(sx), ih.div_ceil(sy), 1],
            test_dispatch: [n_inst, 1, 1],
            reduce_dispatch,
            pack_dispatch,
            levels,
            flat_offsets,
            flat_texels,
            mip_table_bytes: bytes_f32(&mip_table),
            reduce_params_bytes,
            pack_params_bytes,
            flat_init_bytes: bytes_f32(&flat_init),
            inst_base_bytes: bytes_f32(&inst_base),
        }
    }
}

/// B1 车道资源/回读下标面（hzb on 才存在;fg 互斥 ⇒ 24 起编与 A5 区间无冲突）。
#[derive(Debug, Clone)]
struct G31HzbIds {
    hit_t: u32,
    hit_pg: u32,
    depth_hz: u32,
    inst_base: u32,
    flat: u32,
    mip_table: u32,
    stage: Vec<u32>,
    reduce_params: Vec<u32>,
    pack_params: Vec<u32>,
    rects_p1: u32,
    params_p1: u32,
    verdicts_p1: u32,
    rects_p2: u32,
    params_p2: u32,
    verdicts_p2: u32,
    rb_verdicts_p1: u32,
    rb_verdicts_p2: u32,
    rb_flat: u32,
}

/// B1 车道描述组（Mega 四 pass 0..=21 资源 0-byte 面 + encode 22/23 + HZB 追加
/// 面 24+;pass 终序 = primary→shade→mv→resample→resolve→encode→test_p1→
/// reduce×(L−1)→pack×L→test_p2——「先初剔（读上帧平铺)后重建（覆写本帧）再
/// 重测（读本帧平铺)」的帧内金字塔轮换调度字面;Mega scene pass 本体 0-byte
/// 不进 HZB 车道,primary/shade 双 pass 替换之）。
#[allow(clippy::too_many_arguments)]
fn g31_lane_descs_hzb<'x>(
    assets: &'x LaneAssets,
    bits: &'x UnifiedLaneBits,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    hz: &'x G31HzbBits,
    n_instances: usize,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> (
    Vec<ResourceDesc<'x>>,
    Vec<Pass<'x>>,
    Vec<Vec<(u32, TargetState)>>,
    Vec<Readback>,
    G31HzbIds,
) {
    let (resources, passes, barriers, readbacks) = unified_lane_descs(assets, bits, iw, ih, ow, oh);
    let ipc = (iw * ih) as u64;
    let opc = (ow * oh) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let mut resources = resources.to_vec();
    let mut readbacks = readbacks.to_vec();
    // unified 四 pass 解构：mega scene 不进 HZB 车道（0-byte 不触）;mv/resample/
    // resolve 逐字保留（pass 对象与屏障计划同序搬运）。
    let [mega_scene, mv_pass, resample_pass, resolve_pass] = passes;
    let [plan_mega, plan_mv, plan_resample, plan_resolve] = barriers;
    let _ = (mega_scene, plan_mega);
    let buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: true,
        })
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let host_buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: false,
        })
    };
    // 22/23 = 编码参数 + BGRA8 输出（A3 面逐字同）。
    resources.push(init(enc_params_bytes));
    resources.push(buf(opc * 4));
    let mut next = G31_U_RESOURCE_COUNT as u32;
    macro_rules! take {
        ($r:expr) => {{
            let id = next;
            next += 1;
            resources.push($r);
            id
        }};
    }
    let n_rect_bytes = (n_instances.max(1) * 5 * 4) as u64;
    let n_verd_bytes = (n_instances.max(1) * 4) as u64;
    let hit_t = take!(buf(ipc * 4));
    let hit_pg = take!(buf(ipc * 4));
    // HZB 真深度面（g31_hzb_shade ④b 段写出 = 真 ZO NDC;剔除链金字塔 mip0
    // 专用源——U_SCENE_DEPTH 沿用生产字面供 MV/TSR,两路并存互不染指）。
    let depth_hz = take!(buf(ipc * 4));
    let inst_base = take!(init(&hz.inst_base_bytes));
    let flat = take!(init(&hz.flat_init_bytes));
    let mip_table = take!(init(&hz.mip_table_bytes));
    let mut stage = Vec::with_capacity(hz.levels.len() - 1);
    let mut reduce_params = Vec::with_capacity(hz.levels.len() - 1);
    for k in 1..hz.levels.len() {
        let (w, h) = hz.levels[k];
        stage.push(take!(buf((w * h) as u64 * 4)));
        reduce_params.push(take!(init(&hz.reduce_params_bytes[k - 1])));
    }
    let mut pack_params = Vec::with_capacity(hz.levels.len());
    for k in 0..hz.levels.len() {
        pack_params.push(take!(init(&hz.pack_params_bytes[k])));
    }
    let rects_p1 = take!(host_buf(n_rect_bytes));
    let params_p1 = take!(host_buf(8 * 4));
    let verdicts_p1 = take!(buf(n_verd_bytes));
    let rects_p2 = take!(host_buf(n_rect_bytes));
    let params_p2 = take!(host_buf(8 * 4));
    let verdicts_p2 = take!(buf(n_verd_bytes));
    let _ = next;
    let ids = G31HzbIds {
        hit_t,
        hit_pg,
        depth_hz,
        inst_base,
        flat,
        mip_table,
        stage,
        reduce_params,
        pack_params,
        rects_p1,
        params_p1,
        verdicts_p1,
        rects_p2,
        params_p2,
        verdicts_p2,
        rb_verdicts_p1: 5,
        rb_verdicts_p2: 6,
        rb_flat: 7,
    };
    let mut out_passes: Vec<Pass<'x>> = Vec::with_capacity(7 + 2 * hz.levels.len());
    let mut out_barriers: Vec<Vec<(u32, TargetState)>> =
        Vec::with_capacity(7 + 2 * hz.levels.len());
    // ── pass 0:primary（初剔后 TLAS = AS 表 0;读 inst_base/params,写 hitinfo）──
    out_passes.push(Pass::Compute(ComputePass {
        name: &hz.name_primary,
        spirv: &hz.spv_primary,
        entry: None,
        dispatch: DispatchSpec::Direct(hz.primary_dispatch),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: vec![inst_base, U_SCENE_PARAMS, hit_t, hit_pg],
            ..Bindings::default()
        },
    }));
    out_barriers.push(vec![
        (inst_base, TargetState::ShaderRead),
        (U_SCENE_PARAMS, TargetState::ShaderRead),
        (hit_t, TargetState::StorageWrite),
        (hit_pg, TargetState::StorageWrite),
    ]);
    // ── pass 1:shade（全量 TLAS = AS 表 1;阴影射线零剔除 ⇒ 与 Mega 面同域）──
    out_passes.push(Pass::Compute(ComputePass {
        name: &hz.name_shade,
        spirv: &hz.spv_shade,
        entry: None,
        dispatch: DispatchSpec::Direct(hz.shade_dispatch),
        bindings: Bindings {
            accel_structs: vec![1],
            storage_buffers: vec![
                hit_t,
                hit_pg,
                U_TRIS,
                U_MATS,
                U_QUADS,
                U_POINTS,
                U_SCENE_PARAMS,
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
                depth_hz,
            ],
            ..Bindings::default()
        },
    }));
    out_barriers.push(vec![
        (hit_t, TargetState::ShaderRead),
        (hit_pg, TargetState::ShaderRead),
        (U_TRIS, TargetState::ShaderRead),
        (U_MATS, TargetState::ShaderRead),
        (U_QUADS, TargetState::ShaderRead),
        (U_POINTS, TargetState::ShaderRead),
        (U_SCENE_PARAMS, TargetState::ShaderRead),
        (U_SCENE_COLOR, TargetState::StorageWrite),
        (U_SCENE_DEPTH, TargetState::StorageWrite),
        (depth_hz, TargetState::StorageWrite),
    ]);
    // ── pass 2..4:mv/resample/resolve（unified 逐字搬运）+ pass 5:encode（A3 同）──
    out_passes.push(mv_pass);
    out_barriers.push(plan_mv.to_vec());
    out_passes.push(resample_pass);
    out_barriers.push(plan_resample.to_vec());
    out_passes.push(resolve_pass);
    out_barriers.push(plan_resolve.to_vec());
    out_passes.push(Pass::Compute(ComputePass {
        name: "g31_display_encode",
        spirv: enc_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(enc_dispatch),
        bindings: Bindings {
            storage_buffers: vec![U_OUT_COLOR[0], G31_U_ENC_PARAMS, G31_U_ENC_OUT],
            ..Bindings::default()
        },
    }));
    out_barriers.push(G31_U_PLAN_ENCODE.to_vec());
    // ── pass 6:test_p1（全实例 rect vs 上帧金字塔——「上帧金字塔初剔」字面）──
    out_passes.push(Pass::Compute(ComputePass {
        name: &hz.name_test_p1,
        spirv: &hz.spv_test,
        entry: None,
        dispatch: DispatchSpec::Direct(hz.test_dispatch),
        bindings: Bindings {
            storage_buffers: vec![flat, mip_table, rects_p1, params_p1, verdicts_p1],
            ..Bindings::default()
        },
    }));
    out_barriers.push(vec![
        (flat, TargetState::ShaderRead),
        (mip_table, TargetState::ShaderRead),
        (rects_p1, TargetState::ShaderRead),
        (params_p1, TargetState::ShaderRead),
        (verdicts_p1, TargetState::StorageWrite),
    ]);
    // ── pass 7..:reduce×(L−1)（級 k:src = 上級〔k=1 = depth_hz 真深度,余 =
    //    stage k−1〕→ stage k;g27_hzb_reduce 0-byte 冻结消费）──
    for k in 1..hz.levels.len() {
        let src = if k == 1 { depth_hz } else { ids.stage[k - 2] };
        out_passes.push(Pass::Compute(ComputePass {
            name: &hz.reduce_names[k - 1],
            spirv: &hz.spv_reduce,
            entry: None,
            dispatch: DispatchSpec::Direct(hz.reduce_dispatch[k - 1]),
            bindings: Bindings {
                storage_buffers: vec![src, ids.reduce_params[k - 1], ids.stage[k - 1]],
                ..Bindings::default()
            },
        }));
        out_barriers.push(vec![
            (src, TargetState::ShaderRead),
            (ids.reduce_params[k - 1], TargetState::ShaderRead),
            (ids.stage[k - 1], TargetState::StorageWrite),
        ]);
    }
    // ── pack×L（級 0 = depth_hz 真深度原字节平铺〔host mip0 拷贝同语义——
    //    剔除链须真 ZO NDC 域;U_SCENE_DEPTH 生产字面留 MV/TSR 不染指〕,級 k≥1
    //    = stage k;g31_hzb_pack 纯拷贝 glue）──
    for k in 0..hz.levels.len() {
        let src = if k == 0 { depth_hz } else { ids.stage[k - 1] };
        out_passes.push(Pass::Compute(ComputePass {
            name: &hz.pack_names[k],
            spirv: &hz.spv_pack,
            entry: None,
            dispatch: DispatchSpec::Direct(hz.pack_dispatch[k]),
            bindings: Bindings {
                storage_buffers: vec![src, ids.pack_params[k], flat],
                ..Bindings::default()
            },
        }));
        out_barriers.push(vec![
            (src, TargetState::ShaderRead),
            (ids.pack_params[k], TargetState::ShaderRead),
            (flat, TargetState::StorageWrite),
        ]);
    }
    // ── 末 pass:test_p2（上帧被剔集 vs 本帧金字塔——「本帧重建重测」字面）──
    out_passes.push(Pass::Compute(ComputePass {
        name: &hz.name_test_p2,
        spirv: &hz.spv_test,
        entry: None,
        dispatch: DispatchSpec::Direct(hz.test_dispatch),
        bindings: Bindings {
            storage_buffers: vec![flat, mip_table, rects_p2, params_p2, verdicts_p2],
            ..Bindings::default()
        },
    }));
    out_barriers.push(vec![
        (flat, TargetState::ShaderRead),
        (mip_table, TargetState::ShaderRead),
        (rects_p2, TargetState::ShaderRead),
        (params_p2, TargetState::ShaderRead),
        (verdicts_p2, TargetState::StorageWrite),
    ]);
    // ── 回读表：0..=3 = unified 面（OUT_COLOR f32 双 parity/MV/DEPTH）;
    //    4 = BGRA8（A3 面逐字同——G31_RB_BGRA 下标锚）;5/6 = p1/p2 判定（逐帧
    //    决策面）;7 = 平铺金字塔（probe 对拍面）。──
    readbacks.push(Readback::Buffer {
        res: G31_U_ENC_OUT,
        offset: 0,
        size: opc * 4,
    });
    readbacks.push(Readback::Buffer {
        res: verdicts_p1,
        offset: 0,
        size: n_verd_bytes,
    });
    readbacks.push(Readback::Buffer {
        res: verdicts_p2,
        offset: 0,
        size: n_verd_bytes,
    });
    readbacks.push(Readback::Buffer {
        res: flat,
        offset: 0,
        size: (hz.flat_texels * 4) as u64,
    });
    // 8 = depth_hz 真深度回读（probe 对拍面:host 金标准金字塔构建源——剔除链
    // 深度域 = 真 ZO NDC,与设备平铺 mip0 位级同源）。
    readbacks.push(Readback::Buffer {
        res: depth_hz,
        offset: 0,
        size: ipc * 4,
    });
    (resources, out_passes, out_barriers, readbacks, ids)
}

/// B1 一帧 HZB 决策/调度产物（evidence 计数面 + probe 对拍面;生产五段 GPU/
/// 回读面由 G31FrameRec 既有字段承载）。
struct G31HzbFrameRec {
    /// 本帧 p1 实测 rect 数（在屏实例）。
    tested_p1: u32,
    /// p1 判遮挡数（初剔剔除量）。
    occluded_p1: u32,
    /// 视锥面离屏直剔数（像素中性第一关）。
    offscreen: u32,
    /// p2 重测数（上帧终判被剔集）。
    retested_p2: u32,
    /// p2 翻回数（误遮挡重测检出 = 闭环补渲对象）。
    flipped_p2: u32,
    /// 闭环重渲追加提交数（0 = 稳态;>0 = 误剔/出新补渲真实发生）。
    closure_extra_submits: u32,
    /// 迭代上限耗尽 ⇒ 全掩码兜底重渲（精确收敛;如实登记）。
    closure_full_fallback: bool,
    /// 本帧终判可见实例数（下一帧渲染掩码面）。
    visible_final: u32,
    /// 本帧 HZB pass GPU 合计（test×2 + reduce + pack;全提交累计）。
    hzb_gpu_ns: f64,
    /// 闭环重渲追加的生产链 GPU（非末次提交的六段合计;如实分列不混口径）。
    closure_extra_gpu_ns: f64,
    /// host 侧剔除决策耗时（分类/掩码/闭环保守;毫秒）。
    host_ms: f64,
    /// probe 预备帧回读（深度 + 平铺金字塔;非 probe_pre = None）。
    probe_depth: Option<Vec<f32>>,
    probe_flat: Option<Vec<f32>>,
    /// 本帧 p1 判定字节序（末次提交;probe 对拍消费）。
    verdicts_p1: Vec<u8>,
    /// 本帧 p1 rect 流（5 f32/rect）+ 实例号列（probe 对拍 host 复算输入面）。
    rects_p1: Vec<f32>,
    rects_inst_p1: Vec<u32>,
}

/// B1 车道状态机（顺序入口——逐帧 host 决策在环,FIF 流水面天然不适用,
/// A2 约束〔FIF 拒 tlas_update〕同律登记;两阶段调度 + 闭环重渲全记录）。
struct G31HzbLane<'a> {
    session: DeviceFrameSession<'a>,
    parity: usize,
    has_history_state: bool,
    prev_vp_j: Option<Mat4>,
    ids: G31HzbIds,
    groups: Vec<SceneNodeGroup>,
    /// 下一帧渲染掩码（host 决策面;0xFF = 可见 / 0x00 = 剔除）。
    masks: Vec<u8>,
    /// TLAS[0] 当前上传态（等价重更跳过——静态相机稳态零 TLAS 税）。
    uploaded_masks: Vec<u8>,
    /// 上帧终判被剔集（本帧 test_p2 重测对象;rect 流 5 f32/rect + 实例号列）。
    prev_p2_rects: Vec<f32>,
    prev_p2_inst: Vec<u32>,
    /// 本帧 p1 流（决策/对拍消费;5 f32/rect + 实例号列）。
    last_rects_p1: Vec<f32>,
    last_rects_inst: Vec<u32>,
    n_levels: usize,
}

impl<'a> G31HzbLane<'a> {
    fn create(
        resources: &'a [ResourceDesc<'a>],
        passes: &'a [Pass<'a>],
        barriers: &'a [&'a [(u32, TargetState)]],
        readbacks: &'a [Readback],
        accel_structs: &[AccelStructDesc<'a>],
        ids: G31HzbIds,
        groups: Vec<SceneNodeGroup>,
        n_levels: usize,
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        if groups.is_empty() {
            return Err("HZB 面场景零可剔除实例（节点分组为空,fail-closed 不冒充）".into());
        }
        // frame_slots=2（顺序全同步既有面逐字同;FIF 流水面拒 tlas_update,
        // A2 约束登记——逐帧 host 决策在环本就顺序）。
        let session = DeviceFrameSession::new_with_accel_structs(
            resources,
            passes,
            barriers,
            readbacks,
            2,
            accel_structs,
        )?;
        let n = groups.len();
        Ok(Self {
            session,
            parity: 0,
            has_history_state: false,
            prev_vp_j: None,
            ids,
            groups,
            masks: vec![0xFF; n],
            uploaded_masks: vec![0xFF; n],
            prev_p2_rects: Vec::new(),
            prev_p2_inst: Vec::new(),
            last_rects_p1: Vec::new(),
            last_rects_inst: Vec::new(),
            n_levels,
        })
    }

    /// 单次提交（两阶段调度的一拍）：参数三小件 + rect 双流 + 掩码 TLAS 更新
    /// （等价跳过）+ parity 三 pass 绑定轮换 + 回读子集。
    #[allow(clippy::too_many_arguments)]
    fn submit_once(
        &mut self,
        scene_params: &[f32],
        mv_params: &[f32],
        tsr_params: &[f32],
        n_p1: u32,
        rects_p2: &[f32],
        n_p2: u32,
        masks: &[u8],
        readback: G31Readback,
        probe_pre: bool,
        iw: u32,
        ih: u32,
    ) -> Result<DeviceFrameOutput, String> {
        let params_p1 = [
            n_p1 as f32,
            self.n_levels as f32,
            iw as f32,
            ih as f32,
            G31_HZB_CONV_FLAG,
            0.0,
            0.0,
            0.0,
        ];
        let params_p2 = [
            n_p2 as f32,
            self.n_levels as f32,
            iw as f32,
            ih as f32,
            G31_HZB_CONV_FLAG,
            0.0,
            0.0,
            0.0,
        ];
        let ids = &self.ids;
        let mut uploads: Vec<(StableResourceId, u64, Vec<u8>)> = vec![
            (
                StableResourceId(u64::from(U_SCENE_PARAMS) + 1),
                0,
                bytes_f32(scene_params),
            ),
            (
                StableResourceId(u64::from(U_MV_PARAMS) + 1),
                0,
                bytes_f32(mv_params),
            ),
            (
                StableResourceId(u64::from(U_TSR_PARAMS) + 1),
                0,
                bytes_f32(tsr_params),
            ),
        ];
        // rect 流空段不上传（执行器 fail-closed 拒空段;kernel 以 params[0]=n 门
        // 守卫,缓冲陈旧段永不被消费——n=0 拍跳过上传零语义差）。
        if !self.last_rects_p1.is_empty() {
            uploads.push((
                StableResourceId(u64::from(ids.rects_p1) + 1),
                0,
                bytes_f32(&self.last_rects_p1),
            ));
        }
        uploads.push((
            StableResourceId(u64::from(ids.params_p1) + 1),
            0,
            bytes_f32(&params_p1),
        ));
        if !rects_p2.is_empty() {
            uploads.push((
                StableResourceId(u64::from(ids.rects_p2) + 1),
                0,
                bytes_f32(rects_p2),
            ));
        }
        uploads.push((
            StableResourceId(u64::from(ids.params_p2) + 1),
            0,
            bytes_f32(&params_p2),
        ));
        // 掩码 TLAS 更新（等价跳过：静态相机稳态逐帧同掩码 ⇒ 零 TLAS 税;
        // 掩码变化 ⇒ Rebuild——mask 字段在实例缓冲内,write_transforms 同律覆盖）。
        let tlas_update = if masks != self.uploaded_masks.as_slice() {
            let insts: Vec<RayQueryTransformedInstanceDesc> = masks
                .iter()
                .enumerate()
                .map(|(i, &m)| RayQueryTransformedInstanceDesc {
                    blas: i as u32,
                    custom_index: i as u32,
                    mask: m,
                    sbt_record_offset: 0,
                    transform: vk::RAY_QUERY_IDENTITY_TRANSFORM,
                })
                .collect();
            Some((0u32, insts, TlasBuildAction::Refit))
        } else {
            None
        };
        let p = self.parity;
        let binding_overrides = vec![
            (
                3u32,
                Bindings {
                    storage_buffers: vec![
                        U_SCENE_COLOR,
                        U_SCENE_DEPTH,
                        U_TSR_PARAMS,
                        U_CUR_RGB,
                        U_LUMA[p],
                        U_DEPTH_HI[p],
                    ],
                    ..Bindings::default()
                },
            ),
            (
                4u32,
                Bindings {
                    storage_buffers: vec![
                        U_CUR_RGB,
                        U_LUMA[p],
                        U_DEPTH_HI[p],
                        U_MV_OUT,
                        U_REACTIVE,
                        U_OUT_COLOR[1 - p],
                        U_DEPTH_HI[1 - p],
                        U_LUMA[1 - p],
                        U_OUT_SIGN[1 - p],
                        U_OUT_SCORE[1 - p],
                        U_TSR_PARAMS,
                        U_OUT_COLOR[p],
                        U_OUT_SIGN[p],
                        U_OUT_SCORE[p],
                    ],
                    ..Bindings::default()
                },
            ),
            (
                5u32,
                Bindings {
                    storage_buffers: vec![U_OUT_COLOR[p], G31_U_ENC_PARAMS, G31_U_ENC_OUT],
                    ..Bindings::default()
                },
            ),
        ];
        // 回读子集（序即解析序）：BGRA8 → f32 末帧/probe → probe_pre 深度+平铺
        // → p1/p2 判定（逐帧恒在,决策面）。
        let mut subset: Vec<u32> = Vec::new();
        if readback != G31Readback::None {
            subset.push(G31_RB_BGRA);
            if readback == G31Readback::BgraAndColor {
                subset.push(p as u32);
            }
        }
        if probe_pre {
            // probe 深度面 = depth_hz 真深度（回读下标 8;剔除链深度域与设备
            // 平铺 mip0 位级同源——3 = U_SCENE_DEPTH 生产字面不供剔除链消费）。
            subset.push(8);
            subset.push(ids.rb_flat);
        }
        subset.push(ids.rb_verdicts_p1);
        subset.push(ids.rb_verdicts_p2);
        let update = FrameUpdate {
            tlas_update,
            buffer_uploads: uploads,
            binding_overrides,
            push_constant_overrides: vec![],
            readback_subset: Some(subset),
            blas_refit: None, // G31+ 波 B Task B5 字段面:本车道无 BLAS refit(0-byte 默认)
        };
        let prov = self.session.next_provenance_with_update(&update)?;
        let out = self.session.execute_with_frame_update(&prov, &update)?;
        if update.tlas_update.is_some() {
            self.uploaded_masks = masks.to_vec();
        }
        Ok(out)
    }

    /// 一帧：初剔分类（host）→ 提交（两阶段 pass 序）→ collect 结算应见集 →
    /// 误剔/出新闭环重渲（迭代上限 + 全掩码兜底）→ 终判掩码/被剔集滚动。
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        eps: f32,
        quad_count: usize,
        point_count: usize,
        inv_vp: &Mat4,
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback: G31Readback,
        probe_pre: bool,
    ) -> Result<G31FrameRec, String> {
        let t_host = std::time::Instant::now();
        // ── ① 初剔分类（视锥面 + rect 流;cull::Frustum 冻结金标准只读消费）──
        let class = g31_hzb_classify(vp, iw, ih, &self.groups);
        let n = self.groups.len();
        let mut rects: Vec<f32> = Vec::with_capacity(n * 5);
        let mut rect_inst: Vec<u32> = Vec::with_capacity(n);
        let mut offscreen = 0u32;
        for (i, c) in class.iter().enumerate() {
            match c {
                G31HzbClass::Offscreen => offscreen += 1,
                G31HzbClass::Rect {
                    uv_min,
                    uv_max,
                    nearest,
                } => {
                    rect_inst.push(i as u32);
                    rects.extend_from_slice(&[uv_min[0], uv_min[1], uv_max[0], uv_max[1], *nearest]);
                }
            }
        }
        self.last_rects_p1 = rects.clone();
        self.last_rects_inst = rect_inst.clone();
        let n_p1 = rect_inst.len() as u32;
        // ── ② 帧参数三小件（与 off 车道同一打包面逐字同源）──
        let scene_params =
            pack_frame_params(iw, ih, jitter, eps, quad_count, point_count, inv_vp, vp);
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆（mv 参数面）")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        let has_history = !reset && self.has_history_state;
        let tsr_params = pack_tsr_params(iw, ih, ow, oh, jitter, exposure, has_history, false);
        // host 决策面耗时 = 初剔分类 + rect 流打包段（逐提交闭环保守面为 µs 级,
        // 不重复计 GPU 提交段）。
        let host_ms = t_host.elapsed().as_secs_f64() * 1000.0;

        // ── ③ 两阶段提交 + 闭环重渲循环 ──
        let mut rendered = self.masks.clone();
        let mut p2_rects = self.prev_p2_rects.clone();
        let mut p2_inst = self.prev_p2_inst.clone();
        let mut closure_extra_submits = 0u32;
        let mut closure_full_fallback = false;
        let mut hzb_gpu_ns = 0.0f64;
        let mut prod_gpu_total_ns = 0.0f64;
        // 主提交 p1 判定面（probe 对拍消费——「上帧金字塔初剔」字面;闭环重拍的
        // p1 读本帧重建金字塔属第二阶段调度,不进对拍面）。
        let mut v1_main: Option<Vec<u8>> = None;
        // 末次提交面（循环出口赋值）:判定/遥测/回读归属。
        let (out_last, v1_last, v2_last, p2_inst_last);
        loop {
            let n_p2 = p2_inst.len() as u32;
            let out = self.submit_once(
                &scene_params,
                &mv_params,
                &tsr_params,
                n_p1,
                &p2_rects,
                n_p2,
                &rendered,
                readback,
                probe_pre,
                iw,
                ih,
            )?;
            let (v1, v2) = g31_hzb_parse_verdicts(&out, readback, probe_pre, n_p1, n_p2)?;
            if v1_main.is_none() {
                v1_main = Some(v1.clone());
            }
            let prod_ns = g31_hzb_prod_gpu_ns(&out)?;
            prod_gpu_total_ns += prod_ns;
            hzb_gpu_ns += g31_hzb_aux_gpu_ns(&out);
            // 应见集结算：p1 可见 ∪ p2 翻回（offscreen 恒剔）。
            let mut correct = vec![0u8; n];
            for (j, &inst) in rect_inst.iter().enumerate() {
                if v1[j] == 0 {
                    correct[inst as usize] = 0xFF;
                }
            }
            for (j, &inst) in p2_inst.iter().enumerate() {
                if v2[j] == 0 {
                    correct[inst as usize] = 0xFF;
                }
            }
            let need = (0..n).any(|i| correct[i] == 0xFF && rendered[i] == 0);
            if !need {
                out_last = out;
                v1_last = v1;
                v2_last = v2;
                p2_inst_last = p2_inst;
                break;
            }
            // 闭环：并集掩码重渲（并集内每一员要么应见、要么被并集内他员遮挡
            // ⇒ 超集渲染像素安全;金字塔逐次更完备 ⇒ 遮挡集单调扩 ⇒ 不振荡）。
            for (i, c) in correct.iter().enumerate() {
                if *c == 0xFF {
                    rendered[i] = 0xFF;
                }
            }
            // 下一拍重测集 = 在屏且仍被剔（并集外）。
            p2_rects = Vec::new();
            p2_inst = Vec::new();
            for (j, &inst) in rect_inst.iter().enumerate() {
                if rendered[inst as usize] == 0 {
                    p2_inst.push(inst);
                    p2_rects.extend_from_slice(&rects[j * 5..j * 5 + 5]);
                }
            }
            closure_extra_submits += 1;
            if closure_extra_submits >= G31_HZB_CLOSURE_MAX {
                // 迭代上限耗尽 ⇒ 全掩码兜底重渲（= 零剔除精确收敛,必终止）。
                rendered = vec![0xFF; n];
                p2_rects = Vec::new();
                p2_inst = Vec::new();
                closure_full_fallback = true;
                let out2 = self.submit_once(
                    &scene_params,
                    &mv_params,
                    &tsr_params,
                    n_p1,
                    &p2_rects,
                    0,
                    &rendered,
                    readback,
                    probe_pre,
                    iw,
                    ih,
                )?;
                let (v1b, v2b) = g31_hzb_parse_verdicts(&out2, readback, probe_pre, n_p1, 0)?;
                prod_gpu_total_ns += g31_hzb_prod_gpu_ns(&out2)?;
                hzb_gpu_ns += g31_hzb_aux_gpu_ns(&out2);
                out_last = out2;
                v1_last = v1b;
                v2_last = v2b;
                p2_inst_last = p2_inst;
                break;
            }
        }

        // ── ④ 终判滚动：下帧渲染掩码 = 本帧应见集（末次提交判定面）;
        //    下帧 p2 重测集 = 本帧终判被剔（在屏且应见集外）。──
        let mut visible_final = vec![0u8; n];
        for (j, &inst) in rect_inst.iter().enumerate() {
            if v1_last[j] == 0 {
                visible_final[inst as usize] = 0xFF;
            }
        }
        for (j, &inst) in p2_inst_last.iter().enumerate() {
            if v2_last[j] == 0 {
                visible_final[inst as usize] = 0xFF;
            }
        }
        let mut next_p2_rects: Vec<f32> = Vec::new();
        let mut next_p2_inst: Vec<u32> = Vec::new();
        for (j, &inst) in rect_inst.iter().enumerate() {
            if visible_final[inst as usize] == 0 {
                next_p2_inst.push(inst);
                next_p2_rects.extend_from_slice(&rects[j * 5..j * 5 + 5]);
            }
        }
        self.masks = visible_final;
        self.prev_p2_rects = next_p2_rects;
        self.prev_p2_inst = next_p2_inst;

        // ── ⑤ 产物组装（遥测 = 末次提交;HZB/闭环追加 GPU 分列;判定面 = 主提交）──
        let prod_last_ns = g31_hzb_prod_gpu_ns(&out_last)?;
        let closure_extra_ns = prod_gpu_total_ns - prod_last_ns;
        let verdicts_p1_rec = v1_main.clone().unwrap_or_else(|| v1_last.clone());
        let rec = self.rec_from_output_hz(
            out_last,
            readback,
            probe_pre,
            ow,
            oh,
            prod_gpu_total_ns,
            hzb_gpu_ns,
            G31HzbFrameRec {
                tested_p1: n_p1,
                occluded_p1: 0, // 占位——由 rec_from_output_hz 统计口径填入（见下）
                offscreen,
                retested_p2: 0,
                flipped_p2: 0,
                closure_extra_submits,
                closure_full_fallback,
                visible_final: self.masks.iter().filter(|&&m| m == 0xFF).count() as u32,
                hzb_gpu_ns,
                closure_extra_gpu_ns: closure_extra_ns,
                host_ms,
                probe_depth: None,
                probe_flat: None,
                verdicts_p1: verdicts_p1_rec.clone(),
                rects_p1: self.last_rects_p1.clone(),
                rects_inst_p1: self.last_rects_inst.clone(),
            },
            &verdicts_p1_rec,
            &p2_inst_last,
            &v2_last,
        )?;
        self.prev_vp_j = Some(*vp_j);
        self.has_history_state = true;
        self.parity = 1 - self.parity;
        Ok(rec)
    }

    /// 末次提交产物组装（G31FrameRec 既有面 + hzb 块;遥测按 pass 名提取）。
    #[allow(clippy::too_many_arguments)]
    fn rec_from_output_hz(
        &self,
        mut out: DeviceFrameOutput,
        readback: G31Readback,
        probe_pre: bool,
        ow: u32,
        oh: u32,
        prod_gpu_total_ns: f64,
        hzb_gpu_ns: f64,
        mut hz: G31HzbFrameRec,
        v1_last: &[u8],
        p2_inst_last: &[u32],
        v2_last: &[u8],
    ) -> Result<G31FrameRec, String> {
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        let scene_gpu_ns = gpu("g31_hzb_primary")? + gpu("g31_hzb_shade")?;
        let mv_gpu_ns = gpu("g14_mv")?;
        let resample_gpu_ns = gpu("g14_8_tsr_resample")?;
        let resolve_gpu_ns = gpu("g14_8_tsr_resolve")?;
        let encode_gpu_ns = gpu("g31_display_encode")?;
        let t_convert = std::time::Instant::now();
        let bgra_px = (ow * oh * 4) as usize;
        let f32_px = (ow * oh * 3) as usize;
        let mut idx = 0usize;
        let take_rb = |out: &mut DeviceFrameOutput, idx: &mut usize| -> Result<Vec<u8>, String> {
            if *idx >= out.readbacks.len() {
                return Err(format!(
                    "B1 回读路数 {} 少于子集消费序 {idx}",
                    out.readbacks.len()
                ));
            }
            let b = std::mem::take(&mut out.readbacks[*idx]);
            *idx += 1;
            Ok(b)
        };
        let (bgra8, out_color) = if readback == G31Readback::None {
            (None, None)
        } else {
            let b = take_rb(&mut out, &mut idx)?;
            if b.len() != bgra_px {
                return Err(format!("B1 BGRA8 回读字节 {} ≠ {}x{}x4", b.len(), ow, oh));
            }
            let oc = if readback == G31Readback::BgraAndColor {
                let data = read_f32(&take_rb(&mut out, &mut idx)?);
                if data.len() != f32_px {
                    return Err("B1 f32 回读字节数与输出分辨率不符".into());
                }
                Some(data)
            } else {
                None
            };
            (Some(b), oc)
        };
        let (probe_depth, probe_flat) = if probe_pre {
            let d = read_f32(&take_rb(&mut out, &mut idx)?);
            let f = read_f32(&take_rb(&mut out, &mut idx)?);
            (Some(d), Some(f))
        } else {
            (None, None)
        };
        // 判定两路（逐帧恒在子集末两位）。
        let _ = take_rb(&mut out, &mut idx)?; // verdicts_p1 字节（已在 frame() 解析消费）
        let _ = take_rb(&mut out, &mut idx)?; // verdicts_p2
        if idx != out.readbacks.len() {
            return Err(format!(
                "B1 回读消费序 {idx} ≠ 实到路数 {}",
                out.readbacks.len()
            ));
        }
        hz.occluded_p1 = v1_last.iter().filter(|&&b| b == 1).count() as u32;
        hz.retested_p2 = p2_inst_last.len() as u32;
        hz.flipped_p2 = v2_last.iter().filter(|&&b| b == 0).count() as u32;
        hz.probe_depth = probe_depth;
        hz.probe_flat = probe_flat;
        hz.hzb_gpu_ns = hzb_gpu_ns;
        let _ = prod_gpu_total_ns;
        let readback_convert_ms = t_convert.elapsed().as_secs_f64() * 1000.0;
        // C7 profiler 面:全量逐 pass GPU 计时（telemetry 声明序直拷）。
        let pass_gpu_ns: Vec<(String, f64)> = out
            .telemetry
            .passes
            .iter()
            .map(|pp| (pp.name.clone(), pp.gpu_ns))
            .collect();
        Ok(G31FrameRec {
            scene_gpu_ns,
            mv_gpu_ns,
            resample_gpu_ns,
            resolve_gpu_ns,
            encode_gpu_ns,
            fg_gpu_ns: 0.0,
            cpu_record_ns: out.telemetry.cpu_record_ns,
            cpu_submit_ns: out.telemetry.cpu_submit_ns,
            cpu_fence_wait_ns: out.telemetry.cpu_fence_wait_ns,
            validation_error_count: out.telemetry.validation_error_count,
            leaked_object_count: out.telemetry.leaked_object_count,
            leaked_allocation_count: out.telemetry.leaked_allocation_count,
            bgra8,
            gen_bgra8: Vec::new(),
            out_color,
            probe_prev_color: None,
            probe_mv: None,
            probe_gen_out: Vec::new(),
            probe_mvn: None,
            readback_convert_ms,
            hzb: Some(hz),
            svt_requests: None, // B1 HZB 面与 C13 SVT 闭集互斥（恒 None）
            pass_gpu_ns,
        })
    }
}

/// B1 生产链六段 GPU（primary+shade+mv+resample+resolve+encode;末次提交口径
/// 由 rec_from_output_hz 逐名提取,本面供闭环追加量分列）。
fn g31_hzb_prod_gpu_ns(out: &DeviceFrameOutput) -> Result<f64, String> {
    let mut sum = 0.0;
    for name in [
        "g31_hzb_primary",
        "g31_hzb_shade",
        "g14_mv",
        "g14_8_tsr_resample",
        "g14_8_tsr_resolve",
        "g31_display_encode",
    ] {
        sum += out
            .telemetry
            .passes
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.gpu_ns)
            .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))?;
    }
    Ok(sum)
}

/// B1 HZB 辅助 pass GPU 合计（g27_hzb_reduce_l*/g27_hzb_test_p*/g31_hzb_pack_l*
/// 前缀族;缺行 = 0 容差〔辅助面不全不冒充,主链六段缺失才 fail〕）。
fn g31_hzb_aux_gpu_ns(out: &DeviceFrameOutput) -> f64 {
    out.telemetry
        .passes
        .iter()
        .filter(|p| {
            p.name.starts_with("g27_hzb_reduce_l")
                || p.name.starts_with("g27_hzb_test_p")
                || p.name.starts_with("g31_hzb_pack_l")
        })
        .map(|p| p.gpu_ns)
        .sum()
}

/// B1 判定回读解析（子集序 = 末两位;f32 恒 ∈ {0.0,1.0} 门输出 ⇒ >0.5 判读
/// 字节,g27 harness 同律）。返回 (p1 字节列, p2 字节列)（各取前 n 项）。
fn g31_hzb_parse_verdicts(
    out: &DeviceFrameOutput,
    readback: G31Readback,
    probe_pre: bool,
    n_p1: u32,
    n_p2: u32,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let base = match readback {
        G31Readback::None => 0,
        G31Readback::Bgra => 1,
        G31Readback::BgraAndColor => 2,
    } + if probe_pre { 2 } else { 0 };
    let rbs = &out.readbacks;
    if rbs.len() != base + 2 {
        return Err(format!(
            "B1 判定回读路数 {} ≠ {}（readback={readback:?} probe_pre={probe_pre}）",
            rbs.len(),
            base + 2
        ));
    }
    let v1f = read_f32(&rbs[base]);
    let v2f = read_f32(&rbs[base + 1]);
    if (v1f.len() as u32) < n_p1 || (v2f.len() as u32) < n_p2 {
        return Err("B1 判定回读长度小于本拍 rect 数".into());
    }
    let to_bytes = |v: &[f32], n: u32| -> Vec<u8> {
        v.iter()
            .take(n as usize)
            .map(|&x| u8::from(x > 0.5))
            .collect()
    };
    Ok((to_bytes(&v1f, n_p1), to_bytes(&v2f, n_p2)))
}

/// B1 车道形态分派（off = A3 五 pass 车道 0-byte;Hzb = B1 两阶段剔除车道）。
enum G31AnyLane<'a> {
    Off(G31TsrLane<'a>),
    Hzb(G31HzbLane<'a>),
}

/// B1 接线态对拍结果（evidence `hzb.parity` 组装面;判据 = RFC-0044 §1.2 同构
/// 口径的接线态复跑:逐级位级全等 + 判定序列逐字节全等 + 零假阳性独立复核）。
struct G31HzbWiredParity {
    mips: usize,
    n_rects: u32,
    mips_bitexact: bool,
    verdict_equal: bool,
    false_positives: u32,
    occluded: u32,
    pyramid_digest: String,
    host_pyramid_digest: String,
    verdict_digest: String,
    host_verdict_digest: String,
}

/// B1 probe 帧 host 金标准复算对拍（hzb.rs 冻结面只读消费;深度/平铺 =
/// device 回读原字节）。判据三面：
/// ① 车道平铺金字塔 vs host `HzbPyramid::build` 逐级位级全等（to_bits;零容差
///    协议 §1.1——纯 min/max 选择归约 + 纯拷贝 pack glue ⇒ 位级蕴含）;
/// ② p1 判定序列 vs host `test_rect` 逐 rect 逐字节全等（同一金字塔〔上帧
///    深度〕+ 同一 rect 流 ⇒ 生产语义字面——上帧金字塔初剔的对拍）;
/// ③ 零假阳性硬不变量：device 判 Occluded ⇒ `exact_rect_occluded`（对上帧
///    深度——device 消费的金字塔同源）必同判。
#[allow(clippy::too_many_arguments)]
fn g31_hzb_wired_parity(
    depth_data: &[f32],
    flat_data: &[f32],
    iw: u32,
    ih: u32,
    levels: &[(u32, u32)],
    flat_offsets: &[u32],
    rects: &[f32],
    verdicts: &[u8],
) -> Result<G31HzbWiredParity, String> {
    if depth_data.len() != (iw * ih) as usize {
        return Err(format!(
            "probe 深度回读 {} ≠ {}x{}",
            depth_data.len(),
            iw,
            ih
        ));
    }
    let depth_img = ImageF32 {
        w: iw,
        h: ih,
        c: 1,
        data: depth_data.to_vec(),
    };
    let host = HzbPyramid::build(&depth_img, DepthConvention::StandardZ);
    // ① 逐级位级（平铺偏移逐級比;零容差）。
    let mut mips_bitexact = host.mips.len() == levels.len();
    if host.mips.len() != levels.len() {
        eprintln!(
            "[g31_window_present]: HZB 对拍① 级数不等 host={} lane={}",
            host.mips.len(),
            levels.len()
        );
    }
    if mips_bitexact {
        'levels: for (k, m) in host.mips.iter().enumerate() {
            let off = flat_offsets[k] as usize;
            if (m.w, m.h) != levels[k] || off + m.data.len() > flat_data.len() {
                mips_bitexact = false;
                break;
            }
            for (j, v) in m.data.iter().enumerate() {
                if flat_data[off + j].to_bits() != v.to_bits() {
                    // 归因面：首失配点上级 footprint 四纹素位型（红路径诊断）。
                    let (mw, _mh) = (m.w as usize, m.h as usize);
                    let (lx, ly) = (j % mw, j / mw);
                    let (pw, ph) = (levels[k - 1].0 as usize, levels[k - 1].1 as usize);
                    let poff = flat_offsets[k - 1] as usize;
                    let mut fpv = Vec::new();
                    for &(cx, cy) in &[(lx * 2, ly * 2), (lx * 2 + 1, ly * 2), (lx * 2, ly * 2 + 1), (lx * 2 + 1, ly * 2 + 1)] {
                        let (ccx, ccy) = (cx.min(pw - 1), cy.min(ph - 1));
                        fpv.push(flat_data[poff + ccy * pw + ccx]);
                    }
                    eprintln!(
                        "[g31_window_present]: HZB 对拍① 首失配 level={k} j={j}（{},{} 内）dev={:08x} host={:08x}（dev_f={} host_f={}）footprint_bits={:?}",
                        m.w,
                        m.h,
                        flat_data[off + j].to_bits(),
                        v.to_bits(),
                        flat_data[off + j],
                        v,
                        fpv.iter().map(|x| format!("{:08x}", x.to_bits())).collect::<Vec<_>>()
                    );
                    mips_bitexact = false;
                    break 'levels;
                }
            }
        }
    }
    // ② 判定序列逐字节。
    let host_seq: Vec<u8> = rects
        .chunks_exact(5)
        .map(|r| match host.test_rect([r[0], r[1]], [r[2], r[3]], r[4]) {
            Occlusion::Occluded => 1u8,
            Occlusion::Visible => 0u8,
        })
        .collect();
    let verdict_equal = host_seq.as_slice() == verdicts;
    if !verdict_equal {
        // 归因面：② 首差点位（红路径诊断）。
        let mut shown = 0usize;
        for (j, (&h, &d)) in host_seq.iter().zip(verdicts.iter()).enumerate() {
            if h != d && shown < 4 {
                let r = &rects[j * 5..j * 5 + 5];
                eprintln!(
                    "[g31_window_present]: HZB 对拍② 首差 j={j} dev={d} host={h} rect=[{:.6},{:.6},{:.6},{:.6}] nearest={:.6}",
                    r[0], r[1], r[2], r[3], r[4]
                );
                shown += 1;
            }
        }
        eprintln!(
            "[g31_window_present]: HZB 对拍② 差异计数 {}",
            host_seq
                .iter()
                .zip(verdicts.iter())
                .filter(|(a, b)| a != b)
                .count()
        );
    }
    // ③ 零假阳性独立复核（对上帧深度——device 初剔消费的金字塔同源）。
    let mut fp = 0u32;
    let mut occ = 0u32;
    for (j, &b) in verdicts.iter().enumerate() {
        if b == 1 {
            occ += 1;
            let r = &rects[j * 5..j * 5 + 5];
            if !exact_rect_occluded(
                &depth_img,
                DepthConvention::StandardZ,
                [r[0], r[1]],
                [r[2], r[3]],
                r[4],
            ) {
                fp += 1;
            }
        }
    }
    // digest（F11 字面同律:判定字节序 ‖ 金字塔逐级 f32 LE）。
    let mut pyr_bytes = Vec::with_capacity(flat_data.len() * 4);
    for v in flat_data {
        pyr_bytes.extend_from_slice(&v.to_le_bytes());
    }
    let mut host_pyr_bytes = Vec::new();
    for m in &host.mips {
        for v in &m.data {
            host_pyr_bytes.extend_from_slice(&v.to_le_bytes());
        }
    }
    let mut vtrace = verdicts.to_vec();
    vtrace.extend_from_slice(&pyr_bytes);
    let mut htrace = host_seq.clone();
    htrace.extend_from_slice(&host_pyr_bytes);
    Ok(G31HzbWiredParity {
        mips: host.mips.len(),
        n_rects: verdicts.len() as u32,
        mips_bitexact,
        verdict_equal,
        false_positives: fp,
        occluded: occ,
        pyramid_digest: format!("sha256:{}", sha256_hex(&pyr_bytes)),
        host_pyramid_digest: format!("sha256:{}", sha256_hex(&host_pyr_bytes)),
        verdict_digest: format!("sha256:{}", sha256_hex(&vtrace)),
        host_verdict_digest: format!("sha256:{}", sha256_hex(&htrace)),
    })
}


// ---------------------------------------------------------------------------
// A5 接线态对拍探针：probe 帧回读 prev/cur f32 + MV + 生成帧 f32,host 金标准
// `temporal::framegen::interpolate`（0-byte 消费面）以 −mv 复算对拍——p100 ≤
// G26 冻结容差 + SSIM(device, hostref) > SSIM(frame-hold, hostref)。
// ---------------------------------------------------------------------------

/// A5 接线态对拍结果（evidence `wired_parity` 组装面）。
struct G31WiredParity {
    /// device vs host 金标准逐像素最大绝对差（实测事实登记面）。
    p100: f64,
    per_gen_p100: Vec<f64>,
    /// 逐像素结构界最大值（事实登记）。
    max_bound: f64,
    /// 全帧 max(0, |dev−host| − bound)——硬门:恒 0（任何结构缺陷必超界）。
    excess: f64,
    /// 全帧 max(|dev−host| / bound)——硬门:≤ 1。
    excess_ratio: f64,
    in_bound: bool,
    /// device 侧 MVN 与 −MV 回读直比对 max|mvn+mv|（MV 通路位级硬门:恒 0;
    /// probe 帧回读组装期填入）。
    mvn_max_abs_plus_mv: f64,
    ssim_device_vs_hostref: f64,
    ssim_frame_hold_vs_hostref: f64,
    ssim_beats_frame_hold: bool,
    t_values: Vec<f32>,
}

/// A5 接线态对拍结构界——值项保守界（≈8×ULP(1.0):lerp 三段 + exp + 遮挡
/// 混合链舍入,f32 结构推导非手写阈）。
const G31_PROBE_VAL_ULP_ERR: f64 = 2.0e-6;

/// A5 探针采样 16-texel 邻域极差（与 host image.rs::sample_bilinear 同一坐标/
/// clamp 形;(x0−1..=x0+2)×(y0−1..=y0+2) 逐通道 max−min——坐标 ULP 舍入跨纹素
/// 边界翻转（fx/fy 在 0/1 界两侧跳变,采样落入相邻 texel 对）的全覆盖 hull:
/// 设备/host 坐标算术差（FMA 收缩等,≪1px）下任一侧采样值恒在 hull 内）。
fn g31_probe_tap_range16(img: &ImageF32, u: f32, v: f32, ch: u32) -> f32 {
    let xf = u * img.w as f32 - 0.5;
    let yf = v * img.h as f32 - 0.5;
    let x0 = xf.floor() as i32;
    let y0 = yf.floor() as i32;
    let cx = |xx: i32| xx.clamp(0, img.w as i32 - 1) as u32;
    let cy = |yy: i32| yy.clamp(0, img.h as i32 - 1) as u32;
    let mut mx = f32::NEG_INFINITY;
    let mut mn = f32::INFINITY;
    for dy in -1i32..=2 {
        for dx in -1i32..=2 {
            let t = img.get(cx(x0 + dx), cy(y0 + dy), ch);
            mx = mx.max(t);
            mn = mn.min(t);
        }
    }
    mx - mn
}

/// A5 接线态对拍复算：device 生成帧（取反 glue 直通馈入面）vs host 金标准
/// `interpolate(prev, cur, −mv, t_temporal)`（t_i = i/(n+1) 与 device 参数面
/// 同一 f32 位级传参）。frame-hold = 复制 prev 真渲帧（G26 判据同义）。
///
/// 判据面（L1 敏感性分析结构界,非手写阈）：
/// G26 冻结绝对容差（128×72 单位域合成场景标定）在 1080p HDR 生产帧上物理
/// 不适用——诊断实证（probe 帧 run_compute 三方比对 max|lane−run_compute|=0
/// 位级,接线零缺陷）：kernel/host 双方正确 f32 实现的算术差（设备侧 FMA
/// 收缩/坐标积舍入）经两种机制放大——①坐标舍入跨纹素边界翻转采样（值跳变
/// = 边界两侧 texel 差）②w_cons 混合交叉项（d2 扰动经 1/σ² 放大)。故硬门 =
/// 逐像素结构界
/// `bound(x,ch) = frozen_floor + (rangeA16 + rangeB16) + VAL_ULP_ERR×scale
///              + 0.5×|a−b|×w×min(1,δlog)×e`
/// （frozen_floor = G26 标定 threshold 程序读;rangeA16/B16 = a/b 采样 16-texel
/// 邻域逐通道极差,坐标翻转全覆盖;δlog = inv_sigma2×Σ_ch(2|s_ch|(rA+rB)+
/// (rA+rB)²) 为 d2 扰动上界,混合交叉项 = |∂out/∂w|×|Δw| 保守形——全部因子
/// f32/公式结构推导）。结构缺陷（tie-break/缓冲/MV 符号/t 错误）产生 0.1~15
/// 量级差异必超界;p100 作实测事实登记。
#[allow(clippy::too_many_arguments)]
fn g31_wired_parity_probe(
    prev: &[f32],
    cur: &[f32],
    mv: &[f32],
    dev_gens: &[Vec<f32>],
    w: u32,
    h: u32,
    frozen_floor: f64,
) -> Result<G31WiredParity, String> {
    let prev_img = ImageF32 {
        w,
        h,
        c: 3,
        data: prev.to_vec(),
    };
    let cur_img = ImageF32 {
        w,
        h,
        c: 3,
        data: cur.to_vec(),
    };
    // MV 约定换算：g14 m(x) = prev_uv − x → host 面 mv = −m（device 侧由
    // g31_mv_negate 逐元素取反兑现同值;文件头 A5 §2）。
    let mv_neg = ImageF32 {
        w,
        h,
        c: 2,
        data: mv.iter().map(|v| -v).collect(),
    };
    let n = dev_gens.len() as u32;
    if !(1..=2).contains(&n) {
        return Err(format!("A5 probe 生成帧数 {n} 越闭集 1..=2"));
    }
    let params = FrameGenParams {
        inserted_per_pair: n,
        ..FrameGenParams::default()
    };
    let inv_s2 = f64::from(1.0f32 / (params.consistency_sigma * params.consistency_sigma));
    let (wf, hf) = (w as f32, h as f32);
    let mut p100 = 0.0f64;
    let mut per_gen_p100 = Vec::new();
    let mut t_values = Vec::new();
    let mut max_bound = 0.0f64;
    let mut excess = 0.0f64;
    let mut excess_ratio = 0.0f64;
    let mut ssim_device_min = f64::INFINITY;
    let mut ssim_hold_max = f64::NEG_INFINITY;
    for (k, dev) in dev_gens.iter().enumerate() {
        let t = (k as u32 + 1) as f32 / (n + 1) as f32;
        t_values.push(t);
        let host = interpolate(&prev_img, &cur_img, &mv_neg, t, &params);
        let dev_img = ImageF32 {
            w,
            h,
            c: 3,
            data: dev.clone(),
        };
        let cell = dev
            .iter()
            .zip(host.data.iter())
            .map(|(&x, &y)| (x - y).abs() as f64)
            .fold(0.0, f64::max);
        p100 = p100.max(cell);
        per_gen_p100.push(cell);
        // 逐像素结构界核验（host 采样坐标/a/b/16-texel 极差/混合交叉项复算面）。
        for y in 0..h {
            for x in 0..w {
                let px = (y * w + x) as usize;
                let u = (x as f32 + 0.5) / wf;
                let v = (y as f32 + 0.5) / hf;
                let mvx = mv_neg.data[px * 2];
                let mvy = mv_neg.data[px * 2 + 1];
                let (ua, va) = (u - t * mvx, v - t * mvy);
                let (ub, vb) = (u + (1.0 - t) * mvx, v + (1.0 - t) * mvy);
                let a = prev_img.sample_bilinear3(ua, va);
                let b = cur_img.sample_bilinear3(ub, vb);
                // 三通道先行量:d2/w_cons(host 同式)与逐通道 s/16-texel 极差。
                let mut d2 = 0.0f64;
                let mut s = [0.0f64; 3];
                let mut ra = [0.0f32; 3];
                let mut rb = [0.0f32; 3];
                for ch in 0..3 {
                    s[ch] = f64::from((a[ch] - b[ch]).abs());
                    d2 += s[ch] * s[ch];
                    ra[ch] = g31_probe_tap_range16(&prev_img, ua, va, ch as u32);
                    rb[ch] = g31_probe_tap_range16(&cur_img, ub, vb, ch as u32);
                }
                let w_cons = (-d2 * inv_s2).exp();
                // d2 扰动上界(L1:|(s+δ)²−s²| ≤ 2|s|·|δ|+|δ|²,|δ_ch| ≤ ra_ch+rb_ch)。
                let mut d2_perturb = 0.0f64;
                for ch in 0..3 {
                    let rr = f64::from(ra[ch] + rb[ch]);
                    d2_perturb += 2.0 * s[ch] * rr + rr * rr;
                }
                let dlog = (inv_s2 * d2_perturb).min(1.0);
                for ch in 0..3 {
                    let scale = (a[ch].abs()).max(b[ch].abs()).max(1.0);
                    // 混合交叉项:|∂out/∂w| = |lin−near| ≤ 0.5|s_ch|(t=0.5 最劣
                    // 闭式;|Δw| ≤ w·min(1,δlog)·e^min(1,δlog) 保守形)。
                    let cross = 0.5
                        * s[ch]
                        * w_cons
                        * dlog
                        * std::f64::consts::E.powf(dlog);
                    let bound = frozen_floor
                        + f64::from(ra[ch] + rb[ch])
                        + G31_PROBE_VAL_ULP_ERR * f64::from(scale)
                        + cross;
                    let d = (dev[px * 3 + ch] - host.data[px * 3 + ch]).abs() as f64;
                    excess = excess.max(d - bound);
                    excess_ratio = excess_ratio.max(d / bound);
                    max_bound = max_bound.max(bound);
                }
            }
        }
        let s_dev = ssim(&dev_img, &host);
        let s_hold = ssim(&prev_img, &host);
        ssim_device_min = ssim_device_min.min(s_dev);
        ssim_hold_max = ssim_hold_max.max(s_hold);
    }
    Ok(G31WiredParity {
        p100,
        per_gen_p100,
        max_bound,
        excess: excess.max(0.0),
        excess_ratio,
        in_bound: excess <= 0.0,
        mvn_max_abs_plus_mv: 0.0,
        ssim_device_vs_hostref: ssim_device_min,
        ssim_frame_hold_vs_hostref: ssim_hold_max,
        ssim_beats_frame_hold: ssim_device_min > ssim_hold_max,
        t_values,
    })
}

// ---------------------------------------------------------------------------
// A3 游戏循环面：相机（yaw/pitch 参数化,初始 = 契约相机）+ 输入应用 +
// auto-move 确定性轨迹 + BGRA8 digest。
// ---------------------------------------------------------------------------

/// 游戏循环相机（契约 CameraSpec ↔ yaw/pitch 参数化互转;`forward` 约定 =
/// (cos p·sin y, sin p, −cos p·cos y),与契约四元数 forward=q·(0,0,−1) 同系;
/// up0 恒取契约值,roll=0 最小面）。
#[derive(Debug, Clone, Copy)]
struct G31Camera {
    eye: [f32; 3],
    yaw: f32,
    pitch: f32,
    up0: [f32; 3],
    fov_y_rad: f32,
    near: f32,
    far: f32,
}

impl G31Camera {
    fn from_spec(c: &CameraSpec) -> Self {
        let f = c.forward;
        let pitch = f[1].clamp(-1.0, 1.0).asin();
        let yaw = f[0].atan2(-f[2]);
        Self {
            eye: c.eye,
            yaw,
            pitch,
            up0: c.up0,
            fov_y_rad: c.fov_y_rad,
            near: c.near,
            far: c.far,
        }
    }

    fn forward(&self) -> [f32; 3] {
        [
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            -self.pitch.cos() * self.yaw.cos(),
        ]
    }

    fn spec(&self) -> CameraSpec {
        CameraSpec {
            eye: self.eye,
            forward: self.forward(),
            up0: self.up0,
            fov_y_rad: self.fov_y_rad,
            near: self.near,
            far: self.far,
        }
    }
}

/// 交互输入应用（W/S/A/D 平移 + Q/E 降升 + mouse/方向键视角 + `-`/`=` 曝光
/// ±0.25 ev 边沿触发;dt = 上帧墙钟秒, clamp 防挂起大跳）。VK 码:W=0x57
/// A=0x41 S=0x53 D=0x44 Q=0x51 E=0x45 方向键 0x25..=0x28 OEM_MINUS=0xBD
/// OEM_PLUS=0xBB。
fn g31_apply_input(
    cam: &mut G31Camera,
    ev100: &mut f64,
    input: &vk::ExternalInputFrame,
    prev_keys: &mut [u64; 4],
    dt: f32,
) {
    let speed = 2.5 * dt;
    let f = cam.forward();
    let fxz = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6);
    let fwd = [f[0] / fxz, 0.0, f[2] / fxz];
    let right = [-fwd[2], 0.0, fwd[0]];
    if input.key(0x57) {
        for c in 0..3 {
            cam.eye[c] += fwd[c] * speed;
        }
    }
    if input.key(0x53) {
        for c in 0..3 {
            cam.eye[c] -= fwd[c] * speed;
        }
    }
    if input.key(0x41) {
        for c in 0..3 {
            cam.eye[c] -= right[c] * speed;
        }
    }
    if input.key(0x44) {
        for c in 0..3 {
            cam.eye[c] += right[c] * speed;
        }
    }
    if input.key(0x51) {
        cam.eye[1] -= speed;
    }
    if input.key(0x45) {
        cam.eye[1] += speed;
    }
    cam.yaw += input.mouse_dx as f32 * 0.003;
    cam.pitch = (cam.pitch - input.mouse_dy as f32 * 0.003).clamp(-1.55, 1.55);
    if input.key(0x25) {
        cam.yaw -= 1.5 * dt;
    }
    if input.key(0x27) {
        cam.yaw += 1.5 * dt;
    }
    if input.key(0x26) {
        cam.pitch = (cam.pitch + 1.5 * dt).clamp(-1.55, 1.55);
    }
    if input.key(0x28) {
        cam.pitch = (cam.pitch - 1.5 * dt).clamp(-1.55, 1.55);
    }
    // 曝光边沿调整（逐帧 uniform 通路真实工作面:ev100 → exposure → TSR 128B
    // 参数逐帧上传;边沿触发防按住连跳）。
    let prev_minus = (prev_keys[0xBD / 64] >> (0xBD % 64)) & 1 != 0;
    let prev_plus = (prev_keys[0xBB / 64] >> (0xBB % 64)) & 1 != 0;
    if input.key(0xBD) && !prev_minus {
        *ev100 += 0.25;
    }
    if input.key(0xBB) && !prev_plus {
        *ev100 -= 0.25;
    }
    *ev100 = (*ev100).clamp(-8.0, 8.0);
    *prev_keys = input.keys;
}

// ---------------------------------------------------------------------------
// G31+ 波 C Task C4 故障注入探针面（--fault-probe 臂;机制 = env 注入（rt 层
// OnceLock 读取）,验证 = 本参数面——双层门控,默认关零行为变更）
// ---------------------------------------------------------------------------

/// `--fault-probe` 闭集（探针规格;device-lost 三点 = present 会话面,
/// tdr/budget = render_exec 持久帧面）。
const G31_FAULT_PROBES: [&str; 5] = [
    "device-lost-acquire",
    "device-lost-submit",
    "device-lost-present",
    "tdr",
    "budget",
];

/// 探针命中打印面（单行机读;observed=false 即红,退出码 1）。
fn g31_probe_emit(line: &str, ok: bool) -> ! {
    eprintln!("{GTAG}: G31_FAULT_PROBE {line}");
    if ok {
        std::process::exit(0);
    }
    eprintln!("{GTAG}: FAIL fault-probe 观察面不符（期望确定性错误类,实测见上）");
    std::process::exit(1);
}

/// lane 帧错误探针（tdr/budget 臂）:错误类匹配 → 打印退 0;非本探针面 →
/// 返回交原 fail 路。device-lost 三点不经此面（present 站拦截）。
fn g31_probe_lane_failure(spec: &str, fi: u32, err: &str) {
    let (ok, expect) = match spec {
        "tdr" => (
            err.contains("bounded-wait 超时") && err.contains("TDR-suspected"),
            "fence 有界等待超时面（TDR-suspected 确定性 Err,进程不挂死）",
        ),
        "budget" => (
            err.contains("budget 违约") && err.contains("OOM-suspected"),
            "显存 budget 违约面（OOM-suspected 确定性 Err,fail-closed）",
        ),
        _ => return,
    };
    let esc = err.replace(['\\', '"'], "'");
    g31_probe_emit(
        &format!(
            "{{\"probe\":\"{spec}\",\"site\":\"lane.frame\",\"frame\":{fi},\"observed\":{ok},\"expect\":\"{expect}\",\"error\":\"{esc}\"}}"
        ),
        ok,
    );
}

/// present 站错误探针（device-lost 三点臂）:错误类 + poisoned 级联确定性
/// （锁存后第二次 present 与 resize 均须确定性 `Err` 含 poisoned——禁 UB
/// 级联的实演面）全绿 → 打印退 0。
fn g31_probe_present_failure(
    spec: &str,
    fi: u32,
    err: &str,
    w: &mut vk::ExternalImagePresent,
    px: &[u8],
) {
    let point = match spec.strip_prefix("device-lost-") {
        Some(p) => p,
        None => return,
    };
    let op = match point {
        "acquire" => "vkAcquireNextImageKHR",
        "submit" => "vkQueueSubmit",
        "present" => "vkQueuePresentKHR",
        _ => return,
    };
    let first_ok = err.contains("VK_ERROR_DEVICE_LOST")
        && err.contains(op)
        && err.contains("poisoned");
    // 级联面①:poisoned 锁存后第二次 present → 确定性 Err（非 UB 非 panic）。
    let cascade_present = match w.present_rgba8(px) {
        Err(e) => e.contains("poisoned"),
        Ok(()) => false,
    };
    // 级联面②:poisoned 锁存后 resize → 确定性 Err。
    let (cw, ch) = w.extent();
    let cascade_resize = match w.resize(cw, ch) {
        Err(e) => e.contains("poisoned"),
        Ok(_) => false,
    };
    let ok = first_ok && cascade_present && cascade_resize;
    let esc = err.replace(['\\', '"'], "'");
    g31_probe_emit(
        &format!(
            "{{\"probe\":\"{spec}\",\"site\":\"present\",\"frame\":{fi},\"observed\":{first_ok},\"cascade_present_poisoned\":{cascade_present},\"cascade_resize_poisoned\":{cascade_resize},\"expect\":\"device-lost poisoned 锁存 + 级联确定性（RXS-0077 同律,禁 UB 级联）\",\"error\":\"{esc}\"}}"
        ),
        ok,
    );
}

/// `--auto-move` 确定性脚本轨迹（帧号唯一事实源,f64 参数化;返回
/// (yaw, pitch, eye)——绝对位姿非增量,双跑位级一致性的承载面）。
/// - `orbit`：绕初始眼位水平小圆（r=0.35m）+ 正弦摆头（±0.30 rad）;
/// - `dolly`：沿初始前视 XZ 往复 0.50m + 反向摆头（−0.20 rad 幅）。
/// 双轨迹全参数不同源——异轨迹 digest 序列不同的正向构造（防"确定性的坏
/// 内容",G14.10f 教训面）。
fn g31_auto_move_pose(name: &str, cam0: &G31Camera, fi: u32, total: u32) -> (f32, f32, [f32; 3]) {
    let t = f64::from(fi) / f64::from(total.max(1));
    let tau = std::f64::consts::TAU;
    match name {
        "orbit" => {
            let a = tau * t;
            let eye = [
                (f64::from(cam0.eye[0]) + 0.35 * a.sin()) as f32,
                (f64::from(cam0.eye[1]) + 0.05 * (2.0 * a).sin()) as f32,
                (f64::from(cam0.eye[2]) + 0.35 * (a.cos() - 1.0)) as f32,
            ];
            let yaw = (f64::from(cam0.yaw) + 0.30 * a.sin()) as f32;
            (yaw, cam0.pitch, eye)
        }
        "dolly" => {
            let a = tau * t;
            let f = cam0.forward();
            let fxz = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6);
            let d = 0.50 * (std::f64::consts::PI * t).sin();
            let eye = [
                (f64::from(cam0.eye[0]) + f64::from(f[0] / fxz) * d) as f32,
                (f64::from(cam0.eye[1]) + 0.03 * a.sin()) as f32,
                (f64::from(cam0.eye[2]) + f64::from(f[2] / fxz) * d) as f32,
            ];
            let yaw = (f64::from(cam0.yaw) - 0.20 * a.sin()) as f32;
            (yaw, cam0.pitch, eye)
        }
        other => fail(&format!("--auto-move 轨迹 {other} 越闭集(orbit|dolly)")),
    }
}

/// A3 BGRA8 帧内容 digest（payload = `G31BGRA-1\0` + w/h LE + 打包字节;
/// 与 `frame_content_digest` 同模版本前缀纪律,digest 算法 = rurix_pkg
/// sha256 单一事实源）。A1 host 编码域 digest 语义由本面接替（device
/// BGRA8 域,如实登记不冒充同值）。
fn g31_bgra_digest(w: u32, h: u32, bytes: &[u8]) -> String {
    let mut payload = b"G31BGRA-1\0".to_vec();
    payload.extend_from_slice(&w.to_le_bytes());
    payload.extend_from_slice(&h.to_le_bytes());
    payload.extend_from_slice(bytes);
    format!("sha256:{}", sha256_hex(&payload))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut frames: u32 = 120;
    let mut warmup: u32 = 10;
    let mut tier: u32 = 100;
    let mut contract_path = DEFAULT_CONTRACT.to_owned();
    let mut g10_dir = G31_DEFAULT_G10_DIR.to_owned();
    let mut gltf_path = String::new();
    let mut spv_scene = DEFAULT_SPV_SCENE.to_owned();
    let mut spv_mv = DEFAULT_SPV_MV.to_owned();
    let mut spv_resample = DEFAULT_SPV_RESAMPLE.to_owned();
    let mut spv_resolve = DEFAULT_SPV_RESOLVE.to_owned();
    let mut spv_encode = G31_DEFAULT_SPV_ENCODE.to_owned();
    let mut spv_framegen = G31_DEFAULT_SPV_FRAMEGEN.to_owned();
    let mut spv_mvn = G31_DEFAULT_SPV_MVN.to_owned();
    // G37 W2 合入:--lut off|neutral|warm|<path.cube>（#79 色彩分级臂;默认
    // off = 全 0-byte,值域校验延后 g31_lut_assets::from_arg 闭集）。
    let mut lut = "off".to_owned();
    // G37 W2 合入:--pso-report sidecar 路径（#82/#113 PSO 账本;默认 off =
    // 0-byte 不落盘,账本本体默认开——计数恒登 stderr 单行）。
    let mut pso_report: Option<String> = None;
    let mut fg = G31Fg::Off;
    let mut fg_tol: Option<f64> = None;
    let mut slab_table: Option<String> = None;
    let mut slab_arm = "device".to_owned();
    let mut spv_slab = G31_DEFAULT_SPV_SLAB.to_owned();
    // B1 HZB 接线面（--hzb on;kernel SPV 五件默认路径 = `.tmp` 构建产物,
    // CI 门脚本保障编译;g27 两件本体 0-byte 冻结消费）。
    let mut hzb = G31Hzb::Off;
    let mut spv_hzb_primary = G31_DEFAULT_SPV_HZB_PRIMARY.to_owned();
    let mut spv_hzb_shade = G31_DEFAULT_SPV_HZB_SHADE.to_owned();
    let mut spv_hzb_pack = G31_DEFAULT_SPV_HZB_PACK.to_owned();
    let mut spv_hzb_reduce = G31_DEFAULT_SPV_HZB_REDUCE.to_owned();
    let mut spv_hzb_test = G31_DEFAULT_SPV_HZB_TEST.to_owned();
    // B4 纹理采样接线面（--textures on;生产场景 kernel 纹理变体 + 探针 kernel
    // SPV 两件默认路径 = `.tmp` 构建产物,CI 门脚本保障编译;母版 kernel/
    // 车道 0-byte,off = 回归锚）。
    let mut textures = false;
    let mut spv_texture = G31_DEFAULT_SPV_TEXTURE.to_owned();
    let mut spv_texture_probe = G31_DEFAULT_SPV_TEXTURE_PROBE.to_owned();
    // C13 SVT 派生臂面（--svt on 须随 --textures on;SPV 两件默认路径 = `.tmp`
    // 构建产物,CI 门脚本保障编译;--svt-pool-tiles 0 = 全驻留锚臂,N ≥ 1 =
    // 冷启动小池压力臂）。
    let mut svt_on = false;
    let mut svt_pool_tiles: u32 = 0;
    let mut spv_svt = G31_DEFAULT_SPV_SVT.to_owned();
    let mut spv_svt_probe = G31_DEFAULT_SPV_SVT_PROBE.to_owned();
    let mut dump_last_frame: Option<String> = None;
    // 夜间巡航 D3 视觉验收面（仅验证用，默认 None = 0-byte）：末帧 present 的
    // BGRA8 回读裸 dump（w/h u32 LE 头 + 打包字节），供 bloom/dither 等显示链
    // 加性臂的 on/off 像素级对照（不经 --slab-table 闭集）。
    let mut dump_present_raw: Option<String> = None;
    let mut evidence_path = String::new();
    // C7 profiler 输出面（None = 默认关,全零消费）。
    let mut profile_json: Option<String> = None;
    let mut expect_digest: Option<String> = None;
    let mut hidden = false;
    let mut headless = false;
    // 夜间巡航 D1：8-bit 量化 TPDF 抖动（默认 off = 既有面 0-byte/既有 digest
    // 锚零漂移；on = display_encode kernel params[3]=1，消渐变色带）。
    let mut dither = false;
    // 夜间巡航 D2：平滑顶点法线臂（默认 off = 既有面 0-byte/既有 digest 锚
    // 零漂移；on = scene pass 换 g18_smooth_nrm kernel + trinrm 9 f32/tri 侧表
    // + params[43]=1.0；半球环境光经 RURIX_G18_AMBIENT env 门控同面）。
    let mut smooth_nrm = false;
    // 夜间巡航 D6：GGX 高光材质臂（默认 off = 既有面 0-byte/既有 digest 锚
    // 零漂移；on = tri_mr 2 f32/tri 真表替换第 9 路哑表绑定 + params[48]=1.0；
    // 须随 --smooth-normals on，fail-closed）。
    let mut ggx = false;
    // 画质战役 A1：灯光提取加性臂（默认 off = 既有面 0-byte/既有 digest 锚
    // 零漂移；on = emissive 三角确定性聚类 → ≤K 代表点光 append 进 points
    // 面 + params[49]=contrib；须随 --smooth-normals on，fail-closed）。
    let mut lamp_lights = false;
    let mut lamp_gain: Option<f32> = None;
    let mut lamp_k: Option<usize> = None;
    let mut lamp_contrib: Option<f32> = None;
    // 画质战役 Phase C：GI2 R2 低差异 1 反弹间接光加性臂（默认 off = 既有面
    // 0-byte/既有 digest 锚零漂移；on = 质量 kernel GI2 段 params[51..55)
    // 〔[51] 门 [52] frame_idx 逐帧 [53] firefly clamp [54] scale〕；须随
    // --smooth-normals on 且 --textures on〔GI2 段仅 g31_texture_nrm_gi
    // 合流 kernel 存在〕，fail-closed）。
    let mut gi2 = false;
    let mut gi2_scale: Option<f32> = None;
    let mut gi2_clamp: Option<f32> = None;
    // 画质战役 Phase D：TSR 降噪质量档加性臂（默认 off = 既有面 0-byte/既有
    // digest 锚零漂移；on = resolve pass 换载 g31_tsr_resolve_q.spv〔字节
    // 隔离——off 臂恒载 m_c 冻结字节〕+ tsr_params[19..21)〔[19]=稳态 alpha
    // 档默认 0.02〔母版稳态实测 0.1〕/[20]=邻域 clamp K 默认 0〕：Karis 反
    // 亮度加权混合压 emissive 弹出/萤火虫 + 稳态 alpha 深收敛 + 深度验证
    // 3×3 膨胀区间化〔深度边缘像素不再随 jitter 恒拒史〕；与全部质量臂可组合）。
    let mut tsr_quality = false;
    let mut tsrq_min_alpha: Option<f32> = None;
    let mut tsrq_clamp: Option<f32> = None;
    // 画质战役 Phase F：灯具 emissive 贴图加性臂（默认 off = 既有面 0-byte/
    // 既有 digest 锚零漂移；on = 4 张烘焙 emissive 贴图追加 texel heap 槽
    // 70..73 + triem 逐三角槽号侧表 + scene pass 换载
    // g31_texture_nrm_gi_em.spv〔字节隔离——em off 各臂恒载既有锚定字节〕；
    // 须随 --textures on 且 --smooth-normals on，fail-closed；--emissive-dir
    // = 烘焙件目录〔缺件装配期 fail-closed 拒跑〕）。
    let mut emissive_tex = false;
    let mut emissive_dir = String::from("artifacts/day_0828/f_emissive/baked");
    // day_0829 真实感战役 臂①：金属 F0 修伤加性臂（默认 off = 既有面 0-byte/
    // 既有 digest 锚零漂移;on = scene pass 换载 g31_realism_f0.spv〔字节隔离
    // ——off 各臂恒载 night_0828 三既有锚定字节〕+ tri_base 3 f32/tri 未衰减
    // baseColor 侧表〔尾挂 triem 之后,em off 时 triem 绑 -1 回退真表保持签名
    // 序〕+ params[55]=1.0〔扩 G31_REAL_PARAMS_LEN〕；须随 --ggx on 且
    // --textures on〔F0 消费面 = GGX 高光,tri_base tex/常量双路语义随合流
    // kernel;--ggx 已裁须随 --smooth-normals〕，fail-closed）。
    let mut metal_f0 = false;
    // day_0829 真实感战役 臂②：短程 RT AO 加性臂（默认 off = 既有面 0-byte;
    // on = scene pass 换载 g31_realism_ao.spv + params[56..60)〔[56] 门 [57]
    // 遮蔽半径米 [58] 强度 [59] 样本数〕——余弦半球短程遮蔽射线只乘 al·amb
    // 常量环境光项;须随 --smooth-normals on 且 --textures on〔realism 链
    // 基座——AO 半球基 = 平滑法线〕,fail-closed）。
    let mut rt_ao = false;
    let mut rt_ao_radius: Option<f32> = None;
    let mut rt_ao_strength: Option<f32> = None;
    let mut rt_ao_samples: Option<u32> = None;
    // day_0829 真实感战役 臂⑤：点光软阴影加性臂（默认 off = 既有面 0-byte;
    // on = scene pass 换载 g31_realism_soft.spv + params[60..62)〔[60] 门
    // [61] 样本数〕——点灯阴影射线改逐灯半径 points[pb+6] 圆盘采样,R2+帧
    // 旋转 TSR 时域收敛半影,12 点光多重硬影缓解;TODO #27 SMRT 方向简化形,
    // 光度项仍灯心方向如实登记;须随 --lamp-lights on 且 --textures on,
    // fail-closed）。
    let mut soft_shadows = false;
    let mut soft_shadow_samples: Option<u32> = None;
    // day_0829 真实感战役 臂③：光追镜面/glossy 反射加性臂（默认 off = 既有
    // 面 0-byte;on = scene pass 换载 g31_realism_refl.spv + params[62..65)
    // 〔[62] 门 [63] rough 上限 [64] radiance clamp〕——逐像素 1 条 GGX 重要
    // 性采样反射射线,命中点 GI2 形着色,Fresnel(F0)×w(rough) 加性进 spec;
    // 金属/光滑面映出场景主消费面〔F0 修伤臂①为其语义前置,但机制独立可
    // 单开〕;须随 --ggx on 且 --textures on,fail-closed）。
    let mut rt_reflect = false;
    let mut rt_reflect_rough_max: Option<f32> = None;
    let mut rt_reflect_clamp: Option<f32> = None;
    // day_0829 真实感战役 臂⑥：GI2 贴图反弹加性臂（默认 off = 既有面 0-byte;
    // on = scene pass 换载 g31_realism_gitex.spv + params[67] 门——GI2 反弹
    // 命中点 albedo 从 mats 均值直读升级为贴图采样 + emission 逐像素〔留窗
    // 收口:间接光色彩正确性〕;须随 --gi2 on〔反弹段消费面〕,fail-closed
    // ——gi2 已裁 smooth+textures 前置）。
    let mut gi2_tex = false;
    // day_0829 真实感战役 臂④：法线贴图接线加性臂（默认 off = 既有面 0-byte;
    // on = scene pass 换载 g31_realism_nrm.spv〔签名 +trinm/tri_tan〕+ BC5
    // 法线烘焙容器进 heap 新槽 74..143〔cap-1024 起级,头表全重排布——em
    // append 同律〕+ 切线侧表〔装配期 UV 导数法,glTF 无 TANGENT 烘焙期
    // 生成〕+ params[65..67)〔[65] 门 [66] 强度〕;须随 --textures on 且
    // --smooth-normals on,fail-closed;烘焙件缺件装配期 fail-closed）。
    let mut normal_maps = false;
    let mut normal_strength: Option<f32> = None;
    let mut normal_dir = String::from("artifacts/day_0829_realism/a4_normalmap/baked_normals_bin");
    // G37 W2 臂⑦：玻璃透射加性臂（默认 off = 既有面 0-byte/既有 digest 锚零
    // 漂移;on = scene pass 换载 g31_realism_transp.spv〔签名 +tri_transp,新
    // 最高链位〕+ tri_transp 1 f32/tri 透射率侧表〔glTF alphaMode==BLEND ||
    // baseColor.a<1 判定,bistro 唯一命中 = TransparentGlass.DoubleSided〕+
    // params[68]=1.0;主射线穿透重投 + 点光阴影透明衰减修「玻璃隔断雾状楔形」
    // 缺陷〔HANDOVER §H〕;须随 --textures on 且 --smooth-normals on〔realism
    // 链 kernel 形态〕,fail-closed;nm off 时 trinm/tri_tan 绑回退表/哑表保持
    // 签名序）。
    let mut transparency = false;
    let mut transp_alpha: Option<f32> = None;
    // G37 W2 臂⑧:GI2 反弹 RIS 选灯 / 灯片 CDF 面光 NEE(默认 off 零漂移;
    // on = scene pass 换载 g31_realism_ris.spv〔签名 +lamp_tbl,新最高链位〕
    // + 灯片表/CDF 装配〔g37_w2/g31_ris_lamps.rs〕+ params[69..72) 门)。
    let mut gi2_ris = false;
    let mut gi2_ris_m: Option<usize> = None;
    let mut gi2_nee = false;
    // 夜间巡航 D3：bloom 加性臂（默认 off = 既有面 0-byte/既有 digest 锚零漂移;
    // on = resolve 后插 bright→blurH→blurV→composite 四 pass,display_encode 改读
    // 合成缓冲）。strength/threshold/spv 覆盖件须随 --bloom on（fail-closed）。
    let mut bloom = false;
    let mut bloom_strength: Option<f32> = None;
    let mut bloom_threshold: Option<f32> = None;
    let mut spv_bloom_bright: Option<String> = None;
    let mut spv_bloom_blur: Option<String> = None;
    let mut spv_bloom_composite: Option<String> = None;
    // 画质战役 A2：自动曝光加性臂（默认 off = 既有面 0-byte/既有 digest 锚
    // 零漂移;on = encode 前插 reduce/state 两微 pass,增益经 encode 参数
    // reserved 槽 [133] device 写消费——EMA 跨帧状态 ⇒ on 臂口径 = 双跑
    // 位级一致）。子参数/spv 覆盖件须随 on（fail-closed）。
    let mut autoexp = false;
    let mut autoexp_key: Option<f32> = None;
    let mut autoexp_rate: Option<f32> = None;
    let mut autoexp_min: Option<f32> = None;
    let mut autoexp_max: Option<f32> = None;
    let mut spv_ae_reduce: Option<String> = None;
    let mut spv_ae_state: Option<String> = None;
    // A2 验证面（默认 None/0 = 0-byte）：逐帧 presented 亮度序列 sidecar
    // JSON + 周期 presented raw dump（基路径 = --dump-present-raw 派生）。
    let mut present_luma_out: Option<String> = None;
    let mut dump_present_every: u32 = 0;
    let mut auto_move: Option<String> = None;
    let mut ev100_ramp: Option<(f64, f64)> = None;
    // C4 故障注入/窗口风暴臂（参数面门控默认关;机制面 env 由 CI/调用方设置,
    // rt 层 OnceLock 读取——双层门控,常态逐字节零行为变更）。
    let mut fault_probe: Option<String> = None;
    let mut window_storm: u32 = 0;
    let mut storm_soak: u32 = 0;
    // G31+ #58 簇 DAG LOD（off 默认 = 既有面 0-byte;leaf = 全叶逐位对拍锚;
    // on = 装配期误差 cut 出帧 + 主循环逐帧 host cut 统计——出帧几何冻结于
    // 装配 cut,逐帧 AS 更新归 C/E 阶段,统计 sidecar 如实登记不冒充）。
    let mut cluster_lod_mode = String::from("off");
    let mut cluster_pack = String::new();
    let mut cluster_error_px: f32 = 1.0;
    let mut cluster_stats_out: Option<String> = None;
    // G37 W2 #74/#111 VisBuffer + classify/resolve 生产证据臂（off 默认 =
    // 既有面 0-byte;on = 窗口会话内 device 真跑机制链——真窗口相机样本 ×
    // 真场景簇包,presented 面不变;出帧留窗 = #74 shade 桥 + #75 tile 化）。
    let mut visbuffer_on = false;
    let mut visbuffer_out: Option<String> = None;
    let mut visbuffer_samples: u32 = 3;
    let mut visbuffer_res = String::from("96x54");
    // G37 W3 frame_cut 合入:#77×#89 逐帧 device cut→AS 更新证据臂（off 默认 =
    // 既有面 0-byte;on = 循环后以真窗口逐帧相机重放 refit 竞技场 cut→UPDATE
    // build→RQ digest 链——presented 面不变;出帧翻转归 #77 全量,FIF×每槽 AS
    // 归 #90）。
    let mut frame_cut_on = false;
    let mut frame_cut_out: Option<String> = None;
    let mut frame_cut_every: u32 = 1;
    let mut frame_cut_res = String::from("96x54");
    let mut frame_cut_blocks_limit: usize = 0;
    // G31+ #95/#68/#99 WP cell + HLOD（off 默认 = 既有面 0-byte;full = 全 Full
    // 逐位对拍锚;on = 装配期互斥选层出帧 + 主循环逐帧 tick/选层/warmup 切换
    // 统计——出帧几何冻结于装配选层,统计 sidecar = #99 popping 指标事实源,
    // 如实登记不冒充）。--wp-red-arm = 四 RED 臂子模式（机核能红独立证明）。
    let mut wp_hlod_mode = String::from("off");
    let mut wp_pack = String::new();
    let mut wp_threshold_l0: f64 = 1.0;
    let mut wp_radius: f32 = 64.0;
    let mut wp_warmup: u32 = 4;
    let mut wp_budget_cells: u32 = 4;
    let mut wp_stats_out: Option<String> = None;
    let mut wp_red_arm: Option<String> = None;
    // 画质战役 Phase E1：--quality off|full 预设（full = 解析层展开画质终态
    // 组合,见 parse loop 尾展开块)。
    // G37 W4 默认翻转(DEFAULT_FLIP_PLAN 获批执行,2026-08-30):窗口生产默认
    // off→full(十九臂画质终态 = 交付形态);--quality off 升为显式回退档
    // (中性字面零展开零行为,all-off 锚 55e4a92d 面);bench Stage A 默认臂
    // 永不动(g14_3_pipeline_perf 无本预设);诊断/互斥臂(fg base 点/hzb/
    // slab/svt/storm/fault/cluster-lod/wp-hlod/单臂显式写法)须显式
    // --quality off——CI 调用面已按 §2.5 补扫(w4_flip/QUALITY_OFF_SWEEP.md)。
    let mut quality_full = true;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--frames" => {
                frames = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--frames 非 u32"))
            }
            "--warmup" => {
                warmup = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--warmup 非 u32"))
            }
            "--tier" => {
                tier = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--tier 非 u32"))
            }
            "--contract" => contract_path = take_arg(&args, &mut i),
            "--g10-dir" => g10_dir = take_arg(&args, &mut i),
            "--gltf" => gltf_path = take_arg(&args, &mut i),
            "--spv-scene" => spv_scene = take_arg(&args, &mut i),
            "--spv-mv" => spv_mv = take_arg(&args, &mut i),
            "--spv-resample" => spv_resample = take_arg(&args, &mut i),
            "--spv-resolve" => spv_resolve = take_arg(&args, &mut i),
            "--spv-encode" => spv_encode = take_arg(&args, &mut i),
            "--spv-framegen" => spv_framegen = take_arg(&args, &mut i),
            "--spv-mvn" => spv_mvn = take_arg(&args, &mut i),
            "--fg" => {
                fg = match take_arg(&args, &mut i).as_str() {
                    "off" => G31Fg::Off,
                    "x2" => G31Fg::X2,
                    "x3" => G31Fg::X3,
                    other => fail(&format!("--fg 档 {other} 越闭集(off|x2|x3)")),
                }
            }
            "--fg-tol" => {
                fg_tol = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--fg-tol 非 f64")),
                )
            }
            "--slab-table" => slab_table = Some(take_arg(&args, &mut i)),
            "--slab-arm" => slab_arm = take_arg(&args, &mut i),
            "--spv-slab" => spv_slab = take_arg(&args, &mut i),
            "--hzb" => {
                hzb = match take_arg(&args, &mut i).as_str() {
                    "off" => G31Hzb::Off,
                    "on" => G31Hzb::On,
                    other => fail(&format!("--hzb 档 {other} 越闭集(off|on)")),
                }
            }
            "--spv-hzb-primary" => spv_hzb_primary = take_arg(&args, &mut i),
            "--spv-hzb-shade" => spv_hzb_shade = take_arg(&args, &mut i),
            "--spv-hzb-pack" => spv_hzb_pack = take_arg(&args, &mut i),
            "--spv-hzb-reduce" => spv_hzb_reduce = take_arg(&args, &mut i),
            "--spv-hzb-test" => spv_hzb_test = take_arg(&args, &mut i),
            "--textures" => {
                textures = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--textures 档 {other} 越闭集(off|on)")),
                }
            }
            "--spv-texture" => spv_texture = take_arg(&args, &mut i),
            "--svt" => {
                svt_on = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--svt 档 {other} 越闭集(off|on)")),
                }
            }
            "--svt-pool-tiles" => {
                svt_pool_tiles = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--svt-pool-tiles 非 u32"))
            }
            "--spv-svt" => spv_svt = take_arg(&args, &mut i),
            "--spv-svt-probe" => spv_svt_probe = take_arg(&args, &mut i),
            "--spv-texture-probe" => spv_texture_probe = take_arg(&args, &mut i),
            "--dump-last-frame" => dump_last_frame = Some(take_arg(&args, &mut i)),
            // 夜间巡航 D3 视觉验收面（默认 None = 0-byte）。
            "--dump-present-raw" => dump_present_raw = Some(take_arg(&args, &mut i)),
            "--evidence" => evidence_path = take_arg(&args, &mut i),
            // C7 profiler 输出面（逐 pass GPU/CPU 段 + mean/p50/p99 机器可读 JSON;
            // 默认关 = 零收集零写盘零渲染语义变更;开启仅加 host 侧簿记）。
            "--profile-json" => profile_json = Some(take_arg(&args, &mut i)),
            "--expect-digest" => expect_digest = Some(take_arg(&args, &mut i)),
            "--hidden" => hidden = true,
            "--headless-smoke" => headless = true,
            // 夜间巡航 D1：--dither off|on 闭集（默认 off）。
            "--dither" => {
                dither = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--dither 档 {other} 越闭集(off|on)")),
                }
            }
            // G37 W2 合入:--lut 解析臂（值域校验延后 from_arg——off|neutral|warm
            // 闭集 + .cube 路径自由字面,fail-closed 在校验块）。
            "--lut" => lut = take_arg(&args, &mut i),
            // G37 W2 合入:--pso-report 解析臂（PSO 账本 sidecar 落盘路径）。
            "--pso-report" => pso_report = Some(take_arg(&args, &mut i)),
            // 夜间巡航 D2：--smooth-normals off|on 闭集（默认 off）。
            "--smooth-normals" => {
                smooth_nrm = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--smooth-normals 档 {other} 越闭集(off|on)")),
                }
            }
            // 夜间巡航 D6：--ggx off|on 闭集（默认 off）。
            "--ggx" => {
                ggx = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--ggx 档 {other} 越闭集(off|on)")),
                }
            }
            // A1：--lamp-lights off|on 闭集（默认 off）+ 可调面。
            "--lamp-lights" => {
                lamp_lights = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--lamp-lights 档 {other} 越闭集(off|on)")),
                }
            }
            "--lamp-gain" => {
                lamp_gain = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--lamp-gain 非 f32")),
                )
            }
            "--lamp-k" => {
                lamp_k = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--lamp-k 非 usize")),
                )
            }
            "--lamp-contrib" => {
                lamp_contrib = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--lamp-contrib 非 f32")),
                )
            }
            // Phase C：--gi2 off|on 闭集（默认 off）+ 可调面。
            "--gi2" => {
                gi2 = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--gi2 档 {other} 越闭集(off|on)")),
                }
            }
            "--gi2-scale" => {
                gi2_scale = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--gi2-scale 非 f32")),
                )
            }
            "--gi2-clamp" => {
                gi2_clamp = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--gi2-clamp 非 f32")),
                )
            }
            // Phase D：--tsr-quality off|on 闭集（默认 off）+ 可调面。
            "--tsr-quality" => {
                tsr_quality = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--tsr-quality 档 {other} 越闭集(off|on)")),
                }
            }
            "--tsrq-min-alpha" => {
                tsrq_min_alpha = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--tsrq-min-alpha 非 f32")),
                )
            }
            "--tsrq-clamp" => {
                tsrq_clamp = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--tsrq-clamp 非 f32")),
                )
            }
            // Phase F：--emissive-tex off|on 闭集（默认 off）+ 烘焙件目录。
            "--emissive-tex" => {
                emissive_tex = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--emissive-tex 档 {other} 越闭集(off|on)")),
                }
            }
            // day_0829 臂①：--metal-f0 off|on 闭集（默认 off）。
            "--metal-f0" => {
                metal_f0 = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--metal-f0 档 {other} 越闭集(off|on)")),
                }
            }
            // day_0829 臂②：--rt-ao off|on 闭集（默认 off）+ 可调面。
            "--rt-ao" => {
                rt_ao = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--rt-ao 档 {other} 越闭集(off|on)")),
                }
            }
            "--rt-ao-radius" => {
                rt_ao_radius = Some(
                    take_arg(&args, &mut i)
                        .parse::<f32>()
                        .unwrap_or_else(|_| fail("--rt-ao-radius 非 f32")),
                )
            }
            "--rt-ao-strength" => {
                rt_ao_strength = Some(
                    take_arg(&args, &mut i)
                        .parse::<f32>()
                        .unwrap_or_else(|_| fail("--rt-ao-strength 非 f32")),
                )
            }
            "--rt-ao-samples" => {
                rt_ao_samples = Some(
                    take_arg(&args, &mut i)
                        .parse::<u32>()
                        .unwrap_or_else(|_| fail("--rt-ao-samples 非 u32")),
                )
            }
            // day_0829 臂⑤：--soft-shadows off|on 闭集（默认 off）+ 可调面。
            "--soft-shadows" => {
                soft_shadows = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--soft-shadows 档 {other} 越闭集(off|on)")),
                }
            }
            "--soft-shadow-samples" => {
                soft_shadow_samples = Some(
                    take_arg(&args, &mut i)
                        .parse::<u32>()
                        .unwrap_or_else(|_| fail("--soft-shadow-samples 非 u32")),
                )
            }
            // day_0829 臂③：--rt-reflect off|on 闭集（默认 off）+ 可调面。
            "--rt-reflect" => {
                rt_reflect = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--rt-reflect 档 {other} 越闭集(off|on)")),
                }
            }
            "--rt-reflect-rough-max" => {
                rt_reflect_rough_max = Some(
                    take_arg(&args, &mut i)
                        .parse::<f32>()
                        .unwrap_or_else(|_| fail("--rt-reflect-rough-max 非 f32")),
                )
            }
            "--rt-reflect-clamp" => {
                rt_reflect_clamp = Some(
                    take_arg(&args, &mut i)
                        .parse::<f32>()
                        .unwrap_or_else(|_| fail("--rt-reflect-clamp 非 f32")),
                )
            }
            // day_0829 臂⑥：--gi2-tex off|on 闭集（默认 off）。
            "--gi2-tex" => {
                gi2_tex = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--gi2-tex 档 {other} 越闭集(off|on)")),
                }
            }
            // day_0829 臂④：--normal-maps off|on 闭集（默认 off）+ 可调面。
            "--normal-maps" => {
                normal_maps = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--normal-maps 档 {other} 越闭集(off|on)")),
                }
            }
            // G37 W2 臂⑦：--transparency off|on 闭集（默认 off）+ 可调面。
            "--transparency" => {
                transparency = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--transparency 档 {other} 越闭集(off|on)")),
                }
            }
            "--transp-alpha" => {
                transp_alpha = Some(
                    take_arg(&args, &mut i)
                        .parse::<f32>()
                        .unwrap_or_else(|_| fail("--transp-alpha 非 f32")),
                )
            }
            // G37 W2 臂⑧:--gi2-ris/--gi2-ris-m/--gi2-nee 闭集(默认 off)。
            "--gi2-ris" => {
                gi2_ris = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--gi2-ris 档 {other} 越闭集(off|on)")),
                };
            }
            "--gi2-ris-m" => {
                gi2_ris_m = Some(
                    take_arg(&args, &mut i)
                        .parse::<usize>()
                        .unwrap_or_else(|_| fail("--gi2-ris-m 非 usize")),
                );
            }
            "--gi2-nee" => {
                gi2_nee = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--gi2-nee 档 {other} 越闭集(off|on)")),
                };
            }
            "--normal-strength" => {
                normal_strength = Some(
                    take_arg(&args, &mut i)
                        .parse::<f32>()
                        .unwrap_or_else(|_| fail("--normal-strength 非 f32")),
                )
            }
            "--normal-dir" => normal_dir = take_arg(&args, &mut i),
            "--emissive-dir" => emissive_dir = take_arg(&args, &mut i),
            // 夜间巡航 D3：--bloom off|on 闭集（默认 off）+ 可调面。
            "--bloom" => {
                bloom = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--bloom 档 {other} 越闭集(off|on)")),
                }
            }
            "--bloom-strength" => {
                bloom_strength = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--bloom-strength 非 f32")),
                )
            }
            "--bloom-threshold" => {
                bloom_threshold = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--bloom-threshold 非 f32")),
                )
            }
            "--spv-bloom-bright" => spv_bloom_bright = Some(take_arg(&args, &mut i)),
            "--spv-bloom-blur" => spv_bloom_blur = Some(take_arg(&args, &mut i)),
            "--spv-bloom-composite" => spv_bloom_composite = Some(take_arg(&args, &mut i)),
            // 画质战役 A2：--auto-exposure off|on 闭集（默认 off）+ 可调面。
            "--auto-exposure" => {
                autoexp = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--auto-exposure 档 {other} 越闭集(off|on)")),
                }
            }
            "--autoexp-key" => {
                autoexp_key = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--autoexp-key 非 f32")),
                )
            }
            "--autoexp-rate" => {
                autoexp_rate = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--autoexp-rate 非 f32")),
                )
            }
            "--autoexp-min" => {
                autoexp_min = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--autoexp-min 非 f32")),
                )
            }
            "--autoexp-max" => {
                autoexp_max = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--autoexp-max 非 f32")),
                )
            }
            "--spv-autoexp-reduce" => spv_ae_reduce = Some(take_arg(&args, &mut i)),
            "--spv-autoexp-state" => spv_ae_state = Some(take_arg(&args, &mut i)),
            // A2 验证面（默认关 = 0-byte）。
            "--present-luma-out" => present_luma_out = Some(take_arg(&args, &mut i)),
            "--dump-present-every" => {
                dump_present_every = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--dump-present-every 非 u32"))
            }
            "--auto-move" => auto_move = Some(take_arg(&args, &mut i)),
            "--ev100-ramp" => {
                let a: f64 = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--ev100-ramp a 非 f64"));
                let b: f64 = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--ev100-ramp b 非 f64"));
                ev100_ramp = Some((a, b));
            }
            "--fault-probe" => fault_probe = Some(take_arg(&args, &mut i)),
            "--window-storm" => {
                window_storm = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--window-storm 非 u32"))
            }
            "--storm-soak" => {
                storm_soak = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--storm-soak 非 u32"))
            }
            // G31+ #58：簇 DAG LOD 四参数（模式/簇包/阈值/逐帧统计 sidecar）。
            "--cluster-lod" => cluster_lod_mode = take_arg(&args, &mut i),
            "--cluster-pack" => cluster_pack = take_arg(&args, &mut i),
            "--cluster-error-px" => {
                cluster_error_px = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--cluster-error-px 非 f32"))
            }
            "--cluster-stats-out" => cluster_stats_out = Some(take_arg(&args, &mut i)),
            // G37 W2 #74/#111:visbuffer 证据臂四参数（模式/sidecar/采样帧数/画布）。
            "--visbuffer" => {
                visbuffer_on = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--visbuffer {other}：只接受 off|on")),
                }
            }
            "--visbuffer-out" => visbuffer_out = Some(take_arg(&args, &mut i)),
            "--visbuffer-samples" => {
                visbuffer_samples = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--visbuffer-samples 非 u32"))
            }
            "--visbuffer-res" => visbuffer_res = take_arg(&args, &mut i),
            // G37 W3 frame_cut 合入:逐帧 cut 判档臂五参数（模式/sidecar/节拍/
            // 画布/子集阀）。
            "--cluster-per-frame-cut" => {
                frame_cut_on = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--cluster-per-frame-cut {other}：只接受 off|on")),
                }
            }
            "--frame-cut-out" => frame_cut_out = Some(take_arg(&args, &mut i)),
            "--frame-cut-every" => {
                frame_cut_every = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--frame-cut-every 非 u32"))
            }
            "--frame-cut-res" => frame_cut_res = take_arg(&args, &mut i),
            "--frame-cut-blocks-limit" => {
                frame_cut_blocks_limit = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--frame-cut-blocks-limit 非 usize"))
            }
            // G31+ #95/#68/#99：WP cell + HLOD 参数（模式/cell 包/L0 阈值/
            // 距离环/预热帧/流送预算/逐帧统计 sidecar/RED 臂子模式）。
            "--wp-hlod" => wp_hlod_mode = take_arg(&args, &mut i),
            "--wp-pack" => wp_pack = take_arg(&args, &mut i),
            "--wp-threshold-l0" => {
                wp_threshold_l0 = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--wp-threshold-l0 非 f64"))
            }
            "--wp-radius" => {
                wp_radius = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--wp-radius 非 f32"))
            }
            "--wp-warmup" => {
                wp_warmup = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--wp-warmup 非 u32"))
            }
            "--wp-budget-cells" => {
                wp_budget_cells = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--wp-budget-cells 非 u32"))
            }
            "--wp-stats-out" => wp_stats_out = Some(take_arg(&args, &mut i)),
            "--wp-red-arm" => wp_red_arm = Some(take_arg(&args, &mut i)),
            // 画质战役 Phase E1：--quality off|full 预设闭集（默认 off）。
            "--quality" => {
                quality_full = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "full" => true,
                    other => fail(&format!("--quality 档 {other} 越闭集(off|full)")),
                }
            }
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    // 画质战役 Phase E1 --quality full 预设展开（解析层——先于全部臂校验/
    // SPV 换载,展开后下游与显式九臂写法走完全相同路径 ⇒ 位级等价,锚
    // 6bd3af63）。展开面旗标显式重叠 = fail-closed（「--quality full
    // --lamp-gain 2」类双重指定无裁决面,拒跑不猜;微调请弃预设走全显式
    // 写法）。RURIX_G18_AMBIENT env 缺席时注入战役终态档 0.004（lane_body
    // OnceLock 预设槽,env 在位一律优先——forbid(unsafe_code) + edition 2024
    // 下 env::set_var 为 unsafe 不可用;与 env 路径同字面同 parse ⇒ f32
    // 位级同值）。
    if quality_full {
        // day_0828 Phase F：预设展开 +--emissive-tex（F5 并入——full = 十臂;
        // 旧九臂 full 锚 9e5f6300 作废,新锚 F5_ANCHOR.json 登记）。
        // day_0829 真实感战役：预设展开 + 六 realism 臂（全臂单独验收达标
        // 并入——十臂 → 十六臂;--soft-shadow-samples 预设 1 = F1 组合定档
        // 〔2 样本 12.96ms 超 90fps 预算,1 样本 9.54ms 过线,TSR 帧旋转时域
        // 收敛半影仍成立〕;其余臂子参数走各自默认。十臂 γ2.5 锚 de342586
        // 作废,新锚 F2_ANCHOR 登记,作废谱系进 HANDOVER）。
        // G37 W2：预设展开 +--transparency（十六臂 → 十七臂;full 语义变更
        // 重锚归 W4;--transp-alpha 不进 dup 表 = 可与 full 组合微调,rt-ao
        // 子参数同律）。
        const QUALITY_FULL_EXPANSION: [&str; 22] = [
            "--smooth-normals",
            "--ggx",
            "--lamp-lights",
            "--lamp-gain",
            "--textures",
            "--bloom",
            "--dither",
            "--auto-exposure",
            "--tsr-quality",
            "--gi2",
            "--gi2-clamp",
            "--emissive-tex",
            "--metal-f0",
            "--rt-ao",
            "--soft-shadows",
            "--soft-shadow-samples",
            "--rt-reflect",
            "--gi2-tex",
            "--normal-maps",
            "--transparency",
            // G37 W2 ris_nee:两臂进 dup 表(--gi2-ris-m 不进 = 可与 full
            // 组合微调,rt-ao 子参数同律)。
            "--gi2-ris",
            "--gi2-nee",
        ];
        let dup: Vec<&str> = QUALITY_FULL_EXPANSION
            .into_iter()
            .filter(|f| args.iter().any(|a| a == f))
            .collect();
        if !dup.is_empty() {
            fail(&format!(
                "--quality full 与显式旗标 {} 冲突（预设 = 解析层一键展开,重叠指定即语义歧义;微调请弃预设走全显式写法,fail-closed）",
                dup.join(" ")
            ));
        }
        smooth_nrm = true;
        ggx = true;
        lamp_lights = true;
        lamp_gain = Some(4.0);
        textures = true;
        bloom = true;
        dither = true;
        autoexp = true;
        tsr_quality = true;
        gi2 = true;
        gi2_clamp = Some(0.01);
        emissive_tex = true;
        metal_f0 = true;
        rt_ao = true;
        soft_shadows = true;
        soft_shadow_samples = Some(1);
        rt_reflect = true;
        gi2_tex = true;
        normal_maps = true;
        // G37 W2:transparency 并入 full(玻璃透射真解,transp-alpha 走默认)。
        transparency = true;
        // G37 W2 ris_nee:两臂并入 full(EVAL_RESTIR §9.3 修复路径;十七臂
        // → 十九臂,full 语义变更重锚归 W4;--gi2-ris-m 走默认 6)。
        gi2_ris = true;
        gi2_nee = true;
        let _ = G18_AMBIENT_PRESET.set(
            "0.004"
                .parse::<f32>()
                .unwrap_or_else(|_| fail("--quality full 预设环境光字面解析失败（不可达）")),
        );
    }
    if frames == 0 {
        fail("--frames 必须 ≥1");
    }
    if let Some(name) = auto_move.as_deref() {
        if !matches!(name, "orbit" | "dolly") {
            fail(&format!("--auto-move 轨迹 {name} 越闭集(orbit|dolly)"));
        }
    }
    // 夜间巡航 D3 --bloom 闭集校验（fail-closed，不静默降级）：组合面未接线的
    // 既有臂一律互斥；strength/threshold 有限非负；可调面/spv 覆盖件须随 on。
    let bloom_strength_v = bloom_strength.unwrap_or(G31_BLOOM_DEFAULT_STRENGTH);
    let bloom_threshold_v = bloom_threshold.unwrap_or(G31_BLOOM_DEFAULT_THRESHOLD);
    if bloom {
        // day_0828 Phase B：互斥集移除 textures（组合面 = g31_lane_descs_tex_bloom
        // / _tex_nrm_bloom 已接线）;svt 仍互斥（heap 形态 fail-closed,见下）。
        // G37 W3 fg_combo 合入：fg 项加 !quality_full 豁免（comp parity 适配
        // §3.1 已接线,见 g31_apply_fg_full）。
        if (fg != G31Fg::Off && !quality_full) || hzb == G31Hzb::On || svt_on || slab_table.is_some() {
            fail("--bloom on 不与 --fg/--hzb on/--svt on/--slab-table 同跑（组合面未接线,fail-closed;fg×画质组合面经 --quality full 预设字面开放——两点式闭集,散臂微调组合仍拒,G37 W3 fg_combo 判档）");
        }
        if cluster_lod_mode != "off" || wp_hlod_mode != "off" {
            fail("--bloom on 不与 --cluster-lod/--wp-hlod 同跑（组合面未接线,fail-closed）");
        }
        if !(bloom_strength_v.is_finite() && bloom_strength_v >= 0.0) {
            fail("--bloom-strength 必须为有限非负 f32");
        }
        if !(bloom_threshold_v.is_finite() && bloom_threshold_v >= 0.0) {
            fail("--bloom-threshold 必须为有限非负 f32");
        }
    } else if bloom_strength.is_some()
        || bloom_threshold.is_some()
        || spv_bloom_bright.is_some()
        || spv_bloom_blur.is_some()
        || spv_bloom_composite.is_some()
    {
        fail("--bloom-strength/--bloom-threshold/--spv-bloom-* 须随 --bloom on（off 面零消费,fail-closed）");
    }
    // 画质战役 A2 --auto-exposure 闭集校验（fail-closed，不静默降级）：组合面
    // 未接线的既有臂一律互斥（与 --bloom 互斥集同律）;与 --dither/
    // --smooth-normals/--ggx/--lamp-lights/--bloom 可组合;参数域校验;
    // 子参数/spv 覆盖件须随 on。
    let ae_key_v = autoexp_key.unwrap_or(G31_AE_DEFAULT_KEY);
    let ae_rate_v = autoexp_rate.unwrap_or(G31_AE_DEFAULT_RATE);
    let ae_min_v = autoexp_min.unwrap_or(G31_AE_DEFAULT_MIN);
    let ae_max_v = autoexp_max.unwrap_or(G31_AE_DEFAULT_MAX);
    if autoexp {
        // day_0828 Phase B：互斥集移除 textures（AE 尾挂下标族 tex 四形态已接线）。
        // G37 W3 fg_combo 合入：fg 项加 !quality_full 豁免（AE 零适配正交——
        // enc_fg 绑同一 ENC_PARAMS,生成帧自动继承本帧增益,§1.4）。
        if (fg != G31Fg::Off && !quality_full) || hzb == G31Hzb::On || svt_on || slab_table.is_some() {
            fail("--auto-exposure on 不与 --fg/--hzb on/--svt on/--slab-table 同跑（组合面未接线,fail-closed;fg×画质组合面经 --quality full 预设字面开放——两点式闭集,散臂微调组合仍拒,G37 W3 fg_combo 判档）");
        }
        if cluster_lod_mode != "off" || wp_hlod_mode != "off" {
            fail("--auto-exposure on 不与 --cluster-lod/--wp-hlod 同跑（组合面未接线,fail-closed）");
        }
        if !(ae_key_v.is_finite() && ae_key_v > 0.0) {
            fail("--autoexp-key 必须为正有限 f32");
        }
        if !(ae_rate_v.is_finite() && ae_rate_v > 0.0 && ae_rate_v <= 1.0) {
            fail("--autoexp-rate 必须 ∈ (0,1] 有限 f32");
        }
        if !(ae_min_v.is_finite() && ae_min_v > 0.0) {
            fail("--autoexp-min 必须为正有限 f32");
        }
        if !(ae_max_v.is_finite() && ae_max_v >= ae_min_v) {
            fail("--autoexp-max 必须有限且 ≥ --autoexp-min");
        }
    } else if autoexp_key.is_some()
        || autoexp_rate.is_some()
        || autoexp_min.is_some()
        || autoexp_max.is_some()
        || spv_ae_reduce.is_some()
        || spv_ae_state.is_some()
    {
        fail("--autoexp-*/--spv-autoexp-* 须随 --auto-exposure on（off 面零消费,fail-closed）");
    }
    if dump_present_every > 0 && dump_present_raw.is_none() {
        fail("--dump-present-every 须随 --dump-present-raw（派生 dump 基路径,fail-closed）");
    }
    // 夜间巡航 D2 --smooth-normals 闭集校验（fail-closed，不静默降级）：
    // on = scene pass 换 g18_smooth_nrm（trinrm 第 6 路绑定 + params[43]=1.0）
    // ——与改 scene pass/三角汤序的臂互斥；与 --bloom/--dither 可组合
    //（scene 上游/post 下游正交,组合面 = g31_lane_descs_nrm_bloom）。
    if smooth_nrm {
        // day_0828 Phase B：互斥集移除 textures——(smooth && textures) 合流臂
        // 换载 g31_texture_nrm_gi 合体 kernel（tex SPV 面,见下 --textures 段;
        // scene SPV 换载对合流臂为兜底面——descs 首 pass 被 tex 变体替换）。
        // G37 W3 fg_combo 合入：fg 项加 !quality_full 豁免（scene pass 仅改
        // out_color 内容,FG 输入形态正交,§1.4）。
        if (fg != G31Fg::Off && !quality_full) || hzb == G31Hzb::On || svt_on || slab_table.is_some() {
            fail("--smooth-normals on 不与 --fg/--hzb on/--svt on/--slab-table 同跑（组合面未接线,fail-closed;fg×画质组合面经 --quality full 预设字面开放——两点式闭集,散臂微调组合仍拒,G37 W3 fg_combo 判档）");
        }
        if cluster_lod_mode != "off" || wp_hlod_mode != "off" {
            fail("--smooth-normals on 不与 --cluster-lod/--wp-hlod 同跑（几何重建后法线侧表 gather 未接线——D2 登记留窗,fail-closed）");
        }
        // scene SPV 换载（默认字面才换——用户显式 --spv-scene 面尊重不覆盖,
        // 须为 8 路绑定面 kernel）。
        if spv_scene == DEFAULT_SPV_SCENE {
            spv_scene = DEFAULT_SPV_G18_SMOOTH_NRM.to_owned();
        }
    }
    // 夜间巡航 D6 --ggx 闭集校验（fail-closed，不静默降级）：须随
    // --smooth-normals on（GGX 高光依赖平滑法线才正确——flat 面法线下高光
    // 逐三角不连续无意义；且 tri_mr 真表绑定面仅 nrm/nrm_bloom 变体存在）。
    // 互斥集与 --smooth-normals 同（fg/hzb/textures/svt/slab/cluster/wp 上行
    // 已裁——ggx ⇒ smooth_nrm on 故全覆盖）;与 --bloom/--dither 可组合
    //（scene 上游/post 下游正交,组合面 = g31_lane_descs_nrm_bloom 真表支路）。
    if ggx && !smooth_nrm {
        fail("--ggx on 须随 --smooth-normals on（GGX 依赖平滑法线;tri_mr 真表绑定面仅 nrm 变体存在,fail-closed）");
    }
    // 画质战役 A1 --lamp-lights 闭集校验（fail-closed，不静默降级）：须随
    // --smooth-normals on（半径阴影截断/贡献剔除消费面仅 g18_smooth_nrm
    // kernel 存在——母版 kernel 不读 points 槽 7 与 params[49]，开臂无语义;
    // 互斥集随 --smooth-normals 上行已裁）;子参数须随 on（off 面零消费）;
    // 参数域校验（gain 正有限 / k ≥ 1 / contrib 非负有限）。
    if lamp_lights && !smooth_nrm {
        fail("--lamp-lights on 须随 --smooth-normals on（半径阴影截断/贡献剔除消费面仅 g18_smooth_nrm kernel 存在,fail-closed）");
    }
    if !lamp_lights && (lamp_gain.is_some() || lamp_k.is_some() || lamp_contrib.is_some()) {
        fail("--lamp-gain/--lamp-k/--lamp-contrib 须随 --lamp-lights on（off 面零消费,fail-closed）");
    }
    let lamp_gain_v = lamp_gain.unwrap_or(1.0);
    let lamp_k_v = lamp_k.unwrap_or(12);
    let lamp_contrib_v = lamp_contrib.unwrap_or(0.0);
    if lamp_lights {
        if !(lamp_gain_v.is_finite() && lamp_gain_v > 0.0) {
            fail("--lamp-gain 必须为正有限 f32");
        }
        if lamp_k_v == 0 {
            fail("--lamp-k 必须 ≥1");
        }
        if !(lamp_contrib_v.is_finite() && lamp_contrib_v >= 0.0) {
            fail("--lamp-contrib 必须为非负有限 f32");
        }
    }
    // G31+ #58 --cluster-lod 闭集校验（fail-closed，不静默降级）：模式闭集 +
    // 簇包必填 + 与 --hzb/--textures/--slab-table 互斥（cut 重排三角汤 ⇒
    // 节点段基址/UV 同序/slab 预调制序假设破坏,组合面归后续波）。
    let cluster_opt = match cluster_lod_mode.as_str() {
        "off" => ClusterLodOpt::off(),
        m @ ("leaf" | "on") => {
            if cluster_pack.is_empty() {
                fail("--cluster-lod leaf|on 要求 --cluster-pack <RXCP>（g31_cluster_lod_bake 产物）");
            }
            if hzb == G31Hzb::On || textures || slab_table.is_some() {
                fail("--cluster-lod 不与 --hzb on/--textures on/--slab-table 同跑（cut 重排三角汤,节点段/UV/slab 序假设破坏;组合面归后续波,fail-closed）");
            }
            if !(cluster_error_px.is_finite() && cluster_error_px > 0.0) {
                fail("--cluster-error-px 必须为正有限 f32");
            }
            ClusterLodOpt {
                mode: if m == "leaf" {
                    ClusterLodMode::Leaf
                } else {
                    ClusterLodMode::On
                },
                pack_path: cluster_pack.clone(),
                threshold_px: cluster_error_px,
                // 窗口臂驻留压力面归 bench 臂（E 判据在 g14_3 收口）。
                resident_pages: 0,
            }
        }
        other => fail(&format!("--cluster-lod {other}：只接受 off|leaf|on")),
    };
    if cluster_stats_out.is_some() && cluster_opt.mode == ClusterLodMode::Off {
        fail("--cluster-stats-out 须随 --cluster-lod leaf|on（统计面无 cut 无意义）");
    }
    // G37 W2 #74/#111 --visbuffer 闭集校验（fail-closed）：须随 --cluster-lod
    // leaf|on（机制链消费 cut 与 RXCP 簇 DAG）;互斥集随 --cluster-lod 继承
    //（hzb/textures/slab-table/wp-hlod/九臂组合已在其面裁掉,零新增互斥）。
    let visbuffer_opt = if visbuffer_on {
        if cluster_opt.mode == ClusterLodMode::Off {
            fail("--visbuffer on 须随 --cluster-lod leaf|on（机制链消费 cut 与簇 DAG）");
        }
        if visbuffer_samples == 0 {
            fail("--visbuffer-samples 必须 ≥1");
        }
        let (rw, rh) = {
            let mut it = visbuffer_res.split('x');
            let w: u32 = it
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| fail("--visbuffer-res 形如 96x54"));
            let h: u32 = it
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| fail("--visbuffer-res 形如 96x54"));
            if it.next().is_some() || w == 0 || h == 0 {
                fail("--visbuffer-res 形如 96x54（两正整数）");
            }
            (w, h)
        };
        VisBufferArmOpt {
            enabled: true,
            res_w: rw,
            res_h: rh,
            samples: visbuffer_samples,
            out_path: visbuffer_out.clone().unwrap_or_default(),
        }
    } else {
        if visbuffer_out.is_some() {
            fail("--visbuffer-out 须随 --visbuffer on");
        }
        VisBufferArmOpt::off()
    };
    // G37 W3 frame_cut 合入:--cluster-per-frame-cut 闭集校验（fail-closed）：
    // 须随 --cluster-lod leaf|on（消费 cut 与 RXCP 簇 DAG）;互斥集随
    // --cluster-lod 继承（hzb/textures/slab/wp-hlod/九臂组合已在其面裁掉,
    // 零新增互斥）。
    let frame_cut_opt = if frame_cut_on {
        if cluster_opt.mode == ClusterLodMode::Off {
            fail("--cluster-per-frame-cut on 须随 --cluster-lod leaf|on（消费 cut 与簇 DAG）");
        }
        if frame_cut_every == 0 {
            fail("--frame-cut-every 必须 ≥1（1 = 逐帧;>1 = 惰性节拍臂）");
        }
        let (rw, rh) = {
            let mut it = frame_cut_res.split('x');
            let w: u32 = it
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| fail("--frame-cut-res 形如 96x54"));
            let h: u32 = it
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| fail("--frame-cut-res 形如 96x54"));
            if it.next().is_some() || w == 0 || h == 0 {
                fail("--frame-cut-res 形如 96x54（两正整数）");
            }
            (w, h)
        };
        FrameCutArmOpt {
            enabled: true,
            res_w: rw,
            res_h: rh,
            frames: 0,  // 真轨迹帧数 = 实采样本数（登记字段,臂消费 samples.len()）
            step_m: 0.0, // 真轨迹非合成 dolly——0 如实登记
            cut_every: frame_cut_every,
            blocks_limit: frame_cut_blocks_limit,
            // 真窗口轨迹可折返（auto-move dolly 往返实测:window_stats.json
            // 帧 13 折返）⇒ 宽门 = 非常量 + 方向 measured 登记,不误红。
            monotone_gate: false,
            out_path: frame_cut_out.clone().unwrap_or_default(),
        }
    } else {
        if frame_cut_out.is_some() {
            fail("--frame-cut-out 须随 --cluster-per-frame-cut on");
        }
        FrameCutArmOpt::off()
    };
    // G31+ #95/#68 --wp-hlod 闭集校验（fail-closed，不静默降级）：模式闭集 +
    // cell 包必填 + 与 --cluster-lod/--hzb/--textures/--slab-table 互斥
    //（两套几何重组各自重排三角汤/节点段序假设破坏,组合面归后续波）。
    let wp_opt = match wp_hlod_mode.as_str() {
        "off" => WpHlodOpt::off(),
        m @ ("full" | "on") => {
            if wp_pack.is_empty() {
                fail("--wp-hlod full|on 要求 --wp-pack <RXWH>（g31_wp_hlod_bake 产物）");
            }
            if cluster_opt.mode != ClusterLodMode::Off {
                fail("--wp-hlod 不与 --cluster-lod 同跑（两套几何重组各自重排三角汤;组合面归后续波,fail-closed）");
            }
            if hzb == G31Hzb::On || textures || slab_table.is_some() {
                fail("--wp-hlod 不与 --hzb on/--textures on/--slab-table 同跑（cell 重组三角汤,节点段/UV/slab 序假设破坏;组合面归后续波,fail-closed）");
            }
            if !(wp_threshold_l0.is_finite() && wp_threshold_l0 > 0.0) {
                fail("--wp-threshold-l0 必须为正有限 f64");
            }
            if !(wp_radius.is_finite() && wp_radius > 0.0) {
                fail("--wp-radius 必须为正有限 f32");
            }
            if wp_warmup == 0 {
                fail("--wp-warmup 必须 ≥1（预热协议:切换请求 → 原子翻转间隔）");
            }
            WpHlodOpt {
                mode: if m == "full" { WpHlodMode::Full } else { WpHlodMode::On },
                pack_path: wp_pack.clone(),
                threshold_l0: wp_threshold_l0,
                loading_radius_m: wp_radius,
                inner_radius_m: (wp_radius * 0.25).max(1.0),
                budget_cells: wp_budget_cells.max(1),
                warmup_frames: wp_warmup,
            }
        }
        other => fail(&format!("--wp-hlod {other}：只接受 off|full|on")),
    };
    if wp_stats_out.is_some() && wp_opt.mode == WpHlodMode::Off {
        fail("--wp-stats-out 须随 --wp-hlod full|on（统计面无选层无意义）");
    }
    // G31+ #68/#95 四 RED 臂子模式（host 机核能红独立证明;无 GPU 依赖,检出
    // 即 exit 0——门脚本子进程消费面）：
    // ① tamper-digest = cell 包字节篡改 → 读取期 digest 自核验必拒;
    // ② event-order = 乱序 cell 事件（Resident 无 LoadBegin）→ 状态机必拒;
    // ③ double-draw = 同帧同 cell 重复选层 → 互斥机核必拒;
    // ④ runtime-merge = 运行时合并请求 → RXS-0364 L3 零合并锚恒拒。
    if let Some(arm) = wp_red_arm.as_deref() {
        if wp_opt.mode == WpHlodMode::Off {
            fail("--wp-red-arm 须随 --wp-hlod full|on（RED 臂 = 机核注入检测面）");
        }
        use rurix_render::world::hlod::{HlodRuntime, ScreenSizeThresholds};
        use rurix_render::world::partition::{CellEvent, CellEventKind};
        let detected = match arm {
            "tamper-digest" => {
                let mut bytes = std::fs::read(&wp_pack)
                    .unwrap_or_else(|e| fail(&format!("RED 臂读包: {e}")));
                let last = bytes.len() - 1;
                bytes[last] ^= 1;
                let tmp = std::env::temp_dir()
                    .join(format!("g31_wp_red_tamper_{}.rxwh", std::process::id()));
                std::fs::write(&tmp, &bytes)
                    .unwrap_or_else(|e| fail(&format!("RED 臂写临时包: {e}")));
                let r = read_wp_hlod_pack(&tmp);
                let _ = std::fs::remove_file(&tmp);
                match r {
                    Err(e) => {
                        eprintln!("{GTAG}: RED 臂 tamper-digest 检出: {e}");
                        true
                    }
                    Ok(_) => false,
                }
            }
            "event-order" => {
                let mut rt = HlodRuntime::new();
                match rt.apply_cell_events(&[CellEvent {
                    frame: 0,
                    cell: 0,
                    kind: CellEventKind::CellResident,
                }]) {
                    Err(e) => {
                        eprintln!("{GTAG}: RED 臂 event-order 检出: {e}");
                        true
                    }
                    Ok(()) => false,
                }
            }
            "double-draw" => {
                // 正常装载 cell 0 → 同帧重复选层（双绘注入）→ 互斥机核必拒。
                let pack = read_wp_hlod_pack(Path::new(&wp_pack))
                    .unwrap_or_else(|e| fail(&format!("RED 臂读包: {e}")));
                let world = wp_build_world(&pack);
                let Some(ci) = pack.cells.iter().position(|c| c.is_some()) else {
                    fail("RED 臂:包内无非空 cell");
                };
                let mut rt = HlodRuntime::new();
                rt.apply_cell_events(&[
                    CellEvent { frame: 0, cell: ci as u32, kind: CellEventKind::CellLoadBegin },
                    CellEvent { frame: 0, cell: ci as u32, kind: CellEventKind::CellResident },
                ])
                .unwrap_or_else(|e| fail(&format!("RED 臂装载: {e}")));
                let th = ScreenSizeThresholds::new(
                    (0..pack.levels).map(|i| 1.0 / 16f64.powi(i as i32)).collect(),
                )
                .unwrap_or_else(|e| fail(&format!("RED 臂阈值表: {e}")));
                rt.select(&world, ci as u32, 10.0, &th, 1)
                    .unwrap_or_else(|e| fail(&format!("RED 臂首选层: {e}")));
                rt.select(&world, ci as u32, 10.0, &th, 1)
                    .unwrap_or_else(|e| fail(&format!("RED 臂重复选层: {e}")));
                match rt.assert_mutually_exclusive() {
                    Err(e) => {
                        eprintln!("{GTAG}: RED 臂 double-draw 检出: {e}");
                        true
                    }
                    Ok(()) => false,
                }
            }
            "runtime-merge" => {
                let rt = HlodRuntime::new();
                match rt.request_runtime_merge("wp_red_arm_merge", &[0, 1]) {
                    Err(e) => {
                        eprintln!("{GTAG}: RED 臂 runtime-merge 检出（零合并锚）: {e}");
                        true
                    }
                    Ok(()) => false,
                }
            }
            other => fail(&format!(
                "--wp-red-arm {other}：只接受 tamper-digest|event-order|double-draw|runtime-merge"
            )),
        };
        if detected {
            println!("{GTAG}: WP_RED_ARM_DETECTED arm={arm}");
            std::process::exit(0);
        }
        eprintln!("{GTAG}: FAIL RED 臂 {arm} 未检出（机核失效）");
        std::process::exit(1);
    }
    if ev100_ramp.is_some() && auto_move.is_none() {
        fail("--ev100-ramp 须随 --auto-move(交互面用 -/= 键)");
    }
    // B3 slab 闭集约束（fail-fast 如实拒跑,不静默降级）。
    let spv_slab_explicit = args.iter().any(|a| a == "--spv-slab");
    let slab_arm_explicit = args.iter().any(|a| a == "--slab-arm");
    if let Some(st) = slab_table.as_deref() {
        if auto_move.is_none() {
            fail("--slab-table 须随 --auto-move（B3 登记面 = 确定性轨迹 digest_seq;静态无轨迹面非本任务口径）");
        }
        if fg != G31Fg::Off {
            fail("--slab-table 与 --fg 互斥（B3 接线面 = 生产五 pass 现状车道;FG 组合面非本任务口径,如实拒跑不冒充）");
        }
        if !matches!(slab_arm.as_str(), "device" | "host") {
            fail(&format!("--slab-arm {slab_arm} 越闭集(device|host)"));
        }
        if !std::path::Path::new(st).is_file() {
            fail(&format!("--slab-table 资产缺失: {st}（fail-closed 不静默回退）"));
        }
    } else {
        if slab_arm_explicit {
            fail("--slab-arm 须随 --slab-table");
        }
        if spv_slab_explicit {
            fail("--spv-slab 须随 --slab-table");
        }
        if dump_last_frame.is_some() {
            fail("--dump-last-frame 须随 --slab-table（B3 跨臂像素对拍面）");
        }
    }
    // A5 FG 闭集约束（fail-fast 如实拒跑,不静默降级）。
    if fg != G31Fg::Off {
        if auto_move.is_none() {
            fail("--fg 须随 --auto-move（A5 FG 登记面 = 确定性轨迹;交互面 FG 非本任务口径）");
        }
        if tier != 100 {
            fail("--fg 须 --tier 100（kernel 要求 prev/cur/mv 同栅格,MV 产出于 internal 分辨率;tier<100 的 MV 重采样非本任务面,如实拒跑不冒充）");
        }
        if warmup + frames < 2 {
            fail("--fg 须 frames+warmup ≥ 2（生成帧需 prev/cur 真渲帧对）");
        }
        if headless {
            fail("--fg 与 --headless-smoke 互斥（A5 登记面 = 真窗口 present 双口径;无窗退化不记 FG 门）");
        }
        // G37 W3 fg_combo 合入：两点式闭集卫兵——fg 合法形态 = {全画质 off
        // base} ∪ {--quality full 预设字面}（REPORT §2.2;散臂混搭每形态需独立
        // FG 静态下标族,开放 2^N 组合 = 下标族爆炸即 AE 红修 #2 事故几何,full
        // 为唯一生产预设）。语义适配登记：报告 A6 原文「quality_full &&
        // (gi2_ris||gi2_nee) 拒跑」因 W2 已将 ris/nee 并入 full 展开（22 项,
        // 进 dup 表）而语义反转——full 终态即含 ris/nee,FG_FULL 下标族按
        // TEXNRM_BLOOM_RIS+AE 终态（48..=56）定死;按现文件适配为：
        // !quality_full 面对「须随 smooth-normals/textures 传递覆盖」的散臂
        // 全量 fail-fast 显式化（防后续臂解除上游门时静默放行;textures/
        // smooth-normals/bloom/auto-exposure/tsr-quality 五门自带 !quality_full
        // 豁免字面,lut/storm/hzb/slab/svt/headless 各有自身 fg 门,dither 既
        // 有开放——均不重复）。
        if !quality_full
            && (ggx
                || lamp_lights
                || gi2
                || emissive_tex
                || metal_f0
                || rt_ao
                || soft_shadows
                || rt_reflect
                || gi2_tex
                || normal_maps
                || transparency
                || gi2_ris
                || gi2_nee)
        {
            fail("--fg × 画质散臂组合面 = 两点式闭集（fg 合法形态 = 全画质 off base ∪ --quality full 预设字面;FG_FULL 下标族按 TEXNRM_BLOOM_RIS+AE 终态 48..=56 定死,散臂微调混搭下标族爆炸即红修 #2 事故几何——弃 fg 或改用 --quality full,如实拒跑不冒充,G37 W3 fg_combo 判档）");
        }
    }
    // B1 HZB 闭集约束（fail-fast 如实拒跑,不静默降级）。
    let spv_hzb_explicit = args.iter().any(|a| a.starts_with("--spv-hzb"));
    if hzb == G31Hzb::On {
        if fg != G31Fg::Off {
            fail("--hzb on 与 --fg 互斥（B1 接线面 = 生产五 pass 现状车道;FG 组合面非本任务口径,如实拒跑不冒充）");
        }
        if slab_table.is_some() {
            fail("--hzb on 与 --slab-table 互斥（B1/B3 组合面非本任务口径,如实拒跑不冒充）");
        }
        if tier != 100 {
            fail("--hzb on 须 --tier 100（B1 登记面 = bistro 1080p 内部分辨率金字塔拓扑;其它 tier 面非本任务口径,如实拒跑不冒充）");
        }
    } else if spv_hzb_explicit {
        fail("--spv-hzb-* 须随 --hzb on（hzb off 面 = 车道 0-byte,SPV 覆盖位无消费面）");
    }
    // B4 纹理采样闭集约束（fail-fast 如实拒跑,不静默降级）。day_0828 Phase B：
    // ① 移除「须随 --auto-move」——heap+mip 重锚后登记口径 = 静态契约相机
    //   双跑位级一致（a2b/a3 六臂组合同款协议;三验收位裁剪须与 A3 基线同
    //   角度）,--auto-move 轨迹面仍兼容;
    // ② --smooth-normals 组合解除（合流臂换载 g31_texture_nrm_gi）;
    // ③ --bloom/--dither/--auto-exposure/--lamp-lights 全可组合。
    let spv_tex_explicit = args.iter().any(|a| a.starts_with("--spv-texture"));
    if textures {
        // G37 W3 fg_combo 合入：fg 项加 !quality_full 豁免（full 面 FG 经
        // g31_apply_fg_full 施加于 tex_descs 终态,comp parity 双缓冲接线）。
        if fg != G31Fg::Off && !quality_full {
            fail("--textures on 与 --fg 互斥（B4 接线面 = 生产五 pass 现状车道;FG 组合面非本任务口径,如实拒跑不冒充;fg×画质组合面经 --quality full 预设字面开放——两点式闭集,散臂微调组合仍拒,G37 W3 fg_combo 判档）");
        }
        if hzb == G31Hzb::On {
            fail("--textures on 与 --hzb on 互斥（B4/B1 组合面非本任务口径,如实拒跑不冒充）");
        }
        if slab_table.is_some() {
            fail("--textures on 与 --slab-table 互斥（B4/B3 组合面非本任务口径,如实拒跑不冒充）");
        }
        if tier != 100 {
            fail("--textures on 须 --tier 100（B4 登记面 = bistro 1080p 同机同窗 on/off 对照;其它 tier 面非本任务口径,如实拒跑不冒充）");
        }
        // 合流臂 SPV 换载（默认字面才换——用户显式 --spv-texture 面尊重不覆盖）。
        if smooth_nrm && spv_texture == G31_DEFAULT_SPV_TEXTURE {
            spv_texture = G31_DEFAULT_SPV_TEXTURE_NRM.to_owned();
        }
    } else if spv_tex_explicit {
        fail("--spv-texture* 须随 --textures on（textures off 面 = 车道 0-byte,SPV 覆盖位无消费面）");
    }
    // 画质战役 Phase C --gi2 闭集校验（fail-closed，不静默降级）：① 须随
    // --smooth-normals on 且 --textures on（GI2 段仅 g31_texture_nrm_gi 合流
    // kernel 存在——其余 scene kernel 不读 params[51..55)，开臂无语义；互斥
    // 集〔fg/hzb/svt/slab/cluster/wp〕随两前置臂上行已裁全覆盖）；② 子参数
    // 须随 on（off 面零消费）；③ 参数域校验（scale 正有限 / clamp 正有限）。
    if gi2 && !(smooth_nrm && textures) {
        fail("--gi2 on 须随 --smooth-normals on 且 --textures on（GI2 段仅 g31_texture_nrm_gi 合流 kernel 存在,fail-closed）");
    }
    // GI2 变体 SPV 换载（默认合流字面才换——用户显式 --spv-texture 面尊重不
    // 覆盖；路线隔离：off 臂恒载锚定字节，on 臂独载含 GI2 段变体）。
    if gi2 && spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM {
        spv_texture = G31_DEFAULT_SPV_TEXTURE_NRM_GI2.to_owned();
    }
    if !gi2 && (gi2_scale.is_some() || gi2_clamp.is_some()) {
        fail("--gi2-scale/--gi2-clamp 须随 --gi2 on（off 面零消费,fail-closed）");
    }
    let gi2_scale_v = gi2_scale.unwrap_or(1.0);
    let gi2_clamp_v = gi2_clamp.unwrap_or(4.0);
    if gi2 {
        if !(gi2_scale_v.is_finite() && gi2_scale_v > 0.0) {
            fail("--gi2-scale 必须为正有限 f32");
        }
        if !(gi2_clamp_v.is_finite() && gi2_clamp_v > 0.0) {
            fail("--gi2-clamp 必须为正有限 f32");
        }
    }
    // 画质战役 Phase F --emissive-tex 闭集校验（fail-closed，不静默降级）：
    // ① 须随 --smooth-normals on 且 --textures on（triem 绑定 + emissive
    //   采样段仅 g31_texture_nrm_gi_em 合流变体存在——其余 scene kernel 无
    //   第 14 路绑定，开臂无语义；互斥集随两前置臂上行已裁全覆盖）；
    // ② --emissive-dir 须随 on（off 面零消费）；③ 烘焙件缺件/manifest 失配
    //   = 装配期 fail-closed（g31_emissive_append）。
    let emissive_dir_explicit = args.iter().any(|a| a == "--emissive-dir");
    if emissive_tex && !(smooth_nrm && textures) {
        fail("--emissive-tex on 须随 --smooth-normals on 且 --textures on（emissive 采样段仅 g31_texture_nrm_gi_em 合流变体存在,fail-closed）");
    }
    if !emissive_tex && emissive_dir_explicit {
        fail("--emissive-dir 须随 --emissive-tex on（off 面零消费,fail-closed）");
    }
    // Phase F 变体 SPV 换载（默认字面才换——用户显式 --spv-texture 面尊重不
    // 覆盖；字节隔离：em off 各臂恒载既有锚定字节〔nrm/gi2 两件 0-byte〕，
    // em on 独载 em 工件——gi2 on/off 都用 em 工件，GI2 段 params[51] 门控
    // 在内〔上行 gi2 换载后本块再换,两默认字面都接〕）。
    if emissive_tex
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2)
    {
        spv_texture = G31_DEFAULT_SPV_TEXTURE_NRM_EM.to_owned();
    }
    // day_0829 臂① --metal-f0 闭集校验（fail-closed，不静默降级）：须随
    // --ggx on 且 --textures on（F0 消费面 = GGX 高光段;tri_base tex/常量
    // 双路语义随 g31_realism 合流 kernel——--ggx 已裁须随 --smooth-normals,
    // 互斥集随前置臂上行已裁全覆盖）。
    if metal_f0 && !(ggx && textures) {
        fail("--metal-f0 on 须随 --ggx on 且 --textures on（F0 修伤面 = GGX 高光段,g31_realism 合流 kernel,fail-closed）");
    }
    // day_0829 realism 链 SPV 换载（默认字面才换——用户显式 --spv-texture 面
    // 尊重不覆盖;字节隔离:realism 全臂 off 恒载 night_0828 三既有锚定字节
    // 〔nrm/gi2/em 三件 0-byte 不动〕,任一 realism 臂 on 独载链工件——gi2/em
    // on/off 都用链工件,各段 params 门控在内〔上行三换载后本块再换,三默认
    // 字面都接〕）。
    if metal_f0
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_EM)
    {
        spv_texture = G31_DEFAULT_SPV_REALISM_F0.to_owned();
    }
    // day_0829 臂② --rt-ao 闭集校验 + 链换载（默认字面才换;子参数域校验
    // fail-closed）。
    if rt_ao && !(smooth_nrm && textures) {
        fail("--rt-ao on 须随 --smooth-normals on 且 --textures on（AO 半球基 = 平滑法线,g31_realism 链基座,fail-closed）");
    }
    if !rt_ao && (rt_ao_radius.is_some() || rt_ao_strength.is_some() || rt_ao_samples.is_some()) {
        fail("--rt-ao-radius/--rt-ao-strength/--rt-ao-samples 须随 --rt-ao on（off 面零消费,fail-closed）");
    }
    let rt_ao_radius_v = rt_ao_radius.unwrap_or(0.5);
    let rt_ao_strength_v = rt_ao_strength.unwrap_or(0.85);
    let rt_ao_samples_v = rt_ao_samples.unwrap_or(2);
    if rt_ao {
        if !(rt_ao_radius_v.is_finite() && rt_ao_radius_v > 0.0) {
            fail("--rt-ao-radius 必须为正有限 f32（米）");
        }
        if !(rt_ao_strength_v.is_finite() && rt_ao_strength_v > 0.0 && rt_ao_strength_v <= 1.0) {
            fail("--rt-ao-strength 必须 ∈ (0,1] 有限 f32");
        }
        if !(1..=8).contains(&rt_ao_samples_v) {
            fail("--rt-ao-samples 必须 ∈ [1,8]（帧时预算面）");
        }
    }
    if rt_ao
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_EM
            || spv_texture == G31_DEFAULT_SPV_REALISM_F0)
    {
        spv_texture = G31_DEFAULT_SPV_REALISM_AO.to_owned();
    }
    // day_0829 臂⑤ --soft-shadows 闭集校验 + 链换载（默认字面才换;子参数域
    // 校验 fail-closed）。
    if soft_shadows && !(lamp_lights && textures) {
        fail("--soft-shadows on 须随 --lamp-lights on 且 --textures on（软影面 = 点灯阴影射线,g31_realism 链基座,fail-closed）");
    }
    if !soft_shadows && soft_shadow_samples.is_some() {
        fail("--soft-shadow-samples 须随 --soft-shadows on（off 面零消费,fail-closed）");
    }
    let soft_shadow_samples_v = soft_shadow_samples.unwrap_or(2);
    if soft_shadows && !(1..=8).contains(&soft_shadow_samples_v) {
        fail("--soft-shadow-samples 必须 ∈ [1,8]（帧时预算面——12 点光 × N 条射线）");
    }
    if soft_shadows
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_EM
            || spv_texture == G31_DEFAULT_SPV_REALISM_F0
            || spv_texture == G31_DEFAULT_SPV_REALISM_AO)
    {
        spv_texture = G31_DEFAULT_SPV_REALISM_SOFT.to_owned();
    }
    // day_0829 臂③ --rt-reflect 闭集校验 + 链换载（默认字面才换;子参数域
    // 校验 fail-closed）。
    if rt_reflect && !(ggx && textures) {
        fail("--rt-reflect on 须随 --ggx on 且 --textures on（反射面 = F0/roughness 消费,g31_realism 链基座,fail-closed）");
    }
    if !rt_reflect && (rt_reflect_rough_max.is_some() || rt_reflect_clamp.is_some()) {
        fail("--rt-reflect-rough-max/--rt-reflect-clamp 须随 --rt-reflect on（off 面零消费,fail-closed）");
    }
    let rt_reflect_rough_max_v = rt_reflect_rough_max.unwrap_or(0.55);
    let rt_reflect_clamp_v = rt_reflect_clamp.unwrap_or(8.0);
    if rt_reflect {
        if !(rt_reflect_rough_max_v.is_finite()
            && rt_reflect_rough_max_v >= 0.05
            && rt_reflect_rough_max_v <= 1.0)
        {
            fail("--rt-reflect-rough-max 必须 ∈ [0.05,1] 有限 f32");
        }
        if !(rt_reflect_clamp_v.is_finite() && rt_reflect_clamp_v > 0.0) {
            fail("--rt-reflect-clamp 必须为正有限 f32");
        }
    }
    if rt_reflect
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_EM
            || spv_texture == G31_DEFAULT_SPV_REALISM_F0
            || spv_texture == G31_DEFAULT_SPV_REALISM_AO
            || spv_texture == G31_DEFAULT_SPV_REALISM_SOFT)
    {
        spv_texture = G31_DEFAULT_SPV_REALISM_REFL.to_owned();
    }
    // day_0829 臂⑥ --gi2-tex 闭集校验 + 链换载（默认字面才换）。
    if gi2_tex && !gi2 {
        fail("--gi2-tex on 须随 --gi2 on（GI2 反弹段消费面,fail-closed）");
    }
    if gi2_tex
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_EM
            || spv_texture == G31_DEFAULT_SPV_REALISM_F0
            || spv_texture == G31_DEFAULT_SPV_REALISM_AO
            || spv_texture == G31_DEFAULT_SPV_REALISM_SOFT
            || spv_texture == G31_DEFAULT_SPV_REALISM_REFL)
    {
        spv_texture = G31_DEFAULT_SPV_REALISM_GITEX.to_owned();
    }
    // day_0829 臂④ --normal-maps 闭集校验 + 链换载（默认字面才换;子参数域
    // 校验 fail-closed;nrm 为最高链位——签名 +2 buffer,desc/AE 下标族独立）。
    let normal_dir_explicit = args.iter().any(|a| a == "--normal-dir");
    if normal_maps && !(smooth_nrm && textures) {
        fail("--normal-maps on 须随 --smooth-normals on 且 --textures on（TBN 基 = 平滑法线,heap 采样面,fail-closed）");
    }
    if !normal_maps && (normal_strength.is_some() || normal_dir_explicit) {
        fail("--normal-strength/--normal-dir 须随 --normal-maps on（off 面零消费,fail-closed）");
    }
    let normal_strength_v = normal_strength.unwrap_or(1.0);
    if normal_maps && !(normal_strength_v.is_finite() && normal_strength_v > 0.0 && normal_strength_v <= 2.0) {
        fail("--normal-strength 必须 ∈ (0,2] 有限 f32");
    }
    if normal_maps
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_EM
            || spv_texture == G31_DEFAULT_SPV_REALISM_F0
            || spv_texture == G31_DEFAULT_SPV_REALISM_AO
            || spv_texture == G31_DEFAULT_SPV_REALISM_SOFT
            || spv_texture == G31_DEFAULT_SPV_REALISM_REFL
            || spv_texture == G31_DEFAULT_SPV_REALISM_GITEX)
    {
        spv_texture = G31_DEFAULT_SPV_REALISM_NRM.to_owned();
    }
    // G37 W2 臂⑦ --transparency 闭集校验 + 链换载（默认字面才换;子参数域
    // 校验 fail-closed;transp 为新最高链位——签名 +tri_transp,与 realism
    // 六臂正交组合〔transp on 而 nrm off 也走 _transp,链式超集 gate 控制〕）。
    if transparency && !(smooth_nrm && textures) {
        fail("--transparency on 须随 --smooth-normals on 且 --textures on（玻璃透射面 = g31_realism 链基座,fail-closed）");
    }
    if !transparency && transp_alpha.is_some() {
        fail("--transp-alpha 须随 --transparency on（off 面零消费,fail-closed）");
    }
    let transp_alpha_v = transp_alpha.unwrap_or(0.85);
    if transparency && !(transp_alpha_v.is_finite() && transp_alpha_v > 0.0 && transp_alpha_v <= 1.0) {
        fail("--transp-alpha 必须 ∈ (0,1] 有限 f32（逐面透射率;0 = 不透明无意义）");
    }
    if transparency
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_EM
            || spv_texture == G31_DEFAULT_SPV_REALISM_F0
            || spv_texture == G31_DEFAULT_SPV_REALISM_AO
            || spv_texture == G31_DEFAULT_SPV_REALISM_SOFT
            || spv_texture == G31_DEFAULT_SPV_REALISM_REFL
            || spv_texture == G31_DEFAULT_SPV_REALISM_GITEX
            || spv_texture == G31_DEFAULT_SPV_REALISM_NRM)
    {
        spv_texture = G31_DEFAULT_SPV_REALISM_TRANSP.to_owned();
    }
    // G37 W2 臂⑧ --gi2-ris/--gi2-nee 闭集校验 + 链换载(默认字面才换;
    // ris|nee 为新最高链位,与 realism 七臂正交组合)。
    if (gi2_ris || gi2_nee) && !gi2 {
        fail("--gi2-ris/--gi2-nee 须随 --gi2 on(反弹段属 GI2 加性臂,fail-closed)");
    }
    if (gi2_ris || gi2_nee) && !(smooth_nrm && textures) {
        fail("--gi2-ris/--gi2-nee 须随 --smooth-normals on 且 --textures on(g31_realism 链基座,fail-closed)");
    }
    if !gi2_ris && gi2_ris_m.is_some() {
        fail("--gi2-ris-m 须随 --gi2-ris on(off 面零消费,fail-closed)");
    }
    let gi2_ris_m_v = gi2_ris_m.unwrap_or(6);
    if gi2_ris && !(1..=16).contains(&gi2_ris_m_v) {
        fail("--gi2-ris-m 必须 ∈ [1,16](kernel 蓄水池候选域,fail-closed)");
    }
    if (gi2_ris || gi2_nee)
        && (spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_GI2
            || spv_texture == G31_DEFAULT_SPV_TEXTURE_NRM_EM
            || spv_texture == G31_DEFAULT_SPV_REALISM_F0
            || spv_texture == G31_DEFAULT_SPV_REALISM_AO
            || spv_texture == G31_DEFAULT_SPV_REALISM_SOFT
            || spv_texture == G31_DEFAULT_SPV_REALISM_REFL
            || spv_texture == G31_DEFAULT_SPV_REALISM_GITEX
            || spv_texture == G31_DEFAULT_SPV_REALISM_NRM
            || spv_texture == G31_DEFAULT_SPV_REALISM_TRANSP)
    {
        spv_texture = G31_DEFAULT_SPV_REALISM_RIS.to_owned();
    }
    // day_0829 realism 汇总门（任一臂 on ⇒ 链工件 + tri_base/triem 绑定面 +
    // params 扩容;后续臂并入本式。G37 W2:transparency 并入）。
    let realism_any = metal_f0
        || rt_ao
        || soft_shadows
        || rt_reflect
        || gi2_tex
        || normal_maps
        || transparency
        // G37 W2 ris_nee:臂⑧并入(triem 回退/tri_base 哑表/params 扩容)。
        || gi2_ris
        || gi2_nee;
    // 画质战役 Phase D --tsr-quality 闭集校验（fail-closed，不静默降级）：
    // ① 互斥集 = fg/hzb on/svt on/slab/cluster/wp（--fg 组合面未接线；
    //   --hzb 车道 G31HzbLane 自有 prepare 路 tsr_params[19..21) 未接线——
    //   开臂参数面空转即冒充，fail-closed；其余同 bloom/AE 互斥律）；与
    //   --dither/--smooth-normals/--ggx/--lamp-lights/--textures/--gi2/
    //   --bloom/--auto-exposure 全可组合（resolve 面正交）。
    // ② 子参数须随 on（off 面零消费）；③ 参数域（min-alpha ∈ (0,1) 有限 /
    //   clamp 非负有限）。
    if tsr_quality {
        // G37 W3 fg_combo 合入：fg 项加 !quality_full 豁免（resolve 行为仅改
        // out_color 内容,零适配正交,§1.4）。
        if (fg != G31Fg::Off && !quality_full) || hzb == G31Hzb::On || svt_on || slab_table.is_some() {
            fail("--tsr-quality on 不与 --fg/--hzb on/--svt on/--slab-table 同跑（hzb 车道 prepare 路 tsr_params[19..21) 未接线;其余组合面未接线,fail-closed;fg×画质组合面经 --quality full 预设字面开放——两点式闭集,散臂微调组合仍拒,G37 W3 fg_combo 判档）");
        }
        if cluster_lod_mode != "off" || wp_hlod_mode != "off" {
            fail("--tsr-quality on 不与 --cluster-lod/--wp-hlod 同跑（组合面未接线,fail-closed）");
        }
    }
    if !tsr_quality && (tsrq_min_alpha.is_some() || tsrq_clamp.is_some()) {
        fail("--tsrq-min-alpha/--tsrq-clamp 须随 --tsr-quality on（off 面零消费,fail-closed）");
    }
    let tsrq_min_alpha_v = tsrq_min_alpha.unwrap_or(0.02);
    let tsrq_clamp_v = tsrq_clamp.unwrap_or(0.0);
    if tsr_quality {
        if !(tsrq_min_alpha_v.is_finite() && tsrq_min_alpha_v > 0.0 && tsrq_min_alpha_v < 1.0) {
            fail("--tsrq-min-alpha 必须 ∈ (0,1) 有限 f32");
        }
        if !(tsrq_clamp_v.is_finite() && tsrq_clamp_v >= 0.0) {
            fail("--tsrq-clamp 必须为非负有限 f32");
        }
    }
    // Phase D resolve SPV 换载（默认字面才换——用户显式 --spv-resolve 面尊重
    // 不覆盖；字节隔离：off 臂恒载 m_c 冻结字节，on 臂独载
    // g31_tsr_resolve_q.spv——C 相纪律「保锚一律字节隔离」）。
    if tsr_quality && spv_resolve == DEFAULT_SPV_RESOLVE {
        spv_resolve = DEFAULT_SPV_RESOLVE_Q.to_owned();
    }
    // G37 W2 合入:--lut 色彩分级臂(M119 五级链第 4 级;TODO #79 缺级收口)。
    // 传输面 = enc_params 尾挂([134] 门/[135] dim/[136..) 表体),零新资源/
    // 绑定/屏障/下标族;换载「默认字面才换」+ 字节隔离(off 恒载 v2 锚定字节)。
    let lut_asset = match g31_lut_assets::from_arg(&lut) {
        Ok(a) => a,
        Err(e) => fail(&format!("--lut: {e}")),
    };
    if lut_asset.is_some() {
        if fg != G31Fg::Off || hzb == G31Hzb::On || svt_on || slab_table.is_some() {
            fail("--lut 非 off 不与 --fg/--hzb on/--svt on/--slab-table 同跑（组合面未接线,fail-closed）");
        }
        if cluster_lod_mode != "off" || wp_hlod_mode != "off" {
            fail("--lut 非 off 不与 --cluster-lod/--wp-hlod 同跑（组合面未接线,fail-closed）");
        }
        if spv_encode == G31_DEFAULT_SPV_ENCODE {
            spv_encode = G31_DEFAULT_SPV_ENCODE_LUT.to_owned();
        } else {
            fail("--lut 非 off 与显式 --spv-encode 同给（LUT 段归默认链工件;显式 SPV 无 LUT 段即静默失效冒充,fail-closed）");
        }
    }
    // C13 SVT 派生臂闭集约束（fail-fast 如实拒跑,不静默降级）。day_0828
    // Phase B：--svt 与新 texel heap 形态 fail-closed 互斥——SVT 页表/瓦片
    // 集/探针假设 = 2048 网格图集 + texmeta origin + tritex 步幅 1,heap 化
    // 全部破坏;不深修 SVT（终态平移归后续波）,如实拒跑登记。
    let svt_spv_explicit = args
        .iter()
        .any(|a| a.starts_with("--spv-svt") || a.starts_with("--svt-pool-tiles"));
    if svt_on {
        fail("--svt on 与 day_0828 Phase B texel heap 纹理形态互斥（SVT 假设 = 2048 网格图集/texmeta origin/tritex 步幅 1,heap 化未适配——fail-closed 登记,SVT 深修归后续波）");
    } else if svt_spv_explicit {
        fail("--spv-svt*/--svt-pool-tiles 须随 --svt on（svt off 面 = 车道 0-byte,SPV/池覆盖位无消费面）");
    }
    // C4 故障注入/窗口风暴闭集约束（fail-fast 如实拒跑,不静默降级）。
    if let Some(spec) = fault_probe.as_deref() {
        if !G31_FAULT_PROBES.contains(&spec) {
            fail(&format!(
                "--fault-probe {spec} 越闭集({})",
                G31_FAULT_PROBES.join("|")
            ));
        }
        if spec.starts_with("device-lost-") && headless {
            fail("--fault-probe device-lost-* 面 = present 会话,与 --headless-smoke 互斥（无窗口无 present 站,如实拒跑不冒充）");
        }
    }
    if window_storm > 0 || storm_soak > 0 || fault_probe.is_some() {
        let arm = if fault_probe.is_some() {
            "--fault-probe"
        } else if window_storm > 0 {
            "--window-storm"
        } else {
            "--storm-soak"
        };
        if window_storm > 0 && storm_soak > 0 {
            fail("--window-storm 与 --storm-soak 互斥（爆发/周期两臂分开登记,不混口径）");
        }
        if fault_probe.is_some() && (window_storm > 0 || storm_soak > 0) {
            fail("--fault-probe 与 --window-storm/--storm-soak 互斥（探针/风暴分开登记,不混口径）");
        }
        // day_0828 Phase E1:互斥集移除 textures——era 重建走完整变体描述组
        // 重建（tex/tex_nrm/tex_bloom/tex_nrm_bloom 四形态 descs 逐 era 重建,
        // texel heap 为分辨率无关静态侧表）,--quality full × 风暴组合以 E1
        // 风暴真跑验收（e_final/e4_storm_summary.json:干净退出 + resize_eras
        // ≥1 + validation 静默）。--svt 流送状态 × 风暴未验收,由隐式（随
        // textures 上行）转显式 fail-closed。
        if fg != G31Fg::Off || hzb == G31Hzb::On || slab_table.is_some() || svt_on {
            fail(&format!(
                "{arm} 与 --fg/--hzb on/--slab-table/--svt on 互斥（C4 登记面;组合面未验收,如实拒跑不冒充）"
            ));
        }
        if headless && (window_storm > 0 || storm_soak > 0) {
            fail(&format!(
                "{arm} 面 = 真窗口 present 会话,与 --headless-smoke 互斥（无窗退化不记 C4 门）"
            ));
        }
    }
    // A5 冻结容差：--fg-tol 优先,缺省程序读 G26 budget 标定条目（fail-closed）。
    let (fg_tol_v, fg_tol_measured, fg_tol_source) = if fg != G31Fg::Off {
        match fg_tol {
            Some(t) => (t, f64::NAN, "--fg-tol 命令行显式".to_owned()),
            None => match g31_fg_frozen_tol(G31_G26_BUDGET) {
                Ok(v) => v,
                Err(e) => fail(&format!("A5 冻结容差读取: {e}")),
            },
        }
    } else {
        (0.0, f64::NAN, String::new())
    };

    // ① 生产契约(digest 门 == FROZEN;G14.3 同模拒出图纪律)。
    let scene_id = "bistro-interior";
    let (pre, _) = prelude(
        scene_id,
        tier,
        frames,
        false,
        &contract_path,
        expect_digest.as_deref(),
    );
    let contract = &pre.contract;
    let (out_w, out_h, seed) = (pre.out_w, pre.out_h, pre.seed);

    // ② G10 语料转引一致性核验(不等即 RED 拒跑;轨迹基位 = 契约相机,先验后跑)。
    let srow = contract_scene_row(&contract.raw, scene_id).unwrap_or_else(|e| fail(&e));
    let g10_fragment = match g31_g10_corpus_gate(srow, &g10_dir) {
        Ok(f) => f,
        Err(e) => fail(&format!("G10 语料转引一致性核验 RED: {e}")),
    };
    eprintln!(
        "{GTAG}: 契约链就绪 contract_digest={} g10 转引一致性=pass",
        contract.digest
    );

    // ③ 场景装配（B1:--hzb on 面走 assemble_scene_ex 追加逐节点分组——SceneData
    //    各字段与 off 面逐位同值,节点分组 = 纯记录面;剔除对象粒度 = TLAS 实例
    //    粒度 = 逐 mesh 节点）。
    if gltf_path.is_empty() {
        // 缺省 glTF 路径（A1 既有面——契约场景 → 默认资产映射;--gltf 显式优先）。
        gltf_path = default_gltf(scene_id).to_owned();
    }
    let mut hzb_groups: Vec<SceneNodeGroup> = Vec::new();
    // B4 UV sink（textures on 面 = 6 f32/tri 装配产出;off = 空 vec 零消费）。
    let mut tri_uv: Vec<f32> = Vec::new();
    // D2 法线 sink（smooth-normals on 面 = 9 f32/tri 装配产出;off = 空 vec
    // 零消费——不读 NORMAL、不产侧表,0-byte）。
    let mut nrm_sink: Vec<f32> = Vec::new();
    // D6 MR sink（--ggx on 面 = 2 f32/tri 装配产出;off = 空 vec 零消费——
    // 不读 roughnessFactor 进侧表、不产侧表,0-byte）。
    let mut mr_sink: Vec<f32> = Vec::new();
    let mut scene = match if textures && smooth_nrm {
        // day_0828 Phase B 合流臂：UV + 顶点法线（+ --ggx on 时 MR）三 sink
        // 同窗装配（assemble_scene_ex_nrm 全量面;SceneData 各字段与既有臂
        // 逐位同值——sink 皆旁路纯记录）。
        assemble_scene_ex_nrm(
            &contract.raw,
            scene_id,
            Path::new(&gltf_path),
            None,
            Some(&mut tri_uv),
            Some(&mut nrm_sink),
            if ggx { Some(&mut mr_sink) } else { None },
        )
    } else if smooth_nrm && ggx {
        // D6：--ggx on = assemble_scene_nrm_mr 同窗追加法线 + MR 双侧表
        //（CLI 已裁「须随 --smooth-normals on」与互斥,分支序安全）。
        assemble_scene_nrm_mr(
            &contract.raw,
            scene_id,
            Path::new(&gltf_path),
            &mut nrm_sink,
            &mut mr_sink,
        )
    } else if smooth_nrm {
        // D2：smooth-normals on = assemble_scene_nrm 追加顶点法线侧表（CLI
        // 已裁 hzb 互斥,分支序安全）。
        assemble_scene_nrm(&contract.raw, scene_id, Path::new(&gltf_path), &mut nrm_sink)
    } else if hzb == G31Hzb::On {
        assemble_scene_ex(&contract.raw, scene_id, Path::new(&gltf_path), Some(&mut hzb_groups), None)
    } else if textures {
        assemble_scene_uv(&contract.raw, scene_id, Path::new(&gltf_path), &mut tri_uv)
    } else {
        assemble_scene(&contract.raw, scene_id, Path::new(&gltf_path))
    } {
        Ok(s) => s,
        Err(e) => g31_dev_env_or_fail("scene_assets", &e),
    };
    // A1 灯光提取施加点（--lamp-lights on 才 mutate scene.points;off =
    // 零触达——points 面/pack/参数 count 全 0-byte。bench 车道同律,
    // apply_lamp_lights = 共享体同一事实源）。
    if lamp_lights {
        scene = apply_lamp_lights(
            scene,
            &LampOpt {
                enabled: true,
                gain: lamp_gain_v,
                max_k: lamp_k_v,
                contrib: lamp_contrib_v,
                stats_out: String::new(),
            },
        );
    }
    if hzb == G31Hzb::On && hzb_groups.is_empty() {
        fail("HZB 面场景零可剔除实例（节点分组为空,fail-closed 不冒充）");
    }
    // G37 W3 frame_cut 合入:passthrough 源三角流提取（须先于 apply_cluster_lod
    // ——cut 重建后源三角序不复存在;off 空 vec 零消费。簇包预读为 on 臂加性
    // 成本（~49MB 双读）,与 ctx 内簇包同文件同校验,fail-closed 互证）。
    let frame_cut_pt_stream: Vec<f32> = if frame_cut_opt.enabled {
        let p = read_cluster_pack(Path::new(&cluster_opt.pack_path))
            .unwrap_or_else(|e| fail(&format!("--cluster-per-frame-cut 簇包预读: {e}")));
        verify_cluster_pack(&p, &scene)
            .unwrap_or_else(|e| fail(&format!("--cluster-per-frame-cut 簇包校验: {e}")));
        frame_cut_passthrough_stream(&scene, &p.passthrough)
    } else {
        Vec::new()
    };
    // ③.4 G31+ #58 簇 LOD 施加点（off 直通零改动;leaf/on 时以**契约初始相机**
    //     在初始内部分辨率下 cut 重建三角汤——出帧几何本会话冻结;主循环逐帧
    //     cut 统计见下,簇包保留复用）。
    let cluster_ctx: Option<(ClusterLodReport, ClusterPack)> = {
        let init_in_w = ((out_w as u64 * u64::from(tier)) / 100).max(1) as u32;
        let init_in_h = ((out_h as u64 * u64::from(tier)) / 100).max(1) as u32;
        let (s2, ctx) = apply_cluster_lod(scene, &cluster_opt, init_in_w, init_in_h);
        scene = s2;
        if let Some((r, _)) = &ctx {
            eprintln!(
                "{GTAG}: cluster-lod mode={} threshold_px={} blocks={} clusters={}/{} tris out={}/{} ({:.1}%)",
                r.mode,
                r.threshold_px,
                r.blocks,
                r.cut_clusters,
                r.total_clusters,
                r.out_tris,
                r.src_tris,
                100.0 * r.out_tris as f64 / r.src_tris.max(1) as f64,
            );
        }
        ctx
    };
    // ③.4b G31+ #95/#68 WP/HLOD 施加点（off 直通零改动;full/on 时以契约初始
    //     相机做 cell 流送 + 互斥选层重建三角汤——出帧几何本会话冻结;主循环
    //     逐帧 tick/选层/warmup 切换统计见下,上下文保留复用。#68 代理 GPU
    //     绘制腿 = 代理三角随重建进 BLAS 出帧）。
    let mut wp_ctx: Option<(WpHlodReport, WpHlodContext)> = {
        let (s2, ctx) = apply_wp_hlod(scene, &wp_opt);
        scene = s2;
        if let Some((r, _)) = &ctx {
            eprintln!(
                "{GTAG}: wp-hlod mode={} cells full/hlod/culled/pending={}/{}/{}/{} (resident={}/{}) tris: src={} passthrough={} full={} proxy={} out={} ({:.1}%) ticks={} stall_frames={} selection_digest={}",
                r.mode,
                r.cells_full,
                r.cells_hlod,
                r.cells_culled,
                r.cells_pending,
                r.cells_resident,
                r.cells_nonempty,
                r.src_tris,
                r.passthrough_tris,
                r.full_tris,
                r.proxy_tris,
                r.out_tris,
                100.0 * r.out_tris as f64 / r.src_tris.max(1) as f64,
                r.assemble_ticks,
                r.budget_stall_frames,
                &r.selection_digest[..16],
            );
        }
        ctx
    };
    // ③.5 B3 slab 侧表生产接线（--slab-table 面;非 slab 路径 0-byte——资产加载
    //     + 16 槽 host/device 双臂求值对拍 + 逐三角 albedo 预调制,全部仅 slab
    //     模式消费;kernels/g29_slab.rx 与 material/slab.rs 0-byte 冻结消费）。
    let mut slab_report: Option<(SlabSideTableAsset, SlabEval, usize)> = None;
    if let Some(st) = slab_table.as_deref() {
        let asset = match slab_load_asset(st) {
            Ok(a) => a,
            Err(e) => fail(&format!("slab 侧表资产加载: {e}")),
        };
        if asset.scene_id != scene_id {
            fail(&format!(
                "slab 资产 scene_id={} ≠ 生产场景 {scene_id}（资产-场景绑定 fail-closed）",
                asset.scene_id
            ));
        }
        let eval = match slab_evaluate(&asset, &spv_slab) {
            Ok(v) => v,
            Err(e) => g31_dev_env_or_fail("slab_device_eval", &e),
        };
        let arm_r = slab_arm_r(&eval, &slab_arm);
        let n_slab = slab_apply(&mut scene, &asset, &arm_r);
        eprintln!(
            "{GTAG}: slab 接线 arm={} slots=16 mapped_mats={} slab_tris={} parity_p100={:.6e} eval_ms={:.3} abi={}",
            slab_arm,
            asset.material_slots.len(),
            n_slab,
            eval.parity_p100,
            eval.eval_ms,
            asset.abi_digest,
        );
        slab_report = Some((asset, eval, n_slab));
    }
    // ③.6 B4 纹理采样生产接线（--textures on 面;非 textures 路径 0-byte——
    //     资产加载（top-12 律法 + BC1/BC3 解码 + 图集烘焙 + G11.3 manifest
    //     互核）+ 探针双臂对拍（SSBO 腿位级硬门 + sampler 腿结构容差）+
    //     生产场景 kernel 纹理变体 SPV 装载（NoContraction 注入 = 驱动 FMA
    //     收缩禁面,SPV 文件 0-byte 后处理）,全部仅 textures on 消费;
    //     kernels/g14_3_direct_gi.rx 母版与车道 off 面 0-byte 回归锚）。
    let mut tex_report: Option<(G31TexAssetsHeap, G31TexProbeReport)> = None;
    let mut tex_spv_bytes: Vec<u8> = Vec::new();
    // day_0828 Phase F：emissive 资产面（em on 才构造——triem 侧表 + 登记行;
    // 所有者先于 'eras 循环,era 重建期 descs 借用面）。
    let mut em_assets: Option<G31EmissiveAssets> = None;
    // day_0829 臂④：trinm 侧表暂存（--normal-maps on 装配段产出,era 外打包;
    // off 恒空零消费）。
    let mut trinm_vec: Vec<f32> = Vec::new();
    if textures {
        let assets = match g31_tex_load_heap(&scene, Path::new(&gltf_path), &tri_uv) {
            Ok(a) => a,
            Err(e) => g31_dev_env_or_fail("texture_assets", &e),
        };
        // day_0828 Phase F：emissive 贴图装配（em on 面——4 张烘焙件追加 heap
        // 槽 70..73〔头表 74×13 全重排布〕+ triem 侧表 + scale 标定进 texmeta
        // mod 位;追加在探针**之前** ⇒ B4 探针双臂自动覆盖 4 emissive 槽。
        // off = assets 零触达,B4 面 0-byte）。
        let assets = if emissive_tex {
            let mut a = assets;
            let em_list = g31_contract_emissive_list(srow)
                .unwrap_or_else(|e| fail(&format!("Phase F 契约 emissive 段: {e}")));
            let em = match g31_emissive_append(&mut a, &scene.tri_mat, &em_list, &emissive_dir) {
                Ok(x) => x,
                Err(e) => fail(&format!("Phase F emissive 装配: {e}")),
            };
            eprintln!(
                "{GTAG}: Phase F emissive 接线 slots={}..{} em_tris={} appended_u32={}（+{}B） manifest={}（{}） scales={}",
                a.slots.len() - em.rows.len(),
                a.slots.len() - 1,
                em.em_tris,
                em.appended_texels,
                em.appended_texels * 4,
                em.manifest_path,
                &em.manifest_sha256[..15],
                em.rows
                    .iter()
                    .map(|r| format!(
                        "mat{}[{:.4},{:.4},{:.4}]{}",
                        r.material_index,
                        r.scale_rgb[0],
                        r.scale_rgb[1],
                        r.scale_rgb[2],
                        if r.fallback { "(fallback)" } else { "" }
                    ))
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            em_assets = Some(em);
            a
        } else {
            assets
        };
        // day_0829 臂④：法线烘焙容器 append（em 之后 ⇒ 槽号序 = albedo 0..69
        // + em 70..73〔on 时〕+ 法线尾接;off = assets 零触达 0-byte;探针在
        // 后 ⇒ 法线槽 SSBO 位级对拍自动覆盖——em 先例同律）。
        let assets = if normal_maps {
            let mut a = assets;
            let (t, nm_tris, appended) =
                match g31_normals_append(&mut a, &scene.tri_mat, &normal_dir) {
                    Ok(x) => x,
                    Err(e) => fail(&format!("臂④ 法线装配: {e}")),
                };
            eprintln!(
                "{GTAG}: day_0829 臂④ 法线接线 slots={}..{} nm_tris={} appended_u32={}（+{}B）",
                a.slots.len() - 70,
                a.slots.len() - 1,
                nm_tris,
                appended,
                appended * 4,
            );
            trinm_vec = t;
            a
        } else {
            assets
        };
        // day_0828 Phase B：探针律法扩 mip 维（每槽 24 UV × 抽 3 级,lod
        // 显式注入——heap 逐级寻址对拍面;Phase F em on 时探针覆盖 74 槽）。
        let probes = g31_tex_probes_mip(&assets.slots);
        let report = match g31_tex_probe_evaluate_mip(&assets, &probes, &spv_texture_probe) {
            Ok(r) => r,
            Err(e) => g31_dev_env_or_fail("texture_probe", &e),
        };
        if !report.ssbo_bitexact {
            fail(&format!(
                "B4 probe SSBO 腿 device vs host 非位级一致（p100={:.6e} > 0.0 硬门;NoContraction/采样链缺陷即红）",
                report.ssbo_p100
            ));
        }
        if !report.ssbo_double_run_bitexact {
            fail("B4 probe SSBO 腿 device 双跑非位级一致（确定性门红）");
        }
        if report.sampler_max_lsb > 1 {
            fail(&format!(
                "B4 sampler 腿硬件采样 vs host 参考 max_lsb={} > 1（结构容差界红;硬件过滤精度越界）",
                report.sampler_max_lsb
            ));
        }
        if report.nonconstant_slots == 0 {
            fail("B4 映射纹理探针输出全常量（空接线冒充即红,fail-closed）");
        }
        let spv_words = spv_inject_no_contraction(&load_spv(&spv_texture));
        tex_spv_bytes = spv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        eprintln!(
            "{GTAG}: Phase B 纹理接线 mapped={} tex_tris={} heap_texels={} heap_bytes={} header_entries={} probes={} ssbo_p100={:.6e}（位级={} 双跑={}） sampler_max_lsb={}（位级={}） nonconstant_slots={} eval_ms={:.3}",
            assets.slots.len(),
            assets.tex_tris,
            assets.heap_texels,
            assets.heap_texels * 4,
            assets.heap_header_entries,
            report.probe_count,
            report.ssbo_p100,
            report.ssbo_bitexact,
            report.ssbo_double_run_bitexact,
            report.sampler_max_lsb,
            report.sampler_bitexact,
            report.nonconstant_slots,
            report.eval_ms,
        );
        tex_report = Some((assets, report));
    }
    // ③.7 C13 SVT 生产接线（--svt on 派生臂面;非 svt 路径 0-byte——瓦片集
    //     构建（SVT-3 border 复制）+ 流送状态初态（全驻留锚臂/冷启动小池臂）+
    //     探针双臂对拍（①全驻留 SVT vs 整图直采位级硬门〔SVT-1/3〕②部分驻留
    //     请求位级 + host 消费闭环重跑全 hit〔SVT-2〕）+ 生产 kernel SPV 装载
    //     （NoContraction 注入,B4 同律）,全部仅 svt on 消费）。
    let mut svt_report: Option<(G31SvtAssets, G31SvtProbeReport)> = None;
    let mut svt_spv_bytes: Vec<u8> = Vec::new();
    if svt_on {
        let Some((tassets, _)) = tex_report.as_ref() else {
            fail("C13 SVT 须 B4 纹理资产面在案（--textures on 闭集已保证,防御性复核）");
        };
        let sassets = match g31_svt_build(tassets, svt_pool_tiles) {
            Ok(a) => a,
            Err(e) => fail(&format!("C13 SVT 资产装配: {e}")),
        };
        let sprobes = g31_svt_probes(tassets);
        let srep = match g31_svt_probe_evaluate(&sassets, tassets, &sprobes, &spv_svt_probe) {
            Ok(r) => r,
            Err(e) => g31_dev_env_or_fail("svt_probe", &e),
        };
        if !srep.full_bitexact_vs_direct || !srep.full_bitexact_vs_svt_host {
            fail(&format!(
                "C13 SVT 全驻留臂非位级一致（vs 直采={} vs host_svt={} p100={:.6e};SVT-1 页表间接/SVT-3 border 链缺陷即红）",
                srep.full_bitexact_vs_direct, srep.full_bitexact_vs_svt_host, srep.full_p100_vs_direct
            ));
        }
        if !srep.full_double_run_bitexact {
            fail("C13 SVT 全驻留臂 device 双跑非位级一致（确定性门红）");
        }
        if srep.partial_miss_probes == 0 {
            fail("C13 SVT 部分驻留臂零 miss 探针（反馈链空转冒充即红,fail-closed）");
        }
        if !srep.partial_req_bitexact || !srep.partial_out_bitexact {
            fail("C13 SVT 部分驻留臂 device vs host 非位级一致（SVT-2 请求编码/fallback 链缺陷即红）");
        }
        if !srep.closed_loop_all_hit || !srep.closed_loop_bitexact_vs_full {
            fail("C13 SVT 闭环重跑未全 hit 或输出 ≠ 全驻留臂（请求-驻留闭环缺陷即红）");
        }
        let spv_words = spv_inject_no_contraction(&load_spv(&spv_svt));
        svt_spv_bytes = spv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        eprintln!(
            "{GTAG}: C13 SVT 接线 pages={} pool_tiles={}（全驻留={}） probes={}（边界 {}） full_p100={:.6e}（位级={} 双跑={}） partial_miss={} 闭环 loaded={} evicted={} io={}B eval_ms={:.3}",
            sassets.tile_set.page_total(),
            sassets.pool_tiles,
            sassets.full_residency,
            srep.probe_count,
            srep.boundary_probe_count,
            srep.full_p100_vs_direct,
            srep.full_bitexact_vs_direct,
            srep.full_double_run_bitexact,
            srep.partial_miss_probes,
            srep.closed_loop_loaded,
            srep.closed_loop_evicted,
            srep.closed_loop_io_bytes,
            srep.eval_ms,
        );
        svt_report = Some((sassets, srep));
    }
    let eps = scene_eps(&scene.positions);
    eprintln!(
        "{GTAG}: 装配 scene={scene_id} tris={} quads={} points={} output={out_w}x{out_h} eps={eps:.6} auto_move={:?}",
        scene.tri_count,
        scene.quads.len(),
        scene.points.len(),
        auto_move.as_deref(),
    );

    // ④ 真窗口 present 会话先于车道创建(channel_order 决定编码参数 bgra 位;
    //    headless-smoke = 无窗口退化,仅供自检逻辑不计真门)。
    let mut window: Option<vk::ExternalImagePresent> = if headless {
        None
    } else {
        match vk::ExternalImagePresent::create(
            out_w,
            out_h,
            "rurix g31 game loop (bistro-interior 1080p;WASD/QE+mouse 视角,-/= 曝光,ESC 退出)",
            !hidden,
        ) {
            Ok(w) => Some(w),
            Err(e) => g31_dev_env_or_fail("window_present", &e),
        }
    };
    let bgra = window
        .as_ref()
        .map(|w| w.channel_order() == "bgra8_unorm")
        .unwrap_or(true);
    if let Some(w) = window.as_ref() {
        eprintln!(
            "{GTAG}: 窗口就绪 {}x{} channel_order={} visible={}",
            w.extent().0,
            w.extent().1,
            w.channel_order(),
            !hidden
        );
    }

    // ⑤ 游戏循环初态(相机 = 契约位姿;曝光 = 契约 ev100;extent 无关资源一次打包)。
    let mut assets = lane_assets(&scene, pre.in_w, pre.in_h);
    // D2：法线侧表字节面（extent 无关,era 循环外一次打包;off = 空 vec 零消费;
    // on = 9 f32/tri 与装配三角数互核 fail-closed——cluster/wp 重建面 CLI 已裁）。
    let nrm_bytes: Vec<u8> = if smooth_nrm {
        let b = bytes_f32(&nrm_sink);
        if b.len() != scene.tri_count * 9 * 4 {
            fail(&format!(
                "D2 法线侧表长度 {} ≠ tri_count×9×4 = {}（装配/施加点互核 fail-closed）",
                b.len(),
                scene.tri_count * 9 * 4
            ));
        }
        b
    } else {
        Vec::new()
    };
    // D6：MR 侧表字节面（extent 无关,era 循环外一次打包;--ggx on = 2 f32/tri
    // 真表与装配三角数互核 fail-closed;off 但 smooth-normals on = 8B 零哑表
    // ——kernel params[48]=0 门不读;!smooth_normals = 空 vec 零消费）。
    let mr_bytes: Vec<u8> = if smooth_nrm && ggx {
        let b = bytes_f32(&mr_sink);
        if b.len() != scene.tri_count * 2 * 4 {
            fail(&format!(
                "D6 MR 侧表长度 {} ≠ tri_count×2×4 = {}（装配/施加点互核 fail-closed）",
                b.len(),
                scene.tri_count * 2 * 4
            ));
        }
        b
    } else if smooth_nrm {
        G31_TRI_MR_DUMMY.to_vec()
    } else {
        Vec::new()
    };
    // day_0829 臂①：tri_base 侧表字节面（extent 无关,era 循环外一次打包;
    // --metal-f0 on = 3 f32/tri 未衰减 baseColor 真表〔tritex 真表互核〕;
    // 其余 realism 臂 on = 12B 零哑表〔kernel params[55]=0 门地址钳 0 保底读,
    // tri_mr 哑表同律〕;realism 全 off = 空 vec 零消费〔desc None〕）。
    // G37 W2 transparency:transp on 同样须真表——穿透段 tint 读 tri_base
    // [prim×3] 无门〔透射色调 = 未衰减 baseColorFactor,mats 均值面 ×(1−metal)
    // ×灰贴图均值双重衰减不可用〕;--textures 已裁 ⇒ tex_report 必在。
    let tri_base_bytes: Vec<u8> = if metal_f0 || transparency {
        let Some((tassets, _)) = tex_report.as_ref() else {
            fail("臂①/臂⑦ --metal-f0/--transparency 须 B4 纹理报告在案（tritex 真表互核面,fail-closed）");
        };
        let v = g31_assemble_tri_base(
            &contract.raw,
            scene_id,
            Path::new(&gltf_path),
            &scene.tri_mat,
            &tassets.tritex_bytes,
        )
        .unwrap_or_else(|e| fail(&format!("臂①/臂⑦ tri_base 装配: {e}")));
        let b = bytes_f32(&v);
        if b.len() != scene.tri_count * 3 * 4 {
            fail(&format!(
                "臂①/臂⑦ tri_base 侧表长度 {} ≠ tri_count×3×4 = {}（装配/施加点互核 fail-closed）",
                b.len(),
                scene.tri_count * 3 * 4
            ));
        }
        eprintln!(
            "{GTAG}: 臂①/臂⑦ tri_base 侧表 {} tris（{}B,未衰减 baseColor——F0 修伤面/透射色调面）",
            scene.tri_count,
            b.len()
        );
        b
    } else if realism_any {
        vec![0u8; 12]
    } else {
        Vec::new()
    };
    // day_0829 realism：triem 回退真表（realism on && em off 面 = tri_count×
    // (-1.0)——kernel 签名序 triem 恒在 tri_base 前,槽号 <0 = mats 均值面
    // 逐字回退语义;em on 面零消费,off+off 面零消费）。
    let triem_neg_bytes: Vec<u8> = if realism_any && em_assets.is_none() {
        bytes_f32(&vec![-1.0f32; scene.tri_count])
    } else {
        Vec::new()
    };
    // day_0829 臂④：trinm/tri_tan 侧表字节面（extent 无关,era 外一次打包;
    // on = 1 f32/tri 槽号表 + 4 f32/tri 切线表〔UV 导数法,长度互核〕;off =
    // 空 vec 零消费〔desc None〕）。
    let (trinm_bytes_nm, tri_tan_bytes): (Vec<u8>, Vec<u8>) = if normal_maps {
        if trinm_vec.len() != scene.tri_count {
            fail(&format!(
                "臂④ trinm 侧表长度 {} ≠ tri_count = {}（装配/施加点互核 fail-closed）",
                trinm_vec.len(),
                scene.tri_count
            ));
        }
        let tt = g31_assemble_tri_tan(&scene, &tri_uv)
            .unwrap_or_else(|e| fail(&format!("臂④ 切线装配: {e}")));
        if tt.len() != scene.tri_count * 4 {
            fail(&format!(
                "臂④ tri_tan 侧表长度 {} ≠ tri_count×4 = {}（互核 fail-closed）",
                tt.len(),
                scene.tri_count * 4
            ));
        }
        eprintln!(
            "{GTAG}: day_0829 臂④ 切线侧表 {} tris（UV 导数法,{}B）",
            scene.tri_count,
            tt.len() * 4
        );
        (bytes_f32(&trinm_vec), bytes_f32(&tt))
    } else {
        (Vec::new(), Vec::new())
    };
    // G37 W2 臂⑦：tri_transp 侧表字节面（extent 无关,era 外一次打包;on =
    // 1 f32/tri 透射率真表〔判定规则/命中登记见 g31_assemble_tri_transp〕;
    // off = 空 vec 零消费〔desc None〕）。
    let tri_transp_bytes: Vec<u8> = if transparency {
        let (v, hits) = g31_assemble_tri_transp(Path::new(&gltf_path), &scene.tri_mat, transp_alpha_v)
            .unwrap_or_else(|e| fail(&format!("臂⑦ tri_transp 装配: {e}")));
        if v.len() != scene.tri_count {
            fail(&format!(
                "臂⑦ tri_transp 侧表长度 {} ≠ tri_count = {}（装配/施加点互核 fail-closed）",
                v.len(),
                scene.tri_count
            ));
        }
        for (mi, name, tris) in &hits {
            eprintln!(
                "{GTAG}: G37 W2 臂⑦ 透明材质 mat{mi} {name}（{tris} tris,透射率 {transp_alpha_v}——判定 = alphaMode==BLEND || baseColor.a<1）"
            );
        }
        bytes_f32(&v)
    } else {
        Vec::new()
    };
    // G37 W2 臂⑦：transp on 而 --normal-maps off 面的 trinm 回退真表 +
    // tri_tan 零哑表（_transp SPV 签名含法线两路,须占位保持绑定序——trinm
    // [prim] 为 kernel 无门保底读 ⇒ 须 tri_count 全尺寸 -1.0〔triem 回退表
    // 同律,槽号 <0 = 零采样〕;tri_tan 只在 nm while 门内读 ⇒ 16B 零哑表
    // 〔tri_mr 哑表同律〕;nm on 面/transp off 面零消费。G37 W2 ris_nee:
    // ris|nee on 面同用本回退对——_ris SPV 为 transp 超集,条件并入）。
    let (trinm_fb_bytes, tri_tan_dummy): (Vec<u8>, Vec<u8>) =
        if (transparency || gi2_ris || gi2_nee) && !normal_maps {
            (bytes_f32(&vec![-1.0f32; scene.tri_count]), vec![0u8; 16])
        } else {
            (Vec::new(), Vec::new())
        };
    // G37 W2 臂⑧:ris|nee on 而 transparency off 面的 tri_transp 零表
    // (_ris SPV 签名含 tri_transp,阴影重走段 [prim] 无门保底读 ⇒ 须
    // tri_count 全尺寸 0.0〔= 不透明,kernel tp_gate=0 双保险〕)。
    let tri_transp_zero_bytes: Vec<u8> = if (gi2_ris || gi2_nee) && !transparency {
        bytes_f32(&vec![0.0f32; scene.tri_count])
    } else {
        Vec::new()
    };
    // G37 W2 臂⑧:lamp_tbl 字节面(extent 无关,era 外一次构建;nee on =
    // 灯片表 + 功率 CDF 真表〔g37_w2/g31_ris_lamps.rs 单源,确定性双构建〕;
    // ris on 而 nee off = 80B 零哑表〔header Q=0,kernel 保底读域〕;两臂
    // off = 空 vec 零消费〔desc None〕)。前置:points 非空(kernel 候选读
    // 保底;bistro 契约恒 4 盏)。
    let lamp_tbl_bytes: Vec<u8> = if gi2_nee {
        if scene.points.is_empty() {
            fail("臂⑧ --gi2-nee 须场景 points 非空(kernel 候选读保底,fail-closed)");
        }
        let (v, st) = g31_ris_lamps::build_lamp_table(
            &scene.positions,
            &scene.indices,
            &scene.emission,
            &scene.tri_mat,
            SLAB_TRI_NONE,
        )
        .unwrap_or_else(|e| fail(&format!("臂⑧ lamp_tbl 装配: {e}")));
        eprintln!(
            "{GTAG}: G37 W2 臂⑧ 灯片表 {} 片(零面积 {}/pdf 下溢 {},总功率 {:.3},{} f32 = {} B)",
            st.emissive_tris,
            st.zero_area_tris,
            st.pdf_underflow_tris,
            st.total_power,
            st.table_f32_len,
            st.table_f32_len * 4
        );
        bytes_f32(&v)
    } else if gi2_ris {
        if scene.points.is_empty() {
            fail("臂⑧ --gi2-ris 须场景 points 非空(kernel 候选读保底,fail-closed)");
        }
        vec![0u8; g31_ris_lamps::G31_RIS_LAMP_DUMMY_BYTES]
    } else {
        Vec::new()
    };
    // day_0829 realism：params buffer 扩容（任一 realism 臂 on ⇒ 逐帧上传
    // G31_REAL_PARAMS_LEN f32,buffer 同门扩容;off = PARAMS_LEN 既有面 0-byte）。
    if realism_any {
        assets.params0_bytes = vec![0u8; G31_REAL_PARAMS_LEN * 4];
    }
    let cam0 = G31Camera::from_spec(&scene.camera);
    let mut cam = cam0;
    let mut ev100 = f64::from(scene.ev100);
    let jitter_base = (seed % JITTER_WINDOW_MOD) as u32;

    // ⑥ era 循环(era = 一个 extent 生命周期;resize → 车道按新 extent 重建,
    //    TSR 历史 reset;最小化跳过不消费帧预算;ESC/close 干净退出)。
    let total = warmup + frames;
    let mut fi = 0u32;
    let mut exit_reason = "frames_done";
    let mut resize_eras = 0u32;
    let mut render_ms: Vec<f64> = Vec::new();
    let mut present_ms: Vec<f64> = Vec::new();
    // C7 profiler 收集面（--profile-json on 才消费;post-warmup 与 render_ms 同窗;
    // debug label 活跃态随 era 车道重建刷新;Option 面——era 循环必赋值,终读 unwrap_or）。
    let mut profile_frames: Vec<G31ProfileFrame> = Vec::new();
    let mut debug_labels_active: Option<bool> = None;
    // 初值占位读（era 循环首迭代即覆写,终读 unwrap_or——unused_assignments 静默面）。
    let _ = debug_labels_active;
    let mut digest_ms: Vec<f64> = Vec::new();
    let mut encode_gpu_ms: Vec<f64> = Vec::new();
    // D3 bloom 四 pass GPU 合计收集面（bloom on 才消费;off 空 vec 零消费）。
    let mut bloom_gpu_ms: Vec<f64> = Vec::new();
    // A2 autoexp 两 pass GPU 合计收集面（on 才消费;off 空 vec 零消费）。
    let mut autoexp_gpu_ms: Vec<f64> = Vec::new();
    // A2 验证面:逐帧 presented 亮度序列（--present-luma-out 才消费;
    // (帧号, 8bit 归一 Rec.709 luma 均值)）。
    let mut luma_seq: Vec<(u32, f64)> = Vec::new();
    let mut fg_gpu_ms: Vec<f64> = Vec::new();
    let mut render5_gpu_ms: Vec<f64> = Vec::new();
    let mut digest_seq: Vec<String> = Vec::new();
    let mut ev100_seq: Vec<f64> = Vec::new();
    let mut pose_seq: Vec<[f64; 5]> = Vec::new();
    let mut render_digest = String::new();
    let mut presented_digest = String::new();
    let mut prev_keys = [0u64; 4];
    let mut last_frame_wall: Option<std::time::Instant> = None;
    // C4 窗口风暴驱动态（--window-storm/--storm-soak 臂;默认关 = 全零零消费）。
    let mut storm_resize_ops: u64 = 0;
    let mut storm_min_cycles: u64 = 0;
    let mut storm_min_skips: u64 = 0;
    let mut storm_restore_pending = false;
    let mut storm_burst_done = false;
    // 最后一次故障注入帧号:resize toggle/最小化触发后 fi 不推进（era 重建
    // break / 最小化 continue 均回到同帧）,同帧守卫防重复触发死循环。
    let mut storm_last_fault_fi: u32 = u32::MAX;
    // A5 双口径账目(post-warmup 测量窗;real/presented 类型面分离,生成帧禁入
    // real 计数) + 接线态对拍结果(probe 帧一次)。
    let mut real_frames: u64 = 0;
    let mut generated_frames: u64 = 0;
    let mut presented_frames: u64 = 0;
    let mut real_render_seconds: f64 = 0.0;
    let mut present_seconds: f64 = 0.0;
    let mut wired_parity: Option<(G31WiredParity, u32)> = None;
    // B1 HZB 决策/调度记账面（hzb on 才消费;计数 = 全帧〔含 warmup——闭环正确性
    // 证据面〕,ms 序列 = post-warmup〔测量口径,与 real_render 同窗〕）。
    let mut hzb_tested: u64 = 0;
    let mut hzb_occluded: u64 = 0;
    let mut hzb_offscreen: u64 = 0;
    let mut hzb_retested: u64 = 0;
    let mut hzb_flipped: u64 = 0;
    let mut hzb_visible_sum: u64 = 0;
    let mut hzb_closure_frames: u64 = 0;
    let mut hzb_closure_submits: u64 = 0;
    let mut hzb_fallbacks: u64 = 0;
    let mut hzb_gpu_ms: Vec<f64> = Vec::new();
    let mut hzb_scene_gpu_ms: Vec<f64> = Vec::new();
    let mut hzb_host_ms: Vec<f64> = Vec::new();
    let mut hzb_closure_gpu_ms: Vec<f64> = Vec::new();
    /// B1 probe 预备帧（probe_fi−1）回读暂存:（深度, 平铺金字塔）。
    let mut hzb_pre_data: Option<(Vec<f32>, Vec<f32>)> = None;
    let mut hzb_wired_parity: Option<(G31HzbWiredParity, u32)> = None;
    /// B1 mip 拓扑元信息（probe 对拍消费;era 创建期刷新）。
    let mut hzb_levels_meta: Vec<(u32, u32)> = Vec::new();
    let mut hzb_flat_offsets_meta: Vec<u32> = Vec::new();
    // A5 probe 帧号:post-warmup 首生成帧(warmup=0 时 fi=1,首帧无 prev 不可探)。
    let probe_fi = if fg != G31Fg::Off || hzb == G31Hzb::On {
        Some(warmup.max(1))
    } else {
        None
    };
    // B1 evidence 元信息面（hzb on 时代理;跨 era 重建时刷新——CI 腿 resize_eras=0）。
    let mut hzb_meta_json = String::new();
    // C13 SVT 流送状态面（svt on 才消费;svt off = None/空件零消费,既有面 0-byte）:
    // 流送状态机（页表 host 影 + LRU 池 + 瓦片集"盘"面）+ host 池影（era 重建
    // 再同步源 + 瓦片上传应用面）+ era 首态克隆对（descs 借用面——era 内不变,
    // 与逐帧推进的活状态分离避借用冲突）+ 逐帧流送统计（evidence 面）。
    let mut svt_stream: Option<svt::SvtStreaming> = None;
    let mut svt_pool_image: Vec<u8> = Vec::new();
    let mut svt_era_pt: Vec<u8> = Vec::new();
    let mut svt_era_pool: Vec<u8> = Vec::new();
    let mut svt_stats = G31SvtStats::default();
    // G31+ #58 逐帧 cut 统计收集面（--cluster-lod leaf|on 才消费;off 空 vec
    // 零消费。每 16 帧对 cut 做覆盖性机核采样,fail-closed）。
    let mut cluster_frame_stats: Vec<ClusterFrameStat> = Vec::new();
    let mut cluster_stat_ms_total = 0.0f64;
    // G37 W2 #74/#111 visbuffer 相机样本采集面（--visbuffer on 才消费;off 空
    // vec 零消费。device 链循环后跑——不污染 real_render_frame_ms 口径）。
    let visbuffer_sample_set: Vec<u32> = if visbuffer_opt.enabled {
        visbuffer_sample_frames(total, visbuffer_opt.samples)
    } else {
        Vec::new()
    };
    let mut visbuffer_samples_taken: Vec<VisBufferCamSample> = Vec::new();
    // G37 W3 frame_cut 合入:真窗口逐帧相机样本采集面（--cluster-per-frame-cut
    // on 才消费;off 空 vec 零消费。device 链循环后跑——不污染
    // real_render_frame_ms）。
    let mut frame_cut_samples_taken: Vec<FrameCutCamSample> = Vec::new();
    // G31+ #95/#99 逐帧 WP/HLOD 统计收集面（--wp-hlod full|on 才消费;off 空
    // vec 零消费。逐帧 tick 流送 + 互斥选层 + warmup 原子翻转状态机——#99
    // popping 指标事实源）。
    let mut wp_frame_stats: Vec<WpFrameStat> = Vec::new();
    let mut wp_stat_ms_total = 0.0f64;
    if svt_on {
        let Some((sassets, _)) = svt_report.as_ref() else {
            fail("C13 SVT 报告缺失（流送初态面不完整判红）");
        };
        let stream = match g31_svt_streaming_init(sassets) {
            Ok(s) => s,
            Err(e) => fail(&format!("C13 SVT 流送初态: {e}")),
        };
        svt_pool_image = if sassets.full_residency {
            sassets.tile_set.payloads_bytes()
        } else {
            vec![0u8; sassets.pool_tiles as usize * svt::SVT_PHYS_TILE_BYTES]
        };
        svt_stream = Some(stream);
    }
    // G37 W2 合入:PSO 变体账本(#82/#113;era0 = precache 面/era≥1 = 守护面。
    // 窗口管线全在 session 构造期创建,运行期唯一新建点 = era 重建——账本把
    // 该事实变成受门保护的断言,验收 pso_runtime_creates == 0;strict 臂
    // RURIX_G31_PSO_STRICT=1 miss 即 fail,默认告警不断跑)。
    let mut pso_ledger = g31_pso_warmup::G31PsoLedger::new();
    let pso_strict = std::env::var("RURIX_G31_PSO_STRICT").is_ok_and(|v| v == "1");
    'eras: loop {
        let (ew, eh) = window
            .as_ref()
            .map(|w| w.extent())
            .unwrap_or((out_w, out_h));
        let in_w = ((ew as u64 * u64::from(tier)) / 100).max(1) as u32;
        let in_h = ((eh as u64 * u64::from(tier)) / 100).max(1) as u32;
        // extent 联动资源尺寸(internal = floor(输出×tier%),双向 floor 同口径)。
        assets.out_color_size = (in_w as u64) * (in_h as u64) * 12;
        assets.out_depth_size = (in_w as u64) * (in_h as u64) * 4;
        let bits = UnifiedLaneBits::load(
            &spv_scene,
            &spv_mv,
            &spv_resample,
            &spv_resolve,
            in_w,
            in_h,
            ew,
            eh,
            false,
        );
        let enc_words = load_spv(&spv_encode);
        let (ex, ey, _) = spv_local_size(&enc_words);
        let enc_dispatch = [ew.div_ceil(ex), eh.div_ceil(ey), 1];
        let enc_spv_bytes: Vec<u8> = enc_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mut enc_params = aces13_device_encode_params_ex(ew, eh, bgra, dither);
        // G37 W2 合入:--lut 非 off 时 encode 参数尾挂 LUT（[134] 门/[135] dim/
        // [136..136+3N³) 表体;lut_asset 为 era 不变量,buffer 尺寸随
        // enc_params_bytes.len() 自动变长,resize 随车道重建自然重挂;off =
        // 既有 136 f32 参数面 0-byte）。
        if let Some(l) = lut_asset.as_ref() {
            g31_lut_assets::extend_encode_params(&mut enc_params, l);
        }
        let enc_params_bytes = bytes_f32(&enc_params);
        // D3 bloom era 常量面（bloom on 才装配;off = 空件/None 零消费,既有面
        // 0-byte）:三 kernel SPV + dispatch 自各自 SPV LocalSize 派生（SPV 单一
        // 事实源同律;bright/blur 作用于半分辨率 ceil(ew/2)×ceil(eh/2),
        // composite 作用于全分辨率）+ 四份静态参数（extent 联动,resize 随车道
        // 重建）。
        let (bloom_bright_words, bloom_blur_words, bloom_comp_words) = if bloom {
            (
                load_spv(spv_bloom_bright.as_deref().unwrap_or(G31_DEFAULT_SPV_BLOOM_BRIGHT)),
                load_spv(spv_bloom_blur.as_deref().unwrap_or(G31_DEFAULT_SPV_BLOOM_BLUR)),
                load_spv(
                    spv_bloom_composite
                        .as_deref()
                        .unwrap_or(G31_DEFAULT_SPV_BLOOM_COMPOSITE),
                ),
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };
        let bloom_bright_spv_bytes: Vec<u8> = bloom_bright_words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let bloom_blur_spv_bytes: Vec<u8> = bloom_blur_words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let bloom_comp_spv_bytes: Vec<u8> = bloom_comp_words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let bloom_half_w = ew.div_ceil(2);
        let bloom_half_h = eh.div_ceil(2);
        let bloom_bright_params_bytes =
            bytes_f32(&g31_bloom_pack_bright_params(ew, eh, bloom_threshold_v, G31_BLOOM_KNEE));
        let bloom_blur_h_params_bytes =
            bytes_f32(&g31_bloom_pack_blur_params(bloom_half_w, bloom_half_h, 0.0));
        let bloom_blur_v_params_bytes =
            bytes_f32(&g31_bloom_pack_blur_params(bloom_half_w, bloom_half_h, 1.0));
        let bloom_comp_params_bytes = bytes_f32(&g31_bloom_pack_composite_params(
            ew,
            eh,
            bloom_strength_v,
            bloom_half_w,
            bloom_half_h,
        ));
        let bloom_assets = if bloom {
            let (bbx, bby, _) = spv_local_size(&bloom_bright_words);
            let (blx, bly, _) = spv_local_size(&bloom_blur_words);
            let (bcx, bcy, _) = spv_local_size(&bloom_comp_words);
            Some(G31BloomAssets {
                spv_bright: &bloom_bright_spv_bytes,
                spv_blur: &bloom_blur_spv_bytes,
                spv_composite: &bloom_comp_spv_bytes,
                dispatch_bright: [bloom_half_w.div_ceil(bbx), bloom_half_h.div_ceil(bby), 1],
                dispatch_blur: [bloom_half_w.div_ceil(blx), bloom_half_h.div_ceil(bly), 1],
                dispatch_composite: [ew.div_ceil(bcx), eh.div_ceil(bcy), 1],
                bright_params_bytes: &bloom_bright_params_bytes,
                blur_h_params_bytes: &bloom_blur_h_params_bytes,
                blur_v_params_bytes: &bloom_blur_v_params_bytes,
                comp_params_bytes: &bloom_comp_params_bytes,
            })
        } else {
            None
        };
        // A2 自动曝光 era 常量面（on 才装配;off = 空件/None 零消费,既有面
        // 0-byte）:两 kernel SPV（LocalSize fail-closed 复核——reduce 须
        // (256,1,1)/state 须 (1,1,1),单 workgroup dispatch [1,1,1] 归约语义
        // 前提）+ 参数（pixel_count = 输出 extent w·h,encode in_color 消费
        // 同域;extent 联动 resize 随车道重建）。
        let (ae_reduce_words, ae_state_words) = if autoexp {
            (
                load_spv(spv_ae_reduce.as_deref().unwrap_or(G31_DEFAULT_SPV_AE_REDUCE)),
                load_spv(spv_ae_state.as_deref().unwrap_or(G31_DEFAULT_SPV_AE_STATE)),
            )
        } else {
            (Vec::new(), Vec::new())
        };
        if autoexp {
            let rl = spv_local_size(&ae_reduce_words);
            if rl != (256, 1, 1) {
                fail(&format!(
                    "A2 reduce SPV LocalSize {rl:?} ≠ (256,1,1)（单 workgroup 归约前提,fail-closed）"
                ));
            }
            let sl = spv_local_size(&ae_state_words);
            if sl != (1, 1, 1) {
                fail(&format!(
                    "A2 state SPV LocalSize {sl:?} ≠ (1,1,1)（单线程串行前提,fail-closed）"
                ));
            }
        }
        let ae_reduce_spv_bytes: Vec<u8> = ae_reduce_words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let ae_state_spv_bytes: Vec<u8> = ae_state_words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let ae_params_bytes = bytes_f32(&g31_ae_pack_params(
            ew * eh,
            ae_key_v,
            ae_rate_v,
            ae_min_v,
            ae_max_v,
        ));
        let ae_assets = if autoexp {
            Some(G31AutoExpAssets {
                spv_reduce: &ae_reduce_spv_bytes,
                spv_state: &ae_state_spv_bytes,
                params_bytes: &ae_params_bytes,
            })
        } else {
            None
        };
        // A5 FG era 常量面(fg on 才装配;fg off = None,车道 0-byte 现状):
        // kernel SPV 两件(g26_framegen + g31_mv_negate glue) + dispatch =
        // [ew·eh,1,1]/[2·ew·eh,1,1](LocalSize 1,1,1 单像素/单元素线程同律) +
        // 逐 gen 静态参数(t = t_temporal = i/(n+1) 直通,host mfg_between 同式
        // f32 位级;inv_sigma2 = 1/(σ·σ) 金标准默认 σ=0.1 同式预算——
        // FrameGenParams::default() 单一事实源)。
        let (fg_words, mvn_words) = if fg != G31Fg::Off {
            (load_spv(&spv_framegen), load_spv(&spv_mvn))
        } else {
            (Vec::new(), Vec::new())
        };
        let fg_spv_bytes: Vec<u8> = fg_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mvn_spv_bytes: Vec<u8> = mvn_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let fg_sigma = FrameGenParams::default().consistency_sigma;
        let fg_inv_sigma2 = 1.0f32 / (fg_sigma * fg_sigma);
        let fg_n = fg.inserted();
        let mvn_params = g31_mvn_pack_params(2 * ew * eh);
        let mvn_params_bytes = bytes_f32(&mvn_params);
        let fg_params1 = g31_fg_pack_params(ew * eh, ew, eh, 1.0f32 / (fg_n + 1) as f32, fg_inv_sigma2);
        let fg_params1_bytes = bytes_f32(&fg_params1);
        let fg_params2 = if fg == G31Fg::X3 {
            g31_fg_pack_params(ew * eh, ew, eh, 2.0f32 / (fg_n + 1) as f32, fg_inv_sigma2)
        } else {
            Vec::new()
        };
        let fg_params2_bytes = bytes_f32(&fg_params2);
        let fg_assets = if fg != G31Fg::Off {
            Some(G31FgAssets {
                mode: fg,
                spv_bytes: &fg_spv_bytes,
                mvn_spv_bytes: &mvn_spv_bytes,
                dispatch: [ew * eh, 1, 1],
                mvn_dispatch: [2 * ew * eh, 1, 1],
                mvn_params_bytes: &mvn_params_bytes,
                params1_bytes: &fg_params1_bytes,
                params2_bytes: &fg_params2_bytes,
            })
        } else {
            None
        };
        // B1 HZB era 常量面(hzb on 才装配;off = None/空件零消费,既有面 0-byte):
        // kernel SPV 五件(g31_hzb_{primary,shade,pack} 加性件 + g27_hzb_{reduce,
        // test} G27 M-a 本体 0-byte 冻结消费) + mip 拓扑/静态参数/平铺初值(全 1.0
        // = standard-Z 最远 ⇒ 首帧前全 Visible)/inst_base 逐实例前缀和。
        let hzb_bits = if hzb == G31Hzb::On {
            Some(G31HzbBits::load(
                &spv_hzb_primary,
                &spv_hzb_shade,
                &spv_hzb_reduce,
                &spv_hzb_test,
                &spv_hzb_pack,
                in_w,
                in_h,
                &hzb_groups,
            ))
        } else {
            None
        };
        // B1 逐节点 BLAS 引用 + 创建期实例表(hzb on 面;节点段 = 装配序连续段,
        // 与单 BLAS 面位级同 buffer;实例 mask 初值全 0xFF,逐帧掩码经
        // tlas_update 写槽位)。
        let hzb_blas_refs: Vec<&[f32]> = hzb_groups
            .iter()
            .map(|g| {
                let lo = g.tri_offset as usize * 9;
                let hi = lo + g.tri_count as usize * 9;
                &assets.tris[lo..hi]
            })
            .collect();
        let hzb_insts: Vec<RayQueryInstanceDesc> = (0..hzb_groups.len() as u32)
            .map(|i| RayQueryInstanceDesc {
                blas: i,
                custom_index: i,
                mask: 0xFF,
                sbt_record_offset: 0,
            })
            .collect();
        let blas_refs: [&[f32]; 1] = [&assets.tris];
        // B1 HZB 车道描述组(body 级所有者;session 借用面);off 面 descs 同级
        // 托底（条件建造,借用序 = 所有者先于借用者）。
        let hzb_descs = hzb_bits.as_ref().map(|hb| {
            let (r, p, b, rb, ids) = g31_lane_descs_hzb(
                &assets,
                &bits,
                &enc_spv_bytes,
                enc_dispatch,
                &enc_params_bytes,
                hb,
                hzb_groups.len(),
                in_w,
                in_h,
                ew,
                eh,
            );
            (r, p, b, rb, ids)
        });
        let hzb_bar_refs: Vec<&[(u32, TargetState)]> = hzb_descs
            .as_ref()
            .map(|(_, _, b, _, _)| b.iter().map(|x| x.as_slice()).collect())
            .unwrap_or_default();
        // G37 W3 fg_combo：fg 资产两点式分流——full 面（fg×textures 仅经
        // --quality full 预设可达,CLI 卫兵已裁散臂）归 g31_apply_fg_full 施加
        // 于 tex_descs 终态,base 面维持 g31_lane_descs 内嵌现状（0-byte）。
        let (fg_assets_base, fg_assets_full) = if textures {
            (None, fg_assets)
        } else {
            (fg_assets, None)
        };
        let mut off_descs = if hzb != G31Hzb::On {
            Some(g31_lane_descs(
                &assets,
                &bits,
                &enc_spv_bytes,
                enc_dispatch,
                &enc_params_bytes,
                in_w,
                in_h,
                ew,
                eh,
                fg_assets_base,
            ))
        } else {
            None
        };
        // D3 bloom 变体描述组（bloom on 且 textures off 面;tex×bloom 组合归
        // tex_descs 族承载〔day_0828 Phase B〕,off_descs 面同级托底不消费,
        // 借用序 = 所有者先于借用者）。
        let mut bloom_descs = if bloom && !textures {
            Some(g31_lane_descs_bloom(
                &assets,
                &bits,
                &enc_spv_bytes,
                enc_dispatch,
                &enc_params_bytes,
                bloom_assets.as_ref().unwrap(),
                in_w,
                in_h,
                ew,
                eh,
            ))
        } else {
            None
        };
        // D2 平滑法线变体描述组（smooth-normals on 且 textures off 面;
        // (smooth && textures) 合流臂归 tex_descs 族承载〔day_0828 Phase B〕;
        // 与 bloom 组合面 = g31_lane_descs_nrm_bloom（trinrm 挂 32,bloom
        // 下标面 0-byte）;单臂面 = g31_lane_descs_nrm（trinrm 挂 24）。
        let mut nrm_descs = if smooth_nrm && !textures {
            Some(if bloom {
                g31_lane_descs_nrm_bloom(
                    &assets,
                    &bits,
                    &nrm_bytes,
                    &mr_bytes,
                    &enc_spv_bytes,
                    enc_dispatch,
                    &enc_params_bytes,
                    bloom_assets.as_ref().unwrap(),
                    in_w,
                    in_h,
                    ew,
                    eh,
                )
            } else {
                g31_lane_descs_nrm(
                    &assets,
                    &bits,
                    &nrm_bytes,
                    &mr_bytes,
                    &enc_spv_bytes,
                    enc_dispatch,
                    &enc_params_bytes,
                    in_w,
                    in_h,
                    ew,
                    eh,
                )
            })
        } else {
            None
        };
        // B4/Phase B 纹理变体描述组族（textures on 面;闭集互斥 ⇒ hzb/fg/slab
        // 恒 off。day_0828 Phase B 四形态：tex 单臂（既有 g31_lane_descs_tex）
        // / tex+nrm（合体 kernel,trinrm 29/tri_mr 30）/ tex+bloom（tex 五件
        // 尾挂 32..36）/ tex+nrm+bloom（+trinrm 37/tri_mr 38）——scene SPV =
        // tex_spv_bytes（合流面 = g31_texture_nrm_gi,CLI 换载已就位）。
        let mut tex_descs = if textures {
            let Some((tassets, _)) = tex_report.as_ref() else {
                fail("B4 纹理报告缺失（descs 面不完整判红）");
            };
            // day_0828 Phase F：em on = triem 侧表进 tex_nrm 系两形态（CLI 已
            // 裁 em ⇒ smooth_nrm && textures,其余两形态恒 None 0-byte）。
            // day_0829 realism：任一 realism 臂 on 且 em off ⇒ triem 换 -1
            // 回退真表（kernel 签名序保持;em on 面真表零触达）。
            let triem_ref = match (em_assets.as_ref(), realism_any) {
                (Some(e), _) => Some(e.triem_bytes.as_slice()),
                (None, true) => Some(triem_neg_bytes.as_slice()),
                (None, false) => None,
            };
            // day_0829 realism：tri_base（--metal-f0 on 面真表;其余 realism
            // 臂 on = 12B 零哑表;全 off = None 既有面逐字 0-byte）。
            let tri_base_ref = if realism_any {
                Some(tri_base_bytes.as_slice())
            } else {
                None
            };
            // day_0829 臂④：trinm/tri_tan（--normal-maps on 面真表;off =
            // None——SPV 链下位工件 15 buffer,多余绑定即 layout 失配。G37 W2:
            // transp on 而 nm off = 回退真表/哑表对——_transp SPV 签名含法线
            // 两路,须占位保持绑定序）。
            let nm_ref = if normal_maps {
                Some((trinm_bytes_nm.as_slice(), tri_tan_bytes.as_slice()))
            // G37 W2 ris_nee:ris|nee on 面同用回退对(条件并入)。
            } else if transparency || gi2_ris || gi2_nee {
                Some((trinm_fb_bytes.as_slice(), tri_tan_dummy.as_slice()))
            } else {
                None
            };
            // G37 W2 臂⑦：tri_transp（--transparency on 面真表;off = None
            // ——链下位工件无本绑定,多余绑定即 layout 失配）。
            let transp_ref = if transparency {
                Some(tri_transp_bytes.as_slice())
            } else if gi2_ris || gi2_nee {
                // G37 W2 ris_nee:零表占位保持 kernel 签名序。
                Some(tri_transp_zero_bytes.as_slice())
            } else {
                None
            };
            // G37 W2 臂⑧:lamp_tbl(nee 真表/ris 哑表/off None——链下位
            // 工件无本绑定,多余绑定即 layout 失配)。
            let ris_ref = if gi2_ris || gi2_nee {
                Some(lamp_tbl_bytes.as_slice())
            } else {
                None
            };
            Some(match (smooth_nrm, bloom) {
                (true, true) => g31_lane_descs_tex_nrm_bloom(
                    &assets,
                    &bits,
                    &enc_spv_bytes,
                    enc_dispatch,
                    &enc_params_bytes,
                    &tex_spv_bytes,
                    tassets,
                    &nrm_bytes,
                    &mr_bytes,
                    triem_ref,
                    tri_base_ref,
                    nm_ref,
                    transp_ref,
                    ris_ref,
                    bloom_assets.as_ref().unwrap(),
                    in_w,
                    in_h,
                    ew,
                    eh,
                ),
                (true, false) => g31_lane_descs_tex_nrm(
                    &assets,
                    &bits,
                    &enc_spv_bytes,
                    enc_dispatch,
                    &enc_params_bytes,
                    &tex_spv_bytes,
                    tassets,
                    &nrm_bytes,
                    &mr_bytes,
                    triem_ref,
                    tri_base_ref,
                    nm_ref,
                    transp_ref,
                    ris_ref,
                    in_w,
                    in_h,
                    ew,
                    eh,
                ),
                (false, true) => g31_lane_descs_tex_bloom(
                    &assets,
                    &bits,
                    &enc_spv_bytes,
                    enc_dispatch,
                    &enc_params_bytes,
                    &tex_spv_bytes,
                    tassets,
                    bloom_assets.as_ref().unwrap(),
                    in_w,
                    in_h,
                    ew,
                    eh,
                ),
                (false, false) => g31_lane_descs_tex(
                    &assets,
                    &bits,
                    &enc_spv_bytes,
                    enc_dispatch,
                    &enc_params_bytes,
                    &tex_spv_bytes,
                    tassets,
                    in_w,
                    in_h,
                    ew,
                    eh,
                ),
            })
        } else {
            None
        };
        // C13 SVT 变体描述组（svt on 面;era 首态克隆对 = 流送状态/host 池影
        // 当前值——era 重建再同步面;era 内克隆不变,与逐帧推进的活状态分离）。
        let svt_descs = if svt_on {
            let Some((tassets, _)) = tex_report.as_ref() else {
                fail("C13 SVT 须 B4 纹理报告在案（descs 面不完整判红）");
            };
            let Some((sassets, _)) = svt_report.as_ref() else {
                fail("C13 SVT 报告缺失（descs 面不完整判红）");
            };
            let Some(stream) = svt_stream.as_ref() else {
                fail("C13 SVT 流送状态缺失（descs 面不完整判红）");
            };
            svt_era_pt = stream.page_table_bytes();
            svt_era_pool = svt_pool_image.clone();
            Some(g31_lane_descs_svt(
                &assets,
                &bits,
                &enc_spv_bytes,
                enc_dispatch,
                &enc_params_bytes,
                &svt_spv_bytes,
                tassets,
                sassets,
                &svt_era_pt,
                &svt_era_pool,
                in_w,
                in_h,
                ew,
                eh,
            ))
        } else {
            None
        };
        // A2:对将被选中的变体描述组施加 autoexp 变换（encode 摘出重挂;施加
        // 目标与 descs 消费选择序同律 = tex 族（day_0828 Phase B 四形态）>
        // nrm（含 ×bloom 组合内嵌）> bloom > off;fg/hzb/svt/slab 组合 CLI
        // 已裁不达。变体族下标/屏障计划 = 静态常量,helper 内断言资源表连号）。
        if let Some(ae) = ae_assets.as_ref() {
            if textures {
                let d = tex_descs
                    .as_mut()
                    .unwrap_or_else(|| fail("A2: textures on 面 tex_descs 缺失（防御性复核）"));
                // day_0828 Phase F：em on = AE 三件顺延 +1（triem 尾挂后）——
                // tex_nrm 系两形态换 _EM 下标/计划;em off 逐字既有面。
                // day_0829 realism：任一 realism 臂 on = triem 恒 Some +
                // tri_base 尾挂 ⇒ AE 三件再顺延 +1（_REAL 族,guard 先于 em）。
                // G37 W2 transparency：tri_transp 尾挂（nm off 时法线回退表/
                // 哑表仍占位）⇒ AE 三件再顺延 +1（_TRANSP 族,guard 先于 nm/
                // realism——挂载序最尾即下标最高;g31_apply_autoexp assert
                // 连号拦错配）。
                match (smooth_nrm, bloom) {
                    // G37 W2 ris_nee:_RIS guard 最先(lamp_tbl 挂载序最尾
                    // 即下标最高;W1 assert 连号为错配保护网)。
                    (true, true) if gi2_ris || gi2_nee => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_BLOOM_RIS,
                        G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS,
                        G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS,
                        G31_U_BLOOM_COMP_OUT,
                        G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_RIS,
                        G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_RIS,
                    ),
                    (true, false) if gi2_ris || gi2_nee => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_RIS,
                        G31_U_AE_PARAMS_TEXNRM_RIS,
                        G31_U_AE_PARTIALS_TEXNRM_RIS,
                        U_OUT_COLOR[0],
                        G31_U_PLAN_AE_REDUCE_TEXNRM_RIS,
                        G31_U_PLAN_AE_STATE_TEXNRM_RIS,
                    ),
                    (true, true) if transparency => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_BLOOM_TRANSP,
                        G31_U_AE_PARAMS_TEXNRM_BLOOM_TRANSP,
                        G31_U_AE_PARTIALS_TEXNRM_BLOOM_TRANSP,
                        G31_U_BLOOM_COMP_OUT,
                        G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_TRANSP,
                        G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_TRANSP,
                    ),
                    (true, false) if transparency => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_TRANSP,
                        G31_U_AE_PARAMS_TEXNRM_TRANSP,
                        G31_U_AE_PARTIALS_TEXNRM_TRANSP,
                        U_OUT_COLOR[0],
                        G31_U_PLAN_AE_REDUCE_TEXNRM_TRANSP,
                        G31_U_PLAN_AE_STATE_TEXNRM_TRANSP,
                    ),
                    (true, true) if normal_maps => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_BLOOM_NM,
                        G31_U_AE_PARAMS_TEXNRM_BLOOM_NM,
                        G31_U_AE_PARTIALS_TEXNRM_BLOOM_NM,
                        G31_U_BLOOM_COMP_OUT,
                        G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_NM,
                        G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_NM,
                    ),
                    (true, false) if normal_maps => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_NM,
                        G31_U_AE_PARAMS_TEXNRM_NM,
                        G31_U_AE_PARTIALS_TEXNRM_NM,
                        U_OUT_COLOR[0],
                        G31_U_PLAN_AE_REDUCE_TEXNRM_NM,
                        G31_U_PLAN_AE_STATE_TEXNRM_NM,
                    ),
                    (true, true) if realism_any => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_BLOOM_REAL,
                        G31_U_AE_PARAMS_TEXNRM_BLOOM_REAL,
                        G31_U_AE_PARTIALS_TEXNRM_BLOOM_REAL,
                        G31_U_BLOOM_COMP_OUT,
                        G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_REAL,
                        G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_REAL,
                    ),
                    (true, false) if realism_any => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_REAL,
                        G31_U_AE_PARAMS_TEXNRM_REAL,
                        G31_U_AE_PARTIALS_TEXNRM_REAL,
                        U_OUT_COLOR[0],
                        G31_U_PLAN_AE_REDUCE_TEXNRM_REAL,
                        G31_U_PLAN_AE_STATE_TEXNRM_REAL,
                    ),
                    (true, true) if emissive_tex => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_BLOOM_EM,
                        G31_U_AE_PARAMS_TEXNRM_BLOOM_EM,
                        G31_U_AE_PARTIALS_TEXNRM_BLOOM_EM,
                        G31_U_BLOOM_COMP_OUT,
                        G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM_EM,
                        G31_U_PLAN_AE_STATE_TEXNRM_BLOOM_EM,
                    ),
                    (true, false) if emissive_tex => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_EM,
                        G31_U_AE_PARAMS_TEXNRM_EM,
                        G31_U_AE_PARTIALS_TEXNRM_EM,
                        U_OUT_COLOR[0],
                        G31_U_PLAN_AE_REDUCE_TEXNRM_EM,
                        G31_U_PLAN_AE_STATE_TEXNRM_EM,
                    ),
                    (true, true) => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM_BLOOM,
                        G31_U_AE_PARAMS_TEXNRM_BLOOM,
                        G31_U_AE_PARTIALS_TEXNRM_BLOOM,
                        G31_U_BLOOM_COMP_OUT,
                        G31_U_PLAN_AE_REDUCE_TEXNRM_BLOOM,
                        G31_U_PLAN_AE_STATE_TEXNRM_BLOOM,
                    ),
                    (true, false) => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEXNRM,
                        G31_U_AE_PARAMS_TEXNRM,
                        G31_U_AE_PARTIALS_TEXNRM,
                        U_OUT_COLOR[0],
                        G31_U_PLAN_AE_REDUCE_TEXNRM,
                        G31_U_PLAN_AE_STATE_TEXNRM,
                    ),
                    (false, true) => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEX_BLOOM,
                        G31_U_AE_PARAMS_TEX_BLOOM,
                        G31_U_AE_PARTIALS_TEX_BLOOM,
                        G31_U_BLOOM_COMP_OUT,
                        G31_U_PLAN_AE_REDUCE_TEX_BLOOM,
                        G31_U_PLAN_AE_STATE_TEX_BLOOM,
                    ),
                    (false, false) => g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_TEX,
                        G31_U_AE_PARAMS_TEX,
                        G31_U_AE_PARTIALS_TEX,
                        U_OUT_COLOR[0],
                        G31_U_PLAN_AE_REDUCE_TEX,
                        G31_U_PLAN_AE_STATE_TEX,
                    ),
                }
            } else if smooth_nrm {
                let d = nrm_descs
                    .as_mut()
                    .unwrap_or_else(|| fail("A2: smooth-normals on 面 nrm_descs 缺失（防御性复核）"));
                if bloom {
                    g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_NRM_BLOOM,
                        G31_U_AE_PARAMS_NRM_BLOOM,
                        G31_U_AE_PARTIALS_NRM_BLOOM,
                        G31_U_BLOOM_COMP_OUT,
                        G31_U_PLAN_AE_REDUCE_NRM_BLOOM,
                        G31_U_PLAN_AE_STATE_NRM_BLOOM,
                    );
                } else {
                    g31_apply_autoexp(
                        d,
                        ae,
                        G31_U_AE_STATE_NRM,
                        G31_U_AE_PARAMS_NRM,
                        G31_U_AE_PARTIALS_NRM,
                        U_OUT_COLOR[0],
                        G31_U_PLAN_AE_REDUCE_NRM,
                        G31_U_PLAN_AE_STATE_NRM,
                    );
                }
            } else if bloom {
                let d = bloom_descs
                    .as_mut()
                    .unwrap_or_else(|| fail("A2: bloom on 面 bloom_descs 缺失（防御性复核）"));
                g31_apply_autoexp(
                    d,
                    ae,
                    G31_U_AE_STATE_BLOOM,
                    G31_U_AE_PARAMS_BLOOM,
                    G31_U_AE_PARTIALS_BLOOM,
                    G31_U_BLOOM_COMP_OUT,
                    G31_U_PLAN_AE_REDUCE_BLOOM,
                    G31_U_PLAN_AE_STATE_BLOOM,
                );
            } else {
                let d = off_descs
                    .as_mut()
                    .unwrap_or_else(|| fail("A2: 基础面 off_descs 缺失（防御性复核）"));
                g31_apply_autoexp(
                    d,
                    ae,
                    G31_U_AE_STATE,
                    G31_U_AE_PARAMS,
                    G31_U_AE_PARTIALS,
                    U_OUT_COLOR[0],
                    G31_U_PLAN_AE_REDUCE,
                    G31_U_PLAN_AE_STATE,
                );
            }
        }
        // G37 W3 fg_combo：FG × --quality full 组合施加点——必在 A2 autoexp
        // 变换之后（AE 摘出断言前提「变体末位 pass = encode」由本序保证;FG
        // 纯尾挂,施加后末三/五 pass 为 FG 族）。fg_assets_full 仅 fg on ×
        // textures on 面 Some（两点式 ⇒ quality_full 恒真——A 组卫兵已裁散臂;
        // full 终态 = TEXNRM_BLOOM_RIS + AE,helper 内 assert 资源计数 48 钉死）。
        if let Some(fga) = fg_assets_full.as_ref() {
            let d = tex_descs
                .as_mut()
                .unwrap_or_else(|| fail("fg_combo: fg×full 面 tex_descs 缺失（防御性复核）"));
            g31_apply_fg_full(d, fga, &enc_spv_bytes, enc_dispatch, (ew * eh) as u64);
        }
        let mut lane = if let Some((hz_res, hz_pass, _, hz_rb, hz_ids)) = hzb_descs.as_ref() {
            let hb = hzb_bits.as_ref().unwrap();
            // 双 TLAS:表 0 = 初剔后(逐帧掩码),表 1 = 全量(阴影射线零剔除)。
            // 两 desc 引用同 BLAS 表/实例表(只读双借,创建期各自建 AS)。
            let hzb_accel = [
                AccelStructDesc {
                    scene: RayQuerySceneDesc {
                        blas_triangles: &hzb_blas_refs,
                        instances: &hzb_insts,
                    },
                    transforms: None,
                    updatable_blas: &[], // B1 全静态 BLAS（B5 字段面 0-byte 默认）
                },
                AccelStructDesc {
                    scene: RayQuerySceneDesc {
                        blas_triangles: &hzb_blas_refs,
                        instances: &hzb_insts,
                    },
                    transforms: None,
                    updatable_blas: &[],
                },
            ];
            // evidence 元信息面刷新(mip 拓扑/平铺量/实例数)。
            hzb_levels_meta = hb.levels.clone();
            hzb_flat_offsets_meta = hb.flat_offsets.clone();
            let dims: Vec<String> = hb
                .levels
                .iter()
                .map(|&(w, h)| format!("[{w},{h}]"))
                .collect();
            hzb_meta_json = format!(
                "{{\"instances\":{},\"levels\":{},\"level_dims\":[{}],\"flat_texels\":{},\"conv\":\"standard_z\"}}",
                hzb_groups.len(),
                hb.levels.len(),
                dims.join(","),
                hb.flat_texels
            );
            // G37 W2 合入 #82/#113:PSO 变体账本登记——hzb 车道(era0 = precache
            // 面/era≥1 = 守护面;运行期新变体遭遇告警,验收 pso_runtime_creates
            // == 0。同 SPV 多 pass 判同变体,pass 名只落报告行不进键)。
            pso_ledger.begin_session();
            for p in hz_pass.iter() {
                let (pname, pspv) = match p {
                    Pass::Compute(cp) => (cp.name, cp.spirv),
                    Pass::Raster(rp) => (rp.name, rp.vs_spirv), // 现车道纯 compute;raster 出现时 vs/fs 双注册归后续
                };
                if pso_ledger.register(pname, pspv) {
                    eprintln!("{GTAG}: [PSO] 运行期新 PSO 变体遭遇 pass={pname}（era≥1 未预测,#82 告警口径）");
                    if pso_strict {
                        fail("PSO strict:运行期新变体遭遇（RURIX_G31_PSO_STRICT=1,fail-closed）");
                    }
                }
            }
            match G31HzbLane::create(
                hz_res,
                hz_pass,
                &hzb_bar_refs,
                hz_rb,
                &hzb_accel,
                hz_ids.clone(),
                hzb_groups.clone(),
                hb.levels.len(),
            ) {
                Ok(l) => G31AnyLane::Hzb(l),
                Err(e) => g31_dev_env_or_fail("device_lane", &e),
            }
        } else {
            // B4/C13:textures on = 纹理变体描述组,svt on = SVT 变体描述组
            // （闭集互斥 ⇒ off_descs 恒 Some 但不消费;五 pass 与 off 面同图,
            // 仅 scene pass/资源/屏障替换）。D3:bloom on = bloom 变体描述组
            // （九 pass 图,encode 改读合成缓冲;同互斥纪律）。D2:smooth-normals
            // on = nrm 变体描述组（含 ×bloom 组合面;选择序在 bloom 前——组合
            // 臂归 nrm 面承载）。
            let descs = svt_descs
                .as_ref()
                .or(tex_descs.as_ref())
                .or(nrm_descs.as_ref())
                .or(bloom_descs.as_ref())
                .unwrap_or_else(|| off_descs.as_ref().unwrap());
            let accel_structs = [AccelStructDesc {
                scene: RayQuerySceneDesc {
                    blas_triangles: &blas_refs,
                    instances: &assets.instances,
                },
                transforms: None,
                updatable_blas: &[], // B5 字段面 0-byte 默认（全静态）
            }];
            // G37 W2 合入 #82/#113:PSO 变体账本登记——tsr 车道(era0 = precache
            // 面/era≥1 = 守护面;运行期新变体遭遇告警,验收 pso_runtime_creates
            // == 0。同 SPV 多 pass〔encode/encode_fg〕判同变体,与 rurix-rt
            // 会话级 ComputePipelineKey 去重同判)。
            pso_ledger.begin_session();
            for p in descs.passes.iter() {
                let (pname, pspv) = match p {
                    Pass::Compute(cp) => (cp.name, cp.spirv),
                    Pass::Raster(rp) => (rp.name, rp.vs_spirv), // 现车道纯 compute;raster 出现时 vs/fs 双注册归后续
                };
                if pso_ledger.register(pname, pspv) {
                    eprintln!("{GTAG}: [PSO] 运行期新 PSO 变体遭遇 pass={pname}（era≥1 未预测,#82 告警口径）");
                    if pso_strict {
                        fail("PSO strict:运行期新变体遭遇（RURIX_G31_PSO_STRICT=1,fail-closed）");
                    }
                }
            }
            match G31TsrLane::create(descs, &accel_structs, fg, bloom, smooth_nrm, ggx) {
                Ok(mut l) => {
                    // C13 SVT:逐帧状态挂载（请求缓冲字节数 = in_w·in_h·4）。
                    if svt_on {
                        l.set_svt((in_w as usize) * (in_h as usize) * 4);
                    }
                    // A1:--lamp-lights on → params[49]=contrib（off 不挂载
                    // ⇒ 参数面 0-byte）。
                    if lamp_lights {
                        l.set_lamp_contrib(lamp_contrib_v);
                    }
                    // A2:--auto-exposure on → 变体族 (params, partials) 下标
                    // 挂载（prepare_update reduce parity override + encode
                    // override 下标右移消费;off 不挂载 ⇒ override 面 0-byte。
                    // day_0828 Phase B：tex 族四形态下标先行）。
                    if autoexp {
                        // day_0829 realism：AE 三件顺延 +1 分支（guard 先于
                        // 既有;红修 #2——既有 em 无本分支为 Phase F 遗留缺口,
                        // de342586 锚内曾冻结;G37 W1 补 _EM 分支已修复）。
                        // G37 W2 transparency：_TRANSP guard 最先——transp 属
                        // realism 链族但挂载序在 nm 两件之后（下标最高）,故
                        // 优先级带须先于 nm/realism_any 两族;与 g31_apply_
                        // autoexp 调用点 match 序逐字同构,assert 连号为保护网。
                        // G37 W2 ris_nee:_RIS guard 最先(lamp_tbl 挂载序
                        // 最尾即下标最高;与锚⑭ match 序逐字同构)。
                        let (pi, ti) = if textures && smooth_nrm && (gi2_ris || gi2_nee) && bloom {
                            (
                                G31_U_AE_PARAMS_TEXNRM_BLOOM_RIS,
                                G31_U_AE_PARTIALS_TEXNRM_BLOOM_RIS,
                            )
                        } else if textures && smooth_nrm && (gi2_ris || gi2_nee) {
                            (G31_U_AE_PARAMS_TEXNRM_RIS, G31_U_AE_PARTIALS_TEXNRM_RIS)
                        } else if textures && smooth_nrm && transparency && bloom {
                            (
                                G31_U_AE_PARAMS_TEXNRM_BLOOM_TRANSP,
                                G31_U_AE_PARTIALS_TEXNRM_BLOOM_TRANSP,
                            )
                        } else if textures && smooth_nrm && transparency {
                            (
                                G31_U_AE_PARAMS_TEXNRM_TRANSP,
                                G31_U_AE_PARTIALS_TEXNRM_TRANSP,
                            )
                        } else if textures && smooth_nrm && normal_maps && bloom {
                            (
                                G31_U_AE_PARAMS_TEXNRM_BLOOM_NM,
                                G31_U_AE_PARTIALS_TEXNRM_BLOOM_NM,
                            )
                        } else if textures && smooth_nrm && normal_maps {
                            (G31_U_AE_PARAMS_TEXNRM_NM, G31_U_AE_PARTIALS_TEXNRM_NM)
                        } else if textures && smooth_nrm && realism_any && bloom {
                            (
                                G31_U_AE_PARAMS_TEXNRM_BLOOM_REAL,
                                G31_U_AE_PARTIALS_TEXNRM_BLOOM_REAL,
                            )
                        } else if textures && smooth_nrm && realism_any {
                            (G31_U_AE_PARAMS_TEXNRM_REAL, G31_U_AE_PARTIALS_TEXNRM_REAL)
                        } else if textures && smooth_nrm && emissive_tex && bloom {
                            // G37 W1:em+AE override 错位修复(day_0828 Phase F
                            // 遗留,day_0829 HANDOVER §G.1 兑现)——em on 时逐帧
                            // reduce override 须走 _EM 下标族;修复前传 TEXNRM
                            // (32,33)=(triem,真 params) ⇒ AE 实际近似恒等。
                            // 语义变更即重锚,旧十臂 de342586 谱系已作废。
                            (
                                G31_U_AE_PARAMS_TEXNRM_BLOOM_EM,
                                G31_U_AE_PARTIALS_TEXNRM_BLOOM_EM,
                            )
                        } else if textures && smooth_nrm && emissive_tex {
                            (G31_U_AE_PARAMS_TEXNRM_EM, G31_U_AE_PARTIALS_TEXNRM_EM)
                        } else if textures && smooth_nrm && bloom {
                            (G31_U_AE_PARAMS_TEXNRM_BLOOM, G31_U_AE_PARTIALS_TEXNRM_BLOOM)
                        } else if textures && smooth_nrm {
                            (G31_U_AE_PARAMS_TEXNRM, G31_U_AE_PARTIALS_TEXNRM)
                        } else if textures && bloom {
                            (G31_U_AE_PARAMS_TEX_BLOOM, G31_U_AE_PARTIALS_TEX_BLOOM)
                        } else if textures {
                            (G31_U_AE_PARAMS_TEX, G31_U_AE_PARTIALS_TEX)
                        } else if smooth_nrm && bloom {
                            (G31_U_AE_PARAMS_NRM_BLOOM, G31_U_AE_PARTIALS_NRM_BLOOM)
                        } else if smooth_nrm {
                            (G31_U_AE_PARAMS_NRM, G31_U_AE_PARTIALS_NRM)
                        } else if bloom {
                            (G31_U_AE_PARAMS_BLOOM, G31_U_AE_PARTIALS_BLOOM)
                        } else {
                            (G31_U_AE_PARAMS, G31_U_AE_PARTIALS)
                        };
                        l.set_autoexp(pi, ti);
                    }
                    // day_0828 Phase B:--textures on → params[50] = k_pix =
                    // 2·tan(fovy/2)/in_h（mip 选择像素锥角;fov 会话恒定 ⇒
                    // era 常量;off 不挂载 ⇒ 参数面 0-byte）。
                    if textures {
                        l.set_tex_kpix(2.0 * (cam.fov_y_rad * 0.5).tan() / in_h as f32);
                    }
                    // Phase C:--gi2 on → params[51]=1/[53]=clamp/[54]=scale
                    // （off 不挂载 ⇒ 四槽不写参数面 0-byte）;[52]=frame_idx
                    // 逐帧挂载见帧循环。
                    if gi2 {
                        l.set_gi2(gi2_scale_v, gi2_clamp_v);
                    }
                    // day_0829 臂①:--metal-f0 on → params[55]=1.0 + params
                    // 扩容（off 不挂载 ⇒ 不扩不写参数面 0-byte）。
                    if metal_f0 {
                        l.set_metal_f0();
                    }
                    // day_0829 臂②:--rt-ao on → params[56..60)（off 不挂载
                    // ⇒ 四槽不写参数面 0-byte）。
                    if rt_ao {
                        l.set_rt_ao(rt_ao_radius_v, rt_ao_strength_v, rt_ao_samples_v as f32);
                    }
                    // G37 W2 臂⑦:--transparency on → params[68]=1.0（off 不
                    // 挂载 ⇒ 不写参数面 0-byte;透射率在 tri_transp 侧表）。
                    if transparency {
                        l.set_transparency();
                    }
                    // G37 W2 臂⑧:--gi2-ris/--gi2-nee → params[69..72)
                    // (off 不挂载 ⇒ 三槽不写参数面 0-byte)。
                    if gi2_ris || gi2_nee {
                        l.set_gi2_ris(gi2_ris, gi2_ris_m_v as f32, gi2_nee);
                    }
                    // day_0829 臂⑤:--soft-shadows on → params[60..62)（off
                    // 不挂载 ⇒ 两槽不写参数面 0-byte）。
                    if soft_shadows {
                        l.set_soft_shadows(soft_shadow_samples_v as f32);
                    }
                    // day_0829 臂③:--rt-reflect on → params[62..65)（off
                    // 不挂载 ⇒ 三槽不写参数面 0-byte）。
                    if rt_reflect {
                        l.set_rt_reflect(rt_reflect_rough_max_v, rt_reflect_clamp_v);
                    }
                    // day_0829 臂⑥:--gi2-tex on → params[67]（off 不挂载
                    // ⇒ 不写参数面 0-byte）。
                    if gi2_tex {
                        l.set_gi2_tex();
                    }
                    // day_0829 臂④:--normal-maps on → params[65..67)（off
                    // 不挂载 ⇒ 两槽不写参数面 0-byte）。
                    if normal_maps {
                        l.set_normal_maps(normal_strength_v);
                    }
                    // Phase D:--tsr-quality on → tsr_params[19]=min_alpha/
                    // [20]=clamp K（off 不挂载 ⇒ 两槽不写参数面 0-byte）。
                    if tsr_quality {
                        l.set_tsrq(tsrq_min_alpha_v, tsrq_clamp_v);
                    }
                    G31AnyLane::Off(l)
                }
                Err(e) => g31_dev_env_or_fail("device_lane", &e),
            }
        };
        // C7:debug label 活跃态簿记（era 车道重建刷新;profile-json 消费）。
        debug_labels_active = Some(match &lane {
            G31AnyLane::Off(l) => l.session.debug_labels_active(),
            G31AnyLane::Hzb(l) => l.session.debug_labels_active(),
        });
        // A2:autoexp on 时链描述在 display_encode 前插两微 pass 字面（hzb 面
        // CLI 已裁不达,replace 恒中性）。
        let chain_desc = {
            let base_chain = if hzb == G31Hzb::On {
                "hzb_primary→hzb_shade→mv→resample→resolve→display_encode→hzb_test_p1→hzb_reduce×L−1→hzb_pack×L→hzb_test_p2（HZB 两阶段剔除 on".to_owned()
            } else if textures && smooth_nrm && bloom {
                "scene(g31_texture_nrm_gi)→mv→resample→resolve→bloom_bright→bloom_blur_h→bloom_blur_v→bloom_composite→display_encode".to_owned()
            } else if textures && smooth_nrm {
                "scene(g31_texture_nrm_gi)→mv→resample→resolve→display_encode".to_owned()
            } else if textures && bloom {
                "scene(g31_texture_gi)→mv→resample→resolve→bloom_bright→bloom_blur_h→bloom_blur_v→bloom_composite→display_encode".to_owned()
            } else if textures {
                "scene(g31_texture_gi)→mv→resample→resolve→display_encode".to_owned()
            } else if smooth_nrm && bloom {
                "scene(g18_smooth_nrm)→mv→resample→resolve→bloom_bright→bloom_blur_h→bloom_blur_v→bloom_composite→display_encode".to_owned()
            } else if smooth_nrm {
                "scene(g18_smooth_nrm)→mv→resample→resolve→display_encode".to_owned()
            } else if bloom {
                "scene→mv→resample→resolve→bloom_bright→bloom_blur_h→bloom_blur_v→bloom_composite→display_encode".to_owned()
            } else {
                "scene→mv→resample→resolve→display_encode".to_owned()
            };
            if autoexp {
                base_chain.replace(
                    "→display_encode",
                    "→autoexp_reduce→autoexp_state→display_encode",
                )
            } else {
                base_chain
            }
        };
        eprintln!(
            "{GTAG}: era 就绪 extent={ew}x{eh} internal={in_w}x{in_h}（车道:{}{},resize_eras={resize_eras}）",
            chain_desc,
            match fg {
                G31Fg::Off => {
                    if bloom {
                        "（九 pass,bloom on）".to_owned()
                    } else {
                        "（五 pass,fg off）".to_owned()
                    }
                }
                // G37 W3 fg_combo：full 组合面 pass 链字面（fg on × bloom on ⇔
                // --quality full 两点式;FG 尾挂 full+AE 十一 pass 之后——
                // mvn=11/fg1=12/enc_fg1=13(/fg2=14/enc_fg2=15),插值
                // post-bloom comp parity 对）。
                G31Fg::X2 => {
                    if bloom {
                        "→mv_negate→fg1→enc_fg1（十四 pass,--quality full × fg x2——comp parity 双缓冲插值 post-bloom）".to_owned()
                    } else {
                        "→mv_negate→fg1→enc_fg1（八 pass,fg x2）".to_owned()
                    }
                }
                G31Fg::X3 => {
                    if bloom {
                        "→mv_negate→fg1→enc_fg1→fg2→enc_fg2（十六 pass,--quality full × fg x3——comp parity 双缓冲插值 post-bloom）".to_owned()
                    } else {
                        "→mv_negate→fg1→enc_fg1→fg2→enc_fg2（十 pass,fg x3）".to_owned()
                    }
                }
            }
        );
        let mut resized = false;
        let mut era_first = true;
        while fi < total {
            // ── 窗口事件面(输入/resize/最小化/关闭;每帧首段泵)──
            if let Some(w) = window.as_mut() {
                let input = w.poll_input();
                if input.close_requested {
                    exit_reason = "user_close";
                    break 'eras;
                }
                if input.minimized {
                    // 最小化/alt-tab:跳过渲染/present 不消费帧预算(消息泵保持,
                    // 恢复后续跑;8ms 轮询避免空转)。
                    // C4 storm 面:注入臂触发的最小化在跳过面实跑一轮后即恢复
                    // (恢复 WM_SIZE 与 OS 消息面同通路;跳过次数实记)。
                    if storm_restore_pending {
                        storm_min_skips += 1;
                        w.storm_wm_size(ew, eh, false);
                        storm_restore_pending = false;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(8));
                    continue;
                }
                if let Some((nw, nh)) = input.resize_pending {
                    if (nw, nh) != (ew, eh) {
                        if let Err(e) = w.resize(nw, nh) {
                            fail(&format!("窗口 resize {nw}x{nh}: {e}"));
                        }
                        if w.extent() != (ew, eh) {
                            resized = true;
                            resize_eras += 1;
                            break;
                        }
                    }
                }
                if auto_move.is_none() {
                    let dt = last_frame_wall
                        .map(|t| t.elapsed().as_secs_f32())
                        .unwrap_or(1.0 / 60.0)
                        .clamp(0.001, 0.2);
                    g31_apply_input(&mut cam, &mut ev100, &input, &mut prev_keys, dt);
                }
            }
            // ── C4 窗口风暴驱动（--window-storm 爆发臂 / --storm-soak 周期臂;
            //    默认关 = 本块全跳零消费。程序化 resize 走 swapchain  era 重建
            //    真通路;最小化/恢复走 WM_SIZE 同通路注入,跳过面实跑后恢复）──
            if window_storm > 0 && !storm_burst_done {
                storm_burst_done = true;
                let Some(w) = window.as_mut() else {
                    fail("--window-storm 须真窗口会话（headless 已 fail-fast,防御性复核）");
                };
                let (aw, ah) = ((ew / 2).max(64), (eh / 2).max(64));
                for k in 0..window_storm {
                    // 交替 半 extent ↔ 原 extent:真 win32 尺寸变更(SetWindowPos,
                    // 用户拖拽同通路)→ 同步 WM_SIZE 立即泵出消化 ⇒ 每次皆真
                    // swapchain/staging 重建(extent 逐次真实变化)。
                    let (tw, th) = if k % 2 == 0 { (aw, ah) } else { (ew, eh) };
                    w.storm_set_window_size(tw, th);
                    let inp = w.poll_input();
                    if let Some((nw, nh)) = inp.resize_pending {
                        if let Err(e) = w.resize(nw, nh) {
                            fail(&format!("window-storm 第 {} 次 resize {nw}x{nh}: {e}", k + 1));
                        }
                    }
                    storm_resize_ops += 1;
                }
                if w.extent() != (ew, eh) {
                    resized = true;
                    resize_eras += 1;
                    break;
                }
            }
            if storm_soak > 0 && fi > 0 && fi % storm_soak == 0 && storm_last_fault_fi != fi {
                storm_last_fault_fi = fi;
                let Some(w) = window.as_mut() else {
                    fail("--storm-soak 须真窗口会话（headless 已 fail-fast,防御性复核）");
                };
                if fi % (storm_soak * 8) == 0 {
                    // 最小化/恢复循环（alt-tab 等效面:非全屏 exclusive 下 alt-tab
                    // 不失效 swapchain,最小化 = 唯一塌零/失效源——波 A 在案）。
                    w.storm_wm_size(0, 0, true);
                    storm_restore_pending = true;
                    storm_min_cycles += 1;
                    continue; // 下一轮 poll 见 minimized → 跳过面实跑 → 恢复
                }
                // 周期 resize toggle:基准（窗口创建 extent = out_w×out_h)↔
                // 半基准固定两面往返（非逐次减半——extent 在两面间确定性摆动）。
                // 真 win32 尺寸变更 → 下一轮 poll 经 A3 resize_pending 面消化
                // (resize → swapchain/staging/era 全真重建)。
                let (bw, bh) = (out_w, out_h);
                let (aw, ah) = ((bw / 2).max(64), (bh / 2).max(64));
                let (tw, th) = if (ew, eh) == (bw, bh) {
                    (aw, ah)
                } else {
                    (bw, bh)
                };
                w.storm_set_window_size(tw, th);
                storm_resize_ops += 1;
                continue;
            }
            last_frame_wall = Some(std::time::Instant::now());
            // ── auto-move 确定性轨迹(帧号唯一事实源,绝对位姿)──
            if let Some(name) = auto_move.as_deref() {
                let (yaw, pitch, eye) = g31_auto_move_pose(name, &cam0, fi, total);
                cam.yaw = yaw;
                cam.pitch = pitch;
                cam.eye = eye;
                if let Some((a, b)) = ev100_ramp {
                    let t = f64::from(fi) / f64::from(total.max(1));
                    ev100 = a + (b - a) * t;
                }
            }
            // ── 逐帧相机 → vp(192B 帧参数 uniform 通路真实工作面)──
            let spec = cam.spec();
            // G31+ #58 逐帧 host cut 统计（相机逐帧变化 → cut 逐帧重算的
            // measured 面;在 t_render 计时之外,不污染 real_render_frame_ms
            // 口径。出帧几何冻结于装配期 cut,如实登记不冒充）。
            if let Some((_, pack)) = &cluster_ctx {
                let t_stat = std::time::Instant::now();
                let stat = cluster_lod_frame_stat(
                    pack,
                    &spec,
                    in_w,
                    in_h,
                    cluster_opt.threshold_px,
                    fi,
                    fi % 16 == 0,
                );
                cluster_stat_ms_total += t_stat.elapsed().as_secs_f64() * 1e3;
                cluster_frame_stats.push(stat);
            }
            // G37 W2 visbuffer:真窗口逐帧相机样本采集（Copy 零成本;device 链
            // 循环后跑）。
            if visbuffer_opt.enabled && visbuffer_sample_set.contains(&fi) {
                visbuffer_samples_taken.push(VisBufferCamSample {
                    frame: fi,
                    spec,
                    in_w,
                    in_h,
                });
            }
            // G37 W3 frame_cut 合入:逐帧相机样本（Copy 零成本;全帧采集 =
            // 逐帧判档字面）。
            if frame_cut_opt.enabled {
                frame_cut_samples_taken.push(FrameCutCamSample {
                    frame: fi,
                    spec,
                    in_w,
                    in_h,
                });
            }
            // G31+ #95/#99 逐帧 WP/HLOD 状态推进（相机逐帧变化 → 距离环流送
            // tick + 互斥选层 + warmup 原子翻转协议的 measured 面;在 t_render
            // 计时之外。出帧几何冻结于装配期选层,如实登记不冒充）。
            if let Some((_, ctx)) = wp_ctx.as_mut() {
                let t_stat = std::time::Instant::now();
                let stat = wp_hlod_frame_tick(ctx, spec.eye);
                wp_stat_ms_total += t_stat.elapsed().as_secs_f64() * 1e3;
                wp_frame_stats.push(stat);
            }
            let vp = build_vp(&spec, in_w, in_h);
            let inv_vp = vp.inverse().unwrap_or_else(|| fail("view-proj 必须可逆"));
            let exposure = 2.0f32.powf(-(ev100 as f32));
            let j = [
                halton(jitter_base + fi + 1, 2) - 0.5,
                halton(jitter_base + fi + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let last = fi + 1 == total;
            let rb_mode = if last {
                G31Readback::BgraAndColor
            } else if window.is_some() || auto_move.is_some() {
                G31Readback::Bgra
            } else {
                G31Readback::None
            };
            let reset = fi == 0 || era_first;
            era_first = false;
            // A5 probe 帧（FG 面）:post-warmup 首个 gen 活跃帧触发接线态对拍回读
            // (era 首帧无 prev 不可探,顺延至下一 gen 活跃帧;一次性)。B1 HZB 面:
            // probe_pre = probe_fi−1（上帧平铺金字塔 + 深度回读臂）,probe_fi 本帧
            // p1 判定 + host 金标准复算（两帧成对,一次性）。
            let probe = match (&lane, probe_fi) {
                (G31AnyLane::Off(l), Some(pf)) => {
                    fi >= pf && l.gen_active(reset) && wired_parity.is_none()
                }
                _ => false,
            };
            let hzb_pre_frame = (hzb == G31Hzb::On
                && matches!(probe_fi, Some(pf) if fi + 1 == pf)
                && hzb_pre_data.is_none())
                || (hzb == G31Hzb::On && last && std::env::var("RURIX_G31_DUMP_F32").ok().as_deref() == Some("1"));
            let hzb_cmp_frame = hzb == G31Hzb::On
                && matches!(probe_fi, Some(pf) if fi == pf)
                && hzb_wired_parity.is_none();

            // Phase C：GI2 帧序号逐帧挂载（params[52]——R2 时域旋转;off 不
            // 调用零消费;双跑同帧序〔fi 确定性〕⇒ 位级一致口径不破）。
            // day_0829:rt-ao/soft-shadows 时序采样同消费 [52],条件放宽
            // （gi2 off 面由 prepare_update realism 块补写）。
            if gi2 || rt_ao || soft_shadows || rt_reflect {
                if let G31AnyLane::Off(l) = &mut lane {
                    l.set_gi2_frame(fi as f32);
                }
            }
            let t_render = std::time::Instant::now();
            let rec = match match &mut lane {
                G31AnyLane::Off(l) => l.frame(
                    in_w,
                    in_h,
                    ew,
                    eh,
                    j,
                    eps,
                    scene.quads.len(),
                    scene.points.len(),
                    &inv_vp,
                    &vp,
                    &vp_j,
                    exposure,
                    reset,
                    rb_mode,
                    probe,
                ),
                G31AnyLane::Hzb(l) => l.frame(
                    in_w,
                    in_h,
                    ew,
                    eh,
                    j,
                    eps,
                    scene.quads.len(),
                    scene.points.len(),
                    &inv_vp,
                    &vp,
                    &vp_j,
                    exposure,
                    reset,
                    rb_mode,
                    hzb_pre_frame,
                ),
            } {
                Ok(r) => r,
                Err(e) => {
                    // C4 探针面(tdr/budget 臂;命中打印退 0,非探针面直通 fail)。
                    if let Some(spec) = fault_probe.as_deref() {
                        g31_probe_lane_failure(spec, fi, &e);
                    }
                    fail(&format!("帧 {fi} 车道: {e}"));
                }
            };
            let render_el = t_render.elapsed().as_secs_f64() * 1000.0;
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "帧 {fi} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            if rec.leaked_object_count != 0 || rec.leaked_allocation_count != 0 {
                fail(&format!(
                    "帧 {fi} leak 账本非零 object={} allocation={}（资源无泄漏机核判红）",
                    rec.leaked_object_count, rec.leaked_allocation_count
                ));
            }

            // ── C13 SVT 逐帧请求-驻留闭环（svt on 面;帧 N miss 请求 → host
            //    consume（LRU 池 + 页表影 + 瓦片"盘"读取）→ 帧 N+1 上传段;
            //    host 池影同步应用,era 重建再同步源）──
            if svt_on {
                let Some(req_bytes) = rec.svt_requests.as_ref() else {
                    fail(&format!("帧 {fi} SVT 请求缓冲回读缺失（svt on 面证据不完整判红）"));
                };
                let Some(stream) = svt_stream.as_mut() else {
                    fail(&format!("帧 {fi} SVT 流送状态缺失（防御性复核）"));
                };
                let Some((sassets, _)) = svt_report.as_ref() else {
                    fail(&format!("帧 {fi} SVT 报告缺失（防御性复核）"));
                };
                let req = read_f32(req_bytes);
                let plan = match stream.consume(&req) {
                    Ok(p) => p,
                    Err(e) => fail(&format!("帧 {fi} SVT 请求消费: {e}")),
                };
                // host 池影应用（瓦片上传段;与次帧 device 上传同源字节面）。
                let mut pending: Vec<(StableResourceId, u64, Vec<u8>)> =
                    Vec::with_capacity(plan.page_table_writes.len() + plan.tile_uploads.len());
                for &(page_id, entry) in &plan.page_table_writes {
                    pending.push((
                        StableResourceId(u64::from(G31_U_SVT_PAGETABLE) + 1),
                        u64::from(page_id) * 4,
                        entry.to_le_bytes().to_vec(),
                    ));
                }
                for &(slot, page_id) in &plan.tile_uploads {
                    let payload = match sassets.tile_set.page_payload(page_id) {
                        Ok(p) => p,
                        Err(e) => fail(&format!("帧 {fi} SVT 瓦片集读取: {e}")),
                    };
                    let tile_bytes: Vec<u8> =
                        payload.iter().flat_map(|v| v.to_le_bytes()).collect();
                    let off = slot as usize * svt::SVT_PHYS_TILE_BYTES;
                    svt_pool_image[off..off + svt::SVT_PHYS_TILE_BYTES]
                        .copy_from_slice(&tile_bytes);
                    pending.push((
                        StableResourceId(u64::from(G31_U_SVT_POOL) + 1),
                        slot as u64 * svt::SVT_PHYS_TILE_BYTES as u64,
                        tile_bytes,
                    ));
                }
                if let G31AnyLane::Off(l) = &mut lane {
                    l.set_svt_pending(pending);
                }
                // 逐帧统计（evidence 面;只进 evidence 不进硬门,RFC-0016 §4.0-4 同律）。
                svt_stats.miss_px.push(plan.miss_pixels);
                svt_stats.unique_pages.push(plan.unique_pages);
                svt_stats.loaded.push(plan.loaded);
                svt_stats.evicted.push(plan.evicted);
                svt_stats.miss_px_total += u64::from(plan.miss_pixels);
                svt_stats.requested_pages_total += u64::from(plan.unique_pages);
                svt_stats.tiles_loaded_total += u64::from(plan.loaded);
                svt_stats.tiles_evicted_total += u64::from(plan.evicted);
                svt_stats.io_bytes_total += plan.io_bytes;
                if plan.miss_pixels > 0 {
                    svt_stats.fallback_frames += 1;
                }
            }

            // ── B1 HZB 逐帧决策面记账 + 接线态对拍（hzb on 面;probe 两帧成对:
            //    预备帧回读上帧平铺金字塔 + 深度,本帧 p1 判定序列 host 复算）──
            if let Some(hzrec) = rec.hzb.as_ref() {
                hzb_tested += u64::from(hzrec.tested_p1);
                hzb_occluded += u64::from(hzrec.occluded_p1);
                hzb_offscreen += u64::from(hzrec.offscreen);
                hzb_retested += u64::from(hzrec.retested_p2);
                hzb_flipped += u64::from(hzrec.flipped_p2);
                hzb_visible_sum += u64::from(hzrec.visible_final);
                if hzrec.closure_extra_submits > 0 || hzrec.closure_full_fallback {
                    hzb_closure_frames += 1;
                    hzb_closure_submits += u64::from(hzrec.closure_extra_submits) + 1;
                    if hzrec.closure_full_fallback {
                        hzb_fallbacks += 1;
                    }
                }
                if fi >= warmup {
                    hzb_gpu_ms.push(hzrec.hzb_gpu_ns / 1e6);
                    hzb_scene_gpu_ms.push(rec.scene_gpu_ns / 1e6);
                    hzb_host_ms.push(hzrec.host_ms);
                    hzb_closure_gpu_ms.push(hzrec.closure_extra_gpu_ns / 1e6);
                }
                if hzb_pre_frame {
                    let (Some(d), Some(f)) =
                        (hzrec.probe_depth.as_ref(), hzrec.probe_flat.as_ref())
                    else {
                        fail(&format!("帧 {fi} HZB probe 预备回读缺失"));
                    };
                    hzb_pre_data = Some((d.clone(), f.clone()));
                }
                if hzb_cmp_frame {
                    let Some((d, f)) = hzb_pre_data.as_ref() else {
                        fail(&format!("帧 {fi} HZB probe 预备数据缺失（对拍面不完整判红）"));
                    };
                    let wp = match g31_hzb_wired_parity(
                        d,
                        f,
                        in_w,
                        in_h,
                        &hzb_levels_meta,
                        &hzb_flat_offsets_meta,
                        &hzrec.rects_p1,
                        &hzrec.verdicts_p1,
                    ) {
                        Ok(w) => w,
                        Err(e) => fail(&format!("帧 {fi} HZB 接线态对拍复算: {e}")),
                    };
                    if !wp.mips_bitexact {
                        // 现场取证 dump（depth/flat 原字节;离线归因面,仅在红路径）。
                        let dump = |name: &str, v: &[f32]| {
                            let b: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
                            let _ = std::fs::write(format!(".tmp/g31_gates/hzb/{name}"), &b);
                        };
                        dump("probe_depth.bin", d);
                        dump("probe_flat.bin", f);
                        eprintln!(
                            "{GTAG}: HZB probe 现场 dump → .tmp/g31_gates/hzb/probe_{{depth,flat}}.bin"
                        );
                        fail(&format!(
                            "帧 {fi} HZB 接线态对拍：车道平铺金字塔 vs host HzbPyramid::build 非逐级位级全等（①零容差破坏）"
                        ));
                    }
                    if !wp.verdict_equal {
                        fail(&format!(
                            "帧 {fi} HZB 接线态对拍：p1 判定序列与 host test_rect 非逐 rect 全等（②破坏）"
                        ));
                    }
                    if wp.false_positives != 0 {
                        fail(&format!(
                            "帧 {fi} HZB 接线态对拍：假阳性 {}（③硬不变量破坏,exact_rect_occluded 独立复核检出）",
                            wp.false_positives
                        ));
                    }
                    eprintln!(
                        "{GTAG}: 帧 {} HZB 接线态对拍 mips={} 位级全等 + p1 判定 {} rect 逐字节全等 + 零假阳性（剔除 {}）+ digest {}",
                        fi + 1,
                        wp.mips,
                        wp.n_rects,
                        wp.occluded,
                        &wp.verdict_digest[..23]
                    );
                    hzb_wired_parity = Some((wp, fi));
                }
            }

            // ── A5 接线态对拍(probe 帧一次性;host 金标准复算,p100/SSIM 判红即
            //    fail——对拍门接线态维持机核面)──
            if probe {
                let (Some(prev_c), Some(mv_d), Some(cur_c), Some(mvn_d)) = (
                    rec.probe_prev_color.as_ref(),
                    rec.probe_mv.as_ref(),
                    rec.out_color.as_ref(),
                    rec.probe_mvn.as_ref(),
                ) else {
                    fail(&format!("帧 {fi} probe 回读四路缺失"));
                };
                // device 侧 MVN 内容直比对（位级 == −MV 回读;非零即 MV 通路
                // 缺陷硬门——取反为 IEEE 精确运算,任何非零 = stale/错绑）。
                let mvn_diff = mvn_d
                    .iter()
                    .zip(mv_d.iter())
                    .map(|(&x, &y)| (x + y).abs() as f64)
                    .fold(0.0, f64::max);
                if mvn_diff != 0.0 {
                    fail(&format!(
                        "帧 {fi} probe MVN vs −MV 直比对 max|mvn+mv|={mvn_diff:.6e} ≠ 0（MV 通路位级硬门判红）"
                    ));
                }
                if rec.probe_gen_out.len() != fg.inserted() as usize {
                    fail(&format!(
                        "帧 {fi} probe 生成帧路数 {} ≠ {}",
                        rec.probe_gen_out.len(),
                        fg.inserted()
                    ));
                }
                let mut wp = match g31_wired_parity_probe(
                    prev_c,
                    cur_c,
                    mv_d,
                    &rec.probe_gen_out,
                    ew,
                    eh,
                    fg_tol_v,
                ) {
                    Ok(w) => w,
                    Err(e) => fail(&format!("帧 {fi} 接线态对拍复算: {e}")),
                };
                wp.mvn_max_abs_plus_mv = mvn_diff;
                if !wp.in_bound {
                    fail(&format!(
                        "帧 {fi} 接线态对拍 excess={:.6e} > 0（逐像素 ULP 结构界含 G26 冻结地板 {:.6e},ratio={:.6};{}）",
                        wp.excess, fg_tol_v, wp.excess_ratio, fg_tol_source
                    ));
                }
                if !wp.ssim_beats_frame_hold {
                    fail(&format!(
                        "帧 {fi} 接线态 SSIM(device,hostref)={:.10} 未严格胜 frame-hold={:.10}",
                        wp.ssim_device_vs_hostref, wp.ssim_frame_hold_vs_hostref
                    ));
                }
                eprintln!(
                    "{GTAG}: 帧 {} 接线态对拍 p100={:.6e} ≤ bound(max={:.6e};excess={:.3e} ratio={:.4}) ssim_dev={:.10} > ssim_hold={:.10}（host 金标准复算,{}）",
                    fi + 1,
                    wp.p100,
                    wp.max_bound,
                    wp.excess,
                    wp.excess_ratio,
                    wp.ssim_device_vs_hostref,
                    wp.ssim_frame_hold_vs_hostref,
                    fg_tol_source
                );
                wired_parity = Some((wp, fi));
            }

            // ── present(device 已编码;host 仅拷贝/present——A1 逐像素编码段消除,
            //    encode host 墙钟恒 0,device 编码 GPU 耗时经 telemetry 单列)。
            //    A5:present 序 = 生成帧(t 时序升序)→ 真帧(真帧/生成帧序列,
            //    生成帧禁入真实渲染帧率口径,逐 present 墙钟入 presented 口径)──
            let mut pres_el = 0.0f64;
            if let Some(w) = window.as_mut() {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail(&format!("帧 {fi} 窗口面缺 BGRA8 回读"));
                };
                for g in &rec.gen_bgra8 {
                    let t_one = std::time::Instant::now();
                    if let Err(e) = w.present_rgba8(g) {
                        fail(&format!("帧 {fi} 生成帧窗口 present: {e}"));
                    }
                    let el = t_one.elapsed().as_secs_f64() * 1000.0;
                    pres_el += el;
                    if fi >= warmup {
                        present_ms.push(el);
                        present_seconds += el / 1000.0;
                        presented_frames += 1;
                    }
                }
                let t_one = std::time::Instant::now();
                if let Err(e) = w.present_rgba8(px) {
                    // C4 探针面(device-lost 三点臂;命中打印退 0,非探针面直通 fail)。
                    if let Some(spec) = fault_probe.as_deref() {
                        g31_probe_present_failure(spec, fi, &e, w, px);
                    }
                    fail(&format!("帧 {fi} 窗口 present: {e}"));
                }
                let el = t_one.elapsed().as_secs_f64() * 1000.0;
                pres_el += el;
                if fi >= warmup {
                    present_ms.push(el);
                    present_seconds += el / 1000.0;
                    presented_frames += 1;
                }
            }

            // ── digest(auto-move 逐帧序列;default 末帧;税单列不混渲染口径)──
            let t_dig = std::time::Instant::now();
            if auto_move.is_some() {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail(&format!("帧 {fi} auto-move 面缺 BGRA8 回读"));
                };
                digest_seq.push(g31_bgra_digest(ew, eh, px));
                ev100_seq.push(ev100);
                pose_seq.push([
                    f64::from(cam.eye[0]),
                    f64::from(cam.eye[1]),
                    f64::from(cam.eye[2]),
                    f64::from(cam.yaw),
                    f64::from(cam.pitch),
                ]);
            }
            // A2 验证面（默认关 = 0-byte）:逐帧 presented 亮度（8bit 归一
            // Rec.709 luma 全图均值,BGRA/RGBA 序随 bgra 旗标）+ 周期 raw dump
            //（每 n 帧,基路径 = --dump-present-raw 派生 `.f<帧号>`,布局同 D3）。
            if present_luma_out.is_some() {
                if let Some(px) = rec.bgra8.as_ref() {
                    let mut acc = 0.0f64;
                    for c in px.chunks_exact(4) {
                        let (r, b) = if bgra { (c[2], c[0]) } else { (c[0], c[2]) };
                        acc += 0.2126 * f64::from(r) + 0.7152 * f64::from(c[1]) + 0.0722 * f64::from(b);
                    }
                    luma_seq.push((fi, acc / (f64::from(ew) * f64::from(eh) * 255.0)));
                }
            }
            if dump_present_every > 0 && fi % dump_present_every == 0 {
                if let (Some(base), Some(px)) = (dump_present_raw.as_deref(), rec.bgra8.as_ref()) {
                    let mut buf = Vec::with_capacity(8 + px.len());
                    buf.extend_from_slice(&ew.to_le_bytes());
                    buf.extend_from_slice(&eh.to_le_bytes());
                    buf.extend_from_slice(px);
                    let p = format!("{base}.f{fi:04}");
                    std::fs::write(&p, &buf)
                        .unwrap_or_else(|e| fail(&format!("--dump-present-every 写 {p}: {e}")));
                }
            }
            if last {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail("末帧缺 BGRA8 回读".into());
                };
                presented_digest = g31_bgra_digest(ew, eh, px);
                if let Some(dp) = dump_last_frame.as_deref() {
                    // B3 跨臂像素对拍面:BGRA8 raw dump（w/h u32 LE 头 + 打包字节;
                    // device 臂 vs host 参考臂逐字节对拍由 smoke 裁决）。
                    let mut buf = Vec::with_capacity(8 + px.len());
                    buf.extend_from_slice(&ew.to_le_bytes());
                    buf.extend_from_slice(&eh.to_le_bytes());
                    buf.extend_from_slice(px);
                    std::fs::write(dp, &buf)
                        .unwrap_or_else(|e| fail(&format!("--dump-last-frame 写 {dp}: {e}")));
                }
                if let Some(dp) = dump_present_raw.as_deref() {
                    // 夜间巡航 D3 视觉验收面:BGRA8 raw dump（同 B3 布局;仅验证用）。
                    let mut buf = Vec::with_capacity(8 + px.len());
                    buf.extend_from_slice(&ew.to_le_bytes());
                    buf.extend_from_slice(&eh.to_le_bytes());
                    buf.extend_from_slice(px);
                    std::fs::write(dp, &buf)
                        .unwrap_or_else(|e| fail(&format!("--dump-present-raw 写 {dp}: {e}")));
                }
                let Some(out_data) = rec.out_color.as_ref() else {
                    fail("末帧缺 f32 out_color 回读".into());
                };
                // TEMP 像素归因 dump（毕后删除;env 门控,常态零消费）。
                if std::env::var("RURIX_G31_DUMP_F32").ok().as_deref() == Some("1") {
                    let b: Vec<u8> = out_data.iter().flat_map(|v| v.to_le_bytes()).collect();
                    let _ = std::fs::write(".tmp/g31_gates/hzb/last_f32.bin", &b);
                    if let Some(hz) = rec.hzb.as_ref()
                        && let Some(d) = hz.probe_depth.as_ref()
                    {
                        let bd: Vec<u8> = d.iter().flat_map(|v| v.to_le_bytes()).collect();
                        let _ = std::fs::write(".tmp/g31_gates/hzb/last_depth.bin", &bd);
                    }
                }
                if !out_data.iter().all(|v| v.is_finite()) {
                    fail("末帧 TSR 输出非有限");
                }
                render_digest = frame_content_digest(ew, eh, 3, out_data);
            }
            let dig_el = t_dig.elapsed().as_secs_f64() * 1000.0;

            if fi >= warmup {
                render_ms.push(render_el);
                digest_ms.push(dig_el);
                encode_gpu_ms.push(rec.encode_gpu_ns / 1e6);
                // D3:bloom 四 pass GPU 合计（telemetry 全量列按名前缀提取;bloom
                // on 才消费,off 空 vec 零消费）。
                if bloom {
                    bloom_gpu_ms.push(
                        rec.pass_gpu_ns
                            .iter()
                            .filter(|(n, _)| n.starts_with("g31_bloom_"))
                            .map(|(_, ns)| *ns)
                            .sum::<f64>()
                            / 1e6,
                    );
                }
                // A2:autoexp 两 pass GPU 合计（同律按名前缀提取;on 才消费）。
                if autoexp {
                    autoexp_gpu_ms.push(
                        rec.pass_gpu_ns
                            .iter()
                            .filter(|(n, _)| n.starts_with("g31_autoexp_"))
                            .map(|(_, ns)| *ns)
                            .sum::<f64>()
                            / 1e6,
                    );
                }
                // C7 profiler 收集（--profile-json on 才消费;与 render_ms 同窗）。
                if profile_json.is_some() {
                    profile_frames.push(G31ProfileFrame {
                        passes: rec
                            .pass_gpu_ns
                            .iter()
                            .map(|(n, ns)| (n.clone(), ns / 1e6))
                            .collect(),
                        cpu_record_ms: rec.cpu_record_ns as f64 / 1e6,
                        cpu_submit_ms: rec.cpu_submit_ns as f64 / 1e6,
                        cpu_fence_wait_ms: rec.cpu_fence_wait_ns as f64 / 1e6,
                        readback_convert_ms: rec.readback_convert_ms,
                        render_wall_ms: render_el,
                        present_wall_ms: pres_el,
                        digest_ms: dig_el,
                    });
                }
                // A5 双口径账目:real 只计真渲帧(生成帧禁入);单提交墙钟含 FG
                // GPU 段——telemetry 分列 render5/fg GPU 段如实登记。
                real_frames += 1;
                real_render_seconds += render_el / 1000.0;
                generated_frames += rec.gen_bgra8.len() as u64;
                if fg != G31Fg::Off {
                    fg_gpu_ms.push(rec.fg_gpu_ns / 1e6);
                    render5_gpu_ms.push(
                        (rec.scene_gpu_ns
                            + rec.mv_gpu_ns
                            + rec.resample_gpu_ns
                            + rec.resolve_gpu_ns
                            + rec.encode_gpu_ns)
                            / 1e6,
                    );
                }
            }
            if fi == 0 || (fi + 1) % 20 == 0 || fi + 1 == total {
                eprintln!(
                    "{GTAG}: 帧 {}/{total} render={render_el:.3}ms(gpu_encode={:.3}ms{}) present={pres_el:.3}ms digest={dig_el:.3}ms",
                    fi + 1,
                    rec.encode_gpu_ns / 1e6,
                    if fg != G31Fg::Off {
                        format!(" gpu_fg={:.3}ms gen={}", rec.fg_gpu_ns / 1e6, rec.gen_bgra8.len())
                    } else {
                        String::new()
                    },
                );
            }
            fi += 1;
        }
        if fi >= total || !resized {
            break 'eras;
        }
        // resize 触发的 era 更替:车道/资源在新 extent 重建(TSR 历史 reset)。
    }

    let frames_done = fi;
    // C4 面:探针臂跑完全程未触发 = 红（env 未武装/注入点未达——不冒充）;
    // 风暴臂计数汇总（机读单行,CI 门解析进 evidence;默认关臂零输出）。
    if let Some(spec) = fault_probe.as_deref() {
        fail(&format!(
            "--fault-probe {spec} 全程 {frames_done} 帧未触发注入错误（env 注入机制未武装或注入点越帧预算;fail-closed 不冒充）"
        ));
    }
    if window_storm > 0 || storm_soak > 0 {
        eprintln!(
            "{GTAG}: storm resize_ops={storm_resize_ops} min_cycles={storm_min_cycles} min_skips={storm_min_skips} resize_eras={resize_eras} window_storm={window_storm} storm_soak={storm_soak}"
        );
    }
    // G37 W2 合入:PSO 账本收口(sidecar 报告默认 off = 0-byte;计数恒登 stderr
    // 单行——schema rurix.g31.pso_warmup_report.v1,主 evidence schema
    // additionalProperties:false 冻结故新字段一律 sidecar)。
    eprintln!(
        "{GTAG}: [PSO] sessions={} unique_variants={} pso_runtime_creates={}",
        pso_ledger.sessions(),
        pso_ledger.unique_variants(),
        pso_ledger.runtime_creates()
    );
    if let Some(path) = pso_report.as_deref() {
        std::fs::write(path, pso_ledger.report_json())
            .unwrap_or_else(|e| fail(&format!("--pso-report 写 {path}: {e}")));
    }
    // ⑦ 多口径稳态统计(post-warmup;程序产禁手写阈)+ evidence。
    let (r_mean, _, r_cv, r_min, r_max) = g31_stats(&render_ms);
    let (p_mean, _, p_cv, p_min, p_max) = if headless || present_ms.iter().all(|v| *v == 0.0) {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g31_stats(&present_ms)
    };
    let (eg_mean, _, _, _, _) = if encode_gpu_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g31_stats(&encode_gpu_ms)
    };
    // D3 bloom 四 pass GPU 均值（bloom on 面;off = 0.0 零消费）。
    let (bg_mean, _, _, _, _) = if bloom_gpu_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g31_stats(&bloom_gpu_ms)
    };
    let (dg_mean, _, _, _, _) = if digest_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g31_stats(&digest_ms)
    };
    let encode_host_ms = 0.0f64; // device 编码后 host 编码墙钟恒 0(如实登记)
    let overhead_mean = encode_host_ms + p_mean;
    let counts = window.as_ref().map(|w| w.counts());

    let (window_json, p_mean_json, overhead_json) = if headless {
        ("null".to_owned(), "null".to_owned(), "null".to_owned())
    } else {
        let c = counts.unwrap_or(rurix_rt::vk::ExternalPresentCounts {
            frames_presented: 0,
            swapchain_rebuilds: 0,
        });
        let (fw, fh) = window.as_ref().map(|w| w.extent()).unwrap_or((0, 0));
        (
            format!(
                "{{\"visible\":{},\"channel_order\":{},\"extent\":{{\"w\":{fw},\"h\":{fh}}},\"frames_presented\":{},\"swapchain_rebuilds\":{}}}",
                !hidden,
                jstr(if bgra { "bgra8_unorm" } else { "rgba8_unorm" }),
                c.frames_presented,
                c.swapchain_rebuilds
            ),
            format!("{p_mean:.6}"),
            format!("{overhead_mean:.6}"),
        )
    };
    let pstat = |v: f64| -> String {
        if headless {
            "null".to_owned()
        } else {
            format!("{v:.6}")
        }
    };
    let encode_spv_json = format!(
        "{{\"path\":{},\"sha256\":{}}}",
        jstr(&spv_encode.replace('\\', "/")),
        jstr(&g31_file_sha(&spv_encode).unwrap_or_else(|e| fail(&e)))
    );
    // A5 FG 双口径统计与恒等式组(fg off 面不消费)。
    let (fgg_mean, _, _, _, _) = if fg_gpu_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g31_stats(&fg_gpu_ms)
    };
    let (r5g_mean, _, _, _, _) = if render5_gpu_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g31_stats(&render5_gpu_ms)
    };
    let real_render_fps = if real_render_seconds > 0.0 {
        real_frames as f64 / real_render_seconds
    } else {
        0.0
    };
    let presented_fps = if real_render_seconds + present_seconds > 0.0 {
        presented_frames as f64 / (real_render_seconds + present_seconds)
    } else {
        0.0
    };
    // A5 恒等式组(FgAccounting F9 同模;schema 层钉 const true——任何口径混算
    // 或计数面脱节都会翻 false 触发 schema 判红)。
    let id_presented = presented_frames == real_frames + generated_frames;
    let id_real_recompute = real_render_seconds > 0.0
        && real_render_fps == real_frames as f64 / real_render_seconds;
    let id_real_isolated = {
        let perturbed = if real_render_seconds > 0.0 {
            real_frames as f64 / real_render_seconds
        } else {
            0.0
        };
        let _ = generated_frames + 997; // 扰动面:real fps 公式与 generated 无关
        perturbed == real_render_fps
    };
    let id_presented_recompute = real_render_seconds + present_seconds > 0.0
        && presented_fps == presented_frames as f64 / (real_render_seconds + present_seconds);
    let id_digest_seq_len = digest_seq.len() as u64 == u64::from(frames_done);
    let framegen_spv_json = format!(
        "{{\"path\":{},\"sha256\":{}}}",
        jstr(&spv_framegen.replace('\\', "/")),
        jstr(&g31_file_sha(&spv_framegen).unwrap_or_else(|e| fail(&e)))
    );
    let wired_parity_json = if fg != G31Fg::Off {
        let Some((wp, pf)) = wired_parity.as_ref() else {
            fail("A5 接线态对拍未执行（提前退出或未达 gen 活跃帧）——FG 门登记面不完整判红");
        };
        let per_gen: Vec<String> = wp.per_gen_p100.iter().map(|v| format!("{v:.15e}")).collect();
        let tvals: Vec<String> = wp.t_values.iter().map(|v| format!("{v}")).collect();
        format!(
            "{{\"probe_frame\":{},\"p100\":{:.15e},\"per_gen_p100\":[{}],\"frozen_floor\":{:.15e},\"floor_source\":{},\"g26_measured_anchor\":{},\"val_ulp_err\":{:.1e},\"max_bound\":{:.15e},\"excess\":{:.15e},\"excess_ratio\":{:.6},\"in_bound\":{},\"mvn_max_abs_plus_mv\":{:.15e},\"ssim_device_vs_hostref\":{:.12},\"ssim_frame_hold_vs_hostref\":{:.12},\"ssim_beats_frame_hold\":{},\"t_values\":[{}],\"note\":{}}}",
            pf + 1,
            wp.p100,
            per_gen.join(","),
            fg_tol_v,
            jstr(&fg_tol_source),
            if fg_tol_measured.is_nan() {
                "null".to_owned()
            } else {
                format!("{fg_tol_measured:.15e}")
            },
            G31_PROBE_VAL_ULP_ERR,
            wp.max_bound,
            wp.excess,
            wp.excess_ratio,
            wp.in_bound,
            wp.mvn_max_abs_plus_mv,
            wp.ssim_device_vs_hostref,
            wp.ssim_frame_hold_vs_hostref,
            wp.ssim_beats_frame_hold,
            tvals.join(","),
            jstr("接线态对拍:device 生成帧(取反 glue 直通馈入:g14 相机 MV 经 g31_mv_negate 逐元素 IEEE 取反 + prev/cur/t 与 host 同语义)vs host 金标准 temporal::framegen::interpolate(prev, cur, −mv, t)复算;MVN vs −MV 全帧位级直比对(mvn_max_abs_plus_mv 恒 0,MV 通路硬门)。判据面:G26 冻结绝对容差(128×72 单位域合成场景标定)在 1080p HDR 生产帧上物理不适用——诊断实证(probe 帧 run_compute 三方比对 max|lane−run_compute|=0 位级,接线零缺陷):kernel/host 双方正确 f32 实现的算术差经①坐标舍入跨纹素边界翻转采样②w_cons 混合交叉项两机制放大;硬门 = 逐像素 L1 结构界 bound = frozen_floor(G26 标定 threshold 程序读) + (rangeA16+rangeB16 采样 16-texel 邻域逐通道极差,坐标翻转全覆盖) + val_ulp_err×scale + 0.5×|a−b|×w×min(1,δlog)×e^(δlog)(混合交叉项,δlog = inv_sigma2×d2 扰动上界),excess = 全帧 max(0,|dev−host|−bound) 恒 0 为绿;结构缺陷(tie-break/缓冲/MV 符号/t 错误)产生 0.1~15 量级差异必超界。SSIM 对照锚 = host 金标准复算帧(device≈hostref 则继承金标准 SSIM 胜 frame-hold 性质);G26 合成 GT 解析对拍门(绝对容差适用面;p100 ≤ 3.576e-7 冻结锚 + SSIM + 双跑位级)由 ci/g31_framegen_present_smoke.py 接线态复跑维持")
        )
    } else {
        "null".to_owned()
    };
    // ── G31+ #58 逐帧 cut 统计 sidecar（--cluster-lod leaf|on 才消费;独立
    //    JSON 文件不动既有五臂 evidence schema。measured 如实登记不设通过线）──
    if let Some((rep, _)) = &cluster_ctx {
        let n = cluster_frame_stats.len().max(1) as f64;
        let mean_tris =
            cluster_frame_stats.iter().map(|s| s.cut_tris as f64).sum::<f64>() / n;
        let (min_tris, max_tris) = cluster_frame_stats.iter().fold((u64::MAX, 0u64), |a, s| {
            (a.0.min(s.cut_tris), a.1.max(s.cut_tris))
        });
        eprintln!(
            "{GTAG}: cluster-lod 逐帧 cut 统计 frames={} cut_tris mean={:.0} min={} max={} assembled_out={} stat_ms_total={:.1}（出帧几何冻结于装配 cut;逐帧 AS 更新归 C/E 阶段）",
            cluster_frame_stats.len(),
            mean_tris,
            if min_tris == u64::MAX { 0 } else { min_tris },
            max_tris,
            rep.out_tris,
            cluster_stat_ms_total,
        );
        if let Some(path) = &cluster_stats_out {
            let mut sj = String::with_capacity(4096 + cluster_frame_stats.len() * 64);
            sj.push_str(&format!(
                "{{\"schema\":\"rurix.g31.cluster_lod_stats.v1\",\"mode\":{},\"threshold_px\":{},\"blocks\":{},\"total_clusters\":{},\"src_tris\":{},\"passthrough_tris\":{},\"assembled_cut\":{{\"clusters\":{},\"leaf_clusters\":{},\"coarse_tris\":{},\"out_tris\":{}}},\"frame_stats_note\":\"逐帧 host cut 重算 measured(相机驱动;每 16 帧覆盖性机核采样);出帧几何冻结于装配期 cut,逐帧 AS 更新归 C/E 阶段——如实登记不冒充\",\"stat_ms_total\":{:.3},\"frames\":[",
                jstr(rep.mode),
                rep.threshold_px,
                rep.blocks,
                rep.total_clusters,
                rep.src_tris,
                rep.passthrough_tris,
                rep.cut_clusters,
                rep.cut_leaf_clusters,
                rep.coarse_tris,
                rep.out_tris,
                cluster_stat_ms_total,
            ));
            for (k, s) in cluster_frame_stats.iter().enumerate() {
                if k > 0 {
                    sj.push(',');
                }
                sj.push_str(&format!(
                    "{{\"frame\":{},\"cut_clusters\":{},\"cut_leaf_clusters\":{},\"cut_tris\":{}}}",
                    s.frame, s.cut_clusters, s.cut_leaf_clusters, s.cut_tris
                ));
            }
            sj.push_str("]}");
            if let Some(parent) = Path::new(path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(path, sj.as_bytes())
                .unwrap_or_else(|e| fail(&format!("cluster 统计 sidecar 写盘 {path}: {e}")));
            eprintln!("{GTAG}: cluster-lod 统计 sidecar → {path}");
        }
    }
    // ── G37 W2 #74/#111 visbuffer 生产证据臂（--visbuffer on 才消费;循环后
    //    device 真跑机制链——真窗口相机样本 × 真场景簇包;presented 面
    //    0-byte,独立 sidecar 不动既有五臂 evidence schema,#58/#95 同律）──
    if visbuffer_opt.enabled {
        let Some((_, pack)) = &cluster_ctx else {
            fail("--visbuffer on 但簇包上下文缺失（闭集校验面破坏）");
        };
        let stats = run_visbuffer_arm(
            GTAG,
            pack,
            &visbuffer_opt,
            cluster_opt.threshold_px,
            &visbuffer_samples_taken,
        );
        visbuffer_finish(GTAG, pack, &visbuffer_opt, cluster_opt.threshold_px, &stats);
    }
    // ── G37 W3 frame_cut 合入:#77×#89 逐帧 cut→AS 更新判档臂
    //    （--cluster-per-frame-cut on 才消费;循环后重放真窗口轨迹——全簇 refit
    //    竞技场 + RQ 命中流 digest;presented 面 0-byte,独立 sidecar 不动既有
    //    五臂 evidence schema;判据/确定性协议 = g31_frame_cut_arm.rs 头注,
    //    双跑位级内建）──
    if frame_cut_opt.enabled {
        let Some((_, pack)) = &cluster_ctx else {
            fail("--cluster-per-frame-cut on 但簇包上下文缺失（闭集校验面破坏）");
        };
        let stats = run_frame_cut_arm(
            GTAG,
            pack,
            &frame_cut_pt_stream,
            &frame_cut_opt,
            cluster_opt.threshold_px,
            &frame_cut_samples_taken,
        );
        frame_cut_finish(GTAG, pack, &frame_cut_opt, cluster_opt.threshold_px, &stats);
    }
    // ── G31+ #95/#99 逐帧 WP/HLOD 统计 sidecar（--wp-hlod full|on 才消费;
    //    独立 JSON 文件不动既有五臂 evidence schema。#99 popping 指标闭集 =
    //    切换事件表 + 逐帧翻转数/三角跳变;warmup 协议机核 = flip−request ==
    //    warmup 逐事件 fail-closed 断言。measured 如实登记不设通过线）──
    if let Some((rep, ctx)) = &wp_ctx {
        // warmup 原子翻转协议机核（逐事件断言;破坏即拒——#68 互斥切换协议
        // 判据的窗口臂机器证明）。
        for e in &ctx.switch_events {
            if e.flip_frame - e.request_frame != ctx.warmup_frames {
                fail(&format!(
                    "wp-hlod warmup 协议破坏: cell {} 切换 {}→{} 间隔 {} ≠ warmup {}",
                    e.cell,
                    e.from,
                    e.to,
                    e.flip_frame - e.request_frame,
                    ctx.warmup_frames
                ));
            }
        }
        let total_switches: u64 = wp_frame_stats.iter().map(|s| u64::from(s.switches)).sum();
        let max_switches = wp_frame_stats.iter().map(|s| s.switches).max().unwrap_or(0);
        let delta_max = wp_frame_stats.iter().map(|s| s.switch_delta_tris).max().unwrap_or(0);
        let (tris_min, tris_max) = wp_frame_stats.iter().fold((u64::MAX, 0u64), |a, s| {
            (a.0.min(s.out_tris), a.1.max(s.out_tris))
        });
        eprintln!(
            "{GTAG}: wp-hlod 逐帧统计 frames={} switches={} max/frame={} delta_tris_max={} out_tris=[{},{}] stat_ms_total={:.1}（出帧几何冻结于装配选层;popping 指标 = #99 事实源）",
            wp_frame_stats.len(),
            total_switches,
            max_switches,
            delta_max,
            if tris_min == u64::MAX { 0 } else { tris_min },
            tris_max,
            wp_stat_ms_total,
        );
        if let Some(path) = &wp_stats_out {
            let mut sj = String::with_capacity(4096 + wp_frame_stats.len() * 96);
            sj.push_str(&format!(
                "{{\"schema\":\"rurix.g31.wp_hlod_stats.v1\",\"mode\":{},\"cells_total\":{},\"cells_nonempty\":{},\"levels\":{},\"warmup_frames\":{},\"src_tris\":{},\"passthrough_tris\":{},\"assembled\":{{\"full\":{},\"hlod\":{},\"culled\":{},\"pending\":{},\"out_tris\":{},\"proxy_tris\":{},\"selection_digest\":{},\"assemble_ticks\":{},\"budget_stall_frames\":{}}},\"popping\":{{\"total_switches\":{},\"max_switches_per_frame\":{},\"switch_delta_tris_max\":{},\"warmup_protocol_verified\":true}},\"frame_stats_note\":\"逐帧 host tick/选层/warmup 切换状态机 measured(相机驱动距离环流送;原子翻转协议逐事件机核);出帧几何冻结于装配期选层,逐帧 AS 更新归 #77/#89 合流窗——如实登记不冒充\",\"stat_ms_total\":{:.3},\"switch_events\":[",
                jstr(rep.mode),
                rep.cells_total,
                rep.cells_nonempty,
                ctx.pack.levels,
                ctx.warmup_frames,
                rep.src_tris,
                rep.passthrough_tris,
                rep.cells_full,
                rep.cells_hlod,
                rep.cells_culled,
                rep.cells_pending,
                rep.out_tris,
                rep.proxy_tris,
                jstr(&rep.selection_digest),
                rep.assemble_ticks,
                rep.budget_stall_frames,
                total_switches,
                max_switches,
                delta_max,
                wp_stat_ms_total,
            ));
            for (k, e) in ctx.switch_events.iter().enumerate() {
                if k > 0 {
                    sj.push(',');
                }
                sj.push_str(&format!(
                    "{{\"cell\":{},\"from\":{},\"to\":{},\"request_frame\":{},\"flip_frame\":{},\"tris_before\":{},\"tris_after\":{}}}",
                    e.cell,
                    jstr(&e.from),
                    jstr(&e.to),
                    e.request_frame,
                    e.flip_frame,
                    e.tris_before,
                    e.tris_after
                ));
            }
            sj.push_str("],\"frames\":[");
            for (k, s) in wp_frame_stats.iter().enumerate() {
                if k > 0 {
                    sj.push(',');
                }
                sj.push_str(&format!(
                    "{{\"frame\":{},\"resident\":{},\"pending_load\":{},\"full\":{},\"hlod\":{},\"culled\":{},\"switches\":{},\"switch_delta_tris\":{},\"out_tris\":{},\"budget_stall\":{}}}",
                    s.frame,
                    s.resident_cells,
                    s.pending_load,
                    s.full_cells,
                    s.hlod_cells,
                    s.culled_cells,
                    s.switches,
                    s.switch_delta_tris,
                    s.out_tris,
                    s.budget_stall
                ));
            }
            sj.push_str("]}");
            if let Some(parent) = Path::new(path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(path, sj.as_bytes())
                .unwrap_or_else(|e| fail(&format!("wp-hlod 统计 sidecar 写盘 {path}: {e}")));
            eprintln!("{GTAG}: wp-hlod 统计 sidecar → {path}");
        }
    }
    let mut ev = String::with_capacity(8192);
    ev.push('{');
    // G37 W3 fg_combo 合入：fg 分支语义前移——fg×full 组合跑落 FG evidence 面
    // （组合验收机核 = FG 双口径与不污染门;textures/realism 各臂 parity 门已由
    // 各自单臂窗绿件在案,组合窗不重验。fg on ⇒ 非 hzb/svt/slab,textures 分支
    // 加 fg off 限定与物理前移等价,schema 零新字段——REPORT §3.3）。
    if textures && fg == G31Fg::Off {
        // ── B4 纹理采样接线 schema(g31.waveB.texture;A3 游戏循环全字段 +
        //    textures 接线块;--textures on 闭集已保证 auto_move/tier=100/非 fg/
        //    非 hzb/非 slab)──
        let Some((tassets, treport)) = tex_report.as_ref() else {
            fail("B4 纹理报告缺失（evidence 面不完整判红）");
        };
        // day_0828 Phase B：静态契约相机臂合法（auto-move 硬门已解除——重锚
        // 协议 = 双跑位级一致;轨迹面仍兼容）。
        let name = auto_move.as_deref().unwrap_or("static");
        // C13 SVT 派生臂面:schema/gate 字面切换（svt on = g31.waveC.svt 面,
        // textures 块全字段继承 + svt 块追加;svt off = B4 面逐字不变）。
        ev.push_str(&format!(
            "\"schema\":{},",
            jstr(if svt_on { G31_SVT_SCHEMA } else { G31_TEXTURE_SCHEMA })
        ));
        ev.push_str(&format!(
            "\"gate\":{},",
            jstr(if svt_on { G31_SVT_GATE } else { G31_TEXTURE_GATE })
        ));
        ev.push_str(&format!("\"scene\":{},", jstr(scene_id)));
        ev.push_str(&format!("\"tier\":{tier},\"backend\":\"tsr_device\","));
        ev.push_str(&format!("\"trajectory\":{},", jstr(name)));
        ev.push_str(&format!("\"frames\":{frames},\"warmup\":{warmup},"));
        ev.push_str(&format!("\"frames_completed\":{frames_done},"));
        ev.push_str(&format!("\"exit_reason\":{},", jstr(exit_reason)));
        ev.push_str(&format!("\"resize_eras\":{resize_eras},"));
        ev.push_str(&format!("\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},"));
        ev.push_str(&format!(
            "\"internal_resolution\":{{\"w\":{},\"h\":{}}},",
            (out_w as u64 * u64::from(tier) / 100).max(1),
            (out_h as u64 * u64::from(tier) / 100).max(1)
        ));
        ev.push_str(&format!("\"real_render_frame_ms\":{r_mean:.6},"));
        ev.push_str(&format!("\"present_frame_ms\":{p_mean_json},"));
        ev.push_str(&format!("\"present_overhead_ms\":{overhead_json},"));
        ev.push_str(&format!("\"encode_frame_ms\":{encode_host_ms:.6},"));
        ev.push_str(&format!("\"digest_frame_ms\":{dg_mean:.6},"));
        ev.push_str(&format!("\"render_digest\":{},", jstr(&render_digest)));
        ev.push_str(&format!("\"digest\":{},", jstr(&presented_digest)));
        ev.push_str("\"digest_seq\":[");
        for (k, d) in digest_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&jstr(d));
        }
        ev.push_str("],");
        ev.push_str("\"ev100_seq\":[");
        for (k, v) in ev100_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!("{v}"));
        }
        ev.push_str("],");
        ev.push_str("\"camera_poses\":[");
        for (k, p) in pose_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!(
                "[{},{},{},{},{}]",
                p[0], p[1], p[2], p[3], p[4]
            ));
        }
        ev.push_str("],");
        ev.push_str(&format!(
            "\"ev100_ramp\":{},",
            match ev100_ramp {
                Some((a, b)) => format!("{{\"a\":{a},\"b\":{b}}}"),
                None => "null".to_owned(),
            }
        ));
        ev.push_str(&format!("\"headless\":{headless},"));
        ev.push_str(&format!("\"window\":{window_json},"));
        ev.push_str("\"contracts\":{\"production\":");
        ev.push_str(&format!(
            "{{\"path\":{},\"digest\":{}}},",
            jstr(&contract_path.replace('\\', "/")),
            jstr(&contract.digest)
        ));
        ev.push_str(&g10_fragment);
        ev.push_str(&format!(",\"encode_spv\":{encode_spv_json}"));
        ev.push_str("},");
        ev.push_str("\"render_includes_forced_readback\":true,");
        ev.push_str(&format!(
            "\"spv\":{},",
            unified_provenance_json(&spv_scene, &spv_mv, &spv_resample, &spv_resolve)
        ));
        // ── B4 textures 接线块（资产盘点 + 映射律法 + 图集/LUT + 探针双臂 +
        //    kernel 变体 provenance + 缺面登记）──
        let c = &tassets.census;
        let mut ms_rows = String::new();
        for (i, s) in tassets.slots.iter().enumerate() {
            if i > 0 {
                ms_rows.push(',');
            }
            let manifest_src = match &s.manifest_source_digest {
                Some(d) => jstr(d),
                None => "null".to_owned(),
            };
            let manifest_rgba8 = match &s.manifest_rgba8_digest {
                Some(d) => jstr(d),
                None => "null".to_owned(),
            };
            let digest_match = match &s.manifest_rgba8_digest {
                Some(d) => *d == s.rgba8_digest,
                None => false,
            };
            // day_0828 Phase B：heap 化行——origin 废弃;src_w/h = DDS 源
            // mip0（manifest 互核域）;width/height = 存储基级（cap-1024 档）;
            // mip_count/mip_digests = 逐级新 evidence 字段。
            let mut mip_rows = String::new();
            for (mi_, d) in s.mip_digests.iter().enumerate() {
                if mi_ > 0 {
                    mip_rows.push(',');
                }
                mip_rows.push_str(&jstr(d));
            }
            ms_rows.push_str(&format!(
                "{{\"slot\":{i},\"material_index\":{},\"material_name\":{},\"tris\":{},\"texture_uri\":{},\"width\":{},\"height\":{},\"src_width\":{},\"src_height\":{},\"dds_format\":{},\"manifest_source_digest\":{},\"rgba8_digest\":{},\"manifest_rgba8_digest\":{},\"manifest_digest_match\":{digest_match},\"mip_count\":{},\"mip_truncated\":{},\"mip_digests\":[{}],\"mod_r\":{:.9e},\"mod_g\":{:.9e},\"mod_b\":{:.9e}}}",
                s.material_index,
                jstr(&s.material_name),
                s.tris,
                jstr(&s.texture_uri),
                s.width,
                s.height,
                s.src_width,
                s.src_height,
                jstr(&s.dds_format),
                manifest_src,
                jstr(&s.rgba8_digest),
                manifest_rgba8,
                s.mip_count,
                s.mip_truncated,
                mip_rows,
                s.mod_rgb[0],
                s.mod_rgb[1],
                s.mod_rgb[2],
            ));
        }
        let manifest_matched = tassets
            .slots
            .iter()
            .filter(|s| {
                s.manifest_rgba8_digest
                    .as_ref()
                    .map(|d| *d == s.rgba8_digest)
                    .unwrap_or(false)
            })
            .count();
        ev.push_str(&format!(
            "\"textures\":{{\"census\":{{\"materials_total\":{},\"with_base_color_texture\":{},\"with_normal_texture\":{},\"with_metallic_roughness_texture\":{},\"primitives_total\":{},\"primitives_with_texcoord0\":{},\"primitives_with_tangent\":{}}},\"mapping_law\":{},\"mapped_materials\":{},\"tex_tris\":{},\"material_slots\":[{}],\"atlas\":{{\"form\":\"texel_heap\",\"cap\":{},\"mip_slots\":{},\"header_entries\":{},\"heap_texels\":{},\"heap_bytes\":{},\"format\":\"u32_packed_rgba8\",\"digest\":{}}},\"mip_law\":{},\"linlut_digest\":{},\"g11_3_manifest\":{{\"path\":\"milestones/g11/g11_3_dds_transcode_manifest.json\",\"entries_matched\":{},\"entries_total\":{}}},\"probe\":{{\"uv_law\":{},\"probe_count\":{},\"eval_ms\":{:.6},\"ssbo\":{{\"p100\":{:.15e},\"bitexact\":{},\"double_run_bitexact\":{},\"device_digest\":{},\"host_digest\":{}}},\"sampler_leg\":{{\"max_lsb_diff\":{},\"bound_lsb\":1,\"bitexact\":{},\"digest\":{},\"host_digest\":{},\"structural_basis\":{}}}}},\"quality_arms\":{{\"smooth_normals\":{},\"ggx\":{},\"lamp_lights\":{},\"lamp_gain\":{},\"lamp_k\":{},\"lamp_contrib\":{},\"bloom\":{},\"dither\":{},\"auto_exposure\":{},\"gi2\":{},\"gi2_scale\":{},\"gi2_clamp\":{}}},\"spv_texture\":{{\"path\":{},\"sha256\":{},\"no_contraction_injected\":true}},\"spv_texture_probe\":{{\"path\":{},\"sha256\":{},\"no_contraction_injected\":true}},\"gaps\":{}}},",
            c.materials_total,
            c.with_base_color_texture,
            c.with_normal_texture,
            c.with_metallic_roughness_texture,
            c.primitives_total,
            c.primitives_with_texcoord0,
            c.primitives_with_tangent,
            jstr("day_0828 Phase B 全覆盖：逐材质三角数降序 top-70 = 全 70 材质（并列时 material_index 升序）;无 baseColorTexture/零三角材质走既有常量面 0-byte"),
            tassets.slots.len(),
            tassets.tex_tris,
            ms_rows,
            G31_TEX_CAP,
            G31_TEX_MIP_SLOTS,
            tassets.heap_header_entries,
            tassets.heap_texels,
            tassets.heap_texels * 4,
            jstr(&tassets.atlas_digest),
            jstr("nearest-mip + bilinear（4 fetch）;lod = clamp(floor(log2(th·k_pix·k_tri·w_base)), 0, mips−1)——th = committed_t,k_pix = 2·tan(fovy/2)/in_h（params[50]）,k_tri = sqrt(uv_area/world_area)（tritex 步幅 2 第二槽）;级尺寸逐级 ×0.5 折半（pow2 精确）;DDS 源 mip 直搬 cap-1024 档（>cap 源从对应源级起,零重采样）"),
            jstr(&tassets.linlut_digest),
            manifest_matched,
            tassets.slots.len(),
            jstr("24/槽 = 16 网格((j*37+k*11)%256+0.5)/256,((j*101+k*13)%256+0.5)/256 + 4 精确边缘(0/0.5/1−2^-23) + 4 wrap 域(1.25/2.5/3.75/1.5/−0.25/1.3333334/2.0/−0.75);day_0828 Phase B × 抽样 mip 级 {0, mips/2, mips−1} 去重（lod 显式注入,SSBO 腿逐级对拍;sampler 腿 = lod 0 子集）;确定性闭集——ci/g31_texture_sampling_smoke.py 判读器同步归交接项"),
            treport.probe_count,
            treport.eval_ms,
            treport.ssbo_p100,
            treport.ssbo_bitexact,
            treport.ssbo_double_run_bitexact,
            jstr(&treport.ssbo_device_digest),
            jstr(&treport.ssbo_host_digest),
            treport.sampler_max_lsb,
            treport.sampler_bitexact,
            jstr(&treport.sampler_digest),
            jstr(&treport.sampler_host_digest),
            jstr("硬件线性过滤权重量化（实现近似,subtexel ≤2^-8 档）⇒ srgb 域 8-bit 量化（quantum 1/255）翻转 ≤1 LSB;host 参考 = 同式双线性（texel=n/255.0f UNORM 精确）+ (x·255+0.5).floor() 量化镜像;位级一致 = 更强终态;day_0828 Phase B 采样源 = 存储基级（cap-1024 档）"),
            smooth_nrm,
            ggx,
            lamp_lights,
            lamp_gain_v,
            lamp_k_v,
            lamp_contrib_v,
            bloom,
            dither,
            autoexp,
            gi2,
            gi2_scale_v,
            gi2_clamp_v,
            jstr(&spv_texture.replace('\\', "/")),
            jstr(&g31_file_sha(&spv_texture).unwrap_or_else(|e| fail(&e))),
            jstr(&spv_texture_probe.replace('\\', "/")),
            jstr(&g31_file_sha(&spv_texture_probe).unwrap_or_else(|e| fail(&e))),
            jstr("缺面如实登记:① sampler 对象不进 compute 生产车道——RXS-0223 §4.0-2 阶段矩阵（Texture2D/Sampler/TextureRw2D 阶段列 = fragment/vertex/raygen,compute kernel 零 image 绑定,spec 面 0-byte 纪律不扩阶段）;sampler.rs 面消费点 = 装配期 sampler 求值腿（真 image/view/SamplerDesc→VkSampler 硬件采样对拍）,生产车道采样 = SSBO texel heap + 手动双线性（G26 framegen 生产先例同律）;② normal 贴图在树 70/70（BC5）但 glTF 零 TANGENT 属性（primitives_with_tangent=0）——切线空间缺失,法线贴图着色面登记后续;③ rough-metal 贴图 0/70（无 metallicRoughnessTexture,仅 factor 常量;GGX 臂消费 factor 侧表 tri_mr）;④ --svt 与 heap 形态 fail-closed 互斥（SVT 深修归后续波）;⑤ trilinear（级间 lerp）留 flag 后补——首落 nearest-mip"),
        ));
        // ── day_0828 Phase F emissive 接线块（em on 面;off = 整块缺省,既有
        //    面 0-byte。槽行/scale 标定/manifest 三重 sha 已在装配期 fail-
        //    closed 过;g11_3_manifest 计数只覆 DDS 槽——emissive 槽互核域 =
        //    本块 bake manifest,行内 dds_format=png-rgba8-baked 自述来源）──
        if emissive_tex {
            let Some(em) = em_assets.as_ref() else {
                fail("Phase F emissive 资产缺失（evidence 面不完整判红）");
            };
            let mut em_rows = String::new();
            for (k, r) in em.rows.iter().enumerate() {
                if k > 0 {
                    em_rows.push(',');
                }
                em_rows.push_str(&format!(
                    "{{\"slot\":{},\"material_index\":{},\"material_name\":{},\"file\":{},\"source_sha256\":{},\"output_sha256\":{},\"src_width\":{},\"src_height\":{},\"stored_width\":{},\"stored_height\":{},\"mip_count\":{},\"le_linear_rgb\":[{:.9e},{:.9e},{:.9e}],\"tex_linear_mean_rgb\":[{:.9e},{:.9e},{:.9e}],\"scale_rgb\":[{:.9e},{:.9e},{:.9e}],\"tris\":{},\"fallback\":{}}}",
                    r.slot,
                    r.material_index,
                    jstr(&r.material_name),
                    jstr(&r.file),
                    jstr(&r.source_sha256),
                    jstr(&r.output_sha256),
                    r.src_width,
                    r.src_height,
                    r.stored_width,
                    r.stored_height,
                    r.mip_count,
                    r.le_linear_rgb[0],
                    r.le_linear_rgb[1],
                    r.le_linear_rgb[2],
                    r.tex_linear_mean_rgb[0],
                    r.tex_linear_mean_rgb[1],
                    r.tex_linear_mean_rgb[2],
                    r.scale_rgb[0],
                    r.scale_rgb[1],
                    r.scale_rgb[2],
                    r.tris,
                    r.fallback,
                ));
            }
            ev.push_str(&format!(
                "\"emissive\":{{\"enabled\":true,\"dir\":{},\"manifest\":{{\"path\":{},\"sha256\":{}}},\"slots\":[{}],\"em_tris\":{},\"triem_stride_f32\":1,\"triem_bytes\":{},\"heap_appended_u32\":{},\"heap_appended_bytes\":{},\"scale_law\":{},\"fallback_note\":{}}},",
                jstr(&emissive_dir.replace('\\', "/")),
                jstr(&em.manifest_path.replace('\\', "/")),
                jstr(&em.manifest_sha256),
                em_rows,
                em.em_tris,
                em.triem_bytes.len(),
                em.appended_texels,
                em.appended_texels * 4,
                jstr("scale_c = 契约 le_linear_rgb_c / bake manifest mip0 线性均值_c（能量守恒标定:屏面均值回旧常量 Le 口径）;emissive 槽 texmeta mod 位 = scale（与 albedo 槽 mod 两套语义,采样不乘 albedo mod）;通道均值 ≤1e-6 ⇒ scale=0 且该材质整体回退 mats 均值路径（triem=−1）"),
                jstr("fallback=true 行 = 该材质回退均值路径（本场景四材质均值全 >1e-6 预期零回退,出现即如实登记）"),
            ));
        }
        // ── C13 SVT 接线块（svt on 面;页表/瓦片集/池预算/探针双臂/逐帧流送
        //    统计/SPV provenance/缺面登记——svt off = 整块缺省,既有面 0-byte）──
        if svt_on {
            let Some((sassets, srep)) = svt_report.as_ref() else {
                fail("C13 SVT 报告缺失（evidence 面不完整判红）");
            };
            let Some(stream) = svt_stream.as_ref() else {
                fail("C13 SVT 流送状态缺失（evidence 面不完整判红）");
            };
            let ts = &sassets.tile_set;
            let frames_done_u = u64::from(frames_done);
            let total_px = frames_done_u
                .saturating_mul((out_w as u64) * (out_h as u64));
            let miss_rate = if total_px > 0 {
                svt_stats.miss_px_total as f64 / total_px as f64
            } else {
                0.0
            };
            // 收敛帧 = 末个 miss>0 帧之次帧（全零 miss 后缀起点;无则 null）。
            let converged_frame = match svt_stats.miss_px.iter().rposition(|&m| m > 0) {
                Some(k) if k + 1 < svt_stats.miss_px.len() => format!("{}", k + 1),
                Some(_) => "null".to_owned(),
                None => "0".to_owned(),
            };
            let seq_u32 = |v: &[u32]| -> String {
                let mut s = String::from("[");
                for (k, x) in v.iter().enumerate() {
                    if k > 0 {
                        s.push(',');
                    }
                    s.push_str(&format!("{x}"));
                }
                s.push(']');
                s
            };
            ev.push_str(&format!(
                "\"svt\":{{\"virtual_dim\":{},\"tile_dim\":{},\"border\":{},\"phys_tile_dim\":{},\"page_table_dim\":{},\"page_table_entries\":{},\"active_pages_x\":{},\"active_pages_y\":{},\"active_pages\":{},\"tile_set_digest\":{},\"page_table_digest_final\":{},\"pool_tiles\":{},\"full_residency\":{},\"phys_tile_bytes\":{},\"fallback_digest\":{},\"probe\":{{\"uv_law\":{},\"probe_count\":{},\"boundary_probe_count\":{},\"eval_ms\":{:.6},\"full_residency_arm\":{{\"p100_vs_direct\":{:.15e},\"bitexact_vs_direct\":{},\"bitexact_vs_svt_host\":{},\"double_run_bitexact\":{},\"device_digest\":{},\"host_digest\":{},\"boundary_max_abs\":{:.15e}}},\"partial_residency_arm\":{{\"law\":{},\"miss_probes\":{},\"req_bitexact\":{},\"out_bitexact\":{},\"closed_loop_loaded\":{},\"closed_loop_evicted\":{},\"closed_loop_io_bytes\":{},\"closed_loop_all_hit\":{},\"closed_loop_bitexact_vs_full\":{}}}}},\"streaming\":{{\"frames\":{},\"miss_px_total\":{},\"requested_pages_total\":{},\"tiles_loaded_total\":{},\"tiles_evicted_total\":{},\"io_bytes_total\":{},\"io_per_frame_bytes\":{},\"miss_rate\":{:.9e},\"fallback_frames\":{},\"converged_frame\":{},\"miss_px_seq\":{},\"unique_pages_seq\":{},\"loaded_seq\":{},\"evicted_seq\":{}}},\"spv_svt\":{{\"path\":{},\"sha256\":{},\"no_contraction_injected\":true}},\"spv_svt_probe\":{{\"path\":{},\"sha256\":{},\"no_contraction_injected\":true}},\"gaps\":{}}},",
                svt::SVT_VIRTUAL_DIM,
                svt::SVT_TILE_DIM,
                svt::SVT_BORDER,
                svt::SVT_PHYS_DIM,
                svt::SVT_PAGE_TABLE_DIM,
                svt::SVT_PAGE_COUNT,
                ts.pages_x,
                ts.pages_y,
                ts.page_total(),
                jstr(&ts.digest),
                jstr(&stream.page_table_digest()),
                sassets.pool_tiles,
                sassets.full_residency,
                svt::SVT_PHYS_TILE_BYTES,
                jstr(&format!("sha256:{}", sha256_hex(&sassets.fallback_bytes))),
                jstr("32/槽 = B4 24/槽基座（16 网格 + 4 精确边缘 + 4 wrap 域）+ 8 页界聚焦（128m/w×128m/h 双线性跨页 straddle + 左界 wrap straddle;pow2 槽 ⇒ UV 商 f32 精确）;确定性闭集,与 ci/g31_svt_smoke.py 判读器同源"),
                srep.probe_count,
                srep.boundary_probe_count,
                srep.eval_ms,
                srep.full_p100_vs_direct,
                srep.full_bitexact_vs_direct,
                srep.full_bitexact_vs_svt_host,
                srep.full_double_run_bitexact,
                jstr(&srep.full_device_digest),
                jstr(&srep.full_host_digest),
                srep.boundary_max_abs,
                jstr("page_id % 3 == 2 未驻留（恒等槽映射,池容 = 活动页数零驱逐噪声）;host 消费后重跑全 hit"),
                srep.partial_miss_probes,
                srep.partial_req_bitexact,
                srep.partial_out_bitexact,
                srep.closed_loop_loaded,
                srep.closed_loop_evicted,
                srep.closed_loop_io_bytes,
                srep.closed_loop_all_hit,
                srep.closed_loop_bitexact_vs_full,
                frames_done,
                svt_stats.miss_px_total,
                svt_stats.requested_pages_total,
                svt_stats.tiles_loaded_total,
                svt_stats.tiles_evicted_total,
                svt_stats.io_bytes_total,
                if frames_done_u > 0 {
                    svt_stats.io_bytes_total / frames_done_u
                } else {
                    0
                },
                miss_rate,
                svt_stats.fallback_frames,
                converged_frame,
                seq_u32(&svt_stats.miss_px),
                seq_u32(&svt_stats.unique_pages),
                seq_u32(&svt_stats.loaded),
                seq_u32(&svt_stats.evicted),
                jstr(&spv_svt.replace('\\', "/")),
                jstr(&g31_file_sha(&spv_svt).unwrap_or_else(|e| fail(&e))),
                jstr(&spv_svt_probe.replace('\\', "/")),
                jstr(&g31_file_sha(&spv_svt_probe).unwrap_or_else(|e| fail(&e))),
                jstr("缺面如实登记:① 各向异性跨瓦片 = 生产采样闭集双线性唯一过滤面（border=1 恰覆盖 2×2 footprint,aniso/mip 需求不成立——G22 SVT-3 行「border texel 复制/各向异性跨瓦片」之前者落地,后者按现消费面登记 N/A）;② 虚拟地址空间 128K² 满尺寸页表分配,活动区 = bistro 图集 3072 页（图集外恒未驻留,采样域限定图集面）;③ sampler feedback 硬件回读（TODO #85 观察行）未接——UAV 反馈缓冲面为本期合法形态"),
            ));
        }
        ev.push_str(&format!(
            "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"encode_gpu_ms\":{eg_mean:.6},\"present_cv\":{},\"present_min_ms\":{},\"present_max_ms\":{}}},",
            pstat(p_cv),
            pstat(p_min),
            pstat(p_max)
        ));
        ev.push_str(&format!("\"notes\":{}", jstr(
            "B4 纹理采样管线进生产场景面(G31+ 波 B Task B4;G31_PLUS_COMMERCIAL_RENDERER_TODO §1.2 #9):--textures on = 生产场景 kernel 纹理变体(kernels/g31_texture_gi.rx = g14_3_direct_gi.rx 逐字 fork + 贴图采样 albedo;母版 kernel/SPV 0-byte,off 面 = Stage A 回归锚);内容模型从逐三角常量 albedo 升级为贴图采样 albedo(tritex ≥ 0 槽:REPEAT wrap + G26 sample_bilinear 逐字双线性 + 256 项 srgb→linear LUT(零 pow 位级锚) × mod(factor×(1−metallic)——texture_mean_albedo 策略的逐像素泛化;tritex < 0 走既有常量面 0-byte);资产链 = gltf → top-12 律法 → DDS BC1/BC3 bin-local 解码(bcdec 镜像,逐槽 rgba8 digest == G11.3 manifest 互核) → u32 打包图集/texmeta/tritex/UV 四 SSBO 侧表扩展(mats SSBO 0-byte) + LUT SSBO;探针双臂对拍 = SSBO 腿(g31_texture_probe.rx vk::run_compute 单 dispatch,NoContraction 注入驱动 FMA 收缩禁面)device vs host 位级硬门 p100=0.0 + sampler 腿(真 GPU 纹理对象 image/view/sampler 经 sampler.rs SamplerDesc→VkSampler,vk::sampling_shaders_spv 硬件 sample_lod)vs host srgb 域参考结构容差 ≤1 LSB;digest_seq = 逐帧 BGRA8 打包帧 sha256(G31BGRA-1 前缀,device 编码域;确定性双跑位级一致为门,on≠off 为接线真实生效门);real_render_frame_ms = 五 pass 渲染墙钟(含 BGRA8 强制回读,render_includes_forced_readback=true;纹理装配/探针 = 装配期一次性,eval_ms 单列不混帧口径)"
        )));
        ev.push('}');
    } else if slab_table.is_some() {
        // ── B3 slab 接线 schema(g31.waveB.slab;A3 游戏循环全字段 + slab 接线块;
        //    --slab-table 闭集已保证 auto_move/非 fg)──
        let Some((asset, eval, n_slab)) = slab_report.as_ref() else {
            fail("B3 slab 报告缺失（evidence 面不完整判红）");
        };
        let name = auto_move.as_deref().unwrap();
        ev.push_str(&format!("\"schema\":{},", jstr(G31_SLAB_SCHEMA)));
        ev.push_str(&format!("\"gate\":{},", jstr(G31_SLAB_GATE)));
        ev.push_str(&format!("\"scene\":{},", jstr(scene_id)));
        ev.push_str(&format!("\"tier\":{tier},\"backend\":\"tsr_device\","));
        ev.push_str(&format!("\"trajectory\":{},", jstr(name)));
        ev.push_str(&format!("\"frames\":{frames},\"warmup\":{warmup},"));
        ev.push_str(&format!("\"frames_completed\":{frames_done},"));
        ev.push_str(&format!("\"exit_reason\":{},", jstr(exit_reason)));
        ev.push_str(&format!("\"resize_eras\":{resize_eras},"));
        ev.push_str(&format!("\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},"));
        ev.push_str(&format!(
            "\"internal_resolution\":{{\"w\":{},\"h\":{}}},",
            (out_w as u64 * u64::from(tier) / 100).max(1),
            (out_h as u64 * u64::from(tier) / 100).max(1)
        ));
        ev.push_str(&format!("\"real_render_frame_ms\":{r_mean:.6},"));
        ev.push_str(&format!("\"present_frame_ms\":{p_mean_json},"));
        ev.push_str(&format!("\"present_overhead_ms\":{overhead_json},"));
        ev.push_str(&format!("\"encode_frame_ms\":{encode_host_ms:.6},"));
        ev.push_str(&format!("\"digest_frame_ms\":{dg_mean:.6},"));
        ev.push_str(&format!("\"render_digest\":{},", jstr(&render_digest)));
        ev.push_str(&format!("\"digest\":{},", jstr(&presented_digest)));
        ev.push_str("\"digest_seq\":[");
        for (k, d) in digest_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&jstr(d));
        }
        ev.push_str("],");
        ev.push_str("\"ev100_seq\":[");
        for (k, v) in ev100_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!("{v}"));
        }
        ev.push_str("],");
        ev.push_str("\"camera_poses\":[");
        for (k, p) in pose_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!(
                "[{},{},{},{},{}]",
                p[0], p[1], p[2], p[3], p[4]
            ));
        }
        ev.push_str("],");
        ev.push_str(&format!(
            "\"ev100_ramp\":{},",
            match ev100_ramp {
                Some((a, b)) => format!("{{\"a\":{a},\"b\":{b}}}"),
                None => "null".to_owned(),
            }
        ));
        ev.push_str(&format!("\"headless\":{headless},"));
        ev.push_str(&format!("\"window\":{window_json},"));
        ev.push_str("\"contracts\":{\"production\":");
        ev.push_str(&format!(
            "{{\"path\":{},\"digest\":{}}},",
            jstr(&contract_path.replace('\\', "/")),
            jstr(&contract.digest)
        ));
        ev.push_str(&g10_fragment);
        ev.push_str(&format!(",\"encode_spv\":{encode_spv_json}"));
        ev.push_str("},");
        ev.push_str("\"render_includes_forced_readback\":true,");
        ev.push_str(&format!(
            "\"spv\":{},",
            unified_provenance_json(&spv_scene, &spv_mv, &spv_resample, &spv_resolve)
        ));
        // ── slab 接线块（16 槽 ABI + 双臂求值对拍 + 逐三角施加面）──
        let mut ms_rows = String::new();
        for (i, (mi, slot)) in asset.material_slots.iter().enumerate() {
            if i > 0 {
                ms_rows.push(',');
            }
            ms_rows.push_str(&format!(
                "{{\"material_index\":{mi},\"slot\":{slot},\"device_r\":{:.9e},\"host_r\":{:.15e}}}",
                f64::from(eval.device_r[*slot as usize]),
                eval.host_r[*slot as usize],
            ));
        }
        ev.push_str(&format!(
            "\"slab\":{{\"table_path\":{},\"table_sha256\":{},\"asset_scene_id\":{},\"abi_digest\":{},\"n_slots\":16,\"slot_abi\":\"[rc f32 LE, ab f32 LE] × 16（G29 M-b 生成律 rc_k=k/15·0.95、ab_k=(15−k)/15 同源）\",\"arm\":{},\"spv_slab\":{{\"path\":{},\"sha256\":{}}},\"parity_p100\":{:.15e},\"eval_ms\":{:.6},\"device_digest\":{},\"host_digest\":{},\"mapped_materials\":{},\"slab_tris\":{},\"material_slots\":[{}],\"finiteness_first_class\":true,\"semantics\":{}}},",
            jstr(&asset.path.replace('\\', "/")),
            jstr(&g31_file_sha(&asset.path).unwrap_or_else(|e| fail(&e))),
            jstr(&asset.scene_id),
            jstr(&asset.abi_digest),
            jstr(&slab_arm),
            jstr(&spv_slab.replace('\\', "/")),
            jstr(&g31_file_sha(&spv_slab).unwrap_or_else(|e| fail(&e))),
            eval.parity_p100,
            eval.eval_ms,
            jstr(&eval.device_digest),
            jstr(&eval.host_digest),
            asset.material_slots.len(),
            n_slab,
            ms_rows,
            jstr("albedo_final[c] = albedo_dir[c] × R_slot（f32 乘;R = 双层 slab 闭式 total_reflectance;emission 0-byte;非映射材质走既有逐三角 albedo/emission 单层面 0-byte;parity_p100 = 逐槽 |device f32 − host f64| 最大值,G29 M-b 逐槽对拍口径,有限性一等断言先于聚合）"),
        ));
        ev.push_str(&format!(
            "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"encode_gpu_ms\":{eg_mean:.6},\"present_cv\":{},\"present_min_ms\":{},\"present_max_ms\":{}}},",
            pstat(p_cv),
            pstat(p_min),
            pstat(p_max)
        ));
        ev.push_str(&format!("\"notes\":{}", jstr(
            "B3 slab 材质侧表生产接线面(G31+ 波 B Task B3;RD-041-slab 行 g31_anchor 生产接线窗兑现):--slab-table 资产文件驱动(G29 M-b bin-local 16 槽侧表的资产化升级;schema/域/槽序/ABI digest 闭集校验篡改即拒)场景中 Substrate 类双层 slab 材质经 kernels/g29_slab.rx(device 臂;G29 M-a 本体 0-byte 冻结消费,dispatch [16,1,1] 逐槽单 invocation)或 material/slab.rs::total_reflectance(host 参考臂金标准 f64 直调 0-byte)侧表 16 槽查表求值,逐三角 albedo 预调制(albedo×R_slot f32)后进既有 mats SSBO 面——生产 kernel/管线 0-byte,非 slab 材质走既有单层面 0-byte;parity_p100 = device vs host 逐槽对拍(G29 M-b 口径;冻结容差 milestones/g29/g29_budget.json g29.slab_device.host_device_reflectance_tol 程序读, measured p100=1.192e-7 恰一 ULP);digest_seq = 逐帧 BGRA8 打包帧 sha256(G31BGRA-1 前缀,device 编码域;确定性双跑位级一致为门,on≠off 为接线真实生效门);real_render_frame_ms = 五 pass 渲染墙钟(含 BGRA8 强制回读,render_includes_forced_readback=true;slab 求值 = 装配期一次性,eval_ms 单列不混帧口径)"
        )));
        ev.push('}');
    } else if fg != G31Fg::Off {
        // ── A5 FG 接线 schema(g31.waveA.framegen;双口径分离 + 恒等式组 +
        //    接线态对拍;--fg 闭集已保证 auto_move/tier=100/非 headless)──
        let name = auto_move.as_deref().unwrap();
        ev.push_str(&format!("\"schema\":{},", jstr(G31_FRAMEGEN_SCHEMA)));
        ev.push_str(&format!("\"gate\":{},", jstr(G31_FRAMEGEN_GATE)));
        ev.push_str(&format!("\"scene\":{},", jstr(scene_id)));
        ev.push_str(&format!("\"tier\":{tier},\"backend\":\"tsr_device\","));
        ev.push_str(&format!("\"trajectory\":{},", jstr(name)));
        ev.push_str(&format!("\"frames\":{frames},\"warmup\":{warmup},"));
        ev.push_str(&format!("\"frames_completed\":{frames_done},"));
        ev.push_str(&format!("\"exit_reason\":{},", jstr(exit_reason)));
        ev.push_str(&format!("\"resize_eras\":{resize_eras},"));
        ev.push_str(&format!("\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},"));
        ev.push_str(&format!(
            "\"internal_resolution\":{{\"w\":{},\"h\":{}}},",
            (out_w as u64 * u64::from(tier) / 100).max(1),
            (out_h as u64 * u64::from(tier) / 100).max(1)
        ));
        ev.push_str(&format!("\"fg_mode\":{},", jstr(fg.name())));
        ev.push_str(&format!(
            "\"fg_factor\":{},\"inserted_per_pair\":{},",
            fg.factor(),
            fg.inserted()
        ));
        ev.push_str(&format!(
            "\"real_frames\":{real_frames},\"generated_frames\":{generated_frames},\"presented_frames\":{presented_frames},"
        ));
        ev.push_str(&format!("\"real_render_frame_ms\":{r_mean:.6},"));
        ev.push_str(&format!("\"real_render_seconds\":{real_render_seconds:.9},"));
        ev.push_str(&format!("\"real_render_fps\":{real_render_fps:.6},"));
        ev.push_str(&format!("\"present_frame_ms\":{p_mean_json},"));
        ev.push_str(&format!("\"present_seconds\":{present_seconds:.9},"));
        ev.push_str(&format!("\"presented_fps\":{presented_fps:.6},"));
        ev.push_str(&format!("\"encode_frame_ms\":{encode_host_ms:.6},"));
        ev.push_str(&format!("\"digest_frame_ms\":{dg_mean:.6},"));
        ev.push_str(&format!("\"render_digest\":{},", jstr(&render_digest)));
        ev.push_str(&format!("\"digest\":{},", jstr(&presented_digest)));
        ev.push_str("\"digest_seq\":[");
        for (k, d) in digest_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&jstr(d));
        }
        ev.push_str("],");
        ev.push_str("\"ev100_seq\":[");
        for (k, v) in ev100_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!("{v}"));
        }
        ev.push_str("],");
        ev.push_str("\"camera_poses\":[");
        for (k, p) in pose_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!(
                "[{},{},{},{},{}]",
                p[0], p[1], p[2], p[3], p[4]
            ));
        }
        ev.push_str("],");
        ev.push_str(&format!(
            "\"ev100_ramp\":{},",
            match ev100_ramp {
                Some((a, b)) => format!("{{\"a\":{a},\"b\":{b}}}"),
                None => "null".to_owned(),
            }
        ));
        ev.push_str(&format!("\"headless\":{headless},"));
        ev.push_str(&format!("\"window\":{window_json},"));
        ev.push_str("\"contracts\":{\"production\":");
        ev.push_str(&format!(
            "{{\"path\":{},\"digest\":{}}},",
            jstr(&contract_path.replace('\\', "/")),
            jstr(&contract.digest)
        ));
        ev.push_str(&g10_fragment);
        ev.push_str(&format!(",\"encode_spv\":{encode_spv_json}"));
        ev.push_str(&format!(",\"framegen_spv\":{framegen_spv_json}"));
        ev.push_str("},");
        ev.push_str("\"render_includes_forced_readback\":true,");
        ev.push_str(&format!(
            "\"spv\":{},",
            unified_provenance_json(&spv_scene, &spv_mv, &spv_resample, &spv_resolve)
        ));
        ev.push_str(&format!("\"wired_parity\":{wired_parity_json},"));
        ev.push_str(&format!(
            "\"caliber_identities\":{{\"presented_eq_real_plus_generated\":{},\"real_fps_recompute_ok\":{},\"real_fps_isolated_from_generated_ok\":{},\"presented_fps_recompute_ok\":{},\"digest_seq_len_eq_real_frames_total\":{}}},",
            id_presented, id_real_recompute, id_real_isolated, id_presented_recompute, id_digest_seq_len
        ));
        ev.push_str(&format!(
            "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"encode_gpu_ms\":{eg_mean:.6},\"fg_gpu_ms\":{fgg_mean:.6},\"render5_gpu_ms\":{r5g_mean:.6},\"present_cv\":{},\"present_min_ms\":{},\"present_max_ms\":{}}},",
            pstat(p_cv),
            pstat(p_min),
            pstat(p_max)
        ));
        ev.push_str(&format!("\"notes\":{}", jstr(
            "A5 FG/MFG 生产接线面(G30 承接锚 G13-N7 行兑现):--fg x2/x3 将 G26 device kernel g26_framegen.rx 链接入呈现车道(八/十 pass:生产五 pass 0-byte + g31_mv_negate 取反 glue + fg kernel + display_encode 复用),present 序 = 生成帧(t 升序)→ 真帧;MV 馈入 = g14_mv 相机 MV 经 g31_mv_negate 逐元素 IEEE 取反(零数值误差)后直通(prev/cur/−mv/t 与 host 金标准逐字同语义,含 t=0.5 near tie-break)——MV 仅含相机运动+静态场景深度重投影,运动物体 MV 缺口为 A4 已登记项如实登记不冒充(bistro 静态场景面,dyn 实例场景 FG 不接);--fg 闭集 = --auto-move + tier=100(MV 与 out_color 同栅格;tier<100 MV 重采样非本任务面)。双口径(G13-N7 字面纪律):real_render_frame_ms/real_render_fps 只由真渲帧构成(生成帧禁入计数;单提交墙钟含 FG GPU 段,telemetry 分列 stats.render5_gpu_ms/stats.fg_gpu_ms);presented_fps = presented_frames ÷ (real_render_seconds + present_seconds) 独立新口径,与真实渲染帧率并列输出永不混算;caliber_identities 恒等式组 schema 层钉 const true。digest_seq = 逐真渲帧 BGRA8 sha256(G31BGRA-1;fg on/off 同轨迹位级一致 = FG 不回污染渲染车道机核门);digest_frame_ms = 真渲帧 sha256 税单列。wired_parity = 接线态对拍(probe 帧 host 金标准复算;p100 ≤ G26 冻结容差程序读 + SSIM(device,hostref) > SSIM(frame-hold,hostref));G26 合成 GT 对拍门(p100 + SSIM + 双跑位级)由 ci/g31_framegen_present_smoke.py 接线态复跑维持。G37 W3 fg_combo:fg×--quality full 组合面 = FG 插值 post-bloom comp parity 对(composite/encode/AE reduce/FG 逐帧同 parity 槽,FULL 下标族 48..=56 按 TEXNRM_BLOOM_RIS+AE 终态定死),AE 增益经 enc_params[133] 生成帧同读继承,真实帧数值逐位不变 ⇒ digest_seq 不污染门跨 full 面 fg on/off 维持;probe 对拍 prev/cur 换 comp 对回读,判据面公式零改动;两点式闭集 = 全画质 off base ∪ full 预设字面,散臂混搭 fail-closed;screen-space 辉光被场景 MV warp 属商业 FG 已知近似,如实登记"
        )));
        ev.push('}');
    } else if hzb == G31Hzb::On {
        // ── B1 HZB 接线 schema(g31.waveB.hzb;A3 游戏循环全字段 + hzb 接线块;
        //    trajectory = auto_move 名或 "static"〔静态相机测量腿〕)──
        let Some((wp, pf)) = hzb_wired_parity.as_ref() else {
            fail("B1 接线态对拍未执行（提前退出或未达 probe 帧）——HZB 门登记面不完整判红");
        };
        let (hzg_mean, _, _, _, _) = if hzb_gpu_ms.is_empty() {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            g31_stats(&hzb_gpu_ms)
        };
        let (hzs_mean, _, _, _, _) = if hzb_scene_gpu_ms.is_empty() {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            g31_stats(&hzb_scene_gpu_ms)
        };
        let (hzh_mean, _, _, _, _) = if hzb_host_ms.is_empty() {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            g31_stats(&hzb_host_ms)
        };
        let (hzc_mean, _, _, _, _) = if hzb_closure_gpu_ms.is_empty() {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            g31_stats(&hzb_closure_gpu_ms)
        };
        let visible_mean = if frames_done > 0 {
            hzb_visible_sum as f64 / f64::from(frames_done)
        } else {
            0.0
        };
        ev.push_str(&format!("\"schema\":{},", jstr(G31_HZB_SCHEMA)));
        ev.push_str(&format!("\"gate\":{},", jstr(G31_HZB_GATE)));
        ev.push_str(&format!("\"scene\":{},", jstr(scene_id)));
        ev.push_str(&format!("\"tier\":{tier},\"backend\":\"tsr_device\","));
        ev.push_str(&format!(
            "\"trajectory\":{},",
            jstr(auto_move.as_deref().unwrap_or("static"))
        ));
        ev.push_str(&format!("\"frames\":{frames},\"warmup\":{warmup},"));
        ev.push_str(&format!("\"frames_completed\":{frames_done},"));
        ev.push_str(&format!("\"exit_reason\":{},", jstr(exit_reason)));
        ev.push_str(&format!("\"resize_eras\":{resize_eras},"));
        ev.push_str(&format!("\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},"));
        ev.push_str(&format!(
            "\"internal_resolution\":{{\"w\":{},\"h\":{}}},",
            (out_w as u64 * u64::from(tier) / 100).max(1),
            (out_h as u64 * u64::from(tier) / 100).max(1)
        ));
        ev.push_str(&format!("\"real_render_frame_ms\":{r_mean:.6},"));
        ev.push_str(&format!("\"present_frame_ms\":{p_mean_json},"));
        ev.push_str(&format!("\"present_overhead_ms\":{overhead_json},"));
        ev.push_str(&format!("\"encode_frame_ms\":{encode_host_ms:.6},"));
        ev.push_str(&format!("\"digest_frame_ms\":{dg_mean:.6},"));
        ev.push_str(&format!("\"render_digest\":{},", jstr(&render_digest)));
        ev.push_str(&format!("\"digest\":{},", jstr(&presented_digest)));
        ev.push_str("\"digest_seq\":[");
        for (k, d) in digest_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&jstr(d));
        }
        ev.push_str("],");
        ev.push_str("\"ev100_seq\":[");
        for (k, v) in ev100_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!("{v}"));
        }
        ev.push_str("],");
        ev.push_str("\"camera_poses\":[");
        for (k, p) in pose_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!("[{},{},{},{},{}]", p[0], p[1], p[2], p[3], p[4]));
        }
        ev.push_str("],");
        ev.push_str(&format!(
            "\"ev100_ramp\":{},",
            match ev100_ramp {
                Some((a, b)) => format!("{{\"a\":{a},\"b\":{b}}}"),
                None => "null".to_owned(),
            }
        ));
        ev.push_str(&format!("\"headless\":{headless},"));
        ev.push_str(&format!("\"window\":{window_json},"));
        // 契约链 + 编码 SPV + HZB kernel 五件 provenance（g27 两件 0-byte
        // 冻结消费——sha256 为内容面,.tmp 构建产物）。
        let hzb_spv_sha = |p: &str| {
            format!(
                "{{\"path\":{},\"sha256\":{}}}",
                jstr(&p.replace('\\', "/")),
                jstr(&g31_file_sha(p).unwrap_or_else(|e| fail(&e)))
            )
        };
        ev.push_str("\"contracts\":{\"production\":");
        ev.push_str(&format!(
            "{{\"path\":{},\"digest\":{}}},",
            jstr(&contract_path.replace('\\', "/")),
            jstr(&contract.digest)
        ));
        ev.push_str(&g10_fragment);
        ev.push_str(&format!(",\"encode_spv\":{encode_spv_json}"));
        ev.push_str(&format!(
            ",\"hzb_spv\":{{\"primary\":{},\"shade\":{},\"pack\":{},\"reduce\":{},\"test\":{}}}",
            hzb_spv_sha(&spv_hzb_primary),
            hzb_spv_sha(&spv_hzb_shade),
            hzb_spv_sha(&spv_hzb_pack),
            hzb_spv_sha(&spv_hzb_reduce),
            hzb_spv_sha(&spv_hzb_test)
        ));
        ev.push_str("},");
        ev.push_str("\"render_includes_forced_readback\":true,");
        ev.push_str(&format!(
            "\"spv\":{},",
            unified_provenance_json(&spv_hzb_primary, &spv_mv, &spv_resample, &spv_resolve)
        ));
        // ── hzb 接线块（拓扑元信息 + 剔除计数 + 两阶段闭环 + 接线态对拍）──
        ev.push_str(&format!(
            "\"hzb\":{{\"mode\":\"on\",\"meta\":{},\"occlusion\":{{\"tested_p1\":{},\"occluded_p1\":{},\"offscreen\":{},\"retested_p2\":{},\"flipped_p2\":{},\"closure_frames\":{},\"closure_extra_submits\":{},\"closure_full_fallbacks\":{},\"visible_mean\":{:.6}}},\"parity\":{{\"probe_frame\":{},\"mips\":{},\"n_rects\":{},\"mips_bitexact\":{},\"verdict_sequence_equal\":{},\"false_positives\":{},\"occluded\":{},\"pyramid_digest\":{},\"host_pyramid_digest\":{},\"pyramid_digest_equal_host\":{},\"verdict_digest\":{},\"host_verdict_digest\":{},\"verdict_digest_equal_host\":{}}}}},",
            hzb_meta_json,
            hzb_tested,
            hzb_occluded,
            hzb_offscreen,
            hzb_retested,
            hzb_flipped,
            hzb_closure_frames,
            hzb_closure_submits,
            hzb_fallbacks,
            visible_mean,
            pf + 1,
            wp.mips,
            wp.n_rects,
            wp.mips_bitexact,
            wp.verdict_equal,
            wp.false_positives,
            wp.occluded,
            jstr(&wp.pyramid_digest),
            jstr(&wp.host_pyramid_digest),
            wp.pyramid_digest == wp.host_pyramid_digest,
            jstr(&wp.verdict_digest),
            jstr(&wp.host_verdict_digest),
            wp.verdict_digest == wp.host_verdict_digest,
        ));
        ev.push_str(&format!(
            "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"scene_gpu_ms\":{hzs_mean:.6},\"hzb_gpu_ms\":{hzg_mean:.6},\"closure_extra_gpu_ms\":{hzc_mean:.6},\"hzb_host_ms\":{hzh_mean:.6},\"encode_gpu_ms\":{eg_mean:.6},\"present_cv\":{},\"present_min_ms\":{},\"present_max_ms\":{}}},",
            pstat(p_cv),
            pstat(p_min),
            pstat(p_max)
        ));
        ev.push_str(&format!("\"notes\":{}", jstr(
            "B1 HZB 遮挡剔除生产接线面(G31+ 波 B Task B1;G30 承接锚 G27 行「生产接线窗」+ RFC-0044 §5.8 两阶段第二段〔F10 补项〕兑现;G31_PLUS_COMMERCIAL_RENDERER_TODO §1.2 #6 行):--hzb on = bistro 逐 mesh 节点 BLAS 分解(1186 实例;tris/mats SSBO 与单 BLAS 生产面位级同 buffer,g31_hzb_primary 经 inst_base 前缀和把 (inst,prim) 映回全局下标) + 双 TLAS(表 0 = 初剔后〔逐帧实例掩码 tlas_update〕供相机射线,表 1 = 全量零剔除供阴影射线——遮挡物阴影正确性面,RXS-0297 单 TLAS 签名纪律下拆 pass 兑现) + 帧内金字塔轮换(pass 序 = primary→shade→mv→tsr×2→encode→test_p1〔全实例 rect vs 上帧金字塔=「上帧金字塔初剔」字面〕→g27_hzb_reduce×(L−1)+g31_hzb_pack×L〔本帧重建,g27 两 kernel 0-byte 冻结消费〕→test_p2〔上帧被剔集 vs 本帧金字塔=「本帧重建重测」字面〕) + 闭环重渲(collect 结算应见集 = p1 可见 ∪ p2 翻回;应见而有未渲者 ⇒ 掩码并集同帧重渲,迭代 ≤4 未收敛 ⇒ 全掩码兜底=零剔除精确收敛——漏剔合法零害/误剔必翻回补渲,剔除零假阳性 ⇒ 闭环后画面与分解车道全集渲染位级一致,由 RURIX_HZB_ALL_VISIBLE 登记实验臂 digest_seq 逐帧对拍机核门承载;on vs off 关系 = 分解/双 TLAS 结构 ULP 噪声(全可见实验臂同 digest 钉死剔除中性,位级全等结构上不可达,如实登记);剔除链深度域 = 真 ZO NDC(depth_hz 专用面 = g31_hzb_shade ④b 段 vp 行 2/3 另算——U_SCENE_DEPTH 沿用 g14_3_shade_reduce 参数行 25..32 生产字面供 MV/TSR 两路并存;近面内几何 z_ndc<0 合法入塔,nearest 只钳上界 1.0 保负值 ⇒ 严格不等式自遮挡结构上不可达);host 金标准面只读消费 0-byte(geometry/{hzb,cull}.rs:Frustum 视锥离屏第一关 + probe 帧 HzbPyramid::build/test_rect/exact_rect_occluded 复算对拍——hzb.parity 三块硬门 harness fail-fast);深度约定 = standard-Z(小值近/miss=1.0 远,conv=1.0);real_render_frame_ms = 生产链渲染墙钟(含 BGRA8 强制回读 + 逐帧判定小回读〔2×N×4B〕,render_includes_forced_readback=true;闭环重渲墙钟含内——closure_extra_gpu_ms 单列强加 GPU 段);stats.scene_gpu_ms = primary+shade 末次提交 GPU;hzb_gpu_ms = 剔除链 GPU(test×2+reduce+pack 全提交累计);hzb_host_ms = host 初剔分类段;measurement 对照腿 = --hzb off/on 同窗静态相机 ≥100 帧(由 ci/g31_hzb_wiring_smoke.py 裁决)"
        )));
        ev.push('}');
    } else if auto_move.is_some() {
        // ── A3 游戏循环 schema(g31.waveA.gameloop)──
        let name = auto_move.as_deref().unwrap();
        ev.push_str(&format!("\"schema\":{},", jstr(G31_GAMELOOP_SCHEMA)));
        ev.push_str(&format!("\"gate\":{},", jstr(G31_GAMELOOP_GATE)));
        ev.push_str(&format!("\"scene\":{},", jstr(scene_id)));
        ev.push_str(&format!("\"tier\":{tier},\"backend\":\"tsr_device\","));
        ev.push_str(&format!("\"trajectory\":{},", jstr(name)));
        ev.push_str(&format!("\"frames\":{frames},\"warmup\":{warmup},"));
        ev.push_str(&format!("\"frames_completed\":{frames_done},"));
        ev.push_str(&format!("\"exit_reason\":{},", jstr(exit_reason)));
        ev.push_str(&format!("\"resize_eras\":{resize_eras},"));
        ev.push_str(&format!("\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},"));
        ev.push_str(&format!(
            "\"internal_resolution\":{{\"w\":{},\"h\":{}}},",
            (out_w as u64 * u64::from(tier) / 100).max(1),
            (out_h as u64 * u64::from(tier) / 100).max(1)
        ));
        ev.push_str(&format!("\"real_render_frame_ms\":{r_mean:.6},"));
        ev.push_str(&format!("\"present_frame_ms\":{p_mean_json},"));
        ev.push_str(&format!("\"present_overhead_ms\":{overhead_json},"));
        ev.push_str(&format!("\"encode_frame_ms\":{encode_host_ms:.6},"));
        ev.push_str(&format!("\"digest_frame_ms\":{dg_mean:.6},"));
        ev.push_str(&format!("\"render_digest\":{},", jstr(&render_digest)));
        ev.push_str(&format!("\"digest\":{},", jstr(&presented_digest)));
        ev.push_str("\"digest_seq\":[");
        for (k, d) in digest_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&jstr(d));
        }
        ev.push_str("],");
        ev.push_str("\"ev100_seq\":[");
        for (k, v) in ev100_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!("{v}"));
        }
        ev.push_str("],");
        ev.push_str("\"camera_poses\":[");
        for (k, p) in pose_seq.iter().enumerate() {
            if k > 0 {
                ev.push(',');
            }
            ev.push_str(&format!(
                "[{},{},{},{},{}]",
                p[0], p[1], p[2], p[3], p[4]
            ));
        }
        ev.push_str("],");
        ev.push_str(&format!(
            "\"ev100_ramp\":{},",
            match ev100_ramp {
                Some((a, b)) => format!("{{\"a\":{a},\"b\":{b}}}"),
                None => "null".to_owned(),
            }
        ));
        ev.push_str(&format!("\"headless\":{headless},"));
        ev.push_str(&format!("\"window\":{window_json},"));
        ev.push_str("\"contracts\":{\"production\":");
        ev.push_str(&format!(
            "{{\"path\":{},\"digest\":{}}},",
            jstr(&contract_path.replace('\\', "/")),
            jstr(&contract.digest)
        ));
        ev.push_str(&g10_fragment);
        ev.push_str(&format!(",\"encode_spv\":{encode_spv_json}"));
        ev.push_str("},");
        ev.push_str("\"render_includes_forced_readback\":true,");
        ev.push_str(&format!(
            "\"spv\":{},",
            unified_provenance_json(&spv_scene, &spv_mv, &spv_resample, &spv_resolve)
        ));
        ev.push_str(&format!(
            "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"encode_gpu_ms\":{eg_mean:.6},\"present_cv\":{},\"present_min_ms\":{},\"present_max_ms\":{}}},",
            pstat(p_cv),
            pstat(p_min),
            pstat(p_max)
        ));
        ev.push_str(&format!("\"notes\":{}", jstr(
            "A3 游戏循环面:digest_seq = 逐帧 BGRA8 打包帧 sha256(G31BGRA-1 前缀;device 编码域——A1 host f64 编码域 digest 语义不冒充同值);轨迹 orbit/dolly 全参数 f64 帧号驱动,双跑位级一致为确定性门,异轨迹 digest_seq 不同为相机真实生效门(防确定性的坏内容);ev100_seq 逐帧曝光(auto-move --ev100-ramp 坡 / 契约值),经 128B TSR 参数逐帧 uniform 上传;real_render_frame_ms = 五 pass 渲染墙钟(含 BGRA8 8.3MB 强制回读,render_includes_forced_readback=true;不含 present);encode_frame_ms = host 编码墙钟恒 0(device 编码;GPU 耗时分列 stats.encode_gpu_ms);digest_frame_ms = 逐帧 sha256 税单列;camera_poses = [x,y,z,yaw,pitch]×帧;--headless-smoke 无窗口退化仅供自检不计真门"
        )));
        ev.push('}');
    } else {
        // ── A1 默认面 schema(g31.waveA.present;顶层键闭集 0-byte)──
        ev.push_str(&format!("\"schema\":{},", jstr(G31_SCHEMA)));
        ev.push_str(&format!("\"gate\":{},", jstr(G31_GATE)));
        ev.push_str(&format!("\"scene\":{},", jstr(scene_id)));
        ev.push_str(&format!("\"tier\":{tier},\"backend\":\"tsr_device\","));
        ev.push_str(&format!("\"frames\":{frames},\"warmup\":{warmup},"));
        ev.push_str(&format!("\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},"));
        ev.push_str(&format!(
            "\"internal_resolution\":{{\"w\":{},\"h\":{}}},",
            (out_w as u64 * u64::from(tier) / 100).max(1),
            (out_h as u64 * u64::from(tier) / 100).max(1)
        ));
        ev.push_str(&format!("\"real_render_frame_ms\":{r_mean:.6},"));
        ev.push_str(&format!("\"present_frame_ms\":{p_mean_json},"));
        ev.push_str(&format!("\"present_overhead_ms\":{overhead_json},"));
        ev.push_str(&format!("\"encode_frame_ms\":{encode_host_ms:.6},"));
        ev.push_str(&format!("\"render_digest\":{},", jstr(&render_digest)));
        ev.push_str(&format!("\"digest\":{},", jstr(&presented_digest)));
        ev.push_str(&format!("\"headless\":{headless},"));
        ev.push_str(&format!("\"window\":{window_json},"));
        ev.push_str("\"contracts\":{\"production\":");
        ev.push_str(&format!(
            "{{\"path\":{},\"digest\":{}}},",
            jstr(&contract_path.replace('\\', "/")),
            jstr(&contract.digest)
        ));
        ev.push_str(&g10_fragment);
        ev.push_str(&format!(",\"encode_spv\":{encode_spv_json}"));
        ev.push_str("},");
        ev.push_str("\"render_includes_forced_readback\":true,");
        ev.push_str(&format!(
            "\"spv\":{},",
            unified_provenance_json(&spv_scene, &spv_mv, &spv_resample, &spv_resolve)
        ));
        ev.push_str(&format!(
            "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"encode_gpu_ms\":{eg_mean:.6},\"present_cv\":{},\"present_min_ms\":{},\"present_max_ms\":{}}},",
            pstat(p_cv),
            pstat(p_min),
            pstat(p_max)
        ));
        ev.push_str(&format!("\"notes\":{}", jstr(
            "real_render_frame_ms = 生产管线五 pass 渲染耗时(不含 present;含 present 强制的 BGRA8 8.3MB 回读段——生产帧本零回读,强制回读税如实登记 render_includes_forced_readback=true);present_frame_ms = acquire→copy→present→idle 纯 present 腿;present_overhead_ms = encode(=0:A3 device 侧显示编码落地,ACES1.3 RRT+ODT f32 移植 + BT.1886 于第五 pass 链内完成,GPU 耗时分列 stats.encode_gpu_ms;digest 语义 = device BGRA8 域 G31BGRA-1,A1 host f64 编码域 digest 不冒充同值)+present 腿;真实渲染帧率口径禁混 present 开销;游戏循环最小面:WASD/QE 平移 + mouse/方向键视角 + -/= 曝光(逐帧 192B/128B uniform 通路)+ WM_SIZE resize extent 联动 + 最小化跳过 + ESC/关闭干净退出;--headless-smoke 无窗口退化仅供自检逻辑用不计真门(present 口径 null)"
        )));
        ev.push('}');
    }
    let evidence = ev;
    if evidence_path.is_empty() {
        // G37 W3 fg_combo 合入：textures 分支加 fg off 限定（fg 分支语义前移
        // 同律——fg×full 组合默认落 FG evidence 路径）。
        evidence_path = if svt_on {
            "evidence/g31_svt.json".to_owned()
        } else if textures && fg == G31Fg::Off {
            "evidence/g31_texture_sampling.json".to_owned()
        } else if slab_table.is_some() {
            "evidence/g31_slab_wiring.json".to_owned()
        } else if fg != G31Fg::Off {
            "evidence/g31_framegen_present.json".to_owned()
        } else if auto_move.is_some() {
            "evidence/g31_game_loop.json".to_owned()
        } else {
            "evidence/g31_window_present.json".to_owned()
        };
    }
    if let Some(parent) = Path::new(&evidence_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|e| fail(&format!("evidence 目录: {e}")));
        }
    }
    std::fs::write(&evidence_path, format!("{evidence}\n"))
        .unwrap_or_else(|e| fail(&format!("evidence 写入: {e}")));
    eprintln!("{GTAG}: evidence → {}", evidence_path.replace('\\', "/"));

    // ── C7 profiler 输出面（--profile-json;机器可读逐 pass 分解独立落盘——
    //    evidence 面 0-byte,默认关 = 零收集零写盘）──
    if let Some(pj_path) = profile_json.as_deref() {
        let t_prof = std::time::Instant::now();
        let labels_active = debug_labels_active.unwrap_or(false);
        let pj = match g31_profile_json(
            &profile_frames,
            scene_id,
            tier,
            out_w,
            out_h,
            (out_w as u64 * u64::from(tier) / 100).max(1) as u32,
            (out_h as u64 * u64::from(tier) / 100).max(1) as u32,
            warmup,
            headless,
            labels_active,
            &render_digest,
            t_prof,
        ) {
            Ok(s) => s,
            Err(e) => fail(&e),
        };
        if let Some(parent) = Path::new(pj_path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .unwrap_or_else(|e| fail(&format!("profile 目录: {e}")));
            }
        }
        std::fs::write(pj_path, format!("{pj}\n"))
            .unwrap_or_else(|e| fail(&format!("profile 写入: {e}")));
        let write_ms = t_prof.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "{GTAG}: profile → {}（{} 帧逐 pass 分解;assembly+write={write_ms:.3}ms,debug_labels={labels_active}）",
            pj_path.replace('\\', "/"),
            profile_frames.len()
        );
    }

    // A2 验证面:逐帧 presented 亮度序列 sidecar JSON（--present-luma-out;
    // 独立文件不动既有五臂 evidence schema——D3 dump 面同律仅验证用）。
    if let Some(path) = present_luma_out.as_deref() {
        if luma_seq.is_empty() {
            fail("--present-luma-out 全程无 BGRA8 回读帧（headless/无窗口无 auto-move 面无亮度序列,fail-closed）");
        }
        let mut sj = String::with_capacity(96 + luma_seq.len() * 40);
        sj.push_str(&format!(
            "{{\"schema\":\"rurix.g31.present_luma_seq.v1\",\"frames\":{frames},\"warmup\":{warmup},\"auto_exposure\":{autoexp},\"autoexp_key\":{ae_key_v},\"autoexp_rate\":{ae_rate_v},\"autoexp_min\":{ae_min_v},\"autoexp_max\":{ae_max_v},\"seq\":["
        ));
        for (k, (f, m)) in luma_seq.iter().enumerate() {
            if k > 0 {
                sj.push(',');
            }
            sj.push_str(&format!("{{\"frame\":{f},\"mean\":{m:.8}}}"));
        }
        sj.push_str("]}");
        if let Some(parent) = Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, sj.as_bytes())
            .unwrap_or_else(|e| fail(&format!("--present-luma-out 写 {path}: {e}")));
        eprintln!(
            "{GTAG}: presented 亮度序列 → {}（{} 帧）",
            path.replace('\\', "/"),
            luma_seq.len()
        );
    }

    let fps = 1000.0 / r_mean;
    if svt_on {
        let Some((sassets, srep)) = svt_report.as_ref() else {
            fail("C13 SVT 报告缺失（PASS 行面）");
        };
        println!(
            "{GTAG}: PASS gate={} scene={scene_id} tier={tier} backend=tsr_device textures=on svt=on pool_tiles={} full_residency={} pages={} probes={} full_p100={:.6e} partial_miss={} closed_loop_loaded={} miss_rate={:.6e} io_bytes={} fallback_frames={} frames={frames_done} warmup={warmup} exit={exit_reason} resize_eras={resize_eras} real_render_frame_ms={r_mean:.6} fps={fps:.3} present_frame_ms={} encode_gpu_ms={eg_mean:.6} digest={} headless={headless} evidence={}",
            G31_SVT_GATE,
            sassets.pool_tiles,
            sassets.full_residency,
            sassets.tile_set.page_total(),
            srep.probe_count,
            srep.full_p100_vs_direct,
            srep.partial_miss_probes,
            srep.closed_loop_loaded,
            svt_stats.miss_px_total as f64 / (frames_done.max(1) as f64 * (out_w as f64) * (out_h as f64)),
            svt_stats.io_bytes_total,
            svt_stats.fallback_frames,
            p_mean_json,
            jstr(&presented_digest),
            evidence_path.replace('\\', "/"),
        );
        return;
    }
    if textures {
        let Some((tassets, treport)) = tex_report.as_ref() else {
            fail("B4 纹理报告缺失（PASS 行面）");
        };
        // day_0828 Phase B：合流臂 PASS 行追加组合登记（heap 体量 + 质量臂
        // 五件套——off 面空串 0-byte）。Phase C：+ gi2 臂。Phase D：+ tsrq 臂。
        // Phase F：+ emissive 臂。
        let combo_pass = format!(
            "{}{}{}{}{}{}{}{}",
            if bloom {
                format!(" bloom=on bloom_gpu_ms={bg_mean:.6} bloom_strength={bloom_strength_v} bloom_threshold={bloom_threshold_v}")
            } else {
                String::new()
            },
            if let Some(em) = em_assets.as_ref() {
                format!(
                    " emissive_tex=on em_slots={} em_tris={} em_fallback={}",
                    em.rows.len(),
                    em.em_tris,
                    em.rows.iter().filter(|r| r.fallback).count()
                )
            } else {
                String::new()
            },
            if smooth_nrm { " smooth_normals=on" } else { "" },
            if ggx { " ggx=on" } else { "" },
            if lamp_lights {
                format!(" lamp_lights=on lamp_k={lamp_k_v} lamp_gain={lamp_gain_v} lamp_contrib={lamp_contrib_v}")
            } else {
                String::new()
            },
            if gi2 {
                format!(" gi2=on gi2_scale={gi2_scale_v} gi2_clamp={gi2_clamp_v}")
            } else {
                String::new()
            },
            if tsr_quality {
                format!(" tsr_quality=on tsrq_min_alpha={tsrq_min_alpha_v} tsrq_clamp={tsrq_clamp_v}")
            } else {
                String::new()
            },
            if autoexp {
                let (aeg_mean, _, _, _, _) = if autoexp_gpu_ms.is_empty() {
                    (0.0, 0.0, 0.0, 0.0, 0.0)
                } else {
                    g31_stats(&autoexp_gpu_ms)
                };
                format!(
                    " auto_exposure=on autoexp_key={ae_key_v} autoexp_rate={ae_rate_v} autoexp_min={ae_min_v} autoexp_max={ae_max_v} autoexp_gpu_ms={aeg_mean:.6}"
                )
            } else {
                String::new()
            },
        );
        println!(
            "{GTAG}: PASS gate={} scene={scene_id} tier={tier} backend=tsr_device textures=on mapped={} tex_tris={} heap_bytes={} probes={} ssbo_p100={:.6e} sampler_max_lsb={}{combo_pass} frames={frames_done} warmup={warmup} exit={exit_reason} resize_eras={resize_eras} real_render_frame_ms={r_mean:.6} fps={fps:.3} present_frame_ms={} encode_gpu_ms={eg_mean:.6} digest={} headless={headless} evidence={}",
            G31_TEXTURE_GATE,
            tassets.slots.len(),
            tassets.tex_tris,
            tassets.heap_texels * 4,
            treport.probe_count,
            treport.ssbo_p100,
            treport.sampler_max_lsb,
            p_mean_json,
            jstr(&presented_digest),
            evidence_path.replace('\\', "/"),
        );
        return;
    }
    if slab_table.is_some() {
        let Some((_, eval, n_slab)) = slab_report.as_ref() else {
            fail("B3 slab 报告缺失（PASS 行面）");
        };
        println!(
            "{GTAG}: PASS gate={} scene={scene_id} tier={tier} backend=tsr_device slab_arm={} slab_tris={} slab_parity_p100={:.6e} slab_eval_ms={:.3} frames={frames_done} warmup={warmup} exit={exit_reason} resize_eras={resize_eras} real_render_frame_ms={r_mean:.6} fps={fps:.3} present_frame_ms={} encode_gpu_ms={eg_mean:.6} digest={} headless={headless} evidence={}",
            G31_SLAB_GATE,
            slab_arm,
            n_slab,
            eval.parity_p100,
            eval.eval_ms,
            p_mean_json,
            jstr(&presented_digest),
            evidence_path.replace('\\', "/"),
        );
        return;
    }
    if fg != G31Fg::Off {
        let Some((wp, _)) = wired_parity.as_ref() else {
            fail("A5 接线态对拍缺失（PASS 行面）");
        };
        println!(
            "{GTAG}: PASS gate={} scene={scene_id} tier={tier} backend=tsr_device fg={} frames={frames_done} warmup={warmup} exit={exit_reason} resize_eras={resize_eras} real_frames={real_frames} generated_frames={generated_frames} presented_frames={presented_frames} real_render_frame_ms={r_mean:.6} real_render_fps={real_render_fps:.3} present_frame_ms={} presented_fps={presented_fps:.3} present_seconds={present_seconds:.6} encode_ms={encode_host_ms:.6} encode_gpu_ms={eg_mean:.6} fg_gpu_ms={fgg_mean:.6} render5_gpu_ms={r5g_mean:.6} wired_parity_p100={:.6e} digest={} headless={headless} evidence={}",
            G31_FRAMEGEN_GATE,
            fg.name(),
            p_mean_json,
            wp.p100,
            jstr(&presented_digest),
            evidence_path.replace('\\', "/"),
        );
        return;
    }
    // D3:bloom on 面 PASS 行追加登记（off = 空串,既有字面 0-byte）。
    let bloom_pass = if bloom {
        format!(
            " bloom=on bloom_gpu_ms={bg_mean:.6} bloom_strength={bloom_strength_v} bloom_threshold={bloom_threshold_v}"
        )
    } else {
        String::new()
    };
    // D2：smooth-normals on 臂 PASS 行登记（off = 空串,既有字面 0-byte）。
    let nrm_pass = if smooth_nrm {
        " smooth_normals=on".to_owned()
    } else {
        String::new()
    };
    // D6：ggx on 臂 PASS 行登记（off = 空串,既有字面 0-byte）。
    let ggx_pass = if ggx { " ggx=on".to_owned() } else { String::new() };
    // A1：lamp-lights on 臂 PASS 行登记（off = 空串,既有字面 0-byte）。
    let lamp_pass = if lamp_lights {
        format!(" lamp_lights=on lamp_k={lamp_k_v} lamp_gain={lamp_gain_v} lamp_contrib={lamp_contrib_v}")
    } else {
        String::new()
    };
    // A2：auto-exposure on 臂 PASS 行登记（off = 空串,既有字面 0-byte;
    // autoexp_gpu_ms = reduce+state 两 pass GPU 合计 post-warmup 均值）。
    let ae_pass = if autoexp {
        let (aeg_mean, _, _, _, _) = if autoexp_gpu_ms.is_empty() {
            (0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            g31_stats(&autoexp_gpu_ms)
        };
        format!(
            " auto_exposure=on autoexp_key={ae_key_v} autoexp_rate={ae_rate_v} autoexp_min={ae_min_v} autoexp_max={ae_max_v} autoexp_gpu_ms={aeg_mean:.6}"
        )
    } else {
        String::new()
    };
    println!(
        "{GTAG}: PASS scene={scene_id} tier={tier} backend=tsr_device frames={frames_done} warmup={warmup} exit={exit_reason} resize_eras={resize_eras} real_render_frame_ms={r_mean:.6} fps={fps:.3} present_frame_ms={} present_overhead_ms={} encode_ms={encode_host_ms:.6} encode_gpu_ms={eg_mean:.6}{bloom_pass}{nrm_pass}{ggx_pass}{lamp_pass}{ae_pass} digest={} headless={headless} evidence={}",
        p_mean_json,
        overhead_json,
        jstr(&presented_digest),
        evidence_path.replace('\\', "/"),
    );
}
