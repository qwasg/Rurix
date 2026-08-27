//! G35-7 流体统一物理 device probe harness(门 g35.wave7.fluids;
//! RFC-0049 §4.10;G35_CONTRACT G-G35-7;g35_particle_core_device 同模)。
//!
//! ## 集成路径
//!
//! bin-local 全部逻辑:9 kernel(6 新 = kernels/g35_hash_{cellkey,clear,
//! cellrange}.rx + g35_xpbd_{density,apply,velocity}.rx,3 sort =
//! kernels/g35_sort_{hist,spine,scatter}.rx——W1 冻结面**只消费不修改**)经
//! `rurix_rt::vk::run_compute`(G12/G13/G26/G27 compute 派发面同车道)逐
//! kernel 派发;单帧 device 全链 = **cellkey → W1 sort 3-pass(9 dispatch,
//! 键域 < 2^24)→ hash_clear → cellrange → [density → apply]×3 →
//! velocity**(19 dispatch/帧;apply 迭代 = pos/pos_alt host 侧 Vec 交换
//! 承载 Jacobi ping-pong)。公式面与 host 金标准 `particles/fluid.rs`
//! (hash_cellkey_step/hash_clear/hash_cellrange/xpbd_density_step/
//! xpbd_apply_step/xpbd_velocity_step/fluid_frame)逐字同源;host 平行金标准
//! = `fluid_frame()` 逐帧推进对拍。
//!
//! ## 确定性夹具(冻结)
//!
//! dam-break:`fluid::dam_break_fixture(n, seed, params)`(Pcg32 固定 seed
//! 网格摆放 + ±0.01 扰动;间距 0.4h 压缩初态,块基 origin+(2h,3h,2h),
//! 自由落体 ~21 帧触地 ⇒ 32 帧窗内必有预测越界/负 floor 事件——
//! hash_cell_floor_semantics 咬合);params = `fluid::default_params()`
//! (世界 [0,12.8]³、h=0.2、ρ0=1000、m=1、dt=1/60、g=−9.8);默认 32 帧 ×
//! ITER=3 迭代。
//!
//! ## 单帧对拍协议(冻结;协议改动走契约修订)
//!
//! **帧首注入**:每帧 device 九流(pos/prev/vel)= host 金标准帧首状态原
//! 字节注入,device 19 dispatch 链与 host `fluid_frame()` 消费同一帧首状态
//! 后逐流对拍——隔离**单帧**对拍域。理由如实登记:device `.sqrt()` 为
//! Vulkan 非正确舍入语义面(GLSL.std.450 Sqrt 无 IEEE 保证),spiky 梯度链
//! 单 ULP 种子在 dam-break 触地混沌域(帧 ~17 起)经自由跑跨帧 Lyapunov
//! 放大跨越 cell 边界 ⇒ 整数面零容差在自由跑协议下物理不可达;单帧注入下
//! cell_key 预测路径(prev 保存 + mul/add 半隐式 Euler + FDiv + floor,
//! NoContraction 注入)与 host 逐 op 位级 ⇒ 整数面零容差恒成立域,f32 面 =
//! 单帧发散(有界,标定容差)。device 双跑/RED 臂在同协议下成立(同注入
//! 序列双跑位级 = gather 零原子与线程调度无关的机器证明)。
//!
//! ## 判据面
//!
//! ① 整数流(cell_key/sorted_keys/sorted_idx/cell_start/cell_end)device↔
//! host 逐帧 memcmp **零容差位级**(邻居结构位级 ⇒ 邻居集位级);② f32 流
//! (pos_x/y/z、vel_x/y/z、ρ、λ)逐帧(单帧对拍)max abs diff 聚合全帧
//! p100——probe 只输出 measured(`f32_max_abs_diff`),阈值判读归 smoke
//! (milestones/g35/g35_budget.json `g35.fluids.parity_p100` 标定腿,
//! threshold = measured×2.0 程序产);③ floor/clamp 事件计数(host 登记
//! 语义,fluid.rs 计数器)——负 floor 向负无穷 + 越界 clamp 到边界 cell 的
//! 语义见证面;④ 密度误差 measured 登记(device ρ 流:mean |ρ/ρ0−1| 与
//! mean(max(C,0)) 首/末帧);⑤ device 双跑位级(digest = pos/vel/ρ/λ/
//! cell_key/sorted/cell 区间字节 sha256 逐帧链式;双跑同注入序列);
//! ⑥ frame_ms 登记(19 dispatch 链逐帧墙钟均值;run_compute 每 dispatch
//! 重建 instance/device 开销如实计入,measured_local 登记语义非帧率对标)。
//!
//! ## NoContraction(g14_3_lane_body.rs `spv_inject_no_contraction` 同律
//! bin-local 复制;SPV 文件 0-byte 不动)
//!
//! cellkey/density/apply/velocity 四 kernel 含 f32 乘加链,装载期注入
//! NoContraction 禁驱动 FMA 收缩 ⇒ 与 host 逐 op IEEE 对齐;clear/cellrange/
//! sort 三件纯整数不注入。f32 流仍走标定容差协议(注入为容差收敛手段非判据
//! 替代)。
//!
//! ## 三态 / RED 臂
//!
//! 无 Vulkan loader/设备 → `skipped_dev_env` JSON 退 0(非 fake pass;
//! `RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决);`--host-only`
//! 恒可;`--red-arm rho0-tamper` = 双跑异 ρ0(ρ0 / ρ0×1.05)digest 必异
//! (压缩夹具 C>0 恒真 ⇒ λ 必受 ρ0 影响 ⇒ 位置流必变——证明 digest 判据对
//! 约束求解敏感,防镂空 digest 冒充)。
//!
//! ## 用法
//!
//! ```text
//! g35_fluids_device --spv-hash-cellkey <p> --spv-hash-clear <p>
//!     --spv-hash-cellrange <p> --spv-sort-hist <p> --spv-sort-spine <p>
//!     --spv-sort-scatter <p> --spv-xpbd-density <p> --spv-xpbd-apply <p>
//!     --spv-xpbd-velocity <p> [--frames 32] [--n 4096] [--seed 42]
//!     [--evidence-out <path>] [--report-max-diff|--calibrate]
//! g35_fluids_device --red-arm rho0-tamper --spv-... <9 件>
//! g35_fluids_device --host-only [--frames N] [--n N] [--seed N]
//! ```

#![forbid(unsafe_code)]

use std::time::Instant;

use rurix_render::particles::SEG;
use rurix_render::particles::fluid::{self, FluidParams, FluidState};
use rurix_rt::vk;

const TAG: &str = "[g35_fluids_device]";
const DEFAULT_FRAMES: usize = 32;
const DEFAULT_N: usize = 4096;
const DEFAULT_SEED: u64 = 42;
/// RED 臂 ρ0 篡改因子(冻结;压缩夹具下 λ 必变 ⇒ digest 必异)。
const RED_RHO0_SCALE: f32 = 1.05;
/// 3 pass digit 幂(24 位键 = 3 × 8 bit;primitives.rs 逐字同源)。
const DPOWS: [usize; 3] = [1, 256, 65536];
/// f32 对拍流名(判据 ② 全帧 p100 聚合域)。
const F32_STREAMS: [&str; 8] = [
    "pos_x", "pos_y", "pos_z", "vel_x", "vel_y", "vel_z", "rho", "lambda",
];
/// 整数对拍流名(判据 ① 零容差位级域)。
const INT_STREAMS: [&str; 5] = [
    "cell_key",
    "sorted_keys",
    "sorted_idx",
    "cell_start",
    "cell_end",
];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 字节工具(g27_hzb_device / g35_particle_core_device 先例字面)
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

/// SPIR-V NoContraction 后处理(bin-local 同律复制自
/// `src/bin/g14_3_lane/g14_3_lane_body.rs::spv_inject_no_contraction`——
/// g35_particle_core_device 同律;SPV 文件 0-byte 不动):对全部 OpFAdd/
/// OpFSub/OpFMul 结果 id 注入 `OpDecorate %id NoContraction`,禁驱动
/// mul+add FMA 收缩——GPU 浮点序列与 host 严格 IEEE 逐 op 对齐。
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
    spv_hash_cellkey: Option<String>,
    spv_hash_clear: Option<String>,
    spv_hash_cellrange: Option<String>,
    spv_sort_hist: Option<String>,
    spv_sort_spine: Option<String>,
    spv_sort_scatter: Option<String>,
    spv_xpbd_density: Option<String>,
    spv_xpbd_apply: Option<String>,
    spv_xpbd_velocity: Option<String>,
    frames: usize,
    n: usize,
    seed: u64,
    evidence_out: Option<String>,
    red_arm: Option<String>,
    host_only: bool,
    report_max_diff: bool,
}

fn parse_args() -> Args {
    let mut a = Args {
        spv_hash_cellkey: None,
        spv_hash_clear: None,
        spv_hash_cellrange: None,
        spv_sort_hist: None,
        spv_sort_spine: None,
        spv_sort_scatter: None,
        spv_xpbd_density: None,
        spv_xpbd_apply: None,
        spv_xpbd_velocity: None,
        frames: DEFAULT_FRAMES,
        n: DEFAULT_N,
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
            "--spv-hash-cellkey" => a.spv_hash_cellkey = it.next(),
            "--spv-hash-clear" => a.spv_hash_clear = it.next(),
            "--spv-hash-cellrange" => a.spv_hash_cellrange = it.next(),
            "--spv-sort-hist" => a.spv_sort_hist = it.next(),
            "--spv-sort-spine" => a.spv_sort_spine = it.next(),
            "--spv-sort-scatter" => a.spv_sort_scatter = it.next(),
            "--spv-xpbd-density" => a.spv_xpbd_density = it.next(),
            "--spv-xpbd-apply" => a.spv_xpbd_apply = it.next(),
            "--spv-xpbd-velocity" => a.spv_xpbd_velocity = it.next(),
            "--frames" => {
                a.frames = next_or(&mut it, "--frames")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--frames 非法: {e}")));
            }
            "--n" => {
                a.n = next_or(&mut it, "--n")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--n 非法: {e}")));
            }
            "--seed" => {
                a.seed = next_or(&mut it, "--seed")
                    .parse()
                    .unwrap_or_else(|e| fail(&format!("--seed 非法: {e}")));
            }
            "--evidence-out" => a.evidence_out = it.next(),
            "--red-arm" => a.red_arm = it.next(),
            "--host-only" => a.host_only = true,
            // --calibrate = --report-max-diff 别名(标定腿口径:stdout 打
            // f32_max_abs_diff=<v>,smoke 标定腿消费)。
            "--report-max-diff" | "--calibrate" => a.report_max_diff = true,
            other => fail(&format!("未知参数: {other}")),
        }
    }
    if a.frames == 0 {
        fail("--frames 必须 ≥ 1");
    }
    if a.n < 8 || a.n > SEG * 4096 {
        fail(&format!("--n {} 越域(8 ..= {})", a.n, SEG * 4096));
    }
    a
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

struct DevKernels {
    hash_cellkey: Kernel,
    hash_clear: Kernel,
    hash_cellrange: Kernel,
    sort_hist: Kernel,
    sort_spine: Kernel,
    sort_scatter: Kernel,
    xpbd_density: Kernel,
    xpbd_apply: Kernel,
    xpbd_velocity: Kernel,
}

impl DevKernels {
    fn create(args: &Args) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let need = |o: &Option<String>, k: &str| -> Vec<u32> {
            load_spv(o.as_deref().unwrap_or_else(|| fail(&format!("缺 {k}"))))
        };
        // cellkey/density/apply/velocity 注入 NoContraction(f32 乘加链;
        // 头注 §NoContraction);clear/cellrange/sort 三件纯整数不注入。
        Ok(Self {
            hash_cellkey: kernel_of(
                spv_inject_no_contraction(&need(&args.spv_hash_cellkey, "--spv-hash-cellkey")),
                "hash_cellkey",
            )?,
            hash_clear: kernel_of(need(&args.spv_hash_clear, "--spv-hash-clear"), "hash_clear")?,
            hash_cellrange: kernel_of(
                need(&args.spv_hash_cellrange, "--spv-hash-cellrange"),
                "hash_cellrange",
            )?,
            sort_hist: kernel_of(need(&args.spv_sort_hist, "--spv-sort-hist"), "sort_hist")?,
            sort_spine: kernel_of(need(&args.spv_sort_spine, "--spv-sort-spine"), "sort_spine")?,
            sort_scatter: kernel_of(
                need(&args.spv_sort_scatter, "--spv-sort-scatter"),
                "sort_scatter",
            )?,
            xpbd_density: kernel_of(
                spv_inject_no_contraction(&need(&args.spv_xpbd_density, "--spv-xpbd-density")),
                "xpbd_density",
            )?,
            xpbd_apply: kernel_of(
                spv_inject_no_contraction(&need(&args.spv_xpbd_apply, "--spv-xpbd-apply")),
                "xpbd_apply",
            )?,
            xpbd_velocity: kernel_of(
                spv_inject_no_contraction(&need(&args.spv_xpbd_velocity, "--spv-xpbd-velocity")),
                "xpbd_velocity",
            )?,
        })
    }
}

fn dispatch(k: &Kernel, bufs: &mut [Vec<u8>], groups: u32, what: &str) -> Result<(), String> {
    vk::run_compute(&k.spv, &k.entry, bufs, &[], [groups, 1, 1])
        .map_err(|e| format!("{what} dispatch({groups} groups): {e}"))
}

/// device 侧全缓冲(host 持有 `Vec<Vec<u8>>` 跨 kernel 复用;pos/pos_alt =
/// apply Jacobi ping-pong 对,迭代后 host 侧交换角色)。
struct DevState {
    pos: Vec<Vec<u8>>,
    pos_alt: Vec<Vec<u8>>,
    prev: Vec<Vec<u8>>,
    vel: Vec<Vec<u8>>,
    cell_key: Vec<u8>,
    payload: Vec<u8>,
    sorted_keys: Vec<u8>,
    sorted_idx: Vec<u8>,
    cell_start: Vec<u8>,
    cell_end: Vec<u8>,
    rho: Vec<u8>,
    lambda: Vec<u8>,
}

impl DevState {
    /// 从 host 夹具初始化(pos/prev/vel 九流原字节上传;中间流零初始化)。
    fn from_state(st: &FluidState) -> Self {
        let n = st.n();
        Self {
            pos: vec![
                bytes_f32(&st.pos_x),
                bytes_f32(&st.pos_y),
                bytes_f32(&st.pos_z),
            ],
            pos_alt: (0..3).map(|_| vec![0u8; n * 4]).collect(),
            prev: vec![
                bytes_f32(&st.prev_x),
                bytes_f32(&st.prev_y),
                bytes_f32(&st.prev_z),
            ],
            vel: vec![
                bytes_f32(&st.vel_x),
                bytes_f32(&st.vel_y),
                bytes_f32(&st.vel_z),
            ],
            cell_key: vec![0u8; n * 4],
            payload: vec![0u8; n * 4],
            sorted_keys: vec![0u8; n * 4],
            sorted_idx: vec![0u8; n * 4],
            cell_start: vec![0u8; fluid::GRID_CELLS * 4],
            cell_end: vec![0u8; fluid::GRID_CELLS * 4],
            rho: vec![0u8; n * 4],
            lambda: vec![0u8; n * 4],
        }
    }
}

/// device 排序 3-pass 全链(host `primitives::sort_pairs_u24` 镜像拓扑 =
/// g35_primitives_device::sort_pairs 同律:每 pass hist→spine→scatter,
/// pass 间 ping-pong = host 侧 Vec 交接;W1 三 kernel 只消费不修改)。
fn sort_pairs_device(
    dev: &DevKernels,
    keys: &[u32],
    payload: &[u32],
) -> Result<(Vec<u32>, Vec<u32>), String> {
    let n = keys.len();
    let nseg = n.div_ceil(SEG);
    let groups = nseg as u32;
    let mut k = keys.to_vec();
    let mut p = payload.to_vec();
    for dpow in DPOWS {
        let params = bytes_f32(&[n as f32, nseg as f32, dpow as f32, 0.0]);
        // 阶段 1:hist(SSBO 序 = g35_sort_hist.rx 形参声明序)。
        let mut bufs = vec![bytes_u32(&k), params.clone(), vec![0u8; nseg * 256 * 4]];
        dispatch(&dev.sort_hist, &mut bufs, groups, "sort_hist")?;
        let hist = bufs.swap_remove(2);
        // 阶段 2:spine(单 invocation;digit-major off)。
        let mut bufs = vec![hist, params.clone(), vec![0u8; 256 * nseg * 4]];
        dispatch(&dev.sort_spine, &mut bufs, 1, "sort_spine")?;
        let off = bufs.swap_remove(2);
        // 阶段 3:scatter(双流稳定散射)。
        let mut bufs = vec![
            bytes_u32(&k),
            bytes_u32(&p),
            off,
            params,
            vec![0u8; nseg * 256 * 4],
            vec![0u8; n * 4],
            vec![0u8; n * 4],
        ];
        dispatch(&dev.sort_scatter, &mut bufs, groups, "sort_scatter")?;
        k = read_u32(&bufs[5]);
        p = read_u32(&bufs[6]);
    }
    Ok((k, p))
}

/// device 单帧 19 dispatch 链(帧序 = fluid.rs::fluid_frame 字面:cellkey →
/// sort 3-pass → clear → cellrange → [density→apply]×ITER → velocity;
/// buffers 下标与各 kernel 头注 SSBO 序严格一致)。
fn device_frame(
    dev: &DevKernels,
    st: &mut DevState,
    n: usize,
    p: &FluidParams,
) -> Result<(), String> {
    let groups = n as u32;
    let take = std::mem::take::<Vec<u8>>;
    let ox = p.origin[0];
    let oy = p.origin[1];
    let oz = p.origin[2];
    let h = p.cell_size;
    // 1. hash_cellkey(12 SSBO:params/pos3/prev3/vel3/cell_key/payload)。
    {
        let params = [n as f32, p.dt, p.gravity_y, ox, oy, oz, h, 0.0];
        let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(12);
        bufs.push(bytes_f32(&params));
        for k in 0..3 {
            bufs.push(take(&mut st.pos[k]));
        }
        for k in 0..3 {
            bufs.push(take(&mut st.prev[k]));
        }
        for k in 0..3 {
            bufs.push(take(&mut st.vel[k]));
        }
        bufs.push(take(&mut st.cell_key));
        bufs.push(take(&mut st.payload));
        dispatch(&dev.hash_cellkey, &mut bufs, groups, "hash_cellkey")?;
        for k in 0..3 {
            st.pos[k] = take(&mut bufs[1 + k]);
        }
        for k in 0..3 {
            st.prev[k] = take(&mut bufs[4 + k]);
        }
        for k in 0..3 {
            st.vel[k] = take(&mut bufs[7 + k]);
        }
        st.cell_key = take(&mut bufs[10]);
        st.payload = take(&mut bufs[11]);
    }
    // 2. W1 sort 3-pass(9 dispatch;键域 < 262144 < 2^24)。
    {
        let keys = read_u32(&st.cell_key);
        let payload = read_u32(&st.payload);
        let (sk, si) = sort_pairs_device(dev, &keys, &payload)?;
        st.sorted_keys = bytes_u32(&sk);
        st.sorted_idx = bytes_u32(&si);
    }
    // 3. hash_clear(3 SSBO;dispatch = GRID_CELLS/SEG = 1024 段)。
    {
        let nseg_cells = fluid::GRID_CELLS / SEG;
        let params = bytes_f32(&[fluid::GRID_CELLS as f32, nseg_cells as f32, 0.0, 0.0]);
        let mut bufs = vec![params, take(&mut st.cell_start), take(&mut st.cell_end)];
        dispatch(&dev.hash_clear, &mut bufs, nseg_cells as u32, "hash_clear")?;
        st.cell_start = take(&mut bufs[1]);
        st.cell_end = take(&mut bufs[2]);
    }
    // 4. hash_cellrange(4 SSBO;单写者边界检测)。
    {
        let params = bytes_f32(&[n as f32, 0.0, 0.0, 0.0]);
        let mut bufs = vec![
            params,
            take(&mut st.sorted_keys),
            take(&mut st.cell_start),
            take(&mut st.cell_end),
        ];
        dispatch(&dev.hash_cellrange, &mut bufs, groups, "hash_cellrange")?;
        st.sorted_keys = take(&mut bufs[1]);
        st.cell_start = take(&mut bufs[2]);
        st.cell_end = take(&mut bufs[3]);
    }
    // 5. [density → apply] × ITER(apply 后 pos/pos_alt 交换 = Jacobi
    //    ping-pong;系数 host 单源程序产经 params 传入)。
    let poly6 = fluid::poly6_coef(h);
    let spiky = fluid::spiky_grad_coef(h);
    for _ in 0..fluid::ITER {
        {
            let params = [
                n as f32, ox, oy, oz, h, p.rho0, p.mass, poly6, spiky, 0.0, 0.0, 0.0,
            ];
            let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(9);
            bufs.push(bytes_f32(&params));
            for k in 0..3 {
                bufs.push(take(&mut st.pos[k]));
            }
            bufs.push(take(&mut st.sorted_idx));
            bufs.push(take(&mut st.cell_start));
            bufs.push(take(&mut st.cell_end));
            bufs.push(take(&mut st.rho));
            bufs.push(take(&mut st.lambda));
            dispatch(&dev.xpbd_density, &mut bufs, groups, "xpbd_density")?;
            for k in 0..3 {
                st.pos[k] = take(&mut bufs[1 + k]);
            }
            st.sorted_idx = take(&mut bufs[4]);
            st.cell_start = take(&mut bufs[5]);
            st.cell_end = take(&mut bufs[6]);
            st.rho = take(&mut bufs[7]);
            st.lambda = take(&mut bufs[8]);
        }
        {
            let params = [n as f32, ox, oy, oz, h, p.rho0, spiky, 0.0];
            let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(11);
            bufs.push(bytes_f32(&params));
            for k in 0..3 {
                bufs.push(take(&mut st.pos[k]));
            }
            bufs.push(take(&mut st.lambda));
            bufs.push(take(&mut st.sorted_idx));
            bufs.push(take(&mut st.cell_start));
            bufs.push(take(&mut st.cell_end));
            for k in 0..3 {
                bufs.push(take(&mut st.pos_alt[k]));
            }
            dispatch(&dev.xpbd_apply, &mut bufs, groups, "xpbd_apply")?;
            for k in 0..3 {
                st.pos[k] = take(&mut bufs[1 + k]);
            }
            st.lambda = take(&mut bufs[4]);
            st.sorted_idx = take(&mut bufs[5]);
            st.cell_start = take(&mut bufs[6]);
            st.cell_end = take(&mut bufs[7]);
            for k in 0..3 {
                st.pos_alt[k] = take(&mut bufs[8 + k]);
            }
            std::mem::swap(&mut st.pos, &mut st.pos_alt);
        }
    }
    // 6. xpbd_velocity(10 SSBO;PBF 速度更新 + 边界置零分量)。
    {
        let params = [n as f32, p.dt, ox, oy, oz, h, 0.0, 0.0];
        let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(10);
        bufs.push(bytes_f32(&params));
        for k in 0..3 {
            bufs.push(take(&mut st.pos[k]));
        }
        for k in 0..3 {
            bufs.push(take(&mut st.prev[k]));
        }
        for k in 0..3 {
            bufs.push(take(&mut st.vel[k]));
        }
        dispatch(&dev.xpbd_velocity, &mut bufs, groups, "xpbd_velocity")?;
        for k in 0..3 {
            st.pos[k] = take(&mut bufs[1 + k]);
        }
        for k in 0..3 {
            st.prev[k] = take(&mut bufs[4 + k]);
        }
        for k in 0..3 {
            st.vel[k] = take(&mut bufs[7 + k]);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 全链单跑(host 平行金标准逐帧对拍 + 链式 digest)
// ---------------------------------------------------------------------------

struct ChainReport {
    integer_bitexact: bool,
    f32_stream_max: [f32; 8],
    negative_floor_events: u64,
    clamp_events: u64,
    density_abs_err_first: f64,
    density_abs_err_last: f64,
    density_pos_constraint_first: f64,
    density_pos_constraint_last: f64,
    digest: String,
    frame_ms_mean: f64,
    problems: Vec<String>,
}

fn run_chain(dev: &DevKernels, seed: u64, frames: usize, n: usize, p: &FluidParams) -> ChainReport {
    let mut hst = fluid::dam_break_fixture(n, seed, p);
    let mut st = DevState::from_state(&hst);
    let mut r = ChainReport {
        integer_bitexact: true,
        f32_stream_max: [0.0; 8],
        negative_floor_events: 0,
        clamp_events: 0,
        density_abs_err_first: 0.0,
        density_abs_err_last: 0.0,
        density_pos_constraint_first: 0.0,
        density_pos_constraint_last: 0.0,
        digest: "0".repeat(64),
        frame_ms_mean: 0.0,
        problems: Vec::new(),
    };
    let mut ms_total = 0.0f64;
    let problem = |problems: &mut Vec<String>, msg: String| {
        if problems.len() < 16 {
            problems.push(msg);
        }
    };
    for f in 0..frames {
        // ── 帧首注入(单帧对拍协议,头注冻结):device 九流 = host 金标准
        //    帧首状态原字节 ──
        st.pos[0] = bytes_f32(&hst.pos_x);
        st.pos[1] = bytes_f32(&hst.pos_y);
        st.pos[2] = bytes_f32(&hst.pos_z);
        st.prev[0] = bytes_f32(&hst.prev_x);
        st.prev[1] = bytes_f32(&hst.prev_y);
        st.prev[2] = bytes_f32(&hst.prev_z);
        st.vel[0] = bytes_f32(&hst.vel_x);
        st.vel[1] = bytes_f32(&hst.vel_y);
        st.vel[2] = bytes_f32(&hst.vel_z);
        // ── device 19 dispatch 链(墙钟计时)──
        let t0 = Instant::now();
        device_frame(dev, &mut st, n, p).unwrap_or_else(|e| fail(&format!("帧 {f}: {e}")));
        ms_total += t0.elapsed().as_secs_f64() * 1000.0;
        // ── host 金标准 fluid_frame() 平行推进(消费同一帧首状态)──
        let tr = fluid::fluid_frame(&mut hst, p);
        r.negative_floor_events += tr.negative_floor_events;
        r.clamp_events += tr.clamp_events;
        // ── 整数流零容差对拍(cell_key/sorted_keys/sorted_idx/cell_start/
        //    cell_end 逐帧 memcmp)──
        let int_pairs: [(&[u8], Vec<u8>); 5] = [
            (&st.cell_key, bytes_u32(&tr.keys)),
            (&st.sorted_keys, bytes_u32(&tr.sorted_keys)),
            (&st.sorted_idx, bytes_u32(&tr.sorted_idx)),
            (&st.cell_start, bytes_u32(&tr.cell_start)),
            (&st.cell_end, bytes_u32(&tr.cell_end)),
        ];
        for (name, (dev_b, host_b)) in INT_STREAMS.iter().zip(int_pairs.iter()) {
            if dev_b[..] != host_b[..] {
                r.integer_bitexact = false;
                problem(&mut r.problems, format!("帧 {f}: {name} 非位级"));
            }
        }
        // ── f32 流 max abs diff(全帧 p100 聚合;probe 只测不判)──
        let host_f32: [&[f32]; 8] = [
            &hst.pos_x, &hst.pos_y, &hst.pos_z, &hst.vel_x, &hst.vel_y, &hst.vel_z, &tr.rho,
            &tr.lambda,
        ];
        let dev_f32: [&[u8]; 8] = [
            &st.pos[0], &st.pos[1], &st.pos[2], &st.vel[0], &st.vel[1], &st.vel[2], &st.rho,
            &st.lambda,
        ];
        for k in 0..8 {
            let dv = read_f32(dev_f32[k]);
            for i in 0..n {
                let mut d = (dv[i] - host_f32[k][i]).abs();
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
        // ── 密度误差 measured 登记(device ρ 流;首/末帧)──
        let dev_rho = read_f32(&st.rho);
        let abs_err = fluid::mean_density_error(&dev_rho, p.rho0);
        let pos_c = fluid::mean_positive_constraint(&dev_rho, p.rho0);
        if f == 0 {
            r.density_abs_err_first = abs_err;
            r.density_pos_constraint_first = pos_c;
        }
        r.density_abs_err_last = abs_err;
        r.density_pos_constraint_last = pos_c;
        // ── 链式 digest(pos3 ‖ vel3 ‖ ρ ‖ λ ‖ cell_key ‖ sorted_keys ‖
        //    sorted_idx ‖ cell_start ‖ cell_end;sha256(prev_hex ‖ bytes))──
        let mut trace: Vec<u8> =
            Vec::with_capacity(64 + n * 44 + fluid::GRID_CELLS * 8);
        trace.extend_from_slice(r.digest.as_bytes());
        for k in 0..3 {
            trace.extend_from_slice(&st.pos[k]);
        }
        for k in 0..3 {
            trace.extend_from_slice(&st.vel[k]);
        }
        trace.extend_from_slice(&st.rho);
        trace.extend_from_slice(&st.lambda);
        trace.extend_from_slice(&st.cell_key);
        trace.extend_from_slice(&st.sorted_keys);
        trace.extend_from_slice(&st.sorted_idx);
        trace.extend_from_slice(&st.cell_start);
        trace.extend_from_slice(&st.cell_end);
        r.digest = rurix_pkg::sha256::hex_digest(&trace);
    }
    r.frame_ms_mean = ms_total / frames as f64;
    r
}

impl ChainReport {
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
// host-only 腿(host 金标准链恒可跑:AABB/有限性不变量 + 双跑位级)
// ---------------------------------------------------------------------------

fn host_only_leg(args: &Args) -> ! {
    let p = fluid::default_params();
    let hi = fluid::world_max(&p);
    let mut problems: Vec<String> = Vec::new();
    let run = |problems: &mut Vec<String>| -> (Vec<u32>, f64, f64) {
        let mut st = fluid::dam_break_fixture(args.n, args.seed, &p);
        let mut bits: Vec<u32> = Vec::new();
        let mut pos_first = 0.0f64;
        let mut pos_last = 0.0f64;
        for f in 0..args.frames {
            let tr = fluid::fluid_frame(&mut st, &p);
            for i in 0..st.n() {
                let inside = st.pos_x[i] >= p.origin[0]
                    && st.pos_x[i] <= hi[0]
                    && st.pos_y[i] >= p.origin[1]
                    && st.pos_y[i] <= hi[1]
                    && st.pos_z[i] >= p.origin[2]
                    && st.pos_z[i] <= hi[2];
                if !inside && problems.len() < 8 {
                    problems.push(format!("帧 {f}: 粒子 {i} 帧末越界(clamp 不变量破)"));
                }
                if !(tr.rho[i].is_finite() && st.vel_x[i].is_finite()) && problems.len() < 8 {
                    problems.push(format!("帧 {f}: 粒子 {i} ρ/vel 非有限"));
                }
            }
            let e = fluid::mean_positive_constraint(&tr.rho, p.rho0);
            if f == 0 {
                pos_first = e;
            }
            pos_last = e;
            bits.extend(tr.sorted_keys.iter());
            bits.extend(tr.sorted_idx.iter());
            bits.extend(st.pos_x.iter().map(|v| v.to_bits()));
            bits.extend(st.pos_y.iter().map(|v| v.to_bits()));
            bits.extend(st.pos_z.iter().map(|v| v.to_bits()));
            bits.extend(st.vel_x.iter().map(|v| v.to_bits()));
            bits.extend(st.vel_y.iter().map(|v| v.to_bits()));
            bits.extend(st.vel_z.iter().map(|v| v.to_bits()));
        }
        (bits, pos_first, pos_last)
    };
    let (bits_a, first, last) = run(&mut problems);
    let (bits_b, _, _) = run(&mut problems);
    if bits_a != bits_b {
        problems.push("host 双跑非位级".into());
    }
    let ok = problems.is_empty();
    let state = if ok { "pass" } else { "fail" };
    let line = format!(
        "{{\"schema\":\"rurix.g35.fluids_host.v1\",\"mode\":\"host-only\",\"state\":{},\
         \"frames\":{},\"n\":{},\"seed\":{},\"density_pos_constraint_first\":{:e},\
         \"density_pos_constraint_last\":{:e},\"problems\":{},\"base_commit\":{}}}",
        jstr(state),
        args.frames,
        args.n,
        args.seed,
        first,
        last,
        strs_json(&problems),
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    std::process::exit(i32::from(!ok))
}

// ---------------------------------------------------------------------------
// main(默认 = 全档验证:双跑同 seed;--red-arm rho0-tamper = 双跑异 ρ0)
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
                "{{\"schema\":\"rurix.g35.fluids_probe.v1\",\"state\":\"skipped_dev_env\",\
                 \"reason\":{}}}",
                jstr(&e)
            );
            emit_evidence(&line, &args.evidence_out);
            std::process::exit(0);
        }
    };

    let params = fluid::default_params();

    if let Some(arm) = &args.red_arm {
        if arm != "rho0-tamper" {
            fail(&format!("未知 RED 臂: {arm}(rho0-tamper)"));
        }
        // RED 臂:ρ0 篡改双跑 digest 必异(压缩夹具 C>0 恒真 ⇒ λ 必受 ρ0
        // 影响 ⇒ 位置流必变——digest 判据对约束求解敏感性证明)。
        let mut tampered = params;
        tampered.rho0 = params.rho0 * RED_RHO0_SCALE;
        let g = run_chain(&dev, args.seed, args.frames, args.n, &params);
        let r = run_chain(&dev, args.seed, args.frames, args.n, &tampered);
        let detected = g.digest != r.digest;
        let line = format!(
            "{{\"schema\":\"rurix.g35.fluids_red_arm.v1\",\"arm\":\"rho0-tamper\",\
             \"detected\":{detected},\"rho0_green\":{:e},\"rho0_red\":{:e},\
             \"digest_green\":{},\"digest_red\":{}}}",
            params.rho0,
            tampered.rho0,
            jstr(&format!("sha256:{}", g.digest)),
            jstr(&format!("sha256:{}", r.digest)),
        );
        emit_evidence(&line, &args.evidence_out);
        if !detected {
            fail("red-arm rho0-tamper 失效(漏检):ρ0 篡改后 digest 未变");
        }
        eprintln!("{TAG}: red-arm rho0-tamper 检出 — digest 已异");
        std::process::exit(0);
    }

    // ── 全档验证:双跑同 seed(判据 ⑤ device 双跑位级)+ 逐帧对拍(①~④)──
    let a = run_chain(&dev, args.seed, args.frames, args.n, &params);
    let b = run_chain(&dev, args.seed, args.frames, args.n, &params);
    let determinism = a.digest == b.digest;
    let f32_p100 = a.f32_stream_max.iter().copied().fold(0.0f32, f32::max);
    let state = if a.integer_bitexact && determinism && a.problems.is_empty() {
        "pass"
    } else {
        "fail"
    };
    if args.report_max_diff {
        println!("f32_max_abs_diff={f32_p100:e}");
    }
    eprintln!(
        "{TAG}: {} frames={} n={} seed={} int_bitexact={} f32_p100={:e} floor_neg={} clamp={} \
         double_run={} rho_err_first={:.4} rho_err_last={:.4} frame_ms={:.3}",
        state,
        args.frames,
        args.n,
        args.seed,
        a.integer_bitexact,
        f32_p100,
        a.negative_floor_events,
        a.clamp_events,
        determinism,
        a.density_abs_err_first,
        a.density_abs_err_last,
        a.frame_ms_mean,
    );
    let mut problems = a.problems.clone();
    if !determinism {
        problems.push("device 双跑 digest 非位级一致".into());
    }
    let line = format!(
        "{{\"schema\":\"rurix.g35.fluids_probe.v1\",\"state\":{},\
         \"parity_protocol\":\"per-frame-host-state-injection\",\
         \"frames\":{},\"n\":{},\"seed\":{},\"dt\":{:e},\"cell_size\":{:e},\"rho0\":{:e},\
         \"mass\":{:e},\"gravity_y\":{:e},\"grid\":{},\"iter\":{},\
         \"integer_streams\":[\"cell_key\",\"sorted_keys\",\"sorted_idx\",\"cell_start\",\"cell_end\"],\
         \"integer_streams_bitexact\":{},\
         \"f32_max_abs_diff\":{:e},\"f32_stream_max\":{},\
         \"negative_floor_events\":{},\"clamp_events\":{},\
         \"density_mean_abs_err_first\":{:e},\"density_mean_abs_err_last\":{:e},\
         \"density_pos_constraint_first\":{:e},\"density_pos_constraint_last\":{:e},\
         \"determinism_double_run\":{},\"digest_a\":{},\"digest_b\":{},\
         \"frame_ms_mean\":{:.6},\
         \"nocontraction_injected\":[\"g35_hash_cellkey\",\"g35_xpbd_density\",\"g35_xpbd_apply\",\"g35_xpbd_velocity\"],\
         \"problems\":{},\"base_commit\":{}}}",
        jstr(state),
        args.frames,
        args.n,
        args.seed,
        params.dt,
        params.cell_size,
        params.rho0,
        params.mass,
        params.gravity_y,
        fluid::GRID,
        fluid::ITER,
        a.integer_bitexact,
        f32_p100,
        a.stream_max_json(),
        a.negative_floor_events,
        a.clamp_events,
        a.density_abs_err_first,
        a.density_abs_err_last,
        a.density_pos_constraint_first,
        a.density_pos_constraint_last,
        determinism,
        jstr(&format!("sha256:{}", a.digest)),
        jstr(&format!("sha256:{}", b.digest)),
        a.frame_ms_mean,
        strs_json(&problems),
        jstr(&base_commit()),
    );
    emit_evidence(&line, &args.evidence_out);
    if state != "pass" {
        std::process::exit(1);
    }
}
