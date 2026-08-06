//! NetworkPhysicsFrameId / PhysicsTickId / FrameDomainMap(RFC-0021 §3.1)。

use super::NetError;

/// 网络物理帧 ID(权威修正/输入 history/rollback 序列)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkPhysicsFrameId(pub u64);

/// 物理固定步 tick ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicsTickId(pub u64);

/// 跨域显式映射(capture/evidence 必录;隐式"当前帧"拒绝)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDomainMap {
    pub net_frame: NetworkPhysicsFrameId,
    pub physics_tick: PhysicsTickId,
    pub game_frame: Option<u64>,
    pub render_frame: Option<u64>,
}

impl FrameDomainMap {
    pub fn rigid_only(net_frame: NetworkPhysicsFrameId, physics_tick: PhysicsTickId) -> Self {
        Self {
            net_frame,
            physics_tick,
            game_frame: None,
            render_frame: None,
        }
    }

    pub fn validate(&self) -> Result<(), NetError> {
        if self.net_frame.0 != self.physics_tick.0 {
            // v1 fixture:1:1 映射;混用其它域须显式字段,不得静默换算。
            return Err(NetError::Rejected(format!(
                "frame domain mismatch net={} tick={}",
                self.net_frame.0, self.physics_tick.0
            )));
        }
        Ok(())
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"net_frame\":{},\"physics_tick\":{},\"game_frame\":{},\"render_frame\":{}}}",
            self.net_frame.0,
            self.physics_tick.0,
            opt_u64(self.game_frame),
            opt_u64(self.render_frame),
        )
    }
}

fn opt_u64(v: Option<u64>) -> String {
    match v {
        Some(x) => x.to_string(),
        None => "null".into(),
    }
}
