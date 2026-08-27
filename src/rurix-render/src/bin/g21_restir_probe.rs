// Assisted-by: Cursor Agent（G21.2 M-a 实现波）
//! G21.2 M-a ReSTIR 高档 reservoir host 参考臂 probe（门
//! `g21.p0.m_a.restir_high_reservoir_realization`；RFC-0038；M100-high 证据产出）。
//!
//! 职责闭集：64 灯环形夹具 × 20k trial——三估计子（uniform / RIS-16 / RIS 时域
//! 8 帧链 M-cap 60）无偏 3σ 检验 + 等验证预算方差收益 measured + 双跑位级。
//!
//! 用法：`g21_restir_probe --out evidence/g21_restir_probe_<UTC>.json`

#![forbid(unsafe_code)]

use rurix_render::gi::restir_reservoir::{fixture_lights, variance_experiment};

const SEED: u64 = 0x0521_A011_2026_0824;
const N_TRIALS: u32 = 20_000;
const M_CANDIDATES: u32 = 16;
const TEMPORAL_FRAMES: u32 = 8;
const M_CAP: u32 = 60;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_path = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("g21_restir_probe: FAIL 缺 --out <evidence.json>");
            std::process::exit(2);
        });

    let lights = fixture_lights(64);
    let rep1 = variance_experiment(
        &lights,
        M_CANDIDATES,
        N_TRIALS,
        TEMPORAL_FRAMES,
        M_CAP,
        SEED,
    );
    let rep2 = variance_experiment(
        &lights,
        M_CANDIDATES,
        N_TRIALS,
        TEMPORAL_FRAMES,
        M_CAP,
        SEED,
    );
    let double_run_bitexact = rep1 == rep2;

    // 无偏 3σ 检验（σ_mean = sqrt(var/n)）
    let three_sigma = |mean: f64, var: f64| -> (bool, f64, f64) {
        let sigma_mean = (var / f64::from(rep1.n_trials)).sqrt();
        let dev = (mean - rep1.reference).abs();
        (dev < 3.0 * sigma_mean + 1e-9, dev, 3.0 * sigma_mean)
    };
    let (u_ok, u_dev, u_bound) = three_sigma(rep1.uniform_mean, rep1.uniform_var);
    let (r_ok, r_dev, r_bound) = three_sigma(rep1.ris_mean, rep1.ris_var);
    let (t_ok, t_dev, t_bound) = three_sigma(rep1.ris_temporal_mean, rep1.ris_temporal_var);
    let unbiased_all = u_ok && r_ok && t_ok;
    let variance_gain_ok = rep1.variance_reduction > 2.0;
    let temporal_gain_ok = rep1.temporal_reduction > 1.2;

    println!(
        "[g21_restir_probe] ref={:.6} uniform_var={:.6} ris_var={:.6} temporal_var={:.6}",
        rep1.reference, rep1.uniform_var, rep1.ris_var, rep1.ris_temporal_var
    );
    println!(
        "[g21_restir_probe] variance_reduction={:.3} temporal_reduction={:.3} unbiased={} bitexact={}",
        rep1.variance_reduction, rep1.temporal_reduction, unbiased_all, double_run_bitexact
    );

    let payload = format!(
        "{{\"schema_version\":1,\"subject\":\"g21_restir_probe\",\
         \"lights\":64,\"n_trials\":{N_TRIALS},\"m_candidates\":{M_CANDIDATES},\
         \"temporal_frames\":{TEMPORAL_FRAMES},\"m_cap\":{M_CAP},\
         \"reference\":{:.9},\
         \"uniform\":{{\"mean\":{:.9},\"var\":{:.9},\"unbiased_3sigma\":{u_ok},\"dev\":{u_dev:.9},\"bound\":{u_bound:.9}}},\
         \"ris\":{{\"mean\":{:.9},\"var\":{:.9},\"unbiased_3sigma\":{r_ok},\"dev\":{r_dev:.9},\"bound\":{r_bound:.9}}},\
         \"ris_temporal\":{{\"mean\":{:.9},\"var\":{:.9},\"unbiased_3sigma\":{t_ok},\"dev\":{t_dev:.9},\"bound\":{t_bound:.9}}},\
         \"variance_reduction\":{:.6},\"temporal_reduction\":{:.6},\
         \"unbiased_all_3sigma\":{unbiased_all},\
         \"variance_gain_gt2\":{variance_gain_ok},\
         \"temporal_gain_gt1_2\":{temporal_gain_ok},\
         \"double_run_bitexact\":{double_run_bitexact},\
         \"notes\":\"ReSTIR 高档 reservoir host 参考臂：WRS/RIS 无偏权 + 时域 M-cap 合并；等验证预算 measured 方差对照；M100 低档面 0-byte\"}}",
        rep1.reference,
        rep1.uniform_mean,
        rep1.uniform_var,
        rep1.ris_mean,
        rep1.ris_var,
        rep1.ris_temporal_mean,
        rep1.ris_temporal_var,
        rep1.variance_reduction,
        rep1.temporal_reduction,
    );
    if let Some(parent) = std::path::Path::new(&out_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out_path, payload + "\n").expect("写 evidence 失败");
    println!("[g21_restir_probe] evidence → {out_path}");
    if !(unbiased_all && variance_gain_ok && temporal_gain_ok && double_run_bitexact) {
        eprintln!("[g21_restir_probe] FAIL");
        std::process::exit(1);
    }
    println!("[g21_restir_probe] PASS");
}
