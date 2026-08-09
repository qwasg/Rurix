//! CanonicalPhysicsState 二进制前像 + SHA-256(RFC-0021 §4.A1)。

use rurix_pkg::sha256::{digest, hex};

use crate::id::{BodyId, ShapeId};
use crate::types::{BodyKind, BodySemantic, ContactEvent, ContactPhase, PhysicsTransform};
use crate::world::PhysicsWorld;

/// capture 面错误(fail-closed;不 panic)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureError {
    NanFloat { path: String },
    Io(String),
    Parse(String),
    Mismatch(String),
    Rejected(String),
    Backend(String),
}

impl std::fmt::Display for CaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureError::NanFloat { path } => write!(f, "NaN fail-closed at {path}"),
            CaptureError::Io(m)
            | CaptureError::Parse(m)
            | CaptureError::Mismatch(m)
            | CaptureError::Rejected(m)
            | CaptureError::Backend(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for CaptureError {}

/// 浮点 canonical 化:−0 → +0;NaN → Err。
pub fn canon_f32_bits(v: f32) -> Result<u32, CaptureError> {
    canon_f32_bits_at(v, "f32")
}

pub fn canon_f32_bits_at(v: f32, path: &str) -> Result<u32, CaptureError> {
    let bits = v.to_bits();
    let exp = bits & 0x7f80_0000;
    let frac = bits & 0x007f_ffff;
    if exp == 0x7f80_0000 && frac != 0 {
        return Err(CaptureError::NanFloat {
            path: path.to_string(),
        });
    }
    if bits == 0x8000_0000 {
        return Ok(0);
    }
    Ok(bits)
}

fn push_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn push_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// 约束语义快照(hinge v1;`type`=1)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintSemantic {
    pub id: u64,
    pub ctype: u8,
    pub enabled: bool,
    pub motor_state: u16,
    pub body_a: u64,
    pub body_b: u64,
    pub param_digest: u64,
}

/// CanonicalPhysicsState v1。
#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalPhysicsState {
    pub tick: u64,
    pub bodies: Vec<BodySemantic>,
    pub constraints: Vec<ConstraintSemantic>,
}

impl CanonicalPhysicsState {
    pub fn encode_preimage(&self) -> Result<Vec<u8>, CaptureError> {
        let mut buf = Vec::with_capacity(64 + self.bodies.len() * 96);
        buf.extend_from_slice(b"RXPS");
        push_u16(&mut buf, 1); // version
        push_u64(&mut buf, self.tick);
        push_u32(&mut buf, self.bodies.len() as u32);
        let mut bodies = self.bodies.clone();
        bodies.sort_by_key(|b| b.body_id);
        for b in &bodies {
            push_u64(&mut buf, b.body_id.to_bits());
            buf.push(match b.kind {
                BodyKind::Static => 0,
                BodyKind::Kinematic => 1,
                BodyKind::Dynamic => 2,
            });
            buf.push(u8::from(b.is_active));
            push_u16(&mut buf, 0); // pad
            push_u32(&mut buf, b.layer);
            push_u64(&mut buf, b.shape_id.to_bits());
            for (i, c) in b.transform.translation.iter().enumerate() {
                push_u32(&mut buf, canon_f32_bits_at(*c, &format!("body.pos[{i}]"))?);
            }
            for (i, c) in b.transform.rotation.iter().enumerate() {
                push_u32(&mut buf, canon_f32_bits_at(*c, &format!("body.rot[{i}]"))?);
            }
            for (i, c) in b.linvel.iter().enumerate() {
                push_u32(
                    &mut buf,
                    canon_f32_bits_at(*c, &format!("body.linvel[{i}]"))?,
                );
            }
            for (i, c) in b.angvel.iter().enumerate() {
                push_u32(
                    &mut buf,
                    canon_f32_bits_at(*c, &format!("body.angvel[{i}]"))?,
                );
            }
        }
        push_u32(&mut buf, self.constraints.len() as u32);
        let mut cons = self.constraints.clone();
        cons.sort_by_key(|c| c.id);
        for c in &cons {
            push_u64(&mut buf, c.id);
            buf.push(c.ctype);
            buf.push(u8::from(c.enabled));
            push_u16(&mut buf, c.motor_state);
            push_u64(&mut buf, c.body_a);
            push_u64(&mut buf, c.body_b);
            push_u64(&mut buf, c.param_digest);
        }
        // character/destruction/vehicle counts = 0 (version bump later)
        push_u32(&mut buf, 0);
        push_u32(&mut buf, 0);
        push_u32(&mut buf, 0);
        Ok(buf)
    }

    pub fn to_diagnostic_json(&self) -> Result<String, CaptureError> {
        let mut s = String::from("{\n  \"tick\": ");
        s.push_str(&self.tick.to_string());
        s.push_str(",\n  \"bodies\": [\n");
        let mut bodies = self.bodies.clone();
        bodies.sort_by_key(|b| b.body_id);
        for (i, b) in bodies.iter().enumerate() {
            if i > 0 {
                s.push_str(",\n");
            }
            s.push_str("    {");
            s.push_str(&format!(
                "\"body_id\":\"{:016x}\",\"kind\":{},\"is_active\":{},\"layer\":{},\"shape_id\":\"{:016x}\"",
                b.body_id.to_bits(),
                match b.kind {
                    BodyKind::Static => 0,
                    BodyKind::Kinematic => 1,
                    BodyKind::Dynamic => 2,
                },
                b.is_active,
                b.layer,
                b.shape_id.to_bits()
            ));
            s.push_str(",\"pos\":[");
            for (j, c) in b.transform.translation.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                s.push_str(&format!("\"{:08x}\"", canon_f32_bits_at(*c, "pos")?));
            }
            s.push_str("],\"rot\":[");
            for (j, c) in b.transform.rotation.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                s.push_str(&format!("\"{:08x}\"", canon_f32_bits_at(*c, "rot")?));
            }
            s.push_str("],\"linvel\":[");
            for (j, c) in b.linvel.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                s.push_str(&format!("\"{:08x}\"", canon_f32_bits_at(*c, "linvel")?));
            }
            s.push_str("],\"angvel\":[");
            for (j, c) in b.angvel.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                s.push_str(&format!("\"{:08x}\"", canon_f32_bits_at(*c, "angvel")?));
            }
            s.push_str("]}");
        }
        s.push_str("\n  ],\n  \"constraints\": [\n");
        let mut cons = self.constraints.clone();
        cons.sort_by_key(|c| c.id);
        for (i, c) in cons.iter().enumerate() {
            if i > 0 {
                s.push_str(",\n");
            }
            s.push_str(&format!(
                "    {{\"id\":{},\"type\":{},\"enabled\":{},\"motor_state\":{},\"body_a\":\"{:016x}\",\"body_b\":\"{:016x}\",\"param_digest\":\"{:016x}\"}}",
                c.id, c.ctype, c.enabled, c.motor_state, c.body_a, c.body_b, c.param_digest
            ));
        }
        s.push_str("\n  ]\n}\n");
        Ok(s)
    }
}

pub fn hash_canonical_state(state: &CanonicalPhysicsState) -> Result<String, CaptureError> {
    let pre = state.encode_preimage()?;
    Ok(hex(&digest(&pre)))
}

/// 接触事件序列 digest(规范序已由 drain 保证)。
pub fn event_digest(events: &[ContactEvent]) -> Result<String, CaptureError> {
    let mut buf = Vec::with_capacity(events.len() * 48);
    for (idx, e) in events.iter().enumerate() {
        let (a, b) = if e.a <= e.b {
            (e.a.to_bits(), e.b.to_bits())
        } else {
            (e.b.to_bits(), e.a.to_bits())
        };
        push_u64(&mut buf, a);
        push_u64(&mut buf, b);
        buf.push(match e.phase {
            ContactPhase::Begin => 0,
            ContactPhase::Persist => 1,
            ContactPhase::End => 2,
        });
        for (i, c) in e.contact_point.iter().enumerate() {
            push_u32(
                &mut buf,
                canon_f32_bits_at(*c, &format!("evt[{idx}].point[{i}]"))?,
            );
        }
        for (i, c) in e.normal.iter().enumerate() {
            push_u32(
                &mut buf,
                canon_f32_bits_at(*c, &format!("evt[{idx}].normal[{i}]"))?,
            );
        }
        push_u32(
            &mut buf,
            canon_f32_bits_at(e.impulse, &format!("evt[{idx}].impulse"))?,
        );
    }
    Ok(hex(&digest(&buf)))
}

pub fn state_from_world(
    world: &PhysicsWorld,
    tick: u64,
) -> Result<CanonicalPhysicsState, CaptureError> {
    let bodies = world
        .body_semantic_snapshot()
        .map_err(|e| CaptureError::Backend(e.to_string()))?;
    let mut constraints: Vec<ConstraintSemantic> = world
        .constraint_snapshot()
        .into_iter()
        .map(|(id, a, b, enabled, motor)| ConstraintSemantic {
            id,
            ctype: 1, // hinge
            enabled,
            motor_state: motor as u16,
            body_a: a,
            body_b: b,
            param_digest: 0,
        })
        .collect();
    constraints.sort_by_key(|c| c.id);
    Ok(CanonicalPhysicsState {
        tick,
        bodies,
        constraints,
    })
}

/// 诊断用:从 JSON 不全量反序列化(仅 tick/body 计数校验时用 hex 位模式重写路径)。
pub fn empty_state(tick: u64) -> CanonicalPhysicsState {
    CanonicalPhysicsState {
        tick,
        bodies: Vec::new(),
        constraints: Vec::new(),
    }
}

/// 供 inject 写回位姿。
pub fn transform_with_pos_bits(
    t: PhysicsTransform,
    axis: usize,
    bits: u32,
) -> Result<PhysicsTransform, CaptureError> {
    let mut out = t;
    if axis >= 3 {
        return Err(CaptureError::Rejected("pos axis out of range".into()));
    }
    out.translation[axis] = f32::from_bits(bits);
    Ok(out)
}

pub fn body_kind_byte(k: BodyKind) -> u8 {
    match k {
        BodyKind::Static => 0,
        BodyKind::Kinematic => 1,
        BodyKind::Dynamic => 2,
    }
}

pub fn shape_id_bits(s: ShapeId) -> u64 {
    s.to_bits()
}

pub fn body_id_bits(b: BodyId) -> u64 {
    b.to_bits()
}
