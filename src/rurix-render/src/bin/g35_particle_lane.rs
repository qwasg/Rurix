// Assisted-by: cursor:claude-fable-5(G35 GPU 粒子系统 G35-3 粒子渲染接线)
//! G35-3 粒子渲染接线 harness(门 `g35.wave3.render`;RFC-0049 §4.6):GPU 粒子
//! (G35-1/2 已落树并过门)接进生产 TSR 车道——billboard splat + 软粒子 +
//! 粒子 MV + mesh 粒子 TLAS 见证臂。共享车道体 g14_3_lane_body.rs **0-byte
//! 不动**(include! 逐字共享),一切经 bin 局部追加(g34_full_lane 模式)。
//!
//! ## 车道形态(pass/资源布局 = 本 bin 冻结字面)
//!
//! - `--particles off` = **母版**:`unified_lane_descs` Mega 22 资源四 pass
//!   (scene g14_3_direct_gi → mv g14_mv → tsr resample → tsr resolve),
//!   不追加任何资源/pass/readback——digest 位级 == Stage A 锚
//!   (`--static-camera` 锚格模式 160 帧 warmup 10 对
//!   milestones/g14/g14_3_stage_a_digest_anchor.json cell
//!   bistro-interior_t100_tsr_device;经共享体 `UnifiedTsrLane` 原码执行)。
//! - `--particles on` = 母版 + bin 局部追加:mv(pass1) 与 tsr resample 之间
//!   插入 **10 个 Compute pass**(下标 2..=11,冻结序):sim → scan_seg_sum →
//!   scan_spine → scan_seg_apply → particle_compact → emit → indirect_args →
//!   splat_clear → splat → resolve;尾追 device 显示编码(pass 14,g34
//!   encode append 先例)。TSR resample/resolve 顺移至 12/13。
//! - **FrameUpdate 重映射方案**:共享体 `prepare_update_ext` 的 parity
//!   binding_overrides 按 pass 下标寻址(Mega 硬编码 (2,3)/Split (4,5),
//!   g14_3_lane_body.rs L5756)——本 bin **不消费共享体 prepare**,自持
//!   `G35OnLane::prepare`(G34TsrLane 同模 bin 局部复制)按插入后下标
//!   (12,13,14) 构造 overrides,并追加粒子 5 pass 的 A/B ping-pong parity
//!   overrides(2 sim / 6 compact / 7 emit / 10 splat / 11 resolve)。
//!   共享体 0-byte。
//!
//! ## 资源追加(bin 局部,index 22 起;22..=53 共 32 件,总 54)
//!
//! | idx | 资源 | 形态 |
//! |---|---|---|
//! | 22 | 粒子 sim params(8 f32) | host-visible 逐帧上传 |
//! | 23 | scan/compact 共享 params(4 f32,[n,nseg,0,0] 同布局同值) | host-visible 逐帧 |
//! | 24 | emit params(16 f32) | host-visible 逐帧 |
//! | 25 | indirect_args params(4 f32) | host-visible 逐帧 |
//! | 26 | 渲染三件共享 params(80 f32,布局 = pack_render_params 头注;[64..80) 雨丝模式追加段) | host-visible 逐帧 |
//! | 27..=35 | 九流 A 组(pos_x/y/z,vel_x/y/z,age,life,pid;f32/u32×cap) | device |
//! | 36..=44 | 九流 B 组(同序) | device |
//! | 45/46 | flags / scan_out(u32×cap) | device |
//! | 47/48 | seg_sums(u32×nseg_cap) / seg_offsets(u32×(nseg_cap+1)) | device |
//! | 49 | args(u32×8;**BufferUsage.indirect=true**) | device |
//! | 50 | rand_table(f32×65536,创建期一次上传) | device |
//! | 51 | winner(u64×px 内部分辨率) | device |
//! | 52/53 | encode 参数 / BGRA8 输出 | device(g34 同型) |
//!
//! splat pass dispatch = `DispatchSpec::Indirect{res:49,offset:0}`(生产零
//! 回读链:host 不读回 device 计数,dispatch 参数 device 端合成);sim/scan/
//! compact 等 Direct(cap/256 上界 + kernel 内 n_curr 守卫,n_curr 经 host
//! 金标准 `particles::core::frame` 平行推得逐帧上传对拍,禁回读)。九流
//! ping-pong 经逐帧 binding_overrides 换 A/B(奇偶 parity 与 TSR parity 同
//! 计数)。屏障计划逐 pass 保守超集(&'static 常量),bin 内机核审计
//! bindings ⊆ 计划资源集,结果进 evidence(fact barrier_plan_audit)。
//!
//! ## 深度域登记(诚实优先)
//!
//! U_SCENE_DEPTH 生产字面 = 未抖 vp 参数行 25..32(vp 行 0/1,即
//! clip.x/clip.y——g34_unified_gi.rx ⑦ 段「生产字面」注 + g34_full_lane
//! host 金标准 z_quirk 两路并存登记面;真 ZO NDC 归 g34_unified_shade.rx
//! ④b HZB 车道形态,本 Mega 车道无此资源)。本波 splat 硬拒/软粒子按冻结
//! 协议与存储域**同域**实现(kernels/g35_render_splat.rx ③ 段头注);该域
//! 沿视射线为常量 ⇒ 同域比较为屏幕域序判而非距离遮挡,evidence
//! `depth_domain` 字段如实登记。`--occlusion-witness` 因此取**相机后已知墙
//! 后**构型(bistro 室内相机身后墙体;投影 w 门拒绝路径为确定性拒绝面):
//! winner 全零 + 命中像素零 + scene color/render_digest 与 off 面位级等。
//!
//! ## mesh 粒子 TLAS 见证臂(--mesh-particles N,N≤8)
//!
//! A4 先例(unified_lane_descs_dyn + g31_dyn_scene 实例感知 kernel +
//! 逐帧 tlas_update Refit,inflight=1)bin 局部消费:**wired = 1 实例**——
//! g31_dyn_scene 分派映射 pg = prim + inst·dyn_tri_base 为单动态实例语义
//! (inst ≥ 2 时 tris 下标越界),N > 1 的其余实例如实 not_wired 登记
//! (evidence mesh_particles.not_wired_reason;0-byte 纪律下不 hack 共享体
//! /kernel)。立方体按 host 金标准粒子轨迹(单粒子恒速夹具)逐帧
//! tlas_update;ray query 场景自动获得光追阴影(动态实例入 TLAS ⇒ 阴影
//! 射线可命中,登记面)。fact = on≠off render_digest 判别(进程内双车道)。
//!
//! ## 用法
//!
//! ```text
//! g35_particle_lane [--frames N] [--warmup N] [--tier 100]
//!     [--contract <c.json>] [--g10-dir milestones/g10/corpus] [--gltf <scene.gltf>]
//!     [--spv-scene/-mv/-resample/-resolve/-encode <spv>]
//!     [--spv-p-sim/-p-scan-seg-sum/-p-scan-spine/-p-scan-seg-apply/
//!      -p-compact/-p-emit/-p-indirect-args <spv>]
//!     [--spv-splat-clear/--spv-splat/--spv-presolve <spv>]
//!     [--particles on|off] [--static-camera] [--auto-move orbit|dolly|dolly-forward] [--auto-move-amp 1.0]
//!     [--evidence <path>] [--expect-digest <sha256:…>] [--cap 65536] [--seed 42]
//!     [--mv-witness] [--occlusion-witness] [--mesh-particles N] [--headless]
//!     [--cluster-lod off|leaf|on --cluster-pack <RXCP> [--cluster-error-px 1.0]]
//!     [--wp-hlod off|full|on --wp-pack <RXWH> [--wp-threshold-l0 1.0]]
//!     [--dump-present-raw <path>] [--dump-present-every <n>] [--r-world 0.02] [--splat-stretch 1.0]
//!     [--particle-tint r,g,b] [--particle-alpha-scale 1.0]
//!     [--rain-shutter 0.0] [--rain-occlusion on|off] [--ev100 <f32>]
//!     [--scene bistro-interior]
//!     [--emitter-pos x,y,z] [--emitter-spread x,y,z] [--emitter-vel x,y,z]
//!     [--emitter-vel-spread x,y,z] [--emitter-life 1.4] [--emitter-gravity -0.9]
//!     [--emitter-follow-camera on|off] [--emit-max 256]
//! ```
//!
//! 展示面(网站出图)旗标组:**全部默认 = 冻结生产字面,位级零漂移**
//! (stretch/tint/alpha 的 ×1.0、/1.0 均 IEEE 精确,r_world 默认 = 冻结常量;
//! kernel params[56..61] 原 reserved 恒 0 槽位启用,三渲染 kernel 头注同录)。
//! **雨丝模式** `--rain-shutter s`(s ∈ (0,2],缺省 0 = 冻结面):splat 改
//! 运动模糊胶囊(首 pos → 尾 pos − vel·dt·s,长度 = 每帧真实运动量而非固定
//! 拉伸)+ 场景 TLAS 逐粒子遮挡射线(`--rain-occlusion`,缺省 on);resolve
//! 改 tint 作 **display 域**绝对色(1.0 = 显示白,kernel 乘 1/exposure 换回
//! scene-linear)+ tent 剖面 × 亚像素覆盖峰值(远雨自然减淡)+ 末段淡出,
//! 并对赢家足迹(外扩 1.5 px 吸收 TSR 重采样核扩散)写 U_REACTIVE = 1(TSR
//! has_reactive 同步置 1)⇒ 该像素取当前帧,历史钳制不参与;quirk 深度域软粒子
//! 造成的「倒水滴」形不再出现。**注**:day_0831_site rain_probe 的彩色噪点
//! 主因是 `.tmp/g35_gates/render/g31_display_encode.spv` 为 8/27 旧源码
//! (B-spline 基 b1/b2 写错,0e605c34 已修源)编出的过期二进制——现编 SPV 即消;
//! 出图前须用当前 rurixc 重编该 SPV。render params 64 → 80 f32
//! ([61..80) 追加段;冻结面恒 0),splat 追加 vel×3 + AS 绑定,presolve 追加
//! U_REACTIVE 绑定(屏障计划同步;冻结面 kernel 分支逐字不动,双 parity 布局键
//! 恒等)。与 --splat-stretch / --oit ≠ off / 见证夹具互斥(如实拒跑)。
//! **换景** `--scene <id>`(缺省 bistro-interior = 锚格/见证冻结构型):展示面
//! 换景须 `--particles on` + 自定义 `--contract`(含该 scene_id 行:camera/
//! lighting/material_policy)+ `--expect-digest` + 显式 `--gltf`(共享体
//! default_gltf 闭集只认 bistro-interior|cornell-box,本 bin 不触共享体);
//! G10 provenance 文件名按 scene_id 连字符→下划线派生,缺失如实登记 MISSING。
//! 雨天室外候选(bistro-exterior)接入即走此路径。
//! `--dump-present-raw` = 末帧 presented BGRA8 写盘(w/h u32 LE 头同
//! g31_window_present 布局,raw2png.py 直通;回读面 = 既有末帧回读,零追加
//! GPU 读回)。--emitter-* 逐字段覆写生产夹具(与见证夹具互斥;device 上传
//! 与 host 金标准镜像同源消费,整数流一致性不破)。非默认展示参数须随
//! --particles on(off 面携带 = 静默无效冒充,如实拒跑)。参数面回显进
//! evidence `showcase` 块(全默认 = null)。
//!
//! **推轨短片面**(day_0902_rain_night 战役;四旗标**全缺省 = 位级零漂移**):
//! - `--dump-present-every n`(n ≥ 1,须随 `--dump-present-raw <base>`):每 n 帧
//!   (帧号含 warmup,`fi % n == 0`)把该帧 presented BGRA8 写 `<base>.f<帧号 4 位>`
//!   (w/h u32 LE 头同 g31_window_present 逐帧写盘布局);末帧 `<base>` 照旧由
//!   既有 last 分支写。命中帧追加一路 BGRA 回读(`--auto-move` 面本已逐帧回读;
//!   回读 = device→host 拷贝,不改渲染数值);`digest_seq` 门仍只看
//!   `--auto-move`。缺省 None ⇒ `rb.bgra` 表达式 `… || false` 短路回旧值。
//! - `--auto-move-amp f`(f ∈ (0, 64],须随 `--auto-move`):orbit/dolly 位移项
//!   ×f(yaw 角量不乘——角量与轨长无关);新轨迹 `dolly-forward` = 沿契约相机
//!   前向 XZ 归一方向匀速推进 `d = f·t`(t = fi/total;eye.y/yaw/pitch 不变)。
//!   缺省 1.0:`0.35 * 1.0` 等 IEEE 精确 ⇒ orbit/dolly 逐位同旧值。
//! - `--emitter-follow-camera on|off`(须随 `--particles on` + `--auto-move`;与见证
//!   夹具互斥):逐帧把发射中心平移 `eye(fi) − eye(0)`,**先改 `mirror.desc.pos`
//!   再 `mirror.step`** ⇒ host 金标准 `pcore::frame(&self.desc)` 与 device
//!   `lane.frame(&mirror.desc)` 消费同一份 pos,随机带消费律/整数流不变。缺省
//!   off 分支不执行。
//! - `--emit-max n`(n ∈ [256, 4096],须随 `--particles on`;与见证夹具互斥):
//!   emit pass Direct dispatch 上界 + 生产发射预算按 `n/256` 线性放大
//!   (`min((64 + f·17 % 192)·n/256, cap − n_curr)`);缺省 256 走冻结臂字面。
//!   随机带克隆守卫:`r_k = table[(pid·7919 + k) % 65536]`,7919 与 2^16 互素 ⇒
//!   全 7 槽克隆 ⇔ pid ≡ pid′ (mod 65536);同屏存活粒子跨度
//!   `peak·ceil(life/dt) ≥ 65536` 即拒跑(降 --emit-max 或缩 --emitter-life);
//!   `peak·total ≥ 2^24` 拒跑(pid 走 f32 参数面精确域,core `emit_step` 断言面
//!   提前中文化)。evidence `showcase` 追加四键回显 + 顶层 `gltf` 路径/sha 登记。
//!
//! 闭集:--static-camera 与 --auto-move 互斥(缺省 = 静态契约相机);
//! --mv-witness/--occlusion-witness 互斥且须随 --particles on + 静态相机;
//! --mesh-particles 须随 --particles off(隔离见证);--headless 恒真登记
//! (本 bin 即离屏,不开窗)。三态:无 Vulkan/资产缺失 → skipped_dev_env
//! 退 0;RURIX_REQUIRE_REAL=1 翻硬红。
//!
//! G36 W4 geo 组合面(互斥解除;门 `g36.wave1.geo_composition` fact ⑩):
//! --cluster-lod × --wp-hlod × --particles on|off × --oit sorted|wboit 组合
//! 成立(粒子为生成几何,splat/OIT 在场景色之上与场景重组正交;W1 provenance
//! 事实源,cut/选层冻结于装配期契约相机)。geo × 见证/RED 臂维持互斥(标定
//! 夹具构型 = 语义互斥,如实拒跑不冒充)。
#![forbid(unsafe_code)]
// 共享体含本 bin 未消费面(render/bench 腿、dlss/fsr 双臂、EXR/PNG 出图、
// GI 臂、SVT/蒙皮/HZB 面等)——dead_code 豁免如实登记;本 bin 消费面 = 契约
// 解析/scene 装配/Mega·MegaDyn 车道/帧参数/jitter/digest/dyn 资产/
// NoContraction 注入。
#![allow(dead_code)]

include!("g14_3_lane/g14_3_lane_body.rs");

use rurix_render::display::aces13::aces13_device_encode_params;
use rurix_render::particles::core as pcore;
use rurix_render::particles::oit_arms as poit;
use rurix_render::particles::{RAND_K, RAND_TABLE_LEN, SEG, rand_table as p_rand_table};

const G35L_TAG: &str = "[g35_particle_lane]";
/// G35-3 门键(evidence `gate` 字段字面)。
const G35L_GATE: &str = "g35.wave3.render";
/// harness 真跑件 schema 字面(留 .tmp 不注册——门裁决件 = smoke 产
/// rurix.g35.render_gate_evidence.v1,CI_GATES §3 律)。
const G35L_RUN_SCHEMA: &str = "rurix.g35.particle_lane_run.v1";
/// Stage A 锚格字面(--static-camera 锚格模式对拍面;smoke 消费)。
const G35L_ANCHOR_PATH: &str = "milestones/g14/g14_3_stage_a_digest_anchor.json";
const G35L_ANCHOR_CELL: &str = "bistro-interior_t100_tsr_device";
/// device 显示编码 kernel 默认 SPV(g31 A3 同件 0-byte 消费;g34 同字面)。
const G35L_DEFAULT_SPV_ENCODE: &str = ".tmp/g14_gates/m_c/g31_display_encode.spv";
/// 粒子/渲染 kernel 默认 SPV 目录(smoke --gate 现编落此)。
const G35L_SPV_DIR: &str = ".tmp/g35_gates/render";
/// G10 语料目录默认(provenance 登记面;字段核验门归 g34.wave1)。
const G35L_DEFAULT_G10_DIR: &str = "milestones/g10/corpus";

// ---------------------------------------------------------------------------
// 冻结夹具常量(帧协议/发射器/渲染参数;改动 = 契约修订)
// ---------------------------------------------------------------------------

/// 帧步长(秒;G35-2 probe 同字面)。
const G35L_DT: f32 = 1.0 / 60.0;
/// 粒子世界半径(米;splat 半径投影输入,kernel 内 clamp ≤ 3px 半幅)。
const G35L_R_WORLD: f32 = 0.02;
/// 软粒子 fade 范围(存储深度域;深度域 quirk 登记见 bin 头注)。
const G35L_SOFT_RANGE: f32 = 0.05;
/// emit pass Direct dispatch 上界(kernel 内 j < emit_count 守卫;
/// emit_schedule 上界 = 64+191 = 255 < 256)。
/// 推轨短片面 `--emit-max n` 可放大为 n ∈ [256, 4096](dispatch 与生产预算同比
/// `n/256`);缺省即本常量 ⇒ 冻结节奏逐字。
const G35L_EMIT_MAX: u32 = 256;

// ---------------------------------------------------------------------------
// 粒子资源下标(bin 局部,22 起;bin 头注布局表 = 单一事实源)
// ---------------------------------------------------------------------------

const G35L_P_SIM_PARAMS: u32 = 22;
const G35L_P_CORE_PARAMS: u32 = 23;
const G35L_P_EMIT_PARAMS: u32 = 24;
const G35L_P_ARGS_PARAMS: u32 = 25;
const G35L_P_RENDER_PARAMS: u32 = 26;
/// 九流 A 组基址(27..=35:pos_x/y/z,vel_x/y/z,age,life,pid)。
const G35L_GROUP_A: u32 = 27;
/// 九流 B 组基址(36..=44 同序)。
const G35L_GROUP_B: u32 = 36;
const G35L_FLAGS: u32 = 45;
const G35L_SCAN_OUT: u32 = 46;
const G35L_SEG_SUMS: u32 = 47;
const G35L_SEG_OFFSETS: u32 = 48;
const G35L_ARGS: u32 = 49;
const G35L_RAND: u32 = 50;
const G35L_WINNER: u32 = 51;
const G35L_ENC_PARAMS: u32 = 52;
const G35L_ENC_OUT: u32 = 53;
const G35L_RESOURCE_COUNT: usize = 54;

// ---------------------------------------------------------------------------
// G35-4 半透明双臂(门 g35.wave4.sort_oit;RFC-0049 §4.8;host 金标准 =
// particles/oit_arms.rs)——bin 局部加性扩展:--oit off|sorted|wboit 三档
// 闭集;off = G35-3 现面零追加(digest 位级 == 缺省 = 加性 0 破坏机器证明);
// sorted/wboit 档在现 g35_render_resolve(pass 11)之后 TSR 之前插各自 pass
// 组,资源 54..=70 追加。
//
// ## OIT 资源下标(oit ≠ off 追加;54 起)
// | idx | 资源 | 形态 |
// |---|---|---|
// | 54 | P_OIT_PARAMS(96 f32,[0..64) = render_params 逐字镜像 + [64] | host-visible 逐帧 |
// |    | tiles_x [65] tiles_y [66] tile_cnt [67] red_flag [68..96) 0) |  |
// | 55/56/57 | sort params p0/p1/p2(4 f32 [n,nseg,dpow∈{1,256,65536},0]) | host-visible 逐帧 |
// | 58/59 | OIT keys/payload A 组(u32×cap) | device |
// | 60/61 | OIT keys/payload B 组(3-pass 终产物) | device |
// | 62 | sort hist(u32×nseg_cap·256) | device |
// | 63 | sort offs(u32×256·nseg_cap) | device |
// | 64 | sort scratch(u32×nseg_cap·256) | device |
// | 65/66 | tile_start/tile_end(u32×(tile_cnt+1),含溢出 tile) | device |
// | 67 | wboit acc(u32×4px,Q12 定点四通道) | device |
// | 68 | wboit sat(u32×4,[0] = 饱和事件累计计数,逐帧不清零) | device |
// | 69 | tile clear params(4 f32 [ncell,nseg_cells,0,0] 恒值) | host-visible 创建期 |
// | 70 | acc clear params(64 f32 恒值,[6] = 2·px——acc 视 u64×2px 清零) | host-visible 创建期 |
//
// ## pass 布局(冻结序;presolve = 11 之后插入)
// - sorted(+13):12 g35_hash_clear(tile 哨兵,W7 kernel 0-byte 消费)→
//   13 g35_oit_tilekey(indirect,parity)→ 14..22 W1 sort 三 kernel 3-pass
//   9 dispatch(键/payload A→B→A→B,终产物 B = 60/61)→ 23 g35_oit_tilerange
//   → 24 g35_oit_blend_sorted(parity)→ TSR 25/26 → encode 27(28 pass)。
// - wboit(+3):12 g35_splat_clear(acc 清零 0-byte 消费:独立 params[6] =
//   2·px,acc 缓冲按 u64×2px 语义清 0——SSBO 无类型,登记面)→ 13
//   g35_oit_wboit_accum(indirect,parity)→ 14 g35_oit_wboit_resolve →
//   TSR 15/16 → encode 17(18 pass)。
//
// ## 键域/预算硬域守卫(fail-fast 拒跑;oit_arms.rs 头注论证镜像)
// - tile_cnt = ceil(iw/16)·ceil(ih/16) ≤ 4095(OIT_TILE_CNT_MAX)⇒ 溢出键
//   tile_cnt·4096 ≤ 16 773 120 < 2^24(门腿构型 bistro t50 内部 960×540 ⇒
//   60×34 = 2040 ✓;t100 1920×1080 ⇒ 8160 越域拒跑如实登记)。
// - wboit 档 cap ≤ 65536(OIT_WBOIT_CAP_MAX)⇒ 累加和 ≤ 2^32 − 2^16 <
//   u32::MAX 结构性防回绕。
// ---------------------------------------------------------------------------

const G35L_OIT_PARAMS: u32 = 54;
const G35L_OIT_SORT_P0: u32 = 55;
const G35L_OIT_SORT_P1: u32 = 56;
const G35L_OIT_SORT_P2: u32 = 57;
const G35L_OIT_KEYS_A: u32 = 58;
const G35L_OIT_PAY_A: u32 = 59;
const G35L_OIT_KEYS_B: u32 = 60;
const G35L_OIT_PAY_B: u32 = 61;
const G35L_OIT_HIST: u32 = 62;
const G35L_OIT_OFFS: u32 = 63;
const G35L_OIT_SCRATCH: u32 = 64;
const G35L_OIT_TILE_START: u32 = 65;
const G35L_OIT_TILE_END: u32 = 66;
const G35L_OIT_ACC: u32 = 67;
const G35L_OIT_SAT: u32 = 68;
const G35L_OIT_TCLEAR_PARAMS: u32 = 69;
const G35L_OIT_ACLEAR_PARAMS: u32 = 70;
const G35L_OIT_RESOURCE_COUNT: usize = 71;
/// OIT kernel 默认 SPV 目录(smoke --gate 现编落此)。
const G35L_OIT_SPV_DIR: &str = ".tmp/g35_gates/sort_oit";
/// 近远见证第二粒子发射帧(冻结;两粒子 age 差 = 30·dt = 0.5s ⇒ 调色可判)。
const G35L_OIT_WITNESS_F2: u32 = 30;

/// --oit 三档闭集(off = G35-3 现面零追加)。
#[derive(Clone, Copy, PartialEq)]
enum G35Oit {
    Off,
    Sorted,
    Wboit,
}

impl G35Oit {
    fn as_str(self) -> &'static str {
        match self {
            G35Oit::Off => "off",
            G35Oit::Sorted => "sorted",
            G35Oit::Wboit => "wboit",
        }
    }

    /// presolve 之后插入的 pass 数(TSR/encode 下标偏移)。
    fn pass_delta(self) -> u32 {
        match self {
            G35Oit::Off => 0,
            G35Oit::Sorted => 13,
            G35Oit::Wboit => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// 屏障计划(逐 pass 保守超集,&'static 常量;A/B parity 换绑 ⇒ 计划列两组
// 并集)。**状态字面 = StorageWrite 而非 StorageReadWrite,刻意**:执行器
// barrier_fields 对"已在目标态"幂等去重返 None(render_exec.rs 1992),而隐式
// 补全恒把 storage 绑定落回 StorageReadWrite——若计划也写 RW,则首 pass 之后
// 全链条同态 ⇒ 零屏障 ⇒ 同 cmd 内 dispatch 重叠竞争(G35-3 门 determinism
// 红根因:双跑 digest_seq 54 帧偶发 3 帧漂移)。计划写 W 使每 pass 产生
// RW→W(回放)+ W→RW(补全)两条 ALL_SHADERS 执行依赖 + 该缓冲可用/可见链,
// pass 间全序恢复。args 在 splat 处 W 后显式补一条 RW(kernel 读 args[7]
// 守卫面的可见性),再由隐式补全推 (args, IndirectRead)(DispatchSpec::
// Indirect 首位)——三段链覆盖 shader 读 + indirect 读两类访问。
// ---------------------------------------------------------------------------

const G35L_PLAN_SIM: &[(u32, TargetState)] = &[
    (G35L_P_SIM_PARAMS, TargetState::StorageWrite),
    (G35L_FLAGS, TargetState::StorageWrite),
    (27, TargetState::StorageWrite),
    (28, TargetState::StorageWrite),
    (29, TargetState::StorageWrite),
    (30, TargetState::StorageWrite),
    (31, TargetState::StorageWrite),
    (32, TargetState::StorageWrite),
    (33, TargetState::StorageWrite),
    (34, TargetState::StorageWrite),
    (36, TargetState::StorageWrite),
    (37, TargetState::StorageWrite),
    (38, TargetState::StorageWrite),
    (39, TargetState::StorageWrite),
    (40, TargetState::StorageWrite),
    (41, TargetState::StorageWrite),
    (42, TargetState::StorageWrite),
    (43, TargetState::StorageWrite),
];
const G35L_PLAN_SEG_SUM: &[(u32, TargetState)] = &[
    (G35L_FLAGS, TargetState::StorageWrite),
    (G35L_P_CORE_PARAMS, TargetState::StorageWrite),
    (G35L_SEG_SUMS, TargetState::StorageWrite),
];
const G35L_PLAN_SPINE: &[(u32, TargetState)] = &[
    (G35L_SEG_SUMS, TargetState::StorageWrite),
    (G35L_P_CORE_PARAMS, TargetState::StorageWrite),
    (G35L_SEG_OFFSETS, TargetState::StorageWrite),
];
const G35L_PLAN_SEG_APPLY: &[(u32, TargetState)] = &[
    (G35L_FLAGS, TargetState::StorageWrite),
    (G35L_SEG_OFFSETS, TargetState::StorageWrite),
    (G35L_P_CORE_PARAMS, TargetState::StorageWrite),
    (G35L_SCAN_OUT, TargetState::StorageWrite),
];
const G35L_PLAN_COMPACT: &[(u32, TargetState)] = &[
    (G35L_P_CORE_PARAMS, TargetState::StorageWrite),
    (G35L_FLAGS, TargetState::StorageWrite),
    (G35L_SCAN_OUT, TargetState::StorageWrite),
    (27, TargetState::StorageWrite),
    (28, TargetState::StorageWrite),
    (29, TargetState::StorageWrite),
    (30, TargetState::StorageWrite),
    (31, TargetState::StorageWrite),
    (32, TargetState::StorageWrite),
    (33, TargetState::StorageWrite),
    (34, TargetState::StorageWrite),
    (35, TargetState::StorageWrite),
    (36, TargetState::StorageWrite),
    (37, TargetState::StorageWrite),
    (38, TargetState::StorageWrite),
    (39, TargetState::StorageWrite),
    (40, TargetState::StorageWrite),
    (41, TargetState::StorageWrite),
    (42, TargetState::StorageWrite),
    (43, TargetState::StorageWrite),
    (44, TargetState::StorageWrite),
];
const G35L_PLAN_EMIT: &[(u32, TargetState)] = &[
    (G35L_P_EMIT_PARAMS, TargetState::StorageWrite),
    (G35L_SEG_OFFSETS, TargetState::StorageWrite),
    (G35L_RAND, TargetState::StorageWrite),
    (27, TargetState::StorageWrite),
    (28, TargetState::StorageWrite),
    (29, TargetState::StorageWrite),
    (30, TargetState::StorageWrite),
    (31, TargetState::StorageWrite),
    (32, TargetState::StorageWrite),
    (33, TargetState::StorageWrite),
    (34, TargetState::StorageWrite),
    (35, TargetState::StorageWrite),
    (36, TargetState::StorageWrite),
    (37, TargetState::StorageWrite),
    (38, TargetState::StorageWrite),
    (39, TargetState::StorageWrite),
    (40, TargetState::StorageWrite),
    (41, TargetState::StorageWrite),
    (42, TargetState::StorageWrite),
    (43, TargetState::StorageWrite),
    (44, TargetState::StorageWrite),
];
const G35L_PLAN_ARGS: &[(u32, TargetState)] = &[
    (G35L_P_ARGS_PARAMS, TargetState::StorageWrite),
    (G35L_SEG_OFFSETS, TargetState::StorageWrite),
    (G35L_ARGS, TargetState::StorageWrite),
];
const G35L_PLAN_SPLAT_CLEAR: &[(u32, TargetState)] = &[
    (G35L_P_RENDER_PARAMS, TargetState::StorageWrite),
    (G35L_WINNER, TargetState::StorageWrite),
];
const G35L_PLAN_SPLAT: &[(u32, TargetState)] = &[
    (G35L_P_RENDER_PARAMS, TargetState::StorageWrite),
    (G35L_ARGS, TargetState::StorageWrite),
    (G35L_ARGS, TargetState::StorageReadWrite),
    (27, TargetState::StorageWrite),
    (28, TargetState::StorageWrite),
    (29, TargetState::StorageWrite),
    (30, TargetState::StorageWrite),
    (31, TargetState::StorageWrite),
    (32, TargetState::StorageWrite),
    (36, TargetState::StorageWrite),
    (37, TargetState::StorageWrite),
    (38, TargetState::StorageWrite),
    (39, TargetState::StorageWrite),
    (40, TargetState::StorageWrite),
    (41, TargetState::StorageWrite),
    (U_SCENE_DEPTH, TargetState::StorageWrite),
    (G35L_WINNER, TargetState::StorageWrite),
];
const G35L_PLAN_PRESOLVE: &[(u32, TargetState)] = &[
    (G35L_P_RENDER_PARAMS, TargetState::StorageWrite),
    (G35L_WINNER, TargetState::StorageWrite),
    (27, TargetState::StorageWrite),
    (28, TargetState::StorageWrite),
    (29, TargetState::StorageWrite),
    (30, TargetState::StorageWrite),
    (31, TargetState::StorageWrite),
    (32, TargetState::StorageWrite),
    (33, TargetState::StorageWrite),
    (34, TargetState::StorageWrite),
    (36, TargetState::StorageWrite),
    (37, TargetState::StorageWrite),
    (38, TargetState::StorageWrite),
    (39, TargetState::StorageWrite),
    (40, TargetState::StorageWrite),
    (41, TargetState::StorageWrite),
    (42, TargetState::StorageWrite),
    (43, TargetState::StorageWrite),
    (U_SCENE_DEPTH, TargetState::StorageWrite),
    (U_SCENE_COLOR, TargetState::StorageWrite),
    (U_MV_OUT, TargetState::StorageWrite),
    (U_REACTIVE, TargetState::StorageWrite),
];
/// encode pass 屏障计划(g34 G34_U_PLAN_ENCODE 同型:TSR 输出双 parity 并集
/// + 编码参数 + BGRA8 输出)。
const G35L_PLAN_ENCODE: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageWrite),
    (U_OUT_COLOR[1], TargetState::StorageWrite),
    (G35L_ENC_PARAMS, TargetState::StorageWrite),
    (G35L_ENC_OUT, TargetState::StorageWrite),
];

// ── G35-4 OIT pass 屏障计划(同律:StorageWrite 形态,禁全 RW;args 在
// indirect 消费 pass 补 RW 条 = G35L_PLAN_SPLAT 先例;粒子流列双 parity
// 并集)──
const G35L_PLAN_OIT_TILE_CLEAR: &[(u32, TargetState)] = &[
    (G35L_OIT_TCLEAR_PARAMS, TargetState::StorageWrite),
    (G35L_OIT_TILE_START, TargetState::StorageWrite),
    (G35L_OIT_TILE_END, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_TILEKEY: &[(u32, TargetState)] = &[
    (G35L_OIT_PARAMS, TargetState::StorageWrite),
    (G35L_ARGS, TargetState::StorageWrite),
    (G35L_ARGS, TargetState::StorageReadWrite),
    (27, TargetState::StorageWrite),
    (28, TargetState::StorageWrite),
    (29, TargetState::StorageWrite),
    (36, TargetState::StorageWrite),
    (37, TargetState::StorageWrite),
    (38, TargetState::StorageWrite),
    (G35L_OIT_KEYS_A, TargetState::StorageWrite),
    (G35L_OIT_PAY_A, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_HIST_P0: &[(u32, TargetState)] = &[
    (G35L_OIT_KEYS_A, TargetState::StorageWrite),
    (G35L_OIT_SORT_P0, TargetState::StorageWrite),
    (G35L_OIT_HIST, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_HIST_P1: &[(u32, TargetState)] = &[
    (G35L_OIT_KEYS_B, TargetState::StorageWrite),
    (G35L_OIT_SORT_P1, TargetState::StorageWrite),
    (G35L_OIT_HIST, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_HIST_P2: &[(u32, TargetState)] = &[
    (G35L_OIT_KEYS_A, TargetState::StorageWrite),
    (G35L_OIT_SORT_P2, TargetState::StorageWrite),
    (G35L_OIT_HIST, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_SPINE_P0: &[(u32, TargetState)] = &[
    (G35L_OIT_HIST, TargetState::StorageWrite),
    (G35L_OIT_SORT_P0, TargetState::StorageWrite),
    (G35L_OIT_OFFS, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_SPINE_P1: &[(u32, TargetState)] = &[
    (G35L_OIT_HIST, TargetState::StorageWrite),
    (G35L_OIT_SORT_P1, TargetState::StorageWrite),
    (G35L_OIT_OFFS, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_SPINE_P2: &[(u32, TargetState)] = &[
    (G35L_OIT_HIST, TargetState::StorageWrite),
    (G35L_OIT_SORT_P2, TargetState::StorageWrite),
    (G35L_OIT_OFFS, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_SCATTER_P0: &[(u32, TargetState)] = &[
    (G35L_OIT_KEYS_A, TargetState::StorageWrite),
    (G35L_OIT_PAY_A, TargetState::StorageWrite),
    (G35L_OIT_OFFS, TargetState::StorageWrite),
    (G35L_OIT_SORT_P0, TargetState::StorageWrite),
    (G35L_OIT_SCRATCH, TargetState::StorageWrite),
    (G35L_OIT_KEYS_B, TargetState::StorageWrite),
    (G35L_OIT_PAY_B, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_SCATTER_P1: &[(u32, TargetState)] = &[
    (G35L_OIT_KEYS_B, TargetState::StorageWrite),
    (G35L_OIT_PAY_B, TargetState::StorageWrite),
    (G35L_OIT_OFFS, TargetState::StorageWrite),
    (G35L_OIT_SORT_P1, TargetState::StorageWrite),
    (G35L_OIT_SCRATCH, TargetState::StorageWrite),
    (G35L_OIT_KEYS_A, TargetState::StorageWrite),
    (G35L_OIT_PAY_A, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_SCATTER_P2: &[(u32, TargetState)] = &[
    (G35L_OIT_KEYS_A, TargetState::StorageWrite),
    (G35L_OIT_PAY_A, TargetState::StorageWrite),
    (G35L_OIT_OFFS, TargetState::StorageWrite),
    (G35L_OIT_SORT_P2, TargetState::StorageWrite),
    (G35L_OIT_SCRATCH, TargetState::StorageWrite),
    (G35L_OIT_KEYS_B, TargetState::StorageWrite),
    (G35L_OIT_PAY_B, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_TILERANGE: &[(u32, TargetState)] = &[
    (G35L_OIT_SORT_P0, TargetState::StorageWrite),
    (G35L_OIT_KEYS_B, TargetState::StorageWrite),
    (G35L_OIT_TILE_START, TargetState::StorageWrite),
    (G35L_OIT_TILE_END, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_BLEND: &[(u32, TargetState)] = &[
    (G35L_OIT_PARAMS, TargetState::StorageWrite),
    (G35L_OIT_PAY_B, TargetState::StorageWrite),
    (G35L_OIT_TILE_START, TargetState::StorageWrite),
    (G35L_OIT_TILE_END, TargetState::StorageWrite),
    (27, TargetState::StorageWrite),
    (28, TargetState::StorageWrite),
    (29, TargetState::StorageWrite),
    (33, TargetState::StorageWrite),
    (34, TargetState::StorageWrite),
    (36, TargetState::StorageWrite),
    (37, TargetState::StorageWrite),
    (38, TargetState::StorageWrite),
    (42, TargetState::StorageWrite),
    (43, TargetState::StorageWrite),
    (U_SCENE_DEPTH, TargetState::StorageWrite),
    (U_SCENE_COLOR, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_ACC_CLEAR: &[(u32, TargetState)] = &[
    (G35L_OIT_ACLEAR_PARAMS, TargetState::StorageWrite),
    (G35L_OIT_ACC, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_ACCUM: &[(u32, TargetState)] = &[
    (G35L_OIT_PARAMS, TargetState::StorageWrite),
    (G35L_ARGS, TargetState::StorageWrite),
    (G35L_ARGS, TargetState::StorageReadWrite),
    (27, TargetState::StorageWrite),
    (28, TargetState::StorageWrite),
    (29, TargetState::StorageWrite),
    (33, TargetState::StorageWrite),
    (34, TargetState::StorageWrite),
    (36, TargetState::StorageWrite),
    (37, TargetState::StorageWrite),
    (38, TargetState::StorageWrite),
    (42, TargetState::StorageWrite),
    (43, TargetState::StorageWrite),
    (U_SCENE_DEPTH, TargetState::StorageWrite),
    (G35L_OIT_ACC, TargetState::StorageWrite),
    (G35L_OIT_SAT, TargetState::StorageWrite),
];
const G35L_PLAN_OIT_WRESOLVE: &[(u32, TargetState)] = &[
    (G35L_OIT_PARAMS, TargetState::StorageWrite),
    (G35L_OIT_ACC, TargetState::StorageWrite),
    (U_SCENE_COLOR, TargetState::StorageWrite),
];

// ---------------------------------------------------------------------------
// 小件助手(bin 局部;g34 同型复制)
// ---------------------------------------------------------------------------

fn g35l_file_sha(path: &str) -> String {
    match std::fs::read(path) {
        Ok(b) => format!("sha256:{}", sha256_hex(&b)),
        Err(_) => "MISSING".to_owned(),
    }
}

fn g35l_stats(v: &[f64]) -> (f64, f64, f64) {
    if v.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = s.iter().sum::<f64>() / s.len() as f64;
    (mean, s[0], s[s.len() - 1])
}

/// BGRA8 帧内容 digest(payload = `G31BGRA-1\0` + w/h LE + 打包字节;g31 A3
/// /g34 同字面——跨 bin digest 可比对)。
fn g35l_bgra_digest(w: u32, h: u32, bytes: &[u8]) -> String {
    let mut payload = b"G31BGRA-1\0".to_vec();
    payload.extend_from_slice(&w.to_le_bytes());
    payload.extend_from_slice(&h.to_le_bytes());
    payload.extend_from_slice(bytes);
    format!("sha256:{}", sha256_hex(&payload))
}

/// SPV 装载(存在性前置三态;inject = NoContraction 注入——sim/emit 沿 G35-2
/// probe 律,splat/presolve 供 MV 见证 host 对拍收紧;scan/compact/args/clear
/// 纯整数或零浮点面不注入)。
fn g35l_load_spv_bytes(path: &str, inject: bool) -> Vec<u8> {
    if !Path::new(path).is_file() {
        dev_env_or_fail("spv_assets", &format!("SPV 缺失: {path}"));
    }
    let words = load_spv(path);
    let words = if inject {
        spv_inject_no_contraction(&words)
    } else {
        words
    };
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

// ---------------------------------------------------------------------------
// 相机(g34 同型 bin 局部自持:auto-move 确定性轨迹 + spec 重建)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct G35Camera {
    eye: [f32; 3],
    yaw: f32,
    pitch: f32,
    up0: [f32; 3],
    fov_y_rad: f32,
    near: f32,
    far: f32,
}

impl G35Camera {
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

/// auto-move 确定性轨迹(帧号唯一事实源,绝对位姿;g34_auto_move_pose 逐字同模)。
///
/// `amp` = 位移倍率(`--auto-move-amp`,缺省 1.0):orbit/dolly 的**位移项**乘
/// amp,yaw 角量不乘(角量与轨长无关)。乘法写作左结合 `0.35 * amp * a.sin()`
/// = `(0.35·amp)·sin`,amp = 1.0 时 `x * 1.0` IEEE 精确 ⇒ 与旧字面逐位同值。
/// `dolly-forward` = 沿 cam0 前向 XZ 归一方向匀速推进 `d = amp·t`(t = fi/total,
/// 末帧位移 ≈ amp 米),eye.y/yaw/pitch 不变(推轨短片面,相机不摆头)。
fn g35l_auto_move_pose(
    name: &str,
    cam0: &G35Camera,
    fi: u32,
    total: u32,
    amp: f64,
) -> (f32, f32, [f32; 3]) {
    let t = f64::from(fi) / f64::from(total.max(1));
    let tau = std::f64::consts::TAU;
    match name {
        "orbit" => {
            let a = tau * t;
            let eye = [
                (f64::from(cam0.eye[0]) + 0.35 * amp * a.sin()) as f32,
                (f64::from(cam0.eye[1]) + 0.05 * amp * (2.0 * a).sin()) as f32,
                (f64::from(cam0.eye[2]) + 0.35 * amp * (a.cos() - 1.0)) as f32,
            ];
            let yaw = (f64::from(cam0.yaw) + 0.30 * a.sin()) as f32;
            (yaw, cam0.pitch, eye)
        }
        "dolly" => {
            let a = tau * t;
            let f = cam0.forward();
            let fxz = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6);
            let d = 0.50 * amp * (std::f64::consts::PI * t).sin();
            let eye = [
                (f64::from(cam0.eye[0]) + f64::from(f[0] / fxz) * d) as f32,
                (f64::from(cam0.eye[1]) + 0.03 * amp * a.sin()) as f32,
                (f64::from(cam0.eye[2]) + f64::from(f[2] / fxz) * d) as f32,
            ];
            let yaw = (f64::from(cam0.yaw) - 0.20 * a.sin()) as f32;
            (yaw, cam0.pitch, eye)
        }
        "dolly-forward" => {
            let f = cam0.forward();
            let fxz = (f[0] * f[0] + f[2] * f[2]).sqrt().max(1e-6);
            let d = amp * t;
            let eye = [
                (f64::from(cam0.eye[0]) + f64::from(f[0] / fxz) * d) as f32,
                cam0.eye[1],
                (f64::from(cam0.eye[2]) + f64::from(f[2] / fxz) * d) as f32,
            ];
            (cam0.yaw, cam0.pitch, eye)
        }
        other => fail(&format!("--auto-move 轨迹 {other} 越闭集(orbit|dolly|dolly-forward)")),
    }
}

/// 展示面 CLI 三元素解析("x,y,z" 逗号分隔;有限性守卫,fail-closed)。
fn parse_f32_triplet(s: &str, flag: &str) -> [f32; 3] {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        fail(&format!("{flag} 须为 x,y,z 三元素(实到 {} 段)", parts.len()));
    }
    let mut out = [0.0f32; 3];
    for (k, p) in parts.iter().enumerate() {
        let v: f32 = p
            .trim()
            .parse()
            .unwrap_or_else(|_| fail(&format!("{flag} 第 {} 段 {p:?} 非 f32", k + 1)));
        if !v.is_finite() {
            fail(&format!("{flag} 第 {} 段非有限值", k + 1));
        }
        out[k] = v;
    }
    out
}

// ---------------------------------------------------------------------------
// 发射器夹具(冻结常量)与 host 金标准平行镜像
// ---------------------------------------------------------------------------

/// 生产腿发射器(相机前向 2.2m + 微抬,DYN_ORIGIN_AHEAD 同律;缓落夹具——
/// 粒子驻留视野,寿命死亡覆盖经 emit_schedule 换血)。
fn g35l_emitter(cam: &CameraSpec) -> pcore::EmitterDesc {
    pcore::EmitterDesc {
        pos: [
            cam.eye[0] + cam.forward[0] * 2.2,
            cam.eye[1] + cam.forward[1] * 2.2 + 0.05,
            cam.eye[2] + cam.forward[2] * 2.2,
        ],
        spread: [0.25, 0.15, 0.25],
        vel_base: [0.0, 0.9, 0.0],
        vel_spread: [0.35, 0.25, 0.35],
        life_base: 1.4,
        gravity_y: -0.9,
    }
}

/// 见证腿发射器(单粒子确定构型:spread/vel_spread 全零,寿命长于见证窗)。
/// mv 腿:相机前向 2.0m + 上抬 0.3m(上半屏,存储域拒绝梯度可预期,命中
/// 像素非空);occlusion 腿:相机**后方** 2.0m(bistro 室内相机身后已知墙
/// 后;投影 w 门确定性拒绝路径,bin 头注深度域登记)。
fn g35l_witness_emitter(cam: &CameraSpec, behind: bool) -> pcore::EmitterDesc {
    let s: f32 = if behind { -2.0 } else { 2.0 };
    let lift: f32 = if behind { 0.0 } else { 0.3 };
    pcore::EmitterDesc {
        pos: [
            cam.eye[0] + cam.forward[0] * s,
            cam.eye[1] + cam.forward[1] * s + lift,
            cam.eye[2] + cam.forward[2] * s,
        ],
        spread: [0.0, 0.0, 0.0],
        vel_base: if behind { [0.0, 0.0, 0.0] } else { [0.35, 0.15, 0.0] },
        vel_spread: [0.0, 0.0, 0.0],
        life_base: 30.0,
        gravity_y: 0.0,
    }
}

/// G35-4 近远见证发射器(视轴构型:相机前向 2.0m 视轴上 ⇒ 两粒子投影同
/// 像素;纯前向速度 0.4 m/s ⇒ 先发者更远(深度差 = 0.4·Δage);spread/
/// vel_spread 全零,寿命长于见证窗,零重力)。
fn g35l_oit_witness_emitter(cam: &CameraSpec) -> pcore::EmitterDesc {
    pcore::EmitterDesc {
        pos: [
            cam.eye[0] + cam.forward[0] * 2.0,
            cam.eye[1] + cam.forward[1] * 2.0,
            cam.eye[2] + cam.forward[2] * 2.0,
        ],
        spread: [0.0, 0.0, 0.0],
        vel_base: [
            cam.forward[0] * 0.4,
            cam.forward[1] * 0.4,
            cam.forward[2] * 0.4,
        ],
        vel_spread: [0.0, 0.0, 0.0],
        life_base: 30.0,
        gravity_y: 0.0,
    }
}

/// 帧控制块(host 金标准平行推得;device dispatch/params 唯一事实源——
/// 零回读链:host 不读回 device 计数,只对拍验证)。
#[derive(Clone, Copy)]
struct G35FrameCtl {
    n_curr: usize,
    nseg_curr: usize,
    emit_count: usize,
    pid_base: u32,
    args_host: [u32; 8],
    n_next: usize,
}

/// host 金标准平行镜像(particles::core::frame 单源;读 A 写 B 帧末交换——
/// 与 device 帧序逐字同协议;NoContraction 注入面下 W2 实测 f32 位级同源,
/// 整数流零容差 ⇒ n_curr/emit 参数面与 device 内部计数恒一致)。
/// 发射调度闭集(Production = G35-2 冻结字面;SingleF0 = mv/遮挡见证单粒子;
/// OitPair = G35-4 近远见证两粒子:帧 0 与帧 G35L_OIT_WITNESS_F2 各发 1——
/// 纯前向 vel 夹具下先发者更远且 age 大 ⇒ 远者偏红近者偏白,同像素叠序可判)。
#[derive(Clone, Copy, PartialEq)]
enum G35EmitSched {
    Production,
    SingleF0,
    OitPair,
}

struct G35HostMirror {
    a: pcore::ParticlePools,
    b: pcore::ParticlePools,
    desc: pcore::EmitterDesc,
    table: Vec<f32>,
    pid_base: u32,
    cap: usize,
    /// 发射调度模式(见证腿闭集)。
    sched: G35EmitSched,
    /// 生产发射上限(`--emit-max`;缺省 `G35L_EMIT_MAX` ⇒ 冻结节奏逐字,
    /// 非缺省时 Production 预算按 `emit_max/256` 线性放大)。
    emit_max: usize,
}

impl G35HostMirror {
    fn new(cap: usize, seed: u64, desc: pcore::EmitterDesc, sched: G35EmitSched) -> Self {
        Self {
            a: pcore::ParticlePools::with_capacity(cap),
            b: pcore::ParticlePools::with_capacity(cap),
            desc,
            table: p_rand_table(seed),
            pid_base: 0,
            cap,
            sched,
            emit_max: G35L_EMIT_MAX as usize,
        }
    }

    /// 确定性发射预算(G35-2 冻结字面:min(64 + f·17 % 192, cap − n_curr);
    /// SingleF0 = 帧 0 单发射;OitPair = 帧 0/帧 30 各发 1)。
    /// `--emit-max` 非缺省时 Production 臂 = min((64 + f·17 % 192)·emit_max/256,
    /// cap − n_curr)(整数算术,峰值 255·emit_max/256 < emit_max = dispatch 上界);
    /// 缺省臂字面不动。
    fn emit_schedule(&self, f: u32) -> usize {
        match self.sched {
            G35EmitSched::SingleF0 => usize::from(f == 0),
            G35EmitSched::OitPair => usize::from(f == 0 || f == G35L_OIT_WITNESS_F2),
            G35EmitSched::Production if self.emit_max == G35L_EMIT_MAX as usize => (64 + (f as usize * 17) % 192).min(self.cap - self.a.n),
            G35EmitSched::Production => ((64 + (f as usize * 17) % 192) * self.emit_max / G35L_EMIT_MAX as usize).min(self.cap - self.a.n),
        }
    }

    /// 一帧平行推进:返回本帧控制块(device params 上传源),帧末 A/B 交换。
    fn step(&mut self, f: u32) -> G35FrameCtl {
        let n_curr = self.a.n;
        let nseg_curr = n_curr.div_ceil(SEG);
        let emit_count = self.emit_schedule(f);
        let pid_base = self.pid_base;
        let stats = pcore::frame(
            &mut self.a,
            &mut self.b,
            &self.desc,
            &self.table,
            G35L_DT,
            pid_base,
            emit_count,
        );
        self.pid_base += emit_count as u32;
        std::mem::swap(&mut self.a, &mut self.b);
        G35FrameCtl {
            n_curr,
            nseg_curr,
            emit_count,
            pid_base,
            args_host: stats.args,
            n_next: stats.n_next,
        }
    }

    /// 帧末池态(post-swap = 本帧压缩+发射后前缀;见证腿 slot 0 读取面)。
    fn current(&self) -> &pcore::ParticlePools {
        &self.a
    }
}

// ---------------------------------------------------------------------------
// 渲染 params 打包(64 f32;三渲染 kernel 共享布局单源——kernel 头注镜像)
// ---------------------------------------------------------------------------

/// 展示面效果参数(网站出图/演示面;默认 = 冻结生产字面 ⇒ 位级零漂移:
/// stretch/tint/alpha 的 ×1.0 与 /1.0 均 IEEE 精确,r_world 默认 = 冻结常量)。
#[derive(Clone, Copy)]
struct G35FxParams {
    /// 粒子世界半径(米;默认 = G35L_R_WORLD 冻结字面)。
    r_world: f32,
    /// 雨丝竖直拉伸比(params[56];1.0 = 圆点)。
    stretch: f32,
    /// 展示面调色乘子(params[57..60);1.0 = 程序化调色冻结形;雨丝模式下
    /// = 雨滴 scene-linear 绝对色)。
    tint: [f32; 3],
    /// 不透明度乘子(params[60];1.0 = 冻结 alpha)。
    alpha_scale: f32,
    /// 雨丝模式快门占 dt 比(params[61];0.0 = 冻结面〔椭圆 splat + 火焰调色
    /// + quirk 域软粒子〕;>0 = 运动模糊胶囊 + TLAS 逐粒子遮挡 + reactive
    /// 掩码,kernels/g35_render_splat.rx / g35_render_resolve.rx「雨丝模式」
    /// 头注)。
    rain_shutter: f32,
    /// 雨丝模式 TLAS 遮挡开关(params[62];仅 rain_shutter > 0 时消费)。
    rain_occlusion: bool,
}

impl Default for G35FxParams {
    fn default() -> Self {
        Self {
            r_world: G35L_R_WORLD,
            stretch: 1.0,
            tint: [1.0, 1.0, 1.0],
            alpha_scale: 1.0,
            rain_shutter: 0.0,
            rain_occlusion: true,
        }
    }
}

impl G35FxParams {
    /// 雨丝模式启用判(kernel params[61] > 0 同判;TSR has_reactive 同源)。
    fn rain_on(&self) -> bool {
        self.rain_shutter > 0.0
    }
}

/// 雨丝遮挡射线 t_min(米;相机近旁自遮挡防护,kernel 内按射线长度归一)。
const G35L_RAIN_RAY_TMIN: f32 = 0.02;
/// P_PARAMS_RENDER 长度(f32;G35-3 冻结 64 → 雨丝模式扩 80,[64..80) 追加段;
/// OIT params 镜像仍取 [0..64) 逐字——OIT kernel 布局 0-byte)。
const G35L_RENDER_PARAMS_LEN: usize = 80;

/// P_PARAMS_RENDER 布局(kernels/g35_splat_clear/g35_render_splat/
/// g35_render_resolve 头注逐字同源):
///   [0]=iw [1]=ih [2]=r_world [3]=soft_range [4]=d_max(far) [5]=dt
///   [6]=px_count [7]=p11(投影阵 [1][1] = 1/tan(fov_y/2))
///   [8..24)=vp_j [24..40)=vp(未抖;行 0/1 = 生产字面深度域,行 3 = 视深)
///   [40..56)=prev_vp_j [56]=stretch_y [57..60)=tint_rgb
///   [60]=alpha_scale [61]=rain_shutter(0 = 冻结面) [62]=rain_occlusion
///   [63]=rain_inv_exposure(= 1/exposure;雨滴色以 display 域指定,kernel 换回
///   scene-linear) [64..67)=eye_xyz(相机世界位置,遮挡射线原点)
///   [67]=ray_tmin [68..80)=reserved(恒 0)。
///   雨丝模式关闭时 [61..80) 恒 0 ⇒ kernel 冻结面分支逐字执行(默认面位级
///   零漂移;[0..64) 与 G35-3 冻结布局逐字同值)。
#[allow(clippy::too_many_arguments)]
fn g35l_pack_render_params(
    iw: u32,
    ih: u32,
    p11: f32,
    d_max: f32,
    vp_j: &Mat4,
    vp: &Mat4,
    prev_vp_j: &Mat4,
    eye: [f32; 3],
    exposure: f32,
    fx: &G35FxParams,
) -> Vec<f32> {
    let mut v = vec![
        iw as f32,
        ih as f32,
        fx.r_world,
        G35L_SOFT_RANGE,
        d_max,
        G35L_DT,
        (iw * ih) as f32,
        p11,
    ];
    for m in [vp_j, vp, prev_vp_j] {
        for r in 0..4 {
            for c in 0..4 {
                v.push(m.m[r][c]);
            }
        }
    }
    v.resize(G35L_RENDER_PARAMS_LEN, 0.0);
    v[56] = fx.stretch;
    v[57] = fx.tint[0];
    v[58] = fx.tint[1];
    v[59] = fx.tint[2];
    v[60] = fx.alpha_scale;
    if fx.rain_on() {
        v[61] = fx.rain_shutter;
        v[62] = if fx.rain_occlusion { 1.0 } else { 0.0 };
        v[63] = if exposure > 0.0 { 1.0 / exposure } else { 1.0 };
        v[64] = eye[0];
        v[65] = eye[1];
        v[66] = eye[2];
        v[67] = G35L_RAIN_RAY_TMIN;
    }
    v
}

/// P_OIT_PARAMS 布局(kernels/g35_oit_tilekey/g35_oit_blend_sorted/
/// g35_oit_wboit_accum/g35_oit_wboit_resolve 头注逐字同源;host 金标准 =
/// particles/oit_arms.rs 同布局消费):
///   [0..64) = P_PARAMS_RENDER 64 f32 逐字镜像(g35l_pack_render_params);
///   [64]=tiles_x [65]=tiles_y [66]=tile_cnt [67]=red_flag(1 = --red-arm
///   key-invert 键反转篡改臂)[68..96)=reserved(恒 0)。
#[allow(clippy::too_many_arguments)]
fn g35l_pack_oit_params(
    iw: u32,
    ih: u32,
    p11: f32,
    d_max: f32,
    vp_j: &Mat4,
    vp: &Mat4,
    prev_vp_j: &Mat4,
    red_arm: bool,
    fx: &G35FxParams,
) -> Vec<f32> {
    // 镜像段 = 冻结 64 f32(雨丝模式与 OIT 档互斥,[61..80) 追加段截去 ⇒
    // OIT kernel 消费布局 0-byte;eye 传零向量不进镜像)。
    let mut v =
        g35l_pack_render_params(iw, ih, p11, d_max, vp_j, vp, prev_vp_j, [0.0; 3], 1.0, fx);
    v.truncate(64);
    let (tx, ty) = (iw.div_ceil(16), ih.div_ceil(16));
    v.push(tx as f32);
    v.push(ty as f32);
    v.push((tx * ty) as f32);
    v.push(if red_arm { 1.0 } else { 0.0 });
    v.resize(96, 0.0);
    v
}

/// tile 网格参数(tiles_x, tiles_y, tile_cnt;键域守卫消费)。
fn g35l_tile_grid(iw: u32, ih: u32) -> (u32, u32, u32) {
    let (tx, ty) = (iw.div_ceil(16), ih.div_ceil(16));
    (tx, ty, tx * ty)
}

/// 粒子 MV 解析期望(kernels/g35_render_resolve.rx ⑤ 段逐字同式:左结合 +
/// w 门同阈;mv = prev_uv − cur_uv,车道 MV 语义 = g14_mv.rx 输出同约定)。
fn g35l_particle_mv_expect(
    pos: [f32; 3],
    vel: [f32; 3],
    vp_j: &Mat4,
    prev_vp_j: &Mat4,
) -> [f32; 2] {
    let m = &vp_j.m;
    let ccx = ((m[0][0] * pos[0] + m[0][1] * pos[1]) + m[0][2] * pos[2]) + m[0][3];
    let ccy = ((m[1][0] * pos[0] + m[1][1] * pos[1]) + m[1][2] * pos[2]) + m[1][3];
    let ccw = ((m[3][0] * pos[0] + m[3][1] * pos[1]) + m[3][2] * pos[2]) + m[3][3];
    let pp = [
        pos[0] - vel[0] * G35L_DT,
        pos[1] - vel[1] * G35L_DT,
        pos[2] - vel[2] * G35L_DT,
    ];
    let pm = &prev_vp_j.m;
    let pcx = ((pm[0][0] * pp[0] + pm[0][1] * pp[1]) + pm[0][2] * pp[2]) + pm[0][3];
    let pcy = ((pm[1][0] * pp[0] + pm[1][1] * pp[1]) + pm[1][2] * pp[2]) + pm[1][3];
    let pcw = ((pm[3][0] * pp[0] + pm[3][1] * pp[1]) + pm[3][2] * pp[2]) + pm[3][3];
    let mut mvx = 0.0f32;
    let mut mvy = 0.0f32;
    if ccw > 0.000_000_01 && pcw > 0.000_000_01 {
        let cur_u = 0.5 * (ccx / ccw + 1.0);
        let cur_v = 0.5 * (1.0 - ccy / ccw);
        let prev_u = 0.5 * (pcx / pcw + 1.0);
        let prev_v = 0.5 * (1.0 - pcy / pcw);
        mvx = prev_u - cur_u;
        mvy = prev_v - cur_v;
    }
    [mvx, mvy]
}

// ---------------------------------------------------------------------------
// 粒子 SPV/字节所有者 + on 面描述组装配
// ---------------------------------------------------------------------------

/// 粒子 10 kernel SPV 字节 + 创建期常量字节所有者(descs 借用源;声明序 =
/// drop 逆序纪律与 g34 同)。
struct G35ParticleBits {
    spv_sim: Vec<u8>,
    spv_seg_sum: Vec<u8>,
    spv_spine: Vec<u8>,
    spv_seg_apply: Vec<u8>,
    spv_compact: Vec<u8>,
    spv_emit: Vec<u8>,
    spv_args: Vec<u8>,
    spv_clear: Vec<u8>,
    spv_splat: Vec<u8>,
    spv_presolve: Vec<u8>,
    rand_bytes: Vec<u8>,
    sim_params0: Vec<u8>,
    core_params0: Vec<u8>,
    emit_params0: Vec<u8>,
    args_params0: Vec<u8>,
    render_params0: Vec<u8>,
}

impl G35ParticleBits {
    #[allow(clippy::too_many_arguments)]
    fn load(spv: &G35SpvPaths, seed: u64) -> Self {
        Self {
            // NoContraction 注入面:sim/emit = G35-2 probe 律(host 镜像
            // f32 位级同源前提);splat/presolve = MV 见证 host 对拍收紧;
            // scan×3/compact/args 纯整数、clear 零浮点——不注入。
            spv_sim: g35l_load_spv_bytes(&spv.p_sim, true),
            spv_seg_sum: g35l_load_spv_bytes(&spv.p_seg_sum, false),
            spv_spine: g35l_load_spv_bytes(&spv.p_spine, false),
            spv_seg_apply: g35l_load_spv_bytes(&spv.p_seg_apply, false),
            spv_compact: g35l_load_spv_bytes(&spv.p_compact, false),
            spv_emit: g35l_load_spv_bytes(&spv.p_emit, true),
            spv_args: g35l_load_spv_bytes(&spv.p_args, false),
            spv_clear: g35l_load_spv_bytes(&spv.splat_clear, false),
            spv_splat: g35l_load_spv_bytes(&spv.splat, true),
            spv_presolve: g35l_load_spv_bytes(&spv.presolve, true),
            rand_bytes: bytes_f32(&p_rand_table(seed)),
            sim_params0: vec![0u8; 8 * 4],
            core_params0: vec![0u8; 4 * 4],
            emit_params0: vec![0u8; 16 * 4],
            args_params0: vec![0u8; 4 * 4],
            render_params0: vec![0u8; G35L_RENDER_PARAMS_LEN * 4],
        }
    }
}

/// G35-4 OIT kernel SPV 字节 + 创建期常量字节所有者(descs 借用源;
/// oit ≠ off 档装载)。
struct G35OitBits {
    spv_hash_clear: Vec<u8>,
    spv_tilekey: Vec<u8>,
    spv_sort_hist: Vec<u8>,
    spv_sort_spine: Vec<u8>,
    spv_sort_scatter: Vec<u8>,
    spv_tilerange: Vec<u8>,
    spv_blend: Vec<u8>,
    spv_accum: Vec<u8>,
    spv_wresolve: Vec<u8>,
    oit_params0: Vec<u8>,
    sort_p0_0: Vec<u8>,
    sort_p1_0: Vec<u8>,
    sort_p2_0: Vec<u8>,
    /// tile 哨兵清扫恒值 params([ncell = tile_cnt+1, nseg_cells, 0, 0];
    /// g35_hash_clear.rx 参数面 0-byte 消费)。
    tclear_bytes: Vec<u8>,
    /// acc 清零恒值 params(64 f32,[6] = 2·px——g35_splat_clear.rx 只消费
    /// [6],acc u32×4px 按 u64×2px 语义清 0,SSBO 无类型登记面)。
    aclear_bytes: Vec<u8>,
}

impl G35OitBits {
    fn load(spv: &G35SpvPaths, iw: u32, ih: u32) -> Self {
        let (_, _, tile_cnt) = g35l_tile_grid(iw, ih);
        let ncell = tile_cnt + 1; // 含溢出 tile
        let nseg_cells = ncell.div_ceil(256);
        let tclear: Vec<f32> = vec![ncell as f32, nseg_cells as f32, 0.0, 0.0];
        let mut aclear = vec![0.0f32; 64];
        aclear[6] = (iw * ih * 2) as f32;
        Self {
            // NoContraction 注入面:tilekey/blend/accum/wresolve = f32 投影/
            // 合成面(host 对拍收紧);sort×3/tilerange/hash_clear 纯整数或
            // 零浮点判据——不注入。
            spv_hash_clear: g35l_load_spv_bytes(&spv.oit_hash_clear, false),
            spv_tilekey: g35l_load_spv_bytes(&spv.oit_tilekey, true),
            spv_sort_hist: g35l_load_spv_bytes(&spv.oit_sort_hist, false),
            spv_sort_spine: g35l_load_spv_bytes(&spv.oit_sort_spine, false),
            spv_sort_scatter: g35l_load_spv_bytes(&spv.oit_sort_scatter, false),
            spv_tilerange: g35l_load_spv_bytes(&spv.oit_tilerange, false),
            spv_blend: g35l_load_spv_bytes(&spv.oit_blend, true),
            spv_accum: g35l_load_spv_bytes(&spv.oit_accum, true),
            spv_wresolve: g35l_load_spv_bytes(&spv.oit_wresolve, true),
            oit_params0: vec![0u8; 96 * 4],
            sort_p0_0: vec![0u8; 4 * 4],
            sort_p1_0: vec![0u8; 4 * 4],
            sort_p2_0: vec![0u8; 4 * 4],
            tclear_bytes: bytes_f32(&tclear),
            aclear_bytes: bytes_f32(&aclear),
        }
    }
}

/// on 面描述组(Vec 面——session 切片消费;母版 22/4/4 逐项克隆 + 粒子
/// 32 资源/10 pass/3 readback + encode 追加,母版项 0-byte)。
struct G35OnDescs<'x> {
    resources: Vec<ResourceDesc<'x>>,
    passes: Vec<Pass<'x>>,
    barriers: Vec<&'static [(u32, TargetState)]>,
    readbacks: Vec<Readback>,
}

/// parity → 九流组基址(p=0:cur=A/dst=B;帧末交换语义经逐帧换绑承载)。
fn g35l_groups(p: usize) -> (u32, u32) {
    if p == 0 {
        (G35L_GROUP_A, G35L_GROUP_B)
    } else {
        (G35L_GROUP_B, G35L_GROUP_A)
    }
}

fn g35l_bind_sim(p: usize) -> Bindings {
    let (cur, _) = g35l_groups(p);
    Bindings {
        storage_buffers: vec![
            G35L_P_SIM_PARAMS,
            cur,
            cur + 1,
            cur + 2,
            cur + 3,
            cur + 4,
            cur + 5,
            cur + 6,
            cur + 7,
            G35L_FLAGS,
        ],
        ..Bindings::default()
    }
}

fn g35l_bind_compact(p: usize) -> Bindings {
    let (cur, dst) = g35l_groups(p);
    let mut v = vec![G35L_P_CORE_PARAMS, G35L_FLAGS, G35L_SCAN_OUT];
    for k in 0..9 {
        v.push(cur + k);
    }
    for k in 0..9 {
        v.push(dst + k);
    }
    Bindings {
        storage_buffers: v,
        ..Bindings::default()
    }
}

fn g35l_bind_emit(p: usize) -> Bindings {
    let (_, dst) = g35l_groups(p);
    let mut v = vec![G35L_P_EMIT_PARAMS, G35L_SEG_OFFSETS, G35L_RAND];
    for k in 0..9 {
        v.push(dst + k);
    }
    Bindings {
        storage_buffers: v,
        ..Bindings::default()
    }
}

/// splat 绑定(kernel 签名序:tlas / params / args / pos×3 / vel×3 /
/// scene_depth / winner;AS 表 0 = 场景 TLAS,雨丝模式逐粒子遮挡射线消费,
/// 冻结面不 trace——绑定常驻使 A/B parity 覆盖 set0 布局键恒等)。
fn g35l_bind_splat(p: usize) -> Bindings {
    let (_, dst) = g35l_groups(p);
    Bindings {
        accel_structs: vec![0],
        storage_buffers: vec![
            G35L_P_RENDER_PARAMS,
            G35L_ARGS,
            dst,
            dst + 1,
            dst + 2,
            dst + 3,
            dst + 4,
            dst + 5,
            U_SCENE_DEPTH,
            G35L_WINNER,
        ],
        ..Bindings::default()
    }
}

/// presolve 绑定(kernel 签名序:params / vis / pos×3 / vel×3 / age / life /
/// scene_depth / scene_color / mv_out / reactive;U_REACTIVE 仅雨丝模式逐
/// 像素写,冻结面不触——母版 has_reactive = 0 面该缓冲不被 TSR 消费)。
fn g35l_bind_presolve(p: usize) -> Bindings {
    let (_, dst) = g35l_groups(p);
    Bindings {
        storage_buffers: vec![
            G35L_P_RENDER_PARAMS,
            G35L_WINNER,
            dst,
            dst + 1,
            dst + 2,
            dst + 3,
            dst + 4,
            dst + 5,
            dst + 6,
            dst + 7,
            U_SCENE_DEPTH,
            U_SCENE_COLOR,
            U_MV_OUT,
            U_REACTIVE,
        ],
        ..Bindings::default()
    }
}

/// OIT tilekey 绑定(kernel 签名序:params/args/pos×3/keys/payload;
/// B 组 = 压缩+发射后前缀,parity 换绑与 splat 同律)。
fn g35l_bind_oit_tilekey(p: usize) -> Bindings {
    let (_, dst) = g35l_groups(p);
    Bindings {
        storage_buffers: vec![
            G35L_OIT_PARAMS,
            G35L_ARGS,
            dst,
            dst + 1,
            dst + 2,
            G35L_OIT_KEYS_A,
            G35L_OIT_PAY_A,
        ],
        ..Bindings::default()
    }
}

/// OIT blend_sorted 绑定(kernel 签名序:params/payload_b/tile_start/
/// tile_end/pos×3/age/life/scene_depth/scene_color)。
fn g35l_bind_oit_blend(p: usize) -> Bindings {
    let (_, dst) = g35l_groups(p);
    Bindings {
        storage_buffers: vec![
            G35L_OIT_PARAMS,
            G35L_OIT_PAY_B,
            G35L_OIT_TILE_START,
            G35L_OIT_TILE_END,
            dst,
            dst + 1,
            dst + 2,
            dst + 6,
            dst + 7,
            U_SCENE_DEPTH,
            U_SCENE_COLOR,
        ],
        ..Bindings::default()
    }
}

/// OIT wboit_accum 绑定(kernel 签名序:params/args/pos×3/age/life/
/// scene_depth/acc/sat)。
fn g35l_bind_oit_accum(p: usize) -> Bindings {
    let (_, dst) = g35l_groups(p);
    Bindings {
        storage_buffers: vec![
            G35L_OIT_PARAMS,
            G35L_ARGS,
            dst,
            dst + 1,
            dst + 2,
            dst + 6,
            dst + 7,
            U_SCENE_DEPTH,
            G35L_OIT_ACC,
            G35L_OIT_SAT,
        ],
        ..Bindings::default()
    }
}

/// on 面描述组装配:母版四 pass 中缝(pass1 mv 与 pass2 resample 之间)插
/// 10 粒子 pass + 尾追 encode;资源 22..=53 追加;readback 表尾追 scene
/// color/winner/BGRA 三路(母版 4 路 0-byte 前缀)。G35-4:oit ≠ off 时
/// presolve(11)之后插 OIT pass 组(sorted 13 / wboit 3,bin 头注冻结序),
/// 资源 54..=70 + acc/sat 两路 readback 追加;off = 零追加(digest 位级 ==
/// 缺省的结构性保证)。
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn g35_on_descs<'x>(
    mother: (
        [ResourceDesc<'x>; U_RESOURCE_COUNT],
        [Pass<'x>; 4],
        [&'static [(u32, TargetState)]; 4],
        [Readback; 4],
    ),
    pbits: &'x G35ParticleBits,
    oit: G35Oit,
    obits: Option<&'x G35OitBits>,
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
    cap: usize,
    emit_max: u32,
) -> G35OnDescs<'x> {
    let (m_res, m_passes, m_barriers, m_readbacks) = mother;
    let ipc = (iw * ih) as u64;
    let opc = (ow * oh) as u64;
    let nseg_cap = (cap / SEG) as u32;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let indirect_storage = BufferUsage {
        storage: true,
        indirect: true,
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
    // 零初始化池(Box::leak → &'static,session 创建期一次;所有 device 驻留
    // 缓冲创建期零字节上传——未初始化显存垃圾进 fetch_max/scan 消费面是双跑
    // 不定性来源之一,与屏障全序修法双保险)。
    let pool_size = (cap as u64 * 4)
        .max(ipc * 8)
        .max(ipc * 16) // G35-4 wboit acc(u32×4px)零初始化上界
        .max(opc * 4)
        .max((u64::from(nseg_cap) + 1) * 4)
        .max(8 * 4) as usize;
    let zpool: &'static [u8] = Box::leak(vec![0u8; pool_size].into_boxed_slice());
    let dev_buf = move |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: Some(&zpool[..size as usize]),
            device_local: true,
        })
    };
    let mut resources = m_res.to_vec();
    debug_assert_eq!(resources.len(), U_RESOURCE_COUNT);
    // 22..=26 粒子 params 五件(host-visible 逐帧 buffer_uploads 目标)。
    resources.push(host_init(&pbits.sim_params0));
    resources.push(host_init(&pbits.core_params0));
    resources.push(host_init(&pbits.emit_params0));
    resources.push(host_init(&pbits.args_params0));
    resources.push(host_init(&pbits.render_params0));
    // 27..=44 九流 A/B 两组 18 件(f32/u32 同宽 4B×cap;device 驻留)。
    for _ in 0..18 {
        resources.push(dev_buf((cap * 4) as u64));
    }
    // 45..=48 flags/scan_out/seg_sums/seg_offsets。
    resources.push(dev_buf((cap * 4) as u64));
    resources.push(dev_buf((cap * 4) as u64));
    resources.push(dev_buf(u64::from(nseg_cap) * 4));
    resources.push(dev_buf((u64::from(nseg_cap) + 1) * 4));
    // 49 args(indirect 消费面;创建期同池零初始化)。
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: 8 * 4,
        usage: indirect_storage,
        data: Some(&zpool[..8 * 4]),
        device_local: true,
    }));
    // 50 rand_table(创建期一次上传)。
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: pbits.rand_bytes.len() as u64,
        usage: storage,
        data: Some(&pbits.rand_bytes),
        device_local: true,
    }));
    // 51 winner u64(px×8B;splat_clear 逐帧清零)。
    resources.push(dev_buf(ipc * 8));
    // 52/53 encode 两件(g34 同型)。
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: enc_params_bytes.len() as u64,
        usage: storage,
        data: Some(enc_params_bytes),
        device_local: true,
    }));
    resources.push(dev_buf(opc * 4));
    debug_assert_eq!(resources.len(), G35L_RESOURCE_COUNT);
    // ── G35-4 OIT 资源 54..=70(oit ≠ off;bin 头注布局表单一事实源)──
    if let Some(ob) = obits {
        let (_, _, tile_cnt) = g35l_tile_grid(iw, ih);
        let ncell = u64::from(tile_cnt) + 1; // 含溢出 tile
        resources.push(host_init(&ob.oit_params0)); // 54
        resources.push(host_init(&ob.sort_p0_0)); // 55
        resources.push(host_init(&ob.sort_p1_0)); // 56
        resources.push(host_init(&ob.sort_p2_0)); // 57
        for _ in 0..4 {
            // 58..=61 keys/payload A/B(u32×cap;创建期零初始化同池)。
            resources.push(dev_buf((cap * 4) as u64));
        }
        // 62..=64 hist/offs/scratch(u32×nseg_cap·256)。
        for _ in 0..3 {
            resources.push(dev_buf(u64::from(nseg_cap) * 256 * 4));
        }
        // 65/66 tile_start/tile_end(u32×(tile_cnt+1))。
        resources.push(dev_buf(ncell * 4));
        resources.push(dev_buf(ncell * 4));
        // 67 acc(u32×4px)/ 68 sat(u32×4)。
        resources.push(dev_buf(ipc * 16));
        resources.push(dev_buf(4 * 4));
        // 69/70 清扫恒值 params(host-visible 创建期一次)。
        resources.push(host_init(&ob.tclear_bytes));
        resources.push(host_init(&ob.aclear_bytes));
        debug_assert_eq!(resources.len(), G35L_OIT_RESOURCE_COUNT);
    }

    let cp = |name: &'static str, spirv: &'x [u8], dispatch: DispatchSpec, b: Bindings| {
        Pass::Compute(ComputePass {
            name,
            spirv,
            entry: None,
            dispatch,
            bindings: b,
        })
    };
    let seg_d = DispatchSpec::Direct([nseg_cap, 1, 1]);
    let one_d = DispatchSpec::Direct([1, 1, 1]);
    let px_d = DispatchSpec::Direct([iw * ih, 1, 1]);
    let m_passes = m_passes.to_vec();
    let mut passes: Vec<Pass<'x>> = Vec::with_capacity(15);
    passes.push(m_passes[0].clone()); // 0 scene
    passes.push(m_passes[1].clone()); // 1 mv
    passes.push(cp("g35_sim", &pbits.spv_sim, seg_d, g35l_bind_sim(0))); // 2
    passes.push(cp(
        "g35_scan_seg_sum",
        &pbits.spv_seg_sum,
        seg_d,
        Bindings {
            storage_buffers: vec![G35L_FLAGS, G35L_P_CORE_PARAMS, G35L_SEG_SUMS],
            ..Bindings::default()
        },
    )); // 3
    passes.push(cp(
        "g35_scan_spine",
        &pbits.spv_spine,
        one_d,
        Bindings {
            storage_buffers: vec![G35L_SEG_SUMS, G35L_P_CORE_PARAMS, G35L_SEG_OFFSETS],
            ..Bindings::default()
        },
    )); // 4
    passes.push(cp(
        "g35_scan_seg_apply",
        &pbits.spv_seg_apply,
        seg_d,
        Bindings {
            storage_buffers: vec![
                G35L_FLAGS,
                G35L_SEG_OFFSETS,
                G35L_P_CORE_PARAMS,
                G35L_SCAN_OUT,
            ],
            ..Bindings::default()
        },
    )); // 5
    passes.push(cp(
        "g35_particle_compact",
        &pbits.spv_compact,
        seg_d,
        g35l_bind_compact(0),
    )); // 6
    passes.push(cp(
        "g35_emit",
        &pbits.spv_emit,
        // emit_max 缺省 = G35L_EMIT_MAX(冻结字面);--emit-max 放大 dispatch 上界,
        // kernel 内 j < emit_count 守卫不变(LocalSize 1 ⇒ 线程数 = emit_max)。
        DispatchSpec::Direct([emit_max, 1, 1]),
        g35l_bind_emit(0),
    )); // 7
    passes.push(cp(
        "g35_indirect_args",
        &pbits.spv_args,
        one_d,
        Bindings {
            storage_buffers: vec![G35L_P_ARGS_PARAMS, G35L_SEG_OFFSETS, G35L_ARGS],
            ..Bindings::default()
        },
    )); // 8
    passes.push(cp(
        "g35_splat_clear",
        &pbits.spv_clear,
        px_d,
        Bindings {
            storage_buffers: vec![G35L_P_RENDER_PARAMS, G35L_WINNER],
            ..Bindings::default()
        },
    )); // 9
    passes.push(cp(
        "g35_render_splat",
        &pbits.spv_splat,
        DispatchSpec::Indirect {
            res: G35L_ARGS,
            offset: 0,
        },
        g35l_bind_splat(0),
    )); // 10
    passes.push(cp(
        "g35_render_resolve",
        &pbits.spv_presolve,
        px_d,
        g35l_bind_presolve(0),
    )); // 11
    // ── G35-4 OIT pass 组(presolve 之后 TSR 之前;bin 头注冻结序)──
    if let Some(ob) = obits {
        let (_, _, tile_cnt) = g35l_tile_grid(iw, ih);
        let nseg_cells = (tile_cnt + 1).div_ceil(256);
        let sort_seg_d = DispatchSpec::Direct([nseg_cap, 1, 1]);
        let cap_d = DispatchSpec::Direct([cap as u32, 1, 1]);
        let indirect_d = DispatchSpec::Indirect {
            res: G35L_ARGS,
            offset: 0,
        };
        let b = |v: Vec<u32>| Bindings {
            storage_buffers: v,
            ..Bindings::default()
        };
        match oit {
            G35Oit::Sorted => {
                // 12 tile 哨兵清扫(W7 g35_hash_clear 0-byte 消费)。
                passes.push(cp(
                    "g35_hash_clear",
                    &ob.spv_hash_clear,
                    DispatchSpec::Direct([nseg_cells, 1, 1]),
                    b(vec![
                        G35L_OIT_TCLEAR_PARAMS,
                        G35L_OIT_TILE_START,
                        G35L_OIT_TILE_END,
                    ]),
                ));
                // 13 tilekey(indirect;parity 换绑)。
                passes.push(cp(
                    "g35_oit_tilekey",
                    &ob.spv_tilekey,
                    indirect_d,
                    g35l_bind_oit_tilekey(0),
                ));
                // 14..22 W1 sort 三 kernel 3-pass(A→B→A→B,终产物 B)。
                let sort_legs: [(u32, u32, u32, u32, u32); 3] = [
                    (G35L_OIT_SORT_P0, G35L_OIT_KEYS_A, G35L_OIT_PAY_A, G35L_OIT_KEYS_B, G35L_OIT_PAY_B),
                    (G35L_OIT_SORT_P1, G35L_OIT_KEYS_B, G35L_OIT_PAY_B, G35L_OIT_KEYS_A, G35L_OIT_PAY_A),
                    (G35L_OIT_SORT_P2, G35L_OIT_KEYS_A, G35L_OIT_PAY_A, G35L_OIT_KEYS_B, G35L_OIT_PAY_B),
                ];
                for (sp, ki, pi, ko, po) in sort_legs {
                    passes.push(cp(
                        "g35_sort_hist",
                        &ob.spv_sort_hist,
                        sort_seg_d,
                        b(vec![ki, sp, G35L_OIT_HIST]),
                    ));
                    passes.push(cp(
                        "g35_sort_spine",
                        &ob.spv_sort_spine,
                        one_d,
                        b(vec![G35L_OIT_HIST, sp, G35L_OIT_OFFS]),
                    ));
                    passes.push(cp(
                        "g35_sort_scatter",
                        &ob.spv_sort_scatter,
                        sort_seg_d,
                        b(vec![ki, pi, G35L_OIT_OFFS, sp, G35L_OIT_SCRATCH, ko, po]),
                    ));
                }
                // 23 tile 区间(消费 sort p0 的 [0]=n)。
                passes.push(cp(
                    "g35_oit_tilerange",
                    &ob.spv_tilerange,
                    cap_d,
                    b(vec![
                        G35L_OIT_SORT_P0,
                        G35L_OIT_KEYS_B,
                        G35L_OIT_TILE_START,
                        G35L_OIT_TILE_END,
                    ]),
                ));
                // 24 blend(每像素区间升序串行合成;parity 换绑)。
                passes.push(cp(
                    "g35_oit_blend_sorted",
                    &ob.spv_blend,
                    px_d,
                    g35l_bind_oit_blend(0),
                ));
            }
            G35Oit::Wboit => {
                // 12 acc 清零(g35_splat_clear 0-byte 消费:params[6] = 2·px,
                // acc u32×4px 按 u64×2px 语义清 0)。
                passes.push(cp(
                    "g35_splat_clear",
                    &pbits.spv_clear,
                    DispatchSpec::Direct([iw * ih * 2, 1, 1]),
                    b(vec![G35L_OIT_ACLEAR_PARAMS, G35L_OIT_ACC]),
                ));
                // 13 accum(indirect;parity 换绑)。
                passes.push(cp(
                    "g35_oit_wboit_accum",
                    &ob.spv_accum,
                    indirect_d,
                    g35l_bind_oit_accum(0),
                ));
                // 14 resolve(定点和归一化合成)。
                passes.push(cp(
                    "g35_oit_wboit_resolve",
                    &ob.spv_wresolve,
                    px_d,
                    b(vec![G35L_OIT_PARAMS, G35L_OIT_ACC, U_SCENE_COLOR]),
                ));
            }
            G35Oit::Off => unreachable!("obits 只在 oit ≠ off 时装载"),
        }
    }
    passes.push(m_passes[2].clone()); // 12+Δ tsr resample
    passes.push(m_passes[3].clone()); // 13+Δ tsr resolve
    passes.push(cp(
        "g31_display_encode",
        enc_spv,
        DispatchSpec::Direct(enc_dispatch),
        Bindings {
            storage_buffers: vec![U_OUT_COLOR[0], G35L_ENC_PARAMS, G35L_ENC_OUT],
            ..Bindings::default()
        },
    )); // 14+Δ
    // resolve(11)→resample(12) 边:母版 resample 计划全 RW(执行器同态去重 ⇒
    // 零屏障),而 scene_color/mv_out 在中缝被粒子 resolve 覆写——bin 局部
    // 前缀强制 W 过渡(随后隐式补全落回 RW = 真屏障对),母版计划体 0-byte
    // 尾随(Box::leak 一次性,session 创建期)。
    let resample_plan: &'static [(u32, TargetState)] = Box::leak(
        [
            (U_SCENE_COLOR, TargetState::StorageWrite),
            (U_MV_OUT, TargetState::StorageWrite),
        ]
        .iter()
        .copied()
        .chain(m_barriers[2].iter().copied())
        .collect::<Vec<_>>()
        .into_boxed_slice(),
    );
    let mut barriers: Vec<&'static [(u32, TargetState)]> = vec![
        m_barriers[0],
        m_barriers[1],
        G35L_PLAN_SIM,
        G35L_PLAN_SEG_SUM,
        G35L_PLAN_SPINE,
        G35L_PLAN_SEG_APPLY,
        G35L_PLAN_COMPACT,
        G35L_PLAN_EMIT,
        G35L_PLAN_ARGS,
        G35L_PLAN_SPLAT_CLEAR,
        G35L_PLAN_SPLAT,
        G35L_PLAN_PRESOLVE,
    ];
    // G35-4 OIT pass 组计划(冻结序与 pass 装配一一对应;全 StorageWrite
    // 形态同律)。
    match oit {
        G35Oit::Sorted => {
            barriers.extend([
                G35L_PLAN_OIT_TILE_CLEAR,
                G35L_PLAN_OIT_TILEKEY,
                G35L_PLAN_OIT_HIST_P0,
                G35L_PLAN_OIT_SPINE_P0,
                G35L_PLAN_OIT_SCATTER_P0,
                G35L_PLAN_OIT_HIST_P1,
                G35L_PLAN_OIT_SPINE_P1,
                G35L_PLAN_OIT_SCATTER_P1,
                G35L_PLAN_OIT_HIST_P2,
                G35L_PLAN_OIT_SPINE_P2,
                G35L_PLAN_OIT_SCATTER_P2,
                G35L_PLAN_OIT_TILERANGE,
                G35L_PLAN_OIT_BLEND,
            ]);
        }
        G35Oit::Wboit => {
            barriers.extend([
                G35L_PLAN_OIT_ACC_CLEAR,
                G35L_PLAN_OIT_ACCUM,
                G35L_PLAN_OIT_WRESOLVE,
            ]);
        }
        G35Oit::Off => {}
    }
    barriers.extend([resample_plan, m_barriers[3], G35L_PLAN_ENCODE]);
    debug_assert_eq!(barriers.len(), passes.len());
    let mut readbacks = m_readbacks.to_vec();
    // 4 = scene color(见证/遮挡对拍面);5 = winner(见证面);6 = BGRA8;
    // oit ≠ off 追加 7 = acc(wboit 见证对拍)/ 8 = sat(饱和计数登记)。
    readbacks.push(Readback::Buffer {
        res: U_SCENE_COLOR,
        offset: 0,
        size: ipc * 12,
    });
    readbacks.push(Readback::Buffer {
        res: G35L_WINNER,
        offset: 0,
        size: ipc * 8,
    });
    readbacks.push(Readback::Buffer {
        res: G35L_ENC_OUT,
        offset: 0,
        size: opc * 4,
    });
    if obits.is_some() {
        readbacks.push(Readback::Buffer {
            res: G35L_OIT_ACC,
            offset: 0,
            size: ipc * 16,
        });
        readbacks.push(Readback::Buffer {
            res: G35L_OIT_SAT,
            offset: 0,
            size: 16,
        });
        // 9 = scene depth(见证腿 host 期望的软深度输入;末帧子集回读)。
        readbacks.push(Readback::Buffer {
            res: U_SCENE_DEPTH,
            offset: 0,
            size: ipc * 4,
        });
    }
    G35OnDescs {
        resources,
        passes,
        barriers,
        readbacks,
    }
}

// ---------------------------------------------------------------------------
// 屏障计划机核审计(fact barrier_plan_audit:每粒子 pass〔含 encode〕双
// parity bindings ⊆ 计划资源集 + indirect 资源在计划内)
// ---------------------------------------------------------------------------

struct G35AuditRow {
    name: &'static str,
    ok: bool,
    missing: Vec<u32>,
}

fn g35l_audit_pass(
    name: &'static str,
    plan: &[(u32, TargetState)],
    bindings: &[Bindings],
    extra: &[u32],
) -> G35AuditRow {
    let set: Vec<u32> = plan.iter().map(|&(r, _)| r).collect();
    let mut missing: Vec<u32> = Vec::new();
    for b in bindings {
        for &r in &b.storage_buffers {
            if !set.contains(&r) && !missing.contains(&r) {
                missing.push(r);
            }
        }
    }
    for &r in extra {
        if !set.contains(&r) && !missing.contains(&r) {
            missing.push(r);
        }
    }
    G35AuditRow {
        name,
        ok: missing.is_empty(),
        missing,
    }
}

/// 粒子 pass 组 + encode 的屏障计划机核审计(双 parity 并集;splat 追加
/// indirect 资源成员检——IndirectRead 转换本身由执行器隐式补全承载,
/// render_exec pass_requirements_with 首位推导,审计登记该委托)。
/// G35-4:oit ≠ off 时追加该档 OIT pass 组审计行(off = 原 11 行,G35-3
/// 门判读器 len==11 面 0 破坏)。
fn g35l_barrier_audit(oit: G35Oit) -> (bool, Vec<G35AuditRow>) {
    let mut rows = vec![
        g35l_audit_pass(
            "g35_sim",
            G35L_PLAN_SIM,
            &[g35l_bind_sim(0), g35l_bind_sim(1)],
            &[],
        ),
        g35l_audit_pass(
            "g35_scan_seg_sum",
            G35L_PLAN_SEG_SUM,
            &[Bindings {
                storage_buffers: vec![G35L_FLAGS, G35L_P_CORE_PARAMS, G35L_SEG_SUMS],
                ..Bindings::default()
            }],
            &[],
        ),
        g35l_audit_pass(
            "g35_scan_spine",
            G35L_PLAN_SPINE,
            &[Bindings {
                storage_buffers: vec![G35L_SEG_SUMS, G35L_P_CORE_PARAMS, G35L_SEG_OFFSETS],
                ..Bindings::default()
            }],
            &[],
        ),
        g35l_audit_pass(
            "g35_scan_seg_apply",
            G35L_PLAN_SEG_APPLY,
            &[Bindings {
                storage_buffers: vec![
                    G35L_FLAGS,
                    G35L_SEG_OFFSETS,
                    G35L_P_CORE_PARAMS,
                    G35L_SCAN_OUT,
                ],
                ..Bindings::default()
            }],
            &[],
        ),
        g35l_audit_pass(
            "g35_particle_compact",
            G35L_PLAN_COMPACT,
            &[g35l_bind_compact(0), g35l_bind_compact(1)],
            &[],
        ),
        g35l_audit_pass(
            "g35_emit",
            G35L_PLAN_EMIT,
            &[g35l_bind_emit(0), g35l_bind_emit(1)],
            &[],
        ),
        g35l_audit_pass(
            "g35_indirect_args",
            G35L_PLAN_ARGS,
            &[Bindings {
                storage_buffers: vec![G35L_P_ARGS_PARAMS, G35L_SEG_OFFSETS, G35L_ARGS],
                ..Bindings::default()
            }],
            &[],
        ),
        g35l_audit_pass(
            "g35_splat_clear",
            G35L_PLAN_SPLAT_CLEAR,
            &[Bindings {
                storage_buffers: vec![G35L_P_RENDER_PARAMS, G35L_WINNER],
                ..Bindings::default()
            }],
            &[],
        ),
        g35l_audit_pass(
            "g35_render_splat",
            G35L_PLAN_SPLAT,
            &[g35l_bind_splat(0), g35l_bind_splat(1)],
            &[G35L_ARGS], // DispatchSpec::Indirect 消费资源亦须在计划内
        ),
        g35l_audit_pass(
            "g35_render_resolve",
            G35L_PLAN_PRESOLVE,
            &[g35l_bind_presolve(0), g35l_bind_presolve(1)],
            &[],
        ),
        g35l_audit_pass(
            "g31_display_encode",
            G35L_PLAN_ENCODE,
            &[
                Bindings {
                    storage_buffers: vec![U_OUT_COLOR[0], G35L_ENC_PARAMS, G35L_ENC_OUT],
                    ..Bindings::default()
                },
                Bindings {
                    storage_buffers: vec![U_OUT_COLOR[1], G35L_ENC_PARAMS, G35L_ENC_OUT],
                    ..Bindings::default()
                },
            ],
            &[],
        ),
    ];
    let fixed = |v: Vec<u32>| Bindings {
        storage_buffers: v,
        ..Bindings::default()
    };
    match oit {
        G35Oit::Sorted => {
            rows.push(g35l_audit_pass(
                "g35_hash_clear",
                G35L_PLAN_OIT_TILE_CLEAR,
                &[fixed(vec![
                    G35L_OIT_TCLEAR_PARAMS,
                    G35L_OIT_TILE_START,
                    G35L_OIT_TILE_END,
                ])],
                &[],
            ));
            rows.push(g35l_audit_pass(
                "g35_oit_tilekey",
                G35L_PLAN_OIT_TILEKEY,
                &[g35l_bind_oit_tilekey(0), g35l_bind_oit_tilekey(1)],
                &[G35L_ARGS], // DispatchSpec::Indirect 消费资源亦须在计划内
            ));
            let sort_legs: [(&'static str, &'static [(u32, TargetState)], &'static [(u32, TargetState)], &'static [(u32, TargetState)], u32, u32, u32, u32, u32); 3] = [
                ("p0", G35L_PLAN_OIT_HIST_P0, G35L_PLAN_OIT_SPINE_P0, G35L_PLAN_OIT_SCATTER_P0,
                 G35L_OIT_SORT_P0, G35L_OIT_KEYS_A, G35L_OIT_PAY_A, G35L_OIT_KEYS_B, G35L_OIT_PAY_B),
                ("p1", G35L_PLAN_OIT_HIST_P1, G35L_PLAN_OIT_SPINE_P1, G35L_PLAN_OIT_SCATTER_P1,
                 G35L_OIT_SORT_P1, G35L_OIT_KEYS_B, G35L_OIT_PAY_B, G35L_OIT_KEYS_A, G35L_OIT_PAY_A),
                ("p2", G35L_PLAN_OIT_HIST_P2, G35L_PLAN_OIT_SPINE_P2, G35L_PLAN_OIT_SCATTER_P2,
                 G35L_OIT_SORT_P2, G35L_OIT_KEYS_A, G35L_OIT_PAY_A, G35L_OIT_KEYS_B, G35L_OIT_PAY_B),
            ];
            for (leg, hist_plan, spine_plan, scatter_plan, sp, ki, pi, ko, po) in sort_legs {
                let name_hist: &'static str = match leg {
                    "p0" => "g35_sort_hist(p0)",
                    "p1" => "g35_sort_hist(p1)",
                    _ => "g35_sort_hist(p2)",
                };
                let name_spine: &'static str = match leg {
                    "p0" => "g35_sort_spine(p0)",
                    "p1" => "g35_sort_spine(p1)",
                    _ => "g35_sort_spine(p2)",
                };
                let name_scatter: &'static str = match leg {
                    "p0" => "g35_sort_scatter(p0)",
                    "p1" => "g35_sort_scatter(p1)",
                    _ => "g35_sort_scatter(p2)",
                };
                rows.push(g35l_audit_pass(
                    name_hist,
                    hist_plan,
                    &[fixed(vec![ki, sp, G35L_OIT_HIST])],
                    &[],
                ));
                rows.push(g35l_audit_pass(
                    name_spine,
                    spine_plan,
                    &[fixed(vec![G35L_OIT_HIST, sp, G35L_OIT_OFFS])],
                    &[],
                ));
                rows.push(g35l_audit_pass(
                    name_scatter,
                    scatter_plan,
                    &[fixed(vec![ki, pi, G35L_OIT_OFFS, sp, G35L_OIT_SCRATCH, ko, po])],
                    &[],
                ));
            }
            rows.push(g35l_audit_pass(
                "g35_oit_tilerange",
                G35L_PLAN_OIT_TILERANGE,
                &[fixed(vec![
                    G35L_OIT_SORT_P0,
                    G35L_OIT_KEYS_B,
                    G35L_OIT_TILE_START,
                    G35L_OIT_TILE_END,
                ])],
                &[],
            ));
            rows.push(g35l_audit_pass(
                "g35_oit_blend_sorted",
                G35L_PLAN_OIT_BLEND,
                &[g35l_bind_oit_blend(0), g35l_bind_oit_blend(1)],
                &[],
            ));
        }
        G35Oit::Wboit => {
            rows.push(g35l_audit_pass(
                "g35_splat_clear(acc)",
                G35L_PLAN_OIT_ACC_CLEAR,
                &[fixed(vec![G35L_OIT_ACLEAR_PARAMS, G35L_OIT_ACC])],
                &[],
            ));
            rows.push(g35l_audit_pass(
                "g35_oit_wboit_accum",
                G35L_PLAN_OIT_ACCUM,
                &[g35l_bind_oit_accum(0), g35l_bind_oit_accum(1)],
                &[G35L_ARGS],
            ));
            rows.push(g35l_audit_pass(
                "g35_oit_wboit_resolve",
                G35L_PLAN_OIT_WRESOLVE,
                &[fixed(vec![G35L_OIT_PARAMS, G35L_OIT_ACC, U_SCENE_COLOR])],
                &[],
            ));
        }
        G35Oit::Off => {}
    }
    (rows.iter().all(|r| r.ok), rows)
}

// ---------------------------------------------------------------------------
// on 面车道状态机(G34TsrLane 同模 bin 局部;FrameUpdate 重映射 = 本 bin
// 自持 prepare 按插入后 pass 下标构造 overrides)
// ---------------------------------------------------------------------------

/// on 面逐帧回读选择(子集下标升序 = 解析序:[p?] → 2 mv → 4 scene →
/// 5 winner → 6 bgra → 7 acc → 8 sat;acc/sat 路仅 oit ≠ off 存在)。
#[derive(Clone, Copy, Default)]
struct G35Rb {
    out: bool,
    mv: bool,
    scene: bool,
    winner: bool,
    bgra: bool,
    acc: bool,
    sat: bool,
    depth: bool,
}

/// on 面一帧产物。
struct G35FrameRec {
    particle_gpu_ns: f64,
    oit_gpu_ns: f64,
    encode_gpu_ns: f64,
    validation_error_count: u64,
    leaked_object_count: u64,
    leaked_allocation_count: u64,
    out_color: Option<Vec<f32>>,
    mv_out: Option<Vec<f32>>,
    scene_color: Option<Vec<f32>>,
    winner: Option<Vec<u64>>,
    bgra8: Option<Vec<u8>>,
    oit_acc: Option<Vec<u32>>,
    oit_sat: Option<Vec<u32>>,
    scene_depth: Option<Vec<f32>>,
}

struct G35OnLane<'a> {
    session: DeviceFrameSession<'a>,
    parity: usize,
    has_history_state: bool,
    prev_vp_j: Option<Mat4>,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
    p11: f32,
    d_max: f32,
    /// G35-4 半透明档(off = 现面;prepare 重映射/uploads/回读消费面开关)。
    oit: G35Oit,
    /// 红臂旗标(--red-arm key-invert;P_OIT_PARAMS[67] 上传值)。
    red_arm: bool,
    /// 展示面效果参数(默认 = 冻结生产字面,位级零漂移)。
    fx: G35FxParams,
}

impl<'a> G35OnLane<'a> {
    #[allow(clippy::too_many_arguments)]
    fn create(
        descs: &'a G35OnDescs<'a>,
        accel_structs: &[AccelStructDesc<'a>],
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        p11: f32,
        d_max: f32,
        oit: G35Oit,
        red_arm: bool,
        fx: G35FxParams,
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        // frame_slots=2(顺序全同步口径;粒子逐帧换绑走顺序入口,G34 同律)。
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
            iw,
            ih,
            ow,
            oh,
            p11,
            d_max,
            oit,
            red_arm,
            fx,
        })
    }

    /// 本帧 FrameUpdate 组装:母版三小件 + 粒子五 params 上传;overrides =
    /// **重映射下标**(粒子 parity:2 sim/6 compact/7 emit/10 splat/
    /// 11 presolve;TSR parity:12 resample/13 resolve/14 encode——母版
    /// (2,3) 硬编码不消费,共享体 0-byte)。
    #[allow(clippy::too_many_arguments)]
    fn prepare(
        &self,
        jitter: [f32; 2],
        vp: &Mat4,
        vp_j: &Mat4,
        eye: [f32; 3],
        exposure: f32,
        reset: bool,
        ctl: &G35FrameCtl,
        desc: &pcore::EmitterDesc,
        scene_params: Vec<f32>,
        rb: G35Rb,
    ) -> Result<(SubmissionProvenance, FrameUpdate), String> {
        let (iw, ih, ow, oh) = (self.iw, self.ih, self.ow, self.oh);
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆(mv 参数面)")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        let has_history = !reset && self.has_history_state;
        // 雨丝模式:presolve 对赢家足迹写 U_REACTIVE = 1 ⇒ TSR resolve alpha = 1
        // 取当前帧(YCoCg 历史钳制假色根治);冻结面 has_reactive = 0 与母版同字面
        // (reactive = has_reactive·bilinear ⇒ ×0 IEEE 精确,非赢家像素零漂移)。
        let tsr_params = pack_tsr_params(
            iw,
            ih,
            ow,
            oh,
            jitter,
            exposure,
            has_history,
            self.fx.rain_on(),
        );
        let p = self.parity;
        // 粒子五 params(host 金标准平行推得;kernel 头注参数面逐字镜像)。
        let sim_params: Vec<f32> = vec![
            ctl.n_curr as f32,
            ctl.nseg_curr as f32,
            G35L_DT,
            desc.gravity_y,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        let core_params: Vec<f32> = vec![ctl.n_curr as f32, ctl.nseg_curr as f32, 0.0, 0.0];
        let emit_params: Vec<f32> = vec![
            ctl.emit_count as f32,
            ctl.pid_base as f32,
            desc.pos[0],
            desc.pos[1],
            desc.pos[2],
            desc.spread[0],
            desc.spread[1],
            desc.spread[2],
            desc.vel_base[0],
            desc.vel_base[1],
            desc.vel_base[2],
            desc.vel_spread[0],
            desc.vel_spread[1],
            desc.vel_spread[2],
            desc.life_base,
            ctl.nseg_curr as f32, // alive_slot = seg_offsets 总和槽下标
        ];
        let args_params: Vec<f32> = vec![ctl.emit_count as f32, ctl.nseg_curr as f32, 0.0, 0.0];
        let render_params = g35l_pack_render_params(
            iw, ih, self.p11, self.d_max, vp_j, vp, &prev, eye, exposure, &self.fx,
        );
        let up = |res: u32, v: &[f32]| (StableResourceId(u64::from(res) + 1), 0u64, bytes_f32(v));
        let mut uploads: Vec<(StableResourceId, u64, Vec<u8>)> = vec![
            up(U_SCENE_PARAMS, &scene_params),
            up(U_MV_PARAMS, &mv_params),
            up(U_TSR_PARAMS, &tsr_params),
            up(G35L_P_SIM_PARAMS, &sim_params),
            up(G35L_P_CORE_PARAMS, &core_params),
            up(G35L_P_EMIT_PARAMS, &emit_params),
            up(G35L_P_ARGS_PARAMS, &args_params),
            up(G35L_P_RENDER_PARAMS, &render_params),
        ];
        // G35-4 OIT params 上传(oit ≠ off;n = host 金标准平行推得 total =
        // args_host[7],零回读链同律;sort nseg = ceil(n/256))。
        if self.oit != G35Oit::Off {
            let oit_params = g35l_pack_oit_params(
                iw, ih, self.p11, self.d_max, vp_j, vp, &prev, self.red_arm, &self.fx,
            );
            uploads.push(up(G35L_OIT_PARAMS, &oit_params));
            if self.oit == G35Oit::Sorted {
                let n = ctl.args_host[7];
                let nseg = n.div_ceil(256);
                for (res, dpow) in [
                    (G35L_OIT_SORT_P0, 1.0f32),
                    (G35L_OIT_SORT_P1, 256.0),
                    (G35L_OIT_SORT_P2, 65536.0),
                ] {
                    uploads.push(up(res, &[n as f32, nseg as f32, dpow, 0.0]));
                }
            }
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
        let bindings_encode = Bindings {
            storage_buffers: vec![U_OUT_COLOR[p], G35L_ENC_PARAMS, G35L_ENC_OUT],
            ..Bindings::default()
        };
        // G35-4 重映射:OIT pass 组插入 presolve(11)之后 ⇒ TSR/encode 下标
        // 顺延 Δ = pass_delta(off 0 / sorted 13 / wboit 3);OIT parity 换绑
        // pass:sorted 13 tilekey + 24 blend,wboit 13 accum。
        let delta = self.oit.pass_delta();
        let mut binding_overrides: Vec<(u32, Bindings)> = vec![
            (2, g35l_bind_sim(p)),
            (6, g35l_bind_compact(p)),
            (7, g35l_bind_emit(p)),
            (10, g35l_bind_splat(p)),
            (11, g35l_bind_presolve(p)),
        ];
        match self.oit {
            G35Oit::Sorted => {
                binding_overrides.push((13, g35l_bind_oit_tilekey(p)));
                binding_overrides.push((24, g35l_bind_oit_blend(p)));
            }
            G35Oit::Wboit => {
                binding_overrides.push((13, g35l_bind_oit_accum(p)));
            }
            G35Oit::Off => {}
        }
        binding_overrides.push((12 + delta, bindings_resample));
        binding_overrides.push((13 + delta, bindings_resolve));
        binding_overrides.push((14 + delta, bindings_encode));
        let mut subset: Vec<u32> = Vec::new();
        if rb.out {
            subset.push(p as u32);
        }
        if rb.mv {
            subset.push(2);
        }
        if rb.scene {
            subset.push(4);
        }
        if rb.winner {
            subset.push(5);
        }
        if rb.bgra {
            subset.push(6);
        }
        if rb.acc {
            subset.push(7);
        }
        if rb.sat {
            subset.push(8);
        }
        if rb.depth {
            subset.push(9);
        }
        let update = FrameUpdate {
            tlas_update: None,
            buffer_uploads: uploads,
            binding_overrides,
            push_constant_overrides: vec![],
            readback_subset: Some(subset),
            blas_refit: None,
        };
        let prov = self.session.next_provenance_with_update(&update)?;
        Ok((prov, update))
    }

    fn rec_from_output(&self, mut out: DeviceFrameOutput, rb: G35Rb) -> Result<G35FrameRec, String> {
        // 粒子 10 pass GPU 段和(telemetry 名查找;缺行即红)。
        let mut particle_gpu_ns = 0.0f64;
        for name in [
            "g35_sim",
            "g35_scan_seg_sum",
            "g35_scan_spine",
            "g35_scan_seg_apply",
            "g35_particle_compact",
            "g35_emit",
            "g35_indirect_args",
            "g35_splat_clear",
            "g35_render_splat",
            "g35_render_resolve",
        ] {
            particle_gpu_ns += out
                .telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))?;
        }
        let encode_gpu_ns = out
            .telemetry
            .passes
            .iter()
            .find(|pp| pp.name == "g31_display_encode")
            .map(|pp| pp.gpu_ns)
            .ok_or("telemetry 缺 g31_display_encode pass 行")?;
        // G35-4 OIT pass 组 GPU 段和(按档;sort 三 kernel 同名三行经 filter
        // 求和覆盖 9 dispatch)。
        let mut oit_gpu_ns = 0.0f64;
        let oit_names: &[&str] = match self.oit {
            G35Oit::Sorted => &[
                "g35_hash_clear",
                "g35_oit_tilekey",
                "g35_sort_hist",
                "g35_sort_spine",
                "g35_sort_scatter",
                "g35_oit_tilerange",
                "g35_oit_blend_sorted",
            ],
            G35Oit::Wboit => &["g35_oit_wboit_accum", "g35_oit_wboit_resolve"],
            G35Oit::Off => &[],
        };
        for pp in &out.telemetry.passes {
            if oit_names.contains(&pp.name.as_str()) {
                oit_gpu_ns += pp.gpu_ns;
            }
        }
        let mut idx = 0usize;
        let mut take = |out: &mut DeviceFrameOutput| -> Result<Vec<u8>, String> {
            if idx >= out.readbacks.len() {
                return Err(format!("回读路数 {} 少于消费序 {idx}", out.readbacks.len()));
            }
            let b = std::mem::take(&mut out.readbacks[idx]);
            idx += 1;
            Ok(b)
        };
        let (ipc, opc) = ((self.iw * self.ih) as usize, (self.ow * self.oh) as usize);
        let out_color = if rb.out {
            let d = read_f32(&take(&mut out)?);
            if d.len() != opc * 3 {
                return Err("f32 out_color 回读与输出分辨率不符".into());
            }
            Some(d)
        } else {
            None
        };
        let mv_out = if rb.mv {
            let d = read_f32(&take(&mut out)?);
            if d.len() != ipc * 2 {
                return Err("mv_out 回读与内部分辨率不符".into());
            }
            Some(d)
        } else {
            None
        };
        let scene_color = if rb.scene {
            let d = read_f32(&take(&mut out)?);
            if d.len() != ipc * 3 {
                return Err("scene color 回读与内部分辨率不符".into());
            }
            Some(d)
        } else {
            None
        };
        let winner = if rb.winner {
            let b = take(&mut out)?;
            if b.len() != ipc * 8 {
                return Err("winner 回读与内部分辨率不符".into());
            }
            Some(
                b.chunks_exact(8)
                    .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
                    .collect(),
            )
        } else {
            None
        };
        let bgra8 = if rb.bgra {
            let b = take(&mut out)?;
            if b.len() != opc * 4 {
                return Err("BGRA8 回读与输出分辨率不符".into());
            }
            Some(b)
        } else {
            None
        };
        let read_u32 = |b: &[u8]| -> Vec<u32> {
            b.chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        };
        let oit_acc = if rb.acc {
            let b = take(&mut out)?;
            if b.len() != ipc * 16 {
                return Err("OIT acc 回读与内部分辨率不符".into());
            }
            Some(read_u32(&b))
        } else {
            None
        };
        let oit_sat = if rb.sat {
            let b = take(&mut out)?;
            if b.len() != 16 {
                return Err("OIT sat 回读长度不符(u32×4)".into());
            }
            Some(read_u32(&b))
        } else {
            None
        };
        let scene_depth = if rb.depth {
            let d = read_f32(&take(&mut out)?);
            if d.len() != ipc {
                return Err("scene depth 回读与内部分辨率不符".into());
            }
            Some(d)
        } else {
            None
        };
        if idx != out.readbacks.len() {
            return Err(format!("回读消费序 {idx} ≠ 实到路数 {}", out.readbacks.len()));
        }
        Ok(G35FrameRec {
            particle_gpu_ns,
            oit_gpu_ns,
            encode_gpu_ns,
            validation_error_count: out.telemetry.validation_error_count,
            leaked_object_count: out.telemetry.leaked_object_count,
            leaked_allocation_count: out.telemetry.leaked_allocation_count,
            out_color,
            mv_out,
            scene_color,
            winner,
            bgra8,
            oit_acc,
            oit_sat,
            scene_depth,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        jitter: [f32; 2],
        vp: &Mat4,
        vp_j: &Mat4,
        eye: [f32; 3],
        exposure: f32,
        reset: bool,
        ctl: &G35FrameCtl,
        desc: &pcore::EmitterDesc,
        scene_params: Vec<f32>,
        rb: G35Rb,
    ) -> Result<G35FrameRec, String> {
        let (prov, update) =
            self.prepare(jitter, vp, vp_j, eye, exposure, reset, ctl, desc, scene_params, rb)?;
        let out = self.session.execute_with_frame_update(&prov, &update)?;
        let rec = self.rec_from_output(out, rb)?;
        self.prev_vp_j = Some(*vp_j);
        self.has_history_state = true;
        self.parity = 1 - self.parity;
        Ok(rec)
    }
}

// ---------------------------------------------------------------------------
// CLI / 运行模式
// ---------------------------------------------------------------------------

struct G35SpvPaths {
    scene: String,
    mv: String,
    resample: String,
    resolve: String,
    encode: String,
    p_sim: String,
    p_seg_sum: String,
    p_spine: String,
    p_seg_apply: String,
    p_compact: String,
    p_emit: String,
    p_args: String,
    splat_clear: String,
    splat: String,
    presolve: String,
    // G35-4 OIT 九件(W1 sort 三件 + W7 hash_clear 0-byte 消费 + 本波五件)。
    oit_sort_hist: String,
    oit_sort_spine: String,
    oit_sort_scatter: String,
    oit_hash_clear: String,
    oit_tilekey: String,
    oit_tilerange: String,
    oit_blend: String,
    oit_accum: String,
    oit_wresolve: String,
}

#[derive(PartialEq, Clone, Copy)]
enum G35Mode {
    OffFace,
    OnFace,
    MvWitness,
    OcclusionWitness,
    MeshWitness,
    /// G35-4 近远两粒子见证腿(--oit-witness;静态相机 + OitPair 发射)。
    OitWitness,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut frames: u32 = 120;
    let mut warmup: u32 = 10;
    let mut tier: u32 = 100;
    let mut contract_path = DEFAULT_CONTRACT.to_owned();
    let mut g10_dir = G35L_DEFAULT_G10_DIR.to_owned();
    let mut gltf_path = String::new();
    let mut spv = G35SpvPaths {
        scene: DEFAULT_SPV_SCENE.to_owned(),
        mv: DEFAULT_SPV_MV.to_owned(),
        resample: DEFAULT_SPV_RESAMPLE.to_owned(),
        resolve: DEFAULT_SPV_RESOLVE.to_owned(),
        encode: G35L_DEFAULT_SPV_ENCODE.to_owned(),
        p_sim: format!("{G35L_SPV_DIR}/g35_sim.spv"),
        p_seg_sum: format!("{G35L_SPV_DIR}/g35_scan_seg_sum.spv"),
        p_spine: format!("{G35L_SPV_DIR}/g35_scan_spine.spv"),
        p_seg_apply: format!("{G35L_SPV_DIR}/g35_scan_seg_apply.spv"),
        p_compact: format!("{G35L_SPV_DIR}/g35_particle_compact.spv"),
        p_emit: format!("{G35L_SPV_DIR}/g35_emit.spv"),
        p_args: format!("{G35L_SPV_DIR}/g35_indirect_args.spv"),
        splat_clear: format!("{G35L_SPV_DIR}/g35_splat_clear.spv"),
        splat: format!("{G35L_SPV_DIR}/g35_render_splat.spv"),
        presolve: format!("{G35L_SPV_DIR}/g35_render_resolve.spv"),
        oit_sort_hist: format!("{G35L_OIT_SPV_DIR}/g35_sort_hist.spv"),
        oit_sort_spine: format!("{G35L_OIT_SPV_DIR}/g35_sort_spine.spv"),
        oit_sort_scatter: format!("{G35L_OIT_SPV_DIR}/g35_sort_scatter.spv"),
        oit_hash_clear: format!("{G35L_OIT_SPV_DIR}/g35_hash_clear.spv"),
        oit_tilekey: format!("{G35L_OIT_SPV_DIR}/g35_oit_tilekey.spv"),
        oit_tilerange: format!("{G35L_OIT_SPV_DIR}/g35_oit_tilerange.spv"),
        oit_blend: format!("{G35L_OIT_SPV_DIR}/g35_oit_blend_sorted.spv"),
        oit_accum: format!("{G35L_OIT_SPV_DIR}/g35_oit_wboit_accum.spv"),
        oit_wresolve: format!("{G35L_OIT_SPV_DIR}/g35_oit_wboit_resolve.spv"),
    };
    let mut evidence_path = String::new();
    let mut expect_digest: Option<String> = None;
    let mut particles_on = false;
    let mut static_camera = false;
    let mut auto_move: Option<String> = None;
    // 推轨短片面:轨迹位移倍率(1.0 = 冻结轨迹幅度,×1.0 IEEE 精确 ⇒ 位级零漂移)。
    let mut auto_move_amp: f64 = 1.0;
    let mut cap: usize = 65536;
    let mut seed: u64 = 42;
    let mut mv_witness = false;
    let mut occlusion_witness = false;
    let mut mesh_particles: u32 = 0;
    let mut oit = G35Oit::Off;
    let mut oit_witness = false;
    let mut red_arm = false;
    // G36 W4 geo 组合面（--cluster-lod × --wp-hlod × 粒子/OIT;off 默认 =
    // 既有面 0-byte——W1 provenance 事实源,粒子为生成几何与场景重组正交）。
    let mut cluster_lod_mode = String::from("off");
    let mut cluster_pack = String::new();
    let mut cluster_error_px: f32 = 1.0;
    let mut wp_hlod_mode = String::from("off");
    let mut wp_pack = String::new();
    let mut wp_threshold_l0: f64 = 1.0;
    // ── 展示面(网站出图)参数:全部默认 = 冻结生产字面,位级零漂移 ──
    let mut dump_raw_path = String::new();
    // 逐帧出图周期(--dump-present-every;None = 旧行为 0-byte,只写末帧)。
    let mut dump_every: Option<u32> = None;
    let mut fx = G35FxParams::default();
    let mut emitter_pos: Option<[f32; 3]> = None;
    let mut emitter_spread: Option<[f32; 3]> = None;
    let mut emitter_vel: Option<[f32; 3]> = None;
    let mut emitter_vel_spread: Option<[f32; 3]> = None;
    let mut emitter_life: Option<f32> = None;
    let mut emitter_gravity: Option<f32> = None;
    // 推轨短片面:发射中心逐帧跟随相机位移(缺省 off = 分支不执行,0-byte);
    // 生产发射上限(缺省 G35L_EMIT_MAX = 冻结节奏/dispatch 字面)。
    let mut emitter_follow = false;
    let mut emit_max: u32 = G35L_EMIT_MAX;
    let mut ev100_override: Option<f32> = None;
    let mut scene_arg = String::from("bistro-interior");
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
            "--spv-scene" => spv.scene = take_arg(&args, &mut i),
            "--spv-mv" => spv.mv = take_arg(&args, &mut i),
            "--spv-resample" => spv.resample = take_arg(&args, &mut i),
            "--spv-resolve" => spv.resolve = take_arg(&args, &mut i),
            "--spv-encode" => spv.encode = take_arg(&args, &mut i),
            "--spv-p-sim" => spv.p_sim = take_arg(&args, &mut i),
            "--spv-p-scan-seg-sum" => spv.p_seg_sum = take_arg(&args, &mut i),
            "--spv-p-scan-spine" => spv.p_spine = take_arg(&args, &mut i),
            "--spv-p-scan-seg-apply" => spv.p_seg_apply = take_arg(&args, &mut i),
            "--spv-p-compact" => spv.p_compact = take_arg(&args, &mut i),
            "--spv-p-emit" => spv.p_emit = take_arg(&args, &mut i),
            "--spv-p-indirect-args" => spv.p_args = take_arg(&args, &mut i),
            "--spv-splat-clear" => spv.splat_clear = take_arg(&args, &mut i),
            "--spv-splat" => spv.splat = take_arg(&args, &mut i),
            "--spv-presolve" => spv.presolve = take_arg(&args, &mut i),
            "--spv-oit-sort-hist" => spv.oit_sort_hist = take_arg(&args, &mut i),
            "--spv-oit-sort-spine" => spv.oit_sort_spine = take_arg(&args, &mut i),
            "--spv-oit-sort-scatter" => spv.oit_sort_scatter = take_arg(&args, &mut i),
            "--spv-oit-hash-clear" => spv.oit_hash_clear = take_arg(&args, &mut i),
            "--spv-oit-tilekey" => spv.oit_tilekey = take_arg(&args, &mut i),
            "--spv-oit-tilerange" => spv.oit_tilerange = take_arg(&args, &mut i),
            "--spv-oit-blend" => spv.oit_blend = take_arg(&args, &mut i),
            "--spv-oit-accum" => spv.oit_accum = take_arg(&args, &mut i),
            "--spv-oit-wresolve" => spv.oit_wresolve = take_arg(&args, &mut i),
            "--evidence" => evidence_path = take_arg(&args, &mut i),
            "--expect-digest" => expect_digest = Some(take_arg(&args, &mut i)),
            "--particles" => {
                particles_on = match take_arg(&args, &mut i).as_str() {
                    "on" => true,
                    "off" => false,
                    other => fail(&format!("--particles 档 {other} 越闭集(off|on)")),
                }
            }
            "--static-camera" => static_camera = true,
            "--auto-move" => auto_move = Some(take_arg(&args, &mut i)),
            "--auto-move-amp" => {
                auto_move_amp = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--auto-move-amp 非 f64"))
            }
            "--cap" => {
                cap = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--cap 非 usize"))
            }
            "--seed" => {
                seed = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--seed 非 u64"))
            }
            "--mv-witness" => mv_witness = true,
            "--occlusion-witness" => occlusion_witness = true,
            "--oit" => {
                oit = match take_arg(&args, &mut i).as_str() {
                    "off" => G35Oit::Off,
                    "sorted" => G35Oit::Sorted,
                    "wboit" => G35Oit::Wboit,
                    other => fail(&format!("--oit 档 {other} 越闭集(off|sorted|wboit)")),
                }
            }
            "--oit-witness" => oit_witness = true,
            "--red-arm" => {
                let v = take_arg(&args, &mut i);
                if v != "key-invert" {
                    fail(&format!("--red-arm 臂 {v} 越闭集(key-invert)"));
                }
                red_arm = true;
            }
            "--mesh-particles" => {
                mesh_particles = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--mesh-particles 非 u32"))
            }
            "--headless" => {} // 本 bin 即离屏(登记面恒真);旗标闭集接受
            // G36 W4 geo 组合面参数(g14_3/g34 同名旗标同语义)。
            "--cluster-lod" => cluster_lod_mode = take_arg(&args, &mut i),
            "--cluster-pack" => cluster_pack = take_arg(&args, &mut i),
            "--cluster-error-px" => {
                cluster_error_px = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--cluster-error-px 非 f32"))
            }
            "--wp-hlod" => wp_hlod_mode = take_arg(&args, &mut i),
            "--wp-pack" => wp_pack = take_arg(&args, &mut i),
            "--wp-threshold-l0" => {
                wp_threshold_l0 = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--wp-threshold-l0 非 f64"))
            }
            // ── 展示面(网站出图)旗标:默认 = 冻结生产字面,位级零漂移 ──
            "--dump-present-raw" => dump_raw_path = take_arg(&args, &mut i),
            "--dump-present-every" => {
                dump_every = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--dump-present-every 非 u32")),
                )
            }
            "--r-world" => {
                fx.r_world = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--r-world 非 f32"))
            }
            "--splat-stretch" => {
                fx.stretch = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--splat-stretch 非 f32"))
            }
            "--particle-tint" => fx.tint = parse_f32_triplet(&take_arg(&args, &mut i), "--particle-tint"),
            "--particle-alpha-scale" => {
                fx.alpha_scale = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--particle-alpha-scale 非 f32"))
            }
            "--rain-shutter" => {
                fx.rain_shutter = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--rain-shutter 非 f32"))
            }
            "--rain-occlusion" => {
                fx.rain_occlusion = match take_arg(&args, &mut i).as_str() {
                    "on" => true,
                    "off" => false,
                    other => fail(&format!("--rain-occlusion {other} 越闭集(on|off)")),
                }
            }
            "--ev100" => {
                ev100_override = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--ev100 非 f32")),
                )
            }
            "--scene" => scene_arg = take_arg(&args, &mut i),
            "--emitter-pos" => emitter_pos = Some(parse_f32_triplet(&take_arg(&args, &mut i), "--emitter-pos")),
            "--emitter-spread" => emitter_spread = Some(parse_f32_triplet(&take_arg(&args, &mut i), "--emitter-spread")),
            "--emitter-vel" => emitter_vel = Some(parse_f32_triplet(&take_arg(&args, &mut i), "--emitter-vel")),
            "--emitter-vel-spread" => emitter_vel_spread = Some(parse_f32_triplet(&take_arg(&args, &mut i), "--emitter-vel-spread")),
            "--emitter-life" => {
                emitter_life = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--emitter-life 非 f32")),
                )
            }
            "--emitter-gravity" => {
                emitter_gravity = Some(
                    take_arg(&args, &mut i)
                        .parse()
                        .unwrap_or_else(|_| fail("--emitter-gravity 非 f32")),
                )
            }
            // 推轨短片面(day_0902_rain_night):发射器跟随相机 / 发射上限。
            "--emitter-follow-camera" => {
                emitter_follow = match take_arg(&args, &mut i).as_str() {
                    "on" => true,
                    "off" => false,
                    other => fail(&format!("--emitter-follow-camera {other} 越闭集(on|off)")),
                }
            }
            "--emit-max" => {
                emit_max = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--emit-max 非 u32"))
            }
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    // ── 闭集裁决(fail-fast 如实拒跑)──
    if frames == 0 {
        fail("--frames 必须 ≥1");
    }
    if cap == 0 || cap % SEG != 0 || cap > rurix_render::particles::PARTICLE_CAP_MAX {
        fail(&format!("--cap {cap} 须为 SEG=256 正整倍数且 ≤ 池上限"));
    }
    if static_camera && auto_move.is_some() {
        fail("--static-camera 与 --auto-move 互斥");
    }
    if let Some(name) = auto_move.as_deref() {
        if !matches!(name, "orbit" | "dolly" | "dolly-forward") {
            fail(&format!("--auto-move 轨迹 {name} 越闭集(orbit|dolly|dolly-forward)"));
        }
    }
    // 推轨短片面:位移倍率域 + 须随 --auto-move(--auto-move 本身已与见证夹具互斥)。
    if !(auto_move_amp.is_finite() && auto_move_amp > 0.0 && auto_move_amp <= 64.0) {
        fail("--auto-move-amp 须 ∈ (0, 64] 有限值(位移倍率;1.0 = 冻结轨迹幅度)");
    }
    if auto_move_amp != 1.0 && auto_move.is_none() {
        fail("--auto-move-amp 非默认须随 --auto-move");
    }
    if mv_witness && occlusion_witness {
        fail("--mv-witness 与 --occlusion-witness 互斥(单腿单构型)");
    }
    if (mv_witness || occlusion_witness) && !particles_on {
        fail("见证腿须随 --particles on");
    }
    if (mv_witness || occlusion_witness) && auto_move.is_some() {
        fail("见证腿 = 静态契约相机构型,与 --auto-move 互斥");
    }
    if mesh_particles > 0 {
        if !(1..=8).contains(&mesh_particles) {
            fail("--mesh-particles N 须 ∈ [1,8]");
        }
        if particles_on || mv_witness || occlusion_witness {
            fail("--mesh-particles 须随 --particles off 且无见证旗标(隔离见证臂)");
        }
    }
    // ── G35-4 OIT 闭集裁决 ──
    if oit != G35Oit::Off && !particles_on {
        fail("--oit sorted|wboit 须随 --particles on(OIT 臂在粒子 pass 组之上)");
    }
    if oit != G35Oit::Off && (mv_witness || occlusion_witness || mesh_particles > 0) {
        fail("--oit ≠ off 与 mv/遮挡/mesh 见证互斥(单腿单构型)");
    }
    if oit_witness {
        if oit == G35Oit::Off {
            fail("--oit-witness 须随 --oit sorted|wboit(近远见证 = OIT 臂判据)");
        }
        if auto_move.is_some() {
            fail("--oit-witness = 静态契约相机构型,与 --auto-move 互斥");
        }
        if warmup + frames <= G35L_OIT_WITNESS_F2 + 8 {
            fail(&format!(
                "--oit-witness 帧窗须 > {}(第二粒子发射帧 + 判读余量)",
                G35L_OIT_WITNESS_F2 + 8
            ));
        }
    }
    if red_arm && oit != G35Oit::Sorted {
        fail("--red-arm key-invert 须随 --oit sorted(排序臂键反转篡改)");
    }
    if oit == G35Oit::Wboit && cap > poit::OIT_WBOIT_CAP_MAX {
        fail(&format!(
            "--oit wboit 须 cap ≤ {}(定点累加结构性防回绕域:cap·65535 < u32::MAX)",
            poit::OIT_WBOIT_CAP_MAX
        ));
    }
    // ── 展示面(网站出图)闭集裁决:全部默认 = 冻结生产字面;发射器覆写与
    //    见证夹具互斥(见证 = 标定构型,覆写会污染判读域,如实拒跑不冒充)──
    if !(fx.stretch.is_finite() && (1.0..=32.0).contains(&fx.stretch)) {
        fail("--splat-stretch 须 ∈ [1.0, 32.0](1.0 = 圆点冻结形)");
    }
    if fx.tint.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 16.0) {
        fail("--particle-tint 通道须 ∈ [0.0, 16.0](1.0 = 冻结程序化调色)");
    }
    if !(fx.alpha_scale.is_finite() && fx.alpha_scale > 0.0 && fx.alpha_scale <= 1.0) {
        fail("--particle-alpha-scale 须 ∈ (0.0, 1.0](1.0 = 冻结 alpha)");
    }
    if !(fx.r_world.is_finite() && fx.r_world >= 0.001 && fx.r_world <= 0.5) {
        fail("--r-world 须 ∈ [0.001, 0.5] 米(默认 0.02 冻结字面)");
    }
    // ── 雨丝模式闭集裁决(--rain-shutter ∈ (0, 2];0 = 冻结面)──
    if !(fx.rain_shutter.is_finite() && fx.rain_shutter >= 0.0 && fx.rain_shutter <= 2.0) {
        fail("--rain-shutter 须 ∈ (0.0, 2.0](快门占 dt 比;1.0 = 整帧曝光;缺省 0 = 冻结面)");
    }
    if fx.rain_on() && fx.stretch != 1.0 {
        fail("--rain-shutter 与 --splat-stretch 互斥(雨丝长度由速度×快门决定,拉伸比无消费面 = 静默无效冒充)");
    }
    if fx.rain_on() && oit != G35Oit::Off {
        fail("--rain-shutter 与 --oit sorted|wboit 互斥(OIT kernel 消费冻结 64 f32 镜像,无雨丝模式分支;如实拒跑不冒充)");
    }
    if !fx.rain_occlusion && !fx.rain_on() {
        fail("--rain-occlusion off 须随 --rain-shutter > 0(冻结面无遮挡射线消费面)");
    }
    if scene_arg != "bistro-interior" {
        if scene_arg.is_empty()
            || !scene_arg
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            fail("--scene 须为 [a-z0-9-]+ 场景 id(契约 scenes[].scene_id 字面)");
        }
        if !particles_on || mv_witness || occlusion_witness || oit_witness || mesh_particles > 0 {
            fail("--scene ≠ bistro-interior 为展示面换景,须随 --particles on 且与见证夹具互斥(锚格/见证 = bistro-interior 冻结构型)");
        }
        if expect_digest.is_none() {
            fail("--scene ≠ bistro-interior 须同携 --contract <含该场景行的契约> + --expect-digest(FROZEN 契约无该场景行,缺省门必红)");
        }
    }
    if let Some(ev) = ev100_override {
        if !(ev.is_finite() && (-16.0..=16.0).contains(&ev)) {
            fail("--ev100 须 ∈ [-16, 16] 有限值(展示面曝光覆写;缺省 = 契约 ev100)");
        }
        if !particles_on {
            fail("--ev100 须随 --particles on(off 面 = Stage A 锚格语义,曝光覆写即冒充)");
        }
        if mv_witness || occlusion_witness || oit_witness || mesh_particles > 0 {
            fail("--ev100 与见证夹具互斥(见证 = 冻结标定构型)");
        }
    }
    if let Some(l) = emitter_life {
        if !(l.is_finite() && l > 0.0) {
            fail("--emitter-life 须为正有限 f32");
        }
    }
    if let Some(g) = emitter_gravity {
        if !g.is_finite() {
            fail("--emitter-gravity 须为有限 f32");
        }
    }
    let emitter_touched = emitter_pos.is_some()
        || emitter_spread.is_some()
        || emitter_vel.is_some()
        || emitter_vel_spread.is_some()
        || emitter_life.is_some()
        || emitter_gravity.is_some();
    if emitter_touched && (mv_witness || occlusion_witness || oit_witness || mesh_particles > 0) {
        fail("--emitter-* 覆写与见证夹具互斥(见证 = 标定构型,覆写污染判读域)");
    }
    if emitter_touched && !particles_on {
        fail("--emitter-* 覆写须随 --particles on");
    }
    let fx_touched = fx.stretch != 1.0
        || fx.tint != [1.0, 1.0, 1.0]
        || fx.alpha_scale != 1.0
        || fx.r_world != G35L_R_WORLD
        || fx.rain_on();
    if fx_touched && !particles_on {
        fail("--r-world/--splat-stretch/--particle-tint/--particle-alpha-scale/--rain-shutter 须随 --particles on(粒子 pass 组消费面,off 面携带 = 静默无效冒充)");
    }
    if fx_touched && (mv_witness || occlusion_witness || oit_witness || mesh_particles > 0) {
        fail("展示面效果参数与见证夹具互斥(见证 = 冻结标定构型,形/色变化污染判读域)");
    }
    if !dump_raw_path.is_empty() && !particles_on {
        fail("--dump-present-raw 须随 --particles on(展示面出图 = 粒子车道产物)");
    }
    // ── 推轨短片面闭集裁决(day_0902_rain_night;四旗标全缺省 = 0-byte)──
    if let Some(n) = dump_every {
        if n == 0 {
            fail("--dump-present-every 须 ≥ 1(每 n 帧落一帧 presented BGRA8)");
        }
        if dump_raw_path.is_empty() {
            fail("--dump-present-every 须随 --dump-present-raw(逐帧文件 = 该基路径派生 .f<帧号>,fail-closed)");
        }
    }
    if emitter_follow {
        if !particles_on {
            fail("--emitter-follow-camera on 须随 --particles on(发射器 = 粒子车道消费面)");
        }
        if auto_move.is_none() {
            fail("--emitter-follow-camera on 须随 --auto-move(静态相机位移恒零 = 静默无效冒充)");
        }
        if mv_witness || occlusion_witness || oit_witness || mesh_particles > 0 {
            fail("--emitter-follow-camera on 与见证夹具互斥(见证 = 冻结标定发射构型)");
        }
    }
    if emit_max != G35L_EMIT_MAX {
        if !(G35L_EMIT_MAX..=4096).contains(&emit_max) {
            fail("--emit-max 须 ∈ [256, 4096](256 = 冻结节奏 64 + f·17 % 192 ≤ 255)");
        }
        if !particles_on {
            fail("--emit-max 须随 --particles on");
        }
        if mv_witness || occlusion_witness || oit_witness || mesh_particles > 0 {
            fail("--emit-max 与见证夹具互斥(见证 = 单/双粒子标定发射)");
        }
    }
    // ── G36 W4 geo 组合面闭集裁决(fail-closed)：模式闭集 + 包必填 + 参数域
    //    (g14_3 同律字面)。互斥解除范围：geo × --particles on|off × --oit
    //    sorted|wboit(粒子为生成几何,splat/OIT 在场景色之上,与场景重组
    //    正交)。geo × 见证臂(mv/遮挡/mesh/oit-witness/red-arm)维持互斥——
    //    见证 = 标定夹具构型(语义互斥,几何重组会改变夹具遮挡/像素判读域,
    //    如实拒跑不冒充)。──
    let cluster_opt = match cluster_lod_mode.as_str() {
        "off" => ClusterLodOpt::off(),
        m @ ("leaf" | "on") => {
            if cluster_pack.is_empty() {
                fail("--cluster-lod leaf|on 要求 --cluster-pack <RXCP>（g31_cluster_lod_bake 产物）");
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
                resident_pages: 0,
            }
        }
        other => fail(&format!("--cluster-lod {other}：只接受 off|leaf|on")),
    };
    let wp_opt = match wp_hlod_mode.as_str() {
        "off" => WpHlodOpt::off(),
        m @ ("full" | "on") => {
            if wp_pack.is_empty() {
                fail("--wp-hlod full|on 要求 --wp-pack <RXWH>（g31_wp_hlod_bake 产物）");
            }
            if !(wp_threshold_l0.is_finite() && wp_threshold_l0 > 0.0) {
                fail("--wp-threshold-l0 必须为正有限 f64");
            }
            WpHlodOpt {
                mode: if m == "full" {
                    WpHlodMode::Full
                } else {
                    WpHlodMode::On
                },
                pack_path: wp_pack.clone(),
                threshold_l0: wp_threshold_l0,
                loading_radius_m: 64.0,
                inner_radius_m: 16.0,
                budget_cells: 4,
                warmup_frames: 4,
            }
        }
        other => fail(&format!("--wp-hlod {other}：只接受 off|full|on")),
    };
    let geo_on = cluster_opt.mode != ClusterLodMode::Off || wp_opt.mode != WpHlodMode::Off;
    if geo_on && (mv_witness || occlusion_witness || mesh_particles > 0 || oit_witness || red_arm)
    {
        fail("--cluster-lod/--wp-hlod 与见证/RED 臂互斥（见证 = 标定夹具构型;几何重组改变夹具判读域,如实拒跑不冒充）");
    }
    let mode = if mesh_particles > 0 {
        G35Mode::MeshWitness
    } else if mv_witness {
        G35Mode::MvWitness
    } else if occlusion_witness {
        G35Mode::OcclusionWitness
    } else if oit_witness {
        G35Mode::OitWitness
    } else if particles_on {
        G35Mode::OnFace
    } else {
        G35Mode::OffFace
    };

    // ① 生产契约(digest 门 == FROZEN;不等拒跑)。缺省 scene = bistro-interior
    //    (锚格语义);展示面 --scene <id> 换场景须同携自定义 --contract(含该
    //    scene_id 行)+ --expect-digest + --gltf(共享体 default_gltf 只认闭集
    //    两场景,未知 id 即 fail——本 bin 不触共享体,未知场景要求显式 glTF)。
    let scene_id = scene_arg.as_str();
    let (pre, frames) = prelude(
        scene_id,
        tier,
        frames,
        false,
        &contract_path,
        expect_digest.as_deref(),
    );
    let contract = &pre.contract;
    let (out_w, out_h, in_w, in_h, cseed) = (pre.out_w, pre.out_h, pre.in_w, pre.in_h, pre.seed);
    // ② G10 语料 provenance 登记(sha 快照;字段级核验门归 g34.wave1,本门
    //    facts 闭集无 g10 项——登记不裁决;文件名 = scene_id 连字符→下划线,
    //    缺失如实登记 MISSING)。
    let scene_file_stem = scene_id.replace('-', "_");
    let g10_json = format!(
        "{{\"contract\":{},\"camera\":{},\"lighting\":{}}}",
        jstr(&g35l_file_sha(&format!(
            "{g10_dir}/contract_params_{scene_file_stem}.json"
        ))),
        jstr(&g35l_file_sha(&format!("{g10_dir}/camera_{scene_file_stem}.json"))),
        jstr(&g35l_file_sha(&format!(
            "{g10_dir}/lighting_{scene_file_stem}.json"
        ))),
    );
    // ③ 场景装配。
    if gltf_path.is_empty() {
        if !matches!(scene_id, "bistro-interior" | "cornell-box") {
            fail(&format!(
                "--scene {scene_id} 非共享体 default_gltf 闭集(bistro-interior|cornell-box),须显式 --gltf <scene.gltf>"
            ));
        }
        gltf_path = default_gltf(scene_id).to_owned();
    }
    let scene = match assemble_scene(&contract.raw, scene_id, Path::new(&gltf_path)) {
        Ok(s) => s,
        Err(e) => dev_env_or_fail("scene_assets", &e),
    };
    // ③.4 G36 W4：geo 组合施加（--cluster-lod/--wp-hlod × 粒子/OIT;off 默认 =
    //     既有面 0-byte——粒子为生成几何,splat/OIT 在场景色之上与场景重组
    //     正交;cut/选层冻结于装配期契约相机,g31 车道同纪律）。
    let (scene, geo) = apply_geo_combined(scene, &cluster_opt, &wp_opt, in_w, in_h);
    let geo_json = if let Some(g) = &geo {
        if let Some((r, _)) = &g.cluster {
            eprintln!(
                "{G35L_TAG}: cluster-lod mode={} clusters={}/{} tris out={}/{} ({:.1}%)",
                r.mode,
                r.cut_clusters,
                r.total_clusters,
                r.out_tris,
                r.src_tris,
                100.0 * r.out_tris as f64 / r.src_tris.max(1) as f64,
            );
        }
        if let Some((r, _)) = &g.wp {
            eprintln!(
                "{G35L_TAG}: wp-hlod mode={} cells full/hlod/culled={}/{}/{} proxy_tris={}",
                r.mode, r.cells_full, r.cells_hlod, r.cells_culled, r.proxy_tris,
            );
        }
        if let Some(st) = &g.combined {
            eprintln!(
                "{G35L_TAG}: geo 组合 identity={} coarse={} straddle_fallback={} wp_proxy={} out={}",
                st.identity_tris,
                st.coarse_tris,
                st.straddle_fallback_tris,
                st.wp_proxy_tris,
                st.out_tris,
            );
        }
        let cl = g
            .cluster
            .as_ref()
            .map(|(r, _)| format!("{{\"mode\":{},\"out_tris\":{}}}", jstr(r.mode), r.out_tris))
            .unwrap_or_else(|| "null".to_owned());
        let wp = g
            .wp
            .as_ref()
            .map(|(r, _)| {
                format!(
                    "{{\"mode\":{},\"cells_full\":{},\"cells_hlod\":{},\"proxy_tris\":{},\"selection_digest\":{}}}",
                    jstr(r.mode),
                    r.cells_full,
                    r.cells_hlod,
                    r.proxy_tris,
                    jstr(&r.selection_digest),
                )
            })
            .unwrap_or_else(|| "null".to_owned());
        format!(
            "{{\"cluster\":{cl},\"wp\":{wp},\"out_tris\":{},\"frozen_at_assembly\":true}}",
            scene.indices.len()
        )
    } else {
        "null".to_owned()
    };
    let eps = scene_eps(&scene.positions);
    // 展示面 --ev100 覆写(缺省 None = 契约 ev100 逐字,exposure 位级同值)。
    let ev100 = f64::from(ev100_override.unwrap_or(scene.ev100));
    let exposure = 2.0f32.powf(-(ev100 as f32));
    if let Some(ev) = ev100_override {
        eprintln!(
            "{G35L_TAG}: 展示面 --ev100 {ev} 覆写契约 ev100 {}(exposure={exposure:e};仅 on 面出图,digest 非锚格语义)",
            scene.ev100
        );
    }
    let jitter_base = (cseed % JITTER_WINDOW_MOD) as u32;
    let p11 = 1.0f32 / (scene.camera.fov_y_rad * 0.5).tan();
    let d_max = scene.camera.far;
    let total = warmup + frames;
    eprintln!(
        "{G35L_TAG}: 装配 scene={scene_id} tris={} output={out_w}x{out_h} internal={in_w}x{in_h} mode={} particles={} cap={cap} seed={seed}",
        scene.tri_count,
        match mode {
            G35Mode::OffFace => "off",
            G35Mode::OnFace => "on",
            G35Mode::MvWitness => "mv_witness",
            G35Mode::OcclusionWitness => "occlusion_witness",
            G35Mode::MeshWitness => "mesh_witness",
            G35Mode::OitWitness => "oit_witness",
        },
        particles_on,
    );

    // ── 帧循环公共小件:相机位姿 → (vp, inv_vp, vp_j, jitter) ──
    let cam0 = G35Camera::from_spec(&scene.camera);
    let pose = |fi: u32| -> CameraSpec {
        if let Some(name) = auto_move.as_deref() {
            let (yaw, pitch, eye) = g35l_auto_move_pose(name, &cam0, fi, total, auto_move_amp);
            let mut c = cam0;
            c.yaw = yaw;
            c.pitch = pitch;
            c.eye = eye;
            c.spec()
        } else {
            scene.camera
        }
    };
    let jit = |fi: u32| -> [f32; 2] {
        [
            halton(jitter_base + fi + 1, 2) - 0.5,
            halton(jitter_base + fi + 1, 3) - 0.5,
        ]
    };

    // ── off 面(母版;共享体 UnifiedTsrLane 原码——锚格/轨迹两模)与
    //    off 对拍面(MegaDyn 影,scene color 回读)执行器 ──
    #[allow(clippy::type_complexity)]
    let run_off = |want_scene: bool| -> (String, Option<Vec<f32>>, Option<Vec<f32>>, Vec<String>) {
        // (render_digest, last scene_color, last out_color, digest_seq 占位空)
        let assets = lane_assets(&scene, in_w, in_h);
        let bits = UnifiedLaneBits::load(
            &spv.scene,
            &spv.mv,
            &spv.resample,
            &spv.resolve,
            in_w,
            in_h,
            out_w,
            out_h,
            false,
        );
        let blas_refs: [&[f32]; 1] = [&assets.tris];
        let accel = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets.instances,
            },
            transforms: None,
            updatable_blas: &[],
        }];
        let descs_mega;
        let descs_dyn;
        let descs = if want_scene {
            descs_dyn = UnifiedDescs::MegaDyn(unified_lane_descs_dyn(
                &assets, &bits, in_w, in_h, out_w, out_h,
            ));
            &descs_dyn
        } else {
            descs_mega =
                UnifiedDescs::Mega(unified_lane_descs(&assets, &bits, in_w, in_h, out_w, out_h));
            &descs_mega
        };
        let mut lane = match UnifiedTsrLane::create(descs, &accel, 1) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        let mut render_digest = String::new();
        let mut last_scene: Option<Vec<f32>> = None;
        let mut last_out: Option<Vec<f32>> = None;
        for fi in 0..total {
            let spec = pose(fi);
            let vp = build_vp(&spec, in_w, in_h);
            let inv_vp = vp.inverse().unwrap_or_else(|| fail("view-proj 必须可逆"));
            let j = jit(fi);
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let last = fi + 1 == total;
            let reset = fi == 0;
            let rec = if want_scene {
                // MegaDyn 影面:prepare_update_ext 直调(tlas_update=None;
                // scene color 回读 = 遮挡见证对拍源;共享体原码消费)。
                let scene_params = pack_frame_params(
                    in_w,
                    in_h,
                    j,
                    eps,
                    scene.quads.len(),
                    scene.points.len(),
                    &inv_vp,
                    &vp,
                );
                let (prov, update) = lane
                    .prepare_update_ext(
                        in_w, in_h, out_w, out_h, j, &vp_j, exposure, reset, last, last,
                        // G38 L2a 末参 = scene_as_override：None = 零 scene AS
                        // 换槽 override，产物与加参前逐字段同（机械适配 0-byte）。
                        scene_params, None, None,
                    )
                    .unwrap_or_else(|e| fail(&format!("off 影面帧 {fi}: {e}")));
                let out = lane
                    .session
                    .execute_with_frame_update(&prov, &update)
                    .unwrap_or_else(|e| fail(&format!("off 影面帧 {fi}: {e}")));
                let rec = lane
                    .rec_from_output(out, last, last, out_w, out_h, in_w, in_h)
                    .unwrap_or_else(|e| fail(&format!("off 影面帧 {fi}: {e}")));
                lane.advance(&vp_j);
                rec
            } else {
                lane.frame(
                    in_w,
                    in_h,
                    out_w,
                    out_h,
                    j,
                    eps,
                    scene.quads.len(),
                    scene.points.len(),
                    &inv_vp,
                    &vp,
                    &vp_j,
                    exposure,
                    reset,
                    last,
                )
                .unwrap_or_else(|e| fail(&format!("off 面帧 {fi}: {e}")))
            };
            if rec.validation_error_count != 0 {
                fail(&format!("off 面帧 {fi} validation ERROR ≠ 0"));
            }
            if last {
                let out_data = rec
                    .out_color
                    .as_ref()
                    .unwrap_or_else(|| fail("off 面末帧缺 f32 out_color 回读"));
                if !out_data.iter().all(|v| v.is_finite()) {
                    fail("off 面末帧 TSR 输出非有限");
                }
                render_digest = frame_content_digest(out_w, out_h, 3, out_data);
                last_out = rec.out_color.clone();
                last_scene = rec.scene_color.clone();
            }
        }
        (render_digest, last_scene, last_out, Vec::new())
    };

    // ── mesh 粒子 TLAS 见证臂(--mesh-particles N;A4 通路 bin 局部消费,
    //    wired=1 实例;on≠off render_digest 判别进程内双车道)──
    if mode == G35Mode::MeshWitness {
        let spv_dyn = DEFAULT_SPV_DYN_SCENE.to_owned();
        if !Path::new(&spv_dyn).is_file() {
            dev_env_or_fail("mesh_dyn_scene_spv", &format!("SPV 缺失: {spv_dyn}"));
        }
        // host 金标准单粒子轨迹(恒速夹具;镜像 = 立方体位姿唯一事实源)。
        let mut mirror = G35HostMirror::new(
            cap,
            seed,
            g35l_witness_emitter(&scene.camera, false),
            G35EmitSched::SingleF0,
        );
        let assets_dyn = lane_assets_dyn(&scene, in_w, in_h);
        let bits = UnifiedLaneBits::load(
            &spv_dyn,
            &spv.mv,
            &spv.resample,
            &spv.resolve,
            in_w,
            in_h,
            out_w,
            out_h,
            false,
        );
        let scene_tri_end = assets_dyn.dyn_tri_base * 9;
        let blas_refs: [&[f32]; 2] = [
            &assets_dyn.base.tris[..scene_tri_end],
            &assets_dyn.dyn_tris,
        ];
        let accel = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets_dyn.base.instances,
            },
            transforms: None,
            updatable_blas: &[],
        }];
        let descs = UnifiedDescs::MegaDyn(unified_lane_descs_dyn(
            &assets_dyn.base,
            &bits,
            in_w,
            in_h,
            out_w,
            out_h,
        ));
        let mut lane = match UnifiedTsrLane::create(&descs, &accel, 1) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        let mut digest_on = String::new();
        for fi in 0..total {
            let spec = pose(fi);
            let vp = build_vp(&spec, in_w, in_h);
            let inv_vp = vp.inverse().unwrap_or_else(|| fail("view-proj 必须可逆"));
            let j = jit(fi);
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let ctl = mirror.step(fi);
            let _ = ctl;
            let pools = mirror.current();
            if pools.n == 0 {
                fail("mesh 见证粒子消亡(夹具寿命窗破缺)");
            }
            let pos = [pools.pos_x[0], pools.pos_y[0], pools.pos_z[0]];
            let xf = dyn_transform_3x4(pos, 0.0);
            let scene_params = pack_frame_params_dyn(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                assets_dyn.dyn_tri_base,
            );
            let last = fi + 1 == total;
            let rec = lane
                .frame_dyn(
                    in_w,
                    in_h,
                    out_w,
                    out_h,
                    j,
                    &vp_j,
                    exposure,
                    fi == 0,
                    scene_params,
                    (0u32, dyn_frame_instances(xf), TlasBuildAction::Refit),
                    last,
                    false,
                )
                .unwrap_or_else(|e| fail(&format!("mesh 臂帧 {fi}: {e}")));
            if rec.validation_error_count != 0 {
                fail(&format!("mesh 臂帧 {fi} validation ERROR ≠ 0"));
            }
            if last {
                let out_data = rec
                    .out_color
                    .as_ref()
                    .unwrap_or_else(|| fail("mesh 臂末帧缺 f32 out_color 回读"));
                digest_on = frame_content_digest(out_w, out_h, 3, out_data);
            }
        }
        drop(lane);
        let (digest_off, _, _, _) = run_off(false);
        let discriminates = digest_on != digest_off && !digest_on.is_empty();
        let mesh_json = format!(
            "{{\"requested\":{mesh_particles},\"wired\":1,\"not_wired_reason\":{},\"digest_on\":{},\"digest_off\":{},\"discriminates\":{discriminates},\"spv_dyn_scene\":{{\"path\":{},\"sha256\":{}}},\"note\":{}}}",
            if mesh_particles > 1 {
                jstr("g31_dyn_scene 分派映射 pg = prim + inst·dyn_tri_base 为单动态实例语义(inst ≥ 2 时 tris 下标越界)——0-byte 纪律下 N>1 的其余实例如实 not_wired,仅实例 1 接线")
            } else {
                "null".to_owned()
            },
            jstr(&digest_on),
            jstr(&digest_off),
            jstr(&spv_dyn.replace('\\', "/")),
            jstr(&g35l_file_sha(&spv_dyn)),
            jstr("mesh 粒子立方体按 host 金标准粒子轨迹逐帧 tlas_update(Refit,inflight=1,A4 先例);动态实例入 TLAS ⇒ ray query 场景自动获得光追阴影(阴影射线可命中动态实例,登记面)"),
        );
        emit_evidence(
            &evidence_path,
            &EvidenceCtx {
                mode: "mesh_witness",
                scene_id,
                tier,
                frames,
                warmup,
                static_camera: auto_move.is_none(),
                trajectory: auto_move.as_deref(),
                particles_on: false,
                cap,
                seed,
                contract_path: &contract_path,
                contract_digest: &contract.digest,
                gltf_path: &gltf_path,
                g10_json: &g10_json,
                spv: &spv,
                render_digest: &digest_on,
                digest_seq: &[],
                mv_witness_json: "null".to_owned(),
                occlusion_json: "null".to_owned(),
                mesh_json,
                frame_ms_json: "null".to_owned(),
                particle_stats_json: "null".to_owned(),
                oit: G35Oit::Off,
                oit_json: "{\"mode\":\"off\"}".to_owned(),
                oit_witness_json: "null".to_owned(),
                geo_json: geo_json.clone(),
                showcase_json: "null".to_owned(),
            },
        );
        eprintln!("{G35L_TAG}: PASS mesh 见证臂 discriminates={discriminates}");
        return;
    }

    // ── off 面(锚格/轨迹)──
    if mode == G35Mode::OffFace {
        let t0 = std::time::Instant::now();
        let (render_digest, _, _, _) = run_off(false);
        let wall_ms = t0.elapsed().as_secs_f64() * 1000.0;
        emit_evidence(
            &evidence_path,
            &EvidenceCtx {
                mode: "off",
                scene_id,
                tier,
                frames,
                warmup,
                static_camera: auto_move.is_none(),
                trajectory: auto_move.as_deref(),
                particles_on: false,
                cap,
                seed,
                contract_path: &contract_path,
                contract_digest: &contract.digest,
                gltf_path: &gltf_path,
                g10_json: &g10_json,
                spv: &spv,
                render_digest: &render_digest,
                digest_seq: &[],
                mv_witness_json: "null".to_owned(),
                occlusion_json: "null".to_owned(),
                mesh_json: "null".to_owned(),
                frame_ms_json: format!(
                    "{{\"total_wall_ms\":{wall_ms:.3},\"frames_total\":{total}}}"
                ),
                particle_stats_json: "null".to_owned(),
                oit: G35Oit::Off,
                oit_json: "{\"mode\":\"off\"}".to_owned(),
                oit_witness_json: "null".to_owned(),
                geo_json: geo_json.clone(),
                showcase_json: "null".to_owned(),
            },
        );
        eprintln!(
            "{G35L_TAG}: PASS off 面 render_digest={render_digest}(锚格对拍归 smoke;锚 = {G35L_ANCHOR_PATH} cell {G35L_ANCHOR_CELL})"
        );
        return;
    }

    // ── on 面(生产/见证)──
    // G35-4 键域守卫(oit ≠ off;oit_arms.rs 头注论证镜像:tile_cnt ≤ 4095
    // ⇒ 溢出键 < 2^24;bistro t50 内部 960×540 ⇒ 2040 ✓ / t100 8160 拒跑)。
    let (oit_tiles_x, oit_tiles_y, oit_tile_cnt) = g35l_tile_grid(in_w, in_h);
    if oit != G35Oit::Off && oit_tile_cnt > poit::OIT_TILE_CNT_MAX {
        fail(&format!(
            "--oit {} 键域越界:内部 {in_w}x{in_h} ⇒ tile_cnt {oit_tile_cnt} > {}(键 = tile_id·4096 溢出 2^24;取 --tier 50 等低内部分辨率档)",
            oit.as_str(),
            poit::OIT_TILE_CNT_MAX
        ));
    }
    let desc = match mode {
        G35Mode::MvWitness => g35l_witness_emitter(&scene.camera, false),
        G35Mode::OcclusionWitness => g35l_witness_emitter(&scene.camera, true),
        G35Mode::OitWitness => g35l_oit_witness_emitter(&scene.camera),
        _ => {
            // 生产腿:冻结夹具为底,展示面 --emitter-* 逐字段覆写(device
            // emit_params 上传与 host 金标准镜像同源消费 ⇒ 整数流一致性不破)。
            let mut d = g35l_emitter(&scene.camera);
            if let Some(p) = emitter_pos {
                d.pos = p;
            }
            if let Some(s) = emitter_spread {
                d.spread = s;
            }
            if let Some(v) = emitter_vel {
                d.vel_base = v;
            }
            if let Some(vs) = emitter_vel_spread {
                d.vel_spread = vs;
            }
            if let Some(l) = emitter_life {
                d.life_base = l;
            }
            if let Some(g) = emitter_gravity {
                d.gravity_y = g;
            }
            d
        }
    };
    // ── --emit-max 随机带克隆守卫(缺省 256 不评估 = 0-byte)──
    // 消费律 r_k = table[(pid·RAND_K + k) % RAND_TABLE_LEN],RAND_K = 7919 为素数、
    // 与 2^16 互素 ⇒ pid ↦ pid·7919 mod 65536 为双射 ⇒ 两粒子全 7 槽随机数克隆
    // ⇔ pid ≡ pid′ (mod 65536)。同屏同时存活的 pid 跨度上界 = 峰值发射数 ×
    // 存活帧数;跨度 ≥ 65536 即必然出现同屏克隆(位置/速度/寿命全同 ⇒ 视觉重影),
    // fail-closed 拒跑。峰值 = 冻结节奏上界 255 按 emit_max/256 放大(整数算术同
    // emit_schedule)。pid 走 f32 参数面(kernel params[1]),core `emit_step` 断言
    // pid_base + emit_count < 2^24——peak·total 为其上界,提前中文拒跑。
    if emit_max != G35L_EMIT_MAX {
        let peak = 255usize * emit_max as usize / G35L_EMIT_MAX as usize;
        let frames_alive = (desc.life_base / G35L_DT).ceil().max(1.0) as usize;
        let span = peak * frames_alive;
        if span >= RAND_TABLE_LEN {
            fail(&format!(
                "--emit-max {emit_max} 随机带克隆守卫红:峰值发射 {peak}/帧 × 存活 {frames_alive} 帧(life {} s / dt {G35L_DT})= 同屏 pid 跨度 {span} ≥ {RAND_TABLE_LEN}(r_k = table[(pid·{RAND_K}+k) % {RAND_TABLE_LEN}],{RAND_K} 与 2^16 互素 ⇒ 全 7 槽克隆 ⇔ pid ≡ pid′ mod {RAND_TABLE_LEN});降 --emit-max 或缩 --emitter-life",
                desc.life_base
            ));
        }
        if peak * total as usize >= (1usize << 24) {
            fail(&format!(
                "--emit-max {emit_max} pid 精确域守卫红:峰值发射 {peak}/帧 × 总帧 {total} = pid 上界 {} ≥ 2^24(pid 走 f32 参数面精确域;core emit_step 同断言);降 --emit-max 或缩 --frames/--warmup",
                peak * total as usize
            ));
        }
        if span > cap {
            eprintln!(
                "{G35L_TAG}: 提示 --emit-max {emit_max} 同屏 pid 跨度 {span} > cap {cap}:发射预算按 cap − n_curr 钳制(非红,登记面)"
            );
        }
    }
    let sched = match mode {
        G35Mode::MvWitness | G35Mode::OcclusionWitness => G35EmitSched::SingleF0,
        G35Mode::OitWitness => G35EmitSched::OitPair,
        _ => G35EmitSched::Production,
    };
    let witness_mirror = matches!(mode, G35Mode::MvWitness | G35Mode::OcclusionWitness);
    let mut mirror = G35HostMirror::new(cap, seed, desc, sched);
    // --emit-max 注入 host 金标准镜像(缺省即 new() 内 G35L_EMIT_MAX,赋同值无漂移)。
    mirror.emit_max = emit_max as usize;
    let assets = lane_assets(&scene, in_w, in_h);
    let bits = UnifiedLaneBits::load(
        &spv.scene,
        &spv.mv,
        &spv.resample,
        &spv.resolve,
        in_w,
        in_h,
        out_w,
        out_h,
        false,
    );
    let pbits = G35ParticleBits::load(&spv, seed);
    let obits = if oit != G35Oit::Off {
        Some(G35OitBits::load(&spv, in_w, in_h))
    } else {
        None
    };
    let enc_words = {
        if !Path::new(&spv.encode).is_file() {
            dev_env_or_fail("spv_assets", &format!("SPV 缺失: {}", spv.encode));
        }
        load_spv(&spv.encode)
    };
    let (ex, ey, _) = spv_local_size(&enc_words);
    let enc_dispatch = [out_w.div_ceil(ex), out_h.div_ceil(ey), 1];
    let enc_spv_bytes: Vec<u8> = enc_words.iter().flat_map(|w| w.to_le_bytes()).collect();
    // 离屏 bin:编码 bgra 序恒真(g34 headless 同默认;digest 自恰面)。
    let enc_params = aces13_device_encode_params(out_w, out_h, true);
    let enc_params_bytes = bytes_f32(&enc_params);
    let mother = unified_lane_descs(&assets, &bits, in_w, in_h, out_w, out_h);
    let descs = g35_on_descs(
        mother,
        &pbits,
        oit,
        obits.as_ref(),
        &enc_spv_bytes,
        enc_dispatch,
        &enc_params_bytes,
        in_w,
        in_h,
        out_w,
        out_h,
        cap,
        emit_max,
    );
    let blas_refs: [&[f32]; 1] = [&assets.tris];
    let accel = [AccelStructDesc {
        scene: RayQuerySceneDesc {
            blas_triangles: &blas_refs,
            instances: &assets.instances,
        },
        transforms: None,
        updatable_blas: &[],
    }];
    // 屏障计划机核审计(创建前纯 host;进 evidence;OIT 档追加该档 pass 行)。
    let (audit_ok, audit_rows) = g35l_barrier_audit(oit);
    if !audit_ok {
        // 审计失败 = 本 bin 计划声明缺陷,硬红(机核判据先于真跑)。
        for r in audit_rows.iter().filter(|r| !r.ok) {
            eprintln!("{G35L_TAG}: 屏障审计红 pass={} missing={:?}", r.name, r.missing);
        }
        fail("屏障计划机核审计红(bindings ⊄ 计划资源集)");
    }
    let mut lane = match G35OnLane::create(
        &descs, &accel, in_w, in_h, out_w, out_h, p11, d_max, oit, red_arm, fx,
    ) {
        Ok(l) => l,
        Err(e) => dev_env_or_fail("device_lane", &e),
    };
    eprintln!(
        "{G35L_TAG}: on 面车道就绪 {} pass(oit={} Δ={};15+Δ:scene→mv→粒子10→[oit]→tsr×2→encode)资源 {} 屏障审计=绿",
        15 + oit.pass_delta(),
        oit.as_str(),
        oit.pass_delta(),
        if oit == G35Oit::Off {
            G35L_RESOURCE_COUNT
        } else {
            G35L_OIT_RESOURCE_COUNT
        },
    );
    let mut render_ms: Vec<f64> = Vec::new();
    let mut particle_gpu_ms: Vec<f64> = Vec::new();
    let mut oit_gpu_ms: Vec<f64> = Vec::new();
    let mut digest_seq: Vec<String> = Vec::new();
    let mut render_digest = String::new();
    let mut presented_digest = String::new();
    let mut last_ctl: Option<G35FrameCtl> = None;
    let mut last_rec: Option<G35FrameRec> = None;
    let mut last_vp_j: Option<Mat4> = None;
    let mut prev_vp_j_host: Option<Mat4> = None;
    let mut last_prev_vp_j: Option<Mat4> = None;
    // 帧 0 相机位置(--emitter-follow-camera 位移基准;纯 host 位姿求值,无副作用)。
    let eye0 = pose(0).eye;
    for fi in 0..total {
        let spec = pose(fi);
        let vp = build_vp(&spec, in_w, in_h);
        let inv_vp = vp.inverse().unwrap_or_else(|| fail("view-proj 必须可逆"));
        let j = jit(fi);
        let vp_j = jittered_vp(&vp, j, in_w, in_h);
        // --emitter-follow-camera:发射中心 = 基准盒 desc.pos + (eye(fi) − eye(0))。
        // 先改 mirror.desc 再 step ⇒ host 金标准 pcore::frame(&self.desc) 与 device
        // lane.frame(&mirror.desc) 消费同一份 pos(emit_params[2..5) 逐帧上传),
        // 随机带消费律/整数流不变;缺省 off 不执行(mirror.desc 保持构造值)。
        if emitter_follow {
            mirror.desc.pos = [
                desc.pos[0] + (spec.eye[0] - eye0[0]),
                desc.pos[1] + (spec.eye[1] - eye0[1]),
                desc.pos[2] + (spec.eye[2] - eye0[2]),
            ];
        }
        let ctl = mirror.step(fi);
        let scene_params = pack_frame_params(
            in_w,
            in_h,
            j,
            eps,
            scene.quads.len(),
            scene.points.len(),
            &inv_vp,
            &vp,
        );
        let last = fi + 1 == total;
        // --dump-present-every 命中帧(帧号含 warmup);缺省 None ⇒ 恒 false,
        // rb.bgra 表达式 `a || last || false` 短路回旧值。
        let dump_hit = dump_every.is_some_and(|n| fi % n == 0);
        let debug_f32 = std::env::var("G35L_DEBUG_OUT_F32").map(|p| !p.is_empty()).unwrap_or(false);
        let rb = G35Rb {
            out: last,
            mv: last && mode == G35Mode::MvWitness,
            scene: last
                && (mode == G35Mode::OcclusionWitness || mode == G35Mode::OitWitness || debug_f32),
            winner: last && witness_mirror,
            bgra: auto_move.is_some() || last || dump_hit,
            acc: last && mode == G35Mode::OitWitness && oit == G35Oit::Wboit,
            sat: last && oit == G35Oit::Wboit,
            depth: last && mode == G35Mode::OitWitness,
        };
        let t_render = std::time::Instant::now();
        let rec = lane
            .frame(
                j,
                &vp,
                &vp_j,
                spec.eye,
                exposure,
                fi == 0,
                &ctl,
                &mirror.desc,
                scene_params,
                rb,
            )
            .unwrap_or_else(|e| fail(&format!("on 面帧 {fi}: {e}")));
        let render_el = t_render.elapsed().as_secs_f64() * 1000.0;
        if rec.validation_error_count != 0 {
            fail(&format!(
                "帧 {fi} validation ERROR 计数 {} ≠ 0",
                rec.validation_error_count
            ));
        }
        if rec.leaked_object_count != 0 || rec.leaked_allocation_count != 0 {
            fail(&format!("帧 {fi} leak 账本非零(资源无泄漏机核判红)"));
        }
        if fi >= warmup {
            render_ms.push(render_el);
            particle_gpu_ms.push(rec.particle_gpu_ns / 1e6);
            oit_gpu_ms.push(rec.oit_gpu_ns / 1e6);
        }
        if auto_move.is_some() {
            let px = rec
                .bgra8
                .as_ref()
                .unwrap_or_else(|| fail(&format!("帧 {fi} auto-move 面缺 BGRA8 回读")));
            digest_seq.push(g35l_bgra_digest(out_w, out_h, px));
        }
        // --dump-present-every 命中帧落盘:`<base>.f<帧号 4 位>`,w/h u32 LE 头 +
        // BGRA8(g31_window_present 逐帧写盘段同布局);末帧 `<base>` 仍由下方
        // last 分支照旧写(命中且末帧 ⇒ 两文件同内容,如实)。
        if dump_hit {
            let px = rec
                .bgra8
                .as_ref()
                .unwrap_or_else(|| fail(&format!("帧 {fi} dump-present-every 命中帧缺 BGRA8 回读")));
            let mut buf = Vec::with_capacity(8 + px.len());
            buf.extend_from_slice(&out_w.to_le_bytes());
            buf.extend_from_slice(&out_h.to_le_bytes());
            buf.extend_from_slice(px);
            let p = format!("{dump_raw_path}.f{fi:04}");
            std::fs::write(&p, &buf)
                .unwrap_or_else(|e| fail(&format!("--dump-present-every 写 {p}: {e}")));
        }
        if last {
            let out_data = rec
                .out_color
                .as_ref()
                .unwrap_or_else(|| fail("末帧缺 f32 out_color 回读"));
            if !out_data.iter().all(|v| v.is_finite()) {
                fail("末帧 TSR 输出非有限");
            }
            render_digest = frame_content_digest(out_w, out_h, 3, out_data);
            // 调试面(环境变量门控,缺省不触):末帧 TSR f32 输出原样落盘
            // (w/h u32 LE 头 + f32×3/px),供离线核对显示编码前的数值域。
            if let Ok(p) = std::env::var("G35L_DEBUG_OUT_F32") {
                if !p.is_empty() {
                    let mut buf = Vec::with_capacity(8 + out_data.len() * 4);
                    buf.extend_from_slice(&out_w.to_le_bytes());
                    buf.extend_from_slice(&out_h.to_le_bytes());
                    buf.extend_from_slice(&bytes_f32(out_data));
                    std::fs::write(&p, &buf)
                        .unwrap_or_else(|e| fail(&format!("G35L_DEBUG_OUT_F32 写 {p}: {e}")));
                    eprintln!("{G35L_TAG}: debug out_color f32 → {p}");
                    if let Some(sc) = rec.scene_color.as_ref() {
                        let sp = format!("{p}.scene");
                        let mut sb = Vec::with_capacity(8 + sc.len() * 4);
                        sb.extend_from_slice(&in_w.to_le_bytes());
                        sb.extend_from_slice(&in_h.to_le_bytes());
                        sb.extend_from_slice(&bytes_f32(sc));
                        std::fs::write(&sp, &sb)
                            .unwrap_or_else(|e| fail(&format!("G35L_DEBUG_OUT_F32 写 {sp}: {e}")));
                        eprintln!("{G35L_TAG}: debug scene_color f32 → {sp}");
                    }
                }
            }
            let px = rec
                .bgra8
                .as_ref()
                .unwrap_or_else(|| fail("末帧缺 BGRA8 回读"));
            presented_digest = g35l_bgra_digest(out_w, out_h, px);
            // 展示面出图:w/h u32 LE 头 + BGRA8 打包(g31_window_present
            // 写盘段同布局,raw2png.py 直通;回读面 = 既有末帧 presented
            // 回读,零追加 GPU 侧读回)。
            if !dump_raw_path.is_empty() {
                let mut buf = Vec::with_capacity(8 + px.len());
                buf.extend_from_slice(&out_w.to_le_bytes());
                buf.extend_from_slice(&out_h.to_le_bytes());
                buf.extend_from_slice(px);
                std::fs::write(&dump_raw_path, &buf)
                    .unwrap_or_else(|e| fail(&format!("dump-present-raw 写 {dump_raw_path}: {e}")));
                eprintln!(
                    "{G35L_TAG}: dump-present-raw → {dump_raw_path} ({out_w}x{out_h} BGRA8)"
                );
            }
            last_vp_j = Some(vp_j);
            last_prev_vp_j = prev_vp_j_host;
            last_ctl = Some(ctl);
            last_rec = Some(rec);
        } else if fi % 20 == 0 {
            eprintln!(
                "{G35L_TAG}: 帧 {}/{total} render={render_el:.3}ms particle_gpu={:.3}ms n={}",
                fi + 1,
                rec.particle_gpu_ns / 1e6,
                ctl.n_next,
            );
        }
        prev_vp_j_host = Some(vp_j);
    }
    drop(lane);
    let last_rec = last_rec.unwrap_or_else(|| fail("末帧记录缺失(内部破缺)"));
    let last_ctl = last_ctl.unwrap_or_else(|| fail("末帧控制块缺失(内部破缺)"));

    // ── mv 见证腿判读(命中像素 = winner ≠ 0;device MV vs 解析期望)──
    let mv_witness_json = if mode == G35Mode::MvWitness {
        let winner = last_rec
            .winner
            .as_ref()
            .unwrap_or_else(|| fail("mv 见证缺 winner 回读"));
        let mv = last_rec
            .mv_out
            .as_ref()
            .unwrap_or_else(|| fail("mv 见证缺 mv_out 回读"));
        let pools = mirror.current();
        if pools.n == 0 {
            fail("mv 见证粒子消亡(夹具寿命窗破缺)");
        }
        let pos = [pools.pos_x[0], pools.pos_y[0], pools.pos_z[0]];
        let vel = [pools.vel_x[0], pools.vel_y[0], pools.vel_z[0]];
        let vp_j = last_vp_j.unwrap();
        let prev = last_prev_vp_j.unwrap_or(vp_j);
        let expect = g35l_particle_mv_expect(pos, vel, &vp_j, &prev);
        let mut hit_px = 0u64;
        let mut slot_ok = true;
        let mut max_err_px = 0.0f64;
        let mut sample_dev = [0.0f32; 2];
        for (i, &w) in winner.iter().enumerate() {
            if w != 0 {
                hit_px += 1;
                if (w & 0xFF_FFFF) != 0 {
                    slot_ok = false; // 单粒子构型 slot 恒 0
                }
                let dmx = f64::from((mv[i * 2] - expect[0]).abs()) * f64::from(in_w);
                let dmy = f64::from((mv[i * 2 + 1] - expect[1]).abs()) * f64::from(in_h);
                let e = dmx.max(dmy);
                if e >= max_err_px {
                    max_err_px = e;
                    sample_dev = [mv[i * 2], mv[i * 2 + 1]];
                }
            }
        }
        format!(
            "{{\"hit_px\":{hit_px},\"slot_zero\":{slot_ok},\"max_err_px\":{:.9},\"mv_dev_sample\":[{:.9},{:.9}],\"mv_expect\":[{:.9},{:.9}],\"frame\":{}}}",
            if hit_px > 0 { max_err_px } else { f64::NAN },
            sample_dev[0],
            sample_dev[1],
            expect[0],
            expect[1],
            total - 1,
        )
    } else {
        "null".to_owned()
    };

    // ── 遮挡见证腿判读(winner 全零 + scene color/render_digest 与 off 面
    //    位级等;off 影面进程内复跑同帧窗)──
    let occlusion_json = if mode == G35Mode::OcclusionWitness {
        let winner = last_rec
            .winner
            .as_ref()
            .unwrap_or_else(|| fail("遮挡见证缺 winner 回读"));
        let nonzero = winner.iter().filter(|&&w| w != 0).count();
        let on_scene = last_rec
            .scene_color
            .as_ref()
            .unwrap_or_else(|| fail("遮挡见证缺 scene color 回读"));
        let (off_digest, off_scene, _, _) = run_off(true);
        let off_scene = off_scene.unwrap_or_else(|| fail("off 影面缺 scene color 回读"));
        let scene_bitexact = on_scene.len() == off_scene.len()
            && on_scene
                .iter()
                .zip(off_scene.iter())
                .all(|(a, b)| a.to_bits() == b.to_bits());
        let digest_match = render_digest == off_digest;
        format!(
            "{{\"winner_nonzero_px\":{nonzero},\"scene_color_bitexact_with_off\":{scene_bitexact},\"render_digest_match_off\":{digest_match},\"on_digest\":{},\"off_digest\":{},\"config\":{}}}",
            jstr(&render_digest),
            jstr(&off_digest),
            jstr("单粒子置相机后方 2.0m(bistro 室内相机身后已知墙后;splat 投影 w 门 + 同域深度拒绝双路径)——深度域 quirk 登记见 depth_domain 字段"),
        )
    } else {
        "null".to_owned()
    };

    // ── G35-4 近远见证腿判读(--oit-witness:off 影面进程内复跑同帧窗拿
    //    OIT 输入基底〔前 12 pass 位级同〕⇒ host 金标准 oit_arms 全帧期望
    //    ⇒ p100 对拍;红臂 = 期望恒按正协议算 ⇒ p100 必大 = 翻序检出)──
    let oit_witness_json = if mode == G35Mode::OitWitness {
        let dev_scene = last_rec
            .scene_color
            .as_ref()
            .unwrap_or_else(|| fail("oit 见证缺 scene color 回读"));
        let dev_depth = last_rec
            .scene_depth
            .as_ref()
            .unwrap_or_else(|| fail("oit 见证缺 scene depth 回读"));
        let mother2 = unified_lane_descs(&assets, &bits, in_w, in_h, out_w, out_h);
        let descs_off = g35_on_descs(
            mother2,
            &pbits,
            G35Oit::Off,
            None,
            &enc_spv_bytes,
            enc_dispatch,
            &enc_params_bytes,
            in_w,
            in_h,
            out_w,
            out_h,
            cap,
            // 见证影面 = 冻结标定发射(--emit-max 已与见证互斥),字面上界。
            G35L_EMIT_MAX,
        );
        let mut mirror2 = G35HostMirror::new(
            cap,
            seed,
            g35l_oit_witness_emitter(&scene.camera),
            G35EmitSched::OitPair,
        );
        let mut lane2 = match G35OnLane::create(
            &descs_off,
            &accel,
            in_w,
            in_h,
            out_w,
            out_h,
            p11,
            d_max,
            G35Oit::Off,
            false,
            fx,
        ) {
            Ok(l) => l,
            Err(e) => fail(&format!("oit 见证 off 影面车道: {e}")),
        };
        let mut base_scene: Option<Vec<f32>> = None;
        for fi in 0..total {
            let spec = pose(fi);
            let vp = build_vp(&spec, in_w, in_h);
            let inv_vp = vp.inverse().unwrap_or_else(|| fail("view-proj 必须可逆"));
            let j = jit(fi);
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            let ctl = mirror2.step(fi);
            let scene_params = pack_frame_params(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
            );
            let last = fi + 1 == total;
            let rb = G35Rb {
                scene: last,
                ..G35Rb::default()
            };
            let rec = lane2
                .frame(
                    j,
                    &vp,
                    &vp_j,
                    spec.eye,
                    exposure,
                    fi == 0,
                    &ctl,
                    &mirror2.desc,
                    scene_params,
                    rb,
                )
                .unwrap_or_else(|e| fail(&format!("oit 见证 off 影面帧 {fi}: {e}")));
            if rec.validation_error_count != 0 {
                fail(&format!("oit 见证 off 影面帧 {fi} validation ERROR ≠ 0"));
            }
            if last {
                base_scene = rec.scene_color.clone();
            }
        }
        drop(lane2);
        let base = base_scene.unwrap_or_else(|| fail("oit 见证 off 影面缺 scene color 回读"));
        let pools = mirror.current();
        let n = pools.n;
        if n != 2 {
            fail(&format!("oit 见证粒子数 {n} ≠ 2(夹具寿命窗破缺)"));
        }
        let vp_static = build_vp(&scene.camera, in_w, in_h);
        let vp_j_last = last_vp_j.unwrap();
        let prev = last_prev_vp_j.unwrap_or(vp_j_last);
        // 期望恒按正协议(red=false)打包——红臂腿 p100 必大 = 检出面。
        let oparams =
            g35l_pack_oit_params(in_w, in_h, p11, d_max, &vp_j_last, &vp_static, &prev, false, &fx);
        let streams = (
            &pools.pos_x[..n],
            &pools.pos_y[..n],
            &pools.pos_z[..n],
            &pools.age[..n],
            &pools.life[..n],
        );
        let mut expect = base.clone();
        let mut acc_diff_max = 0u64;
        let mut sat_host = 0u32;
        if oit == G35Oit::Sorted {
            let _ = poit::oit_sorted_frame(&oparams, n, streams, dev_depth, &mut expect);
        } else {
            let (acc_h, sh) = poit::wboit_frame(&oparams, n, streams, dev_depth, &mut expect);
            sat_host = sh;
            let acc_d = last_rec
                .oit_acc
                .as_ref()
                .unwrap_or_else(|| fail("oit 见证缺 acc 回读"));
            for (d, h) in acc_d.iter().zip(acc_h.iter()) {
                let diff = u64::from(d.abs_diff(*h));
                if diff > acc_diff_max {
                    acc_diff_max = diff;
                }
            }
        }
        let mut p100 = 0.0f64;
        let mut changed_px = 0u64;
        for i in 0..(in_w * in_h) as usize {
            let mut px_changed = false;
            for c in 0..3 {
                let d = f64::from((dev_scene[i * 3 + c] - expect[i * 3 + c]).abs());
                if d > p100 {
                    p100 = d;
                }
                if dev_scene[i * 3 + c].to_bits() != base[i * 3 + c].to_bits() {
                    px_changed = true;
                }
            }
            if px_changed {
                changed_px += 1;
            }
        }
        format!(
            "{{\"arm\":{},\"red_arm\":{red_arm},\"particles\":2,\"second_emit_frame\":{G35L_OIT_WITNESS_F2},\"p100_vs_host\":{p100:.9e},\"changed_px\":{changed_px},\"acc_max_int_diff\":{},\"sat_device\":{},\"sat_host\":{},\"base\":{},\"note\":{}}}",
            jstr(oit.as_str()),
            if oit == G35Oit::Wboit {
                format!("{acc_diff_max}")
            } else {
                "null".to_owned()
            },
            if oit == G35Oit::Wboit {
                format!("{}", last_rec.oit_sat.as_ref().map(|s| s[0]).unwrap_or(0))
            } else {
                "null".to_owned()
            },
            if oit == G35Oit::Wboit {
                format!("{sat_host}")
            } else {
                "null".to_owned()
            },
            jstr("in-process --oit off shadow lane(前 12 pass 位级同 ⇒ 末帧 scene_color = OIT 输入基底)"),
            jstr("近远两粒子视轴夹具:帧 0/30 各发 1,纯前向 0.4 m/s ⇒ 先发者更远且 age 大偏红;host 期望恒按正协议(red=false)⇒ 红臂 p100 必大 = 键反转翻序检出"),
        )
    } else {
        "null".to_owned()
    };

    let (r_mean, r_min, r_max) = g35l_stats(&render_ms);
    let (pg_mean, _, _) = g35l_stats(&particle_gpu_ms);
    let particle_stats_json = format!(
        "{{\"n_final\":{},\"pids_issued\":{},\"args_last_host\":[{},{},{},{},{},{},{},{}],\"nseg_cap\":{},\"emit_max\":{emit_max},\"emit_schedule\":{}}}",
        last_ctl.n_next,
        mirror.pid_base,
        last_ctl.args_host[0],
        last_ctl.args_host[1],
        last_ctl.args_host[2],
        last_ctl.args_host[3],
        last_ctl.args_host[4],
        last_ctl.args_host[5],
        last_ctl.args_host[6],
        last_ctl.args_host[7],
        cap / SEG,
        if witness_mirror {
            jstr("witness:frame0 单发射")
        } else if emit_max == G35L_EMIT_MAX {
            jstr("min(64 + frame*17 % 192, cap - n_curr)")
        } else {
            jstr(&format!("min((64 + frame*17 % 192)*{emit_max}/256, cap - n_curr)"))
        },
    );
    let (og_mean, _, _) = g35l_stats(&oit_gpu_ms);
    let frame_ms_json = format!(
        "{{\"real_render_frame_ms\":{r_mean:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"particle_gpu_mean_ms\":{pg_mean:.6},\"oit_gpu_mean_ms\":{og_mean:.6},\"frames_measured\":{}}}",
        render_ms.len(),
    );
    // G35-4 生产 OIT 登记块(键域论证数字 + wboit 定点冻结面 + 饱和计数)。
    let oit_json = if oit == G35Oit::Off {
        "{\"mode\":\"off\"}".to_owned()
    } else {
        format!(
            "{{\"mode\":{},\"red_arm\":{red_arm},\"tile\":{{\"size_px\":16,\"tiles_x\":{oit_tiles_x},\"tiles_y\":{oit_tiles_y},\"tile_cnt\":{oit_tile_cnt},\"overflow_key\":{},\"key_domain_note\":{}}},\"wboit\":{},\"pass_layout\":{}}}",
            jstr(oit.as_str()),
            u64::from(oit_tile_cnt) * 4096,
            jstr("复合键 = tile_id·4096 + (4095−depth12);tile_cnt ≤ 4095 拒跑守卫 ⇒ 溢出键 ≤ 4095·4096 = 16773120 < 2^24 = 16777216;最大合法键 = tile_cnt·4096 − 1 < 溢出键(排序后溢出粒子全落尾,blend 侧像素 tile_id < tile_cnt 恒不取)"),
            if oit == G35Oit::Wboit {
                format!(
                    "{{\"scale_q12\":4096,\"delta_max\":65535,\"cap_bound\":65536,\"sat_events_total\":{},\"saturation_note\":{}}}",
                    last_rec.oit_sat.as_ref().map(|s| s[0]).unwrap_or(0),
                    jstr("饱和 = 加前 clamp(delta ≤ 65535 = 2^16−1)+ 事件累计计数(逐帧不清零单调登记);cap ≤ 65536 守卫 ⇒ 累加和 ≤ 2^32−2^16 < u32::MAX 结构性防回绕 = clamp 到 u32::MAX 语义的不可达顶证明;整数加可交换 ⇒ 双跑位级"),
                )
            } else {
                "null".to_owned()
            },
            jstr(match oit {
                G35Oit::Sorted =>
                    "presolve(11)后插 13 pass:hash_clear(tile 哨兵)→tilekey→sort 3-pass 9 dispatch(键/payload A→B→A→B)→tilerange→blend_sorted;TSR 25/26 encode 27(28 pass)",
                G35Oit::Wboit =>
                    "presolve(11)后插 3 pass:splat_clear(acc u64×2px 语义清零)→wboit_accum→wboit_resolve;TSR 15/16 encode 17(18 pass)",
                G35Oit::Off => "",
            }),
        )
    };
    // 展示面参数回显(全默认 = null;非默认 = 出图参数面如实登记,复跑可溯)。
    let showcase_json = if !fx_touched
        && !emitter_touched
        && dump_raw_path.is_empty()
        && ev100_override.is_none()
        && dump_every.is_none()
        && auto_move_amp == 1.0
        && !emitter_follow
        && emit_max == G35L_EMIT_MAX
    {
        "null".to_owned()
    } else {
        let emitter_json = if emitter_touched {
            format!(
                "{{\"pos\":[{},{},{}],\"spread\":[{},{},{}],\"vel_base\":[{},{},{}],\"vel_spread\":[{},{},{}],\"life_base\":{},\"gravity_y\":{}}}",
                desc.pos[0], desc.pos[1], desc.pos[2],
                desc.spread[0], desc.spread[1], desc.spread[2],
                desc.vel_base[0], desc.vel_base[1], desc.vel_base[2],
                desc.vel_spread[0], desc.vel_spread[1], desc.vel_spread[2],
                desc.life_base, desc.gravity_y,
            )
        } else {
            "null".to_owned()
        };
        let rain_json = if fx.rain_on() {
            format!(
                "{{\"shutter\":{},\"occlusion\":{},\"ray_tmin_m\":{},\"streak_max_px\":128,\"tsr_reactive\":true,\"note\":{}}}",
                fx.rain_shutter,
                fx.rain_occlusion,
                G35L_RAIN_RAY_TMIN,
                jstr("雨丝模式:splat = 运动模糊胶囊(首 pos → 尾 pos − vel·dt·shutter,tent 半径 clamp(rpx,0.5,2.25)+0.75,赢家足迹外扩 1.5 px)+ TLAS 逐粒子遮挡射线(eye → pos);resolve = tint(display 域绝对色 × 1/exposure)+ tent 剖面 × 亚像素覆盖峰值 × 末段淡出,赢家足迹写 U_REACTIVE = 1 且 has_reactive = 1 ⇒ TSR 取当前帧;不读 quirk 深度域"),
            )
        } else {
            "null".to_owned()
        };
        // 推轨短片面四键(缺省值如实回显:null / 1 / false / null / 256)。
        let emitter_pos_final_json = if emitter_follow {
            format!(
                "[{},{},{}]",
                mirror.desc.pos[0], mirror.desc.pos[1], mirror.desc.pos[2]
            )
        } else {
            "null".to_owned()
        };
        format!(
            "{{\"dump_present_raw\":{},\"dump_present_every\":{},\"r_world\":{},\"splat_stretch\":{},\"particle_tint\":[{},{},{}],\"particle_alpha_scale\":{},\"rain\":{},\"ev100_override\":{},\"emitter_override\":{},\"auto_move_amp\":{},\"emitter_follow_camera\":{},\"emitter_pos_final\":{},\"emit_max\":{}}}",
            if dump_raw_path.is_empty() {
                "null".to_owned()
            } else {
                jstr(&dump_raw_path.replace('\\', "/"))
            },
            dump_every.map_or("null".to_owned(), |n| format!("{n}")),
            fx.r_world, fx.stretch, fx.tint[0], fx.tint[1], fx.tint[2], fx.alpha_scale,
            rain_json,
            ev100_override.map_or("null".to_owned(), |v| format!("{v}")),
            emitter_json,
            auto_move_amp,
            emitter_follow,
            emitter_pos_final_json,
            emit_max,
        )
    };
    emit_evidence(
        &evidence_path,
        &EvidenceCtx {
            mode: match mode {
                G35Mode::OnFace => "on",
                G35Mode::MvWitness => "mv_witness",
                G35Mode::OcclusionWitness => "occlusion_witness",
                G35Mode::OitWitness => "oit_witness",
                _ => unreachable!(),
            },
            scene_id,
            tier,
            frames,
            warmup,
            static_camera: auto_move.is_none(),
            trajectory: auto_move.as_deref(),
            particles_on: true,
            cap,
            seed,
            contract_path: &contract_path,
            contract_digest: &contract.digest,
            gltf_path: &gltf_path,
            g10_json: &g10_json,
            spv: &spv,
            render_digest: &render_digest,
            digest_seq: &digest_seq,
            mv_witness_json,
            occlusion_json,
            mesh_json: "null".to_owned(),
            frame_ms_json,
            particle_stats_json,
            oit,
            oit_json,
            oit_witness_json,
            geo_json,
            showcase_json,
        },
    );
    eprintln!(
        "{G35L_TAG}: PASS on 面 oit={} frames={total} render={r_mean:.3}ms particle_gpu={pg_mean:.3}ms oit_gpu={og_mean:.3}ms presented={presented_digest}",
        oit.as_str(),
    );
}

// ---------------------------------------------------------------------------
// evidence 落盘(harness 真跑件;门裁决件归 smoke)
// ---------------------------------------------------------------------------

struct EvidenceCtx<'e> {
    mode: &'static str,
    scene_id: &'e str,
    tier: u32,
    frames: u32,
    warmup: u32,
    static_camera: bool,
    trajectory: Option<&'e str>,
    particles_on: bool,
    cap: usize,
    seed: u64,
    contract_path: &'e str,
    contract_digest: &'e str,
    /// 实际装配的 glTF 路径(缺省场景 = 共享体 default_gltf 解析结果;顶层
    /// `gltf` 块登记路径 + sha256,读不到如实 MISSING)。
    gltf_path: &'e str,
    g10_json: &'e str,
    spv: &'e G35SpvPaths,
    render_digest: &'e str,
    digest_seq: &'e [String],
    mv_witness_json: String,
    occlusion_json: String,
    mesh_json: String,
    frame_ms_json: String,
    particle_stats_json: String,
    /// G35-4 半透明档(审计行数按档;off = G35-3 现面 11 行)。
    oit: G35Oit,
    /// 生产 OIT 登记块(tile 键域论证/wboit 定点冻结面/pass 布局)。
    oit_json: String,
    /// 近远见证腿判读块(--oit-witness;p100/acc 整数差/饱和计数)。
    oit_witness_json: String,
    /// G36 W4 geo 组合块(--cluster-lod/--wp-hlod;off = "null" 0-byte)。
    geo_json: String,
    /// 展示面(网站出图)参数回显块(全默认 = "null";非默认 = 参数面如实登记)。
    showcase_json: String,
}

fn emit_evidence(path: &str, c: &EvidenceCtx) {
    let spv_entry = |p: &str| {
        format!(
            "{{\"path\":{},\"sha256\":{}}}",
            jstr(&p.replace('\\', "/")),
            jstr(&g35l_file_sha(p))
        )
    };
    let (audit_ok, audit_rows) = g35l_barrier_audit(c.oit);
    let mut audit_rows_json = String::new();
    for (k, r) in audit_rows.iter().enumerate() {
        if k > 0 {
            audit_rows_json.push(',');
        }
        audit_rows_json.push_str(&format!("{{\"pass\":{},\"ok\":{}}}", jstr(r.name), r.ok));
    }
    let mut seq_json = String::new();
    for (k, d) in c.digest_seq.iter().enumerate() {
        if k > 0 {
            seq_json.push(',');
        }
        seq_json.push_str(&jstr(d));
    }
    let digest_seq_sha = if c.digest_seq.is_empty() {
        "null".to_owned()
    } else {
        jstr(&format!(
            "sha256:{}",
            sha256_hex(c.digest_seq.join("\n").as_bytes())
        ))
    };
    let mut ev = String::with_capacity(8192);
    ev.push('{');
    ev.push_str(&format!("\"schema\":{},", jstr(G35L_RUN_SCHEMA)));
    ev.push_str(&format!("\"gate\":{},", jstr(G35L_GATE)));
    ev.push_str(&format!("\"mode\":{},", jstr(c.mode)));
    ev.push_str(&format!("\"scene\":{},", jstr(c.scene_id)));
    ev.push_str(&format!("\"tier\":{},", c.tier));
    ev.push_str(&format!(
        "\"frames\":{},\"warmup\":{},\"static_camera\":{},\"trajectory\":{},",
        c.frames,
        c.warmup,
        c.static_camera,
        match c.trajectory {
            Some(t) => jstr(t),
            None => "null".to_owned(),
        }
    ));
    ev.push_str(&format!(
        "\"particles\":{},\"cap\":{},\"seed\":{},\"dt\":{},",
        jstr(if c.particles_on { "on" } else { "off" }),
        c.cap,
        c.seed,
        G35L_DT,
    ));
    ev.push_str(&format!(
        "\"contract\":{{\"path\":{},\"digest\":{}}},",
        jstr(&c.contract_path.replace('\\', "/")),
        jstr(c.contract_digest)
    ));
    // 装配 glTF 登记(推轨短片面复跑可溯:路径 = 实际装配路径,缺省场景为
    // 共享体 default_gltf 解析结果;sha256 读不到 = MISSING)。
    ev.push_str(&format!(
        "\"gltf\":{{\"path\":{},\"sha256\":{}}},",
        jstr(&c.gltf_path.replace('\\', "/")),
        jstr(&g35l_file_sha(c.gltf_path))
    ));
    ev.push_str(&format!("\"g10_provenance\":{},", c.g10_json));
    ev.push_str(&format!(
        "\"spv\":{{\"scene\":{},\"mv\":{},\"resample\":{},\"resolve\":{},\"encode\":{},\"p_sim\":{},\"p_scan_seg_sum\":{},\"p_scan_spine\":{},\"p_scan_seg_apply\":{},\"p_compact\":{},\"p_emit\":{},\"p_indirect_args\":{},\"splat_clear\":{},\"splat\":{},\"presolve\":{}}},",
        spv_entry(&c.spv.scene),
        spv_entry(&c.spv.mv),
        spv_entry(&c.spv.resample),
        spv_entry(&c.spv.resolve),
        spv_entry(&c.spv.encode),
        spv_entry(&c.spv.p_sim),
        spv_entry(&c.spv.p_seg_sum),
        spv_entry(&c.spv.p_spine),
        spv_entry(&c.spv.p_seg_apply),
        spv_entry(&c.spv.p_compact),
        spv_entry(&c.spv.p_emit),
        spv_entry(&c.spv.p_args),
        spv_entry(&c.spv.splat_clear),
        spv_entry(&c.spv.splat),
        spv_entry(&c.spv.presolve),
    ));
    // G35-4 OIT SPV 九件(oit ≠ off 腿才装载消费;off 腿如实登记路径 sha,
    // 文件缺失 = MISSING)。
    if c.oit != G35Oit::Off {
        ev.push_str(&format!(
            "\"spv_oit\":{{\"sort_hist\":{},\"sort_spine\":{},\"sort_scatter\":{},\"hash_clear\":{},\"tilekey\":{},\"tilerange\":{},\"blend_sorted\":{},\"wboit_accum\":{},\"wboit_resolve\":{}}},",
            spv_entry(&c.spv.oit_sort_hist),
            spv_entry(&c.spv.oit_sort_spine),
            spv_entry(&c.spv.oit_sort_scatter),
            spv_entry(&c.spv.oit_hash_clear),
            spv_entry(&c.spv.oit_tilekey),
            spv_entry(&c.spv.oit_tilerange),
            spv_entry(&c.spv.oit_blend),
            spv_entry(&c.spv.oit_accum),
            spv_entry(&c.spv.oit_wresolve),
        ));
    }
    ev.push_str(&format!("\"render_digest\":{},", jstr(c.render_digest)));
    ev.push_str(&format!("\"digest_seq\":[{seq_json}],"));
    ev.push_str(&format!("\"digest_seq_sha\":{digest_seq_sha},"));
    ev.push_str(&format!(
        "\"barrier_plan_audit\":{{\"all_bindings_subset\":{audit_ok},\"passes\":[{audit_rows_json}],\"indirect_note\":{}}},",
        jstr("splat 的 args IndirectRead 转换由执行器隐式补全承载(render_exec pass_requirements_with 对 DispatchSpec::Indirect 首位推导);显式计划列 args StorageReadWrite(kernel 读 args[7] 守卫面),双屏障链覆盖两类访问"),
    ));
    ev.push_str(&format!(
        "\"zero_readback\":{{\"splat_dispatch\":{},\"production_readback_particle_buffers\":false,\"note\":{}}},",
        jstr("DispatchSpec::Indirect{res:args,offset:0}"),
        jstr("生产帧循环零粒子缓冲回读;args/seg_offsets/winner 只在见证腿末帧按子集回读(见证面非生产口径);dispatch 计数 host 金标准平行推得对拍,不读回 device"),
    ));
    ev.push_str(&format!(
        "\"depth_domain\":{},",
        jstr("U_SCENE_DEPTH 生产字面 = 未抖 vp 参数行 25..32(vp 行 0/1,clip.x/clip.y;g34_unified_gi.rx ⑦ 段「生产字面」注,真 ZO NDC 归 g34_unified_shade.rx ④b HZB 车道形态)——本波硬拒/软粒子与存储域同域实现;该域沿视射线为常量 ⇒ 同域比较为屏幕域序判而非距离遮挡,如实登记;遮挡见证取相机后已知墙后构型(投影 w 门确定性拒绝路径)"),
    ));
    ev.push_str(&format!("\"mv_witness\":{},", c.mv_witness_json));
    ev.push_str(&format!("\"occlusion_witness\":{},", c.occlusion_json));
    ev.push_str(&format!("\"mesh_particles\":{},", c.mesh_json));
    ev.push_str(&format!("\"oit\":{},", c.oit_json));
    ev.push_str(&format!("\"oit_witness\":{},", c.oit_witness_json));
    ev.push_str(&format!("\"geo\":{},", c.geo_json));
    ev.push_str(&format!("\"showcase\":{},", c.showcase_json));
    ev.push_str(&format!("\"frame_ms\":{},", c.frame_ms_json));
    ev.push_str(&format!("\"particle_stats\":{},", c.particle_stats_json));
    ev.push_str("\"headless\":true,");
    ev.push_str(&format!(
        "\"anchor\":{{\"path\":{},\"cell\":{}}},",
        jstr(G35L_ANCHOR_PATH),
        jstr(G35L_ANCHOR_CELL)
    ));
    ev.push_str(&format!(
        "\"notes\":{}",
        jstr("G35-3 粒子渲染接线:g14_3_lane_body 共享体 0-byte(include! 逐字共享);--particles off = 母版 Mega 22 资源四 pass 位级(Stage A 锚);on = mv 与 TSR 之间插 10 粒子 pass(sim→scan×3→compact→emit→indirect_args→splat_clear→splat→resolve)+ encode,资源 22..=53 bin 局部追加;FrameUpdate 重映射 = bin 自持 prepare 按插入后下标 (12,13,14) 构造 TSR/encode overrides + 粒子 5 pass A/B parity overrides(共享体 (2,3) 硬编码不消费);splat = u64 fetch_max 赢家(key = 16777215 − depth_key24(d_view,far) 反深度<<40|slot)+ DispatchSpec::Indirect 零回读;resolve = 程序化调色(暖白→橙红×(1−t)×8.0)+ 软粒子 alpha=(1−t)·soft + 粒子 MV mv=project_curr(pos)−project_prev(pos−vel·dt) 覆写相机 MV;host 金标准 particles::core::frame 平行镜像驱动 n_curr/emit 参数面(整数流零容差 + NoContraction 注入面 W2 实测位级同源)。")
    ));
    ev.push('}');
    if path.is_empty() {
        println!("{ev}");
    } else {
        std::fs::write(path, format!("{ev}\n"))
            .unwrap_or_else(|e| fail(&format!("evidence 写 {path}: {e}")));
        eprintln!("{G35L_TAG}: evidence → {path}");
    }
}
