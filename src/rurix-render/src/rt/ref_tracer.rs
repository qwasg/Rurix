//! CPU 参考效果(报告4 §3.2 P0 效果;RFC-0016 章 F 对拍金标准)。
//!
//! **本文件签名与语义 = W2-J device(Vulkan ray query)效果的对拍契约**:device 腿
//! 实现必须在同一 TLAS、同一 GBuffer 输入、同一采样预算/seed 下,产出与本参考
//! 逐元素一致(容差 = 浮点结合序)的结果。语义锚点:
//!
//! - [`rtao_reference`] — 法线半球**余弦加权**采样,任一采样在 `(0, radius)`
//!   内命中即记遮蔽,AO = 1 − 遮蔽率;射线原点沿法线偏移 [`RAY_EPS`] 防自交;
//!   RNG = [`Pcg32`](确定性,同 seed 同序列,逐像素逐采样顺序消费)。
//! - [`hard_shadow_reference`] — 自着色点沿「指向光源方向」发一根阴影光线,
//!   `(0, +∞)` 任一命中即 0(影),否则 1(亮);原点沿光方向偏移 [`RAY_EPS`]。
//! - 无效像素约定(两效果一致):位置任一分量非有限(NaN/±inf = 无几何像素)
//!   产 1.0;RTAO 额外将零长/非有限法线判无效产 1.0;`samples_per_pixel = 0`
//!   与零长光方向退化为全 1(无采样/无方向即无遮蔽)。

use crate::rt::bvh::{BlasSet, Ray, Tlas, Vec3};

/// 射线原点偏移量(自交规避;对拍契约常量,device 腿必须同值)。
pub const RAY_EPS: f32 = 1e-3;

// ---------------------------------------------------------------------------
// PCG32:确定性 RNG(对拍契约:同 seed 同序列,跨平台位级一致)
// ---------------------------------------------------------------------------

/// PCG32 伪随机数发生器(pcg 基本方案;64 位状态 32 位输出,固定流)。
/// 确定性承诺:同 `seed` 产同一位级序列,与平台/线程无关。
#[derive(Debug, Clone)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    /// 以 `seed` 初始化(标准 pcg 播种序列;固定流常量,跨调用确定)。
    pub fn new(seed: u64) -> Self {
        let mut rng = Pcg32 {
            state: 0,
            inc: (0xda3e_39cb_94b9_5bdb_u64 << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// 下一个 32 位伪随机数。
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// 下一个均匀浮点,区间 `[0, 1)`(高 24 位 / 2²⁴,确定性)。
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

// ---------------------------------------------------------------------------
// 采样工具(确定性;全部经 Pcg32 喂入)
// ---------------------------------------------------------------------------

/// 法线半球余弦加权采样方向(单位长)。`r1, r2 ∈ [0,1)` 均匀输入。
/// 余弦加权:pdf ∝ cos θ,方向 = t·x + b·y + n·z(z = √(1−r2))。
///
/// **可见性(G7.4 W3c 加性,数值语义 0-byte)**:自私有升 `pub`,使 device 对拍
/// harness 能与 [`rtao_reference`] 消费**同一函数实例**生成采样方向(RTAO device
/// kernel 的方向 buffer 为 host 同源输入,见 `apps/uc06-renderer/kernels/rtao.rx`
/// 头注 provenance 段)——避免在 harness 侧产生第四份采样公式。函数体、运算序与
/// 返回值**逐字不变**;仅 `pub` 修饰符与本段文档为增量。
pub fn cosine_sample_hemisphere(n: Vec3, r1: f32, r2: f32) -> Vec3 {
    let (t, b) = orthonormal_basis(n);
    let phi = 2.0 * core::f32::consts::PI * r1;
    let r = r2.sqrt();
    let x = r * phi.cos();
    let y = r * phi.sin();
    let z = (1.0 - r2).max(0.0).sqrt();
    (t * x + b * y + n * z).normalize()
}

/// 由法线构造正交基(确定性分支;`t ⟂ n`,`b = n × t` 已单位长)。
fn orthonormal_basis(n: Vec3) -> (Vec3, Vec3) {
    let up = if n.y.abs() < 0.999 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let t = up.cross(n).normalize();
    let b = n.cross(t);
    (t, b)
}

// ---------------------------------------------------------------------------
// 参考效果(对拍契约签名)
// ---------------------------------------------------------------------------

/// RTAO 参考(GBuffer 法线半球余弦加权采样,any_hit 遮蔽率)。
///
/// **对拍契约(W2-J device 效果金标准)**:
/// - `positions` / `normals`:GBuffer 世界空间位置/法线图,逐像素一一对应,
///   尺寸必须相等(不等即 panic,调用契约);
/// - `tlas` + `blases`:场景两级加速结构(与 device 同一份几何输入);
/// - `samples_per_pixel`:每像素采样数;`radius`:遮蔽判定距离(命中区间上界,
///   开区间);`seed`:PCG32 种子——同 seed 同输入产逐元素相同输出;
/// - 输出:逐像素 AO ∈ [0, 1](1 = 无遮蔽);无效像素产 1.0(见模块文档约定)。
///
/// 像素处理顺序 = 图序,RNG 单流顺序消费(逐像素逐采样),保证确定性。
pub fn rtao_reference<B: BlasSet + ?Sized>(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    tlas: &Tlas,
    blases: &B,
    samples_per_pixel: u32,
    radius: f32,
    seed: u64,
) -> Vec<f32> {
    assert_eq!(
        positions.len(),
        normals.len(),
        "rtao_reference: positions/normals 图尺寸必须一致"
    );
    let mut rng = Pcg32::new(seed);
    let mut out = Vec::with_capacity(positions.len());
    for (p, n) in positions.iter().zip(normals) {
        let p = Vec3::from_array(*p);
        let n = Vec3::from_array(*n);
        if !p.is_finite() || !n.is_finite() || n.length() == 0.0 || samples_per_pixel == 0 {
            // 无效像素 / 零采样:无遮蔽(契约,见模块文档)。
            out.push(1.0);
            continue;
        }
        let n = n.normalize();
        let origin = p + n * RAY_EPS;
        let mut occluded = 0u32;
        for _ in 0..samples_per_pixel {
            let r1 = rng.next_f32();
            let r2 = rng.next_f32();
            let dir = cosine_sample_hemisphere(n, r1, r2);
            if tlas.any_hit(blases, &Ray { origin, dir }, radius) {
                occluded += 1;
            }
        }
        out.push(1.0 - occluded as f32 / samples_per_pixel as f32);
    }
    out
}

/// RT 硬阴影参考(向光源一根光线,输出 0/1 可见性)。
///
/// **对拍契约(W2-J device 效果金标准)**:
/// - `positions`:GBuffer 世界空间位置图(无效像素 = 任一分量非有限,产 1.0);
/// - `light_dir`:指向光源的方向(无需归一,内部归一化;零长/非有限方向退化
///   为全 1,无方向即无遮蔽);
/// - 输出:逐像素可见性(1 = 亮,0 = 影);命中区间 `(0, +∞)`,原点沿光方向
///   偏移 [`RAY_EPS`];边界精度 = 遮挡网格的精确投影(双面命中,无 front-face
///   剔除)。
pub fn hard_shadow_reference<B: BlasSet + ?Sized>(
    positions: &[[f32; 3]],
    light_dir: [f32; 3],
    tlas: &Tlas,
    blases: &B,
) -> Vec<f32> {
    let l = Vec3::from_array(light_dir);
    let degenerate = !l.is_finite() || l.length() == 0.0;
    let dir = l.normalize();
    positions
        .iter()
        .map(|p| {
            let p = Vec3::from_array(*p);
            if degenerate || !p.is_finite() {
                return 1.0;
            }
            let origin = p + dir * RAY_EPS;
            if tlas.any_hit(blases, &Ray { origin, dir }, f32::INFINITY) {
                0.0
            } else {
                1.0
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::bvh::{InstanceDesc, Transform3x4, TriBvh};

    /// z=0 → y=0 平面四边形辅助:x,z ∈ [x0,x1]×[z0,z1],位于高度 y。
    fn quad_y(x0: f32, x1: f32, z0: f32, z1: f32, y: f32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        (
            vec![[x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1]],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    /// x=0 墙面四边形:y,z ∈ [y0,y1]×[z0,z1]。
    fn quad_x(y0: f32, y1: f32, z0: f32, z1: f32, x: f32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        (
            vec![[x, y0, z0], [x, y1, z0], [x, y1, z1], [x, y0, z1]],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    /// z=0 墙面四边形:x,y ∈ [x0,x1]×[y0,y1]。
    fn quad_z(x0: f32, x1: f32, y0: f32, y1: f32, z: f32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        (
            vec![[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    /// 三面直角墙角场景:地板 y=0(x,z ∈ [0,2])、墙 x=0(y,z ∈ [0,2])、
    /// 墙 z=0(x,y ∈ [0,2])。着色点 (0.2, 0.2, 0.2) 距三面各 0.2,法线沿角平分线
    /// (1,1,1)/√3——任一分量明显为负的采样方向即在近处命中对应墙面,遮蔽稳健显著。
    fn trihedral_corner() -> (Vec<TriBvh>, Tlas) {
        let floor = quad_y(0.0, 2.0, 0.0, 2.0, 0.0);
        let wall_x = quad_x(0.0, 2.0, 0.0, 2.0, 0.0);
        let wall_z = quad_z(0.0, 2.0, 0.0, 2.0, 0.0);
        let (pos, idx) = merge(&[floor, wall_x, wall_z]);
        scene_of(&pos, &idx)
    }

    /// 网格(位置 + 索引)。
    type Mesh = (Vec<[f32; 3]>, Vec<[u32; 3]>);

    /// 把多份网格合并为单一 BLAS(共享顶点数组)。
    fn merge(meshes: &[Mesh]) -> Mesh {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<[u32; 3]> = Vec::new();
        for (pos, idx) in meshes {
            let base = positions.len() as u32;
            positions.extend(pos.iter().copied());
            indices.extend(idx.iter().map(|t| [t[0] + base, t[1] + base, t[2] + base]));
        }
        (positions, indices)
    }

    /// 单 BLAS 单实例(恒等变换)场景。
    fn scene_of(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> (Vec<TriBvh>, Tlas) {
        let blases = vec![TriBvh::build(positions, indices)];
        let tlas = Tlas::build(
            &[InstanceDesc {
                blas: 0,
                transform: Transform3x4::IDENTITY,
                mask: 0xFF,
                flags: 0,
            }],
            &blases,
        );
        (blases, tlas)
    }

    #[test]
    fn pcg32_deterministic_sequence() {
        let mut a = Pcg32::new(42);
        let mut b = Pcg32::new(42);
        for _ in 0..16 {
            assert_eq!(a.next_u32(), b.next_u32(), "同 seed 同序列");
        }
        let mut c = Pcg32::new(7);
        for _ in 0..16 {
            let v = c.next_f32();
            assert!((0.0..1.0).contains(&v), "值域 [0,1)");
        }
    }

    #[test]
    fn rtao_open_plane_is_fully_lit() {
        // 平面上方无遮挡:地板 x,z ∈ [−2,2],着色点 (0, 0.5, 0) 法线 +y。
        // 半球采样恒有 y 分量 > 0,不可能命中地板 → AO 精确 1。
        let (pos, idx) = quad_y(-2.0, 2.0, -2.0, 2.0, 0.0);
        let (blases, tlas) = scene_of(&pos, &idx);
        let ao = rtao_reference(
            &[[0.0, 0.5, 0.0]],
            &[[0.0, 1.0, 0.0]],
            &tlas,
            &blases,
            8,
            1.0,
            1,
        );
        assert_eq!(ao, vec![1.0]);
    }

    #[test]
    fn rtao_corner_occludes_significantly() {
        // 三面直角墙角:点 (0.2, 0.2, 0.2),法线 (1,1,1)/√3(角平分线)。
        let (blases, tlas) = trihedral_corner();
        let s = 1.0 / 3.0f32.sqrt();
        let positions = [[0.2, 0.2, 0.2]];
        let normals = [[s, s, s]];
        let ao = rtao_reference(&positions, &normals, &tlas, &blases, 32, 1.0, 7);
        assert_eq!(ao.len(), 1);
        assert!(ao[0] < 0.9, "墙角应显著遮蔽,AO={}", ao[0]);
        assert!(ao[0] > 0.05, "半球大半仍开阔,AO={}", ao[0]);
        // 确定性:同 seed 同输入逐元素相等。
        let ao2 = rtao_reference(&positions, &normals, &tlas, &blases, 32, 1.0, 7);
        assert_eq!(ao, ao2);
    }

    #[test]
    fn rtao_radius_bounds_occlusion() {
        // 同墙角场景,radius 小于到三面墙距离(0.2):遮蔽不可能发生 → AO 精确 1。
        let (blases, tlas) = trihedral_corner();
        let s = 1.0 / 3.0f32.sqrt();
        let ao = rtao_reference(
            &[[0.2, 0.2, 0.2]],
            &[[s, s, s]],
            &tlas,
            &blases,
            32,
            0.05,
            7,
        );
        assert_eq!(ao, vec![1.0], "radius 内无遮挡物则 AO=1");
    }

    #[test]
    fn rtao_invalid_pixels_are_one() {
        let (pos, idx) = quad_y(-2.0, 2.0, -2.0, 2.0, 0.0);
        let (blases, tlas) = scene_of(&pos, &idx);
        let positions = [
            [f32::NAN, 0.0, 0.0], // NaN 位置 = 无几何像素
            [0.0, 0.5, 0.0],      // 零长法线 = 无效
            [0.0, 0.5, 0.0],      // 有效(对照)
        ];
        let normals = [[0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let ao = rtao_reference(&positions, &normals, &tlas, &blases, 4, 1.0, 3);
        assert_eq!(ao[0], 1.0);
        assert_eq!(ao[1], 1.0);
        assert_eq!(ao[2], 1.0, "开阔地板上方对照像素亦全亮");
        // 零采样退化:全 1。
        let ao0 = rtao_reference(&positions, &normals, &tlas, &blases, 0, 1.0, 3);
        assert_eq!(ao0, vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn hard_shadow_plate_projection_exact() {
        // 地板 y=0(x,z ∈ [−4,4])为接收面;遮挡板 y=1,x ∈ [0,1],z ∈ [0,1]。
        // 光方向 = normalize(−0.5, 1, 0)(指向光源):dx/dy = −0.5,
        // 地板点 (px, 0, pz) 的阴影光线在板高度处 x_at = px − 0.5;
        // x_at ∈ [0,1] 且 pz ∈ [0,1] → 影(0),否则亮(1)。
        let floor = quad_y(-4.0, 4.0, -4.0, 4.0, 0.0);
        let plate = quad_y(0.0, 1.0, 0.0, 1.0, 1.0);
        let (pos, idx) = merge(&[floor, plate]);
        let (blases, tlas) = scene_of(&pos, &idx);
        let cases: Vec<([f32; 3], f32)> = vec![
            ([0.55, 0.0, 0.5], 0.0), // 投影 x=0.05,板内(近边界)
            ([0.6, 0.0, 0.5], 0.0),  // 投影 0.1
            ([1.4, 0.0, 0.5], 0.0),  // 投影 0.9
            ([1.45, 0.0, 0.5], 0.0), // 投影 0.95(近边界)
            ([0.4, 0.0, 0.5], 1.0),  // 投影 −0.1,板外
            ([1.6, 0.0, 0.5], 1.0),  // 投影 1.1,板外
            ([0.6, 0.0, 1.5], 1.0),  // z 出板范围
            ([0.6, 0.0, -1.5], 1.0), // z 出板范围(负向)
        ];
        let positions: Vec<[f32; 3]> = cases.iter().map(|c| c.0).collect();
        let expected: Vec<f32> = cases.iter().map(|c| c.1).collect();
        let vis = hard_shadow_reference(&positions, [-0.5, 1.0, 0.0], &tlas, &blases);
        assert_eq!(vis, expected, "边界精确到网格投影");
    }

    #[test]
    fn hard_shadow_invalid_inputs_are_lit() {
        let (pos, idx) = quad_y(-2.0, 2.0, -2.0, 2.0, 0.0);
        let (blases, tlas) = scene_of(&pos, &idx);
        // NaN 位置 = 无几何像素 → 1。
        let vis = hard_shadow_reference(&[[f32::NAN, 0.0, 0.0]], [0.0, 1.0, 0.0], &tlas, &blases);
        assert_eq!(vis, vec![1.0]);
        // 零长光方向 → 全 1(无方向即无遮蔽)。
        let vis = hard_shadow_reference(&[[0.0, 0.0, 0.0]], [0.0, 0.0, 0.0], &tlas, &blases);
        assert_eq!(vis, vec![1.0]);
    }

    #[test]
    fn shadow_ray_no_self_intersection_on_receiver() {
        // 无遮挡物时接收面(地板)上的点全亮:验证 RAY_EPS 自交规避
        // (沿光方向偏移后,接收面自身 hit t < 0 不被接受)。
        let (pos, idx) = quad_y(-4.0, 4.0, -4.0, 4.0, 0.0);
        let (blases, tlas) = scene_of(&pos, &idx);
        let positions: Vec<[f32; 3]> = (0..8).map(|i| [i as f32 - 3.5, 0.0, 0.5]).collect();
        let vis = hard_shadow_reference(&positions, [-0.5, 1.0, 0.0], &tlas, &blases);
        assert_eq!(vis, vec![1.0; 8]);
    }
}
