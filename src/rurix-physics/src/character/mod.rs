//! M71 RurixCharacter 版本化状态块(RFC-0021 §4.B2;`physics-character` feature)。
//!
//! 自研 collide-and-slide 产品层;不要求与 Jolt CharacterVirtual 对拍。
//! 完整运动学闭环由波次 subject `g8.wave6b.m71.character_virtual` 承载;
//! 本模块为 M67 波次并行分支提供可 capture/rollback 的状态 schema。

use crate::capture::canonical::{canon_f32_bits_at, CaptureError};
use crate::id::BodyId;
use crate::types::PhysicsTransform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GroundState {
    OnGround = 0,
    OnSteepGround = 1,
    NotSupported = 2,
    InAir = 3,
}

/// CharacterVirtualState 独立版本化状态块(§4.B2 最小字段集)。
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterVirtualState {
    pub schema_version: u32,
    pub character_id: u64,
    pub transform: PhysicsTransform,
    pub linear_velocity: [f32; 3],
    pub ground_state: GroundState,
    pub ground_body: Option<BodyId>,
    pub support_normal: [f32; 3],
    pub stair_slope_active: bool,
    pub platform_relative: PhysicsTransform,
    pub user_state: u64,
}

impl CharacterVirtualState {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn new(character_id: u64, transform: PhysicsTransform) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            character_id,
            transform,
            linear_velocity: [0.0; 3],
            ground_state: GroundState::InAir,
            ground_body: None,
            support_normal: [0.0, 1.0, 0.0],
            stair_slope_active: false,
            platform_relative: PhysicsTransform::IDENTITY,
            user_state: 0,
        }
    }

    /// 参与 semantic hash 的 canonical 前像片段。
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CaptureError> {
        let mut buf = Vec::with_capacity(128);
        buf.extend_from_slice(&self.schema_version.to_le_bytes());
        buf.extend_from_slice(&self.character_id.to_le_bytes());
        for v in self.transform.translation {
            buf.extend_from_slice(&canon_f32_bits_at(v, "char.t")?.to_le_bytes());
        }
        for v in self.transform.rotation {
            buf.extend_from_slice(&canon_f32_bits_at(v, "char.r")?.to_le_bytes());
        }
        for v in self.linear_velocity {
            buf.extend_from_slice(&canon_f32_bits_at(v, "char.v")?.to_le_bytes());
        }
        buf.push(self.ground_state as u8);
        buf.extend_from_slice(
            &self
                .ground_body
                .map(|b| b.to_bits())
                .unwrap_or(0)
                .to_le_bytes(),
        );
        Ok(buf)
    }
}

/// 最小角色控制器句柄(状态可捕获;运动学闭环见 M71 subject)。
#[derive(Debug, Clone, PartialEq)]
pub struct RurixCharacter {
    pub state: CharacterVirtualState,
}

impl RurixCharacter {
    pub fn new(character_id: u64, transform: PhysicsTransform) -> Self {
        Self {
            state: CharacterVirtualState::new(character_id, transform),
        }
    }
}
