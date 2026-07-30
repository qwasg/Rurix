//! 单通道效果时域滤波 + 空间补洞 host 完整实现(报告4 §3.2 P0「时域/空间
//! 降噪」与 §5「效果缓冲与降噪链」;RFC-0016 §4.F F2 时域滤波,验收门
//! G-G5-6 静态收敛口径)。
//!
//! **底座纪律(RFC-0016 章 H「禁效果 pass 私写重投影」,G-G5-7 代码审计点)**:
//! 重投影、历史验证、disocclusion、邻域方差裁剪一律经
//! [`crate::temporal::common`] 公共底座——本文件只做标量组装,不私写任何
//! 重投影/邻域统计数学。
//!
//! 滤波链(逐像素,单通道效果信号 v ∈ \[0,1\]):
//! 1. 历史验证:`validate_history_with_mv`(深度相对差 + 法线点积 + 出屏
//!    disocclusion,底座全链路入口);
//! 2. 历史重投影:`reproject_sample`(MV 双线性重采样 + 屏内 mask,底座);
//! 3. 邻域裁剪:单通道标量形态 μ±γσ——通道复制 3 份经底座
//!    `neighborhood_variance_bounds`(3 通道同值则逐通道边界相同,取 ch0),
//!    保证邻域统计单源;
//! 4. 指数混合:valid → α·cur + (1−α)·clamp(hist)(默认 α=0.1,有效累积窗
//!    ~10 帧);invalid(disocclusion/深度/法线突变)→ 直接取当前帧,
//!    宁可噪声回归也不拖影(与 TAA 同原则,报告7 §2.1 三不信场景)。
//!
//! 首帧用法:无历史时调用方直取当前帧为 history(双缓冲跨帧持有,图外
//! imported 外部资源纪律,报告5 §2.3);RTAO 低 spp 帧可先用 [`spatial_fill`]
//! 补无效洞再入滤波(P1 最小空间补洞)。

use crate::temporal::common::{
    neighborhood_variance_bounds, reproject_sample, validate_history_with_mv,
};
use crate::temporal::image::ImageF32;

// ---------------------------------------------------------------------------
// 时域滤波
// ---------------------------------------------------------------------------

/// 时域滤波参数(默认 α=0.1;深度/法线判据阈值与 TAA 单测口径同档)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemporalFilterParams {
    /// 当前帧混合权重(历史权重 = 1 − α;默认 0.1)。
    pub blend_alpha: f32,
    /// 方差裁剪 γ(μ±γσ;默认 1.0)。
    pub variance_gamma: f32,
    /// 历史验证深度相对容差(默认 0.1,底座 `validate_history_with_mv` 判据)。
    pub depth_rel_tol: f32,
    /// 历史验证法线点积下限(默认 0.9)。
    pub normal_dot_min: f32,
}

impl Default for TemporalFilterParams {
    fn default() -> Self {
        Self {
            blend_alpha: 0.1,
            variance_gamma: 1.0,
            depth_rel_tol: 0.1,
            normal_dot_min: 0.9,
        }
    }
}

/// 单通道效果时域滤波:重投影 + 历史验证 + 邻域方差裁剪 + 指数混合
/// (全部数学经 temporal 公共底座;disocclusion 处取当前帧)。
///
/// - `cur` / `history`:单通道效果图(AO/可见性),同尺寸;
/// - `mv`:2 通道 uv 偏移(历史采样位置 = uv+mv,见
///   [`crate::temporal::common::compute_camera_mv`]);
/// - `cur_depth` / `prev_depth` / `cur_normal` / `prev_normal`:历史验证
///   输入(深度单通道、法线 3 通道世界空间,prev 为上一帧未重投影原图,
///   重投影由底座内部完成);
/// - 输出:单通道滤波结果;调用方将其作为下一帧 `history`(双缓冲轮换)。
///
/// # Panics
/// 通道/尺寸契约违约即 panic。
#[allow(clippy::too_many_arguments)]
pub fn temporal_filter_effect(
    cur: &ImageF32,
    history: &ImageF32,
    mv: &ImageF32,
    cur_depth: &ImageF32,
    prev_depth: &ImageF32,
    cur_normal: &ImageF32,
    prev_normal: &ImageF32,
    params: &TemporalFilterParams,
) -> ImageF32 {
    assert!(
        cur.c == 1 && history.same_shape(cur),
        "temporal_filter_effect: cur/history 必须同尺寸单通道"
    );
    assert!(
        mv.c == 2 && mv.w == cur.w && mv.h == cur.h,
        "temporal_filter_effect: mv 必须同尺寸 2 通道"
    );
    assert!(
        cur_depth.c == 1
            && prev_depth.same_shape(cur_depth)
            && cur_depth.w == cur.w
            && cur_depth.h == cur.h,
        "temporal_filter_effect: 深度图形状不符"
    );
    assert!(
        cur_normal.c == 3
            && prev_normal.same_shape(cur_normal)
            && cur_normal.w == cur.w
            && cur_normal.h == cur.h,
        "temporal_filter_effect: 法线图形状不符"
    );
    let (w, h) = (cur.w, cur.h);
    // 1. 历史验证(底座全链路:内部重投影 prev 深度/法线 + 出屏取交)。
    let validity = validate_history_with_mv(
        cur_depth,
        prev_depth,
        cur_normal,
        prev_normal,
        mv,
        params.depth_rel_tol,
        params.normal_dot_min,
    );
    // 2. 历史效果图重投影(底座:双线性 + 屏内 mask)。
    let (hist_reproj, inside) = reproject_sample(history, mv);
    // 3. 标量 μ±γσ 邻域裁剪边界(通道复制 3 份经底座方差统计,取 ch0;
    //    邻域数学单源,不私写)。
    let cur3 = ImageF32::from_fn(w, h, 3, |x, y, _| cur.get(x, y, 0));
    let (lo, hi) = neighborhood_variance_bounds(&cur3, params.variance_gamma);
    let alpha = params.blend_alpha;
    let mut out = ImageF32::new(w, h, 1);
    for y in 0..h {
        for x in 0..w {
            if validity.get(x, y, 0) < 0.5 || inside.get(x, y, 0) < 0.5 {
                // disocclusion / 深度法线突变 / 重投影出屏:直接取当前帧。
                out.set(x, y, 0, cur.get(x, y, 0));
                continue;
            }
            let hc = hist_reproj
                .get(x, y, 0)
                .clamp(lo.get(x, y, 0), hi.get(x, y, 0));
            out.set(x, y, 0, alpha * cur.get(x, y, 0) + (1.0 - alpha) * hc);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 空间补洞(P1 最小;报告4 §5 降噪链的空间臂)
// ---------------------------------------------------------------------------

/// 空间补洞深度相似判据(相对容差 5%;深度断裂不跨界的锚定常量)。
pub const SPATIAL_FILL_DEPTH_REL_TOL: f32 = 0.05;

/// 空间补洞:validity = 0 的像素用 5×5 邻域内「valid 且深度相似」样本的
/// 均值填补(供低 spp 帧入时域滤波前补无效洞;P1 最小形态,不迭代扩散)。
///
/// - valid 像素(validity ≥ 0.5)原样透传;
/// - 无效像素:遍历 5×5 邻域,样本须 (a) 自身 valid、(b) 深度与中心满足
///   |Δd| ≤ [`SPATIAL_FILL_DEPTH_REL_TOL`]·max(d_c, d_n)(深度断裂不跨界)、
///   (c) 值有限;有 ≥1 个合格样本则取均值,否则保留原值(补不动不伪造);
/// - 单趟确定性(行主序、只读 `cur` 写 `out`,无顺序依赖)。
///
/// # Panics
/// 三图非同尺寸单通道即 panic。
pub fn spatial_fill(cur: &ImageF32, validity: &ImageF32, depth: &ImageF32) -> ImageF32 {
    assert!(
        cur.c == 1 && validity.same_shape(cur) && depth.same_shape(cur),
        "spatial_fill: 三图必须同尺寸单通道"
    );
    let (w, h) = (cur.w, cur.h);
    let mut out = cur.clone();
    for y in 0..h {
        for x in 0..w {
            if validity.get(x, y, 0) >= 0.5 {
                continue;
            }
            let dc = depth.get(x, y, 0);
            let mut sum = 0.0f32;
            let mut cnt = 0u32;
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let xx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                    let yy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                    if validity.get(xx, yy, 0) < 0.5 {
                        continue;
                    }
                    let dn = depth.get(xx, yy, 0);
                    if !dc.is_finite() || !dn.is_finite() {
                        continue;
                    }
                    if (dc - dn).abs() > SPATIAL_FILL_DEPTH_REL_TOL * dc.max(dn).max(1e-6) {
                        continue;
                    }
                    let v = cur.get(xx, yy, 0);
                    if !v.is_finite() {
                        continue;
                    }
                    sum += v;
                    cnt += 1;
                }
            }
            if cnt > 0 {
                out.set(x, y, 0, sum / cnt as f32);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::bvh::{InstanceDesc, Tlas, Transform3x4, TriBvh};
    use crate::rt::effects::{EffectInputs, EffectStats, gbuffer_from_scene, rtao_pass};
    use crate::temporal::common::{
        Mat4, compute_camera_mv, look_at_rh, perspective_rh_zo, validate_history_with_mv,
    };

    /// 网格(位置 + 索引)。
    type Mesh = (Vec<[f32; 3]>, Vec<[u32; 3]>);

    /// 地板四边形(法线 +y,winding 已校验;与 effects 单测同场景几何)。
    fn quad_y_up(x0: f32, x1: f32, z0: f32, z1: f32, y: f32) -> Mesh {
        (
            vec![[x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1]],
            vec![[0, 2, 1], [0, 3, 2]],
        )
    }

    /// x=0 墙面四边形(法线 +x)。
    fn quad_x_up(y0: f32, y1: f32, z0: f32, z1: f32, x: f32) -> Mesh {
        (
            vec![[x, y0, z0], [x, y1, z0], [x, y1, z1], [x, y0, z1]],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    /// z=0 墙面四边形(法线 +z)。
    fn quad_z_up(x0: f32, x1: f32, y0: f32, y1: f32, z: f32) -> Mesh {
        (
            vec![[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

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

    /// 三面墙角 + 开阔平面(与 effects 单测同一份对拍主场景)。
    fn corner_open_scene() -> (Vec<TriBvh>, Tlas) {
        let floor = quad_y_up(-4.0, 4.0, -4.0, 4.0, 0.0);
        let wall_x = quad_x_up(0.0, 2.0, 0.0, 2.0, 0.0);
        let wall_z = quad_z_up(0.0, 2.0, 0.0, 2.0, 0.0);
        let (pos, idx) = merge(&[floor, wall_x, wall_z]);
        scene_of(&pos, &idx)
    }

    /// 与 effects 单测同一墙角相机(上缘含天空像素)。
    fn corner_camera() -> Mat4 {
        let proj = perspective_rh_zo(0.9, 1.0, 0.1, 50.0);
        let view = look_at_rh([2.4, 2.0, 2.4], [0.0, 0.7, 0.0], [0.0, 1.0, 0.0]);
        proj.mul(&view)
    }

    const W: u32 = 24;
    const H: u32 = 24;
    const RADIUS: f32 = 1.0;
    const FRAMES: u32 = 16;

    /// 静态场景 16 帧低 spp(2) RTAO + 时域滤波累积序列(确定性固定种子)。
    /// 返回 (逐帧滤波输出, 逐帧原始低 spp 输入, 高 spp(64) 单帧参考, GBuffer)。
    fn run_static_accumulation() -> (Vec<ImageF32>, Vec<ImageF32>, ImageF32, ImageF32, ImageF32) {
        let (blases, tlas) = corner_open_scene();
        let vp = corner_camera();
        let (depth, normals) = gbuffer_from_scene(&vp, W, H, &tlas, &blases);
        let inputs = EffectInputs::new(&depth, &normals, vp, &tlas, &blases);
        // 静态相机:MV 经底座 compute_camera_mv 计算(≈0,端到端口径)。
        let mv = compute_camera_mv(&depth, &vp, &vp);
        let params = TemporalFilterParams::default();
        let mut stats = EffectStats::default();
        let reference = rtao_pass(&inputs, 64, RADIUS, 0, 0xBEEF_0001, &mut stats);
        let mut history: Option<ImageF32> = None;
        let mut outs = Vec::with_capacity(FRAMES as usize);
        let mut raws = Vec::with_capacity(FRAMES as usize);
        for f in 0..FRAMES {
            let cur = rtao_pass(&inputs, 2, RADIUS, f, 0xF00D_0001, &mut stats);
            raws.push(cur.clone());
            let out = match &history {
                None => cur.clone(),
                Some(hist) => temporal_filter_effect(
                    &cur, hist, &mv, &depth, &depth, &normals, &normals, &params,
                ),
            };
            history = Some(out.clone());
            outs.push(out);
        }
        (outs, raws, reference, depth, normals)
    }

    #[test]
    fn temporal_filter_converges_static_16_frames() {
        // **静态场景 16 帧收敛(G-G5-6)**:低 spp(2) 时域累积末帧 vs 高
        // spp(64) 单帧的 MSE,必须 < 低 spp 单帧 MSE 的 15%。机理:指数混合
        // α=0.1 把逐帧独立噪声方差压到 ~α/(2−α) ≈ 1/19(frame_index 混入
        // 种子保证帧间采样去相关)。
        let (outs, raws, reference, _, _) = run_static_accumulation();
        let mse_low = ImageF32::mse(&raws[0], &reference);
        let mse_final = ImageF32::mse(&outs[(FRAMES - 1) as usize], &reference);
        assert!(mse_low > 0.0, "低 spp 单帧必须有噪声");
        assert!(
            mse_final < 0.15 * mse_low,
            "累积末帧 MSE={mse_final:.6} 应 < 15% × 低 spp 单帧 MSE={mse_low:.6}"
        );
        // 收敛趋势:末 4 帧均值 MSE < 首 4 帧均值(与 TAA 收敛验收同形态)。
        let mses: Vec<f64> = outs.iter().map(|o| ImageF32::mse(o, &reference)).collect();
        let first_avg = mses[..4].iter().sum::<f64>() / 4.0;
        let last_avg = mses[12..].iter().sum::<f64>() / 4.0;
        assert!(
            last_avg < first_avg,
            "首段 {first_avg:.6} 应 > 末段 {last_avg:.6}"
        );
        eprintln!(
            "[filter_converge] low={mse_low:.6} final={mse_final:.6} ratio={:.4} \
             first4={first_avg:.6} last4={last_avg:.6}",
            mse_final / mse_low
        );
    }

    #[test]
    fn temporal_filter_frame_diff_decays() {
        // **帧间差趋零(G-G5-6 口径)**:时域稳定性以「滤波输出帧间差 vs 未
        // 滤波原始帧间差」度量(TAA `taa_jitter_residual_far_below_unfiltered`
        // 同口径——恒定 α 滤波器对 iid 噪声的稳态帧间差与首帧同阶,故分母取
        // 未滤波序列帧间差,「趋零」= 滤波把帧间起伏压到原始起伏的零头)。
        let (outs, raws, _, _, _) = run_static_accumulation();
        let raw_diff = ImageF32::mse(&raws[14], &raws[15]);
        let filt_last = ImageF32::mse(&outs[14], &outs[15]);
        assert!(
            filt_last < 0.1 * raw_diff,
            "末帧间差={filt_last:.8} 应 < 10% × 原始帧间差={raw_diff:.8}"
        );
        eprintln!(
            "[frame_diff] raw={raw_diff:.8} filt_last={filt_last:.8} ratio={:.4}",
            filt_last / raw_diff
        );
    }

    #[test]
    fn temporal_filter_disocclusion_no_ghost() {
        // 物体移开后揭露区无鬼影(报告7 §2.1 三不信场景一,标量形态):
        // 8px 亮条遮挡物帧 0→1 右移 1px;MV=0(物体 MV 缺失最恶劣情形,
        // 深度验证必须兜住)。揭露列输出必须 == 当前帧(若混入历史亮残影
        // 0.9 必超差),且 validity 经公共底座断言。
        let (w, h) = (32u32, 16u32);
        let value = |t: u32| {
            ImageF32::from_fn(
                w,
                h,
                1,
                |x, _, _| {
                    if x >= t && x < t + 8 { 0.9 } else { 0.1 }
                },
            )
        };
        let depth_of = |t: u32| {
            ImageF32::from_fn(
                w,
                h,
                1,
                |x, _, _| {
                    if x >= t && x < t + 8 { 0.3 } else { 0.9 }
                },
            )
        };
        let normal = ImageF32::from_fn(w, h, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        let mv = ImageF32::new(w, h, 2);
        let (cur, history) = (value(1), value(0));
        let (d_cur, d_prev) = (depth_of(1), depth_of(0));
        // validity 经公共底座(审计点:与滤波内部同一入口)。
        let validity = validate_history_with_mv(&d_cur, &d_prev, &normal, &normal, &mv, 0.1, 0.9);
        assert!(validity.get(0, 4, 0) < 0.5, "揭露列深度突变应 invalid");
        assert!(validity.get(8, 4, 0) < 0.5, "新遮挡列深度突变应 invalid");
        for x in 1..8 {
            assert!(validity.get(x, 4, 0) > 0.5, "({x}) 遮挡物内部应 valid");
        }
        for x in 9..w {
            assert!(validity.get(x, 4, 0) > 0.5, "({x}) 背景应 valid");
        }
        let out = temporal_filter_effect(
            &cur,
            &history,
            &mv,
            &d_cur,
            &d_prev,
            &normal,
            &normal,
            &TemporalFilterParams::default(),
        );
        // 揭露区输出 == 当前帧(历史 0.9 亮残影若混入 → 0.82,必超差)。
        assert!((out.get(0, 4, 0) - 0.1).abs() < 1e-6, "揭露列鬼影");
        assert!((out.get(8, 4, 0) - 0.9).abs() < 1e-6, "新遮挡列应取当前帧");
        // 不变区域为混合结果:内部亮、远处背景暗。
        for x in 2..7 {
            assert!((out.get(x, 4, 0) - 0.9).abs() < 1e-6, "({x}) 内部应亮");
        }
        for x in 10..20 {
            assert!((out.get(x, 4, 0) - 0.1).abs() < 1e-6, "({x}) 背景应暗");
        }
    }

    #[test]
    fn spatial_fill_anchors_to_neighborhood_mean() {
        // 人工挖洞:渐变场 v = 0.2 + 0.05·x,洞 x∈[3,6)×y∈[2,5) 内置垃圾 0。
        // 补值必须与「valid 且深度相似 5×5 邻域均值」锚定(逐像素独立重算
        // 期望均值,容差 1e-6);valid 像素原样透传。
        let (w, h) = (10u32, 8u32);
        let depth = ImageF32::from_fn(w, h, 1, |_, _, _| 0.5);
        let mut cur = ImageF32::from_fn(w, h, 1, |x, _, _| 0.2 + 0.05 * x as f32);
        let mut validity = ImageF32::from_fn(w, h, 1, |_, _, _| 1.0);
        for y in 2..5 {
            for x in 3..6 {
                validity.set(x, y, 0, 0.0);
                cur.set(x, y, 0, 0.0);
            }
        }
        let filled = spatial_fill(&cur, &validity, &depth);
        for y in 0..h {
            for x in 0..w {
                if validity.get(x, y, 0) >= 0.5 {
                    assert_eq!(filled.get(x, y, 0), cur.get(x, y, 0), "valid 透传");
                    continue;
                }
                // 独立重算期望均值(与实现同一判定规则的测试侧锚定)。
                let (mut sum, mut cnt) = (0.0f32, 0u32);
                for dy in -2i32..=2 {
                    for dx in -2i32..=2 {
                        let xx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                        let yy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                        if validity.get(xx, yy, 0) >= 0.5 {
                            sum += cur.get(xx, yy, 0);
                            cnt += 1;
                        }
                    }
                }
                assert!(cnt > 0);
                let expect = sum / cnt as f32;
                assert!(
                    (filled.get(x, y, 0) - expect).abs() < 1e-6,
                    "({x},{y}) 补值 {} 应锚定邻域均值 {expect}",
                    filled.get(x, y, 0)
                );
            }
        }
    }

    #[test]
    fn spatial_fill_respects_depth_break() {
        // 深度断裂不跨界:左半 depth=0.3 值 0.2,右半 depth=0.8 值 0.9;
        // 跨界洞 x∈[4,8)——左半洞像素只能取左侧邻域(补 0.2),右半洞像素
        // 只能取右侧邻域(补 0.9);跨界均值(~0.43/0.66)出现即判失败。
        let (w, h) = (12u32, 8u32);
        let depth = ImageF32::from_fn(w, h, 1, |x, _, _| if x < 6 { 0.3 } else { 0.8 });
        let mut cur = ImageF32::from_fn(w, h, 1, |x, _, _| if x < 6 { 0.2 } else { 0.9 });
        let mut validity = ImageF32::from_fn(w, h, 1, |_, _, _| 1.0);
        for y in 3..5 {
            for x in 4..8 {
                validity.set(x, y, 0, 0.0);
                cur.set(x, y, 0, 0.0);
            }
        }
        let filled = spatial_fill(&cur, &validity, &depth);
        for y in 3..5 {
            for x in 4..8 {
                let expect = if x < 6 { 0.2 } else { 0.9 };
                assert!(
                    (filled.get(x, y, 0) - expect).abs() < 1e-6,
                    "({x},{y}) 跨界补值 {} 应为同侧 {expect}",
                    filled.get(x, y, 0)
                );
            }
        }
    }

    #[test]
    fn spatial_fill_no_valid_neighbors_keeps_current() {
        // 全图 invalid:无任何合格样本 → 保留原值(补不动不伪造)。
        let (w, h) = (4u32, 4u32);
        let cur = ImageF32::from_fn(w, h, 1, |x, y, _| 0.1 * (x + y * w) as f32);
        let validity = ImageF32::new(w, h, 1);
        let depth = ImageF32::from_fn(w, h, 1, |_, _, _| 0.5);
        let filled = spatial_fill(&cur, &validity, &depth);
        assert_eq!(filled.data, cur.data, "无合格样本必须原值保留");
    }

    #[test]
    fn temporal_filter_params_default() {
        let p = TemporalFilterParams::default();
        assert_eq!(p.blend_alpha, 0.1, "默认 α=0.1(规格)");
        assert!(p.variance_gamma > 0.0 && p.depth_rel_tol > 0.0 && p.normal_dot_min > 0.0);
    }

    #[test]
    #[should_panic(expected = "temporal_filter_effect")]
    fn temporal_filter_shape_validation_panics() {
        let img1 = ImageF32::new(4, 4, 1);
        let bad3 = ImageF32::new(4, 4, 3); // cur 通道违约:必须单通道
        let mv = ImageF32::new(4, 4, 2);
        let _ = temporal_filter_effect(
            &bad3,
            &img1,
            &mv,
            &img1,
            &img1,
            &bad3,
            &bad3,
            &TemporalFilterParams::default(),
        );
    }
}
