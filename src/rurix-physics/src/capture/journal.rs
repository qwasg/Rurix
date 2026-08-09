//! journal.jsonl 命令与每 tick 行。

use super::canonical::{CaptureError, canon_f32_bits_at};
use crate::id::BodyId;
use crate::types::{BodyDesc, BodyKind, MassProps, PhysicsTransform, ShapeDesc};

#[derive(Debug, Clone, PartialEq)]
pub struct PostTick {
    pub semantic_state_hash: String,
    pub event_digest: String,
    pub contacts_emitted: u32,
    pub contacts_dropped: u64,
    pub ring_backlog: u32,
    pub saturation_query_casts: u64,
    pub saturation_contact_events: u64,
    pub saturation_body_writes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JournalCommand {
    CreateBodies {
        descs: Vec<BodyDesc>,
        assigned_ids: Vec<u64>,
    },
    RemoveBodies {
        ids: Vec<u64>,
    },
    ApplyImpulse {
        body: u64,
        impulse: [f32; 3],
    },
    SetVelocity {
        body: u64,
        linear: [f32; 3],
        angular: [f32; 3],
    },
    MoveKinematic {
        body: u64,
        transform: PhysicsTransform,
    },
    PageResident {
        page_resource: u32,
        page: u32,
        descs: Vec<BodyDesc>,
        assigned_ids: Vec<u64>,
    },
    PageUnload {
        page_resource: u32,
        page: u32,
        receipt_bodies: Vec<u64>,
    },
    AddConstraint {
        ctype: u8,
        body_a: u64,
        body_b: u64,
        point: [f32; 3],
        hinge_axis: [f32; 3],
        normal_axis: [f32; 3],
        assigned_id: u64,
    },
    RemoveConstraint {
        id: u64,
    },
    SetMotor {
        id: u64,
        state: u32,
        target: f32,
    },
    QueryRay {
        origin: [f32; 3],
        dir: [f32; 3],
        t_min: f32,
        t_max: f32,
        layer_mask: u64,
        expected_hits: Vec<(u64, u32)>, // (body_bits, t_bits)
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalTick {
    pub tick: u64,
    pub pre: Vec<JournalCommand>,
    pub post: PostTick,
}

fn fhex(v: f32, path: &str) -> Result<String, CaptureError> {
    Ok(format!("{:08x}", canon_f32_bits_at(v, path)?))
}

fn write_transform(t: &PhysicsTransform) -> Result<String, CaptureError> {
    Ok(format!(
        "{{\"t\":[\"{}\",\"{}\",\"{}\"],\"r\":[\"{}\",\"{}\",\"{}\",\"{}\"]}}",
        fhex(t.translation[0], "t0")?,
        fhex(t.translation[1], "t1")?,
        fhex(t.translation[2], "t2")?,
        fhex(t.rotation[0], "r0")?,
        fhex(t.rotation[1], "r1")?,
        fhex(t.rotation[2], "r2")?,
        fhex(t.rotation[3], "r3")?,
    ))
}

fn write_shape(s: &ShapeDesc) -> Result<String, CaptureError> {
    match s {
        ShapeDesc::Sphere { radius } => Ok(format!(
            "{{\"Sphere\":{{\"radius\":\"{}\"}}}}",
            fhex(*radius, "radius")?
        )),
        ShapeDesc::Box { half_extents } => Ok(format!(
            "{{\"Box\":{{\"half_extents\":[\"{}\",\"{}\",\"{}\"]}}}}",
            fhex(half_extents[0], "hx")?,
            fhex(half_extents[1], "hy")?,
            fhex(half_extents[2], "hz")?,
        )),
        ShapeDesc::Capsule {
            half_height,
            radius,
        } => Ok(format!(
            "{{\"Capsule\":{{\"half_height\":\"{}\",\"radius\":\"{}\"}}}}",
            fhex(*half_height, "hh")?,
            fhex(*radius, "cr")?
        )),
        ShapeDesc::ConvexHull { points } => {
            let mut s = String::from("{\"ConvexHull\":{\"points\":[");
            for (i, p) in points.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                s.push_str(&format!(
                    "[\"{}\",\"{}\",\"{}\"]",
                    fhex(p[0], "p0")?,
                    fhex(p[1], "p1")?,
                    fhex(p[2], "p2")?
                ));
            }
            s.push_str("]}}");
            Ok(s)
        }
        ShapeDesc::StaticMesh { .. } => Err(CaptureError::Rejected(
            "StaticMesh journal encode not in M66 v1 corpus path".into(),
        )),
    }
}

fn write_desc(d: &BodyDesc) -> Result<String, CaptureError> {
    let kind = match d.kind {
        BodyKind::Static => 0,
        BodyKind::Kinematic => 1,
        BodyKind::Dynamic => 2,
    };
    Ok(format!(
        "{{\"kind\":{},\"shape\":{},\"layer\":{},\"mass\":\"{}\",\"friction\":\"{}\",\"restitution\":\"{}\",\"allow_sleep\":{},\"ccd\":{},\"transform\":{}}}",
        kind,
        write_shape(&d.shape)?,
        d.layer,
        fhex(d.mass_props.mass, "mass")?,
        fhex(d.mass_props.friction, "fric")?,
        fhex(d.mass_props.restitution, "rest")?,
        d.mass_props.allow_sleep,
        d.ccd,
        write_transform(&d.transform)?,
    ))
}

fn write_ids(ids: &[u64]) -> String {
    let mut s = String::from("[");
    for (i, id) in ids.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("\"{id:016x}\""));
    }
    s.push(']');
    s
}

fn write_cmd(cmd: &JournalCommand) -> Result<String, CaptureError> {
    match cmd {
        JournalCommand::CreateBodies {
            descs,
            assigned_ids,
        } => {
            let mut ds = String::from("[");
            for (i, d) in descs.iter().enumerate() {
                if i > 0 {
                    ds.push(',');
                }
                ds.push_str(&write_desc(d)?);
            }
            ds.push(']');
            Ok(format!(
                "{{\"create_bodies\":{{\"descs\":{ds},\"assigned_ids\":{}}}}}",
                write_ids(assigned_ids)
            ))
        }
        JournalCommand::RemoveBodies { ids } => Ok(format!(
            "{{\"remove_bodies\":{{\"ids\":{}}}}}",
            write_ids(ids)
        )),
        JournalCommand::ApplyImpulse { body, impulse } => Ok(format!(
            "{{\"apply_impulse\":{{\"body\":\"{body:016x}\",\"impulse\":[\"{}\",\"{}\",\"{}\"]}}}}",
            fhex(impulse[0], "i0")?,
            fhex(impulse[1], "i1")?,
            fhex(impulse[2], "i2")?,
        )),
        JournalCommand::SetVelocity {
            body,
            linear,
            angular,
        } => Ok(format!(
            "{{\"set_velocity\":{{\"body\":\"{body:016x}\",\"linear\":[\"{}\",\"{}\",\"{}\"],\"angular\":[\"{}\",\"{}\",\"{}\"]}}}}",
            fhex(linear[0], "l0")?,
            fhex(linear[1], "l1")?,
            fhex(linear[2], "l2")?,
            fhex(angular[0], "a0")?,
            fhex(angular[1], "a1")?,
            fhex(angular[2], "a2")?,
        )),
        JournalCommand::MoveKinematic { body, transform } => Ok(format!(
            "{{\"move_kinematic\":{{\"body\":\"{body:016x}\",\"transform\":{}}}}}",
            write_transform(transform)?
        )),
        JournalCommand::PageResident {
            page_resource,
            page,
            descs,
            assigned_ids,
        } => {
            let mut ds = String::from("[");
            for (i, d) in descs.iter().enumerate() {
                if i > 0 {
                    ds.push(',');
                }
                ds.push_str(&write_desc(d)?);
            }
            ds.push(']');
            Ok(format!(
                "{{\"page_resident\":{{\"page_key\":[{page_resource},{page}],\"descs\":{ds},\"assigned_ids\":{}}}}}",
                write_ids(assigned_ids)
            ))
        }
        JournalCommand::PageUnload {
            page_resource,
            page,
            receipt_bodies,
        } => Ok(format!(
            "{{\"page_unload\":{{\"page_key\":[{page_resource},{page}],\"receipt_bodies\":{}}}}}",
            write_ids(receipt_bodies)
        )),
        JournalCommand::AddConstraint {
            ctype,
            body_a,
            body_b,
            point,
            hinge_axis,
            normal_axis,
            assigned_id,
        } => Ok(format!(
            "{{\"add_constraint\":{{\"type\":{ctype},\"body_a\":\"{body_a:016x}\",\"body_b\":\"{body_b:016x}\",\"point\":[\"{}\",\"{}\",\"{}\"],\"hinge_axis\":[\"{}\",\"{}\",\"{}\"],\"normal_axis\":[\"{}\",\"{}\",\"{}\"],\"assigned_id\":{assigned_id}}}}}",
            fhex(point[0], "p0")?,
            fhex(point[1], "p1")?,
            fhex(point[2], "p2")?,
            fhex(hinge_axis[0], "h0")?,
            fhex(hinge_axis[1], "h1")?,
            fhex(hinge_axis[2], "h2")?,
            fhex(normal_axis[0], "n0")?,
            fhex(normal_axis[1], "n1")?,
            fhex(normal_axis[2], "n2")?,
        )),
        JournalCommand::RemoveConstraint { id } => {
            Ok(format!("{{\"remove_constraint\":{{\"id\":{id}}}}}"))
        }
        JournalCommand::SetMotor { id, state, target } => Ok(format!(
            "{{\"set_motor\":{{\"id\":{id},\"state\":{state},\"target\":\"{}\"}}}}",
            fhex(*target, "mot")?
        )),
        JournalCommand::QueryRay {
            origin,
            dir,
            t_min,
            t_max,
            layer_mask,
            expected_hits,
        } => {
            let mut hits = String::from("[");
            for (i, (b, t)) in expected_hits.iter().enumerate() {
                if i > 0 {
                    hits.push(',');
                }
                hits.push_str(&format!("{{\"body\":\"{b:016x}\",\"t\":\"{t:08x}\"}}"));
            }
            hits.push(']');
            Ok(format!(
                "{{\"query_ray\":{{\"origin\":[\"{}\",\"{}\",\"{}\"],\"dir\":[\"{}\",\"{}\",\"{}\"],\"t_min\":\"{}\",\"t_max\":\"{}\",\"layer_mask\":{layer_mask},\"expected_hits\":{hits}}}}}",
                fhex(origin[0], "o0")?,
                fhex(origin[1], "o1")?,
                fhex(origin[2], "o2")?,
                fhex(dir[0], "d0")?,
                fhex(dir[1], "d1")?,
                fhex(dir[2], "d2")?,
                fhex(*t_min, "tmin")?,
                fhex(*t_max, "tmax")?,
            ))
        }
    }
}

impl JournalTick {
    pub fn to_json_line(&self) -> Result<String, CaptureError> {
        let mut pre = String::from("[");
        for (i, c) in self.pre.iter().enumerate() {
            if i > 0 {
                pre.push(',');
            }
            pre.push_str(&write_cmd(c)?);
        }
        pre.push(']');
        Ok(format!(
            "{{\"tick\":{},\"pre\":{pre},\"post\":{{\"semantic_state_hash\":\"{}\",\"event_digest\":\"{}\",\"contacts_emitted\":{},\"contacts_dropped\":{},\"ring_backlog\":{},\"saturation\":{{\"query_casts\":{},\"contact_events\":{},\"body_writes\":{}}}}}}}",
            self.tick,
            self.post.semantic_state_hash,
            self.post.event_digest,
            self.post.contacts_emitted,
            self.post.contacts_dropped,
            self.post.ring_backlog,
            self.post.saturation_query_casts,
            self.post.saturation_contact_events,
            self.post.saturation_body_writes,
        ))
    }
}

fn parse_hex_u64(s: &str) -> Result<u64, CaptureError> {
    let s = s.trim().trim_matches('"');
    u64::from_str_radix(s, 16).map_err(|e| CaptureError::Parse(format!("u64 hex: {e}")))
}

fn parse_hex_f32(s: &str) -> Result<f32, CaptureError> {
    let s = s.trim().trim_matches('"');
    let bits =
        u32::from_str_radix(s, 16).map_err(|e| CaptureError::Parse(format!("f32 hex: {e}")))?;
    Ok(f32::from_bits(bits))
}

fn extract_bracket_array(s: &str, after: &str) -> Result<String, CaptureError> {
    let i = s
        .find(after)
        .ok_or_else(|| CaptureError::Parse(format!("missing {after}")))?;
    let rest = &s[i + after.len()..];
    let start = rest
        .find('[')
        .ok_or_else(|| CaptureError::Parse("[".into()))?;
    let mut depth = 0i32;
    let bytes = rest.as_bytes();
    let mut end = start;
    for (off, &b) in bytes[start..].iter().enumerate() {
        if b == b'[' {
            depth += 1;
        } else if b == b']' {
            depth -= 1;
            if depth == 0 {
                end = start + off;
                break;
            }
        }
    }
    Ok(rest[start + 1..end].to_string())
}

fn parse_id_list(inner: &str) -> Result<Vec<u64>, CaptureError> {
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    inner.split(',').map(|p| parse_hex_u64(p.trim())).collect()
}

fn parse_f32_3(inner: &str) -> Result<[f32; 3], CaptureError> {
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(CaptureError::Parse("f32x3".into()));
    }
    Ok([
        parse_hex_f32(parts[0])?,
        parse_hex_f32(parts[1])?,
        parse_hex_f32(parts[2])?,
    ])
}

fn parse_transform(obj: &str) -> Result<PhysicsTransform, CaptureError> {
    let t_inner = extract_bracket_array(obj, "\"t\"")?;
    let r_inner = extract_bracket_array(obj, "\"r\"")?;
    let t = parse_f32_3(&t_inner)?;
    let parts: Vec<&str> = r_inner.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return Err(CaptureError::Parse("quat".into()));
    }
    Ok(PhysicsTransform {
        translation: t,
        rotation: [
            parse_hex_f32(parts[0])?,
            parse_hex_f32(parts[1])?,
            parse_hex_f32(parts[2])?,
            parse_hex_f32(parts[3])?,
        ],
    })
}

fn parse_shape(obj: &str) -> Result<ShapeDesc, CaptureError> {
    if obj.contains("\"Sphere\"") {
        let key = "\"radius\"";
        let i = obj
            .find(key)
            .ok_or_else(|| CaptureError::Parse("radius".into()))?;
        let rest = &obj[i + key.len()..];
        let q1 = rest
            .find('"')
            .ok_or_else(|| CaptureError::Parse("r1".into()))?;
        let q2 = rest[q1 + 1..]
            .find('"')
            .ok_or_else(|| CaptureError::Parse("r2".into()))?;
        let hex = &rest[q1 + 1..q1 + 1 + q2];
        Ok(ShapeDesc::Sphere {
            radius: parse_hex_f32(hex)?,
        })
    } else if obj.contains("\"Box\"") {
        let inner = extract_bracket_array(obj, "\"half_extents\"")?;
        Ok(ShapeDesc::Box {
            half_extents: parse_f32_3(&inner)?,
        })
    } else if obj.contains("\"Capsule\"") {
        // crude
        let hh_key = "\"half_height\"";
        let i = obj
            .find(hh_key)
            .ok_or_else(|| CaptureError::Parse("hh".into()))?;
        let rest = &obj[i + hh_key.len()..];
        let q1 = rest.find('"').unwrap();
        let q2 = rest[q1 + 1..].find('"').unwrap();
        let hh = parse_hex_f32(&rest[q1 + 1..q1 + 1 + q2])?;
        let rk = "\"radius\"";
        let i2 = obj
            .find(rk)
            .ok_or_else(|| CaptureError::Parse("cr".into()))?;
        let rest2 = &obj[i2 + rk.len()..];
        let a = rest2.find('"').unwrap();
        let b = rest2[a + 1..].find('"').unwrap();
        let rad = parse_hex_f32(&rest2[a + 1..a + 1 + b])?;
        Ok(ShapeDesc::Capsule {
            half_height: hh,
            radius: rad,
        })
    } else {
        Err(CaptureError::Parse("unsupported shape".into()))
    }
}

fn parse_desc(obj: &str) -> Result<BodyDesc, CaptureError> {
    fn num_after(obj: &str, key: &str) -> Result<u32, CaptureError> {
        let i = obj
            .find(key)
            .ok_or_else(|| CaptureError::Parse(key.into()))?;
        let rest = obj[i + key.len()..].trim_start_matches([' ', ':']);
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest[..end]
            .parse()
            .map_err(|e| CaptureError::Parse(format!("{key}:{e}")))
    }
    fn bool_after(obj: &str, key: &str) -> bool {
        if let Some(i) = obj.find(key) {
            let rest = obj[i + key.len()..].trim_start_matches([' ', ':']);
            rest.starts_with("true")
        } else {
            false
        }
    }
    fn hex_after(obj: &str, key: &str) -> Result<f32, CaptureError> {
        let i = obj
            .find(key)
            .ok_or_else(|| CaptureError::Parse(key.into()))?;
        let rest = &obj[i + key.len()..];
        let q1 = rest
            .find('"')
            .ok_or_else(|| CaptureError::Parse("hex1".into()))?;
        let q2 = rest[q1 + 1..]
            .find('"')
            .ok_or_else(|| CaptureError::Parse("hex2".into()))?;
        parse_hex_f32(&rest[q1 + 1..q1 + 1 + q2])
    }
    let kind = match num_after(obj, "\"kind\"")? {
        0 => BodyKind::Static,
        1 => BodyKind::Kinematic,
        2 => BodyKind::Dynamic,
        _ => return Err(CaptureError::Parse("kind".into())),
    };
    let shape_i = obj
        .find("\"shape\"")
        .ok_or_else(|| CaptureError::Parse("shape".into()))?;
    let shape = parse_shape(&obj[shape_i..])?;
    let xf_i = obj
        .find("\"transform\"")
        .ok_or_else(|| CaptureError::Parse("xf".into()))?;
    let transform = parse_transform(&obj[xf_i..])?;
    Ok(BodyDesc {
        kind,
        shape,
        layer: num_after(obj, "\"layer\"")?,
        mass_props: MassProps {
            mass: hex_after(obj, "\"mass\"")?,
            friction: hex_after(obj, "\"friction\"")?,
            restitution: hex_after(obj, "\"restitution\"")?,
            allow_sleep: bool_after(obj, "\"allow_sleep\""),
        },
        ccd: bool_after(obj, "\"ccd\""),
        transform,
    })
}

fn split_top_level_objects(inner: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = None;
    for (i, ch) in inner.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start {
                        out.push(inner[s..=i].to_string());
                    }
                    start = None;
                }
            }
            _ => {}
        }
    }
    out
}

fn parse_cmd(obj: &str) -> Result<JournalCommand, CaptureError> {
    if obj.contains("\"create_bodies\"") {
        let descs_inner = extract_bracket_array(obj, "\"descs\"")?;
        let ids_inner = extract_bracket_array(obj, "\"assigned_ids\"")?;
        let descs = split_top_level_objects(&descs_inner)
            .into_iter()
            .map(|o| parse_desc(&o))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(JournalCommand::CreateBodies {
            descs,
            assigned_ids: parse_id_list(&ids_inner)?,
        })
    } else if obj.contains("\"remove_bodies\"") {
        let ids_inner = extract_bracket_array(obj, "\"ids\"")?;
        Ok(JournalCommand::RemoveBodies {
            ids: parse_id_list(&ids_inner)?,
        })
    } else if obj.contains("\"apply_impulse\"") {
        let body_key = "\"body\"";
        let i = obj
            .find(body_key)
            .ok_or_else(|| CaptureError::Parse("body".into()))?;
        let rest = &obj[i + body_key.len()..];
        let q1 = rest.find('"').unwrap();
        let q2 = rest[q1 + 1..].find('"').unwrap();
        let body = parse_hex_u64(&rest[q1 + 1..q1 + 1 + q2])?;
        let imp = extract_bracket_array(obj, "\"impulse\"")?;
        Ok(JournalCommand::ApplyImpulse {
            body,
            impulse: parse_f32_3(&imp)?,
        })
    } else if obj.contains("\"set_velocity\"") {
        let body_key = "\"body\"";
        let i = obj.find(body_key).unwrap();
        let rest = &obj[i + body_key.len()..];
        let q1 = rest.find('"').unwrap();
        let q2 = rest[q1 + 1..].find('"').unwrap();
        let body = parse_hex_u64(&rest[q1 + 1..q1 + 1 + q2])?;
        let lin = extract_bracket_array(obj, "\"linear\"")?;
        let ang = extract_bracket_array(obj, "\"angular\"")?;
        Ok(JournalCommand::SetVelocity {
            body,
            linear: parse_f32_3(&lin)?,
            angular: parse_f32_3(&ang)?,
        })
    } else if obj.contains("\"move_kinematic\"") {
        let body_key = "\"body\"";
        let i = obj.find(body_key).unwrap();
        let rest = &obj[i + body_key.len()..];
        let q1 = rest.find('"').unwrap();
        let q2 = rest[q1 + 1..].find('"').unwrap();
        let body = parse_hex_u64(&rest[q1 + 1..q1 + 1 + q2])?;
        let xf_i = obj.find("\"transform\"").unwrap();
        Ok(JournalCommand::MoveKinematic {
            body,
            transform: parse_transform(&obj[xf_i..])?,
        })
    } else if obj.contains("\"page_resident\"") {
        let pk = extract_bracket_array(obj, "\"page_key\"")?;
        let parts: Vec<&str> = pk.split(',').map(str::trim).collect();
        let page_resource: u32 = parts[0]
            .parse()
            .map_err(|e| CaptureError::Parse(format!("{e}")))?;
        let page: u32 = parts[1]
            .parse()
            .map_err(|e| CaptureError::Parse(format!("{e}")))?;
        let descs_inner = extract_bracket_array(obj, "\"descs\"")?;
        let ids_inner = extract_bracket_array(obj, "\"assigned_ids\"")?;
        let descs = split_top_level_objects(&descs_inner)
            .into_iter()
            .map(|o| parse_desc(&o))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(JournalCommand::PageResident {
            page_resource,
            page,
            descs,
            assigned_ids: parse_id_list(&ids_inner)?,
        })
    } else if obj.contains("\"page_unload\"") {
        let pk = extract_bracket_array(obj, "\"page_key\"")?;
        let parts: Vec<&str> = pk.split(',').map(str::trim).collect();
        let page_resource: u32 = parts[0].parse().unwrap();
        let page: u32 = parts[1].parse().unwrap();
        let bodies = extract_bracket_array(obj, "\"receipt_bodies\"")?;
        Ok(JournalCommand::PageUnload {
            page_resource,
            page,
            receipt_bodies: parse_id_list(&bodies)?,
        })
    } else if obj.contains("\"add_constraint\"") {
        fn hex_field(obj: &str, key: &str) -> Result<u64, CaptureError> {
            let i = obj
                .find(key)
                .ok_or_else(|| CaptureError::Parse(key.into()))?;
            let rest = &obj[i + key.len()..];
            let q1 = rest.find('"').unwrap();
            let q2 = rest[q1 + 1..].find('"').unwrap();
            parse_hex_u64(&rest[q1 + 1..q1 + 1 + q2])
        }
        fn num_field(obj: &str, key: &str) -> Result<u64, CaptureError> {
            let i = obj
                .find(key)
                .ok_or_else(|| CaptureError::Parse(key.into()))?;
            let rest = obj[i + key.len()..].trim_start_matches([' ', ':']);
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            rest[..end]
                .parse()
                .map_err(|e| CaptureError::Parse(format!("{e}")))
        }
        Ok(JournalCommand::AddConstraint {
            ctype: num_field(obj, "\"type\"")? as u8,
            body_a: hex_field(obj, "\"body_a\"")?,
            body_b: hex_field(obj, "\"body_b\"")?,
            point: parse_f32_3(&extract_bracket_array(obj, "\"point\"")?)?,
            hinge_axis: parse_f32_3(&extract_bracket_array(obj, "\"hinge_axis\"")?)?,
            normal_axis: parse_f32_3(&extract_bracket_array(obj, "\"normal_axis\"")?)?,
            assigned_id: num_field(obj, "\"assigned_id\"")?,
        })
    } else if obj.contains("\"remove_constraint\"") {
        let i = obj.find("\"id\"").unwrap();
        let rest = obj[i + 4..].trim_start_matches([' ', ':']);
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let id: u64 = rest[..end].parse().unwrap();
        Ok(JournalCommand::RemoveConstraint { id })
    } else if obj.contains("\"set_motor\"") {
        let i = obj.find("\"id\"").unwrap();
        let rest = obj[i + 4..].trim_start_matches([' ', ':']);
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let id: u64 = rest[..end].parse().unwrap();
        let si = obj.find("\"state\"").unwrap();
        let srest = obj[si + 7..].trim_start_matches([' ', ':']);
        let send = srest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(srest.len());
        let state: u32 = srest[..send].parse().unwrap();
        let ti = obj.find("\"target\"").unwrap();
        let trest = &obj[ti + 8..];
        let q1 = trest.find('"').unwrap();
        let q2 = trest[q1 + 1..].find('"').unwrap();
        let target = parse_hex_f32(&trest[q1 + 1..q1 + 1 + q2])?;
        Ok(JournalCommand::SetMotor { id, state, target })
    } else if obj.contains("\"query_ray\"") {
        let origin = parse_f32_3(&extract_bracket_array(obj, "\"origin\"")?)?;
        let dir = parse_f32_3(&extract_bracket_array(obj, "\"dir\"")?)?;
        fn hex_key(obj: &str, key: &str) -> Result<f32, CaptureError> {
            let i = obj
                .find(key)
                .ok_or_else(|| CaptureError::Parse(key.into()))?;
            let rest = &obj[i + key.len()..];
            let q1 = rest.find('"').unwrap();
            let q2 = rest[q1 + 1..].find('"').unwrap();
            parse_hex_f32(&rest[q1 + 1..q1 + 1 + q2])
        }
        let t_min = hex_key(obj, "\"t_min\"")?;
        let t_max = hex_key(obj, "\"t_max\"")?;
        let lm_i = obj.find("\"layer_mask\"").unwrap();
        let lm_rest = obj[lm_i + 12..].trim_start_matches([' ', ':']);
        let lm_end = lm_rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(lm_rest.len());
        let layer_mask: u64 = lm_rest[..lm_end].parse().unwrap();
        let hits_inner = extract_bracket_array(obj, "\"expected_hits\"")?;
        let mut expected_hits = Vec::new();
        for h in split_top_level_objects(&hits_inner) {
            let bi = h.find("\"body\"").unwrap();
            let rest = &h[bi + 6..];
            let q1 = rest.find('"').unwrap();
            let q2 = rest[q1 + 1..].find('"').unwrap();
            let body = parse_hex_u64(&rest[q1 + 1..q1 + 1 + q2])?;
            let ti = h.find("\"t\"").unwrap();
            let trest = &h[ti + 3..];
            let a = trest.find('"').unwrap();
            let b = trest[a + 1..].find('"').unwrap();
            let tbits = u32::from_str_radix(&trest[a + 1..a + 1 + b], 16).unwrap();
            expected_hits.push((body, tbits));
        }
        Ok(JournalCommand::QueryRay {
            origin,
            dir,
            t_min,
            t_max,
            layer_mask,
            expected_hits,
        })
    } else {
        Err(CaptureError::Parse(format!(
            "unknown cmd {}",
            &obj[..obj.len().min(64)]
        )))
    }
}

impl JournalTick {
    pub fn parse_json_line(line: &str) -> Result<Self, CaptureError> {
        let line = line.trim();
        if line.is_empty() {
            return Err(CaptureError::Parse("empty journal line".into()));
        }
        let tick_key = "\"tick\"";
        let ti = line
            .find(tick_key)
            .ok_or_else(|| CaptureError::Parse("tick".into()))?;
        let trest = line[ti + tick_key.len()..].trim_start_matches([' ', ':']);
        let tend = trest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(trest.len());
        let tick: u64 = trest[..tend]
            .parse()
            .map_err(|e| CaptureError::Parse(format!("tick:{e}")))?;
        let pre_inner = extract_bracket_array(line, "\"pre\"")?;
        let pre = split_top_level_objects(&pre_inner)
            .into_iter()
            .map(|o| parse_cmd(&o))
            .collect::<Result<Vec<_>, _>>()?;
        fn str_field(line: &str, key: &str) -> Result<String, CaptureError> {
            let i = line
                .find(key)
                .ok_or_else(|| CaptureError::Parse(key.into()))?;
            let rest = &line[i + key.len()..];
            let q1 = rest
                .find('"')
                .ok_or_else(|| CaptureError::Parse("sf1".into()))?;
            let q2 = rest[q1 + 1..]
                .find('"')
                .ok_or_else(|| CaptureError::Parse("sf2".into()))?;
            Ok(rest[q1 + 1..q1 + 1 + q2].to_string())
        }
        fn num_field(line: &str, key: &str) -> Result<u64, CaptureError> {
            let i = line
                .find(key)
                .ok_or_else(|| CaptureError::Parse(key.into()))?;
            let rest = line[i + key.len()..].trim_start_matches([' ', ':']);
            let end = rest
                .find(|c: char| !(c.is_ascii_digit()))
                .unwrap_or(rest.len());
            rest[..end]
                .parse()
                .map_err(|e| CaptureError::Parse(format!("{key}:{e}")))
        }
        Ok(JournalTick {
            tick,
            pre,
            post: PostTick {
                semantic_state_hash: str_field(line, "\"semantic_state_hash\"")?,
                event_digest: str_field(line, "\"event_digest\"")?,
                contacts_emitted: num_field(line, "\"contacts_emitted\"")? as u32,
                contacts_dropped: num_field(line, "\"contacts_dropped\"")?,
                ring_backlog: num_field(line, "\"ring_backlog\"")? as u32,
                saturation_query_casts: num_field(line, "\"query_casts\"")?,
                saturation_contact_events: num_field(line, "\"contact_events\"")?,
                saturation_body_writes: num_field(line, "\"body_writes\"")?,
            },
        })
    }
}

pub fn body_ids_bits(ids: &[BodyId]) -> Vec<u64> {
    ids.iter().map(|id| id.to_bits()).collect()
}
