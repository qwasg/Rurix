//! G8.5a M19 `g8.p0.m19.vsm_page_cache` device 腿。
//!
//! host 金标准 = `rurix_render::shadow::page_cache::run_m19_fixture`;
//! device 三段:
//!   ⓪ **逐帧 mark 段(A2.1)**:16 帧逐帧 dispatch `vsm_page_mark_project`
//!      (主相机深度 → 反投影 → 选级 → 出窗回退 → 原子位图),readback 位图与
//!      host 镜像 `Vsm::page_mark_bits` 逐位 + 逐槽对拍。此前该核**编进 SPV 但零
//!      消费**,fixture 用 `vsm.mark_slot(l,x,y)`(host 预知 page id)直接标页 ——
//!      设计 §2.1「帧循环第一行」/§2.3「第一核」在 device 面上是空的,A2.1 即为
//!      其清零。RED 两轴:`--m19-red-skip-mark`(不 dispatch,零位图冒充)与
//!      `--m19-red-host-mark`(host 预知四页冒充,位图逐帧恒定 ⇒ F13+ 分叉)。
//!   ① **逐帧 digest 段(A2)**:16 帧逐帧把该帧页表/物理池上传,`vsm_sample`
//!      (+ F12 起 `vsm_sample_local`)真消费,readback 出
//!      `page_table` / `depth_pool` / `sample` 三个 digest —— 原像**全部来自 device
//!      readback**,与 golden(`tests/vsm_page_cache/golden/m19_digests.json`)逐帧比对。
//!      此前这三个字段是 host `run_m19_fixture` 的 digest 直填(host 代绿),smoke 侧
//!      还只做 truthiness 判定(任意垃圾串都过),本段即为其清零。
//!   ② multi-view 深度段:单 dispatch `vsm_depth_raster_mv` 覆盖 ≥5 视图脏页批次,
//!      深度 readback 与 host pool 对拍(G7.5 冻结 1e-6 口径)。
//! validation 经 `RURIX_VK_VALIDATION=1`。
//!
//! RED 轴(全部作用在 **device 上传面**,host 金标准不动):
//!   * `red_stale`:抑制一次失效 —— 逐帧上传**上一帧**的物理池(页表说驻留且净,
//!     内容却是上一帧 = stale 页)+ mv 段 z 区间拧偏 → `depth_pool` digest 与深度
//!     对拍双红。(注:快照取于光栅后,此刻无脏位可清,故 stale 只能落在池内容上。)
//!   * `red_missing_local`:local 页不入批 —— local 页表段清零 + mv 段 local
//!     `tri_count=0` → `page_table` digest(F12–15)与深度对拍必红。

use rurix_render::shadow::clipmap::LightBasis;
use rurix_render::shadow::events::sha256_hex;
use rurix_render::shadow::page_cache::{
    FrameDeviceSnapshot, M19RunResult, MarkFrameSnapshot, run_m19_fixture,
};
use rurix_render::shadow::vsm::{ShadowTri, Vsm};
use rurix_rt::render_exec::{
    self, Bindings, BufferDesc, BufferUsage, ComputePass, DispatchSpec, KernelWave, Pass, Readback,
    ResourceDesc,
};

const MV_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vsm_depth_raster_mv.spv"));
const SAMPLE_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vsm_sample.spv"));
const SAMPLE_LOCAL_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vsm_sample_local.spv"));
const MARK_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vsm_page_mark_project.spv"));
const PAGE_TEXELS: usize = 128 * 128;
/// `vsm_page_mark_project.rx` 的 `page_bits: AtomicView<global, u32, (4096,)>` 声明字数。
const MARK_BITS_WORDS: usize = 4096;
/// 每级位图字数(128×128 bit)。
const MARK_WORDS_PER_LEVEL: usize = 512;

/// mark 段 RED 轴(全部作用在**「device 位图从哪来」**这一点上)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkRed {
    /// 正常:每帧 dispatch `vsm_page_mark_project` 并 readback。
    None,
    /// 跳过 dispatch,拿零位图冒充 device 结果(= 核编译进 SPV 但无人消费的旧态)。
    SkipDispatch,
    /// 不 dispatch,改用 **host 预知 page id**(A2.1 前的 `mark_slot` 硬编码四页)
    /// 生成位图冒充 device 结果 —— 位图逐帧恒定,与深度驱动的真位图在 F13+ 分叉。
    HostImpostor,
}

fn storage<'a>(size: usize, data: Option<&'a [u8]>) -> ResourceDesc<'a> {
    ResourceDesc::Buffer(BufferDesc {
        size: size as u64,
        usage: BufferUsage {
            storage: true,
            ..Default::default()
        },
        data,
    })
}

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

fn gate() -> Option<render_exec::DeviceCaps> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("[uc06 M19] SKIP: vulkan loader 不可用");
        return None;
    }
    let caps = render_exec::probe_device_caps().ok()?;
    match render_exec::require_wave(&caps, KernelWave::W1) {
        Ok(()) => Some(caps),
        Err(e) => {
            eprintln!("[uc06 M19] SKIP: W1 能力链缺失: {e}");
            None
        }
    }
}

fn dispatch_mv(
    tris: &[f32],
    pages: &[f32],
    meta: &[u32],
    page_count: u32,
) -> Result<Vec<f32>, String> {
    let out_len = page_count as usize * PAGE_TEXELS;
    let tris_b = bytes_f32(tris);
    let pages_b = bytes_f32(pages);
    let meta_b = bytes_u32(meta);
    let resources = [
        storage(tris_b.len(), Some(&tris_b)),
        storage(pages_b.len(), Some(&pages_b)),
        storage(meta_b.len(), Some(&meta_b)),
        storage(out_len * 4, None),
    ];
    let readbacks = [Readback::Buffer {
        res: 3,
        offset: 0,
        size: (out_len * 4) as u64,
    }];
    let passes = [Pass::Compute(ComputePass {
        name: "vsm_depth_raster_mv",
        spirv: MV_SPV,
        entry: None,
        dispatch: DispatchSpec::Direct([page_count * PAGE_TEXELS as u32, 1, 1]),
        bindings: Bindings {
            storage_buffers: vec![0, 1, 2, 3],
            push_constants: bytes_u32(&[page_count]),
            ..Default::default()
        },
    })];
    let barriers: [&[(u32, render_exec::TargetState)]; 1] = [&[]];
    let out = render_exec::execute_frame(&resources, &passes, &barriers, &readbacks)?;
    Ok(read_f32(&out[0]))
}

/// 一帧 device `vsm_page_mark_project`:主相机深度 → 反投影/选级/回退 → 位图。
///
/// 输入逐字段对齐 kernel 签名(buffer 声明序 depth/inv_vp/lparams/page_bits;
/// push 常量声明序 pixel_count/width/levels + cam/right/up/fwd/base_radius)。
/// 返回 readback 出的**全量** 4096 字位图(声明形态 `(4096,)`)。
fn dispatch_mark(m: &MarkFrameSnapshot) -> Result<Vec<u32>, String> {
    let depth_b = bytes_f32(&m.depth);
    let inv_b = bytes_f32(&m.inv_vp);
    let lp_b = bytes_f32(&m.lparams);
    let bits_b = bytes_u32(&vec![0u32; MARK_BITS_WORDS]);
    let resources = [
        storage(depth_b.len(), Some(&depth_b)),
        storage(inv_b.len(), Some(&inv_b)),
        storage(lp_b.len(), Some(&lp_b)),
        storage(bits_b.len(), Some(&bits_b)),
    ];
    let readbacks = [Readback::Buffer {
        res: 3,
        offset: 0,
        size: bits_b.len() as u64,
    }];
    let pixel_count = m.depth.len() as u32;
    let mut push = bytes_u32(&[pixel_count, m.width, m.levels]);
    for v in [
        m.cam[0],
        m.cam[1],
        m.cam[2],
        m.right[0],
        m.right[1],
        m.right[2],
        m.up[0],
        m.up[1],
        m.up[2],
        m.fwd[0],
        m.fwd[1],
        m.fwd[2],
        m.base_radius,
    ] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    let out = dispatch_one(
        "vsm_page_mark_project",
        MARK_SPV,
        &resources,
        vec![0, 1, 2, 3],
        push,
        pixel_count,
        &readbacks,
    )?;
    Ok(read_u32(&out[0]))
}

/// A2.1 前 host 「预知 page id」的四页硬编码(`mark_slot` 时代的标记源)。
const IMPOSTOR_SLOTS: [(u8, u8, u8); 4] = [(0, 0, 0), (0, 1, 0), (1, 0, 0), (2, 0, 0)];

/// 一帧 mark 段结果(device 位图 vs host 镜像位图的逐位/逐槽对拍)。
struct FrameMarkResult {
    frame: u32,
    /// device 位图(截到 `levels*512` 字)。
    bits: Vec<u32>,
    /// device 位图置位数(= 本帧标记页数;0 ⇒ 段空转)。
    set_bits: u32,
    /// 与 host 镜像位图不等的字数。
    word_mismatches: u32,
    /// 位图反解后与 host 标记槽列表不一致的槽数。
    slot_mismatches: u32,
    /// `levels*512` 之后仍被写脏的字数(核不应触及,非 0 = 越界写)。
    tail_dirty: u32,
    matched: bool,
    dispatched: bool,
}

fn run_frame_mark(m: &MarkFrameSnapshot, red: MarkRed) -> Result<FrameMarkResult, String> {
    let words = m.levels as usize * MARK_WORDS_PER_LEVEL;
    if m.host_bits.len() != words {
        return Err(format!("host 位图字数 {} ≠ {words}", m.host_bits.len()));
    }
    let (full, dispatched) = match red {
        MarkRed::None => (dispatch_mark(m)?, true),
        // RED:不 dispatch —— 位图不是 device 产的。
        MarkRed::SkipDispatch => (vec![0u32; MARK_BITS_WORDS], false),
        MarkRed::HostImpostor => {
            let mut bits = vec![0u32; MARK_BITS_WORDS];
            for &(l, x, y) in &IMPOSTOR_SLOTS {
                let idx = usize::from(l) * (128 * 128) + usize::from(y) * 128 + usize::from(x);
                bits[idx / 32] |= 1u32 << (idx % 32);
            }
            (bits, false)
        }
    };
    if full.len() < MARK_BITS_WORDS {
        return Err(format!("mark 位图 readback 字数 {} 不足", full.len()));
    }
    let bits = full[..words].to_vec();
    let tail_dirty = full[words..].iter().filter(|w| **w != 0).count() as u32;
    let set_bits: u32 = bits.iter().map(|w| w.count_ones()).sum();
    let word_mismatches = bits
        .iter()
        .zip(m.host_bits.iter())
        .filter(|(a, b)| a != b)
        .count() as u32;
    let dev_slots = Vsm::marked_slots_from_bitmap(&bits, m.levels as u8);
    let slot_mismatches = if dev_slots.len() != m.host_slots.len() {
        dev_slots.len().abs_diff(m.host_slots.len()) as u32
            + dev_slots
                .iter()
                .zip(m.host_slots.iter())
                .filter(|(a, b)| a != b)
                .count() as u32
    } else {
        dev_slots
            .iter()
            .zip(m.host_slots.iter())
            .filter(|(a, b)| a != b)
            .count() as u32
    };
    Ok(FrameMarkResult {
        frame: m.frame,
        matched: word_mismatches == 0 && slot_mismatches == 0 && tail_dirty == 0 && set_bits > 0,
        bits,
        set_bits,
        word_mismatches,
        slot_mismatches,
        tail_dirty,
        dispatched,
    })
}

fn flatten_tris_light(basis: &LightBasis, tris: &[ShadowTri]) -> Vec<f32> {
    let mut out = Vec::with_capacity(tris.len() * 9);
    for t in tris {
        for v in t.v {
            let l = basis.to_light(v);
            out.extend_from_slice(&l);
        }
    }
    out
}

/// 一帧 device 侧 readback 出的三个 digest + 采样逐值对拍结果。
struct FrameDeviceDigest {
    frame: u32,
    /// sha256(dir 页表 readback ++ local 页表 readback) —— `digest_tables` 同原像。
    page_table: String,
    /// sha256(物理池 readback) —— `digest_pool` 同原像。
    depth_pool: String,
    /// sha256(device 方向光采样值 ++ device local 采样值) —— `digest_samples` 同原像。
    sample: String,
    /// device 采样值与 host oracle 不一致数(0/1 二值 ⇒ 零容差)。
    sample_mismatches: u32,
    /// 本帧 device 采样值总数。
    sample_count: u32,
    /// device 判为**遮蔽**(0.0)的采样数 —— 非退化证据(全 lit 的采样臂等于空转)。
    shadowed: u32,
    /// 本帧是否含 local spot 臂。
    has_local: bool,
    /// 本帧 dispatch 数(1 或 2)。
    dispatches: u32,
}

/// 逐帧 device 消费 + readback digest。
///
/// 上传的 `entries`/`pool` 逐字取自 host 快照(装配面),但**digest 的原像取自
/// device readback**:kernel 真读这些 buffer 算出 0/1 采样值,readback 回来再算
/// sha256 —— 故三个 digest 与 golden 比对时不存在 host 代填路径。
fn run_frame_digests(
    host: &M19RunResult,
    red_stale: bool,
    red_missing_local: bool,
) -> Result<Vec<FrameDeviceDigest>, String> {
    host.device_frames
        .iter()
        .enumerate()
        .map(|(i, snap)| {
            // RED stale:上一帧的物理池 = 「本帧那次失效被抑制了」的直接后果
            // (页表说驻留且净,内容却还是上一帧)→ depth_pool digest 必红。
            let stale_pool = if red_stale && i > 0 {
                Some(&host.device_frames[i - 1].pool)
            } else {
                None
            };
            run_one_frame_digest(snap, stale_pool, red_missing_local)
        })
        .collect()
}

fn run_one_frame_digest(
    snap: &FrameDeviceSnapshot,
    stale_pool: Option<&Vec<f32>>,
    red_missing_local: bool,
) -> Result<FrameDeviceDigest, String> {
    let levels = snap.levels as usize;
    let dir_len = levels * PAGE_TEXELS;
    if snap.entries.len() < dir_len {
        return Err(format!(
            "F{} 页表快照长度 {} < levels*16384 = {dir_len}",
            snap.frame,
            snap.entries.len()
        ));
    }
    let dir_entries = snap.entries[..dir_len].to_vec();
    let mut local_entries = snap.entries[dir_len..].to_vec();

    // RED:local 页不入批 = local 页表段清零(非驻留)。
    if red_missing_local {
        for e in local_entries.iter_mut() {
            *e = 0;
        }
    }

    let samples: Vec<f32> = snap.sample_pts.iter().flat_map(|p| *p).collect();
    let samples_b = bytes_f32(&samples);
    let lparams_b = bytes_f32(&snap.lparams);
    let entries_b = bytes_u32(&dir_entries);
    let pool_upload = stale_pool.unwrap_or(&snap.pool);
    if pool_upload.len() != snap.pool.len() {
        return Err(format!("F{} stale pool 长度不符", snap.frame));
    }
    let pool_b = bytes_f32(pool_upload);
    let out_size = snap.sample_pts.len() * 4;
    let resources = [
        storage(samples_b.len(), Some(&samples_b)),
        storage(lparams_b.len(), Some(&lparams_b)),
        storage(entries_b.len(), Some(&entries_b)),
        storage(pool_b.len(), Some(&pool_b)),
        storage(out_size, None),
    ];
    // readback ②③ = kernel 实际绑定的页表/池 buffer:digest 原像取自 device 内存。
    let readbacks = [
        Readback::Buffer {
            res: 4,
            offset: 0,
            size: out_size as u64,
        },
        Readback::Buffer {
            res: 2,
            offset: 0,
            size: entries_b.len() as u64,
        },
        Readback::Buffer {
            res: 3,
            offset: 0,
            size: pool_b.len() as u64,
        },
    ];
    let mut push = bytes_u32(&[snap.sample_pts.len() as u32, snap.levels, snap.pool_pages]);
    for v in [
        snap.cam[0],
        snap.cam[1],
        snap.cam[2],
        snap.right[0],
        snap.right[1],
        snap.right[2],
        snap.up[0],
        snap.up[1],
        snap.up[2],
        snap.fwd[0],
        snap.fwd[1],
        snap.fwd[2],
        snap.base_radius,
        snap.depth_bias,
    ] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    let out = dispatch_one(
        "vsm_sample",
        SAMPLE_SPV,
        &resources,
        vec![0, 1, 2, 3, 4],
        push,
        snap.sample_pts.len() as u32,
        &readbacks,
    )?;
    let dir_values = read_f32(&out[0]);
    let mut pt_bytes = out[1].clone();
    let pool_readback = out[2].clone();

    let mut sample_bytes = out[0].clone();
    let mut shadowed = dir_values.iter().filter(|v| **v == 0.0).count() as u32;
    let mut sample_mismatches = dir_values
        .iter()
        .zip(snap.host_dir_values.iter())
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count() as u32;
    let mut sample_count = dir_values.len() as u32;
    let mut dispatches = 1u32;

    if let Some(loc) = snap.local.as_ref() {
        let lsamples: Vec<f32> = loc.query_pts.iter().flat_map(|p| *p).collect();
        let lsamples_b = bytes_f32(&lsamples);
        let lentries_b = bytes_u32(&local_entries);
        let lout_size = loc.query_pts.len() * 4;
        let lres = [
            storage(lsamples_b.len(), Some(&lsamples_b)),
            storage(lentries_b.len(), Some(&lentries_b)),
            storage(pool_b.len(), Some(&pool_b)),
            storage(lout_size, None),
        ];
        let lrb = [
            Readback::Buffer {
                res: 3,
                offset: 0,
                size: lout_size as u64,
            },
            Readback::Buffer {
                res: 1,
                offset: 0,
                size: lentries_b.len() as u64,
            },
        ];
        let mut lpush = bytes_u32(&[loc.query_pts.len() as u32, snap.pool_pages]);
        for v in [
            loc.page_world,
            loc.z_range[0],
            loc.z_range[1],
            snap.depth_bias,
        ] {
            lpush.extend_from_slice(&v.to_le_bytes());
        }
        let lout = dispatch_one(
            "vsm_sample_local",
            SAMPLE_LOCAL_SPV,
            &lres,
            vec![0, 1, 2, 3],
            lpush,
            loc.query_pts.len() as u32,
            &lrb,
        )?;
        let lvalues = read_f32(&lout[0]);
        shadowed += lvalues.iter().filter(|v| **v == 0.0).count() as u32;
        sample_mismatches += lvalues
            .iter()
            .zip(loc.host_values.iter())
            .filter(|(a, b)| a.to_bits() != b.to_bits())
            .count() as u32;
        sample_count += lvalues.len() as u32;
        // digest 拼接序 = host `digest_samples(dir ++ local)`。
        sample_bytes.extend_from_slice(&lout[0]);
        // 页表 digest 尾段 = local 单级页表(host `digest_tables` 同序)。
        pt_bytes.extend_from_slice(&lout[1]);
        dispatches = 2;
    }

    Ok(FrameDeviceDigest {
        frame: snap.frame,
        page_table: sha256_hex(&pt_bytes),
        depth_pool: sha256_hex(&pool_readback),
        sample: sha256_hex(&sample_bytes),
        sample_mismatches,
        sample_count,
        shadowed,
        has_local: snap.local.is_some(),
        dispatches,
    })
}

fn dispatch_one(
    name: &'static str,
    spirv: &'static [u8],
    resources: &[ResourceDesc<'_>],
    storage_buffers: Vec<u32>,
    push_constants: Vec<u8>,
    threads: u32,
    readbacks: &[Readback],
) -> Result<Vec<Vec<u8>>, String> {
    let passes = [Pass::Compute(ComputePass {
        name,
        spirv,
        entry: None,
        dispatch: DispatchSpec::Direct([threads, 1, 1]),
        bindings: Bindings {
            storage_buffers,
            push_constants,
            ..Default::default()
        },
    })];
    let barriers: [&[(u32, render_exec::TargetState)]; 1] = [&[]];
    render_exec::execute_frame(resources, &passes, &barriers, readbacks)
}

fn flatten_local(tris: &[[[f32; 3]; 3]]) -> Vec<f32> {
    let mut out = Vec::with_capacity(tris.len() * 9);
    for t in tris {
        for v in t {
            out.extend_from_slice(v);
        }
    }
    out
}

/// 对 fixture 中首个 multi-view(≥5)且含脏页的帧做 device 深度对拍。
pub fn run_m19_device(
    red_stale: bool,
    red_missing_local: bool,
    mark_red: MarkRed,
) -> Option<Result<String, String>> {
    let _caps = gate()?;
    let host = run_m19_fixture();
    match run_device_inner(&host, red_stale, red_missing_local, mark_red) {
        Ok(json) => Some(Ok(json)),
        Err(e) => Some(Err(e)),
    }
}

fn run_device_inner(
    host: &M19RunResult,
    red_stale: bool,
    red_missing_local: bool,
    mark_red: MarkRed,
) -> Result<String, String> {
    let batch = host
        .batches
        .iter()
        .find(|b| b.view_count >= 5 && !b.pages.is_empty())
        .ok_or("无 multi-view 脏页批次")?;

    let basis = LightBasis::from_direction([0.02, 0.0, -1.0]);
    // F12+ 灯已微转;fixture F5 后方向为 [0.02,0,-1]
    let mut tris = flatten_tris_light(&basis, &batch.dir_tris);
    let dir_tri_count = batch.dir_tris.len();
    let local_off = dir_tri_count;
    // host 对照始终含 local 三角形;RED 只在 device meta 上置 tri_count=0。
    tris.extend(flatten_local(&batch.local_tris_light));
    let local_tri_count = batch.local_tris_light.len();

    let selected: Vec<_> = batch.pages.clone();
    // host 金标准恒含 local;RED missing-local = device 侧 local tri_count 置 0。
    let host_depth = host_gather_selected(
        &selected,
        &tris,
        dir_tri_count,
        local_off,
        local_tri_count,
        /*tamper_z*/ false,
    );
    let mut pages = Vec::new();
    let mut meta = Vec::new();
    for p in &selected {
        let is_local = p.view_id >= 4;
        let (tri_off, tri_count) = if is_local {
            if red_missing_local {
                (0u32, 0u32)
            } else {
                (local_off as u32, local_tri_count as u32)
            }
        } else {
            (0u32, dir_tri_count as u32)
        };
        let mut z0 = p.z_range[0];
        let mut z1 = p.z_range[1];
        if red_stale {
            // 抑制失效:故意把 z 区间拧偏 → digest 必红
            z0 += 0.25;
            z1 += 0.25;
        }
        pages.extend_from_slice(&[p.origin[0], p.origin[1], p.page_world, z0, z1]);
        meta.extend_from_slice(&[tri_off, tri_count]);
    }

    let page_count = (pages.len() / 5) as u32;
    if page_count == 0 {
        return Err("device 页批次为空".into());
    }
    let device = dispatch_mv(&tris, &pages, &meta, page_count)?;

    // ⓪ 逐帧 mark 段(A2.1 主体;设计 §2.1 帧循环**第一行**)。
    // 每帧一次 `vsm_page_mark_project` dispatch + 位图 readback,与 host 镜像位图
    // 逐位 + 逐槽对拍;host 的 MarkHit/MarkMiss 只在全帧位图相等时才算成立。
    let marks: Vec<FrameMarkResult> = host
        .device_frames
        .iter()
        .map(|s| run_frame_mark(&s.mark, mark_red))
        .collect::<Result<_, _>>()?;
    let mark_frames_matched = marks.iter().filter(|m| m.matched).count() as u32;
    let mark_bits_total: u32 = marks.iter().map(|m| m.set_bits).sum();
    let mark_word_mismatches: u32 = marks.iter().map(|m| m.word_mismatches).sum();
    let mark_slot_mismatches: u32 = marks.iter().map(|m| m.slot_mismatches).sum();
    let mark_tail_dirty: u32 = marks.iter().map(|m| m.tail_dirty).sum();
    let mark_dispatches = marks.iter().filter(|m| m.dispatched).count() as u32;
    // 位图去重数:深度驱动的位图必随帧变化;恒定位图 = host 预知 page id 的等价物。
    let mut distinct: Vec<&Vec<u32>> = Vec::new();
    for m in &marks {
        if !distinct.iter().any(|b| **b == m.bits) {
            distinct.push(&m.bits);
        }
    }
    let mark_distinct_bitmaps = distinct.len() as u32;
    // 「device 真读深度缓冲」的**结构性**证据(不靠自述字段):F12→F13 的唯一输入
    // 差异就是深度(inv_vp / lparams / cam / 灯基 / base_radius / levels 逐字相同 ——
    // 相机自 F4 起不动、灯基自 F5 起不变、窗口与 z 区间随之冻结),而两帧 device
    // 位图不同 ⇒ 位图只能是深度反投影的结果,不可能从 lparams/常量凑出来。
    let mark_depth_is_causal = if marks.len() > 13 {
        let (a, b) = (&host.device_frames[12].mark, &host.device_frames[13].mark);
        let same_inputs = a.inv_vp == b.inv_vp
            && a.lparams == b.lparams
            && a.cam == b.cam
            && a.right == b.right
            && a.up == b.up
            && a.fwd == b.fwd
            && a.base_radius == b.base_radius
            && a.levels == b.levels;
        same_inputs && a.depth != b.depth && marks[12].bits != marks[13].bits
    } else {
        false
    };
    let mark_all_match = mark_depth_is_causal
        && mark_frames_matched == marks.len() as u32
        && mark_word_mismatches == 0
        && mark_slot_mismatches == 0
        && mark_tail_dirty == 0
        && mark_bits_total > 0
        && mark_distinct_bitmaps >= 2
        && mark_dispatches == marks.len() as u32;

    // ① 逐帧 device digest 段(A2 主体)。
    let frames = run_frame_digests(host, red_stale, red_missing_local)?;
    if frames.len() != host.digests.len() {
        return Err(format!(
            "device 逐帧段 {} 帧 ≠ host golden {} 帧",
            frames.len(),
            host.digests.len()
        ));
    }
    let mut pt_matched = 0u32;
    let mut pool_matched = 0u32;
    let mut sample_matched = 0u32;
    let mut sample_mismatches = 0u32;
    let mut frames_with_local = 0u32;
    let mut frame_dispatches = 0u32;
    let mut shadowed_samples = 0u32;
    let mut total_samples = 0u32;
    for (d, g) in frames.iter().zip(host.digests.iter()) {
        if d.page_table == g.page_table {
            pt_matched += 1;
        }
        if d.depth_pool == g.depth_pool {
            pool_matched += 1;
        }
        if d.sample == g.sample {
            sample_matched += 1;
        }
        sample_mismatches += d.sample_mismatches;
        shadowed_samples += d.shadowed;
        total_samples += d.sample_count;
        if d.has_local {
            frames_with_local += 1;
        }
        frame_dispatches += d.dispatches;
    }
    let n = frames.len() as u32;
    let frames_json = frames
        .iter()
        .map(|d| {
            format!(
                "{{\"frame\":{},\"page_table\":\"{}\",\"depth_pool\":\"{}\",\
                 \"sample\":\"{}\",\"sample_count\":{},\"sample_mismatches\":{},\
                 \"shadowed\":{},\"has_local\":{},\"dispatches\":{}}}",
                d.frame,
                d.page_table,
                d.depth_pool,
                d.sample,
                d.sample_count,
                d.sample_mismatches,
                d.shadowed,
                d.has_local,
                d.dispatches
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let digests_all_match =
        pt_matched == n && pool_matched == n && sample_matched == n && sample_mismatches == 0;
    let last = frames.last().ok_or("device 逐帧段为空")?;

    let mut bitexact = 0u32;
    let mut max_abs = 0.0f32;
    for (&a, &b) in device.iter().zip(host_depth.iter()) {
        if a.to_bits() == b.to_bits() {
            bitexact += 1;
        }
        max_abs = max_abs.max((a - b).abs());
    }
    let depth_digest = sha256_hex(
        &device
            .iter()
            .flat_map(|f| f.to_bits().to_le_bytes())
            .collect::<Vec<_>>(),
    );
    let host_digest = sha256_hex(
        &host_depth
            .iter()
            .flat_map(|f| f.to_bits().to_le_bytes())
            .collect::<Vec<_>>(),
    );
    // G7.5 VSM depth 曾以 measured 1e-6 冻结;本门 multi-view 臂在 4070 类
    // 设备上实测 max_abs ~1e-7。bitexact 全等仍优先;否则 ≤1e-6 记对拍通过。
    const TOL_DEPTH: f32 = 1e-6;
    let depth_match = max_abs <= TOL_DEPTH;
    let red_mode = red_stale || red_missing_local || mark_red != MarkRed::None;
    let pass = depth_match && digests_all_match && mark_all_match && !red_mode;
    // RED 轴**逐臂**判红:被扰动的那条轴自己必须翻红(不许由别的轴顶包)。
    let red_ok = red_mode
        && (!red_stale || (!depth_match || pool_matched < n))
        && (!red_missing_local || (pt_matched < n || sample_matched < n))
        && (mark_red == MarkRed::None || !mark_all_match);

    // validation ERROR 计数:取 render_exec messenger 的**进程实数**(A2.1 前是
    // `let validation_errors = 0u32;` 写死字面量)。`messenger` 位为假 ⇒ layer 没
    // 装上,0 不可信 —— smoke 的 `validation_zero` 要求两者同时成立。
    let validation_errors = render_exec::validation_error_total();
    let validation_messenger = render_exec::validation_messenger_installed();
    let marks_json = marks
        .iter()
        .map(|m| {
            format!(
                "{{\"frame\":{},\"mark_bits_set\":{},\"word_mismatches\":{},\
                 \"slot_mismatches\":{},\"tail_dirty\":{},\"dispatched\":{},\"match\":{}}}",
                m.frame,
                m.set_bits,
                m.word_mismatches,
                m.slot_mismatches,
                m.tail_dirty,
                m.dispatched,
                m.matched
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{{\
         \"subject\":\"g8_m19_vsm_page_cache\",\
         \"device_state\":\"executed\",\
         \"view_count\":{},\
         \"page_count\":{},\
         \"dispatch_count\":{},\
         \"depth_texels\":{},\
         \"bitexact_texels\":{},\
         \"measured_depth_max_abs\":{:.9e},\
         \"tol_depth\":{:.9e},\
         \"depth_digest\":\"{}\",\
         \"host_depth_digest\":\"{}\",\
         \"depth_match\":{},\
         \"frames_checked\":{},\
         \"frames_with_local\":{},\
         \"page_table_digest_frames_matched\":{},\
         \"depth_pool_digest_frames_matched\":{},\
         \"sample_digest_frames_matched\":{},\
         \"sample_value_mismatches\":{},\
         \"device_samples_total\":{},\
         \"device_samples_shadowed\":{},\
         \"digests_all_match\":{},\
         \"device_frames\":[{}],\
         \"page_table_digest\":\"{}\",\
         \"depth_pool_digest\":\"{}\",\
         \"sample_digest\":\"{}\",\
         \"digest_provenance\":\"device_readback\",\
         \"mark_provenance\":\"{}\",\
         \"mark_kernel\":\"vsm_page_mark_project\",\
         \"mark_dispatches\":{},\
         \"mark_frames_matched\":{},\
         \"mark_bits_total\":{},\
         \"mark_word_mismatches\":{},\
         \"mark_slot_mismatches\":{},\
         \"mark_tail_dirty\":{},\
         \"mark_distinct_bitmaps\":{},\
         \"mark_pixels_per_frame\":{},\
         \"mark_depth_is_causal\":{},\
         \"mark_all_match\":{},\
         \"mark_frames\":[{}],\
         \"host_events_sha256\":\"{}\",\
         \"validation_errors\":{},\
         \"validation_messenger\":{},\
         \"red_stale\":{},\
         \"red_missing_local\":{},\
         \"red_skip_mark\":{},\
         \"red_host_mark\":{},\
         \"red_ok\":{},\
         \"pass\":{}\
         }}",
        batch.view_count,
        page_count,
        frame_dispatches + 1 + mark_dispatches,
        device.len(),
        bitexact,
        max_abs,
        TOL_DEPTH,
        depth_digest,
        host_digest,
        depth_match,
        n,
        frames_with_local,
        pt_matched,
        pool_matched,
        sample_matched,
        sample_mismatches,
        total_samples,
        shadowed_samples,
        digests_all_match,
        frames_json,
        last.page_table,
        last.depth_pool,
        last.sample,
        match mark_red {
            MarkRed::None => "device_readback",
            MarkRed::SkipDispatch => "red_skip_dispatch",
            MarkRed::HostImpostor => "red_host_precomputed_page_ids",
        },
        mark_dispatches,
        mark_frames_matched,
        mark_bits_total,
        mark_word_mismatches,
        mark_slot_mismatches,
        mark_tail_dirty,
        mark_distinct_bitmaps,
        host.device_frames
            .first()
            .map(|s| s.mark.pixels)
            .unwrap_or(0),
        mark_depth_is_causal,
        mark_all_match,
        marks_json,
        host.events_sha256,
        validation_errors,
        validation_messenger,
        red_stale,
        red_missing_local,
        mark_red == MarkRed::SkipDispatch,
        mark_red == MarkRed::HostImpostor,
        red_ok,
        pass || red_ok
    ))
}

/// host gather 对照(与 kernel 同序同公式;tamper_z=false = 金标准臂)。
fn host_gather_selected(
    pages: &[rurix_render::shadow::vsm::DirtyPageRef],
    tris: &[f32],
    dir_tri_count: usize,
    local_off: usize,
    local_tri_count: usize,
    tamper_z: bool,
) -> Vec<f32> {
    let mut out = Vec::with_capacity(pages.len() * PAGE_TEXELS);
    for p in pages {
        let is_local = p.view_id >= 4;
        let (tri_off, tri_count) = if is_local {
            (local_off, local_tri_count)
        } else {
            (0, dir_tri_count)
        };
        let mut zr = p.z_range;
        if tamper_z {
            zr[0] += 0.25;
            zr[1] += 0.25;
        }
        let mut page = vec![1.0f32; PAGE_TEXELS];
        for k in 0..tri_count {
            let b = (tri_off + k) * 9;
            if b + 8 >= tris.len() {
                break;
            }
            let v = [
                [tris[b], tris[b + 1], tris[b + 2]],
                [tris[b + 3], tris[b + 4], tris[b + 5]],
                [tris[b + 6], tris[b + 7], tris[b + 8]],
            ];
            raster_into(&mut page, v, p.origin, p.page_world, zr);
        }
        out.extend_from_slice(&page);
    }
    out
}

fn raster_into(page: &mut [f32], v: [[f32; 3]; 3], origin: [f32; 2], pw: f32, zr: [f32; 2]) {
    let n = 128.0f32;
    let mut tx = [0.0f32; 3];
    let mut ty = [0.0f32; 3];
    let mut dep = [0.0f32; 3];
    for i in 0..3 {
        tx[i] = (v[i][0] - origin[0]) / pw * n;
        ty[i] = (v[i][1] - origin[1]) / pw * n;
        dep[i] = (v[i][2] - zr[0]) / (zr[1] - zr[0]);
    }
    let edge = |ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32| {
        (bx - ax) * (py - ay) - (by - ay) * (px - ax)
    };
    let area = edge(tx[0], ty[0], tx[1], ty[1], tx[2], ty[2]);
    if area.abs() < 1e-12 {
        return;
    }
    for j in 0..128 {
        for i in 0..128 {
            let (px, py) = (i as f32 + 0.5, j as f32 + 0.5);
            let w0 = edge(tx[1], ty[1], tx[2], ty[2], px, py) / area;
            let w1 = edge(tx[2], ty[2], tx[0], ty[0], px, py) / area;
            let w2 = edge(tx[0], ty[0], tx[1], ty[1], px, py) / area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let d = w0 * dep[0] + w1 * dep[1] + w2 * dep[2];
                let cell = &mut page[j * 128 + i];
                if d < *cell {
                    *cell = d;
                }
            }
        }
    }
}
