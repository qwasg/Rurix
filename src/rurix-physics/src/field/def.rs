//! 场定义层(Field Nodes)+ 目标语义层八枚举(RFC-0024 §4.B1)。
//!
//! 场 = 空间标量/向量函数 + 元数据;节点图组合,图 schema 版本化 +
//! canonical 序列化 + cook 确定性(承 RFC-0021 §5.1 共同头纪律)。
//! 首期不接通用可视化 node graph 编辑器。

use std::fmt;

use rurix_pkg::sha256::{digest, hex};

use super::filter::FieldFilter;
use super::lifecycle::FieldLifecycle;

/// Field schema 冻结 ID/版本(骨架期 v1;非法版本 fail-closed)。
pub const FIELD_SCHEMA_ID: &str = "rurix.physics.field";
pub const FIELD_SCHEMA_VERSION: u32 = 1;

/// 目标语义层首期八枚举(RFC-0024 §4.B1 冻结;Buoyancy = RuriX 加性扩展,
/// M124 共用求值管线;扩枚举须先经 destruction damage + 浮力两个真实用户)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum FieldPhysicsType {
    /// 线性力(沿场梯度/向量)。
    LinearForce = 0,
    /// 应变(anchor/connection strain 预置)。
    Strain = 1,
    /// 速度(直接速度目标)。
    Velocity = 2,
    /// 力矩。
    Torque = 3,
    /// 睡眠控制。
    Sleeping = 4,
    /// 禁用(域级 disable)。
    Disabled = 5,
    /// 碰撞组改写。
    CollisionGroup = 6,
    /// 浮力(RuriX 加性扩展;M124 解析浮力共用管线)。
    Buoyancy = 7,
}

impl FieldPhysicsType {
    /// 冻结八项(canonical 序 = 声明序;门脚本逐项 accept 的枚举源)。
    pub const ALL: [FieldPhysicsType; 8] = [
        Self::LinearForce,
        Self::Strain,
        Self::Velocity,
        Self::Torque,
        Self::Sleeping,
        Self::Disabled,
        Self::CollisionGroup,
        Self::Buoyancy,
    ];

    /// canonical 名(schema/journal 面唯一合法字面)。
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::LinearForce => "LinearForce",
            Self::Strain => "Strain",
            Self::Velocity => "Velocity",
            Self::Torque => "Torque",
            Self::Sleeping => "Sleeping",
            Self::Disabled => "Disabled",
            Self::CollisionGroup => "CollisionGroup",
            Self::Buoyancy => "Buoyancy",
        }
    }

    /// 自 canonical 名还原;未知名 = `Err(IllegalPhysicsType)`(非法枚举
    /// RED 臂的 fail-closed 面)。
    pub fn parse(s: &str) -> Result<Self, FieldError> {
        Ok(match s {
            "LinearForce" => Self::LinearForce,
            "Strain" => Self::Strain,
            "Velocity" => Self::Velocity,
            "Torque" => Self::Torque,
            "Sleeping" => Self::Sleeping,
            "Disabled" => Self::Disabled,
            "CollisionGroup" => Self::CollisionGroup,
            "Buoyancy" => Self::Buoyancy,
            other => return Err(FieldError::IllegalPhysicsType(other.into())),
        })
    }

    /// 自 discriminator 还原;越界 fail-closed。
    pub fn from_u8(v: u8) -> Result<Self, FieldError> {
        Ok(match v {
            0 => Self::LinearForce,
            1 => Self::Strain,
            2 => Self::Velocity,
            3 => Self::Torque,
            4 => Self::Sleeping,
            5 => Self::Disabled,
            6 => Self::CollisionGroup,
            7 => Self::Buoyancy,
            other => return Err(FieldError::IllegalPhysicsType(format!("disc {other}"))),
        })
    }
}

/// 场定义层基元(RFC-0024 §4.B1 冻结六件;analytic-surface 为浮力水面
/// 函数预留,M124 共用求值管线)。
#[derive(Debug, Clone, PartialEq)]
pub enum FieldNodeKind {
    /// 径向衰减:`falloff = max(0, 1 - dist/radius)`。
    RadialFalloff {
        /// 中心。
        center: [f32; 3],
        /// 半径(> 0)。
        radius: f32,
    },
    /// 盒(内含 = 1,外 = 0;轴对齐)。
    Box {
        /// 最小角。
        min: [f32; 3],
        /// 最大角。
        max: [f32; 3],
    },
    /// 球(内含 = 1,外 = 0)。
    Sphere {
        /// 中心。
        center: [f32; 3],
        /// 半径(> 0)。
        radius: f32,
    },
    /// 确定性 hash-noise(整数格点;骨架期 0..1 标量)。
    Noise {
        /// 格点尺度(> 0)。
        scale: f32,
        /// 种子(确定性面)。
        seed: u64,
    },
    /// 曲线驱动(分段线性;points 单调 x 升序,采样 x = 距锚点距离)。
    CurveDriven {
        /// (x, y) 折线点(≥2)。
        points: Vec<(f32, f32)>,
        /// 锚点。
        anchor: [f32; 3],
    },
    /// 解析曲面(预留:浮力水面函数;骨架期 = 平面 `z = height`)。
    AnalyticSurface {
        /// 水面高度(骨架期平面参数)。
        height: f32,
    },
}

/// 场节点(基元 + 权重 + 子节点图;图 schema 版本化)。
#[derive(Debug, Clone, PartialEq)]
pub struct FieldNode {
    /// 节点稳定 ID(图内唯一;canonical 面)。
    pub node_id: String,
    /// 基元。
    pub kind: FieldNodeKind,
    /// 节点权重(默认 1.0;组合 = 加权和)。
    pub weight: f32,
    /// 子节点(节点图组合;canonical 序 = 声明序)。
    pub children: Vec<FieldNode>,
}

/// 场定义(三层解耦的层一 + 目标语义 + 生命周期 + 过滤;digest 冻结面)。
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    /// schema 头(版本化)。
    pub schema_id: String,
    /// schema 版本(= `FIELD_SCHEMA_VERSION`)。
    pub schema_version: u32,
    /// 场稳定 ID(注册/journal 面唯一键)。
    pub field_id: String,
    /// 根节点。
    pub root: FieldNode,
    /// 目标语义(首期八枚举之一)。
    pub physics_type: FieldPhysicsType,
    /// 生命周期。
    pub lifecycle: FieldLifecycle,
    /// 过滤(场定义的一部分,进 digest;默认空集匹配 = 无影响)。
    pub filter: FieldFilter,
}

/// Field 域错误(fail-closed 单一出口;门脚本 RED 臂锚字面)。
#[derive(Debug, Clone, PartialEq)]
pub enum FieldError {
    /// 非法 `FieldPhysicsType`(非法枚举 RED 臂)。
    IllegalPhysicsType(String),
    /// 未知 schema id / 版本。
    UnknownSchema(String),
    /// 定义非法(半径非正 / 折线点不足 / 节点 ID 冲突等)。
    InvalidDef(String),
    /// 未注册场操作(注销/变更未注册场 = fail-closed)。
    NotRegistered(String),
    /// 重复注册。
    AlreadyRegistered(String),
    /// 生命周期违例(Transient 进 journal / Construction 运行时注册等)。
    LifecycleViolation(String),
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IllegalPhysicsType(s) => write!(f, "IllegalPhysicsType({s})"),
            Self::UnknownSchema(s) => write!(f, "UnknownSchema({s})"),
            Self::InvalidDef(s) => write!(f, "InvalidDef({s})"),
            Self::NotRegistered(s) => write!(f, "NotRegistered({s})"),
            Self::AlreadyRegistered(s) => write!(f, "AlreadyRegistered({s})"),
            Self::LifecycleViolation(s) => write!(f, "LifecycleViolation({s})"),
        }
    }
}

impl std::error::Error for FieldError {}

fn push_f32(buf: &mut String, v: f32) {
    // canonical f32 面 = bit-exact 十六进制(NaN 输入在 schema 校验期
    // fail-closed,见 FieldDef::validate)。
    buf.push_str(&format!("{:08x}", v.to_bits()));
}

fn push_node(buf: &mut String, n: &FieldNode) {
    buf.push_str("{\"node_id\":\"");
    buf.push_str(&n.node_id.replace('\\', "\\\\").replace('"', "\\\""));
    buf.push_str("\",\"kind\":");
    match &n.kind {
        FieldNodeKind::RadialFalloff { center, radius } => {
            buf.push_str("{\"radial_falloff\":{\"center\":[");
            for (i, c) in center.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                push_f32(buf, *c);
            }
            buf.push_str("],\"radius\":");
            push_f32(buf, *radius);
            buf.push_str("}}");
        }
        FieldNodeKind::Box { min, max } => {
            buf.push_str("{\"box\":{\"min\":[");
            for (i, c) in min.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                push_f32(buf, *c);
            }
            buf.push_str("],\"max\":[");
            for (i, c) in max.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                push_f32(buf, *c);
            }
            buf.push_str("]}}");
        }
        FieldNodeKind::Sphere { center, radius } => {
            buf.push_str("{\"sphere\":{\"center\":[");
            for (i, c) in center.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                push_f32(buf, *c);
            }
            buf.push_str("],\"radius\":");
            push_f32(buf, *radius);
            buf.push_str("}}");
        }
        FieldNodeKind::Noise { scale, seed } => {
            buf.push_str("{\"noise\":{\"scale\":");
            push_f32(buf, *scale);
            buf.push_str(&format!(",\"seed\":{seed}}}"));
        }
        FieldNodeKind::CurveDriven { points, anchor } => {
            buf.push_str("{\"curve_driven\":{\"points\":[");
            for (i, (x, y)) in points.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                buf.push('[');
                push_f32(buf, *x);
                buf.push(',');
                push_f32(buf, *y);
                buf.push(']');
            }
            buf.push_str("],\"anchor\":[");
            for (i, c) in anchor.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                push_f32(buf, *c);
            }
            buf.push_str("]}}");
        }
        FieldNodeKind::AnalyticSurface { height } => {
            buf.push_str("{\"analytic_surface\":{\"height\":");
            push_f32(buf, *height);
            buf.push_str("}}");
        }
    }
    buf.push_str(",\"weight\":");
    push_f32(buf, n.weight);
    buf.push_str(",\"children\":[");
    for (i, c) in n.children.iter().enumerate() {
        if i > 0 {
            buf.push(',');
        }
        push_node(buf, c);
    }
    buf.push_str("]}");
}

impl FieldNode {
    /// 确定性标量采样(骨架期 = 基元解析求值 + 加权和;NaN 输入已由
    /// schema 校验期 fail-closed,采样面不再复检)。
    pub fn sample(&self, p: [f32; 3]) -> f32 {
        let base = match &self.kind {
            FieldNodeKind::RadialFalloff { center, radius } => {
                let d = dist3(p, *center);
                (1.0 - d / radius).max(0.0)
            }
            FieldNodeKind::Box { min, max } => {
                let inside = (0..3).all(|i| p[i] >= min[i] && p[i] <= max[i]);
                if inside { 1.0 } else { 0.0 }
            }
            FieldNodeKind::Sphere { center, radius } => {
                if dist3(p, *center) <= *radius {
                    1.0
                } else {
                    0.0
                }
            }
            FieldNodeKind::Noise { scale, seed } => hash_noise(p, *scale, *seed),
            FieldNodeKind::CurveDriven { points, anchor } => {
                let x = dist3(p, *anchor);
                sample_curve(points, x)
            }
            FieldNodeKind::AnalyticSurface { height } => {
                // 骨架期平面:z < height → 1(浸没);z >= height → 0。
                if p[2] < *height { 1.0 } else { 0.0 }
            }
        };
        let mut acc = base * self.weight;
        for c in &self.children {
            acc += c.sample(p);
        }
        acc
    }
}

impl FieldDef {
    /// 构造(冻结 schema 头;filter 默认空集匹配 = 零影响,由调用方显式给)。
    pub fn new(
        field_id: impl Into<String>,
        root: FieldNode,
        physics_type: FieldPhysicsType,
        lifecycle: FieldLifecycle,
        filter: FieldFilter,
    ) -> Self {
        Self {
            schema_id: FIELD_SCHEMA_ID.into(),
            schema_version: FIELD_SCHEMA_VERSION,
            field_id: field_id.into(),
            root,
            physics_type,
            lifecycle,
            filter,
        }
    }

    /// schema 校验(fail-closed;NaN / 非正半径 / 折线点不足 / 空 field_id /
    /// 节点 ID 冲突全拒)。
    pub fn validate(&self) -> Result<(), FieldError> {
        if self.schema_id != FIELD_SCHEMA_ID {
            return Err(FieldError::UnknownSchema(self.schema_id.clone()));
        }
        if self.schema_version != FIELD_SCHEMA_VERSION {
            return Err(FieldError::UnknownSchema(format!(
                "version {}",
                self.schema_version
            )));
        }
        if self.field_id.is_empty() {
            return Err(FieldError::InvalidDef("empty field_id".into()));
        }
        let mut ids = std::collections::BTreeSet::new();
        validate_node(&self.root, &mut ids)?;
        if !self.filter.domain_mask_valid() {
            return Err(FieldError::InvalidDef(
                "filter domain_mask has bits outside five-domain set".into(),
            ));
        }
        Ok(())
    }

    /// canonical JSON(digest 前像;三层 + 生命周期 + filter 全进)。
    pub fn canonical_json(&self) -> String {
        let mut s = String::from("{\"schema_id\":\"");
        s.push_str(&self.schema_id);
        s.push_str("\",\"schema_version\":");
        s.push_str(&self.schema_version.to_string());
        s.push_str(",\"field_id\":\"");
        s.push_str(&self.field_id.replace('\\', "\\\\").replace('"', "\\\""));
        s.push_str("\",\"physics_type\":\"");
        s.push_str(self.physics_type.canonical_name());
        s.push_str("\",\"lifecycle\":\"");
        s.push_str(self.lifecycle.canonical_name());
        s.push_str("\",\"root\":");
        push_node(&mut s, &self.root);
        s.push_str(",\"filter\":");
        s.push_str(&self.filter.canonical_json());
        s.push('}');
        s
    }

    /// 定义 digest(冻结面;filter 进 digest 承 RFC-0024 §4.B3)。
    pub fn digest(&self) -> String {
        hex(&digest(self.canonical_json().as_bytes()))
    }
}

fn validate_node(
    n: &FieldNode,
    ids: &mut std::collections::BTreeSet<String>,
) -> Result<(), FieldError> {
    if n.node_id.is_empty() {
        return Err(FieldError::InvalidDef("empty node_id".into()));
    }
    if !ids.insert(n.node_id.clone()) {
        return Err(FieldError::InvalidDef(format!(
            "duplicate node_id {}",
            n.node_id
        )));
    }
    if n.weight.is_nan() {
        return Err(FieldError::InvalidDef("NaN weight".into()));
    }
    match &n.kind {
        FieldNodeKind::RadialFalloff { center, radius } => {
            if center.iter().any(|c| c.is_nan()) || radius.is_nan() || *radius <= 0.0 {
                return Err(FieldError::InvalidDef("radial_falloff bad params".into()));
            }
        }
        FieldNodeKind::Box { min, max } => {
            if min.iter().chain(max.iter()).any(|c| c.is_nan()) || !(0..3).all(|i| min[i] <= max[i])
            {
                return Err(FieldError::InvalidDef("box bad params".into()));
            }
        }
        FieldNodeKind::Sphere { center, radius } => {
            if center.iter().any(|c| c.is_nan()) || radius.is_nan() || *radius <= 0.0 {
                return Err(FieldError::InvalidDef("sphere bad params".into()));
            }
        }
        FieldNodeKind::Noise { scale, .. } => {
            if scale.is_nan() || *scale <= 0.0 {
                return Err(FieldError::InvalidDef("noise bad scale".into()));
            }
        }
        FieldNodeKind::CurveDriven { points, anchor } => {
            if points.len() < 2
                || points.iter().any(|(x, y)| x.is_nan() || y.is_nan())
                || anchor.iter().any(|c| c.is_nan())
                || !points.windows(2).all(|w| w[0].0 <= w[1].0)
            {
                return Err(FieldError::InvalidDef("curve_driven bad params".into()));
            }
        }
        FieldNodeKind::AnalyticSurface { height } => {
            if height.is_nan() {
                return Err(FieldError::InvalidDef("analytic_surface NaN height".into()));
            }
        }
    }
    for c in &n.children {
        validate_node(c, ids)?;
    }
    Ok(())
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// 确定性 hash-noise(整数格点;位运算混合,与 FP 环境无关的面只在
/// 格点取整处,格内常数——骨架期零插值,确定性优先于平滑)。
fn hash_noise(p: [f32; 3], scale: f32, seed: u64) -> f32 {
    let xi = (p[0] * scale).floor() as i64;
    let yi = (p[1] * scale).floor() as i64;
    let zi = (p[2] * scale).floor() as i64;
    let mut h = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(xi as u64)
        .wrapping_mul(0xBF58_476D_1CE4_E5B9)
        .wrapping_add(yi as u64)
        .wrapping_mul(0x94D0_49BB_1331_11EB)
        .wrapping_add(zi as u64);
    h ^= h >> 29;
    h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 32;
    // 0..1 标量(高 24 位 / 2^24)。
    ((h >> 40) & 0xFF_FFFF) as f32 / 16_777_216.0
}

fn sample_curve(points: &[(f32, f32)], x: f32) -> f32 {
    if x <= points[0].0 {
        return points[0].1;
    }
    for w in points.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if x <= x1 {
            if x1 == x0 {
                return y1;
            }
            let t = (x - x0) / (x1 - x0);
            return y0 + t * (y1 - y0);
        }
    }
    points[points.len() - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::lifecycle::FieldLifecycle;

    fn def_of(kind: FieldNodeKind) -> FieldDef {
        FieldDef::new(
            "f0",
            FieldNode {
                node_id: "n0".into(),
                kind,
                weight: 1.0,
                children: vec![],
            },
            FieldPhysicsType::LinearForce,
            FieldLifecycle::Transient,
            FieldFilter::default(),
        )
    }

    #[test]
    fn eight_enum_parse_roundtrip_and_illegal_fails_closed() {
        assert_eq!(FieldPhysicsType::ALL.len(), 8);
        for (i, t) in FieldPhysicsType::ALL.iter().enumerate() {
            assert_eq!(FieldPhysicsType::from_u8(i as u8).unwrap(), *t);
            assert_eq!(FieldPhysicsType::parse(t.canonical_name()).unwrap(), *t);
        }
        assert!(matches!(
            FieldPhysicsType::parse("LinearForces"),
            Err(FieldError::IllegalPhysicsType(_))
        ));
        assert!(matches!(
            FieldPhysicsType::from_u8(8),
            Err(FieldError::IllegalPhysicsType(_))
        ));
    }

    #[test]
    fn digest_deterministic_and_filter_in_digest() {
        let a = def_of(FieldNodeKind::Sphere {
            center: [0.0; 3],
            radius: 1.0,
        });
        let mut b = a.clone();
        assert_eq!(a.digest(), b.digest());
        b.filter.domain_mask = 0b00001;
        assert_ne!(a.digest(), b.digest(), "filter 进 digest");
    }

    #[test]
    fn nan_and_bad_params_fail_closed() {
        let mut d = def_of(FieldNodeKind::Sphere {
            center: [f32::NAN; 3],
            radius: 1.0,
        });
        assert!(d.validate().is_err());
        d = def_of(FieldNodeKind::Sphere {
            center: [0.0; 3],
            radius: 0.0,
        });
        assert!(d.validate().is_err());
        d = def_of(FieldNodeKind::CurveDriven {
            points: vec![(0.0, 1.0)],
            anchor: [0.0; 3],
        });
        assert!(d.validate().is_err());
    }
}
