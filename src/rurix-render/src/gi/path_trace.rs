//! G9.4 M96 M17 Path Tracer 参照器 host 面(spec/global_illumination.md RXS-0357;
//! RFC-0022 §4.10;门 `g9.p0.m96.path_tracer_reference`)。
//!
//! 本模块 = megakernel 参照器的 **host 数据面/对拍面**:
//! - [`PtScene`] / [`PtCamera`] / [`PtLightQuad`]:确定性冻结场景 fixtures
//!   ([`m96_cornell_scene`] 漫反射 Cornell 类 + [`m96_direct_light_scene`] 直接光
//!   dominant),**与 pbrt-v4 共享场景/材质输入**([`pbrt_scene_text`] 导出同源
//!   生成;不共享光照算法——golden diff 可归因到算法层而非输入层,RXS-0357 L3);
//! - [`PtScene::validate`]:起步范围冻结的 fail-closed 承载——材质集合只含
//!   Lambert/发光两类,任何 specular/透射/体积材质 typed [`PtError`] 显式拒绝
//!   (焦散/体积/specular 链 out,RXS-0357 L1);
//! - [`rng`]:固定 seed 确定性协议的 RNG 流布局(RXS-0357 L2;承 G8
//!   `ref_tracer` PCG32 对拍模式)——单一流、按索引寻址、逐像素逐采样排布,
//!   device kernel 与 host oracle 消费**同一缓冲**;
//! - [`trace_host`]:host oracle(与 device megakernel 公式面逐字同源;
//!   NEE/MIS/RR 三要素与 [`PtSwitches`] 开关),用于单测数值锚与算法层佐证——
//!   **仅 host 输出不能充绿**(MAP M96 行),门绿由 device 腿承载;
//! - [`pack_params`] / [`pack_mats`] / [`pack_tris`]:device 输入打包(kernel
//!   头注参数面逐字同源);
//! - pbrt 对照面:[`pbrt_scene_text`] 场景导出 + [`read_pfm`] 图像回读 +
//!   [`rel_dev`] / [`rel_mae`] 收敛度量 + [`ToleranceBand`] 冻结容差带
//!   比对器(measured 后冻结,禁手写,P-09;fail-closed)。
//!
//! ## 确定性协议(RXS-0357 L2 冻结字面)
//! - **固定 seed 两次运行位级一致**:同 seed ⇒ RNG 流位级一致 ⇒ device 输出
//!   (out_rgb‖out_stats‖out_samples 字节)位级一致;canonical digest =
//!   SHA-256(三路输出字节依序拼接),不含路径/mtime/随机 seed。
//! - **累加序与 RNG 流冻结**:逐像素独立顺序累加(禁 atomic 顺序敏感累加);
//!   PCG32 流推进序 = 逐像素逐采样图序;采样维序 = [cam_u, cam_v] 后每 bounce
//!   [nee_u, nee_v, bsdf_r1, bsdf_r2, rr](见 [`rng`])。
//! - **逐像素 sample count 导出 + 方差/收敛曲线进 evidence**:`out_samples`
//!   逐像素导出;`out_stats` 携带 Σlum/Σlum²(方差 = E[x²]−E[x]²);收敛曲线 =
//!   spp 序列上的 rel-MAE(vs pbrt 1024spp 参照),进 evidence 字段。
//! - **匹配深度**:max_bounces = 4(1/2/full 三深度 golden 由下游 GI 档位各自
//!   定义容差前提,本参照器按冻结深度产 golden)。

use crate::rt::bvh::{InstanceDesc, Ray, Tlas, Transform3x4, TriBvh, Vec3};
use crate::rt::ref_tracer::RAY_EPS;

/// π 的 f32 冻结常量(与 device kernel `let pi: f32 = 3.1415927` 位级同值——
/// f32 最近邻 π;`core::f32::consts::PI` 即该位模式)。
pub const PT_PI: f32 = core::f32::consts::PI;

// ---------------------------------------------------------------------------
// 错误面(fail-closed typed Err;严禁 UB,本模块一切失败为类型化拒绝)
// ---------------------------------------------------------------------------

/// M96 host 面错误(装配/装载/比对全部 fail-closed)。
#[derive(Debug, Clone, PartialEq)]
pub enum PtError {
    /// 起步范围外材质(specular/透射/体积)——RXS-0357 L1 显式拒绝。
    OutOfScopeMaterial {
        ///  offending 三角形下标。
        tri: u32,
        /// 材质类别名。
        kind: &'static str,
    },
    /// 场景结构非法(空场景/光源 quad 与发光三角不匹配/相机非有限等)。
    InvalidScene(String),
    /// 配置非法(spp=0 / max_bounces=0 等)。
    InvalidConfig(String),
    /// pbrt 对照面错误(PFM 解析失败/容差带文件损坏/偏差越带等)。
    PbrtBridge(String),
}

impl std::fmt::Display for PtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PtError::OutOfScopeMaterial { tri, kind } => write!(
                f,
                "起步范围冻结:三角 {tri} 材质类别 {kind} 非 Lambert/发光(RXS-0357 L1:焦散/体积/specular 链 out)"
            ),
            PtError::InvalidScene(m) => write!(f, "场景非法: {m}"),
            PtError::InvalidConfig(m) => write!(f, "配置非法: {m}"),
            PtError::PbrtBridge(m) => write!(f, "pbrt 对照面: {m}"),
        }
    }
}

impl std::error::Error for PtError {}

// ---------------------------------------------------------------------------
// 场景类型(冻结 fixtures;与 pbrt 共享输入)
// ---------------------------------------------------------------------------

/// 材质类别。起步范围冻结:**仅 Lambert / Emission 两类合法**;其余类别构造
/// 即为了被拒绝(specular 链/体积/焦散 out——[`PtScene::validate`] typed Err)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MaterialKind {
    /// Lambert 漫反射(线性 albedo)。
    Lambert {
        /// 线性空间反照率 RGB。
        albedo: [f32; 3],
    },
    /// 发光面(Lambert 基底 + 单面发光;光源 quad 专属)。
    Emission {
        /// 线性空间反照率 RGB(pbrt 侧默认 matte 对齐)。
        albedo: [f32; 3],
        /// 发光辐射度 RGB(线性)。
        emission: [f32; 3],
    },
    /// 镜面(specular 链源;起步范围 out,validate 必拒)。
    Specular {
        /// 反射率 RGB。
        reflectance: [f32; 3],
    },
    /// 透射(焦散源;起步范围 out,validate 必拒)。
    Transmission {
        /// 透射率 RGB。
        transmittance: [f32; 3],
    },
    /// 参与介质(体积;起步范围 out,validate 必拒)。
    Volume {
        /// 密度。
        density: f32,
    },
}

/// 相机(pinhole;方形画幅 ⇒ 水平 fov = 垂直 fov,与 pbrt `perspective` 对齐)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PtCamera {
    /// 眼点(世界空间)。
    pub origin: [f32; 3],
    /// 前向单位向量(look_at − eye 归一)。
    pub forward: [f32; 3],
    /// 右向单位向量(forward × up_world 归一,与 pbrt LookAt 同式)。
    pub right: [f32; 3],
    /// 上向单位向量(right × forward)。
    pub up: [f32; 3],
    /// tan(fov/2)(弧度制半角正切)。
    pub tan_half_fov: f32,
    /// 图宽(像素)。
    pub width: u32,
    /// 图高(像素)。
    pub height: u32,
}

impl PtCamera {
    /// look-at 构造(与 pbrt-v4 `LookAt` **逐字同式**:right = up×dir、
    /// up = dir×right,util/transform.cpp:96-97;`fov_deg` = 垂直全角,度数;
    /// 方形画幅 ⇒ 水平 = 垂直)。左右手系错位 = 图像水平镜像,实测锚:
    /// cornell spp64 rel_dev 0.295(错)→ 0.113(对)。
    pub fn look_at(
        eye: [f32; 3],
        at: [f32; 3],
        up_world: [f32; 3],
        fov_deg: f32,
        width: u32,
        height: u32,
    ) -> PtCamera {
        let eye = Vec3::from_array(eye);
        let forward = (Vec3::from_array(at) - eye).normalize();
        let right = Vec3::from_array(up_world).cross(forward).normalize();
        let up = forward.cross(right);
        PtCamera {
            origin: eye.to_array(),
            forward: forward.to_array(),
            right: right.to_array(),
            up: up.to_array(),
            tan_half_fov: (fov_deg.to_radians() * 0.5).tan(),
            width,
            height,
        }
    }
}

/// 光源 quad(平行四边形;p00 + e1·u + e2·v,(u,v)∈[0,1)²;单面发光,
/// 法线 = normalize(cross(e1, e2)) 绕向决定)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PtLightQuad {
    /// 角点 p00。
    pub p00: [f32; 3],
    /// 边 e1 = p10 − p00。
    pub e1: [f32; 3],
    /// 边 e2 = p01 − p00。
    pub e2: [f32; 3],
    /// 发光辐射度 RGB(线性)。
    pub emission: [f32; 3],
}

impl PtLightQuad {
    /// 绕向法线(单位长;发光单面方向)。
    pub fn normal(&self) -> [f32; 3] {
        let e1 = Vec3::from_array(self.e1);
        let e2 = Vec3::from_array(self.e2);
        e1.cross(e2).normalize().to_array()
    }

    /// 面积 |cross(e1, e2)|。
    pub fn area(&self) -> f32 {
        let e1 = Vec3::from_array(self.e1);
        let e2 = Vec3::from_array(self.e2);
        e1.cross(e2).length()
    }
}

/// M96 参照器场景(单 BLAS 三角形汤 + 逐三角材质 + 单光源 quad + 相机)。
#[derive(Debug, Clone)]
pub struct PtScene {
    /// 稳定场景名(容差带/golden 键)。
    pub name: &'static str,
    /// 顶点位置(世界空间)。
    pub positions: Vec<[f32; 3]>,
    /// 三角形索引(顶点序 = 绕向 = 几何法线;device `primitiveIndex` 序)。
    pub indices: Vec<[u32; 3]>,
    /// 逐三角材质(与 `indices` 同长)。
    pub materials: Vec<MaterialKind>,
    /// 光源 quad(与发光三角几何**逐字一致**——validate 机核)。
    pub light: PtLightQuad,
    /// 相机。
    pub camera: PtCamera,
    /// 场景射线 t 上界(t_max;场景对角线外)。
    pub t_max: f32,
}

/// 场景校验(起步范围冻结的 fail-closed 承载;RXS-0357 L1)。
impl PtScene {
    /// fail-closed 校验:
    /// - 材质集合仅 Lambert/Emission(其余类别 typed Err 显式拒绝);
    /// - 发光三角集合 = 光源 quad 的两个半三角,几何逐字一致、绕向法线与
    ///   quad 法线一致、面积各半;
    /// - 全部顶点/材质参数有限非负;场景非空;相机基向量正交单位。
    pub fn validate(&self) -> Result<(), PtError> {
        if self.indices.is_empty() || self.positions.is_empty() {
            return Err(PtError::InvalidScene("空场景".into()));
        }
        if self.materials.len() != self.indices.len() {
            return Err(PtError::InvalidScene(format!(
                "材质数 {} ≠ 三角数 {}",
                self.materials.len(),
                self.indices.len()
            )));
        }
        for (t, idx) in self.indices.iter().enumerate() {
            for &vi in idx {
                if vi as usize >= self.positions.len() {
                    return Err(PtError::InvalidScene(format!(
                        "三角 {t} 索引 {vi} 越界(顶点数 {})",
                        self.positions.len()
                    )));
                }
            }
        }
        for (i, p) in self.positions.iter().enumerate() {
            if !p.iter().all(|c| c.is_finite()) {
                return Err(PtError::InvalidScene(format!("顶点 {i} 非有限")));
            }
        }
        let mut emissive: Vec<usize> = Vec::new();
        for (t, m) in self.materials.iter().enumerate() {
            match m {
                MaterialKind::Lambert { albedo } => {
                    if !albedo.iter().all(|c| c.is_finite() && *c >= 0.0 && *c < 1.0) {
                        return Err(PtError::InvalidScene(format!(
                            "三角 {t} albedo 越域 [0,1):{albedo:?}"
                        )));
                    }
                }
                MaterialKind::Emission { albedo, emission } => {
                    if !albedo.iter().all(|c| c.is_finite() && *c >= 0.0 && *c < 1.0) {
                        return Err(PtError::InvalidScene(format!(
                            "三角 {t} 发光面 albedo 越域:{albedo:?}"
                        )));
                    }
                    if !emission.iter().all(|c| c.is_finite() && *c >= 0.0) {
                        return Err(PtError::InvalidScene(format!(
                            "三角 {t} emission 非有限/负:{emission:?}"
                        )));
                    }
                    emissive.push(t);
                }
                MaterialKind::Specular { .. } => {
                    return Err(PtError::OutOfScopeMaterial {
                        tri: t as u32,
                        kind: "specular",
                    });
                }
                MaterialKind::Transmission { .. } => {
                    return Err(PtError::OutOfScopeMaterial {
                        tri: t as u32,
                        kind: "transmission(焦散源)",
                    });
                }
                MaterialKind::Volume { .. } => {
                    return Err(PtError::OutOfScopeMaterial {
                        tri: t as u32,
                        kind: "volume(体积)",
                    });
                }
            }
        }
        // 光源 quad ↔ 发光三角逐字一致(两半三角,各半面积,绕向法线一致)。
        if emissive.len() != 2 {
            return Err(PtError::InvalidScene(format!(
                "发光三角数 {} ≠ 2(单光源 quad 纪律)",
                emissive.len()
            )));
        }
        let p00 = Vec3::from_array(self.light.p00);
        let p10 = p00 + Vec3::from_array(self.light.e1);
        let p01 = p00 + Vec3::from_array(self.light.e2);
        let p11 = p01 + Vec3::from_array(self.light.e1);
        let expected: [[Vec3; 3]; 2] = [[p00, p10, p11], [p00, p11, p01]];
        let ln = Vec3::from_array(self.light.normal());
        let area = self.light.area();
        if !(area.is_finite() && area > 0.0) {
            return Err(PtError::InvalidScene("光源 quad 面积非正".into()));
        }
        for (k, &t) in emissive.iter().enumerate() {
            let tri = self.indices[t];
            let vs = [
                Vec3::from_array(self.positions[tri[0] as usize]),
                Vec3::from_array(self.positions[tri[1] as usize]),
                Vec3::from_array(self.positions[tri[2] as usize]),
            ];
            for (j, (v, e)) in vs.iter().zip(expected[k].iter()).enumerate() {
                if v.to_array() != e.to_array() {
                    return Err(PtError::InvalidScene(format!(
                        "发光三角 {t} 顶点 {j} 与光源 quad 不逐字一致:{v:?} vs {e:?}"
                    )));
                }
            }
            let n = (vs[1] - vs[0]).cross(vs[2] - vs[0]);
            if n.dot(ln) <= 0.0 {
                return Err(PtError::InvalidScene(format!(
                    "发光三角 {t} 绕向法线与光源 quad 法线反向"
                )));
            }
            if (n.length() - area).abs() > 1e-6 * area {
                return Err(PtError::InvalidScene(format!(
                    "发光三角 {t} 面积 {} ≠ quad 面积 {area}",
                    n.length()
                )));
            }
        }
        // 相机基向量正交单位 + 有限。
        let (f, r, u) = (
            Vec3::from_array(self.camera.forward),
            Vec3::from_array(self.camera.right),
            Vec3::from_array(self.camera.up),
        );
        for (nm, v) in [("forward", f), ("right", r), ("up", u)] {
            if !v.is_finite() || (v.length() - 1.0).abs() > 1e-5 {
                return Err(PtError::InvalidScene(format!("相机 {nm} 非单位/非有限")));
            }
        }
        if f.dot(r).abs() > 1e-4 || f.dot(u).abs() > 1e-4 {
            return Err(PtError::InvalidScene("相机基向量非正交".into()));
        }
        if !(self.t_max.is_finite() && self.t_max > 0.0) {
            return Err(PtError::InvalidScene("t_max 非正".into()));
        }
        Ok(())
    }

    /// 单 BLAS 三角形汤(9 f32/三角,序 = `indices` 序 = device primitiveIndex)。
    pub fn blas_triangles(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.indices.len() * 9);
        for idx in &self.indices {
            for &vi in idx {
                out.extend_from_slice(&self.positions[vi as usize]);
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// 冻结 fixtures(RXS-0357 L3:两个对照场景——漫反射 Cornell 类 + 直接光 dominant)
// ---------------------------------------------------------------------------

/// 四边形 → 两三角(顶点序 (a,b,c),(a,c,d);法线 = normalize(cross(b−a, c−a))
/// 由调用方选点序决定)。
fn quad(
    positions: &mut Vec<[f32; 3]>,
    indices: &mut Vec<[u32; 3]>,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    d: [f32; 3],
) {
    let base = positions.len() as u32;
    positions.extend_from_slice(&[a, b, c, d]);
    indices.push([base, base + 1, base + 2]);
    indices.push([base, base + 2, base + 3]);
}

/// 四边形 + 材质(两三角同材质)。
#[allow(clippy::too_many_arguments)]
fn push_quad(
    positions: &mut Vec<[f32; 3]>,
    indices: &mut Vec<[u32; 3]>,
    materials: &mut Vec<MaterialKind>,
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    d: [f32; 3],
    m: MaterialKind,
) {
    quad(positions, indices, a, b, c, d);
    materials.push(m);
    materials.push(m);
}

/// 分量积(Vec3 无 Mul<Vec3>;host oracle 的 throughput×albedo 类运算载体)。
fn cmul(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x * b.x, a.y * b.y, a.z * b.z)
}

/// 轴对齐盒(12 三角;绕向朝外)。
fn add_box(
    positions: &mut Vec<[f32; 3]>,
    indices: &mut Vec<[u32; 3]>,
    min: [f32; 3],
    max: [f32; 3],
) {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    // 外法线盒:每面四点按外绕向。
    // −y 底 / +y 顶 / −z 前 / +z 后 / −x 左 / +x 右。
    quad(positions, indices, [x0, y0, z0], [x0, y0, z1], [x1, y0, z1], [x1, y0, z0]); // −y
    quad(positions, indices, [x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]); // +y
    quad(positions, indices, [x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]); // −z
    quad(positions, indices, [x0, y0, z1], [x0, y1, z1], [x1, y1, z1], [x1, y0, z1]); // +z
    quad(positions, indices, [x0, y0, z0], [x0, y1, z0], [x0, y1, z1], [x0, y0, z1]); // −x
    quad(positions, indices, [x1, y0, z0], [x1, y0, z1], [x1, y1, z1], [x1, y1, z0]); // +x
}

/// 冻结 fixture①:Cornell-box 类漫反射场景(单位盒 [0,1]³,正面 z=0 开口,
/// 天花下挂单面发光 quad,中央一单盒;左红右绿墙壁)。
///
/// 几何/材质/相机全部冻结常量;发光两三角与光源 quad 逐字一致(validate 机核)。
/// pbrt 对照场景经 [`pbrt_scene_text`] 从同一 fixture 导出(共享输入面)。
pub fn m96_cornell_scene() -> PtScene {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();
    let mut materials: Vec<MaterialKind> = Vec::new();
    let white = [0.73, 0.73, 0.73];
    let red = [0.61, 0.06, 0.06];
    let green = [0.12, 0.45, 0.15];
    // 地板 y=0(法线 +y)。
    push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, 0.0],
        MaterialKind::Lambert { albedo: white },
    );
    // 天花 y=1(法线 −y)。
    push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 1.0, 0.0],
        [1.0, 1.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
        MaterialKind::Lambert { albedo: white },
    );
    // 后墙 z=1(法线 −z 朝室内)。
    push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
        MaterialKind::Lambert { albedo: white },
    );
    // 左墙 x=0(法线 +x,红)。
    push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0],
        MaterialKind::Lambert { albedo: red },
    );
    // 右墙 x=1(法线 −x,绿)。
    push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
        MaterialKind::Lambert { albedo: green },
    );
    // 中央单盒(灰;0.30×0.55×0.30 坐地)。
    let box_base_tri = indices.len();
    add_box(
        &mut positions,
        &mut indices,
        [0.42, 0.0, 0.38],
        [0.72, 0.55, 0.68],
    );
    for _ in box_base_tri..indices.len() {
        materials.push(MaterialKind::Lambert { albedo: [0.60, 0.60, 0.60] });
    }
    // 光源 quad(天花下挂 y=0.995,法线 −y;emission 12)。
    let lp00 = [0.35, 0.995, 0.35];
    let le1 = [0.30, 0.0, 0.0];
    let le2 = [0.0, 0.0, 0.30];
    let light = PtLightQuad {
        p00: lp00,
        e1: le1,
        e2: le2,
        emission: [12.0, 12.0, 12.0],
    };
    let lp10 = [lp00[0] + le1[0], lp00[1], lp00[2]];
    let lp01 = [lp00[0], lp00[1], lp00[2] + le2[2]];
    let lp11 = [lp10[0], lp10[1], lp01[2]];
    // 绕向使法线 = −y:(p00, p10, p11),(p00, p11, p01)。
    push_quad(
        &mut positions,
        &mut indices,
        &mut materials,
        lp00,
        lp10,
        lp11,
        lp01,
        MaterialKind::Emission {
            albedo: [0.5, 0.5, 0.5],
            emission: light.emission,
        },
    );
    let camera = PtCamera::look_at(
        [0.5, 0.5, -0.9],
        [0.5, 0.5, 0.55],
        [0.0, 1.0, 0.0],
        50.0,
        64,
        64,
    );
    PtScene {
        name: "m96_cornell",
        positions,
        indices,
        materials,
        light,
        camera,
        t_max: 100.0,
    }
}

/// 冻结 fixture②:直接光 dominant 场景(地板 + 顶置光源 quad,开放空间——
/// 多反弹贡献 ≈ 0,直接光/NEE 等价性的紧对照)。
pub fn m96_direct_light_scene() -> PtScene {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();
    let mut materials: Vec<MaterialKind> = Vec::new();
    // 地板 y=0,x,z ∈ [0,2](法线 +y,灰 0.7)。
    quad(
        &mut positions,
        &mut indices,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 2.0],
        [2.0, 0.0, 2.0],
        [2.0, 0.0, 0.0],
    );
    materials.push(MaterialKind::Lambert { albedo: [0.7, 0.7, 0.7] });
    materials.push(MaterialKind::Lambert { albedo: [0.7, 0.7, 0.7] });
    // 光源 quad(y=1.2,x,z ∈ [0.7,1.3],法线 −y;emission 8)。
    let lp00 = [0.7, 1.2, 0.7];
    let le1 = [0.6, 0.0, 0.0];
    let le2 = [0.0, 0.0, 0.6];
    let light = PtLightQuad {
        p00: lp00,
        e1: le1,
        e2: le2,
        emission: [8.0, 8.0, 8.0],
    };
    let lp10 = [lp00[0] + le1[0], lp00[1], lp00[2]];
    let lp01 = [lp00[0], lp00[1], lp00[2] + le2[2]];
    let lp11 = [lp10[0], lp10[1], lp01[2]];
    quad(&mut positions, &mut indices, lp00, lp10, lp11, lp01);
    materials.push(MaterialKind::Emission {
        albedo: [0.5, 0.5, 0.5],
        emission: light.emission,
    });
    materials.push(MaterialKind::Emission {
        albedo: [0.5, 0.5, 0.5],
        emission: light.emission,
    });
    let camera = PtCamera::look_at(
        [1.0, 1.1, -1.2],
        [1.0, 0.1, 0.9],
        [0.0, 1.0, 0.0],
        45.0,
        64,
        64,
    );
    PtScene {
        name: "m96_direct",
        positions,
        indices,
        materials,
        light,
        camera,
        t_max: 100.0,
    }
}

/// 冻结场景集(pbrt 对照 ≥2 场景判据面;序 = 容差带/golden 键序)。
pub fn m96_scenes() -> Vec<PtScene> {
    vec![m96_cornell_scene(), m96_direct_light_scene()]
}

// ---------------------------------------------------------------------------
// 运行配置与开关(NEE/MIS/RR 三要素各自可开关 = RED 臂承载面)
// ---------------------------------------------------------------------------

/// 三要素开关(MIS/RR 可关 = RED 臂;NEE 恒开——关 NEE 不属条款 RED 面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtSwitches {
    /// MIS(BSDF×光源双分布 balance heuristic);关 = 两策略裸加(RED 臂)。
    pub mis: bool,
    /// 俄罗斯轮盘;关 = 路径恒跑满 max_bounces(RED 臂)。
    pub rr: bool,
}

impl PtSwitches {
    /// 正例臂全开关。
    pub const REFERENCE: PtSwitches = PtSwitches { mis: true, rr: true };
}

/// 运行配置(冻结协议面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtConfig {
    /// 每像素采样数(spp)。
    pub spp: u32,
    /// 最大 bounce 数(匹配深度;冻结 = 4)。
    pub max_bounces: u32,
    /// RR 起始 bounce(< 此 bounce 不轮盘)。
    pub rr_min_bounce: u32,
    /// 固定 seed(device 腿 RNG 流)。
    pub seed: u64,
    /// 开关。
    pub switches: PtSwitches,
}

impl PtConfig {
    /// 冻结基线(spp 由序列驱动;seed = [`M96_SEED`])。
    pub fn reference(spp: u32) -> PtConfig {
        PtConfig {
            spp,
            max_bounces: M96_MAX_BOUNCES,
            rr_min_bounce: M96_RR_MIN_BOUNCE,
            seed: M96_SEED,
            switches: PtSwitches::REFERENCE,
        }
    }

    /// fail-closed 配置校验。
    pub fn validate(&self) -> Result<(), PtError> {
        if self.spp == 0 {
            return Err(PtError::InvalidConfig("spp = 0".into()));
        }
        if self.max_bounces == 0 {
            return Err(PtError::InvalidConfig("max_bounces = 0".into()));
        }
        if self.rr_min_bounce >= self.max_bounces {
            return Err(PtError::InvalidConfig(format!(
                "rr_min_bounce {} ≥ max_bounces {}",
                self.rr_min_bounce, self.max_bounces
            )));
        }
        Ok(())
    }
}

/// 冻结 seed(device 腿;canonical digest 不含此值——digest 是输出字节哈希)。
pub const M96_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
/// 冻结最大 bounce(匹配深度 full 档深度)。
pub const M96_MAX_BOUNCES: u32 = 4;
/// 冻结 RR 起始 bounce(= 2 < max_bounces−1:轮盘终止效应须可传播到后续
/// bounce 贡献,否则跳 RR 臂数学上不可检出——实测锚 `host_oracle_red_arms_detectable`)。
pub const M96_RR_MIN_BOUNCE: u32 = 2;
/// 冻结 spp 序列(收敛曲线采样点)。
pub const M96_SPP_SEQUENCE: [u32; 4] = [1, 4, 16, 64];
/// pbrt 参照 spp(收敛曲线零点)。
pub const M96_PBRT_REF_SPP: u32 = 1024;
/// pbrt 腿固定 seed(与 device seed 独立;两侧估计量同均值,seed 无需对齐;
/// 32 位域——pbrt `integer` 形参为 32 位有符号,实测 >2³¹ 解析失败)。
pub const M96_PBRT_SEED: u64 = 0x5DEE_CE66;

// ---------------------------------------------------------------------------
// RNG 流布局(确定性协议核心;RXS-0357 L2;承 G8 ref_tracer PCG32 对拍模式)
// ---------------------------------------------------------------------------

/// 固定 seed 确定性协议的 RNG 流面(device kernel 与 host oracle 同源消费)。
///
/// **冻结布局**(采样维序/流推进序为协议面):
/// - 单一流:[`Pcg32`] 以 `seed` 播种,按**像素图序 × 采样序 × 维序**顺序产出
///   全部 `f32`;
/// - 每采样维数 = 2(相机 jitter)+ [`DIMS_PER_BOUNCE`] × max_bounces;
/// - 每 bounce 维序 = `[nee_u, nee_v, bsdf_r1, bsdf_r2, rr]`;
/// - 内核/host 一律**按索引寻址**(流位置与路径动态无关:RR 提前灭活不改
///   后续采样起始索引)。
pub mod rng {
    use crate::rt::ref_tracer::Pcg32;

    /// 每 bounce 随机维数(NEE 2 + BSDF 2 + RR 1)。
    pub const DIMS_PER_BOUNCE: usize = 5;
    /// 每采样相机维数。
    pub const DIMS_CAMERA: usize = 2;

    /// 每采样 floats(= 2 + 5·max_bounces)。
    pub fn sample_stride(max_bounces: u32) -> usize {
        DIMS_CAMERA + DIMS_PER_BOUNCE * max_bounces as usize
    }

    /// 流总长(= pixel_count · spp · sample_stride)。
    pub fn stream_len(pixel_count: usize, spp: u32, max_bounces: u32) -> usize {
        pixel_count * spp as usize * sample_stride(max_bounces)
    }

    /// 采样 (pixel, sample) 的流起始下标。
    pub fn sample_base(pixel: usize, sample: usize, spp: u32, max_bounces: u32) -> usize {
        (pixel * spp as usize + sample) * sample_stride(max_bounces)
    }

    /// bounce `b` 的五维在采样段内的偏移。
    pub fn bounce_base(sample_base: usize, bounce: usize) -> usize {
        sample_base + DIMS_CAMERA + bounce * DIMS_PER_BOUNCE
    }

    /// 生成整条流(单 Pcg32 实例,图序顺序产出;承 G8 对拍模式)。
    pub fn generate_stream(pixel_count: usize, spp: u32, max_bounces: u32, seed: u64) -> Vec<f32> {
        let mut rng = Pcg32::new(seed);
        let mut out = Vec::with_capacity(stream_len(pixel_count, spp, max_bounces));
        for _ in 0..stream_len(pixel_count, spp, max_bounces) {
            out.push(rng.next_f32());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// G12.2 生产化核心波子模块(RXS-0398~0401;RFC-0029)——M96 参照器冻结面 0-byte
// 只消费不回写;生产化 host 数据面/host oracle 与 device megakernel
// `kernels/g12_pt_production.rx` 公式面逐字同源。
// ---------------------------------------------------------------------------

/// G12.2 生产化路径追踪 host 面(MIS 完整面/吞吐自适应 RR/低差异序列采样/
/// 自适应收敛判据;详见子模块文档)。
pub mod prod;

// ---------------------------------------------------------------------------
// device 输入打包(kernel 头注参数面逐字同源)
// ---------------------------------------------------------------------------

/// 材质打包:8 f32/三角(albedo.rgb, emission.rgb, flags=0, pad)。
pub fn pack_mats(scene: &PtScene) -> Vec<f32> {
    let mut out = Vec::with_capacity(scene.indices.len() * 8);
    for m in &scene.materials {
        let (albedo, emission) = match m {
            MaterialKind::Lambert { albedo } => (*albedo, [0.0; 3]),
            MaterialKind::Emission { albedo, emission } => (*albedo, *emission),
            // validate 先行拒绝;打包遇范围外类别确定性置零(不产路径)。
            _ => ([0.0; 3], [0.0; 3]),
        };
        out.extend_from_slice(&albedo);
        out.extend_from_slice(&emission);
        out.push(0.0); // flags(0 = Lambert;内核不消费,留 provenance)
        out.push(0.0); // pad
    }
    out
}

/// 三角形打包:9 f32/三角(序 = `indices` 序 = device primitiveIndex)。
pub fn pack_tris(scene: &PtScene) -> Vec<f32> {
    scene.blas_triangles()
}

/// 参数打包:42 f32(kernel 头注布局逐字同源)。
pub fn pack_params(scene: &PtScene, cfg: &PtConfig) -> Vec<f32> {
    let cam = &scene.camera;
    let l = &scene.light;
    let ln = l.normal();
    let pixel_count = cam.width * cam.height;
    let mut p = Vec::with_capacity(42);
    p.push(pixel_count as f32);
    p.push(cfg.spp as f32);
    p.push(cfg.max_bounces as f32);
    p.push(cam.width as f32);
    p.push(cam.height as f32);
    p.push(if cfg.switches.mis { 1.0 } else { 0.0 });
    p.push(if cfg.switches.rr { 1.0 } else { 0.0 });
    p.push(cfg.rr_min_bounce as f32);
    p.push(RAY_EPS);
    p.push(scene.t_max);
    p.extend_from_slice(&cam.origin);
    p.extend_from_slice(&cam.forward);
    p.extend_from_slice(&cam.right);
    p.extend_from_slice(&cam.up);
    p.push(cam.tan_half_fov);
    p.push(1.0 / cam.width as f32);
    p.push(1.0 / cam.height as f32);
    p.extend_from_slice(&l.p00);
    p.extend_from_slice(&l.e1);
    p.extend_from_slice(&l.e2);
    p.push(l.area());
    p.extend_from_slice(&l.emission);
    p.extend_from_slice(&ln);
    p.push(rng::sample_stride(cfg.max_bounces) as f32);
    debug_assert_eq!(p.len(), 42);
    p
}

// ---------------------------------------------------------------------------
// host oracle(与 device megakernel 公式面同源;RXS-0357 L2 对拍模式)
// ---------------------------------------------------------------------------

/// 渲染输出(逐像素均值 RGB + Σlum/Σlum² + 实际采样数)。
#[derive(Debug, Clone, PartialEq)]
pub struct PtImage {
    /// 图宽。
    pub width: u32,
    /// 图高。
    pub height: u32,
    /// 逐像素均值辐射度 RGB(3 f32/px,线性)。
    pub rgb: Vec<f32>,
    /// 逐像素亮度累加 Σlum。
    pub sum_lum: Vec<f32>,
    /// 逐像素亮度平方累加 Σlum²(方差 = Σlum²/spp − (Σlum/spp)²)。
    pub sumsq_lum: Vec<f32>,
    /// 逐像素实际采样数(sample count 导出面)。
    pub samples: Vec<u32>,
}

impl PtImage {
    /// 像素数。
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// 全图均值亮度。
    pub fn mean_luminance(&self) -> f64 {
        let mut acc = 0.0f64;
        for px in 0..self.pixel_count() {
            let l = (f64::from(self.rgb[px * 3])
                + f64::from(self.rgb[px * 3 + 1])
                + f64::from(self.rgb[px * 3 + 2]))
                / 3.0;
            acc += l;
        }
        acc / self.pixel_count() as f64
    }

    /// 全图平均逐像素方差(亮度域;收敛曲线/evidence 字段面)。
    pub fn mean_pixel_variance(&self) -> f64 {
        let mut acc = 0.0f64;
        for px in 0..self.pixel_count() {
            let n = f64::from(self.samples[px].max(1));
            let mean = f64::from(self.sum_lum[px]) / n;
            let var = (f64::from(self.sumsq_lum[px]) / n - mean * mean).max(0.0);
            acc += var;
        }
        acc / self.pixel_count() as f64
    }
}

/// BSDF(Lambert)求值 = albedo/π(数值锚面)。
pub fn lambert_bsdf(albedo: f32) -> f32 {
    albedo / PT_PI
}

/// 余弦加权半球采样 pdf = cos/π(数值锚面)。
pub fn cosine_hemisphere_pdf(cos: f32) -> f32 {
    cos / PT_PI
}

/// MIS power-2 heuristic(光源策略权;与 pbrt-v4 `PowerHeuristic` 逐字同形,
/// 数值锚面)——`w_l = pdf_l²/(pdf_l²+pdf_b²)` 的安全形
/// `1/(1 + (pdf_b·area·cos_l/dist²)²)`(无零除;cos_l 调用方已截断 ≥0)。
pub fn mis_weight_light(pdf_b: f32, area: f32, cos_l: f32, dist2: f32) -> f32 {
    let r = pdf_b * area * cos_l / dist2;
    1.0 / (1.0 + r * r)
}

/// MIS power-2 heuristic(BSDF 策略权;pbrt 同形,数值锚面)——
/// `w_b = pdf_b²/(pdf_b²+pdf_l²)` 的安全形 `1/(1 + (t²/(area·cos_emit·pdf_b))²)`。
pub fn mis_weight_bsdf(t: f32, area: f32, cos_emit: f32, pdf_b: f32) -> f32 {
    let r = (t * t) / (area * cos_emit * pdf_b);
    1.0 / (1.0 + r * r)
}

/// 单条路径求值(host oracle;与 kernel 公式面同源——分支形态可读化,
/// 算术式逐字同式)。`stream` = [`rng::generate_stream`] 产物(同缓冲同索引)。
#[allow(clippy::too_many_arguments)]
fn trace_path_host<B: crate::rt::bvh::BlasSet + ?Sized>(
    tlas: &Tlas,
    blases: &B,
    scene: &PtScene,
    cfg: &PtConfig,
    stream: &[f32],
    pixel: usize,
    sample: usize,
) -> [f32; 3] {
    let cam = &scene.camera;
    let px = pixel % cam.width as usize;
    let py = pixel / cam.width as usize;
    let sb = rng::sample_base(pixel, sample, cfg.spp, cfg.max_bounces);
    let inv_w = 1.0 / cam.width as f32;
    let inv_h = 1.0 / cam.height as f32;
    let ju = (px as f32 + stream[sb]) * inv_w;
    let jv = (py as f32 + stream[sb + 1]) * inv_h;
    let sx = (2.0 * ju - 1.0) * cam.tan_half_fov;
    let sy = (1.0 - 2.0 * jv) * cam.tan_half_fov;
    let f = Vec3::from_array(cam.forward);
    let r = Vec3::from_array(cam.right);
    let u = Vec3::from_array(cam.up);
    let dir = (f + r * sx + u * sy).normalize();
    let mut origin = Vec3::from_array(cam.origin);
    let mut d = dir;
    let mut thr = Vec3::new(1.0, 1.0, 1.0);
    let mut li = Vec3::new(0.0, 0.0, 0.0);
    let mut prev_pdf = 1.0f32;
    let mut first = true;
    let ln = Vec3::from_array(scene.light.normal());
    let le = Vec3::from_array(scene.light.emission);
    let area = scene.light.area();
    let lp00 = Vec3::from_array(scene.light.p00);
    let le1 = Vec3::from_array(scene.light.e1);
    let le2 = Vec3::from_array(scene.light.e2);
    for b in 0..cfg.max_bounces as usize {
        let bb = rng::bounce_base(sb, b);
        let hit = tlas.intersect(blases, &Ray { origin, dir: d });
        let Some(hit) = hit else {
            break; // miss:吸收零态(thr 归零即路径终结;流索引无关)
        };
        let prim = hit.tri as usize;
        let ng = Vec3::from_array(hit.normal);
        let p = origin + d * hit.t;
        // 着色法线面向入射光线(双面 Lambert)。
        let n = if ng.dot(d) > 0.0 { ng * (-1.0) } else { ng };
        let (albedo, emission) = match &scene.materials[prim] {
            MaterialKind::Lambert { albedo } => (*albedo, [0.0; 3]),
            MaterialKind::Emission { albedo, emission } => (*albedo, *emission),
            _ => ([0.0; 3], [0.0; 3]), // validate 先行;oracle 遇范围外不产路径
        };
        let al = Vec3::from_array(albedo);
        let em = Vec3::from_array(emission);
        // ① BSDF 命中发光面(单面 + MIS w_b)。
        let cos_emit = -ng.dot(d);
        if emission.iter().any(|c| *c > 0.0) && cos_emit > 0.0 {
            let w_b = if first {
                1.0
            } else if cfg.switches.mis {
                mis_weight_bsdf(hit.t, area, cos_emit, prev_pdf)
            } else {
                1.0
            };
            li = li + cmul(thr, em) * w_b;
        }
        // ② NEE(光源 quad 均匀采样 + 阴影光线 + MIS w_l)。
        let q = lp00 + le1 * stream[bb] + le2 * stream[bb + 1];
        let wv = q - p;
        let dist2 = wv.dot(wv).max(1e-12);
        let dist = dist2.sqrt();
        let wi = wv * (1.0 / dist);
        let cos_s = n.dot(wi).max(0.0);
        let cos_l = (-ln.dot(wi)).max(0.0);
        if cos_s > 0.0 && cos_l > 0.0 {
            // 贡献 = thr·(albedo/π)·cos_s·Le·cos_l·area/(π·dist²)(pdf_l 折合形)。
            let nee_core = cos_s * cos_l * area / (PT_PI * dist2);
            let w_l = if cfg.switches.mis {
                mis_weight_light(cos_s / PT_PI, area, cos_l, dist2)
            } else {
                1.0
            };
            let shadow_origin = p + n * RAY_EPS;
            let t_sh = (dist - 2.0 * RAY_EPS).max(RAY_EPS);
            let blocked = tlas.any_hit(
                blases,
                &Ray {
                    origin: shadow_origin,
                    dir: wi,
                },
                t_sh,
            );
            if !blocked {
                li = li + cmul(cmul(thr, al), le) * (nee_core * w_l);
            }
        }
        // ③ BSDF 采样(余弦加权半球;ref_tracer::cosine_sample_hemisphere 同式)。
        let nd = crate::rt::ref_tracer::cosine_sample_hemisphere(n, stream[bb + 2], stream[bb + 3]);
        prev_pdf = cosine_hemisphere_pdf(nd.dot(n));
        thr = cmul(thr, al); // (albedo/π)·cos/(cos/π) = albedo
        // ④ RR(p = max 通道 clamp [0,1];b ≥ rr_min 启用)。
        if cfg.switches.rr && b as u32 >= cfg.rr_min_bounce {
            let p_surv = thr.x.max(thr.y).max(thr.z).clamp(0.0, 1.0);
            if stream[bb + 4] > p_surv {
                break; // 轮盘终止
            }
            thr = thr * (1.0 / p_surv.max(1e-6));
        }
        origin = p + n * RAY_EPS;
        d = nd;
        first = false;
    }
    [li.x, li.y, li.z]
}

/// host oracle 全图渲染(逐像素顺序累加;确定性 = 流 + 图序 + f32 逐式)。
pub fn trace_host(scene: &PtScene, cfg: &PtConfig, stream: &[f32]) -> Result<PtImage, PtError> {
    scene.validate()?;
    cfg.validate()?;
    let pixel_count = (scene.camera.width * scene.camera.height) as usize;
    let need = rng::stream_len(pixel_count, cfg.spp, cfg.max_bounces);
    if stream.len() != need {
        return Err(PtError::InvalidConfig(format!(
            "RNG 流长 {} ≠ 期望 {need}(pixel={pixel_count} spp={} bounces={})",
            stream.len(),
            cfg.spp,
            cfg.max_bounces
        )));
    }
    let blases = vec![TriBvh::build(&scene.positions, &scene.indices)];
    let tlas = Tlas::build(
        &[InstanceDesc {
            blas: 0,
            transform: Transform3x4::IDENTITY,
            mask: 0xFF,
            flags: 0,
        }],
        &blases,
    );
    let mut rgb = vec![0.0f32; pixel_count * 3];
    let mut sum_lum = vec![0.0f32; pixel_count];
    let mut sumsq_lum = vec![0.0f32; pixel_count];
    let mut samples = vec![0u32; pixel_count];
    let bset: &[TriBvh] = &blases;
    for px in 0..pixel_count {
        let mut acc = [0.0f32; 3];
        for s in 0..cfg.spp as usize {
            let li = trace_path_host(&tlas, bset, scene, cfg, stream, px, s);
            acc[0] += li[0];
            acc[1] += li[1];
            acc[2] += li[2];
            let lum = (li[0] + li[1] + li[2]) / 3.0;
            sum_lum[px] += lum;
            sumsq_lum[px] += lum * lum;
            samples[px] += 1;
        }
        rgb[px * 3] = acc[0] / cfg.spp as f32;
        rgb[px * 3 + 1] = acc[1] / cfg.spp as f32;
        rgb[px * 3 + 2] = acc[2] / cfg.spp as f32;
    }
    Ok(PtImage {
        width: scene.camera.width,
        height: scene.camera.height,
        rgb,
        sum_lum,
        sumsq_lum,
        samples,
    })
}

/// 渲染输出 canonical digest(SHA-256(out_rgb ‖ Σ/Σ² 统计 ‖ sample count 字节,
/// 依序拼接);不含路径/mtime/seed——RXS-0357 L2 协议面)。
pub fn image_digest(img: &PtImage) -> [u8; 32] {
    let mut pre = Vec::with_capacity(img.rgb.len() * 4 + img.sum_lum.len() * 8 + img.samples.len() * 4);
    for v in img.rgb.iter().chain(img.sum_lum.iter()).chain(img.sumsq_lum.iter()) {
        pre.extend_from_slice(&v.to_le_bytes());
    }
    for v in &img.samples {
        pre.extend_from_slice(&v.to_le_bytes());
    }
    rurix_pkg::sha256::digest(&pre)
}

// ---------------------------------------------------------------------------
// pbrt-v4 对照面(共享场景/材质输入;不共享光照算法)
// ---------------------------------------------------------------------------

/// 导出 pbrt-v4 场景文本(确定性字节:浮点最短 round-trip 格式,行序冻结)。
///
/// `spp` → `Sampler "independent" "integer pixelsamples"`;`seed` → sampler seed;
/// `out_name` = Film 输出 basename(EXR;harness 以 cwd 控制落点)。
/// 映射面:发光材质 → `AreaLightSource "diffuse"` + 默认 matte 基底(albedo 0.5);
/// Lambert → `Material "diffuse" "rgb reflectance"`;`Integrator "path"
/// "integer maxdepth" [max_bounces]`;相机 → `LookAt` + `perspective` fov 垂直全角
/// (方形画幅 ⇒ 水平=垂直,与 host [`PtCamera`] 同式)。
pub fn pbrt_scene_text(scene: &PtScene, cfg: &PtConfig, seed: u64, out_name: &str) -> String {
    let cam = &scene.camera;
    let eye = cam.origin;
    // at = origin + forward(单位长)。
    let at = [
        eye[0] + cam.forward[0],
        eye[1] + cam.forward[1],
        eye[2] + cam.forward[2],
    ];
    // 垂直全角(度)= 2·atan(tan_half)。
    let fov_deg = cam.tan_half_fov.atan().to_degrees() * 2.0;
    let mut s = String::new();
    s.push_str(&format!(
        "# G9.4 M96 pbrt-v4 对照场景(由 rurix gi::path_trace fixture 同源导出;RXS-0357 L3)\n\
         # scene={} spp={} seed={} maxdepth={}\n",
        scene.name, cfg.spp, seed, cfg.max_bounces
    ));
    s.push_str(&format!(
        "Film \"rgb\" \"integer xresolution\" [{}] \"integer yresolution\" [{}] \"string filename\" [\"{}\"]\n",
        cam.width, cam.height, out_name
    ));
    s.push_str(&format!(
        "Sampler \"independent\" \"integer pixelsamples\" [{}] \"integer seed\" [{}]\n",
        cfg.spp, seed
    ));
    s.push_str(&format!(
        "Integrator \"path\" \"integer maxdepth\" [{}]\n",
        cfg.max_bounces
    ));
    s.push_str(&format!(
        "LookAt {} {} {}  {} {} {}  0 1 0\n",
        eye[0], eye[1], eye[2], at[0], at[1], at[2]
    ));
    s.push_str(&format!("Camera \"perspective\" \"float fov\" [{}]\n", fov_deg));
    s.push_str("WorldBegin\n");
    // 逐材质分组导出(同材质连续三角并入一个 trianglemesh)。
    let mut run: Vec<usize> = Vec::new(); // 当前组的三角下标
    let mut run_key: Option<MaterialKind> = None;
    let flush = |run: &mut Vec<usize>, key: Option<MaterialKind>, s: &mut String| {
        let Some(key) = key else { return };
        if run.is_empty() {
            return;
        }
        s.push_str("AttributeBegin\n");
        match key {
            MaterialKind::Lambert { albedo } => {
                s.push_str(&format!(
                    "  Material \"diffuse\" \"rgb reflectance\" [{} {} {}]\n",
                    albedo[0], albedo[1], albedo[2]
                ));
            }
            MaterialKind::Emission { albedo, emission } => {
                s.push_str(&format!(
                    "  AreaLightSource \"diffuse\" \"rgb L\" [{} {} {}]\n",
                    emission[0], emission[1], emission[2]
                ));
                s.push_str(&format!(
                    "  Material \"diffuse\" \"rgb reflectance\" [{} {} {}]\n",
                    albedo[0], albedo[1], albedo[2]
                ));
            }
            _ => unreachable!("validate 已拒范围外材质"),
        }
        let mut idx_text = String::new();
        let mut p_text = String::new();
        let mut vbase = 0u32;
        let mut vmap: std::collections::BTreeMap<u32, u32> = Default::default();
        for &t in run.iter() {
            for &vi in &scene.indices[t] {
                let next = *vmap.entry(vi).or_insert_with(|| {
                    let p = scene.positions[vi as usize];
                    p_text.push_str(&format!("{} {} {} ", p[0], p[1], p[2]));
                    let id = vbase;
                    vbase += 1;
                    id
                });
                idx_text.push_str(&format!("{} ", next));
            }
        }
        s.push_str(&format!(
            "  Shape \"trianglemesh\" \"integer indices\" [{}] \"point3 P\" [{}]\n",
            idx_text.trim_end(),
            p_text.trim_end()
        ));
        s.push_str("AttributeEnd\n");
        run.clear();
    };
    for (t, m) in scene.materials.iter().enumerate() {
        if Some(*m) != run_key {
            flush(&mut run, run_key, &mut s);
            run_key = Some(*m);
        }
        run.push(t);
    }
    flush(&mut run, run_key, &mut s);
    // pbrt-v4 无 WorldEnd 指令(正常解析路径对 WorldEnd 报 syntaxError;
    // 文件结束即世界结束,parser.cpp:975-982)。
    s
}

/// pbrt 场景文件名(冻结命名:`<scene>_spp<N>.pbrt`;参照档 `ref` 后缀)。
pub fn pbrt_scene_filename(scene: &str, spp: u32) -> String {
    if spp == M96_PBRT_REF_SPP {
        format!("{scene}_ref{M96_PBRT_REF_SPP}.pbrt")
    } else {
        format!("{scene}_spp{spp}.pbrt")
    }
}

/// PFM 回读(little-endian RGB float32;header `PF\n<w> <h>\n<-scale>\n`,
/// 行序 = **自底向上**(PFM 约定)——本函数按 PFM 约定翻转为行序自顶向下,
/// 与 rurix 像素序(行主、行 0 = 顶)对齐)。
pub fn read_pfm(bytes: &[u8]) -> Result<(u32, u32, Vec<f32>), PtError> {
    let err = |m: &str| PtError::PbrtBridge(format!("PFM 解析: {m}"));
    let text_end = |from: usize| -> Result<usize, PtError> {
        bytes[from..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| from + p)
            .ok_or_else(|| err("header 截断"))
    };
    let l0 = text_end(0)?;
    if &bytes[..l0] != b"PF" {
        return Err(err("魔数非 PF(非 RGB float PFM)"));
    }
    let l1 = text_end(l0 + 1)?;
    let dims = std::str::from_utf8(&bytes[l0 + 1..l1]).map_err(|_| err("尺寸行非 UTF-8"))?;
    let mut it = dims.split_whitespace();
    let w: u32 = it
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| err("宽缺失"))?;
    let h: u32 = it
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| err("高缺失"))?;
    let l2 = text_end(l1 + 1)?;
    let scale: f32 = std::str::from_utf8(&bytes[l1 + 1..l2])
        .map_err(|_| err("scale 行非 UTF-8"))?
        .trim()
        .parse()
        .map_err(|_| err("scale 非数值"))?;
    if scale >= 0.0 {
        return Err(err("scale ≥ 0(期望 little-endian 负 scale)"));
    }
    let data = &bytes[l2 + 1..];
    let n = (w * h) as usize;
    if data.len() < n * 12 {
        return Err(err(&format!("数据截断:{} < {}", data.len(), n * 12)));
    }
    let mut out = vec![0.0f32; n * 3];
    for y in 0..h as usize {
        let src_row = h as usize - 1 - y; // PFM 自底向上 → 翻转为自顶向下
        for x in 0..w as usize {
            let src = (src_row * w as usize + x) * 3;
            let dst = (y * w as usize + x) * 3;
            for c in 0..3 {
                let b = &data[(src + c) * 4..(src + c) * 4 + 4];
                out[dst + c] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            }
        }
    }
    Ok((w, h, out))
}

/// PFM 写出(自底向上行序,PFM 约定;rurix 像素序自顶向下 → 写时翻转)。
/// 调试/对照落盘用(harness `--emit-host-oracle-pfm`)。
pub fn write_pfm(img: &PtImage) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"PF\n");
    out.extend_from_slice(format!("{} {}\n", img.width, img.height).as_bytes());
    out.extend_from_slice(b"-1.0\n"); // little-endian
    let w = img.width as usize;
    let h = img.height as usize;
    for y in (0..h).rev() {
        for x in 0..w {
            let i = (y * w + x) * 3;
            for c in 0..3 {
                out.extend_from_slice(&img.rgb[i + c].to_le_bytes());
            }
        }
    }
    out
}

/// 相对偏差(收敛带度量;冻结公式):
/// `mean_{px,ch} |A−B| / (mean_{px,ch} B + 1e-4)`(B = pbrt 侧为分母基准)。
pub fn rel_dev(a: &[f32], b: &[f32]) -> Result<f64, PtError> {
    if a.len() != b.len() || a.is_empty() {
        return Err(PtError::PbrtBridge(format!(
            "rel_dev 长度失配/空:{} vs {}",
            a.len(),
            b.len()
        )));
    }
    let mut num = 0.0f64;
    let mut den = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        if !x.is_finite() || !y.is_finite() {
            return Err(PtError::PbrtBridge("rel_dev 输入非有限".into()));
        }
        num += f64::from((x - y).abs());
        den += f64::from(*y);
    }
    Ok(num / (den + 1e-4))
}

/// 相对 MAE(收敛曲线度量;冻结公式):同 [`rel_dev`] 形,参照 = 收敛参照图。
pub fn rel_mae(img: &[f32], reference: &[f32]) -> Result<f64, PtError> {
    rel_dev(img, reference)
}

// ---------------------------------------------------------------------------
// 冻结容差带(measured 后冻结,P-09;fail-closed 比对器)
// ---------------------------------------------------------------------------

/// 容差带单条目(场景 × spp)。
#[derive(Debug, Clone, PartialEq)]
pub struct BandEntry {
    /// 场景名。
    pub scene: String,
    /// spp。
    pub spp: u32,
    /// 冻结 golden digest(device 正例臂输出 SHA-256 hex)。
    pub golden_digest: String,
    /// 冻结容差带(dev 上界 = measured × [`M96_BAND_MARGIN`];measured 实测,禁手写)。
    pub band_rel_dev: f64,
    /// 冻结时实测 rurix↔pbrt 相对偏差(provenance)。
    pub measured_rel_dev: f64,
    /// 冻结时 rurix 收敛曲线值(rel-MAE vs pbrt ref;provenance)。
    pub curve_rurix: f64,
    /// 冻结时 pbrt 收敛曲线值(rel-MAE vs pbrt ref;provenance)。
    pub curve_pbrt: f64,
}

/// 容差带冻结 margin 规则(带 = 实测偏差 × 2;规则冻结于代码,基值实测,
/// provenance 全字段留痕——禁手写掩盖,P-09)。
pub const M96_BAND_MARGIN: f64 = 2.0;

/// 冻结容差带(`milestones/g9/g9_m96_pbrt_tolerance_band.json` 的内存形)。
#[derive(Debug, Clone, PartialEq)]
pub struct ToleranceBand {
    /// provenance:冻结时刻 UTC。
    pub frozen_at_utc: String,
    /// provenance:device 名。
    pub device_name: String,
    /// provenance:pbrt 版本行(`pbrt --version` 首行)。
    pub pbrt_version: String,
    /// provenance:pbrt 源树 commit。
    pub pbrt_commit: String,
    /// provenance:pbrt 可执行文件 SHA-256 hex。
    pub pbrt_exe_sha256: String,
    /// 逐 (scene, spp) 条目。
    pub entries: Vec<BandEntry>,
}

impl ToleranceBand {
    /// 查条目(fail-closed:缺条目 = Err)。
    pub fn entry(&self, scene: &str, spp: u32) -> Result<&BandEntry, PtError> {
        self.entries
            .iter()
            .find(|e| e.scene == scene && e.spp == spp)
            .ok_or_else(|| {
                PtError::PbrtBridge(format!("容差带缺条目 scene={scene} spp={spp}"))
            })
    }

    /// 比对(fail-closed):偏差 ≤ 带 且 digest == golden;违例逐条列名。
    pub fn check(
        &self,
        scene: &str,
        spp: u32,
        rel_dev: f64,
        digest_hex: &str,
    ) -> Result<(), PtError> {
        let e = self.entry(scene, spp)?;
        if digest_hex != e.golden_digest {
            return Err(PtError::PbrtBridge(format!(
                "scene={scene} spp={spp} digest {digest_hex} ≠ golden {}",
                e.golden_digest
            )));
        }
        if rel_dev.is_nan() || rel_dev > e.band_rel_dev {
            return Err(PtError::PbrtBridge(format!(
                "scene={scene} spp={spp} rel_dev {rel_dev:.6e} 越带(上界 {:.6e})",
                e.band_rel_dev
            )));
        }
        Ok(())
    }

    /// 序列化(手工 JSON;字段序冻结,浮点 `{:e}` 确定性格式)。
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n  \"schema\": \"rurix.g9m96.pbrt_tolerance_band.v1\",\n");
        s.push_str(&format!("  \"frozen_at_utc\": \"{}\",\n", self.frozen_at_utc));
        s.push_str(&format!("  \"device_name\": \"{}\",\n", self.device_name));
        s.push_str(&format!("  \"pbrt_version\": \"{}\",\n", self.pbrt_version));
        s.push_str(&format!("  \"pbrt_commit\": \"{}\",\n", self.pbrt_commit));
        s.push_str(&format!(
            "  \"pbrt_exe_sha256\": \"{}\",\n",
            self.pbrt_exe_sha256
        ));
        s.push_str(&format!(
            "  \"freeze_rule\": \"band_rel_dev = measured_rel_dev * {:.1}(规则冻结于 gi::path_trace::M96_BAND_MARGIN;基值 = 冻结批实测,禁手写 P-09)\",\n",
            M96_BAND_MARGIN
        ));
        s.push_str(&format!(
            "  \"spp_sequence\": \"{}\",\n",
            M96_SPP_SEQUENCE
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
        s.push_str(&format!("  \"ref_spp\": \"{}\",\n", M96_PBRT_REF_SPP));
        s.push_str(&format!("  \"seed_device\": \"{}\",\n", M96_SEED));
        s.push_str(&format!("  \"seed_pbrt\": \"{}\",\n", M96_PBRT_SEED));
        s.push_str("  \"entries\": [\n");
        for (i, e) in self.entries.iter().enumerate() {
            s.push_str(&format!(
                "    {{\"scene\": \"{}\", \"spp\": \"{}\", \"golden_digest\": \"{}\", \"band_rel_dev\": \"{:e}\", \"measured_rel_dev\": \"{:e}\", \"curve_rurix\": \"{:e}\", \"curve_pbrt\": \"{:e}\"}}{}\n",
                e.scene,
                e.spp,
                e.golden_digest,
                e.band_rel_dev,
                e.measured_rel_dev,
                e.curve_rurix,
                e.curve_pbrt,
                if i + 1 == self.entries.len() { "" } else { "," }
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }

    /// 解析(fail-closed:schema 不符/键缺失/数值非法/条目重复一律 Err)。
    pub fn parse(text: &str) -> Result<ToleranceBand, PtError> {
        let err = |m: &str| PtError::PbrtBridge(format!("容差带解析: {m}"));
        if !text.contains("\"schema\": \"rurix.g9m96.pbrt_tolerance_band.v1\"") {
            return Err(err("schema 失配"));
        }
        let get_str = |key: &str| -> Result<String, PtError> {
            let needle = format!("\"{key}\": \"");
            let start = text
                .find(&needle)
                .ok_or_else(|| err(&format!("缺键 {key}")))?
                + needle.len();
            let end = text[start..]
                .find('"')
                .ok_or_else(|| err(&format!("键 {key} 值未闭合")))?
                + start;
            Ok(text[start..end].to_string())
        };
        let mut entries = Vec::new();
        let entries_sec = text
            .split("\"entries\": [")
            .nth(1)
            .ok_or_else(|| err("缺 entries 段"))?;
        for chunk in entries_sec.split('{').skip(1) {
            let body = chunk.split('}').next().ok_or_else(|| err("条目未闭合"))?;
            let field = |key: &str| -> Result<String, PtError> {
                let needle = format!("\"{key}\": \"");
                let start = body
                    .find(&needle)
                    .ok_or_else(|| err(&format!("条目缺键 {key}")))?
                    + needle.len();
                let end = body[start..]
                    .find('"')
                    .ok_or_else(|| err(&format!("条目键 {key} 值未闭合")))?
                    + start;
                Ok(body[start..end].to_string())
            };
            let scene = field("scene")?;
            let spp: u32 = field("spp")?
                .parse()
                .map_err(|_| err("spp 非数值"))?;
            let golden_digest = field("golden_digest")?;
            if golden_digest.len() != 64 || !golden_digest.chars().all(|c| c.is_ascii_hexdigit())
            {
                return Err(err("golden_digest 非 64-hex"));
            }
            let num = |key: &str| -> Result<f64, PtError> {
                field(key)?
                    .parse()
                    .map_err(|_| err(&format!("{key} 非数值")))
            };
            let band_rel_dev = num("band_rel_dev")?;
            let measured_rel_dev = num("measured_rel_dev")?;
            let curve_rurix = num("curve_rurix")?;
            let curve_pbrt = num("curve_pbrt")?;
            if !(band_rel_dev > 0.0 && band_rel_dev.is_finite()) {
                return Err(err("band_rel_dev 非正/非有限"));
            }
            if entries
                .iter()
                .any(|e: &BandEntry| e.scene == scene && e.spp == spp)
            {
                return Err(err(&format!("条目重复 scene={scene} spp={spp}")));
            }
            entries.push(BandEntry {
                scene,
                spp,
                golden_digest,
                band_rel_dev,
                measured_rel_dev,
                curve_rurix,
                curve_pbrt,
            });
        }
        if entries.is_empty() {
            return Err(err("entries 为空"));
        }
        Ok(ToleranceBand {
            frozen_at_utc: get_str("frozen_at_utc")?,
            device_name: get_str("device_name")?,
            pbrt_version: get_str("pbrt_version")?,
            pbrt_commit: get_str("pbrt_commit")?,
            pbrt_exe_sha256: get_str("pbrt_exe_sha256")?,
            entries,
        })
    }
}

// ---------------------------------------------------------------------------
// 单测(RXS-0357 锚定;host 面——RNG 确定性 / BSDF·MIS 数值锚 / 场景装载 /
// 容差带比对器 fail-closed / conformance 锚消费 / host oracle 收敛 sanity)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0357
    #[test]
    fn rng_stream_layout_and_determinism() {
        // 布局公式锚:stride = 2 + 5·bounces;总长 = px·spp·stride。
        assert_eq!(rng::sample_stride(4), 22);
        assert_eq!(rng::stream_len(8, 4, 4), 8 * 4 * 22);
        assert_eq!(rng::sample_base(3, 2, 4, 4), (3 * 4 + 2) * 22);
        assert_eq!(rng::bounce_base(100, 3), 100 + 2 + 15);
        // 确定性:同 seed 位级一致。
        let a = rng::generate_stream(4, 2, 4, M96_SEED);
        let b = rng::generate_stream(4, 2, 4, M96_SEED);
        assert_eq!(a, b, "同 seed 流位级一致");
        // 值域 [0,1)。
        assert!(a.iter().all(|v| (0.0..1.0).contains(v)));
        // 改 seed ⇒ 流可检测分叉(RED 臂①的流层锚)。
        let c = rng::generate_stream(4, 2, 4, M96_SEED + 1);
        assert_ne!(a, c, "改 seed 流必分叉");
        assert!(a.iter().zip(c.iter()).any(|(x, y)| x != y));
    }

    //@ spec: RXS-0357
    #[test]
    fn bsdf_mis_numeric_anchors() {
        // Lambert:bsdf·cos/pdf ≈ albedo(代数消去恒等;f32 舍入内,≤2e-7)。
        let a = 0.73f32;
        let cos = 0.42f32;
        let pdf = cosine_hemisphere_pdf(cos);
        assert!(
            (lambert_bsdf(a) * cos / pdf - a).abs() <= 2e-7,
            "恒等式 f·cos/pdf ≈ albedo"
        );
        // MIS 权数值锚(frozen f32 位级 golden;改公式即翻红):
        //   w_l(1,1,1,1) = 1/(1+1) = 0.5 精确。
        assert_eq!(mis_weight_light(1.0, 1.0, 1.0, 1.0).to_bits(), 0x3F00_0000);
        //   w_b(1,1,1,1) = 1/(1+1) = 0.5 精确。
        assert_eq!(mis_weight_bsdf(1.0, 1.0, 1.0, 1.0).to_bits(), 0x3F00_0000);
        //   倾向性:pdf_b 大 ⇒ w_l 小(光源采样策略权重随 BSDF pdf 升而降)。
        let w_small = mis_weight_light(0.01, 0.09, 0.8, 4.0);
        let w_big = mis_weight_light(2.0, 0.09, 0.8, 4.0);
        assert!(w_small > w_big, "w_l 随 pdf_b 单调降");
        //   w_b + w_l 一致性:同 pdf 下两权和 ≈ 1(power-2 heuristic 恒等:
        //   p²/(p²+q²) + q²/(p²+q²) = 1)。
        let pdf_l = 4.0 / (0.09 * 0.8);
        let pdf_b = 0.6f32;
        let wl = pdf_l * pdf_l / (pdf_l * pdf_l + pdf_b * pdf_b);
        let wb = mis_weight_bsdf((4.0f32).sqrt(), 0.09, 0.8, pdf_b);
        // w_b 以 t² 形写:1/(1+(t²/(a·c·pb))²);t²=4 ⇒ pdf_l 形 = t²/(a·c);等价核验。
        assert!(
            (wb - pdf_b * pdf_b / (pdf_b * pdf_b + pdf_l * pdf_l)).abs() < 1e-6,
            "w_b = pdf_b²/(pdf_b²+pdf_l²)"
        );
        assert!(((wl + wb) - 1.0).abs() < 1e-5, "power-2 权和 = 1");
        // pdf 锚:cosine_hemisphere_pdf(1) = 1/π(f32 位级)。
        assert_eq!(
            cosine_hemisphere_pdf(1.0).to_bits(),
            (1.0f32 / PT_PI).to_bits()
        );
    }

    //@ spec: RXS-0357
    #[test]
    fn scene_fixtures_validate_and_pack() {
        for scene in m96_scenes() {
            scene.validate().expect("冻结 fixture 必过校验");
            let mats = pack_mats(&scene);
            assert_eq!(mats.len(), scene.indices.len() * 8);
            let tris = pack_tris(&scene);
            assert_eq!(tris.len(), scene.indices.len() * 9);
            let params = pack_params(&scene, &PtConfig::reference(16));
            assert_eq!(params.len(), 42);
            // 光源 quad 面积/法线锚。
            assert!(scene.light.area() > 0.0);
        }
        // Cornell 锚:24 三角(5 墙×2 + 盒 12 + 光 2),发光三角 = 末尾 2。
        let c = m96_cornell_scene();
        assert_eq!(c.indices.len(), 24);
        assert!(matches!(
            c.materials[c.indices.len() - 1],
            MaterialKind::Emission { .. }
        ));
    }

    //@ spec: RXS-0357
    #[test]
    fn out_of_scope_materials_fail_closed() {
        // 起步范围冻结:specular/透射/体积材质 typed Err 显式拒绝(逐类别)。
        for bad in [
            MaterialKind::Specular {
                reflectance: [1.0; 3],
            },
            MaterialKind::Transmission {
                transmittance: [1.0; 3],
            },
            MaterialKind::Volume { density: 1.0 },
        ] {
            let mut scene = m96_cornell_scene();
            scene.materials[0] = bad;
            let Err(e) = scene.validate() else {
                panic!("范围外材质未被拒绝:{bad:?}");
            };
            assert!(
                matches!(e, PtError::OutOfScopeMaterial { tri: 0, .. }),
                "typed Err 形态:{e:?}"
            );
        }
        // 光源 quad 与发光三角漂移 → InvalidScene(装载面 RED;发光三角 = 22/23)。
        let mut scene = m96_cornell_scene();
        scene.indices[23] = [scene.indices[23][0], scene.indices[23][2], scene.indices[23][1]];
        assert!(scene.validate().is_err(), "发光三角绕向漂移必拒");
    }

    //@ spec: RXS-0357
    #[test]
    fn host_oracle_deterministic_and_scope_sane() {
        let scene = m96_direct_light_scene();
        let cfg = PtConfig::reference(16);
        let stream = rng::generate_stream(
            (scene.camera.width * scene.camera.height) as usize,
            cfg.spp,
            cfg.max_bounces,
            cfg.seed,
        );
        let a = trace_host(&scene, &cfg, &stream).expect("oracle 渲染");
        let b = trace_host(&scene, &cfg, &stream).expect("oracle 渲染");
        assert_eq!(a, b, "host oracle 同 seed 双跑位级一致");
        assert_eq!(a.samples, vec![16u32; a.pixel_count()], "逐像素 sample count = spp");
        // 直接光场景:全图均值亮度显著为正且有限(光源可见/地板受光)。
        let lum = a.mean_luminance();
        assert!(lum > 0.01 && lum.is_finite(), "直接光场景亮度 {lum}");
        // 方差非负。
        assert!(a.mean_pixel_variance() >= 0.0);
    }

    //@ spec: RXS-0357
    #[test]
    fn host_oracle_red_arms_detectable() {
        // 三臂 RED 的 host 预锚(改 seed / 跳 RR / 关 MIS 各臂输出必分叉)。
        let scene = m96_cornell_scene();
        let cfg = PtConfig::reference(16);
        let px = (scene.camera.width * scene.camera.height) as usize;
        let golden = trace_host(
            &scene,
            &cfg,
            &rng::generate_stream(px, cfg.spp, cfg.max_bounces, cfg.seed),
        )
        .expect("golden");
        let golden_d = image_digest(&golden);
        // 臂①改 seed。
        let alt = trace_host(
            &scene,
            &cfg,
            &rng::generate_stream(px, cfg.spp, cfg.max_bounces, cfg.seed ^ 0xABCD),
        )
        .expect("seed 臂");
        assert_ne!(golden_d, image_digest(&alt), "改 seed 臂必分叉(RED)");
        // 臂②跳 RR。
        let mut no_rr = cfg;
        no_rr.switches.rr = false;
        let alt = trace_host(
            &scene,
            &no_rr,
            &rng::generate_stream(px, cfg.spp, cfg.max_bounces, cfg.seed),
        )
        .expect("跳 RR 臂");
        assert_ne!(golden_d, image_digest(&alt), "跳 RR 臂必分叉(RED)");
        // 臂③关 MIS。
        let mut no_mis = cfg;
        no_mis.switches.mis = false;
        let alt = trace_host(
            &scene,
            &no_mis,
            &rng::generate_stream(px, cfg.spp, cfg.max_bounces, cfg.seed),
        )
        .expect("关 MIS 臂");
        assert_ne!(golden_d, image_digest(&alt), "关 MIS 臂必分叉(RED)");
    }

    //@ spec: RXS-0357
    #[test]
    fn pbrt_scene_text_deterministic_and_contains_frozen_fields() {
        let scene = m96_cornell_scene();
        let cfg = PtConfig::reference(16);
        let a = pbrt_scene_text(&scene, &cfg, M96_PBRT_SEED, "m96_cornell_spp16.exr");
        let b = pbrt_scene_text(&scene, &cfg, M96_PBRT_SEED, "m96_cornell_spp16.exr");
        assert_eq!(a, b, "pbrt 场景导出确定性");
        assert!(a.contains("Sampler \"independent\" \"integer pixelsamples\" [16]"));
        assert!(a.contains("Integrator \"path\" \"integer maxdepth\" [4]"));
        assert!(a.contains("AreaLightSource \"diffuse\" \"rgb L\" [12 12 12]"));
        assert!(a.contains("LookAt 0.5 0.5 -0.9"));
        assert!(a.contains("WorldBegin\n"));
        assert!(!a.contains("WorldEnd"), "pbrt-v4 无 WorldEnd 指令");
    }

    //@ spec: RXS-0357
    #[test]
    fn tolerance_band_comparator_fail_closed() {
        let entry = BandEntry {
            scene: "m96_cornell".into(),
            spp: 16,
            golden_digest: "ab".repeat(32),
            band_rel_dev: 0.05,
            measured_rel_dev: 0.025,
            curve_rurix: 0.03,
            curve_pbrt: 0.028,
        };
        let band = ToleranceBand {
            frozen_at_utc: "2026-08-12T00:00:00Z".into(),
            device_name: "test".into(),
            pbrt_version: "pbrt-v4 test".into(),
            pbrt_commit: "deadbeef".into(),
            pbrt_exe_sha256: "ff".repeat(32),
            entries: vec![entry],
        };
        // 序列化 ⇄ 解析 roundtrip。
        let text = band.to_json();
        let back = ToleranceBand::parse(&text).expect("roundtrip");
        assert_eq!(band, back);
        // 正例:带内 + digest 全等 ⇒ Ok。
        back.check("m96_cornell", 16, 0.049, &"ab".repeat(32)).expect("带内放行");
        // RED:digest 分叉 ⇒ 拒。
        assert!(back.check("m96_cornell", 16, 0.049, &"cd".repeat(32)).is_err());
        // RED:越带 ⇒ 拒。
        assert!(back.check("m96_cornell", 16, 0.051, &"ab".repeat(32)).is_err());
        // RED:缺条目 ⇒ 拒(fail-closed 不静默放行)。
        assert!(back.check("m96_cornell", 32, 0.001, &"ab".repeat(32)).is_err());
        // RED:坏 schema ⇒ 拒。
        assert!(ToleranceBand::parse("{\"schema\": \"bogus\"}").is_err());
        // RED:条目缺键 ⇒ 拒。
        let broken = text.replace("golden_digest", "gd");
        assert!(ToleranceBand::parse(&broken).is_err());
        // RED:带非正 ⇒ 拒(`{:e}` 格式 0.05 → "5e-2")。
        let neg = text.replace("\"5e-2\"", "\"-5e-2\"");
        assert!(neg != text, "替换须命中");
        assert!(ToleranceBand::parse(&neg).is_err());
    }

    //@ spec: RXS-0357
    #[test]
    fn conformance_anchor_corpus_present() {
        // 消费锚定义务:G9.4 锚定语料在位且锚定本条款。
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/gi");
        let accept = root.join("accept/pt_reference_fixed_seed_minimal.rx");
        let reject = root.join("reject/pt_seed_changed_nondeterministic.rx");
        for f in [&accept, &reject] {
            let text = std::fs::read_to_string(f).expect("锚定语料在位");
            assert!(
                text.contains("//@ spec: RXS-0357"),
                "{} 缺 RXS-0357 锚",
                f.display()
            );
        }
        // reject 语料负例面 = 改 seed 不红即漏检(转正路径注释在位)。
        let rej = std::fs::read_to_string(&reject).expect("reject 语料");
        assert!(rej.contains("负例臂"), "reject 语料负例面注释在位");
    }

    //@ spec: RXS-0357
    #[test]
    fn pfm_roundtrip_and_orientation() {
        // PFM 读面:构造 2×2 已知图(自底向上写),读回须翻转为自顶向下。
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PF\n2 2\n-1.0\n");
        // PFM 行序自底向上:先写底行(row1 数据),再写顶行(row0 数据)。
        let rows_bottom_up: [[f32; 6]; 2] = [
            [10.0, 0.0, 0.0, 11.0, 0.0, 0.0], // 底行(显示 row1)
            [20.0, 0.0, 0.0, 21.0, 0.0, 0.0], // 顶行(显示 row0)
        ];
        for row in rows_bottom_up {
            for v in row {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        let (rw, rh, img) = read_pfm(&bytes).expect("PFM 解析");
        assert_eq!((rw, rh), (2, 2));
        // 自顶向下序:row0 应为 20/21。
        assert_eq!(img[0], 20.0);
        assert_eq!(img[3], 21.0);
        assert_eq!(img[6], 10.0);
        assert_eq!(img[9], 11.0);
        // fail-closed:坏魔数/截断必 Err。
        assert!(read_pfm(b"PX\n2 2\n-1.0\n").is_err());
        assert!(read_pfm(b"PF\n2 2\n-1.0\nabcd").is_err());
    }
}
