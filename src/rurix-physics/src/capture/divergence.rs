//! Divergence 定位器:两帧 canonical 状态字段 diff。

use super::canonical::{
    CanonicalPhysicsState, CaptureError, ConstraintSemantic, canon_f32_bits_at,
    hash_canonical_state,
};
use crate::types::BodySemantic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDiff {
    pub path: String,
    pub stable_id: String,
    pub expected_bits: u32,
    pub actual_bits: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DivergenceLocate {
    pub first_divergence_tick: u64,
    pub diffs: Vec<FieldDiff>,
}

/// 比较两帧;相等 → None。
pub fn locate_divergence(
    expected: &CanonicalPhysicsState,
    actual: &CanonicalPhysicsState,
) -> Result<Option<DivergenceLocate>, CaptureError> {
    if hash_canonical_state(expected)? == hash_canonical_state(actual)? {
        return Ok(None);
    }
    let mut diffs = diff_bodies(&expected.bodies, &actual.bodies)?;
    diffs.extend(diff_constraints(&expected.constraints, &actual.constraints));
    if diffs.is_empty() {
        Ok(None)
    } else {
        Ok(Some(DivergenceLocate {
            first_divergence_tick: expected.tick.min(actual.tick),
            diffs,
        }))
    }
}

fn diff_bodies(
    expected: &[BodySemantic],
    actual: &[BodySemantic],
) -> Result<Vec<FieldDiff>, CaptureError> {
    let mut diffs = Vec::new();
    if expected.len() != actual.len() {
        diffs.push(FieldDiff {
            path: "body_count".into(),
            stable_id: String::new(),
            expected_bits: expected.len() as u32,
            actual_bits: actual.len() as u32,
        });
    }
    let n = expected.len().min(actual.len());
    for i in 0..n {
        diff_body(&expected[i], &actual[i], &mut diffs)?;
    }
    Ok(diffs)
}

fn diff_body(
    e: &BodySemantic,
    a: &BodySemantic,
    out: &mut Vec<FieldDiff>,
) -> Result<(), CaptureError> {
    let id = format!("{:016x}", e.body_id.to_bits());
    if e.body_id != a.body_id {
        out.push(FieldDiff {
            path: "body_id".into(),
            stable_id: id.clone(),
            expected_bits: e.body_id.index(),
            actual_bits: a.body_id.index(),
        });
    }
    if e.is_active != a.is_active {
        out.push(FieldDiff {
            path: "flags.is_active".into(),
            stable_id: id.clone(),
            expected_bits: u32::from(e.is_active),
            actual_bits: u32::from(a.is_active),
        });
    }
    for (name, ev, av) in [
        (
            "pos.x",
            e.transform.translation[0],
            a.transform.translation[0],
        ),
        (
            "pos.y",
            e.transform.translation[1],
            a.transform.translation[1],
        ),
        (
            "pos.z",
            e.transform.translation[2],
            a.transform.translation[2],
        ),
        ("linvel.x", e.linvel[0], a.linvel[0]),
        ("linvel.y", e.linvel[1], a.linvel[1]),
        ("linvel.z", e.linvel[2], a.linvel[2]),
        ("angvel.x", e.angvel[0], a.angvel[0]),
        ("angvel.y", e.angvel[1], a.angvel[1]),
        ("angvel.z", e.angvel[2], a.angvel[2]),
    ] {
        let eb = canon_f32_bits_at(ev, name)?;
        let ab = canon_f32_bits_at(av, name)?;
        if eb != ab {
            out.push(FieldDiff {
                path: name.into(),
                stable_id: id.clone(),
                expected_bits: eb,
                actual_bits: ab,
            });
        }
    }
    Ok(())
}

fn diff_constraints(
    expected: &[ConstraintSemantic],
    actual: &[ConstraintSemantic],
) -> Vec<FieldDiff> {
    let mut diffs = Vec::new();
    if expected.len() != actual.len() {
        diffs.push(FieldDiff {
            path: "constraint_count".into(),
            stable_id: String::new(),
            expected_bits: expected.len() as u32,
            actual_bits: actual.len() as u32,
        });
    }
    let n = expected.len().min(actual.len());
    for i in 0..n {
        let e = &expected[i];
        let a = &actual[i];
        if e.motor_state != a.motor_state {
            diffs.push(FieldDiff {
                path: "motor_state".into(),
                stable_id: format!("{}", e.id),
                expected_bits: u32::from(e.motor_state),
                actual_bits: u32::from(a.motor_state),
            });
        }
    }
    diffs
}
