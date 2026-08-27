//! G35-1 GPU 粒子基元 device probe harness(门 g35.wave1.primitives;
//! RFC-0049 §4.2 判据事实源;g27_hzb_device 同模)。
//!
//! ## 集成路径
//!
//! bin-local 全部逻辑:`DevicePrimitives` 经 `rurix_rt::vk::run_compute`
//! (G12/G13/G26/G27 compute 派发面同车道)驱动七 kernel——scan 三件
//! (`kernels/g35_scan_{seg_sum,spine,seg_apply}.rx`,冻结面只读消费)+
//! sort 三件(`kernels/g35_sort_{hist,spine,scatter}.rx`,3 pass ×
//! (hist→spine→scatter),pass 间 ping-pong = host 侧 Vec 交接)+ compact
//! (`kernels/g35_compact_u32.rx`,scan 链产物消费面)。公式面与 host 金标准
//! `particles/{scan,primitives}.rs` **逐字同源**;SSBO 序 = kernel 形参声明
//! 序 = buffers 下标,u32/f32 缓冲手工 to_le_bytes/from_le_bytes 打包
//! (g27_hzb_device 字节面逐字同律)。
//!
//! ## 夹具
//!
//! `particles::Pcg32` 固定 seed(35/54)单源:keys = next_u32() % 2^24
//! (深度键域)、payload = 原下标(稳定性判据载体)、flags = next_u32() % 2
//! (压缩域);规模闭集 {256, 4096, 65536, --scale}(去重升序;--scale
//! 默认 65536,上限 PARTICLE_CAP_MAX = 1048576)。
//!
//! ## 判据面(mod.rs 整数域协议:全零容差,无标定腿)
//!
//! ① scan 三 kernel 全链输出与 host `scan::exclusive_scan_segmented`
//!    **位级相等**(scan_bitexact);② sort 全链输出与 host
//!    `primitives::sort_pairs_u24` 位级相等(sort_bitexact)且与独立参考
//!    `sort_pairs_reference`(std 稳定 sort)互核相等(防同一错误两处照抄);
//! ③ 稳定性:同键 payload 保序(host 侧验证函数;payload = 原下标 ⇒
//!    稳定 ⇔ 同键段 payload 严格递增;判据咬合前提 = 重复键对 ≥ 1);
//! ④ compact 输出与 host `primitives::compact_u32` 位级相等;⑤ device
//!    双跑位级(全链重跑逐流 memcmp);⑥ throughput measured 登记
//!    (keys/秒,诚实登记不设通过线)。
//!
//! ## RED 臂(构造性注入协议;g27 tamper 同律)
//!
//! `--red-arm tamper`:host 预算 pass 0(dpow=1)digit-major 非空块序列,
//! 选首个 digit ≥ 1 且非末位的块,其 off 槽 +1(块整体右移 1:原首槽失写
//! 落缓冲初始 0、块尾与后继块首同槽竞写——散射 = 输入多重集的双射,注入后
//! pass 1 输出多重集必变且后续 pass 保多重集 ⇒ 末端输出必异于 host,
//! 构造性保证检出;digit ≥ 1 排除 0 键退化,非末位排除越界写)。命中 →
//! 退 0 且 evidence 记 red_arm_effective=true;未命中 → 退 1(判据成摆设)。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备 → stdout 打印 `{"status":"skipped_dev_env",...}`
//! 退 0(非 fake pass);env `RURIX_REQUIRE_REAL=1` 时翻硬退 1。host 腿恒可
//! `--host-only`(跳 device 只跑 host 自证);判据不符 / RED 臂失效 ⇒ 退 1。
//!
//! ## 用法
//!
//! ```text
//! g35_primitives_device --spv-scan-seg-sum <spv> --spv-scan-spine <spv> \
//!   --spv-scan-seg-apply <spv> --spv-sort-hist <spv> --spv-sort-spine <spv> \
//!   --spv-sort-scatter <spv> --spv-compact <spv> [--evidence-out <path>] \
//!   [--scale <n>]                                   # 全档验证(默认)
//! g35_primitives_device --red-arm tamper --spv-… [--evidence-out <path>]
//! g35_primitives_device --host-only [--evidence-out <path>] [--scale <n>]
//! ```

#![forbid(unsafe_code)]

use rurix_render::particles::{PARTICLE_CAP_MAX, Pcg32, SEG, primitives, scan};
use rurix_rt::vk;
use std::time::Instant;

const TAG: &str = "[g35_primitives_device]";
const PROBE_SCHEMA: &str = "rurix.g35.primitives_probe.v1";
const GATE_KEY: &str = "g35.wave1.primitives";
/// 夹具 seed/stream(固定单源;mod.rs 随机带单源纪律)。
const FIXTURE_SEED: u64 = 35;
const FIXTURE_STREAM: u64 = 54;
/// 3 pass digit 幂(24 位键 = 3 × 8 bit;primitives.rs 逐字同源)。
const DPOWS: [usize; 3] = [1, 256, 65536];
/// RED 臂夹具规模(固定;honest 前置 + 注入双臂同夹具)。
const RED_ARM_N: usize = 4096;
/// throughput 口径字面(诚实登记不设通过线)。
const THROUGHPUT_MEASURED: &str = "measured_local(probe 车道 vk::run_compute 逐 dispatch 建/毁 \
     instance+device 含上传/回读;单跑 3 pass 9 dispatch host 墙钟;非生产 DeviceFrameSession 口径)";

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 字节工具(g27_hzb_device 字面同律;u32 面)
// ---------------------------------------------------------------------------

fn bytes_u32(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_u32(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
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

/// 参数面(4 f32:[0]=n [1]=nseg [2]=dpow(scan/compact 面恒 0)[3]=reserved;
/// kernel 头注逐字同源,n ≤ 2^24 f32 精确)。
fn params_bytes(n: usize, nseg: usize, dpow: usize) -> Vec<u8> {
    bytes_f32(&[n as f32, nseg as f32, dpow as f32, 0.0])
}

// ---------------------------------------------------------------------------
// 夹具(固定 seed 单源)
// ---------------------------------------------------------------------------

struct Fixture {
    keys: Vec<u32>,
    payload: Vec<u32>,
    flags: Vec<u32>,
}

fn fixture(n: usize) -> Fixture {
    let mut rng = Pcg32::new(FIXTURE_SEED, FIXTURE_STREAM);
    let keys: Vec<u32> = (0..n).map(|_| rng.next_u32() % 16_777_216).collect();
    let payload: Vec<u32> = (0..n as u32).collect();
    let flags: Vec<u32> = (0..n).map(|_| rng.next_u32() % 2).collect();
    Fixture {
        keys,
        payload,
        flags,
    }
}

/// 规模闭集 {256, 4096, 65536, --scale}(去重升序)。
fn scales_closed_set(scale: usize) -> Vec<usize> {
    let mut v = vec![256usize, 4096, 65536, scale];
    v.sort_unstable();
    v.dedup();
    v
}

/// 稳定性 host 侧验证函数(判据 ③):payload = 原下标 ⇒ 稳定 ⇔ 同键段
/// payload 严格递增;返回 (保序, 相邻同键对数——咬合前提 ≥ 1 归判读面)。
fn stability_verify(keys_sorted: &[u32], payload_sorted: &[u32]) -> (bool, u64) {
    let mut dup_pairs = 0u64;
    let mut ok = true;
    for w in 1..keys_sorted.len() {
        if keys_sorted[w] == keys_sorted[w - 1] {
            dup_pairs += 1;
            if payload_sorted[w] <= payload_sorted[w - 1] {
                ok = false;
            }
        }
    }
    (ok, dup_pairs)
}

// ---------------------------------------------------------------------------
// device 臂(bin-local;经 vk::run_compute 逐 kernel 派发)
// ---------------------------------------------------------------------------

struct Kernel {
    spv: Vec<u32>,
    entry: String,
}

fn kernel_of(spv: Vec<u32>, what: &str) -> Result<Kernel, String> {
    let entry = vk::entry_point_name(&spv).ok_or(format!("{what} SPV 无 OpEntryPoint"))?;
    Ok(Kernel { spv, entry })
}

fn dispatch(k: &Kernel, bufs: &mut [Vec<u8>], groups: u32, what: &str) {
    vk::run_compute(&k.spv, &k.entry, bufs, &[], [groups, 1, 1])
        .unwrap_or_else(|e| fail(&format!("{what} dispatch({groups} groups)失败: {e}")));
}

struct DevicePrimitives {
    scan_seg_sum: Kernel,
    scan_spine: Kernel,
    scan_seg_apply: Kernel,
    sort_hist: Kernel,
    sort_spine: Kernel,
    sort_scatter: Kernel,
    compact: Kernel,
}

impl DevicePrimitives {
    #[allow(clippy::too_many_arguments)]
    fn create(
        spv_scan_seg_sum: Vec<u32>,
        spv_scan_spine: Vec<u32>,
        spv_scan_seg_apply: Vec<u32>,
        spv_sort_hist: Vec<u32>,
        spv_sort_spine: Vec<u32>,
        spv_sort_scatter: Vec<u32>,
        spv_compact: Vec<u32>,
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        Ok(Self {
            scan_seg_sum: kernel_of(spv_scan_seg_sum, "scan_seg_sum")?,
            scan_spine: kernel_of(spv_scan_spine, "scan_spine")?,
            scan_seg_apply: kernel_of(spv_scan_seg_apply, "scan_seg_apply")?,
            sort_hist: kernel_of(spv_sort_hist, "sort_hist")?,
            sort_spine: kernel_of(spv_sort_spine, "sort_spine")?,
            sort_scatter: kernel_of(spv_sort_scatter, "sort_scatter")?,
            compact: kernel_of(spv_compact, "compact_u32")?,
        })
    }

    /// device 3-pass 排序全链(host `sort_pairs_u24` 镜像拓扑:每 pass
    /// hist→spine→scatter,pass 间 ping-pong = host 侧 Vec 交接)。
    /// tamper = RED 臂构造性注入:pass 0 spine 产 off 后对槽 slot +1。
    fn sort_pairs(&self, keys: &[u32], payload: &[u32], tamper: Option<usize>) -> (Vec<u32>, Vec<u32>) {
        let n = keys.len();
        let nseg = n.div_ceil(SEG);
        let groups = nseg as u32;
        let mut k = keys.to_vec();
        let mut p = payload.to_vec();
        for (pass, dpow) in DPOWS.into_iter().enumerate() {
            // 阶段 1:hist(SSBO 序 = g35_sort_hist.rx 形参声明序)。
            let mut bufs = vec![
                bytes_u32(&k),
                params_bytes(n, nseg, dpow),
                vec![0u8; nseg * 256 * 4],
            ];
            dispatch(&self.sort_hist, &mut bufs, groups, "sort_hist");
            let hist_bytes = bufs.swap_remove(2);
            // 阶段 2:spine(单 invocation;digit-major off)。
            let mut bufs = vec![
                hist_bytes,
                params_bytes(n, nseg, dpow),
                vec![0u8; 256 * nseg * 4],
            ];
            dispatch(&self.sort_spine, &mut bufs, 1, "sort_spine");
            let mut off = read_u32(&bufs[2]);
            if pass == 0 {
                if let Some(slot) = tamper {
                    off[slot] += 1; // RED 臂:off 缓冲某槽 +1(头注构造性协议)
                }
            }
            // 阶段 3:scatter(双流稳定散射)。
            let mut bufs = vec![
                bytes_u32(&k),
                bytes_u32(&p),
                bytes_u32(&off),
                params_bytes(n, nseg, dpow),
                vec![0u8; nseg * 256 * 4],
                vec![0u8; n * 4],
                vec![0u8; n * 4],
            ];
            dispatch(&self.sort_scatter, &mut bufs, groups, "sort_scatter");
            p = read_u32(&bufs[6]);
            k = read_u32(&bufs[5]);
        }
        (k, p)
    }

    /// device scan 三 kernel 全链(host `exclusive_scan_segmented` 镜像
    /// 拓扑);返回 (exclusive scan, 总和槽 seg_offsets[nseg])。
    fn scan_chain(&self, values: &[u32]) -> (Vec<u32>, u32) {
        let n = values.len();
        let nseg = n.div_ceil(SEG);
        let groups = nseg as u32;
        let mut bufs = vec![
            bytes_u32(values),
            params_bytes(n, nseg, 0),
            vec![0u8; nseg * 4],
        ];
        dispatch(&self.scan_seg_sum, &mut bufs, groups, "scan_seg_sum");
        let seg_sums_bytes = bufs.swap_remove(2);
        let mut bufs = vec![
            seg_sums_bytes,
            params_bytes(n, nseg, 0),
            vec![0u8; (nseg + 1) * 4],
        ];
        dispatch(&self.scan_spine, &mut bufs, 1, "scan_spine");
        let seg_offsets_bytes = bufs.swap_remove(2);
        let total = read_u32(&seg_offsets_bytes)[nseg];
        let mut bufs = vec![
            bytes_u32(values),
            seg_offsets_bytes,
            params_bytes(n, nseg, 0),
            vec![0u8; n * 4],
        ];
        dispatch(&self.scan_seg_apply, &mut bufs, groups, "scan_seg_apply");
        (read_u32(&bufs[3]), total)
    }

    /// device compact(scan 链产物消费面);out 容量 = max(total,1)
    /// (Vulkan 零尺寸缓冲不可创建;total=0 时哨兵槽不被写),回读截断至 total。
    fn compact(&self, values: &[u32], flags: &[u32], scan_out: &[u32], total: u32) -> Vec<u32> {
        let n = values.len();
        let nseg = n.div_ceil(SEG);
        let out_cap = (total as usize).max(1);
        let mut bufs = vec![
            bytes_u32(values),
            bytes_u32(flags),
            bytes_u32(scan_out),
            params_bytes(n, nseg, 0),
            vec![0u8; out_cap * 4],
        ];
        dispatch(&self.compact, &mut bufs, nseg as u32, "compact_u32");
        let mut out = read_u32(&bufs[4]);
        out.truncate(total as usize);
        out
    }
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

fn base_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// evidence 落盘(--evidence-out;stdout 恒打印同一行)。
fn emit(line: &str, args: &Args) {
    println!("{line}");
    if let Some(path) = &args.evidence_out {
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
    spv_scan_seg_sum: Option<String>,
    spv_scan_spine: Option<String>,
    spv_scan_seg_apply: Option<String>,
    spv_sort_hist: Option<String>,
    spv_sort_spine: Option<String>,
    spv_sort_scatter: Option<String>,
    spv_compact: Option<String>,
    evidence_out: Option<String>,
    red_arm: Option<String>,
    host_only: bool,
    scale: usize,
}

fn parse_args() -> Args {
    let mut a = Args {
        spv_scan_seg_sum: None,
        spv_scan_spine: None,
        spv_scan_seg_apply: None,
        spv_sort_hist: None,
        spv_sort_spine: None,
        spv_sort_scatter: None,
        spv_compact: None,
        evidence_out: None,
        red_arm: None,
        host_only: false,
        scale: 65536,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            "--spv-scan-seg-sum" => a.spv_scan_seg_sum = it.next(),
            "--spv-scan-spine" => a.spv_scan_spine = it.next(),
            "--spv-scan-seg-apply" => a.spv_scan_seg_apply = it.next(),
            "--spv-sort-hist" => a.spv_sort_hist = it.next(),
            "--spv-sort-spine" => a.spv_sort_spine = it.next(),
            "--spv-sort-scatter" => a.spv_sort_scatter = it.next(),
            "--spv-compact" => a.spv_compact = it.next(),
            "--evidence-out" => a.evidence_out = it.next(),
            "--red-arm" => a.red_arm = it.next(),
            "--host-only" => a.host_only = true,
            "--scale" => {
                let v = it.next().unwrap_or_else(|| fail("--scale 缺值"));
                a.scale = v
                    .parse::<usize>()
                    .unwrap_or_else(|e| fail(&format!("--scale {v} 非法: {e}")));
            }
            other => fail(&format!("未知参数: {other}")),
        }
    }
    if a.scale == 0 || a.scale > PARTICLE_CAP_MAX {
        fail(&format!(
            "--scale {} 越域(1 ..= PARTICLE_CAP_MAX {})",
            a.scale, PARTICLE_CAP_MAX
        ));
    }
    a
}

fn device_arm(args: &Args) -> Result<DevicePrimitives, String> {
    let need = |o: &Option<String>, flag: &str| -> Vec<u32> {
        load_spv(o.as_deref().unwrap_or_else(|| fail(&format!("缺 {flag}"))))
    };
    DevicePrimitives::create(
        need(&args.spv_scan_seg_sum, "--spv-scan-seg-sum"),
        need(&args.spv_scan_spine, "--spv-scan-spine"),
        need(&args.spv_scan_seg_apply, "--spv-scan-seg-apply"),
        need(&args.spv_sort_hist, "--spv-sort-hist"),
        need(&args.spv_sort_spine, "--spv-sort-spine"),
        need(&args.spv_sort_scatter, "--spv-sort-scatter"),
        need(&args.spv_compact, "--spv-compact"),
    )
}

/// 三态之 SKIP:无 Vulkan loader/设备 → 退 0;RURIX_REQUIRE_REAL=1 翻硬退 1。
fn skip_dev_env(reason: &str, args: &Args) -> ! {
    let line = format!(
        "{{\"schema\":{},\"status\":\"skipped_dev_env\",\"mode\":\"device\",\"gate\":{},\"reason\":{}}}",
        jstr(PROBE_SCHEMA),
        jstr(GATE_KEY),
        jstr(reason)
    );
    emit(&line, args);
    if std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1") {
        eprintln!("{TAG}: FAIL RURIX_REQUIRE_REAL=1 但 Vulkan 设备面不可用({reason})");
        std::process::exit(1);
    }
    std::process::exit(0)
}

// ---------------------------------------------------------------------------
// 全档验证单规模(判据 ①~⑤;facts 同名字段)
// ---------------------------------------------------------------------------

struct ScaleReport {
    n: usize,
    nseg: usize,
    scan_bitexact: bool,
    sort_bitexact: bool,
    sort_reference_equal: bool,
    sort_stable: bool,
    duplicate_pairs: u64,
    compact_bitexact: bool,
    compact_kept: u32,
    double_run_bitexact: bool,
    sort_ms: f64,
}

impl ScaleReport {
    fn all_green(&self) -> bool {
        self.scan_bitexact
            && self.sort_bitexact
            && self.sort_reference_equal
            && self.sort_stable
            && self.compact_bitexact
            && self.double_run_bitexact
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"n\":{},\"nseg\":{},\"scan_bitexact\":{},\"sort_bitexact\":{},\
             \"sort_reference_equal\":{},\"sort_stable\":{},\"duplicate_pairs\":{},\
             \"compact_bitexact\":{},\"compact_kept\":{},\"double_run_bitexact\":{},\
             \"sort_ms\":{:.3}}}",
            self.n,
            self.nseg,
            self.scan_bitexact,
            self.sort_bitexact,
            self.sort_reference_equal,
            self.sort_stable,
            self.duplicate_pairs,
            self.compact_bitexact,
            self.compact_kept,
            self.double_run_bitexact,
            self.sort_ms,
        )
    }
}

fn run_scale(dev: &DevicePrimitives, n: usize) -> ScaleReport {
    let fx = fixture(n);
    // host 金标准腿(恒跑)+ 独立参考互核腿(防同一错误两处照抄)。
    let (hk, hp) = primitives::sort_pairs_u24(&fx.keys, &fx.payload);
    let (rk, rp) = primitives::sort_pairs_reference(&fx.keys, &fx.payload);
    let (hscan, htotal) = scan::exclusive_scan_segmented(&fx.flags);
    let hcomp = primitives::compact_u32(&fx.keys, &fx.flags);
    // device 全链双跑(⑤ 逐流 memcmp 位级)。
    let run = || {
        let t0 = Instant::now();
        let (dk, dp) = dev.sort_pairs(&fx.keys, &fx.payload, None);
        let sort_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let (dscan, dtotal) = dev.scan_chain(&fx.flags);
        let dcomp = dev.compact(&fx.keys, &fx.flags, &dscan, dtotal);
        (dk, dp, dscan, dtotal, dcomp, sort_ms)
    };
    let (dk, dp, dscan, dtotal, dcomp, sort_ms) = run();
    let (dk2, dp2, dscan2, dtotal2, dcomp2, _) = run();
    let double_run =
        dk == dk2 && dp == dp2 && dscan == dscan2 && dtotal == dtotal2 && dcomp == dcomp2;
    let (stable, dup_pairs) = stability_verify(&dk, &dp);
    ScaleReport {
        n,
        nseg: n.div_ceil(SEG),
        scan_bitexact: dscan == hscan && dtotal == htotal,
        sort_bitexact: dk == hk && dp == hp,
        sort_reference_equal: dk == rk && dp == rp,
        sort_stable: stable,
        duplicate_pairs: dup_pairs,
        compact_bitexact: dcomp == hcomp,
        compact_kept: htotal,
        double_run_bitexact: double_run,
        sort_ms,
    }
}

// ---------------------------------------------------------------------------
// RED 臂:tamper(构造性注入;头注协议)
// ---------------------------------------------------------------------------

/// 注入点预算:pass 0(dpow=1)digit-major 非空块序列,选首个 digit ≥ 1 且
/// 非末位的块 → off 槽 d·nseg + s(不可达即 None,如实 FAIL 不冒充)。
fn plan_tamper(keys: &[u32]) -> Option<usize> {
    let n = keys.len();
    let nseg = n.div_ceil(SEG);
    let hist = primitives::sort_hist(keys, nseg, 1);
    let mut blocks: Vec<(usize, usize)> = Vec::new();
    for d in 0..256 {
        for s in 0..nseg {
            if hist[s * 256 + d] > 0 {
                blocks.push((d, s));
            }
        }
    }
    for (i, &(d, s)) in blocks.iter().enumerate() {
        if d >= 1 && i + 1 < blocks.len() {
            return Some(d * nseg + s);
        }
    }
    None
}

fn red_arm_tamper(dev: &DevicePrimitives) -> Result<String, String> {
    let fx = fixture(RED_ARM_N);
    let (hk, hp) = primitives::sort_pairs_u24(&fx.keys, &fx.payload);
    // honest 前置:注入前 device 必须与 host 位级等(证差异确由注入引起)。
    let (ok_k, ok_p) = dev.sort_pairs(&fx.keys, &fx.payload, None);
    if ok_k != hk || ok_p != hp {
        return Err("红臂前置失败:honest 臂 device ≠ host(绿臂判据自身红)".into());
    }
    let slot = plan_tamper(&fx.keys)
        .ok_or("构造性注入点不可达:digit-major 非空块 < 2 或全落 digit 0")?;
    let (tk, tp) = dev.sort_pairs(&fx.keys, &fx.payload, Some(slot));
    let keys_diff = tk.iter().zip(hk.iter()).filter(|(a, b)| a != b).count();
    let payload_diff = tp.iter().zip(hp.iter()).filter(|(a, b)| a != b).count();
    if keys_diff == 0 && payload_diff == 0 {
        return Err(format!(
            "红臂漏检:off[{slot}] +1 注入后输出仍与 host 位级相等(判据成摆设)"
        ));
    }
    Ok(format!(
        "off[{slot}] +1 注入检出:keys 异 {keys_diff} 槽,payload 异 {payload_diff} 槽(n={RED_ARM_N})"
    ))
}

// ---------------------------------------------------------------------------
// host 腿(--host-only:host 金标准 × 独立参考互核自证)
// ---------------------------------------------------------------------------

fn host_only_leg(args: &Args) -> ! {
    let mut problems: Vec<String> = Vec::new();
    let mut rows: Vec<String> = Vec::new();
    for &n in &scales_closed_set(args.scale) {
        let fx = fixture(n);
        let (hk, hp) = primitives::sort_pairs_u24(&fx.keys, &fx.payload);
        let (rk, rp) = primitives::sort_pairs_reference(&fx.keys, &fx.payload);
        let sort_eq = hk == rk && hp == rp;
        let (stable, dup_pairs) = stability_verify(&hk, &hp);
        let compact_eq = primitives::compact_u32(&fx.keys, &fx.flags)
            == primitives::compact_reference(&fx.keys, &fx.flags);
        let (sa, ta) = scan::exclusive_scan_segmented(&fx.flags);
        let (sb, tb) = scan::exclusive_scan_reference(&fx.flags);
        let scan_eq = sa == sb && ta == tb;
        if !(sort_eq && stable && compact_eq && scan_eq) {
            problems.push(format!(
                "n={n} sort_eq={sort_eq} stable={stable} compact_eq={compact_eq} scan_eq={scan_eq}"
            ));
        }
        rows.push(format!(
            "{{\"n\":{n},\"sort_reference_equal\":{sort_eq},\"sort_stable\":{stable},\
             \"duplicate_pairs\":{dup_pairs},\"compact_reference_equal\":{compact_eq},\
             \"scan_reference_equal\":{scan_eq}}}"
        ));
        eprintln!(
            "{TAG}: host n={n} sort_eq={sort_eq} stable={stable} dup_pairs={dup_pairs} \
             compact_eq={compact_eq} scan_eq={scan_eq}"
        );
    }
    let status = if problems.is_empty() { "pass" } else { "fail" };
    let line = format!(
        "{{\"schema\":{},\"status\":{},\"mode\":\"host-only\",\"gate\":{},\"scales\":[{}],\
         \"base_commit\":{}}}",
        jstr(PROBE_SCHEMA),
        jstr(status),
        jstr(GATE_KEY),
        rows.join(","),
        jstr(&base_commit()),
    );
    emit(&line, args);
    std::process::exit(i32::from(!problems.is_empty()))
}

// ---------------------------------------------------------------------------
// main(默认 = 全档验证:规模闭集逐档 ①~⑥)
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    if args.host_only {
        host_only_leg(&args);
    }

    let dev = match device_arm(&args) {
        Ok(d) => d,
        Err(e) => skip_dev_env(&e, &args),
    };

    if let Some(arm) = &args.red_arm {
        if arm != "tamper" {
            fail(&format!("未知 RED 臂: {arm}(tamper)"));
        }
        match red_arm_tamper(&dev) {
            Ok(detail) => {
                eprintln!("{TAG}: red-arm tamper 检出 — {detail}");
                let line = format!(
                    "{{\"schema\":{},\"status\":\"pass\",\"mode\":\"red-arm\",\"gate\":{},\
                     \"red_arm_effective\":true,\"red_arm_detail\":{},\"base_commit\":{}}}",
                    jstr(PROBE_SCHEMA),
                    jstr(GATE_KEY),
                    jstr(&detail),
                    jstr(&base_commit()),
                );
                emit(&line, &args);
                std::process::exit(0);
            }
            Err(e) => {
                let line = format!(
                    "{{\"schema\":{},\"status\":\"fail\",\"mode\":\"red-arm\",\"gate\":{},\
                     \"red_arm_effective\":false,\"red_arm_detail\":{},\"base_commit\":{}}}",
                    jstr(PROBE_SCHEMA),
                    jstr(GATE_KEY),
                    jstr(&e),
                    jstr(&base_commit()),
                );
                emit(&line, &args);
                fail(&format!("red-arm tamper 失效(漏检): {e}"));
            }
        }
    }

    // ── 全档验证(规模闭集逐档;判据 ①~⑤ 全零容差 + ⑥ throughput 登记)──
    let scales = scales_closed_set(args.scale);
    let mut problems: Vec<String> = Vec::new();
    let mut reports: Vec<ScaleReport> = Vec::new();
    for &n in &scales {
        let r = run_scale(&dev, n);
        if !r.scan_bitexact {
            problems.push(format!("n={n} scan 非位级(①零容差)"));
        }
        if !r.sort_bitexact {
            problems.push(format!("n={n} sort ≠ host sort_pairs_u24(②位级)"));
        }
        if !r.sort_reference_equal {
            problems.push(format!("n={n} sort ≠ 独立参考 sort_pairs_reference(②互核)"));
        }
        if !r.sort_stable {
            problems.push(format!("n={n} 同键 payload 逆序(③稳定性)"));
        }
        if !r.compact_bitexact {
            problems.push(format!("n={n} compact 非位级(④零容差)"));
        }
        if !r.double_run_bitexact {
            problems.push(format!("n={n} device 双跑非位级(⑤)"));
        }
        eprintln!(
            "{TAG}: n={} scan={} sort={} ref={} stable={} dup_pairs={} compact={} double_run={} sort_ms={:.3}",
            r.n,
            r.scan_bitexact,
            r.sort_bitexact,
            r.sort_reference_equal,
            r.sort_stable,
            r.duplicate_pairs,
            r.compact_bitexact,
            r.double_run_bitexact,
            r.sort_ms,
        );
        reports.push(r);
    }
    // ③ 咬合前提:窗内至少一档存在重复键对(否则稳定性判据空转 = 红)。
    let dup_max = reports.iter().map(|r| r.duplicate_pairs).max().unwrap_or(0);
    if dup_max == 0 {
        problems.push("稳定性判据空转:全规模零重复键对(③咬合前提)".into());
    }
    // ⑥ throughput(最大规模 run-A 排序全链墙钟;诚实登记不设通过线)。
    let top = reports
        .last()
        .unwrap_or_else(|| fail("规模闭集为空(构造上不可达)"));
    let keys_per_sec = top.n as f64 / (top.sort_ms / 1000.0);
    let scales_json: Vec<String> = reports.iter().map(|r| r.to_json()).collect();
    let status = if problems.is_empty() { "pass" } else { "fail" };
    let line = format!(
        "{{\"schema\":{},\"status\":{},\"mode\":\"device\",\"gate\":{},\"scales\":[{}],\
         \"scan_bitexact\":{},\"sort_bitexact\":{},\"sort_reference_equal\":{},\
         \"sort_stability\":{},\"stability_duplicate_pairs_max\":{},\"compact_bitexact\":{},\
         \"determinism_double_run\":{},\
         \"throughput\":{{\"keys_per_sec\":{:.1},\"sort_n\":{},\"sort_ms\":{:.3},\"measured\":{}}},\
         \"red_arm_effective\":null,\"base_commit\":{}}}",
        jstr(PROBE_SCHEMA),
        jstr(status),
        jstr(GATE_KEY),
        scales_json.join(","),
        reports.iter().all(|r| r.scan_bitexact),
        reports.iter().all(|r| r.sort_bitexact),
        reports.iter().all(|r| r.sort_reference_equal),
        reports.iter().all(|r| r.sort_stable) && dup_max > 0,
        dup_max,
        reports.iter().all(|r| r.compact_bitexact),
        reports.iter().all(|r| r.double_run_bitexact),
        keys_per_sec,
        top.n,
        top.sort_ms,
        jstr(THROUGHPUT_MEASURED),
        jstr(&base_commit()),
    );
    emit(&line, &args);
    let all_green = reports.iter().all(|r| r.all_green()) && problems.is_empty();
    if !all_green {
        for p in &problems {
            eprintln!("{TAG}: FAIL {p}");
        }
        std::process::exit(1);
    }
}
