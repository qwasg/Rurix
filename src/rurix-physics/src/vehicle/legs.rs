//! M70 wave6d subject 六腿(G8.6_G8.8_PHYSICS_CLOSEOUT_DESIGN.md §5.3):
//! asset_roundtrip / fixed_input_replay_hash_equal / rollback_correction_converges
//! / tire_light_object_contact_regression_golden / state_serialization_roundtrip
//! / telemetry_trace_golden。每腿独立布尔;`falsify_leg` 为逐腿 RED 证伪臂。

use rurix_pkg::sha256::{digest, hex};

use super::VehicleAsset;
use super::sim::{
    ROLLBACK_TICK, TICKS, VehicleSim, VehicleState, input_log_line, parse_input_log, scripted_input,
};

pub const LEG_NAMES: [&str; 6] = [
    "asset_roundtrip",
    "fixed_input_replay_hash_equal",
    "rollback_correction_converges",
    "tire_light_object_contact_regression_golden",
    "state_serialization_roundtrip",
    "telemetry_trace_golden",
];

/// 轮胎-轻物体接触回归 golden(2026-08-08 本机首跑 measured 冻结;falsify 臂证比较非空转)。
pub const GOLDEN_CONTACT_DIGEST: &str =
    "4445af5eb68501404e94eff89d72ae6e77f2c0f00831dd017a6f629f7c476fdd";
/// 遥测 trace golden(同上)。
pub const GOLDEN_TELEMETRY_DIGEST: &str =
    "f25ae9387720b02cf9446422895a8bd11382bc543b27bd334ebe7c6a14b0b443";

#[derive(Debug, Clone)]
pub struct VehicleSubjectReport {
    pub ok: bool,
    pub asset_roundtrip: bool,
    pub fixed_input_replay_hash_equal: bool,
    pub rollback_correction_converges: bool,
    pub tire_light_object_contact_regression_golden: bool,
    pub state_serialization_roundtrip: bool,
    pub telemetry_trace_golden: bool,
    pub final_state_hash: String,
    pub contact_digest: String,
    pub contact_events: usize,
    pub telemetry_digest: String,
    pub telemetry_lines: usize,
    pub detail: String,
}

fn asset_digest(a: &VehicleAsset) -> String {
    hex(&digest(a.canonical_json().as_bytes()))
}

/// 腿 1:asset canonical roundtrip 字节稳定 + 未知版本 fail-closed。
fn leg_asset_roundtrip() -> bool {
    let a = VehicleAsset::demo();
    let json = a.canonical_json();
    let b = match VehicleAsset::parse_canonical(&json) {
        Ok(b) => b,
        Err(_) => return false,
    };
    if b.canonical_json() != json || asset_digest(&b) != asset_digest(&a) {
        return false;
    }
    // 未知 schema_version 必须 fail-closed(真 RED 臂内建于腿)。
    let bumped = json.replacen("\"schema_version\":1", "\"schema_version\":99", 1);
    VehicleAsset::parse_canonical(&bumped).is_err()
}

/// 全场景跑 240 tick;返回 (末态 hash, 遥测行, 接触事件行)。
fn run_full(asset: &VehicleAsset, bump_scale: f32) -> (String, Vec<String>, Vec<String>) {
    let mut sim = VehicleSim::new(asset);
    sim.bump_scale = bump_scale;
    let mut tele = Vec::new();
    let mut contacts = Vec::new();
    for t in 0..TICKS {
        let m = sim.step(asset, &scripted_input(t));
        tele.push(m.canonical_line());
        if m.contact_pen_m > 0.0 {
            contacts.push(format!(
                "{}:{:08x}:{:08x}",
                m.tick,
                m.contact_pen_m.to_bits(),
                m.obj_x.to_bits()
            ));
        }
    }
    (sim.state.state_hash(), tele, contacts)
}

fn trace_digest(lines: &[String]) -> String {
    hex(&digest(lines.join(";").as_bytes()))
}

/// 腿 2:固定输入两次重放末态 hash 全等 + 输入日志序列化往返后再放仍全等。
fn leg_fixed_input_replay_hash_equal(asset: &VehicleAsset) -> (bool, String) {
    let (h1, _, _) = run_full(asset, 1.0);
    let (h2, _, _) = run_full(asset, 1.0);
    // journal 等价物:输入日志落盘→重载→逐 tick 驱动。
    let log = (0..TICKS)
        .map(|t| input_log_line(t, &scripted_input(t)))
        .collect::<Vec<_>>()
        .join("\n");
    let inputs = match parse_input_log(&log) {
        Ok(v) => v,
        Err(_) => return (false, h1),
    };
    let mut sim = VehicleSim::new(asset);
    for (t, input) in inputs.iter().enumerate() {
        if t as u64 >= TICKS {
            return (false, h1);
        }
        sim.step(asset, input);
    }
    (h1 == h2 && sim.state.state_hash() == h1, h1)
}

/// 腿 3:快照回滚到 ROLLBACK_TICK + 重放剩余输入 → 与连续模拟逐位收敛。
fn leg_rollback_correction_converges(asset: &VehicleAsset) -> bool {
    let (hash_server, _, _) = run_full(asset, 1.0);
    let mut ahead = VehicleSim::new(asset);
    for t in 0..ROLLBACK_TICK {
        ahead.step(asset, &scripted_input(t));
    }
    let snap = ahead.state.serialize();
    let restored = match VehicleState::parse(&snap) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut sim = VehicleSim::from_state(restored);
    for t in ROLLBACK_TICK..TICKS {
        sim.step(asset, &scripted_input(t));
    }
    sim.state.state_hash() == hash_server
}

/// 腿 4:轮胎推挤轻物体接触回归 golden(非空断言 + digest 冻结比较)。
fn leg_tire_contact(asset: &VehicleAsset) -> (bool, String, usize) {
    let (_, _, contacts) = run_full(asset, 1.0);
    let d = trace_digest(&contacts);
    (
        contacts.len() >= 3 && d == GOLDEN_CONTACT_DIGEST,
        d,
        contacts.len(),
    )
}

/// 腿 5:中途状态序列化→解析→再序列化字节相等 + state hash 相等。
fn leg_state_serialization_roundtrip(asset: &VehicleAsset) -> bool {
    let mut sim = VehicleSim::new(asset);
    for t in 0..137u64 {
        sim.step(asset, &scripted_input(t));
    }
    let blob = sim.state.serialize();
    let parsed = match VehicleState::parse(&blob) {
        Ok(p) => p,
        Err(_) => return false,
    };
    parsed.serialize() == blob && parsed.state_hash() == sim.state.state_hash()
}

/// 腿 6:遥测 trace golden(行数非空断言 + digest 冻结比较)。
fn leg_telemetry(asset: &VehicleAsset) -> (bool, String, usize) {
    let (_, tele, _) = run_full(asset, 1.0);
    let d = trace_digest(&tele);
    (
        tele.len() == TICKS as usize && d == GOLDEN_TELEMETRY_DIGEST,
        d,
        tele.len(),
    )
}

/// wave6d subject 六腿取证(替代旧单 bool 薄壳)。
pub fn run_vehicle_subject() -> VehicleSubjectReport {
    let asset = VehicleAsset::demo();
    let asset_roundtrip = leg_asset_roundtrip();
    let (fixed_input_replay_hash_equal, final_state_hash) =
        leg_fixed_input_replay_hash_equal(&asset);
    let rollback_correction_converges = leg_rollback_correction_converges(&asset);
    let (tire_light_object_contact_regression_golden, contact_digest, contact_events) =
        leg_tire_contact(&asset);
    let state_serialization_roundtrip = leg_state_serialization_roundtrip(&asset);
    let (telemetry_trace_golden, telemetry_digest, telemetry_lines) = leg_telemetry(&asset);
    let ok = asset_roundtrip
        && fixed_input_replay_hash_equal
        && rollback_correction_converges
        && tire_light_object_contact_regression_golden
        && state_serialization_roundtrip
        && telemetry_trace_golden;
    VehicleSubjectReport {
        ok,
        asset_roundtrip,
        fixed_input_replay_hash_equal,
        rollback_correction_converges,
        tire_light_object_contact_regression_golden,
        state_serialization_roundtrip,
        telemetry_trace_golden,
        final_state_hash,
        contact_digest,
        contact_events,
        telemetry_digest,
        telemetry_lines,
        detail: if ok {
            "vehicle 6-leg subject PASS".into()
        } else {
            "vehicle 6-leg subject FAIL".into()
        },
    }
}

/// 逐腿 RED 证伪臂:对该腿输入做最小摄动,返回腿在此摄动下的结果(期望 false)。
/// 返回 None = 未知腿名。仅供 ci selftest 断言"摄动必红",不产生任何绿。
pub fn falsify_leg(leg: &str) -> Option<bool> {
    let asset = VehicleAsset::demo();
    match leg {
        // 篡改 asset 字节后 strict roundtrip 必须检出。
        "asset_roundtrip" => {
            let json = asset.canonical_json();
            let tampered = json.replacen(
                "\"asset_id\":\"demo_buggy_v1\"",
                "\"asset_id\":\"demo_buggy_v2\"",
                1,
            );
            let undetected = match VehicleAsset::parse_canonical(&tampered) {
                Ok(b) => b.canonical_json() == json && asset_digest(&b) == asset_digest(&asset),
                Err(_) => false,
            };
            Some(undetected)
        }
        // 第二遍重放篡改一条输入 → 末态 hash 必须不等。
        "fixed_input_replay_hash_equal" => {
            let (h1, _, _) = run_full(&asset, 1.0);
            let mut sim = VehicleSim::new(&asset);
            for t in 0..TICKS {
                let mut input = scripted_input(t);
                if t == 100 {
                    input.throttle += 0.001;
                }
                sim.step(&asset, &input);
            }
            Some(sim.state.state_hash() == h1)
        }
        // 回滚后重放序列篡改一条 → 收敛断言必须失败。
        "rollback_correction_converges" => {
            let (hash_server, _, _) = run_full(&asset, 1.0);
            let mut ahead = VehicleSim::new(&asset);
            for t in 0..ROLLBACK_TICK {
                ahead.step(&asset, &scripted_input(t));
            }
            let restored = match VehicleState::parse(&ahead.state.serialize()) {
                Ok(s) => s,
                Err(_) => return Some(false),
            };
            let mut sim = VehicleSim::from_state(restored);
            for t in ROLLBACK_TICK..TICKS {
                let mut input = scripted_input(t);
                if t == 210 {
                    input.brake += 0.001;
                }
                sim.step(&asset, &input);
            }
            Some(sim.state.state_hash() == hash_server)
        }
        // 轻物体初始位置摄动(接触 trace 可观测量)→ 接触 digest 必须偏离 golden。
        "tire_light_object_contact_regression_golden" => {
            let mut sim = VehicleSim::new(&asset);
            sim.state.obj_x += 0.05;
            let mut contacts = Vec::new();
            for t in 0..TICKS {
                let m = sim.step(&asset, &scripted_input(t));
                if m.contact_pen_m > 0.0 {
                    contacts.push(format!(
                        "{}:{:08x}:{:08x}",
                        m.tick,
                        m.contact_pen_m.to_bits(),
                        m.obj_x.to_bits()
                    ));
                }
            }
            Some(trace_digest(&contacts) == GOLDEN_CONTACT_DIGEST)
        }
        // 状态字节篡改 → 解析/再序列化必须检出。
        "state_serialization_roundtrip" => {
            let mut sim = VehicleSim::new(&asset);
            for t in 0..137u64 {
                sim.step(&asset, &scripted_input(t));
            }
            let blob = sim.state.serialize();
            // 无论当前 gear 值为何都篡改它(避免替换了不存在的字面量)。
            let gear_key = "\"gear\":";
            let tampered = match blob.find(gear_key) {
                Some(i) => {
                    let d = i + gear_key.len();
                    let cur = blob[d..d + 1].parse::<u8>().unwrap_or(0);
                    format!("{}{}{}", &blob[..d], cur + 1, &blob[d + 1..])
                }
                None => blob.clone(),
            };
            let undetected = match VehicleState::parse(&tampered) {
                Ok(p) => p.serialize() == blob && p.state_hash() == sim.state.state_hash(),
                Err(_) => false,
            };
            Some(undetected)
        }
        // 遥测前像篡改一条输入 → digest 必须偏离 golden。
        "telemetry_trace_golden" => {
            let mut sim = VehicleSim::new(&asset);
            let mut tele = Vec::new();
            for t in 0..TICKS {
                let mut input = scripted_input(t);
                if t == 50 {
                    input.throttle += 0.001;
                }
                let m = sim.step(&asset, &input);
                tele.push(m.canonical_line());
            }
            Some(trace_digest(&tele) == GOLDEN_TELEMETRY_DIGEST)
        }
        _ => None,
    }
}
