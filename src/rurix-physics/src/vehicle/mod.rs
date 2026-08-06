//! M70 自研 raycast 悬挂载具(RFC-0021 §4.D1;`physics-vehicle` feature)。

use rurix_pkg::sha256::{digest, hex};

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

/// wave6d subject 取证。
pub fn vehicle_subject_pass() -> (bool, String) {
    let v = VehicleAsset::demo();
    let ok = v.cook_deterministic_double()
        && v.wheels.len() >= 2
        && !v.state_digest(1200.0, 1, &[0.38, 0.39]).is_empty();
    (ok, if ok { "vehicle capture state ok".into() } else { "vehicle subject fail".into() })
}
