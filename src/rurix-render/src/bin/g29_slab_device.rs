//! G29.2 M-a/M-b slab device kernel + 侧表供参加性臂 harness(门
//! g29.p0.m_a.slab_device_kernel + g29.p0.m_b.slab_side_table_arm;
//! G29_CONTRACT §4.2 M-a/M-b 行逐字;RFC-0046 §1/§2 判据事实源;
//! g28_restir_device 同模)。
//!
//! ## 集成路径
//!
//! bin-local 全部逻辑:`DeviceSlab` 经 `rurix_rt::vk::run_compute`
//! (G12/G13/G26/G27/G28 compute 派发面同车道)单 dispatch 驱动
//! `kernels/g29_slab.rx`(逐样本单 invocation,dispatch [n,1,1])。
//! host 金标准 `material/slab.rs::total_reflectance`(f64 直调)只消费不改写
//! ——**material/ 整目录 vs g28-closed 0-byte**(RFC-0046 §1.7 冻结机核归
//! CI;reserved RED 守卫本体同受机核保护)。
//!
//! ## M-a 网格(RFC-0046 §1.1;F4 血缘钉死)
//!
//! 16641 样本 = 129×129 参数网格(rc = i/128、ab = j/128,i,j ∈ 0..=128)
//! = `g22_slab_probe` GRID=128 经 `furnace_audit` (grid+1)² 格点口径;host
//! 单源生成一次原字节上传,device 不重算格点。公式面(修法 A 分母安全化):
//! `R = rc + tc·tc·ab / max(denom, 1e-30)`——角点 rc=ab=1 → 分子 0 →
//! 0/1e-30 = 0 → R = rc = 1.0 位级同 host `denom ≤ 0 ⇒ 1.0` 分支值。
//!
//! ## 判据面(RFC-0046 §1.4;程序产禁手写)
//!
//! - ⓪ **输出有限性一等断言**:16641 样本全量 `is_finite`,任一非有限 →
//!   硬 FAIL(**先于对拍聚合执行**——封死 NaN 经 `f64::max(NaN, x) = x`
//!   聚合静默吞掉的假绿路径,F3);
//! - ① 逐样本 |device R − host R| p100 ≤ 标定容差(threshold = measured ×
//!   2.0 冻结 k 程序产归 CI;实测 p100=0 → 零容差零条目);
//! - ② 白炉行登记:ab=1 列(129 样本)device dev 最大值如实登记——host 白炉
//!   R 位级 ≡ 1.0 可断言(网格值下 tc²/tc 数学商可表示,f64 正确舍入);
//!   device dev 来源 = Vulkan FP32 OpFDiv ≤2.5 ULP + FMA 收缩可能性,不冒充
//!   解析 0;覆盖论证:白炉行 ⊂ 网格 ⇒ 已被判据①容差线传递覆盖(F1);
//! - ③ 能量上界 device 复核:全样本 device R ≤ 1 + 容差;
//! - ④ device 双跑位级一致(固定输入两跑输出缓冲 digest 位级相等);
//! - ⑤ kernel-bias RED 臂:red_bias = 0.05 → 对拍必超容差;**臂间判据归属
//!   (F11)**:RED 臂仅评判据①,判据⓪②③④不跨臂执行。
//!
//! ## 侧表供参加性臂(--side-table;RFC-0046 §2;bin-local)
//!
//! 16 材质槽 slab 参数侧表(bin 内合成独立 SSBO:逐槽 [rc, ab],rc_k =
//! k/15·0.95、ab_k = (15−k)/15;**0.95 上限系有意规避 denom→0 角点区**——
//! 角点语义覆盖由 M-a 主网格独担,F5;host 单源生成一次原字节上传,device
//! 不重算槽参数——k/15 非 2 幂分母求值序位级敏感)。device 逐槽求值 + host
//! `total_reflectance` 直调对拍(p100 同 M-a 容差协议)+ **逐槽白炉互核**
//! (每槽 ab=1 变体 host/device 双端重算,dev 逐槽登记)+ 双跑位级。
//! **防混淆声明(F8)**:本臂「侧表」= bin-local slab 参数 SSBO(bin 内合成
//! 不落资产),与 material/ 生产资产侧表设施(RFC-0025 Burley/Marschner
//! 通道)零关系零触碰,禁挂接其编解码/digest 设施;MaterialClosure 32B 布局
//! 与 reserved 拓扑位零触碰(graph/types.rs 0-byte 机核归 CI)。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备 → device 腿 `skipped_dev_env` JSON 退 0(非 fake
//! pass;`RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决);
//! --host-only 纯 host 恒跑;判据不符 / RED 臂失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g29_slab_device --spv <k.spv> --tol <F>          # 全档验证(默认)
//! g29_slab_device --calibrate --spv <k.spv>        # 标定腿(全样本 p100)
//! g29_slab_device --red-arm kernel-bias --spv <k.spv> [--tol <F>]
//! g29_slab_device --probe --spv <k.spv> --tol <F> [--out <path>]  # soak 快车道
//! g29_slab_device --side-table --spv <k.spv> --tol <F> [--out <path>]  # M-b 臂
//! g29_slab_device --host-only [--out <path>]
//! ```

#![forbid(unsafe_code)]

use rurix_render::material::slab::SlabStack;
use rurix_rt::vk;

const TAG: &str = "[g29_slab_device]";
/// 网格血缘(RFC-0046 §1.1 F4):g22_slab_probe GRID=128 经 (grid+1)² 格点。
const GRID: usize = 128;
const N_SAMPLES: usize = (GRID + 1) * (GRID + 1);
/// probe soak 快车道:2048 样本前缀子集(host 单源同律)。
const PROBE_SAMPLES: usize = 2048;
/// RED 臂注入幅(RFC-0046 §1.4 ⑤;g13/g26/g28 同值;标定容差绝对上界 =
/// RED_BIAS × 0.5 由 CI 断言)。
const RED_BIAS: f32 = 0.05;
/// 侧表槽数(RFC-0046 §2.1 字面)。
const N_SLOTS: usize = 16;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 夹具(参数网格/侧表 host 单源生成一次原字节上传;device 不重算)
// ---------------------------------------------------------------------------

/// M-a 参数网格(16641 样本逐样本 [rc, ab];rc=i/128 外层、ab=j/128 内层,
/// 行主序 idx = i·129+j;白炉行 = ab=1 列 j=128;角点 = idx 16640)。
fn grid_samples() -> Vec<f32> {
    let mut v = Vec::with_capacity(N_SAMPLES * 2);
    for i in 0..=GRID {
        let rc = i as f32 / GRID as f32;
        for j in 0..=GRID {
            let ab = j as f32 / GRID as f32;
            v.push(rc);
            v.push(ab);
        }
    }
    v
}

/// M-b 侧表(RFC-0046 §2.1 字面:rc_k = k/15·0.95、ab_k = (15−k)/15;0.95
/// 上限有意规避 denom→0 角点区——角点语义覆盖由 M-a 主网格独担;host 单源
/// 生成一次原字节上传,device 不重算槽参数〔k/15 非 2 幂分母求值序位级敏感〕)。
fn side_table_samples() -> Vec<f32> {
    let mut v = Vec::with_capacity(N_SLOTS * 2);
    for k in 0..N_SLOTS {
        v.push(k as f32 / 15.0 * 0.95);
        v.push((15 - k) as f32 / 15.0);
    }
    v
}

/// host 参考 = SlabStack::new(rc, ab).total_reflectance()(f64)逐样本直调
/// (冻结金标准只消费不改写)。
fn host_reference(samples: &[f32]) -> Vec<f64> {
    samples
        .chunks_exact(2)
        .map(|p| SlabStack::new(p[0], p[1]).total_reflectance())
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
// device 臂(bin-local;单 dispatch [n,1,1] 经 vk::run_compute)
// ---------------------------------------------------------------------------

/// kernel 参数面打包(与 g29_slab.rx 参数面逐字同源;8 f32 位级编码:
/// [0]=n_samples [1]=red_bias [2..=7]=reserved 恒 0,F10c 区间写法)。
fn pack_params(n: usize, red_bias: f32) -> Vec<f32> {
    let mut v = vec![n as f32, red_bias];
    v.resize(8, 0.0);
    v
}

struct DeviceSlab {
    spv: Vec<u32>,
    entry: String,
    red_bias: f32,
}

impl DeviceSlab {
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

    /// 单 dispatch 全样本;返回输出缓冲(1 f32/样本 R + red_bias)。
    fn run(&self, samples: &[f32], n: usize) -> Vec<f32> {
        let params = pack_params(n, self.red_bias);
        let mut bufs = vec![
            bytes_f32(samples),
            bytes_f32(&params),
            vec![0u8; n * 4],
        ];
        vk::run_compute(
            &self.spv,
            &self.entry,
            &mut bufs,
            &[],
            [u32::try_from(n).expect("n 超 u32"), 1, 1],
        )
        .unwrap_or_else(|e| panic!("slab dispatch 失败: {e}"));
        read_f32(&bufs[2])
    }
}

// ---------------------------------------------------------------------------
// 判据(⓪有限性一等断言 / ①p100 / ②白炉行 / ③能量上界 / ④双跑位级)
// ---------------------------------------------------------------------------

/// 判据⓪:输出有限性一等断言(RFC-0046 §1.4 F3)——首个非有限样本下标;
/// **必须先于一切 max 聚合调用**(Rust `f64::max(NaN, x) = x` 吞 NaN 陷阱)。
fn first_nonfinite(out: &[f32]) -> Option<usize> {
    out.iter().position(|x| !x.is_finite())
}

/// 判据①:逐样本 |device − host| 绝对差 p100(device f32 提升 f64;
/// 调用前提 = 判据⓪已过,无 NaN 进聚合)。
fn parity_p100(out: &[f32], host: &[f64]) -> f64 {
    let mut p = 0.0f64;
    for (o, h) in out.iter().zip(host) {
        let d = (f64::from(*o) - h).abs();
        if d > p {
            p = d;
        }
    }
    p
}

/// 判据②:白炉行(ab=1 列 j=128,129 样本)device dev 最大值(vs 解析 1.0;
/// 如实登记面——dev 非零真实来源 = OpFDiv ≤2.5 ULP + FMA 收缩,RFC-0046 §1.2)。
fn furnace_row_device_dev_max(out: &[f32]) -> f64 {
    let mut m = 0.0f64;
    for i in 0..=GRID {
        let d = (f64::from(out[i * (GRID + 1) + GRID]) - 1.0).abs();
        if d > m {
            m = d;
        }
    }
    m
}

/// host 白炉行位级恒等断言(RFC-0046 §1.2:网格值 rc=i/128 下 tc/tc²/denom=tc
/// 逐步精确、数学商 tc²/tc = tc 可表示,f64 正确舍入 ⇒ R ≡ 1.0 位级)。
fn host_furnace_bitexact_one(host: &[f64]) -> bool {
    (0..=GRID).all(|i| host[i * (GRID + 1) + GRID].to_bits() == 1.0f64.to_bits())
}

/// 判据③:能量上界 device 复核(全样本 max;调用前提 = 判据⓪已过)。
fn device_max_r(out: &[f32]) -> f64 {
    let mut m = f64::NEG_INFINITY;
    for &x in out {
        let v = f64::from(x);
        if v > m {
            m = v;
        }
    }
    m
}

// ---------------------------------------------------------------------------
// JSON 出报(手写,零新依赖;g28_restir_device 同模)
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
// 参数
// ---------------------------------------------------------------------------

struct Args {
    spv: Option<String>,
    tol: f64,
    calibrate: bool,
    red_arm: Option<String>,
    probe: bool,
    side_table: bool,
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
        side_table: false,
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
            "--side-table" => a.side_table = true,
            "--host-only" => a.host_only = true,
            "--out" => a.out = it.next(),
            other => fail(&format!("未知参数: {other}")),
        }
    }
    a
}

fn device_arm(args: &Args) -> Result<DeviceSlab, String> {
    let spv = load_spv(args.spv.as_deref().unwrap_or_else(|| fail("缺 --spv")));
    DeviceSlab::create(spv)
}

// ---------------------------------------------------------------------------
// 标定腿(全 16641 样本绝对差 p100;两跑位级一致由 CI 裁决;
// 实测 p100=0 → 零容差零条目 measured 事实,RFC-0046 §1.3)
// ---------------------------------------------------------------------------

fn calibrate_leg(args: &Args) -> ! {
    let dev = match device_arm(args) {
        Ok(d) => d,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g29slab.calibration_skip.v1\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(&e)
            );
            std::process::exit(0);
        }
    };
    let samples = grid_samples();
    let host = host_reference(&samples);
    let out = dev.run(&samples, N_SAMPLES);
    // 判据⓪同律先行:标定腿 p100 亦为 max 聚合,NaN 吞没假绿路径同样封死。
    if let Some(idx) = first_nonfinite(&out) {
        fail(&format!(
            "标定腿判据⓪失败:样本 {idx} 输出非有限(有限性一等断言先于聚合,RFC-0046 §1.4)"
        ));
    }
    let mut diffs = Vec::with_capacity(N_SAMPLES);
    let mut p100 = 0.0f64;
    for (o, h) in out.iter().zip(&host) {
        let d = (f64::from(*o) - h).abs();
        diffs.push(d);
        if d > p100 {
            p100 = d;
        }
    }
    let protocol = format!(
        "slab device vs host 金标准(material/slab.rs::total_reflectance f64 直调)同输入\
         逐样本反照率对拍容差({N_SAMPLES} 样本 = 129×129 参数网格 rc=i/128、ab=j/128\
         〔g22_slab_probe GRID=128 (grid+1)² 格点口径〕,host 单源生成原字节上传,kernel \
         全 f32 修法 A 分母安全化,判据⓪有限性一等断言先于聚合;逐样本绝对差 p100;\
         threshold = measured × 2.0 冻结 k,方向 max,禁手写;实测 p100=0 → 零容差零条目;\
         RFC-0046 §1.3)"
    );
    println!(
        "{{\"schema\":\"rurix.g29slab.calibration_entry.v1\",\"entry_id\":\"g29.slab_device.host_device_reflectance_tol\",\"results\":{{\"trimmed_mean\":{:.15e}}},\"protocol\":{},\"sample_manifest\":{{\"count\":{},\"digest\":{}}},\"provenance\":{{\"gpu\":\"device\",\"backend\":\"slab_device\",\"base_commit\":{}}},\"timestamp\":{}}}",
        p100,
        jstr(&protocol),
        N_SAMPLES,
        jstr(&format!("sha256:{}", sha256_f64(&diffs))),
        jstr(&base_commit()),
        jstr(&utc_now()),
    );
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// RED 臂(kernel-bias:params[1] 注入 RED_BIAS=0.05 → 对拍必超容差;
// 臂间判据归属 F11:仅评判据①,判据⓪②③④不跨臂执行)
// ---------------------------------------------------------------------------

fn red_arm_kernel_bias(args: &Args) -> Result<String, String> {
    let samples = grid_samples();
    let host = host_reference(&samples);
    let honest = device_arm(args)?;
    let honest_out = honest.run(&samples, N_SAMPLES);
    let honest_p100 = parity_p100(&honest_out, &host);
    let mut tampered = device_arm(args)?;
    tampered.red_bias = RED_BIAS;
    let tampered_out = tampered.run(&samples, N_SAMPLES);
    let tampered_p100 = parity_p100(&tampered_out, &host);
    let detected = if args.tol > 0.0 {
        tampered_p100 > args.tol
    } else {
        // 零容差态:偏置注入 ⇒ p100 ≈ 0.05 ≠ 0 必红(平凡成立仍恒跑)。
        tampered_p100 > honest_p100 + f64::from(RED_BIAS) * 0.5
    };
    if !detected {
        return Err(format!(
            "kernel-bias 漏检:tampered p100={tampered_p100:.6e} vs tol={:.6e} honest={honest_p100:.6e}",
            args.tol
        ));
    }
    Ok(format!(
        "honest p100={honest_p100:.6e} tampered p100={tampered_p100:.6e} tol={:.6e}(注入后白炉行 R ≈ 1.05 属预期注入效果,判据③不跨臂,F11)",
        args.tol
    ))
}

// ---------------------------------------------------------------------------
// probe(soak 快车道:2048 样本前缀子集对拍 + 双跑位级;单行 JSON)
// ---------------------------------------------------------------------------

fn probe_leg(args: &Args) -> ! {
    let dev = match device_arm(args) {
        Ok(d) => d,
        Err(e) => {
            let line = format!(
                "{{\"schema\":\"rurix.g29slab.probe.v1\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(&e)
            );
            emit_line(&line, &args.out);
            std::process::exit(0);
        }
    };
    let full = grid_samples();
    let samples = full[..PROBE_SAMPLES * 2].to_vec();
    let host = host_reference(&samples);
    let out_a = dev.run(&samples, PROBE_SAMPLES);
    // 判据⓪先行(soak 快车道同律)。
    let finite_all = first_nonfinite(&out_a).is_none();
    if !finite_all {
        let line = format!(
            "{{\"schema\":\"rurix.g29slab.probe.v1\",\"state\":\"fail\",\"samples\":{PROBE_SAMPLES},\"finite_all\":false,\"first_nonfinite_index\":{}}}",
            first_nonfinite(&out_a).unwrap_or(0)
        );
        emit_line(&line, &args.out);
        std::process::exit(1);
    }
    let out_b = dev.run(&samples, PROBE_SAMPLES);
    let p100 = parity_p100(&out_a, &host);
    let digest_a = sha256_f32(&out_a);
    let digest_b = sha256_f32(&out_b);
    let bitexact = digest_a == digest_b;
    let in_tol = p100 <= args.tol;
    let state = if in_tol && bitexact { "pass" } else { "fail" };
    let line = format!(
        "{{\"schema\":\"rurix.g29slab.probe.v1\",\"state\":{},\"samples\":{PROBE_SAMPLES},\"finite_all\":true,\"p100_vs_host\":{:.15e},\"tol\":{:.15e},\"in_tol\":{},\"bitexact\":{},\"digest\":{},\"base_commit\":{}}}",
        jstr(state),
        p100,
        args.tol,
        in_tol,
        bitexact,
        jstr(&digest_a),
        jstr(&base_commit()),
    );
    emit_line(&line, &args.out);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// M-b 侧表供参加性臂(--side-table;RFC-0046 §2;device 腿三态)
// ---------------------------------------------------------------------------

fn side_table_leg(args: &Args) -> ! {
    let dev = match device_arm(args) {
        Ok(d) => d,
        Err(e) => {
            let line = format!(
                "{{\"schema\":\"rurix.g29slab.side_table.v1\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(&e)
            );
            emit_line(&line, &args.out);
            std::process::exit(0);
        }
    };
    // ── 侧表 + 逐槽白炉互核变体(每槽 ab=1;host 单源生成,device 不重算)──
    let table = side_table_samples();
    let host = host_reference(&table);
    let furnace: Vec<f32> = (0..N_SLOTS)
        .flat_map(|k| [table[k * 2], 1.0f32])
        .collect();
    let furnace_host = host_reference(&furnace);

    let out_a = dev.run(&table, N_SLOTS);
    let fur_a = dev.run(&furnace, N_SLOTS);
    // ── 判据⓪:全槽 is_finite 硬 FAIL 先行(主表 + 白炉变体;F3 同律)──
    let nonfinite = first_nonfinite(&out_a).or_else(|| first_nonfinite(&fur_a));
    if let Some(idx) = nonfinite {
        let line = format!(
            "{{\"schema\":\"rurix.g29slab.side_table.v1\",\"state\":\"fail\",\"n_slots\":{N_SLOTS},\"finite_all\":false,\"first_nonfinite_index\":{idx},\"finiteness_checked_before_aggregation\":true}}"
        );
        emit_line(&line, &args.out);
        std::process::exit(1);
    }
    // ── 判据①:逐槽对拍 p100 ≤ 容差(M-a 同源容差协议)──
    let p100 = parity_p100(&out_a, &host);
    let in_tol = p100 <= args.tol;
    // ── 判据②:逐槽白炉互核 dev 登记(host/device 双端;如实登记面——侧表
    //    槽值 rc_k 非 2 幂网格值,host 白炉 dev 不再有位级 0 论证,照实登记)──
    let mut fur_host_dev_max = 0.0f64;
    let mut fur_dev_dev_max = 0.0f64;
    for k in 0..N_SLOTS {
        let hd = (furnace_host[k] - 1.0).abs();
        let dd = (f64::from(fur_a[k]) - 1.0).abs();
        if hd > fur_host_dev_max {
            fur_host_dev_max = hd;
        }
        if dd > fur_dev_dev_max {
            fur_dev_dev_max = dd;
        }
    }
    // ── 判据③:双跑位级(主表 + 白炉变体两组 dispatch 各两跑 digest)──
    let out_b = dev.run(&table, N_SLOTS);
    let fur_b = dev.run(&furnace, N_SLOTS);
    let digest_a = format!("{}:{}", sha256_f32(&out_a), sha256_f32(&fur_a));
    let digest_b = format!("{}:{}", sha256_f32(&out_b), sha256_f32(&fur_b));
    let bitexact = digest_a == digest_b;

    let mut rows: Vec<String> = Vec::with_capacity(N_SLOTS);
    for k in 0..N_SLOTS {
        rows.push(format!(
            "{{\"k\":{k},\"rc\":{:.9e},\"ab\":{:.9e},\"host_r\":{:.15e},\"device_r\":{:.9e},\"absdiff\":{:.9e},\"furnace_host_r\":{:.15e},\"furnace_device_r\":{:.9e},\"furnace_host_dev\":{:.9e},\"furnace_device_dev\":{:.9e}}}",
            f64::from(table[k * 2]),
            f64::from(table[k * 2 + 1]),
            host[k],
            f64::from(out_a[k]),
            (f64::from(out_a[k]) - host[k]).abs(),
            furnace_host[k],
            f64::from(fur_a[k]),
            (furnace_host[k] - 1.0).abs(),
            (f64::from(fur_a[k]) - 1.0).abs(),
        ));
    }

    let state = if in_tol && bitexact { "pass" } else { "fail" };
    eprintln!(
        "{TAG}: side-table n_slots={N_SLOTS} p100={p100:.6e} tol={:.6e} in_tol={in_tol} furnace_dev(host/device)={fur_host_dev_max:.3e}/{fur_dev_dev_max:.3e} bitexact={bitexact}",
        args.tol
    );
    let line = format!(
        "{{\"schema\":\"rurix.g29slab.side_table.v1\",\"state\":{},\"n_slots\":{N_SLOTS},\"slot_params\":\"rc_k=k/15*0.95, ab_k=(15-k)/15(0.95 有意规避 denom→0 角点区,角点覆盖归 M-a 主网格,RFC-0046 §2.1)\",\"finite_all\":true,\"finiteness_checked_before_aggregation\":true,\"parity_p100\":{:.15e},\"tol\":{:.15e},\"in_tol\":{},\"furnace_host_dev_max\":{:.9e},\"furnace_device_dev_max\":{:.9e},\"double_run_bitexact\":{},\"digest\":{},\"per_slot_rows\":[{}],\"conflation_note\":\"bin-local 独立 SSBO(bin 内合成不落资产);与 material/ 生产资产侧表设施零关系零触碰(RFC-0046 §2.1 防混淆声明);MaterialClosure 32B 与 reserved 拓扑位零触碰\",\"base_commit\":{}}}",
        jstr(state),
        p100,
        args.tol,
        in_tol,
        fur_host_dev_max,
        fur_dev_dev_max,
        bitexact,
        jstr(&digest_a),
        rows.join(","),
        jstr(&base_commit()),
    );
    emit_line(&line, &args.out);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// host 腿(纯 host 恒跑:白炉行位级恒等 + 能量上界 + 双跑位级)
// ---------------------------------------------------------------------------

fn host_only_leg(args: &Args) -> ! {
    let samples = grid_samples();
    let h1 = host_reference(&samples);
    let h2 = host_reference(&samples);
    let bitexact = sha256_f64(&h1) == sha256_f64(&h2);
    let furnace_bitexact = host_furnace_bitexact_one(&h1);
    let mut max_r = 0.0f64;
    for &x in &h1 {
        if x > max_r {
            max_r = x;
        }
    }
    let bound_ok = max_r <= 1.0 + 1e-9;
    let corner_one = h1[N_SAMPLES - 1].to_bits() == 1.0f64.to_bits();
    let state = if bitexact && furnace_bitexact && bound_ok && corner_one {
        "pass"
    } else {
        "fail"
    };
    let line = format!(
        "{{\"schema\":\"rurix.g29slab.harness.v1\",\"mode\":\"host-only\",\"state\":{},\"n_samples\":{N_SAMPLES},\"host_furnace_row_bitexact_one\":{},\"host_max_r\":{:.15e},\"energy_bounded_1e9\":{},\"corner_host_bitexact_one\":{},\"double_run_bitexact\":{},\"digest\":{}}}",
        jstr(state),
        furnace_bitexact,
        max_r,
        bound_ok,
        corner_one,
        bitexact,
        jstr(&sha256_f64(&h1)),
    );
    emit_line(&line, &args.out);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// main(默认 = 全档验证:⓪有限性一等断言 → ①p100 → ②白炉行 → ③能量上界
// → ④双跑位级;RFC-0046 §1.4 序)
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    if args.calibrate {
        calibrate_leg(&args);
    }
    if args.probe {
        probe_leg(&args);
    }
    if args.side_table {
        side_table_leg(&args);
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
                    "{{\"schema\":\"rurix.g29slab.red_arm.v1\",\"arm\":{},\"detected\":true,\"detail\":{}}}",
                    jstr(arm),
                    jstr(&detail)
                );
                std::process::exit(0);
            }
            Err(e) if e.contains("不可用") => {
                println!(
                    "{{\"schema\":\"rurix.g29slab.red_arm.v1\",\"arm\":{},\"detected\":false,\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                    jstr(arm),
                    jstr(&e)
                );
                std::process::exit(0);
            }
            Err(e) => fail(&format!("red-arm {arm} 失效(漏检): {e}")),
        }
    }

    // ── 全档验证 ──
    let samples = grid_samples();
    let host = host_reference(&samples);
    // host 腿恒跑前置:白炉行 host f64 位级 ≡ 1.0(RFC-0046 §1.2 可断言面;
    // 破断即金标准语义漂移,不进对拍)。
    if !host_furnace_bitexact_one(&host) {
        fail("host 白炉行非位级 1.0(金标准语义漂移;RFC-0046 §1.2)");
    }

    let dev = match device_arm(&args) {
        Ok(d) => d,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g29slab.harness.v1\",\"mode\":\"device\",\"state\":\"skipped_dev_env\",\"skip_reason\":{}}}",
                jstr(&e)
            );
            return;
        }
    };

    let out_a = dev.run(&samples, N_SAMPLES);
    // ── 判据⓪:输出有限性一等断言(先于对拍聚合执行硬 FAIL;F3)──
    if let Some(idx) = first_nonfinite(&out_a) {
        let line = format!(
            "{{\"schema\":\"rurix.g29slab.harness.v1\",\"mode\":\"device\",\"state\":\"fail\",\"problems\":[\"判据⓪失败:样本 {idx} 输出非有限(有限性一等断言先于聚合硬 FAIL,RFC-0046 §1.4)\"],\"n_samples\":{N_SAMPLES},\"finite_all\":false,\"first_nonfinite_index\":{idx},\"finiteness_checked_before_aggregation\":true}}"
        );
        println!("{line}");
        std::process::exit(1);
    }

    let mut problems: Vec<String> = Vec::new();
    // ── 判据①:逐样本 p100 ≤ 标定容差 ──
    let p100 = parity_p100(&out_a, &host);
    let in_tol = p100 <= args.tol;
    if !in_tol {
        problems.push(format!("p100={p100:.6e} 超容差 {:.6e}", args.tol));
    }
    // ── 判据②:白炉行 device dev 最大值如实登记(登记面不另设线——覆盖论证
    //    F1:白炉行 ⊂ 网格 ⇒ 已被判据①容差线传递覆盖)──
    let furnace_dev = furnace_row_device_dev_max(&out_a);
    // ── 判据③:能量上界 device 复核 ──
    let max_r = device_max_r(&out_a);
    let energy_bound = 1.0 + args.tol;
    let energy_ok = max_r <= energy_bound;
    if !energy_ok {
        problems.push(format!(
            "能量上界失败:device max R={max_r:.9} > 1+tol={energy_bound:.9}"
        ));
    }
    // ── 判据④:device 双跑位级一致 ──
    let out_b = dev.run(&samples, N_SAMPLES);
    let digest_a = sha256_f32(&out_a);
    let digest_b = sha256_f32(&out_b);
    let bitexact = digest_a == digest_b;
    if !bitexact {
        problems.push("device 双跑非位级一致".into());
    }
    // ── 角点登记(rc=ab=1,idx 16640:修法 A 下 device R 应位级 1.0)──
    let corner_r = out_a[N_SAMPLES - 1];
    let corner_bitexact = corner_r.to_bits() == 1.0f32.to_bits();

    let state = if problems.is_empty() { "pass" } else { "fail" };
    eprintln!(
        "{TAG}: 全档验证 n_samples={N_SAMPLES} finite_all=true p100={p100:.6e} tol={:.6e} furnace_dev={furnace_dev:.3e} max_r={max_r:.9} corner_r={corner_r:.9}(bitexact_one={corner_bitexact}) bitexact={bitexact}",
        args.tol
    );
    println!(
        "{{\"schema\":\"rurix.g29slab.harness.v1\",\"mode\":\"device\",\"state\":{},\"problems\":{},\"n_samples\":{N_SAMPLES},\"grid\":{GRID},\"finite_all\":true,\"finiteness_checked_before_aggregation\":true,\"p100_vs_host\":{:.15e},\"tol\":{:.15e},\"in_tol\":{},\"furnace_row\":{{\"samples\":{},\"host_bitexact_one\":true,\"device_dev_max\":{:.9e},\"note\":\"dev 非零来源 = Vulkan FP32 OpFDiv ≤2.5 ULP + FMA 收缩可能性(RFC-0046 §1.2);登记面被判据①容差线传递覆盖(F1)\"}},\"energy_bound\":{{\"device_max_r\":{:.15e},\"bound\":{:.15e},\"pass\":{}}},\"corner\":{{\"rc\":1.0,\"ab\":1.0,\"device_r\":{:.9e},\"host_r\":1.0,\"device_bitexact_one\":{}}},\"bitexact\":{},\"digest\":{},\"base_commit\":{}}}",
        jstr(state),
        strs_json(&problems),
        p100,
        args.tol,
        in_tol,
        GRID + 1,
        furnace_dev,
        max_r,
        energy_bound,
        energy_ok,
        f64::from(corner_r),
        corner_bitexact,
        bitexact,
        jstr(&digest_a),
        jstr(&base_commit()),
    );
    if !problems.is_empty() {
        std::process::exit(1);
    }
}
