//! G35-4 半透明双臂 host 金标准(排序 back-to-front + 定点 WBOIT)——门
//! `g35.wave4.sort_oit`(RFC-0049 §4.8 评审修订后冻结协议;契约锚 =
//! milestones/g35/G35_CONTRACT.md G-G35-4)。
//!
//! ## 双臂冻结协议(本文件 = host 单源;device kernel 逐字同源镜像)
//!
//! ### 臂 A:tile 排序 back-to-front(位级确定基准)
//! - **复合键**(= kernels/g35_oit_tilekey.rx [`oit_tilekey`]):
//!   `key = tile_id·4096 + (4095 − depth12)`,tile = 16px 内部分辨率网格
//!   (tile_id = (cy/16)·tiles_x + cx/16,cx/cy = 粒子中心像素 floor 归属,
//!   [`OIT_TILE`]),depth12 = floor(clamp(d_view/d_max,0,1)·4095)。
//!   反键(4095−)⇒ radix 升序 = tile 内 far→near = painter's(RFC-0049
//!   §4.3 F4 反键律的 12 位 tile 段化)。**屏外/被剔粒子键 = tile_cnt·4096
//!   溢出 tile**(w 门/视深门/中心像素屏外三路;合成侧像素 tile_id <
//!   tile_cnt 恒不取)。**键域论证**:tile_cnt ≤ 4095 硬域(lane 拒跑守卫)
//!   ⇒ 溢出键 ≤ 4095·4096 = 16 773 120 < 2^24 = 16 777 216,最大合法键 =
//!   (tile_cnt−1)·4096+4095 = tile_cnt·4096 − 1 < 溢出键(排序后溢出粒子
//!   全落尾)——门腿构型 bistro t50 内部 960×540 ⇒ tiles 60×34 = 2040 ✓
//!   (t100 1920×1080 ⇒ 8160 > 4095 越域拒跑如实登记)。
//! - **tile 归属 = 中心像素单键**(v1 冻结):粒子只在中心归属 tile 的像素
//!   域合成;rpx ≤ 3 < 16 ⇒ 跨 tile 截断带 ≤ 3px 如实登记(多 tile 展开
//!   归评估窗,不预支)。
//! - **区间**(= kernels/g35_oit_tilerange.rx [`oit_tile_ranges`]):排序键流
//!   按 `key/4096`(= tile_id)分组检测边界(g35_hash_cellrange.rx 按完整键
//!   分组**不可直用**——本臂区间域 = tile 段非完整键,故自写同形 kernel);
//!   哨兵 = start 0xFFFFFFFF / end 0(g35_hash_clear.rx 清扫面 0-byte 消费,
//!   空区间 while 零次直落)。
//! - **串行合成**(= kernels/g35_oit_blend_sorted.rx [`oit_blend_pixel`]):
//!   每像素迭代其 tile 区间**升序 = far→near**,逐粒子圆形覆盖测试 + 同域
//!   硬拒(d_p > scene_depth 跳过)+ 程序化调色 + 软粒子 α(三式与
//!   g35_render_splat.rx ①②③ 段 / g35_render_resolve.rx ②③ 段逐字同式)
//!   + `C = C·(1−α) + c·α` 固定序串行 ⇒ 位级确定(radix 稳定序 + 段内
//!   下标序全序,与线程调度无关)。
//!
//! ### 臂 B:WBOIT 定点整数累加(顺序无关;RFC-0049 §4.8 F5)
//! - **Q 格式冻结**:SCALE = 4096(Q12,[`OIT_SCALE`]);舍入 = floor;
//!   **饱和 = 加前检查 clamp**:单项加数 delta = floor(x·4096) 先 clamp 到
//!   [`OIT_DELTA_MAX`] = 65535(= 2^16−1;clamp 触发 = 饱和事件,原子计数
//!   如实登记)——结构性防回绕论证:单像素累加次数 ≤ total ≤ cap ≤ 65536
//!   = 2^16([`OIT_WBOIT_CAP_MAX`],lane 拒跑守卫)⇒ 累加和 ≤ 2^16·(2^16−1)
//!   = 2^32 − 2^16 < u32::MAX,u32 fetch_add 恒不越顶 ⇒ "clamp 到 u32::MAX"
//!   语义以不可达顶的结构性证明兑现;整数加法可交换可结合 ⇒ 与片元到达序
//!   无关 ⇒ 双跑位级确定。
//! - **权函数 w(z) 冻结**(参照 = src/rurix-render/src/oit/algorithms.rs
//!   `run_weighted` nvpro `oitWeighted` 权重式,对齐关系:dist_weight 与
//!   alpha_weight 两因子公式逐字同形,尾除 3000 = dist_weight clamp 上界
//!   归一进 Q12 加数预算——host 参照臂为 f32 浮点累加近似档,本臂定点化后
//!   数值面进 evidence 不进硬门):`depth_z = d_view·10;dist_w =
//!   clamp(0.03/(1e-5 + (depth_z/200)^4), 1e-2, 3e3);premult = col·α;
//!   aw = min(1, max(premult.r,g,b,α)·40 + 0.01)²;w = aw·dist_w/3000`。
//! - **累加式**([`wboit_accum_particle`] = kernels/g35_oit_wboit_accum.rx):
//!   每粒子每覆盖像素(splat 同式包围盒 + 圆形测试 + 同域硬拒)
//!   `acc_r/g/b += floor(c·α·w·4096) clamp 65535;acc_w += floor(α·w·4096)
//!   clamp 65535`(c = 程序化调色 col,α = fade·soft resolve 同式)。
//! - **resolve 冻结**([`wboit_resolve_pixel`] = kernels/g35_oit_wboit_resolve
//!   .rx;参照 host 臂 `c = accum.rgb/max(accum.a,1e-5);px = c·(1−reveal) +
//!   bg·reveal`):`sum_w = acc_w/4096;c_ch = (acc_ch/4096)/max(sum_w,1e-5);
//!   α_out = min(1, sum_w);C = C·(1−α_out) + c·α_out`——reveal 连乘在定点
//!   整数域不可交换故不可原子化,以 `min(1, Σw·α)` 替代冻结(诚实登记:
//!   非 nvpro reveal 语义,近似档口径)。
//!
//! ## 参数面(P_OIT_PARAMS 96 f32;bin/g35_particle_lane.rs
//! `g35l_pack_oit_params` 单源,kernel 头注镜像)
//! `[0..64)` = P_PARAMS_RENDER 64 f32 逐字镜像([0]=iw [1]=ih [2]=r_world
//! [3]=soft_range [4]=d_max [5]=dt [6]=px_count [7]=p11 [8..24)=vp_j
//! [24..40)=vp [40..56)=prev_vp_j [56..64)=0);`[64]=tiles_x [65]=tiles_y
//! [66]=tile_cnt [67]=red_flag(1 = 红臂键反转篡改:去掉 4095− 翻转,
//! 近远序见证必检出)[68..96)=reserved(恒 0)`。

/// tile 边长(px;内部分辨率网格)。
pub const OIT_TILE: usize = 16;
/// 复合键深度段基数(= 2^12;tile_id·4096 + depth 段)。
pub const OIT_DEPTH_SLOTS: u32 = 4096;
/// depth12 满刻度(= 2^12 − 1;f32 精确域)。
pub const OIT_DEPTH_MAX: f32 = 4095.0;
/// tile 数硬域上限(键域 < 2^24 论证前提;lane 拒跑守卫)。
pub const OIT_TILE_CNT_MAX: u32 = 4095;
/// WBOIT 定点 Q12 刻度。
pub const OIT_SCALE: f32 = 4096.0;
/// WBOIT 单项加数预算(= 2^16 − 1;加前 clamp = 饱和事件)。
pub const OIT_DELTA_MAX: u32 = 65535;
/// WBOIT 臂池容量硬域(= 2^16;cap·DELTA_MAX = 2^32 − 2^16 < u32::MAX
/// 结构性防回绕论证前提;lane 拒跑守卫)。
pub const OIT_WBOIT_CAP_MAX: usize = 65536;
/// WBOIT dist_weight clamp 上界 = 权归一分母(nvpro 参照面)。
pub const OIT_W_NORM: f32 = 3000.0;

// ---------------------------------------------------------------------------
// 共享小式(splat/resolve kernel 同式;各消费函数内联展开保持逐字同源,
// 此处仅供单测/lane 期望复用)
// ---------------------------------------------------------------------------

/// 生产字面深度域 d_p(未抖 vp 行 0/1 + |w|≤1e-8 → 1.0 门;
/// g35_render_splat.rx ③ 段逐字同式,params = 96 f32 布局)。
pub fn oit_depth_p(params: &[f32], px_: f32, py_: f32, pz_: f32) -> f32 {
    let big: f32 = 1e30;
    let qz = ((params[24] * px_ + params[25] * py_) + params[26] * pz_) + params[27];
    let qw = ((params[28] * px_ + params[29] * py_) + params[30] * pz_) + params[31];
    let qw_abs = qw.max(0.0 - qw);
    let gate_w = ((qw_abs - 0.000_000_01) * big).min(1.0).max(0.0);
    let qw_safe = qw + (1.0 - gate_w);
    gate_w * (qz / qw_safe) + (1.0 - gate_w)
}

/// 程序化调色 + fade(g35_render_resolve.rx ② 段逐字同式)。
/// 返回 (col_r, col_g, col_b, fade)。
pub fn oit_shade(age: f32, life: f32) -> (f32, f32, f32, f32) {
    let mut t = age / life.max(0.000_001);
    if t < 0.0 {
        t = 0.0;
    }
    if t > 1.0 {
        t = 1.0;
    }
    let base_r = 1.0 + (1.0 - 1.0) * t;
    let base_g = 0.9 + (0.3 - 0.9) * t;
    let base_b = 0.7 + (0.05 - 0.7) * t;
    let fade = 1.0 - t;
    (base_r * 8.0 * fade, base_g * 8.0 * fade, base_b * 8.0 * fade, fade)
}

/// 投影到屏幕(g35_render_splat.rx ①② 段逐字同式)。
/// 返回 Some((cxf, cyf, rpx, d_view));w 门/视深门失败 → None(剔除)。
pub fn oit_project(params: &[f32], px_: f32, py_: f32, pz_: f32) -> Option<(f32, f32, f32, f32)> {
    let iwf = params[0];
    let ihf = params[1];
    let r_world = params[2];
    let p11 = params[7];
    let cx = ((params[8] * px_ + params[9] * py_) + params[10] * pz_) + params[11];
    let cy = ((params[12] * px_ + params[13] * py_) + params[14] * pz_) + params[15];
    let cw = ((params[20] * px_ + params[21] * py_) + params[22] * pz_) + params[23];
    if cw <= 0.000_001 {
        return None;
    }
    let ndx = cx / cw;
    let ndy = cy / cw;
    let cxf = (ndx + 1.0) * 0.5 * iwf - 0.5;
    let cyf = (1.0 - ndy) * 0.5 * ihf - 0.5;
    let d_view = ((params[36] * px_ + params[37] * py_) + params[38] * pz_) + params[39];
    if d_view <= 0.000_001 {
        return None;
    }
    let mut rpx = r_world * 0.5 * ihf * p11 / d_view;
    if rpx > 3.0 {
        rpx = 3.0;
    }
    if rpx < 0.5 {
        rpx = 0.5;
    }
    Some((cxf, cyf, rpx, d_view))
}

// ---------------------------------------------------------------------------
// ① tilekey(= kernels/g35_oit_tilekey.rx 逐字同源)
// ---------------------------------------------------------------------------

/// 逐粒子复合键(= g35_oit_tilekey.rx 主体逐字同源):
/// `key = tile_id·4096 + (4095 − depth12)`;屏外/被剔 = 溢出键
/// `tile_cnt·4096`;red_flag(params[67] ≥ 0.5)= 键反转篡改臂
/// (去掉 4095− 翻转,depth 段直用 depth12 ⇒ 升序 = near→far 翻序,
/// 见证必检出)。
pub fn oit_tilekey(params: &[f32], px_: f32, py_: f32, pz_: f32) -> u32 {
    let iwf = params[0];
    let ihf = params[1];
    let d_max = params[4];
    let tiles_x = params[64] as usize;
    let tile_cnt = params[66] as u32;
    let red = params[67];
    let mut key = tile_cnt * OIT_DEPTH_SLOTS;
    if let Some((cxf, cyf, _rpx, d_view)) = oit_project(params, px_, py_, pz_) {
        if cxf >= 0.0 && cxf < iwf && cyf >= 0.0 && cyf < ihf {
            let cpx = cxf.floor() as usize;
            let cpy = cyf.floor() as usize;
            let tile_id = ((cpy / OIT_TILE) * tiles_x + cpx / OIT_TILE) as u32;
            let mut t = d_view / d_max;
            if t < 0.0 {
                t = 0.0;
            }
            if t > 1.0 {
                t = 1.0;
            }
            let d12 = (t * OIT_DEPTH_MAX).floor() as u32;
            let dk = if red >= 0.5 { d12 } else { 4095 - d12 };
            key = tile_id * OIT_DEPTH_SLOTS + dk;
        }
    }
    key
}

// ---------------------------------------------------------------------------
// ② tile 区间(= kernels/g35_oit_tilerange.rx 逐字同源)
// ---------------------------------------------------------------------------

/// 排序键流 tile 区间边界(= g35_oit_tilerange.rx 逐字同源):分组 =
/// `key/4096`(tile_id 段,g35_hash_cellrange 完整键分组不可直用故自写);
/// 未触 tile 保持哨兵 start=0xFFFFFFFF / end=0(g35_hash_clear 清扫面,
/// 空区间循环零次直落)。n_tiles 含溢出 tile(= tile_cnt+1)。
pub fn oit_tile_ranges(sorted_keys: &[u32], n_tiles: usize) -> (Vec<u32>, Vec<u32>) {
    let n = sorted_keys.len();
    let mut start = vec![u32::MAX; n_tiles];
    let mut end = vec![0u32; n_tiles];
    for i in 0..n {
        let g = (sorted_keys[i] / OIT_DEPTH_SLOTS) as usize;
        if i == 0 || sorted_keys[i - 1] / OIT_DEPTH_SLOTS != sorted_keys[i] / OIT_DEPTH_SLOTS {
            start[g] = i as u32;
        }
        if i == n - 1 || sorted_keys[i + 1] / OIT_DEPTH_SLOTS != sorted_keys[i] / OIT_DEPTH_SLOTS {
            end[g] = (i + 1) as u32;
        }
    }
    (start, end)
}

// ---------------------------------------------------------------------------
// ③ 串行合成(= kernels/g35_oit_blend_sorted.rx 逐字同源)
// ---------------------------------------------------------------------------

/// 每像素 tile 区间固定序串行合成(= g35_oit_blend_sorted.rx 主体逐字同源):
/// 区间升序 = far→near painter's;逐粒子圆形覆盖测试 + 软深度 α +
/// `C = C·(1−α) + c·α`。**循环体零分支 gate 代数化**(SPIR-V 结构化控制流
/// 承载;gate 门式 = splat ③ 段 gate_w 先例):w 门/视深门/圆形测试折算为
/// 乘法因子 `alpha = fade·soft·cw_gate·dv_gate·circle_gate`;硬拒(d_p >
/// scene_depth)由 soft clamp 到 0 自然涵盖(blend 无 winner 竞争,α = 0
/// 即无效合成——与 splat 显式硬拒分支语义等价登记);安全分母
/// `cw_safe = cw·gate + (1−gate)` 恒正防 NaN(gate_w 先例的乘法收紧变体,
/// 头注登记)。圆形判据 gate = ((rq²−dist²)·1e30 + 1) clamp [0,1](dist² ≤
/// rq² 含等边界保留)。`streams` = (pos_x, pos_y, pos_z, age, life) B 组
/// 前缀;`rgb` = 该像素 scene_color 三通道原位合成。
#[allow(clippy::too_many_arguments)]
pub fn oit_blend_pixel(
    params: &[f32],
    x: usize,
    y: usize,
    payload: &[u32],
    tile_start: &[u32],
    tile_end: &[u32],
    streams: (&[f32], &[f32], &[f32], &[f32], &[f32]),
    scene_depth_i: f32,
    rgb: &mut [f32; 3],
) {
    let (pos_x, pos_y, pos_z, age, life) = streams;
    let iwf = params[0];
    let ihf = params[1];
    let r_world = params[2];
    let soft_range = params[3];
    let p11 = params[7];
    let tiles_x = params[64] as usize;
    let g = (y / OIT_TILE) * tiles_x + x / OIT_TILE;
    let big: f32 = 1e30;
    let sd = scene_depth_i;
    let e = tile_end[g] as usize;
    let mut j = tile_start[g] as usize;
    while j < e {
        let slot = payload[j] as usize;
        let px_ = pos_x[slot];
        let py_ = pos_y[slot];
        let pz_ = pos_z[slot];
        // 投影(splat ①② 段同式;gate 代数化)。
        let ccx = ((params[8] * px_ + params[9] * py_) + params[10] * pz_) + params[11];
        let ccy = ((params[12] * px_ + params[13] * py_) + params[14] * pz_) + params[15];
        let ccw = ((params[20] * px_ + params[21] * py_) + params[22] * pz_) + params[23];
        let cw_gate = ((ccw - 0.000_001) * big).min(1.0).max(0.0);
        let cw_safe = ccw * cw_gate + (1.0 - cw_gate);
        let ndx = ccx / cw_safe;
        let ndy = ccy / cw_safe;
        let cxf = (ndx + 1.0) * 0.5 * iwf - 0.5;
        let cyf = (1.0 - ndy) * 0.5 * ihf - 0.5;
        let d_view = ((params[36] * px_ + params[37] * py_) + params[38] * pz_) + params[39];
        let dv_gate = ((d_view - 0.000_001) * big).min(1.0).max(0.0);
        let dv_safe = d_view * dv_gate + (1.0 - dv_gate);
        let rpx = (r_world * 0.5 * ihf * p11 / dv_safe).min(3.0).max(0.5);
        let fdx = (x as f32) - cxf;
        let fdy = (y as f32) - cyf;
        let circle_gate = (((rpx * rpx - (fdx * fdx + fdy * fdy)) * big) + 1.0)
            .min(1.0)
            .max(0.0);
        // 生产字面深度域 d_p(splat ③ 段逐字同式)。
        let qz = ((params[24] * px_ + params[25] * py_) + params[26] * pz_) + params[27];
        let qw = ((params[28] * px_ + params[29] * py_) + params[30] * pz_) + params[31];
        let qw_abs = qw.max(0.0 - qw);
        let gate_w = ((qw_abs - 0.000_000_01) * big).min(1.0).max(0.0);
        let qw_safe = qw + (1.0 - gate_w);
        let d_p = gate_w * (qz / qw_safe) + (1.0 - gate_w);
        // 调色 + 软 α(resolve ②③ 段同式)。
        let (col_r, col_g, col_b, fade) = oit_shade(age[slot], life[slot]);
        let soft = ((sd - d_p) / soft_range).min(1.0).max(0.0);
        let alpha = fade * soft * cw_gate * dv_gate * circle_gate;
        rgb[0] = rgb[0] * (1.0 - alpha) + col_r * alpha;
        rgb[1] = rgb[1] * (1.0 - alpha) + col_g * alpha;
        rgb[2] = rgb[2] * (1.0 - alpha) + col_b * alpha;
        j += 1;
    }
}

/// 全帧 sorted 臂期望(lane 见证腿/单测消费):tilekey → 稳定 radix 排序
/// (payload = slot)→ 区间 → 逐像素串行合成写 `scene_color`(基底原位)。
/// 返回排序产物 (keys, payload)(诊断面)。
#[allow(clippy::too_many_arguments)]
pub fn oit_sorted_frame(
    params: &[f32],
    n: usize,
    streams: (&[f32], &[f32], &[f32], &[f32], &[f32]),
    scene_depth: &[f32],
    scene_color: &mut [f32],
) -> (Vec<u32>, Vec<u32>) {
    let (pos_x, pos_y, pos_z, _, _) = streams;
    let iw = params[0] as usize;
    let ih = params[1] as usize;
    let tile_cnt = params[66] as usize;
    let mut keys = Vec::with_capacity(n);
    let mut payload = Vec::with_capacity(n);
    for s in 0..n {
        keys.push(oit_tilekey(params, pos_x[s], pos_y[s], pos_z[s]));
        payload.push(s as u32);
    }
    let (sk, sp) = super::primitives::sort_pairs_u24(&keys, &payload);
    let (ts, te) = oit_tile_ranges(&sk, tile_cnt + 1);
    for y in 0..ih {
        for x in 0..iw {
            let i = y * iw + x;
            let mut rgb = [
                scene_color[i * 3],
                scene_color[i * 3 + 1],
                scene_color[i * 3 + 2],
            ];
            oit_blend_pixel(params, x, y, &sp, &ts, &te, streams, scene_depth[i], &mut rgb);
            scene_color[i * 3] = rgb[0];
            scene_color[i * 3 + 1] = rgb[1];
            scene_color[i * 3 + 2] = rgb[2];
        }
    }
    (sk, sp)
}

// ---------------------------------------------------------------------------
// ④ WBOIT 定点累加 + resolve(= g35_oit_wboit_accum/resolve.rx 逐字同源)
// ---------------------------------------------------------------------------

/// 单项加数定点量化(floor 舍入 + 加前 clamp 饱和;= accum kernel 逐字同式)。
/// 返回 (delta, saturated)。
pub fn wboit_quantize(x: f32) -> (u32, bool) {
    let mut q = (x * OIT_SCALE).floor();
    if q < 0.0 {
        q = 0.0;
    }
    // f32 域比较(kernel 逐字同式;floor 后整数值 f32 在 2^24 内精确,
    // > 65535.0 判据与整数比较等价)。
    if q > 65535.0 {
        (OIT_DELTA_MAX, true)
    } else {
        (q as u32, false)
    }
}

/// WBOIT 权函数(nvpro 参照冻结;头注对齐关系)。
pub fn wboit_weight(d_view: f32, col: (f32, f32, f32), alpha: f32) -> f32 {
    let depth_z = d_view * 10.0;
    let q = depth_z / 200.0;
    let q2 = q * q;
    let q4 = q2 * q2;
    let dist_w = (0.03 / (0.000_01 + q4)).max(0.01).min(OIT_W_NORM);
    let pr = col.0 * alpha;
    let pg = col.1 * alpha;
    let pb = col.2 * alpha;
    let aw_in = pr.max(pg).max(pb).max(alpha);
    let aw = (aw_in * 40.0 + 0.01).min(1.0);
    aw * aw * dist_w / OIT_W_NORM
}

/// 单粒子全覆盖像素定点累加(= g35_oit_wboit_accum.rx 主体逐字同源):
/// splat 同式包围盒 + 圆形测试 + 同域硬拒;acc 布局 = px·4 u32
/// (i·4 + {0=r,1=g,2=b,3=w});sat = 饱和事件累计计数(可交换 fetch_add)。
#[allow(clippy::too_many_arguments)]
pub fn wboit_accum_particle(
    params: &[f32],
    px_: f32,
    py_: f32,
    pz_: f32,
    age_: f32,
    life_: f32,
    scene_depth: &[f32],
    acc: &mut [u32],
    sat: &mut u32,
) {
    let iw = params[0] as usize;
    let ih = params[1] as usize;
    let iwf = params[0];
    let ihf = params[1];
    let soft_range = params[3];
    let Some((cxf, cyf, rpx, d_view)) = oit_project(params, px_, py_, pz_) else {
        return;
    };
    let mut x0f = (cxf - rpx).floor();
    let mut x1f = (cxf + rpx).floor();
    let mut y0f = (cyf - rpx).floor();
    let mut y1f = (cyf + rpx).floor();
    if x0f < 0.0 {
        x0f = 0.0;
    }
    if y0f < 0.0 {
        y0f = 0.0;
    }
    if x1f > iwf - 1.0 {
        x1f = iwf - 1.0;
    }
    if y1f > ihf - 1.0 {
        y1f = ihf - 1.0;
    }
    if x1f < x0f || y1f < y0f {
        return;
    }
    let big: f32 = 1e30;
    let d_p = oit_depth_p(params, px_, py_, pz_);
    let (col_r, col_g, col_b, fade) = oit_shade(age_, life_);
    let rq2 = rpx * rpx;
    let (x0, x1, y0, y1) = (x0f as usize, x1f as usize, y0f as usize, y1f as usize);
    let _ = ih; // 包围盒 clamp 不变量:i = y·iw+x < iw·ih(kernel 同注)
    // 逐像素零分支体(gate 代数化 = kernel ③ 段逐字同式:圆形测试 gate、
    // 硬拒由 soft clamp 0 涵盖、饱和经 gate 计数)。
    for y in y0..=y1 {
        for x in x0..=x1 {
            let i = y * iw + x;
            let fdx = (x as f32) - cxf;
            let fdy = (y as f32) - cyf;
            let circle_gate = (((rq2 - (fdx * fdx + fdy * fdy)) * big) + 1.0)
                .min(1.0)
                .max(0.0);
            let soft = ((scene_depth[i] - d_p) / soft_range).min(1.0).max(0.0);
            let alpha = fade * soft * circle_gate;
            let w = wboit_weight(d_view, (col_r, col_g, col_b), alpha);
            let (dr, s0) = wboit_quantize(col_r * alpha * w);
            let (dg, s1) = wboit_quantize(col_g * alpha * w);
            let (db, s2) = wboit_quantize(col_b * alpha * w);
            let (dw, s3) = wboit_quantize(alpha * w);
            acc[i * 4] += dr;
            acc[i * 4 + 1] += dg;
            acc[i * 4 + 2] += db;
            acc[i * 4 + 3] += dw;
            *sat += u32::from(s0) + u32::from(s1) + u32::from(s2) + u32::from(s3);
        }
    }
}

/// WBOIT resolve 单像素(= g35_oit_wboit_resolve.rx 主体逐字同源;
/// 头注冻结式:`sum_w = acc_w/4096;c = (acc/4096)/max(sum_w,1e-5);
/// α_out = min(1,sum_w);C = C·(1−α_out) + c·α_out`;acc_w = 0 ⇒ 背景不动)。
pub fn wboit_resolve_pixel(a: [u32; 4], rgb: &mut [f32; 3]) {
    if a[3] == 0 {
        return;
    }
    let sum_w = a[3] as f32 / OIT_SCALE;
    let denom = sum_w.max(0.000_01);
    let cr = (a[0] as f32 / OIT_SCALE) / denom;
    let cg = (a[1] as f32 / OIT_SCALE) / denom;
    let cb = (a[2] as f32 / OIT_SCALE) / denom;
    let alpha = sum_w.min(1.0);
    rgb[0] = rgb[0] * (1.0 - alpha) + cr * alpha;
    rgb[1] = rgb[1] * (1.0 - alpha) + cg * alpha;
    rgb[2] = rgb[2] * (1.0 - alpha) + cb * alpha;
}

/// 全帧 WBOIT 臂期望(lane 见证腿/单测消费):逐粒子累加(下标序——整数
/// 可交换 ⇒ 与序无关,单测排列不变性判据)→ 逐像素 resolve 写 scene_color
/// 基底原位。返回 (acc, sat)。
pub fn wboit_frame(
    params: &[f32],
    n: usize,
    streams: (&[f32], &[f32], &[f32], &[f32], &[f32]),
    scene_depth: &[f32],
    scene_color: &mut [f32],
) -> (Vec<u32>, u32) {
    let (pos_x, pos_y, pos_z, age, life) = streams;
    let iw = params[0] as usize;
    let ih = params[1] as usize;
    let mut acc = vec![0u32; iw * ih * 4];
    let mut sat = 0u32;
    for s in 0..n {
        wboit_accum_particle(
            params, pos_x[s], pos_y[s], pos_z[s], age[s], life[s], scene_depth, &mut acc, &mut sat,
        );
    }
    for i in 0..iw * ih {
        let a = [acc[i * 4], acc[i * 4 + 1], acc[i * 4 + 2], acc[i * 4 + 3]];
        let mut rgb = [
            scene_color[i * 3],
            scene_color[i * 3 + 1],
            scene_color[i * 3 + 2],
        ];
        wboit_resolve_pixel(a, &mut rgb);
        scene_color[i * 3] = rgb[0];
        scene_color[i * 3 + 1] = rgb[1];
        scene_color[i * 3 + 2] = rgb[2];
    }
    (acc, sat)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 单测夹具 params:32×32 内部分辨率、单位化正交投影面——vp_j 恒等映
    /// 射(cx = px_,cy = py_,cw = 1)⇒ cxf = (px_+1)·16 − 0.5;vp 行 0/1
    /// 深度域 = pz_(qz = pz_,qw = 1);视深行 3 = pz_;tiles 2×2。
    fn fixture_params() -> Vec<f32> {
        let mut p = vec![0.0f32; 96];
        p[0] = 32.0; // iw
        p[1] = 32.0; // ih
        p[2] = 0.02; // r_world
        p[3] = 0.05; // soft_range
        p[4] = 100.0; // d_max
        p[5] = 1.0 / 60.0; // dt
        p[6] = 32.0 * 32.0; // px_count
        p[7] = 1.0; // p11
        // vp_j:行 0 = [1,0,0,0],行 1 = [0,1,0,0],行 3 = [0,0,0,1](cw=1)。
        p[8] = 1.0;
        p[13] = 1.0;
        p[23] = 1.0;
        // vp(未抖):行 0 = [0,0,1,0](qz = pz_),行 1 = [0,0,0,1](qw=1),
        // 行 3 = [0,0,1,0](d_view = pz_)。
        p[26] = 1.0;
        p[31] = 1.0;
        p[38] = 1.0;
        p[64] = 2.0; // tiles_x
        p[65] = 2.0; // tiles_y
        p[66] = 4.0; // tile_cnt
        p
    }

    /// 屏幕像素 (x, y) 反解夹具世界坐标(cxf = x ⇔ px_ = (x+0.5)/16 − 1)。
    fn world_at(x: f32, y: f32, depth: f32) -> (f32, f32, f32) {
        ((x + 0.5) / 16.0 - 1.0, 1.0 - (y + 0.5) / 16.0, depth)
    }

    /// ① tile 边界:中心像素 floor 归属(15.x → tile 0,16.x → tile 1)、
    /// 键反键单调(近者键大)、屏外/后向 = 溢出键、红臂翻转。
    #[test]
    fn tilekey_tile_boundary_and_inverted_depth() {
        let p = fixture_params();
        let (ax, ay, az) = world_at(15.9, 8.0, 10.0);
        let (bx, by, bz) = world_at(16.1, 8.0, 10.0);
        let ka = oit_tilekey(&p, ax, ay, az);
        let kb = oit_tilekey(&p, bx, by, bz);
        assert_eq!(ka / OIT_DEPTH_SLOTS, 0, "15.9px 中心必归 tile 0(floor 语义)");
        assert_eq!(kb / OIT_DEPTH_SLOTS, 1, "16.1px 中心必归 tile 1");
        // 反键:近者(depth 小)键大 ⇒ 排序升序 = far→near。
        let (nx, ny, nz) = world_at(8.0, 8.0, 5.0);
        let (fx, fy, fz) = world_at(8.0, 8.0, 50.0);
        let kn = oit_tilekey(&p, nx, ny, nz);
        let kf = oit_tilekey(&p, fx, fy, fz);
        assert_eq!(kn / OIT_DEPTH_SLOTS, kf / OIT_DEPTH_SLOTS, "同像素同 tile");
        assert!(kn > kf, "反键律:近者键必大(升序 = far→near painter's)");
        // 红臂(params[67]=1):翻转 ⇒ 近者键小。
        let mut pr = p.clone();
        pr[67] = 1.0;
        assert!(
            oit_tilekey(&pr, nx, ny, nz) < oit_tilekey(&pr, fx, fy, fz),
            "红臂键反转必须翻序"
        );
        // 屏外(x 负)/ 相机后(cw ≤ 0)= 溢出键。
        let overflow = 4 * OIT_DEPTH_SLOTS;
        let (ox, oy, oz) = world_at(-3.0, 8.0, 10.0);
        assert_eq!(oit_tilekey(&p, ox, oy, oz), overflow, "屏外必落溢出 tile");
        assert!(overflow < 16_777_216, "溢出键在 2^24 域内");
        // 键域上界论证数字(头注):tile_cnt = 4095 时溢出键 = 16 773 120。
        assert_eq!(OIT_TILE_CNT_MAX * OIT_DEPTH_SLOTS, 16_773_120);
    }

    /// ② 区间:排序键流按 tile 段分组;未触 tile 保持哨兵空区间。
    #[test]
    fn tile_ranges_grouping_and_sentinel() {
        // tile 0 两条目、tile 2 一条目、溢出 tile(4)一条目;tile 1/3 未触。
        let keys = vec![100u32, 200, 2 * 4096 + 7, 4 * 4096];
        let (s, e) = oit_tile_ranges(&keys, 5);
        assert_eq!((s[0], e[0]), (0, 2));
        assert_eq!((s[2], e[2]), (2, 3));
        assert_eq!((s[4], e[4]), (3, 4));
        assert_eq!((s[1], e[1]), (u32::MAX, 0), "未触 tile 保持哨兵(循环零次)");
        assert_eq!((s[3], e[3]), (u32::MAX, 0));
        // 空键流:全哨兵。
        let (s0, e0) = oit_tile_ranges(&[], 3);
        assert!(s0.iter().all(|&v| v == u32::MAX) && e0.iter().all(|&v| v == 0));
    }

    /// ③ 近远序见证(painter's):近远两粒子同像素,sorted 臂合成 = 手工
    /// 远先近后 over 链位级;红臂(键反转)合成必异且 = 手工翻序链。
    #[test]
    fn near_far_order_witness_and_red_arm_flip() {
        let p = fixture_params();
        let (nx, ny, nz) = world_at(8.0, 8.0, 5.0); // 近(age 小 ⇒ 偏白)
        let (fx, fy, fz) = world_at(8.0, 8.0, 5.2); // 远(age 大 ⇒ 偏红)
        let pos_x = [nx, fx];
        let pos_y = [ny, fy];
        let pos_z = [nz, fz];
        let age = [0.2f32, 1.4];
        let life = [10.0f32, 10.0];
        let streams = (&pos_x[..], &pos_y[..], &pos_z[..], &age[..], &life[..]);
        let depth = vec![1e9f32; 32 * 32]; // 远背景 ⇒ 硬拒不触发,soft = 1
        let base = vec![0.1f32; 32 * 32 * 3];
        let mut sorted = base.clone();
        oit_sorted_frame(&p, 2, streams, &depth, &mut sorted);
        // 手工 painter's:远(slot 1)先、近(slot 0)后。
        let px = 8usize + 8 * 32;
        let manual = |order: [usize; 2]| -> [f32; 3] {
            let mut rgb = [0.1f32; 3];
            for &s in &order {
                let (cr, cg, cb, fade) = oit_shade(age[s], life[s]);
                let alpha = fade * 1.0; // soft = 1(深背景)
                rgb[0] = rgb[0] * (1.0 - alpha) + cr * alpha;
                rgb[1] = rgb[1] * (1.0 - alpha) + cg * alpha;
                rgb[2] = rgb[2] * (1.0 - alpha) + cb * alpha;
            }
            rgb
        };
        let expect_ok = manual([1, 0]);
        let expect_flip = manual([0, 1]);
        for c in 0..3 {
            assert_eq!(
                sorted[px * 3 + c].to_bits(),
                expect_ok[c].to_bits(),
                "sorted 臂中心像素必须 = 远先近后 painter's 链(通道 {c})"
            );
        }
        assert_ne!(
            expect_ok.map(f32::to_bits),
            expect_flip.map(f32::to_bits),
            "夹具有效性:两序合成必须可分辨"
        );
        // 红臂:键反转 ⇒ 合成 = 翻序链(检出面)。
        let mut pr = p.clone();
        pr[67] = 1.0;
        let mut red = base.clone();
        oit_sorted_frame(&pr, 2, streams, &depth, &mut red);
        for c in 0..3 {
            assert_eq!(
                red[px * 3 + c].to_bits(),
                expect_flip[c].to_bits(),
                "红臂必须 = 翻序链(近先远后;通道 {c})"
            );
        }
    }

    /// ④ 饱和语义:delta 加前 clamp 到 65535 + 饱和计数;结构性防回绕
    /// (cap 上界 × DELTA_MAX < u32::MAX)。
    #[test]
    fn wboit_saturation_clamp_and_no_overflow() {
        let (d, s) = wboit_quantize(1e9);
        assert_eq!(d, OIT_DELTA_MAX, "越预算加数必须 clamp 到 65535");
        assert!(s, "clamp 触发必须计饱和事件");
        let (d2, s2) = wboit_quantize(1.0);
        assert_eq!(d2, 4096, "Q12:1.0 → 4096(floor)");
        assert!(!s2);
        let (d3, s3) = wboit_quantize(-1.0);
        assert_eq!(d3, 0, "负域 floor 后钳 0");
        assert!(!s3);
        // 结构性防回绕:2^16 项 × 65535 = 2^32 − 2^16 < u32::MAX。
        let total: u64 = (OIT_WBOIT_CAP_MAX as u64) * u64::from(OIT_DELTA_MAX);
        assert!(total <= u64::from(u32::MAX), "cap·DELTA_MAX 必须不越 u32 顶");
        assert_eq!(total, (1u64 << 32) - (1u64 << 16));
    }

    /// ⑤ WBOIT 排列不变性 + 双跑位级:整数累加可交换 ⇒ 粒子序无关;
    /// sorted 全链双跑位级。
    #[test]
    fn wboit_permutation_invariant_and_double_run_bitexact() {
        let p = fixture_params();
        let n = 6usize;
        let mut pos_x = Vec::new();
        let mut pos_y = Vec::new();
        let mut pos_z = Vec::new();
        let mut age = Vec::new();
        let mut life = Vec::new();
        for s in 0..n {
            let (x, y, z) = world_at(6.0 + s as f32 * 0.7, 9.0, 4.0 + s as f32);
            pos_x.push(x);
            pos_y.push(y);
            pos_z.push(z);
            age.push(0.1 + s as f32 * 0.2);
            life.push(5.0);
        }
        let depth = vec![1e9f32; 32 * 32];
        let base = vec![0.05f32; 32 * 32 * 3];
        let streams = (&pos_x[..], &pos_y[..], &pos_z[..], &age[..], &life[..]);
        let mut c1 = base.clone();
        let (acc1, sat1) = wboit_frame(&p, n, streams, &depth, &mut c1);
        // 逆序排列同粒子集:acc 必须位级等(整数加法可交换)。
        let rx: Vec<f32> = pos_x.iter().rev().copied().collect();
        let ry: Vec<f32> = pos_y.iter().rev().copied().collect();
        let rz: Vec<f32> = pos_z.iter().rev().copied().collect();
        let ra: Vec<f32> = age.iter().rev().copied().collect();
        let rl: Vec<f32> = life.iter().rev().copied().collect();
        let mut c2 = base.clone();
        let (acc2, sat2) =
            wboit_frame(&p, n, (&rx, &ry, &rz, &ra, &rl), &depth, &mut c2);
        assert_eq!(acc1, acc2, "定点累加必须与粒子到达序无关(可交换)");
        assert_eq!(sat1, sat2);
        assert_eq!(
            c1.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            c2.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "resolve 后帧必须位级等(顺序无关臂)"
        );
        assert!(acc1.iter().any(|&v| v != 0), "夹具有效性:累加非空");
        // sorted 全链双跑位级。
        let mut s1 = base.clone();
        let mut s2 = base.clone();
        let (k1, p1) = oit_sorted_frame(&p, n, streams, &depth, &mut s1);
        let (k2, p2) = oit_sorted_frame(&p, n, streams, &depth, &mut s2);
        assert_eq!(k1, k2);
        assert_eq!(p1, p2);
        assert_eq!(
            s1.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            s2.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "sorted 臂双跑必须位级等"
        );
        // 三臂判别夹具面:sorted ≠ wboit ≠ 基底。
        assert_ne!(
            s1.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            c1.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "双臂语义不同必须可分辨"
        );
        assert_ne!(
            s1.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            base.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
    }
}
