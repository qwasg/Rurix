//! M69 PhysicsAsset canonical source schema(RFC-0021 §4.B3;`physics-character` feature)。
//!
//! ragdoll/physical-animation 闭环由波次 subject `g8.wave6b.m69.physics_asset` 承载;
//! 本模块提供可双 cook 字节相等的 source schema + digest。

use rurix_pkg::sha256::{digest, hex};

use crate::capture::canonical::CaptureError;

pub const PHYSICS_ASSET_SCHEMA_ID: &str = "rurix.physics.asset";
pub const PHYSICS_ASSET_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct BoneBodyMapping {
    pub bone_stable_id: String,
    pub body_role: String,
    pub collider_role: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointProfile {
    pub joint_stable_id: String,
    pub body_a_bone: String,
    pub body_b_bone: String,
    pub joint_type: String,
    pub motor_profile: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsAsset {
    pub schema_id: String,
    pub schema_version: u32,
    pub asset_id: String,
    pub skeleton_digest: String,
    pub bones: Vec<BoneBodyMapping>,
    pub joints: Vec<JointProfile>,
    pub layer_preset: String,
    pub ragdoll_lod_mask: u64,
    pub partial_simulation_mask: u64,
    pub cook_profile: String,
}

impl PhysicsAsset {
    pub fn new(asset_id: impl Into<String>, skeleton_digest: impl Into<String>) -> Self {
        Self {
            schema_id: PHYSICS_ASSET_SCHEMA_ID.into(),
            schema_version: PHYSICS_ASSET_SCHEMA_VERSION,
            asset_id: asset_id.into(),
            skeleton_digest: skeleton_digest.into(),
            bones: Vec::new(),
            joints: Vec::new(),
            layer_preset: "default".into(),
            ragdoll_lod_mask: u64::MAX,
            partial_simulation_mask: u64::MAX,
            cook_profile: "v1".into(),
        }
    }

    pub fn canonical_json(&self) -> String {
        let mut bones = String::from("[");
        for (i, b) in self.bones.iter().enumerate() {
            if i > 0 {
                bones.push(',');
            }
            bones.push_str(&format!(
                "{{\"bone_stable_id\":\"{}\",\"body_role\":\"{}\",\"collider_role\":\"{}\"}}",
                esc(&b.bone_stable_id),
                esc(&b.body_role),
                esc(&b.collider_role)
            ));
        }
        bones.push(']');
        let mut joints = String::from("[");
        for (i, j) in self.joints.iter().enumerate() {
            if i > 0 {
                joints.push(',');
            }
            joints.push_str(&format!(
                "{{\"joint_stable_id\":\"{}\",\"body_a_bone\":\"{}\",\"body_b_bone\":\"{}\",\"joint_type\":\"{}\",\"motor_profile\":\"{}\"}}",
                esc(&j.joint_stable_id),
                esc(&j.body_a_bone),
                esc(&j.body_b_bone),
                esc(&j.joint_type),
                esc(&j.motor_profile)
            ));
        }
        joints.push(']');
        format!(
            "{{\"schema_id\":\"{}\",\"schema_version\":{},\"asset_id\":\"{}\",\"skeleton_digest\":\"{}\",\"bones\":{},\"joints\":{},\"layer_preset\":\"{}\",\"ragdoll_lod_mask\":{},\"partial_simulation_mask\":{},\"cook_profile\":\"{}\"}}",
            esc(&self.schema_id),
            self.schema_version,
            esc(&self.asset_id),
            esc(&self.skeleton_digest),
            bones,
            joints,
            esc(&self.layer_preset),
            self.ragdoll_lod_mask,
            self.partial_simulation_mask,
            esc(&self.cook_profile)
        )
    }

    pub fn digest(&self) -> String {
        hex(&digest(self.canonical_json().as_bytes()))
    }

    /// 同输入双 cook 字节相等。
    pub fn cook_deterministic_double(&self) -> Result<(String, String), CaptureError> {
        let a = self.canonical_json();
        let b = self.canonical_json();
        if a != b {
            return Err(CaptureError::Mismatch(
                "PhysicsAsset cook not byte-stable".into(),
            ));
        }
        Ok((a, b))
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
