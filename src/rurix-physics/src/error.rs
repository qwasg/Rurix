//! 库层错误枚举(RFC-0017 §5.1:零新 RX 码;`PhysicsError` 不属 §4.0-3 冻结面,
//! 可追加非 RX 变体)。所有错误路径确定性 `Err`,不 panic、不静默回退(P-01)。

use std::fmt;

use crate::id::BodyId;
use crate::types::BackendKind;

/// 物理库错误(safe 面唯一错误类型;sys 上岸错误在 world.rs〔§4.C4 v1.2 crate 内
/// 唯一 sanctioned sys 消费模块〕映射为本枚举,sys 类型不进 safe 公共 API)。
#[derive(Debug, Clone, PartialEq)]
pub enum PhysicsError {
    /// 请求的后端未编译进本构建(`--no-default-features` 下的 Jolt;Rapier 在
    /// G6.4 实现前一律走此路径,RFC-0017 §4.0-7 / §4.D1)。
    BackendNotCompiled(BackendKind),
    /// 后端已编译但初始化/运行失败(JoltC 构建缺失等;fail-closed,§4.C1)。
    BackendUnavailable(String),
    /// body/shape index 池耗尽(`WorldDesc::max_bodies` 上限 + 退休槽位,§4.A2)。
    PoolExhausted,
    /// 无效 `BodyId`(未创建 / 已移除 / generation 失配)二次使用(§4.A2/§4.C3)。
    InvalidBody(BodyId),
    /// 描述非法(动态 StaticMesh / 尺寸越界 / layer 超 `layer_count` / 索引越界等)。
    InvalidDesc(String),
    /// `step(dt)` 与 `WorldDesc::dt_fixed` 不一致(固定步纪律,§4.A1;位级精确比较)。
    FixedStepMismatch {
        /// 世界配置的固定步长。
        expected: f32,
        /// 调用方实传入的步长。
        got: f32,
    },
    /// 预算饱和硬错误变体(保留:当前查询/事件面默认语义为确定性截断 + 饱和计数,
    /// 不走 Err;供后续需要硬失败的面使用,§4.A6)。
    BudgetSaturated,
}

impl fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhysicsError::BackendNotCompiled(kind) => {
                write!(f, "后端 {kind} 未编译进本构建")
            }
            PhysicsError::BackendUnavailable(msg) => write!(f, "后端不可用:{msg}"),
            PhysicsError::PoolExhausted => write!(f, "body/shape index 池耗尽"),
            PhysicsError::InvalidBody(id) => write!(f, "无效 BodyId {id}(未创建/已移除)"),
            PhysicsError::InvalidDesc(msg) => write!(f, "描述非法:{msg}"),
            PhysicsError::FixedStepMismatch { expected, got } => {
                write!(f, "固定步长失配:配置 {expected},实传 {got}")
            }
            PhysicsError::BudgetSaturated => write!(f, "SyncBudget 饱和"),
        }
    }
}

impl std::error::Error for PhysicsError {}
