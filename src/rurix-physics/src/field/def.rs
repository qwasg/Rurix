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

/// analytic-surface 基元最小闭集(spec/physics.md RXS-0374 L2;RFC-0024
/// §4.B1「为浮力水面函数预留」完整期兑现):**sphere / plane / box 三形**,
/// 提供解析符号距离与梯度解析采样;闭集外形状首期 fail-closed 拒绝(不静默
/// 退化采样);M124 浮力水面函数与本闭集共用同一求值管线,禁第二套曲面采样。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnalyticSurfacePrimitive {
    /// 球:sdf(p) = |p - center| - radius。
    Sphere {
        /// 中心。
        center: [f32; 3],
        /// 半径(> 0)。
        radius: f32,
    },
    /// 平面:sdf(p) = dot(normal, p) - offset(normal 须为单位法线)。
    Plane {
        /// 单位法线(有限且非零;归一性 = 定义方纪律)。
        normal: [f32; 3],
        /// 偏移。
        offset: f32,
    },
    /// 轴对齐箱:sdf(p) = 标准 AABB 符号距离(外正内负)。
    Box {
        /// 最小角。
        min: [f32; 3],
        /// 最大角。
        max: [f32; 3],
    },
}

impl AnalyticSurfacePrimitive {
    /// 闭集 canonical 名(journal/schema 面唯一合法字面)。
    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Sphere { .. } => "sphere",
            Self::Plane { .. } => "plane",
            Self::Box { .. } => "box",
        }
    }

    /// 闭集成员判定(闭集外形状名 = fail-closed,不静默退化采样)。
    pub fn closed_set_member(name: &str) -> bool {
        matches!(name, "sphere" | "plane" | "box")
    }

    /// 参数校验(fail-closed:NaN / 非正半径 / 零法线 / min>max 全拒)。
    pub fn validate(&self) -> Result<(), FieldError> {
        match self {
            Self::Sphere { center, radius } => {
                if center.iter().any(|c| c.is_nan()) || radius.is_nan() || *radius <= 0.0 {
                    return Err(FieldError::InvalidDef(
                        "analytic_surface_primitive sphere bad params".into(),
                    ));
                }
            }
            Self::Plane { normal, offset } => {
                let n2 = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
                if normal.iter().any(|c| c.is_nan()) || offset.is_nan() || n2 <= 0.0 {
                    return Err(FieldError::InvalidDef(
                        "analytic_surface_primitive plane bad params".into(),
                    ));
                }
            }
            Self::Box { min, max } => {
                if min.iter().chain(max.iter()).any(|c| c.is_nan())
                    || !(0..3).all(|i| min[i] <= max[i])
                {
                    return Err(FieldError::InvalidDef(
                        "analytic_surface_primitive box bad params".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// 解析符号距离(闭集三形全解析;同输入双跑位级一致)。
    pub fn signed_distance(&self, p: [f32; 3]) -> f32 {
        match self {
            Self::Sphere { center, radius } => dist3(p, *center) - radius,
            Self::Plane { normal, offset } => {
                normal[0] * p[0] + normal[1] * p[1] + normal[2] * p[2] - offset
            }
            Self::Box { min, max } => {
                let c = [
                    (min[0] + max[0]) * 0.5,
                    (min[1] + max[1]) * 0.5,
                    (min[2] + max[2]) * 0.5,
                ];
                let h = [
                    (max[0] - min[0]) * 0.5,
                    (max[1] - min[1]) * 0.5,
                    (max[2] - min[2]) * 0.5,
                ];
                let q = [
                    (p[0] - c[0]).abs() - h[0],
                    (p[1] - c[1]).abs() - h[1],
                    (p[2] - c[2]).abs() - h[2],
                ];
                let outside = {
                    let m = [q[0].max(0.0), q[1].max(0.0), q[2].max(0.0)];
                    (m[0] * m[0] + m[1] * m[1] + m[2] * m[2]).sqrt()
                };
                let inside = q[0].max(q[1]).max(q[2]).min(0.0);
                outside + inside
            }
        }
    }

    /// 解析梯度(sdf 梯度;盒外点 = 最近点方向,盒内点 = 最大穿透轴符号向,
    /// 球心/面退化点 = 零向——确定性优先于平滑,同输入双跑位级一致)。
    pub fn gradient(&self, p: [f32; 3]) -> [f32; 3] {
        match self {
            Self::Sphere { center, .. } => {
                let d = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
                let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                if len == 0.0 {
                    [0.0; 3]
                } else {
                    [d[0] / len, d[1] / len, d[2] / len]
                }
            }
            Self::Plane { normal, .. } => *normal,
            Self::Box { min, max } => {
                let sdf = self.signed_distance(p);
                if sdf > 0.0 {
                    // 盒外:梯度 = (p - clamp(p)) 归一。
                    let clamped = [
                        p[0].clamp(min[0], max[0]),
                        p[1].clamp(min[1], max[1]),
                        p[2].clamp(min[2], max[2]),
                    ];
                    let d = [p[0] - clamped[0], p[1] - clamped[1], p[2] - clamped[2]];
                    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                    if len == 0.0 {
                        [0.0; 3]
                    } else {
                        [d[0] / len, d[1] / len, d[2] / len]
                    }
                } else {
                    // 盒内:最大穿透轴符号向(q 最大分量轴)。
                    let c = [
                        (min[0] + max[0]) * 0.5,
                        (min[1] + max[1]) * 0.5,
                        (min[2] + max[2]) * 0.5,
                    ];
                    let h = [
                        (max[0] - min[0]) * 0.5,
                        (max[1] - min[1]) * 0.5,
                        (max[2] - min[2]) * 0.5,
                    ];
                    let q = [
                        (p[0] - c[0]).abs() - h[0],
                        (p[1] - c[1]).abs() - h[1],
                        (p[2] - c[2]).abs() - h[2],
                    ];
                    let axis = if q[0] >= q[1] && q[0] >= q[2] {
                        0
                    } else if q[1] >= q[2] {
                        1
                    } else {
                        2
                    };
                    let mut g = [0.0; 3];
                    g[axis] = if p[axis] >= c[axis] { 1.0 } else { -1.0 };
                    g
                }
            }
        }
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
    /// 解析曲面基元闭集(RXS-0374 L2 完整期加性面;sphere/plane/box 三形,
    /// 解析符号距离 + 梯度解析采样;骨架期 `AnalyticSurface{height}` 字面
    /// 0-byte 维持)。
    AnalyticSurfacePrimitive {
        /// 闭集基元(闭集外形状不可表达;跨边界文本经
        /// [`AnalyticSurfacePrimitive::closed_set_member`] fail-closed)。
        primitive: AnalyticSurfacePrimitive,
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

fn push_node(buf: &mut String, n: &FieldNode, wire: bool) {
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
            if wire {
                // 骨架期 canonical 冻结字节形态:noise arm 少闭合一层(golden
                // digest 前像,0-byte 不动);**线格式 v1 = well-formed JSON**,
                // 本分支补上 kind 对象闭合层(两格式经 def digest 锚互锁)。
                buf.push('}');
            }
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
        FieldNodeKind::AnalyticSurfacePrimitive { primitive } => {
            buf.push_str("{\"analytic_surface_primitive\":");
            push_primitive(buf, primitive);
            buf.push('}');
        }
    }
    buf.push_str(",\"weight\":");
    push_f32(buf, n.weight);
    buf.push_str(",\"children\":[");
    for (i, c) in n.children.iter().enumerate() {
        if i > 0 {
            buf.push(',');
        }
        push_node(buf, c, wire);
    }
    buf.push_str("]}");
}

fn push_primitive(buf: &mut String, prim: &AnalyticSurfacePrimitive) {
    match prim {
        AnalyticSurfacePrimitive::Sphere { center, radius } => {
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
        AnalyticSurfacePrimitive::Plane { normal, offset } => {
            buf.push_str("{\"plane\":{\"normal\":[");
            for (i, c) in normal.iter().enumerate() {
                if i > 0 {
                    buf.push(',');
                }
                push_f32(buf, *c);
            }
            buf.push_str("],\"offset\":");
            push_f32(buf, *offset);
            buf.push_str("}}");
        }
        AnalyticSurfacePrimitive::Box { min, max } => {
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
    }
}

impl FieldNode {
    /// 确定性标量采样(骨架期 = 基元解析求值 + 加权和;NaN 输入已由
    /// schema 校验期 fail-closed,采样面不再复检)。
    pub fn sample(&self, p: [f32; 3]) -> f32 {
        let mut acc = self.sample_local(p) * self.weight;
        for c in &self.children {
            acc += c.sample(p);
        }
        acc
    }

    /// 本节点基元局部采样(不含权重与子节点;数值梯度面的采样源)。
    fn sample_local(&self, p: [f32; 3]) -> f32 {
        match &self.kind {
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
            FieldNodeKind::AnalyticSurfacePrimitive { primitive } => {
                // 完整期闭集:内部(sdf ≤ 0)= 1,外部 = 0。
                if primitive.signed_distance(p) <= 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// 场梯度(RXS-0374 L1 完整期面):analytic-surface 闭集 = **解析梯度**
    /// (基元自身解析采样);其余基元 = 固定 eps 中心差分数值梯度——纯 host
    /// f32 定序运算,同输入双跑位级一致。
    pub fn gradient(&self, p: [f32; 3]) -> [f32; 3] {
        const GRAD_EPS: f32 = 1e-3;
        let local = match &self.kind {
            FieldNodeKind::AnalyticSurfacePrimitive { primitive } => primitive.gradient(p),
            _ => {
                let mut g = [0.0f32; 3];
                for axis in 0..3 {
                    let mut pa = p;
                    let mut pb = p;
                    pa[axis] += GRAD_EPS;
                    pb[axis] -= GRAD_EPS;
                    g[axis] =
                        (self.sample_local(pa) - self.sample_local(pb)) / (2.0 * GRAD_EPS);
                }
                g
            }
        };
        let mut acc = [
            local[0] * self.weight,
            local[1] * self.weight,
            local[2] * self.weight,
        ];
        for c in &self.children {
            let cg = c.gradient(p);
            acc[0] += cg[0];
            acc[1] += cg[1];
            acc[2] += cg[2];
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
        self.canonical_json_mode(false)
    }

    /// 线格式 v1 JSON(RXS-0374 L3 主流 journal 载荷面):与 canonical 同
    /// 构但 **well-formed**——骨架期 canonical 的 Noise 基元 arm 少闭合一层
    /// (冻结 golden digest 前像,字面 0-byte 不动),其不平衡字节会破坏主流
    /// journal 行的括号配对解析;线格式补上该闭合层,经 `def.digest()`(冻
    /// 结 canonical 前像)锚与 canonical 互锁,显式版本化(`v` 字段)而非
    /// 静默重解释。
    pub fn wire_json(&self) -> String {
        self.canonical_json_mode(true)
    }

    fn canonical_json_mode(&self, wire: bool) -> String {
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
        push_node(&mut s, &self.root, wire);
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
        FieldNodeKind::AnalyticSurfacePrimitive { primitive } => {
            primitive.validate()?;
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

// —— canonical JSON 还原面(RXS-0374 L3:主流 journal replay 的场定义重建源;
// encode→decode 往返无损 + 非 canonical 字节 fail-closed,不静默重解释)——

/// 最小 JSON 值(canonical 前像形态闭集:对象/数组/字符串/数字串/bool;
/// f32 为**无引号** 8 位 hex 串,u64 为十进制串——词法层只收原始数字串,
/// 语义解释归取值函数〔`as_u64` 十进制 / `json_f32` hex〕)。
#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<JsonValue>),
    Obj(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn key<'a>(&'a self, k: &str) -> Result<&'a JsonValue, FieldError> {
        let JsonValue::Obj(pairs) = self else {
            return Err(FieldError::InvalidDef(format!("expect object for {k}")));
        };
        pairs
            .iter()
            .find(|(pk, _)| pk == k)
            .map(|(_, v)| v)
            .ok_or_else(|| FieldError::InvalidDef(format!("missing key {k}")))
    }

    fn as_str(&self) -> Result<&str, FieldError> {
        let JsonValue::Str(s) = self else {
            return Err(FieldError::InvalidDef("expect string".into()));
        };
        Ok(s)
    }

    fn as_u64(&self) -> Result<u64, FieldError> {
        let JsonValue::Num(raw) = self else {
            return Err(FieldError::InvalidDef("expect u64".into()));
        };
        if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
            return Err(FieldError::InvalidDef(format!("u64 decimal expected: {raw}")));
        }
        raw.parse::<u64>()
            .map_err(|e| FieldError::InvalidDef(format!("u64: {e}")))
    }

    fn as_arr(&self) -> Result<&[JsonValue], FieldError> {
        let JsonValue::Arr(a) = self else {
            return Err(FieldError::InvalidDef("expect array".into()));
        };
        Ok(a)
    }
}

struct JsonParser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> JsonParser<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\t' | b'\r') {
            self.i += 1;
        }
    }

    fn value(&mut self) -> Result<JsonValue, FieldError> {
        self.ws();
        match self.b.get(self.i) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(JsonValue::Str(self.string()?)),
            Some(b't') => self.literal("true", JsonValue::Bool(true)),
            Some(b'f') if self.b[self.i..].starts_with(b"false") => {
                self.literal("false", JsonValue::Bool(false))
            }
            // 数字串 = [0-9a-f] 连续段(canonical f32 无引号 8 位 hex 可
            // 以 a-f 字母起头,如 0xbe800000;u64 十进制只含数字)。
            Some(c) if c.is_ascii_digit() || (b'a'..=b'f').contains(c) => self.number(),
            other => Err(FieldError::InvalidDef(format!(
                "json value unexpected byte {other:?}"
            ))),
        }
    }

    fn literal(&mut self, lit: &str, v: JsonValue) -> Result<JsonValue, FieldError> {
        if self.b[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            Ok(v)
        } else {
            Err(FieldError::InvalidDef(format!("json literal {lit}")))
        }
    }

    fn number(&mut self) -> Result<JsonValue, FieldError> {
        // 数字串 = [0-9a-f] 连续段(f32 无引号 8 位 hex 与 u64 十进制共用
        // 词法;语义解释归取值函数)。
        let start = self.i;
        while self.i < self.b.len()
            && (self.b[self.i].is_ascii_digit() || (b'a'..=b'f').contains(&self.b[self.i]))
        {
            self.i += 1;
        }
        if start == self.i {
            return Err(FieldError::InvalidDef("json number empty".into()));
        }
        let s = std::str::from_utf8(&self.b[start..self.i])
            .map_err(|e| FieldError::InvalidDef(format!("json number utf8: {e}")))?;
        Ok(JsonValue::Num(s.to_string()))
    }

    fn string(&mut self) -> Result<String, FieldError> {
        self.i += 1; // opening quote
        let mut out = String::new();
        loop {
            let Some(&c) = self.b.get(self.i) else {
                return Err(FieldError::InvalidDef("json string unterminated".into()));
            };
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else {
                        return Err(FieldError::InvalidDef("json escape truncated".into()));
                    };
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        other => {
                            return Err(FieldError::InvalidDef(format!(
                                "json escape \\{}",
                                other as char
                            )));
                        }
                    }
                }
                _ => {
                    // canonical 面只含 ASCII/可见字符与已转移两字符;其余字节
                    // 按 UTF-8 连续段收集(生成面不产多字节,防御性收集)。
                    let start = self.i - 1;
                    let mut end = self.i;
                    while end < self.b.len() && self.b[end] != b'"' && self.b[end] != b'\\' {
                        end += 1;
                    }
                    let seg = std::str::from_utf8(&self.b[start..end]).map_err(|e| {
                        FieldError::InvalidDef(format!("json string utf8: {e}"))
                    })?;
                    out.push_str(seg);
                    self.i = end;
                }
            }
        }
    }

    fn object(&mut self) -> Result<JsonValue, FieldError> {
        self.i += 1; // {
        let mut pairs = Vec::new();
        self.ws();
        if self.b.get(self.i) == Some(&b'}') {
            self.i += 1;
            return Ok(JsonValue::Obj(pairs));
        }
        loop {
            self.ws();
            if self.b.get(self.i) != Some(&b'"') {
                return Err(FieldError::InvalidDef("json object key".into()));
            }
            let k = self.string()?;
            self.ws();
            if self.b.get(self.i) != Some(&b':') {
                return Err(FieldError::InvalidDef("json object colon".into()));
            }
            self.i += 1;
            let v = self.value()?;
            pairs.push((k, v));
            self.ws();
            match self.b.get(self.i) {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(JsonValue::Obj(pairs));
                }
                other => {
                    return Err(FieldError::InvalidDef(format!(
                        "json object sep {other:?}"
                    )));
                }
            }
        }
    }

    fn array(&mut self) -> Result<JsonValue, FieldError> {
        self.i += 1; // [
        let mut items = Vec::new();
        self.ws();
        if self.b.get(self.i) == Some(&b']') {
            self.i += 1;
            return Ok(JsonValue::Arr(items));
        }
        loop {
            let v = self.value()?;
            items.push(v);
            self.ws();
            match self.b.get(self.i) {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b']') => {
                    self.i += 1;
                    return Ok(JsonValue::Arr(items));
                }
                other => {
                    return Err(FieldError::InvalidDef(format!("json array sep {other:?}")));
                }
            }
        }
    }
}

fn parse_json_value(text: &str) -> Result<JsonValue, FieldError> {
    let mut p = JsonParser {
        b: text.as_bytes(),
        i: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(FieldError::InvalidDef("json trailing bytes".into()));
    }
    Ok(v)
}

fn json_f32(v: &JsonValue) -> Result<f32, FieldError> {
    // canonical f32 面 = 无引号 8 位 hex(位表示;push_f32 生成面)。
    let JsonValue::Num(raw) = v else {
        return Err(FieldError::InvalidDef("f32 expect hex num".into()));
    };
    if raw.len() != 8 || !raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(FieldError::InvalidDef(format!("f32 hex: {raw}")));
    }
    let bits = u32::from_str_radix(raw, 16)
        .map_err(|e| FieldError::InvalidDef(format!("f32 hex: {e}")))?;
    Ok(f32::from_bits(bits))
}

fn json_vec3(v: &JsonValue) -> Result<[f32; 3], FieldError> {
    let a = v.as_arr()?;
    if a.len() != 3 {
        return Err(FieldError::InvalidDef("vec3 len".into()));
    }
    Ok([json_f32(&a[0])?, json_f32(&a[1])?, json_f32(&a[2])?])
}

fn json_primitive(v: &JsonValue) -> Result<AnalyticSurfacePrimitive, FieldError> {
    let JsonValue::Obj(pairs) = v else {
        return Err(FieldError::InvalidDef("primitive expect object".into()));
    };
    if pairs.len() != 1 {
        return Err(FieldError::InvalidDef("primitive single key".into()));
    }
    let (k, body) = &pairs[0];
    if !AnalyticSurfacePrimitive::closed_set_member(k) {
        // 闭集外形状首期 fail-closed 拒绝(不静默退化采样)。
        return Err(FieldError::InvalidDef(format!(
            "analytic-surface primitive outside closed set: {k}"
        )));
    }
    match k.as_str() {
        "sphere" => Ok(AnalyticSurfacePrimitive::Sphere {
            center: json_vec3(body.key("center")?)?,
            radius: json_f32(body.key("radius")?)?,
        }),
        "plane" => Ok(AnalyticSurfacePrimitive::Plane {
            normal: json_vec3(body.key("normal")?)?,
            offset: json_f32(body.key("offset")?)?,
        }),
        "box" => Ok(AnalyticSurfacePrimitive::Box {
            min: json_vec3(body.key("min")?)?,
            max: json_vec3(body.key("max")?)?,
        }),
        other => Err(FieldError::InvalidDef(format!(
            "analytic-surface primitive outside closed set: {other}"
        ))),
    }
}

fn json_node(v: &JsonValue) -> Result<FieldNode, FieldError> {
    let node_id = v.key("node_id")?.as_str()?.to_string();
    let kind_v = v.key("kind")?;
    let JsonValue::Obj(kpairs) = kind_v else {
        return Err(FieldError::InvalidDef("node kind expect object".into()));
    };
    if kpairs.len() != 1 {
        return Err(FieldError::InvalidDef("node kind single key".into()));
    }
    let (kk, kb) = &kpairs[0];
    let kind = match kk.as_str() {
        "radial_falloff" => FieldNodeKind::RadialFalloff {
            center: json_vec3(kb.key("center")?)?,
            radius: json_f32(kb.key("radius")?)?,
        },
        "box" => FieldNodeKind::Box {
            min: json_vec3(kb.key("min")?)?,
            max: json_vec3(kb.key("max")?)?,
        },
        "sphere" => FieldNodeKind::Sphere {
            center: json_vec3(kb.key("center")?)?,
            radius: json_f32(kb.key("radius")?)?,
        },
        "noise" => FieldNodeKind::Noise {
            scale: json_f32(kb.key("scale")?)?,
            seed: kb.key("seed")?.as_u64()?,
        },
        "curve_driven" => {
            let pts_v = kb.key("points")?.as_arr()?;
            let mut points = Vec::with_capacity(pts_v.len());
            for pv in pts_v {
                let pair = pv.as_arr()?;
                if pair.len() != 2 {
                    return Err(FieldError::InvalidDef("curve point pair".into()));
                }
                points.push((json_f32(&pair[0])?, json_f32(&pair[1])?));
            }
            FieldNodeKind::CurveDriven {
                points,
                anchor: json_vec3(kb.key("anchor")?)?,
            }
        }
        "analytic_surface" => FieldNodeKind::AnalyticSurface {
            height: json_f32(kb.key("height")?)?,
        },
        "analytic_surface_primitive" => FieldNodeKind::AnalyticSurfacePrimitive {
            primitive: json_primitive(kb)?,
        },
        other => {
            return Err(FieldError::InvalidDef(format!("unknown node kind {other}")));
        }
    };
    let weight = json_f32(v.key("weight")?)?;
    let children_v = v.key("children")?.as_arr()?;
    let mut children = Vec::with_capacity(children_v.len());
    for cv in children_v {
        children.push(json_node(cv)?);
    }
    Ok(FieldNode {
        node_id,
        kind,
        weight,
        children,
    })
}

fn json_filter(v: &JsonValue) -> Result<super::filter::FieldFilter, FieldError> {
    fn str_list(v: &JsonValue) -> Result<Vec<String>, FieldError> {
        let mut out = Vec::new();
        for item in v.as_arr()? {
            out.push(item.as_str()?.to_string());
        }
        Ok(out)
    }
    Ok(super::filter::FieldFilter {
        object_state_mask: v.key("object_state_mask")?.as_u64()?,
        domain_mask: v.key("domain_mask")?.as_u64()?,
        layer_mask: v.key("layer_mask")?.as_u64()?,
        explicit_include: str_list(v.key("include")?)?,
        explicit_exclude: str_list(v.key("exclude")?)?,
    })
}

impl FieldDef {
    /// 自线格式 v1 JSON 还原(RXS-0374 L3 主流 journal replay 重建源):
    /// 结构解析 + schema 校验 + **线格式字节回写相等**三重 fail-closed;
    /// 版本变化显式迁移而非静默重解释(未知 schema id/版本即
    /// [`FieldError::UnknownSchema`])。digest 锚 = 冻结 canonical 前像
    /// (`def.digest()`),与线格式互锁。
    pub fn parse_wire_json(text: &str) -> Result<Self, FieldError> {
        let v = parse_json_value(text)?;
        let def = FieldDef {
            schema_id: v.key("schema_id")?.as_str()?.to_string(),
            schema_version: v.key("schema_version")?.as_u64()? as u32,
            field_id: v.key("field_id")?.as_str()?.to_string(),
            physics_type: FieldPhysicsType::parse(v.key("physics_type")?.as_str()?)?,
            lifecycle: FieldLifecycle::parse(v.key("lifecycle")?.as_str()?)?,
            root: json_node(v.key("root")?)?,
            filter: json_filter(v.key("filter")?)?,
        };
        def.validate()?;
        if def.wire_json() != text {
            return Err(FieldError::InvalidDef(
                "non-canonical field wire bytes (silent reinterpretation refused)".into(),
            ));
        }
        Ok(def)
    }
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

    //@ spec: RXS-0374
    #[test]
    fn analytic_surface_closed_set_sdf_gradient_deterministic() {
        // 最小闭集 = sphere/plane/box 三形;解析符号距离 + 梯度已知值锚定。
        let sphere = AnalyticSurfacePrimitive::Sphere {
            center: [0.0; 3],
            radius: 2.0,
        };
        assert_eq!(sphere.signed_distance([0.0; 3]), -2.0);
        assert_eq!(sphere.signed_distance([3.0, 0.0, 0.0]), 1.0);
        assert_eq!(sphere.gradient([3.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
        assert_eq!(sphere.gradient([0.0; 3]), [0.0; 3], "球心退化零向");

        let plane = AnalyticSurfacePrimitive::Plane {
            normal: [0.0, 0.0, 1.0],
            offset: 1.5,
        };
        assert_eq!(plane.signed_distance([0.0, 0.0, 1.0]), -0.5);
        assert_eq!(plane.gradient([9.0, 9.0, 9.0]), [0.0, 0.0, 1.0]);

        let b = AnalyticSurfacePrimitive::Box {
            min: [-1.0; 3],
            max: [1.0; 3],
        };
        assert_eq!(b.signed_distance([0.0; 3]), -1.0);
        assert_eq!(b.signed_distance([2.0, 0.0, 0.0]), 1.0);
        assert_eq!(b.gradient([2.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
        // 盒内等穿透平局取首轴(确定性决胜规则)。
        assert_eq!(b.gradient([0.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);

        // 同输入双跑位级一致(求值确定性)。
        let p = [0.3, -1.2, 4.5];
        for prim in [sphere, plane, b] {
            assert_eq!(prim.signed_distance(p), prim.signed_distance(p));
            assert_eq!(prim.gradient(p), prim.gradient(p));
            assert!(prim.validate().is_ok());
        }
        // 闭集外形状首期 fail-closed(不静默退化采样)。
        for outside in ["capsule", "cone", "mesh", "cylinder", ""] {
            assert!(!AnalyticSurfacePrimitive::closed_set_member(outside));
        }
        for inside in ["sphere", "plane", "box"] {
            assert!(AnalyticSurfacePrimitive::closed_set_member(inside));
        }
        // 非法参数 fail-closed。
        assert!(
            AnalyticSurfacePrimitive::Sphere {
                center: [0.0; 3],
                radius: 0.0
            }
            .validate()
            .is_err()
        );
        assert!(
            AnalyticSurfacePrimitive::Plane {
                normal: [0.0; 3],
                offset: 0.0
            }
            .validate()
            .is_err()
        );
        assert!(
            AnalyticSurfacePrimitive::Box {
                min: [1.0; 3],
                max: [0.0; 3]
            }
            .validate()
            .is_err()
        );
    }

    //@ spec: RXS-0374
    #[test]
    fn canonical_json_parse_roundtrip_bitexact() {
        // encode→decode 往返无损(闭集基元 + 子图 + filter 全进)。
        let mut def = def_of(FieldNodeKind::AnalyticSurfacePrimitive {
            primitive: AnalyticSurfacePrimitive::Sphere {
                center: [1.0, 2.0, 3.0],
                radius: 2.5,
            },
        });
        def.root.children.push(FieldNode {
            node_id: "child".into(),
            kind: FieldNodeKind::AnalyticSurfacePrimitive {
                primitive: AnalyticSurfacePrimitive::Plane {
                    normal: [0.0, 1.0, 0.0],
                    offset: -0.25,
                },
            },
            weight: 0.5,
            children: vec![],
        });
        let cj = def.wire_json();
        let back = FieldDef::parse_wire_json(&cj).expect("wire parse");
        assert_eq!(back, def, "往返无损");
        assert_eq!(back.digest(), def.digest(), "digest 锚一致");
        // 非噪声基元的线格式与冻结 canonical 字节逐位一致(两格式同构面)。
        assert_eq!(cj, def.canonical_json());
        // 篡改半径位表示:解析所得定义 digest 必改(完整性由 digest 锚承载)。
        let radius_hex = format!("\"radius\":{:08x}", 2.5f32.to_bits());
        let tampered = cj.replacen(&radius_hex, "\"radius\":40200001", 1);
        assert_ne!(tampered, cj, "半径 canonical 字面须在位");
        let tdef = FieldDef::parse_wire_json(&tampered).expect("tampered still canonical");
        assert_ne!(tdef.digest(), def.digest(), "篡改必改 digest");
        // 非 canonical 字节(多余空白)fail-closed(不静默重解释)。
        let spaced = cj.replacen("\"radius\":", "\"radius\":  ", 1);
        assert!(FieldDef::parse_wire_json(&spaced).is_err());
        // 闭集外形状文本注入 fail-closed。
        let injected = cj.replace(
            "\"analytic_surface_primitive\":{\"sphere\"",
            "\"analytic_surface_primitive\":{\"capsule\"",
        );
        if injected != cj {
            assert!(FieldDef::parse_wire_json(&injected).is_err());
        }
        // 未知 schema 版本 = UnknownSchema(显式迁移纪律,不静默重解释)。
        let mut bad = def.clone();
        bad.schema_version = 2;
        assert!(matches!(
            FieldDef::parse_wire_json(&bad.wire_json()),
            Err(FieldError::UnknownSchema(_))
        ));
    }

    //@ spec: RXS-0374
    #[test]
    fn noise_canonical_frozen_bytes_and_wire_wellformed_roundtrip() {
        // 骨架期冻结字节形态(G9.2 golden digest 前像,0-byte 不动):Noise
        // 基元 arm 少闭合一层——本测试钉死该冻结字面,防误改。
        let d = def_of(FieldNodeKind::Noise {
            scale: 0.5,
            seed: 42,
        });
        assert!(
            d.canonical_json().contains("\"seed\":42},\"weight\""),
            "冻结 canonical 的 noise 少闭合层字面维持"
        );
        // 线格式 v1 = well-formed(补闭合层):往返无损 + digest 锚互锁。
        let w = d.wire_json();
        assert!(
            w.contains("\"seed\":42}},\"weight\""),
            "线格式补 kind 闭合层"
        );
        let back = FieldDef::parse_wire_json(&w).expect("wire parse");
        assert_eq!(back, d, "噪声定义线格式往返无损");
        assert_eq!(back.digest(), d.digest(), "digest 锚 = 冻结 canonical 前像");
    }
}
