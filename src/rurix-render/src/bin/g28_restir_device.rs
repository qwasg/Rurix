//! G28.2 M-a/M-b ReSTIR device kernel + 空间重用加性臂 harness(门
//! g28.p0.m_a.restir_device_kernel + g28.p0.m_b.restir_spatial_reuse_arm;
//! G28_CONTRACT §4.2 M-a/M-b 行逐字;RFC-0045 §1/§2 判据事实源;
//! g26_framegen_device 同模)。
//!
//! ## 集成路径
//!
//! bin-local 全部逻辑:`DeviceRestir` 经 `rurix_rt::vk::run_compute`
//! (G12/G13/G26/G27 compute 派发面同车道)单 dispatch 驱动
//! `kernels/g28_restir.rx`(逐 trial 单 invocation,dispatch [n_trials,1,1])。
//! host 金标准 `gi/restir_reservoir.rs`(estimate_ris/update/merge/
//! unbiased_weight/target_phat/exact_direct/fixture_lights/Pcg32)只消费不改写
//! ——**gi/restir_reservoir.rs + gi/multi_light.rs vs g27-closed 0-byte**
//! (RFC-0045 §1.8 冻结机核归 CI)。
//!
//! ## 随机带单源纪律(RFC-0045 §1.2;录制形态禁第二实现)
//!
//! PCG32 u64 状态面整体留 host;bin-local 录制器以 `Pcg32` Copy 快照 + 冻结
//! 模块 `update` 本体驱动录制(消费判定事实 = update 后 `r.w_sum > 0`;消费值
//! = update 前快照重放 `next_f32()`;候选抽取与 w 提升两行字面同源复写)——
//! 禁在 bin 内复刻 update/merge 判定逻辑的第二份消费点实现。**录制自检锚
//! (F2)**:录制循环终态 reservoir 四元组 (y, phat_y, w_sum, m) 与
//! `estimate_ris` 直调终态逐 trial 位级相等,不等即 FAIL 不进对拍。host 对拍
//! 参考值 = `estimate_ris` **直调**产出(录制器只产随机带不产参考值)。
//!
//! ## 判据面(RFC-0045 §1.5)
//!
//! - **前置整数锚**:逐 trial 保留样本 y(device f32,−1 哨兵 = 空)与 host y
//!   全等(真实承重锚)+ 判定带消费计数 device vs host 模拟全等(钉死夹具下
//!   恒 16 的平凡化事实如实照登,恒跑防协议漂移);
//! - device vs host 逐 trial estimate 绝对差 p100 ≤ 标定容差(标定腿程序产
//!   threshold = measured × 2.0 冻结 k 归 CI;实测 p100=0 → 零容差零条目);
//! - 无偏 3σ 维持:device 20000 estimate 均值(host f64 顺序累加聚合)vs
//!   `exact_direct` 解析参考,dev < 3σ_mean + 1e-9;
//! - device 双跑位级一致(固定输入两跑输出缓冲 digest 位级相等);
//! - kernel-bias RED 臂:params[3] 注入 RED_BIAS=0.05 输出面加性偏置 →
//!   对拍必超容差检出。
//!
//! ## 空间重用加性臂(--spatial;RFC-0045 §2;纯 host 不需 GPU)
//!
//! 8×8 着色点网格(N=8 闭集)× fixture_lights(64);每点每 trial 单流
//! stream = (t·64+p)·4+3;本点 `estimate_ris`(16 候选)→ gather 合并前快照
//! 闭集 → von Neumann 4 邻接字面固定序 (−1,0)(+1,0)(0,−1)(0,+1) →
//! **受点重评快照变换后直调冻结 merge(禁第二实现)**:
//! other' = Reservoir { y, phat_y: p̂_受点, w_sum: p̂_受点·W_other·other.m,
//! m: other.m }(W_other = other.unbiased_weight() 冻结 API 直调;m_cap=60)
//! → estimate = phat_y·unbiased_weight() 逐字;no-reuse 对照 = 合并前快照
//! estimate(同 trial 同流成对,零额外 RNG)。判据 = 聚合 3σ 硬门 + 逐点 5σ
//! 结构兜底 + 逐点 3σ 诊断表 64 行登记(非门面,多重比较口径注明)+ 方差再
//! 收益 min/mean/max measured 登记(**不设通过线**)+ 双跑位级。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备 → M-a device 腿 `skipped_dev_env` JSON 退 0(非
//! fake pass;`RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决);
//! --spatial 与 --host-only 纯 host 恒跑;判据不符 / RED 臂失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g28_restir_device --spv <k.spv> --tol <F>          # 全档验证(默认)
//! g28_restir_device --calibrate --spv <k.spv>        # 标定腿(全 trial p100)
//! g28_restir_device --red-arm kernel-bias --spv <k.spv> [--tol <F>]
//! g28_restir_device --probe --spv <k.spv> --tol <F> [--out <path>]  # soak 快车道
//! g28_restir_device --spatial [--out <path>]         # M-b 空间臂(纯 host)
//! g28_restir_device --host-only
//! ```

#![forbid(unsafe_code)]

use rurix_render::gi::restir_reservoir::{
    Pcg32, PointLight, Reservoir, ShadePoint, estimate_ris, exact_direct, fixture_lights,
    target_phat,
};
use rurix_rt::vk;

const TAG: &str = "[g28_restir_device]";
/// 夹具字面(RFC-0045 §1.4;g21_restir_probe 逐字同源)。
const SEED: u64 = 0x0521_A011_2026_0824;
const N_TRIALS: u32 = 20_000;
const M_CANDIDATES: u32 = 16;
const N_LIGHTS: u32 = 64;
/// probe soak 快车道:2000 trial 子集。
const PROBE_TRIALS: u32 = 2_000;
/// RED 臂注入幅(RFC-0045 §1.5 ④;g13/g26 同值;标定容差绝对上界 =
/// RED_BIAS × 0.5 由 CI 断言)。
const RED_BIAS: f32 = 0.05;
/// 空间臂(RFC-0045 §2):8×8 网格闭集 + M-cap 60(probe 字面)。
const GRID_N: usize = 8;
const N_POINTS: usize = GRID_N * GRID_N;
const M_CAP: u32 = 60;
const N_TRIALS_SPATIAL: u32 = 20_000;
/// von Neumann 4-邻接字面固定序(RFC-0045 §2.2;(di,dj) 于行主序 p = i·8+j)。
const NEIGHBOR_ORDER: [(i64, i64); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 夹具(variance_experiment 字面着色点;灯表 host 单源生成一次原字节上传)
// ---------------------------------------------------------------------------

fn shade_point() -> ShadePoint {
    ShadePoint {
        pos: [0.0, 0.0, 0.0],
        normal: [0.0, 1.0, 0.0],
    }
}

fn lights_flat(lights: &[PointLight]) -> Vec<f32> {
    lights
        .iter()
        .flat_map(|l| [l.pos[0], l.pos[1], l.pos[2], l.intensity])
        .collect()
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn load_spv(path: &str) -> Vec<u32> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("读 {path}: {e}")));
    if bytes.len() % 4 != 0 {
        fail("SPIR-V 字节数非 4 对齐");
    }
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn sha256_f32(v: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for &x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    rurix_pkg::sha256::hex_digest(&bytes)
}

fn sha256_f64(v: &[f64]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 8);
    for &x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    rurix_pkg::sha256::hex_digest(&bytes)
}

// ---------------------------------------------------------------------------
// 随机带录制器(RFC-0045 §1.2 字面;冻结 update 本体驱动 + Copy 快照重放;
// 录制自检锚 F2:录制终态 vs estimate_ris 直调终态逐 trial 位级相等)
// ---------------------------------------------------------------------------

struct Bands {
    /// 候选带:每 trial 满长度 16 个 [0,63] 整数值(f32 精确承载)。
    cand: Vec<f32>,
    /// 判定带:仅在 update 真实消费点(w_sum>0)产出的 next_f32 变长序列。
    dec: Vec<f32>,
    /// 逐 trial 三元组 [cand_offset, dec_offset, dec_len](F1;统一走表)。
    offsets: Vec<f32>,
    /// host 对拍参考 = estimate_ris 直调(每 trial 新 Pcg32 同流)。
    host_est: Vec<f64>,
    /// host 保留样本 y(整数锚真实承重面)。
    host_y: Vec<usize>,
    /// host 判定带消费计数模拟(钉死夹具下恒 16,平凡化事实照登)。
    host_dec_len: Vec<u32>,
}

fn record_bands(lights: &[PointLight], n_trials: u32) -> Bands {
    let sp = shade_point();
    let n = lights.len();
    let mut b = Bands {
        cand: Vec::with_capacity(n_trials as usize * M_CANDIDATES as usize),
        dec: Vec::with_capacity(n_trials as usize * M_CANDIDATES as usize),
        offsets: Vec::with_capacity(n_trials as usize * 3),
        host_est: Vec::with_capacity(n_trials as usize),
        host_y: Vec::with_capacity(n_trials as usize),
        host_dec_len: Vec::with_capacity(n_trials as usize),
    };
    for t in 0..n_trials {
        // RIS 流字面:stream = t·4+1(variance_experiment 三流 k=1 = RIS)。
        let mut rng = Pcg32::new(SEED, u64::from(t) * 4 + 1);
        let mut r = Reservoir::empty();
        let cand_offset = b.cand.len();
        let dec_offset = b.dec.len();
        for _ in 0..M_CANDIDATES {
            // 候选抽取与 w 提升两行字面同源复写(RFC-0045 §1.2 允许面)。
            let cand = (rng.next_u32() as usize) % n;
            let phat = target_phat(&sp, &lights[cand]);
            let w = f64::from(phat) * n as f64;
            let pre = rng; // Pcg32 Copy 快照(update 前)
            r.update(cand, phat, w, &mut rng); // 冻结 update 本体驱动
            if r.w_sum > 0.0 {
                // 消费判定事实 = update 后 w_sum>0;消费值 = 快照重放 next_f32。
                let mut replay = pre;
                b.dec.push(replay.next_f32());
            }
            b.cand.push(cand as f32);
        }
        let dec_len = b.dec.len() - dec_offset;
        // ── 录制自检锚(F2):录制终态 vs estimate_ris 直调终态逐 trial 位级 ──
        let mut rng_ref = Pcg32::new(SEED, u64::from(t) * 4 + 1);
        let (est_ref, r_ref) = estimate_ris(&sp, lights, M_CANDIDATES, &mut rng_ref);
        if r.y != r_ref.y
            || r.phat_y.to_bits() != r_ref.phat_y.to_bits()
            || r.w_sum.to_bits() != r_ref.w_sum.to_bits()
            || r.m != r_ref.m
        {
            fail(&format!(
                "录制自检锚失败 trial {t}:录制终态 (y={}, phat_y={:e}, w_sum={:e}, m={}) ≠ \
                 estimate_ris 直调终态 (y={}, phat_y={:e}, w_sum={:e}, m={})(位级)",
                r.y, r.phat_y, r.w_sum, r.m, r_ref.y, r_ref.phat_y, r_ref.w_sum, r_ref.m
            ));
        }
        b.offsets
            .extend_from_slice(&[cand_offset as f32, dec_offset as f32, dec_len as f32]);
        b.host_est.push(est_ref);
        b.host_y.push(r_ref.y);
        b.host_dec_len.push(dec_len as u32);
    }
    b
}

// ---------------------------------------------------------------------------
// device 臂(bin-local;单 dispatch [n_trials,1,1] 经 vk::run_compute)
// ---------------------------------------------------------------------------

/// kernel 参数面打包(与 g28_restir.rx 参数面逐字同源;8 f32 位级编码)。
fn pack_params(n_trials: u32, red_bias: f32) -> Vec<f32> {
    let mut v = vec![
        n_trials as f32,
        N_LIGHTS as f32,
        M_CANDIDATES as f32,
        red_bias,
    ];
    v.resize(8, 0.0);
    v
}

struct DeviceRestir {
    spv: Vec<u32>,
    entry: String,
    red_bias: f32,
}

impl DeviceRestir {
    fn create(spv: Vec<u32>) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let entry = vk::entry_point_name(&spv).ok_or("SPV 无 OpEntryPoint")?;
        Ok(Self {
            spv,
            entry,
            red_bias: 0.0,
        })
    }

    /// 单 dispatch 全 trial;返回输出缓冲(4 f32/trial
    /// [estimate, y(−1=空), dec_consumed, phat_y])。
    fn run(&self, lights: &[PointLight], bands: &Bands, n_trials: u32) -> Vec<f32> {
        let params = pack_params(n_trials, self.red_bias);
        let mut bufs = vec![
            bytes_f32(&lights_flat(lights)),
            bytes_f32(&bands.cand),
            bytes_f32(&bands.dec),
            bytes_f32(&bands.offsets),
            bytes_f32(&params),
            vec![0u8; n_trials as usize * 16],
        ];
        vk::run_compute(&self.spv, &self.entry, &mut bufs, &[], [n_trials, 1, 1])
            .unwrap_or_else(|e| panic!("restir dispatch 失败: {e}"));
        read_f32(&bufs[5])
    }
}

// ---------------------------------------------------------------------------
// 判据(整数锚 / estimate p100 / 无偏 3σ / 双跑位级)
// ---------------------------------------------------------------------------

/// y 整数锚:device f32(−1 哨兵 = 空)转回 usize 与 host y 全等。
fn y_matches(dev_y: f32, host_y: usize) -> bool {
    if host_y == usize::MAX {
        dev_y == -1.0
    } else {
        dev_y == host_y as f32
    }
}

struct IntegerAnchor {
    y_all_equal: bool,
    dec_all_equal: bool,
    dec_constant_16: bool,
    first_mismatch: Option<usize>,
}

fn check_integer_anchor(out: &[f32], bands: &Bands, n_trials: u32) -> IntegerAnchor {
    let mut a = IntegerAnchor {
        y_all_equal: true,
        dec_all_equal: true,
        dec_constant_16: true,
        first_mismatch: None,
    };
    for t in 0..n_trials as usize {
        let dev_y = out[t * 4 + 1];
        let dev_dec = out[t * 4 + 2];
        if !y_matches(dev_y, bands.host_y[t]) {
            a.y_all_equal = false;
            if a.first_mismatch.is_none() {
                a.first_mismatch = Some(t);
            }
        }
        if dev_dec != bands.host_dec_len[t] as f32 {
            a.dec_all_equal = false;
            if a.first_mismatch.is_none() {
                a.first_mismatch = Some(t);
            }
        }
        if bands.host_dec_len[t] != M_CANDIDATES {
            a.dec_constant_16 = false;
        }
    }
    a
}

/// 逐 trial estimate 绝对差 p100(device f32 提升 f64 vs host f64 直调参考)。
fn estimate_p100(out: &[f32], host_est: &[f64], n_trials: u32) -> f64 {
    let mut p100 = 0.0f64;
    for t in 0..n_trials as usize {
        let d = (f64::from(out[t * 4]) - host_est[t]).abs();
        if d > p100 {
            p100 = d;
        }
    }
    p100
}

/// 无偏 3σ(RFC-0045 §1.5 ②):device estimate 均值(f64 顺序累加)vs
/// exact_direct;返回 (pass, mean, dev, bound)。
fn unbiased_3sigma(out: &[f32], reference: f64, n_trials: u32) -> (bool, f64, f64, f64) {
    let n = n_trials as usize;
    let mut sum = 0.0f64;
    for t in 0..n {
        sum += f64::from(out[t * 4]);
    }
    let mean = sum / n as f64;
    let mut var_sum = 0.0f64;
    for t in 0..n {
        let d = f64::from(out[t * 4]) - mean;
        var_sum += d * d;
    }
    let var = var_sum / (n - 1) as f64;
    let sigma_mean = (var / n as f64).sqrt();
    let dev = (mean - reference).abs();
    let bound = 3.0 * sigma_mean;
    (dev < bound + 1e-9, mean, dev, bound)
}

// ---------------------------------------------------------------------------
// JSON 出报(手写,零新依赖;g26_framegen_device 同模)
// ---------------------------------------------------------------------------

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn jstr(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn strs_json(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| jstr(s)).collect();
    format!("[{}]", inner.join(","))
}

fn base_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// 提交时刻(git log -1 %cI;同 commit 内确定——标定腿两跑位级一致的时间戳面)。
fn utc_now() -> String {
    std::process::Command::new("git")
        .args(["log", "-1", "--format=%cI"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

struct Args {
    spv: Option<String>,
    tol: f64,
    calibrate: bool,
    red_arm: Option<String>,
    probe: bool,
    spatial: bool,
    host_only: bool,
    out: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        spv: None,
        tol: 0.0,
        calibrate: false,
        red_arm: None,
        probe: false,
        spatial: false,
        host_only: false,
        out: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            "--spv" => a.spv = it.next(),
            "--tol" => {
                a.tol = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --tol 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--tol 非 f64"))
            }
            "--calibrate" => a.calibrate = true,
            "--red-arm" => a.red_arm = it.next(),
            "--probe" => a.probe = true,
            "--spatial" => a.spatial = true,
            "--host-only" => a.host_only = true,
            "--out" => a.out = it.next(),
            other => fail(&format!("未知参数: {other}")),
        }
    }
    a
}

fn device_arm(args: &Args) -> Result<DeviceRestir, String> {
    let spv = load_spv(args.spv.as_deref().unwrap_or_else(|| fail("缺 --spv")));
    DeviceRestir::create(spv)
}

fn emit_line(line: &str, out: &Option<String>) {
    println!("{line}");
    if let Some(path) = out {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, format!("{line}\n"))
            .unwrap_or_else(|e| fail(&format!("写 --out {path}: {e}")));
    }
}

// ---------------------------------------------------------------------------
// 标定腿(全 20000 trial estimate 绝对差 p100;两跑位级一致由 CI 裁决)
// ---------------------------------------------------------------------------

fn calibrate_leg(args: &Args) -> ! {
    let dev = match device_arm(args) {
        Ok(d) => d,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g28restir.calibration_skip.v1\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(&e)
            );
            std::process::exit(0);
        }
    };
    let lights = fixture_lights(N_LIGHTS);
    let bands = record_bands(&lights, N_TRIALS);
    let out = dev.run(&lights, &bands, N_TRIALS);
    let mut diffs = Vec::with_capacity(N_TRIALS as usize);
    let mut p100 = 0.0f64;
    for t in 0..N_TRIALS as usize {
        let d = (f64::from(out[t * 4]) - bands.host_est[t]).abs();
        diffs.push(d);
        if d > p100 {
            p100 = d;
        }
    }
    let protocol = format!(
        "ReSTIR device vs host 金标准(estimate_ris 直调)同输入逐 trial estimate 对拍容差\
         ({N_TRIALS} trial × M={M_CANDIDATES} 候选 × {N_LIGHTS} 灯环形夹具,随机带 host 单源\
         预生成已对齐消费序 + 录制自检锚位级前置,逐 trial 绝对差 p100;threshold = \
         measured × 2.0 冻结 k,方向 max,禁手写;实测 p100=0 → 零容差零条目;RFC-0045 §1.5)"
    );
    println!(
        "{{\"schema\":\"rurix.g28restir.calibration_entry.v1\",\"entry_id\":\"g28.restir_device.host_device_estimate_tol\",\"results\":{{\"trimmed_mean\":{:.15e}}},\"protocol\":{},\"sample_manifest\":{{\"count\":{},\"digest\":{}}},\"provenance\":{{\"gpu\":\"device\",\"backend\":\"restir_device\",\"base_commit\":{}}},\"timestamp\":{}}}",
        p100,
        jstr(&protocol),
        N_TRIALS,
        jstr(&format!("sha256:{}", sha256_f64(&diffs))),
        jstr(&base_commit()),
        jstr(&utc_now()),
    );
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// RED 臂(kernel-bias:params[3] 注入 RED_BIAS=0.05 → 对拍必超容差)
// ---------------------------------------------------------------------------

fn red_arm_kernel_bias(args: &Args) -> Result<String, String> {
    let lights = fixture_lights(N_LIGHTS);
    let bands = record_bands(&lights, N_TRIALS);
    let honest = device_arm(args)?;
    let honest_out = honest.run(&lights, &bands, N_TRIALS);
    let honest_p100 = estimate_p100(&honest_out, &bands.host_est, N_TRIALS);
    let mut tampered = device_arm(args)?;
    tampered.red_bias = RED_BIAS;
    let tampered_out = tampered.run(&lights, &bands, N_TRIALS);
    let tampered_p100 = estimate_p100(&tampered_out, &bands.host_est, N_TRIALS);
    let detected = if args.tol > 0.0 {
        tampered_p100 > args.tol
    } else {
        // 零容差态:偏置注入 ⇒ p100 ≈ 0.05 ≠ 0 必红(RFC-0045 §1.5 ④ 平凡成立仍恒跑)。
        tampered_p100 > honest_p100 + f64::from(RED_BIAS) * 0.5
    };
    if !detected {
        return Err(format!(
            "kernel-bias 漏检:tampered p100={tampered_p100:.6e} vs tol={:.6e} honest={honest_p100:.6e}",
            args.tol
        ));
    }
    Ok(format!(
        "honest p100={honest_p100:.6e} tampered p100={tampered_p100:.6e} tol={:.6e}",
        args.tol
    ))
}

// ---------------------------------------------------------------------------
// probe(soak 快车道:2000 trial 子集对拍 + y 锚 + 双跑位级;单行 JSON)
// ---------------------------------------------------------------------------

fn probe_leg(args: &Args) -> ! {
    let dev = match device_arm(args) {
        Ok(d) => d,
        Err(e) => {
            let line = format!(
                "{{\"schema\":\"rurix.g28restir.probe.v1\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(&e)
            );
            emit_line(&line, &args.out);
            std::process::exit(0);
        }
    };
    let lights = fixture_lights(N_LIGHTS);
    let bands = record_bands(&lights, PROBE_TRIALS);
    let out_a = dev.run(&lights, &bands, PROBE_TRIALS);
    let out_b = dev.run(&lights, &bands, PROBE_TRIALS);
    let anchor = check_integer_anchor(&out_a, &bands, PROBE_TRIALS);
    let p100 = estimate_p100(&out_a, &bands.host_est, PROBE_TRIALS);
    let digest_a = sha256_f32(&out_a);
    let digest_b = sha256_f32(&out_b);
    let bitexact = digest_a == digest_b;
    let in_tol = p100 <= args.tol;
    let anchor_ok = anchor.y_all_equal && anchor.dec_all_equal;
    let state = if in_tol && bitexact && anchor_ok {
        "pass"
    } else {
        "fail"
    };
    let line = format!(
        "{{\"schema\":\"rurix.g28restir.probe.v1\",\"state\":{},\"trials\":{},\"p100_vs_host\":{:.15e},\"tol\":{:.15e},\"in_tol\":{},\"y_anchor_all_equal\":{},\"dec_consumed_all_equal\":{},\"bitexact\":{},\"digest\":{},\"base_commit\":{}}}",
        jstr(state),
        PROBE_TRIALS,
        p100,
        args.tol,
        in_tol,
        anchor.y_all_equal,
        anchor.dec_all_equal,
        bitexact,
        jstr(&digest_a),
        jstr(&base_commit()),
    );
    emit_line(&line, &args.out);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// M-b 空间重用加性臂(--spatial;纯 host;RFC-0045 §2)
// ---------------------------------------------------------------------------

/// 8×8 网格闭集(行主序 p = i·8+j;pos = (−1.75+0.5·i, 0, −1.75+0.5·j))。
fn spatial_points() -> Vec<ShadePoint> {
    let mut pts = Vec::with_capacity(N_POINTS);
    for i in 0..GRID_N {
        for j in 0..GRID_N {
            pts.push(ShadePoint {
                pos: [-1.75 + 0.5 * i as f32, 0.0, -1.75 + 0.5 * j as f32],
                normal: [0.0, 1.0, 0.0],
            });
        }
    }
    pts
}

/// 一次空间臂全跑:返回 (reuse 矩阵, no-reuse 矩阵),行主序 [t·64+p] f64。
fn run_spatial(lights: &[PointLight], pts: &[ShadePoint], n_trials: u32) -> (Vec<f64>, Vec<f64>) {
    let n = n_trials as usize;
    let mut reuse = vec![0.0f64; n * N_POINTS];
    let mut noreuse = vec![0.0f64; n * N_POINTS];
    let mut snaps: Vec<Reservoir> = Vec::with_capacity(N_POINTS);
    let mut rngs: Vec<Pcg32> = Vec::with_capacity(N_POINTS);
    for t in 0..n {
        snaps.clear();
        rngs.clear();
        // ── 本点 RIS(流 = (t·64+p)·4+3;k=3 残差类与 §1/probe 三流构造性不相交);
        //    gather 合并前快照闭集(禁 in-place 链式污染)──
        for (p, sp) in pts.iter().enumerate() {
            let stream = (t as u64 * N_POINTS as u64 + p as u64) * 4 + 3;
            let mut rng = Pcg32::new(SEED, stream);
            let (est, r) = estimate_ris(sp, lights, M_CANDIDATES, &mut rng);
            // no-reuse 对照 = 合并前快照 estimate(同 trial 同流成对,零额外 RNG)。
            noreuse[t * N_POINTS + p] = est;
            snaps.push(r);
            rngs.push(rng);
        }
        // ── 受点重评快照变换后直调冻结 merge(邻域字面固定序;m_cap=60)──
        for p in 0..N_POINTS {
            let (pi, pj) = ((p / GRID_N) as i64, (p % GRID_N) as i64);
            let mut merged = snaps[p];
            let rng = &mut rngs[p];
            for (di, dj) in NEIGHBOR_ORDER {
                let (ni, nj) = (pi + di, pj + dj);
                if ni < 0 || ni >= GRID_N as i64 || nj < 0 || nj >= GRID_N as i64 {
                    // 越界邻居缺席 = 不调用 merge(与空 other 早退同义零消费)。
                    continue;
                }
                let q = (ni as usize) * GRID_N + nj as usize;
                let other = &snaps[q];
                if other.y == usize::MAX {
                    // 空 other:直调冻结 merge 早退(零消费;夹具全支撑下不可达,
                    // 走冻结本体裁决而非 bin 内复刻判定)。
                    merged.merge(other, rng, M_CAP);
                    continue;
                }
                // 受点重评快照变换(RFC-0045 §2.2 F5):
                // other' = { y, p̂_受点(y), p̂_受点(y)·W_other·other.m, other.m };
                // W_other = other.unbiased_weight() 冻结 API 直调。merge 对 other'
                // 的字面等效权 = p̂_受点·W_other·m_other 恰为受点重评律(代数恒等
                // 在档),合并本体仍为冻结 merge 直调——bin 内零合并判定复刻。
                let w_other = other.unbiased_weight();
                let phat_recv = target_phat(&pts[p], &lights[other.y]);
                let other_prime = Reservoir {
                    y: other.y,
                    phat_y: phat_recv,
                    w_sum: f64::from(phat_recv) * w_other * f64::from(other.m),
                    m: other.m,
                };
                merged.merge(&other_prime, rng, M_CAP);
            }
            // estimate = phat_y·unbiased_weight() 逐字(§1.1 禁化简同律)。
            reuse[t * N_POINTS + p] = f64::from(merged.phat_y) * merged.unbiased_weight();
        }
    }
    (reuse, noreuse)
}

/// 顺序 f64 均值/样本方差(RFC-0045 §5.7:统计聚合 host f64 顺序累加)。
fn mean_var(vals: &[f64]) -> (f64, f64) {
    let n = vals.len();
    let mut sum = 0.0f64;
    for &v in vals {
        sum += v;
    }
    let mean = sum / n as f64;
    let mut var_sum = 0.0f64;
    for &v in vals {
        var_sum += (v - mean) * (v - mean);
    }
    (mean, var_sum / (n - 1) as f64)
}

fn spatial_leg(args: &Args) -> ! {
    let lights = fixture_lights(N_LIGHTS);
    let pts = spatial_points();
    let n = N_TRIALS_SPATIAL as usize;
    let refs: Vec<f64> = pts.iter().map(|p| exact_direct(p, &lights)).collect();
    let mut ref_grid = 0.0f64;
    for &r in &refs {
        ref_grid += r;
    }
    ref_grid /= N_POINTS as f64;

    let t0 = std::time::Instant::now();
    let (reuse_a, noreuse_a) = run_spatial(&lights, &pts, N_TRIALS_SPATIAL);
    let single_run_seconds = t0.elapsed().as_secs_f64();
    let (reuse_b, noreuse_b) = run_spatial(&lights, &pts, N_TRIALS_SPATIAL);
    // ── 双跑位级:全网格 estimate 矩阵(reuse + no-reuse)位级相等 ──
    let digest_a = format!("{}:{}", sha256_f64(&reuse_a), sha256_f64(&noreuse_a));
    let digest_b = format!("{}:{}", sha256_f64(&reuse_b), sha256_f64(&noreuse_b));
    let bitexact = digest_a == digest_b;

    // ── 聚合 3σ 硬门:逐 trial 网格均值序列的 n-trial 均值 vs 64 点参考均值 ──
    let mut grid_means = Vec::with_capacity(n);
    for t in 0..n {
        let mut s = 0.0f64;
        for p in 0..N_POINTS {
            s += reuse_a[t * N_POINTS + p];
        }
        grid_means.push(s / N_POINTS as f64);
    }
    let (agg_mean, agg_var) = mean_var(&grid_means);
    let agg_sigma_mean = (agg_var / n as f64).sqrt();
    let agg_dev = (agg_mean - ref_grid).abs();
    let agg_bound = 3.0 * agg_sigma_mean;
    let aggregate_pass = agg_dev < agg_bound + 1e-9;

    // ── 逐点:5σ 结构兜底(门)+ 3σ 诊断登记(非门,多重比较口径注明)+
    //    方差再收益 measured 登记(不设通过线)──
    let mut rows: Vec<String> = Vec::with_capacity(N_POINTS);
    let mut all_within_5 = true;
    let mut within_3_count = 0usize;
    let mut worst_ratio = 0.0f64;
    let mut worst_point = 0usize;
    let mut gain_min = f64::INFINITY;
    let mut gain_max = f64::NEG_INFINITY;
    let mut gain_sum = 0.0f64;
    for p in 0..N_POINTS {
        let series_reuse: Vec<f64> = (0..n).map(|t| reuse_a[t * N_POINTS + p]).collect();
        let series_noreuse: Vec<f64> = (0..n).map(|t| noreuse_a[t * N_POINTS + p]).collect();
        let (m_r, v_r) = mean_var(&series_reuse);
        let (_m_n, v_n) = mean_var(&series_noreuse);
        let sigma_mean = (v_r / n as f64).sqrt();
        let dev = (m_r - refs[p]).abs();
        let within_3 = dev < 3.0 * sigma_mean + 1e-9;
        let within_5 = dev < 5.0 * sigma_mean + 1e-9;
        if within_3 {
            within_3_count += 1;
        }
        if !within_5 {
            all_within_5 = false;
        }
        let ratio = if sigma_mean > 0.0 {
            dev / sigma_mean
        } else {
            0.0
        };
        if ratio > worst_ratio {
            worst_ratio = ratio;
            worst_point = p;
        }
        let gain = v_n / v_r.max(1e-30);
        if gain < gain_min {
            gain_min = gain;
        }
        if gain > gain_max {
            gain_max = gain;
        }
        gain_sum += gain;
        rows.push(format!(
            "{{\"p\":{},\"i\":{},\"j\":{},\"mean\":{:.9e},\"reference\":{:.9e},\"dev\":{:.9e},\"sigma_mean\":{:.9e},\"dev_over_sigma\":{:.4},\"within_3sigma\":{},\"within_5sigma\":{},\"var_no_reuse\":{:.9e},\"var_reuse\":{:.9e},\"var_gain\":{:.6}}}",
            p,
            p / GRID_N,
            p % GRID_N,
            m_r,
            refs[p],
            dev,
            sigma_mean,
            ratio,
            within_3,
            within_5,
            v_n,
            v_r,
            gain,
        ));
    }
    let gain_mean = gain_sum / N_POINTS as f64;

    let state = if aggregate_pass && all_within_5 && bitexact {
        "pass"
    } else {
        "fail"
    };
    eprintln!(
        "{TAG}: spatial n_trials={N_TRIALS_SPATIAL} 单跑 {single_run_seconds:.1}s agg_dev={agg_dev:.3e} bound={agg_bound:.3e} pass={aggregate_pass} all_within_5sigma={all_within_5} within_3sigma={within_3_count}/64 gain(min/mean/max)={gain_min:.3}/{gain_mean:.3}/{gain_max:.3} bitexact={bitexact}"
    );
    let line = format!(
        "{{\"schema\":\"rurix.g28restir.spatial.v1\",\"state\":{},\"grid\":\"{GRID_N}x{GRID_N}\",\"n_points\":{N_POINTS},\"m_candidates\":{M_CANDIDATES},\"m_cap\":{M_CAP},\"neighbor_order\":\"(-1,0)(+1,0)(0,-1)(0,+1)\",\"window\":{{\"n_trials\":{N_TRIALS_SPATIAL},\"downgraded\":false,\"single_run_seconds\":{:.3},\"note\":\"RFC-0045 §2.4 基线窗长 20000 trial;3σ 判据在本窗长下有效\"}},\"aggregate_3sigma\":{{\"mean\":{:.12e},\"reference\":{:.12e},\"dev\":{:.9e},\"sigma_mean\":{:.9e},\"bound_3sigma\":{:.9e},\"pass\":{}}},\"per_point_5sigma\":{{\"all_within\":{},\"worst_dev_over_sigma\":{:.4},\"worst_point\":{}}},\"per_point_3sigma_diagnostic\":{{\"within_count\":{},\"of\":{N_POINTS},\"gate\":false,\"note\":\"3σ×64 点族期望假红 ≈ 0.17——诊断登记面非门面(多重比较口径,RFC-0045 §2.4 ①);门面 = 聚合 3σ + 逐点 5σ\"}},\"variance_gain\":{{\"min\":{:.6},\"mean\":{:.6},\"max\":{:.6},\"no_pass_line\":true,\"note\":\"var(no-reuse)/var(reuse) measured 如实登记不设通过线(G6 无硬门纪律;RFC-0045 §2.4 ②)\"}},\"double_run_bitexact\":{},\"digest\":{},\"per_point_rows\":[{}],\"base_commit\":{}}}",
        jstr(state),
        single_run_seconds,
        agg_mean,
        ref_grid,
        agg_dev,
        agg_sigma_mean,
        agg_bound,
        aggregate_pass,
        all_within_5,
        worst_ratio,
        worst_point,
        within_3_count,
        gain_min,
        gain_mean,
        gain_max,
        bitexact,
        jstr(&digest_a),
        rows.join(","),
        jstr(&base_commit()),
    );
    emit_line(&line, &args.out);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// host 腿(录制自检锚恒跑 + host 直调 3σ + 录制双跑位级)
// ---------------------------------------------------------------------------

fn host_only_leg(args: &Args) -> ! {
    let lights = fixture_lights(N_LIGHTS);
    let sp = shade_point();
    let reference = exact_direct(&sp, &lights);
    // record_bands 内嵌录制自检锚(逐 trial 位级,失败即 exit 1)。
    let bands_a = record_bands(&lights, N_TRIALS);
    let bands_b = record_bands(&lights, N_TRIALS);
    let band_digest_a = format!(
        "{}:{}:{}",
        sha256_f32(&bands_a.cand),
        sha256_f32(&bands_a.dec),
        sha256_f32(&bands_a.offsets)
    );
    let band_digest_b = format!(
        "{}:{}:{}",
        sha256_f32(&bands_b.cand),
        sha256_f32(&bands_b.dec),
        sha256_f32(&bands_b.offsets)
    );
    let bands_bitexact = band_digest_a == band_digest_b
        && sha256_f64(&bands_a.host_est) == sha256_f64(&bands_b.host_est);
    let (mean, var) = mean_var(&bands_a.host_est);
    let sigma_mean = (var / f64::from(N_TRIALS)).sqrt();
    let dev = (mean - reference).abs();
    let unbiased = dev < 3.0 * sigma_mean + 1e-9;
    let state = if unbiased && bands_bitexact {
        "pass"
    } else {
        "fail"
    };
    let line = format!(
        "{{\"schema\":\"rurix.g28restir.harness.v1\",\"mode\":\"host-only\",\"state\":{},\"n_trials\":{N_TRIALS},\"recorder_selfcheck_bitexact\":true,\"host\":{{\"mean\":{:.12e},\"reference\":{:.12e},\"dev\":{:.9e},\"bound_3sigma\":{:.9e},\"unbiased_3sigma\":{}}},\"bands_double_run_bitexact\":{},\"band_digest\":{}}}",
        jstr(state),
        mean,
        reference,
        dev,
        3.0 * sigma_mean,
        unbiased,
        bands_bitexact,
        jstr(&band_digest_a),
    );
    emit_line(&line, &args.out);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// main(默认 = 全档验证:整数锚前置 → estimate p100 → 无偏 3σ → 双跑位级)
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    if args.calibrate {
        calibrate_leg(&args);
    }
    if args.probe {
        probe_leg(&args);
    }
    if args.spatial {
        spatial_leg(&args);
    }
    if args.host_only {
        host_only_leg(&args);
    }
    if let Some(arm) = &args.red_arm {
        if arm != "kernel-bias" {
            fail(&format!("未知 RED 臂: {arm}(kernel-bias)"));
        }
        match red_arm_kernel_bias(&args) {
            Ok(detail) => {
                eprintln!("{TAG}: red-arm {arm} 检出 — {detail}");
                println!(
                    "{{\"schema\":\"rurix.g28restir.red_arm.v1\",\"arm\":{},\"detected\":true,\"detail\":{}}}",
                    jstr(arm),
                    jstr(&detail)
                );
                std::process::exit(0);
            }
            Err(e) if e.contains("不可用") => {
                println!(
                    "{{\"schema\":\"rurix.g28restir.red_arm.v1\",\"arm\":{},\"detected\":false,\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                    jstr(arm),
                    jstr(&e)
                );
                std::process::exit(0);
            }
            Err(e) => fail(&format!("red-arm {arm} 失效(漏检): {e}")),
        }
    }

    // ── 全档验证 ──
    let lights = fixture_lights(N_LIGHTS);
    let sp = shade_point();
    let reference = exact_direct(&sp, &lights);
    // host 腿恒跑:record_bands 内嵌录制自检锚(逐 trial 位级,失败即 exit 1)。
    let bands = record_bands(&lights, N_TRIALS);

    let dev = match device_arm(&args) {
        Ok(d) => d,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g28restir.harness.v1\",\"mode\":\"device\",\"state\":\"skipped_dev_env\",\"skip_reason\":{},\"recorder_selfcheck_bitexact\":true}}",
                jstr(&e)
            );
            return;
        }
    };

    let mut problems: Vec<String> = Vec::new();
    let out_a = dev.run(&lights, &bands, N_TRIALS);
    // ① 前置整数锚(y 真实承重 + 消费计数平凡化恒跑)——desync 即 FAIL 不进容差比较。
    let anchor = check_integer_anchor(&out_a, &bands, N_TRIALS);
    if !anchor.y_all_equal {
        problems.push(format!(
            "y 整数锚失败:保留样本 device vs host 不全等(首失配 trial {:?};f32 判定边界翻转即如实红,容差协议不豁免整数判定面)",
            anchor.first_mismatch
        ));
    }
    if !anchor.dec_all_equal {
        problems.push(format!(
            "判定带消费计数锚失败(首失配 trial {:?})",
            anchor.first_mismatch
        ));
    }
    // ② 逐 trial estimate 绝对差 p100 ≤ 标定容差(整数锚通过后方进入)。
    let p100 = estimate_p100(&out_a, &bands.host_est, N_TRIALS);
    let in_tol = p100 <= args.tol;
    if anchor.y_all_equal && anchor.dec_all_equal && !in_tol {
        problems.push(format!("estimate p100={p100:.6e} 超容差 {:.6e}", args.tol));
    }
    // ③ 无偏 3σ 维持(独立直接对解析参考复核,纵深防御并列)。
    let (unbiased, dev_mean, dev_dev, dev_bound) = unbiased_3sigma(&out_a, reference, N_TRIALS);
    if !unbiased {
        problems.push(format!(
            "device 无偏 3σ 失败:mean={dev_mean:.9} ref={reference:.9} dev={dev_dev:.3e} > bound={dev_bound:.3e}"
        ));
    }
    // ④ device 双跑位级一致(输出缓冲 digest)。
    let out_b = dev.run(&lights, &bands, N_TRIALS);
    let digest_a = sha256_f32(&out_a);
    let digest_b = sha256_f32(&out_b);
    let bitexact = digest_a == digest_b;
    if !bitexact {
        problems.push("device 双跑非位级一致".into());
    }

    let state = if problems.is_empty() { "pass" } else { "fail" };
    eprintln!(
        "{TAG}: 全档验证 n_trials={N_TRIALS} y_anchor={} dec_anchor={}(恒16={}) p100={p100:.6e} tol={:.6e} unbiased_3sigma={unbiased} bitexact={bitexact}",
        anchor.y_all_equal, anchor.dec_all_equal, anchor.dec_constant_16, args.tol
    );
    println!(
        "{{\"schema\":\"rurix.g28restir.harness.v1\",\"mode\":\"device\",\"state\":{},\"problems\":{},\"n_trials\":{N_TRIALS},\"m_candidates\":{M_CANDIDATES},\"n_lights\":{N_LIGHTS},\"recorder_selfcheck_bitexact\":true,\"y_anchor_all_equal\":{},\"dec_consumed_all_equal\":{},\"dec_consumed_constant_16\":{},\"p100_vs_host\":{:.15e},\"tol\":{:.15e},\"in_tol\":{},\"unbiased\":{{\"mean\":{:.12e},\"reference\":{:.12e},\"dev\":{:.9e},\"bound_3sigma\":{:.9e},\"pass\":{}}},\"bitexact\":{},\"digest\":{},\"base_commit\":{}}}",
        jstr(state),
        strs_json(&problems),
        anchor.y_all_equal,
        anchor.dec_all_equal,
        anchor.dec_constant_16,
        p100,
        args.tol,
        in_tol,
        dev_mean,
        reference,
        dev_dev,
        dev_bound,
        unbiased,
        bitexact,
        jstr(&digest_a),
        jstr(&base_commit()),
    );
    if !problems.is_empty() {
        std::process::exit(1);
    }
}
