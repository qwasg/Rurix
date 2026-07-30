//! GI 时域累积(报告2 §3.1 时域累积;RFC-0016 章 E2 + 章 H「禁效果 pass 私写
//! 重投影」——G-G5-7 代码审计点)。
//!
//! **历史 = 跨帧外部资源双缓冲**(RFC-0016 §4.0-3 / 章 E2:探针缓存历史 SH +
//! 深度锚点为 imported 资源):调用方在图外持有 [`GiHistory`] 两槽轮换,本模块
//! 只做「当前帧 + 上一帧历史 → 混合输出 + 新历史」纯函数。
//!
//! 全部重投影/历史验证一律经 [`crate::temporal::common`] 公共底座
//! ([`reproject_sample`] / [`validate_history_with_mv`]),探针级与像素级共用
//! 同一 MV 语义(`mv = prev_uv − cur_uv`,由
//! [`crate::temporal::common::compute_camera_mv`] 产出):
//! - 探针级:探针 SH 打包为 4 张 L1 系数图(探针分辨率,3ch×4)经
//!   `reproject_sample` 重采样;深度/法线历史在探针锚点采集为探针分辨率图,经
//!   `validate_history_with_mv` 出 validity(含出屏 disocclusion);
//! - 像素级:全屏 irradiance 历史经 `reproject_sample` 重采样,全屏深度/法线
//!   经 `validate_history_with_mv` 出 validity;
//! - 指数混合:`out = prev·(1−α) + cur·α`,validity = 0 ⇒ 用当前帧(disocclusion
//!   无鬼影)。

use crate::gi::probe::ProbeGrid;
use crate::gi::sh::ShL1Rgb;
use crate::temporal::common::{
    TemporalFrameDesc, TemporalResourceSpec, reproject_sample, validate_history_with_mv,
};
use crate::temporal::image::ImageF32;

/// 时域累积参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemporalParams {
    /// 指数混合系数(新样本权重):`out = prev·(1−α) + cur·α`。
    pub alpha: f32,
    /// 深度相对容差(公共底座历史验证判据一)。
    pub depth_rel_tol: f32,
    /// 法线点积下限(公共底座历史验证判据二)。
    pub normal_dot_min: f32,
}

impl Default for TemporalParams {
    fn default() -> Self {
        TemporalParams {
            alpha: 0.1,
            depth_rel_tol: 0.1,
            normal_dot_min: 0.9,
        }
    }
}

/// GI 跨帧历史(外部资源双缓冲语义;全屏深度/法线与 TAA 历史共享同一份双
/// 缓冲语义,不另开资源——见 [`gi_frame_desc`] 文档)。
#[derive(Debug, Clone)]
pub struct GiHistory {
    /// 探针 SH 历史(探针网格行主序;混合后输出即下一帧历史)。
    pub probe_sh: Vec<ShL1Rgb>,
    /// 像素 irradiance 历史(全屏 3ch)。
    pub irradiance: ImageF32,
    /// 全屏深度历史(1ch,NDC z)。
    pub depth: ImageF32,
    /// 全屏法线历史(3ch,世界空间)。
    pub normal: ImageF32,
}

impl GiHistory {
    /// 由当前帧产物构建新历史(探针数/图形状校验即 panic,调用契约)。
    pub fn from_frame(
        grid: &ProbeGrid,
        shs: &[ShL1Rgb],
        irradiance: &ImageF32,
        depth: &ImageF32,
        normals: &ImageF32,
    ) -> GiHistory {
        assert_eq!(
            grid.probes.len(),
            shs.len(),
            "GiHistory::from_frame: 探针数与 SH 组数不符"
        );
        assert!(
            irradiance.w == depth.w
                && irradiance.h == depth.h
                && normals.w == depth.w
                && normals.h == depth.h
                && irradiance.c == 3
                && depth.c == 1
                && normals.c == 3,
            "GiHistory::from_frame: 图形状不符"
        );
        GiHistory {
            probe_sh: shs.to_vec(),
            irradiance: irradiance.clone(),
            depth: depth.clone(),
            normal: normals.clone(),
        }
    }
}

/// 当前帧输入包(时域累积调用面,控制参数个数)。
pub struct GiFrame<'a> {
    /// 探针网格。
    pub grid: &'a ProbeGrid,
    /// 当前帧探针 SH(与 grid.probes 对齐)。
    pub shs: &'a [ShL1Rgb],
    /// 当前帧全屏 irradiance(3ch)。
    pub irradiance: &'a ImageF32,
    /// 当前帧全屏深度(1ch)。
    pub depth: &'a ImageF32,
    /// 当前帧全屏法线(3ch)。
    pub normals: &'a ImageF32,
}

/// 探针 SH ↔ 4 张 L1 系数图(探针分辨率 3ch×4;`reproject_sample` 的 c ≤ 4
/// 通道约束的适配层——只打包/拆包,重投影全部在公共底座)。
fn sh_to_coeff_images(shs: &[ShL1Rgb], gw: u32, gh: u32) -> [ImageF32; 4] {
    let mut imgs = [
        ImageF32::new(gw, gh, 3),
        ImageF32::new(gw, gh, 3),
        ImageF32::new(gw, gh, 3),
        ImageF32::new(gw, gh, 3),
    ];
    for (idx, sh) in shs.iter().enumerate() {
        let (i, j) = (idx as u32 % gw, idx as u32 / gw);
        for (k, img) in imgs.iter_mut().enumerate() {
            img.set_pixel3(i, j, sh.c[k]);
        }
    }
    imgs
}

/// [`sh_to_coeff_images`] 的逆。
fn coeff_images_to_sh(imgs: &[ImageF32; 4]) -> Vec<ShL1Rgb> {
    let (gw, gh) = (imgs[0].w, imgs[0].h);
    let mut out = Vec::with_capacity((gw * gh) as usize);
    for j in 0..gh {
        for i in 0..gw {
            let mut sh = ShL1Rgb::ZERO;
            for (k, img) in imgs.iter().enumerate() {
                sh.c[k] = img.pixel3(i, j);
            }
            out.push(sh);
        }
    }
    out
}

/// 探针分辨率的深度/法线/MV 采集(锚点最近邻采样;只搬运数值,重投影与验证
/// 全部在公共底座)。`src_depth/src_normal/src_mv` 为全屏图。
fn probe_anchor_images(
    grid: &ProbeGrid,
    src_depth: &ImageF32,
    src_normal: &ImageF32,
    src_mv: &ImageF32,
) -> (ImageF32, ImageF32, ImageF32) {
    let (gw, gh) = (grid.w, grid.h);
    let mut pd = ImageF32::new(gw, gh, 1);
    let mut pn = ImageF32::new(gw, gh, 3);
    let mut pmv = ImageF32::new(gw, gh, 2);
    for j in 0..gh {
        for i in 0..gw {
            let [u, v] = grid.anchor_uv(i, j);
            pd.set(i, j, 0, src_depth.sample_nearest(u, v, 0));
            for ch in 0..3 {
                pn.set(i, j, ch, src_normal.sample_nearest(u, v, ch));
            }
            pmv.set(i, j, 0, src_mv.sample_nearest(u, v, 0));
            pmv.set(i, j, 1, src_mv.sample_nearest(u, v, 1));
        }
    }
    (pd, pn, pmv)
}

/// GI 时域累积:探针 SH + 像素 irradiance 经公共底座重投影验证后指数混合,
/// 返回(混合后探针 SH, 混合后像素 irradiance, 新历史)。
///
/// # Panics
/// 形状失配(历史与当前帧分辨率/探针数不符,MV 尺寸不符)即 panic,调用契约。
pub fn temporal_accumulate(
    frame: &GiFrame,
    history: &GiHistory,
    mv: &ImageF32,
    params: &TemporalParams,
) -> (Vec<ShL1Rgb>, ImageF32, GiHistory) {
    let grid = frame.grid;
    let (gw, gh) = (grid.w, grid.h);
    assert_eq!(
        grid.probes.len(),
        history.probe_sh.len(),
        "temporal_accumulate: 历史探针数与当前网格不符"
    );
    assert_eq!(
        grid.probes.len(),
        frame.shs.len(),
        "temporal_accumulate: 探针数与 SH 组数不符"
    );
    assert!(
        frame.irradiance.w == frame.depth.w
            && frame.irradiance.h == frame.depth.h
            && frame.irradiance.c == 3
            && frame.depth.c == 1
            && frame.normals.c == 3
            && frame.normals.same_shape(frame.irradiance),
        "temporal_accumulate: 当前帧图形状不符"
    );
    assert!(
        history.depth.same_shape(frame.depth)
            && history.normal.same_shape(frame.normals)
            && history.irradiance.same_shape(frame.irradiance)
            && mv.c == 2
            && mv.w == frame.depth.w
            && mv.h == frame.depth.h,
        "temporal_accumulate: 历史/MV 与当前帧形状不符"
    );
    let alpha = params.alpha.clamp(0.0, 1.0);

    // ---- 探针级:SH 重投影 + 锚点深度/法线历史验证(公共底座) ----
    let (cur_pd, cur_pn, pmv) = probe_anchor_images(grid, frame.depth, frame.normals, mv);
    let (prev_pd, prev_pn, _) = probe_anchor_images(grid, &history.depth, &history.normal, mv);
    let probe_mask = validate_history_with_mv(
        &cur_pd,
        &prev_pd,
        &cur_pn,
        &prev_pn,
        &pmv,
        params.depth_rel_tol,
        params.normal_dot_min,
    );
    let prev_coeff = sh_to_coeff_images(&history.probe_sh, gw, gh);
    let mut reproj_coeff: Vec<ImageF32> = Vec::with_capacity(4);
    for img in &prev_coeff {
        let (reproj, _inside) = reproject_sample(img, &pmv);
        reproj_coeff.push(reproj);
    }
    let prev_sh: [ImageF32; 4] = reproj_coeff.try_into().expect("恰 4 张系数图");
    let prev_sh = coeff_images_to_sh(&prev_sh);
    let mut out_sh = Vec::with_capacity(grid.probes.len());
    for (idx, probe) in grid.probes.iter().enumerate() {
        let (i, j) = (idx as u32 % gw, idx as u32 / gw);
        let cur = frame.shs[idx];
        let valid = probe.valid && probe_mask.get(i, j, 0) > 0.5;
        out_sh.push(if valid {
            prev_sh[idx].lerp(cur, alpha)
        } else {
            cur
        });
    }

    // ---- 像素级:irradiance 重投影 + 全屏深度/法线历史验证(公共底座) ----
    let (prev_irr, _inside) = reproject_sample(&history.irradiance, mv);
    let mask = validate_history_with_mv(
        frame.depth,
        &history.depth,
        frame.normals,
        &history.normal,
        mv,
        params.depth_rel_tol,
        params.normal_dot_min,
    );
    let mut out_irr = frame.irradiance.clone();
    for y in 0..out_irr.h {
        for x in 0..out_irr.w {
            if mask.get(x, y, 0) > 0.5 {
                let prev = prev_irr.pixel3(x, y);
                let cur = frame.irradiance.pixel3(x, y);
                out_irr.set_pixel3(
                    x,
                    y,
                    [
                        prev[0] * (1.0 - alpha) + cur[0] * alpha,
                        prev[1] * (1.0 - alpha) + cur[1] * alpha,
                        prev[2] * (1.0 - alpha) + cur[2] * alpha,
                    ],
                );
            }
        }
    }

    let new_history = GiHistory::from_frame(grid, &out_sh, &out_irr, frame.depth, frame.normals);
    (out_sh, out_irr, new_history)
}

/// GI 支路图集成描述(声明面;RFC-0016 章 E1「全 compute pass」与 §4.0-3 跨帧
/// imported 纪律;执行体 = 本模块 host 管线,W3 device 建图引用)。
///
/// 跨帧历史(imported)= 探针 SH(in/out)+ 像素 irradiance(in/out);全屏
/// 深度/法线历史与 TAA 共享同一份双缓冲(`taa.depth_prev`/`taa.normal_prev`,
/// 不重复声明)。`gi.probe_sh_*` 为 4 张 L1 系数图的逻辑组(Rgba16Float × 4)。
pub fn gi_frame_desc() -> TemporalFrameDesc {
    use crate::graph::types::AccessKind::{ShaderRead, ShaderWrite};
    use crate::graph::types::TextureFormat::{R32Float, Rg16Float, Rgba16Float};
    TemporalFrameDesc {
        pass_name: "gi.screen_probe",
        resources: vec![
            TemporalResourceSpec {
                name: "gi.depth",
                access: ShaderRead,
                format: R32Float,
                imported: false,
            },
            TemporalResourceSpec {
                name: "gi.normal",
                access: ShaderRead,
                format: Rgba16Float,
                imported: false,
            },
            TemporalResourceSpec {
                name: "gi.mv",
                access: ShaderRead,
                format: Rg16Float,
                imported: false,
            },
            TemporalResourceSpec {
                name: "gi.probe_sh_in",
                access: ShaderRead,
                format: Rgba16Float,
                imported: true,
            },
            TemporalResourceSpec {
                name: "gi.probe_sh_out",
                access: ShaderWrite,
                format: Rgba16Float,
                imported: true,
            },
            TemporalResourceSpec {
                name: "gi.irradiance_in",
                access: ShaderRead,
                format: Rgba16Float,
                imported: true,
            },
            TemporalResourceSpec {
                name: "gi.irradiance_out",
                access: ShaderWrite,
                format: Rgba16Float,
                imported: true,
            },
            TemporalResourceSpec {
                name: "gi.output",
                access: ShaderWrite,
                format: Rgba16Float,
                imported: false,
            },
        ],
    }
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sh_coeff_images_roundtrip() {
        // 打包/拆包恒等(适配层不引入误差)。
        let (gw, gh) = (4u32, 3u32);
        let shs: Vec<ShL1Rgb> = (0..(gw * gh) as usize)
            .map(|idx| {
                let mut sh = ShL1Rgb::ZERO;
                for (k, co) in sh.c.iter_mut().enumerate() {
                    for (ch, v) in co.iter_mut().enumerate() {
                        *v = (idx * 13 + k * 7 + ch * 3) as f32 * 0.03125 - 0.5;
                    }
                }
                sh
            })
            .collect();
        let imgs = sh_to_coeff_images(&shs, gw, gh);
        let back = coeff_images_to_sh(&imgs);
        assert_eq!(back, shs, "系数图打包/拆包应逐位恒等");
    }

    #[test]
    fn gi_frame_desc_resources() {
        let desc = gi_frame_desc();
        assert_eq!(desc.pass_name, "gi.screen_probe");
        // 资源名互异。
        let mut names: Vec<_> = desc.resources.iter().map(|r| r.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), desc.resources.len());
        // 跨帧历史四件 imported(RFC §4.0-3 / 章 E2 双缓冲纪律)。
        for name in [
            "gi.probe_sh_in",
            "gi.probe_sh_out",
            "gi.irradiance_in",
            "gi.irradiance_out",
        ] {
            let r = desc
                .resources
                .iter()
                .find(|r| r.name == name)
                .expect("资源存在");
            assert!(r.imported, "{name} 应为 imported");
        }
        // 写访问恰好两条:历史回写两件 + 输出一件 = 三条?——probe_sh_out /
        // irradiance_out / output,逐一核验。
        let writes: Vec<_> = desc
            .resources
            .iter()
            .filter(|r| r.access.is_write())
            .map(|r| r.name)
            .collect();
        assert_eq!(
            writes,
            ["gi.probe_sh_out", "gi.irradiance_out", "gi.output"]
        );
    }
}
