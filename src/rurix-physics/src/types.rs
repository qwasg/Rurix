//! 冻结接口数据类型面(RFC-0017 §4.A1/A2/A4/A5/A6,§4.0-3 字面冻结不漂移)。
//! 全部为纯数据 + 纯校验(无后端依赖);库内不出现 4×4/3×4 矩阵类型
//! (`PhysicsTransform` 为唯一桥接输入,3×4 合成在 bridge,§4.A2/P-11)。

use crate::error::PhysicsError;
use crate::id::{BodyId, ShapeId};

/// 后端枚举(§4.A1;`Jolt` 生产默认,`Rapier` 快路径 feature `rapier`,G6.4 实现)。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    /// Jolt 生产默认后端(feature `jolt`,默认 on,经 rurix-physics-sys vendor 构建)。
    #[default]
    Jolt,
    /// Rapier 快路径后端(feature `rapier`,默认 off,G6.4 实现;feature 未编译 →
    /// 确定性 `Err(BackendNotCompiled)`,P-01 不静默回退)。
    Rapier,
    /// Jolt 5.6 评估后端(G9.6 M125,RXS-0377;feature `jolt56`,默认 off,经
    /// rurix-physics-sys56 独立 vendor 线构建;**评估用途,不升格生产默认**——
    /// 与 5.3 基线 `Jolt` 并存同进程各自实例化,feature 未编译 → 确定性
    /// `Err(BackendNotCompiled)`,P-01 不静默回退)。
    Jolt56,
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendKind::Jolt => write!(f, "Jolt"),
            BackendKind::Rapier => write!(f, "Rapier"),
            BackendKind::Jolt56 => write!(f, "Jolt56"),
        }
    }
}

/// 世界描述(§4.A1 + `dt_fixed`[固定步校验在 safe 层] + `contact_capacity`
/// [事件 ring 容量,§4.A5])。
#[derive(Debug, Clone, PartialEq)]
pub struct WorldDesc {
    /// 后端选择(默认 Jolt)。
    pub backend: BackendKind,
    /// 重力加速度(m/s²,各分量须有限)。
    pub gravity: [f32; 3],
    /// object layer 数(≥ 1;Jolt ObjectLayer 位宽约束内,上限随 sys crate 定案)。
    pub layer_count: u32,
    /// body 池上限(≥ 1;body 与 shape 两个 arena 同容)。
    pub max_bodies: u32,
    /// job 线程数;`None` = 可用并行度(默认库内线程池,§4.A3 job 适配层)。
    pub job_threads: Option<u32>,
    /// 固定步长(秒,> 0 且有限;`step` 只收此值,accumulator 在宿主,§4.A1)。
    pub dt_fixed: f32,
    /// 接触事件 ring 容量(溢出确定性丢最旧 + 计数;0 = 全丢,§4.A5)。
    pub contact_capacity: u32,
}

impl Default for WorldDesc {
    fn default() -> Self {
        WorldDesc {
            backend: BackendKind::Jolt,
            gravity: [0.0, -9.81, 0.0],
            layer_count: 8,
            max_bodies: 65_536,
            job_threads: None,
            dt_fixed: 1.0 / 60.0,
            contact_capacity: 4096,
        }
    }
}

impl WorldDesc {
    /// 世界配置校验(先于任何后端初始化;违例确定性 `Err(InvalidDesc)`)。
    pub(crate) fn validate(&self) -> Result<(), PhysicsError> {
        if !self.gravity.iter().all(|g| g.is_finite()) {
            return Err(PhysicsError::InvalidDesc("gravity 分量须有限".into()));
        }
        if self.layer_count == 0 {
            return Err(PhysicsError::InvalidDesc("layer_count 须 ≥ 1".into()));
        }
        if self.max_bodies == 0 {
            return Err(PhysicsError::InvalidDesc("max_bodies 须 ≥ 1".into()));
        }
        if !self.dt_fixed.is_finite() || self.dt_fixed <= 0.0 {
            return Err(PhysicsError::InvalidDesc("dt_fixed 须为正且有限".into()));
        }
        Ok(())
    }
}

/// 刚体变换(§4.A2:与 `GpuScene` 实例 3×4 变换的唯一桥接输入;xyzw 四元数)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsTransform {
    /// 平移(世界系,米)。
    pub translation: [f32; 3],
    /// 旋转(xyzw 四元数,调用方负责单位化)。
    pub rotation: [f32; 4],
}

impl PhysicsTransform {
    /// 恒等变换(原点 + 单位四元数)。
    pub const IDENTITY: PhysicsTransform = PhysicsTransform {
        translation: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
}

impl Default for PhysicsTransform {
    fn default() -> Self {
        PhysicsTransform::IDENTITY
    }
}

/// 体类型(§4.A2 `BodyDesc.kind`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyKind {
    /// 静态(唯一可持 `ShapeDesc::StaticMesh` 的类型)。
    Static,
    /// 运动学(无限质量,由宿主驱动)。
    Kinematic,
    /// 动态(受力/冲量/睡眠)。
    Dynamic,
}

/// 单 body 语义快照(M66 canonical hash 白名单字段;RFC-0021 §4.A1)。
#[derive(Debug, Clone, PartialEq)]
pub struct BodySemantic {
    pub body_id: BodyId,
    pub kind: BodyKind,
    pub is_active: bool,
    pub layer: u32,
    pub shape_id: ShapeId,
    pub transform: PhysicsTransform,
    pub linvel: [f32; 3],
    pub angvel: [f32; 3],
}

/// 质量与材质参数(§4.A2 `mass_props`;sys 投影按字段摊平)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MassProps {
    /// 质量(kg;Dynamic 须 > 0 且有限,Static/Kinematic 忽略)。
    pub mass: f32,
    /// 摩擦系数(≥ 0 且有限)。
    pub friction: f32,
    /// 恢复系数(≥ 0 且有限)。
    pub restitution: f32,
    /// 允许睡眠(Jolt 内建睡眠默认开;睡眠体零 MV/零变换脏写,§4.A3)。
    pub allow_sleep: bool,
}

impl Default for MassProps {
    fn default() -> Self {
        MassProps {
            mass: 1.0,
            friction: 0.5,
            restitution: 0.0,
            allow_sleep: true,
        }
    }
}

/// 形状描述(§4.A2;`StaticMesh` 仅 Static 体,动态 mesh → `Err(InvalidDesc)`)。
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeDesc {
    /// 球(半径 > 0)。
    Sphere {
        /// 半径(米)。
        radius: f32,
    },
    /// 盒(半长各分量 > 0)。
    Box {
        /// 半长(米,xyz)。
        half_extents: [f32; 3],
    },
    /// 胶囊(圆柱半高 ≥ 0、半径 > 0;轴沿本地 Y)。
    Capsule {
        /// 圆柱段半高(米)。
        half_height: f32,
        /// 半径(米)。
        radius: f32,
    },
    /// 凸包(顶点非空且有限)。
    ConvexHull {
        /// 凸包点集。
        points: Vec<[f32; 3]>,
    },
    /// 三角形汤静态网格(仅 Static 体;顶点/索引非空,索引不越界)。
    StaticMesh {
        /// 顶点表。
        vertices: Vec<[f32; 3]>,
        /// 三角形索引(指向 `vertices`)。
        triangles: Vec<[u32; 3]>,
    },
}

impl ShapeDesc {
    /// 形状自身尺寸/数据校验(与体类型无关的部分)。
    pub(crate) fn validate_dims(&self) -> Result<(), PhysicsError> {
        match self {
            ShapeDesc::Sphere { radius } => positive("Sphere.radius", *radius),
            ShapeDesc::Box { half_extents } => {
                if half_extents.iter().all(|e| e.is_finite() && *e > 0.0) {
                    Ok(())
                } else {
                    Err(PhysicsError::InvalidDesc(
                        "Box.half_extents 各分量须 > 0 且有限".into(),
                    ))
                }
            }
            ShapeDesc::Capsule {
                half_height,
                radius,
            } => {
                if !half_height.is_finite() || *half_height < 0.0 {
                    return Err(PhysicsError::InvalidDesc(
                        "Capsule.half_height 须 ≥ 0 且有限".into(),
                    ));
                }
                positive("Capsule.radius", *radius)
            }
            ShapeDesc::ConvexHull { points } => {
                if points.is_empty() {
                    return Err(PhysicsError::InvalidDesc("ConvexHull.points 非空".into()));
                }
                if !points.iter().flatten().all(|c| c.is_finite()) {
                    return Err(PhysicsError::InvalidDesc(
                        "ConvexHull.points 坐标须有限".into(),
                    ));
                }
                Ok(())
            }
            ShapeDesc::StaticMesh {
                vertices,
                triangles,
            } => {
                if vertices.is_empty() || triangles.is_empty() {
                    return Err(PhysicsError::InvalidDesc(
                        "StaticMesh 顶点/三角形非空".into(),
                    ));
                }
                if !vertices.iter().flatten().all(|c| c.is_finite()) {
                    return Err(PhysicsError::InvalidDesc(
                        "StaticMesh 顶点坐标须有限".into(),
                    ));
                }
                let n = vertices.len() as u32;
                if triangles.iter().flatten().any(|i| *i >= n) {
                    return Err(PhysicsError::InvalidDesc(
                        "StaticMesh 三角形索引越界".into(),
                    ));
                }
                Ok(())
            }
        }
    }
}

fn positive(name: &str, v: f32) -> Result<(), PhysicsError> {
    if v.is_finite() && v > 0.0 {
        Ok(())
    } else {
        Err(PhysicsError::InvalidDesc(format!("{name} 须 > 0 且有限")))
    }
}

/// 体描述(§4.A2 + 初始变换[sys 边界契约 `SysBodyDesc` 投影含初始变换])。
#[derive(Debug, Clone, PartialEq)]
pub struct BodyDesc {
    /// 体类型。
    pub kind: BodyKind,
    /// 形状。
    pub shape: ShapeDesc,
    /// object layer(< `WorldDesc::layer_count`)。
    pub layer: u32,
    /// 质量/材质参数。
    pub mass_props: MassProps,
    /// CCD 开关(映射 Jolt MotionQuality Discrete/LinearCast,§4.A3)。
    pub ccd: bool,
    /// 初始变换(默认恒等)。
    pub transform: PhysicsTransform,
}

impl BodyDesc {
    /// 描述校验(先于任何后端调用;违例确定性 `Err(InvalidDesc)`,P-01)。
    pub(crate) fn validate(&self, layer_count: u32) -> Result<(), PhysicsError> {
        self.shape.validate_dims()?;
        if self.kind != BodyKind::Static && matches!(self.shape, ShapeDesc::StaticMesh { .. }) {
            return Err(PhysicsError::InvalidDesc(
                "StaticMesh 仅 Static 体(动态 mesh 不支持,§4.A2)".into(),
            ));
        }
        if self.layer >= layer_count {
            return Err(PhysicsError::InvalidDesc(format!(
                "layer {} 超出 layer_count {}",
                self.layer, layer_count
            )));
        }
        if self.kind == BodyKind::Dynamic
            && (!self.mass_props.mass.is_finite() || self.mass_props.mass <= 0.0)
        {
            return Err(PhysicsError::InvalidDesc(
                "Dynamic 体 mass 须 > 0 且有限".into(),
            ));
        }
        if !self.mass_props.friction.is_finite() || self.mass_props.friction < 0.0 {
            return Err(PhysicsError::InvalidDesc("friction 须 ≥ 0 且有限".into()));
        }
        if !self.mass_props.restitution.is_finite() || self.mass_props.restitution < 0.0 {
            return Err(PhysicsError::InvalidDesc(
                "restitution 须 ≥ 0 且有限".into(),
            ));
        }
        if !self
            .transform
            .translation
            .iter()
            .chain(self.transform.rotation.iter())
            .all(|c| c.is_finite())
        {
            return Err(PhysicsError::InvalidDesc(
                "初始变换分量须有限(rotation 应为单位四元数)".into(),
            ));
        }
        Ok(())
    }
}

/// 单步统计(§4.A1;`step_time` 仅供 evidence,不进硬门,P-09)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepStats {
    /// 本步激活 body 数。
    pub active_bodies: u32,
    /// 本步入睡 body 数。
    pub slept_this_step: u32,
    /// 本步归一化后入 ring 的接触事件数。
    pub contacts_emitted: u32,
    /// 本步确定性丢弃的接触事件数(ring 溢出丢最旧 + sys 侧丢弃 + 未知 token)。
    pub contacts_dropped: u32,
    /// 本步耗时(仅 evidence 埋点)。
    pub step_time: std::time::Duration,
}

/// 接触事件相位(§4.A5;声明序 = 规范序中的相位序)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContactPhase {
    /// 接触开始。
    Begin,
    /// 接触持续。
    Persist,
    /// 接触结束。
    End,
}

/// 接触事件(§4.A5;step 结束边界按 `(min(a,b), max(a,b), phase)` 规范序
/// 排序去重后入有界 ring——事件序列确定性 = 归一化后序列语义)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactEvent {
    /// 参与 body 之一(规范序按 `min(a,b)`/`max(a,b)` 排序,与存储序无关)。
    pub a: BodyId,
    /// 参与 body 之二。
    pub b: BodyId,
    /// 相位。
    pub phase: ContactPhase,
    /// 接触点(世界系)。
    pub contact_point: [f32; 3],
    /// 接触法线(世界系)。
    pub normal: [f32; 3],
    /// 冲量大小。
    pub impulse: f32,
}

/// 射线查询(§4.A4;step 外并发调用,Jolt 路径硬需求)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueryRay {
    /// 射线原点(世界系)。
    pub origin: [f32; 3],
    /// 射线方向(调用方负责归一化语义)。
    pub dir: [f32; 3],
    /// 参数区间下界(≤ `t_max`)。
    pub t_min: f32,
    /// 参数区间上界(有限)。
    pub t_max: f32,
    /// layer 位掩码(bit i = object layer i)。
    pub layer_mask: u64,
}

impl QueryRay {
    pub(crate) fn validate(&self) -> Result<(), PhysicsError> {
        if !self
            .origin
            .iter()
            .chain(self.dir.iter())
            .all(|c| c.is_finite())
        {
            return Err(PhysicsError::InvalidDesc(
                "QueryRay origin/dir 分量须有限".into(),
            ));
        }
        if !self.t_min.is_finite() || !self.t_max.is_finite() || self.t_min > self.t_max {
            return Err(PhysicsError::InvalidDesc(
                "QueryRay 须满足 t_min ≤ t_max 且均有限".into(),
            ));
        }
        Ok(())
    }
}

/// 形状 cast 查询(§4.A4;`dir` 为位移方向,扫掠 `[0, t_max]`)。
#[derive(Debug, Clone, PartialEq)]
pub struct QueryShape {
    /// 扫掠形状。
    pub shape: ShapeDesc,
    /// 起始变换。
    pub start: PhysicsTransform,
    /// 位移方向(调用方负责归一化语义)。
    pub dir: [f32; 3],
    /// 扫掠参数上界(≥ 0 且有限)。
    pub t_max: f32,
    /// layer 位掩码。
    pub layer_mask: u64,
}

impl QueryShape {
    pub(crate) fn validate(&self) -> Result<(), PhysicsError> {
        self.shape.validate_dims()?;
        if !self
            .start
            .translation
            .iter()
            .chain(self.start.rotation.iter())
            .chain(self.dir.iter())
            .all(|c| c.is_finite())
        {
            return Err(PhysicsError::InvalidDesc(
                "QueryShape start/dir 分量须有限".into(),
            ));
        }
        if !self.t_max.is_finite() || self.t_max < 0.0 {
            return Err(PhysicsError::InvalidDesc(
                "QueryShape.t_max 须 ≥ 0 且有限".into(),
            ));
        }
        Ok(())
    }
}

/// 查询命中(§4.A4;`shape` 由 safe 层按 body→shape 记录回填)。
/// 返回序列按 `(t, BodyId)` 规范序排序(C-2:排序后序列 = 确定性面)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueryHit {
    /// 命中 body。
    pub body: BodyId,
    /// 命中参数(射线/扫掠参数)。
    pub t: f32,
    /// 命中点(世界系)。
    pub position: [f32; 3],
    /// 命中法线(世界系)。
    pub normal: [f32; 3],
    /// 命中 body 的 shape(body→ShapeId 记录回填)。
    pub shape: ShapeId,
}

/// overlap 命中(§4.A4;无扫掠参数,规范序 = `BodyId` 升序)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlapHit {
    /// 命中 body。
    pub body: BodyId,
    /// 命中 body 的 shape(body→ShapeId 记录回填)。
    pub shape: ShapeId,
}
