//! 光照求值(G6.3 uc08;照 uc06 同款单层闭合)——单层 principled 材质闭合(32B)
//! 延迟求值 + VSM 阴影采样 + GI 间接 + RTAO 调制。host 参考执行:逐像素从
//! VisBuffer/材质窄缓冲解出 [`MaterialClosure`],以朗伯直接光×VSM 可见性 +
//! GI 探针 irradiance×albedo/π + AO 调制 + 自发光合成 HDR。
//!
//! 与 uc06 的差异:`make_vsm` 改吃**当帧**世界三角形(物理驱动几何每帧变,
//! 深度范围按当帧包络定);PSO 面同律(变体预测器 precache,渲染期零告警)。

use rurix_render::graph::types::MaterialClosure;
use rurix_render::graph::types::TextureFormat;
use rurix_render::material::closure::unpack;
use rurix_render::material::pso_cache::{
    PassShaderTemplate, PsoCache, PsoDesc, predict_precache_list,
};
use rurix_render::material::table::MaterialTable;
use rurix_render::shadow::vsm::{ShadowTri, Vsm, VsmConfig};
use rurix_render::temporal::image::ImageF32;

use crate::scene::{SUN_COLOR, SUN_DIR, VSM_LIGHT_DIR};

/// 单层 principled 朗伯求值(报告6 §2.1 Slab 子集):
/// `color = albedo/π × (sun×shadow + gi_irradiance) × ao + emissive`。
/// 法线 = **世界空间表面法线**(由调用方自 GBuffer 供给)。
pub fn shade_pixel(
    closure: &MaterialClosure,
    world_normal: [f32; 3],
    shadow: f32,
    gi_irradiance: [f32; 3],
    ao: f32,
) -> [f32; 3] {
    let p = unpack(closure);
    let n = world_normal;
    // 指向光源方向 = -SUN_DIR(SUN_DIR = 光线传播方向,太阳→场景);N·L 朗伯项。
    let to_light = normalize([-SUN_DIR[0], -SUN_DIR[1], -SUN_DIR[2]]);
    let ndl = (n[0] * to_light[0] + n[1] * to_light[1] + n[2] * to_light[2]).max(0.0);
    let mut out = [0.0f32; 3];
    for c in 0..3 {
        let direct = SUN_COLOR[c] * ndl * shadow;
        let indirect = gi_irradiance[c];
        let lambert = p.albedo[c] / std::f32::consts::PI * (direct + indirect) * ao;
        out[c] = lambert + p.emissive[c];
    }
    out
}

/// 全屏材质求值(逐像素按材质窄缓冲索引 MaterialTable;无效像素 = 天空色)。
#[allow(clippy::too_many_arguments)]
pub fn shade_frame(
    mat_ids: &[u16],
    invalid: u16,
    w: u32,
    h: u32,
    materials: &MaterialTable,
    normals: &ImageF32,
    shadow_map: &ImageF32,
    gi_irradiance: &ImageF32,
    ao: &ImageF32,
    sky_color: [f32; 3],
) -> ImageF32 {
    let mut hdr = ImageF32::new(w, h, 3);
    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) as usize;
            let mid = mat_ids[idx];
            if mid == invalid {
                hdr.set_pixel3(x, y, sky_color);
                continue;
            }
            let c = &materials.closures()[mid as usize];
            let px = [
                gi_irradiance.get(x, y, 0),
                gi_irradiance.get(x, y, 1),
                gi_irradiance.get(x, y, 2),
            ];
            let n = [
                normals.get(x, y, 0),
                normals.get(x, y, 1),
                normals.get(x, y, 2),
            ];
            let color = shade_pixel(c, n, shadow_map.get(x, y, 0), px, ao.get(x, y, 0));
            hdr.set_pixel3(x, y, color);
        }
    }
    hdr
}

/// VSM 实例(当帧世界三角形驱动;页池预算按场景规模取 512)。
///
/// clipmap 基准半径与深度范围按**当帧**场景包络定(物理驱动几何每帧变):
/// R₀ = 2(覆盖落点近场),depth_extent 按当帧世界 AABB 灯向跨度 +10% 余量
/// (过浅钳出 [0,1] 恒 lit,过深损失精度;停靠实例由调用方排除,见
/// `scene::world_tris_now`)。
pub fn make_vsm(world_tris: &[ShadowTri]) -> Vsm {
    let basis = rurix_render::shadow::clipmap::LightBasis::from_direction(VSM_LIGHT_DIR);
    let mut zmin = f32::INFINITY;
    let mut zmax = f32::NEG_INFINITY;
    for t in world_tris {
        for v in t.v {
            let z = basis.to_light(v)[2];
            zmin = zmin.min(z);
            zmax = zmax.max(z);
        }
    }
    let extent = ((zmax - zmin) * 0.55).max(1.0);
    let cfg = VsmConfig {
        clip: rurix_render::shadow::clipmap::ClipmapConfig {
            levels: 4,
            base_radius: 2.0,
            depth_extent: extent,
        },
        // 池预算 = 512 页(demo 场景标记页远少于此;uc06 同款固定预算纪律)。
        pool_pages: 512,
        ..Default::default()
    };
    Vsm::new(cfg, VSM_LIGHT_DIR, crate::scene::CAMERA.eye)
}

/// PSO 集(变体预测器 + precache;渲染期零告警判据面)。
///
/// pass 模板 = 深度/阴影/velocity/base 四件(报告6 §3 有限集);compile 桩 =
/// 确定性伪编译产物(字符串描述),precache 全量后置告警计数归零。
pub struct PsoSet {
    pub cache: PsoCache<String>,
    #[allow(dead_code)]
    pub templates: Vec<PassShaderTemplate>,
}

pub fn make_pso_set(materials: &MaterialTable) -> PsoSet {
    let templates: Vec<PassShaderTemplate> = vec![
        PassShaderTemplate {
            vs_entry: "depth_vs".into(),
            fs_entry: "depth_fs".into(),
            color_formats: vec![TextureFormat::Depth32Float],
            depth_format: Some(TextureFormat::Depth32Float),
        },
        PassShaderTemplate {
            vs_entry: "shadow_vs".into(),
            fs_entry: "shadow_fs".into(),
            color_formats: vec![TextureFormat::R32Float],
            depth_format: Some(TextureFormat::Depth32Float),
        },
        PassShaderTemplate {
            vs_entry: "velocity_vs".into(),
            fs_entry: "velocity_fs".into(),
            color_formats: vec![TextureFormat::Rg16Float],
            depth_format: Some(TextureFormat::Depth32Float),
        },
        PassShaderTemplate {
            vs_entry: "base_vs".into(),
            fs_entry: "base_fs".into(),
            color_formats: vec![TextureFormat::Rgba16Float, TextureFormat::Rg16Float],
            depth_format: None,
        },
    ];

    let predicted = predict_precache_list(materials.closures(), &templates);
    let mut cache: PsoCache<String> = PsoCache::new();
    cache.precache(&predicted, |d| format!("pso[{:016x}]", d.stable_hash()));
    PsoSet { cache, templates }
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

#[allow(dead_code)]
fn _typecheck_pso_desc(_: &PsoDesc) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pso_precache_covers_predicted_variants_zero_warnings() {
        let s = crate::scene::build_scene();
        let mut set = make_pso_set(&s.materials);
        // 预测集 = 材质(7) × pass(4) = 28 变体,precache 后逐变体命中零告警。
        let predicted = predict_precache_list(s.materials.closures(), &set.templates);
        assert_eq!(predicted.len(), 28, "7 材质 × 4 pass");
        for d in &predicted {
            let _ = set
                .cache
                .get_or_compile(d, |d| format!("late[{:x}]", d.stable_hash()));
        }
        assert_eq!(set.cache.warnings(), 0, "渲染期编译告警必须归零");
    }

    #[test]
    fn shade_pixel_lambert_terms() {
        // 无阴影/无 GI/无 AO 调制:朗伯直接光 × albedo/π,法线朝上 → N·L = to_light.y。
        let s = crate::scene::build_scene();
        let closure = &s.materials.closures()[0];
        let world_n = [0.0, 1.0, 0.0];
        let lit = shade_pixel(closure, world_n, 1.0, [0.0; 3], 1.0);
        let to_light = normalize([-SUN_DIR[0], -SUN_DIR[1], -SUN_DIR[2]]);
        let ndl = to_light[1];
        let p = unpack(closure);
        let _ = &p;
        for c in 0..3 {
            let expect = p.albedo[c] / std::f32::consts::PI * SUN_COLOR[c] * ndl;
            let tol = SUN_COLOR[c] * ndl / std::f32::consts::PI / 255.0 + 1e-5;
            assert!(
                (lit[c] - expect).abs() < tol,
                "朗伯直接光项 {c}: {lit:?} vs {expect} (tol {tol})"
            );
        }
        // 阴影 = 0 时直接光归零,仅 GI 项。
        let gi = [0.5, 0.4, 0.3];
        let shaded = shade_pixel(closure, world_n, 0.0, gi, 1.0);
        for c in 0..3 {
            let expect = p.albedo[c] / std::f32::consts::PI * gi[c];
            let tol = gi[c] / std::f32::consts::PI / 255.0 + 1e-5;
            assert!((shaded[c] - expect).abs() < tol, "阴影为零仅 GI {c}");
        }
    }
}
