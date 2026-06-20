//! uc03-demo — Rurix UC-03 旗舰验收 demo(M7.4,D-M7-4,契约 G-M7-1;01 §6 旗舰用例:
//! SPH 仿真 + 软光栅出图)。
//!
//! 本 crate 是**工程编排**:把确定性 SPH(光滑粒子流体力学)仿真的逐帧粒子态映射为
//! 屏幕空间三角形,交 G0 软光栅管线([`soft_raster`],RXS-0118~0121)渲染,再经
//! image-io(RXS-0114~0117)PPM P6 确定编码落盘为图像序列。复用既有条款,不新增
//! spec 语义面。
//!
//! 纪律:**全 safe**(`unsafe_code = "deny"`,继承 workspace lints);纯函数、确定性
//! ——固定初值 + 固定时间步长,无随机量 / 时间戳 / 平台相关字节。同输入两次运行逐帧
//! 像素逐字节一致(对接 RXS-0116/0117 与 M7.3 确定性帧像素口径),为 G-M7-1 确定性
//! 图像序列门铺底。
//!
//! 仿真为 host/CPU 编排(契约 D-M7-4 / kernel_hot_reload 允许 host 编排);邻居遍历序
//! 与积分序按粒子下标固定,归约序确定(规避浮点累加序非确定性)。

use image_io::{ImageBuffer, Rgb};
use soft_raster::{HEIGHT, Tri, Vertex, WIDTH, render_hdr, tonemap_frame};

/// 实时窗口呈现通路（G1.1，feature `d3d12-present`；RFC-0001 / RXS-0142~0143）。
/// 复用 rurix-rt interop scope 帧 typestate;G0 软光栅 kernel 语义面 0-byte，仅新增呈现通路。
#[cfg(feature = "d3d12-present")]
pub mod present;

/// 粒子网格列数(初始布局)。
pub const GRID_NX: u32 = 6;
/// 粒子网格行数(初始布局)。
pub const GRID_NY: u32 = 5;
/// 初始粒子间距(仿真/屏幕像素单位)。
pub const SPACING: f32 = 2.0;
/// 初始块左上角 x(屏幕像素)。
pub const ORIGIN_X: f32 = 5.0;
/// 初始块左上角 y(屏幕像素)。
pub const ORIGIN_Y: f32 = 2.0;

/// SPH 光滑核半径 `h`。
pub const SMOOTHING_H: f32 = 4.0;
/// 粒子质量。
pub const MASS: f32 = 1.0;
/// 静息密度 `rho0`。
pub const REST_DENSITY: f32 = 1.0;
/// 压力刚度 `k`(状态方程 `p = k*(rho - rho0)`)。
pub const STIFFNESS: f32 = 6.0;
/// 黏度系数 `mu`。
pub const VISCOSITY: f32 = 0.6;
/// 重力加速度(屏幕 +y 向下)。
pub const GRAVITY: f32 = 12.0;
/// 仿真时间步长。
pub const DT: f32 = 0.02;
/// 每帧积分子步数。
pub const SUBSTEPS: u32 = 4;
/// 边界速度阻尼(碰撞反弹保留比)。
pub const BOUNDARY_DAMP: f32 = 0.4;
/// 速度大小上限(确定性防发散)。
pub const MAX_SPEED: f32 = 18.0;
/// 三角形 billboard 半边长(屏幕像素)。
pub const SPRITE_SIZE: f32 = 1.5;
/// 域内边界余量(像素)。
const MARGIN: f32 = 1.0;

/// 单个流体粒子状态(位置 / 速度,分量 `f32`)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    /// 位置 x(屏幕像素)。
    pub x: f32,
    /// 位置 y(屏幕像素,+y 向下)。
    pub y: f32,
    /// 速度 x。
    pub vx: f32,
    /// 速度 y。
    pub vy: f32,
}

/// 确定性初始粒子布局(`GRID_NX * GRID_NY` 规则网格,零初速)。
#[must_use]
pub fn initial_particles() -> Vec<Particle> {
    let mut ps = Vec::with_capacity((GRID_NX * GRID_NY) as usize);
    for j in 0..GRID_NY {
        for i in 0..GRID_NX {
            ps.push(Particle {
                x: ORIGIN_X + (i as f32) * SPACING,
                y: ORIGIN_Y + (j as f32) * SPACING,
                vx: 0.0,
                vy: 0.0,
            });
        }
    }
    ps
}

/// Poly6 密度核的未归一化形状项 `(h^2 - r^2)^3`(`r <= h`,否则 0)。
///
/// 归一化常数对所有粒子同义,被状态方程刚度 [`STIFFNESS`] 吸收,故此处省略常数仍
/// 保确定性与单调性。
#[must_use]
fn poly6(r2: f32, h2: f32) -> f32 {
    if r2 < h2 {
        let d = h2 - r2;
        d * d * d
    } else {
        0.0
    }
}

/// Spiky 梯度核标量项 `(h - r)^2`(`0 < r <= h`,否则 0);用于压力力。
#[must_use]
fn spiky_grad(r: f32, h: f32) -> f32 {
    if r > 0.0 && r < h {
        let d = h - r;
        d * d
    } else {
        0.0
    }
}

/// 黏度拉普拉斯核标量项 `(h - r)`(`0 < r <= h`,否则 0)。
#[must_use]
fn visc_lap(r: f32, h: f32) -> f32 {
    if r > 0.0 && r < h { h - r } else { 0.0 }
}

/// 逐粒子密度(SPH,确定性升序遍历邻居)。
#[must_use]
fn densities(ps: &[Particle]) -> Vec<f32> {
    let h2 = SMOOTHING_H * SMOOTHING_H;
    let mut rho = vec![0.0f32; ps.len()];
    for (i, pi) in ps.iter().enumerate() {
        let mut acc = 0.0f32;
        for pj in ps {
            let dx = pi.x - pj.x;
            let dy = pi.y - pj.y;
            acc += MASS * poly6(dx * dx + dy * dy, h2);
        }
        rho[i] = acc;
    }
    rho
}

/// 单子步显式积分(半隐式欧拉):密度 → 压力 → 压力/黏度/重力力 → 速度位置更新 →
/// 边界碰撞钳制。纯函数(返回新态),邻居遍历序固定 → 归约序确定。
#[must_use]
pub fn integrate(ps: &[Particle], dt: f32) -> Vec<Particle> {
    let n = ps.len();
    let h = SMOOTHING_H;
    let rho = densities(ps);
    // 压力(状态方程;静息密度归一化吸收常数,只取相对项)。
    let pressure: Vec<f32> = rho
        .iter()
        .map(|&r| STIFFNESS * (r - REST_DENSITY))
        .collect();

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let pi = ps[i];
        let mut fx = 0.0f32;
        let mut fy = 0.0f32;
        for j in 0..n {
            if i == j {
                continue;
            }
            let pj = ps[j];
            let dx = pi.x - pj.x;
            let dy = pi.y - pj.y;
            let r = (dx * dx + dy * dy).sqrt();
            if r > 0.0 && r < h {
                let inv_r = 1.0 / r;
                // 压力力(对称化,Spiky 梯度沿 i←j 方向)。
                let p_term = (pressure[i] + pressure[j]) * 0.5 * spiky_grad(r, h);
                fx += p_term * dx * inv_r;
                fy += p_term * dy * inv_r;
                // 黏度力(速度差,拉普拉斯核)。
                let lap = VISCOSITY * visc_lap(r, h);
                fx += lap * (pj.vx - pi.vx);
                fy += lap * (pj.vy - pi.vy);
            }
        }
        // 重力(+y 向下)。
        fy += GRAVITY * rho[i].max(REST_DENSITY);

        let inv_m = 1.0 / MASS;
        let mut vx = pi.vx + dt * fx * inv_m;
        let mut vy = pi.vy + dt * fy * inv_m;
        // 速度钳制(确定性防发散)。
        let speed2 = vx * vx + vy * vy;
        let max2 = MAX_SPEED * MAX_SPEED;
        if speed2 > max2 {
            let s = MAX_SPEED / speed2.sqrt();
            vx *= s;
            vy *= s;
        }
        let mut x = pi.x + dt * vx;
        let mut y = pi.y + dt * vy;

        // 边界碰撞(钳制 + 速度反弹阻尼);保粒子常驻屏幕内。
        let x_lo = MARGIN;
        let x_hi = (WIDTH as f32) - MARGIN;
        let y_lo = MARGIN;
        let y_hi = (HEIGHT as f32) - MARGIN;
        if x < x_lo {
            x = x_lo;
            vx = -vx * BOUNDARY_DAMP;
        } else if x > x_hi {
            x = x_hi;
            vx = -vx * BOUNDARY_DAMP;
        }
        if y < y_lo {
            y = y_lo;
            vy = -vy * BOUNDARY_DAMP;
        } else if y > y_hi {
            y = y_hi;
            vy = -vy * BOUNDARY_DAMP;
        }
        out.push(Particle { x, y, vx, vy });
    }
    out
}

/// 推进一帧(`SUBSTEPS` 个固定子步)。
#[must_use]
pub fn step_frame(ps: &[Particle]) -> Vec<Particle> {
    let mut cur = ps.to_vec();
    for _ in 0..SUBSTEPS {
        cur = integrate(&cur, DT);
    }
    cur
}

/// 速度 → 颜色(慢=冷蓝 / 快=暖橙;R、B 通道恒不等 → 帧通道非对称,确定性)。
#[must_use]
pub fn speed_color(vx: f32, vy: f32) -> [f32; 3] {
    let speed = (vx * vx + vy * vy).sqrt();
    let t = (speed / 8.0).clamp(0.0, 1.0);
    // R、B 区间不相交(r ∈ [0.10,0.45] < b ∈ [0.60,0.95]),保 R != B 恒成立
    // → 篡改 R/B 通道序必改字节(反 YAML-only);t 增大暖色升、冷色降,体现速度。
    let r = 0.10 + 0.35 * t;
    let g = 0.20 + 0.50 * t;
    let b = 0.95 - 0.35 * t;
    [r, g, b]
}

/// 粒子态 → 屏幕空间三角形列表(每粒子一个 billboard,RXS-0118~0119 输入)。
///
/// 顶点绕序使 [`soft_raster`] 边函数二倍面积为正(覆盖判定生效);深度按粒子下标
/// 单调分配(低下标更近),令重叠区合成序确定(RXS-0120)。
#[must_use]
pub fn particles_to_tris(ps: &[Particle]) -> Vec<Tri> {
    let n = ps.len().max(1) as f32;
    let s = SPRITE_SIZE;
    ps.iter()
        .enumerate()
        .map(|(i, p)| {
            let z = 0.25 + 0.5 * (i as f32) / n;
            let [r, g, b] = speed_color(p.vx, p.vy);
            let mk = |x: f32, y: f32| Vertex { x, y, z, r, g, b };
            Tri {
                v: [mk(p.x - s, p.y - s), mk(p.x + s, p.y - s), mk(p.x, p.y + s)],
            }
        })
        .collect()
}

/// 渲染单帧粒子态 → 行主序 HDR RGB 帧(完整软光栅
/// binning → tile 光栅 → depth 管线，分量 `0…1`)。
#[must_use]
pub fn render_particles_hdr(ps: &[Particle]) -> Vec<[f32; 3]> {
    let tris = particles_to_tris(ps);
    render_hdr(&tris)
}

/// 渲染单帧粒子态 → image-io `ImageBuffer<Rgb>`(软光栅 HDR → tonemap)。
#[must_use]
pub fn render_particles(ps: &[Particle]) -> ImageBuffer<Rgb> {
    let hdr = render_particles_hdr(ps);
    tonemap_frame(&hdr)
}

/// 从固定初值起跑 `frames` 帧,产确定性图像序列(SPH → 软光栅 → tonemap)。
#[must_use]
pub fn render_sequence(frames: u32) -> Vec<ImageBuffer<Rgb>> {
    let mut ps = initial_particles();
    let mut out = Vec::with_capacity(frames as usize);
    for _ in 0..frames {
        out.push(render_particles(&ps));
        ps = step_frame(&ps);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image_io::{ImageFormat, encode};

    // 初始布局确定性:粒子数 = 网格、首粒子在原点、零初速。
    #[test]
    fn initial_layout_is_deterministic() {
        let a = initial_particles();
        let b = initial_particles();
        assert_eq!(a, b);
        assert_eq!(a.len(), (GRID_NX * GRID_NY) as usize);
        assert_eq!(
            a[0],
            Particle {
                x: ORIGIN_X,
                y: ORIGIN_Y,
                vx: 0.0,
                vy: 0.0
            }
        );
    }

    // 仿真稳定性:积分多帧后粒子常驻屏幕内、无 NaN/Inf(边界钳制 + 速度钳制)。
    #[test]
    fn simulation_stays_on_screen_and_finite() {
        let mut ps = initial_particles();
        for _ in 0..40 {
            ps = step_frame(&ps);
        }
        for p in &ps {
            assert!(p.x.is_finite() && p.y.is_finite(), "粒子坐标须有限");
            assert!(p.vx.is_finite() && p.vy.is_finite(), "粒子速度须有限");
            assert!(p.x >= 0.0 && p.x <= WIDTH as f32, "x 须在屏幕域内");
            assert!(p.y >= 0.0 && p.y <= HEIGHT as f32, "y 须在屏幕域内");
        }
    }

    // 颜色通道非对称:速度着色下 R != B(确保篡改 R/B 通道序必改字节,反 YAML-only)。
    #[test]
    fn speed_color_channels_asymmetric() {
        for &(vx, vy) in &[(0.0f32, 0.0f32), (4.0, 0.0), (0.0, 12.0), (6.0, 6.0)] {
            let [r, _g, b] = speed_color(vx, vy);
            assert!(
                (r - b).abs() > 1e-3,
                "R/B 通道须非对称(speed_color),实得 r={r} b={b}"
            );
        }
    }

    //@ spec: RXS-0119
    // billboard 三角形可被软光栅覆盖:粒子中心像素落在其 billboard 内 → 帧非空。
    #[test]
    fn particles_render_visible_pixels() {
        let ps = initial_particles();
        let frame = render_particles(&ps);
        // 至少一个像素非背景黑(粒子被光栅化覆盖)。
        let mut any_lit = false;
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                let px = frame.get(x, y).unwrap();
                if px.r > 0.0 || px.g > 0.0 || px.b > 0.0 {
                    any_lit = true;
                }
            }
        }
        assert!(any_lit, "首帧应有被粒子覆盖的非背景像素");
    }

    // 实时呈现复用的 HDR 路径必须与离屏 `render_particles` 逐像素同义。
    #[test]
    fn hdr_scene_matches_offscreen_frame() {
        let ps = initial_particles();
        let hdr = render_particles_hdr(&ps);
        let frame = render_particles(&ps);
        assert_eq!(hdr.len(), (WIDTH * HEIGHT) as usize);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let i = (y * WIDTH + x) as usize;
                let px = frame.get(x, y).unwrap();
                assert_eq!(hdr[i], [px.r, px.g, px.b]);
            }
        }
    }

    //@ spec: RXS-0121
    // 确定性图像序列:同初值两次跑序列 → 逐帧 PPM 编码逐字节一致(G-M7-1 口径)。
    #[test]
    fn sequence_is_byte_deterministic() {
        let seq_a = render_sequence(6);
        let seq_b = render_sequence(6);
        assert_eq!(seq_a.len(), 6);
        for (a, b) in seq_a.iter().zip(seq_b.iter()) {
            let ba = encode(a, ImageFormat::Ppm).unwrap();
            let bb = encode(b, ImageFormat::Ppm).unwrap();
            assert_eq!(ba, bb, "同初值两次序列编码应逐字节一致(确定性)");
            assert!(!ba.is_empty());
        }
        // 序列含运动:并非所有帧逐字节相同(粒子随重力演化)。
        let f0 = encode(&seq_a[0], ImageFormat::Ppm).unwrap();
        let f5 = encode(&seq_a[5], ImageFormat::Ppm).unwrap();
        assert_ne!(f0, f5, "序列应体现粒子运动(首末帧不应逐字节相同)");
    }
}
