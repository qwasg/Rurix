use std::fs;
use std::path::PathBuf;

use rurix_pkg::sha256::{digest, hex};
use rurix_physics::capture::{default_budget, jolt_world_desc, BudgetProfile};
use rurix_physics::WorldDesc;

pub const CORPUS_ROOT: &str = "conformance/physics/replay";

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub fn corpus_dir(scenario: &str) -> PathBuf {
    repo_root().join(CORPUS_ROOT).join(scenario)
}

pub fn all_scenario_ids() -> &'static [&'static str] {
    &[
        "box_stack_settle",
        "sphere_impulse_script",
        "create_destroy_churn",
        "streaming_page_cycle",
        "ccd_bullet_thin_wall",
        "kinematic_platform",
        "joint_pendulum_motor",
        "query_mid_replay",
        "contact_ring_saturation",
        "mixed_soup_72",
    ]
}

pub fn build_fingerprint() -> String {
    let rustc = std::process::Command::new("rustc")
        .arg("-V")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "rustc-unknown".into());
    format!(
        "rurix-physics/{} {}",
        env!("CARGO_PKG_VERSION"),
        rustc.trim()
    )
}

pub fn joltc_abi_digest() -> String {
    let root = repo_root();
    let f1 = root.join("src/rurix-physics-sys/vendor/JoltC/JoltC/Functions.h");
    let f2 = root.join("src/rurix-physics-sys/vendor/JoltC/JoltC/Enums.h");
    let mut buf = Vec::new();
    if let Ok(b) = fs::read(&f1) {
        buf.extend_from_slice(&b);
    }
    if let Ok(b) = fs::read(&f2) {
        buf.extend_from_slice(&b);
    }
    hex(&digest(&buf))
}

pub fn scenario_world_desc(scenario: &str) -> WorldDesc {
    let cc = match scenario {
        "contact_ring_saturation" => 8,
        _ => 4096,
    };
    jolt_world_desc(cc)
}

pub fn scenario_budget(scenario: &str) -> BudgetProfile {
    default_budget(&scenario_world_desc(scenario))
}

pub fn json_bool(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

pub fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
