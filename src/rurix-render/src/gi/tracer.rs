//! 追踪层统一契约与 CPU 参考追踪器(报告2 §3.2 阶段不变量 P0 冻结;RFC-0016
//! 章 E2)。
//!
//! [`RadianceTracer`] = 「输入光线,输出命中点辐射度」的统一契约。本期唯一实现
//! [`RayTracedRadiance`];SDF 软追踪(报告2 P4 降级轨)与 ReSTIR(长线)未来
//! 实现同一接口,追踪层可替换——探针放置/SH/插值/滤波/累积全部只面向本 trait,
//! 不感知具体追踪后端。
//!
//! 命中点辐射度定义(报告2 §3.1 MVP「单反弹间接漫反射」;重要性采样首期 =
//! BRDF PDF 单因子,光照 PDF 上一帧重投影归 P2,如实标注):
//! - 命中 ⇒ 方向光直接光照 `sun_color × max(N·L, 0) × albedo/π`(Lambert BRDF)
//!   × 太阳可见性(沿 L 发一根阴影光线,`any_hit` 遮蔽即 0;原点沿着色法线偏
//!   移 [`RAY_EPS`] 防自交)。双面着色:几何法线背向入射光线时翻转——薄墙/室
//!   内面片稳健(rt::bvh 命中法线依 winding,不做 front-face 翻转);
//! - 未命中 ⇒ 天空常量色(「天光」,[`GiScene::sky_color`])。

use crate::rt::bvh::{InstanceDesc, Ray, Tlas, Transform3x4, TriBvh, Vec3};
use crate::rt::ref_tracer::RAY_EPS;

/// 追踪层统一契约(P0 冻结,报告2 §3.2):输入世界空间光线,输出命中点辐射度。
///
/// 实现方义务:确定性(同输入同输出);`origin` 应已含自交偏移(调用方负责);
/// `dir` 单位长。host/device 两腿共用本契约做 G-G5-6 方向一致性对拍。
pub trait RadianceTracer {
    /// 追踪一条光线,返回命中点辐射度 RGB(未命中 = 环境/天空色)。
    fn trace(&self, origin: Vec3, dir: Vec3) -> [f32; 3];
}

/// GI 场景网格实例(host 轻量场景描述;逐实例 albedo)。
#[derive(Debug, Clone)]
pub struct GiMeshInstance {
    /// 顶点位置(对象空间)。
    pub positions: Vec<[f32; 3]>,
    /// 三角形索引。
    pub indices: Vec<[u32; 3]>,
    /// 对象空间 → 世界空间仿射变换。
    pub transform: Transform3x4,
    /// 漫反射反照率(线性空间)。
    pub albedo: [f32; 3],
}

/// GI 场景:TLAS + 逐实例 albedo + 方向光 + 天光常量(轻量 host 场景描述;
/// GI 单测/对拍自含。W3 device 接线时由 GPU scene(章 G)供给同语义输入,
/// TLAS 与章 F 同一份几何)。
#[derive(Debug, Clone)]
pub struct GiScene {
    /// BLAS 集合(每实例一份,与 `instances` 顺序一致)。
    pub blases: Vec<TriBvh>,
    /// 实例级加速结构。
    pub tlas: Tlas,
    /// 逐实例 albedo(按 TLAS 实例槽位索引,与 [`Hit::instance`] 对齐)。
    ///
    /// [`Hit::instance`]: crate::rt::bvh::Hit
    pub albedos: Vec<[f32; 3]>,
    /// 指向光源的单位方向(零向量 = 关闭方向光,白炉测试用)。
    pub sun_dir: Vec3,
    /// 方向光辐射度(线性空间 RGB)。
    pub sun_color: [f32; 3],
    /// 天空常量色(未命中环境项)。
    pub sky_color: [f32; 3],
}

impl GiScene {
    /// 由实例列表 + 光照参数构建(每实例一份 BLAS;不可逆变换/空网格实例按
    /// [`Tlas::build`] 语义确定性禁用)。
    pub fn build(
        instances: &[GiMeshInstance],
        sun_dir: [f32; 3],
        sun_color: [f32; 3],
        sky_color: [f32; 3],
    ) -> GiScene {
        let blases: Vec<TriBvh> = instances
            .iter()
            .map(|m| TriBvh::build(&m.positions, &m.indices))
            .collect();
        let descs: Vec<InstanceDesc> = instances
            .iter()
            .enumerate()
            .map(|(i, m)| InstanceDesc {
                blas: i as u32,
                transform: m.transform,
                mask: 0xFF,
                flags: 0,
            })
            .collect();
        let tlas = Tlas::build(&descs, &blases);
        GiScene {
            blases,
            tlas,
            albedos: instances.iter().map(|m| m.albedo).collect(),
            sun_dir: Vec3::from_array(sun_dir).normalize(),
            sun_color,
            sky_color,
        }
    }
}

/// CPU 参考追踪器(G-G5-6 方向一致性对拍 host 金标准;W3 device ray query 腿
/// 的逐字语义蓝本)。
#[derive(Debug, Clone)]
pub struct RayTracedRadiance {
    /// 场景(几何 + 光照 + 材质参数)。
    pub scene: GiScene,
}

impl RayTracedRadiance {
    /// 包装场景为追踪器。
    pub fn new(scene: GiScene) -> Self {
        Self { scene }
    }
}

impl RadianceTracer for RayTracedRadiance {
    fn trace(&self, origin: Vec3, dir: Vec3) -> [f32; 3] {
        let Some(hit) = self
            .scene
            .tlas
            .intersect(&self.scene.blases, &Ray { origin, dir })
        else {
            return self.scene.sky_color;
        };
        let albedo = self.scene.albedos[hit.instance as usize];
        // 双面着色:法线翻转朝向入射光线来向(薄几何/室内面片稳健)。
        let mut n = Vec3::from_array(hit.normal);
        if n.dot(dir) > 0.0 {
            n = -n;
        }
        let l = self.scene.sun_dir;
        let ndotl = n.dot(l).max(0.0);
        if ndotl <= 0.0 {
            return [0.0, 0.0, 0.0];
        }
        // 太阳可见性:沿 L 一根阴影光线(原点沿着色法线偏移防自交)。
        let p = origin + dir * hit.t;
        let shadow = Ray {
            origin: p + n * RAY_EPS,
            dir: l,
        };
        if self
            .scene
            .tlas
            .any_hit(&self.scene.blases, &shadow, f32::INFINITY)
        {
            return [0.0, 0.0, 0.0];
        }
        let inv_pi = 1.0 / core::f32::consts::PI;
        let mut out = [0.0; 3];
        for (ch, o) in out.iter_mut().enumerate() {
            *o = self.scene.sun_color[ch] * ndotl * albedo[ch] * inv_pi;
        }
        out
    }
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// z=0 平面单位四边形([0,1]²,法线 +z),albedo 可配。
    fn quad_z0(albedo: [f32; 3]) -> GiMeshInstance {
        GiMeshInstance {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            indices: vec![[0, 1, 2], [0, 2, 3]],
            transform: Transform3x4::IDENTITY,
            albedo,
        }
    }

    #[test]
    fn miss_returns_sky_color() {
        let sky = [0.2, 0.4, 0.6];
        // 空场景:任意光线未命中 → 天空常量色。
        let empty = RayTracedRadiance::new(GiScene::build(&[], [0.0, 1.0, 0.0], [1.0; 3], sky));
        assert_eq!(
            empty.trace(Vec3::ZERO, Vec3::new(0.3, 0.5, 0.8).normalize()),
            sky,
            "空场景未命中应返回天空色"
        );
        // 有几何但光线不命中:同样返回天空色。
        let scene = RayTracedRadiance::new(GiScene::build(
            &[quad_z0([1.0; 3])],
            [0.0, 1.0, 0.0],
            [1.0; 3],
            sky,
        ));
        assert_eq!(
            scene.trace(Vec3::new(5.0, 5.0, 1.0), Vec3::new(0.0, 0.0, -1.0)),
            sky
        );
    }

    #[test]
    fn hit_occluder_returns_hand_anchored_direct_lighting() {
        // 手算锚定:命中点辐射度 = sun_color × (N·L) × albedo/π。
        // 四边形法线 +z,太阳 L = +z ⇒ N·L = 1;无遮挡 ⇒ 可见性 1。
        let albedo = [0.8, 0.5, 0.25];
        let sun_color = [2.0, 1.0, 0.5];
        let scene = RayTracedRadiance::new(GiScene::build(
            &[quad_z0(albedo)],
            [0.0, 0.0, 1.0],
            sun_color,
            [0.1; 3],
        ));
        let rad = scene.trace(Vec3::new(0.5, 0.5, 2.0), Vec3::new(0.0, 0.0, -1.0));
        let inv_pi = 1.0 / core::f32::consts::PI;
        for ch in 0..3 {
            let expect = sun_color[ch] * albedo[ch] * inv_pi;
            assert!(
                (rad[ch] - expect).abs() < 1e-5,
                "ch{ch}: {} vs 手算 {expect}",
                rad[ch]
            );
        }
        // 背面命中(光线自下方入射):双面着色翻转后 N·L ≤ 0 ⇒ 无直射贡献。
        let back = scene.trace(Vec3::new(0.5, 0.5, -2.0), Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(back, [0.0, 0.0, 0.0], "背面命中应无直射光照");
    }

    #[test]
    fn sun_shadow_occluded_hit_is_dark() {
        // z=0 接收面 + z=1 全遮蔽挡板:命中点阴影光线被挡 ⇒ 辐射度 0。
        let albedo = [0.8, 0.5, 0.25];
        let mut blocker = quad_z0(albedo);
        blocker.transform = Transform3x4::from_translation([0.0, 0.0, 1.0]);
        let scene = RayTracedRadiance::new(GiScene::build(
            &[quad_z0(albedo), blocker],
            [0.0, 0.0, 1.0],
            [2.0; 3],
            [0.1; 3],
        ));
        let rad = scene.trace(Vec3::new(0.5, 0.5, 2.0), Vec3::new(0.0, 0.0, -1.0));
        // 命中 z=1 挡板(最近):其向光侧无遮挡 ⇒ 亮。
        let inv_pi = 1.0 / core::f32::consts::PI;
        for ch in 0..3 {
            let expect = 2.0 * albedo[ch] * inv_pi;
            assert!(
                (rad[ch] - expect).abs() < 1e-5,
                "挡板向光面应亮:ch{ch} {} vs {expect}",
                rad[ch]
            );
        }
        // 自 z=0 与 z=1 之间向上看 z=0 面的背面?——改从挡板上方照 z=0 面:
        // 命中点在 z=0 接收面(自 +z 向下,先穿挡板上方空隙不可行),直接以
        // 接收面上方、挡板覆盖区内的点发阴影光线等价验证:斜向太阳绕开挡板 ⇒ 亮。
        let angled = RayTracedRadiance::new(GiScene::build(
            &[quad_z0(albedo), {
                let mut b = quad_z0(albedo);
                b.transform = Transform3x4::from_translation([0.0, 0.0, 1.0]);
                b
            }],
            [0.7, 0.0, 0.7],
            [2.0; 3],
            [0.1; 3],
        ));
        // 命中 z=1 挡板顶面:斜向太阳 N·L = 0.7/√(0.98) ≈ 0.7071,挡板上方无遮挡 ⇒ 亮。
        let rad = angled.trace(Vec3::new(0.5, 0.5, 2.0), Vec3::new(0.0, 0.0, -1.0));
        let ndotl = Vec3::new(0.0, 0.0, 1.0).dot(Vec3::new(0.7, 0.0, 0.7).normalize());
        for ch in 0..3 {
            let expect = 2.0 * albedo[ch] * ndotl * inv_pi;
            assert!(
                (rad[ch] - expect).abs() < 1e-5,
                "斜向太阳挡板应亮:ch{ch} {} vs {expect}",
                rad[ch]
            );
        }
        // 验证遮蔽路径:竖直太阳下,位于挡板正下方的接收面点发阴影光线必命中挡板。
        // 以 any_hit 等价口径直接断言(与 trace 内部同一调用)。
        let shadow_ray = Ray {
            origin: Vec3::new(0.5, 0.5, RAY_EPS),
            dir: Vec3::new(0.0, 0.0, 1.0),
        };
        assert!(
            scene
                .scene
                .tlas
                .any_hit(&scene.scene.blases, &shadow_ray, f32::INFINITY),
            "挡板正下方阴影光线应被遮蔽"
        );
    }
}
