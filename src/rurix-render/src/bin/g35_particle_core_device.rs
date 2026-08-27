//! G35-2 粒子核心运行时 device probe harness(门 g35.wave2.particle_core;
//! RFC-0049 §4.3;G35_CONTRACT §4 契约;g27_hzb_device 三态/RED 臂同模)。
//!
//! ## 集成路径
//!
//! bin-local 全部逻辑:7 kernel(4 新 = kernels/g35_{sim,particle_compact,
//! emit,indirect_args}.rx + 3 scan = kernels/g35_scan_{seg_sum,spine,
//! seg_apply}.rx,scan 三件**只消费不修改**)经 `rurix_rt::vk::run_compute`
//! (G12/G13/G26/G27 compute 派发面同车道)逐 kernel 派发,SoA 9 流 ping-pong
//! 双组 buffers 由本 bin 持有 `Vec<Vec<u8>>` 跨 kernel 复用、帧末交换;公式面
//! 与 host 金标准 `particles/core.rs`(sim_step/compact_step/emit_step/
//! indirect_args/frame)逐字同源。**host 平行金标准 = core::frame() 逐帧推进**
//! (n_curr 由 host 维护,生产车道届时走 DispatchSpec::Indirect);中间流
//! (flags/scan_out/seg_offsets)由帧前 clone 重放同一 `sim_step` + 冻结
//! [`scan`] 三段取得(与 frame() 内部同一代码路径)。
//!
//! ## 确定性脚本(冻结夹具)
//!
//! emit_count 序列 = `min(64 + frame·17 % 192, cap − n_curr)`;EmitterDesc
//! 固定常量([`emitter`]);dt = 1/60;随机带单源 = particles/mod.rs
//! `rand_table(seed)` host 生成一次原字节上传,device 只读消费。
//!
//! ## 判据面
//!
//! ① 整数流(flags/scan_out/seg_offsets/pid/args)device↔host 逐帧 memcmp
//! **零容差位级**;② f32 流(pos_x/y/z、vel_x/y/z、age、life)逐帧 max abs
//! diff 聚合全帧 p100 —— probe 只输出 measured(`f32_max_abs_diff`),阈值
//! 判读归 smoke(milestones/g35/g35_budget.json 标定腿,threshold =
//! measured×2.0 程序产);③ pid 持久唯一(每帧无重复 + 幸存段 ⊆ 上帧集 +
//! 新发射段 == [pid_base, pid_base+emit) 精确区间);④ indirect args 零回读
//! 链(device args 8 槽 == host 平行推得 + args[7] == alive+emit 恒等式——
//! host 不读回 device 计数,只对拍验证);⑤ device 双跑位级(digest = 所有
//! 流字节 sha256 逐帧链式:B 9 流有效前缀 ‖ flags ‖ scan_out ‖ seg_offsets ‖
//! args);⑥ frame_ms 登记(device 7 dispatch 链逐帧墙钟均值;run_compute
//! 每 dispatch 重建 instance/device,该开销如实计入,measured_local 登记
//! 语义非帧率对标)。
//!
//! ## NoContraction(g14_3_lane_body.rs `spv_inject_no_contraction` 同律
//! bin-local 复制;SPV 文件 0-byte 不动)
//!
//! sim/emit 两 kernel 含 f32 乘加链(`a + b·dt` / `base + (r·2−1)·spread`),
//! 装载期注入 NoContraction 禁驱动 FMA 收缩 ⇒ 与 host 逐 op IEEE 对齐;
//! compact 纯搬运零算术、indirect_args 纯整数,不注入。f32 流仍走标定容差
//! 协议(注入为容差收敛手段非判据替代)。
//!
//! ## 三态 / RED 臂
//!
//! 无 Vulkan loader/设备 → `skipped_dev_env` JSON 退 0(非 fake pass;
//! `RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决);`--host-only`
//! 恒可;`--red-arm seed-change` = 双跑换 seed(seed / seed+1)digest 必异
//! (证明 digest 判据对流内容敏感,防镂空 digest 冒充)。
//!
//! ## 用法
//!
//! ```text
//! g35_particle_core_device --spv-sim <p> --spv-compact <p> --spv-emit <p>
//!     --spv-indirect-args <p> --spv-scan-seg-sum <p> --spv-scan-spine <p>
//!     --spv-scan-seg-apply <p> [--frames 64] [--cap 65536] [--seed 42]
//!     [--evidence-out <path>] [--report-max-diff]
//! g35_particle_core_device --red-arm seed-change --spv-... <7 件>
//! g35_particle_core_device --host-only [--frames N] [--cap N] [--seed N]
//! ```

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::time::Instant;

use rurix_render::particles::core::{EmitterDesc, FrameStats, ParticlePools, frame, sim_step};
use rurix_render::particles::{SEG, rand_table, scan};
use rurix_rt::vk;

const TAG: &str = "[g35_particle_core_device]";
const DEFAULT_FRAMES: usize = 64;
const DEFAULT_CAP: usize = 65536;
const DEFAULT_SEED: u64 = 42;
/// dt = 1/60(冻结确定性脚本)。
const DT: f32 = 1.0 / 60.0;
/// f32 对拍流名(B 组下标 0..8;pid(下标 8)走整数零容差面)。
const F32_STREAMS: [&str; 8] = [
    "pos_x", "pos_y", "pos_z", "vel_x", "vel_y", "vel_z", "age", "life",
];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

/// 冻结发射器夹具(确定性脚本;life ∈ [0.8, 1.6) ⇒ 64 帧 @1/60s 窗内必有
/// 寿命耗尽死亡,压缩腿非空转)。
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

/// 确定性发射预算(冻结脚本;host/device 同源消费)。
fn emit_schedule(f: usize, n_curr: usize, cap: usize) -> usize {
    (64 + (f * 17) % 192).min(cap - n_curr)
}

// ---------------------------------------------------------------------------
// 字节工具(g27_hzb_device 先例字面)
// ---------------------------------------------------------------------------

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
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

/// SPIR-V NoContraction 后处理(bin-local 同律复制自
/// `src/bin/g14_3_lane/g14_3_lane_body.rs::spv_inject_no_contraction`——
/// 该函数为 bin-local 非库导出,g27_hzb_device 对 hzb.rs 私有方法「bin-local
/// 同律复制」先例;SPV 文件 0-byte 不动):对全部 OpFAdd/OpFSub/OpFMul 结果
/// id 注入 `OpDecorate %id NoContraction`,禁驱动 mul+add FMA 收缩——GPU 浮点
/// 序列与 host 严格 IEEE 逐 op 对齐。
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
// JSON 出报(手写零新依赖;g27_hzb_device 同模)
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

/// 出报(stdout 恒打;--evidence-out 同步落盘,g27 emit_probe 同模)。
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

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

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
    red_arm: Option<String>,
    host_only: bool,
    report_max_diff: bool,
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
        red_arm: None,
        host_only: false,
        report_max_diff: false,
    };
    let mut it = std::env::args().skip(1);
    let next_or = |it: &mut dyn Iterator<Item = String>, k: &str| {
        it.next().unwrap_or_else(|| fail(&format!("{k} 缺值")))
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
            "--red-arm" => a.red_arm = it.next(),
            "--host-only" => a.host_only = true,
            "--report-max-diff" => a.report_max_diff = true,
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

// ---------------------------------------------------------------------------
// device 臂(bin-local;经 vk::run_compute 逐 kernel 派发)
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
        let need = |o: &Option<String>, k: &str| -> String {
            o.clone().unwrap_or_else(|| fail(&format!("缺 {k}")))
        };
        // sim/emit 注入 NoContraction(f32 乘加链 kernel;头注 §NoContraction);
        // compact 纯搬运 / indirect_args 纯整数 / scan 三件纯整数,不注入。
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

/// device 侧全缓冲(host 持有 `Vec<Vec<u8>>` 跨 kernel 复用;9 流序 =
/// 0 pos_x / 1 pos_y / 2 pos_z / 3 vel_x / 4 vel_y / 5 vel_z / 6 age /
/// 7 life / 8 pid——与 4 kernel 头注 SSBO 序严格一致)。
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

/// device 单帧 7 kernel 链(G35-P v1 帧序;buffers 下标与各 kernel 头注
/// SSBO 序严格一致);nseg == 0 时 sim/seg_sum/seg_apply/compact 零段跳过,
/// spine/indirect_args 恒跑(总和槽/args 恒需),emit == 0 跳过 emit。
#[allow(clippy::too_many_arguments)]
fn device_frame(
    dev: &DevKernels,
    st: &mut DevState,
    n: usize,
    desc: &EmitterDesc,
    pid_base: u32,
    emit_count: usize,
) -> Result<(), String> {
    let nseg = n.div_ceil(SEG);
    let take = std::mem::take::<Vec<u8>>;
    // 1. sim(10 SSBO:params/pos3/vel3/age/life/flags)。
    if nseg > 0 {
        let params = [n as f32, nseg as f32, DT, desc.gravity_y, 0.0, 0.0, 0.0, 0.0];
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

/// host 池 f32 流按下标取(0..8 = F32_STREAMS 序)。
fn host_stream(p: &ParticlePools, k: usize) -> &[f32] {
    match k {
        0 => &p.pos_x,
        1 => &p.pos_y,
        2 => &p.pos_z,
        3 => &p.vel_x,
        4 => &p.vel_y,
        5 => &p.vel_z,
        6 => &p.age,
        7 => &p.life,
        _ => unreachable!("f32 流下标 0..8"),
    }
}

// ---------------------------------------------------------------------------
// 全链单跑(host 平行金标准逐帧对拍 + 链式 digest)
// ---------------------------------------------------------------------------

struct ChainReport {
    integer_bitexact: bool,
    f32_stream_max: [f32; 8],
    pid_unique: bool,
    pid_survivor_subset: bool,
    pid_emit_range_exact: bool,
    pids_issued: u32,
    args_match: bool,
    args_identity: bool,
    args_last: [u32; 8],
    digest: String,
    frame_ms_mean: f64,
    n_final: usize,
    alive_final: u32,
    problems: Vec<String>,
}

fn run_chain(dev: &DevKernels, seed: u64, frames: usize, cap: usize) -> ChainReport {
    let desc = emitter();
    let table = rand_table(seed);
    let mut ha = ParticlePools::with_capacity(cap);
    let mut hb = ParticlePools::with_capacity(cap);
    let mut st = DevState::new(cap, bytes_f32(&table));
    let mut pid_base = 0u32;
    let mut prev_pids: HashSet<u32> = HashSet::new();
    let mut r = ChainReport {
        integer_bitexact: true,
        f32_stream_max: [0.0; 8],
        pid_unique: true,
        pid_survivor_subset: true,
        pid_emit_range_exact: true,
        pids_issued: 0,
        args_match: true,
        args_identity: true,
        args_last: [0; 8],
        digest: "0".repeat(64),
        frame_ms_mean: 0.0,
        n_final: 0,
        alive_final: 0,
        problems: Vec::new(),
    };
    let mut ms_total = 0.0f64;
    let problem = |problems: &mut Vec<String>, msg: String| {
        if problems.len() < 16 {
            problems.push(msg);
        }
    };
    for f in 0..frames {
        let n = ha.n; // device n_curr 由 host 平行金标准维护(零回读)
        let nseg = n.div_ceil(SEG);
        let emit = emit_schedule(f, n, cap);
        // ── host 金标准 frame() 平行推进;中间流经帧前 clone 重放同一
        //    sim_step + 冻结 scan 三段取得(frame() 内部同一代码路径)──
        let mut pre = ha.clone();
        let stats: FrameStats = frame(&mut ha, &mut hb, &desc, &table, DT, pid_base, emit);
        let hflags = sim_step(&mut pre, DT, desc.gravity_y);
        let hsums = scan::seg_sums(&hflags, nseg);
        let hspine = scan::spine(&hsums);
        let hscan = scan::seg_apply(&hflags, &hspine, nseg);
        // ── device 7 kernel 链(墙钟计时)──
        let t0 = Instant::now();
        device_frame(dev, &mut st, n, &desc, pid_base, emit)
            .unwrap_or_else(|e| fail(&format!("帧 {f}: {e}")));
        ms_total += t0.elapsed().as_secs_f64() * 1000.0;
        // ── 整数流零容差对拍(flags/scan_out/seg_offsets/pid/args)──
        let dev_flags = read_u32(&st.flags);
        if dev_flags[..n] != hflags[..] {
            r.integer_bitexact = false;
            problem(&mut r.problems, format!("帧 {f}: flags 非位级"));
        }
        let dev_scan = read_u32(&st.scan_out);
        if dev_scan[..n] != hscan[..] {
            r.integer_bitexact = false;
            problem(&mut r.problems, format!("帧 {f}: scan_out 非位级"));
        }
        let dev_spine = read_u32(&st.seg_offsets);
        if dev_spine[..nseg + 1] != hspine[..] {
            r.integer_bitexact = false;
            problem(&mut r.problems, format!("帧 {f}: seg_offsets 非位级"));
        }
        let dev_args_v = read_u32(&st.args);
        let mut dev_args = [0u32; 8];
        dev_args.copy_from_slice(&dev_args_v[..8]);
        if dev_args != stats.args {
            r.integer_bitexact = false;
            r.args_match = false;
            problem(
                &mut r.problems,
                format!("帧 {f}: args {dev_args:?} ≠ host {:?}", stats.args),
            );
        }
        if dev_args[7] != stats.alive_total + emit as u32 {
            r.args_identity = false;
            problem(
                &mut r.problems,
                format!("帧 {f}: args[7]={} ≠ alive+emit={}", dev_args[7], stats.alive_total + emit as u32),
            );
        }
        let n_next = stats.n_next;
        let dev_pid = read_u32(&st.b[8]);
        if dev_pid[..n_next] != hb.pid[..n_next] {
            r.integer_bitexact = false;
            problem(&mut r.problems, format!("帧 {f}: pid 流非位级"));
        }
        // ── f32 流 max abs diff(全帧 p100 聚合;probe 只测不判)──
        for k in 0..8 {
            let dev_f = read_f32(&st.b[k]);
            let host_f = host_stream(&hb, k);
            for i in 0..n_next {
                let mut d = (dev_f[i] - host_f[i]).abs();
                if !d.is_finite() {
                    d = f32::INFINITY;
                    problem(
                        &mut r.problems,
                        format!("帧 {f}: {} 流出现非有限差(i={i})", F32_STREAMS[k]),
                    );
                }
                if d > r.f32_stream_max[k] {
                    r.f32_stream_max[k] = d;
                }
            }
        }
        // ── pid 持久性(device 流直判:唯一 + 幸存 ⊆ 上帧 + 新段精确区间)──
        let mut cur_pids: HashSet<u32> = HashSet::with_capacity(n_next);
        for &p in &dev_pid[..n_next] {
            if !cur_pids.insert(p) {
                r.pid_unique = false;
                problem(&mut r.problems, format!("帧 {f}: pid {p} 重复"));
            }
        }
        let alive = stats.alive_total as usize;
        if !dev_pid[..alive].iter().all(|p| prev_pids.contains(p)) {
            r.pid_survivor_subset = false;
            problem(&mut r.problems, format!("帧 {f}: 幸存段非上帧子集"));
        }
        let range_ok = dev_pid[alive..n_next]
            .iter()
            .enumerate()
            .all(|(j, &p)| p == pid_base + j as u32);
        if !range_ok {
            r.pid_emit_range_exact = false;
            problem(&mut r.problems, format!("帧 {f}: 新发射段非精确区间"));
        }
        prev_pids = cur_pids;
        // ── 链式 digest(B 9 流有效前缀 ‖ flags ‖ scan_out ‖ seg_offsets ‖
        //    args;sha256(prev_hex ‖ frame_bytes))──
        let mut trace: Vec<u8> = Vec::with_capacity(r.digest.len() + n_next * 36 + n * 8 + 64);
        trace.extend_from_slice(r.digest.as_bytes());
        for k in 0..9 {
            trace.extend_from_slice(&st.b[k][..n_next * 4]);
        }
        trace.extend_from_slice(&st.flags[..n * 4]);
        trace.extend_from_slice(&st.scan_out[..n * 4]);
        trace.extend_from_slice(&st.seg_offsets[..(nseg + 1) * 4]);
        trace.extend_from_slice(&st.args);
        r.digest = rurix_pkg::sha256::hex_digest(&trace);
        // ── 帧末交换(读 A 写 B;n_curr_next = alive+emit host 平行推得)──
        pid_base += emit as u32;
        r.alive_final = stats.alive_total;
        r.n_final = n_next;
        std::mem::swap(&mut ha, &mut hb);
        std::mem::swap(&mut st.a, &mut st.b);
        r.args_last = dev_args;
    }
    r.pids_issued = pid_base;
    r.frame_ms_mean = ms_total / frames as f64;
    r
}

impl ChainReport {
    fn hard_pass(&self) -> bool {
        self.integer_bitexact
            && self.pid_unique
            && self.pid_survivor_subset
            && self.pid_emit_range_exact
            && self.args_match
            && self.args_identity
    }

    fn stream_max_json(&self) -> String {
        let inner: Vec<String> = F32_STREAMS
            .iter()
            .zip(self.f32_stream_max.iter())
            .map(|(name, v)| format!("{}:{:e}", jstr(name), v))
            .collect();
        format!("{{{}}}", inner.join(","))
    }
}

// ---------------------------------------------------------------------------
// host-only 腿(host 金标准链恒可跑:守恒/args 恒等式/pid 唯一)
// ---------------------------------------------------------------------------

fn host_only_leg(args: &Args) -> ! {
    let desc = emitter();
    let table = rand_table(args.seed);
    let mut a = ParticlePools::with_capacity(args.cap);
    let mut b = ParticlePools::with_capacity(args.cap);
    let mut pid_base = 0u32;
    let mut ok = true;
    let mut deaths = 0u64;
    for f in 0..args.frames {
        let emit = emit_schedule(f, a.n, args.cap);
        let mut replay = a.clone();
        let flags = sim_step(&mut replay, DT, desc.gravity_y);
        let alive: u32 = flags.iter().sum();
        deaths += (a.n as u64) - u64::from(alive);
        let stats = frame(&mut a, &mut b, &desc, &table, DT, pid_base, emit);
        let uniq: HashSet<u32> = b.pid[..b.n].iter().copied().collect();
        if stats.alive_total != alive
            || stats.n_next != alive as usize + emit
            || stats.args[7] != stats.alive_total + emit as u32
            || uniq.len() != b.n
        {
            ok = false;
            eprintln!("{TAG}: host 帧 {f} 不变量破(alive={alive} stats={stats:?})");
        }
        pid_base += emit as u32;
        std::mem::swap(&mut a, &mut b);
    }
    let state = if ok { "pass" } else { "fail" };
    let line = format!(
        "{{\"schema\":\"rurix.g35.particle_core_host.v1\",\"mode\":\"host-only\",\"state\":{},\
         \"frames\":{},\"cap\":{},\"seed\":{},\"n_final\":{},\"pids_issued\":{},\"deaths\":{deaths},\
         \"base_commit\":{}}}",
        jstr(state),
        args.frames,
        args.cap,
        args.seed,
        a.n,
        pid_base,
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    std::process::exit(i32::from(!ok))
}

// ---------------------------------------------------------------------------
// main(默认 = 全档验证:双跑同 seed;--red-arm seed-change = 双跑异 seed)
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    if args.host_only {
        host_only_leg(&args);
    }

    let dev = match DevKernels::create(&args) {
        Ok(d) => d,
        Err(e) => {
            let line = format!(
                "{{\"schema\":\"rurix.g35.particle_core_probe.v1\",\"state\":\"skipped_dev_env\",\
                 \"reason\":{}}}",
                jstr(&e)
            );
            emit_evidence(&line, &args.evidence_out);
            std::process::exit(0);
        }
    };

    if let Some(arm) = &args.red_arm {
        if arm != "seed-change" {
            fail(&format!("未知 RED 臂: {arm}(seed-change)"));
        }
        // RED 臂:换 seed 双跑 digest 必异(digest 判据对流内容敏感性证明;
        // emit_count 序列与 seed 无关,差异只可能经随机带进流内容)。
        let g = run_chain(&dev, args.seed, args.frames, args.cap);
        let r = run_chain(&dev, args.seed + 1, args.frames, args.cap);
        let detected = g.digest != r.digest;
        let line = format!(
            "{{\"schema\":\"rurix.g35.particle_core_red_arm.v1\",\"arm\":\"seed-change\",\
             \"detected\":{detected},\"seed_green\":{},\"seed_red\":{},\
             \"digest_green\":{},\"digest_red\":{}}}",
            args.seed,
            args.seed + 1,
            jstr(&format!("sha256:{}", g.digest)),
            jstr(&format!("sha256:{}", r.digest)),
        );
        emit_evidence(&line, &args.evidence_out);
        if !detected {
            fail("red-arm seed-change 失效(漏检):换 seed 后 digest 未变");
        }
        eprintln!("{TAG}: red-arm seed-change 检出 — digest 已异");
        std::process::exit(0);
    }

    // ── 全档验证:双跑同 seed(判据 ⑤ device 双跑位级)+ 逐帧对拍(①③④)──
    let a = run_chain(&dev, args.seed, args.frames, args.cap);
    let b = run_chain(&dev, args.seed, args.frames, args.cap);
    let determinism = a.digest == b.digest;
    let f32_p100 = a.f32_stream_max.iter().copied().fold(0.0f32, f32::max);
    let pid_ok = a.pid_unique && a.pid_survivor_subset && a.pid_emit_range_exact;
    let args_ok = a.args_match && a.args_identity;
    let state = if a.hard_pass() && determinism {
        "pass"
    } else {
        "fail"
    };
    if args.report_max_diff {
        println!("f32_max_abs_diff={f32_p100:e}");
    }
    eprintln!(
        "{TAG}: {} frames={} cap={} seed={} int_bitexact={} f32_p100={:e} pid={} args={} \
         double_run={} n_final={} alive_final={} frame_ms={:.3}",
        state,
        args.frames,
        args.cap,
        args.seed,
        a.integer_bitexact,
        f32_p100,
        pid_ok,
        args_ok,
        determinism,
        a.n_final,
        a.alive_final,
        a.frame_ms_mean,
    );
    let mut problems = a.problems.clone();
    if !determinism {
        problems.push("device 双跑 digest 非位级一致".into());
    }
    let line = format!(
        "{{\"schema\":\"rurix.g35.particle_core_probe.v1\",\"state\":{},\
         \"frames\":{},\"cap\":{},\"seed\":{},\"dt\":{:e},\
         \"emit_schedule\":\"min(64 + frame*17 % 192, cap - n_curr)\",\
         \"integer_streams\":[\"flags\",\"scan_out\",\"seg_offsets\",\"pid\",\"args\"],\
         \"integer_streams_bitexact\":{},\
         \"f32_max_abs_diff\":{:e},\"f32_stream_max\":{},\
         \"pid_persistent_unique\":{},\"pid_unique\":{},\"pid_survivor_subset\":{},\
         \"pid_emit_range_exact\":{},\"pids_issued\":{},\
         \"indirect_args_device_match\":{},\"args_match\":{},\"args_identity\":{},\
         \"args_last\":[{}],\
         \"determinism_double_run\":{},\"digest_a\":{},\"digest_b\":{},\
         \"frame_ms_mean\":{:.6},\"n_final\":{},\"alive_final\":{},\
         \"nocontraction_injected\":[\"g35_sim\",\"g35_emit\"],\
         \"problems\":{},\"base_commit\":{}}}",
        jstr(state),
        args.frames,
        args.cap,
        args.seed,
        DT,
        a.integer_bitexact,
        f32_p100,
        a.stream_max_json(),
        pid_ok,
        a.pid_unique,
        a.pid_survivor_subset,
        a.pid_emit_range_exact,
        a.pids_issued,
        args_ok,
        a.args_match,
        a.args_identity,
        a.args_last
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(","),
        determinism,
        jstr(&format!("sha256:{}", a.digest)),
        jstr(&format!("sha256:{}", b.digest)),
        a.frame_ms_mean,
        a.n_final,
        a.alive_final,
        strs_json(&problems),
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    if state != "pass" {
        std::process::exit(1);
    }
}
