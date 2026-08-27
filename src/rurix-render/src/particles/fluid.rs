//! G35-7 流体统一物理 host 金标准(count-sort 空间哈希邻居 + XPBD/PBF 密度
//! 约束)——门 `g35.wave7.fluids`(RFC-0049 §4.10;契约 = mod.rs G35-P v1
//! 冻结头,事实源 = milestones/g35/G35_CONTRACT.md G-G35-7)。
//!
//! 与 device 六 kernel(`kernels/g35_hash_cellkey.rx` / `g35_hash_clear.rx` /
//! `g35_hash_cellrange.rx` / `g35_xpbd_density.rx` / `g35_xpbd_apply.rx` /
//! `g35_xpbd_velocity.rx`)**逐字同源**;排序面**只消费不修改** W1 sort 三
//! kernel([`super::primitives::sort_pairs_u24`] = 其 host 镜像,3-pass 稳定
//! LSD radix,键域 < 2^24)。
//!
//! ## 冻结协议(RFC-0049 §4.10 F14 + G35-7 brief 细则;协议改动走契约修订)
//!
//! - **网格域 v1 = 密集网格**:[`GRID`] = 64,[`GRID_CELLS`] = 64³ = 262144;
//!   `cell_id = (cz·64 + cy)·64 + cx < 2^24`(24 位键域 ✓,W1 排序直接消费);
//!   世界 AABB = `[origin, origin + 64·cell_size]³`(params 传 origin +
//!   cell_size;上界表达式冻结 `o + 64.0·cs`)。
//! - **cell 语义**:`cell = floor((p − origin)/cell_size)` 逐轴(f32 `.floor()`
//!   = 负坐标向负无穷取整,截断语义被否);越界 **f32 域 clamp 到 [0,63]**
//!   后才转 usize(负 f32 直转 usize 为未定义域,禁);越界/负 floor 事件
//!   host 侧如实登记([`CellKeyOut`] 计数器,kernel 不计数——登记语义归
//!   host 金标准)。
//! - **邻居管线**(每帧):① [`hash_cellkey_step`](= g35_hash_cellkey.rx):
//!   PBF 预测步(半隐式 Euler,运算序逐字冻结 = core.rs::sim_step 同律:
//!   `prev = pos; vy += g·dt; px += vx·dt; py += vy·dt〔消费更新后 vy〕;
//!   pz += vz·dt`)融合逐粒子 cell_key(u32)+ identity payload(粒子下标);
//!   ② W1 sort 3-pass 稳定排序 (cell_key, particle_index) → (sorted_keys,
//!   sorted_idx)(稳定序 = 确定序);③ [`hash_clear`](= g35_hash_clear.rx,
//!   独立 clear kernel 承载冻结:cell_start 清 [`CELL_START_EMPTY`] =
//!   0xFFFFFFFF,cell_end 清 0)+ [`hash_cellrange`](= g35_hash_cellrange.rx):
//!   逐 sorted 位置 i 单写者边界检测(`i==0 ∨ keys[i]≠keys[i−1]` ⇒
//!   `cell_start[keys[i]]=i`;`i==n−1 ∨ keys[i]≠keys[i+1]` ⇒
//!   `cell_end[keys[i]]=i+1`)——单写者 = 确定;空 cell 语义 start=0xFFFFFFFF
//!   > end=0 ⇒ 区间循环零次直落。
//! - **核函数(系数公式冻结;数值经 [`poly6_coef`]/[`spiky_grad_coef`] host
//!   单源程序产、经 params 传 device,禁 kernel 内重算)**:
//!   poly6 `W(r,h) = 315/(64π h⁹)·(h²−r²)³`(r<h);spiky 梯度
//!   `∇W = −45/(π h⁶)·(h−r)²·r̂`(r̂ = (pᵢ−pⱼ)/r;r=0 时梯度项跳过);
//!   `h = cell_size` 冻结相等,邻域 = 27 cell 固定序(oz→oy→ox 升序,cell 内
//!   = sorted 区间升序;cell 中心 = 本次调用当前位置的 cell 逐次重算,区间
//!   结构 = 帧首预测位置构建帧内冻结)。
//! - **XPBD/PBF 密度约束**(gather-only 零原子;[`ITER`] = 3 固定迭代冻结,
//!   禁自适应早停):[`xpbd_density_step`](= g35_xpbd_density.rx):
//!   `ρᵢ = Σⱼ m·W`(含自项 j=i 的 W(0));`Cᵢ = ρᵢ/ρ0 − 1`(仅 C>0 约束);
//!   `λᵢ = −Cᵢ/((|Σⱼ∇W|² + Σⱼ|∇W|²)/ρ0² + ε)`(Σ|∇W|² 读作 k∈{i}∪N(i) 全
//!   梯度和 = Macklin–Müller 2013 式(11) 全式;[`XPBD_EPS`] ε=100 冻结,
//!   kernel/host 字面双源同值);[`xpbd_apply_step`](= g35_xpbd_apply.rx):
//!   `Δpᵢ = (1/ρ0)Σⱼ(λᵢ+λⱼ)·∇W`(同序 gather),
//!   `pos_out = clamp(pos_in + Δp, origin, origin + 64·cs)` 逐轴
//!   (`.max(lo).min(hi)` 序冻结)——**Jacobi ping-pong**(读上迭代写本迭代,
//!   禁 Gauss-Seidel 原子竞写);迭代 3 次 = 交替 density/apply;
//!   [`xpbd_velocity_step`](= g35_xpbd_velocity.rx,**独立第六 kernel 承载
//!   冻结**——不并入 apply 末迭代):帧末 `vel = (pos − pos_prev)/dt` 逐轴 +
//!   边界速度置零分量(`pos ≤ lo ∨ pos ≥ hi ⇒ v轴 = 0`,冻结式)。
//! - **确定性**:全 gather 无原子无 shared;邻居遍历序 = cell 序 × 段内
//!   sorted 序(W1 稳定排序保证);整数面(cell_key/sorted_keys/sorted_idx/
//!   cell_start/cell_end)device↔host **零容差位级**;f32 面(pos/vel/ρ/λ)
//!   标定容差(g35_budget.json `g35.fluids.parity_p100`,threshold =
//!   measured×2.0 程序产禁手写);device 双跑一律位级。
//! - **MPM 评估窗**:不实现(G2P/P2G 散射需原子或图着色,与确定性协议冲突
//!   待裁——RFC-0049 §4.10 登记字面;evidence notes 引用)。
//!
//! host 独立朴素 O(n²) 参考([`naive_neighbor_sets`]/[`naive_density`])与
//! 分段哈希分解互核,防「同一错误两处照抄」(scan.rs/primitives.rs 同律)。

use super::primitives;

/// v1 密集网格每轴 cell 数(冻结;cell_id < 64³ = 262144 < 2^24 键域)。
pub const GRID: usize = 64;
/// 总 cell 数 = GRID³。
pub const GRID_CELLS: usize = GRID * GRID * GRID;
/// XPBD 固定迭代次数(冻结;禁自适应早停——确定性协议)。
pub const ITER: usize = 3;
/// PBF 松弛 ε(冻结字面;kernel 内 `+ 100.0` 字面同值,禁经 params 篡改)。
pub const XPBD_EPS: f32 = 100.0;
/// cell_start 空哨兵(帧首清值;空 cell 区间 [0xFFFFFFFF, 0) 循环零次直落)。
pub const CELL_START_EMPTY: u32 = 4_294_967_295;
/// cell_end 空哨兵(帧首清值)。
pub const CELL_END_EMPTY: u32 = 0;

/// 流体参数面(世界 AABB = [origin, origin + GRID·cell_size]³;
/// h = cell_size 冻结相等)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidParams {
    /// 世界 AABB 原点(kernel params 传)。
    pub origin: [f32; 3],
    /// cell 边长 = SPH 平滑半径 h(冻结相等)。
    pub cell_size: f32,
    /// 静息密度 ρ0(PBF 约束目标;RED 臂篡改对象)。
    pub rho0: f32,
    /// 粒子质量 m(ρ = Σ m·W)。
    pub mass: f32,
    /// 帧步长(秒)。
    pub dt: f32,
    /// 重力 y 分量(预测步唯一外力;g35_sim.rx 同律)。
    pub gravity_y: f32,
}

/// 冻结默认参数(dam-break 夹具域:世界 [0,12.8]³、h=0.2、ρ0=1000、m=1、
/// dt=1/60、g=−9.8;间距 0.4h 压缩初态下 ρ≈m/间距³≈1953≈2ρ0,收敛方向咬合)。
pub fn default_params() -> FluidParams {
    FluidParams {
        origin: [0.0, 0.0, 0.0],
        cell_size: 0.2,
        rho0: 1000.0,
        mass: 1.0,
        dt: 1.0 / 60.0,
        gravity_y: -9.8,
    }
}

/// 流体粒子 SoA 状态(9 流;prev = 帧首预测前位置,velocity 步消费)。
#[derive(Clone, Debug)]
pub struct FluidState {
    /// 位置 x(预测/迭代修正后的当前值)。
    pub pos_x: Vec<f32>,
    /// 位置 y。
    pub pos_y: Vec<f32>,
    /// 位置 z。
    pub pos_z: Vec<f32>,
    /// 帧首位置 x(cellkey 步保存;velocity 步 (pos−prev)/dt 消费)。
    pub prev_x: Vec<f32>,
    /// 帧首位置 y。
    pub prev_y: Vec<f32>,
    /// 帧首位置 z。
    pub prev_z: Vec<f32>,
    /// 速度 x。
    pub vel_x: Vec<f32>,
    /// 速度 y。
    pub vel_y: Vec<f32>,
    /// 速度 z。
    pub vel_z: Vec<f32>,
}

impl FluidState {
    /// 粒子数(9 流等长)。
    pub fn n(&self) -> usize {
        self.pos_x.len()
    }
}

/// poly6 系数 = 315/(64π h⁹)(公式冻结;host 单源程序产,device 经 params
/// 只读消费——禁 kernel 内重算,消除超越函数/求幂的双源漂移风险)。
pub fn poly6_coef(h: f32) -> f32 {
    let h2 = h * h;
    let h4 = h2 * h2;
    let h9 = h4 * h4 * h;
    315.0 / (64.0 * std::f32::consts::PI * h9)
}

/// spiky 梯度系数 = −45/(π h⁶)(公式冻结;含负号——梯度向量 =
/// coef·(h−r)²·r̂,host 单源程序产经 params 传 device)。
pub fn spiky_grad_coef(h: f32) -> f32 {
    let h2 = h * h;
    let h6 = h2 * h2 * h2;
    0.0 - 45.0 / (std::f32::consts::PI * h6)
}

/// cell 轴向未钳 floor(= kernel `((p − o)/cs).floor()` 逐字;f32 `.floor()`
/// 对负值向负无穷取整——floor-division 语义显式,截断语义被否)。
pub fn cell_axis_floor(p: f32, o: f32, cs: f32) -> f32 {
    ((p - o) / cs).floor()
}

/// cell 轴向坐标(= kernel 逐字:floor → f32 域 clamp [0,63] → usize;
/// 越界 clamp 到边界 cell 如实登记语义,负 f32 直转 usize 为未定义域禁)。
pub fn cell_axis(p: f32, o: f32, cs: f32) -> usize {
    let mut f = cell_axis_floor(p, o, cs);
    if f < 0.0 {
        f = 0.0;
    }
    if f > 63.0 {
        f = 63.0;
    }
    f as usize
}

/// 逐粒子 cell_key(= kernel 键式逐字:`((cz·64 + cy)·64 + cx) as u32`)。
pub fn position_cell_key(px: f32, py: f32, pz: f32, p: &FluidParams) -> u32 {
    let cx = cell_axis(px, p.origin[0], p.cell_size);
    let cy = cell_axis(py, p.origin[1], p.cell_size);
    let cz = cell_axis(pz, p.origin[2], p.cell_size);
    ((cz * 64 + cy) * 64 + cx) as u32
}

/// 世界 AABB 上界(冻结表达式 `o + 64.0·cs` 逐轴;kernel 同式)。
pub fn world_max(p: &FluidParams) -> [f32; 3] {
    [
        p.origin[0] + 64.0 * p.cell_size,
        p.origin[1] + 64.0 * p.cell_size,
        p.origin[2] + 64.0 * p.cell_size,
    ]
}

/// cellkey 步输出(keys/payload = 排序消费面;计数器 = host 登记语义,
/// kernel 不计数——device 等价性由 cell_key 整数流零容差位级承载)。
#[derive(Clone, Debug)]
pub struct CellKeyOut {
    /// 逐粒子 cell_key(u32 < 262144 < 2^24)。
    pub keys: Vec<u32>,
    /// identity payload = 粒子下标(稳定排序 ⇒ sorted_idx 确定序)。
    pub payload: Vec<u32>,
    /// 负 floor 事件数(逐粒子逐轴 floor < 0 计数——负坐标语义见证)。
    pub negative_floor_events: u64,
    /// clamp 事件数(逐粒子逐轴 floor 越 [0,63] 计数——越界登记语义)。
    pub clamp_events: u64,
}

/// 帧序第 1 步(= kernels/g35_hash_cellkey.rx 逐字同源):PBF 预测步
/// (prev 保存 + 半隐式 Euler,运算序冻结 vy→px→py〔新 vy〕→pz)融合
/// cell_key + identity payload。
pub fn hash_cellkey_step(st: &mut FluidState, p: &FluidParams) -> CellKeyOut {
    let n = st.n();
    let mut keys = vec![0u32; n];
    let mut payload = vec![0u32; n];
    let mut negative_floor_events = 0u64;
    let mut clamp_events = 0u64;
    for i in 0..n {
        // 运算序逐字冻结(kernel 同序):prev 保存 → vy → px → py(新 vy)→ pz。
        st.prev_x[i] = st.pos_x[i];
        st.prev_y[i] = st.pos_y[i];
        st.prev_z[i] = st.pos_z[i];
        st.vel_y[i] = st.vel_y[i] + p.gravity_y * p.dt;
        st.pos_x[i] = st.pos_x[i] + st.vel_x[i] * p.dt;
        st.pos_y[i] = st.pos_y[i] + st.vel_y[i] * p.dt;
        st.pos_z[i] = st.pos_z[i] + st.vel_z[i] * p.dt;
        // 登记计数(host 侧语义;cell_axis 与 kernel 逐字同式)。
        for (v, o) in [
            (st.pos_x[i], p.origin[0]),
            (st.pos_y[i], p.origin[1]),
            (st.pos_z[i], p.origin[2]),
        ] {
            let f = cell_axis_floor(v, o, p.cell_size);
            if f < 0.0 {
                negative_floor_events += 1;
            }
            if f < 0.0 || f > 63.0 {
                clamp_events += 1;
            }
        }
        keys[i] = position_cell_key(st.pos_x[i], st.pos_y[i], st.pos_z[i], p);
        payload[i] = i as u32;
    }
    CellKeyOut {
        keys,
        payload,
        negative_floor_events,
        clamp_events,
    }
}

/// 帧序第 3a 步(= kernels/g35_hash_clear.rx 逐字同源语义):cell_start 清
/// 0xFFFFFFFF、cell_end 清 0(空 cell 区间循环零次直落)。
pub fn hash_clear() -> (Vec<u32>, Vec<u32>) {
    (
        vec![CELL_START_EMPTY; GRID_CELLS],
        vec![CELL_END_EMPTY; GRID_CELLS],
    )
}

/// 帧序第 3b 步(= kernels/g35_hash_cellrange.rx 逐字同源):逐 sorted 位置
/// 单写者边界检测(每 cell 的 start/end 各恰一写者 = 确定,零原子)。
pub fn hash_cellrange(sorted_keys: &[u32], cell_start: &mut [u32], cell_end: &mut [u32]) {
    let n = sorted_keys.len();
    for i in 0..n {
        let k = sorted_keys[i] as usize;
        if i == 0 {
            cell_start[k] = i as u32;
        } else if sorted_keys[i] != sorted_keys[i - 1] {
            cell_start[k] = i as u32;
        }
        if i == n - 1 {
            cell_end[k] = (i + 1) as u32;
        } else if sorted_keys[i] != sorted_keys[i + 1] {
            cell_end[k] = (i + 1) as u32;
        }
    }
}

/// XPBD 迭代第 1 半步(= kernels/g35_xpbd_density.rx 逐字同源):27-cell
/// 固定序 gather 产 ρ + λ(公式冻结面见模块头注;乘加序逐字 = kernel)。
#[allow(clippy::too_many_arguments)]
pub fn xpbd_density_step(
    pos_x: &[f32],
    pos_y: &[f32],
    pos_z: &[f32],
    sorted_idx: &[u32],
    cell_start: &[u32],
    cell_end: &[u32],
    p: &FluidParams,
) -> (Vec<f32>, Vec<f32>) {
    let n = pos_x.len();
    let ox = p.origin[0];
    let oy = p.origin[1];
    let oz = p.origin[2];
    let h = p.cell_size;
    let rho0 = p.rho0;
    let m = p.mass;
    let poly6 = poly6_coef(h);
    let spiky = spiky_grad_coef(h);
    let h2 = h * h;
    let mut rho = vec![0.0f32; n];
    let mut lambda = vec![0.0f32; n];
    for i in 0..n {
        let pix = pos_x[i];
        let piy = pos_y[i];
        let piz = pos_z[i];
        let cx = cell_axis(pix, ox, h);
        let cy = cell_axis(piy, oy, h);
        let cz = cell_axis(piz, oz, h);
        let mut rho_acc = 0.0f32;
        let mut gx = 0.0f32;
        let mut gy = 0.0f32;
        let mut gz = 0.0f32;
        let mut g2 = 0.0f32;
        // 27-cell 固定序 gather(oz→oy→ox 升序;kernel while 循环逐字同序)。
        for oz_c in 0..3usize {
            let zz = cz + oz_c;
            if zz >= 1 && zz <= 64 {
                let ncz = zz - 1;
                for oy_c in 0..3usize {
                    let yy = cy + oy_c;
                    if yy >= 1 && yy <= 64 {
                        let ncy = yy - 1;
                        for ox_c in 0..3usize {
                            let xx = cx + ox_c;
                            if xx >= 1 && xx <= 64 {
                                let ncx = xx - 1;
                                let c = (ncz * 64 + ncy) * 64 + ncx;
                                let mut s = cell_start[c] as usize;
                                let e = cell_end[c] as usize;
                                while s < e {
                                    let j = sorted_idx[s] as usize;
                                    let rx = pix - pos_x[j];
                                    let ry = piy - pos_y[j];
                                    let rz = piz - pos_z[j];
                                    let r2 = rx * rx + ry * ry + rz * rz;
                                    if r2 < h2 {
                                        let d2 = h2 - r2;
                                        rho_acc = rho_acc + m * (poly6 * (d2 * (d2 * d2)));
                                        if r2 > 0.0 {
                                            let r = r2.sqrt();
                                            let hr = h - r;
                                            let sgc = spiky * (hr * hr) / r;
                                            let wx = sgc * rx;
                                            let wy = sgc * ry;
                                            let wz = sgc * rz;
                                            gx = gx + wx;
                                            gy = gy + wy;
                                            gz = gz + wz;
                                            g2 = g2 + (wx * wx + wy * wy + wz * wz);
                                        }
                                    }
                                    s += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        rho[i] = rho_acc;
        let ci = rho_acc / rho0 - 1.0;
        if ci > 0.0 {
            let denom = (gx * gx + gy * gy + gz * gz + g2) / (rho0 * rho0) + 100.0;
            lambda[i] = 0.0 - ci / denom;
        } else {
            lambda[i] = 0.0;
        }
    }
    (rho, lambda)
}

/// XPBD 迭代第 2 半步(= kernels/g35_xpbd_apply.rx 逐字同源):同序 gather
/// 邻居 λ 产 Δp,pos_out = clamp(pos_in + Δp/ρ0 语义, AABB)——Jacobi
/// ping-pong(读 pos_in 写 pos_out,禁原地)。
#[allow(clippy::too_many_arguments)]
pub fn xpbd_apply_step(
    pos_x: &[f32],
    pos_y: &[f32],
    pos_z: &[f32],
    lambda: &[f32],
    sorted_idx: &[u32],
    cell_start: &[u32],
    cell_end: &[u32],
    p: &FluidParams,
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let n = pos_x.len();
    let ox = p.origin[0];
    let oy = p.origin[1];
    let oz = p.origin[2];
    let h = p.cell_size;
    let rho0 = p.rho0;
    let spiky = spiky_grad_coef(h);
    let h2 = h * h;
    let bx1 = ox + 64.0 * h;
    let by1 = oy + 64.0 * h;
    let bz1 = oz + 64.0 * h;
    let mut out_x = vec![0.0f32; n];
    let mut out_y = vec![0.0f32; n];
    let mut out_z = vec![0.0f32; n];
    for i in 0..n {
        let pix = pos_x[i];
        let piy = pos_y[i];
        let piz = pos_z[i];
        let li = lambda[i];
        let cx = cell_axis(pix, ox, h);
        let cy = cell_axis(piy, oy, h);
        let cz = cell_axis(piz, oz, h);
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        let mut dz = 0.0f32;
        for oz_c in 0..3usize {
            let zz = cz + oz_c;
            if zz >= 1 && zz <= 64 {
                let ncz = zz - 1;
                for oy_c in 0..3usize {
                    let yy = cy + oy_c;
                    if yy >= 1 && yy <= 64 {
                        let ncy = yy - 1;
                        for ox_c in 0..3usize {
                            let xx = cx + ox_c;
                            if xx >= 1 && xx <= 64 {
                                let ncx = xx - 1;
                                let c = (ncz * 64 + ncy) * 64 + ncx;
                                let mut s = cell_start[c] as usize;
                                let e = cell_end[c] as usize;
                                while s < e {
                                    let j = sorted_idx[s] as usize;
                                    let rx = pix - pos_x[j];
                                    let ry = piy - pos_y[j];
                                    let rz = piz - pos_z[j];
                                    let r2 = rx * rx + ry * ry + rz * rz;
                                    if r2 < h2 && r2 > 0.0 {
                                        let r = r2.sqrt();
                                        let hr = h - r;
                                        let sgc = spiky * (hr * hr) / r;
                                        let fs = (li + lambda[j]) * sgc;
                                        dx = dx + fs * rx;
                                        dy = dy + fs * ry;
                                        dz = dz + fs * rz;
                                    }
                                    s += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        // Δp = (1/ρ0)Σ(λᵢ+λⱼ)∇W;clamp 序冻结 .max(lo).min(hi)。
        let px2 = pix + dx / rho0;
        let py2 = piy + dy / rho0;
        let pz2 = piz + dz / rho0;
        out_x[i] = px2.max(ox).min(bx1);
        out_y[i] = py2.max(oy).min(by1);
        out_z[i] = pz2.max(oz).min(bz1);
    }
    (out_x, out_y, out_z)
}

/// 帧末速度步(= kernels/g35_xpbd_velocity.rx 逐字同源;独立第六 kernel
/// 承载冻结):vel = (pos − prev)/dt 逐轴 + 边界速度置零分量
/// (pos ≤ lo ∨ pos ≥ hi ⇒ 该轴 v=0,判序冻结 x-lo/x-hi/y-lo/y-hi/z-lo/z-hi)。
pub fn xpbd_velocity_step(st: &mut FluidState, p: &FluidParams) {
    let n = st.n();
    let ox = p.origin[0];
    let oy = p.origin[1];
    let oz = p.origin[2];
    let h = p.cell_size;
    let dt = p.dt;
    let bx1 = ox + 64.0 * h;
    let by1 = oy + 64.0 * h;
    let bz1 = oz + 64.0 * h;
    for i in 0..n {
        let mut vx = (st.pos_x[i] - st.prev_x[i]) / dt;
        let mut vy = (st.pos_y[i] - st.prev_y[i]) / dt;
        let mut vz = (st.pos_z[i] - st.prev_z[i]) / dt;
        if st.pos_x[i] <= ox {
            vx = 0.0;
        }
        if st.pos_x[i] >= bx1 {
            vx = 0.0;
        }
        if st.pos_y[i] <= oy {
            vy = 0.0;
        }
        if st.pos_y[i] >= by1 {
            vy = 0.0;
        }
        if st.pos_z[i] <= oz {
            vz = 0.0;
        }
        if st.pos_z[i] >= bz1 {
            vz = 0.0;
        }
        st.vel_x[i] = vx;
        st.vel_y[i] = vy;
        st.vel_z[i] = vz;
    }
}

/// 单帧全链轨迹(device probe 对拍面:整数流零容差 + f32 流标定容差)。
#[derive(Clone, Debug)]
pub struct FluidFrameTrace {
    /// 未排序 cell_key(整数流)。
    pub keys: Vec<u32>,
    /// identity payload(排序输入)。
    pub payload: Vec<u32>,
    /// 排序后键(整数流)。
    pub sorted_keys: Vec<u32>,
    /// 排序后粒子下标(整数流;cell-major 确定序)。
    pub sorted_idx: Vec<u32>,
    /// cell 区间起(整数流;空 cell = 0xFFFFFFFF)。
    pub cell_start: Vec<u32>,
    /// cell 区间止(整数流;空 cell = 0)。
    pub cell_end: Vec<u32>,
    /// 末迭代密度 ρ(f32 流)。
    pub rho: Vec<f32>,
    /// 末迭代乘子 λ(f32 流)。
    pub lambda: Vec<f32>,
    /// 负 floor 事件数(本帧;host 登记语义)。
    pub negative_floor_events: u64,
    /// clamp 事件数(本帧)。
    pub clamp_events: u64,
}

/// 单帧全链(= device 链 cellkey → W1 sort 3-pass → clear+cellrange →
/// [density→apply]×ITER → velocity 的 host 单源;排序消费
/// [`primitives::sort_pairs_u24`] 不修改)。
pub fn fluid_frame(st: &mut FluidState, p: &FluidParams) -> FluidFrameTrace {
    let ck = hash_cellkey_step(st, p);
    let (sorted_keys, sorted_idx) = primitives::sort_pairs_u24(&ck.keys, &ck.payload);
    let (mut cell_start, mut cell_end) = hash_clear();
    hash_cellrange(&sorted_keys, &mut cell_start, &mut cell_end);
    let n = st.n();
    let mut rho = vec![0.0f32; n];
    let mut lambda = vec![0.0f32; n];
    for _ in 0..ITER {
        let (r, l) = xpbd_density_step(
            &st.pos_x,
            &st.pos_y,
            &st.pos_z,
            &sorted_idx,
            &cell_start,
            &cell_end,
            p,
        );
        rho = r;
        lambda = l;
        let (nx, ny, nz) = xpbd_apply_step(
            &st.pos_x,
            &st.pos_y,
            &st.pos_z,
            &lambda,
            &sorted_idx,
            &cell_start,
            &cell_end,
            p,
        );
        st.pos_x = nx;
        st.pos_y = ny;
        st.pos_z = nz;
    }
    xpbd_velocity_step(st, p);
    FluidFrameTrace {
        keys: ck.keys,
        payload: ck.payload,
        sorted_keys,
        sorted_idx,
        cell_start,
        cell_end,
        rho,
        lambda,
        negative_floor_events: ck.negative_floor_events,
        clamp_events: ck.clamp_events,
    }
}

/// 密度误差均值 mean |ρ/ρ0 − 1|(f64 登记口径;evidence 面 measured 消费,
/// 非位级对拍对象)。
pub fn mean_density_error(rho: &[f32], rho0: f32) -> f64 {
    if rho.is_empty() {
        return 0.0;
    }
    let sum: f64 = rho
        .iter()
        .map(|&r| (f64::from(r) / f64::from(rho0) - 1.0).abs())
        .sum();
    sum / rho.len() as f64
}

/// 正约束违反均值 mean(max(ρ/ρ0 − 1, 0))(f64 登记口径)——PBF 仅约束
/// C>0(压缩),ρ<ρ0 为无约束自由表面;求解器消解对象即本量,收敛方向性
/// 断言消费本指标(|ρ/ρ0−1| 会因自由膨胀回升,非 Lyapunov 量)。
pub fn mean_positive_constraint(rho: &[f32], rho0: f32) -> f64 {
    if rho.is_empty() {
        return 0.0;
    }
    let sum: f64 = rho
        .iter()
        .map(|&r| (f64::from(r) / f64::from(rho0) - 1.0).max(0.0))
        .sum();
    sum / rho.len() as f64
}

/// dam-break 夹具(冻结:side = 最小 s 使 s³ ≥ n;粒子 i → (iz,iy,ix)
/// 字典序网格摆放;间距 = 0.4·h 压缩初态;块基 = origin + (2h, 3h, 2h);
/// 扰动 = (r·2−1)·0.01 逐轴,Pcg32(seed, 54) 单源逐粒子 x/y/z 三抽;
/// vel = 0,prev = pos)。
pub fn dam_break_fixture(n: usize, seed: u64, p: &FluidParams) -> FluidState {
    assert!(n > 0, "夹具粒子数必须 ≥ 1");
    let mut side = 1usize;
    while side * side * side < n {
        side += 1;
    }
    let spacing = 0.4 * p.cell_size;
    let base = [
        p.origin[0] + 2.0 * p.cell_size,
        p.origin[1] + 3.0 * p.cell_size,
        p.origin[2] + 2.0 * p.cell_size,
    ];
    let mut rng = super::Pcg32::new(seed, 54);
    let mut pos_x = vec![0.0f32; n];
    let mut pos_y = vec![0.0f32; n];
    let mut pos_z = vec![0.0f32; n];
    for i in 0..n {
        let iz = i / (side * side);
        let iy = (i / side) % side;
        let ix = i % side;
        let rx = rng.next_f32();
        let ry = rng.next_f32();
        let rz = rng.next_f32();
        pos_x[i] = base[0] + ix as f32 * spacing + (rx * 2.0 - 1.0) * 0.01;
        pos_y[i] = base[1] + iy as f32 * spacing + (ry * 2.0 - 1.0) * 0.01;
        pos_z[i] = base[2] + iz as f32 * spacing + (rz * 2.0 - 1.0) * 0.01;
    }
    FluidState {
        prev_x: pos_x.clone(),
        prev_y: pos_y.clone(),
        prev_z: pos_z.clone(),
        pos_x,
        pos_y,
        pos_z,
        vel_x: vec![0.0; n],
        vel_y: vec![0.0; n],
        vel_z: vec![0.0; n],
    }
}

// ---------------------------------------------------------------------------
// 独立朴素 O(n²) 参考(互核用,防「同一错误两处照抄」;小 n 域)
// ---------------------------------------------------------------------------

/// 朴素邻居集(逐对全扫;j ≠ i 且 r² < h²;j 升序天然有序)。
pub fn naive_neighbor_sets(pos_x: &[f32], pos_y: &[f32], pos_z: &[f32], h: f32) -> Vec<Vec<u32>> {
    let n = pos_x.len();
    let h2 = h * h;
    let mut sets = vec![Vec::new(); n];
    for i in 0..n {
        for j in 0..n {
            if j != i {
                let rx = pos_x[i] - pos_x[j];
                let ry = pos_y[i] - pos_y[j];
                let rz = pos_z[i] - pos_z[j];
                if rx * rx + ry * ry + rz * rz < h2 {
                    sets[i].push(j as u32);
                }
            }
        }
    }
    sets
}

/// 网格哈希邻居集(生产管线路径:cell_key → 稳定排序 → cellrange → 27-cell
/// 固定序 gather;收集 j ≠ i 且 r² < h²,升序排序后返回——与朴素集相等 =
/// r<h 全捕获判据)。
pub fn grid_neighbor_sets(
    pos_x: &[f32],
    pos_y: &[f32],
    pos_z: &[f32],
    p: &FluidParams,
) -> Vec<Vec<u32>> {
    let n = pos_x.len();
    let h = p.cell_size;
    let h2 = h * h;
    let keys: Vec<u32> = (0..n)
        .map(|i| position_cell_key(pos_x[i], pos_y[i], pos_z[i], p))
        .collect();
    let payload: Vec<u32> = (0..n as u32).collect();
    let (sorted_keys, sorted_idx) = primitives::sort_pairs_u24(&keys, &payload);
    let (mut cell_start, mut cell_end) = hash_clear();
    hash_cellrange(&sorted_keys, &mut cell_start, &mut cell_end);
    let mut sets = vec![Vec::new(); n];
    for i in 0..n {
        let cx = cell_axis(pos_x[i], p.origin[0], h);
        let cy = cell_axis(pos_y[i], p.origin[1], h);
        let cz = cell_axis(pos_z[i], p.origin[2], h);
        for oz_c in 0..3usize {
            let zz = cz + oz_c;
            if zz >= 1 && zz <= 64 {
                let ncz = zz - 1;
                for oy_c in 0..3usize {
                    let yy = cy + oy_c;
                    if yy >= 1 && yy <= 64 {
                        let ncy = yy - 1;
                        for ox_c in 0..3usize {
                            let xx = cx + ox_c;
                            if xx >= 1 && xx <= 64 {
                                let ncx = xx - 1;
                                let c = (ncz * 64 + ncy) * 64 + ncx;
                                let mut s = cell_start[c] as usize;
                                let e = cell_end[c] as usize;
                                while s < e {
                                    let j = sorted_idx[s] as usize;
                                    if j != i {
                                        let rx = pos_x[i] - pos_x[j];
                                        let ry = pos_y[i] - pos_y[j];
                                        let rz = pos_z[i] - pos_z[j];
                                        if rx * rx + ry * ry + rz * rz < h2 {
                                            sets[i].push(j as u32);
                                        }
                                    }
                                    s += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        sets[i].sort_unstable();
    }
    sets
}

/// 朴素密度(逐对全扫,j 升序求和序;含自项 j=i 的 W(0)——与网格 gather
/// 求和序不同 ⇒ f32 浮点非结合,互核走相对容差非位级)。
pub fn naive_density(pos_x: &[f32], pos_y: &[f32], pos_z: &[f32], p: &FluidParams) -> Vec<f32> {
    let n = pos_x.len();
    let h = p.cell_size;
    let h2 = h * h;
    let poly6 = poly6_coef(h);
    let m = p.mass;
    let mut rho = vec![0.0f32; n];
    for i in 0..n {
        let mut acc = 0.0f32;
        for j in 0..n {
            let rx = pos_x[i] - pos_x[j];
            let ry = pos_y[i] - pos_y[j];
            let rz = pos_z[i] - pos_z[j];
            let r2 = rx * rx + ry * ry + rz * rz;
            if r2 < h2 {
                let d2 = h2 - r2;
                acc = acc + m * (poly6 * (d2 * (d2 * d2)));
            }
        }
        rho[i] = acc;
    }
    rho
}

#[cfg(test)]
mod tests {
    use super::super::Pcg32;
    use super::*;

    /// 散布夹具:n−2 粒子落 [origin+0.3, origin+2.1]³ 稠密子域(咬合前提:
    /// 邻居对数 > 0),末两粒子钉死近角/远角边界 cell(边界 gather 覆盖)。
    fn scatter_fixture(n: usize, seed: u64, p: &FluidParams) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let mut rng = Pcg32::new(seed, 54);
        let mut px = vec![0.0f32; n];
        let mut py = vec![0.0f32; n];
        let mut pz = vec![0.0f32; n];
        for i in 0..n {
            px[i] = p.origin[0] + 0.3 + rng.next_f32() * 1.8;
            py[i] = p.origin[1] + 0.3 + rng.next_f32() * 1.8;
            pz[i] = p.origin[2] + 0.3 + rng.next_f32() * 1.8;
        }
        let hi = world_max(p);
        px[n - 2] = p.origin[0] + 0.01;
        py[n - 2] = p.origin[1] + 0.01;
        pz[n - 2] = p.origin[2] + 0.01;
        px[n - 1] = hi[0] - 0.01;
        py[n - 1] = hi[1] - 0.01;
        pz[n - 1] = hi[2] - 0.01;
        (px, py, pz)
    }

    #[test]
    fn neighbor_sets_match_naive_reference() {
        let p = default_params();
        let (px, py, pz) = scatter_fixture(300, 9, &p);
        let grid = grid_neighbor_sets(&px, &py, &pz, &p);
        let naive = naive_neighbor_sets(&px, &py, &pz, p.cell_size);
        assert_eq!(grid, naive, "网格哈希邻居集 ≠ 朴素 O(n²) 邻居集");
        let pairs: usize = naive.iter().map(Vec::len).sum();
        assert!(pairs > 0, "夹具必须含邻居对(咬合前提;得 {pairs})");
    }

    #[test]
    fn grid_density_matches_naive_reference() {
        // 求和序不同(cell 序 vs 下标序)⇒ f32 非结合,互核走相对容差。
        let p = default_params();
        let (px, py, pz) = scatter_fixture(200, 11, &p);
        let keys: Vec<u32> = (0..px.len())
            .map(|i| position_cell_key(px[i], py[i], pz[i], &p))
            .collect();
        let payload: Vec<u32> = (0..px.len() as u32).collect();
        let (sk, si) = primitives::sort_pairs_u24(&keys, &payload);
        let (mut cs, mut ce) = hash_clear();
        hash_cellrange(&sk, &mut cs, &mut ce);
        let (rho, _) = xpbd_density_step(&px, &py, &pz, &si, &cs, &ce, &p);
        let naive = naive_density(&px, &py, &pz, &p);
        for i in 0..rho.len() {
            let tol = 1e-3 * naive[i].abs().max(1.0);
            assert!(
                (rho[i] - naive[i]).abs() <= tol,
                "i={i} 网格 ρ={} ≠ 朴素 ρ={}(容差 {tol})",
                rho[i],
                naive[i]
            );
        }
    }

    #[test]
    fn cellrange_matches_naive_counts() {
        let p = default_params();
        let (px, py, pz) = scatter_fixture(257, 13, &p);
        let n = px.len();
        let keys: Vec<u32> = (0..n)
            .map(|i| position_cell_key(px[i], py[i], pz[i], &p))
            .collect();
        let payload: Vec<u32> = (0..n as u32).collect();
        let (sk, _) = primitives::sort_pairs_u24(&keys, &payload);
        let (mut cs, mut ce) = hash_clear();
        hash_cellrange(&sk, &mut cs, &mut ce);
        // 朴素:逐 cell 计数 + 前缀(独立构造互核)。
        let mut count = vec![0u32; GRID_CELLS];
        for &k in &keys {
            count[k as usize] += 1;
        }
        let mut acc = 0u32;
        for c in 0..GRID_CELLS {
            if count[c] > 0 {
                assert_eq!(cs[c], acc, "cell {c} start 不符");
                assert_eq!(ce[c], acc + count[c], "cell {c} end 不符");
                acc += count[c];
            } else {
                assert_eq!(cs[c], CELL_START_EMPTY, "空 cell {c} start 非哨兵");
                assert_eq!(ce[c], CELL_END_EMPTY, "空 cell {c} end 非哨兵");
            }
        }
        assert_eq!(acc as usize, n);
    }

    #[test]
    fn density_converges_toward_rest_density() {
        // dam-break 压缩初态静置(g=0)数帧:正约束违反均值 mean(max(C,0))
        // 方向性下降(ρ 均值→ρ0 的约束面口径;仅 C>0 受约束,自由表面
        // ρ<ρ0 不入指标),不设死值(brief 判据字面)。
        let mut p = default_params();
        p.gravity_y = 0.0;
        let mut st = dam_break_fixture(512, 7, &p);
        let mut first = 0.0f64;
        let mut last = 0.0f64;
        for f in 0..8 {
            let tr = fluid_frame(&mut st, &p);
            let e = mean_positive_constraint(&tr.rho, p.rho0);
            assert!(e.is_finite(), "帧 {f} 约束违反非有限");
            if f == 0 {
                first = e;
            }
            last = e;
        }
        assert!(
            first > 0.05,
            "压缩初态正约束违反必须显著(咬合前提;得 {first})"
        );
        assert!(
            last < first,
            "正约束违反必须向 ρ0 方向收敛(first={first} last={last})"
        );
    }

    #[test]
    fn double_run_bitexact_host() {
        // 同夹具双跑全流位级(f32 以 to_bits 严格比较;host 确定性自证)。
        let p = default_params();
        let run = || {
            let mut st = dam_break_fixture(512, 42, &p);
            let mut bits: Vec<u32> = Vec::new();
            for _ in 0..4 {
                let tr = fluid_frame(&mut st, &p);
                bits.extend(tr.keys.iter());
                bits.extend(tr.sorted_keys.iter());
                bits.extend(tr.sorted_idx.iter());
                bits.extend(tr.cell_start.iter());
                bits.extend(tr.cell_end.iter());
                bits.extend(tr.rho.iter().map(|v| v.to_bits()));
                bits.extend(tr.lambda.iter().map(|v| v.to_bits()));
                bits.extend(st.pos_x.iter().map(|v| v.to_bits()));
                bits.extend(st.pos_y.iter().map(|v| v.to_bits()));
                bits.extend(st.pos_z.iter().map(|v| v.to_bits()));
                bits.extend(st.vel_x.iter().map(|v| v.to_bits()));
                bits.extend(st.vel_y.iter().map(|v| v.to_bits()));
                bits.extend(st.vel_z.iter().map(|v| v.to_bits()));
            }
            bits
        };
        assert_eq!(run(), run(), "host 双跑必须位级一致");
    }

    #[test]
    fn out_of_domain_clamp_semantics() {
        // 越界粒子 clamp 到边界 cell 如实登记(负向 → cell 0,正向 → cell 63)。
        let p = default_params(); // 世界 [0, 12.8]³
        assert_eq!(position_cell_key(1.0, 1.0, 1.0, &p), (5 * 64 + 5) * 64 + 5);
        assert_eq!(position_cell_key(-0.7, -0.7, -0.7, &p), 0, "负越界必落角 cell 0");
        assert_eq!(
            position_cell_key(13.5, 13.5, 13.5, &p),
            ((63 * 64 + 63) * 64 + 63) as u32,
            "正越界必落角 cell 262143"
        );
        // 全管线登记:越界粒子必在 cellrange 结构中可达(不静默丢)。
        let px = vec![1.0f32, -0.7, 13.5];
        let py = vec![1.0f32, -0.7, 13.5];
        let pz = vec![1.0f32, -0.7, 13.5];
        let keys: Vec<u32> = (0..3)
            .map(|i| position_cell_key(px[i], py[i], pz[i], &p))
            .collect();
        let (sk, si) = primitives::sort_pairs_u24(&keys, &[0u32, 1, 2]);
        let (mut cs, mut ce) = hash_clear();
        hash_cellrange(&sk, &mut cs, &mut ce);
        assert_eq!(cs[0], 0, "角 cell 0 区间起必登记");
        assert_eq!(ce[0], 1);
        assert_eq!(si[0], 1, "越界粒子 1 必在 cell 0 区间内");
        assert_eq!(cs[262143], 2);
        assert_eq!(ce[262143], 3);
        // cellkey 步计数器(vel=0 ⇒ 预测恒等):负 floor 3 轴 + clamp 6 轴。
        let mut st = FluidState {
            prev_x: px.clone(),
            prev_y: py.clone(),
            prev_z: pz.clone(),
            pos_x: px,
            pos_y: py,
            pos_z: pz,
            vel_x: vec![0.0; 3],
            vel_y: vec![0.0; 3],
            vel_z: vec![0.0; 3],
        };
        let mut p0 = p;
        p0.gravity_y = 0.0;
        let ck = hash_cellkey_step(&mut st, &p0);
        assert_eq!(ck.negative_floor_events, 3, "负 floor 事件 = 粒子 1 三轴");
        assert_eq!(ck.clamp_events, 6, "clamp 事件 = 粒子 1 三轴 + 粒子 2 三轴");
        assert_eq!(ck.keys, keys, "预测恒等下键必与直算一致");
    }

    #[test]
    fn negative_coordinate_floor_semantics() {
        // f32 .floor() 对负商向负无穷取整(截断语义会给 0/−1 → 判据咬合)。
        let cs = 0.2;
        assert_eq!(cell_axis_floor(-0.06, 0.0, cs), -1.0, "−0.3 商必 floor 到 −1");
        assert_eq!(cell_axis_floor(-0.3, 0.0, cs), -2.0, "−1.5 商必 floor 到 −2");
        assert_eq!(cell_axis_floor(0.06, 0.0, cs), 0.0);
        assert_eq!(cell_axis(-0.06, 0.0, cs), 0, "负 floor 后 clamp 到 0");
        assert_eq!(cell_axis(-0.3, 0.0, cs), 0);
        // 负 origin 域:界内负坐标正常寻 cell(非 clamp 路径)。
        assert_eq!(cell_axis(-6.35, -6.4, cs), 0);
        assert_eq!(cell_axis(-0.05, -6.4, cs), 31); // (6.35/0.2)=31.749→31
        assert_eq!(cell_axis(6.35, -6.4, cs), 63);
    }

    #[test]
    fn xpbd_expands_compressed_pair() {
        // 两粒子过近(C>0)⇒ apply 必推离(方向性;λ<0 + 梯度方向咬合)。
        let p = default_params();
        let px = vec![1.0f32, 1.05];
        let py = vec![1.0f32, 1.0];
        let pz = vec![1.0f32, 1.0];
        let keys: Vec<u32> = (0..2)
            .map(|i| position_cell_key(px[i], py[i], pz[i], &p))
            .collect();
        let (sk, si) = primitives::sort_pairs_u24(&keys, &[0u32, 1]);
        let (mut cs, mut ce) = hash_clear();
        hash_cellrange(&sk, &mut cs, &mut ce);
        // 人工高密度:两粒子距 0.05 << h=0.2 ⇒ ρ > ρ0 用小 ρ0 保 C>0。
        let mut p2 = p;
        p2.rho0 = 100.0;
        let (rho, lambda) = xpbd_density_step(&px, &py, &pz, &si, &cs, &ce, &p2);
        assert!(rho[0] > p2.rho0, "压缩对密度必超 ρ0(得 {})", rho[0]);
        assert!(lambda[0] < 0.0 && lambda[1] < 0.0, "C>0 ⇒ λ 必负");
        let (nx, _, _) = xpbd_apply_step(&px, &py, &pz, &lambda, &si, &cs, &ce, &p2);
        let d0 = (px[1] - px[0]).abs();
        let d1 = (nx[1] - nx[0]).abs();
        assert!(d1 > d0, "apply 必推离压缩对(d0={d0} d1={d1})");
    }

    #[test]
    fn apply_clamps_to_world_aabb_and_velocity_zeroes() {
        // 越界粒子 apply 后必 clamp 回 AABB;velocity 步边界分量必置零。
        let p = default_params();
        let mut st = FluidState {
            pos_x: vec![-0.5f32],
            pos_y: vec![13.0f32],
            pos_z: vec![1.0f32],
            prev_x: vec![1.0f32],
            prev_y: vec![1.0f32],
            prev_z: vec![0.9f32],
            vel_x: vec![0.0],
            vel_y: vec![0.0],
            vel_z: vec![0.0],
        };
        let keys = vec![position_cell_key(-0.5, 13.0, 1.0, &p)];
        let (sk, si) = primitives::sort_pairs_u24(&keys, &[0u32]);
        let (mut cs, mut ce) = hash_clear();
        hash_cellrange(&sk, &mut cs, &mut ce);
        let lambda = vec![0.0f32];
        let (nx, ny, nz) =
            xpbd_apply_step(&st.pos_x, &st.pos_y, &st.pos_z, &lambda, &si, &cs, &ce, &p);
        assert_eq!(nx[0], 0.0, "x 负越界必 clamp 到下界");
        assert!((ny[0] - 12.8).abs() < 1e-5, "y 正越界必 clamp 到上界(得 {})", ny[0]);
        assert_eq!(nz[0], 1.0, "界内轴不动");
        st.pos_x = nx;
        st.pos_y = ny;
        st.pos_z = nz;
        xpbd_velocity_step(&mut st, &p);
        assert_eq!(st.vel_x[0], 0.0, "clamp 轴速度分量必置零");
        assert_eq!(st.vel_y[0], 0.0);
        assert!(st.vel_z[0] != 0.0, "非边界轴速度 = (pos−prev)/dt ≠ 0");
    }

    #[test]
    fn frozen_constants_and_coefs() {
        assert_eq!(GRID, 64);
        assert_eq!(GRID_CELLS, 262_144);
        assert!(GRID_CELLS < (1 << 24), "cell_id 必须在 24 位键域内");
        assert_eq!(ITER, 3);
        assert_eq!(XPBD_EPS, 100.0);
        assert_eq!(CELL_START_EMPTY, u32::MAX);
        // 系数公式冻结(h=0.2):315/(64π·0.2⁹) 与 −45/(π·0.2⁶)。
        let h = 0.2f32;
        let p6 = poly6_coef(h);
        let sg = spiky_grad_coef(h);
        assert!((p6 - 3_059_924.0).abs() / 3_059_924.0 < 1e-3, "poly6 系数漂移: {p6}");
        assert!((sg + 223_811.16).abs() / 223_811.16 < 1e-3, "spiky 系数漂移: {sg}");
        assert!(sg < 0.0, "spiky 梯度系数必含负号");
    }
}
