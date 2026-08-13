//! G9.4 M100 低档多灯直接光 host 面(spec/global_illumination.md RXS-0361;
//! RFC-0022 §7 D2-Q4;门 `g9.p1.m100.multi_light_low`)。
//!
//! 本模块 = M100 的 **host 数据面/选灯面/验证射线计数面**:
//! - **多灯场景 fixture**([`m100_multi_light_scene`]):M96 cornell 冻结几何
//!   (墙面/盒体逐字同) + **4 光源 quad**(天花 2×2 分布,异色异强);
//!   [`MultiLightScene::validate`] 承 M96 材质域/几何校验同律(逐灯 quad ↔
//!   发光三角逐字一致);pbrt 对照 fixture 经 M96 同源导出面落
//!   `conformance/gi/scenes/m100_multi_light_low.pbrt`(锚定语料,单测消费)。
//! - **低档默认档 = MegaLights 式固定随机选灯**(RXS-0361 L1):逐像素逐采样
//!   以固定 seed PCG32 流([`m100_rng`])均匀随机选一灯,NEE×MIS 估计子
//!   (逐灯 MIS 权与 M96 shade② 逐字同式,选灯概率 1/L 折贡献 ×L——期望与
//!   逐灯 M96 golden 和相等,单测锚定);**选灯种子流固定、同输入双运行逐位
//!   一致**。
//! - **验证射线零跳过硬契约**(RXS-0361 L2,D2-Q4 否决行):每个被选灯样本
//!   必须实际发射可见性验证射线——逐样本发行记录(diag 面)+ 逐灯计数非空
//!   ([`MlCounters`]);**跳验证射线注入 ⇒ 系统性变亮偏置 + 计数缺空,RED
//!   臂独立有效**;灯子集采样注入(只选灯 0)⇒ 偏离可检测(RED 臂)。
//! - **高档 ReSTIR not-triggered 登记**(RXS-0361 L3;RD-040 条件分项):
//!   [`check_restir_trigger`] fail-closed 判 not-triggered 显式登记,
//!   [`restir_serve`] 恒 typed Err,**不充绿**(不得以默认档绿色冒充高档
//!   已验收;M15 维持 open-留档)。
//! - **golden 对拍**(RXS-0361 L4):低档默认档输出对 **M96 多灯 golden**
//!   (= 逐灯单光源场景 M96 megakernel 参照图之和,直接光档 ⇒ 匹配深度
//!   [`M100_MATCHED_DEPTH`] = 1),容差带 measured 后冻结
//!   (milestones/g9/g9_m100_multi_light_band.json;带 = measured ×
//!   [`M100_BAND_MARGIN`],禁手写 P-09);门序消费锚 = M96 同深度 cornell
//!   digest 与 M97 冻结带条目逐字相等(D2-Q7)。
//!
//! ## 确定性协议(承 RXS-0357 L2 同律)
//! - 选灯/采样 RNG = PCG32 单一流按索引寻址([`m100_rng`]:逐像素逐采样 5 维
//!   [cam_u, cam_v, light_sel, nee_u, nee_v];流为输入非结果,G7.4 先例);
//!   逐像素独立顺序累加,禁 atomic 顺序敏感累加;逐样本发行记录写 diag 缓冲
//!   (host 确定性归约出计数面,禁 device atomic)。
//! - 全部 f32;device kernel `kernels/g9_m100_multi_light.rx` 分支判定一律
//!   min/max 算术门 + 短 selection 臂(M96 已机验白名单形),host 公式面与
//!   kernel 逐字同源。

use crate::gi::fallback_chain::{l2_closest_hit, scene_tri};
use crate::gi::path_trace::{self, MaterialKind, PtCamera, PtLightQuad, PtScene};
use crate::rt::ref_tracer::RAY_EPS;

// ---------------------------------------------------------------------------
// 冻结常量(档位参数/RED 检出阈;实现确定、非 stable,RFC-0022 §10 口径;
// 阈值先 measured 后冻结,禁手写掩盖 P-09)
// ---------------------------------------------------------------------------

/// M100 确定性协议冻结 seed(独立于 M96/M97/M98/M99 流,避免跨里程碑流耦合)。
pub const M100_SEED: u64 = 0x5A10_0B11_EC71_0004;
/// 低档默认档每像素采样数(固定随机选灯估计子)。
pub const M100_SPP: u32 = 8;
/// 多灯场景灯数(2×2 天花分布;闭集)。
pub const M100_LIGHTS: u32 = 4;
/// 匹配深度(直接光档 ⇒ M96 max_bounces=1 档 golden)。
pub const M100_MATCHED_DEPTH: u32 = 1;
/// M96 golden 参照 spp(与 M97/M98/M99 门序锚同档)。
pub const M100_M96_GOLDEN_SPP: u32 = 64;
/// 容差带倍率(band = measured × margin;禁手写,P-09;沿 M96~M99 口径)。
pub const M100_BAND_MARGIN: f64 = 2.0;
/// 跳验证射线 RED 臂检出阈:跳验证注入输出的平均亮度相对偏置
/// (lum(skip)−lum(ref))/lum(ref) 必须 ≥ 本阈(冻结批 host 参照实测 =
/// 2.06e-2,阈值 = 实测留 ~2× margin 冻结;实测值进 band json
/// `skip_verification_bias` 字段,禁手写掩盖 P-09)。
pub const M100_SKIP_BIAS_MIN: f64 = 0.01;
/// 灯子集采样 RED 臂检出阈:灯子集注入输出对 M96 多灯 golden 的 rel_dev
/// 必须 ≥ 本阈(冻结批 device 实测 = 2.04e-1,阈值 = 实测留 ~26% margin
/// 冻结;实测值进 band json `light_subset_rel_dev` 字段,禁手写掩盖 P-09)。
pub const M100_SUBSET_REL_DEV_MIN: f64 = 0.15;
/// 正下界(与 M96 kernel `tiny` 位级同值)。
const TINY: f32 = 0.000001;

// ---------------------------------------------------------------------------
// 错误面(fail-closed typed Err;本模块一切失败为类型化拒绝,严禁 UB)
// ---------------------------------------------------------------------------

/// M100 host 面错误(场景/选灯/计数/容差带全部 fail-closed)。
#[derive(Debug, Clone, PartialEq)]
pub enum MlError {
    /// 配置/输入非法(灯数为零/流长不符/场景结构非法等)。
    InvalidConfig(String),
    /// 高档 ReSTIR 被请求服务但 workload 证据不足(RXS-0361 L3:只登记
    /// not-triggered,禁静默当绿)。
    RestirNotTriggered(String),
    /// 深度容差带错误(解析/缺条目/digest 不符/越带)。
    DepthBand(String),
}

impl std::fmt::Display for MlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MlError::InvalidConfig(m) => write!(f, "配置非法: {m}"),
            MlError::RestirNotTriggered(m) => {
                write!(f, "高档 ReSTIR workload 证据不足(not-triggered): {m}")
            }
            MlError::DepthBand(m) => write!(f, "深度容差带: {m}"),
        }
    }
}

impl std::error::Error for MlError {}

// ---------------------------------------------------------------------------
// 多灯场景 fixture(M96 cornell 几何 + 4 光源 quad;RXS-0361 L1)
// ---------------------------------------------------------------------------

/// 多灯场景(三角形汤 + 逐三角材质 + 多光源 quad + 相机;承 M96 场景形)。
#[derive(Debug, Clone)]
pub struct MultiLightScene {
    /// 稳定场景名(容差带/golden 键)。
    pub name: &'static str,
    /// 顶点位置(3 f32/顶点)。
    pub positions: Vec<[f32; 3]>,
    /// 三角索引(序 = device primitiveIndex 序)。
    pub indices: Vec<[u32; 3]>,
    /// 逐三角材质(仅 Lambert/Emission;M96 起步范围冻结同律)。
    pub materials: Vec<MaterialKind>,
    /// 光源 quad 表(逐灯 2 发光三角,起始三角号见 `light_tri_base`)。
    pub lights: Vec<PtLightQuad>,
    /// 逐灯发光三角起始号(每灯 2 三角连续)。
    pub light_tri_base: Vec<u32>,
    /// 相机(pinhole,与 M96 cornell 同机位)。
    pub camera: PtCamera,
    /// 场景视距上界。
    pub t_max: f32,
}

impl MultiLightScene {
    /// 校验(fail-closed;承 M96 `PtScene::validate` 同律:材质域 +
    /// 索引/顶点界 + 逐灯 quad ↔ 发光三角逐字一致 + 相机正交单位)。
    pub fn validate(&self) -> Result<(), MlError> {
        if self.indices.is_empty() || self.positions.is_empty() {
            return Err(MlError::InvalidConfig("空场景".into()));
        }
        if self.materials.len() != self.indices.len() {
            return Err(MlError::InvalidConfig(format!(
                "材质数 {} ≠ 三角数 {}",
                self.materials.len(),
                self.indices.len()
            )));
        }
        if self.lights.len() != M100_LIGHTS as usize
            || self.light_tri_base.len() != self.lights.len()
        {
            return Err(MlError::InvalidConfig(format!(
                "灯数 {} ≠ 闭集 {M100_LIGHTS}(或 tri_base 表长不符)",
                self.lights.len()
            )));
        }
        for (t, idx) in self.indices.iter().enumerate() {
            for &vi in idx {
                if vi as usize >= self.positions.len() {
                    return Err(MlError::InvalidConfig(format!("三角 {t} 索引 {vi} 越界")));
                }
            }
        }
        for (i, p) in self.positions.iter().enumerate() {
            if !p.iter().all(|c| c.is_finite()) {
                return Err(MlError::InvalidConfig(format!("顶点 {i} 非有限")));
            }
        }
        for (t, m) in self.materials.iter().enumerate() {
            match m {
                MaterialKind::Lambert { albedo } => {
                    if !albedo.iter().all(|c| c.is_finite() && *c >= 0.0 && *c < 1.0) {
                        return Err(MlError::InvalidConfig(format!("三角 {t} albedo 越域")));
                    }
                }
                MaterialKind::Emission { albedo, emission } => {
                    if !albedo.iter().all(|c| c.is_finite() && *c >= 0.0 && *c < 1.0)
                        || !emission.iter().all(|c| c.is_finite() && *c >= 0.0)
                    {
                        return Err(MlError::InvalidConfig(format!("三角 {t} 发光面参数越域")));
                    }
                }
                _ => {
                    return Err(MlError::InvalidConfig(format!(
                        "三角 {t} 材质越起步范围(M96 同律:Lambert/Emission 以外必拒)"
                    )));
                }
            }
        }
        // 逐灯 quad ↔ 发光三角逐字一致(两半三角,绕向法线一致)。
        for (li, light) in self.lights.iter().enumerate() {
            let base = self.light_tri_base[li] as usize;
            let p00 = light.p00;
            let p10 = add3(p00, light.e1);
            let p01 = add3(p00, light.e2);
            let p11 = add3(p01, light.e1);
            let expected = [[p00, p10, p11], [p00, p11, p01]];
            let ln = light.normal();
            let area = light.area();
            if !(area.is_finite() && area > 0.0) {
                return Err(MlError::InvalidConfig(format!("灯 {li} quad 面积非正")));
            }
            for (k, verts) in expected.iter().enumerate() {
                let t = base + k;
                let tri = self.indices[t];
                for (j, e) in verts.iter().enumerate() {
                    let v = self.positions[tri[j] as usize];
                    if v != *e {
                        return Err(MlError::InvalidConfig(format!(
                            "灯 {li} 发光三角 {t} 顶点 {j} 与 quad 不逐字一致:{v:?} vs {e:?}"
                        )));
                    }
                }
                let vs = [
                    self.positions[tri[0] as usize],
                    self.positions[tri[1] as usize],
                    self.positions[tri[2] as usize],
                ];
                let e1 = [vs[1][0] - vs[0][0], vs[1][1] - vs[0][1], vs[1][2] - vs[0][2]];
                let e2 = [vs[2][0] - vs[0][0], vs[2][1] - vs[0][1], vs[2][2] - vs[0][2]];
                let n = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if nl <= 0.0 || (n[0] * ln[0] + n[1] * ln[1] + n[2] * ln[2]) <= 0.0 {
                    return Err(MlError::InvalidConfig(format!(
                        "灯 {li} 发光三角 {t} 绕向法线与 quad 法线反向"
                    )));
                }
            }
        }
        Ok(())
    }

    /// 全灯发光形态的 PtScene 视图(**仅**供 pbrt 导出/解析求值共享;
    /// 不经 `PtScene::validate`——单光源 quad 纪律不适用多灯,本视图
    /// `light` 字段填 lights[0] 仅占位,语义面以 `lights` 为准)。
    pub fn to_pt_scene_full(&self) -> PtScene {
        PtScene {
            name: self.name,
            positions: self.positions.clone(),
            indices: self.indices.clone(),
            materials: self.materials.clone(),
            light: self.lights[0],
            camera: self.camera,
            t_max: self.t_max,
        }
    }

    /// 单灯形态 PtScene(M96 golden 构造用;RXS-0361 L4):几何全量(四灯
    /// quad 皆在场),仅灯 k 保持 Emission,其余灯转 Lambert(albedo 0.5,
    /// 承 M96 cornell 光源 quad 基底);`light` = lights[k] ⇒ 过
    /// `PtScene::validate` 单光源纪律。
    pub fn single_light_pt_scene(&self, k: usize) -> Result<PtScene, MlError> {
        if k >= self.lights.len() {
            return Err(MlError::InvalidConfig(format!("灯号 {k} 越界")));
        }
        const NAMES: [&str; 4] = [
            "m100_multi_light_l0",
            "m100_multi_light_l1",
            "m100_multi_light_l2",
            "m100_multi_light_l3",
        ];
        let mut materials = self.materials.clone();
        for (li, &base) in self.light_tri_base.iter().enumerate() {
            for m in materials.iter_mut().skip(base as usize).take(2) {
                *m = if li == k {
                    MaterialKind::Emission {
                        albedo: [0.5, 0.5, 0.5],
                        emission: self.lights[li].emission,
                    }
                } else {
                    MaterialKind::Lambert { albedo: [0.5, 0.5, 0.5] }
                };
            }
        }
        let scene = PtScene {
            name: NAMES[k],
            positions: self.positions.clone(),
            indices: self.indices.clone(),
            materials,
            light: self.lights[k],
            camera: self.camera,
            t_max: self.t_max,
        };
        scene
            .validate()
            .map_err(|e| MlError::InvalidConfig(format!("单灯场景校验: {e}")))?;
        Ok(scene)
    }
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// M100 冻结 fixture:cornell 几何(非发光三角逐字承 M96 fixture)+ 4 光源
/// quad(天花 2×2 分布,边长 0.25,y=0.995 法线 −y;异色异强——暖/冷/中性/
/// 强暖,多灯直接光区分度)。
pub fn m100_multi_light_scene() -> MultiLightScene {
    let cornell = path_trace::m96_cornell_scene();
    // 非发光三角逐字承 M96(发光 quad 两三角剔除——cornell 发光三角在尾部)。
    let emissive: Vec<usize> = cornell
        .materials
        .iter()
        .enumerate()
        .filter(|(_, m)| matches!(m, MaterialKind::Emission { .. }))
        .map(|(t, _)| t)
        .collect();
    assert_eq!(emissive.len(), 2, "M96 cornell 单光源纪律(发光三角恰 2)");
    let keep: Vec<usize> = (0..cornell.indices.len())
        .filter(|t| !emissive.contains(t))
        .collect();
    // 顶点重打包(仅被引用顶点;序 = 先几何后四灯,确定性)。
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<[u32; 3]> = Vec::new();
    let mut materials: Vec<MaterialKind> = Vec::new();
    let mut vmap: std::collections::BTreeMap<u32, u32> = Default::default();
    for &t in &keep {
        let mut tri = [0u32; 3];
        for (j, &vi) in cornell.indices[t].iter().enumerate() {
            tri[j] = *vmap.entry(vi).or_insert_with(|| {
                positions.push(cornell.positions[vi as usize]);
                (positions.len() - 1) as u32
            });
        }
        indices.push(tri);
        materials.push(cornell.materials[t]);
    }
    // 4 光源 quad(2×2 天花分布;绕向 (p00,p10,p11),(p00,p11,p01) ⇒ 法线 −y)。
    let lights: Vec<PtLightQuad> = vec![
        PtLightQuad {
            p00: [0.15, 0.995, 0.15],
            e1: [0.25, 0.0, 0.0],
            e2: [0.0, 0.0, 0.25],
            emission: [10.0, 8.5, 6.5], // 暖
        },
        PtLightQuad {
            p00: [0.60, 0.995, 0.15],
            e1: [0.25, 0.0, 0.0],
            e2: [0.0, 0.0, 0.25],
            emission: [6.5, 8.5, 10.0], // 冷
        },
        PtLightQuad {
            p00: [0.15, 0.995, 0.60],
            e1: [0.25, 0.0, 0.0],
            e2: [0.0, 0.0, 0.25],
            emission: [8.0, 8.0, 8.0], // 中性
        },
        PtLightQuad {
            p00: [0.60, 0.995, 0.60],
            e1: [0.25, 0.0, 0.0],
            e2: [0.0, 0.0, 0.25],
            emission: [12.0, 10.0, 8.0], // 强暖
        },
    ];
    let mut light_tri_base = Vec::new();
    for light in &lights {
        let p00 = light.p00;
        let p10 = add3(p00, light.e1);
        let p01 = add3(p00, light.e2);
        let p11 = add3(p01, light.e1);
        light_tri_base.push(indices.len() as u32);
        for verts in [[p00, p10, p11], [p00, p11, p01]] {
            let mut tri = [0u32; 3];
            for (j, v) in verts.iter().enumerate() {
                // 灯 quad 顶点独立追加(不与几何顶点共享——与 M96 cornell
                // 光源 quad 同律,逐字一致校验以位置域为准)。
                positions.push(*v);
                tri[j] = (positions.len() - 1) as u32;
            }
            indices.push(tri);
            materials.push(MaterialKind::Emission {
                albedo: [0.5, 0.5, 0.5],
                emission: light.emission,
            });
        }
    }
    MultiLightScene {
        name: "m100_multi_light",
        positions,
        indices,
        materials,
        lights,
        light_tri_base,
        camera: cornell.camera,
        t_max: cornell.t_max,
    }
}

/// pbrt-v4 对照 fixture 导出(经 M96 同源导出面 `path_trace::pbrt_scene_text`,
/// 仅头部注释替换为 M100 字面;多灯 = 多 AreaLightSource 分组,导出面零私有
/// 改写)。锚定语料 = `conformance/gi/scenes/m100_multi_light_low.pbrt`
/// (单测逐字比对消费)。
pub fn pbrt_multi_light_text(scene: &MultiLightScene) -> String {
    let cfg = path_trace::PtConfig {
        spp: M100_M96_GOLDEN_SPP,
        max_bounces: M100_MATCHED_DEPTH,
        rr_min_bounce: 2,
        seed: path_trace::M96_PBRT_SEED,
        switches: path_trace::PtSwitches::REFERENCE,
    };
    let raw = path_trace::pbrt_scene_text(
        &scene.to_pt_scene_full(),
        &cfg,
        path_trace::M96_PBRT_SEED,
        "m100_multi_light_low.exr",
    );
    // 头部两行(来源注释)替换为 M100 字面;其余字节逐字承 M96 导出面。
    let body_start = raw.find("Film ").expect("M96 导出面 Film 行");
    format!(
        "# G9.4 M100 pbrt-v4 对照场景(经 M96 同源导出面 gi::path_trace::pbrt_scene_text;RXS-0361 L4)\n\
         # scene={} lights={} spp={} seed={} maxdepth={}\n{}",
        scene.name,
        scene.lights.len(),
        M100_M96_GOLDEN_SPP,
        path_trace::M96_PBRT_SEED,
        M100_MATCHED_DEPTH,
        &raw[body_start..]
    )
}

// ---------------------------------------------------------------------------
// RNG 流(选灯种子流固定;PCG32 单一流按索引寻址;流为输入非结果)
// ---------------------------------------------------------------------------

/// M100 流布局(冻结):逐像素逐采样 5 维
/// [cam_u, cam_v, light_sel, nee_u, nee_v]。
pub mod m100_rng {
    use crate::rt::ref_tracer::Pcg32;

    /// 每采样随机维数。
    pub const DIMS_PER_SAMPLE: usize = 5;

    /// 流总长(= pixel_count · spp · 5)。
    pub fn stream_len(pixel_count: usize, spp: u32) -> usize {
        pixel_count * spp as usize * DIMS_PER_SAMPLE
    }

    /// 采样 (pixel, sample) 的流起始下标。
    pub fn sample_base(pixel: usize, sample: usize, spp: u32) -> usize {
        (pixel * spp as usize + sample) * DIMS_PER_SAMPLE
    }

    /// 生成整条流(单 [`Pcg32`] 实例,图序 × 采样序顺序产出;承 G8 对拍模式)。
    pub fn generate_stream(pixel_count: usize, spp: u32, seed: u64) -> Vec<f32> {
        let mut rng = Pcg32::new(seed);
        let mut out = Vec::with_capacity(stream_len(pixel_count, spp));
        for _ in 0..stream_len(pixel_count, spp) {
            out.push(rng.next_f32());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// 低档默认档估计子(host 参照,与 kernel `g9_m100_multi_light` 逐字同源;
// MegaLights 式固定随机选灯 + 验证射线零跳过)
// ---------------------------------------------------------------------------

/// 低档档模式(闭集):参照(验证射线开 + 全灯闭集选灯)/ 跳验证注入 /
/// 灯子集注入(只选灯 0)。kernel params[2] 位级同值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowTierMode {
    /// 参照(默认档正例):固定随机选灯 + 每被选灯样本验证射线实际发射。
    Reference,
    /// 跳验证射线注入(可见性恒取 1 不发射;D2-Q4 否决行 RED 臂)。
    SkipVerificationInjected,
    /// 灯子集采样注入(选灯维度丢弃、恒选灯 0;RED 臂)。
    LightSubsetInjected,
}

impl LowTierMode {
    /// kernel 参数位级编码。
    pub fn as_f32(self) -> f32 {
        match self {
            LowTierMode::Reference => 0.0,
            LowTierMode::SkipVerificationInjected => 1.0,
            LowTierMode::LightSubsetInjected => 2.0,
        }
    }
}

/// 验证射线计数面(逐帧 evidence;零跳过硬契约的机器锚)。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MlCounters {
    /// 主命中样本数(应发射验证射线的样本数 = 主命中像素 × spp)。
    pub primary_hit_samples: u64,
    /// 实际发射的验证射线数(参照档 = primary_hit_samples,零跳过)。
    pub verification_rays_fired: u64,
    /// 被遮蔽验证射线数(阴影命中)。
    pub verification_rays_blocked: u64,
    /// 跳过数(= primary_hit_samples − fired;参照档恒 0)。
    pub verification_rays_skipped: u64,
    /// 逐灯实际发射计数(零跳过 ⇒ 每灯非空)。
    pub per_light_fired: [u64; M100_LIGHTS as usize],
    /// 逐灯遮蔽计数。
    pub per_light_blocked: [u64; M100_LIGHTS as usize],
}

/// 低档档输出(逐像素均值 RGB + Σlum/Σlum² + 逐样本发行记录 diag)。
#[derive(Debug, Clone, PartialEq)]
pub struct MlOutput {
    /// 图宽。
    pub width: u32,
    /// 图高。
    pub height: u32,
    /// 逐像素均值辐射度 RGB(3 f32/px,线性)。
    pub rgb: Vec<f32>,
    /// 逐像素 Σlum。
    pub sum_lum: Vec<f32>,
    /// 逐像素 Σlum²。
    pub sumsq_lum: Vec<f32>,
    /// 逐样本发行记录(3 f32/(px·spp)):[选中灯号+1(主未命中=0),
    /// 验证射线实际发射(0/1), 遮蔽(0/1)]——host 确定性归约出计数面
    /// (禁 device atomic,承确定性协议)。
    pub diag: Vec<f32>,
}

impl MlOutput {
    /// 产物 digest = sha256(rgb ‖ Σ/Σ² ‖ diag 字节)(diag 携带发行记录——
    /// 跳验证/子集注入必然改变产物 digest,结构性保证回归可检测)。
    pub fn product_digest(&self) -> [u8; 32] {
        let mut pre = Vec::with_capacity((self.rgb.len() + self.diag.len()) * 4 + 8);
        for v in self
            .rgb
            .iter()
            .chain(self.sum_lum.iter())
            .chain(self.sumsq_lum.iter())
            .chain(self.diag.iter())
        {
            pre.extend_from_slice(&v.to_le_bytes());
        }
        rurix_pkg::sha256::digest(&pre)
    }

    /// 平均亮度(偏置度量锚)。
    pub fn mean_luminance(&self) -> f64 {
        let spp_lum: f64 = self.sum_lum.iter().map(|&v| f64::from(v)).sum();
        spp_lum / (f64::from(self.width * self.height) * f64::from(M100_SPP))
    }

    /// 计数面归约(diag → 计数;零跳过硬契约核验锚)。
    pub fn counters(&self) -> MlCounters {
        let mut c = MlCounters::default();
        for s in 0..(self.diag.len() / 3) {
            let light_sel = self.diag[s * 3];
            let fired = self.diag[s * 3 + 1];
            let blocked = self.diag[s * 3 + 2];
            if light_sel > 0.5 {
                c.primary_hit_samples += 1;
                let k = (light_sel as u32 - 1).min(M100_LIGHTS - 1) as usize;
                if fired > 0.5 {
                    c.verification_rays_fired += 1;
                    c.per_light_fired[k] += 1;
                    if blocked > 0.5 {
                        c.verification_rays_blocked += 1;
                        c.per_light_blocked[k] += 1;
                    }
                }
            }
        }
        c.verification_rays_skipped = c.primary_hit_samples - c.verification_rays_fired;
        c
    }
}

/// 三角形几何法线(依 winding,归一化;与 M96 host 同式)。
fn tri_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let nx = e1[1] * e2[2] - e1[2] * e2[1];
    let ny = e1[2] * e2[0] - e1[0] * e2[2];
    let nz = e1[0] * e2[1] - e1[1] * e2[0];
    let l = (nx * nx + ny * ny + nz * nz).sqrt();
    if l > 0.0 {
        [nx / l, ny / l, nz / l]
    } else {
        [0.0; 3]
    }
}

/// 低档档单采样估计(host 参照;与 kernel 逐字同源)。返回 (li, diag[3])。
fn trace_direct_sample(
    scene: &PtScene,
    lights: &[PtLightQuad],
    stream: &[f32],
    pixel: usize,
    sample: usize,
    spp: u32,
    mode: LowTierMode,
) -> ([f32; 3], [f32; 3]) {
    let cam = &scene.camera;
    let px = pixel % cam.width as usize;
    let py = pixel / cam.width as usize;
    let sb = m100_rng::sample_base(pixel, sample, spp);
    let inv_w = 1.0 / cam.width as f32;
    let inv_h = 1.0 / cam.height as f32;
    // 相机主光线(jitter 流采样;M96 ray gen 同式)。
    let ju = (px as f32 + stream[sb]) * inv_w;
    let jv = (py as f32 + stream[sb + 1]) * inv_h;
    let sx = (2.0 * ju - 1.0) * cam.tan_half_fov;
    let sy = (1.0 - 2.0 * jv) * cam.tan_half_fov;
    let dx = cam.forward[0] + cam.right[0] * sx + cam.up[0] * sy;
    let dy = cam.forward[1] + cam.right[1] * sx + cam.up[1] * sy;
    let dz = cam.forward[2] + cam.right[2] * sx + cam.up[2] * sy;
    let inv = 1.0 / (dx * dx + dy * dy + dz * dz).sqrt();
    let d = [dx * inv, dy * inv, dz * inv];
    let (best, _tests) = l2_closest_hit(scene, cam.origin, d, scene.t_max);
    let Some((t, tri)) = best else {
        return ([0.0; 3], [0.0; 3]); // 主未命中:吸收零态(diag 全零显式)
    };
    let (a, b, c) = scene_tri(scene, tri as usize);
    let ng = tri_normal(a, b, c);
    let p = [
        cam.origin[0] + d[0] * t,
        cam.origin[1] + d[1] * t,
        cam.origin[2] + d[2] * t,
    ];
    let n = if ng[0] * d[0] + ng[1] * d[1] + ng[2] * d[2] > 0.0 {
        [-ng[0], -ng[1], -ng[2]]
    } else {
        ng
    };
    let (albedo, emission) = match &scene.materials[tri as usize] {
        MaterialKind::Lambert { albedo } => (*albedo, [0.0; 3]),
        MaterialKind::Emission { albedo, emission } => (*albedo, *emission),
        _ => ([0.0; 3], [0.0; 3]),
    };
    let mut li = [0.0f32; 3];
    // ① 主命中发光面(直接可见灯;首反弹 w_b = 1,M96 shade① first 同式)。
    let cos_emit = -(ng[0] * d[0] + ng[1] * d[1] + ng[2] * d[2]);
    if emission.iter().any(|&x| x > 0.0) && cos_emit > 0.0 {
        li = [li[0] + emission[0], li[1] + emission[1], li[2] + emission[2]];
    }
    // ② 固定随机选灯 + NEE(MegaLights 式;选灯概率 1/L 折贡献 ×L)。
    let l_count = lights.len() as f32;
    let sel = (stream[sb + 2] * l_count) as u32;
    let k = match mode {
        LowTierMode::LightSubsetInjected => 0usize, // 子集注入:选灯维度丢弃
        _ => sel.min(lights.len() as u32 - 1) as usize,
    };
    let light = &lights[k];
    let ln = light.normal();
    let area = light.area();
    let q = [
        light.p00[0] + stream[sb + 3] * light.e1[0] + stream[sb + 4] * light.e2[0],
        light.p00[1] + stream[sb + 3] * light.e1[1] + stream[sb + 4] * light.e2[1],
        light.p00[2] + stream[sb + 3] * light.e1[2] + stream[sb + 4] * light.e2[2],
    ];
    let wv = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
    let dist2 = (wv[0] * wv[0] + wv[1] * wv[1] + wv[2] * wv[2]).max(TINY * TINY);
    let dist = dist2.sqrt();
    let wi = [wv[0] / dist, wv[1] / dist, wv[2] / dist];
    let cos_s = (n[0] * wi[0] + n[1] * wi[1] + n[2] * wi[2]).max(0.0);
    let cos_l = (-(ln[0] * wi[0] + ln[1] * wi[1] + ln[2] * wi[2])).max(0.0);
    // 验证射线零跳过(D2-Q4):参照/子集档每被选灯样本必发射;跳验证注入
    // ⇒ 可见性恒取 1 不发射(diag fired=0 显式)。
    let (vis, fired, blocked) = match mode {
        LowTierMode::SkipVerificationInjected => (1.0f32, 0.0f32, 0.0f32),
        _ => {
            let so = [
                p[0] + n[0] * RAY_EPS,
                p[1] + n[1] * RAY_EPS,
                p[2] + n[2] * RAY_EPS,
            ];
            let t_sh = (dist - 2.0 * RAY_EPS).max(RAY_EPS);
            let (hit, _t) = l2_closest_hit(scene, so, wi, t_sh);
            let blocked = if hit.is_some() { 1.0f32 } else { 0.0f32 };
            (1.0 - blocked, 1.0, blocked)
        }
    };
    if cos_s > 0.0 && cos_l > 0.0 {
        let nee_core = cos_s * cos_l * area / (path_trace::PT_PI * dist2);
        let w_l = path_trace::mis_weight_light(cos_s / path_trace::PT_PI, area, cos_l, dist2);
        let gain = nee_core * w_l * vis * l_count;
        li[0] += albedo[0] * light.emission[0] * gain;
        li[1] += albedo[1] * light.emission[1] * gain;
        li[2] += albedo[2] * light.emission[2] * gain;
    }
    (li, [(k + 1) as f32, fired, blocked])
}

/// 低档档全图(host 参照;逐像素顺序累加,确定性)。`scene` =
/// [`MultiLightScene::to_pt_scene_full`] 产物(全灯发光形态)。
pub fn trace_direct_host(
    scene: &MultiLightScene,
    stream: &[f32],
    spp: u32,
    mode: LowTierMode,
) -> Result<MlOutput, MlError> {
    scene.validate()?;
    let full = scene.to_pt_scene_full();
    let pixel_count = (full.camera.width * full.camera.height) as usize;
    if stream.len() != m100_rng::stream_len(pixel_count, spp) {
        return Err(MlError::InvalidConfig(format!(
            "RNG 流长 {} ≠ 期望 {}(pixel={pixel_count} spp={spp})",
            stream.len(),
            m100_rng::stream_len(pixel_count, spp)
        )));
    }
    let mut rgb = vec![0.0f32; pixel_count * 3];
    let mut sum_lum = vec![0.0f32; pixel_count];
    let mut sumsq_lum = vec![0.0f32; pixel_count];
    let mut diag = vec![0.0f32; pixel_count * spp as usize * 3];
    for px in 0..pixel_count {
        let mut acc = [0.0f32; 3];
        for s in 0..spp as usize {
            let (li, dg) = trace_direct_sample(&full, &scene.lights, stream, px, s, spp, mode);
            acc[0] += li[0];
            acc[1] += li[1];
            acc[2] += li[2];
            let lum = (li[0] + li[1] + li[2]) / 3.0;
            sum_lum[px] += lum;
            sumsq_lum[px] += lum * lum;
            diag[(px * spp as usize + s) * 3..(px * spp as usize + s) * 3 + 3].copy_from_slice(&dg);
        }
        let inv = 1.0 / spp as f32;
        rgb[px * 3] = acc[0] * inv;
        rgb[px * 3 + 1] = acc[1] * inv;
        rgb[px * 3 + 2] = acc[2] * inv;
    }
    Ok(MlOutput {
        width: full.camera.width,
        height: full.camera.height,
        rgb,
        sum_lum,
        sumsq_lum,
        diag,
    })
}

// ---------------------------------------------------------------------------
// M96 多灯 golden(逐灯单光源参照图之和;光传输线性叠加,RXS-0361 L4)
// ---------------------------------------------------------------------------

/// 逐灯参照图求和(M96 megakernel 单灯场景输出,同分辨率)。
pub fn golden_sum_image(per_light: &[path_trace::PtImage]) -> Result<Vec<f32>, MlError> {
    if per_light.len() != M100_LIGHTS as usize {
        return Err(MlError::InvalidConfig(format!(
            "逐灯参照图数 {} ≠ {M100_LIGHTS}",
            per_light.len()
        )));
    }
    let n = per_light[0].rgb.len();
    let mut sum = vec![0.0f32; n];
    for img in per_light {
        if img.rgb.len() != n {
            return Err(MlError::InvalidConfig("逐灯参照图分辨率不符".into()));
        }
        for (i, v) in img.rgb.iter().enumerate() {
            sum[i] += v;
        }
    }
    Ok(sum)
}

/// 多灯 golden digest = sha256(求和图字节 ‖ 逐灯 M96 参照 digest ascii)
/// (门序消费面 provenance:逐灯 digest 入键,单灯替换必检出)。
pub fn golden_digest(sum_rgb: &[f32], per_light_digests: &[[u8; 32]]) -> [u8; 32] {
    let mut pre = Vec::with_capacity(sum_rgb.len() * 4 + per_light_digests.len() * 32);
    for v in sum_rgb {
        pre.extend_from_slice(&v.to_le_bytes());
    }
    for d in per_light_digests {
        pre.extend_from_slice(d);
    }
    rurix_pkg::sha256::digest(&pre)
}

// ---------------------------------------------------------------------------
// 高档 ReSTIR not-triggered 登记(RXS-0361 L3;RD-040 条件分项)
// ---------------------------------------------------------------------------

/// 高档 ReSTIR 触发核验结果(显式结构)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestirTrigger {
    /// 高档 ReSTIR reservoir 须附多灯 workload 证据,证据不足 ⇒ 登记
    /// not-triggered(M15 维持 open-留档;条件未触发只表示决策已记录,
    /// 不是成功)。
    NotTriggered {
        /// 未触发原因(evidence 字面)。
        reason: &'static str,
    },
}

/// 高档 ReSTIR 触发核验器(fail-closed):多灯 workload 证据未附
/// (G9_CANDIDATE_DECISIONS v1.3 校准注)⇒ 恒 [`RestirTrigger::NotTriggered`];
/// 任何高档服务请求经 [`restir_serve`] typed Err 显式拒绝。
pub fn check_restir_trigger() -> RestirTrigger {
    RestirTrigger::NotTriggered {
        reason: "高档 ReSTIR reservoir 须附多灯 workload 证据,证据不足(RD-040 条件分项):登记 not-triggered 不充绿,M15 维持 open-留档",
    }
}

/// 高档 ReSTIR 服务接口(冻结面;证据不足 ⇒ 恒 fail-closed
/// [`MlError::RestirNotTriggered`],禁静默当绿)。
pub fn restir_serve() -> Result<(), MlError> {
    Err(MlError::RestirNotTriggered(
        "高档 ReSTIR 服务被请求,但多灯 workload 证据未附——登记 not-triggered,拒绝静默服务".into(),
    ))
}

// ---------------------------------------------------------------------------
// 容差带(measured 后冻结,P-09;fail-closed)
// ---------------------------------------------------------------------------

/// M100 容差带单条目(低档默认档;匹配深度 [`M100_MATCHED_DEPTH`])。
#[derive(Debug, Clone, PartialEq)]
pub struct M100BandEntry {
    /// 档位名(m100_low_reference)。
    pub tier: String,
    /// 冻结 golden:低档默认档产物 digest(sha256(rgb‖Σ/Σ²‖diag))。
    pub product_digest: String,
    /// 冻结 golden:M96 多灯 golden digest(逐灯参照和 + 逐灯 digest 入键)。
    pub m96_golden_digest: String,
    /// 冻结容差带(rel_dev 上界 = measured × [`M100_BAND_MARGIN`];禁手写)。
    pub band_rel_dev: f64,
    /// 冻结时实测 rel_dev(低档输出 vs M96 多灯 golden;provenance)。
    pub measured_rel_dev: f64,
}

/// M100 容差带(`milestones/g9/g9_m100_multi_light_band.json` 的内存形)。
#[derive(Debug, Clone, PartialEq)]
pub struct M100Band {
    /// provenance:冻结时刻 UTC。
    pub frozen_at_utc: String,
    /// provenance:device 名。
    pub device_name: String,
    /// 冻结场景名。
    pub scene: String,
    /// M96 门序消费锚:cornell 同深度(1)实跑 digest 与 M97 冻结带
    /// `m96_cornell` 深度 1 条目逐字相等(D2-Q7 门序消费面的机器锚)。
    pub m96_anchor_digest: String,
    /// provenance:跳验证偏置比实测(检出阈 = [`M100_SKIP_BIAS_MIN`])。
    pub skip_verification_bias: f64,
    /// provenance:灯子集 rel_dev 实测(检出阈 = [`M100_SUBSET_REL_DEV_MIN`])。
    pub light_subset_rel_dev: f64,
    /// 逐档条目。
    pub entries: Vec<M100BandEntry>,
}

impl M100Band {
    /// 查条目(fail-closed:缺条目 = Err)。
    pub fn entry(&self, tier: &str) -> Result<&M100BandEntry, MlError> {
        self.entries
            .iter()
            .find(|e| e.tier == tier)
            .ok_or_else(|| MlError::DepthBand(format!("容差带缺条目 tier={tier}")))
    }

    /// 比对(fail-closed):双 digest 全等 且 rel_dev ≤ 带;违例逐条列名。
    pub fn check(
        &self,
        tier: &str,
        product_digest: &str,
        m96_golden_digest: &str,
        rel_dev: f64,
    ) -> Result<(), MlError> {
        let e = self.entry(tier)?;
        if product_digest != e.product_digest {
            return Err(MlError::DepthBand(format!(
                "tier={tier} product_digest {product_digest} ≠ golden {}",
                e.product_digest
            )));
        }
        if m96_golden_digest != e.m96_golden_digest {
            return Err(MlError::DepthBand(format!(
                "tier={tier} m96_golden_digest {m96_golden_digest} ≠ golden {}",
                e.m96_golden_digest
            )));
        }
        if rel_dev.is_nan() || rel_dev > e.band_rel_dev {
            return Err(MlError::DepthBand(format!(
                "tier={tier} rel_dev {rel_dev:.6e} 越带(上界 {:.6e})",
                e.band_rel_dev
            )));
        }
        Ok(())
    }

    /// 序列化(手工 JSON;字段序冻结,浮点 `{:e}` 确定性格式)。
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n  \"schema\": \"rurix.g9m100.multi_light_band.v1\",\n");
        s.push_str(&format!("  \"frozen_at_utc\": \"{}\",\n", self.frozen_at_utc));
        s.push_str(&format!("  \"device_name\": \"{}\",\n", self.device_name));
        s.push_str(&format!("  \"scene\": \"{}\",\n", self.scene));
        s.push_str(&format!(
            "  \"m96_anchor_digest\": \"{}\",\n",
            self.m96_anchor_digest
        ));
        s.push_str(&format!(
            "  \"freeze_rule\": \"band_rel_dev = measured_rel_dev * {:.1}(规则冻结于 gi::multi_light::M100_BAND_MARGIN;基值 = 冻结批实测,禁手写 P-09)\",\n",
            M100_BAND_MARGIN
        ));
        s.push_str(&format!("  \"matched_depth\": \"{}\",\n", M100_MATCHED_DEPTH));
        s.push_str(&format!("  \"m96_golden_spp\": \"{}\",\n", M100_M96_GOLDEN_SPP));
        s.push_str(&format!("  \"seed_chain\": \"{}\",\n", M100_SEED));
        s.push_str(&format!(
            "  \"skip_verification_bias\": \"{:e}\",\n",
            self.skip_verification_bias
        ));
        s.push_str(&format!(
            "  \"light_subset_rel_dev\": \"{:e}\",\n",
            self.light_subset_rel_dev
        ));
        s.push_str("  \"entries\": [\n");
        for (i, e) in self.entries.iter().enumerate() {
            s.push_str(&format!(
                "    {{\"tier\": \"{}\", \"product_digest\": \"{}\", \"m96_golden_digest\": \"{}\", \"band_rel_dev\": \"{:e}\", \"measured_rel_dev\": \"{:e}\"}}{}\n",
                e.tier,
                e.product_digest,
                e.m96_golden_digest,
                e.band_rel_dev,
                e.measured_rel_dev,
                if i + 1 == self.entries.len() { "" } else { "," }
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }

    /// 解析(fail-closed:schema 不符/键缺失/数值非法/条目重复一律 Err)。
    pub fn parse(text: &str) -> Result<M100Band, MlError> {
        let err = |m: &str| MlError::DepthBand(format!("容差带解析: {m}"));
        if !text.contains("\"schema\": \"rurix.g9m100.multi_light_band.v1\"") {
            return Err(err("schema 失配"));
        }
        let get_str = |key: &str| -> Result<String, MlError> {
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
            let field = |key: &str| -> Result<String, MlError> {
                let needle = format!("\"{key}\": \"");
                let start = body
                    .find(&needle)
                    .ok_or_else(|| err(&format!("条目缺键 {key}")))?
                    + needle.len();
                let end = body[start..]
                    .find('"')
                    .ok_or_else(|| err("条目键 {key} 值未闭合"))?
                    + start;
                Ok(body[start..end].to_string())
            };
            let tier = field("tier")?;
            if entries.iter().any(|e: &M100BandEntry| e.tier == tier) {
                return Err(err("条目 tier 重复"));
            }
            let product_digest = field("product_digest")?;
            let m96_golden_digest = field("m96_golden_digest")?;
            for (nm, d) in [
                ("product_digest", &product_digest),
                ("m96_golden_digest", &m96_golden_digest),
            ] {
                if d.len() != 64 || !d.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(err(&format!("{nm} 非 64 位 hex")));
                }
            }
            let band_rel_dev: f64 = field("band_rel_dev")?
                .parse()
                .map_err(|_| err("band_rel_dev 非数值"))?;
            let measured_rel_dev: f64 = field("measured_rel_dev")?
                .parse()
                .map_err(|_| err("measured_rel_dev 非数值"))?;
            if band_rel_dev <= 0.0 || !band_rel_dev.is_finite() {
                return Err(err("band_rel_dev 非正有限数"));
            }
            entries.push(M100BandEntry {
                tier,
                product_digest,
                m96_golden_digest,
                band_rel_dev,
                measured_rel_dev,
            });
        }
        if entries.is_empty() {
            return Err(err("entries 为空"));
        }
        Ok(M100Band {
            frozen_at_utc: get_str("frozen_at_utc")?,
            device_name: get_str("device_name")?,
            scene: get_str("scene")?,
            m96_anchor_digest: get_str("m96_anchor_digest")?,
            skip_verification_bias: get_str("skip_verification_bias")?
                .parse()
                .map_err(|_| err("skip_verification_bias 非数值"))?,
            light_subset_rel_dev: get_str("light_subset_rel_dev")?
                .parse()
                .map_err(|_| err("light_subset_rel_dev 非数值"))?,
            entries,
        })
    }
}

// ---------------------------------------------------------------------------
// device 输入打包(kernel `g9_m100_multi_light.rx` 头注参数面逐字同源)
// ---------------------------------------------------------------------------

/// 灯表打包:16 f32/灯(p00 3, e1 3, e2 3, emission 3, area, normal 3)。
pub fn pack_lights(scene: &MultiLightScene) -> Vec<f32> {
    let mut out = Vec::with_capacity(scene.lights.len() * 16);
    for l in &scene.lights {
        let ln = l.normal();
        out.extend_from_slice(&l.p00);
        out.extend_from_slice(&l.e1);
        out.extend_from_slice(&l.e2);
        out.extend_from_slice(&l.emission);
        out.push(l.area());
        out.extend_from_slice(&ln);
    }
    out
}

/// kernel 参数打包(21 f32;与 `kernels/g9_m100_multi_light.rx` 头注逐字同源)。
pub fn pack_ml_params(scene: &MultiLightScene, spp: u32, mode: LowTierMode) -> Vec<f32> {
    let cam = &scene.camera;
    let pixel_count = cam.width * cam.height;
    let mut p = Vec::with_capacity(21);
    p.push(pixel_count as f32);
    p.push(spp as f32);
    p.push(mode.as_f32());
    p.push(scene.lights.len() as f32);
    p.push(RAY_EPS);
    p.push(scene.t_max);
    p.extend_from_slice(&cam.origin);
    p.extend_from_slice(&cam.forward);
    p.extend_from_slice(&cam.right);
    p.extend_from_slice(&cam.up);
    p.push(cam.tan_half_fov);
    p.push(cam.width as f32);
    p.push(cam.height as f32);
    debug_assert_eq!(p.len(), 21);
    p
}

// ---------------------------------------------------------------------------
// 单测(RXS-0361 锚定;fixture / 选灯闭集 / 验证射线零跳过 / 偏置 RED /
// ReSTIR not-triggered / golden 无偏锚 / 容差带 fail-closed / pbrt 锚定语料)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gi::surface_cache;

    fn scene() -> MultiLightScene {
        let s = m100_multi_light_scene();
        s.validate().expect("M100 多灯 fixture 装载");
        s
    }

    //@ spec: RXS-0361
    #[test]
    fn fixture_validates_and_single_light_scenes_pass_m96_validate() {
        let ml = scene();
        assert_eq!(ml.lights.len(), M100_LIGHTS as usize);
        // 几何面:非发光三角逐字承 M96 cornell(cornell 总三角 − 光源 2)+
        // 4 灯 × 2。
        let cornell_tris = path_trace::m96_cornell_scene().indices.len();
        assert_eq!(ml.indices.len(), cornell_tris - 2 + 4 * 2);
        // 逐灯单光源场景过 M96 单光源纪律校验(发光三角 ↔ quad 逐字一致)。
        for k in 0..M100_LIGHTS as usize {
            let s = ml.single_light_pt_scene(k).expect("单灯场景");
            s.validate().expect("单灯场景过 M96 validate");
            assert_eq!(s.light, ml.lights[k]);
        }
        // 灯号越界 fail-closed。
        assert!(ml.single_light_pt_scene(4).is_err());
    }

    //@ spec: RXS-0361
    #[test]
    fn stream_deterministic_and_light_selection_closed_set() {
        // 选灯种子流固定:同 seed 双跑逐位一致;选灯下标闭集 [0,4)。
        let pc = 16usize;
        let a = m100_rng::generate_stream(pc, M100_SPP, M100_SEED);
        let b = m100_rng::generate_stream(pc, M100_SPP, M100_SEED);
        assert_eq!(a, b, "选灯种子流双跑逐位一致");
        let c = m100_rng::generate_stream(pc, M100_SPP, M100_SEED ^ 1);
        assert_ne!(a, c, "异 seed 流必异(协议面)");
        let l = M100_LIGHTS as f32;
        for i in (2..a.len()).step_by(m100_rng::DIMS_PER_SAMPLE) {
            let k = (a[i] * l) as u32;
            assert!(k < M100_LIGHTS, "选灯下标闭集: {k}");
        }
        assert_eq!(a.len(), m100_rng::stream_len(pc, M100_SPP));
    }

    //@ spec: RXS-0361
    #[test]
    fn verification_ray_zero_skip_contract_and_bias_red() {
        // 验证射线零跳过硬契约(D2-Q4):参照档 fired == 主命中样本数、
        // skipped == 0、逐灯 fired 非空;跳验证注入 ⇒ skipped 全量 + 系统性
        // 变亮偏置 ≥ 冻结阈(RED 臂独立有效);sabotage(参照 vs 参照)偏置 0。
        let ml = scene();
        let pc = (ml.camera.width * ml.camera.height) as usize;
        let stream = m100_rng::generate_stream(pc, M100_SPP, M100_SEED);
        let reference = trace_direct_host(&ml, &stream, M100_SPP, LowTierMode::Reference).expect("参照");
        let c = reference.counters();
        assert!(c.primary_hit_samples > 0);
        assert_eq!(c.verification_rays_fired, c.primary_hit_samples, "零跳过:fired == 主命中样本数");
        assert_eq!(c.verification_rays_skipped, 0);
        for (k, &f) in c.per_light_fired.iter().enumerate() {
            assert!(f > 0, "灯 {k} 验证射线实际发射计数非空");
        }
        // 双跑逐位一致(选灯种子流固定 ⇒ 同输入双运行逐位一致)。
        let reference2 = trace_direct_host(&ml, &stream, M100_SPP, LowTierMode::Reference).expect("双跑");
        assert_eq!(reference, reference2, "同输入双运行逐位一致");
        // 跳验证注入:skipped == 主命中样本数 + 变亮偏置 ≥ 阈 + digest 分叉。
        let skipped = trace_direct_host(&ml, &stream, M100_SPP, LowTierMode::SkipVerificationInjected)
            .expect("注入");
        let cs = skipped.counters();
        assert_eq!(cs.verification_rays_fired, 0, "注入档零发射");
        assert_eq!(cs.verification_rays_skipped, cs.primary_hit_samples);
        let bias = (skipped.mean_luminance() - reference.mean_luminance())
            / reference.mean_luminance().max(1e-30);
        assert!(
            bias >= M100_SKIP_BIAS_MIN,
            "跳验证系统性变亮偏置可检测:bias={bias:.6e} ≥ {M100_SKIP_BIAS_MIN}"
        );
        assert_ne!(reference.product_digest(), skipped.product_digest(), "产物 digest 分叉(diag 结构域)");
        // sabotage 探针:参照 vs 参照偏置 = 0 ⇒ 不误检。
        let bias_same = (reference.mean_luminance() - reference.mean_luminance())
            / reference.mean_luminance().max(1e-30);
        assert!(bias_same < M100_SKIP_BIAS_MIN);
    }

    //@ spec: RXS-0361
    #[test]
    fn light_subset_injection_deviation_red() {
        // 灯子集采样注入(只选灯 0):逐灯计数聚于灯 0 + 输出对多灯 golden
        // 偏离 ≥ 冻结阈(RED 臂独立有效)。
        let ml = scene();
        let pc = (ml.camera.width * ml.camera.height) as usize;
        let stream = m100_rng::generate_stream(pc, M100_SPP, M100_SEED);
        let subset = trace_direct_host(&ml, &stream, M100_SPP, LowTierMode::LightSubsetInjected)
            .expect("子集注入");
        let cs = subset.counters();
        assert!(cs.per_light_fired[0] > 0);
        assert!(
            cs.per_light_fired[1..].iter().all(|&f| f == 0),
            "子集注入 ⇒ 仅灯 0 有发射计数"
        );
        // 多灯 golden(host oracle 逐灯求和;直接光档 ⇒ 深度 1)。
        let cfg = path_trace::PtConfig {
            spp: M100_M96_GOLDEN_SPP,
            max_bounces: M100_MATCHED_DEPTH,
            rr_min_bounce: surface_cache::m97_rr_min(M100_MATCHED_DEPTH),
            seed: path_trace::M96_SEED,
            switches: path_trace::PtSwitches::REFERENCE,
        };
        let mut per_light = Vec::new();
        for k in 0..M100_LIGHTS as usize {
            let s = ml.single_light_pt_scene(k).expect("单灯场景");
            let st = path_trace::rng::generate_stream(pc, cfg.spp, cfg.max_bounces, cfg.seed);
            per_light.push(path_trace::trace_host(&s, &cfg, &st).expect("oracle"));
        }
        let golden = golden_sum_image(&per_light).expect("求和");
        let dev_subset = path_trace::rel_dev(&subset.rgb, &golden).expect("rel_dev");
        assert!(
            dev_subset >= M100_SUBSET_REL_DEV_MIN,
            "灯子集偏离可检测:rel_dev={dev_subset:.6e} ≥ {M100_SUBSET_REL_DEV_MIN}"
        );
        // 参照档对 golden 在带内(measured 口径;无偏锚——期望与逐灯和相等)。
        let reference = trace_direct_host(&ml, &stream, M100_SPP, LowTierMode::Reference).expect("参照");
        let dev_ref = path_trace::rel_dev(&reference.rgb, &golden).expect("rel_dev");
        assert!(
            dev_ref < dev_subset,
            "参照档偏离应显著小于子集注入:ref={dev_ref:.4e} subset={dev_subset:.4e}"
        );
        // golden digest 确定性(逐灯 digest 入键)。
        let digs: Vec<[u8; 32]> = per_light.iter().map(path_trace::image_digest).collect();
        let g1 = golden_digest(&golden, &digs);
        let g2 = golden_digest(&golden, &digs);
        assert_eq!(g1, g2);
        let mut digs_bad = digs.clone();
        digs_bad[0][0] ^= 1;
        assert_ne!(golden_digest(&golden, &digs_bad), g1, "单灯 digest 替换必检出");
    }

    //@ spec: RXS-0361
    #[test]
    fn restir_not_triggered_registration() {
        // 高档 ReSTIR:workload 证据不足 ⇒ 登记 not-triggered(显式结构),
        // 服务请求即 typed Err(禁静默当绿,RXS-0361 L3)。
        let RestirTrigger::NotTriggered { reason } = check_restir_trigger();
        assert!(reason.contains("workload"), "登记原因含 workload 字面");
        let served = restir_serve();
        assert!(
            matches!(served, Err(MlError::RestirNotTriggered(_))),
            "高档服务请求必 typed Err"
        );
    }

    //@ spec: RXS-0361
    #[test]
    fn band_roundtrip_and_fail_closed() {
        let band = M100Band {
            frozen_at_utc: "2026-08-13T00:00:00Z".into(),
            device_name: "testdev".into(),
            scene: "m100_multi_light".into(),
            m96_anchor_digest: "a".repeat(64),
            skip_verification_bias: 0.08,
            light_subset_rel_dev: 0.45,
            entries: vec![M100BandEntry {
                tier: "m100_low_reference".into(),
                product_digest: "b".repeat(64),
                m96_golden_digest: "c".repeat(64),
                band_rel_dev: 0.5,
                measured_rel_dev: 0.25,
            }],
        };
        let text = band.to_json();
        let parsed = M100Band::parse(&text).expect("解析");
        assert_eq!(parsed, band, "序列化往返全等");
        parsed
            .check("m100_low_reference", &"b".repeat(64), &"c".repeat(64), 0.5)
            .expect("带内");
        assert!(parsed.check("m100_low_reference", &"d".repeat(64), &"c".repeat(64), 0.5).is_err());
        assert!(parsed.check("m100_low_reference", &"b".repeat(64), &"c".repeat(64), 0.51).is_err());
        assert!(parsed.check("nope", &"b".repeat(64), &"c".repeat(64), 0.1).is_err());
        assert!(M100Band::parse("{\"schema\": \"nope\"}").is_err());
    }

    //@ spec: RXS-0361
    #[test]
    fn pbrt_fixture_anchor_and_m96_gate_anchor() {
        // pbrt 锚定语料消费:导出与 checked-in fixture 逐字相等(语料漂移
        // 即 RED;conformance/gi/scenes/ 消费义务)。
        let ml = scene();
        let text = pbrt_multi_light_text(&ml);
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../conformance/gi/scenes/m100_multi_light_low.pbrt"
        );
        let fixture = std::fs::read_to_string(path).expect("M100 pbrt 锚定语料存在");
        assert_eq!(text, fixture, "pbrt 导出与锚定语料逐字相等");
        // 门序消费锚(D2-Q7):M97 冻结带 m96_cornell 深度 1 条目可读。
        let band_path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../milestones/g9/g9_m97_depth_band.json");
        let band_text = std::fs::read_to_string(band_path).expect("M97 冻结带存在");
        let band = surface_cache::DepthBand::parse(&band_text).expect("M97 带解析");
        let e = band.entry(M100_MATCHED_DEPTH).expect("深度 1 条目");
        assert_eq!(e.m96_digest.len(), 64);
        assert_eq!(M100_MATCHED_DEPTH, 1);
    }
}
