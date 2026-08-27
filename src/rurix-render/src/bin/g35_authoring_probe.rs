//! G35-8 作者面资产 host probe(门 `g35.wave8.authoring`;RFC-0049 §3/§4.11;
//! 契约事实源 = milestones/g35/G35_CONTRACT.md D-G35-8)。
//!
//! ## 集成路径(纯 host 确定性面,无 device 依赖故无 required-features)
//!
//! `--asset <json>` 经 [`particles::emitter_asset`](rurix_render::particles::emitter_asset)
//! 解析(十字段闭集 fail-closed)→ [`EmitterRuntime`] 驱动
//! `particles::core::frame` host 金标准全链 64 帧(读 A 写 B 帧末交换,
//! G35-P v1 帧序冻结)→ `--reload-at 32 --asset2 <json>` 热重载(纯参数面
//! 替换:desc/curve 换新,池/pid/帧钟连续)。
//!
//! ## 判据面(全 host 机器事实;evidence JSON 旗标承载)
//!
//! ① **重载生效**:重载轨迹 digest ≠ 无重载基线 digest(digest = 逐帧
//!   n ‖ pid ‖ 8 f32 流 bits ‖ args 链式 sha256);且重载帧 emit ==
//!   asset2 曲线该帧求值(下一帧生效语义:reload 于帧 r−1 末调用,帧 r
//!   起消费新参数面)。
//! ② **旧粒子连续(不瞬移)**:全程逐帧(含重载边界帧)幸存粒子 = 上帧态
//!   在**当前活跃 gravity** 下冻结运算序单步重放 bitwise 全等。
//! ③ **pid 序列连续**:每帧无重复 + 幸存段 ⊆ 上帧集 + 新发射段 ==
//!   [pid_base, pid_base+emit) 精确区间;跨重载 pid_base 单调不重置。
//! ④ **曲线求值互核**:emit_count_at 与 probe 独立参考实现(单循环阶梯查表
//!   第二实现)逐帧全等 + 双求值确定(scan.rs 双实现互核先例)。
//! ⑤ **双跑位级**:同 seed 同资产全链双跑 digest 位级一致。
//! ⑥ **发射钳制登记**:accepted = min(requested, cap − n_curr)(RFC §4.4
//!   F7),rejected 计数如实登记(judgment 面非硬门)。
//!
//! ## 非法资产 typed 退出码
//!
//! 资产解析违例 → stderr `AUTHORING_ASSET_ERR kind=<七类闭集> detail=<…>` +
//! **退出码 3**(kind 闭集 = emitter_asset::AssetError::kind_name:Json/
//! NotObject/MissingField/UnknownField/Type/EnumOutOfSet/Domain);用法/IO
//! 错 = 退出码 2;判据红 = 退出码 1;全绿 = 0。
//!
//! ## RED 臂(`--red-arm field-tamper`)
//!
//! 资产字段篡改必检出双面:(a) 值篡改——同 seed 双跑,第二跑 gravity_y/
//! vel_base 内存面篡改 → 轨迹 digest 必异(digest 判据对资产参数敏感,防
//! 镂空 digest 冒充);(b) schema 篡改——资产文本注入闭集外字段 → 解析必
//! typed Err(fail-closed 面)。
//!
//! ## 内嵌契约样例(`--write-samples <dir>` 写 .tmp,不进 milestones)
//!
//! 样例 A(const/billboard/alpha)/ 样例 B(step/mesh/additive)——两资产
//! 参数面可分辨(gravity/vel/curve 全异),重载判据有效性前提。
//!
//! ## 用法
//!
//! ```text
//! g35_authoring_probe --write-samples <dir>
//! g35_authoring_probe --asset <a.json> [--asset2 <b.json> --reload-at 32]
//!     [--frames 64] [--cap 2048] [--seed 42] [--evidence-out <p>]
//! g35_authoring_probe --red-arm field-tamper --asset <a.json> [...]
//! ```

#![forbid(unsafe_code)]

use std::collections::HashMap;

use rurix_render::particles::SEG;
use rurix_render::particles::core::{ParticlePools, frame};
use rurix_render::particles::emitter_asset::{EmitCurve, EmitterAsset, EmitterRuntime};
use rurix_render::particles::rand_table;

const TAG: &str = "[g35_authoring_probe]";
const DEFAULT_FRAMES: usize = 64;
const DEFAULT_CAP: usize = 2048;
const DEFAULT_SEED: u64 = 42;
const DEFAULT_RELOAD_AT: u32 = 32;
/// dt = 1/60(冻结确定性脚本;g35_particle_core_device 同律)。
const DT: f32 = 1.0 / 60.0;

/// 内嵌契约样例 A(const/billboard/alpha;--write-samples 落 .tmp)。
const SAMPLE_ASSET_A: &str = r#"{
  "name": "campfire_sparks_a",
  "pos": [0.0, 1.0, -0.5],
  "spread": [0.4, 0.2, 0.4],
  "vel_base": [0.0, 3.0, 0.0],
  "vel_spread": [1.0, 0.5, 1.0],
  "life_base": 1.2,
  "gravity_y": -9.8,
  "emit_curve": {"kind": "const", "value": 24.7},
  "render": "billboard",
  "blend": "alpha"
}
"#;

/// 内嵌契约样例 B(step/mesh/additive;与 A 全参数面可分辨)。
const SAMPLE_ASSET_B: &str = r#"{
  "name": "ember_burst_b",
  "pos": [0.5, 1.5, 0.0],
  "spread": [0.2, 0.1, 0.2],
  "vel_base": [0.0, 4.5, 0.5],
  "vel_spread": [0.8, 0.3, 0.8],
  "life_base": 0.9,
  "gravity_y": -3.0,
  "emit_curve": {"kind": "step", "frames": [0, 8, 40], "values": [16.0, 48.9, 8.0]},
  "render": "mesh",
  "blend": "additive"
}
"#;

fn usage_fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(2)
}

// ---------------------------------------------------------------------------
// JSON 出报(手写零新依赖;g35_particle_core_device 同模)
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

fn u32s_json(v: &[u32]) -> String {
    let inner: Vec<String> = v.iter().map(|x| x.to_string()).collect();
    format!("[{}]", inner.join(","))
}

fn strs_json(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| jstr(s)).collect();
    format!("[{}]", inner.join(","))
}

/// 出报(stdout 恒打;--evidence-out 同步落盘)。
fn emit_evidence(line: &str, out: &Option<String>) {
    println!("{line}");
    if let Some(path) = out {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, format!("{line}\n"))
            .unwrap_or_else(|e| usage_fail(&format!("写 --evidence-out {path}: {e}")));
    }
}

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

struct Args {
    asset: Option<String>,
    asset2: Option<String>,
    reload_at: u32,
    frames: usize,
    cap: usize,
    seed: u64,
    evidence_out: Option<String>,
    red_arm: Option<String>,
    write_samples: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        asset: None,
        asset2: None,
        reload_at: DEFAULT_RELOAD_AT,
        frames: DEFAULT_FRAMES,
        cap: DEFAULT_CAP,
        seed: DEFAULT_SEED,
        evidence_out: None,
        red_arm: None,
        write_samples: None,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0usize;
    let val = |i: &mut usize, name: &str| -> String {
        *i += 1;
        argv.get(*i)
            .unwrap_or_else(|| usage_fail(&format!("{name} 缺值")))
            .clone()
    };
    while i < argv.len() {
        match argv[i].as_str() {
            "--asset" => a.asset = Some(val(&mut i, "--asset")),
            "--asset2" => a.asset2 = Some(val(&mut i, "--asset2")),
            "--reload-at" => {
                a.reload_at = val(&mut i, "--reload-at")
                    .parse()
                    .unwrap_or_else(|_| usage_fail("--reload-at 非 u32"))
            }
            "--frames" => {
                a.frames = val(&mut i, "--frames")
                    .parse()
                    .unwrap_or_else(|_| usage_fail("--frames 非 usize"))
            }
            "--cap" => {
                a.cap = val(&mut i, "--cap")
                    .parse()
                    .unwrap_or_else(|_| usage_fail("--cap 非 usize"))
            }
            "--seed" => {
                a.seed = val(&mut i, "--seed")
                    .parse()
                    .unwrap_or_else(|_| usage_fail("--seed 非 u64"))
            }
            "--evidence-out" => a.evidence_out = Some(val(&mut i, "--evidence-out")),
            "--red-arm" => a.red_arm = Some(val(&mut i, "--red-arm")),
            "--write-samples" => a.write_samples = Some(val(&mut i, "--write-samples")),
            other => usage_fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    if a.cap == 0 || a.cap % SEG != 0 {
        usage_fail(&format!("--cap {} 须为 SEG={SEG} 正整倍数", a.cap));
    }
    if a.frames == 0 {
        usage_fail("--frames 须 ≥ 1");
    }
    a
}

/// 资产装载:IO 错 = 退 2;解析违例 = typed 退 3(kind 闭集 token)。
fn load_asset(path: &str) -> EmitterAsset {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| usage_fail(&format!("读资产 {path}: {e}")));
    match EmitterAsset::parse(&text) {
        Ok(a) => a,
        Err(e) => {
            eprintln!(
                "{TAG}: AUTHORING_ASSET_ERR kind={} detail={}",
                e.kind_name(),
                e
            );
            std::process::exit(3)
        }
    }
}

// ---------------------------------------------------------------------------
// 场景跑器(host 全链;判据旗标 + 链式 digest)
// ---------------------------------------------------------------------------

/// 单粒子冻结运算序重放(core::sim_step 同序;0..2 pos / 3..5 vel / 6 age)。
fn advance(s: &mut [f32; 8], dt: f32, g: f32) {
    s[4] += g * dt;
    s[0] += s[3] * dt;
    s[1] += s[4] * dt;
    s[2] += s[5] * dt;
    s[6] += dt;
}

/// 场景输出(判据旗标 = 机器事实;problems 携带首因诊断)。
struct RunOut {
    digest: String,
    pid_unique: bool,
    pid_survivor_subset: bool,
    pid_emit_range_exact: bool,
    old_particles_continuous: bool,
    continuity_checked: usize,
    boundary_survivors_checked: usize,
    requested_at_reload: u32,
    accepted_at_reload: u32,
    emit_head: Vec<u32>,
    rejected_total: u64,
    pids_issued: u32,
    alive_final: u32,
    n_final: usize,
    problems: Vec<String>,
}

/// 全链 host 场景:asset_a 起跑,`reload` = Some((帧, 资产)) 时该帧起消费
/// 新参数面(reload 语义 = 纯参数替换,池/pid/帧钟连续)。
fn run_scenario(
    asset_a: &EmitterAsset,
    reload: Option<(u32, &EmitterAsset)>,
    frames: usize,
    cap: usize,
    seed: u64,
) -> RunOut {
    let table = rand_table(seed);
    let mut rt = EmitterRuntime::new(asset_a.clone());
    let mut a = ParticlePools::with_capacity(cap);
    let mut b = ParticlePools::with_capacity(cap);
    let mut pid_base = 0u32;
    let mut out = RunOut {
        digest: format!("sha256:{}", "0".repeat(64)),
        pid_unique: true,
        pid_survivor_subset: true,
        pid_emit_range_exact: true,
        old_particles_continuous: true,
        continuity_checked: 0,
        boundary_survivors_checked: 0,
        requested_at_reload: 0,
        accepted_at_reload: 0,
        emit_head: Vec::new(),
        rejected_total: 0,
        pids_issued: 0,
        alive_final: 0,
        n_final: 0,
        problems: Vec::new(),
    };
    let mut digest_hex = "0".repeat(64);
    let mut prev: HashMap<u32, [f32; 8]> = HashMap::new();
    let problem = |v: &mut Vec<String>, m: String| {
        if v.len() < 8 {
            v.push(m);
        }
    };
    for f in 0..frames as u32 {
        if let Some((at, asset_b)) = reload {
            if f == at {
                // 热重载:帧 at−1 末调用 → 帧 at 起生效(下一帧生效语义)。
                rt.reload((*asset_b).clone());
            }
        }
        let g = rt.asset().gravity_y;
        let requested = rt.next_emit_count();
        // 发射钳制(RFC-0049 §4.4 F7):accepted = min(requested, cap − n_curr);
        // rejected 确定性钳制计数如实登记(零随机丢弃)。
        let accepted = (requested as usize).min(cap - a.n);
        out.rejected_total += (requested as usize - accepted) as u64;
        if let Some((at, _)) = reload {
            if f == at {
                out.requested_at_reload = requested;
                out.accepted_at_reload = accepted as u32;
            }
        }
        if out.emit_head.len() < 8 {
            out.emit_head.push(accepted as u32);
        }
        let st = frame(&mut a, &mut b, &rt.asset().to_desc(), &table, DT, pid_base, accepted);
        // ── pid 连续 + 旧粒子连续(逐帧机器事实;含重载边界帧)──
        let mut cur: HashMap<u32, [f32; 8]> = HashMap::new();
        for i in 0..b.n {
            let stt = [
                b.pos_x[i], b.pos_y[i], b.pos_z[i], b.vel_x[i], b.vel_y[i], b.vel_z[i], b.age[i],
                b.life[i],
            ];
            if cur.insert(b.pid[i], stt).is_some() {
                out.pid_unique = false;
                problem(&mut out.problems, format!("帧 {f}: pid {} 重复", b.pid[i]));
            }
        }
        let at_boundary = matches!(reload, Some((at, _)) if f == at);
        for (pid, now) in &cur {
            if let Some(p) = prev.get(pid) {
                let mut want = *p;
                advance(&mut want, DT, g);
                for k in 0..7 {
                    if want[k].to_bits() != now[k].to_bits() {
                        out.old_particles_continuous = false;
                        problem(
                            &mut out.problems,
                            format!("帧 {f}: pid {pid} 分量 {k} 不连续(瞬移检出)"),
                        );
                        break;
                    }
                }
                out.continuity_checked += 1;
                if at_boundary {
                    out.boundary_survivors_checked += 1;
                }
            } else if !(*pid >= pid_base && (*pid as u64) < pid_base as u64 + accepted as u64) {
                out.pid_emit_range_exact = false;
                problem(
                    &mut out.problems,
                    format!("帧 {f}: pid {pid} 非幸存亦非发射区间 [{pid_base}, +{accepted})"),
                );
            }
        }
        // 幸存段 ⊆ 上帧集(死亡只减不增;新段已在上环判)。
        if f > 0 {
            let survivors_ok = cur
                .keys()
                .all(|pid| prev.contains_key(pid) || *pid >= pid_base);
            if !survivors_ok {
                out.pid_survivor_subset = false;
                problem(&mut out.problems, format!("帧 {f}: 幸存段非上帧子集"));
            }
        }
        prev = cur;
        pid_base += accepted as u32;
        // ── 链式 digest(n ‖ pid ‖ 8 f32 流 bits ‖ args)──
        let mut trace: Vec<u8> = Vec::with_capacity(64 + b.n * 36 + 32);
        trace.extend_from_slice(digest_hex.as_bytes());
        trace.extend_from_slice(&(b.n as u32).to_le_bytes());
        for i in 0..b.n {
            trace.extend_from_slice(&b.pid[i].to_le_bytes());
            for v in [
                b.pos_x[i], b.pos_y[i], b.pos_z[i], b.vel_x[i], b.vel_y[i], b.vel_z[i], b.age[i],
                b.life[i],
            ] {
                trace.extend_from_slice(&v.to_bits().to_le_bytes());
            }
        }
        for w in st.args {
            trace.extend_from_slice(&w.to_le_bytes());
        }
        digest_hex = rurix_pkg::sha256::hex_digest(&trace);
        out.alive_final = st.alive_total;
        out.n_final = st.n_next;
        std::mem::swap(&mut a, &mut b);
    }
    out.pids_issued = pid_base;
    out.digest = format!("sha256:{digest_hex}");
    out
}

/// 曲线求值互核:emit_count_at vs probe 独立参考实现(阶梯 = 单循环最后
/// 命中;const = floor)逐帧全等 + 双求值确定(scan 双实现互核先例)。
fn curve_crosscheck(asset: &EmitterAsset, frames: u32) -> (bool, Vec<u32>) {
    let reference = |f: u32| -> u32 {
        match &asset.emit_curve {
            EmitCurve::Const { value } => value.floor() as u32,
            EmitCurve::Step { frames, values } => {
                let mut hit: Option<usize> = None;
                for (i, sf) in frames.iter().enumerate() {
                    if *sf <= f {
                        hit = Some(i);
                    }
                }
                hit.map(|i| values[i].floor() as u32).unwrap_or(0)
            }
        }
    };
    let mut ok = true;
    let mut samples = Vec::new();
    for f in 0..frames {
        let lib = asset.emit_count_at(f);
        ok &= lib == reference(f) && lib == asset.emit_count_at(f);
        if samples.len() < 12 {
            samples.push(lib);
        }
    }
    (ok, samples)
}

// ---------------------------------------------------------------------------
// 模式腿
// ---------------------------------------------------------------------------

fn write_samples(dir: &str) -> i32 {
    let d = std::path::Path::new(dir);
    std::fs::create_dir_all(d).unwrap_or_else(|e| usage_fail(&format!("建目录 {dir}: {e}")));
    let pa = d.join("campfire_sparks_a.emitter.json");
    let pb = d.join("ember_burst_b.emitter.json");
    std::fs::write(&pa, SAMPLE_ASSET_A).unwrap_or_else(|e| usage_fail(&format!("写样例 A: {e}")));
    std::fs::write(&pb, SAMPLE_ASSET_B).unwrap_or_else(|e| usage_fail(&format!("写样例 B: {e}")));
    // 内嵌样例自证:两份必须解析绿且参数面可分辨(判据有效性前提)。
    let a = EmitterAsset::parse(SAMPLE_ASSET_A).unwrap_or_else(|e| usage_fail(&format!("样例 A 内嵌违例: {e}")));
    let b = EmitterAsset::parse(SAMPLE_ASSET_B).unwrap_or_else(|e| usage_fail(&format!("样例 B 内嵌违例: {e}")));
    if a.to_desc() == b.to_desc() {
        usage_fail("内嵌样例 A/B 参数面不可分辨(重载判据失效)");
    }
    println!("AUTHORING_SAMPLES a={} b={}", pa.display(), pb.display());
    0
}

fn run_green(args: &Args) -> i32 {
    let t0 = std::time::Instant::now();
    let asset_path = args
        .asset
        .as_deref()
        .unwrap_or_else(|| usage_fail("--asset 必需(或 --write-samples)"));
    let asset_a = load_asset(asset_path);
    let asset_b = args.asset2.as_deref().map(load_asset);
    if asset_b.is_some() && (args.reload_at as usize) >= args.frames {
        usage_fail(&format!(
            "--reload-at {} 须 < --frames {}",
            args.reload_at, args.frames
        ));
    }
    let reload = asset_b.as_ref().map(|b| (args.reload_at, b));

    // 双跑(判据⑤)+ 基线(判据①)。
    let r1 = run_scenario(&asset_a, reload, args.frames, args.cap, args.seed);
    let r2 = run_scenario(&asset_a, reload, args.frames, args.cap, args.seed);
    let double_ok = r1.digest == r2.digest;
    let (baseline_digest, reload_effective, reload_next_frame_effective) = match &asset_b {
        Some(b) => {
            let base = run_scenario(&asset_a, None, args.frames, args.cap, args.seed);
            let expected_b = b.emit_count_at(args.reload_at);
            let expected_a = asset_a.emit_count_at(args.reload_at);
            (
                base.digest.clone(),
                r1.digest != base.digest,
                r1.accepted_at_reload == expected_b && expected_b != expected_a,
            )
        }
        None => (String::new(), false, false),
    };
    // 曲线互核(判据④;A 全窗 + B 若在场)。
    let (ca_ok, ca_samples) = curve_crosscheck(&asset_a, args.frames as u32);
    let (cb_ok, cb_samples) = match &asset_b {
        Some(b) => curve_crosscheck(b, args.frames as u32),
        None => (true, Vec::new()),
    };
    let curve_ok = ca_ok && cb_ok;
    let pid_ok = r1.pid_unique && r1.pid_survivor_subset && r1.pid_emit_range_exact;
    let judgments_ok = double_ok
        && curve_ok
        && pid_ok
        && r1.old_particles_continuous
        && (asset_b.is_none() || (reload_effective && reload_next_frame_effective));
    let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let line = format!(
        concat!(
            "{{\"schema\":\"rurix.g35.authoring_probe.v1\",\"mode\":\"green\",",
            "\"frames\":{},\"cap\":{},\"seed\":{},\"dt\":{:.9},",
            "\"asset_name\":{},\"asset2_name\":{},\"reload_at\":{},",
            "\"reload_effective_digest_diff\":{},\"digest_baseline\":{},",
            "\"reload_next_frame_effective\":{},\"requested_at_reload\":{},\"accepted_at_reload\":{},",
            "\"old_particles_continuous\":{},\"continuity_checked\":{},\"boundary_survivors_checked\":{},",
            "\"pid_unique\":{},\"pid_survivor_subset\":{},\"pid_emit_range_exact\":{},",
            "\"curve_crosscheck_ok\":{},\"curve_samples_a\":{},\"curve_samples_b\":{},",
            "\"emit_head\":{},\"double_run_bitexact\":{},\"digest_a\":{},\"digest_b\":{},",
            "\"rejected_total\":{},\"pids_issued\":{},\"alive_final\":{},\"n_final\":{},",
            "\"elapsed_ms\":{:.3},\"problems\":{}}}"
        ),
        args.frames,
        args.cap,
        args.seed,
        DT as f64,
        jstr(&asset_a.name),
        asset_b
            .as_ref()
            .map(|b| jstr(&b.name))
            .unwrap_or_else(|| "null".into()),
        args.reload_at,
        reload_effective,
        if baseline_digest.is_empty() {
            "null".into()
        } else {
            jstr(&baseline_digest)
        },
        reload_next_frame_effective,
        r1.requested_at_reload,
        r1.accepted_at_reload,
        r1.old_particles_continuous,
        r1.continuity_checked,
        r1.boundary_survivors_checked,
        r1.pid_unique,
        r1.pid_survivor_subset,
        r1.pid_emit_range_exact,
        curve_ok,
        u32s_json(&ca_samples),
        u32s_json(&cb_samples),
        u32s_json(&r1.emit_head),
        double_ok,
        jstr(&r1.digest),
        jstr(&r2.digest),
        r1.rejected_total,
        r1.pids_issued,
        r1.alive_final,
        r1.n_final,
        elapsed_ms,
        strs_json(&r1.problems),
    );
    emit_evidence(&line, &args.evidence_out);
    if judgments_ok {
        println!("AUTHORING_PROBE_OK");
        0
    } else {
        eprintln!("{TAG}: FAIL 判据红(problems={:?})", r1.problems);
        1
    }
}

fn run_red_arm(args: &Args, arm: &str) -> i32 {
    if arm != "field-tamper" {
        usage_fail(&format!("--red-arm {arm} 越闭集 {{field-tamper}}"));
    }
    let asset_path = args
        .asset
        .as_deref()
        .unwrap_or_else(|| usage_fail("--asset 必需"));
    let text = std::fs::read_to_string(asset_path)
        .unwrap_or_else(|e| usage_fail(&format!("读资产 {asset_path}: {e}")));
    let asset = load_asset(asset_path);
    // (a) 值篡改臂:gravity_y/vel_base 字段内存面篡改 → digest 必异。
    let mut tampered = asset.clone();
    tampered.gravity_y += 1.5;
    tampered.vel_base[1] += 0.5;
    let green = run_scenario(&asset, None, args.frames, args.cap, args.seed);
    let red = run_scenario(&tampered, None, args.frames, args.cap, args.seed);
    let detected = green.digest != red.digest;
    // (b) schema 篡改臂:文本注入闭集外字段 → 解析必 typed Err(fail-closed)。
    let tampered_text = text.replacen('{', "{\"tampered_field\": 1.0,", 1);
    let schema_err = EmitterAsset::parse(&tampered_text).err();
    let schema_detected = schema_err.is_some();
    let schema_kind = schema_err
        .as_ref()
        .map(|e| e.kind_name())
        .unwrap_or("NONE");
    let line = format!(
        concat!(
            "{{\"schema\":\"rurix.g35.authoring_probe_red.v1\",\"mode\":\"red-arm\",",
            "\"arm\":\"field-tamper\",\"frames\":{},\"cap\":{},\"seed\":{},",
            "\"detected\":{},\"digest_green\":{},\"digest_red\":{},",
            "\"schema_tamper_detected\":{},\"schema_tamper_kind\":{}}}"
        ),
        args.frames,
        args.cap,
        args.seed,
        detected,
        jstr(&green.digest),
        jstr(&red.digest),
        schema_detected,
        jstr(schema_kind),
    );
    emit_evidence(&line, &args.evidence_out);
    if detected && schema_detected {
        println!("AUTHORING_RED_ARM_OK");
        0
    } else {
        eprintln!("{TAG}: FAIL RED 臂漏检(detected={detected} schema={schema_detected})");
        1
    }
}

fn main() {
    let args = parse_args();
    let code = if let Some(dir) = &args.write_samples {
        write_samples(dir)
    } else if let Some(arm) = args.red_arm.clone() {
        run_red_arm(&args, &arm)
    } else {
        run_green(&args)
    };
    std::process::exit(code)
}
