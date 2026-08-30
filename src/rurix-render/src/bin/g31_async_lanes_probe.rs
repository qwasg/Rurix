// Assisted-by: Cursor Agent(G37 W3 深水区:异步 compute 三件套判档实施窗)
//! G31+ 异步 compute 三件套(#57/#59/#60/#62,合流前置 #88)**判档 harness**。
//!
//! 设计事实源 = `artifacts/day_0830_delivery/w3_deep/async/PLAN.md`(+ 同目录
//! PATCH_PROPOSAL / RFC_DRAFT);实施记录 = 同目录 `IMPL_REPORT.md`。独立 probe
//! bin,不进 CI 门、不进生产车道;device 腿经 rurix-rt `vk::run_async_lanes`
//! (G37 W3 async 加性面)真跑,本 bin 维持 `#![forbid(unsafe_code)]`。
//!
//! ## 三臂判档(PLAN §2.2/§2.5)
//!
//! 1. **arm_single**:`enable_async=false` **重编译**(显式 single-queue plan,
//!    RFC-0019 §4.8.3;非忽略 fence——趟 3 屏障 stage 随车道重推)+ 单队列执行,
//!    双跑(噪声地板)。
//! 2. **arm_dual**:`enable_async=true` 编译 → 段切分([`plan_submission_segments`])
//!    → **timeline 值域合法化**([`legalize_submission`]:配对/全序/值回退提交前
//!    validator,RFC_DRAFT 修订行 3)→ 双队列(graphics + compute-only)+ 单条
//!    timeline semaphore 执行,双跑(位级重跑一致)。
//! 3. **等价门(硬前置)**:两臂全部输出资源 readback sha256 逐字节相等 + dual
//!    双跑位级一致 + 每跑首帧/末帧一致(竞态金丝雀);不等 = RED,整窗不判收益。
//!
//! 无 compute-only family / 无 timeline → **显式单队列回落**(off 臂重编译产物,
//! evidence 标 `single_queue_fallback`,不充多队列绿)。
//!
//! ## workload(uc06 异步三 pass 形状镜像;#60 首批白名单)
//!
//! `gi_probe_trace` / `rtao` / `hard_shadow` 标 AsyncCompute,`ao_filter` 回图形;
//! pass 工作 = 确定性整数迭代 kernel(手编 SPIR-V,[`async_probe_kernel_spv`];
//! CPU 参照 [`cpu_reference_buffers`] 同式)——`--scale N` 参数化异步段时长
//! (判据:异步段 ≥0.5ms,报告5 三条件之一)。真 AO/GI kernel 接线属 go 后实施窗。
//!
//! ## 用法
//!
//! ```text
//! g31_async_lanes_probe [--selftest] [--judge] [--frames N] [--warmup N]
//!                       [--scale N] [--out FILE]
//! ```
//!
//! - `--selftest`:纯 CPU 自证(段切分/合法化红绿/回落重编译/kernel 参照),零 GPU。
//! - `--judge`:输出 M59 重判两态结论(PLAN §2.5:硬前置 digest 等价;go = 中位
//!   改善 ≥3% ∧ ≥0.15ms ∧ 重叠率 ≥50%;噪声门同臂双跑中位差 <1%)。
//! - 无设备/无 loader → `skipped_dev_env` 三态退 0(`RURIX_REQUIRE_REAL=1` 翻硬红)。

#![forbid(unsafe_code)]

use rurix_render::graph::types::{
    AccessKind, PassDesc, PassId, QueueClass, ResAccess, ResourceDesc, ResourceId, ResourceKind,
    TextureFormat,
};
use rurix_render::graph::{CompileOptions, CompiledGraph, RenderGraph};
use rurix_rt::vk as rvk;

// ---------------------------------------------------------------------------
// 判档帧图(uc06 异步三 pass 形状镜像;分辨率仅池 size 估算用)
// ---------------------------------------------------------------------------

const W: u32 = 640;
const H: u32 = 360;
/// 每资源 buffer 元素数(u32;device 判档 workload 与图分辨率同域)。
const ELEM_COUNT: u32 = W * H;

fn tex(name: &str, format: TextureFormat) -> ResourceDesc {
    ResourceDesc {
        name: name.to_owned(),
        kind: ResourceKind::Texture2d {
            width: W,
            height: H,
            format,
            mip_levels: 1,
        },
        imported: false,
    }
}

fn ra(res: ResourceId, access: AccessKind) -> ResAccess {
    ResAccess { res, access }
}

fn pass(name: &str, queue: QueueClass, reads: Vec<ResAccess>, writes: Vec<ResAccess>) -> PassDesc {
    PassDesc {
        name: name.to_owned(),
        queue,
        reads,
        writes,
    }
}

/// 建判档图(线性序 = 依赖语义序,uc06 形状裁剪到 8 pass):
/// 0 `gbuffer`(gfx)→ 1 `vsm_page_mark`(gfx,写 imported 页表)→
/// 2 `gi_probe_trace`(**async**)→ 3 `rtao`(**async**)→ 4 `hard_shadow`(**async**)→
/// 5 `ao_filter`(gfx)→ 6 `deferred_shade`(gfx)→ 7 `blit`(gfx,imported 根)。
///
/// 预期 fence 弧(趟 4 去重后):`(0→5, v=1)`(rtao 产出经 ao_filter 消费)与
/// `(0→6, v=2)`(gi/shadow 产出经 deferred_shade 消费)。
fn build_probe_graph() -> RenderGraph {
    let mut g = RenderGraph::new();
    let albedo = g.create(tex("gbuf:Albedo", TextureFormat::Rgba8Unorm));
    let normal = g.create(tex("gbuf:Normal", TextureFormat::Rgba16Float));
    let depth = g.create(tex("gbuf:Depth", TextureFormat::Depth32Float));
    let vsm_marks = g.import(tex("vsm:PageMarks", TextureFormat::R32Uint));
    let gi_probes = g.create(tex("gi:Probes", TextureFormat::Rgba16Float));
    let ao_raw = g.create(tex("ao:Raw", TextureFormat::Rgba8Unorm));
    let shadow_mask = g.create(tex("shadow:Mask", TextureFormat::Rgba8Unorm));
    let ao_filtered = g.create(tex("ao:Filtered", TextureFormat::Rgba8Unorm));
    let hdr = g.create(tex("shade:Hdr", TextureFormat::Rgba16Float));
    let backbuffer = g.import(tex("backbuffer", TextureFormat::Rgba8Unorm));

    g.add_pass(pass(
        "gbuffer",
        QueueClass::Graphics,
        vec![],
        vec![
            ra(albedo, AccessKind::ColorTarget),
            ra(normal, AccessKind::ColorTarget),
            ra(depth, AccessKind::DepthTarget),
        ],
    )); // 0
    g.add_pass(pass(
        "vsm_page_mark",
        QueueClass::Graphics,
        vec![ra(depth, AccessKind::DepthRead)],
        vec![ra(vsm_marks, AccessKind::ShaderWrite)],
    )); // 1
    g.add_pass(pass(
        "gi_probe_trace",
        QueueClass::AsyncCompute,
        vec![
            ra(normal, AccessKind::ShaderRead),
            ra(depth, AccessKind::DepthRead),
        ],
        vec![ra(gi_probes, AccessKind::ShaderWrite)],
    )); // 2
    g.add_pass(pass(
        "rtao",
        QueueClass::AsyncCompute,
        vec![
            ra(normal, AccessKind::ShaderRead),
            ra(depth, AccessKind::DepthRead),
        ],
        vec![ra(ao_raw, AccessKind::ShaderWrite)],
    )); // 3
    g.add_pass(pass(
        "hard_shadow",
        QueueClass::AsyncCompute,
        vec![ra(depth, AccessKind::DepthRead)],
        vec![ra(shadow_mask, AccessKind::ShaderWrite)],
    )); // 4
    g.add_pass(pass(
        "ao_filter",
        QueueClass::Graphics,
        vec![ra(ao_raw, AccessKind::ShaderRead)],
        vec![ra(ao_filtered, AccessKind::ShaderWrite)],
    )); // 5
    g.add_pass(pass(
        "deferred_shade",
        QueueClass::Graphics,
        vec![
            ra(albedo, AccessKind::ShaderRead),
            ra(gi_probes, AccessKind::ShaderRead),
            ra(ao_filtered, AccessKind::ShaderRead),
            ra(shadow_mask, AccessKind::ShaderRead),
        ],
        vec![ra(hdr, AccessKind::ColorTarget)],
    )); // 6
    g.add_pass(pass(
        "blit",
        QueueClass::Graphics,
        vec![ra(hdr, AccessKind::ShaderRead)],
        vec![ra(backbuffer, AccessKind::ColorTarget)],
    )); // 7
    g
}

// ---------------------------------------------------------------------------
// 段切分参考实现(host 纯函数;执行器逐字消费,禁二次推导)
// ---------------------------------------------------------------------------

/// 提交段:同车道连续 pass 子列 + timeline 等待/信号点(值域 = `FencePair.value = v`
/// 的确定性映射 `(2v-1, 2v)`,PLAN §2.1-3;同队列信号严格递增,RFC-0019 §4.8.2 判据)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct SubmissionSegment {
    queue: QueueClass,
    passes: Vec<PassId>,
    /// 段首 wait 的 timeline 值(精确值等待;跨队列 wait-before-signal 合法)。
    wait_points: Vec<u64>,
    /// 段末 signal 的 timeline 值。
    signal_points: Vec<u64>,
}

/// 线性序 → 双队列提交段。切点:①车道翻转 ②`signal_after` pass 之后
/// ③`wait_before` pass 之前。点挂接(每弧 `(s, w, v)`):
/// 图形生产段(段尾 = `s`)signal `2v-1`;首个位于 `s` 之后的异步段 wait `2v-1`;
/// 末个位于 `w` 之前的异步段 signal `2v`;图形消费段(段首 = `w`)wait `2v`。
///
/// 覆盖判档形状(弧窗内异步 run 连续)。交错形状(弧窗内被图形 pass 割裂的多异步段)
/// 的一般化留 go 后实施窗;提交前 validator 见 [`legalize_submission`]。
fn plan_submission_segments(compiled: &CompiledGraph) -> Vec<SubmissionSegment> {
    use std::collections::BTreeSet;
    let passes = compiled.passes();
    let fences = compiled.fences();
    let signal_cuts: BTreeSet<u32> = fences.iter().map(|f| f.signal_after.0).collect();
    let wait_cuts: BTreeSet<u32> = fences.iter().map(|f| f.wait_before.0).collect();

    // 切段。
    let mut segments: Vec<SubmissionSegment> = Vec::new();
    for (i, p) in passes.iter().enumerate() {
        let cut_before = i == 0
            || passes[i - 1].queue() != p.queue()
            || wait_cuts.contains(&p.id().0)
            || signal_cuts.contains(&passes[i - 1].id().0);
        if cut_before {
            segments.push(SubmissionSegment {
                queue: p.queue(),
                passes: Vec::new(),
                wait_points: Vec::new(),
                signal_points: Vec::new(),
            });
        }
        segments
            .last_mut()
            .expect("首 pass 恒切段,segments 非空")
            .passes
            .push(p.id());
    }

    // 点挂接。
    for f in fences {
        let sig_producer = 2 * f.value - 1;
        let sig_async = 2 * f.value;
        if let Some(seg) = segments
            .iter_mut()
            .find(|s| s.queue == QueueClass::Graphics && s.passes.last() == Some(&f.signal_after))
        {
            seg.signal_points.push(sig_producer);
        }
        if let Some(seg) = segments
            .iter_mut()
            .find(|s| s.queue == QueueClass::Graphics && s.passes.first() == Some(&f.wait_before))
        {
            seg.wait_points.push(sig_async);
        }
        if let Some(seg) = segments.iter_mut().find(|s| {
            s.queue == QueueClass::AsyncCompute
                && s.passes.first().is_some_and(|p| p.0 > f.signal_after.0)
        }) {
            seg.wait_points.push(sig_producer);
        }
        if let Some(seg) = segments.iter_mut().rev().find(|s| {
            s.queue == QueueClass::AsyncCompute
                && s.passes.last().is_some_and(|p| p.0 < f.wait_before.0)
        }) {
            seg.signal_points.push(sig_async);
        }
    }
    for seg in &mut segments {
        seg.wait_points.sort_unstable();
        seg.wait_points.dedup();
        seg.signal_points.sort_unstable();
        seg.signal_points.dedup();
    }
    segments
}

// ---------------------------------------------------------------------------
// timeline 值域合法化(提交前 validator;RFC_DRAFT 修订行 3「半对/漏 wait/错值/
// 值回退 = 提交前确定性 RED」的 host 面)
// ---------------------------------------------------------------------------
//
// 本窗 measured 发现(登记进 IMPL_REPORT):`(2v-1, 2v)` 逐弧点映射在**共享生产者**
// 弧形(判档形状两弧 signal_after 同为 pass 0)下不可直接提交单条 timeline——
// Vulkan timeline 语义为「wait 于 counter ≥ 值即满足 + signal 值全局严格递增」,
// 弧点 {1,3} 同段签发后异步段再签 2 即值回退(非法),且 3 会提前解锁 wait(2)。
// 故提交面按**段级信号事件**合法化:signal 事件须全序(happens-before 链),链序
// 赋值 1..n;wait 取其生产段合法化值的 max(≥ 语义 + 生产者全序 ⇒ max 蕴含全部);
// 线性序尾段追加 frame-end 信号(host 帧末等待锚)。原 (2v-1,2v) 点保留在 plan
// evidence(语义层),合法化值为提交层;两层映射均进 receipt。

/// 合法化后的提交段(vk 执行器逐字消费形)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct LegalizedSegment {
    queue: QueueClass,
    passes: Vec<PassId>,
    wait_value: Option<u64>,
    signal_value: Option<u64>,
}

/// 合法化产物:段表 + 每帧值域跨度(= 末信号值,含 frame-end)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct LegalizedSubmission {
    segments: Vec<LegalizedSegment>,
    span: u64,
}

/// timeline 值域合法化 + 提交前 validator(纯 host,确定性):
///
/// 1. **配对核验**:每个 wait 点须恰有一个签发段(漏 signal/半对 = Err);每个
///    signal 点须有等待者(孤儿 signal = Err);同点双签 = Err。
/// 2. **全序核验**:签发段间须构成 happens-before 链(同队列提交序 + 弧边可达);
///    非全序 ⇒ 单条 timeline 值域不可表达(值回退风险)= Err。
/// 3. **赋值**:链序签发段依次取 1..n;wait 取生产段值 max;线性序尾段(须无
///    signal 点且可达自全部签发段)追加 frame-end 信号 n+1;span = n+1。
/// 4. 零 fence 计划(off 臂):span=0、全 None(单队列形态,无 timeline)。
fn legalize_submission(segs: &[SubmissionSegment]) -> Result<LegalizedSubmission, String> {
    use std::collections::BTreeMap;
    let n = segs.len();
    if n == 0 {
        return Err("空段表".into());
    }
    let no_points = segs
        .iter()
        .all(|s| s.wait_points.is_empty() && s.signal_points.is_empty());
    if no_points {
        if segs.iter().any(|s| s.queue == QueueClass::AsyncCompute) {
            return Err("零 fence 计划含异步段(装配矛盾)".into());
        }
        return Ok(LegalizedSubmission {
            segments: segs
                .iter()
                .map(|s| LegalizedSegment {
                    queue: s.queue,
                    passes: s.passes.clone(),
                    wait_value: None,
                    signal_value: None,
                })
                .collect(),
            span: 0,
        });
    }

    // 1) 配对:点 → 签发段;wait → 生产段边。
    let mut signal_owner: BTreeMap<u64, usize> = BTreeMap::new();
    for (i, s) in segs.iter().enumerate() {
        for &p in &s.signal_points {
            if signal_owner.insert(p, i).is_some() {
                return Err(format!("timeline 点 {p} 双签(同点两段 signal)"));
            }
        }
    }
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut waited: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    for (i, s) in segs.iter().enumerate() {
        for &w in &s.wait_points {
            let Some(&owner) = signal_owner.get(&w) else {
                return Err(format!("wait 点 {w} 无签发段(漏 signal/半对)"));
            };
            if owner == i {
                return Err(format!("段 {i} 自等待点 {w}"));
            }
            edges.push((owner, i));
            waited.insert(w);
        }
    }
    for (&p, &owner) in &signal_owner {
        if !waited.contains(&p) {
            return Err(format!("signal 点 {p}(段 {owner})无等待者(孤儿 signal/半对)"));
        }
    }

    // 2) happens-before 可达闭包:弧边 + 同队列提交序(相邻链)。
    let mut reach = vec![vec![false; n]; n];
    for &(a, b) in &edges {
        reach[a][b] = true;
    }
    for q in [QueueClass::Graphics, QueueClass::AsyncCompute] {
        let idxs: Vec<usize> = (0..n).filter(|&i| segs[i].queue == q).collect();
        for w in idxs.windows(2) {
            reach[w[0]][w[1]] = true;
        }
    }
    for k in 0..n {
        for i in 0..n {
            if reach[i][k] {
                for j in 0..n {
                    if reach[k][j] {
                        reach[i][j] = true;
                    }
                }
            }
        }
    }
    // 签发段(线性序)须构成链。
    let signal_segs: Vec<usize> = (0..n).filter(|&i| !segs[i].signal_points.is_empty()).collect();
    for w in signal_segs.windows(2) {
        if !reach[w[0]][w[1]] {
            return Err(format!(
                "signal 事件非全序(段 {} 与段 {} 无 happens-before)——单 timeline 值域不可表达(值回退风险)",
                w[0], w[1]
            ));
        }
    }

    // 3) 赋值 + frame-end。
    let mut value_of_seg: BTreeMap<usize, u64> = BTreeMap::new();
    for (k, &si) in signal_segs.iter().enumerate() {
        value_of_seg.insert(si, (k + 1) as u64);
    }
    let n_signals = signal_segs.len() as u64;
    let last = n - 1;
    if !segs[last].signal_points.is_empty() {
        return Err("线性序尾段已带 signal 点,frame-end 语义冲突".into());
    }
    for &si in &signal_segs {
        if !reach[si][last] {
            return Err(format!(
                "线性序尾段不可达自签发段 {si},frame-end 不覆盖全部 signal 事件"
            ));
        }
    }
    let mut out: Vec<LegalizedSegment> = Vec::with_capacity(n);
    for (i, s) in segs.iter().enumerate() {
        let wait_value = s
            .wait_points
            .iter()
            .map(|w| value_of_seg[&signal_owner[w]])
            .max();
        let mut signal_value = value_of_seg.get(&i).copied();
        if i == last {
            signal_value = Some(n_signals + 1);
        }
        out.push(LegalizedSegment {
            queue: s.queue,
            passes: s.passes.clone(),
            wait_value,
            signal_value,
        });
    }
    Ok(LegalizedSubmission {
        segments: out,
        span: n_signals + 1,
    })
}

// ---------------------------------------------------------------------------
// 判档 workload(uc06 三 pass 形状的等价 compute 负载;确定性整数 kernel)
// ---------------------------------------------------------------------------

// buffer 表下标(0..=9 = 图资源线性序,digest 域;10 = 零 dummy;11/12 = shade 链稿)。
const B_ALBEDO: usize = 0;
const B_NORMAL: usize = 1;
const B_DEPTH: usize = 2;
const B_VSM: usize = 3;
const B_GI: usize = 4;
const B_AO_RAW: usize = 5;
const B_SHADOW: usize = 6;
const B_AO_F: usize = 7;
const B_HDR: usize = 8;
const B_BACK: usize = 9;
const B_DUMMY: usize = 10;
const B_S0: usize = 11;
const B_S1: usize = 12;
/// buffer 总数。
const BUF_COUNT: usize = 13;
/// digest 域(全部图资源输出;dummy/稿不入 digest)。
const RES_BUF_COUNT: usize = 10;

/// 单 pass 的 dispatch 表(out, in_a, in_b, seed, iters)。
type DispatchTable = Vec<(usize, usize, usize, u32, u32)>;

/// workload 参数(--scale 参数化面;PLAN §2.5「异步段 ≥0.5ms」判据的调节旋钮)。
#[derive(Debug, Clone, Copy)]
struct WorkloadParams {
    /// 轻 pass 迭代(gbuffer/ao_filter/shade/blit)。
    light_iters: u32,
    /// 异步三 pass 每 pass 迭代(= 256 × scale)。
    heavy_iters: u32,
    /// 与异步段并行的图形 pass(vsm_page_mark)迭代(= 3 × heavy,时长对齐异步段)。
    gfx_concurrent_iters: u32,
}

impl WorkloadParams {
    fn from_scale(scale: u32) -> Self {
        let heavy = 256 * scale.max(1);
        WorkloadParams {
            light_iters: 64,
            heavy_iters: heavy,
            gfx_concurrent_iters: 3 * heavy,
        }
    }
}

/// 8 pass 的 dispatch 表(线性序;镜像图读写声明——digest 等价门的物理依赖面)。
fn build_dispatch_tables(p: WorkloadParams) -> Vec<(&'static str, DispatchTable)> {
    vec![
        (
            "gbuffer",
            vec![
                (B_ALBEDO, B_DUMMY, B_DUMMY, 0xA1B0_0001, p.light_iters),
                (B_NORMAL, B_DUMMY, B_DUMMY, 0xA1B0_0002, p.light_iters),
                (B_DEPTH, B_DUMMY, B_DUMMY, 0xA1B0_0003, p.light_iters),
            ],
        ),
        (
            "vsm_page_mark",
            vec![(B_VSM, B_DEPTH, B_DUMMY, 0xB100_0001, p.gfx_concurrent_iters)],
        ),
        (
            "gi_probe_trace",
            vec![(B_GI, B_NORMAL, B_DEPTH, 0xC200_0001, p.heavy_iters)],
        ),
        (
            "rtao",
            vec![(B_AO_RAW, B_NORMAL, B_DEPTH, 0xC300_0001, p.heavy_iters)],
        ),
        (
            "hard_shadow",
            vec![(B_SHADOW, B_DEPTH, B_DUMMY, 0xC400_0001, p.heavy_iters)],
        ),
        (
            "ao_filter",
            vec![(B_AO_F, B_AO_RAW, B_DUMMY, 0xD500_0001, p.light_iters)],
        ),
        (
            "deferred_shade",
            vec![
                (B_S0, B_ALBEDO, B_GI, 0xE600_0001, p.light_iters),
                (B_S1, B_S0, B_AO_F, 0xE600_0002, p.light_iters),
                (B_HDR, B_S1, B_SHADOW, 0xE600_0003, p.light_iters),
            ],
        ),
        (
            "blit",
            vec![(B_BACK, B_HDR, B_DUMMY, 0xF700_0001, p.light_iters)],
        ),
    ]
}

/// kernel 数学(CPU 参照;与 [`async_probe_kernel_spv`] 逐运算同式,纯 u32 wrapping
/// ⇒ device/host 位级同值)。
fn kernel_ref(gid: u32, a: u32, b: u32, seed: u32, iters: u32) -> u32 {
    let mut h = seed ^ gid.wrapping_mul(0x9E37_79B9);
    let mut x = a ^ b.rotate_left(16);
    let mut k = 0u32;
    while k < iters {
        h = (h ^ x).wrapping_mul(0x85EB_CA6B);
        h ^= h >> 13;
        x = x.wrapping_add(0x9E37_79B9);
        k += 1;
    }
    h ^ x
}

/// CPU 参照:零初始化 buffer 表 → 按线性 pass 序执行全部 dispatch(依赖序 =
/// 声明序,纯函数与队列/时序无关)→ 返回 buffer 表。
fn cpu_reference_buffers(tables: &[(&'static str, DispatchTable)]) -> Vec<Vec<u32>> {
    let mut bufs: Vec<Vec<u32>> = vec![vec![0u32; ELEM_COUNT as usize]; BUF_COUNT];
    for (_, table) in tables {
        for &(out, ia, ib, seed, iters) in table {
            for gid in 0..ELEM_COUNT {
                let a = bufs[ia][gid as usize];
                let b = bufs[ib][gid as usize];
                bufs[out][gid as usize] = kernel_ref(gid, a, b, seed, iters);
            }
        }
    }
    bufs
}

/// digest 域:图资源 buffer(0..=9)拼接字节 sha256。
fn digest_resource_bytes(readback: &[Vec<u8>]) -> String {
    let mut all: Vec<u8> = Vec::with_capacity(RES_BUF_COUNT * readback[0].len());
    for b in readback.iter().take(RES_BUF_COUNT) {
        all.extend_from_slice(b);
    }
    rurix_pkg::sha256::hex_digest(&all)
}

fn digest_cpu_reference(bufs: &[Vec<u32>]) -> String {
    let mut all: Vec<u8> = Vec::with_capacity(RES_BUF_COUNT * bufs[0].len() * 4);
    for b in bufs.iter().take(RES_BUF_COUNT) {
        for v in b {
            all.extend_from_slice(&v.to_le_bytes());
        }
    }
    rurix_pkg::sha256::hex_digest(&all)
}

// ---------------------------------------------------------------------------
// 手编 SPIR-V compute kernel(3 SSBO + 12B push constant;SPIR-V 1.0,
// mesh_witness_fs_spv 手编先例同法;合法性经 device 真跑 + validation layer 机核)
// ---------------------------------------------------------------------------

/// 判档 kernel SPIR-V:`out[i] = mix(seed, i, in_a[i], in_b[i], iters)`(确定性
/// u32 迭代;与 [`kernel_ref`] 逐运算同式)。布局:set0 binding0=out /
/// binding1=in_a / binding2=in_b(BufferBlock);push constant
/// `{elem_count@0, seed@4, iters@8}`;LocalSize 256。
fn async_probe_kernel_spv() -> Vec<u32> {
    fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
        v.push(op | ((ops.len() as u32 + 1) << 16));
        v.extend_from_slice(ops);
    }
    // header: magic / version 1.0 / generator 0 / bound 72 / schema 0。
    let mut v = vec![0x0723_0203, 0x0001_0000, 0, 72, 0];
    inst(&mut v, 17, &[1]); // OpCapability Shader
    inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    inst(&mut v, 15, &[5, 30, 0x6E69_616D, 0, 6]); // OpEntryPoint GLCompute %30 "main" %6
    inst(&mut v, 16, &[30, 17, 256, 1, 1]); // OpExecutionMode %30 LocalSize 256 1 1
    inst(&mut v, 71, &[6, 11, 28]); // OpDecorate %6 BuiltIn GlobalInvocationId
    inst(&mut v, 71, &[8, 6, 4]); // OpDecorate %8 ArrayStride 4
    inst(&mut v, 72, &[9, 0, 35, 0]); // OpMemberDecorate %9 0 Offset 0
    inst(&mut v, 71, &[9, 3]); // OpDecorate %9 BufferBlock
    inst(&mut v, 71, &[11, 34, 0]); // %11(out) DescriptorSet 0
    inst(&mut v, 71, &[11, 33, 0]); // %11 Binding 0
    inst(&mut v, 71, &[12, 34, 0]); // %12(in_a) DescriptorSet 0
    inst(&mut v, 71, &[12, 33, 1]); // %12 Binding 1
    inst(&mut v, 71, &[13, 34, 0]); // %13(in_b) DescriptorSet 0
    inst(&mut v, 71, &[13, 33, 2]); // %13 Binding 2
    inst(&mut v, 72, &[14, 0, 35, 0]); // pc.elem_count Offset 0
    inst(&mut v, 72, &[14, 1, 35, 4]); // pc.seed Offset 4
    inst(&mut v, 72, &[14, 2, 35, 8]); // pc.iters Offset 8
    inst(&mut v, 71, &[14, 2]); // OpDecorate %14 Block
    inst(&mut v, 19, &[1]); // %1 = OpTypeVoid
    inst(&mut v, 33, &[2, 1]); // %2 = OpTypeFunction %1
    inst(&mut v, 21, &[3, 32, 0]); // %3 = OpTypeInt 32 0
    inst(&mut v, 23, &[4, 3, 3]); // %4 = uvec3
    inst(&mut v, 32, &[5, 1, 4]); // %5 = ptr Input uvec3
    inst(&mut v, 59, &[5, 6, 1]); // %6 = Variable Input(gid)
    inst(&mut v, 32, &[7, 1, 3]); // %7 = ptr Input uint
    inst(&mut v, 29, &[8, 3]); // %8 = RuntimeArray uint
    inst(&mut v, 30, &[9, 8]); // %9 = Struct{rtarr}
    inst(&mut v, 32, &[10, 2, 9]); // %10 = ptr Uniform %9
    inst(&mut v, 59, &[10, 11, 2]); // %11 = out
    inst(&mut v, 59, &[10, 12, 2]); // %12 = in_a
    inst(&mut v, 59, &[10, 13, 2]); // %13 = in_b
    inst(&mut v, 30, &[14, 3, 3, 3]); // %14 = PC struct{u32×3}
    inst(&mut v, 32, &[15, 9, 14]); // %15 = ptr PushConstant %14
    inst(&mut v, 59, &[15, 16, 9]); // %16 = pc
    inst(&mut v, 32, &[17, 9, 3]); // %17 = ptr PushConstant uint
    inst(&mut v, 32, &[18, 2, 3]); // %18 = ptr Uniform uint
    inst(&mut v, 20, &[19]); // %19 = bool
    inst(&mut v, 43, &[3, 20, 0]); // %20 = 0u
    inst(&mut v, 43, &[3, 21, 1]); // %21 = 1u
    inst(&mut v, 43, &[3, 22, 2]); // %22 = 2u
    inst(&mut v, 43, &[3, 23, 16]); // %23 = 16u
    inst(&mut v, 43, &[3, 24, 13]); // %24 = 13u
    inst(&mut v, 43, &[3, 25, 0x9E37_79B9]); // %25
    inst(&mut v, 43, &[3, 26, 0x85EB_CA6B]); // %26
    inst(&mut v, 54, &[1, 30, 0, 2]); // OpFunction %1 %30 None %2
    inst(&mut v, 248, &[31]); // entry label
    inst(&mut v, 65, &[7, 32, 6, 20]); // gid.x ptr
    inst(&mut v, 61, &[3, 33, 32]); // %33 = gid
    inst(&mut v, 65, &[17, 34, 16, 20]); // pc.elem_count ptr
    inst(&mut v, 61, &[3, 35, 34]); // %35 = n
    inst(&mut v, 174, &[19, 36, 33, 35]); // %36 = gid >= n
    inst(&mut v, 247, &[38, 0]); // OpSelectionMerge %38
    inst(&mut v, 250, &[36, 37, 38]); // BranchConditional %36 %37 %38
    inst(&mut v, 248, &[37]); // %37(early return)
    inst(&mut v, 253, &[]); // OpReturn
    inst(&mut v, 248, &[38]); // %38(body)
    inst(&mut v, 65, &[18, 39, 12, 20, 33]); // in_a.data[gid] ptr
    inst(&mut v, 61, &[3, 40, 39]); // %40 = a
    inst(&mut v, 65, &[18, 41, 13, 20, 33]); // in_b.data[gid] ptr
    inst(&mut v, 61, &[3, 42, 41]); // %42 = b
    inst(&mut v, 65, &[17, 43, 16, 21]); // pc.seed ptr
    inst(&mut v, 61, &[3, 44, 43]); // %44 = seed
    inst(&mut v, 65, &[17, 45, 16, 22]); // pc.iters ptr
    inst(&mut v, 61, &[3, 46, 45]); // %46 = iters
    inst(&mut v, 132, &[3, 47, 33, 25]); // %47 = gid * GOLDEN
    inst(&mut v, 198, &[3, 48, 44, 47]); // %48 = h0 = seed ^ %47
    inst(&mut v, 196, &[3, 49, 42, 23]); // %49 = b << 16
    inst(&mut v, 194, &[3, 50, 42, 23]); // %50 = b >> 16
    inst(&mut v, 197, &[3, 51, 49, 50]); // %51 = rotl(b,16)
    inst(&mut v, 198, &[3, 52, 40, 51]); // %52 = x0 = a ^ %51
    inst(&mut v, 249, &[53]); // Branch loop header
    inst(&mut v, 248, &[53]); // %53 loop header
    inst(&mut v, 245, &[3, 54, 48, 38, 68, 57]); // %54 = phi h
    inst(&mut v, 245, &[3, 55, 52, 38, 69, 57]); // %55 = phi x
    inst(&mut v, 245, &[3, 56, 20, 38, 66, 57]); // %56 = phi k
    inst(&mut v, 246, &[59, 57, 0]); // OpLoopMerge merge=%59 cont=%57
    inst(&mut v, 249, &[60]); // Branch %60
    inst(&mut v, 248, &[60]); // %60 cond block
    inst(&mut v, 176, &[19, 61, 56, 46]); // %61 = k < iters
    inst(&mut v, 250, &[61, 62, 59]); // BranchConditional body/merge
    inst(&mut v, 248, &[62]); // %62 body
    inst(&mut v, 198, &[3, 63, 54, 55]); // %63 = h ^ x
    inst(&mut v, 132, &[3, 64, 63, 26]); // %64 = %63 * MIX
    inst(&mut v, 194, &[3, 67, 64, 24]); // %67 = %64 >> 13
    inst(&mut v, 198, &[3, 68, 64, 67]); // %68 = h'
    inst(&mut v, 128, &[3, 69, 55, 25]); // %69 = x' = x + GOLDEN
    inst(&mut v, 249, &[57]); // Branch cont
    inst(&mut v, 248, &[57]); // %57 continue
    inst(&mut v, 128, &[3, 66, 56, 21]); // %66 = k+1
    inst(&mut v, 249, &[53]); // back edge
    inst(&mut v, 248, &[59]); // %59 merge
    inst(&mut v, 198, &[3, 70, 54, 55]); // %70 = h ^ x
    inst(&mut v, 65, &[18, 71, 11, 20, 33]); // out.data[gid] ptr
    inst(&mut v, 62, &[71, 70]); // OpStore
    inst(&mut v, 253, &[]); // OpReturn
    inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

/// SPIR-V 流完整性(host 自检):magic/version/bound + 指令字长链恰覆盖模块。
fn spirv_stream_check(spv: &[u32]) -> Result<(), String> {
    if spv.len() < 5 || spv[0] != 0x0723_0203 {
        return Err("SPIR-V magic 缺失".into());
    }
    if spv[1] != 0x0001_0000 {
        return Err("SPIR-V 版本非 1.0".into());
    }
    let bound = spv[3];
    let mut i = 5usize;
    while i < spv.len() {
        let wc = (spv[i] >> 16) as usize;
        if wc == 0 || i + wc > spv.len() {
            return Err(format!("指令字长链断裂 @word {i}"));
        }
        i += wc;
    }
    for &w in &spv[5..] {
        let _ = w;
    }
    if bound != 72 {
        return Err(format!("bound 漂移:{bound} ≠ 72"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// host 计划 → vk 执行计划转换(逐字消费面)
// ---------------------------------------------------------------------------

fn map_queue(q: QueueClass) -> rvk::AsyncLaneQueueKind {
    match q {
        QueueClass::Graphics => rvk::AsyncLaneQueueKind::Graphics,
        QueueClass::AsyncCompute => rvk::AsyncLaneQueueKind::Compute,
    }
}

fn to_vk_passes(
    compiled: &CompiledGraph,
    tables: &[(&'static str, DispatchTable)],
) -> Vec<rvk::AsyncLanePassSpec> {
    let passes = compiled.passes();
    assert_eq!(passes.len(), tables.len(), "pass 数与 dispatch 表不一致");
    passes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            assert_eq!(p.id().0 as usize, i, "线性序 ≠ PassId(装配前提破缺)");
            rvk::AsyncLanePassSpec {
                name: tables[i].0.to_owned(),
                queue: map_queue(p.queue()),
                dispatches: tables[i]
                    .1
                    .iter()
                    .map(|&(out, ia, ib, seed, iters)| rvk::AsyncLaneDispatchSpec {
                        out_buf: out,
                        in_a: ia,
                        in_b: ib,
                        seed,
                        iters,
                    })
                    .collect(),
            }
        })
        .collect()
}

fn to_vk_segments(legal: &LegalizedSubmission) -> Vec<rvk::AsyncLaneSubmitSegment> {
    legal
        .segments
        .iter()
        .map(|s| rvk::AsyncLaneSubmitSegment {
            queue: map_queue(s.queue),
            pass_indices: s.passes.iter().map(|p| p.0 as usize).collect(),
            wait_value: s.wait_value,
            signal_value: s.signal_value,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 统计(measured;median 主口径)
// ---------------------------------------------------------------------------

fn median(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut v = samples.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).expect("有限值"));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

/// 单跑摘要(evidence 行)。
struct RunSummary {
    digest_first: String,
    digest_final: String,
    frame_ms_gpu: Vec<f64>,
    frame_ms_wall: Vec<f64>,
    async_busy_ms: Vec<f64>,
    gfx_busy_ms: Vec<f64>,
    overlap_ms: Vec<f64>,
    overlap_ratio: Vec<f64>,
    timestamps_valid: bool,
    timestamp_period_ns: f32,
    queue_mode: &'static str,
    sharing_mode: &'static str,
    device_name: String,
    graphics_family: u32,
    compute_family: Option<u32>,
    final_timeline_value: Option<u64>,
}

fn summarize_run(rep: &rvk::AsyncLanesReport) -> RunSummary {
    let ms = |ns: u64| ns as f64 * 1.0e-6;
    RunSummary {
        digest_first: digest_resource_bytes(&rep.readback_first),
        digest_final: digest_resource_bytes(&rep.readback_final),
        frame_ms_gpu: rep.samples.iter().map(|s| ms(s.frame_ns)).collect(),
        frame_ms_wall: rep.samples.iter().map(|s| ms(s.wall_ns)).collect(),
        async_busy_ms: rep.samples.iter().map(|s| ms(s.async_busy_ns)).collect(),
        gfx_busy_ms: rep
            .samples
            .iter()
            .map(|s| ms(s.graphics_busy_ns))
            .collect(),
        overlap_ms: rep.samples.iter().map(|s| ms(s.overlap_ns)).collect(),
        overlap_ratio: rep
            .samples
            .iter()
            .map(|s| {
                if s.async_busy_ns == 0 {
                    0.0
                } else {
                    s.overlap_ns as f64 / s.async_busy_ns as f64
                }
            })
            .collect(),
        timestamps_valid: rep.timestamps_valid,
        timestamp_period_ns: rep.timestamp_period_ns,
        queue_mode: rep.queue_mode,
        sharing_mode: rep.sharing_mode,
        device_name: rep.device_name.clone(),
        graphics_family: rep.graphics_family,
        compute_family: rep.compute_family,
        final_timeline_value: rep.final_timeline_value,
    }
}

// ---------------------------------------------------------------------------
// evidence JSON(独立 probe 面 schema;键序固定;measured 真值如实)
// ---------------------------------------------------------------------------

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn queue_tag(q: QueueClass) -> &'static str {
    match q {
        QueueClass::Graphics => "graphics",
        QueueClass::AsyncCompute => "async_compute",
    }
}

fn u64s_json(v: &[u64]) -> String {
    let items: Vec<String> = v.iter().map(u64::to_string).collect();
    format!("[{}]", items.join(", "))
}

fn opt_u64_json(v: Option<u64>) -> String {
    v.map_or("null".to_owned(), |x| x.to_string())
}

fn segments_json(segments: &[SubmissionSegment], indent: &str) -> String {
    let mut s = String::from("[\n");
    for (i, seg) in segments.iter().enumerate() {
        let ids: Vec<String> = seg.passes.iter().map(|p| p.0.to_string()).collect();
        s.push_str(&format!(
            "{indent}  {{ \"queue\": \"{}\", \"passes\": [{}], \"wait\": {}, \"signal\": {} }}{}\n",
            queue_tag(seg.queue),
            ids.join(", "),
            u64s_json(&seg.wait_points),
            u64s_json(&seg.signal_points),
            if i + 1 == segments.len() { "" } else { "," },
        ));
    }
    s.push_str(&format!("{indent}]"));
    s
}

fn legalized_json(legal: &LegalizedSubmission, indent: &str) -> String {
    let mut s = String::from("{\n");
    s.push_str(&format!("{indent}  \"span\": {},\n", legal.span));
    s.push_str(&format!("{indent}  \"segments\": [\n"));
    for (i, seg) in legal.segments.iter().enumerate() {
        let ids: Vec<String> = seg.passes.iter().map(|p| p.0.to_string()).collect();
        s.push_str(&format!(
            "{indent}    {{ \"queue\": \"{}\", \"passes\": [{}], \"wait_value\": {}, \"signal_value\": {} }}{}\n",
            queue_tag(seg.queue),
            ids.join(", "),
            opt_u64_json(seg.wait_value),
            opt_u64_json(seg.signal_value),
            if i + 1 == legal.segments.len() { "" } else { "," },
        ));
    }
    s.push_str(&format!("{indent}  ]\n"));
    s.push_str(&format!("{indent}}}"));
    s
}

fn arm_plan_json(name: &str, compiled: &CompiledGraph, segments: &[SubmissionSegment]) -> String {
    let barrier_count: usize = compiled.barriers().iter().map(|(_, b)| b.len()).sum();
    let fences: Vec<String> = compiled
        .fences()
        .iter()
        .map(|f| {
            format!(
                "{{ \"signal_after\": {}, \"wait_before\": {}, \"value\": {}, \"timeline_points\": [{}, {}] }}",
                f.signal_after.0,
                f.wait_before.0,
                f.value,
                2 * f.value - 1,
                2 * f.value,
            )
        })
        .collect();
    format!(
        "    \"{name}\": {{\n      \"passes\": {},\n      \"barriers\": {barrier_count},\n      \"fences\": [{}],\n      \"segments\": {}\n    }}",
        compiled.passes().len(),
        fences.join(", "),
        segments_json(segments, "      "),
    )
}

fn run_json(r: &RunSummary, indent: &str) -> String {
    format!(
        "{{\n{indent}  \"queue_mode\": \"{}\",\n{indent}  \"sharing_mode\": \"{}\",\n{indent}  \"device_name\": \"{}\",\n{indent}  \"graphics_family\": {},\n{indent}  \"compute_family\": {},\n{indent}  \"timestamps_valid\": {},\n{indent}  \"timestamp_period_ns\": {},\n{indent}  \"frames\": {},\n{indent}  \"digest_first\": \"{}\",\n{indent}  \"digest_final\": \"{}\",\n{indent}  \"frame_ms_median_gpu\": {:.4},\n{indent}  \"frame_ms_median_wall\": {:.4},\n{indent}  \"gfx_busy_ms_median\": {:.4},\n{indent}  \"async_busy_ms_median\": {:.4},\n{indent}  \"overlap_ms_median\": {:.4},\n{indent}  \"overlap_ratio_median\": {:.4},\n{indent}  \"final_timeline_value\": {}\n{indent}}}",
        r.queue_mode,
        r.sharing_mode,
        json_escape(&r.device_name),
        r.graphics_family,
        r.compute_family.map_or("null".to_owned(), |v| v.to_string()),
        r.timestamps_valid,
        r.timestamp_period_ns,
        r.frame_ms_gpu.len(),
        r.digest_first,
        r.digest_final,
        median(&r.frame_ms_gpu),
        median(&r.frame_ms_wall),
        median(&r.gfx_busy_ms),
        median(&r.async_busy_ms),
        median(&r.overlap_ms),
        median(&r.overlap_ratio),
        opt_u64_json(r.final_timeline_value),
    )
}

fn caps_json(caps: &rvk::AsyncQueueCaps, indent: &str) -> String {
    format!(
        "{{\n{indent}  \"device_name\": \"{}\",\n{indent}  \"api_version\": {},\n{indent}  \"timeline_semaphore\": {},\n{indent}  \"graphics_family\": {},\n{indent}  \"graphics_timestamp_bits\": {},\n{indent}  \"compute_only_family\": {},\n{indent}  \"compute_only_timestamp_bits\": {},\n{indent}  \"distinct_compute_family\": {},\n{indent}  \"timestamp_period_ns\": {},\n{indent}  \"dual_queue_eligible\": {}\n{indent}}}",
        json_escape(&caps.device_name),
        caps.api_version,
        caps.timeline_semaphore,
        caps.graphics_family,
        caps.graphics_timestamp_bits,
        caps.compute_only_family
            .map_or("null".to_owned(), |v| v.to_string()),
        caps.compute_only_timestamp_bits,
        caps.distinct_compute_family
            .map_or("null".to_owned(), |v| v.to_string()),
        caps.timestamp_period_ns,
        caps.dual_queue_eligible(),
    )
}

// ---------------------------------------------------------------------------
// selftest(纯 CPU 自证;--selftest 与 #[cfg(test)] 双承载同一批判据)
// ---------------------------------------------------------------------------

fn check_fence_golden() -> Result<(), String> {
    let c = build_probe_graph()
        .compile(CompileOptions::default())
        .map_err(|e| format!("编译失败:{e:?}"))?;
    if c.passes().len() != 8 {
        return Err(format!("pass 数漂移:{}", c.passes().len()));
    }
    let f = c.fences();
    if f.len() != 2
        || (f[0].signal_after, f[0].wait_before, f[0].value) != (PassId(0), PassId(5), 1)
        || (f[1].signal_after, f[1].wait_before, f[1].value) != (PassId(0), PassId(6), 2)
    {
        return Err(format!("fence 弧 golden 漂移:{f:?}"));
    }
    Ok(())
}

fn check_segments_golden() -> Result<(), String> {
    let c = build_probe_graph()
        .compile(CompileOptions::default())
        .map_err(|e| format!("编译失败:{e:?}"))?;
    let segs = plan_submission_segments(&c);
    if segs.len() != 5 {
        return Err(format!("段数漂移:{}", segs.len()));
    }
    if segs[2].queue != QueueClass::AsyncCompute
        || segs[2].wait_points != vec![1, 3]
        || segs[2].signal_points != vec![2, 4]
    {
        return Err(format!("异步段点漂移:{:?}", segs[2]));
    }
    Ok(())
}

/// 合法化 golden(判档形状):签发链 seg0→seg2,值 1/2;seg3/seg4 wait 2;
/// 尾段 frame-end 信号 3;span=3。
fn check_legalize_golden() -> Result<(), String> {
    let c = build_probe_graph()
        .compile(CompileOptions::default())
        .map_err(|e| format!("编译失败:{e:?}"))?;
    let segs = plan_submission_segments(&c);
    let legal = legalize_submission(&segs)?;
    let expect: Vec<(Option<u64>, Option<u64>)> = vec![
        (None, Some(1)),    // seg0 gbuffer:签发 1(原点 {1,3} 段级归并)
        (None, None),       // seg1 vsm_page_mark
        (Some(1), Some(2)), // seg2 async 三 pass:等 1 签 2(原点 wait{1,3}/signal{2,4})
        (Some(2), None),    // seg3 ao_filter:等 2
        (Some(2), Some(3)), // seg4 shade+blit:等 2 + frame-end 签 3
    ];
    let got: Vec<(Option<u64>, Option<u64>)> = legal
        .segments
        .iter()
        .map(|s| (s.wait_value, s.signal_value))
        .collect();
    if got != expect || legal.span != 3 {
        return Err(format!("合法化 golden 漂移:{got:?} span={}", legal.span));
    }
    Ok(())
}

/// 合法化 RED 臂:漏 signal(半对)/ 孤儿 signal / 双签 / 非全序全部确定性拒。
fn check_legalize_red_arms() -> Result<(), String> {
    let c = build_probe_graph()
        .compile(CompileOptions::default())
        .map_err(|e| format!("编译失败:{e:?}"))?;
    let base = plan_submission_segments(&c);
    // ① 漏 signal:抹掉异步段 signal 点 → seg3/seg4 的 wait 无签发段。
    let mut t1 = base.clone();
    t1[2].signal_points.clear();
    if legalize_submission(&t1).is_ok() {
        return Err("漏 signal 未拒(半对应 RED)".into());
    }
    // ② 孤儿 signal:seg1 加无人等待的点 9。
    let mut t2 = base.clone();
    t2[1].signal_points.push(9);
    if legalize_submission(&t2).is_ok() {
        return Err("孤儿 signal 未拒(半对应 RED)".into());
    }
    // ③ 双签:seg1 也签点 2。
    let mut t3 = base.clone();
    t3[1].signal_points.push(2);
    if legalize_submission(&t3).is_ok() {
        return Err("同点双签未拒".into());
    }
    // ④ 非全序:两签发段无 happens-before(值回退风险)。
    let t4 = vec![
        SubmissionSegment {
            queue: QueueClass::Graphics,
            passes: vec![PassId(0)],
            wait_points: vec![],
            signal_points: vec![1],
        },
        SubmissionSegment {
            queue: QueueClass::AsyncCompute,
            passes: vec![PassId(1)],
            wait_points: vec![],
            signal_points: vec![2],
        },
        SubmissionSegment {
            queue: QueueClass::Graphics,
            passes: vec![PassId(2)],
            wait_points: vec![1, 2],
            signal_points: vec![],
        },
    ];
    match legalize_submission(&t4) {
        Ok(_) => return Err("非全序签发未拒(值回退风险应 RED)".into()),
        Err(e) if e.contains("非全序") => {}
        Err(e) => return Err(format!("非全序拒因漂移:{e}")),
    }
    Ok(())
}

/// 回落重编译判据:off 臂零 fence 单段全图形,且屏障批与 on 臂**不同**
/// (趟 3 stage 随车道重推 ⇒「回落必须重编译而非忽略 fence」的结构证据)。
fn check_fallback_recompile() -> Result<(), String> {
    let on = build_probe_graph()
        .compile(CompileOptions::default())
        .map_err(|e| format!("编译失败:{e:?}"))?;
    let off = build_probe_graph()
        .compile(CompileOptions {
            enable_async: false,
            ..CompileOptions::default()
        })
        .map_err(|e| format!("编译失败:{e:?}"))?;
    if !off.fences().is_empty() {
        return Err("off 臂 fence 非空".into());
    }
    let segs = plan_submission_segments(&off);
    if segs.len() != 1 || segs[0].queue != QueueClass::Graphics || segs[0].passes.len() != 8 {
        return Err(format!("off 臂段形态漂移:{segs:?}"));
    }
    let legal = legalize_submission(&segs)?;
    if legal.span != 0 || legal.segments[0].signal_value.is_some() {
        return Err("off 臂不应携 timeline 值域".into());
    }
    let count = |c: &CompiledGraph| -> usize { c.barriers().iter().map(|(_, b)| b.len()).sum() };
    let (n_on, n_off) = (count(&on), count(&off));
    if n_on == n_off {
        return Err(format!(
            "on/off 屏障批相同({n_on})——重编译语义存疑(应随车道重推)"
        ));
    }
    Ok(())
}

fn check_kernel_reference() -> Result<(), String> {
    // 确定性:双跑位级一致 + 迭代数敏感 + 输入敏感。
    let a = kernel_ref(1234, 7, 9, 0xC200_0001, 64);
    let b = kernel_ref(1234, 7, 9, 0xC200_0001, 64);
    if a != b {
        return Err("kernel 参照非确定".into());
    }
    if kernel_ref(1234, 7, 9, 0xC200_0001, 65) == a {
        return Err("iters 不敏感".into());
    }
    if kernel_ref(1234, 8, 9, 0xC200_0001, 64) == a {
        return Err("输入不敏感".into());
    }
    let p = WorkloadParams::from_scale(1);
    let tables = build_dispatch_tables(p);
    let d1 = digest_cpu_reference(&cpu_reference_buffers(&tables));
    let d2 = digest_cpu_reference(&cpu_reference_buffers(&tables));
    if d1 != d2 {
        return Err("CPU 参照 digest 非确定".into());
    }
    Ok(())
}

fn check_spirv_stream() -> Result<(), String> {
    spirv_stream_check(&async_probe_kernel_spv())
}

fn run_selftest() -> i32 {
    let checks: Vec<(&str, fn() -> Result<(), String>)> = vec![
        ("fence_golden", check_fence_golden),
        ("segments_golden", check_segments_golden),
        ("legalize_golden", check_legalize_golden),
        ("legalize_red_arms", check_legalize_red_arms),
        ("fallback_recompile", check_fallback_recompile),
        ("kernel_reference", check_kernel_reference),
        ("spirv_stream", check_spirv_stream),
    ];
    let mut fail = 0;
    for (name, f) in checks {
        match f() {
            Ok(()) => println!("PASS {name}"),
            Err(e) => {
                println!("FAIL {name}: {e}");
                fail += 1;
            }
        }
    }
    if fail == 0 {
        println!("selftest: 7/7 过(纯 CPU,零 GPU)");
        0
    } else {
        println!("selftest: {fail} 项失败");
        1
    }
}

// ---------------------------------------------------------------------------
// CLI + 主流程
// ---------------------------------------------------------------------------

struct Args {
    selftest: bool,
    judge: bool,
    frames: u32,
    warmup: u32,
    scale: u32,
    out: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        selftest: false,
        judge: false,
        frames: 120,
        warmup: 20,
        scale: 8,
        out: None,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        let take_u32 = |i: &mut usize| -> Result<u32, String> {
            *i += 1;
            argv.get(*i)
                .ok_or_else(|| format!("{} 缺参数", argv[*i - 1]))?
                .parse::<u32>()
                .map_err(|e| format!("{} 参数非 u32:{e}", argv[*i - 1]))
        };
        match argv[i].as_str() {
            "--selftest" => a.selftest = true,
            "--judge" => a.judge = true,
            "--frames" => a.frames = take_u32(&mut i)?,
            "--warmup" => a.warmup = take_u32(&mut i)?,
            "--scale" => a.scale = take_u32(&mut i)?,
            "--out" => {
                i += 1;
                a.out = Some(
                    argv.get(i)
                        .ok_or("--out 缺参数".to_owned())?
                        .clone(),
                );
            }
            other => return Err(format!("未知参数 {other}(--selftest/--judge/--frames/--warmup/--scale/--out)")),
        }
        i += 1;
    }
    if a.frames == 0 {
        return Err("--frames 须 ≥1".into());
    }
    Ok(a)
}

/// dev-env degrade 判别(g35 三态纪律):loader/设备缺失 → skipped(exit 0),
/// `RURIX_REQUIRE_REAL=1` 翻硬红。
fn is_dev_env_degrade(e: &str) -> bool {
    e.contains("不可用") || e.contains("无 Vulkan 物理设备")
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("参数错误: {e}");
            std::process::exit(2);
        }
    };
    if args.selftest {
        std::process::exit(run_selftest());
    }
    let require_real = std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1");

    // ── host 计划面(双臂编译 + 段切分 + 合法化;PLAN §2.1)──
    let on = build_probe_graph()
        .compile(CompileOptions::default())
        .expect("判档图应编译通过(合法图)");
    let off = build_probe_graph()
        .compile(CompileOptions {
            enable_async: false,
            ..CompileOptions::default()
        })
        .expect("判档图应编译通过(合法图)");
    let segs_on = plan_submission_segments(&on);
    let segs_off = plan_submission_segments(&off);
    let legal_on = match legalize_submission(&segs_on) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("timeline 值域合法化 RED: {e}");
            std::process::exit(1);
        }
    };
    let legal_off = legalize_submission(&segs_off).expect("off 臂零点恒合法");

    // ── workload(uc06 三 pass 形状等价负载;--scale 参数化)──
    let params = WorkloadParams::from_scale(args.scale);
    let tables = build_dispatch_tables(params);
    let spv = async_probe_kernel_spv();
    spirv_stream_check(&spv).expect("kernel 流自检");
    let cpu_digest = digest_cpu_reference(&cpu_reference_buffers(&tables));

    // ── 能力探测(#62 硬前置)──
    let caps = match rvk::probe_async_queue_caps() {
        Ok(c) => c,
        Err(e) if is_dev_env_degrade(&e) && !require_real => {
            println!(
                "{{\n  \"probe\": \"g31_async_lanes_probe\",\n  \"mode\": \"skipped_dev_env\",\n  \"reason\": \"{}\"\n}}",
                json_escape(&e)
            );
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("能力探测失败: {e}");
            std::process::exit(1);
        }
    };
    let dual_eligible = caps.dual_queue_eligible();
    let fallback_reason = if dual_eligible {
        None
    } else {
        Some(format!(
            "timeline={} compute_only_family={:?} api_version=0x{:x}",
            caps.timeline_semaphore, caps.compute_only_family, caps.api_version
        ))
    };

    // ── 三臂执行 ──
    let passes_single = to_vk_passes(&off, &tables);
    let segments_single = to_vk_segments(&legal_off);
    let plan_single = rvk::AsyncLanesPlan {
        spv: &spv,
        buffer_count: BUF_COUNT,
        elem_count: ELEM_COUNT,
        passes: &passes_single,
        segments: &segments_single,
        timeline_span: legal_off.span,
        dual_queue: false,
        frames: args.frames,
        warmup: args.warmup,
    };
    let run_arm = |plan: &rvk::AsyncLanesPlan<'_>, tag: &str| -> RunSummary {
        match rvk::run_async_lanes(plan) {
            Ok(r) => summarize_run(&r),
            Err(e) if is_dev_env_degrade(&e) && !require_real => {
                println!(
                    "{{\n  \"probe\": \"g31_async_lanes_probe\",\n  \"mode\": \"skipped_dev_env\",\n  \"reason\": \"{}\"\n}}",
                    json_escape(&e)
                );
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("{tag} 臂执行失败: {e}");
                std::process::exit(1);
            }
        }
    };
    eprintln!("[arm_single] run 1/2 …");
    let single_1 = run_arm(&plan_single, "single");
    eprintln!("[arm_single] run 2/2 …");
    let single_2 = run_arm(&plan_single, "single");

    let (passes_dual, segments_dual);
    let mut dual_runs: Vec<RunSummary> = Vec::new();
    if dual_eligible {
        passes_dual = to_vk_passes(&on, &tables);
        segments_dual = to_vk_segments(&legal_on);
        let plan_dual = rvk::AsyncLanesPlan {
            spv: &spv,
            buffer_count: BUF_COUNT,
            elem_count: ELEM_COUNT,
            passes: &passes_dual,
            segments: &segments_dual,
            timeline_span: legal_on.span,
            dual_queue: true,
            frames: args.frames,
            warmup: args.warmup,
        };
        eprintln!("[arm_dual] run 1/2 …");
        dual_runs.push(run_arm(&plan_dual, "dual"));
        eprintln!("[arm_dual] run 2/2 …");
        dual_runs.push(run_arm(&plan_dual, "dual"));
    }

    // ── 等价门(硬前置;PLAN §2.2)──
    let mut equivalence_ok = true;
    let mut equivalence_notes: Vec<String> = Vec::new();
    for (tag, r) in [("single#1", &single_1), ("single#2", &single_2)]
        .into_iter()
        .chain(
            dual_runs
                .iter()
                .enumerate()
                .map(|(i, r)| (if i == 0 { "dual#1" } else { "dual#2" }, r)),
        )
    {
        if r.digest_first != r.digest_final {
            equivalence_ok = false;
            equivalence_notes.push(format!("{tag} 首帧/末帧 digest 不等(帧内竞态金丝雀)"));
        }
    }
    if single_1.digest_final != single_2.digest_final {
        equivalence_ok = false;
        equivalence_notes.push("single 双跑 digest 不等".into());
    }
    let dual_rerun_equal = dual_runs.len() == 2 && dual_runs[0].digest_final == dual_runs[1].digest_final;
    if dual_runs.len() == 2 && !dual_rerun_equal {
        equivalence_ok = false;
        equivalence_notes.push("dual 双跑 digest 不等(位级重跑不一致)".into());
    }
    let single_vs_dual_equal = dual_runs
        .first()
        .map(|d| d.digest_final == single_1.digest_final);
    if single_vs_dual_equal == Some(false) {
        equivalence_ok = false;
        equivalence_notes.push("single vs dual digest 不等(等价门破缺)".into());
    }
    let matches_cpu = single_1.digest_final == cpu_digest;
    let hard_gate = if dual_runs.is_empty() {
        "not-triggered"
    } else if equivalence_ok {
        "pass"
    } else {
        "fail"
    };

    // ── measured 聚合(两跑合并中位;噪声 = 双跑中位差)──
    let pool = |a: &[f64], b: &[f64]| -> Vec<f64> {
        let mut v = a.to_vec();
        v.extend_from_slice(b);
        v
    };
    let use_gpu_time = single_1.timestamps_valid
        && single_2.timestamps_valid
        && dual_runs.iter().all(|r| r.timestamps_valid);
    let frame_series = |r: &RunSummary| -> Vec<f64> {
        if use_gpu_time {
            r.frame_ms_gpu.clone()
        } else {
            r.frame_ms_wall.clone()
        }
    };
    let m_single_1 = median(&frame_series(&single_1));
    let m_single_2 = median(&frame_series(&single_2));
    let m_single = median(&pool(&frame_series(&single_1), &frame_series(&single_2)));
    let noise_single_pct = if m_single_1 > 0.0 {
        100.0 * (m_single_1 - m_single_2).abs() / m_single_1
    } else {
        f64::INFINITY
    };
    let (m_dual, noise_dual_pct, overlap_ratio_med, async_busy_med) = if dual_runs.len() == 2 {
        let m1 = median(&frame_series(&dual_runs[0]));
        let m2 = median(&frame_series(&dual_runs[1]));
        (
            median(&pool(&frame_series(&dual_runs[0]), &frame_series(&dual_runs[1]))),
            if m1 > 0.0 {
                100.0 * (m1 - m2).abs() / m1
            } else {
                f64::INFINITY
            },
            median(&pool(&dual_runs[0].overlap_ratio, &dual_runs[1].overlap_ratio)),
            median(&pool(&dual_runs[0].async_busy_ms, &dual_runs[1].async_busy_ms)),
        )
    } else {
        (0.0, f64::INFINITY, 0.0, 0.0)
    };
    let improvement_ms = m_single - m_dual;
    let improvement_pct = if m_single > 0.0 {
        100.0 * improvement_ms / m_single
    } else {
        0.0
    };

    // ── 两态判定(PLAN §2.5;--judge)──
    let mut reasons: Vec<String> = Vec::new();
    let verdict = if dual_runs.is_empty() {
        reasons.push(format!(
            "single_queue_fallback(双队列硬前置缺失:{})",
            fallback_reason.clone().unwrap_or_default()
        ));
        "no-go"
    } else if !equivalence_ok {
        reasons.push("硬前置 digest 等价门 FAIL(整窗不判收益)".into());
        "red"
    } else {
        if !use_gpu_time {
            reasons.push("GPU 时间戳无效(timestamp_valid_bits=0/period≤0)——测量无效".into());
        }
        if noise_single_pct >= 1.0 || noise_dual_pct >= 1.0 {
            reasons.push(format!(
                "噪声门未过(同臂双跑中位差 single={noise_single_pct:.2}% dual={noise_dual_pct:.2}% ≥1%)——测量无效"
            ));
        }
        if async_busy_med < 0.5 {
            reasons.push(format!(
                "异步段时长 {async_busy_med:.3}ms <0.5ms(报告5 三条件;调大 --scale)"
            ));
        }
        if improvement_pct < 3.0 {
            reasons.push(format!("中位改善 {improvement_pct:.2}% <3%"));
        }
        if improvement_ms < 0.15 {
            reasons.push(format!("中位改善 {improvement_ms:.3}ms <0.15ms"));
        }
        if overlap_ratio_med < 0.5 {
            reasons.push(format!("重叠率中位 {overlap_ratio_med:.2} <50%"));
        }
        if reasons.is_empty() { "go" } else { "no-go" }
    };

    // ── evidence JSON ──
    let mut j = String::new();
    j.push_str("{\n");
    j.push_str("  \"probe\": \"g31_async_lanes_probe\",\n");
    j.push_str("  \"mode\": \"device\",\n");
    j.push_str(&format!(
        "  \"params\": {{ \"frames\": {}, \"warmup\": {}, \"scale\": {}, \"elem_count\": {}, \"buffers\": {}, \"light_iters\": {}, \"heavy_iters\": {}, \"gfx_concurrent_iters\": {} }},\n",
        args.frames,
        args.warmup,
        args.scale,
        ELEM_COUNT,
        BUF_COUNT,
        params.light_iters,
        params.heavy_iters,
        params.gfx_concurrent_iters
    ));
    j.push_str(&format!("  \"caps\": {},\n", caps_json(&caps, "  ")));
    j.push_str("  \"plan\": {\n");
    j.push_str(&format!("{},\n", arm_plan_json("arm_async_on", &on, &segs_on)));
    j.push_str(&format!(
        "    \"arm_async_on_legalized\": {},\n",
        legalized_json(&legal_on, "    ")
    ));
    j.push_str(&format!("{}\n", arm_plan_json("arm_async_off", &off, &segs_off)));
    j.push_str("  },\n");
    j.push_str(&format!(
        "  \"fallback\": {},\n",
        fallback_reason.as_ref().map_or("null".to_owned(), |r| format!(
            "{{ \"single_queue_fallback\": true, \"reason\": \"{}\" }}",
            json_escape(r)
        ))
    ));
    j.push_str("  \"arms\": {\n");
    j.push_str(&format!(
        "    \"single\": [{}, {}],\n",
        run_json(&single_1, "    "),
        run_json(&single_2, "    ")
    ));
    if dual_runs.len() == 2 {
        j.push_str(&format!(
            "    \"dual\": [{}, {}]\n",
            run_json(&dual_runs[0], "    "),
            run_json(&dual_runs[1], "    ")
        ));
    } else {
        j.push_str("    \"dual\": []\n");
    }
    j.push_str("  },\n");
    j.push_str(&format!(
        "  \"equivalence\": {{\n    \"hard_gate\": \"{hard_gate}\",\n    \"digest_single\": \"{}\",\n    \"digest_dual\": {},\n    \"dual_rerun_bitwise_equal\": {},\n    \"cpu_reference_digest\": \"{cpu_digest}\",\n    \"matches_cpu_reference\": {matches_cpu},\n    \"notes\": [{}]\n  }},\n",
        single_1.digest_final,
        dual_runs
            .first()
            .map_or("null".to_owned(), |d| format!("\"{}\"", d.digest_final)),
        dual_rerun_equal,
        equivalence_notes
            .iter()
            .map(|n| format!("\"{}\"", json_escape(n)))
            .collect::<Vec<_>>()
            .join(", "),
    ));
    j.push_str(&format!(
        "  \"measured\": {{\n    \"time_source\": \"{}\",\n    \"frame_ms_median_single\": {m_single:.4},\n    \"frame_ms_median_dual\": {m_dual:.4},\n    \"improvement_pct\": {improvement_pct:.3},\n    \"improvement_ms\": {improvement_ms:.4},\n    \"overlap_ratio_median\": {overlap_ratio_med:.4},\n    \"async_busy_ms_median\": {async_busy_med:.4},\n    \"noise_single_pct\": {noise_single_pct:.3},\n    \"noise_dual_pct\": {noise_dual_pct:.3}\n  }},\n",
        if use_gpu_time { "gpu_timestamp" } else { "wall_clock" },
    ));
    if args.judge {
        j.push_str(&format!(
            "  \"judge\": {{\n    \"verdict\": \"{verdict}\",\n    \"criteria\": \"digest 等价硬前置;go = 中位改善 ≥3% ∧ ≥0.15ms ∧ 重叠率 ≥50%;噪声门 <1%;异步段 ≥0.5ms\",\n    \"reasons\": [{}]\n  }}\n",
            reasons
                .iter()
                .map(|r| format!("\"{}\"", json_escape(r)))
                .collect::<Vec<_>>()
                .join(", "),
        ));
    } else {
        j.push_str("  \"judge\": null\n");
    }
    j.push_str("}\n");

    println!("{j}");
    if let Some(path) = &args.out {
        if let Err(e) = std::fs::write(path, &j) {
            eprintln!("evidence 落盘失败 {path}: {e}");
            std::process::exit(1);
        }
        eprintln!("evidence → {path}");
    }
    if hard_gate == "fail" {
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// 单测(--selftest 同一批判据的 cargo test 承载)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rurix_render::graph::types::FencePair;

    /// 判档图形状:8 pass 全幸存;异步三 pass 车道正确;fence 弧去重后恰两条,
    /// timeline 值 1 起单调(compile.rs 趟 4 契约)。
    #[test]
    fn probe_graph_shape_and_fences() {
        let c = build_probe_graph()
            .compile(CompileOptions::default())
            .expect("合法图");
        assert_eq!(c.passes().len(), 8);
        assert!(
            c.passes()[2..=4]
                .iter()
                .all(|p| p.queue() == QueueClass::AsyncCompute),
            "gi_probe_trace/rtao/hard_shadow 应在异步车道"
        );
        assert_eq!(
            c.fences(),
            &[
                FencePair {
                    signal_after: PassId(0),
                    wait_before: PassId(5),
                    value: 1,
                },
                FencePair {
                    signal_after: PassId(0),
                    wait_before: PassId(6),
                    value: 2,
                },
            ],
            "fence 弧 golden 漂移"
        );
    }

    /// 段切分 golden:5 段;点映射 (2v-1, 2v);同队列 signal 严格递增。
    #[test]
    fn segments_follow_fence_arcs() {
        let c = build_probe_graph()
            .compile(CompileOptions::default())
            .expect("合法图");
        let segs = plan_submission_segments(&c);
        let ids = |v: &[u32]| v.iter().map(|&i| PassId(i)).collect::<Vec<_>>();
        let expected = vec![
            SubmissionSegment {
                queue: QueueClass::Graphics,
                passes: ids(&[0]),
                wait_points: vec![],
                signal_points: vec![1, 3],
            },
            SubmissionSegment {
                queue: QueueClass::Graphics,
                passes: ids(&[1]),
                wait_points: vec![],
                signal_points: vec![],
            },
            SubmissionSegment {
                queue: QueueClass::AsyncCompute,
                passes: ids(&[2, 3, 4]),
                wait_points: vec![1, 3],
                signal_points: vec![2, 4],
            },
            SubmissionSegment {
                queue: QueueClass::Graphics,
                passes: ids(&[5]),
                wait_points: vec![2],
                signal_points: vec![],
            },
            SubmissionSegment {
                queue: QueueClass::Graphics,
                passes: ids(&[6, 7]),
                wait_points: vec![4],
                signal_points: vec![],
            },
        ];
        assert_eq!(segs, expected, "段切分 golden 漂移");
    }

    /// 回落臂:off 重编译零 fence、全图形车道、单段(显式 single-queue plan)。
    #[test]
    fn fallback_arm_is_single_segment() {
        let c = build_probe_graph()
            .compile(CompileOptions {
                enable_async: false,
                ..CompileOptions::default()
            })
            .expect("合法图");
        assert!(c.fences().is_empty());
        let segs = plan_submission_segments(&c);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].queue, QueueClass::Graphics);
        assert_eq!(segs[0].passes.len(), 8);
        assert!(segs[0].wait_points.is_empty() && segs[0].signal_points.is_empty());
    }

    /// timeline 值域合法化 golden(签发链 1→2 + frame-end 3;共享生产者弧形的
    /// 段级归并——本窗 measured 发现,IMPL_REPORT 登记)。
    #[test]
    fn legalize_golden_matches() {
        check_legalize_golden().expect("合法化 golden");
    }

    /// 提交前 validator RED 臂:半对/孤儿/双签/非全序确定性拒(RFC_DRAFT 修订行 3)。
    #[test]
    fn legalize_red_arms_reject() {
        check_legalize_red_arms().expect("RED 臂");
    }

    /// 回落必须重编译:off 臂屏障批与 on 臂不同(趟 3 stage 随车道重推)。
    #[test]
    fn fallback_requires_recompile() {
        check_fallback_recompile().expect("重编译判据");
    }

    /// kernel 参照确定性 + SPIR-V 流完整性。
    #[test]
    fn kernel_and_spirv_selfcheck() {
        check_kernel_reference().expect("kernel 参照");
        check_spirv_stream().expect("SPIR-V 流");
    }
}
