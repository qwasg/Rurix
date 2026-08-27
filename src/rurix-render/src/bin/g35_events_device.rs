//! G35-6 事件/数据通道 + particle_view 双向桥 device probe harness(门
//! g35.wave6.events;RFC-0049 §4.9/评审 F15 修订后基线;G35_CONTRACT §4
//! 契约;g35_particle_core_device 三态/RED 臂同模)。
//!
//! ## 集成路径
//!
//! bin-local 全部逻辑:9 kernel(2 新 = kernels/g35_event_{collect,spawn}.rx
//! + 7 W2 消费面 = g35_{sim,particle_compact,emit,indirect_args}.rx + 3 scan,
//! **W2 七件只消费不修改**)经 `rurix_rt::vk::run_compute` 逐 kernel 派发,
//! SoA 9 流 ping-pong 双组 + 事件缓冲(GPU 死亡事件 meta/payload/count 跨帧
//! 持有)由本 bin 持有 `Vec<Vec<u8>>` 跨 kernel 复用、帧末交换;公式面与
//! host 金标准 `particles/events.rs`(event_collect_step/event_spawn_step/
//! event_frame)逐字同源,host 平行金标准 = `event_frame()` 逐帧推进。
//!
//! ## G35-6 帧序(冻结;13 dispatch 上界/帧)
//!
//! 1 sim → 2..4 存活稳定 scan 三段 → 5 compact → 6 emit(脚本发射)→
//! 7 **event_spawn**(当帧 host 队列 + **上一帧** GPU 死亡事件缓冲双源合并,
//! 次序冻结 host 先;spawn 先于 collect 读取事件缓冲 = 单缓冲跨帧协议)→
//! 8 **event_collect 相 0**(death_flags = 1 − flags)→ 9..11 死亡稳定
//! scan 三段(**复用 W2 三 scan kernel**)→ 12 **event_collect 相 1**
//! (scatter + ev_count 计数,溢出如实登记)→ 13 indirect_args
//! (emit_count 面 = scripted + accepted,host 平行推得)。
//!
//! ## 零回读要件
//!
//! device 端 alive/gpu 事件计数一律 SSBO 直读(seg_offsets 总和槽 /
//! ev_count);host 只以平行金标准推进 n_curr/pid_base/emit_effective,
//! **不读回任何 device 计数入链**(readback 仅用于对拍验证);spawn
//! dispatch = 2·EVENT_CAP 保守上界 + 界内守卫。
//!
//! ## 确定性脚本(冻结夹具)
//!
//! dt = 1/15;帧 0 脚本爆发 min(16000, cap)(死亡窗 [0.8,1.6)s ⇒ 帧
//! 13..24 每帧 ≈ 1333 死亡 > EVENT_CAP,死亡溢出钳制腿非空转);此后
//! scripted = min(32 + f·11 % 96, cap − n_curr);host 合成事件(方向 B
//! v1 演示域,不真接物理世界)= 帧 12 push 1200 / 帧 30 push 1100(队列
//! 溢出裁剪腿)/ 其余 (f·7)%5,payload 由 host Pcg32 单源派生;发射随机
//! 走 rand_table 单源。
//!
//! ## 判据面
//!
//! ① 整数流(flags/scan_out/seg_offsets/death_flags/death_scan/
//! death_offsets/ev_meta/ev_count/src_meta/spawn_counts/pid/args)
//! device↔host 逐帧 memcmp **零容差位级**;② f32 流(B 8 流 + ev_payload
//! + 发射段)max abs diff 聚合 p100 —— probe 只输出 measured,阈值判读归
//! smoke(milestones/g35/g35_budget.json g35.events.f32_parity_p100 标定
//! 腿);③ 溢出裁剪稳定(同帧事件集正/逆序装配 trim 位级同果 + pushed =
//! kept + overflow 如实登记);④ 双源发射(src_meta 消费见证位级 =
//! host 先 GPU 后次序 + spawn_counts == host 平行推得 + 发射段 pid 精确
//! 区间);⑤ GPU 二次发射零回读(ev_count == host〔kept/total〕+
//! secondary 帧数样本量门);⑥ particle_view 桥 roundtrip(末帧 device
//! 九流 readback → GpuParticleSnapshot → pid 定址读 == readback 原值位级;
//! 物理侧 ExternalParticlesAdapter 同判据归 smoke 腿 cargo test);
//! ⑦ device 双跑位级(链式 digest);⑧ frame_ms 登记(measured_local,
//! 含 run_compute 逐 dispatch 会话重建开销,非帧率对标)。
//!
//! ## NoContraction(g35_particle_core_device 同律 bin-local 复制)
//!
//! sim/emit/event_spawn 三 kernel 含 f32 乘加链,装载期注入禁驱动 FMA
//! 收缩;compact/scan/indirect_args/event_collect 纯整数或纯搬运不注入。
//!
//! ## 三态 / RED 臂
//!
//! 无 Vulkan loader/设备 → `skipped_dev_env` JSON 退 0;`--host-only`
//! 恒可;`--red-arm payload-tamper` = 帧 12 上传 host 事件 payload 词 0
//! 篡改 +1.0(device 侧上传件,host 金标准不动)⇒ 与绿链 digest 必异
//! (事件 payload 篡改必检出,防镂空 digest 冒充)。
//!
//! ## 用法
//!
//! ```text
//! g35_events_device --spv-sim <p> --spv-compact <p> --spv-emit <p>
//!     --spv-indirect-args <p> --spv-scan-seg-sum <p> --spv-scan-spine <p>
//!     --spv-scan-seg-apply <p> --spv-event-collect <p> --spv-event-spawn <p>
//!     [--frames 64] [--cap 16384] [--seed 42] [--evidence-out <path>]
//!     [--report-max-diff]
//! g35_events_device --red-arm payload-tamper --spv-... <9 件>
//! g35_events_device --host-only [--frames N] [--cap N] [--seed N]
//! ```

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::time::Instant;

use rurix_render::particles::core::EmitterDesc;
use rurix_render::particles::events::{
    EVENT_CAP, EVENT_KIND_HOST, EVENT_META_WORDS, EVENT_PAYLOAD_WORDS, EventQueue,
    EventSpawnParams, GpuParticleSnapshot, ParticleEvent, event_frame,
};
use rurix_render::particles::{Pcg32, SEG, rand_table, scan};
use rurix_rt::vk;

const TAG: &str = "[g35_events_device]";
const DEFAULT_FRAMES: usize = 64;
const DEFAULT_CAP: usize = 16384;
const DEFAULT_SEED: u64 = 42;
/// dt = 1/15(冻结确定性脚本;死亡窗压进 64 帧 + 单帧死亡 > EVENT_CAP)。
const DT: f32 = 1.0 / 15.0;
/// 帧 0 脚本爆发(死亡溢出腿夹具)。
const BURST: usize = 16000;
/// RED 臂 payload 篡改帧(host 队列溢出帧,事件必非空)。
const TAMPER_FRAME: usize = 12;
/// f32 对拍流名(0..8 = B 组 8 流;8 = GPU 死亡事件 payload)。
const F32_STREAMS: [&str; 9] = [
    "pos_x", "pos_y", "pos_z", "vel_x", "vel_y", "vel_z", "age", "life", "ev_payload",
];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

/// 冻结发射器夹具(life ∈ [0.8, 1.6) ⇒ dt=1/15 下爆发死亡窗 = 帧 13..24)。
fn emitter() -> EmitterDesc {
    EmitterDesc {
        pos: [0.0, 2.0, -0.5],
        spread: [0.6, 0.3, 0.6],
        vel_base: [0.0, 2.5, 0.0],
        vel_spread: [1.25, 0.5, 1.25],
        life_base: 1.6,
        gravity_y: -9.8,
    }
}

/// 冻结事件发射参数夹具(事件粒子 life ∈ [0.6, 1.2) ⇒ 二次死亡链继续)。
fn spawn_params() -> EventSpawnParams {
    EventSpawnParams {
        spread: 0.05,
        vel_spread: 0.5,
        life_base: 1.2,
    }
}

/// 确定性脚本发射预算(帧 0 爆发;此后小流量,恒钳 cap − n_curr)。
fn emit_schedule(f: usize, n_curr: usize, cap: usize) -> usize {
    if f == 0 {
        BURST.min(cap - n_curr)
    } else {
        (32 + (f * 11) % 96).min(cap - n_curr)
    }
}

/// host 合成事件计数脚本(帧 12/30 = 队列溢出裁剪腿)。
fn host_event_count(f: usize) -> usize {
    match f {
        12 => 1200,
        30 => 1100,
        _ => (f * 7) % 5,
    }
}

/// host 合成事件原始序列(方向 B v1 演示域;payload 由 host Pcg32 单源
/// 派生,producer_id/slot 确定性编号)。
fn host_events_raw(f: usize, seed: u64) -> Vec<ParticleEvent> {
    let mut rng = Pcg32::new(seed.wrapping_add(f as u64 * 1_000_003), 91);
    (0..host_event_count(f))
        .map(|k| ParticleEvent {
            producer_id: 1_000_000 + (f as u32) * 4096 + k as u32,
            slot: k as u32,
            kind: EVENT_KIND_HOST,
            payload: [
                rng.next_f32() * 8.0 - 4.0,
                rng.next_f32() * 8.0 - 4.0,
                rng.next_f32() * 8.0 - 4.0,
                rng.next_f32() * 4.0 - 2.0,
                rng.next_f32() * 4.0 - 2.0,
            ],
        })
        .collect()
}

/// 原始序列 → 装配(正/逆序 push + trim;溢出裁剪稳定性对拍消费)。
fn queue_from(raw: &[ParticleEvent], reverse: bool) -> EventQueue {
    let mut q = EventQueue::new();
    if reverse {
        for ev in raw.iter().rev() {
            q.push(*ev);
        }
    } else {
        for ev in raw {
            q.push(*ev);
        }
    }
    q.trim();
    q
}

fn queues_bits_eq(a: &EventQueue, b: &EventQueue) -> bool {
    a.len() == b.len()
        && a.overflow_count() == b.overflow_count()
        && a.events().iter().zip(b.events()).all(|(x, y)| x.bits_eq(y))
}

// ---------------------------------------------------------------------------
// 字节工具(g35_particle_core_device 先例字面)
// ---------------------------------------------------------------------------

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_u32(v: &[u32]) -> Vec<u8> {
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

/// SPIR-V NoContraction 后处理(g14_3_lane_body/g35_particle_core_device
/// bin-local 同律复制;SPV 文件 0-byte 不动):对全部 OpFAdd/OpFSub/OpFMul
/// 结果 id 注入 `OpDecorate %id NoContraction`,禁驱动 mul+add FMA 收缩。
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
            71 if first_decorate.is_none() => first_decorate = Some(i),
            19..=39 if first_type.is_none() => first_type = Some(i),
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
        out.push(71u32 | (3 << 16));
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
    spv_event_collect: Option<String>,
    spv_event_spawn: Option<String>,
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
        spv_event_collect: None,
        spv_event_spawn: None,
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
            "--spv-event-collect" => a.spv_event_collect = it.next(),
            "--spv-event-spawn" => a.spv_event_spawn = it.next(),
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
    if a.frames < 31 {
        fail("--frames 必须 ≥ 31(冻结脚本溢出帧 12/30 + 死亡窗覆盖)");
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
    spv_collect: Vec<u32>,
    entry_collect: String,
    spv_spawn: Vec<u32>,
    entry_spawn: String,
}

impl DevKernels {
    fn create(args: &Args) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let need = |o: &Option<String>, k: &str| -> String {
            o.clone().unwrap_or_else(|| fail(&format!("缺 {k}")))
        };
        // sim/emit/event_spawn 注入 NoContraction(f32 乘加链;头注
        // §NoContraction);compact/scan/indirect_args/event_collect 纯整数
        // 或纯搬运,不注入。
        let spv_sim = spv_inject_no_contraction(&load_spv(&need(&args.spv_sim, "--spv-sim")));
        let spv_compact = load_spv(&need(&args.spv_compact, "--spv-compact"));
        let spv_emit = spv_inject_no_contraction(&load_spv(&need(&args.spv_emit, "--spv-emit")));
        let spv_args = load_spv(&need(&args.spv_indirect_args, "--spv-indirect-args"));
        let spv_seg_sum = load_spv(&need(&args.spv_scan_seg_sum, "--spv-scan-seg-sum"));
        let spv_spine = load_spv(&need(&args.spv_scan_spine, "--spv-scan-spine"));
        let spv_seg_apply = load_spv(&need(&args.spv_scan_seg_apply, "--spv-scan-seg-apply"));
        let spv_collect = load_spv(&need(&args.spv_event_collect, "--spv-event-collect"));
        let spv_spawn =
            spv_inject_no_contraction(&load_spv(&need(&args.spv_event_spawn, "--spv-event-spawn")));
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
            entry_collect: entry(&spv_collect, "event_collect")?,
            entry_spawn: entry(&spv_spawn, "event_spawn")?,
            spv_sim,
            spv_compact,
            spv_emit,
            spv_args,
            spv_seg_sum,
            spv_spine,
            spv_seg_apply,
            spv_collect,
            spv_spawn,
        })
    }
}

/// device 侧全缓冲(9 流序 = 0 pos_x / 1 pos_y / 2 pos_z / 3 vel_x /
/// 4 vel_y / 5 vel_z / 6 age / 7 life / 8 pid;事件缓冲 ev_* 跨帧持有 =
/// 单缓冲跨帧协议,spawn 先读 collect 后写)。
struct DevState {
    a: Vec<Vec<u8>>,
    b: Vec<Vec<u8>>,
    flags: Vec<u8>,
    scan_out: Vec<u8>,
    seg_sums: Vec<u8>,
    seg_offsets: Vec<u8>,
    death_flags: Vec<u8>,
    death_scan: Vec<u8>,
    death_seg_sums: Vec<u8>,
    death_offsets: Vec<u8>,
    ev_meta: Vec<u8>,
    ev_payload: Vec<u8>,
    ev_count: Vec<u8>,
    src_meta: Vec<u8>,
    spawn_counts: Vec<u8>,
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
            death_flags: vec![0u8; cap * 4],
            death_scan: vec![0u8; cap * 4],
            death_seg_sums: vec![0u8; nseg_cap * 4],
            death_offsets: vec![0u8; (nseg_cap + 1) * 4],
            ev_meta: vec![0u8; EVENT_CAP * EVENT_META_WORDS * 4],
            ev_payload: vec![0u8; EVENT_CAP * EVENT_PAYLOAD_WORDS * 4],
            ev_count: vec![0u8; 2 * 4],
            src_meta: vec![0u8; 2 * EVENT_CAP * EVENT_META_WORDS * 4],
            spawn_counts: vec![0u8; 4 * 4],
            args: vec![0u8; 8 * 4],
            rand: rand_bytes,
        }
    }
}

/// device 单帧 9 kernel/13 dispatch 链(G35-6 帧序冻结;buffers 下标与各
/// kernel 头注 SSBO 序严格一致);nseg == 0 时段级 dispatch 跳过,双 spine
/// 与 collect 相 1 及 spawn/args 恒跑(计数/总和槽恒需)。
#[allow(clippy::too_many_arguments)]
fn device_frame(
    dev: &DevKernels,
    st: &mut DevState,
    n: usize,
    desc: &EmitterDesc,
    sp: &EventSpawnParams,
    pid_base: u32,
    scripted: usize,
    host_count: usize,
    host_meta_bytes: &[u8],
    host_payload_bytes: &[u8],
    emit_effective: u32,
    cap: usize,
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
    let scan_params = bytes_f32(&[n as f32, nseg as f32, 0.0, 0.0]);
    // 2. 存活 scan_seg_sum(values/params/seg_sums)。
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
    // 3. 存活 scan_spine(恒跑——nseg=0 时写总和槽 0)。
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
    // 4. 存活 scan_seg_apply(values/seg_offsets/params/out_scan)。
    if nseg > 0 {
        let mut bufs = vec![
            take(&mut st.flags),
            take(&mut st.seg_offsets),
            scan_params.clone(),
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
    // 5. particle_compact(21 SSBO:params/flags/scan_out/A×9/B×9)。
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
    // 6. emit 脚本发射(12 SSBO;alive_slot = params[15] = nseg 零回读)。
    if scripted > 0 {
        let d = desc;
        let params = [
            scripted as f32,
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
            scripted as u32,
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
    // 7. event_spawn(19 SSBO;双源合并,上一帧事件缓冲先读后被 collect
    //    覆写 = 单缓冲跨帧协议;dispatch 2·EVENT_CAP 保守上界零回读)。
    {
        let params = [
            host_count as f32,
            scripted as f32,
            (pid_base as usize + scripted) as f32,
            nseg as f32,
            cap as f32,
            sp.spread,
            sp.vel_spread,
            sp.life_base,
            EVENT_CAP as f32,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(19);
        bufs.push(bytes_f32(&params));
        bufs.push(take(&mut st.seg_offsets));
        bufs.push(take(&mut st.ev_count));
        bufs.push(host_meta_bytes.to_vec());
        bufs.push(host_payload_bytes.to_vec());
        bufs.push(take(&mut st.ev_meta));
        bufs.push(take(&mut st.ev_payload));
        bufs.push(take(&mut st.rand));
        for k in 0..9 {
            bufs.push(take(&mut st.b[k]));
        }
        bufs.push(take(&mut st.src_meta));
        bufs.push(take(&mut st.spawn_counts));
        vk::run_compute(&dev.spv_spawn, &dev.entry_spawn, &mut bufs, &[], [
            2 * EVENT_CAP as u32,
            1,
            1,
        ])
        .map_err(|e| format!("event_spawn dispatch: {e}"))?;
        st.seg_offsets = take(&mut bufs[1]);
        st.ev_count = take(&mut bufs[2]);
        st.ev_meta = take(&mut bufs[5]);
        st.ev_payload = take(&mut bufs[6]);
        st.rand = take(&mut bufs[7]);
        for k in 0..9 {
            st.b[k] = take(&mut bufs[8 + k]);
        }
        st.src_meta = take(&mut bufs[17]);
        st.spawn_counts = take(&mut bufs[18]);
    }
    // 8/12 共享的 collect 派发闭包(14 SSBO,相位经 params[2])。
    let run_collect = |st: &mut DevState, phase: f32, groups: u32| -> Result<(), String> {
        let params = bytes_f32(&[
            n as f32,
            nseg as f32,
            phase,
            EVENT_CAP as f32,
            0.0,
            0.0,
            0.0,
            0.0,
        ]);
        let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(14);
        bufs.push(params);
        bufs.push(take(&mut st.flags));
        bufs.push(take(&mut st.death_flags));
        bufs.push(take(&mut st.death_scan));
        bufs.push(take(&mut st.death_offsets));
        bufs.push(take(&mut st.a[0]));
        bufs.push(take(&mut st.a[1]));
        bufs.push(take(&mut st.a[2]));
        bufs.push(take(&mut st.a[3]));
        bufs.push(take(&mut st.a[4]));
        bufs.push(take(&mut st.a[8]));
        bufs.push(take(&mut st.ev_meta));
        bufs.push(take(&mut st.ev_payload));
        bufs.push(take(&mut st.ev_count));
        vk::run_compute(&dev.spv_collect, &dev.entry_collect, &mut bufs, &[], [
            groups, 1, 1,
        ])
        .map_err(|e| format!("event_collect(phase {phase}) dispatch: {e}"))?;
        st.flags = take(&mut bufs[1]);
        st.death_flags = take(&mut bufs[2]);
        st.death_scan = take(&mut bufs[3]);
        st.death_offsets = take(&mut bufs[4]);
        st.a[0] = take(&mut bufs[5]);
        st.a[1] = take(&mut bufs[6]);
        st.a[2] = take(&mut bufs[7]);
        st.a[3] = take(&mut bufs[8]);
        st.a[4] = take(&mut bufs[9]);
        st.a[8] = take(&mut bufs[10]);
        st.ev_meta = take(&mut bufs[11]);
        st.ev_payload = take(&mut bufs[12]);
        st.ev_count = take(&mut bufs[13]);
        Ok(())
    };
    // 8. event_collect 相 0(death_flags 派生)。
    if nseg > 0 {
        run_collect(st, 0.0, nseg as u32)?;
    }
    // 9. 死亡 scan_seg_sum(复用 W2 kernel,消费不修改)。
    if nseg > 0 {
        let mut bufs = vec![
            take(&mut st.death_flags),
            scan_params.clone(),
            take(&mut st.death_seg_sums),
        ];
        vk::run_compute(&dev.spv_seg_sum, &dev.entry_seg_sum, &mut bufs, &[], [
            nseg as u32,
            1,
            1,
        ])
        .map_err(|e| format!("death scan_seg_sum dispatch: {e}"))?;
        st.death_flags = take(&mut bufs[0]);
        st.death_seg_sums = take(&mut bufs[2]);
    }
    // 10. 死亡 scan_spine(恒跑——空池帧总和槽 0,collect 相 1 计数归零)。
    {
        let mut bufs = vec![
            take(&mut st.death_seg_sums),
            scan_params.clone(),
            take(&mut st.death_offsets),
        ];
        vk::run_compute(&dev.spv_spine, &dev.entry_spine, &mut bufs, &[], [1, 1, 1])
            .map_err(|e| format!("death scan_spine dispatch: {e}"))?;
        st.death_seg_sums = take(&mut bufs[0]);
        st.death_offsets = take(&mut bufs[2]);
    }
    // 11. 死亡 scan_seg_apply。
    if nseg > 0 {
        let mut bufs = vec![
            take(&mut st.death_flags),
            take(&mut st.death_offsets),
            scan_params,
            take(&mut st.death_scan),
        ];
        vk::run_compute(&dev.spv_seg_apply, &dev.entry_seg_apply, &mut bufs, &[], [
            nseg as u32,
            1,
            1,
        ])
        .map_err(|e| format!("death scan_seg_apply dispatch: {e}"))?;
        st.death_flags = take(&mut bufs[0]);
        st.death_offsets = take(&mut bufs[1]);
        st.death_scan = take(&mut bufs[3]);
    }
    // 12. event_collect 相 1(scatter + 计数;恒跑 max(nseg,1))。
    run_collect(st, 1.0, (nseg.max(1)) as u32)?;
    // 13. indirect_args(3 SSBO;emit_count 面 = scripted + accepted,
    //     host 平行金标准推得非回读)。
    {
        let params = bytes_f32(&[emit_effective as f32, nseg as f32, 0.0, 0.0]);
        let mut bufs = vec![params, take(&mut st.seg_offsets), take(&mut st.args)];
        vk::run_compute(&dev.spv_args, &dev.entry_args, &mut bufs, &[], [1, 1, 1])
            .map_err(|e| format!("indirect_args dispatch: {e}"))?;
        st.seg_offsets = take(&mut bufs[1]);
        st.args = take(&mut bufs[2]);
    }
    Ok(())
}

/// host 池 f32 流按下标取(0..8 = F32_STREAMS 前 8 序)。
fn host_stream(p: &rurix_render::particles::core::ParticlePools, k: usize) -> &[f32] {
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
    f32_stream_max: [f32; 9],
    spawn_seg_f32_max: f32,
    spawn_src_meta_bitexact: bool,
    spawn_counts_match: bool,
    ev_count_match: bool,
    spawn_pid_range_exact: bool,
    pid_unique: bool,
    pid_survivor_subset: bool,
    pids_issued: u32,
    args_match: bool,
    args_identity: bool,
    args_last: [u32; 8],
    overflow_frames: Vec<(usize, usize, usize, u64)>,
    trim_dual_order_stable: bool,
    death_overflow_frames: usize,
    secondary_frames: usize,
    gpu_accepted_total: u64,
    host_accepted_total: u64,
    snapshot_ok: bool,
    snapshot_checked: usize,
    digest: String,
    frame_ms_mean: f64,
    n_final: usize,
    alive_final: u32,
    problems: Vec<String>,
}

fn run_chain(
    dev: &DevKernels,
    seed: u64,
    frames: usize,
    cap: usize,
    tamper_frame: Option<usize>,
) -> ChainReport {
    let desc = emitter();
    let sp = spawn_params();
    let table = rand_table(seed);
    let mut ha = rurix_render::particles::core::ParticlePools::with_capacity(cap);
    let mut hb = rurix_render::particles::core::ParticlePools::with_capacity(cap);
    let mut st = DevState::new(cap, bytes_f32(&table));
    let mut pid_base = 0u32;
    let mut gpu_prev: Vec<ParticleEvent> = Vec::new();
    let mut prev_pids: HashSet<u32> = HashSet::new();
    let mut r = ChainReport {
        integer_bitexact: true,
        f32_stream_max: [0.0; 9],
        spawn_seg_f32_max: 0.0,
        spawn_src_meta_bitexact: true,
        spawn_counts_match: true,
        ev_count_match: true,
        spawn_pid_range_exact: true,
        pid_unique: true,
        pid_survivor_subset: true,
        pids_issued: 0,
        args_match: true,
        args_identity: true,
        args_last: [0; 8],
        overflow_frames: Vec::new(),
        trim_dual_order_stable: true,
        death_overflow_frames: 0,
        secondary_frames: 0,
        gpu_accepted_total: 0,
        host_accepted_total: 0,
        snapshot_ok: false,
        snapshot_checked: 0,
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
        let scripted = emit_schedule(f, n, cap);
        // ── host 队列装配(唯一装配面 push/trim;溢出裁剪稳定性对拍)──
        let raw = host_events_raw(f, seed);
        let pushed = raw.len();
        let q = queue_from(&raw, false);
        if pushed >= 2 && !queues_bits_eq(&q, &queue_from(&raw, true)) {
            r.trim_dual_order_stable = false;
            problem(&mut r.problems, format!("帧 {f}: 乱序 push trim 不同果"));
        }
        if pushed > EVENT_CAP {
            r.overflow_frames.push((f, pushed, q.len(), q.overflow_count()));
        }
        // ── host 平行金标准帧(event_frame 单源)──
        let out = event_frame(
            &mut ha, &mut hb, &desc, &sp, &table, DT, &q, &gpu_prev, pid_base, scripted,
        );
        let sh = out.stats;
        // ── 上传字节(RED 臂:device 侧 payload 篡改,host 金标准不动)──
        let host_meta_b = bytes_u32(&q.meta_words());
        let mut host_payload_b = bytes_f32(&q.payload_words());
        if tamper_frame == Some(f) && !q.is_empty() {
            let w = f32::from_le_bytes([
                host_payload_b[0],
                host_payload_b[1],
                host_payload_b[2],
                host_payload_b[3],
            ]) + 1.0;
            host_payload_b[0..4].copy_from_slice(&w.to_le_bytes());
        }
        let emit_effective = scripted as u32 + sh.spawn.accepted_total;
        // ── device 13 dispatch 链(墙钟计时)──
        let t0 = Instant::now();
        device_frame(
            dev,
            &mut st,
            n,
            &desc,
            &sp,
            pid_base,
            scripted,
            q.len(),
            &host_meta_b,
            &host_payload_b,
            emit_effective,
            cap,
        )
        .unwrap_or_else(|e| fail(&format!("帧 {f}: {e}")));
        ms_total += t0.elapsed().as_secs_f64() * 1000.0;
        // ── 整数流零容差对拍 ──
        let int_check = |name: &str, got: &[u32], want: &[u32], r: &mut ChainReport| {
            if got != want {
                r.integer_bitexact = false;
                problem(&mut r.problems, format!("帧 {f}: {name} 非位级"));
            }
        };
        let hsums = scan::seg_sums(&out.flags, nseg);
        let hspine = scan::spine(&hsums);
        let dsums = scan::seg_sums(&out.collect.death_flags, nseg);
        let dspine = scan::spine(&dsums);
        let dev_flags = read_u32(&st.flags);
        int_check("flags", &dev_flags[..n], &out.flags, &mut r);
        let dev_scan = read_u32(&st.scan_out);
        int_check("scan_out", &dev_scan[..n], &out.scan_out, &mut r);
        let dev_spine = read_u32(&st.seg_offsets);
        int_check("seg_offsets", &dev_spine[..nseg + 1], &hspine, &mut r);
        let dev_dflags = read_u32(&st.death_flags);
        int_check("death_flags", &dev_dflags[..n], &out.collect.death_flags, &mut r);
        let dev_dscan = read_u32(&st.death_scan);
        int_check("death_scan", &dev_dscan[..n], &out.collect.death_scan, &mut r);
        let dev_dspine = read_u32(&st.death_offsets);
        int_check("death_offsets", &dev_dspine[..nseg + 1], &dspine, &mut r);
        // ── ev_count 零回读见证(kept/total 双槽;溢出如实登记)──
        let kept = out.collect.kept as usize;
        let dev_evcount = read_u32(&st.ev_count);
        if dev_evcount[..2] != [out.collect.kept, out.collect.death_total][..] {
            r.ev_count_match = false;
            r.integer_bitexact = false;
            problem(
                &mut r.problems,
                format!(
                    "帧 {f}: ev_count {:?} ≠ host [{}, {}]",
                    &dev_evcount[..2],
                    out.collect.kept,
                    out.collect.death_total
                ),
            );
        }
        if out.collect.death_total > out.collect.kept {
            r.death_overflow_frames += 1;
        }
        // ── GPU 死亡事件缓冲(meta 位级 + payload f32 容差)──
        let dev_evmeta = read_u32(&st.ev_meta);
        let mut hmeta = Vec::with_capacity(kept * EVENT_META_WORDS);
        for ev in &out.collect.events {
            hmeta.extend_from_slice(&[ev.producer_id, ev.slot, ev.kind]);
        }
        int_check("ev_meta", &dev_evmeta[..kept * EVENT_META_WORDS], &hmeta, &mut r);
        let dev_evpay = read_f32(&st.ev_payload);
        for (e, ev) in out.collect.events.iter().enumerate() {
            for w in 0..EVENT_PAYLOAD_WORDS {
                let mut d = (dev_evpay[e * EVENT_PAYLOAD_WORDS + w] - ev.payload[w]).abs();
                if !d.is_finite() {
                    d = f32::INFINITY;
                    problem(&mut r.problems, format!("帧 {f}: ev_payload 非有限差"));
                }
                if d > r.f32_stream_max[8] {
                    r.f32_stream_max[8] = d;
                }
            }
        }
        // ── spawn_counts 零回读见证 + src_meta 双源次序见证 ──
        let accepted = sh.spawn.accepted_total as usize;
        let dev_spawn_counts = read_u32(&st.spawn_counts);
        if dev_spawn_counts[..3]
            != [
                sh.spawn.accepted_total,
                sh.spawn.host_accepted,
                sh.spawn.gpu_accepted,
            ][..]
        {
            r.spawn_counts_match = false;
            r.integer_bitexact = false;
            problem(
                &mut r.problems,
                format!(
                    "帧 {f}: spawn_counts {:?} ≠ host {:?}",
                    &dev_spawn_counts[..3],
                    sh.spawn
                ),
            );
        }
        let dev_srcmeta = read_u32(&st.src_meta);
        let mut merged = Vec::with_capacity(accepted * EVENT_META_WORDS);
        for j in 0..accepted {
            let ev = if j < q.len() { &q.events()[j] } else { &gpu_prev[j - q.len()] };
            merged.extend_from_slice(&[ev.producer_id, ev.slot, ev.kind]);
        }
        if dev_srcmeta[..accepted * EVENT_META_WORDS] != merged[..] {
            r.spawn_src_meta_bitexact = false;
            r.integer_bitexact = false;
            problem(&mut r.problems, format!("帧 {f}: src_meta 非位级(双源次序破)"));
        }
        // ── B 组流(pid 位级 + f32 逐流 max;发射段单独聚合)──
        let n_next = sh.n_next;
        let used = sh.alive_total as usize + scripted;
        let dev_pid = read_u32(&st.b[8]);
        int_check("pid", &dev_pid[..n_next], &hb.pid[..n_next], &mut r);
        for k in 0..8 {
            let dev_f = read_f32(&st.b[k]);
            let host_f = host_stream(&hb, k);
            for i in 0..n_next {
                let mut d = (dev_f[i] - host_f[i]).abs();
                if !d.is_finite() {
                    d = f32::INFINITY;
                    problem(
                        &mut r.problems,
                        format!("帧 {f}: {} 流非有限差(i={i})", F32_STREAMS[k]),
                    );
                }
                if d > r.f32_stream_max[k] {
                    r.f32_stream_max[k] = d;
                }
                if i >= used && i < used + accepted && d > r.spawn_seg_f32_max {
                    r.spawn_seg_f32_max = d;
                }
            }
        }
        // ── pid 持久性(唯一 + 幸存子集 + 新段精确区间〔脚本+事件双段
        //    pid 连续 ⇒ 单区间判〕)──
        let mut cur_pids: HashSet<u32> = HashSet::with_capacity(n_next);
        for &p in &dev_pid[..n_next] {
            if !cur_pids.insert(p) {
                r.pid_unique = false;
                problem(&mut r.problems, format!("帧 {f}: pid {p} 重复"));
            }
        }
        let alive = sh.alive_total as usize;
        if !dev_pid[..alive].iter().all(|p| prev_pids.contains(p)) {
            r.pid_survivor_subset = false;
            problem(&mut r.problems, format!("帧 {f}: 幸存段非上帧子集"));
        }
        if !dev_pid[alive..n_next]
            .iter()
            .enumerate()
            .all(|(j, &p)| p == pid_base + j as u32)
        {
            r.spawn_pid_range_exact = false;
            problem(&mut r.problems, format!("帧 {f}: 新段非精确区间(三段涵盖破)"));
        }
        prev_pids = cur_pids;
        // ── indirect args 零回读链 ──
        let dev_args_v = read_u32(&st.args);
        let mut dev_args = [0u32; 8];
        dev_args.copy_from_slice(&dev_args_v[..8]);
        if dev_args != sh.args {
            r.args_match = false;
            r.integer_bitexact = false;
            problem(
                &mut r.problems,
                format!("帧 {f}: args {dev_args:?} ≠ host {:?}", sh.args),
            );
        }
        if dev_args[7] != sh.alive_total + emit_effective {
            r.args_identity = false;
            problem(
                &mut r.problems,
                format!(
                    "帧 {f}: args[7]={} ≠ alive+emit_eff={}",
                    dev_args[7],
                    sh.alive_total + emit_effective
                ),
            );
        }
        // ── 二次发射样本量 ──
        if sh.spawn.gpu_accepted > 0 {
            r.secondary_frames += 1;
        }
        r.gpu_accepted_total += u64::from(sh.spawn.gpu_accepted);
        r.host_accepted_total += u64::from(sh.spawn.host_accepted);
        // ── 链式 digest(sha256(prev_hex ‖ 帧字节);全 device 流)──
        let mut trace: Vec<u8> = Vec::with_capacity(64 + n_next * 36 + n * 16 + kept * 32 + 128);
        trace.extend_from_slice(r.digest.as_bytes());
        for k in 0..9 {
            trace.extend_from_slice(&st.b[k][..n_next * 4]);
        }
        trace.extend_from_slice(&st.flags[..n * 4]);
        trace.extend_from_slice(&st.scan_out[..n * 4]);
        trace.extend_from_slice(&st.seg_offsets[..(nseg + 1) * 4]);
        trace.extend_from_slice(&st.death_flags[..n * 4]);
        trace.extend_from_slice(&st.death_scan[..n * 4]);
        trace.extend_from_slice(&st.death_offsets[..(nseg + 1) * 4]);
        trace.extend_from_slice(&st.ev_meta[..kept * EVENT_META_WORDS * 4]);
        trace.extend_from_slice(&st.ev_payload[..kept * EVENT_PAYLOAD_WORDS * 4]);
        trace.extend_from_slice(&st.ev_count);
        trace.extend_from_slice(&st.src_meta[..accepted * EVENT_META_WORDS * 4]);
        trace.extend_from_slice(&st.spawn_counts);
        trace.extend_from_slice(&st.args);
        r.digest = rurix_pkg::sha256::hex_digest(&trace);
        // ── 帧末交换(读 A 写 B;n_next host 平行推得零回读)──
        pid_base += scripted as u32 + sh.spawn.accepted_total;
        r.alive_final = sh.alive_total;
        r.n_final = n_next;
        r.args_last = dev_args;
        gpu_prev = out.collect.events;
        std::mem::swap(&mut ha, &mut hb);
        std::mem::swap(&mut st.a, &mut st.b);
    }
    // ── particle_view 桥 roundtrip(方向 A:末帧 device 九流 readback →
    //    GpuParticleSnapshot → pid 定址读 == readback 原值位级)──
    {
        let n_final = r.n_final;
        let sx = read_f32(&st.a[0]);
        let sy = read_f32(&st.a[1]);
        let sz = read_f32(&st.a[2]);
        let vx = read_f32(&st.a[3]);
        let vy = read_f32(&st.a[4]);
        let vz = read_f32(&st.a[5]);
        let pid = read_u32(&st.a[8]);
        let snap = GpuParticleSnapshot::from_streams(&sx, &sy, &sz, &vx, &vy, &vz, &pid, n_final);
        let mut ok = n_final > 0;
        for i in 0..n_final {
            match snap.lookup(pid[i]) {
                Some((p, v)) => {
                    let want_p = [sx[i], sy[i], sz[i]];
                    let want_v = [vx[i], vy[i], vz[i]];
                    for k in 0..3 {
                        if p[k].to_bits() != want_p[k].to_bits()
                            || v[k].to_bits() != want_v[k].to_bits()
                        {
                            ok = false;
                            problem(
                                &mut r.problems,
                                format!("snapshot roundtrip: pid {} 分量 {k} 非位级", pid[i]),
                            );
                        }
                    }
                }
                None => {
                    ok = false;
                    problem(
                        &mut r.problems,
                        format!("snapshot roundtrip: pid {} 定址丢失(静默失败检出)", pid[i]),
                    );
                }
            }
        }
        r.snapshot_ok = ok;
        r.snapshot_checked = n_final;
    }
    r.pids_issued = pid_base;
    r.frame_ms_mean = ms_total / frames as f64;
    r
}

impl ChainReport {
    fn hard_pass(&self) -> bool {
        self.integer_bitexact
            && self.spawn_src_meta_bitexact
            && self.spawn_counts_match
            && self.ev_count_match
            && self.spawn_pid_range_exact
            && self.pid_unique
            && self.pid_survivor_subset
            && self.args_match
            && self.args_identity
            && self.trim_dual_order_stable
            && self.snapshot_ok
            && self.secondary_frames >= 1
            && !self.overflow_frames.is_empty()
            && self.death_overflow_frames >= 1
    }

    fn stream_max_json(&self) -> String {
        let inner: Vec<String> = F32_STREAMS
            .iter()
            .zip(self.f32_stream_max.iter())
            .map(|(name, v)| format!("{}:{:e}", jstr(name), v))
            .collect();
        format!("{{{}}}", inner.join(","))
    }

    fn overflow_json(&self) -> String {
        let inner: Vec<String> = self
            .overflow_frames
            .iter()
            .map(|&(f, pushed, kept, ovf)| {
                format!(
                    "{{\"frame\":{f},\"pushed\":{pushed},\"kept\":{kept},\"overflow\":{ovf}}}"
                )
            })
            .collect();
        format!("[{}]", inner.join(","))
    }
}

// ---------------------------------------------------------------------------
// host-only 腿(host 金标准链恒可跑:守恒/双源计数/溢出登记/双跑位级)
// ---------------------------------------------------------------------------

fn host_only_leg(args: &Args) -> ! {
    fn run(seed: u64, frames: usize, cap: usize) -> (Vec<u32>, u64, usize, u64, u32) {
        let desc = emitter();
        let sp = spawn_params();
        let table = rand_table(seed);
        let mut a = rurix_render::particles::core::ParticlePools::with_capacity(cap);
        let mut b = rurix_render::particles::core::ParticlePools::with_capacity(cap);
        let mut pid_base = 0u32;
        let mut gpu_prev: Vec<ParticleEvent> = Vec::new();
        let mut overflow_total = 0u64;
        let mut secondary = 0usize;
        let mut gpu_total = 0u64;
        for f in 0..frames {
            let scripted = emit_schedule(f, a.n, cap);
            let q = queue_from(&host_events_raw(f, seed), false);
            overflow_total += q.overflow_count();
            let out = event_frame(
                &mut a, &mut b, &desc, &sp, &table, DT, &q, &gpu_prev, pid_base, scripted,
            );
            let sh = out.stats;
            assert_eq!(
                sh.n_next,
                sh.alive_total as usize + scripted + sh.spawn.accepted_total as usize,
                "host 帧 {f}: 守恒破"
            );
            let uniq: HashSet<u32> = b.pid[..b.n].iter().copied().collect();
            assert_eq!(uniq.len(), b.n, "host 帧 {f}: pid 重复");
            if sh.spawn.gpu_accepted > 0 {
                secondary += 1;
            }
            gpu_total += u64::from(sh.spawn.gpu_accepted);
            pid_base += scripted as u32 + sh.spawn.accepted_total;
            gpu_prev = out.collect.events;
            std::mem::swap(&mut a, &mut b);
        }
        let mut bits = Vec::with_capacity(a.n * 9 + 1);
        bits.push(a.n as u32);
        for i in 0..a.n {
            bits.push(a.pos_x[i].to_bits());
            bits.push(a.pos_y[i].to_bits());
            bits.push(a.pos_z[i].to_bits());
            bits.push(a.vel_x[i].to_bits());
            bits.push(a.vel_y[i].to_bits());
            bits.push(a.vel_z[i].to_bits());
            bits.push(a.age[i].to_bits());
            bits.push(a.life[i].to_bits());
            bits.push(a.pid[i]);
        }
        (bits, overflow_total, secondary, gpu_total, pid_base)
    }
    let (b1, ovf, sec, gpu_total, pids) = run(args.seed, args.frames, args.cap);
    let (b2, _, _, _, _) = run(args.seed, args.frames, args.cap);
    let ok = b1 == b2 && sec >= 1 && ovf > 0 && gpu_total >= 1;
    let line = format!(
        "{{\"schema\":\"rurix.g35.events_host.v1\",\"mode\":\"host-only\",\"state\":{},\
         \"frames\":{},\"cap\":{},\"seed\":{},\"double_run_bitexact\":{},\
         \"host_overflow_total\":{ovf},\"secondary_frames\":{sec},\
         \"gpu_accepted_total\":{gpu_total},\"pids_issued\":{pids},\"base_commit\":{}}}",
        jstr(if ok { "pass" } else { "fail" }),
        args.frames,
        args.cap,
        args.seed,
        b1 == b2,
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    std::process::exit(i32::from(!ok))
}

// ---------------------------------------------------------------------------
// main(默认 = 全档验证:双跑同 seed;--red-arm payload-tamper = 绿链 vs
// 帧 12 payload 篡改链 digest 必异)
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
                "{{\"schema\":\"rurix.g35.events_probe.v1\",\"state\":\"skipped_dev_env\",\
                 \"reason\":{}}}",
                jstr(&e)
            );
            emit_evidence(&line, &args.evidence_out);
            std::process::exit(0);
        }
    };

    if let Some(arm) = &args.red_arm {
        if arm != "payload-tamper" {
            fail(&format!("未知 RED 臂: {arm}(payload-tamper)"));
        }
        // RED 臂:帧 12 host 事件 payload 词 0 上传件篡改 +1.0(host 金标准
        // 不动)⇒ 篡改流入发射段 pos ⇒ 与绿链 digest 必异(事件 payload
        // 篡改必检出;digest 判据对事件净荷敏感性证明)。
        let g = run_chain(&dev, args.seed, args.frames, args.cap, None);
        let t = run_chain(&dev, args.seed, args.frames, args.cap, Some(TAMPER_FRAME));
        let detected = g.digest != t.digest;
        let line = format!(
            "{{\"schema\":\"rurix.g35.events_red_arm.v1\",\"arm\":\"payload-tamper\",\
             \"detected\":{detected},\"tamper_frame\":{TAMPER_FRAME},\
             \"tamper\":\"host_payload[0] += 1.0(device 上传件;host 金标准不动)\",\
             \"digest_green\":{},\"digest_red\":{}}}",
            jstr(&format!("sha256:{}", g.digest)),
            jstr(&format!("sha256:{}", t.digest)),
        );
        emit_evidence(&line, &args.evidence_out);
        if !detected {
            fail("red-arm payload-tamper 失效(漏检):篡改后 digest 未变");
        }
        eprintln!("{TAG}: red-arm payload-tamper 检出 — digest 已异");
        std::process::exit(0);
    }

    // ── 全档验证:双跑同 seed(判据 ⑦ device 双跑位级)+ 逐帧对拍 ──
    let a = run_chain(&dev, args.seed, args.frames, args.cap, None);
    let b = run_chain(&dev, args.seed, args.frames, args.cap, None);
    let determinism = a.digest == b.digest;
    let f32_p100 = a
        .f32_stream_max
        .iter()
        .copied()
        .fold(a.spawn_seg_f32_max, f32::max);
    let state = if a.hard_pass() && determinism {
        "pass"
    } else {
        "fail"
    };
    if args.report_max_diff {
        println!("f32_max_abs_diff={f32_p100:e}");
    }
    eprintln!(
        "{TAG}: {} frames={} cap={} seed={} int_bitexact={} f32_p100={:e} spawn_seg={:e} \
         overflow_frames={} death_overflow={} secondary={} snapshot={} double_run={} n_final={} \
         frame_ms={:.3}",
        state,
        args.frames,
        args.cap,
        args.seed,
        a.integer_bitexact,
        f32_p100,
        a.spawn_seg_f32_max,
        a.overflow_frames.len(),
        a.death_overflow_frames,
        a.secondary_frames,
        a.snapshot_ok,
        determinism,
        a.n_final,
        a.frame_ms_mean,
    );
    let mut problems = a.problems.clone();
    if !determinism {
        problems.push("device 双跑 digest 非位级一致".into());
    }
    let line = format!(
        "{{\"schema\":\"rurix.g35.events_probe.v1\",\"state\":{},\
         \"frames\":{},\"cap\":{},\"seed\":{},\"dt\":{:e},\"burst\":{},\
         \"emit_schedule\":\"f0: min(16000, cap-n); else min(32 + f*11 % 96, cap - n_curr)\",\
         \"host_event_schedule\":\"f12: 1200; f30: 1100; else (f*7) % 5\",\
         \"event_cap\":{},\
         \"integer_streams\":[\"flags\",\"scan_out\",\"seg_offsets\",\"death_flags\",\
         \"death_scan\",\"death_offsets\",\"ev_meta\",\"ev_count\",\"src_meta\",\
         \"spawn_counts\",\"pid\",\"args\"],\
         \"integer_streams_bitexact\":{},\
         \"f32_max_abs_diff\":{:e},\"f32_stream_max\":{},\"spawn_seg_f32_max\":{:e},\
         \"event_overflow\":{{\"frames\":{},\"trim_dual_order_stable\":{},\
         \"death_overflow_frames\":{}}},\
         \"spawn_parity\":{{\"src_meta_bitexact\":{},\"counts_match\":{},\
         \"pid_range_exact\":{}}},\
         \"zero_readback\":{{\"ev_count_match\":{},\"spawn_counts_match\":{},\
         \"secondary_frames\":{},\"gpu_accepted_total\":{},\"host_accepted_total\":{}}},\
         \"snapshot_roundtrip\":{{\"ok\":{},\"checked\":{}}},\
         \"pid_unique\":{},\"pid_survivor_subset\":{},\"pids_issued\":{},\
         \"args_match\":{},\"args_identity\":{},\"args_last\":[{}],\
         \"determinism_double_run\":{},\"digest_a\":{},\"digest_b\":{},\
         \"frame_ms_mean\":{:.6},\"n_final\":{},\"alive_final\":{},\
         \"nocontraction_injected\":[\"g35_sim\",\"g35_emit\",\"g35_event_spawn\"],\
         \"problems\":{},\"base_commit\":{}}}",
        jstr(state),
        args.frames,
        args.cap,
        args.seed,
        DT,
        BURST,
        EVENT_CAP,
        a.integer_bitexact,
        f32_p100,
        a.stream_max_json(),
        a.spawn_seg_f32_max,
        a.overflow_json(),
        a.trim_dual_order_stable,
        a.death_overflow_frames,
        a.spawn_src_meta_bitexact,
        a.spawn_counts_match,
        a.spawn_pid_range_exact,
        a.ev_count_match,
        a.spawn_counts_match,
        a.secondary_frames,
        a.gpu_accepted_total,
        a.host_accepted_total,
        a.snapshot_ok,
        a.snapshot_checked,
        a.pid_unique,
        a.pid_survivor_subset,
        a.pids_issued,
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
