//! G35-9 确定性回放/回滚 device probe harness(门 g35.wave9.replay;
//! RFC-0049 §4.12;g35_particle_core_device 七 kernel 链/digest 链式/双跑/
//! 三态同模的「journal 化」变体)。
//!
//! ## 集成路径
//!
//! bin-local 全部逻辑:消费 W2 七 kernel(g35_{sim,particle_compact,emit,
//! indirect_args}.rx + scan 三件,**只消费不修改**)经 `rurix_rt::vk::
//! run_compute` 逐 kernel 派发,SoA 9 流 ping-pong 双组 `Vec<Vec<u8>>` 帧末
//! 交换——device 帧链与 g35_particle_core_device **逐字同模**(digest 口径
//! 位级可比);journal/检查点结构与序列化 = `particles/replay.rs`(v1 冻结
//! 布局,魔数 "G35J"/"G35C")。host 平行金标准 = core::frame() 逐帧推进,
//! 整数流(flags/scan_out/seg_offsets/pid/args)零容差对拍维持(f32 容差面
//! 归 g35.wave2 门,本门全位级:digest 域已按字节覆盖 f32 流)。
//!
//! ## 四腿
//!
//! - `--record`:确定性脚本(emit = min(64 + f·17 % 192, cap − n_curr),
//!   冻结夹具 emitter,dt = 1/60)跑 device 全链 N 帧,逐帧 digest 链
//!   (全流字节 sha256 链式,g35_particle_core_device 同式)+ 每 K=16 帧
//!   **帧开始前** readback 九流+pid_base+n_curr 存检查点 → journal 落
//!   `--journal-out`、digest 链落 `--digest-out`(逐行 64 hex)、检查点落
//!   `--checkpoint-out`(缺省 = `<journal-out>.ckpt`)。
//! - `--replay`:**仅凭 journal 重建输入**(seed→随机带、emitter、dt、逐帧
//!   emit 序列)重跑 device 全链——GPU 重仿真非 host 回放;逐帧 digest 与
//!   录制链位级全等(首异帧 = -1)+ 同输入双跑位级(determinism_double_run)。
//! - `--rollback <k> --to <j>`:检查点 k 恢复 device 缓冲(上传恢复)→
//!   重仿真帧 k..=j;digest 链种子 = 录制链 digest[k−1](k=0 全零种子)⇒
//!   恢复帧自身 digest 全等(checkpoint_restore_bitexact)+ 逐帧至 j 全等
//!   (rollback_resim_bitexact)。网络回滚语义 = 检查点 + 输入重放。
//! - `--red-arm journal-tamper`:篡改 journal 帧 32 emit_count(+1)重放,
//!   断言 digest 链首异帧 == 32(分歧可定位见证——确定性系统独有性质,
//!   Niagara GPU sim 做不到)。
//!
//! ## NoContraction / 三态
//!
//! sim/emit 装载期注入 NoContraction(g35_particle_core_device 同律 bin-local
//! 复制;SPV 文件 0-byte 不动)。无 Vulkan loader/设备 → `skipped_dev_env`
//! JSON 退 0(非 fake pass;`RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke
//! 脚本层裁决)。
//!
//! ## 用法
//!
//! ```text
//! g35_replay_device --record --spv-sim <p> --spv-compact <p> --spv-emit <p>
//!     --spv-indirect-args <p> --spv-scan-seg-sum <p> --spv-scan-spine <p>
//!     --spv-scan-seg-apply <p> --journal-out <p> --digest-out <p>
//!     [--checkpoint-out <p>] [--frames 64] [--cap 65536] [--seed 42]
//!     [--evidence-out <p>]
//! g35_replay_device --replay --journal <p> --digest <p> --spv-... <7 件>
//! g35_replay_device --rollback 16 --to 48 --journal <p> --digest <p>
//!     [--checkpoint <p>] --spv-... <7 件>
//! g35_replay_device --red-arm journal-tamper --journal <p> --digest <p>
//!     --spv-... <7 件>
//! ```

#![forbid(unsafe_code)]

use std::time::Instant;

use rurix_render::particles::core::{EmitterDesc, ParticlePools, frame, sim_step};
use rurix_render::particles::replay::{
    Checkpoint, CheckpointFile, FrameRecord, Journal, JournalHeader,
};
use rurix_render::particles::{SEG, rand_table, scan};
use rurix_rt::vk;

const TAG: &str = "[g35_replay_device]";
const DEFAULT_FRAMES: usize = 64;
const DEFAULT_CAP: usize = 65536;
const DEFAULT_SEED: u64 = 42;
/// dt = 1/60(冻结确定性脚本;journal header 登记)。
const DT: f32 = 1.0 / 60.0;
/// 检查点间隔 K(冻结;录制腿逐 K 帧帧开始前捕获)。
const CHECKPOINT_INTERVAL: usize = 16;
/// 红臂篡改帧(冻结;首异帧见证锚)。
const TAMPER_FRAME: usize = 32;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

/// 冻结发射器夹具(确定性脚本;g35_particle_core_device 同值本 probe 独立
/// 冻结——life ∈ [0.8, 1.6) ⇒ 64 帧 @1/60s 窗内必有寿命耗尽死亡)。
fn emitter() -> EmitterDesc {
    EmitterDesc {
        pos: [0.0, 1.5, -0.25],
        spread: [0.5, 0.25, 0.5],
        vel_base: [0.0, 3.0, 0.0],
        vel_spread: [1.5, 0.75, 1.5],
        life_base: 1.6,
        gravity_y: -9.8,
    }
}

/// 确定性发射预算(冻结脚本;仅录制腿消费——回放/回滚/红臂一律直接消费
/// journal 记录,不重算脚本)。
fn emit_schedule(f: usize, n_curr: usize, cap: usize) -> usize {
    (64 + (f * 17) % 192).min(cap - n_curr)
}

// ---------------------------------------------------------------------------
// 字节工具(g35_particle_core_device 先例字面)
// ---------------------------------------------------------------------------

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_u32(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
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

/// SPIR-V NoContraction 后处理(g35_particle_core_device 同律 bin-local
/// 复制,原出处 g14_3_lane_body.rs;SPV 文件 0-byte 不动):对全部
/// OpFAdd/OpFSub/OpFMul 结果 id 注入 `OpDecorate %id NoContraction`,
/// 禁驱动 mul+add FMA 收缩——GPU 浮点序列与 host 严格 IEEE 逐 op 对齐。
fn spv_inject_no_contraction(spv: &[u32]) -> Vec<u32> {
    let mut result_ids: Vec<u32> = Vec::new();
    let mut i = 5usize; // SPIR-V header 5 字
    let mut first_decorate: Option<usize> = None;
    let mut first_type: Option<usize> = None;
    while i < spv.len() {
        let w = spv[i];
        let wc = (w >> 16) as usize;
        let op = w & 0xFFFF;
        if wc == 0 || i + wc > spv.len() {
            fail("SPIR-V 指令流越界(NoContraction 注入)");
        }
        match op {
            // OpDecorate(annotation 段前沿 = 注入锚)。
            71 if first_decorate.is_none() => first_decorate = Some(i),
            // OpType*(备用锚:无 annotation 段时插在 type 段前)。
            19..=39 if first_type.is_none() => first_type = Some(i),
            // OpFAdd(129)/OpFSub(131)/OpFMul(133) 结果 id。
            129 | 131 | 133 => result_ids.push(spv[i + 2]),
            _ => {}
        }
        i += wc;
    }
    let at = first_decorate
        .or(first_type)
        .unwrap_or_else(|| fail("SPIR-V 无 annotation/type 段锚(NoContraction 注入)"));
    let mut out = Vec::with_capacity(spv.len() + result_ids.len() * 3);
    out.extend_from_slice(&spv[..at]);
    for id in &result_ids {
        out.push(71u32 | (3 << 16)); // OpDecorate(wc=3)
        out.push(*id);
        out.push(42); // Decoration NoContraction
    }
    out.extend_from_slice(&spv[at..]);
    out
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

/// 出报(stdout 恒打;--evidence-out 同步落盘,g35_particle_core 同模)。
fn emit_evidence(line: &str, out: &Option<String>) {
    println!("{line}");
    if let Some(path) = out {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, format!("{line}\n"))
            .unwrap_or_else(|e| fail(&format!("写 --evidence-out {path}: {e}")));
    }
}

fn sha_tag(hex: &str) -> String {
    jstr(&format!("sha256:{hex}"))
}

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Record,
    Replay,
    Rollback,
    RedArm,
}

struct Args {
    spv_sim: Option<String>,
    spv_compact: Option<String>,
    spv_emit: Option<String>,
    spv_indirect_args: Option<String>,
    spv_scan_seg_sum: Option<String>,
    spv_scan_spine: Option<String>,
    spv_scan_seg_apply: Option<String>,
    frames: usize,
    cap: usize,
    seed: u64,
    evidence_out: Option<String>,
    mode: Option<Mode>,
    journal: Option<String>,
    journal_out: Option<String>,
    digest: Option<String>,
    digest_out: Option<String>,
    checkpoint: Option<String>,
    rollback_k: usize,
    rollback_to: usize,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        spv_sim: None,
        spv_compact: None,
        spv_emit: None,
        spv_indirect_args: None,
        spv_scan_seg_sum: None,
        spv_scan_spine: None,
        spv_scan_seg_apply: None,
        frames: DEFAULT_FRAMES,
        cap: DEFAULT_CAP,
        seed: DEFAULT_SEED,
        evidence_out: None,
        mode: None,
        journal: None,
        journal_out: None,
        digest: None,
        digest_out: None,
        checkpoint: None,
        rollback_k: 0,
        rollback_to: 0,
        red_arm: None,
    };
    let mut it = std::env::args().skip(1);
    let next_or = |it: &mut dyn Iterator<Item = String>, k: &str| {
        it.next().unwrap_or_else(|| fail(&format!("{k} 缺值")))
    };
    let set_mode = |a: &mut Args, m: Mode| {
        if a.mode.is_some() {
            fail("--record/--replay/--rollback/--red-arm 四腿互斥,只许其一");
        }
        a.mode = Some(m);
    };
    while let Some(k) = it.next() {
        match k.as_str() {
            "--spv-sim" => a.spv_sim = it.next(),
            "--spv-compact" => a.spv_compact = it.next(),
            "--spv-emit" => a.spv_emit = it.next(),
            "--spv-indirect-args" => a.spv_indirect_args = it.next(),
            "--spv-scan-seg-sum" => a.spv_scan_seg_sum = it.next(),
            "--spv-scan-spine" => a.spv_scan_spine = it.next(),
            "--spv-scan-seg-apply" => a.spv_scan_seg_apply = it.next(),
            "--record" => set_mode(&mut a, Mode::Record),
            "--replay" => set_mode(&mut a, Mode::Replay),
            "--rollback" => {
                set_mode(&mut a, Mode::Rollback);
                a.rollback_k = next_or(&mut it, "--rollback")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--rollback 非法: {e}")));
            }
            "--to" => {
                a.rollback_to = next_or(&mut it, "--to")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--to 非法: {e}")));
            }
            "--red-arm" => {
                set_mode(&mut a, Mode::RedArm);
                a.red_arm = it.next();
            }
            "--journal" => a.journal = it.next(),
            "--journal-out" => a.journal_out = it.next(),
            "--digest" => a.digest = it.next(),
            "--digest-out" => a.digest_out = it.next(),
            "--checkpoint" | "--checkpoint-out" => a.checkpoint = it.next(),
            "--frames" => {
                a.frames = next_or(&mut it, "--frames")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--frames 非法: {e}")));
            }
            "--cap" => {
                a.cap = next_or(&mut it, "--cap")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--cap 非法: {e}")));
            }
            "--seed" => {
                a.seed = next_or(&mut it, "--seed")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--seed 非法: {e}")));
            }
            "--evidence-out" => a.evidence_out = it.next(),
            other => fail(&format!("未知参数: {other}")),
        }
    }
    if a.frames == 0 {
        fail("--frames 必须 ≥ 1");
    }
    if a.cap == 0 || a.cap % SEG != 0 {
        fail(&format!("--cap 必须为 SEG={SEG} 正整倍数(得 {})", a.cap));
    }
    a
}

fn need(o: &Option<String>, k: &str) -> String {
    o.clone().unwrap_or_else(|| fail(&format!("缺 {k}")))
}

// ---------------------------------------------------------------------------
// device 臂(bin-local;g35_particle_core_device 逐字同模)
// ---------------------------------------------------------------------------

struct DevKernels {
    spv_sim: Vec<u32>,
    entry_sim: String,
    spv_compact: Vec<u32>,
    entry_compact: String,
    spv_emit: Vec<u32>,
    entry_emit: String,
    spv_args: Vec<u32>,
    entry_args: String,
    spv_seg_sum: Vec<u32>,
    entry_seg_sum: String,
    spv_spine: Vec<u32>,
    entry_spine: String,
    spv_seg_apply: Vec<u32>,
    entry_seg_apply: String,
}

impl DevKernels {
    fn create(args: &Args) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        // sim/emit 注入 NoContraction(f32 乘加链 kernel);compact 纯搬运 /
        // indirect_args 纯整数 / scan 三件纯整数,不注入(particle_core 同律)。
        let spv_sim = spv_inject_no_contraction(&load_spv(&need(&args.spv_sim, "--spv-sim")));
        let spv_compact = load_spv(&need(&args.spv_compact, "--spv-compact"));
        let spv_emit = spv_inject_no_contraction(&load_spv(&need(&args.spv_emit, "--spv-emit")));
        let spv_args = load_spv(&need(&args.spv_indirect_args, "--spv-indirect-args"));
        let spv_seg_sum = load_spv(&need(&args.spv_scan_seg_sum, "--spv-scan-seg-sum"));
        let spv_spine = load_spv(&need(&args.spv_scan_spine, "--spv-scan-spine"));
        let spv_seg_apply = load_spv(&need(&args.spv_scan_seg_apply, "--spv-scan-seg-apply"));
        let entry = |spv: &[u32], k: &str| -> Result<String, String> {
            vk::entry_point_name(spv).ok_or(format!("{k} SPV 无 OpEntryPoint"))
        };
        Ok(Self {
            entry_sim: entry(&spv_sim, "sim")?,
            entry_compact: entry(&spv_compact, "compact")?,
            entry_emit: entry(&spv_emit, "emit")?,
            entry_args: entry(&spv_args, "indirect_args")?,
            entry_seg_sum: entry(&spv_seg_sum, "scan_seg_sum")?,
            entry_spine: entry(&spv_spine, "scan_spine")?,
            entry_seg_apply: entry(&spv_seg_apply, "scan_seg_apply")?,
            spv_sim,
            spv_compact,
            spv_emit,
            spv_args,
            spv_seg_sum,
            spv_spine,
            spv_seg_apply,
        })
    }
}

/// device 侧全缓冲(9 流序 = 0 pos_x / 1 pos_y / 2 pos_z / 3 vel_x /
/// 4 vel_y / 5 vel_z / 6 age / 7 life / 8 pid——与 kernel SSBO 序及
/// replay.rs Checkpoint 流序严格一致)。
struct DevState {
    a: Vec<Vec<u8>>,
    b: Vec<Vec<u8>>,
    flags: Vec<u8>,
    scan_out: Vec<u8>,
    seg_sums: Vec<u8>,
    seg_offsets: Vec<u8>,
    args: Vec<u8>,
    rand: Vec<u8>,
}

impl DevState {
    fn new(cap: usize, rand_bytes: Vec<u8>) -> Self {
        let nseg_cap = cap / SEG;
        Self {
            a: (0..9).map(|_| vec![0u8; cap * 4]).collect(),
            b: (0..9).map(|_| vec![0u8; cap * 4]).collect(),
            flags: vec![0u8; cap * 4],
            scan_out: vec![0u8; cap * 4],
            seg_sums: vec![0u8; nseg_cap * 4],
            seg_offsets: vec![0u8; (nseg_cap + 1) * 4],
            args: vec![0u8; 8 * 4],
            rand: rand_bytes,
        }
    }
}

/// device 单帧 7 kernel 链(G35-P v1 帧序;g35_particle_core_device 逐字
/// 同模);nseg == 0 时 sim/seg_sum/seg_apply/compact 零段跳过,spine/
/// indirect_args 恒跑,emit == 0 跳过 emit。
#[allow(clippy::too_many_arguments)]
fn device_frame(
    dev: &DevKernels,
    st: &mut DevState,
    n: usize,
    desc: &EmitterDesc,
    dt: f32,
    pid_base: u32,
    emit_count: usize,
) -> Result<(), String> {
    let nseg = n.div_ceil(SEG);
    let take = std::mem::take::<Vec<u8>>;
    // 1. sim(10 SSBO:params/pos3/vel3/age/life/flags)。
    if nseg > 0 {
        let params = [n as f32, nseg as f32, dt, desc.gravity_y, 0.0, 0.0, 0.0, 0.0];
        let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(10);
        bufs.push(bytes_f32(&params));
        for k in 0..8 {
            bufs.push(take(&mut st.a[k]));
        }
        bufs.push(take(&mut st.flags));
        vk::run_compute(&dev.spv_sim, &dev.entry_sim, &mut bufs, &[], [
            nseg as u32,
            1,
            1,
        ])
        .map_err(|e| format!("sim dispatch: {e}"))?;
        for k in 0..8 {
            st.a[k] = take(&mut bufs[1 + k]);
        }
        st.flags = take(&mut bufs[9]);
    }
    // 2a. scan_seg_sum(values/params/seg_sums)。
    let scan_params = bytes_f32(&[n as f32, nseg as f32, 0.0, 0.0]);
    if nseg > 0 {
        let mut bufs = vec![take(&mut st.flags), scan_params.clone(), take(&mut st.seg_sums)];
        vk::run_compute(&dev.spv_seg_sum, &dev.entry_seg_sum, &mut bufs, &[], [
            nseg as u32,
            1,
            1,
        ])
        .map_err(|e| format!("scan_seg_sum dispatch: {e}"))?;
        st.flags = take(&mut bufs[0]);
        st.seg_sums = take(&mut bufs[2]);
    }
    // 2b. scan_spine(seg_sums/params/seg_offsets;恒跑——nseg=0 时写总和槽 0)。
    {
        let mut bufs = vec![
            take(&mut st.seg_sums),
            scan_params.clone(),
            take(&mut st.seg_offsets),
        ];
        vk::run_compute(&dev.spv_spine, &dev.entry_spine, &mut bufs, &[], [1, 1, 1])
            .map_err(|e| format!("scan_spine dispatch: {e}"))?;
        st.seg_sums = take(&mut bufs[0]);
        st.seg_offsets = take(&mut bufs[2]);
    }
    // 2c. scan_seg_apply(values/seg_offsets/params/out_scan)。
    if nseg > 0 {
        let mut bufs = vec![
            take(&mut st.flags),
            take(&mut st.seg_offsets),
            scan_params,
            take(&mut st.scan_out),
        ];
        vk::run_compute(&dev.spv_seg_apply, &dev.entry_seg_apply, &mut bufs, &[], [
            nseg as u32,
            1,
            1,
        ])
        .map_err(|e| format!("scan_seg_apply dispatch: {e}"))?;
        st.flags = take(&mut bufs[0]);
        st.seg_offsets = take(&mut bufs[1]);
        st.scan_out = take(&mut bufs[3]);
    }
    // 3. particle_compact(21 SSBO:params/flags/scan_out/A×9/B×9)。
    if nseg > 0 {
        let params = bytes_f32(&[n as f32, nseg as f32, 0.0, 0.0]);
        let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(21);
        bufs.push(params);
        bufs.push(take(&mut st.flags));
        bufs.push(take(&mut st.scan_out));
        for k in 0..9 {
            bufs.push(take(&mut st.a[k]));
        }
        for k in 0..9 {
            bufs.push(take(&mut st.b[k]));
        }
        vk::run_compute(&dev.spv_compact, &dev.entry_compact, &mut bufs, &[], [
            nseg as u32,
            1,
            1,
        ])
        .map_err(|e| format!("particle_compact dispatch: {e}"))?;
        st.flags = take(&mut bufs[1]);
        st.scan_out = take(&mut bufs[2]);
        for k in 0..9 {
            st.a[k] = take(&mut bufs[3 + k]);
        }
        for k in 0..9 {
            st.b[k] = take(&mut bufs[12 + k]);
        }
    }
    // 4. emit(12 SSBO:params16/seg_offsets/rand_table/B×9;alive_slot =
    //    params[15] = nseg——device 读 seg_offsets[nseg] 零回读)。
    if emit_count > 0 {
        let d = desc;
        let params = [
            emit_count as f32,
            pid_base as f32,
            d.pos[0],
            d.pos[1],
            d.pos[2],
            d.spread[0],
            d.spread[1],
            d.spread[2],
            d.vel_base[0],
            d.vel_base[1],
            d.vel_base[2],
            d.vel_spread[0],
            d.vel_spread[1],
            d.vel_spread[2],
            d.life_base,
            nseg as f32,
        ];
        let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(12);
        bufs.push(bytes_f32(&params));
        bufs.push(take(&mut st.seg_offsets));
        bufs.push(take(&mut st.rand));
        for k in 0..9 {
            bufs.push(take(&mut st.b[k]));
        }
        vk::run_compute(&dev.spv_emit, &dev.entry_emit, &mut bufs, &[], [
            emit_count as u32,
            1,
            1,
        ])
        .map_err(|e| format!("emit dispatch: {e}"))?;
        st.seg_offsets = take(&mut bufs[1]);
        st.rand = take(&mut bufs[2]);
        for k in 0..9 {
            st.b[k] = take(&mut bufs[3 + k]);
        }
    }
    // 5. indirect_args(3 SSBO:params/seg_offsets/args_out;单 invocation)。
    {
        let params = bytes_f32(&[emit_count as f32, nseg as f32, 0.0, 0.0]);
        let mut bufs = vec![params, take(&mut st.seg_offsets), take(&mut st.args)];
        vk::run_compute(&dev.spv_args, &dev.entry_args, &mut bufs, &[], [1, 1, 1])
            .map_err(|e| format!("indirect_args dispatch: {e}"))?;
        st.seg_offsets = take(&mut bufs[1]);
        st.args = take(&mut bufs[2]);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 帧链跑腿(录制/回放/回滚共用单一代码路径;host 金标准平行对拍维持)
// ---------------------------------------------------------------------------

struct ChainRun {
    /// 逐帧链式 digest(纯 64 hex;下标 = 帧 − start_frame)。
    digests: Vec<String>,
    /// 逐 K 帧检查点(仅 capture_checkpoints 时;帧开始前捕获)。
    checkpoints: Vec<Checkpoint>,
    /// 逐帧实际消费 emit(录制腿 = 冻结脚本产;其余腿 = journal 镜像)。
    emit_counts: Vec<u32>,
    /// host 平行金标准整数流(flags/scan_out/seg_offsets/pid/args)零容差
    /// 全帧位级(f32 容差面归 g35.wave2 门;digest 已按字节覆盖 f32 流)。
    host_parallel_bitexact: bool,
    problems: Vec<String>,
    frame_ms_mean: f64,
    n_final: usize,
    pids_issued: u32,
}

/// device 帧链驱动(digest 口径 = g35_particle_core_device 逐字同式:
/// sha256(prev_hex ‖ B 9 流有效前缀 ‖ flags ‖ scan_out ‖ seg_offsets ‖
/// args) 逐帧链式)。`start` = None 全零起步(digest 种子全零 64 hex)/
/// Some((检查点, digest 链种子)) 恢复起步(上传恢复,B 组清零即可——
/// digest 只覆盖各流有效前缀且该前缀每帧被全量重写)。
fn run_chain(
    dev: &DevKernels,
    header: &JournalHeader,
    emits: Option<&[u32]>,
    capture_checkpoints: bool,
    start: Option<(&Checkpoint, &str)>,
    end_frame: usize,
) -> ChainRun {
    let cap = header.cap as usize;
    let desc = header.emitter;
    let table = rand_table(header.seed);
    let (start_frame, mut ha, mut st, mut pid_base, mut digest) = match start {
        None => (
            0usize,
            ParticlePools::with_capacity(cap),
            DevState::new(cap, bytes_f32(&table)),
            0u32,
            "0".repeat(64),
        ),
        Some((ck, seed)) => {
            let ha = ck.restore_pools();
            if ha.capacity() != cap {
                fail(&format!("检查点容量 {} ≠ journal cap {cap}", ha.capacity()));
            }
            let mut st = DevState::new(cap, bytes_f32(&table));
            for (k, s) in ck.streams.iter().enumerate() {
                st.a[k] = s.clone();
            }
            (ck.frame as usize, ha, st, ck.pid_base, seed.to_string())
        }
    };
    let mut hb = ParticlePools::with_capacity(cap);
    let mut r = ChainRun {
        digests: Vec::with_capacity(end_frame - start_frame),
        checkpoints: Vec::new(),
        emit_counts: Vec::with_capacity(end_frame - start_frame),
        host_parallel_bitexact: true,
        problems: Vec::new(),
        frame_ms_mean: 0.0,
        n_final: 0,
        pids_issued: 0,
    };
    let mut ms_total = 0.0f64;
    let problem = |problems: &mut Vec<String>, ok: &mut bool, msg: String| {
        *ok = false;
        if problems.len() < 16 {
            problems.push(msg);
        }
    };
    for f in start_frame..end_frame {
        // 检查点 = 帧开始前捕获(九流 A 组全容量原字节 + pid_base + n_curr)。
        if capture_checkpoints && f % CHECKPOINT_INTERVAL == 0 {
            r.checkpoints.push(Checkpoint {
                frame: f as u32,
                pid_base,
                n_curr: ha.n as u32,
                streams: st.a.clone(),
            });
        }
        let n = ha.n;
        let nseg = n.div_ceil(SEG);
        let emit = match emits {
            Some(e) => e[f] as usize,
            None => emit_schedule(f, n, cap),
        };
        // ── host 金标准 frame() 平行推进 + 中间流重放(particle_core 同式)──
        let mut pre = ha.clone();
        let stats = frame(&mut ha, &mut hb, &desc, &table, header.dt, pid_base, emit);
        let hflags = sim_step(&mut pre, header.dt, desc.gravity_y);
        let hsums = scan::seg_sums(&hflags, nseg);
        let hspine = scan::spine(&hsums);
        let hscan = scan::seg_apply(&hflags, &hspine, nseg);
        // ── device 7 kernel 链(墙钟计时)──
        let t0 = Instant::now();
        device_frame(dev, &mut st, n, &desc, header.dt, pid_base, emit)
            .unwrap_or_else(|e| fail(&format!("帧 {f}: {e}")));
        ms_total += t0.elapsed().as_secs_f64() * 1000.0;
        // ── 整数流零容差对拍(flags/scan_out/seg_offsets/args/pid)──
        let dev_flags = read_u32(&st.flags);
        if dev_flags[..n] != hflags[..] {
            problem(&mut r.problems, &mut r.host_parallel_bitexact, format!("帧 {f}: flags 非位级"));
        }
        let dev_scan = read_u32(&st.scan_out);
        if dev_scan[..n] != hscan[..] {
            problem(&mut r.problems, &mut r.host_parallel_bitexact, format!("帧 {f}: scan_out 非位级"));
        }
        let dev_spine = read_u32(&st.seg_offsets);
        if dev_spine[..nseg + 1] != hspine[..] {
            problem(&mut r.problems, &mut r.host_parallel_bitexact, format!("帧 {f}: seg_offsets 非位级"));
        }
        let dev_args = read_u32(&st.args);
        if dev_args[..8] != stats.args[..] {
            problem(
                &mut r.problems,
                &mut r.host_parallel_bitexact,
                format!("帧 {f}: args {:?} ≠ host {:?}", &dev_args[..8], stats.args),
            );
        }
        let n_next = stats.n_next;
        let dev_pid = read_u32(&st.b[8]);
        if dev_pid[..n_next] != hb.pid[..n_next] {
            problem(&mut r.problems, &mut r.host_parallel_bitexact, format!("帧 {f}: pid 流非位级"));
        }
        // ── 链式 digest(particle_core 逐字同式)──
        let mut trace: Vec<u8> = Vec::with_capacity(digest.len() + n_next * 36 + n * 8 + 64);
        trace.extend_from_slice(digest.as_bytes());
        for k in 0..9 {
            trace.extend_from_slice(&st.b[k][..n_next * 4]);
        }
        trace.extend_from_slice(&st.flags[..n * 4]);
        trace.extend_from_slice(&st.scan_out[..n * 4]);
        trace.extend_from_slice(&st.seg_offsets[..(nseg + 1) * 4]);
        trace.extend_from_slice(&st.args);
        digest = rurix_pkg::sha256::hex_digest(&trace);
        r.digests.push(digest.clone());
        // ── 帧末交换(读 A 写 B;n_curr 由 host 平行金标准维护零回读)──
        r.emit_counts.push(emit as u32);
        pid_base += emit as u32;
        r.n_final = n_next;
        std::mem::swap(&mut ha, &mut hb);
        std::mem::swap(&mut st.a, &mut st.b);
    }
    r.pids_issued = pid_base;
    r.frame_ms_mean = ms_total / (end_frame - start_frame).max(1) as f64;
    r
}

/// 首异帧(-1 = 全等;长度不齐 = 短边处分歧)。
fn first_divergence(a: &[String], b: &[String]) -> i64 {
    let m = a.len().min(b.len());
    for i in 0..m {
        if a[i] != b[i] {
            return i as i64;
        }
    }
    if a.len() != b.len() { m as i64 } else { -1 }
}

fn write_file(path: &str, bytes: &[u8], what: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, bytes).unwrap_or_else(|e| fail(&format!("写 {what} {path}: {e}")));
}

// ---------------------------------------------------------------------------
// 四腿
// ---------------------------------------------------------------------------

fn record_leg(dev: &DevKernels, args: &Args) -> ! {
    let desc = emitter();
    let header = JournalHeader {
        seed: args.seed,
        cap: args.cap as u32,
        frames: args.frames as u32,
        dt: DT,
        gravity_y: desc.gravity_y,
        emitter: desc,
    };
    let run = run_chain(dev, &header, None, true, None, args.frames);
    let journal = Journal {
        header,
        records: run
            .emit_counts
            .iter()
            .map(|&e| FrameRecord { emit_count: e })
            .collect(),
    };
    journal.validate_v1().unwrap_or_else(|e| fail(&format!("录制产 journal 未过 v1 校验: {e}")));
    let jpath = need(&args.journal_out, "--journal-out");
    let jbytes = journal.serialize();
    write_file(&jpath, &jbytes, "journal");
    let dpath = need(&args.digest_out, "--digest-out");
    let mut dtext = run.digests.join("\n");
    dtext.push('\n');
    write_file(&dpath, dtext.as_bytes(), "digest 链");
    let ckpath = args
        .checkpoint
        .clone()
        .unwrap_or_else(|| format!("{jpath}.ckpt"));
    let ckfile = CheckpointFile {
        cap: header.cap,
        interval: CHECKPOINT_INTERVAL as u32,
        checkpoints: run.checkpoints,
    };
    write_file(&ckpath, &ckfile.serialize(), "检查点文件");
    let jsha = rurix_pkg::sha256::hex_digest(&jbytes);
    let ck_frames: Vec<String> = ckfile.checkpoints.iter().map(|c| c.frame.to_string()).collect();
    let ok = run.host_parallel_bitexact;
    let state = if ok { "pass" } else { "fail" };
    eprintln!(
        "{TAG}: record {state} frames={} cap={} seed={} journal={} B checkpoints=[{}] \
         digest_final={} host_int_bitexact={} frame_ms={:.3}",
        args.frames,
        args.cap,
        args.seed,
        jbytes.len(),
        ck_frames.join(","),
        &run.digests[args.frames - 1][..16],
        ok,
        run.frame_ms_mean,
    );
    let line = format!(
        "{{\"schema\":\"rurix.g35.replay_record.v1\",\"state\":{},\"frames\":{},\"cap\":{},\
         \"seed\":{},\"dt\":{:e},\"emit_schedule\":\"min(64 + frame*17 % 192, cap - n_curr)\",\
         \"journal_path\":{},\"journal_bytes\":{},\"journal_sha256\":{},\
         \"digest_path\":{},\"digest_final\":{},\
         \"checkpoint_path\":{},\"checkpoint_interval\":{CHECKPOINT_INTERVAL},\
         \"checkpoint_frames\":[{}],\"host_parallel_bitexact\":{},\
         \"n_final\":{},\"pids_issued\":{},\"frame_ms_mean\":{:.6},\
         \"problems\":{},\"base_commit\":{}}}",
        jstr(state),
        args.frames,
        args.cap,
        args.seed,
        DT,
        jstr(&jpath.replace('\\', "/")),
        jbytes.len(),
        sha_tag(&jsha),
        jstr(&dpath.replace('\\', "/")),
        sha_tag(&run.digests[args.frames - 1]),
        jstr(&ckpath.replace('\\', "/")),
        ck_frames.join(","),
        ok,
        run.n_final,
        run.pids_issued,
        run.frame_ms_mean,
        strs_json(&run.problems),
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    std::process::exit(i32::from(!ok))
}

/// 读录制产物(journal + digest 链;fail-closed 校验)。
fn load_recorded(args: &Args) -> (Journal, Vec<String>, String) {
    let jpath = need(&args.journal, "--journal");
    let jbytes = std::fs::read(&jpath).unwrap_or_else(|e| fail(&format!("读 journal {jpath}: {e}")));
    let journal =
        Journal::deserialize(&jbytes).unwrap_or_else(|e| fail(&format!("journal 反序列化: {e}")));
    journal
        .validate_v1()
        .unwrap_or_else(|e| fail(&format!("journal v1 校验: {e}")));
    let jsha = rurix_pkg::sha256::hex_digest(&jbytes);
    let dpath = need(&args.digest, "--digest");
    let dtext =
        std::fs::read_to_string(&dpath).unwrap_or_else(|e| fail(&format!("读 digest 链 {dpath}: {e}")));
    let chain: Vec<String> = dtext.lines().map(str::to_string).filter(|l| !l.is_empty()).collect();
    if chain.len() != journal.header.frames as usize {
        fail(&format!(
            "digest 链行数 {} ≠ journal frames {}",
            chain.len(),
            journal.header.frames
        ));
    }
    for (i, l) in chain.iter().enumerate() {
        if l.len() != 64 || !l.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
            fail(&format!("digest 链第 {i} 行形态非法"));
        }
    }
    (journal, chain, jsha)
}

fn replay_leg(dev: &DevKernels, args: &Args) -> ! {
    let (journal, chain, jsha) = load_recorded(args);
    let frames = journal.header.frames as usize;
    let emits: Vec<u32> = journal.records.iter().map(|r| r.emit_count).collect();
    // 仅凭 journal 重建输入(seed/emitter/dt/emit 序列)——GPU 重仿真双跑。
    let r1 = run_chain(dev, &journal.header, Some(&emits), false, None, frames);
    let r2 = run_chain(dev, &journal.header, Some(&emits), false, None, frames);
    let first_div = first_divergence(&r1.digests, &chain);
    let bitexact = first_div == -1;
    let determinism = r1.digests == r2.digests;
    let host_ok = r1.host_parallel_bitexact && r2.host_parallel_bitexact;
    let ok = bitexact && determinism && host_ok;
    let state = if ok { "pass" } else { "fail" };
    eprintln!(
        "{TAG}: replay {state} frames={frames} record_replay_bitexact={bitexact} \
         first_divergence={first_div} double_run={determinism} host_int_bitexact={host_ok} \
         frame_ms={:.3}",
        r1.frame_ms_mean,
    );
    let mut problems = r1.problems.clone();
    problems.extend(r2.problems.iter().cloned());
    if !determinism {
        problems.push("回放双跑 digest 非位级一致".into());
    }
    let line = format!(
        "{{\"schema\":\"rurix.g35.replay_replay.v1\",\"state\":{},\"frames\":{frames},\
         \"cap\":{},\"seed\":{},\"journal_sha256\":{},\
         \"record_replay_bitexact\":{bitexact},\"first_divergence\":{first_div},\
         \"determinism_double_run\":{determinism},\
         \"digest_recorded_final\":{},\"digest_replay_final\":{},\"digest_run2_final\":{},\
         \"host_parallel_bitexact\":{host_ok},\"n_final\":{},\"pids_issued\":{},\
         \"frame_ms_mean\":{:.6},\"problems\":{},\"base_commit\":{}}}",
        jstr(state),
        journal.header.cap,
        journal.header.seed,
        sha_tag(&jsha),
        sha_tag(&chain[frames - 1]),
        sha_tag(&r1.digests[frames - 1]),
        sha_tag(&r2.digests[frames - 1]),
        r1.n_final,
        r1.pids_issued,
        r1.frame_ms_mean,
        strs_json(&problems),
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    std::process::exit(i32::from(!ok))
}

fn rollback_leg(dev: &DevKernels, args: &Args) -> ! {
    let (journal, chain, _) = load_recorded(args);
    let frames = journal.header.frames as usize;
    let (k, j) = (args.rollback_k, args.rollback_to);
    if !(k < j && j < frames) {
        fail(&format!("回滚域非法:需 k < j < frames(得 k={k} j={j} frames={frames})"));
    }
    let ckpath = args
        .checkpoint
        .clone()
        .unwrap_or_else(|| format!("{}.ckpt", need(&args.journal, "--journal")));
    let ckbytes =
        std::fs::read(&ckpath).unwrap_or_else(|e| fail(&format!("读检查点文件 {ckpath}: {e}")));
    let ckfile = CheckpointFile::deserialize(&ckbytes)
        .unwrap_or_else(|e| fail(&format!("检查点文件反序列化: {e}")));
    if ckfile.cap != journal.header.cap {
        fail(&format!("检查点 cap {} ≠ journal cap {}", ckfile.cap, journal.header.cap));
    }
    let Some(ck) = ckfile.checkpoints.iter().find(|c| c.frame as usize == k) else {
        fail(&format!("帧 {k} 无检查点(在档: {:?})",
            ckfile.checkpoints.iter().map(|c| c.frame).collect::<Vec<_>>()))
    };
    // digest 链种子 = 录制链 digest[k−1](k=0 全零种子)⇒ 恢复帧自身可对拍。
    let seed = if k == 0 { "0".repeat(64) } else { chain[k - 1].clone() };
    let emits: Vec<u32> = journal.records.iter().map(|r| r.emit_count).collect();
    let run = run_chain(dev, &journal.header, Some(&emits), false, Some((ck, &seed)), j + 1);
    let matches: Vec<bool> = (k..=j).map(|f| run.digests[f - k] == chain[f]).collect();
    let restore_match = matches[0];
    let resim_bitexact = matches.iter().all(|&m| m);
    let at_j_match = matches[j - k];
    let ok = resim_bitexact && run.host_parallel_bitexact;
    let state = if ok { "pass" } else { "fail" };
    eprintln!(
        "{TAG}: rollback {state} k={k} to={j} restore_match={restore_match} \
         resim_bitexact={resim_bitexact} at_j={at_j_match} host_int_bitexact={} frame_ms={:.3}",
        run.host_parallel_bitexact, run.frame_ms_mean,
    );
    let line = format!(
        "{{\"schema\":\"rurix.g35.replay_rollback.v1\",\"state\":{},\"k\":{k},\"to\":{j},\
         \"frames\":{frames},\"checkpoint_interval\":{},\"checkpoint_frame\":{},\
         \"restore_frame_digest_match\":{restore_match},\"resim_bitexact\":{resim_bitexact},\
         \"digest_at_j_match\":{at_j_match},\"frames_resimmed\":{},\
         \"digest_recorded_at_j\":{},\"digest_resim_at_j\":{},\
         \"host_parallel_bitexact\":{},\"frame_ms_mean\":{:.6},\
         \"problems\":{},\"base_commit\":{}}}",
        jstr(state),
        ckfile.interval,
        ck.frame,
        j - k + 1,
        sha_tag(&chain[j]),
        sha_tag(&run.digests[j - k]),
        run.host_parallel_bitexact,
        run.frame_ms_mean,
        strs_json(&run.problems),
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    std::process::exit(i32::from(!ok))
}

fn red_arm_leg(dev: &DevKernels, args: &Args) -> ! {
    let arm = args.red_arm.clone().unwrap_or_default();
    if arm != "journal-tamper" {
        fail(&format!("未知 RED 臂: {arm}(journal-tamper)"));
    }
    let (journal, chain, _) = load_recorded(args);
    let frames = journal.header.frames as usize;
    if frames <= TAMPER_FRAME {
        fail(&format!("红臂需 frames > {TAMPER_FRAME}(得 {frames})"));
    }
    // 篡改帧 32 emit_count(+1)——分歧可定位见证:帧 0..32 输入未变必须
    // 逐帧全等,帧 32 起 digest 链必异且首异帧精确 == 32。
    let mut emits: Vec<u32> = journal.records.iter().map(|r| r.emit_count).collect();
    emits[TAMPER_FRAME] += 1;
    let run = run_chain(dev, &journal.header, Some(&emits), false, None, frames);
    let first_div = first_divergence(&run.digests, &chain);
    let detected = first_div >= 0;
    let witness = first_div == TAMPER_FRAME as i64;
    let line = format!(
        "{{\"schema\":\"rurix.g35.replay_red_arm.v1\",\"arm\":\"journal-tamper\",\
         \"tamper\":\"emit_count+1\",\"tampered_frame\":{TAMPER_FRAME},\
         \"expected_divergence\":{TAMPER_FRAME},\"detected\":{detected},\
         \"first_divergence\":{first_div},\"frames\":{frames},\
         \"digest_recorded_final\":{},\"digest_tampered_final\":{},\
         \"host_parallel_bitexact\":{},\"base_commit\":{}}}",
        sha_tag(&chain[frames - 1]),
        sha_tag(&run.digests[frames - 1]),
        run.host_parallel_bitexact,
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    if !(detected && witness) {
        fail(&format!(
            "red-arm journal-tamper 失效:首异帧 {first_div} ≠ 篡改帧 {TAMPER_FRAME}(漏检或错位)"
        ));
    }
    eprintln!("{TAG}: red-arm journal-tamper 检出 — 首异帧 == {TAMPER_FRAME}(分歧可定位)");
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// main(三态:无 Vulkan → skipped_dev_env 退 0;REQUIRE_REAL 裁决归 smoke)
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();
    let Some(mode) = args.mode else {
        fail("须指定 --record / --replay / --rollback <k> --to <j> / --red-arm journal-tamper 之一")
    };
    let dev = match DevKernels::create(&args) {
        Ok(d) => d,
        Err(e) => {
            let line = format!(
                "{{\"schema\":\"rurix.g35.replay_probe.v1\",\"state\":\"skipped_dev_env\",\
                 \"reason\":{}}}",
                jstr(&e)
            );
            emit_evidence(&line, &args.evidence_out);
            std::process::exit(0);
        }
    };
    match mode {
        Mode::Record => record_leg(&dev, &args),
        Mode::Replay => replay_leg(&dev, &args),
        Mode::Rollback => rollback_leg(&dev, &args),
        Mode::RedArm => red_arm_leg(&dev, &args),
    }
}
