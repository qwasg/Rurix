//! 故障注入(F-12):tick T step 之前翻转白名单字段单 bit。

use super::canonical::{CaptureError, canon_f32_bits_at};
use crate::id::BodyId;
use crate::types::PhysicsTransform;
use crate::world::PhysicsWorld;

#[derive(Debug, Clone)]
pub struct InjectRequest {
    pub tick: u64,
    pub body: BodyId,
    pub field: String,
    pub bit: u8,
}

/// 白名单外字段 → 确定性拒绝。
pub fn whitelist_reject(field: &str) -> Result<(), CaptureError> {
    const OK: &[&str] = &[
        "pos.x", "pos.y", "pos.z", "rot.x", "rot.y", "rot.z", "rot.w", "linvel.x", "linvel.y",
        "linvel.z", "angvel.x", "angvel.y", "angvel.z",
    ];
    if OK.contains(&field) {
        Ok(())
    } else {
        Err(CaptureError::Rejected(format!(
            "non-whitelist injection field {field}"
        )))
    }
}

/// 仅当 `current_tick == req.tick` 时写入;否则 no-op。
pub fn inject_before_tick(
    world: &mut PhysicsWorld,
    req: &InjectRequest,
    current_tick: u64,
) -> Result<(), CaptureError> {
    if current_tick != req.tick {
        return Ok(());
    }
    whitelist_reject(&req.field)?;
    if req.bit >= 32 {
        return Err(CaptureError::Rejected("bit must be < 32".into()));
    }
    let body = req.body;
    let t = world
        .body_transform(body)
        .map_err(|e| CaptureError::Backend(e.to_string()))?;
    let (lin, ang) = world
        .body_velocities(body)
        .map_err(|e| CaptureError::Backend(e.to_string()))?;

    let flip = |v: f32, path: &str| -> Result<f32, CaptureError> {
        let bits = canon_f32_bits_at(v, path)?;
        Ok(f32::from_bits(bits ^ (1u32 << req.bit)))
    };

    match req.field.as_str() {
        "pos.x" | "pos.y" | "pos.z" => {
            let axis = match req.field.as_str() {
                "pos.x" => 0,
                "pos.y" => 1,
                _ => 2,
            };
            let mut nt = t;
            nt.translation[axis] = flip(t.translation[axis], &req.field)?;
            world
                .set_position_rotation_dont_activate(body, nt)
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
        }
        "rot.x" | "rot.y" | "rot.z" | "rot.w" => {
            let axis = match req.field.as_str() {
                "rot.x" => 0,
                "rot.y" => 1,
                "rot.z" => 2,
                _ => 3,
            };
            let mut nt = PhysicsTransform {
                translation: t.translation,
                rotation: t.rotation,
            };
            nt.rotation[axis] = flip(t.rotation[axis], &req.field)?;
            world
                .set_position_rotation_dont_activate(body, nt)
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
        }
        "linvel.x" | "linvel.y" | "linvel.z" => {
            let axis = match req.field.as_str() {
                "linvel.x" => 0,
                "linvel.y" => 1,
                _ => 2,
            };
            let mut nl = lin;
            nl[axis] = flip(lin[axis], &req.field)?;
            world
                .set_linear_velocity(body, nl)
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
        }
        "angvel.x" | "angvel.y" | "angvel.z" => {
            let axis = match req.field.as_str() {
                "angvel.x" => 0,
                "angvel.y" => 1,
                _ => 2,
            };
            let mut na = ang;
            na[axis] = flip(ang[axis], &req.field)?;
            world
                .set_angular_velocity(body, na)
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
        }
        other => return Err(CaptureError::Rejected(format!("unreachable {other}"))),
    }
    Ok(())
}
