//! 时域公共底座(报告7 §2.1「必须自研」底层;RFC-0016 章 H 前半)。
//!
//! 一次投资处处受益:Halton 抖动序列、最小 4x4 矩阵工具、相机 motion vectors、
//! 深度/法线历史验证 + disocclusion 检测、YCoCg 邻域裁剪(AABB/方差)——TAA 只是
//! 它上面的薄 pass,GI/阴影/RT 的一切时域滤波复用同一验证例程,禁效果 pass 私写
//! 重投影(报告7 §3 映射要点)。

use crate::graph::types::{AccessKind, TextureFormat};
use crate::temporal::image::ImageF32;

// ---------------------------------------------------------------------------
// Halton 抖动序列(报告7 §2.1:抖动采样序列 Halton 与相机 jitter 矩阵进全局 uniform)
// ---------------------------------------------------------------------------

/// 进制反函数(radical inverse):Halton 低差异序列单点,∈ [0,1)。
///
/// `index` 从 1 起(index = 0 恒为 0,通常跳过);`base` ≥ 2(与相邻维度互素,
/// 像素抖动惯例 base 2/3)。
pub fn halton(mut index: u32, base: u32) -> f32 {
    assert!(base >= 2, "Halton base 必须 ≥2");
    let mut f = 1.0f32;
    let mut r = 0.0f32;
    let b = base as f32;
    while index > 0 {
        f /= b;
        r += f * (index % base) as f32;
        index /= base;
    }
    r
}

/// 像素抖动序列:base 2/3 Halton 居中到 [-0.5, 0.5) 像素单位。
///
/// 相机侧用法:投影矩阵叠加 jitter/w、jitter/h 的亚像素平移;host 参考实现直接
/// 在采样坐标上加 jitter(等价)。均值随点数趋 0,保证长程累积无偏。
pub fn jitter_sequence(n: u32) -> Vec<[f32; 2]> {
    (0..n)
        .map(|i| [halton(i + 1, 2) - 0.5, halton(i + 1, 3) - 0.5])
        .collect()
}

// ---------------------------------------------------------------------------
// 最小 4x4 矩阵工具(行主序 m[row][col],列向量约定 clip = M·v;
// 相机 MV 反投影/重投影用——报告7 §2.1「完整 motion vectors」的相机速度分量)
// ---------------------------------------------------------------------------

/// 4x4 矩阵(行主序存储,列向量左乘约定)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn identity() -> Self {
        let mut m = [[0.0f32; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        Self { m }
    }

    /// 矩阵乘法 self·rhs(先作用 rhs 后作用 self,列向量约定)。
    pub fn mul(&self, rhs: &Mat4) -> Mat4 {
        let mut out = [[0.0f32; 4]; 4];
        for (r, orow) in out.iter_mut().enumerate() {
            for (c, o) in orow.iter_mut().enumerate() {
                *o = (0..4).map(|k| self.m[r][k] * rhs.m[k][c]).sum();
            }
        }
        Mat4 { m: out }
    }

    /// 齐次向量变换 out = M·v。
    pub fn transform_vec4(&self, v: [f32; 4]) -> [f32; 4] {
        let mut out = [0.0f32; 4];
        for (r, o) in out.iter_mut().enumerate() {
            *o = (0..4).map(|k| self.m[r][k] * v[k]).sum();
        }
        out
    }

    /// 伴随法求逆(一般 4x4;奇异矩阵返回 None)。
    pub fn inverse(&self) -> Option<Mat4> {
        let m = &self.m;
        let mut cof = [[0.0f32; 4]; 4];
        for (i, cof_row) in cof.iter_mut().enumerate() {
            for (j, c) in cof_row.iter_mut().enumerate() {
                // 删第 i 行第 j 列的 3x3 子式
                let mut sub = [[0.0f32; 3]; 3];
                let mut si = 0usize;
                for (r, mrow) in m.iter().enumerate() {
                    if r == i {
                        continue;
                    }
                    let mut sj = 0usize;
                    for (ccol, &v) in mrow.iter().enumerate() {
                        if ccol == j {
                            continue;
                        }
                        sub[si][sj] = v;
                        sj += 1;
                    }
                    si += 1;
                }
                let sign = if (i + j) % 2 == 0 { 1.0 } else { -1.0 };
                *c = sign * det3(&sub);
            }
        }
        // 行列式 = 第一行与第一行代数余子式点积
        let det: f32 = (0..4).map(|j| m[0][j] * cof[0][j]).sum();
        if det.abs() < 1e-12 {
            return None;
        }
        let inv_det = 1.0 / det;
        // 伴随矩阵 = 代数余子式矩阵的转置
        let mut out = [[0.0f32; 4]; 4];
        for (r, orow) in out.iter_mut().enumerate() {
            for (ccol, o) in orow.iter_mut().enumerate() {
                *o = cof[ccol][r] * inv_det;
            }
        }
        Some(Mat4 { m: out })
    }
}

fn det3(a: &[[f32; 3]; 3]) -> f32 {
    a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize3(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = dot3(v, v).sqrt();
    if len < 1e-12 {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

/// 右手系透视投影(深度 [0,1] ZO,Vulkan/D3D 口径;视线朝 -z)。
pub fn perspective_rh_zo(fov_y: f32, aspect: f32, z_near: f32, z_far: f32) -> Mat4 {
    assert!(fov_y > 0.0 && aspect > 0.0, "透视参数必须为正");
    assert!(z_near > 0.0 && z_far > z_near, "深度区间非法");
    let f = 1.0 / (fov_y * 0.5).tan();
    let mut m = [[0.0f32; 4]; 4];
    m[0][0] = f / aspect;
    m[1][1] = f;
    m[2][2] = z_far / (z_near - z_far);
    m[2][3] = z_near * z_far / (z_near - z_far);
    m[3][2] = -1.0;
    Mat4 { m }
}

/// 右手系视图矩阵(eye 看向 center,up 参考上方向)。
pub fn look_at_rh(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> Mat4 {
    let f = normalize3(sub3(center, eye)).expect("eye 不得与 center 重合");
    let s = normalize3(cross3(f, up)).expect("up 不得与视线平行");
    let u = cross3(s, f);
    let mut m = [[0.0f32; 4]; 4];
    m[0] = [s[0], s[1], s[2], -dot3(s, eye)];
    m[1] = [u[0], u[1], u[2], -dot3(u, eye)];
    m[2] = [-f[0], -f[1], -f[2], dot3(f, eye)];
    m[3] = [0.0, 0.0, 0.0, 1.0];
    Mat4 { m }
}

// ---------------------------------------------------------------------------
// 相机 motion vectors(报告7 §2.1:逐像素速度场是历史重投影的数学前提;
// 几何/蒙皮/WPO 三类物体速度由 G5.3 几何侧并入,本底座先给相机分量)
// ---------------------------------------------------------------------------

/// 相机 MV:2 通道 uv 偏移图,mv = prev_uv - cur_uv(历史采样位置 = uv + mv)。
///
/// 逐像素把当前帧深度反投影到世界空间,再用上一帧 view_proj 投影回去取 uv 差。
/// NDC 约定与 [`perspective_rh_zo`] 一致:ndc.x = 2u-1,ndc.y = 1-2v(uv 原点左上),
/// 深度 [0,1]。齐次 w 失效(无穷远/相机背后)的像素 MV 置 0,交由历史验证兜底拒绝。
pub fn compute_camera_mv(
    depth: &ImageF32,
    cur_view_proj: &Mat4,
    prev_view_proj: &Mat4,
) -> ImageF32 {
    assert_eq!(depth.c, 1, "深度图必须单通道");
    let inv_cur = cur_view_proj.inverse().expect("cur_view_proj 必须可逆");
    let (w, h) = (depth.w as f32, depth.h as f32);
    let mut mv = ImageF32::new(depth.w, depth.h, 2);
    for y in 0..depth.h {
        for x in 0..depth.w {
            let u = (x as f32 + 0.5) / w;
            let v = (y as f32 + 0.5) / h;
            let ndc = [2.0 * u - 1.0, 1.0 - 2.0 * v, depth.get(x, y, 0), 1.0];
            let world4 = inv_cur.transform_vec4(ndc);
            if world4[3].abs() < 1e-8 {
                continue;
            }
            let world = [
                world4[0] / world4[3],
                world4[1] / world4[3],
                world4[2] / world4[3],
                1.0,
            ];
            let prev_clip = prev_view_proj.transform_vec4(world);
            if prev_clip[3] <= 1e-8 {
                continue;
            }
            let prev_u = 0.5 * (prev_clip[0] / prev_clip[3] + 1.0);
            let prev_v = 0.5 * (1.0 - prev_clip[1] / prev_clip[3]);
            mv.set(x, y, 0, prev_u - u);
            mv.set(x, y, 1, prev_v - v);
        }
    }
    mv
}

// ---------------------------------------------------------------------------
// 历史重投影与验证(报告7 §2.1 三不信场景:遮挡揭开/着色变化/MV 说谎;
// 深度/法线一致性测试 + 出屏检测 = 后验可信度,reactive mask 的先验可信度
// 与本结果取并集,属 G5.3-H 后半)
// ---------------------------------------------------------------------------

/// 按 MV 双线性重采样历史图,并给出屏内 mask(1 通道 0/1)。
///
/// 采样位置 uv+mv 出 [0,1] 即 disocclusion(上一帧根本看不到该表面),mask 置 0、
/// 输出像素留 0——调用方必须以 mask 拒绝该处历史,不得使用 clamp 边界值。
pub fn reproject_sample(src: &ImageF32, mv: &ImageF32) -> (ImageF32, ImageF32) {
    assert!(
        mv.c == 2 && mv.w == src.w && mv.h == src.h,
        "MV 与源图尺寸不符"
    );
    let (w, h) = (src.w as f32, src.h as f32);
    let mut out = ImageF32::new(src.w, src.h, src.c);
    let mut inside = ImageF32::new(src.w, src.h, 1);
    for y in 0..src.h {
        for x in 0..src.w {
            let u = (x as f32 + 0.5) / w;
            let v = (y as f32 + 0.5) / h;
            let su = u + mv.get(x, y, 0);
            let sv = v + mv.get(x, y, 1);
            if (0.0..=1.0).contains(&su) && (0.0..=1.0).contains(&sv) {
                inside.set(x, y, 0, 1.0);
                for ch in 0..src.c {
                    out.set(x, y, ch, src.sample_bilinear(su, sv, ch));
                }
            }
        }
    }
    (out, inside)
}

/// 历史验证:深度相对差 + 法线点积双判据(报告7 §2.1 深度/法线一致性测试),
/// 输出 1 通道 0/1 mask。
///
/// `prev_*_reproj` 必须已按 MV 重采样到当前帧(见 [`reproject_sample`]);
/// 深度判据 |cur-prev| ≤ depth_rel_tol·max(cur,prev),法线判据 dot ≥ normal_dot_min
/// (零向量法线按不可信拒绝)。深度突变处(遮挡揭开/新遮挡)置 0。
pub fn validate_history(
    cur_depth: &ImageF32,
    prev_depth_reproj: &ImageF32,
    cur_normal: &ImageF32,
    prev_normal_reproj: &ImageF32,
    depth_rel_tol: f32,
    normal_dot_min: f32,
) -> ImageF32 {
    assert!(
        cur_depth.c == 1 && cur_depth.same_shape(prev_depth_reproj),
        "深度图形状不符"
    );
    assert!(
        cur_normal.c == 3
            && cur_normal.same_shape(prev_normal_reproj)
            && cur_normal.w == cur_depth.w
            && cur_normal.h == cur_depth.h,
        "法线图形状不符"
    );
    let mut mask = ImageF32::new(cur_depth.w, cur_depth.h, 1);
    for y in 0..cur_depth.h {
        for x in 0..cur_depth.w {
            let dc = cur_depth.get(x, y, 0);
            let dp = prev_depth_reproj.get(x, y, 0);
            let depth_ok = (dc - dp).abs() <= depth_rel_tol * dc.max(dp).max(1e-6);
            let normal_ok = match (
                normalize3(cur_normal.pixel3(x, y)),
                normalize3(prev_normal_reproj.pixel3(x, y)),
            ) {
                (Some(nc), Some(np)) => dot3(nc, np) >= normal_dot_min,
                _ => false,
            };
            mask.set(x, y, 0, if depth_ok && normal_ok { 1.0 } else { 0.0 });
        }
    }
    mask
}

/// 全链路历史验证:内部按 MV 重投影 prev 深度/法线,与屏内 mask 取交
/// (重投影出屏 = disocclusion 置 0)。TAA/时域滤波的标准入口。
pub fn validate_history_with_mv(
    cur_depth: &ImageF32,
    prev_depth: &ImageF32,
    cur_normal: &ImageF32,
    prev_normal: &ImageF32,
    mv: &ImageF32,
    depth_rel_tol: f32,
    normal_dot_min: f32,
) -> ImageF32 {
    let (prev_d, inside_d) = reproject_sample(prev_depth, mv);
    let (prev_n, inside_n) = reproject_sample(prev_normal, mv);
    let mut mask = validate_history(
        cur_depth,
        &prev_d,
        cur_normal,
        &prev_n,
        depth_rel_tol,
        normal_dot_min,
    );
    for ((m, &id), &in_) in mask
        .data
        .iter_mut()
        .zip(inside_d.data.iter())
        .zip(inside_n.data.iter())
    {
        *m *= id * in_;
    }
    mask
}

// ---------------------------------------------------------------------------
// YCoCg 邻域裁剪(报告7 §2.1:邻域裁剪是鬼影的直接克星,标准做法在 YCoCg
// 空间对 3x3 邻域建 AABB 钳制历史色;可选方差裁剪 μ±γσ)
// ---------------------------------------------------------------------------

/// RGB → YCoCg(可逆形式;亮度与色度分离,邻域统计在亮度上更稳)。
pub fn rgb_to_ycocg(rgb: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = rgb;
    [
        0.25 * r + 0.5 * g + 0.25 * b,
        0.5 * r - 0.5 * b,
        -0.25 * r + 0.5 * g - 0.25 * b,
    ]
}

/// YCoCg → RGB([`rgb_to_ycocg`] 的精确逆变换)。
pub fn ycocg_to_rgb(ycc: [f32; 3]) -> [f32; 3] {
    let [y, co, cg] = ycc;
    let tmp = y - cg;
    [tmp + co, y + cg, tmp - co]
}

/// 整图 RGB → YCoCg(要求 c ≥ 3,输出 3 通道)。
pub fn rgb_image_to_ycocg(img: &ImageF32) -> ImageF32 {
    assert!(img.c >= 3, "RGB 图必须 ≥3 通道");
    let mut out = ImageF32::new(img.w, img.h, 3);
    for y in 0..img.h {
        for x in 0..img.w {
            out.set_pixel3(x, y, rgb_to_ycocg(img.pixel3(x, y)));
        }
    }
    out
}

/// 整图 YCoCg → RGB(要求 c = 3,输出 3 通道)。
pub fn ycocg_image_to_rgb(img: &ImageF32) -> ImageF32 {
    assert_eq!(img.c, 3, "YCoCg 图必须 3 通道");
    let mut out = ImageF32::new(img.w, img.h, 3);
    for y in 0..img.h {
        for x in 0..img.w {
            out.set_pixel3(x, y, ycocg_to_rgb(img.pixel3(x, y)));
        }
    }
    out
}

/// 3x3 邻域 AABB(逐像素、逐通道 min/max,边界复制;输入为 YCoCg 图)。
pub fn neighborhood_aabb(cur_ycocg: &ImageF32) -> (ImageF32, ImageF32) {
    assert_eq!(cur_ycocg.c, 3, "邻域统计输入必须 3 通道");
    let (w, h) = (cur_ycocg.w, cur_ycocg.h);
    let mut lo = ImageF32::new(w, h, 3);
    let mut hi = ImageF32::new(w, h, 3);
    for y in 0..h {
        for x in 0..w {
            for ch in 0..3 {
                let mut mn = f32::INFINITY;
                let mut mx = f32::NEG_INFINITY;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let xx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                        let yy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                        let v = cur_ycocg.get(xx, yy, ch);
                        mn = mn.min(v);
                        mx = mx.max(v);
                    }
                }
                lo.set(x, y, ch, mn);
                hi.set(x, y, ch, mx);
            }
        }
    }
    (lo, hi)
}

/// 3x3 邻域方差裁剪边界 μ ± γσ(报告7 §2.1 可选方差裁剪;比 AABB 更紧,
/// 平坦区收敛更快,代价是可能钳掉真实高频)。
pub fn neighborhood_variance_bounds(cur_ycocg: &ImageF32, gamma: f32) -> (ImageF32, ImageF32) {
    assert_eq!(cur_ycocg.c, 3, "邻域统计输入必须 3 通道");
    let (w, h) = (cur_ycocg.w, cur_ycocg.h);
    let mut lo = ImageF32::new(w, h, 3);
    let mut hi = ImageF32::new(w, h, 3);
    for y in 0..h {
        for x in 0..w {
            for ch in 0..3 {
                let mut sum = 0.0f32;
                let mut sq = 0.0f32;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let xx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                        let yy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                        let v = cur_ycocg.get(xx, yy, ch);
                        sum += v;
                        sq += v * v;
                    }
                }
                let mean = sum / 9.0;
                let var = (sq / 9.0 - mean * mean).max(0.0);
                let sigma = var.sqrt();
                lo.set(x, y, ch, mean - gamma * sigma);
                hi.set(x, y, ch, mean + gamma * sigma);
            }
        }
    }
    (lo, hi)
}

/// 逐像素逐通道钳制到 [lo, hi](历史色钳入当前帧邻域色域)。
pub fn clamp_to_bounds(img: &ImageF32, lo: &ImageF32, hi: &ImageF32) -> ImageF32 {
    assert!(img.same_shape(lo) && img.same_shape(hi), "裁剪边界形状不符");
    let mut out = img.clone();
    for (o, (&l, &h)) in out.data.iter_mut().zip(lo.data.iter().zip(hi.data.iter())) {
        *o = o.clamp(l, h);
    }
    out
}

// ---------------------------------------------------------------------------
// 图集成描述(W3 demo 建图引用;资源名 + 冻结契约 AccessKind/TextureFormat
// 组合,跨帧历史一律 imported——报告5 §2.3 纪律:图只推导状态转换,不入
// transient 池,双缓冲轮换由调用方在图外持有)
// ---------------------------------------------------------------------------

/// TAA pass 单条资源声明。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporalResourceSpec {
    pub name: &'static str,
    pub access: AccessKind,
    pub format: TextureFormat,
    /// true = 跨帧外部资源(历史颜色/深度/法线;imported 语义,双缓冲轮换)。
    pub imported: bool,
}

/// TAA pass 图集成描述(声明面;执行体见 [`crate::temporal::taa`])。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalFrameDesc {
    pub pass_name: &'static str,
    pub resources: Vec<TemporalResourceSpec>,
}

/// TAA pass 所需资源组合。
///
/// 历史三件套(history_color/depth/normal prev)与 history_color_out 为 imported
/// 跨帧资源:第 N 帧的 history_color_out 即第 N+1 帧的 history_color_in(双缓冲,
/// 别名共享由调用方管理)。历史颜色 P0 取 Rgba16Float;P1 TSR 可降级 R11G11B10
/// 省带宽(报告7 §2.2 机制清单 1)。
pub fn taa_frame_desc() -> TemporalFrameDesc {
    use AccessKind::{ShaderRead, ShaderWrite};
    use TextureFormat::{R32Float, Rg16Float, Rgba16Float};
    TemporalFrameDesc {
        pass_name: "temporal.taa_resolve",
        resources: vec![
            TemporalResourceSpec {
                name: "taa.cur_color",
                access: ShaderRead,
                format: Rgba16Float,
                imported: false,
            },
            TemporalResourceSpec {
                name: "taa.history_color_in",
                access: ShaderRead,
                format: Rgba16Float,
                imported: true,
            },
            TemporalResourceSpec {
                name: "taa.depth_cur",
                access: ShaderRead,
                format: R32Float,
                imported: false,
            },
            TemporalResourceSpec {
                name: "taa.depth_prev",
                access: ShaderRead,
                format: R32Float,
                imported: true,
            },
            TemporalResourceSpec {
                name: "taa.normal_cur",
                access: ShaderRead,
                format: Rgba16Float,
                imported: false,
            },
            TemporalResourceSpec {
                name: "taa.normal_prev",
                access: ShaderRead,
                format: Rgba16Float,
                imported: true,
            },
            TemporalResourceSpec {
                name: "taa.mv",
                access: ShaderRead,
                format: Rg16Float,
                imported: false,
            },
            TemporalResourceSpec {
                name: "taa.history_color_out",
                access: ShaderWrite,
                format: Rgba16Float,
                imported: true,
            },
            TemporalResourceSpec {
                name: "taa.output",
                access: ShaderWrite,
                format: Rgba16Float,
                imported: false,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 相机沿 +x 平移、看向 -z 的 view_proj(纯平移 MV 测试用)。
    fn camera_vp(eye_x: f32) -> Mat4 {
        let proj = perspective_rh_zo(1.0, 1.0, 0.1, 100.0);
        let view = look_at_rh([eye_x, 0.0, 0.0], [eye_x, 0.0, -1.0], [0.0, 1.0, 0.0]);
        proj.mul(&view)
    }

    #[test]
    fn halton_known_values() {
        assert!((halton(1, 2) - 0.5).abs() < 1e-7);
        assert!((halton(2, 2) - 0.25).abs() < 1e-7);
        assert!((halton(3, 2) - 0.75).abs() < 1e-7);
        assert!((halton(1, 3) - 1.0 / 3.0).abs() < 1e-7);
        assert!((halton(2, 3) - 2.0 / 3.0).abs() < 1e-7);
        assert!((halton(3, 3) - 1.0 / 9.0).abs() < 1e-7);
    }

    #[test]
    fn jitter_zero_mean_distinct_in_range() {
        let seq = jitter_sequence(8);
        assert_eq!(seq.len(), 8);
        // 8 点均值 < 0.1(验收口径:均值趋 0)
        let (mx, my) = seq
            .iter()
            .fold((0.0f32, 0.0f32), |(ax, ay), j| (ax + j[0], ay + j[1]));
        assert!((mx / 8.0).abs() < 0.1, "mx={}", mx / 8.0);
        assert!((my / 8.0).abs() < 0.1, "my={}", my / 8.0);
        // 范围 [-0.5, 0.5)
        for j in &seq {
            assert!(j[0] >= -0.5 && j[0] < 0.5 && j[1] >= -0.5 && j[1] < 0.5);
        }
        // 互异
        for i in 0..8 {
            for k in (i + 1)..8 {
                assert!(
                    (seq[i][0] - seq[k][0]).abs() > 1e-7 || (seq[i][1] - seq[k][1]).abs() > 1e-7,
                    "({i},{k}) 重复"
                );
            }
        }
    }

    #[test]
    fn mat4_inverse_roundtrip() {
        let proj = perspective_rh_zo(1.1, 16.0 / 9.0, 0.1, 100.0);
        let view = look_at_rh([1.5, 2.0, 3.0], [0.0, 0.5, -1.0], [0.0, 1.0, 0.0]);
        let a = proj.mul(&view);
        let inv = a.inverse().expect("view_proj 可逆");
        // A·inv(A) ≈ I 且 inv(A)·A ≈ I
        for prod in [a.mul(&inv), inv.mul(&a)] {
            for r in 0..4 {
                for c in 0..4 {
                    let expect = if r == c { 1.0 } else { 0.0 };
                    assert!(
                        (prod.m[r][c] - expect).abs() < 1e-3,
                        "({r},{c}) = {}",
                        prod.m[r][c]
                    );
                }
            }
        }
        // 奇异矩阵返回 None
        let singular = Mat4 {
            m: [[1.0f32; 4]; 4],
        };
        assert!(singular.inverse().is_none());
    }

    #[test]
    fn mat4_mul_identity() {
        let a = perspective_rh_zo(0.9, 1.0, 0.5, 50.0);
        let b = a.mul(&Mat4::identity());
        for r in 0..4 {
            for c in 0..4 {
                assert!((b.m[r][c] - a.m[r][c]).abs() < 1e-7);
            }
        }
    }

    #[test]
    fn camera_mv_static_is_zero() {
        // 静止相机:重投影不动,MV ≈ 0(报告7 P0 验收口径之一)
        let depth = ImageF32::from_fn(16, 16, 1, |x, y, _| {
            0.7 + 0.28 * ((x * 7 + y * 13) % 11) as f32 / 10.0
        });
        let vp = camera_vp(0.0);
        let mv = compute_camera_mv(&depth, &vp, &vp);
        for &v in &mv.data {
            assert!(v.abs() < 1e-4, "静止 MV 非零:{v}");
        }
    }

    #[test]
    fn camera_mv_translation_direction_consistent() {
        // 相机纯平移 +x:静态世界点在屏幕上整体反向滑动,MV.x 全场同号、MV.y ≈ 0
        let depth = ImageF32::from_fn(16, 16, 1, |x, y, _| 0.7 + 0.25 * ((x + y) % 5) as f32 / 4.0);
        let prev = camera_vp(0.0);
        let cur = camera_vp(0.05);
        let mv = compute_camera_mv(&depth, &cur, &prev);
        for y in 0..16 {
            for x in 0..16 {
                let mx = mv.get(x, y, 0);
                let my = mv.get(x, y, 1);
                assert!(mx > 0.0, "({x},{y}) mx={mx} 应与全场同号");
                assert!(my.abs() < 1e-5, "({x},{y}) my={my} 应 ≈0");
            }
        }
    }

    #[test]
    fn ycocg_roundtrip() {
        for r in 0..5 {
            for g in 0..5 {
                for b in 0..5 {
                    let rgb = [r as f32 * 0.25, g as f32 * 0.25, b as f32 * 0.25];
                    let back = ycocg_to_rgb(rgb_to_ycocg(rgb));
                    for ch in 0..3 {
                        assert!((back[ch] - rgb[ch]).abs() < 1e-5);
                    }
                }
            }
        }
        // 整图往返
        let img = ImageF32::from_fn(8, 8, 3, |x, y, ch| {
            ((x * 3 + y * 5 + ch * 7) % 13) as f32 / 13.0
        });
        let back = ycocg_image_to_rgb(&rgb_image_to_ycocg(&img));
        for i in 0..img.data.len() {
            assert!((back.data[i] - img.data[i]).abs() < 1e-5);
        }
    }

    #[test]
    fn validate_history_accepts_static() {
        let depth = ImageF32::from_fn(8, 8, 1, |x, y, _| 0.5 + 0.01 * (x + y) as f32);
        let normal = ImageF32::from_fn(8, 8, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        let mask = validate_history(&depth, &depth, &normal, &normal, 0.1, 0.9);
        assert!(mask.data.iter().all(|&v| v > 0.5));
    }

    #[test]
    fn validate_history_rejects_depth_jump() {
        // 右半深度突变(遮挡揭开)→ 置 0;左半一致 → 保留
        let cur = ImageF32::from_fn(8, 8, 1, |_, _, _| 0.5);
        let mut prev = cur.clone();
        for y in 0..8 {
            for x in 4..8 {
                prev.set(x, y, 0, 0.05);
            }
        }
        let normal = ImageF32::from_fn(8, 8, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        let mask = validate_history(&cur, &prev, &normal, &normal, 0.1, 0.9);
        for y in 0..8 {
            for x in 0..4 {
                assert!(mask.get(x, y, 0) > 0.5, "({x},{y}) 应保留");
            }
            for x in 4..8 {
                assert!(mask.get(x, y, 0) < 0.5, "({x},{y}) 深度突变应拒绝");
            }
        }
    }

    #[test]
    fn validate_history_rejects_normal_flip() {
        let depth = ImageF32::from_fn(8, 8, 1, |_, _, _| 0.5);
        let cur_n = ImageF32::from_fn(8, 8, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        // 左半反向(dot=-1)、右半轻微倾斜(dot≈0.995)
        let prev_n = ImageF32::from_fn(8, 8, 3, |x, _, ch| {
            if x < 4 {
                if ch == 2 { -1.0 } else { 0.0 }
            } else {
                match ch {
                    0 => 0.1,
                    2 => (1.0f32 - 0.01f32).sqrt(),
                    _ => 0.0,
                }
            }
        });
        let mask = validate_history(&depth, &depth, &cur_n, &prev_n, 0.1, 0.9);
        for y in 0..8 {
            for x in 0..4 {
                assert!(mask.get(x, y, 0) < 0.5, "({x},{y}) 法线反向应拒绝");
            }
            for x in 4..8 {
                assert!(mask.get(x, y, 0) > 0.5, "({x},{y}) 轻微倾斜应保留");
            }
        }
    }

    #[test]
    fn reproject_out_of_screen_is_invalid() {
        // MV 把采样点推出屏 → disocclusion,屏内 mask 0
        let src = ImageF32::from_fn(8, 8, 1, |x, _, _| x as f32);
        let mut mv = ImageF32::new(8, 8, 2);
        for i in (0..mv.data.len()).step_by(2) {
            mv.data[i] = -1.0;
        }
        let (_, inside) = reproject_sample(&src, &mv);
        assert!(inside.data.iter().all(|&v| v < 0.5));
        // MV = 0:全屏内,且纹素中心重采样精确还原原图
        let mv0 = ImageF32::new(8, 8, 2);
        let (out, inside0) = reproject_sample(&src, &mv0);
        assert!(inside0.data.iter().all(|&v| v > 0.5));
        for i in 0..src.data.len() {
            assert!((out.data[i] - src.data[i]).abs() < 1e-6);
        }
        // 全链路:出屏与深度判据取交
        let normal = ImageF32::from_fn(8, 8, 3, |_, _, ch| if ch == 2 { 1.0 } else { 0.0 });
        let mask = validate_history_with_mv(&src, &src, &normal, &normal, &mv, 0.5, 0.9);
        assert!(mask.data.iter().all(|&v| v < 0.5));
    }

    #[test]
    fn neighborhood_aabb_clips_history() {
        // 5x5:中心 1.0、其余 0.0;角像素 3x3 不含中心 → AABB [0,0]
        let cur = ImageF32::from_fn(5, 5, 3, |x, y, _| if x == 2 && y == 2 { 1.0 } else { 0.0 });
        let (lo, hi) = neighborhood_aabb(&cur);
        assert!(lo.get(0, 0, 0).abs() < 1e-6 && hi.get(0, 0, 0).abs() < 1e-6);
        assert!((hi.get(2, 2, 0) - 1.0).abs() < 1e-6);
        // 历史 0.8:角处被钳到 0,中心处 0.8 ∈ [0,1] 保留
        let hist = ImageF32::from_fn(5, 5, 3, |_, _, _| 0.8);
        let clipped = clamp_to_bounds(&hist, &lo, &hi);
        assert!(clipped.get(0, 0, 0).abs() < 1e-6);
        assert!((clipped.get(2, 2, 0) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn variance_clip_bounds() {
        // 平坦区:σ=0,bounds 塌缩到 μ
        let flat = ImageF32::from_fn(4, 4, 3, |_, _, _| 0.5);
        let (lo, hi) = neighborhood_variance_bounds(&flat, 1.0);
        for i in 0..lo.data.len() {
            assert!((lo.data[i] - 0.5).abs() < 1e-5);
            assert!((hi.data[i] - 0.5).abs() < 1e-5);
        }
        // 渐变 3x3:中心 μ=0.5,σ=sqrt(1.5/9)
        let grad = ImageF32::from_fn(3, 3, 3, |x, _, _| x as f32 / 2.0);
        let (lo2, hi2) = neighborhood_variance_bounds(&grad, 1.0);
        let sigma = (1.5f32 / 9.0).sqrt();
        assert!((lo2.get(1, 1, 0) - (0.5 - sigma)).abs() < 1e-5);
        assert!((hi2.get(1, 1, 0) - (0.5 + sigma)).abs() < 1e-5);
    }

    #[test]
    fn taa_frame_desc_resources() {
        let desc = taa_frame_desc();
        assert_eq!(desc.pass_name, "temporal.taa_resolve");
        // 资源名互异
        let mut names: Vec<_> = desc.resources.iter().map(|r| r.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), desc.resources.len());
        // 历史四件 imported(报告5 §2.3 跨帧纪律)
        for name in [
            "taa.history_color_in",
            "taa.history_color_out",
            "taa.depth_prev",
            "taa.normal_prev",
        ] {
            let r = desc
                .resources
                .iter()
                .find(|r| r.name == name)
                .expect("资源存在");
            assert!(r.imported, "{name} 应为 imported");
        }
        // 写访问恰好两条:输出 + 历史回写
        let writes = desc
            .resources
            .iter()
            .filter(|r| r.access.is_write())
            .count();
        assert_eq!(writes, 2);
    }
}
