//! G35-5 碰撞与力场 host 金标准(ray query 同帧碰撞 + 力场)——门
//! `g35.wave5.collision`(RFC-0049 §4.7 评审修订后基线;契约 = mod.rs
//! G35-P v1 冻结头,事实源 = milestones/g35/G35_CONTRACT.md)。
//!
//! 交付面(G35_PLAN §2 波 5):**同帧 TLAS ray query 碰撞**(第 k 帧碰撞
//! 查询消费第 k 帧场景状态——反打 Niagara GPU RT 碰撞的异步一帧延迟:
//! Niagara 的 GPU 场景射线追踪碰撞读取的是上一帧末的加速结构,障碍第 k 帧
//! 突移粒子第 k+1 帧才响应;本波 probe 每帧用当帧障碍变换重建场景再跑
//! sim_collide,host 金标准同语义对拍)+ 深度缓冲碰撞**对照臂** + 显式降级
//! 链 `--collision ray_query|depth_buffer|off`(F12 三档 CLI 闭集;ray_query
//! 档无 TLAS 能力 → typed 错误退出 fail-closed,**禁静默换臂**)+ 力场
//! (gravity/wind/drag v1 闭集)。device 面 = `kernels/g35_sim_collide.rx`
//! (AccelStruct ray query 臂,run_ray_query_effects 车道)与
//! `kernels/g35_sim_collide_depth.rx`(深度图 SSBO 对照臂,run_compute
//! 车道);probe = `bin/g35_collision_device.rs`。
//!
//! ## 力场 v1 闭集(冻结;运算序 host/device 逐字同源)
//!
//! gravity_y + wind_xyz 常量加速度 + 线性阻尼 drag。顺序冻结:
//! ```text
//! vx = vx + wx·dt;  vy = vy + (g + wy)·dt;  vz = vz + wz·dt;   (vel += (g+wind)·dt)
//! k = 1 − drag·dt;  vx = vx·k;  vy = vy·k;  vz = vz·k;         (vel ×= (1−drag·dt))
//! 再碰撞查询(碰撞消费力场更新后的 vel)。
//! ```
//! 语义与 `rurix-physics` 对齐(**只对齐不依赖**,渲染 crate 不引物理
//! crate):y-up 右手系;gravity_y 为 y 轴加速度分量(m/s²),与
//! `rurix_physics::types::WorldDesc::gravity = [0,−9.81,0]` 同向同单位约定
//! (重力向下 = 负 y);wind 为常量加速度场(m/s²,LinearForce 语义);drag
//! 为线性阻尼系数(1/s,`vel·(1−drag·dt)` 半隐式折减,对齐线性 damping
//! 语义)。
//!
//! ## 碰撞响应(冻结,host/device 逐字同源;RFC-0049 §4.7)
//!
//! 射线 = pos → pos+vel·dt,**参数化冻结**:origin = pos,dir = vel
//! (不归一),t 域 = (0, dt) 开区间(TriBvh `intersect_within` 同区间;
//! t 单位 = 时间,与 dir 长度乘积即位移——bvh.rs「t 参数仿射不变」约定)。
//! 命中(committed t ∈ (0, dt)):
//! ```text
//! c  = pos + vel·t                     (碰点;componentwise cx = px + vx·t)
//! n  = 三角几何法线(committed primitive 顶点叉积,与 host TriBvh
//!      face_normal 同式同序:(b−a)×(c−a) 后倒数乘归一化 self·(1/l);
//!      朝向翻转使 n·vel < 0:nd > 0 时 n = −n)
//! pos' = c + n·eps                     (eps = COLLISION_EPS = 1e-3 冻结)
//! v_n = dot(vel,n)·n;  v_t = vel − v_n
//! vel' = mu_t·v_t − e·v_n              (e = 0.5,mu_t = 0.8 冻结缺省,
//!                                       params 面可调)
//! age 照常(age += dt)。
//! ```
//! 未命中(或 |vel|² = 0 零方向守卫,两侧同守卫):走原 sim 积分
//! `pos += vel·dt`(力场已代 sim 的 vy += g·dt 步,消费更新后 vel = 半隐式
//! Euler 语义保持),age += dt,flags = (age < life)。
//!
//! ## 深度缓冲对照臂(冻结;**对照教育臂非生产档**——屏幕空间局限如实登记)
//!
//! 固定正交俯视深度图([`DepthGrid`] xz 网格,格值 = 该格俯视最高地形 y,
//! host 合成上传 [`synth_topdown_depth`]):积分候选点 (qx,qy,qz) 的 xz 投到
//! 网格,`qy < h` → 同式响应,法线取 +y 简化(v_n = (0,vy,0),c =
//! (qx,h,qz))。**局限登记**:仅表达俯视单层高度场——垂直侧面/悬空底面/
//! 多层几何不可表达,侧向逼近会被错误弹到格顶(单测
//! `depth_arm_side_hit_limitation_manifests` 使该局限如实显形);法线恒 +y
//! 在斜面上非真实法线。`res == 0` = 显式 off 档(纯力场 + 积分,降级链
//! 第三档的 device 承载形)。
//!
//! ## 确定性协议
//!
//! 逐粒子独立、无原子、随机零消费;device 双跑位级;device↔host f32 流走
//! 标定容差(milestones/g35/g35_budget.json `g35.collision.parity_p100`,
//! threshold = measured×2.0 程序产禁手写——ray query t 值 RT core vs host
//! 有 ULP 级差,g34 先例,容差协议正为此设)。

use super::core::ParticlePools;
use crate::rt::bvh::{Ray, TriBvh, Vec3};

/// 碰撞抬升距离(pos' = c + n·eps;RFC-0049 §4.7 冻结,禁调)。
pub const COLLISION_EPS: f32 = 1e-3;
/// 恢复系数 e 冻结缺省(params 面可调;头注冻结面)。
pub const DEFAULT_RESTITUTION: f32 = 0.5;
/// 切向保留系数 mu_t 冻结缺省(params 面可调;头注冻结面)。
pub const DEFAULT_MU_T: f32 = 0.8;
/// 深度图「无地形」哨兵(俯视 miss 格;任何 qy 均不小于它 ⇒ 恒不响应)。
pub const DEPTH_NO_TERRAIN: f32 = -1.0e30;

/// 力场参数(v1 闭集:gravity_y + wind_xyz 常量 + 线性阻尼 drag;
/// 语义对齐见模块头注「力场 v1 闭集」)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldParams {
    /// 重力 y 分量(m/s²;向下 = 负,rurix-physics WorldDesc.gravity 同向)。
    pub gravity_y: f32,
    /// 常量风加速度(m/s²;LinearForce 语义对齐)。
    pub wind: [f32; 3],
    /// 线性阻尼系数(1/s;vel·(1−drag·dt),drag·dt < 1 调用方纪律)。
    pub drag: f32,
}

/// 碰撞响应参数(冻结公式的系数面;缺省 = 冻结常量)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionParams {
    /// 恢复系数 e ∈ [0,1](法向反弹折减)。
    pub e: f32,
    /// 切向保留 mu_t ∈ [0,1](1 = 无摩擦滑移,0 = 切向全吸收)。
    pub mu_t: f32,
}

impl Default for CollisionParams {
    fn default() -> Self {
        Self {
            e: DEFAULT_RESTITUTION,
            mu_t: DEFAULT_MU_T,
        }
    }
}

/// 固定正交俯视深度图网格(x ∈ [x0, x0+res·cell),z ∈ [z0, z0+res·cell),
/// 行主序 idx = iz·res + ix;`res == 0` = 显式 off 档)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthGrid {
    /// 网格 x 起点(世界坐标)。
    pub x0: f32,
    /// 网格 z 起点。
    pub z0: f32,
    /// 格边长(> 0;res == 0 时不消费)。
    pub cell: f32,
    /// 每轴格数(0 = off 档:纯力场 + 积分,不碰撞)。
    pub res: usize,
}

/// 碰撞步输出:`flags` = 存活位(age < life,sim 同律);`hit` = 命中登记
/// (0 = miss,1 + tri = 命中三角下标 +1——device
/// `1 + committed_primitive_index` 同编码,同帧见证/对照臂消费面;深度臂
/// 命中恒 1,无三角语义)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollideOut {
    /// 存活 flags(u32 0/1;len = n)。
    pub flags: Vec<u32>,
    /// 命中登记(u32;len = n)。
    pub hit: Vec<u32>,
}

/// 力场步(= 两 kernel 力场段逐字同源;顺序冻结见模块头注):
/// `vel += (g+wind)·dt` 后 `vel ×= (1−drag·dt)`。只改 vel,不动 pos/age。
pub fn apply_fields(p: &mut ParticlePools, f: &FieldParams, dt: f32) {
    let n = p.n;
    let g = f.gravity_y;
    let wx = f.wind[0];
    let wy = f.wind[1];
    let wz = f.wind[2];
    let k = 1.0 - f.drag * dt;
    for i in 0..n {
        // 运算序逐字冻结(kernel 同序):加速度段 x→y→z,再阻尼段 x→y→z。
        p.vel_x[i] = p.vel_x[i] + wx * dt;
        p.vel_y[i] = p.vel_y[i] + (g + wy) * dt;
        p.vel_z[i] = p.vel_z[i] + wz * dt;
        p.vel_x[i] = p.vel_x[i] * k;
        p.vel_y[i] = p.vel_y[i] * k;
        p.vel_z[i] = p.vel_z[i] * k;
    }
}

/// ray query 碰撞步 host 金标准(= kernels/g35_sim_collide.rx 碰撞段逐字
/// 同源;调用序 = [`apply_fields`] 后本步——kernel 单 dispatch 内联同序)。
/// 射线/响应/守卫全式见模块头注「碰撞响应」;`bvh` = 当帧场景(同帧语义:
/// 第 k 帧查询消费第 k 帧障碍变换构建的 BVH,调用方每帧重建/refit)。
pub fn collide_step(
    p: &mut ParticlePools,
    bvh: &TriBvh,
    cp: &CollisionParams,
    dt: f32,
) -> CollideOut {
    let n = p.n;
    let mut flags = vec![0u32; n];
    let mut hit = vec![0u32; n];
    for i in 0..n {
        let px = p.pos_x[i];
        let py = p.pos_y[i];
        let pz = p.pos_z[i];
        let vx = p.vel_x[i];
        let vy = p.vel_y[i];
        let vz = p.vel_z[i];
        // 零方向守卫(device 同守卫:HW ray query 零方向未定义,两侧同判)。
        let len2 = (vx * vx + vy * vy) + vz * vz;
        let mut resolved = false;
        if len2 > 0.0 {
            let ray = Ray {
                origin: Vec3::new(px, py, pz),
                dir: Vec3::new(vx, vy, vz),
            };
            // t ∈ (0, dt) 开区间(intersect_within 契约);t 单位 = 时间。
            if let Some(h) = bvh.intersect_within(&ray, dt) {
                let t = h.t;
                let cx = px + vx * t;
                let cy = py + vy * t;
                let cz = pz + vz * t;
                // 法线 = TriBvh face_normal((b−a)×(c−a) 倒数乘归一;kernel
                // 由 tris SSBO 镜像按 committed_primitive_index 同式重算,
                // 单测 kernel_normal_formula_matches_tribvh 锚定同式同序)。
                let mut nx = h.normal[0];
                let mut ny = h.normal[1];
                let mut nz = h.normal[2];
                let mut nd = (nx * vx + ny * vy) + nz * vz;
                if nd > 0.0 {
                    nx = 0.0 - nx;
                    ny = 0.0 - ny;
                    nz = 0.0 - nz;
                    nd = 0.0 - nd;
                }
                p.pos_x[i] = cx + nx * COLLISION_EPS;
                p.pos_y[i] = cy + ny * COLLISION_EPS;
                p.pos_z[i] = cz + nz * COLLISION_EPS;
                // v_n = dot(vel,n)·n;v_t = vel − v_n;vel' = mu_t·v_t − e·v_n。
                let vnx = nd * nx;
                let vny = nd * ny;
                let vnz = nd * nz;
                let vtx = vx - vnx;
                let vty = vy - vny;
                let vtz = vz - vnz;
                p.vel_x[i] = cp.mu_t * vtx - cp.e * vnx;
                p.vel_y[i] = cp.mu_t * vty - cp.e * vny;
                p.vel_z[i] = cp.mu_t * vtz - cp.e * vnz;
                hit[i] = 1 + h.tri;
                resolved = true;
            }
        }
        if !resolved {
            // 未命中走原 sim 积分(力场已代 vy += g·dt 步;半隐式语义保持)。
            p.pos_x[i] = px + vx * dt;
            p.pos_y[i] = py + vy * dt;
            p.pos_z[i] = pz + vz * dt;
        }
        p.age[i] = p.age[i] + dt;
        flags[i] = u32::from(p.age[i] < p.life[i]);
    }
    CollideOut { flags, hit }
}

/// 深度缓冲对照臂碰撞步 host 金标准(= kernels/g35_sim_collide_depth.rx
/// 逐字同源;局限登记见模块头注「深度缓冲对照臂」)。`grid.res == 0` =
/// 显式 off 档(纯力场后积分;`depth` 不消费,可为空)。
pub fn depth_collide_step(
    p: &mut ParticlePools,
    grid: &DepthGrid,
    depth: &[f32],
    cp: &CollisionParams,
    dt: f32,
) -> CollideOut {
    if grid.res > 0 {
        assert_eq!(
            depth.len(),
            grid.res * grid.res,
            "深度图长度必须 = res²(行主序 iz·res + ix)"
        );
    }
    let n = p.n;
    let mut flags = vec![0u32; n];
    let mut hit = vec![0u32; n];
    for i in 0..n {
        let vx = p.vel_x[i];
        let vy = p.vel_y[i];
        let vz = p.vel_z[i];
        // 积分候选点(kernel 同序:先积分再投影判)。
        let qx = p.pos_x[i] + vx * dt;
        let qy = p.pos_y[i] + vy * dt;
        let qz = p.pos_z[i] + vz * dt;
        let mut resolved = false;
        if grid.res > 0 {
            let fx = (qx - grid.x0) / grid.cell;
            let fz = (qz - grid.z0) / grid.cell;
            // 域外 = miss(负值先判再转 usize——ConvertFToU 负域未定义,
            // kernel 同守卫);floor 语义 = 非负截断。
            if fx >= 0.0 && fz >= 0.0 {
                let ix = fx as usize;
                let iz = fz as usize;
                if ix < grid.res && iz < grid.res {
                    let h = depth[iz * grid.res + ix];
                    if qy < h {
                        // 同式响应,n = (0,1,0) 简化:c = (qx,h,qz);
                        // v_n = (0,vy,0),v_t = (vx,0,vz)。
                        p.pos_x[i] = qx;
                        p.pos_y[i] = h + COLLISION_EPS;
                        p.pos_z[i] = qz;
                        p.vel_x[i] = cp.mu_t * vx;
                        p.vel_y[i] = 0.0 - cp.e * vy;
                        p.vel_z[i] = cp.mu_t * vz;
                        hit[i] = 1;
                        resolved = true;
                    }
                }
            }
        }
        if !resolved {
            p.pos_x[i] = qx;
            p.pos_y[i] = qy;
            p.pos_z[i] = qz;
        }
        p.age[i] = p.age[i] + dt;
        flags[i] = u32::from(p.age[i] < p.life[i]);
    }
    CollideOut { flags, hit }
}

/// 深度图合成(对照臂 host 上传源;固定正交俯视):逐格中心自 y = +1e4 向
/// −y 打射线取最高命中 y;miss 格 = [`DEPTH_NO_TERRAIN`]。调用方每帧以当帧
/// 场景 BVH 重合成(同帧语义与 ray query 臂同律)。
pub fn synth_topdown_depth(bvh: &TriBvh, grid: &DepthGrid) -> Vec<f32> {
    let mut out = vec![DEPTH_NO_TERRAIN; grid.res * grid.res];
    for iz in 0..grid.res {
        for ix in 0..grid.res {
            let cx = grid.x0 + (ix as f32 + 0.5) * grid.cell;
            let cz = grid.z0 + (iz as f32 + 0.5) * grid.cell;
            let ray = Ray {
                origin: Vec3::new(cx, 1.0e4, cz),
                dir: Vec3::new(0.0, -1.0, 0.0),
            };
            if let Some(h) = bvh.intersect(&ray) {
                out[iz * grid.res + ix] = 1.0e4 - h.t;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::rand_table;
    use super::super::{RAND_K, RAND_TABLE_LEN, SEG};
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    /// 解析场景:地板(y=0,±8)两三角 + 45° 斜面(x∈[−3,−1] 自 y=0 升至
    /// y=2)两三角。测试夹具(probe 有自己的冻结夹具,互不镜像)。
    fn floor_slope_tris() -> Vec<[f32; 3]> {
        vec![
            // 地板 t0/t1。
            [-8.0, 0.0, -8.0],
            [8.0, 0.0, -8.0],
            [8.0, 0.0, 8.0],
            [-8.0, 0.0, -8.0],
            [8.0, 0.0, 8.0],
            [-8.0, 0.0, 8.0],
            // 斜面 t2/t3(A(−3,2,−2) B(−1,0,−2) C(−1,0,2) D(−3,2,2))。
            [-3.0, 2.0, -2.0],
            [-1.0, 0.0, -2.0],
            [-1.0, 0.0, 2.0],
            [-3.0, 2.0, -2.0],
            [-1.0, 0.0, 2.0],
            [-3.0, 2.0, 2.0],
        ]
    }

    fn build_bvh(verts: &[[f32; 3]]) -> TriBvh {
        assert_eq!(verts.len() % 3, 0);
        let indices: Vec<[u32; 3]> = (0..verts.len() as u32 / 3)
            .map(|t| [t * 3, t * 3 + 1, t * 3 + 2])
            .collect();
        TriBvh::build(verts, &indices)
    }

    /// 轴对齐箱 8 三角(顶/底/±x 四面;probe 场景「可动方块」同构形)。
    fn box_tris(center: [f32; 3], half: [f32; 3]) -> Vec<[f32; 3]> {
        let (cx, cy, cz) = (center[0], center[1], center[2]);
        let (hx, hy, hz) = (half[0], half[1], half[2]);
        let (x0, x1) = (cx - hx, cx + hx);
        let (y0, y1) = (cy - hy, cy + hy);
        let (z0, z1) = (cz - hz, cz + hz);
        vec![
            // 顶面 y1。
            [x0, y1, z0], [x1, y1, z0], [x1, y1, z1],
            [x0, y1, z0], [x1, y1, z1], [x0, y1, z1],
            // 底面 y0。
            [x0, y0, z0], [x1, y0, z1], [x1, y0, z0],
            [x0, y0, z0], [x0, y0, z1], [x1, y0, z1],
            // +x 面。
            [x1, y0, z0], [x1, y1, z1], [x1, y1, z0],
            [x1, y0, z0], [x1, y0, z1], [x1, y1, z1],
            // −x 面。
            [x0, y0, z0], [x0, y1, z0], [x0, y1, z1],
            [x0, y0, z0], [x0, y1, z1], [x0, y0, z1],
        ]
    }

    fn pool_with(particles: &[([f32; 3], [f32; 3])]) -> ParticlePools {
        let mut p = ParticlePools::with_capacity(SEG);
        for (i, (pos, vel)) in particles.iter().enumerate() {
            p.pos_x[i] = pos[0];
            p.pos_y[i] = pos[1];
            p.pos_z[i] = pos[2];
            p.vel_x[i] = vel[0];
            p.vel_y[i] = vel[1];
            p.vel_z[i] = vel[2];
            p.life[i] = 100.0;
            p.pid[i] = i as u32;
        }
        p.n = particles.len();
        p
    }

    /// 逐流位级快照(core.rs pool_bits 同律)。
    fn pool_bits(p: &ParticlePools) -> Vec<u32> {
        let mut out = Vec::with_capacity(p.n * 8 + 1);
        out.push(p.n as u32);
        for i in 0..p.n {
            for v in [
                p.pos_x[i], p.pos_y[i], p.pos_z[i], p.vel_x[i], p.vel_y[i], p.vel_z[i], p.age[i],
            ] {
                out.push(v.to_bits());
            }
            out.push(p.pid[i]);
        }
        out
    }

    /// ① 力场冻结运算序:单粒子一步 = ((v + (g+w)·dt)·(1−drag·dt)) 位级重放。
    #[test]
    fn apply_fields_frozen_order_bitexact() {
        let f = FieldParams {
            gravity_y: -9.8,
            wind: [0.3, 0.1, -0.2],
            drag: 0.05,
        };
        let mut p = pool_with(&[([0.0, 1.0, 0.0], [1.0, 2.0, -0.5])]);
        apply_fields(&mut p, &f, DT);
        let k = 1.0 - f.drag * DT;
        let ex = (1.0 + f.wind[0] * DT) * k;
        let ey = (2.0 + (f.gravity_y + f.wind[1]) * DT) * k;
        let ez = (-0.5 + f.wind[2] * DT) * k;
        assert_eq!(p.vel_x[0].to_bits(), ex.to_bits(), "vx 运算序漂移");
        assert_eq!(p.vel_y[0].to_bits(), ey.to_bits(), "vy 运算序漂移(g+wy 内和先算)");
        assert_eq!(p.vel_z[0].to_bits(), ez.to_bits(), "vz 运算序漂移");
        // pos/age 不动(力场步只改 vel)。
        assert_eq!(p.pos_y[0], 1.0);
        assert_eq!(p.age[0], 0.0);
        // 风/阻尼方向语义:wind_x > 0 ⇒ vx 增;drag ⇒ 速降(对照无阻尼)。
        let mut q = pool_with(&[([0.0, 1.0, 0.0], [1.0, 2.0, -0.5])]);
        apply_fields(
            &mut q,
            &FieldParams {
                drag: 0.0,
                ..f
            },
            DT,
        );
        assert!(p.vel_x[0] > 1.0, "wind_x > 0 必须加速 +x");
        let sp = |p: &ParticlePools| {
            (p.vel_x[0] * p.vel_x[0] + p.vel_y[0] * p.vel_y[0] + p.vel_z[0] * p.vel_z[0]).sqrt()
        };
        assert!(sp(&p) < sp(&q), "drag 必须使速率低于无阻尼对照");
    }

    /// ② 地板反弹:冻结响应式的解析核验(c/pos'/vel' 全分量)。
    #[test]
    fn floor_bounce_matches_frozen_response() {
        let bvh = build_bvh(&floor_slope_tris());
        let cp = CollisionParams::default();
        // pos (0,0.1,0) vel (1,−12,0):t* = 0.1/12 ∈ (0, dt)。
        let mut p = pool_with(&[([0.0, 0.1, 0.0], [1.0, -12.0, 0.0])]);
        let out = collide_step(&mut p, &bvh, &cp, DT);
        assert!(out.hit[0] >= 1 && out.hit[0] <= 2, "应命中地板三角(t0/t1)");
        let t = 0.1 / 12.0;
        // 地板法线翻转后 = (0,1,0):c = (t, 0, 0),pos' = c + n·eps。
        assert!(approx(p.pos_x[0], t, 1e-6));
        assert!(approx(p.pos_y[0], COLLISION_EPS, 1e-6));
        assert!(approx(p.pos_z[0], 0.0, 1e-6));
        // v_n = (0,−12,0),v_t = (1,0,0) ⇒ vel' = (0.8·1, −0.5·(−12), 0)。
        assert!(approx(p.vel_x[0], cp.mu_t * 1.0, 1e-6));
        assert!(approx(p.vel_y[0], -cp.e * -12.0, 1e-5));
        assert!(approx(p.vel_z[0], 0.0, 1e-6));
        assert_eq!(p.age[0], DT, "age 照常");
        assert_eq!(out.flags[0], 1);
    }

    /// ③ 斜面反弹:法向分量按 −e 反射、切向按 mu_t 保留(投影分解核验),
    /// 反弹后必须背离表面(dot(vel', n) > 0)。
    #[test]
    fn slope_bounce_reflects_about_slope_normal() {
        let bvh = build_bvh(&floor_slope_tris());
        let cp = CollisionParams::default();
        // 朝斜面(x∈[−3,−1] 斜升)打:起点 (−1.5,1.0,0) 速度 (−30,−30,0),
        // 斜面点 x=−1.5 处高 0.5 ⇒ 必穿。
        let v0 = [-30.0f32, -30.0, 0.0];
        let mut p = pool_with(&[([-1.5, 1.0, 0.0], v0)]);
        let out = collide_step(&mut p, &bvh, &cp, DT);
        assert!(out.hit[0] == 3 || out.hit[0] == 4, "应命中斜面三角(t2/t3),得 {}", out.hit[0]);
        // 斜面单位法线(朝上臂):(1,1,0)/√2。
        let s = 1.0 / 2.0f32.sqrt();
        let n = [s, s, 0.0f32];
        let nd0 = v0[0] * n[0] + v0[1] * n[1] + v0[2] * n[2];
        assert!(nd0 < 0.0, "入射必须朝向表面");
        let vn = [nd0 * n[0], nd0 * n[1], nd0 * n[2]];
        let vt = [v0[0] - vn[0], v0[1] - vn[1], v0[2] - vn[2]];
        let expect = [
            cp.mu_t * vt[0] - cp.e * vn[0],
            cp.mu_t * vt[1] - cp.e * vn[1],
            cp.mu_t * vt[2] - cp.e * vn[2],
        ];
        assert!(approx(p.vel_x[0], expect[0], 1e-3));
        assert!(approx(p.vel_y[0], expect[1], 1e-3));
        assert!(approx(p.vel_z[0], expect[2], 1e-3));
        let nd1 = p.vel_x[0] * n[0] + p.vel_y[0] * n[1] + p.vel_z[0] * n[2];
        assert!(nd1 > 0.0, "反弹后必须背离表面");
    }

    /// ④ 能量不增(e ≤ 1,mu_t ≤ 1):|vel'|² = mu²|v_t|² + e²|v_n|² ≤ |vel|²
    /// 逐响应事件成立;多帧轨迹总动能 + 势能不增(力场关断,纯碰撞损耗)。
    #[test]
    fn energy_never_increases_across_bounces() {
        let bvh = build_bvh(&floor_slope_tris());
        let cp = CollisionParams::default();
        let mut p = pool_with(&[
            ([0.0, 2.0, 0.0], [1.0, -6.0, 0.5]),
            ([-1.6, 1.5, 0.3], [-2.0, -5.0, 0.0]),
            ([2.0, 0.5, -1.0], [0.5, -9.0, 1.5]),
        ]);
        let g = -9.8f32;
        let f = FieldParams {
            gravity_y: g,
            wind: [0.0; 3],
            drag: 0.0,
        };
        let energy = |p: &ParticlePools, i: usize| {
            let ke = 0.5
                * (p.vel_x[i] * p.vel_x[i] + p.vel_y[i] * p.vel_y[i] + p.vel_z[i] * p.vel_z[i]);
            ke + (-g) * p.pos_y[i]
        };
        let mut bounces = 0usize;
        for _ in 0..240 {
            // 势能基准在力场步后取(重力已注入动能面,与碰撞损耗解耦)。
            apply_fields(&mut p, &f, DT);
            let before: Vec<f32> = (0..p.n).map(|i| energy(&p, i)).collect();
            let sp_before: Vec<f32> = (0..p.n)
                .map(|i| p.vel_x[i] * p.vel_x[i] + p.vel_y[i] * p.vel_y[i] + p.vel_z[i] * p.vel_z[i])
                .collect();
            let out = collide_step(&mut p, &bvh, &cp, DT);
            for i in 0..p.n {
                if out.hit[i] != 0 {
                    bounces += 1;
                    let sp_after = p.vel_x[i] * p.vel_x[i]
                        + p.vel_y[i] * p.vel_y[i]
                        + p.vel_z[i] * p.vel_z[i];
                    assert!(
                        sp_after <= sp_before[i] * (1.0 + 1e-5),
                        "响应事件速率²增大:{} → {}(e≤1/mu_t≤1 破)",
                        sp_before[i],
                        sp_after
                    );
                    // 机械能对照(eps 抬升引入 ≤ |g|·eps 势能,计入容差)。
                    assert!(
                        energy(&p, i) <= before[i] + (-g) * COLLISION_EPS + 1e-3,
                        "响应事件机械能不增破"
                    );
                }
            }
        }
        assert!(bounces >= 6, "样本量门:240 帧内应有多次反弹(得 {bounces})");
        // 全程未穿地(位置恒 ≥ 地板;穿模 = 响应失效)。
        for i in 0..p.n {
            assert!(p.pos_y[i] >= -1e-4, "粒子 {i} 穿透地板:y={}", p.pos_y[i]);
        }
    }

    /// ⑤ 零方向守卫 + 未命中 = 原 sim 积分(位级)。
    #[test]
    fn zero_velocity_guard_and_miss_integration() {
        let bvh = build_bvh(&floor_slope_tris());
        let cp = CollisionParams::default();
        let mut p = pool_with(&[
            ([1.0, 5.0, 1.0], [0.0, 0.0, 0.0]),  // 零方向
            ([1.0, 5.0, 1.0], [0.5, 1.0, -0.25]), // 高空 miss
        ]);
        let out = collide_step(&mut p, &bvh, &cp, DT);
        assert_eq!(out.hit, vec![0, 0]);
        assert_eq!(p.pos_x[0].to_bits(), 1.0f32.to_bits(), "零方向:pos 积分零位移");
        assert_eq!(p.pos_y[0].to_bits(), 5.0f32.to_bits());
        assert_eq!(p.pos_x[1].to_bits(), (1.0f32 + 0.5 * DT).to_bits(), "miss = pos += vel·dt");
        assert_eq!(p.pos_y[1].to_bits(), (5.0f32 + 1.0 * DT).to_bits());
        assert_eq!(p.pos_z[1].to_bits(), (1.0f32 + -0.25 * DT).to_bits());
        assert_eq!(p.age[0].to_bits(), DT.to_bits(), "age 照常");
    }

    /// ⑥ 同帧位移响应(同帧语义 host 见证):障碍箱第 3 帧突移到粒子正下方
    /// ——gold(k 帧查 k 帧场景)与 static(箱不动)首异帧 == 3(突移当帧
    /// 即响应);late(k 帧查 k−1 帧场景 = Niagara 一帧延迟对照)在突移帧
    /// 与 static 位级一致 = 迟延模型当帧**无**响应——判别器双向有效。
    #[test]
    fn same_frame_displacement_witness() {
        let cp = CollisionParams::default();
        let f = FieldParams {
            gravity_y: -9.8,
            wind: [0.0; 3],
            drag: 0.0,
        };
        const MOVE_FRAME: usize = 3;
        let box_center = |frame: usize| -> [f32; 3] {
            if frame < MOVE_FRAME {
                [6.0, 0.55, 0.0]
            } else {
                [0.0, 0.55, 0.0]
            }
        };
        let scene = |frame: usize| -> TriBvh {
            let mut verts = floor_slope_tris();
            verts.extend(box_tris(box_center(frame), [0.75, 0.5, 0.75]));
            build_bvh(&verts)
        };
        // 粒子 0 初高 1.10:自由落体下,恰在第 3 帧线段跨越箱顶 y=1.05
        // (帧 0..2 段末 1.0889/1.0752/1.0587 均 > 1.05,帧 3 段末 1.0394
        // < 1.05)——箱到位当帧即碰的夹具解析解。
        let init = || {
            pool_with(&[
                ([0.0, 1.10, 0.0], [0.0, -0.5, 0.0]),
                ([0.3, 1.2, 0.2], [0.0, -1.0, 0.0]),
            ])
        };
        let run = |scene_of: &dyn Fn(usize) -> TriBvh, frames: usize| -> Vec<Vec<u32>> {
            let mut p = init();
            let mut snaps = Vec::new();
            for k in 0..frames {
                apply_fields(&mut p, &f, DT);
                collide_step(&mut p, &scene_of(k), &cp, DT);
                snaps.push(pool_bits(&p));
            }
            snaps
        };
        let gold = run(&scene, 8);
        let stat = run(&|_| scene(0), 8);
        let late = run(&|k| scene(k.saturating_sub(1)), 8);
        let first_div = |a: &[Vec<u32>], b: &[Vec<u32>]| -> Option<usize> {
            (0..a.len()).find(|&k| a[k] != b[k])
        };
        assert_eq!(
            first_div(&gold, &stat),
            Some(MOVE_FRAME),
            "同帧语义:突移帧即首异帧(k 帧查询消费 k 帧场景)"
        );
        assert_eq!(
            late[MOVE_FRAME], stat[MOVE_FRAME],
            "一帧延迟对照(Niagara 模型)在突移帧必须与 static 位级一致(当帧无响应)"
        );
        assert_ne!(
            gold[MOVE_FRAME], late[MOVE_FRAME],
            "同帧模型与一帧延迟模型在突移帧必须可分辨——判别器有效性"
        );
    }

    /// ⑦ 固定输入双跑位级(host 金标准确定性;随机零消费面)。
    #[test]
    fn double_run_bitexact_and_seed_sensitive() {
        let mut verts = floor_slope_tris();
        verts.extend(box_tris([0.0, 0.55, 0.0], [0.75, 0.5, 0.75]));
        let bvh = build_bvh(&verts);
        let cp = CollisionParams::default();
        let f = FieldParams {
            gravity_y: -9.8,
            wind: [0.3, 0.0, 0.1],
            drag: 0.05,
        };
        let run = |seed: u64| -> Vec<u32> {
            let table = rand_table(seed);
            let mut p = ParticlePools::with_capacity(SEG);
            for j in 0..64usize {
                let r = |k: usize| table[(j * RAND_K + k) % RAND_TABLE_LEN];
                p.pos_x[j] = (r(0) * 2.0 - 1.0) * 2.5;
                p.pos_y[j] = 3.0 + (r(1) * 2.0 - 1.0) * 0.4;
                p.pos_z[j] = (r(2) * 2.0 - 1.0) * 1.5;
                p.vel_x[j] = (r(3) * 2.0 - 1.0) * 0.5;
                p.vel_y[j] = -2.0 + (r(4) * 2.0 - 1.0) * 0.5;
                p.vel_z[j] = (r(5) * 2.0 - 1.0) * 0.5;
                p.life[j] = 100.0;
                p.pid[j] = j as u32;
            }
            p.n = 64;
            for _ in 0..40 {
                apply_fields(&mut p, &f, DT);
                collide_step(&mut p, &bvh, &cp, DT);
            }
            pool_bits(&p)
        };
        assert_eq!(run(42), run(42), "同 seed 双跑必须位级一致");
        assert_ne!(run(42), run(43), "异 seed 必须可分辨");
    }

    /// ⑧ kernel 法线公式同式同序锚:componentwise 叉积 + 倒数乘归一
    /// (g35_sim_collide.rx 文本形)与 TriBvh face_normal(Vec3 算子)位级
    /// 全等——「命中面 SSBO 镜像重算 = host 法线」的机器锚。
    #[test]
    fn kernel_normal_formula_matches_tribvh() {
        let mut verts = floor_slope_tris();
        verts.extend(box_tris([0.4, 0.55, -0.2], [0.75, 0.5, 0.75]));
        for tri in verts.chunks_exact(3) {
            let (a, b, c) = (tri[0], tri[1], tri[2]);
            // kernel 文本形(g35_sim_collide.rx 碰撞段逐字)。
            let e1x = b[0] - a[0];
            let e1y = b[1] - a[1];
            let e1z = b[2] - a[2];
            let e2x = c[0] - a[0];
            let e2y = c[1] - a[1];
            let e2z = c[2] - a[2];
            let ngx = e1y * e2z - e1z * e2y;
            let ngy = e1z * e2x - e1x * e2z;
            let ngz = e1x * e2y - e1y * e2x;
            let nl = (ngx * ngx + ngy * ngy + ngz * ngz).sqrt();
            let mut inv_nl = 0.0;
            if nl > 0.0 {
                inv_nl = 1.0 / nl;
            }
            let k = [ngx * inv_nl, ngy * inv_nl, ngz * inv_nl];
            // host Vec3 算子形(TriBvh face_normal 同式)。
            let h = (Vec3::from_array(b) - Vec3::from_array(a))
                .cross(Vec3::from_array(c) - Vec3::from_array(a))
                .normalize()
                .to_array();
            for d in 0..3 {
                assert_eq!(
                    k[d].to_bits(),
                    h[d].to_bits(),
                    "法线公式分量 {d} 非位级同式(kernel 文本形 ≠ Vec3 算子形)"
                );
            }
        }
    }

    /// ⑨ 深度对照臂:地板-only 场景上与 ray query 臂轨迹贴合(沉降高度带),
    /// off 档(res=0)= 纯积分位级。
    #[test]
    fn depth_arm_tracks_ray_query_on_floor_and_off_tier_is_pure_integration() {
        let floor_only: Vec<[f32; 3]> = floor_slope_tris()[..6].to_vec();
        let bvh = build_bvh(&floor_only);
        let grid = DepthGrid {
            x0: -8.0,
            z0: -8.0,
            cell: 0.25,
            res: 64,
        };
        let depth = synth_topdown_depth(&bvh, &grid);
        // 地板域内格全 0 高(合成正确性)。
        assert!(depth.iter().all(|&h| h.abs() < 1e-4), "地板-only 合成应全 0 高");
        let cp = CollisionParams::default();
        let f = FieldParams {
            gravity_y: -9.8,
            wind: [0.0; 3],
            drag: 0.1,
        };
        let init = || {
            pool_with(&[
                ([0.5, 2.0, 0.5], [0.4, -3.0, -0.2]),
                ([-2.0, 1.5, 1.0], [-0.3, -2.0, 0.4]),
            ])
        };
        let (mut rq, mut dp) = (init(), init());
        for _ in 0..180 {
            apply_fields(&mut rq, &f, DT);
            collide_step(&mut rq, &bvh, &cp, DT);
            apply_fields(&mut dp, &f, DT);
            depth_collide_step(&mut dp, &grid, &depth, &cp, DT);
        }
        for i in 0..rq.n {
            // 双臂都沉降到地板贴地带(eps 抬升近旁)。
            assert!(
                rq.pos_y[i] >= 0.0 && rq.pos_y[i] < 0.05,
                "RQ 臂粒子 {i} 未沉降:y={}",
                rq.pos_y[i]
            );
            assert!(
                dp.pos_y[i] >= 0.0 && dp.pos_y[i] < 0.05,
                "深度臂粒子 {i} 未沉降:y={}",
                dp.pos_y[i]
            );
            assert!(
                (rq.pos_y[i] - dp.pos_y[i]).abs() < 0.02,
                "地板-only 双臂沉降高度应贴合:rq={} dp={}",
                rq.pos_y[i],
                dp.pos_y[i]
            );
        }
        // off 档:res=0 ⇒ 与手工积分位级同(降级链第三档语义)。
        let mut off = pool_with(&[([0.5, 2.0, 0.5], [0.4, -3.0, -0.2])]);
        let off_grid = DepthGrid {
            x0: 0.0,
            z0: 0.0,
            cell: 1.0,
            res: 0,
        };
        let out = depth_collide_step(&mut off, &off_grid, &[], &cp, DT);
        assert_eq!(out.hit, vec![0]);
        assert_eq!(off.pos_x[0].to_bits(), (0.5f32 + 0.4 * DT).to_bits());
        assert_eq!(off.pos_y[0].to_bits(), (2.0f32 + -3.0 * DT).to_bits());
        assert_eq!(off.pos_z[0].to_bits(), (0.5f32 + -0.2 * DT).to_bits());
    }

    /// ⑩ 深度臂屏幕空间局限如实显形(头注登记的教育对照):侧向逼近箱体时
    /// ray query 臂沿 −x 面正确反射(vx 反号),深度臂把粒子错误弹到格顶
    /// (俯视高度场不可表达垂直侧面)。
    #[test]
    fn depth_arm_side_hit_limitation_manifests() {
        let mut verts = floor_slope_tris();
        verts.extend(box_tris([0.0, 0.55, 0.0], [0.75, 0.5, 0.75]));
        let bvh = build_bvh(&verts);
        let grid = DepthGrid {
            x0: -8.0,
            z0: -8.0,
            cell: 0.125,
            res: 128,
        };
        let depth = synth_topdown_depth(&bvh, &grid);
        let cp = CollisionParams::default();
        // 水平朝 −x 面(x=−0.75)逼近:y=0.5 在箱侧半高;线段 −0.9 → −0.7
        // (12·dt = 0.2)跨越侧面平面。
        let init = || pool_with(&[([-0.9, 0.5, 0.0], [12.0, 0.0, 0.0])]);
        let (mut rq, mut dp) = (init(), init());
        let orq = collide_step(&mut rq, &bvh, &cp, DT);
        let odp = depth_collide_step(&mut dp, &grid, &depth, &cp, DT);
        assert!(orq.hit[0] != 0, "RQ 臂应命中箱 −x 侧面");
        assert!(rq.vel_x[0] < 0.0, "RQ 臂侧面反射:vx 必反号(得 {})", rq.vel_x[0]);
        assert!(approx(rq.pos_x[0], -0.75 - COLLISION_EPS, 1e-3), "RQ 臂停在侧面外侧");
        // 深度臂:候选点落入箱顶footprint,y=0.5 < 格高 1.05 ⇒ 错误弹顶。
        assert_eq!(odp.hit[0], 1, "深度臂按俯视高度场触发");
        assert!(
            approx(dp.pos_y[0], 1.05 + COLLISION_EPS, 1e-3),
            "深度臂局限:被弹到箱顶 y≈1.051(得 {})——头注登记的屏幕空间局限",
            dp.pos_y[0]
        );
        assert!(dp.vel_x[0] > 0.0, "深度臂 vx 仅 mu_t 折减不反号(法线 +y 简化)");
    }
}
