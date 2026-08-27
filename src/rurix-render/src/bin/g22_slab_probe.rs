// Assisted-by: Cursor Agent（G22.2 M-a 实现波）
//! G22.2 M-a slab 材质能量守恒 host 参考臂 probe（门
//! `g22.p0.m_a.slab_material_host_realization`；RFC-0039）。
//!
//! 职责闭集：128×128 参数网格白炉审计——白炉恒等 + 全域 R ≤ 1 + 对 a_b 单调 +
//! 闭式↔级数+尾和恒等式（1e-9 浮点级）+ lerp 连续性 + 双跑位级。
//!
//! 用法：`g22_slab_probe --out evidence/g22_slab_probe_<UTC>.json`

#![forbid(unsafe_code)]

use rurix_render::material::slab::{SlabStack, furnace_audit};

const GRID: u32 = 128;
const BOUNCES: u32 = 96;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_path = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("g22_slab_probe: FAIL 缺 --out <evidence.json>");
            std::process::exit(2);
        });

    let rep1 = furnace_audit(GRID, BOUNCES);
    let rep2 = furnace_audit(GRID, BOUNCES);
    let double_run_bitexact = rep1 == rep2;

    let white_ok = rep1.white_furnace_dev < 1e-9;
    let bound_ok = rep1.max_total <= 1.0 + 1e-9;
    let mono_ok = rep1.monotonic_violations == 0;
    let identity_ok = rep1.series_closed_form_max_dev < 1e-9;
    // lerp 连续性（16 段步进反照率跳变上界）
    let a = SlabStack::new(0.1, 0.2);
    let b = SlabStack::new(0.85, 0.9);
    let mut lerp_max_step = 0.0f64;
    let mut prev = SlabStack::lerp(&a, &b, 0.0).total_reflectance();
    for k in 1..=16 {
        let cur = SlabStack::lerp(&a, &b, k as f32 / 16.0).total_reflectance();
        lerp_max_step = lerp_max_step.max((cur - prev).abs());
        prev = cur;
    }
    let lerp_ok = lerp_max_step < 0.12;

    println!(
        "[g22_slab_probe] samples={} max_total={:.9} white_dev={:.2e} mono_violations={} identity_dev={:.2e} lerp_max_step={:.4}",
        rep1.samples,
        rep1.max_total,
        rep1.white_furnace_dev,
        rep1.monotonic_violations,
        rep1.series_closed_form_max_dev,
        lerp_max_step
    );

    let payload = format!(
        "{{\"schema_version\":1,\"subject\":\"g22_slab_probe\",\
         \"grid\":{GRID},\"bounces\":{BOUNCES},\"samples\":{},\
         \"max_total\":{:.12},\"white_furnace_dev\":{:.3e},\
         \"monotonic_violations\":{},\"series_identity_max_dev\":{:.3e},\
         \"lerp_max_step\":{:.6},\
         \"white_furnace_identity\":{white_ok},\"energy_bounded\":{bound_ok},\
         \"monotonic_in_base_albedo\":{mono_ok},\"series_identity_1e9\":{identity_ok},\
         \"lerp_continuity\":{lerp_ok},\"double_run_bitexact\":{double_run_bitexact},\
         \"notes\":\"slab 双层能量守恒闭合白炉审计：白炉恒等/能量上界/单调/闭式恒等式/lerp 连续/双跑位级\"}}",
        rep1.samples,
        rep1.max_total,
        rep1.white_furnace_dev,
        rep1.monotonic_violations,
        rep1.series_closed_form_max_dev,
        lerp_max_step,
    );
    if let Some(parent) = std::path::Path::new(&out_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out_path, payload + "\n").expect("写 evidence 失败");
    println!("[g22_slab_probe] evidence → {out_path}");
    if !(white_ok && bound_ok && mono_ok && identity_ok && lerp_ok && double_run_bitexact) {
        eprintln!("[g22_slab_probe] FAIL");
        std::process::exit(1);
    }
    println!("[g22_slab_probe] PASS");
}
