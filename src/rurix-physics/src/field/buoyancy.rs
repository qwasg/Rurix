//! G9.6 M124 解析浮力模型(spec/physics.md RXS-0376;RFC-0024 §4.D;判据逐字引
//! G9_ACCEPTANCE_MAP §3 M124 行,gate `g9.p1.m124.buoyancy_field_channel`)。
//!
//! 冻结纪律:
//! - **走 Field 通道**:水体区域 = persistent field,解析水面函数 = 场定义层
//!   `AnalyticSurface` 基元(骨架期平面 `z = height`,RXS-0374 L2 求值管线
//!   共用,禁第二套曲面采样);`FieldPhysicsType::Buoyancy` 语义;浮力是 Field
//!   统一抽象的**第二个真实用户**(第一个 = destruction damage)——浮力不得
//!   长成第二套空间影响管线,本模块只产 `FieldCouplingSample`,施加唯一经
//!   [`super::couple::apply_field_impulses`] → `ParticleAdapter::set_force_impulse`
//!   (impulse/force 唯一写口)。
//! - **浮力求值器消费场参数**(RXS-0376 L1):介质密度/线性/角阻力系数为场
//!   定义的一部分(`weight = fluid_density × 重力模`,子节点
//!   `CurveDriven(points=[(0,linear_drag),(1,angular_drag)])` 承载阻力系数),
//!   进 `FieldDef::digest`;场景/输入侧参数(形状闭集、体元尺寸)进
//!   [`BuoyancyBodySpec::digest`],双侧 digest 均进 [`BuoyancySceneInput::digest`]
//!   = capture header `joltc_abi_digest` 槽的输入锚。
//! - **求值语义**(RXS-0376 L2):每 tick 对落入 filter 的 RigidBody 域
//!   `PhysicsParticleRef` 计算 clipped 浸入体积与浸没质心 → 浮力 impulse +
//!   浮力矩 + 线性/角阻力 impulse;**形状支持分层**——首期 convex/primitive
//!   解析 clip(`BuoyancyShape` = Sphere/Box/Capsule 闭集);闭集外形状 =
//!   [`BuoyancyError::UnsupportedShapeOutsideClosedSet`] fail-closed(任意
//!   mesh 走离线预计算 voxelized volume table cooked artifact 通道,见
//!   [`BuoyancyVoxelTable`],不经旁路面)。
//! - **确定性内置**:固定 dt + 解析水面函数,**禁帧率相关插值、禁墙钟相
//!   位**——求值只是 tick 索引/场定义/体规格的纯函数,同输入双跑位级一致;
//!   全部输入/输出进 command journal(主流 capture 面见
//!   [`super::buoyancy_capture`])。
//! - **浮力权威不经 Taichi**(M49 联动维持 defer,RFC-0024 §8 红线 0-byte)。
//!
//! 旁路面(供 RED 臂消费,RXS-0376 L3):[`bypass_set_velocity`] /
//! [`bypass_teleport`] 模拟「不经 Field 通道直接写速度/位置/transform」的
//! 浮力旁路 API——[`reject_buoyancy_bypass`] 一律 fail-closed typed Err,
//! 旁路即门红。

use std::fmt;

use rurix_pkg::sha256::{digest, hex};

use crate::capture::canonical::CaptureError;
use crate::particle_view::{PhysicsParticleRef, rigid_body_ref};
use crate::types::{BodyDesc, BodySemantic, ShapeDesc};
use crate::world::PhysicsWorld;

use super::def::{FieldDef, FieldNodeKind, FieldPhysicsType};
use super::eval::FieldEvaluator;

/// 浮力介质/阻力参数在场定义内的 canonical 载体字面(RED 锚与语料锚
/// 共用;`CurveDriven` 子节点 points 恰为 [(0,linear),(1,angular)] 两元)。
pub const BUOYANCY_DRAG_NODE_ID: &str = "drag";
/// 锚定阻力点序(canonical 面;x=0 → linear,x=1 → angular)。
pub const BUOYANCY_DRAG_POINTS: u32 = 2;

/// M124 域错误(fail-closed 单一出口;harness RED 臂锚字面)。
#[derive(Debug, Clone, PartialEq)]
pub enum BuoyancyError {
    /// 场未携带浮力介质参数(`FieldPhysicsType::Buoyancy` 场缺 drag 子节点
    /// 或参数非法)——场通道未接线。
    FieldChannelMissingParams(String),
    /// 闭集外形状(Sphere/Box/Capsule 解析 clip 闭集之外)未经 voxel table
    /// cooked 通道——fail-closed,不静默退化。
    UnsupportedShapeOutsideClosedSet(String),
    /// 浮力旁路 API 注入(不经 Field 通道直接写速度/位置/transform)——
    /// 旁路即门红。
    BypassApiRejected(String),
    /// 输入非法(NaN/非正密度/非正体元尺寸等)。
    InvalidInput(String),
}

impl fmt::Display for BuoyancyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldChannelMissingParams(s) => write!(f, "FieldChannelMissingParams({s})"),
            Self::UnsupportedShapeOutsideClosedSet(s) => {
                write!(f, "UnsupportedShapeOutsideClosedSet({s})")
            }
            Self::BypassApiRejected(s) => write!(f, "BypassApiRejected({s})"),
            Self::InvalidInput(s) => write!(f, "InvalidInput({s})"),
        }
    }
}

impl std::error::Error for BuoyancyError {}

// ———————————————————— 形状闭集与解析 clip ————————————————————

/// 浮力形状支持分层首期闭集(convex/primitive 解析 clip;RFC-0024 §4.D
/// 「首期 convex/primitive 解析 clip」)。闭集外 = voxel table 通道。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuoyancyShape {
    /// 球(半径 > 0)。
    Sphere { radius: f32 },
    /// 轴对齐箱(半长逐轴 > 0;clip 逐轴分式解析)。
    Box { half_extents: [f32; 3] },
    /// 竖轴胶囊(y 轴;half_height > 0、radius > 0;clip = 截顶圆柱 +
    /// 截角球帽解析)。
    Capsule { half_height: f32, radius: f32 },
}

impl BuoyancyShape {
    /// canonical 名(journal/digest 面唯一合法字面)。
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Sphere { .. } => "sphere",
            Self::Box { .. } => "box",
            Self::Capsule { .. } => "capsule",
        }
    }

    /// 自 `ShapeDesc` 解析(首期 RigidBody 域;`StaticMesh`/`ConvexHull` 等
    /// 闭集外形状 → `UnsupportedShapeOutsideClosedSet`,不静默退化采样)。
    pub fn from_shape_desc(desc: &ShapeDesc) -> Result<Self, BuoyancyError> {
        Ok(match desc {
            ShapeDesc::Sphere { radius } => Self::Sphere { radius: *radius },
            ShapeDesc::Box { half_extents } => Self::Box {
                half_extents: *half_extents,
            },
            ShapeDesc::Capsule {
                half_height,
                radius,
            } => Self::Capsule {
                half_height: *half_height,
                radius: *radius,
            },
            ShapeDesc::ConvexHull { .. } | ShapeDesc::StaticMesh { .. } => {
                return Err(BuoyancyError::UnsupportedShapeOutsideClosedSet(
                    "convex_hull/static_mesh: 任意 mesh 须走 voxelized volume table cooked 通道"
                        .into(),
                ));
            }
        })
    }

    /// 参数校验(NaN/非正尺寸 fail-closed)。
    pub fn validate(&self) -> Result<(), BuoyancyError> {
        let bad = |m: &str| BuoyancyError::InvalidInput(m.into());
        match self {
            Self::Sphere { radius } => {
                if radius.is_nan() || *radius <= 0.0 {
                    return Err(bad("sphere radius"));
                }
            }
            Self::Box { half_extents } => {
                if half_extents.iter().any(|h| h.is_nan() || *h <= 0.0) {
                    return Err(bad("box half_extents"));
                }
            }
            Self::Capsule {
                half_height,
                radius,
            } => {
                if half_height.is_nan() || *half_height <= 0.0 || radius.is_nan() || *radius <= 0.0
                {
                    return Err(bad("capsule dims"));
                }
            }
        }
        Ok(())
    }

    /// 全浸体积(解析;V_total)。
    pub fn volume(&self) -> f32 {
        match *self {
            Self::Sphere { radius } => 4.0 / 3.0 * std::f32::consts::PI * radius * radius * radius,
            Self::Box { half_extents: h } => 8.0 * h[0] * h[1] * h[2],
            Self::Capsule {
                half_height,
                radius,
            } => {
                std::f32::consts::PI * radius * radius * (2.0 * half_height)
                    + 4.0 / 3.0 * std::f32::consts::PI * radius * radius * radius
            }
        }
    }

    /// 特征长度(drag 归一面;canonical 定义 = 各形状主轴半径量级)。
    pub fn characteristic_length(&self) -> f32 {
        match *self {
            Self::Sphere { radius } => radius,
            Self::Box { half_extents: h } => (h[0] + h[1] + h[2]) / 3.0,
            Self::Capsule { radius, .. } => radius,
        }
    }

    /// 解析水面(骨架期平面 `z = height`,与 `FieldNodeKind::AnalyticSurface`
    /// 同一基元语义)对**竖立姿态**的 clipped 浸入体积分式 + 浸没质心相对
    /// 体心偏移(+z 侧朝上;输出域 clamp 至 [0,1])。
    ///
    /// `center_z` = 体心世界 z;`water_z` = 解析水面高度(场消费侧取自场定义
    /// `AnalyticSurface{height}` 参数)。首期为解析 clip 闭集模型——姿态项
    /// 不进本面(细长体/翻滚体 corpus 以零初始角速度直立入水 canonical
    /// 场景承载,角速度演化经浮力矩 + 角阻力项体现);旋转体的姿态相关
    /// clip 归 voxel table 分层(cooked artifact)。
    pub fn submerged_fraction(&self, center_z: f32, water_z: f32) -> (f32, f32) {
        match *self {
            Self::Sphere { radius } => {
                // 球帽解析:cap 高 h = clamp(water_z - (center_z - r), 0, 2r);
                // V_cap = π h²(r - h/3);浸没(底部)球帽质心相对球心 =
                // -3(2r - h)²/(4(3r - h))(自球心朝帽向;底部帽 = 朝下)。
                // 端点:h→0 退化 -r(点帽于下极点),h = 2r 全浸归 0。
                let h = (water_z - (center_z - radius)).clamp(0.0, 2.0 * radius);
                if h <= 0.0 {
                    return (0.0, 0.0);
                }
                if h >= 2.0 * radius {
                    return (1.0, 0.0);
                }
                let v_cap = std::f32::consts::PI * h * h * (radius - h / 3.0);
                let frac = v_cap / self.volume();
                let com_off =
                    -3.0 * (2.0 * radius - h) * (2.0 * radius - h) / (4.0 * (3.0 * radius - h));
                (frac.clamp(0.0, 1.0), com_off)
            }
            Self::Box { half_extents: h } => {
                // 竖立箱逐轴分式解析(浸入只沿 z):frac = clamp((water - bottom)
                // / (2 hz), 0, 1);浸没质心 = 浸入段中点。
                let bottom = center_z - h[2];
                let frac = ((water_z - bottom) / (2.0 * h[2])).clamp(0.0, 1.0);
                let com_off = if frac <= 0.0 {
                    0.0
                } else {
                    // 浸入段 [bottom, water] 中点相对体心。
                    (-h[2] + (frac * 2.0 * h[2]) * 0.5) - 0.0
                };
                (frac, com_off)
            }
            Self::Capsule {
                half_height,
                radius,
            } => {
                // 竖直胶囊(y 轴胶囊直立 ⇒ 长轴沿 z):自下而上 = 下半球帽 +
                // 圆柱段 + 上半球帽三段解析累加。
                let bottom = center_z - (half_height + radius);
                let d = (water_z - bottom).clamp(0.0, 2.0 * (half_height + radius));
                if d <= 0.0 {
                    return (0.0, 0.0);
                }
                if d >= 2.0 * (half_height + radius) {
                    // 全浸精确端点(分段累加的 f32 残差不进 frac=1 语义)。
                    return (1.0, 0.0);
                }
                let total = self.volume();
                let mut vol = 0.0f32;
                let mut moment = 0.0f32; // 对 bottom 的一阶矩
                // 段 1:下半球帽(截角 = d 截于 [0, r])。
                let h1 = d.min(radius);
                if h1 > 0.0 {
                    let v1 = spherical_cap_segment(radius, 0.0, h1);
                    let m1 = spherical_cap_segment_moment(radius, 0.0, h1);
                    vol += v1;
                    moment += m1;
                }
                // 段 2:圆柱段(截于 [r, r + 2·hh])。
                if d > radius {
                    let h2 = (d - radius).min(2.0 * half_height);
                    if h2 > 0.0 {
                        let area = std::f32::consts::PI * radius * radius;
                        let v2 = area * h2;
                        let z_mid = radius + h2 * 0.5;
                        vol += v2;
                        moment += v2 * z_mid;
                    }
                }
                // 段 3:上半球自顶部向下收拢(截于 [r+2hh, 2r+2hh])——
                // 顶帽体积/矩(自上半球球心起算)平移至自 bottom 起算系。
                let top_start = radius + 2.0 * half_height;
                if d > top_start {
                    let h3 = (d - top_start).min(2.0 * radius);
                    let v3 = spherical_cap_from_top(radius, h3);
                    let m3_local = spherical_cap_from_top_moment(radius, h3);
                    vol += v3;
                    moment += v3 * (top_start + radius) + m3_local;
                }
                let frac = (vol / total).clamp(0.0, 1.0);
                let com_off = if vol > 0.0 {
                    moment / vol - (half_height + radius)
                } else {
                    0.0
                };
                (frac, com_off)
            }
        }
    }
}

/// 球自底部起 [z0, z1] 截段体积(球心在原点,底部 = -r;段坐标自底部
/// 起算,z ∈ [0, 2r])。
fn spherical_cap_segment(radius: f32, z0: f32, z1: f32) -> f32 {
    // V(z) = ∫0^z π (r² - (t - r)²) dt = π (r z² - z³/3) 的牛顿-莱布尼茨差。
    let v = |z: f32| std::f32::consts::PI * (radius * z * z - z * z * z / 3.0);
    v(z1) - v(z0)
}

/// 同截段对底部的一阶矩(∫ z · dV)。
fn spherical_cap_segment_moment(radius: f32, z0: f32, z1: f32) -> f32 {
    // M(z) = ∫0^z t · π (r² - (t - r)²) dt = π (2r z³/3 - z⁴/4) 差分。
    let m = |z: f32| std::f32::consts::PI * (2.0 * radius * z * z * z / 3.0 - z * z * z * z / 4.0);
    m(z1) - m(z0)
}

/// 上半球自顶部向下高 h 的球帽体积(上半球 = 球的上半,坐标自球心起)。
fn spherical_cap_from_top(radius: f32, h: f32) -> f32 {
    // 顶帽(全球坐标自底部 [2r - h, 2r] 段)。
    spherical_cap_segment(radius, 2.0 * radius - h, 2.0 * radius)
}

/// 顶帽对上半球中心(= 全球心)的一阶矩(带符号;用于平移合成)。
fn spherical_cap_from_top_moment(radius: f32, h: f32) -> f32 {
    // 段 [2r - h, 2r](自全球底部起算)的一阶矩,减段体积×(自底部到球心
    // 的平移 r)即得相对球心的矩。
    let v = spherical_cap_segment(radius, 2.0 * radius - h, 2.0 * radius);
    let m_bottom = spherical_cap_segment_moment(radius, 2.0 * radius - h, 2.0 * radius);
    m_bottom - v * radius
}

// ———————————————————— 体规格(canonical 场景 fixture 面) ————————————————————

/// 浮力体规格(canonical 场景 fixture 的体侧参数;进 `digest` = 输入锚)。
#[derive(Debug, Clone, PartialEq)]
pub struct BuoyancyBodySpec {
    /// 体稳定 ID(fixture 声明序键)。
    pub body_id: String,
    /// 形状闭集成员(解析 clip)。
    pub shape: BuoyancyShape,
    /// 体密度(kg/m³;> 0 且有限——浮力盈亏由 体密度 vs 场介质密度 决定)。
    pub density: f32,
    /// 初始位置(世界系;零初始角速度 = canonical 姿态约束)。
    pub position: [f32; 3],
    /// 初始线速度(canonical 场景恒零;保留显式字段 = 全输入进 journal)。
    pub initial_velocity: [f32; 3],
    /// 材质(摩擦/恢复;进 BodyDesc,固定步确定性面)。
    pub friction: f32,
    /// 恢复系数。
    pub restitution: f32,
}

impl BuoyancyBodySpec {
    /// 质量(= 密度 × 全浸体积;确定性标量面)。
    pub fn mass(&self) -> f32 {
        self.density * self.shape.volume()
    }

    /// 校验(密度/尺寸/初值有限性;fail-closed)。
    pub fn validate(&self) -> Result<(), BuoyancyError> {
        if self.body_id.is_empty() {
            return Err(BuoyancyError::InvalidInput("empty body_id".into()));
        }
        self.shape.validate()?;
        if self.density.is_nan() || self.density <= 0.0 {
            return Err(BuoyancyError::InvalidInput("density".into()));
        }
        if !self
            .position
            .iter()
            .chain(self.initial_velocity.iter())
            .all(|c| c.is_finite())
        {
            return Err(BuoyancyError::InvalidInput("position/velocity".into()));
        }
        if !self.friction.is_finite() || !self.restitution.is_finite() {
            return Err(BuoyancyError::InvalidInput("friction/restitution".into()));
        }
        Ok(())
    }

    /// 转 `BodyDesc`(世界构建面;动态体、layer 0、禁睡眠——canonical
    /// 场景逐 tick 全活跃,过滤 AWAKE 匹配稳定)。
    pub fn to_body_desc(&self) -> BodyDesc {
        let shape = match self.shape {
            BuoyancyShape::Sphere { radius } => ShapeDesc::Sphere { radius },
            BuoyancyShape::Box { half_extents } => ShapeDesc::Box { half_extents },
            BuoyancyShape::Capsule {
                half_height,
                radius,
            } => ShapeDesc::Capsule {
                half_height,
                radius,
            },
        };
        BodyDesc {
            kind: crate::types::BodyKind::Dynamic,
            shape,
            layer: 0,
            mass_props: crate::types::MassProps {
                mass: self.mass(),
                friction: self.friction,
                restitution: self.restitution,
                allow_sleep: false,
            },
            ccd: false,
            transform: crate::types::PhysicsTransform {
                translation: self.position,
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
        }
    }

    /// canonical JSON(输入锚前像;f32 = 8 位 hex 位表示)。
    pub fn canonical_json(&self) -> String {
        fn hx(v: f32) -> String {
            format!("{:08x}", v.to_bits())
        }
        let mut s = format!("{{\"body_id\":\"{}\"", self.body_id.replace('"', "\\\""));
        s.push_str(&format!(",\"shape\":\"{}\"", self.shape.canonical_name()));
        match self.shape {
            BuoyancyShape::Sphere { radius } => {
                s.push_str(&format!(",\"radius\":{}", hx(radius)));
            }
            BuoyancyShape::Box { half_extents } => {
                s.push_str(&format!(
                    ",\"half_extents\":[{},{},{}]",
                    hx(half_extents[0]),
                    hx(half_extents[1]),
                    hx(half_extents[2])
                ));
            }
            BuoyancyShape::Capsule {
                half_height,
                radius,
            } => {
                s.push_str(&format!(
                    ",\"half_height\":{},\"radius\":{}",
                    hx(half_height),
                    hx(radius)
                ));
            }
        }
        s.push_str(&format!(",\"density\":{}", hx(self.density)));
        s.push_str(&format!(
            ",\"position\":[{},{},{}]",
            hx(self.position[0]),
            hx(self.position[1]),
            hx(self.position[2])
        ));
        s.push_str(&format!(
            ",\"initial_velocity\":[{},{},{}]",
            hx(self.initial_velocity[0]),
            hx(self.initial_velocity[1]),
            hx(self.initial_velocity[2])
        ));
        s.push_str(&format!(
            ",\"friction\":{},\"restitution\":{}",
            hx(self.friction),
            hx(self.restitution)
        ));
        s.push('}');
        s
    }

    /// 体侧 digest(场景输入锚的分量)。
    pub fn digest(&self) -> String {
        hex(&digest(self.canonical_json().as_bytes()))
    }
}

// ———————————————————— voxelized volume table(cooked artifact 面) ————————————————————

/// voxelized volume table(任意 mesh 分层支持的离线预计算 cooked artifact
/// 通道面;**版本化**——schema 版本钉死,未知版本 fail-closed)。本模块只
/// 承载通道的**注册/校验/digest 面**:体元表本体经资产管线 cooked
/// artifact 生成与消费(RFC-0024 §5「buoyancy cooked artifact」事实源行);
/// 运行时不做网格 clip(闭集外形状必须经本通道登记,否则
/// `UnsupportedShapeOutsideClosedSet`)。
pub const VOXEL_TABLE_SCHEMA_ID: &str = "rurix.physics.buoyancy_voxel_table";
/// 当前 schema 版本(版本化纪律,承 RFC-0021 §5.1 共同头)。
pub const VOXEL_TABLE_SCHEMA_VERSION: u32 = 1;

/// voxelized volume table 注册面(cooked artifact 引用 + digest 锚;体元
/// 载荷本体由 cooked 通道承载,本面只钉锚——防止「离线表未走 cooked 通道」
/// 的运行时即兴 clip)。
#[derive(Debug, Clone, PartialEq)]
pub struct BuoyancyVoxelTable {
    /// schema 头。
    pub schema_id: String,
    /// schema 版本。
    pub schema_version: u32,
    /// 表稳定 ID(资产键)。
    pub table_id: String,
    /// 体元尺寸(m;> 0 且有限)。
    pub voxel_size: f32,
    /// 体元数(cooked 载荷规模锚)。
    pub voxel_count: u32,
    /// cooked 载荷 digest(资产管线产出锚;本面不承载体元字节)。
    pub payload_digest: String,
}

impl BuoyancyVoxelTable {
    /// 校验 + 版本钉死(未知版本 fail-closed,显式迁移纪律)。
    pub fn validate(&self) -> Result<(), BuoyancyError> {
        if self.schema_id != VOXEL_TABLE_SCHEMA_ID {
            return Err(BuoyancyError::InvalidInput(format!(
                "voxel table schema id {}",
                self.schema_id
            )));
        }
        if self.schema_version != VOXEL_TABLE_SCHEMA_VERSION {
            return Err(BuoyancyError::InvalidInput(format!(
                "voxel table schema version {} (explicit migration required)",
                self.schema_version
            )));
        }
        if self.table_id.is_empty()
            || self.payload_digest.len() != 64
            || self.voxel_size.is_nan()
            || self.voxel_size <= 0.0
            || self.voxel_count == 0
        {
            return Err(BuoyancyError::InvalidInput("voxel table fields".into()));
        }
        Ok(())
    }

    /// canonical JSON + digest(版本化锚)。
    pub fn digest(&self) -> String {
        let s = format!(
            "{{\"schema_id\":\"{}\",\"schema_version\":{},\"table_id\":\"{}\",\"voxel_size\":\"{:08x}\",\"voxel_count\":{},\"payload_digest\":\"{}\"}}",
            self.schema_id,
            self.schema_version,
            self.table_id,
            self.voxel_size.to_bits(),
            self.voxel_count,
            self.payload_digest
        );
        hex(&digest(s.as_bytes()))
    }
}

// ———————————————————— 场参数消费面(介质参数 = 场定义的一部分) ————————————————————

/// 浮力介质参数(自场定义消费;RXS-0376 L1「浮力求值器消费场密度/速度
/// 参数——水面函数与介质参数为场定义的一部分,进 digest」)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuoyancyMedium {
    /// 解析水面高度(场根 `AnalyticSurface{height}` 参数;米)。
    pub water_height: f32,
    /// 介质密度(kg/m³;> 0)。canonical 承载 = 场根 `weight` × 世界重力模
    /// (weight = fluid_density × |g|,由 [`crate::field::buoyancy_capture`]
    /// 场景构造面钉死——同一重力模复算,双跑位级一致)。
    pub fluid_density: f32,
    /// 线性阻力系数(场 `CurveDriven` 子节点 point x=0 的 y 值)。
    pub linear_drag: f32,
    /// 角阻力系数(场 `CurveDriven` 子节点 point x=1 的 y 值)。
    pub angular_drag: f32,
}

/// 场定义 → 浮力介质参数(fail-closed:非 Buoyancy 语义 / 非解析水面基元 /
/// 缺阻力子节点 / 参数非法 ⇒ `FieldChannelMissingParams`,不静默退化为
/// 视觉-only 成功——RXS-0376 Implementation Requirements 冻结句)。
pub fn medium_from_field(
    def: &FieldDef,
    gravity_magnitude: f32,
) -> Result<BuoyancyMedium, BuoyancyError> {
    let missing = |m: &str| BuoyancyError::FieldChannelMissingParams(m.into());
    if def.physics_type != FieldPhysicsType::Buoyancy {
        return Err(missing("physics_type != Buoyancy"));
    }
    let water_height = match def.root.kind {
        FieldNodeKind::AnalyticSurface { height } => height,
        _ => return Err(missing("root kind != AnalyticSurface(解析水面函数基元)")),
    };
    if !gravity_magnitude.is_finite() || gravity_magnitude <= 0.0 {
        return Err(BuoyancyError::InvalidInput("gravity_magnitude".into()));
    }
    if def.root.weight.is_nan() || def.root.weight <= 0.0 {
        return Err(missing("root weight = fluid_density × |g| 非正"));
    }
    let fluid_density = def.root.weight / gravity_magnitude;
    // 阻力子节点:恰一个 CurveDriven、points 恰 [(0,linear),(1,angular)]。
    if def.root.children.len() != 1 {
        return Err(missing("drag CurveDriven 子节点缺失(恰一)"));
    }
    let child = &def.root.children[0];
    let FieldNodeKind::CurveDriven { points, .. } = &child.kind else {
        return Err(missing("drag 子节点非 CurveDriven"));
    };
    if points.len() != BUOYANCY_DRAG_POINTS as usize || points[0].0 != 0.0 || points[1].0 != 1.0 {
        return Err(missing(
            "drag points 非 [(0,linear),(1,angular)] canonical 形",
        ));
    }
    let (linear_drag, angular_drag) = (points[0].1, points[1].1);
    if !fluid_density.is_finite()
        || !linear_drag.is_finite()
        || !angular_drag.is_finite()
        || fluid_density <= 0.0
        || linear_drag < 0.0
        || angular_drag < 0.0
    {
        return Err(missing("介质参数非法(密度非正/阻力负/NaN)"));
    }
    Ok(BuoyancyMedium {
        water_height,
        fluid_density,
        linear_drag,
        angular_drag,
    })
}

// ———————————————————— 浮力求值器(走 Field 通道;RXS-0376 L2) ————————————————————

/// 单 tick 单体浮力求值输出(impulse 载荷面;经 Field 耦合面施加进求解器
/// 主流——本结构不触世界)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuoyancyImpulse {
    /// 浮力 impulse(竖直向上;浸入体积 × 介质密度 × g × dt)。
    pub buoyancy_impulse: [f32; 3],
    /// 浮力矩(浸没质心偏离体心产生的恢复力矩 impulse 等价量)。
    pub buoyancy_torque: [f32; 3],
    /// 线性阻力 impulse(-c_l · v_submerged · dt,按浸没分式加权)。
    pub linear_drag_impulse: [f32; 3],
    /// 角阻力 impulse(-c_a · ω · dt,按浸没分式加权)。
    pub angular_drag_impulse: [f32; 3],
    /// 浸入分式(记账/diagnostic 面)。
    pub submerged_fraction: f32,
    /// 浸没质心相对体心偏移(z 向;记账面)。
    pub com_offset: f32,
}

impl BuoyancyImpulse {
    /// 合成线性 impulse(浮力 + 线性阻力;耦合面 `ImpulseWrite::Linear`
    /// 唯一写口的载荷)。
    pub fn net_linear(&self) -> [f32; 3] {
        [
            self.buoyancy_impulse[0] + self.linear_drag_impulse[0],
            self.buoyancy_impulse[1] + self.linear_drag_impulse[1],
            self.buoyancy_impulse[2] + self.linear_drag_impulse[2],
        ]
    }

    /// 合成角 impulse(浮力矩 + 角阻力;**记账面**——M121 写路径仅线
    /// impulse/force,角冲量口不在 RFC-0017 §4.A 冻结 API 内(couple.rs
    /// 既有诚实登记同口径);角分量进 journal 参与逐位对拍,施加面不接线)。
    pub fn net_angular(&self) -> [f32; 3] {
        [
            self.buoyancy_torque[0] + self.angular_drag_impulse[0],
            self.buoyancy_torque[1] + self.angular_drag_impulse[1],
            self.buoyancy_torque[2] + self.angular_drag_impulse[2],
        ]
    }

    /// 是否零贡献(零浸没 + 零阻力 → 不施加不记账,退化基线逐位一致面)。
    pub fn is_zero(&self) -> bool {
        self.net_linear() == [0.0; 3] && self.net_angular() == [0.0; 3]
    }

    /// canonical 文本(journal 行字面;f32 位表示)。
    pub fn canonical_text(&self) -> String {
        fn hx(v: f32) -> String {
            format!("{:08x}", v.to_bits())
        }
        let lin = self.net_linear();
        let ang = self.net_angular();
        format!(
            "buoyancy:lin=[{},{},{}]:ang=[{},{},{}]:frac={}:com={}",
            hx(lin[0]),
            hx(lin[1]),
            hx(lin[2]),
            hx(ang[0]),
            hx(ang[1]),
            hx(ang[2]),
            hx(self.submerged_fraction),
            hx(self.com_offset)
        )
    }
}

/// 浮力求值器(无状态;消费 `FieldEvaluator` 同一求值实例语义——水面采样
/// 与场梯度同管线,禁第二套曲面采样)。求值 = 纯函数:输入 = 场定义(介质
/// 参数+水面)+ 体规格 + 运行时状态(位置/线速度/角速度快照)+ 固定 dt;
/// **禁帧率相关插值、禁墙钟相位**(本面无任何时钟/帧率输入)。
#[derive(Debug, Default, Clone, Copy)]
pub struct BuoyancyEvaluator {
    /// 场求值器(单一源;水面采样经 `AnalyticSurface` 基元同一面)。
    pub field_evaluator: FieldEvaluator,
}

impl BuoyancyEvaluator {
    /// 构造(无状态实例语义唯一)。
    pub fn new() -> Self {
        Self {
            field_evaluator: FieldEvaluator::new(),
        }
    }

    /// 单场单体单 tick 求值。
    ///
    /// `def` = 浮力场(`FieldPhysicsType::Buoyancy` + `AnalyticSurface` 根);
    /// `gravity` = 世界重力(向量;向上轴 = 重力反方向,z 主轴);`dt` = 固定
    /// 步长(锁死面由调用方/world 层执行);`state` = 体运行时快照。
    pub fn evaluate(
        &self,
        def: &FieldDef,
        medium: &BuoyancyMedium,
        spec: &BuoyancyBodySpec,
        state: &BodySemantic,
        gravity: [f32; 3],
        dt: f32,
    ) -> Result<BuoyancyImpulse, BuoyancyError> {
        // 水面采样经场求值器同一管线(消费锚:sample 面在场内 0/1;介质
        // 参数已在 medium_from_field 期还原并校验)。
        let center = state.transform.translation;
        let inside = self.field_evaluator.sample(def, center);
        let _ = inside; // 采样面消费留痕(0/1 阶跃;clip 分式为解析细粒度面)。
        let (frac, com_off) = spec
            .shape
            .submerged_fraction(center[2], medium.water_height);
        if frac <= 0.0 {
            return Ok(BuoyancyImpulse {
                buoyancy_impulse: [0.0; 3],
                buoyancy_torque: [0.0; 3],
                linear_drag_impulse: [0.0; 3],
                angular_drag_impulse: [0.0; 3],
                submerged_fraction: 0.0,
                com_offset: 0.0,
            });
        }
        let g_mag =
            (gravity[0] * gravity[0] + gravity[1] * gravity[1] + gravity[2] * gravity[2]).sqrt();
        let volume = spec.shape.volume();
        // 浮力 impulse = ρ_f · V_sub · |g| · dt,方向 = 重力反方向(竖直向上)
        // ——场 occupancy 梯度指向水下(浸没度递增向),浮力取梯度**反向**(解
        // 析水面平面法线向上;经场求值器梯度面消费锚定,禁第二套曲面采样)。
        let grad = self.field_evaluator.gradient(def, center);
        let up_dir = if grad == [0.0; 3] {
            // 深水/全浸内部梯度退化(阶跃基元数值梯度在内部为零)——方向锚 =
            // 重力反方向主轴(解析水面平面语义,确定性决胜规则)。
            [0.0, 0.0, 1.0]
        } else {
            let len = (grad[0] * grad[0] + grad[1] * grad[1] + grad[2] * grad[2]).sqrt();
            [-grad[0] / len, -grad[1] / len, -grad[2] / len]
        };
        let buoy_mag = medium.fluid_density * volume * frac * g_mag * dt;
        let buoyancy_impulse = [
            up_dir[0] * buoy_mag,
            up_dir[1] * buoy_mag,
            up_dir[2] * buoy_mag,
        ];
        // 浮力矩:浸没质心偏离体心 ⇒ 恢复力矩 τ = r × F(浸没质心相对体心
        // z 偏移 com_off;竖立姿态下 r = [0,0,com_off],F 竖直 ⇒ τ 水平分量为
        // 零,角速度演化的恢复项主要来自姿态偏离——首期记账面以 com_off ×
        // buoy_mag 标量进 x/y 轴零面,姿态项归 voxel/完整姿态 clip 分层)。
        // 确定性优先:本项在直立 canonical 姿态下解析为零,经角阻力面体现
        // 角通道消费;非零姿态的力矩 clip 归离线 voxel table 分层(形状支持
        // 分层判据字面)。
        let buoyancy_torque = [0.0f32; 3];
        // 线性/角阻力 impulse:浸没分式加权,-c·v·dt 形态;阻力系数为场参
        // 数(介质定义面),速度为体运行时快照(全部输入进 journal 前提)。
        let lin = state.linvel;
        let ang = state.angvel;
        let cl = medium.linear_drag * frac * dt;
        let ca = medium.angular_drag * frac * dt;
        let linear_drag_impulse = [-cl * lin[0], -cl * lin[1], -cl * lin[2]];
        let angular_drag_impulse = [-ca * ang[0], -ca * ang[1], -ca * ang[2]];
        Ok(BuoyancyImpulse {
            buoyancy_impulse,
            buoyancy_torque,
            linear_drag_impulse,
            angular_drag_impulse,
            submerged_fraction: frac,
            com_offset: com_off,
        })
    }
}

// ———————————————————— 旁路 RED 面(RXS-0376 L3) ————————————————————

/// 旁路 API 语义标记(harness RED 臂与静态审计共用锚字面)。
pub const BYPASS_SET_VELOCITY_LITERAL: &str = "buoyancy_set_velocity";
/// 旁路 teleport 字面。
pub const BYPASS_TELEPORT_LITERAL: &str = "buoyancy_teleport";

/// 浮力旁路 API 注入模拟(**不经 Field 通道直接写速度**的负例探针面;
/// 本函数是「旁路若存在会长什么样」的显式标本,harness RED 臂以
/// [`reject_buoyancy_bypass`] 判红消费——任何真实浮力路径不得调用本面)。
pub fn bypass_set_velocity() -> &'static str {
    BYPASS_SET_VELOCITY_LITERAL
}

/// 浮力旁路 teleport 注入模拟(直接写位置/transform 的负例标本面)。
pub fn bypass_teleport() -> &'static str {
    BYPASS_TELEPORT_LITERAL
}

/// 旁路判定(fail-closed typed Err;旁路即门红——本判定是 M124 RED 臂的
/// 单一出口,旁路字面注入一律 `BypassApiRejected`)。
pub fn reject_buoyancy_bypass(api_name: &str) -> Result<(), BuoyancyError> {
    Err(BuoyancyError::BypassApiRejected(format!(
        "{api_name}: 浮力必须走 Field 通道(persistent field + FieldPhysicsType::Buoyancy),\
         不经 Field 通道直接写速度/位置/transform 的浮力旁路 API 注入即 RED"
    )))
}

// ———————————————————— 场景输入锚 ————————————————————

/// 浮力 canonical 场景输入(fixture 全量参数;digest = capture header 输入
/// 锚 + corpus fixture digest)。
#[derive(Debug, Clone, PartialEq)]
pub struct BuoyancySceneInput {
    /// 场景 ID(canonical 名;corpus 目录名同字面)。
    pub scenario_id: String,
    /// tick 数。
    pub ticks: u64,
    /// 固定 dt(锁死面;world desc 同源)。
    pub dt_fixed: f32,
    /// 世界重力向量。
    pub gravity: [f32; 3],
    /// 浮力场定义(persistent + Buoyancy 语义;介质参数进 def digest)。
    pub field: FieldDef,
    /// 体规格集(canonical 序 = 声明序)。
    pub bodies: Vec<BuoyancyBodySpec>,
}

impl BuoyancySceneInput {
    /// 校验(场介质参数可还原 + 体规格全合法 + 场景非空;fail-closed)。
    pub fn validate(&self) -> Result<(), BuoyancyError> {
        if self.scenario_id.is_empty() || self.ticks < 2 {
            return Err(BuoyancyError::InvalidInput("scenario/ticks".into()));
        }
        if !self.dt_fixed.is_finite() || self.dt_fixed <= 0.0 {
            return Err(BuoyancyError::InvalidInput("dt_fixed".into()));
        }
        if !self.gravity.iter().all(|g| g.is_finite()) {
            return Err(BuoyancyError::InvalidInput("gravity".into()));
        }
        let g_mag = (self.gravity[0] * self.gravity[0]
            + self.gravity[1] * self.gravity[1]
            + self.gravity[2] * self.gravity[2])
            .sqrt();
        medium_from_field(&self.field, g_mag)?;
        if self.bodies.is_empty() {
            return Err(BuoyancyError::InvalidInput("empty bodies".into()));
        }
        for b in &self.bodies {
            b.validate()?;
        }
        Ok(())
    }

    /// 场景输入 digest(场 def digest × 体 digest × 标量参数合成;capture
    /// header `joltc_abi_digest` 槽的输入锚)。
    pub fn digest(&self) -> String {
        let mut buf = format!(
            "scenario:{}:ticks:{}:dt={:08x}:g=[{:08x},{:08x},{:08x}]\n",
            self.scenario_id,
            self.ticks,
            self.dt_fixed.to_bits(),
            self.gravity[0].to_bits(),
            self.gravity[1].to_bits(),
            self.gravity[2].to_bits()
        );
        buf.push_str(&format!("field_digest:{}\n", self.field.digest()));
        for b in &self.bodies {
            buf.push_str(&format!("body:{}:{}\n", b.body_id, b.digest()));
        }
        hex(&digest(buf.as_bytes()))
    }
}

/// 世界快照 → 浮力求值粒子集(RigidBody 域;录制/replay 同一重建面)。
pub fn scenario_body_states(world: &PhysicsWorld) -> Result<Vec<BodySemantic>, CaptureError> {
    world
        .body_semantic_snapshot()
        .map_err(|e| CaptureError::Backend(e.to_string()))
}

/// `BodySemantic` → `PhysicsParticleRef`(RigidBody 域唯一构造口面)。
pub fn particle_of(sem: &BodySemantic) -> PhysicsParticleRef {
    rigid_body_ref(sem.body_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::def::FieldNode;
    use crate::field::filter::{FieldFilter, domain_bit, object_state_bits};
    use crate::field::lifecycle::FieldLifecycle;
    use crate::particle_view::ParticleDomain;

    pub(crate) fn sample_buoyancy_field(weight: f32, height: f32) -> FieldDef {
        FieldDef::new(
            "water",
            FieldNode {
                node_id: "water_root".into(),
                kind: FieldNodeKind::AnalyticSurface { height },
                weight,
                children: vec![FieldNode {
                    node_id: BUOYANCY_DRAG_NODE_ID.into(),
                    kind: FieldNodeKind::CurveDriven {
                        points: vec![(0.0, 0.9), (1.0, 0.6)],
                        anchor: [0.0; 3],
                    },
                    weight: 1.0,
                    children: vec![],
                }],
            },
            FieldPhysicsType::Buoyancy,
            FieldLifecycle::Persistent,
            FieldFilter {
                object_state_mask: object_state_bits::AWAKE,
                domain_mask: domain_bit(ParticleDomain::RigidBody),
                layer_mask: 1,
                explicit_include: vec![],
                explicit_exclude: vec![],
            },
        )
    }

    //@ spec: RXS-0376
    #[test]
    fn analytic_clip_known_values_and_closed_set() {
        // 球半浸:水线过球心 → frac = 0.5,质心偏移 = -3r/8(球帽公式已知值)。
        let s = BuoyancyShape::Sphere { radius: 1.0 };
        let (frac, com) = s.submerged_fraction(0.0, 0.0);
        assert!((frac - 0.5).abs() < 1e-6, "半球 frac={frac}");
        assert!((com - (-0.375)).abs() < 1e-6, "半球质心 com={com}");
        // 全浸/零浸端点。
        assert_eq!(s.submerged_fraction(10.0, 0.0).0, 0.0);
        assert_eq!(s.submerged_fraction(-10.0, 0.0).0, 1.0);
        // 箱:线性分式。
        let b = BuoyancyShape::Box {
            half_extents: [1.0, 1.0, 1.0],
        };
        assert_eq!(b.submerged_fraction(0.5, 0.0).0, 0.25);
        assert_eq!(b.submerged_fraction(0.5, 0.0).1, -0.75);
        // 胶囊:部分浸没单调递增(自底向上三段解析)。
        let c = BuoyancyShape::Capsule {
            half_height: 1.0,
            radius: 0.5,
        };
        let f0 = c.submerged_fraction(4.0, 0.0).0;
        let f1 = c.submerged_fraction(0.0, -0.5).0;
        let f2 = c.submerged_fraction(0.0, 0.5).0;
        assert!(f0 == 0.0 && f1 > 0.0 && f2 > f1 && f2 < 1.0);
        // 同输入双跑位级一致(确定性面)。
        let p = (0.3, 0.7);
        assert_eq!(
            s.submerged_fraction(p.0, p.1),
            s.submerged_fraction(p.0, p.1)
        );
        // 闭集外形状 fail-closed(不静默退化采样)。
        assert!(matches!(
            BuoyancyShape::from_shape_desc(&ShapeDesc::ConvexHull {
                points: vec![[0.0; 3]; 4]
            }),
            Err(BuoyancyError::UnsupportedShapeOutsideClosedSet(_))
        ));
    }

    //@ spec: RXS-0376
    #[test]
    fn medium_from_field_channel_params_and_fail_closed_faces() {
        // weight = ρ·|g| = 1000 × 9.81。
        let def = sample_buoyancy_field(9810.0, 0.5);
        let m = medium_from_field(&def, 9.81).expect("medium");
        assert!((m.fluid_density - 1000.0).abs() < 1e-2);
        assert_eq!(m.water_height, 0.5);
        assert_eq!(m.linear_drag, 0.9);
        assert_eq!(m.angular_drag, 0.6);
        // 非 Buoyancy 语义 / 缺阻力子节点 / 非解析水面根 全 fail-closed。
        let mut bad = def.clone();
        bad.physics_type = FieldPhysicsType::LinearForce;
        assert!(matches!(
            medium_from_field(&bad, 9.81),
            Err(BuoyancyError::FieldChannelMissingParams(_))
        ));
        let mut bad2 = def.clone();
        bad2.root.children.clear();
        assert!(medium_from_field(&bad2, 9.81).is_err());
        let mut bad3 = def.clone();
        bad3.root.kind = FieldNodeKind::Sphere {
            center: [0.0; 3],
            radius: 1.0,
        };
        assert!(medium_from_field(&bad3, 9.81).is_err());
        // 介质参数进 def digest(改阻力值 digest 必改)。
        let mut def2 = def.clone();
        def2.root.children[0].kind = FieldNodeKind::CurveDriven {
            points: vec![(0.0, 1.9), (1.0, 0.6)],
            anchor: [0.0; 3],
        };
        assert_ne!(
            def.digest(),
            def2.digest(),
            "介质参数为场定义一部分进 digest"
        );
    }

    //@ spec: RXS-0376
    #[test]
    fn evaluator_deterministic_buoyancy_and_drag_signs() {
        let def = sample_buoyancy_field(9810.0, 0.0);
        let m = medium_from_field(&def, 9.81).unwrap();
        let ev = BuoyancyEvaluator::new();
        let spec = BuoyancyBodySpec {
            body_id: "b".into(),
            shape: BuoyancyShape::Sphere { radius: 0.5 },
            density: 500.0,
            position: [0.0; 3],
            initial_velocity: [0.0; 3],
            friction: 0.5,
            restitution: 0.0,
        };
        spec.validate().unwrap();
        let state = BodySemantic {
            body_id: crate::id::BodyId::new(0, 1),
            kind: crate::types::BodyKind::Dynamic,
            is_active: true,
            layer: 0,
            shape_id: crate::id::ShapeId::new(0, 1),
            transform: crate::types::PhysicsTransform {
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            linvel: [0.0, 0.0, -1.0],
            angvel: [0.0, 0.5, 0.0],
        };
        let dt = 1.0 / 60.0;
        let a = ev
            .evaluate(&def, &m, &spec, &state, [0.0, 0.0, -9.81], dt)
            .expect("eval");
        let b = ev
            .evaluate(&def, &m, &spec, &state, [0.0, 0.0, -9.81], dt)
            .expect("eval");
        assert_eq!(a, b, "同输入双跑位级一致");
        // 半浸球:浮力向上(z+),线性阻力与速度反向(下沉 ⇒ 阻力 +z),
        // 角阻力与角速度反向。
        assert!(a.buoyancy_impulse[2] > 0.0);
        assert!(a.linear_drag_impulse[2] > 0.0, "下沉阻力向上");
        assert!(a.angular_drag_impulse[1] < 0.0, "角阻力反向");
        assert!(a.submerged_fraction > 0.49 && a.submerged_fraction < 0.51);
        // 完全离水 → 零贡献。
        let mut out = state.clone();
        out.transform.translation = [0.0, 0.0, 10.0];
        let dry = ev
            .evaluate(&def, &m, &spec, &out, [0.0, 0.0, -9.81], dt)
            .unwrap();
        assert!(dry.is_zero(), "离水零 impulse(退化基线面)");
    }

    //@ spec: RXS-0376
    #[test]
    fn bypass_api_rejected_typed_err_single_literal() {
        for api in [bypass_set_velocity(), bypass_teleport()] {
            let e = reject_buoyancy_bypass(api).unwrap_err();
            assert!(
                matches!(e, BuoyancyError::BypassApiRejected(_)),
                "旁路一律 typed Err"
            );
            assert!(e.to_string().contains("BypassApiRejected"));
        }
    }

    //@ spec: RXS-0376
    #[test]
    fn voxel_table_versioned_fail_closed_and_scene_input_digest() {
        let t = BuoyancyVoxelTable {
            schema_id: VOXEL_TABLE_SCHEMA_ID.into(),
            schema_version: VOXEL_TABLE_SCHEMA_VERSION,
            table_id: "t0".into(),
            voxel_size: 0.05,
            voxel_count: 4096,
            payload_digest: "ab".repeat(32),
        };
        t.validate().unwrap();
        let mut bad = t.clone();
        bad.schema_version = 99;
        assert!(bad.validate().is_err(), "未知版本 fail-closed");
        assert_eq!(t.digest(), t.digest());

        let scene = BuoyancySceneInput {
            scenario_id: "s".into(),
            ticks: 8,
            dt_fixed: 1.0 / 60.0,
            gravity: [0.0, 0.0, -9.81],
            field: sample_buoyancy_field(9810.0, 0.0),
            bodies: vec![BuoyancyBodySpec {
                body_id: "b0".into(),
                shape: BuoyancyShape::Box {
                    half_extents: [0.5; 3],
                },
                density: 400.0,
                position: [0.0, 0.0, 0.2],
                initial_velocity: [0.0; 3],
                friction: 0.5,
                restitution: 0.0,
            }],
        };
        scene.validate().unwrap();
        assert_eq!(scene.digest(), scene.digest(), "输入锚确定性");
        let mut scene2 = scene.clone();
        scene2.bodies[0].density = 401.0;
        assert_ne!(scene.digest(), scene2.digest(), "体参数进输入锚");
    }
}
