//! G9.4 M97 Surface Cache host 面(spec/global_illumination.md RXS-0358;
//! RFC-0022 §4.6;门 `g9.p0.m97.surface_cache`)。
//!
//! 本模块 = Lumen Card 式表面缓存的 **host 数据面/对拍面**:
//! - [`parameterize`]:离线 Card 参数化器(cook 期;方向聚类 = 主轴方向类 + 投影
//!   面选择,**≤12 Card/mesh 默认上限可配**、超限按表面积确定性裁剪,配置域
//!   fail-closed)— Card 参数(朝向/分辨率/图集槽位)确定性输出,同输入双跑
//!   位级一致;
//! - [`CardBakeRecord`]:页内 Card 烘焙记录 **32B ABI 镜像**(沿 M94
//!   `ClasBakeRecord` 先例——与 `rurix_geom_pages::logical_v2` CLAS 段字段序
//!   逐字同构,跨 crate 一致性单测锚定);
//! - [`build_atlas_page`]:Card 图集页 = **RXPL major=2 页**(每 Card 一簇记录
//!   + CLAS 段 32B 记录镜像),复用 M91 冻结 ABI 编码/解码,**禁止私定磁盘格式**
//!   (RXS-0358 L5);编码产物 digest 进 golden;
//! - [`capture_host`]:运行时辐射度缓存的 host oracle(与 device kernel
//!   `kernels/g9_m97_cache_capture.rx` 公式面同源——投影射线首命中 + M96
//!   NEE/MIS/RR 弹射体逐字同式),单测数值锚;
//! - [`count_leak_pixels`]:漏光检测校验面(低于 ambient 的黑色裂缝 = 漏光
//!   像素;RXS-0358 L3/L4);
//! - [`inject_card_hole`]:Card 空洞注入(图集覆盖人为挖洞;RED 臂承载);
//! - [`DepthBand`]:按匹配深度(1/2/full bounce)对 M96 golden 的容差带
//!   (measured 后冻结,禁手写 P-09;fail-closed 比对器)。
//!
//! ## 确定性协议(承 RXS-0357 L2 同律)
//! - 参数化/图集打包/capture 采样全部确定性:同输入 ⇒ 位级一致;canonical
//!   digest = SHA-256(产物字节依序拼接),不含路径/mtime/seed。
//! - capture RNG 流 = PCG32 单一流按索引寻址([`m97_rng`];逐图集 texel × 逐
//!   采样 × 逐 bounce 5 维排布,与路径动态无关),seed = [`M97_SEED`] 冻结。
//! - 消费渲染主光线 = 像素中心(jitter-free;RFC-0019 §4.6.1 运动约定同口径)。
//!
//! ## 只丢能量不漏光(硬契约,RXS-0358 L3)
//! Card 未覆盖区域消费回退 = ambient 项([`M97_AMBIENT`]):输出非负、无低于
//! ambient 的黑色裂缝;缺失覆盖表现为**能量缺失**(能量差 measured 进
//! evidence)而非错误染色——漏光像素计数恒 0;注入空洞 + 关回退变体
//! 必被 [`count_leak_pixels`] 检出(漏光像素计数 > 0,RED 臂)。

use crate::rt::bvh::{InstanceDesc, Ray, Tlas, Transform3x4, TriBvh, Vec3};
use crate::rt::ref_tracer::RAY_EPS;

use super::path_trace::{
    self, MaterialKind, PtScene, cosine_hemisphere_pdf, mis_weight_bsdf, mis_weight_light,
};

// ---------------------------------------------------------------------------
// 冻结常量(协议面)
// ---------------------------------------------------------------------------

/// 冻结 capture seed(device 腿;canonical digest 不含此值——digest 是输出字节哈希)。
pub const M97_SEED: u64 = 0x5C97_4A2D_8E1F_6B03;
/// 匹配深度三档(RXS-0357 L2:1/2/full bounce;full = M96 冻结 max_bounces)。
pub const M97_DEPTHS: [u32; 3] = [1, 2, path_trace::M96_MAX_BOUNCES];
/// 默认 Card/mesh 上限(Lumen 口径 12;可配,见 [`ScConfig`])。
pub const M97_DEFAULT_MAX_CARDS_PER_MESH: u32 = 12;
/// Card/mesh 上限硬顶(配置域 fail-closed 上界;超出 = 配置非法)。
pub const M97_HARD_MAX_CARDS_PER_MESH: u32 = 64;
/// 冻结 Card 分辨率默认值( texels/边;图集槽位 = texel_base 平坦排布)。
pub const M97_DEFAULT_CARD_RES: u32 = 8;
/// 冻结 capture 每 texel 采样数默认值。
pub const M97_DEFAULT_SAMPLES_PER_TEXEL: u32 = 16;
/// 缺失覆盖回退 ambient 项(线性 RGB;「无低于 ambient 的黑色裂缝」的地板)。
pub const M97_AMBIENT: [f32; 3] = [0.02, 0.02, 0.02];
/// 漏光检测阈值(低于 ambient − 本 eps 即漏光像素;浮点余量)。
pub const M97_LEAK_EPS: f32 = 1e-4;
/// 深度容差带 margin(带 = measured × 2;规则冻结,基值实测,P-09)。
pub const M97_BAND_MARGIN: f64 = 2.0;
/// 深度对照 M96 golden 的 spp(冻结;与 M96 容差带 spp 序列末档一致)。
pub const M97_M96_GOLDEN_SPP: u32 = 64;
/// 逐深度 RR 起始 bounce:`min(M96_RR_MIN_BOUNCE, depth−1)`——深度内 RR
/// 输出中性(末 bounce 的 RR 只缩放不再消费的吞吐),full 档回落 M96 冻结值。
pub fn m97_rr_min(depth: u32) -> u32 {
    path_trace::M96_RR_MIN_BOUNCE.min(depth.saturating_sub(1))
}

// ---------------------------------------------------------------------------
// 错误面(fail-closed typed Err;本模块一切失败为类型化拒绝,严禁 UB)
// ---------------------------------------------------------------------------

/// M97 host 面错误(参数化/图集页/打包/比对全部 fail-closed)。
#[derive(Debug, Clone, PartialEq)]
pub enum ScError {
    /// 配置非法(上限/分辨率/采样数越域)。
    InvalidConfig(String),
    /// 场景/网格非法(空 mesh、三角范围越界、退化/非有限几何)。
    InvalidScene(String),
    /// 图集页错误(RXPL v2 编码/解码/往返失败)。
    AtlasPage(String),
    /// 深度容差带错误(解析/比对失败)。
    DepthBand(String),
}

impl std::fmt::Display for ScError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScError::InvalidConfig(m) => write!(f, "配置非法: {m}"),
            ScError::InvalidScene(m) => write!(f, "场景非法: {m}"),
            ScError::AtlasPage(m) => write!(f, "图集页: {m}"),
            ScError::DepthBand(m) => write!(f, "深度容差带: {m}"),
        }
    }
}

impl std::error::Error for ScError {}

// ---------------------------------------------------------------------------
// 配置与 mesh 描述(离线参数化输入)
// ---------------------------------------------------------------------------

/// cook 期参数化配置(≤12/mesh 可配 + Card 分辨率 + 每 texel 采样数;
/// 全部字段进 CardSet canonical digest——配置漂移 ⇒ digest 漂移)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScConfig {
    /// Card/mesh 上限(默认 12,Lumen 口径;域 [1, [`M97_HARD_MAX_CARDS_PER_MESH`]])。
    pub max_cards_per_mesh: u32,
    /// Card 分辨率(texels/边;域 [1, 64])。
    pub card_res: u32,
    /// capture 每 texel 采样数(域 [1, 1024])。
    pub samples_per_texel: u32,
}

impl Default for ScConfig {
    fn default() -> Self {
        ScConfig {
            max_cards_per_mesh: M97_DEFAULT_MAX_CARDS_PER_MESH,
            card_res: M97_DEFAULT_CARD_RES,
            samples_per_texel: M97_DEFAULT_SAMPLES_PER_TEXEL,
        }
    }
}

impl ScConfig {
    /// fail-closed 配置校验(配置域外一律 typed Err;「可配上限 fail-closed」
    /// 判据承载)。
    pub fn validate(&self) -> Result<(), ScError> {
        if self.max_cards_per_mesh == 0 || self.max_cards_per_mesh > M97_HARD_MAX_CARDS_PER_MESH
        {
            return Err(ScError::InvalidConfig(format!(
                "max_cards_per_mesh {} 越域 [1, {M97_HARD_MAX_CARDS_PER_MESH}]",
                self.max_cards_per_mesh
            )));
        }
        if self.card_res == 0 || self.card_res > 64 {
            return Err(ScError::InvalidConfig(format!(
                "card_res {} 越域 [1, 64]",
                self.card_res
            )));
        }
        if self.samples_per_texel == 0 || self.samples_per_texel > 1024 {
            return Err(ScError::InvalidConfig(format!(
                "samples_per_texel {} 越域 [1, 1024]",
                self.samples_per_texel
            )));
        }
        Ok(())
    }
}

/// mesh 描述(参数化输入单元:场景三角形汤的连续三角范围 + 稳定名)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScMesh {
    /// 稳定 mesh 名(cook profile/provenance 键)。
    pub name: String,
    /// 首三角(场景 `indices` 下标,含)。
    pub tri_start: u32,
    /// 三角数。
    pub tri_count: u32,
}

// ---------------------------------------------------------------------------
// Card(离线参数化输出单元;确定性)
// ---------------------------------------------------------------------------

/// Card 参数集(朝向/分辨率/图集槽位确定性输出;RXS-0358 L1)。
#[derive(Debug, Clone, PartialEq)]
pub struct ScCard {
    /// Card 稳定 id(= 参数化输出序;图集槽位/digest 序)。
    pub card_id: u32,
    /// 所属 mesh(输入 `meshes` 下标)。
    pub mesh_index: u32,
    /// Card 朝向(投影面法线 = 主轴方向,单位;精确 ±轴)。
    pub normal: [f32; 3],
    /// Card 平面原点((u,v) = (0,0) 角点的世界坐标,在平面上)。
    pub origin: [f32; 3],
    /// 面内 u 轴(单位,⊥ normal;确定性轴表)。
    pub axis_u: [f32; 3],
    /// 面内 v 轴(单位,⊥ normal/u)。
    pub axis_v: [f32; 3],
    /// u 向世界尺寸(>0)。
    pub size_u: f32,
    /// v 向世界尺寸(>0)。
    pub size_v: f32,
    /// 覆盖三角(场景 `indices` 下标集,升序;参数化分区,互不重叠)。
    pub tris: Vec<u32>,
    /// 页内图元偏移(图集页索引段内,图元计;[`CardBakeRecord`] 镜像字段)。
    pub page_tri_offset: u32,
    /// 分辨率 u(texels)。
    pub res_u: u32,
    /// 分辨率 v(texels)。
    pub res_v: u32,
    /// 图集槽位 = 平坦 texel 基址(逐 Card 顺序累加;槽位 = [texel_base,
    /// texel_base + res_u·res_v) 区间,不重叠)。
    pub texel_base: u32,
    /// 覆盖三角 AABB min(逐位 min/max;[`CardBakeRecord`] 镜像字段)。
    pub aabb_min: [f32; 3],
    /// 覆盖三角 AABB max。
    pub aabb_max: [f32; 3],
    /// 投影射线高度(自平面沿 normal 的提升;capture 首射线 t_max = 2·margin)。
    pub margin: f32,
    /// 法向锥 cutoff(= 覆盖三角法线与 Card 法线最小点积,clamp [−1,1])。
    pub cone_cutoff: f32,
}

impl ScCard {
    /// texel 数(= res_u·res_v)。
    pub fn texel_count(&self) -> u32 {
        self.res_u * self.res_v
    }

    /// Card 中心(AABB 中点;页簇记录 center 字段)。
    pub fn center(&self) -> [f32; 3] {
        [
            (self.aabb_min[0] + self.aabb_max[0]) * 0.5,
            (self.aabb_min[1] + self.aabb_max[1]) * 0.5,
            (self.aabb_min[2] + self.aabb_max[2]) * 0.5,
        ]
    }
}

/// 参数化产物(Card 集合 + 逐三角→Card 映射 + 图集 texel 总量;确定性)。
#[derive(Debug, Clone, PartialEq)]
pub struct CardSet {
    /// Card 序列(card_id 序)。
    pub cards: Vec<ScCard>,
    /// 逐场景三角 → Card id(无 Card = [`TRI_NO_CARD`];参数化分区满载时无 sentinel)。
    pub tri_to_card: Vec<u32>,
    /// 图集 texel 总量(= Σ card.texel_count())。
    pub total_texels: u32,
    /// 生成配置(digest 域)。
    pub config: ScConfig,
}

/// 逐三角映射 sentinel(无 Card 覆盖;打包为 f32 = −1.0,device 算术门消费)。
pub const TRI_NO_CARD: u32 = u32::MAX;

impl CardSet {
    /// canonical 字节(确定性;digest 域 = 配置 + 全 Card 字段 + 覆盖三角集)。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        fn put_u32(out: &mut Vec<u8>, v: u32) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        fn put_f32(out: &mut Vec<u8>, v: f32) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"SC-CARDSET-V1\0");
        put_u32(&mut out, self.config.max_cards_per_mesh);
        put_u32(&mut out, self.config.card_res);
        put_u32(&mut out, self.config.samples_per_texel);
        put_u32(&mut out, self.cards.len() as u32);
        put_u32(&mut out, self.total_texels);
        for c in &self.cards {
            put_u32(&mut out, c.card_id);
            put_u32(&mut out, c.mesh_index);
            for &v in c
                .normal
                .iter()
                .chain(c.origin.iter())
                .chain(c.axis_u.iter())
                .chain(c.axis_v.iter())
            {
                put_f32(&mut out, v);
            }
            put_f32(&mut out, c.size_u);
            put_f32(&mut out, c.size_v);
            put_u32(&mut out, c.tris.len() as u32);
            for &t in &c.tris {
                put_u32(&mut out, t);
            }
            put_u32(&mut out, c.page_tri_offset);
            put_u32(&mut out, c.res_u);
            put_u32(&mut out, c.res_v);
            put_u32(&mut out, c.texel_base);
            for &v in c.aabb_min.iter().chain(c.aabb_max.iter()) {
                put_f32(&mut out, v);
            }
            put_f32(&mut out, c.margin);
            put_f32(&mut out, c.cone_cutoff);
        }
        out
    }

    /// canonical digest(SHA-256;参数化产物 golden 键)。
    pub fn digest(&self) -> [u8; 32] {
        rurix_pkg::sha256::digest(&self.canonical_bytes())
    }
}

// ---------------------------------------------------------------------------
// 离线 Card 参数化器(方向聚类 + 投影面选择 + 超限裁剪;确定性)
// ---------------------------------------------------------------------------

/// 主轴方向类(0..6:axis·2 + sign;聚类键)。
fn axis_class(n: Vec3) -> u32 {
    let ax = n.x.abs();
    let ay = n.y.abs();
    let az = n.z.abs();
    let axis = if ax >= ay && ax >= az {
        0
    } else if ay >= az {
        1
    } else {
        2
    };
    let sign = match axis {
        0 => n.x < 0.0,
        1 => n.y < 0.0,
        _ => n.z < 0.0,
    };
    (axis as u32) * 2 + u32::from(sign)
}

/// 类键 → 精确 ±轴单位法线。
fn class_normal(class: u32) -> [f32; 3] {
    let mut n = [0.0; 3];
    n[(class / 2) as usize] = if class % 2 == 0 { 1.0 } else { -1.0 };
    n
}

/// 类键 → 确定性面内轴表(与法线 ⊥;与符号无关)。
fn class_axes(class: u32) -> ([f32; 3], [f32; 3]) {
    match class / 2 {
        0 => ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        1 => ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        _ => ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    }
}

/// 三角几何法线(单位)与面积(2×面积 = |cross|)。退化三角 typed Err。
fn tri_normal_area(
    positions: &[[f32; 3]],
    idx: [u32; 3],
) -> Result<(Vec3, f32), ScError> {
    let a = Vec3::from_array(positions[idx[0] as usize]);
    let b = Vec3::from_array(positions[idx[1] as usize]);
    let c = Vec3::from_array(positions[idx[2] as usize]);
    let cr = (b - a).cross(c - a);
    let len = cr.length();
    if !(len.is_finite() && len > 0.0) {
        return Err(ScError::InvalidScene(format!(
            "退化/非有限三角 {idx:?}(|cross| = {len})"
        )));
    }
    Ok((cr * (1.0 / len), len * 0.5))
}

/// 离线 Card 参数化(cook 期;RXS-0358 L1)。
///
/// 算法(确定性):逐 mesh 按**主轴方向类**聚类三角(方向聚类),每类一 Card,
/// 投影面 = 类法线正交面(投影面选择),Card 平面 = 覆盖顶点法向坐标均值面,
/// 面内 AABB 定 origin/size;类数超 `max_cards_per_mesh` 时按**表面积降序**
/// 保留上限个类、被裁三角重指派到点积最大的保留类(裁剪策略;同面积按类键
/// 升序决胜)。输出序 = mesh 序 × 类键升序(裁剪后),texel 槽位顺序累加。
pub fn parameterize(
    positions: &[[f32; 3]],
    indices: &[[u32; 3]],
    meshes: &[ScMesh],
    config: &ScConfig,
) -> Result<CardSet, ScError> {
    config.validate()?;
    if positions.is_empty() || indices.is_empty() {
        return Err(ScError::InvalidScene("空场景".into()));
    }
    if meshes.is_empty() {
        return Err(ScError::InvalidScene("空 mesh 集".into()));
    }
    for (i, p) in positions.iter().enumerate() {
        if !p.iter().all(|c| c.is_finite()) {
            return Err(ScError::InvalidScene(format!("顶点 {i} 非有限")));
        }
    }
    // mesh 范围校核 + 全覆盖校核(逐三角恰属一 mesh)。
    let mut mesh_of_tri = vec![u32::MAX; indices.len()];
    for (mi, m) in meshes.iter().enumerate() {
        let end = m.tri_start as usize + m.tri_count as usize;
        if end > indices.len() {
            return Err(ScError::InvalidScene(format!(
                "mesh `{}` 三角范围 [{}, {}) 越界(三角数 {})",
                m.name,
                m.tri_start,
                end,
                indices.len()
            )));
        }
        for t in m.tri_start as usize..end {
            if mesh_of_tri[t] != u32::MAX {
                return Err(ScError::InvalidScene(format!(
                    "三角 {t} 被多 mesh 重复覆盖"
                )));
            }
            mesh_of_tri[t] = mi as u32;
        }
    }
    if let Some(t) = mesh_of_tri.iter().position(|&m| m == u32::MAX) {
        return Err(ScError::InvalidScene(format!("三角 {t} 未被任何 mesh 覆盖")));
    }

    // 逐三角法线/面积(一次计算,聚类/锥/面积共用)。
    let mut normals = Vec::with_capacity(indices.len());
    let mut areas = Vec::with_capacity(indices.len());
    for (t, &idx) in indices.iter().enumerate() {
        for vi in idx {
            if vi as usize >= positions.len() {
                return Err(ScError::InvalidScene(format!(
                    "三角 {t} 索引 {vi} 越界(顶点数 {})",
                    positions.len()
                )));
            }
        }
        let (n, a) = tri_normal_area(positions, idx)?;
        normals.push(n);
        areas.push(a);
    }

    let mut cards: Vec<ScCard> = Vec::new();
    let mut tri_to_card = vec![TRI_NO_CARD; indices.len()];
    let mut texel_base = 0u32;
    for (mi, m) in meshes.iter().enumerate() {
        // 方向聚类:类键 → 三角集(三角序 = mesh 内升序,BTreeMap 键序确定)。
        let mut classes: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
        for t in m.tri_start..m.tri_start + m.tri_count {
            classes
                .entry(axis_class(normals[t as usize]))
                .or_default()
                .push(t);
        }
        // 超限裁剪:表面积降序(同面积类键升序)保留上限;被裁三角重指派到
        // 点积最大的保留类(同点积类键升序决胜)——确定性。
        let mut order: Vec<u32> = classes.keys().copied().collect();
        let area_of = |cls: u32| -> f32 {
            classes[&cls]
                .iter()
                .fold(0.0f32, |acc, &t| acc + areas[t as usize])
        };
        if order.len() as u32 > config.max_cards_per_mesh {
            let mut by_area = order.clone();
            by_area.sort_by(|a, b| {
                area_of(*b)
                    .total_cmp(&area_of(*a))
                    .then_with(|| a.cmp(b))
            });
            let kept: Vec<u32> = by_area[..config.max_cards_per_mesh as usize].to_vec();
            let dropped: Vec<u32> = by_area[config.max_cards_per_mesh as usize..].to_vec();
            for d in dropped {
                let tris = classes.remove(&d).expect("类在集");
                for t in tris {
                    let tn = normals[t as usize];
                    let mut best: Option<(f32, u32)> = None;
                    for &k in &kept {
                        let kn = Vec3::from_array(class_normal(k));
                        let score = tn.dot(kn);
                        best = Some(match best {
                            None => (score, k),
                            Some((bs, bk)) => {
                                if score > bs || (score == bs && k < bk) {
                                    (score, k)
                                } else {
                                    (bs, bk)
                                }
                            }
                        });
                    }
                    let (_, bk) = best.expect("kept 非空(上限 ≥1)");
                    classes.get_mut(&bk).expect("保留类在集").push(t);
                }
            }
            order = kept;
            // 重指派后回到类键升序(确定性输出序)。
            order.sort_unstable();
            for v in classes.values_mut() {
                v.sort_unstable();
            }
        }

        for cls in order {
            let tris = classes.remove(&cls).expect("类在集");
            let normal = class_normal(cls);
            let (axis_u, axis_v) = class_axes(cls);
            let n = Vec3::from_array(normal);
            let au = Vec3::from_array(axis_u);
            let av = Vec3::from_array(axis_v);
            // 平面法向坐标 = 覆盖顶点均值(平面簇逐字精确;合并簇近似,投影
            // 射线在 margin 内找回真实表面)。
            let mut off_acc = 0.0f32;
            let mut off_n = 0.0f32;
            let (mut min_u, mut max_u) = (f32::INFINITY, f32::NEG_INFINITY);
            let (mut min_v, mut max_v) = (f32::INFINITY, f32::NEG_INFINITY);
            let mut aabb_min = [f32::INFINITY; 3];
            let mut aabb_max = [f32::NEG_INFINITY; 3];
            let mut cone_cutoff = 1.0f32;
            for &t in &tris {
                cone_cutoff = cone_cutoff.min(normals[t as usize].dot(n));
                for &vi in &indices[t as usize] {
                    let p = Vec3::from_array(positions[vi as usize]);
                    off_acc += p.dot(n);
                    off_n += 1.0;
                    min_u = min_u.min(p.dot(au));
                    max_u = max_u.max(p.dot(au));
                    min_v = min_v.min(p.dot(av));
                    max_v = max_v.max(p.dot(av));
                    for k in 0..3 {
                        aabb_min[k] = aabb_min[k].min(positions[vi as usize][k]);
                        aabb_max[k] = aabb_max[k].max(positions[vi as usize][k]);
                    }
                }
            }
            let plane_off = off_acc / off_n;
            let size_u = max_u - min_u;
            let size_v = max_v - min_v;
            if !(size_u > 0.0 && size_v > 0.0) {
                return Err(ScError::InvalidScene(format!(
                    "mesh `{}` 类 {cls} 投影面退化(size {size_u}×{size_v})",
                    m.name
                )));
            }
            let origin_v = n * plane_off + au * min_u + av * min_v;
            let margin = (0.25 * size_u.max(size_v)).max(4.0 * RAY_EPS);
            let card_id = cards.len() as u32;
            let card = ScCard {
                card_id,
                mesh_index: mi as u32,
                normal,
                origin: origin_v.to_array(),
                axis_u,
                axis_v,
                size_u,
                size_v,
                tris: tris.clone(),
                page_tri_offset: 0, // 图集页打包时回填
                res_u: config.card_res,
                res_v: config.card_res,
                texel_base,
                aabb_min,
                aabb_max,
                margin,
                cone_cutoff: cone_cutoff.clamp(-1.0, 1.0),
            };
            for &t in &tris {
                tri_to_card[t as usize] = card_id;
            }
            texel_base += card.texel_count();
            cards.push(card);
        }
    }
    Ok(CardSet {
        cards,
        tri_to_card,
        total_texels: texel_base,
        config: *config,
    })
}

// ---------------------------------------------------------------------------
// 页内 Card 烘焙记录 32B ABI 镜像(RXS-0358 L5;沿 M94 ClasBakeRecord 先例)
// ---------------------------------------------------------------------------

/// Card 烘焙输入记录(页内 ABI 镜像:`page_tri_offset:u32`、`tri_count:u32`
/// 与 Card 级 AABB 6×f32,小端;**32B 定长**,与 `rurix_geom_pages::logical_v2`
/// 的 CLAS 段逐字段同构——跨 crate 一致性由单测锚定,沿 M94 `ClasBakeRecord`
/// 32B 镜像先例)。
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardBakeRecord {
    /// 页内三角形偏移(图元计)。
    pub triangle_offset: u32,
    /// Card 覆盖三角形数。
    pub triangle_count: u32,
    /// Card 级 AABB min。
    pub aabb_min: [f32; 3],
    /// Card 级 AABB max。
    pub aabb_max: [f32; 3],
}

impl CardBakeRecord {
    /// 自 Card 推导(AABB 逐位 min/max,与 [`ScCard`] 同口径)。
    pub fn of_card(triangle_offset: u32, card: &ScCard) -> Self {
        Self {
            triangle_offset,
            triangle_count: card.tris.len() as u32,
            aabb_min: card.aabb_min,
            aabb_max: card.aabb_max,
        }
    }

    /// 小端编码(32B;与 logical_v2 CLAS 段字段序逐字一致:offset/count/aabb)。
    pub fn encode_le(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[0..4].copy_from_slice(&self.triangle_offset.to_le_bytes());
        out[4..8].copy_from_slice(&self.triangle_count.to_le_bytes());
        for (k, &v) in self.aabb_min.iter().enumerate() {
            out[8 + k * 4..12 + k * 4].copy_from_slice(&v.to_le_bytes());
        }
        for (k, &v) in self.aabb_max.iter().enumerate() {
            out[20 + k * 4..24 + k * 4].copy_from_slice(&v.to_le_bytes());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Card 图集页(RXPL major=2;复用 M91 冻结 ABI,禁私定磁盘格式)
// ---------------------------------------------------------------------------

/// Card 图集页 page_id(冻结常量;digest 域)。
pub const M97_ATLAS_PAGE_ID: u64 = 0x0000_0970_0000_0001;

/// 由 CardSet 构建 RXPL v2 图集页(每 Card 一簇;v1 段携带 Card 覆盖几何
/// [逐三角 3 顶点汤,页内 u8 索引],CLAS 段 = [`CardBakeRecord`] 32B 镜像,
/// 骨骼段 = 非蒙皮零值三字段)。页 = 合法 v2 页,`decode_logical_page_v2`
/// 往返无损——「图集页复用页格式 ABI 不私定格式」的承载面。
///
/// 返回 (页, 编码字节);编码字节 digest = 图集页产物 golden 键。
pub fn build_atlas_page(
    set: &mut CardSet,
    positions: &[[f32; 3]],
    indices: &[[u32; 3]],
) -> Result<(rurix_geom_pages::logical_v2::LogicalPageV2, Vec<u8>), ScError> {
    use rurix_geom_pages::logical::{FLAG_ROOT, LogicalPage, PageClusterRecord, quantize_center};
    use rurix_geom_pages::logical_v2::{LogicalPageV2, V2ClusterExt, encode_logical_page_v2};

    // 场景 bounds(页 header 面)。
    let mut bounds = [f32::INFINITY; 6];
    bounds[3] = f32::NEG_INFINITY;
    bounds[4] = f32::NEG_INFINITY;
    bounds[5] = f32::NEG_INFINITY;
    for p in positions {
        for k in 0..3 {
            bounds[k] = bounds[k].min(p[k]);
            bounds[3 + k] = bounds[3 + k].max(p[k]);
        }
    }

    let mut clusters = Vec::with_capacity(set.cards.len());
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut page_indices: Vec<u8> = Vec::new();
    let mut ext = Vec::with_capacity(set.cards.len());
    for card in set.cards.iter_mut() {
        let vertex_offset = vertices.len() as u32;
        let page_tri_offset = page_indices.len() as u32 / 3;
        card.page_tri_offset = page_tri_offset;
        for &t in &card.tris {
            let base = vertices.len();
            if base + 3 > 256 {
                return Err(ScError::AtlasPage(format!(
                    "Card {} 顶点数越 u8 索引域(>256)",
                    card.card_id
                )));
            }
            for &vi in &indices[t as usize] {
                vertices.push(positions[vi as usize]);
            }
            page_indices.extend_from_slice(&[base as u8, (base + 1) as u8, (base + 2) as u8]);
        }
        let center = card.center();
        let (qx, qy, qz) = quantize_center(center, bounds);
        let c = Vec3::from_array(center);
        let r = (Vec3::from_array(card.aabb_max) - c).length();
        clusters.push(PageClusterRecord {
            cluster_id: card.card_id,
            qx,
            qy,
            qz,
            center,
            radius: r,
            cone_axis: card.normal,
            cone_cutoff: card.cone_cutoff,
            error: 0.0,
            parent_error: f32::MAX,
            vertex_offset,
            triangle_offset: page_tri_offset,
            vertex_count: (card.tris.len() * 3) as u32,
            triangle_count: card.tris.len() as u32,
            level: 0,
            group: card.mesh_index,
        });
        ext.push(V2ClusterExt::unskinned(card.aabb_min, card.aabb_max));
    }
    let page = LogicalPageV2 {
        base: LogicalPage {
            page_id: M97_ATLAS_PAGE_ID,
            flags: FLAG_ROOT,
            lod_level_min: 0,
            lod_level_max: 0,
            bounds,
            clusters,
            vertices,
            indices: page_indices,
            dependency_page_ids: Vec::new(),
            dag_links: Vec::new(),
        },
        ext,
    };
    let bytes = encode_logical_page_v2(&page);
    // 装配期自核验:冻结 ABI 解码往返无损(不私定格式的机器承载)。
    let back = rurix_geom_pages::logical_v2::decode_logical_page_v2(&bytes)
        .map_err(|e| ScError::AtlasPage(format!("图集页编码自解码失败: {e}")))?;
    if back != page {
        return Err(ScError::AtlasPage("图集页往返有损".into()));
    }
    Ok((page, bytes))
}

/// 图集页产物 digest(SHA-256 编码字节;golden 键)。
pub fn atlas_page_digest(page_bytes: &[u8]) -> [u8; 32] {
    rurix_pkg::sha256::digest(page_bytes)
}

// ---------------------------------------------------------------------------
// device 输入打包(kernel 头注参数面逐字同源;全 f32 缓冲,承 M96 打包纪律)
// ---------------------------------------------------------------------------

/// Card 打包步长(24 f32/Card):
/// [0..3]=origin [3..6]=axis_u [6..9]=axis_v [9..12]=normal
/// [12]=size_u [13]=size_v [14]=res_u [15]=res_v [16]=texel_base [17]=margin
/// [18..24]=pad(0)。
pub const CARD_STRIDE: usize = 24;

/// Card 参数打包(device cards 缓冲)。
pub fn pack_cards(set: &CardSet) -> Vec<f32> {
    let mut out = Vec::with_capacity(set.cards.len() * CARD_STRIDE);
    for c in &set.cards {
        out.extend_from_slice(&c.origin);
        out.extend_from_slice(&c.axis_u);
        out.extend_from_slice(&c.axis_v);
        out.extend_from_slice(&c.normal);
        out.push(c.size_u);
        out.push(c.size_v);
        out.push(c.res_u as f32);
        out.push(c.res_v as f32);
        out.push(c.texel_base as f32);
        out.push(c.margin);
        out.extend_from_slice(&[0.0; 6]);
        debug_assert_eq!(out.len() % CARD_STRIDE, 0);
    }
    out
}

/// 逐 texel → Card id 打包(f32;capture kernel 直查)。
pub fn pack_texel_card(set: &CardSet) -> Vec<f32> {
    let mut out = vec![0.0f32; set.total_texels as usize];
    for c in &set.cards {
        for t in c.texel_base..c.texel_base + c.texel_count() {
            out[t as usize] = c.card_id as f32;
        }
    }
    out
}

/// 逐三角 → Card id 打包(f32;无 Card = −1.0 sentinel,render kernel 算术门)。
pub fn pack_tri_to_card(set: &CardSet) -> Vec<f32> {
    set.tri_to_card
        .iter()
        .map(|&c| {
            if c == TRI_NO_CARD {
                -1.0
            } else {
                c as f32
            }
        })
        .collect()
}

/// capture 参数打包(24 f32;kernel 头注布局逐字同源):
/// [0]=texel_count [1]=samples_per_texel [2]=depth [3]=rr_min
/// [4]=ray_eps [5]=t_max [6]=rng_stride(=5·depth)
/// [7..10]=light_p00 [10..13]=light_e1 [13..16]=light_e2 [16]=light_area
/// [17..20]=light_emission [20..23]=light_normal [23]=pad。
pub fn pack_capture_params(
    scene: &PtScene,
    config: &ScConfig,
    total_texels: u32,
    depth: u32,
) -> Vec<f32> {
    let l = &scene.light;
    let ln = l.normal();
    let mut p = Vec::with_capacity(24);
    p.push(total_texels as f32);
    p.push(config.samples_per_texel as f32);
    p.push(depth as f32);
    p.push(m97_rr_min(depth) as f32);
    p.push(RAY_EPS);
    p.push(scene.t_max);
    p.push(m97_rng::sample_stride(depth) as f32);
    p.extend_from_slice(&l.p00);
    p.extend_from_slice(&l.e1);
    p.extend_from_slice(&l.e2);
    p.push(l.area());
    p.extend_from_slice(&l.emission);
    p.extend_from_slice(&ln);
    p.push(0.0);
    debug_assert_eq!(p.len(), 24);
    p
}

/// render 参数打包(24 f32;kernel 头注布局逐字同源):
/// [0]=pixel_count [1]=width [2]=height [3]=fallback_on(0/1)
/// [4..7]=ambient_rgb [7..10]=cam_origin [10..13]=forward [13..16]=right
/// [16..19]=up [19]=tan(fov/2) [20]=1/width [21]=1/height [22]=ray_eps [23]=t_max。
pub fn pack_render_params(scene: &PtScene, fallback_on: bool) -> Vec<f32> {
    let cam = &scene.camera;
    let mut p = Vec::with_capacity(24);
    p.push((cam.width * cam.height) as f32);
    p.push(cam.width as f32);
    p.push(cam.height as f32);
    p.push(if fallback_on { 1.0 } else { 0.0 });
    p.extend_from_slice(&M97_AMBIENT);
    p.extend_from_slice(&cam.origin);
    p.extend_from_slice(&cam.forward);
    p.extend_from_slice(&cam.right);
    p.extend_from_slice(&cam.up);
    p.push(cam.tan_half_fov);
    p.push(1.0 / cam.width as f32);
    p.push(1.0 / cam.height as f32);
    p.push(RAY_EPS);
    p.push(scene.t_max);
    debug_assert_eq!(p.len(), 24);
    p
}

// ---------------------------------------------------------------------------
// capture RNG 流(确定性协议核心;承 RXS-0357 L2 同律——单一流按索引寻址)
// ---------------------------------------------------------------------------

/// capture 流布局(冻结):逐图集 texel × 逐采样 × 逐 bounce 5 维
/// [nee_u, nee_v, bsdf_r1, bsdf_r2, rr](无相机维——首射线 = 确定性投影射线)。
pub mod m97_rng {
    use crate::rt::ref_tracer::Pcg32;

    /// 每 bounce 随机维数(NEE 2 + BSDF 2 + RR 1;与 M96 同构)。
    pub const DIMS_PER_BOUNCE: usize = 5;

    /// 每采样 floats(= 5·depth)。
    pub fn sample_stride(depth: u32) -> usize {
        DIMS_PER_BOUNCE * depth as usize
    }

    /// 流总长(= total_texels · samples · stride)。
    pub fn stream_len(total_texels: usize, samples: u32, depth: u32) -> usize {
        total_texels * samples as usize * sample_stride(depth)
    }

    /// 采样 (texel, sample) 的流起始下标。
    pub fn sample_base(texel: usize, sample: usize, samples: u32, depth: u32) -> usize {
        (texel * samples as usize + sample) * sample_stride(depth)
    }

    /// 生成整条流(单 Pcg32 实例,图集 texel 序顺序产出;seed = M97 冻结)。
    pub fn generate_stream(total_texels: usize, samples: u32, depth: u32, seed: u64) -> Vec<f32> {
        let mut rng = Pcg32::new(seed);
        let mut out = Vec::with_capacity(stream_len(total_texels, samples, depth));
        for _ in 0..stream_len(total_texels, samples, depth) {
            out.push(rng.next_f32());
        }
        out
    }
}

// ---------------------------------------------------------------------------
// 产物 digest 域(SHA-256 字节依序拼接;不含路径/mtime/seed)
// ---------------------------------------------------------------------------

/// f32 切片 → LE 字节(digest 域)。
pub fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// 运行时辐射度缓存产物 digest(SHA-256(radiance ‖ coverage 字节))。
pub fn cache_product_digest(radiance: &[f32], coverage: &[f32]) -> [u8; 32] {
    let mut pre = bytes_f32(radiance);
    pre.extend_from_slice(&bytes_f32(coverage));
    rurix_pkg::sha256::digest(&pre)
}

/// 消费渲染产物 digest(SHA-256(rgb ‖ flags 字节))。
pub fn render_product_digest(rgb: &[f32], flags: &[f32]) -> [u8; 32] {
    let mut pre = bytes_f32(rgb);
    pre.extend_from_slice(&bytes_f32(flags));
    rurix_pkg::sha256::digest(&pre)
}

// ---------------------------------------------------------------------------
// 漏光检测校验面与空洞注入(RXS-0358 L3/L4)
// ---------------------------------------------------------------------------

/// render kernel 逐像素 flag(写 f32,精确整数值):
/// 0 = 相机 miss(场景外,不参与漏光判定);1 = 缓存命中;2 = 缺失覆盖回退区。
pub const FLAG_CAMERA_MISS: f32 = 0.0;
/// 缓存命中 flag。
pub const FLAG_CACHE_HIT: f32 = 1.0;
/// 缺失覆盖回退 flag。
pub const FLAG_FALLBACK: f32 = 2.0;

/// 漏光像素计数(RXS-0358 L3「漏光像素计数=0」/L4「漏光像素计数≠0 即 RED」
/// 的机器承载):**缺失覆盖回退区**(flag = [`FLAG_FALLBACK`])中任一通道
/// 非有限或低于 `ambient − eps` 即一漏光像素(低于 ambient 的黑色裂缝)。
/// 缓存命中区不参与判定(其低值为合法暗部,非裂缝);相机 miss 同。
pub fn count_leak_pixels(rgb: &[f32], flags: &[f32], ambient: [f32; 3], eps: f32) -> usize {
    let mut count = 0usize;
    for (px, &fl) in flags.iter().enumerate() {
        if fl != FLAG_FALLBACK {
            continue;
        }
        let r = rgb[px * 3];
        let g = rgb[px * 3 + 1];
        let b = rgb[px * 3 + 2];
        let leak = ![r, g, b].iter().all(|v| v.is_finite())
            || r < ambient[0] - eps
            || g < ambient[1] - eps
            || b < ambient[2] - eps;
        if leak {
            count += 1;
        }
    }
    count
}

/// Card 空洞注入(图集覆盖人为挖洞;RED 臂承载):置该 Card 覆盖三角的
/// 逐三角映射为 sentinel(−1.0 f32 打包面)+ 清零该 Card 全部 texel 覆盖——
/// 消费侧该 Card 区域即「缺失覆盖」。返回被挖洞的三角数(>0 佐证注入有效)。
pub fn inject_card_hole(
    tri_to_card_packed: &mut [f32],
    coverage: &mut [f32],
    set: &CardSet,
    card_id: u32,
) -> Result<usize, ScError> {
    let card = set
        .cards
        .iter()
        .find(|c| c.card_id == card_id)
        .ok_or_else(|| ScError::InvalidScene(format!("Card {card_id} 不在集")))?;
    let mut holed = 0usize;
    for (t, m) in tri_to_card_packed.iter_mut().enumerate() {
        if *m == card_id as f32 && card.tris.contains(&(t as u32)) {
            *m = -1.0;
            holed += 1;
        }
    }
    for t in card.texel_base..card.texel_base + card.texel_count() {
        coverage[t as usize] = 0.0;
    }
    if holed == 0 {
        return Err(ScError::InvalidScene(format!(
            "Card {card_id} 空洞注入无效(无三角被挖洞)"
        )));
    }
    Ok(holed)
}

// ---------------------------------------------------------------------------
// 按匹配深度对 M96 golden 的容差带(measured 后冻结,P-09;fail-closed)
// ---------------------------------------------------------------------------

/// 深度带单条目(匹配深度一档:1 / 2 / full bounce)。
#[derive(Debug, Clone, PartialEq)]
pub struct DepthBandEntry {
    /// 匹配深度(bounce 数)。
    pub depth: u32,
    /// 冻结 golden:辐射度缓存产物 digest(capture 输出)。
    pub cache_digest: String,
    /// 冻结 golden:消费渲染产物 digest。
    pub render_digest: String,
    /// 冻结 golden:M96 同深度参照产物 digest。
    pub m96_digest: String,
    /// 冻结容差带(rel_dev 上界 = measured × [`M97_BAND_MARGIN`];禁手写)。
    pub band_rel_dev: f64,
    /// 冻结时实测 rel_dev(surface cache 渲染 vs M96 同深度;provenance)。
    pub measured_rel_dev: f64,
}

/// 深度容差带(`milestones/g9/g9_m97_depth_band.json` 的内存形)。
#[derive(Debug, Clone, PartialEq)]
pub struct DepthBand {
    /// provenance:冻结时刻 UTC。
    pub frozen_at_utc: String,
    /// provenance:device 名。
    pub device_name: String,
    /// 冻结场景名(M96 冻结 fixture)。
    pub scene: String,
    /// 冻结 golden:Card 参数化产物 digest。
    pub cardset_digest: String,
    /// 冻结 golden:图集页产物 digest(RXPL v2 编码字节)。
    pub atlas_page_digest: String,
    /// M96 门序锚:本带 full 档 M96 参照 digest 与 M96 冻结容差带
    /// `m96_cornell_spp64` 条目逐字相等(D2-Q7 门序消费面的机器锚)。
    pub m96_anchor_digest: String,
    /// 逐深度条目(1/2/full)。
    pub entries: Vec<DepthBandEntry>,
}

impl DepthBand {
    /// 查条目(fail-closed:缺条目 = Err)。
    pub fn entry(&self, depth: u32) -> Result<&DepthBandEntry, ScError> {
        self.entries
            .iter()
            .find(|e| e.depth == depth)
            .ok_or_else(|| ScError::DepthBand(format!("容差带缺条目 depth={depth}")))
    }

    /// 比对(fail-closed):三 digest 全等 且 rel_dev ≤ 带;违例逐条列名。
    pub fn check(
        &self,
        depth: u32,
        cache_digest: &str,
        render_digest: &str,
        m96_digest: &str,
        rel_dev: f64,
    ) -> Result<(), ScError> {
        let e = self.entry(depth)?;
        for (name, got, want) in [
            ("cache_digest", cache_digest, e.cache_digest.as_str()),
            ("render_digest", render_digest, e.render_digest.as_str()),
            ("m96_digest", m96_digest, e.m96_digest.as_str()),
        ] {
            if got != want {
                return Err(ScError::DepthBand(format!(
                    "depth={depth} {name} {got} ≠ golden {want}"
                )));
            }
        }
        if rel_dev.is_nan() || rel_dev > e.band_rel_dev {
            return Err(ScError::DepthBand(format!(
                "depth={depth} rel_dev {rel_dev:.6e} 越带(上界 {:.6e})",
                e.band_rel_dev
            )));
        }
        Ok(())
    }

    /// 序列化(手工 JSON;字段序冻结,浮点 `{:e}` 确定性格式)。
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n  \"schema\": \"rurix.g9m97.depth_band.v1\",\n");
        s.push_str(&format!("  \"frozen_at_utc\": \"{}\",\n", self.frozen_at_utc));
        s.push_str(&format!("  \"device_name\": \"{}\",\n", self.device_name));
        s.push_str(&format!("  \"scene\": \"{}\",\n", self.scene));
        s.push_str(&format!(
            "  \"cardset_digest\": \"{}\",\n",
            self.cardset_digest
        ));
        s.push_str(&format!(
            "  \"atlas_page_digest\": \"{}\",\n",
            self.atlas_page_digest
        ));
        s.push_str(&format!(
            "  \"m96_anchor_digest\": \"{}\",\n",
            self.m96_anchor_digest
        ));
        s.push_str(&format!(
            "  \"freeze_rule\": \"band_rel_dev = measured_rel_dev * {:.1}(规则冻结于 gi::surface_cache::M97_BAND_MARGIN;基值 = 冻结批实测,禁手写 P-09)\",\n",
            M97_BAND_MARGIN
        ));
        s.push_str(&format!(
            "  \"depths\": \"{}\",\n",
            M97_DEPTHS
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
        s.push_str(&format!("  \"seed_capture\": \"{}\",\n", M97_SEED));
        s.push_str("  \"entries\": [\n");
        for (i, e) in self.entries.iter().enumerate() {
            s.push_str(&format!(
                "    {{\"depth\": \"{}\", \"cache_digest\": \"{}\", \"render_digest\": \"{}\", \"m96_digest\": \"{}\", \"band_rel_dev\": \"{:e}\", \"measured_rel_dev\": \"{:e}\"}}{}\n",
                e.depth,
                e.cache_digest,
                e.render_digest,
                e.m96_digest,
                e.band_rel_dev,
                e.measured_rel_dev,
                if i + 1 == self.entries.len() { "" } else { "," }
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }

    /// 解析(fail-closed:schema 不符/键缺失/数值非法/条目重复一律 Err)。
    pub fn parse(text: &str) -> Result<DepthBand, ScError> {
        let err = |m: &str| ScError::DepthBand(format!("容差带解析: {m}"));
        if !text.contains("\"schema\": \"rurix.g9m97.depth_band.v1\"") {
            return Err(err("schema 失配"));
        }
        let get_str = |key: &str| -> Result<String, ScError> {
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
            let field = |key: &str| -> Result<String, ScError> {
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
            let depth: u32 = field("depth")?
                .parse()
                .map_err(|_| err("depth 非数值"))?;
            let cache_digest = field("cache_digest")?;
            let render_digest = field("render_digest")?;
            let m96_digest = field("m96_digest")?;
            for (nm, d) in [
                ("cache_digest", &cache_digest),
                ("render_digest", &render_digest),
                ("m96_digest", &m96_digest),
            ] {
                if d.len() != 64 || !d.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(err(&format!("{nm} 非 64-hex")));
                }
            }
            let num = |key: &str| -> Result<f64, ScError> {
                field(key)?
                    .parse()
                    .map_err(|_| err(&format!("{key} 非数值")))
            };
            let band_rel_dev = num("band_rel_dev")?;
            let measured_rel_dev = num("measured_rel_dev")?;
            if !(band_rel_dev > 0.0 && band_rel_dev.is_finite()) {
                return Err(err("band_rel_dev 非正/非有限"));
            }
            if entries.iter().any(|e: &DepthBandEntry| e.depth == depth) {
                return Err(err(&format!("条目重复 depth={depth}")));
            }
            entries.push(DepthBandEntry {
                depth,
                cache_digest,
                render_digest,
                m96_digest,
                band_rel_dev,
                measured_rel_dev,
            });
        }
        if entries.is_empty() {
            return Err(err("entries 为空"));
        }
        Ok(DepthBand {
            frozen_at_utc: get_str("frozen_at_utc")?,
            device_name: get_str("device_name")?,
            scene: get_str("scene")?,
            cardset_digest: get_str("cardset_digest")?,
            atlas_page_digest: get_str("atlas_page_digest")?,
            m96_anchor_digest: get_str("m96_anchor_digest")?,
            entries,
        })
    }
}

// ---------------------------------------------------------------------------
// host oracle(capture 公式面的 host 同源镜像;单测数值锚/算法层佐证——
// 仅 host 输出不能充绿,门绿由 device 腿承载)
// ---------------------------------------------------------------------------

/// 分量积(host oracle 的 throughput×albedo 类运算载体)。
fn cmul(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x * b.x, a.y * b.y, a.z * b.z)
}

/// 单 texel capture(host oracle;与 kernel 公式面同源——投影射线首命中 +
/// M96 NEE/MIS/RR 弹射体逐字同式)。返回 (texel 辐射度 RGB, 覆盖 0/1)。
#[allow(clippy::too_many_arguments)]
fn capture_texel_host<B: crate::rt::bvh::BlasSet + ?Sized>(
    tlas: &Tlas,
    blases: &B,
    scene: &PtScene,
    card: &ScCard,
    texel_local: usize,
    depth: u32,
    samples: u32,
    stream: &[f32],
    texel_index: usize,
) -> ([f32; 3], f32) {
    let res_u = card.res_u as usize;
    let tu = texel_local % res_u;
    let tv = texel_local / res_u;
    let fu = (tu as f32 + 0.5) / card.res_u as f32;
    let fv = (tv as f32 + 0.5) / card.res_v as f32;
    let o = Vec3::from_array(card.origin);
    let au = Vec3::from_array(card.axis_u);
    let av = Vec3::from_array(card.axis_v);
    let cn = Vec3::from_array(card.normal);
    let pp = o + au * (fu * card.size_u) + av * (fv * card.size_v);
    let proj_origin = pp + cn * card.margin;
    let proj_dir = cn * (-1.0);
    let rr_min = m97_rr_min(depth);
    let ln = Vec3::from_array(scene.light.normal());
    let le = Vec3::from_array(scene.light.emission);
    let area = scene.light.area();
    let lp00 = Vec3::from_array(scene.light.p00);
    let le1 = Vec3::from_array(scene.light.e1);
    let le2 = Vec3::from_array(scene.light.e2);
    let mut acc = Vec3::new(0.0, 0.0, 0.0);
    let mut cov = 0.0f32;
    for s in 0..samples as usize {
        let sb = m97_rng::sample_base(texel_index, s, samples, depth);
        let mut origin = proj_origin;
        let mut d = proj_dir;
        let mut thr = Vec3::new(1.0, 1.0, 1.0);
        let mut li = Vec3::new(0.0, 0.0, 0.0);
        let mut prev_pdf = 1.0f32;
        let mut first = true;
        for b in 0..depth as usize {
            let bb = sb + b * m97_rng::DIMS_PER_BOUNCE;
            let ray_tmax = if b == 0 { card.margin * 2.0 } else { scene.t_max };
            let hit = tlas.intersect(blases, &Ray { origin, dir: d });
            let Some(hit) = hit.filter(|h| h.t <= ray_tmax) else {
                if b == 0 {
                    cov = 0.0;
                }
                break; // miss:吸收零态
            };
            if b == 0 {
                cov = 1.0;
            }
            let prim = hit.tri as usize;
            let ng = Vec3::from_array(hit.normal);
            let p = origin + d * hit.t;
            let n = if ng.dot(d) > 0.0 { ng * (-1.0) } else { ng };
            let (albedo, emission) = match &scene.materials[prim] {
                MaterialKind::Lambert { albedo } => (*albedo, [0.0; 3]),
                MaterialKind::Emission { albedo, emission } => (*albedo, *emission),
                _ => ([0.0; 3], [0.0; 3]), // validate 先行;oracle 遇范围外不产路径
            };
            let al = Vec3::from_array(albedo);
            let em = Vec3::from_array(emission);
            // ① BSDF 命中发光面(单面 + MIS w_b;首命中 w=1)。
            let cos_emit = -ng.dot(d);
            if emission.iter().any(|c| *c > 0.0) && cos_emit > 0.0 {
                let w_b = if first {
                    1.0
                } else {
                    mis_weight_bsdf(hit.t, area, cos_emit, prev_pdf)
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
                let nee_core = cos_s * cos_l * area / (path_trace::PT_PI * dist2);
                let w_l = mis_weight_light(cos_s / path_trace::PT_PI, area, cos_l, dist2);
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
            // ③ BSDF 采样(余弦加权半球;ref_tracer 同式)。
            let nd =
                crate::rt::ref_tracer::cosine_sample_hemisphere(n, stream[bb + 2], stream[bb + 3]);
            prev_pdf = cosine_hemisphere_pdf(nd.dot(n));
            thr = cmul(thr, al);
            // ④ RR(b ≥ rr_min 启用;p = max 通道 clamp [0,1])。
            if b as u32 >= rr_min {
                let p_surv = thr.x.max(thr.y).max(thr.z).clamp(0.0, 1.0);
                if stream[bb + 4] > p_surv {
                    break;
                }
                thr = thr * (1.0 / p_surv.max(1e-6));
            }
            origin = p + n * RAY_EPS;
            d = nd;
            first = false;
        }
        acc = acc + li;
    }
    let inv = 1.0 / samples as f32;
    ([acc.x * inv, acc.y * inv, acc.z * inv], cov)
}

/// host oracle 全图集 capture(逐 texel 独立;确定性 = 流 + texel 序 + f32 逐式)。
/// 返回 (radiance[3·total_texels], coverage[total_texels])。
pub fn capture_host(
    scene: &PtScene,
    set: &CardSet,
    depth: u32,
    stream: &[f32],
) -> Result<(Vec<f32>, Vec<f32>), ScError> {
    scene
        .validate()
        .map_err(|e| ScError::InvalidScene(format!("场景校验: {e}")))?;
    if depth == 0 {
        return Err(ScError::InvalidConfig("depth = 0".into()));
    }
    let samples = set.config.samples_per_texel;
    let need = m97_rng::stream_len(set.total_texels as usize, samples, depth);
    if stream.len() != need {
        return Err(ScError::InvalidConfig(format!(
            "RNG 流长 {} ≠ 期望 {need}(texels={} samples={samples} depth={depth})",
            stream.len(),
            set.total_texels
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
    let mut radiance = vec![0.0f32; set.total_texels as usize * 3];
    let mut coverage = vec![0.0f32; set.total_texels as usize];
    let bset: &[TriBvh] = &blases;
    for card in &set.cards {
        for local in 0..card.texel_count() as usize {
            let texel = card.texel_base as usize + local;
            let (v, cov) = capture_texel_host(
                &tlas, bset, scene, card, local, depth, samples, stream, texel,
            );
            radiance[texel * 3] = v[0];
            radiance[texel * 3 + 1] = v[1];
            radiance[texel * 3 + 2] = v[2];
            coverage[texel] = cov;
        }
    }
    Ok((radiance, coverage))
}

/// 直接光辐照度数值积分锚(确定性网格求积;光源 quad g×g 单元中心采样):
/// `E = Le_lum · Σ_cells cos_s·cos_l/dist²·dA_cell`(无遮蔽假设——单测锚用
/// 开放位形)。返回照度标量(单通道;Le 各向同性白)。
pub fn quadrature_irradiance(light: &path_trace::PtLightQuad, p: [f32; 3], n: [f32; 3], grid: u32) -> f64 {
    let lp00 = Vec3::from_array(light.p00);
    let le1 = Vec3::from_array(light.e1);
    let le2 = Vec3::from_array(light.e2);
    let ln = Vec3::from_array(light.normal());
    let pt = Vec3::from_array(p);
    let nn = Vec3::from_array(n);
    let le_lum = f64::from(light.emission[0]);
    let cell_area = f64::from(light.area()) / f64::from(grid * grid);
    let mut acc = 0.0f64;
    for iu in 0..grid {
        for iv in 0..grid {
            let u = (f64::from(iu) + 0.5) / f64::from(grid);
            let v = (f64::from(iv) + 0.5) / f64::from(grid);
            let q = lp00 + le1 * (u as f32) + le2 * (v as f32);
            let wv = q - pt;
            let dist2 = (wv.dot(wv) as f64).max(1e-12);
            let dist = dist2.sqrt();
            let wi = wv * (1.0 / dist as f32);
            let cos_s = f64::from(nn.dot(wi).max(0.0));
            let cos_l = f64::from((-ln.dot(wi)).max(0.0));
            acc += le_lum * cos_s * cos_l / dist2 * cell_area;
        }
    }
    acc
}

// ---------------------------------------------------------------------------
// 单测(RXS-0358 锚定;host 面——参数化确定性/裁剪/上限 fail-closed/32B 镜像
// 跨 crate 锚/图集页 RXPL v2 往返与篡改拒/打包面/漏光检测双臂/空洞注入/
// 容差带比对器 fail-closed/host oracle 数值锚/conformance 锚消费)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试场景:Cornell 冻结 fixture + mesh 划分(与 harness 同源)。
    fn cornell() -> (PtScene, Vec<ScMesh>) {
        let scene = path_trace::m96_cornell_scene();
        let meshes = m97_cornell_meshes();
        (scene, meshes)
    }

    //@ spec: RXS-0358
    #[test]
    fn parameterize_deterministic_and_capped() {
        let (scene, meshes) = cornell();
        let cfg = ScConfig::default();
        let a = parameterize(&scene.positions, &scene.indices, &meshes, &cfg).expect("参数化");
        let b = parameterize(&scene.positions, &scene.indices, &meshes, &cfg).expect("参数化");
        assert_eq!(a, b, "同输入双跑位级一致");
        assert_eq!(a.digest(), b.digest(), "digest 确定性");
        // ≤12/mesh:Cornell 每 mesh ≤ 6 Card(内盒 6 面),全集 12 Card。
        for m in 0..meshes.len() as u32 {
            let n = a.cards.iter().filter(|c| c.mesh_index == m).count();
            assert!(
                n <= cfg.max_cards_per_mesh as usize,
                "mesh {m} Card 数 {n} 超上限"
            );
        }
        assert_eq!(a.cards.len(), 12, "5 墙 × 1 + 盒 6 + 光 1");
        // 分区互斥完备:每三角恰一 Card;tri_to_card 无 sentinel。
        let mut seen = vec![false; scene.indices.len()];
        for c in &a.cards {
            for &t in &c.tris {
                assert!(!seen[t as usize], "三角 {t} 重复覆盖");
                seen[t as usize] = true;
                assert_eq!(a.tri_to_card[t as usize], c.card_id);
            }
        }
        assert!(seen.iter().all(|&v| v), "分区完备");
        // 图集槽位不重叠且全覆盖。
        assert_eq!(
            a.total_texels,
            a.cards.iter().map(|c| c.texel_count()).sum::<u32>()
        );
        for c in &a.cards {
            assert_eq!(c.res_u, 8);
            assert_eq!(c.res_v, 8);
            assert!(c.margin > 0.0);
            assert!(c.size_u > 0.0 && c.size_v > 0.0);
        }
    }

    //@ spec: RXS-0358
    #[test]
    fn parameterize_card_frames_and_projection_exact() {
        let (scene, meshes) = cornell();
        let a = parameterize(&scene.positions, &scene.indices, &meshes, &ScConfig::default())
            .expect("参数化");
        for c in &a.cards {
            let n = Vec3::from_array(c.normal);
            let au = Vec3::from_array(c.axis_u);
            let av = Vec3::from_array(c.axis_v);
            // 正交单位 + 法线精确 ±轴。
            assert!((n.length() - 1.0).abs() < 1e-6);
            assert!(n.dot(au).abs() < 1e-6 && n.dot(av).abs() < 1e-6);
            assert!((n.dot(au.cross(av)).abs() - 1.0).abs() < 1e-6, "面内轴叉积 = ±法线");
            assert!(
                c.normal
                    .iter()
                    .all(|&v| v == 0.0 || v == 1.0 || v == -1.0),
                "精确 ±轴法线"
            );
            // 平面性:覆盖顶点法向坐标逐字一致(Cornell 全面片平面)。
            let o = Vec3::from_array(c.origin);
            let off = o.dot(n);
            for &t in &c.tris {
                for &vi in &scene.indices[t as usize] {
                    let p = Vec3::from_array(scene.positions[vi as usize]);
                    assert!(
                        (p.dot(n) - off).abs() < 1e-5,
                        "Card {} 平面性漂移",
                        c.card_id
                    );
                }
            }
            // 锥 cutoff ≈ 1(全平行;归一化 f32 舍入内)。
            assert!((c.cone_cutoff - 1.0).abs() < 1e-6, "锥 cutoff = {}", c.cone_cutoff);
        }
    }

    //@ spec: RXS-0358
    #[test]
    fn config_fail_closed_and_trim_deterministic() {
        // 配置域 fail-closed:0 / 超硬顶一律 typed Err。
        for bad in [0, M97_HARD_MAX_CARDS_PER_MESH + 1] {
            let cfg = ScConfig {
                max_cards_per_mesh: bad,
                ..ScConfig::default()
            };
            assert!(cfg.validate().is_err(), "上限 {bad} 必拒");
        }
        let cfg = ScConfig {
            card_res: 0,
            ..ScConfig::default()
        };
        assert!(cfg.validate().is_err());
        let cfg = ScConfig {
            samples_per_texel: 0,
            ..ScConfig::default()
        };
        assert!(cfg.validate().is_err());
        // 裁剪:上限 4 < 盒 6 类 ⇒ 盒 mesh 裁到 4 Card,全部三角仍被覆盖,
        // 双跑位级一致(确定性裁剪策略)。
        let (scene, meshes) = cornell();
        let cfg = ScConfig {
            max_cards_per_mesh: 4,
            ..ScConfig::default()
        };
        let a = parameterize(&scene.positions, &scene.indices, &meshes, &cfg).expect("裁剪");
        let b = parameterize(&scene.positions, &scene.indices, &meshes, &cfg).expect("裁剪");
        assert_eq!(a, b, "裁剪确定性");
        let box_cards = a.cards.iter().filter(|c| c.mesh_index == 5).count();
        assert_eq!(box_cards, 4, "盒 mesh 裁到上限 4");
        assert!(
            a.tri_to_card.iter().all(|&c| c != TRI_NO_CARD),
            "裁剪后分区仍完备"
        );
        // digest 随配置漂移(配置进 digest 域)。
        let full = parameterize(
            &scene.positions,
            &scene.indices,
            &meshes,
            &ScConfig::default(),
        )
        .expect("参数化");
        assert_ne!(a.digest(), full.digest());
    }

    //@ spec: RXS-0358
    #[test]
    fn bake_record_abi_matches_logical_v2() {
        // 跨 crate ABI 锚:logical_v2 CLAS_RECORD_SIZE == 本记录尺寸(M94 先例)。
        assert_eq!(
            size_of::<CardBakeRecord>(),
            rurix_geom_pages::logical_v2::CLAS_RECORD_SIZE,
            "Card 烘焙记录 32B ABI 漂移"
        );
        let (scene, meshes) = cornell();
        let a = parameterize(&scene.positions, &scene.indices, &meshes, &ScConfig::default())
            .expect("参数化");
        let card = &a.cards[0];
        let rec = CardBakeRecord::of_card(7, card);
        assert_eq!(rec.triangle_offset, 7);
        assert_eq!(rec.triangle_count, card.tris.len() as u32);
        // 编码字段序:offset(u32 LE) | count(u32 LE) | aabb 6×f32 LE。
        let enc = rec.encode_le();
        assert_eq!(&enc[0..4], &7u32.to_le_bytes());
        assert_eq!(
            &enc[4..8],
            &(card.tris.len() as u32).to_le_bytes()
        );
        assert_eq!(&enc[8..12], &card.aabb_min[0].to_le_bytes());
        assert_eq!(&enc[28..32], &card.aabb_max[2].to_le_bytes());
    }

    //@ spec: RXS-0358
    #[test]
    fn atlas_page_rxpl_v2_roundtrip_and_tamper_reject() {
        let (scene, meshes) = cornell();
        let mut a = parameterize(&scene.positions, &scene.indices, &meshes, &ScConfig::default())
            .expect("参数化");
        let (page, bytes) =
            build_atlas_page(&mut a, &scene.positions, &scene.indices).expect("图集页");
        // 合法 v2 页:major=2 + 冻结解码往返无损(复用 M91 ABI,不私定)。
        assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 2);
        let back = rurix_geom_pages::logical_v2::decode_logical_page_v2(&bytes).expect("解码");
        assert_eq!(back, page, "往返无损");
        assert_eq!(back.base.clusters.len(), a.cards.len());
        // CLAS 段记录 = CardBakeRecord 32B 镜像逐字段一致。
        for (c, e) in back.base.clusters.iter().zip(back.ext.iter()) {
            let rec = CardBakeRecord::of_card(c.triangle_offset, &a.cards[c.cluster_id as usize]);
            assert_eq!(c.triangle_offset, rec.triangle_offset);
            assert_eq!(c.triangle_count, rec.triangle_count);
            assert_eq!(e.aabb_min, rec.aabb_min);
            assert_eq!(e.aabb_max, rec.aabb_max);
        }
        // 篡改 digest 的页 fail-closed 拒(私定/篡改格式 variant → 装配期拒)。
        let mut bad = bytes.clone();
        bad[104] ^= 0x01;
        assert!(rurix_geom_pages::logical_v2::decode_logical_page_v2(&bad).is_err());
        // 非 RXPL 魔数(私定格式)拒。
        let mut priv_fmt = bytes.clone();
        priv_fmt[0..4].copy_from_slice(b"SCAT");
        assert!(matches!(
            rurix_geom_pages::logical_v2::decode_logical_page_v2(&priv_fmt),
            Err(rurix_geom_pages::logical::PageDecodeError::BadMagic)
        ));
        // digest 确定性。
        assert_eq!(atlas_page_digest(&bytes), atlas_page_digest(&bytes));
    }

    //@ spec: RXS-0358
    #[test]
    fn pack_layouts_and_rng_stream() {
        let (scene, meshes) = cornell();
        let a = parameterize(&scene.positions, &scene.indices, &meshes, &ScConfig::default())
            .expect("参数化");
        let cards = pack_cards(&a);
        assert_eq!(cards.len(), a.cards.len() * CARD_STRIDE);
        // 首 Card 字段锚:origin/axis/normal/size/res/texel_base/margin。
        let c0 = &a.cards[0];
        assert_eq!(cards[0], c0.origin[0]);
        assert_eq!(cards[12], c0.size_u);
        assert_eq!(cards[14], 8.0);
        assert_eq!(cards[16], 0.0);
        let tc = pack_texel_card(&a);
        assert_eq!(tc.len(), a.total_texels as usize);
        assert_eq!(tc[0], 0.0);
        assert_eq!(tc[a.total_texels as usize - 1], (a.cards.len() - 1) as f32);
        let t2c = pack_tri_to_card(&a);
        assert_eq!(t2c.len(), scene.indices.len());
        assert!(t2c.iter().all(|&v| v >= 0.0), "全覆盖无 sentinel");
        let params = pack_capture_params(&scene, &a.config, a.total_texels, 2);
        assert_eq!(params.len(), 24);
        assert_eq!(params[2], 2.0);
        assert_eq!(params[3], m97_rr_min(2) as f32);
        let rp = pack_render_params(&scene, true);
        assert_eq!(rp.len(), 24);
        assert_eq!(rp[3], 1.0);
        assert_eq!(rp[4], M97_AMBIENT[0]);
        // RNG:布局公式锚 + 确定性 + 改 seed 分叉。
        assert_eq!(m97_rng::sample_stride(2), 10);
        assert_eq!(m97_rng::stream_len(8, 4, 2), 8 * 4 * 10);
        assert_eq!(m97_rng::sample_base(3, 2, 4, 2), (3 * 4 + 2) * 10);
        let x = m97_rng::generate_stream(4, 2, 2, M97_SEED);
        let y = m97_rng::generate_stream(4, 2, 2, M97_SEED);
        assert_eq!(x, y, "同 seed 流位级一致");
        let z = m97_rng::generate_stream(4, 2, 2, M97_SEED + 1);
        assert_ne!(x, z, "改 seed 流必分叉");
    }

    //@ spec: RXS-0358
    #[test]
    fn leak_detector_both_arms() {
        let amb = M97_AMBIENT;
        let eps = M97_LEAK_EPS;
        // GREEN 臂形态:回退区 = ambient ⇒ 0 漏光。
        let rgb = vec![amb[0], amb[1], amb[2], 0.5, 0.5, 0.5, 0.0, 0.0, 0.0];
        let flags = vec![FLAG_FALLBACK, FLAG_CACHE_HIT, FLAG_CAMERA_MISS];
        assert_eq!(count_leak_pixels(&rgb, &flags, amb, eps), 0);
        // RED 臂形态:回退区低于 ambient(黑色裂缝)⇒ 检出。
        let rgb_bad = vec![0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 0.0, 0.0, 0.0];
        assert_eq!(count_leak_pixels(&rgb_bad, &flags, amb, eps), 1);
        // 命中区低值 = 合法暗部,非漏光。
        let dark_hit = vec![0.0, 0.0, 0.0];
        let f_hit = vec![FLAG_CACHE_HIT];
        assert_eq!(count_leak_pixels(&dark_hit, &f_hit, amb, eps), 0);
        // 非有限回退值 = 漏光。
        let nan_px = vec![f32::NAN, 0.0, 0.0];
        assert_eq!(count_leak_pixels(&nan_px, &flags[..1], amb, eps), 1);
        // 恰在 ambient − eps 边界 = 非漏光(闭区间)。
        let edge = vec![amb[0] - eps, amb[1] - eps, amb[2] - eps];
        assert_eq!(count_leak_pixels(&edge, &flags[..1], amb, eps), 0);
    }

    //@ spec: RXS-0358
    #[test]
    fn hole_injection_marks_coverage() {
        let (scene, meshes) = cornell();
        let a = parameterize(&scene.positions, &scene.indices, &meshes, &ScConfig::default())
            .expect("参数化");
        let mut t2c = pack_tri_to_card(&a);
        let mut cov = vec![1.0f32; a.total_texels as usize];
        // 地板 Card(mesh 0,+y 类;相机可见 + 直接光直射,与 harness 受害 Card 同源)。
        let victim = a
            .cards
            .iter()
            .find(|c| c.mesh_index == 0 && c.normal == [0.0, 1.0, 0.0])
            .expect("地板 Card 在集");
        let holed = inject_card_hole(&mut t2c, &mut cov, &a, victim.card_id).expect("注入");
        assert_eq!(holed, victim.tris.len());
        for &t in &victim.tris {
            assert_eq!(t2c[t as usize], -1.0, "挖洞三角 sentinel");
        }
        for t in victim.texel_base..victim.texel_base + victim.texel_count() {
            assert_eq!(cov[t as usize], 0.0, "挖洞 texel 覆盖清零");
        }
        // 未挖洞区域不受影响(三角 2 = 天花,非受害 Card)。
        assert_eq!(t2c[2], a.tri_to_card[2] as f32);
        // 未知 Card 注入 typed Err。
        assert!(inject_card_hole(&mut t2c, &mut cov, &a, 999).is_err());
    }

    //@ spec: RXS-0358
    #[test]
    fn depth_band_comparator_fail_closed() {
        let entry = DepthBandEntry {
            depth: 2,
            cache_digest: "ab".repeat(32),
            render_digest: "cd".repeat(32),
            m96_digest: "ef".repeat(32),
            band_rel_dev: 0.05,
            measured_rel_dev: 0.025,
        };
        let band = DepthBand {
            frozen_at_utc: "2026-08-12T00:00:00Z".into(),
            device_name: "test".into(),
            scene: "m96_cornell".into(),
            cardset_digest: "11".repeat(32),
            atlas_page_digest: "22".repeat(32),
            m96_anchor_digest: "33".repeat(32),
            entries: vec![entry],
        };
        let text = band.to_json();
        let back = DepthBand::parse(&text).expect("roundtrip");
        assert_eq!(band, back);
        // 正例:三 digest 全等 + 带内 ⇒ Ok。
        back.check(2, &"ab".repeat(32), &"cd".repeat(32), &"ef".repeat(32), 0.049)
            .expect("带内放行");
        // RED:任一 digest 分叉 ⇒ 拒。
        assert!(
            back.check(2, &"00".repeat(32), &"cd".repeat(32), &"ef".repeat(32), 0.01)
                .is_err()
        );
        // RED:越带 ⇒ 拒。
        assert!(
            back.check(2, &"ab".repeat(32), &"cd".repeat(32), &"ef".repeat(32), 0.051)
                .is_err()
        );
        // RED:缺条目 ⇒ 拒(fail-closed 不静默放行)。
        assert!(
            back.check(1, &"ab".repeat(32), &"cd".repeat(32), &"ef".repeat(32), 0.001)
                .is_err()
        );
        // RED:坏 schema / 条目缺键 / 带非正 ⇒ 拒。
        assert!(DepthBand::parse("{\"schema\": \"bogus\"}").is_err());
        let broken = text.replace("cache_digest", "cd_");
        assert!(DepthBand::parse(&broken).is_err());
        let neg = text.replace("\"5e-2\"", "\"-5e-2\"");
        assert!(neg != text, "替换须命中");
        assert!(DepthBand::parse(&neg).is_err());
    }

    //@ spec: RXS-0358
    #[test]
    fn host_oracle_capture_deterministic_and_sane() {
        let (scene, meshes) = cornell();
        let cfg = ScConfig::default();
        let a = parameterize(&scene.positions, &scene.indices, &meshes, &cfg).expect("参数化");
        let stream = m97_rng::generate_stream(a.total_texels as usize, cfg.samples_per_texel, 2, M97_SEED);
        let (r1, c1) = capture_host(&scene, &a, 2, &stream).expect("capture");
        let (r2, c2) = capture_host(&scene, &a, 2, &stream).expect("capture");
        assert_eq!(r1, r2, "host oracle 同 seed 双跑位级一致");
        assert_eq!(c1, c2);
        // 全覆盖(Cornell 闭合 fixture:全部 texel 投影命中)。
        assert!(c1.iter().all(|&v| v == 1.0), "完整图集覆盖");
        // 辐射度有限非负。
        assert!(r1.iter().all(|v| v.is_finite() && *v >= 0.0));
        // 能量 sane:深度 2 ≥ 深度 1 ≥ 0(多反弹只加能量,Lambert 正性)。
        let s1 = m97_rng::generate_stream(a.total_texels as usize, cfg.samples_per_texel, 1, M97_SEED);
        let (r_d1, _) = capture_host(&scene, &a, 1, &s1).expect("capture d1");
        let e1: f64 = r_d1.iter().map(|&v| f64::from(v)).sum();
        let e2: f64 = r1.iter().map(|&v| f64::from(v)).sum();
        assert!(e1 > 0.0 && e2 >= e1, "能量单调 e1={e1} e2={e2}");
    }

    //@ spec: RXS-0358
    #[test]
    fn host_oracle_nee_numeric_anchor() {
        // 数值锚:直接光场景(depth=1)地板中心正对光源 texel 的 capture 均值
        // ≈ (albedo/π)·E_求积(无遮蔽开放位形;5% MC 容差,S=1024 收敛)。
        let scene = path_trace::m96_direct_light_scene();
        let meshes = vec![
            ScMesh {
                name: "floor".into(),
                tri_start: 0,
                tri_count: 2,
            },
            ScMesh {
                name: "light".into(),
                tri_start: 2,
                tri_count: 2,
            },
        ];
        let cfg = ScConfig {
            samples_per_texel: 1024,
            ..ScConfig::default()
        };
        let a = parameterize(&scene.positions, &scene.indices, &meshes, &cfg).expect("参数化");
        let depth = 1;
        let stream =
            m97_rng::generate_stream(a.total_texels as usize, cfg.samples_per_texel, depth, M97_SEED);
        let (r, c) = capture_host(&scene, &a, depth, &stream).expect("capture");
        assert!(c.iter().all(|&v| v == 1.0));
        // 地板 Card 中心 texel(res 8 × 8 的中心 4 texel 之一)。
        let floor = &a.cards[0];
        assert_eq!(floor.normal, [0.0, 1.0, 0.0]);
        let res = floor.res_u as usize;
        let local = (res / 2 - 1) + (res / 2 - 1) * res;
        let texel = floor.texel_base as usize + local;
        let got = f64::from(r[texel * 3]);
        // 解析侧:texel 表面点 + 求积 E;oracle 估计 = albedo/π·E·w_l 均值
        // (w_l ≤ 1 ⇒ got < 解析);锚 = 同号同量级 + 偏差 < 25%(w_l 折算)。
        let o = Vec3::from_array(floor.origin);
        let au = Vec3::from_array(floor.axis_u);
        let av = Vec3::from_array(floor.axis_v);
        let tu = (local % res) as f32 + 0.5;
        let tv = (local / res) as f32 + 0.5;
        let p = o + au * (tu / res as f32 * floor.size_u) + av * (tv / res as f32 * floor.size_v);
        let e_q = quadrature_irradiance(&scene.light, p.to_array(), floor.normal, 64);
        let analytic = f64::from(0.7f32 / path_trace::PT_PI) * e_q;
        assert!(got > 0.0 && analytic > 0.0);
        let dev = (got - analytic).abs() / analytic;
        assert!(
            dev < 0.25,
            "NEE 数值锚:got={got:.6} analytic={analytic:.6} dev={dev:.4}"
        );
    }

    //@ spec: RXS-0358
    #[test]
    fn product_digests_deterministic_and_domain_separated() {
        // 产物 digest 面:确定性 + 域分离(cache/render/页/cardset 四域互异)。
        let rad = vec![0.5f32, 0.25, 0.125, 1.0];
        let cov = vec![1.0f32, 0.0];
        let a = cache_product_digest(&rad, &cov);
        assert_eq!(a, cache_product_digest(&rad, &cov), "digest 确定性");
        let flags = vec![1.0f32, 2.0];
        let r = render_product_digest(&rad, &flags);
        assert_eq!(r, render_product_digest(&rad, &flags));
        assert_ne!(a, r, "域分离:cache ≠ render");
        // 字节序敏感(任一字节漂移 ⇒ digest 漂移)。
        let mut rad2 = rad.clone();
        rad2[0] = 0.5 + f32::EPSILON;
        assert_ne!(a, cache_product_digest(&rad2, &cov));
        // 图集页 digest = 字节直哈希(与 cardset canonical 域互异)。
        let (scene, meshes) = cornell();
        let mut set =
            parameterize(&scene.positions, &scene.indices, &meshes, &ScConfig::default())
                .expect("参数化");
        let (_p, bytes) =
            build_atlas_page(&mut set, &scene.positions, &scene.indices).expect("页");
        assert_ne!(atlas_page_digest(&bytes), set.digest());
    }

    //@ spec: RXS-0358
    #[test]
    fn conformance_anchor_corpus_present() {
        // 消费锚定义务:G9.4 M97 锚定语料在位且锚定本条款。
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/gi");
        let reject = root.join("reject/surface_cache_card_hole_leak.rx");
        let text = std::fs::read_to_string(&reject).expect("锚定语料在位");
        assert!(
            text.contains("//@ spec: RXS-0358"),
            "{} 缺 RXS-0358 锚",
            reject.display()
        );
        // 负例面注释在位(空洞注入/漏光检测/回退 ambient 语义)。
        assert!(text.contains("漏光像素计数"), "reject 语料负例面注释在位");
        assert!(text.contains("ambient"), "回退 ambient 语义注释在位");
    }
}

// ---------------------------------------------------------------------------
// 冻结 fixtures 配套(M96 冻结场景的 mesh 划分;与 harness 消费同源)
// ---------------------------------------------------------------------------

/// M97 消费 M96 冻结 Cornell fixture 的 mesh 划分(7 mesh:5 墙 + 内盒 + 光;
/// 三角范围与 [`path_trace::m96_cornell_scene`] 构造序逐字一致——地板 0..2、
/// 天花 2..4、后墙 4..6、左墙 6..8、右墙 8..10、内盒 10..22、光源 22..24)。
pub fn m97_cornell_meshes() -> Vec<ScMesh> {
    let mk = |name: &str, tri_start: u32, tri_count: u32| ScMesh {
        name: name.into(),
        tri_start,
        tri_count,
    };
    vec![
        mk("floor", 0, 2),
        mk("ceiling", 2, 2),
        mk("back_wall", 4, 2),
        mk("left_wall", 6, 2),
        mk("right_wall", 8, 2),
        mk("inner_box", 10, 12),
        mk("light", 22, 2),
    ]
}
