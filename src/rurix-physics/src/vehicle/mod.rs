//! M70 自研 raycast 悬挂载具(RFC-0021 §4.D1;`physics-vehicle` feature)。
//!
//! wave6d subject 六腿取证见 [`legs`](G8.6_G8.8_PHYSICS_CLOSEOUT_DESIGN.md §5.3);
//! 确定性仿真态与 canonical 状态序列化见 [`sim`]。

use rurix_pkg::sha256::{digest, hex};

use crate::capture::canonical::CaptureError;

pub mod legs;
pub mod sim;

use sim::Cursor;

pub const VEHICLE_SCHEMA_ID: &str = "rurix.physics.vehicle";
pub const VEHICLE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct WheelDesc {
    pub id: String,
    pub radius_m: f32,
    pub suspension_rest_m: f32,
    pub stiffness: f32,
    pub damping: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VehicleAsset {
    pub schema_id: String,
    pub schema_version: u32,
    pub asset_id: String,
    pub wheels: Vec<WheelDesc>,
    pub gear_ratios: Vec<f32>,
    pub cook_profile: String,
}

impl VehicleAsset {
    pub fn demo() -> Self {
        Self {
            schema_id: VEHICLE_SCHEMA_ID.into(),
            schema_version: VEHICLE_SCHEMA_VERSION,
            asset_id: "demo_buggy_v1".into(),
            wheels: vec![
                WheelDesc {
                    id: "fl".into(),
                    radius_m: 0.35,
                    suspension_rest_m: 0.4,
                    stiffness: 35000.0,
                    damping: 4500.0,
                },
                WheelDesc {
                    id: "fr".into(),
                    radius_m: 0.35,
                    suspension_rest_m: 0.4,
                    stiffness: 35000.0,
                    damping: 4500.0,
                },
            ],
            gear_ratios: vec![3.5, 2.1, 1.4, 1.0],
            cook_profile: "v1".into(),
        }
    }

    pub fn canonical_json(&self) -> String {
        let mut wheels = String::from("[");
        for (i, w) in self.wheels.iter().enumerate() {
            if i > 0 {
                wheels.push(',');
            }
            wheels.push_str(&format!(
                "{{\"id\":\"{}\",\"radius_m\":{:.3},\"suspension_rest_m\":{:.3},\"stiffness\":{:.1},\"damping\":{:.1}}}",
                w.id, w.radius_m, w.suspension_rest_m, w.stiffness, w.damping
            ));
        }
        wheels.push(']');
        format!(
            "{{\"schema_id\":\"{}\",\"schema_version\":{},\"asset_id\":\"{}\",\"wheels\":{},\"gear_ratios\":{:?},\"cook_profile\":\"{}\"}}",
            self.schema_id,
            self.schema_version,
            self.asset_id,
            wheels,
            self.gear_ratios,
            self.cook_profile
        )
    }

    /// 严格解析 `canonical_json` 产物:固定字段序、未知 schema_id/version
    /// fail-closed、非有限浮点 fail-closed、尾部垃圾拒绝。
    pub fn parse_canonical(text: &str) -> Result<Self, CaptureError> {
        let mut c = Cursor::new(text);
        c.expect("{\"schema_id\":\"")?;
        let schema_id = c.take_until("\"")?.to_string();
        c.take(1)?;
        if schema_id != VEHICLE_SCHEMA_ID {
            return Err(CaptureError::Rejected(format!(
                "unknown vehicle schema_id {schema_id}"
            )));
        }
        c.expect(",\"schema_version\":")?;
        let schema_version = c.take_u64_until(",\"asset_id\":\"")? as u32;
        if schema_version != VEHICLE_SCHEMA_VERSION {
            return Err(CaptureError::Rejected(format!(
                "unknown vehicle schema_version {schema_version}"
            )));
        }
        let asset_id = c.take_until("\"")?.to_string();
        c.take(1)?;
        c.expect(",\"wheels\":[")?;
        let mut wheels = Vec::new();
        if !c.rest().starts_with(']') {
            loop {
                c.expect("{\"id\":\"")?;
                let id = c.take_until("\"")?.to_string();
                c.take(1)?;
                c.expect(",\"radius_m\":")?;
                let radius_m = c.take_f32_until(",\"suspension_rest_m\":", "radius_m")?;
                c.expect(",\"suspension_rest_m\":")?;
                let suspension_rest_m = c.take_f32_until(",\"stiffness\":", "suspension_rest_m")?;
                c.expect(",\"stiffness\":")?;
                let stiffness = c.take_f32_until(",\"damping\":", "stiffness")?;
                c.expect(",\"damping\":")?;
                let damping = c.take_f32_until("}", "damping")?;
                c.expect("}")?;
                for (path, v) in [
                    ("radius_m", radius_m),
                    ("suspension_rest_m", suspension_rest_m),
                    ("stiffness", stiffness),
                    ("damping", damping),
                ] {
                    if !v.is_finite() {
                        return Err(CaptureError::NanFloat { path: path.into() });
                    }
                }
                wheels.push(WheelDesc {
                    id,
                    radius_m,
                    suspension_rest_m,
                    stiffness,
                    damping,
                });
                match c.take(1)? {
                    "," => continue,
                    "]" => break,
                    other => return Err(CaptureError::Parse(format!("wheel sep: {other}"))),
                }
            }
        } else {
            c.expect("]")?;
        }
        c.expect(",\"gear_ratios\":[")?;
        let ratios_text = c.take_until("]")?;
        c.expect("]")?;
        let mut gear_ratios = Vec::new();
        if !ratios_text.trim().is_empty() {
            for part in ratios_text.split(',') {
                let v: f32 = part
                    .trim()
                    .parse()
                    .map_err(|e| CaptureError::Parse(format!("gear_ratio: {e}")))?;
                if !v.is_finite() {
                    return Err(CaptureError::NanFloat {
                        path: "gear_ratios".into(),
                    });
                }
                gear_ratios.push(v);
            }
        }
        c.expect(",\"cook_profile\":\"")?;
        let cook_profile = c.take_until("\"")?.to_string();
        c.take(1)?;
        c.expect("}")?;
        c.expect_end()?;
        Ok(Self {
            schema_id,
            schema_version,
            asset_id,
            wheels,
            gear_ratios,
            cook_profile,
        })
    }

    pub fn state_digest(&self, rpm: f32, gear: u8, suspension: &[f32]) -> String {
        let mut s = format!("rpm{:.3}:gear{}:", rpm, gear);
        for v in suspension {
            s.push_str(&format!("{:.5};", v));
        }
        s.push_str(&self.canonical_json());
        hex(&digest(s.as_bytes()))
    }

    pub fn cook_deterministic_double(&self) -> bool {
        let a = self.canonical_json();
        let b = self.canonical_json();
        a == b
    }
}
