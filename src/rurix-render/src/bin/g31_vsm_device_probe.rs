//! G31 W2 判档件:VSM 页管线 device 闭环 probe(TODO #104/#106 判档配套;
//! 判档报告 = `artifacts/day_0830_delivery/w2_wiring/vsm_shadow/REPORT.md`)。
//!
//! ## 判档背景(报告 §2 结构论证的代码面)
//!
//! 生产窗口车道(g14_3/g31)阴影 = RayQuery compute 内联遮挡射线
//! (`kernels/g14_3_direct_gi.rx` 阴影射线;day_0829 soft-shadows 臂 = 逐灯
//! 圆盘采样),无 shadow map 生成 pass 可摊销 ⇒ VSM 页管线(mark→alloc→
//! invalidate→只重光栅脏页→采样)在该车道**无消费面**,接入判 no-go/留窗
//! (既有登记:`milestones/g31/g31_rejudgment_windows.json` SMRT 行半②)。
//!
//! 本 probe 是方案 A 的补件:**不接生产车道**,而是给 G31 战役一个独立可
//! 复跑的 device 闭环判档件,消费冻结金标准 `shadow::page_cache::run_m19_fixture`
//! (16 帧脚本;vsm.rs / page_cache.rs 本体 0-byte),三腿:
//!
//! * **腿⓪ mark**:逐帧 dispatch `vsm_page_mark_project`(主相机深度 →
//!   反投影 → 选级 → 出窗回退 → 原子位图),readback 与 host 镜像
//!   [`Vsm::page_mark_bits`] 逐位 + 逐槽对拍(A2.1 同判据;历史「编而不
//!   dispatch」缺口已由 uc06 M19 门 A2.1 关闭,本腿 = G31 侧独立复跑面)。
//! * **腿① invalidate→raster**:逐帧把该帧「脏且驻留」页批次(五类失效源
//!   CasterMoved/ClipmapScroll/LightChanged/NonVirtualCaster/Evicted 的直接
//!   产物)交 `vsm_depth_raster_mv` 在 device 重建,与逐帧金标准
//!   `FrameDigest.dirty_depth`(至今无 device 腿消费的 golden 轴)对拍:
//!   sha256 严格臂如实登记 + 逐纹素 max_abs ≤ 1e-6(G7.5 冻结口径)硬判据。
//! * **腿② alloc→sample**:逐帧 dispatch `vsm_sample`(+F12 起
//!   `vsm_sample_local`)——device 真读页表(驻留/脏/物理页 = alloc/失效
//!   决策的落地态)与物理池产 0/1,逐值位级对拍 + sample digest 对拍。
//!
//! 判读(судья)逻辑纯 host 且与 `--selftest` 共用同一函数:selftest 先以
//! host 镜像喂судья(绿臂必须全绿),再做三条证伪臂(位图翻位 / 纹素扰动
//! +2e-6 / 采样值翻转 —— 各自的腿必须翻红),证明判据非空转。
//!
//! ## 用法
//!
//! ```text
//! g31_vsm_device_probe --selftest [--out <json>]        # 纯 CPU:金标准自跑 + судья自证
//! g31_vsm_device_probe --spv-dir <dir> [--out <json>]   # device 腿(真 GPU)
//! ```
//!
//! `--spv-dir` 需含 `vsm_page_mark_project.spv` / `vsm_depth_raster_mv.spv` /
//! `vsm_sample.spv` / `vsm_sample_local.spv`(源 = `apps/uc06-renderer/kernels/`
//! 冻结 `.rx`,rurixc --target vulkan 产物 + spirv-val 通过;本 bin 只运行时
//! 装载,不含编译面)。三态纪律:无 Vulkan loader/设备 → `skipped_dev_env`
//! 退 0(`RURIX_REQUIRE_REAL=1` 时翻硬红退 1);判据不符退 1。

#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use rurix_render::shadow::clipmap::LightBasis;
use rurix_render::shadow::events::sha256_hex;
use rurix_render::shadow::local::LOCAL_LEVEL_TAG;
use rurix_render::shadow::page_cache::{
    FrameDeviceSnapshot, M19RunResult, MarkFrameSnapshot, ShadowViewBatch, run_m19_fixture,
};
use rurix_render::shadow::vsm::Vsm;
use rurix_rt::render_exec::{
    self, Bindings, BufferDesc, BufferUsage, ComputePass, DispatchSpec, KernelWave, Pass, Readback,
    ResourceDesc,
};

const TAG: &str = "[g31_vsm_device_probe]";
/// 物理页纹素数 = 页表单级槽数(128×128)。
const PAGE_TEXELS: usize = 128 * 128;
/// `vsm_page_mark_project.rx` 的 `page_bits` 声明字数(readback 全量)。
const MARK_BITS_WORDS: usize = 4096;
/// 每级位图字数(128×128 bit)。
const MARK_WORDS_PER_LEVEL: usize = 512;
/// 深度对拍容差(G7.5 VSM depth measured 冻结口径;uc06 M19 mv 臂同值)。
const TOL_DEPTH: f32 = 1e-6;
/// 腿① RED 臂纹素扰动量(> TOL_DEPTH,судья必须翻红)。
const RED_TEXEL_EPS: f32 = 2e-6;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG} FAIL: {msg}");
    std::process::exit(2)
}

// ---------------------------------------------------------------------------
// 字节胶水
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

/// f32 序列位型拼接 sha256(= fixture `digest_samples` / `dirty_depth` 同原像)。
fn sha_f32bits(v: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for &f in v {
        bytes.extend_from_slice(&f.to_bits().to_le_bytes());
    }
    sha256_hex(&bytes)
}

// ---------------------------------------------------------------------------
// 期望值构造(host;selftest 与 device 判读共用同一数据路径)
// ---------------------------------------------------------------------------

/// 逐帧脏页批次的 host 期望纹素 = 金标准快照物理池按批次序切片拼接
/// (`FrameDigest.dirty_depth` 的原像;selftest 逐帧核验该等式)。
fn expected_raster_texels(batch: &ShadowViewBatch, pool: &[f32]) -> Result<Vec<f32>, String> {
    let mut out = Vec::with_capacity(batch.pages.len() * PAGE_TEXELS);
    for p in &batch.pages {
        let o = usize::from(p.phys) * PAGE_TEXELS;
        if o + PAGE_TEXELS > pool.len() {
            return Err(format!(
                "F{} phys {} 超出池快照({} f32)",
                batch.frame,
                p.phys,
                pool.len()
            ));
        }
        out.extend_from_slice(&pool[o..o + PAGE_TEXELS]);
    }
    Ok(out)
}

/// 逐帧 `vsm_depth_raster_mv` 输入装配:三角形(方向光世界 tris 经该帧灯基
/// 变换 ++ local 灯空间 tris)+ 页描述(origin/pw/z 区间)+ 页 meta
/// (tri_off/tri_count;local 页取 local 段)。
fn frame_raster_inputs(
    snap: &FrameDeviceSnapshot,
    batch: &ShadowViewBatch,
) -> (Vec<f32>, Vec<f32>, Vec<u32>) {
    // 快照 right/up/fwd = 该帧 raster 时刻灯基(F5 灯转在帧首,帧内不变)。
    let basis = LightBasis {
        right: snap.right,
        up: snap.up,
        fwd: snap.fwd,
    };
    let dir_tri_count = batch.dir_tris.len() as u32;
    let local_tri_count = batch.local_tris_light.len() as u32;
    let mut tris = Vec::with_capacity(((dir_tri_count + local_tri_count) as usize) * 9);
    for t in &batch.dir_tris {
        for v in t.v {
            tris.extend_from_slice(&basis.to_light(v));
        }
    }
    for t in &batch.local_tris_light {
        for v in t {
            tris.extend_from_slice(v);
        }
    }
    let mut pages = Vec::with_capacity(batch.pages.len() * 5);
    let mut meta = Vec::with_capacity(batch.pages.len() * 2);
    for p in &batch.pages {
        pages.extend_from_slice(&[
            p.origin[0],
            p.origin[1],
            p.page_world,
            p.z_range[0],
            p.z_range[1],
        ]);
        if p.level == LOCAL_LEVEL_TAG {
            meta.extend_from_slice(&[dir_tri_count, local_tri_count]);
        } else {
            meta.extend_from_slice(&[0, dir_tri_count]);
        }
    }
    (tris, pages, meta)
}

// ---------------------------------------------------------------------------
// судья(判读;纯 host,selftest 证伪臂与 device 腿共用)
// ---------------------------------------------------------------------------

/// 腿⓪ 单帧判读:device 位图(全量 4096 字)vs host 镜像位图。
struct MarkJudge {
    frame: u32,
    set_bits: u32,
    word_mismatches: u32,
    slot_mismatches: u32,
    tail_dirty: u32,
    matched: bool,
}

fn judge_mark(m: &MarkFrameSnapshot, full_bits: &[u32]) -> Result<MarkJudge, String> {
    let words = m.levels as usize * MARK_WORDS_PER_LEVEL;
    if m.host_bits.len() != words {
        return Err(format!("F{} host 位图字数 {} ≠ {words}", m.frame, m.host_bits.len()));
    }
    if full_bits.len() < MARK_BITS_WORDS {
        return Err(format!("F{} 位图 readback 字数 {} 不足", m.frame, full_bits.len()));
    }
    let bits = &full_bits[..words];
    let tail_dirty = full_bits[words..].iter().filter(|w| **w != 0).count() as u32;
    let set_bits: u32 = bits.iter().map(|w| w.count_ones()).sum();
    let word_mismatches = bits
        .iter()
        .zip(m.host_bits.iter())
        .filter(|(a, b)| a != b)
        .count() as u32;
    let dev_slots = Vsm::marked_slots_from_bitmap(bits, m.levels as u8);
    let slot_mismatches = dev_slots.len().abs_diff(m.host_slots.len()) as u32
        + dev_slots
            .iter()
            .zip(m.host_slots.iter())
            .filter(|(a, b)| a != b)
            .count() as u32;
    Ok(MarkJudge {
        frame: m.frame,
        matched: word_mismatches == 0 && slot_mismatches == 0 && tail_dirty == 0 && set_bits > 0,
        set_bits,
        word_mismatches,
        slot_mismatches,
        tail_dirty,
    })
}

/// 腿① 单帧判读:device 重光栅纹素 vs 期望(金标准池切片)。
struct RasterJudge {
    frame: u32,
    pages: u32,
    texels: u32,
    bitexact: u32,
    max_abs: f32,
    /// sha256(device 纹素位型) == golden `dirty_depth`(严格臂,如实登记)。
    digest_match: bool,
    /// 硬判据:max_abs ≤ 1e-6(G7.5 冻结)且批次非空。
    matched: bool,
}

fn judge_raster(
    frame: u32,
    pages: u32,
    device: &[f32],
    expected: &[f32],
    golden_dirty_depth: &str,
) -> Result<RasterJudge, String> {
    if device.len() != expected.len() {
        return Err(format!(
            "F{frame} 纹素数 device {} ≠ 期望 {}",
            device.len(),
            expected.len()
        ));
    }
    let mut bitexact = 0u32;
    let mut max_abs = 0.0f32;
    for (&a, &b) in device.iter().zip(expected.iter()) {
        if a.to_bits() == b.to_bits() {
            bitexact += 1;
        }
        max_abs = max_abs.max((a - b).abs());
    }
    Ok(RasterJudge {
        frame,
        pages,
        texels: device.len() as u32,
        bitexact,
        max_abs,
        digest_match: sha_f32bits(device) == golden_dirty_depth,
        matched: !device.is_empty() && max_abs <= TOL_DEPTH,
    })
}

/// 腿② 单帧判读:device 采样值(方向光 + local)vs host oracle + golden digest。
struct SampleJudge {
    frame: u32,
    count: u32,
    mismatches: u32,
    shadowed: u32,
    has_local: bool,
    digest_match: bool,
    matched: bool,
}

fn judge_samples(
    snap: &FrameDeviceSnapshot,
    dir_values: &[f32],
    local_values: Option<&[f32]>,
    golden_sample: &str,
) -> Result<SampleJudge, String> {
    if dir_values.len() != snap.host_dir_values.len() {
        return Err(format!("F{} 方向光采样数不符", snap.frame));
    }
    let mut mismatches = dir_values
        .iter()
        .zip(snap.host_dir_values.iter())
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count() as u32;
    let mut all = dir_values.to_vec();
    match (snap.local.as_ref(), local_values) {
        (Some(loc), Some(lv)) => {
            if lv.len() != loc.host_values.len() {
                return Err(format!("F{} local 采样数不符", snap.frame));
            }
            mismatches += lv
                .iter()
                .zip(loc.host_values.iter())
                .filter(|(a, b)| a.to_bits() != b.to_bits())
                .count() as u32;
            all.extend_from_slice(lv);
        }
        (None, None) => {}
        _ => return Err(format!("F{} local 臂在位性不符", snap.frame)),
    }
    let shadowed = all.iter().filter(|v| **v == 0.0).count() as u32;
    let digest_match = sha_f32bits(&all) == golden_sample;
    Ok(SampleJudge {
        frame: snap.frame,
        count: all.len() as u32,
        mismatches,
        shadowed,
        has_local: snap.local.is_some(),
        digest_match,
        matched: mismatches == 0 && digest_match && !all.is_empty(),
    })
}

/// F12→F13 深度因果结构证据(A2.1 同口径):两帧除深度外全部 mark 输入逐字
/// 相同(相机 F4 起不动、灯基 F5 起不变),位图却不同 ⇒ 位图只能是深度反
/// 投影产物。`bits_a/bits_b` 由调用方给(host 镜像或 device readback)。
fn depth_is_causal(host: &M19RunResult, bits_a: &[u32], bits_b: &[u32]) -> bool {
    if host.device_frames.len() <= 13 {
        return false;
    }
    let (a, b) = (&host.device_frames[12].mark, &host.device_frames[13].mark);
    let same_inputs = a.inv_vp == b.inv_vp
        && a.lparams == b.lparams
        && a.cam == b.cam
        && a.right == b.right
        && a.up == b.up
        && a.fwd == b.fwd
        && a.base_radius == b.base_radius
        && a.levels == b.levels;
    same_inputs && a.depth != b.depth && bits_a != bits_b
}

// ---------------------------------------------------------------------------
// --selftest:金标准自跑 + судья自证(纯 CPU)
// ---------------------------------------------------------------------------

fn check(fails: &mut Vec<String>, name: &str, ok: bool) {
    if !ok {
        fails.push(name.to_owned());
    }
}

fn run_selftest() -> (String, bool) {
    let r = run_m19_fixture();
    let mut fails: Vec<String> = Vec::new();

    // ① 金标准脚本判据位(冻结 fixture 的设计谓词)。
    let c = &r.checks;
    check(&mut fails, "cross_frame_cache_hit", c.cross_frame_cache_hit);
    check(
        &mut fails,
        "invalidation_reasons_exhaustive",
        c.invalidation_reasons_exhaustive,
    );
    check(&mut fails, "clipmap_scroll_hit", c.clipmap_scroll_hit);
    check(&mut fails, "local_light_page_hit", c.local_light_page_hit);
    check(&mut fails, "non_virtual_caster_hit", c.non_virtual_caster_hit);
    check(&mut fails, "multi_view_batch", c.multi_view_batch);
    check(&mut fails, "evictions_present", r.evict_count > 0);
    check(
        &mut fails,
        "frames_16",
        r.device_frames.len() == 16 && r.digests.len() == 16 && r.batches.len() == 16,
    );

    // ② 逐帧原像重建:судья吃的期望值与 golden digest 同源(四轴)。
    let empty_sha = sha256_hex(&[]);
    let mut preimage_ok = 0u32;
    for ((snap, dig), batch) in r.device_frames.iter().zip(r.digests.iter()).zip(r.batches.iter()) {
        let mut pt = Vec::with_capacity(snap.entries.len() * 4);
        for e in &snap.entries {
            pt.extend_from_slice(&e.to_le_bytes());
        }
        let pt_ok = sha256_hex(&pt) == dig.page_table;
        let pool_ok = sha_f32bits(&snap.pool) == dig.depth_pool;
        let mut vals = snap.host_dir_values.clone();
        if let Some(loc) = &snap.local {
            vals.extend_from_slice(&loc.host_values);
        }
        let sample_ok = sha_f32bits(&vals) == dig.sample && vals == dig.sample_values;
        let dirty_ok = match expected_raster_texels(batch, &snap.pool) {
            Ok(tex) if tex.is_empty() => dig.dirty_depth == empty_sha,
            Ok(tex) => sha_f32bits(&tex) == dig.dirty_depth,
            Err(e) => {
                fails.push(format!("dirty_preimage_F{}:{e}", snap.frame));
                false
            }
        };
        if pt_ok && pool_ok && sample_ok && dirty_ok {
            preimage_ok += 1;
        } else {
            fails.push(format!(
                "preimage_F{}(pt={pt_ok},pool={pool_ok},sample={sample_ok},dirty={dirty_ok})",
                snap.frame
            ));
        }
    }
    check(&mut fails, "preimage_all_frames", preimage_ok == 16);

    // ③ 绿臂:судья吃 host 镜像必须全绿(mark/raster/sample 三腿逐帧)。
    let mut green_mark = 0u32;
    let mut green_raster = 0u32;
    let mut green_sample = 0u32;
    let mut raster_frames_nonempty = 0u32;
    for ((snap, dig), batch) in r.device_frames.iter().zip(r.digests.iter()).zip(r.batches.iter()) {
        let mut full = snap.mark.host_bits.clone();
        full.resize(MARK_BITS_WORDS, 0);
        match judge_mark(&snap.mark, &full) {
            Ok(j) if j.matched => green_mark += 1,
            Ok(j) => fails.push(format!("green_mark_F{}(word={},slot={})", j.frame, j.word_mismatches, j.slot_mismatches)),
            Err(e) => fails.push(format!("green_mark_F{}:{e}", snap.frame)),
        }
        match expected_raster_texels(batch, &snap.pool) {
            Ok(tex) if tex.is_empty() => green_raster += 1, // 空批帧:golden 空串已在②核验
            Ok(tex) => {
                raster_frames_nonempty += 1;
                match judge_raster(snap.frame, batch.pages.len() as u32, &tex, &tex, &dig.dirty_depth) {
                    Ok(j) if j.matched && j.digest_match => green_raster += 1,
                    Ok(j) => fails.push(format!("green_raster_F{}(max_abs={:e},digest={})", j.frame, j.max_abs, j.digest_match)),
                    Err(e) => fails.push(format!("green_raster_F{}:{e}", snap.frame)),
                }
            }
            Err(e) => fails.push(format!("green_raster_F{}:{e}", snap.frame)),
        }
        let local_vals = snap.local.as_ref().map(|l| l.host_values.as_slice());
        match judge_samples(snap, &snap.host_dir_values, local_vals, &dig.sample) {
            Ok(j) if j.matched => green_sample += 1,
            Ok(j) => fails.push(format!("green_sample_F{}(mism={},digest={})", j.frame, j.mismatches, j.digest_match)),
            Err(e) => fails.push(format!("green_sample_F{}:{e}", snap.frame)),
        }
    }
    check(&mut fails, "green_mark_16", green_mark == 16);
    check(&mut fails, "green_raster_16", green_raster == 16);
    check(&mut fails, "green_sample_16", green_sample == 16);
    check(
        &mut fails,
        "raster_nonempty_frames_present",
        raster_frames_nonempty > 0,
    );

    // 结构证据(host 镜像位图上):位图逐帧非常量 + F12→F13 深度因果。
    let mut distinct: Vec<&Vec<u32>> = Vec::new();
    for s in &r.device_frames {
        if !distinct.iter().any(|b| **b == s.mark.host_bits) {
            distinct.push(&s.mark.host_bits);
        }
    }
    check(&mut fails, "host_distinct_bitmaps_ge2", distinct.len() >= 2);
    check(
        &mut fails,
        "host_depth_is_causal",
        depth_is_causal(
            &r,
            &r.device_frames[12].mark.host_bits,
            &r.device_frames[13].mark.host_bits,
        ),
    );
    // 非退化:采样两臂都出现(全 lit / 全 shadow 均为судья空转)。
    let (mut lit, mut shadowed) = (0usize, 0usize);
    for d in &r.digests {
        lit += d.sample_values.iter().filter(|v| **v == 1.0).count();
        shadowed += d.sample_values.iter().filter(|v| **v == 0.0).count();
    }
    check(&mut fails, "samples_both_arms", lit > 0 && shadowed > 0);

    // ④ 证伪臂:各腿судья必须能翻红(臂间独立,只动被测腿的输入)。
    let red_mark = {
        let m7 = &r.device_frames[7].mark;
        let mut full = m7.host_bits.clone();
        full.resize(MARK_BITS_WORDS, 0);
        full[0] ^= 1; // 翻一位
        matches!(judge_mark(m7, &full), Ok(j) if !j.matched)
    };
    check(&mut fails, "red_mark_flips", red_mark);
    let red_raster = {
        let mut ok = false;
        for (snap, (dig, batch)) in r
            .device_frames
            .iter()
            .zip(r.digests.iter().zip(r.batches.iter()))
        {
            let tex = expected_raster_texels(batch, &snap.pool).unwrap_or_default();
            if tex.is_empty() {
                continue;
            }
            let mut tampered = tex.clone();
            tampered[0] += RED_TEXEL_EPS;
            ok = matches!(
                judge_raster(snap.frame, batch.pages.len() as u32, &tampered, &tex, &dig.dirty_depth),
                Ok(j) if !j.matched && !j.digest_match
            );
            break;
        }
        ok
    };
    check(&mut fails, "red_raster_flips", red_raster);
    let red_sample = {
        let snap = &r.device_frames[0];
        let mut vals = snap.host_dir_values.clone();
        vals[0] = 1.0 - vals[0]; // 0/1 翻转
        matches!(
            judge_samples(snap, &vals, None, &r.digests[0].sample),
            Ok(j) if !j.matched
        )
    };
    check(&mut fails, "red_sample_flips", red_sample);

    let pass = fails.is_empty();
    let mut js = String::new();
    let _ = write!(
        js,
        "{{\"subject\":\"g31_vsm_device_probe_selftest\",\
         \"frames\":16,\
         \"host_events_sha256\":\"{}\",\
         \"evict_count\":{},\
         \"preimage_frames_ok\":{},\
         \"green_mark\":{},\"green_raster\":{},\"green_sample\":{},\
         \"raster_frames_nonempty\":{},\
         \"host_distinct_bitmaps\":{},\
         \"red_mark_flips\":{},\"red_raster_flips\":{},\"red_sample_flips\":{},\
         \"fails\":[{}],\
         \"selftest_pass\":{}}}",
        r.events_sha256,
        r.evict_count,
        preimage_ok,
        green_mark,
        green_raster,
        green_sample,
        raster_frames_nonempty,
        distinct.len(),
        red_mark,
        red_raster,
        red_sample,
        fails
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(","),
        pass
    );
    (js, pass)
}

// ---------------------------------------------------------------------------
// device 腿(真 GPU;судья同上)
// ---------------------------------------------------------------------------

fn storage<'a>(size: usize, data: Option<&'a [u8]>) -> ResourceDesc<'a> {
    ResourceDesc::Buffer(BufferDesc {
        size: size as u64,
        usage: BufferUsage {
            storage: true,
            ..Default::default()
        },
        data,
        device_local: false,
    })
}

fn gate() -> Option<render_exec::DeviceCaps> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("{TAG} SKIP: vulkan loader 不可用");
        return None;
    }
    let caps = render_exec::probe_device_caps().ok()?;
    match render_exec::require_wave(&caps, KernelWave::W1) {
        Ok(()) => Some(caps),
        Err(e) => {
            eprintln!("{TAG} SKIP: W1 能力链缺失: {e}");
            None
        }
    }
}

fn dispatch_compute(
    name: &'static str,
    spirv: &[u8],
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

/// 一帧 `vsm_page_mark_project`(输入布局逐字段对齐 kernel 签名;uc06
/// device_m19 同形,SPV 改运行时装载)。
fn dispatch_mark(spv: &[u8], m: &MarkFrameSnapshot) -> Result<Vec<u32>, String> {
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
    let out = dispatch_compute(
        "vsm_page_mark_project",
        spv,
        &resources,
        vec![0, 1, 2, 3],
        push,
        pixel_count,
        &readbacks,
    )?;
    Ok(read_u32(&out[0]))
}

/// 一帧脏页批次 `vsm_depth_raster_mv`(uc06 dispatch_mv 同形)。
fn dispatch_raster(
    spv: &[u8],
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
    let out = dispatch_compute(
        "vsm_depth_raster_mv",
        spv,
        &resources,
        vec![0, 1, 2, 3],
        bytes_u32(&[page_count]),
        page_count * PAGE_TEXELS as u32,
        &readbacks,
    )?;
    Ok(read_f32(&out[0]))
}

/// 一帧 `vsm_sample`(方向光;device 真读页表/池,readback 采样值)。
fn dispatch_sample(spv: &[u8], snap: &FrameDeviceSnapshot) -> Result<Vec<f32>, String> {
    let levels = snap.levels as usize;
    let dir_len = levels * PAGE_TEXELS;
    if snap.entries.len() < dir_len {
        return Err(format!("F{} 页表快照长度不足", snap.frame));
    }
    let samples: Vec<f32> = snap.sample_pts.iter().flat_map(|p| *p).collect();
    let samples_b = bytes_f32(&samples);
    let lparams_b = bytes_f32(&snap.lparams);
    let entries_b = bytes_u32(&snap.entries[..dir_len]);
    let pool_b = bytes_f32(&snap.pool);
    let out_size = snap.sample_pts.len() * 4;
    let resources = [
        storage(samples_b.len(), Some(&samples_b)),
        storage(lparams_b.len(), Some(&lparams_b)),
        storage(entries_b.len(), Some(&entries_b)),
        storage(pool_b.len(), Some(&pool_b)),
        storage(out_size, None),
    ];
    let readbacks = [Readback::Buffer {
        res: 4,
        offset: 0,
        size: out_size as u64,
    }];
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
    let out = dispatch_compute(
        "vsm_sample",
        spv,
        &resources,
        vec![0, 1, 2, 3, 4],
        push,
        snap.sample_pts.len() as u32,
        &readbacks,
    )?;
    Ok(read_f32(&out[0]))
}

/// 一帧 `vsm_sample_local`(F12 起 spot 臂)。
fn dispatch_sample_local(spv: &[u8], snap: &FrameDeviceSnapshot) -> Result<Vec<f32>, String> {
    let loc = snap.local.as_ref().ok_or("local 臂不在位")?;
    let samples: Vec<f32> = loc.query_pts.iter().flat_map(|p| *p).collect();
    let samples_b = bytes_f32(&samples);
    let entries_b = bytes_u32(&loc.entries);
    let pool_b = bytes_f32(&snap.pool);
    let out_size = loc.query_pts.len() * 4;
    let resources = [
        storage(samples_b.len(), Some(&samples_b)),
        storage(entries_b.len(), Some(&entries_b)),
        storage(pool_b.len(), Some(&pool_b)),
        storage(out_size, None),
    ];
    let readbacks = [Readback::Buffer {
        res: 3,
        offset: 0,
        size: out_size as u64,
    }];
    let mut push = bytes_u32(&[loc.query_pts.len() as u32, snap.pool_pages]);
    for v in [loc.page_world, loc.z_range[0], loc.z_range[1], snap.depth_bias] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    let out = dispatch_compute(
        "vsm_sample_local",
        spv,
        &resources,
        vec![0, 1, 2, 3],
        push,
        loc.query_pts.len() as u32,
        &readbacks,
    )?;
    Ok(read_f32(&out[0]))
}

struct SpvSet {
    mark: Vec<u8>,
    raster_mv: Vec<u8>,
    sample: Vec<u8>,
    sample_local: Vec<u8>,
}

fn load_spv_set(dir: &Path) -> SpvSet {
    let load = |stem: &str| -> Vec<u8> {
        let p = dir.join(format!("{stem}.spv"));
        let bytes = std::fs::read(&p).unwrap_or_else(|e| fail(&format!("读 {}: {e}", p.display())));
        if bytes.len() < 20 || bytes[0..4] != [0x03, 0x02, 0x23, 0x07] {
            fail(&format!("{} 不是 SPIR-V(magic 不符或过短)", p.display()));
        }
        bytes
    };
    SpvSet {
        mark: load("vsm_page_mark_project"),
        raster_mv: load("vsm_depth_raster_mv"),
        sample: load("vsm_sample"),
        sample_local: load("vsm_sample_local"),
    }
}

fn run_device(spv_dir: &Path) -> Result<(String, bool), String> {
    let spv = load_spv_set(spv_dir);
    let host = run_m19_fixture();
    let empty_sha = sha256_hex(&[]);

    // 腿⓪ mark:逐帧 dispatch + судья。
    let mut marks: Vec<(MarkJudge, Vec<u32>)> = Vec::with_capacity(16);
    for snap in &host.device_frames {
        let full = dispatch_mark(&spv.mark, &snap.mark)?;
        let words = snap.mark.levels as usize * MARK_WORDS_PER_LEVEL;
        let trimmed = full[..words.min(full.len())].to_vec();
        let j = judge_mark(&snap.mark, &full)?;
        marks.push((j, trimmed));
    }
    let mark_frames_matched = marks.iter().filter(|(j, _)| j.matched).count() as u32;
    let mark_bits_total: u32 = marks.iter().map(|(j, _)| j.set_bits).sum();
    let mark_word_mism: u32 = marks.iter().map(|(j, _)| j.word_mismatches).sum();
    let mark_slot_mism: u32 = marks.iter().map(|(j, _)| j.slot_mismatches).sum();
    let mark_tail: u32 = marks.iter().map(|(j, _)| j.tail_dirty).sum();
    let mut distinct: Vec<&Vec<u32>> = Vec::new();
    for (_, bits) in &marks {
        if !distinct.iter().any(|b| **b == *bits) {
            distinct.push(bits);
        }
    }
    let mark_causal = depth_is_causal(&host, &marks[12].1, &marks[13].1);
    let mark_all = mark_frames_matched == 16
        && mark_word_mism == 0
        && mark_slot_mism == 0
        && mark_tail == 0
        && mark_bits_total > 0
        && distinct.len() >= 2
        && mark_causal;

    // 腿① invalidate→raster:逐帧脏页批次 device 重建 vs golden dirty_depth。
    let mut rasters: Vec<RasterJudge> = Vec::new();
    let mut empty_golden_ok = true;
    let mut raster_dispatches = 0u32;
    for (snap, (dig, batch)) in host
        .device_frames
        .iter()
        .zip(host.digests.iter().zip(host.batches.iter()))
    {
        if batch.pages.is_empty() {
            // 空批帧(跨帧 cache hit 的直接证据):golden 必须 = 空串 sha。
            empty_golden_ok &= dig.dirty_depth == empty_sha;
            continue;
        }
        let expected = expected_raster_texels(batch, &snap.pool)?;
        let (tris, pages, meta) = frame_raster_inputs(snap, batch);
        let page_count = batch.pages.len() as u32;
        let device = dispatch_raster(&spv.raster_mv, &tris, &pages, &meta, page_count)?;
        raster_dispatches += 1;
        rasters.push(judge_raster(
            snap.frame,
            page_count,
            &device,
            &expected,
            &dig.dirty_depth,
        )?);
    }
    let raster_frames_matched = rasters.iter().filter(|j| j.matched).count() as u32;
    let raster_digest_matched = rasters.iter().filter(|j| j.digest_match).count() as u32;
    let raster_pages_total: u32 = rasters.iter().map(|j| j.pages).sum();
    let raster_texels_total: u32 = rasters.iter().map(|j| j.texels).sum();
    let raster_bitexact: u32 = rasters.iter().map(|j| j.bitexact).sum();
    let raster_max_abs = rasters.iter().map(|j| j.max_abs).fold(0.0f32, f32::max);
    let raster_all = !rasters.is_empty()
        && raster_frames_matched == rasters.len() as u32
        && empty_golden_ok
        && raster_pages_total > 0;

    // 腿② alloc→sample:逐帧采样(+F12 起 local)。
    let mut samples: Vec<SampleJudge> = Vec::with_capacity(16);
    let mut sample_dispatches = 0u32;
    for (snap, dig) in host.device_frames.iter().zip(host.digests.iter()) {
        let dir_values = dispatch_sample(&spv.sample, snap)?;
        sample_dispatches += 1;
        let local_values = if snap.local.is_some() {
            sample_dispatches += 1;
            Some(dispatch_sample_local(&spv.sample_local, snap)?)
        } else {
            None
        };
        samples.push(judge_samples(
            snap,
            &dir_values,
            local_values.as_deref(),
            &dig.sample,
        )?);
    }
    let sample_frames_matched = samples.iter().filter(|j| j.matched).count() as u32;
    let sample_mism: u32 = samples.iter().map(|j| j.mismatches).sum();
    let sample_total: u32 = samples.iter().map(|j| j.count).sum();
    let sample_shadowed: u32 = samples.iter().map(|j| j.shadowed).sum();
    let frames_with_local = samples.iter().filter(|j| j.has_local).count() as u32;
    let sample_all = sample_frames_matched == 16 && sample_mism == 0 && sample_shadowed > 0;

    let validation_errors = render_exec::validation_error_total();
    let validation_messenger = render_exec::validation_messenger_installed();
    let pass = mark_all && raster_all && sample_all;

    let mark_json = marks
        .iter()
        .map(|(j, _)| {
            format!(
                "{{\"frame\":{},\"set_bits\":{},\"word_mism\":{},\"slot_mism\":{},\"tail_dirty\":{},\"match\":{}}}",
                j.frame, j.set_bits, j.word_mismatches, j.slot_mismatches, j.tail_dirty, j.matched
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let raster_json = rasters
        .iter()
        .map(|j| {
            format!(
                "{{\"frame\":{},\"pages\":{},\"texels\":{},\"bitexact\":{},\"max_abs\":{:.9e},\"digest_match\":{},\"match\":{}}}",
                j.frame, j.pages, j.texels, j.bitexact, j.max_abs, j.digest_match, j.matched
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let sample_json = samples
        .iter()
        .map(|j| {
            format!(
                "{{\"frame\":{},\"count\":{},\"mism\":{},\"shadowed\":{},\"has_local\":{},\"digest_match\":{},\"match\":{}}}",
                j.frame, j.count, j.mismatches, j.shadowed, j.has_local, j.digest_match, j.matched
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let mut js = String::new();
    let _ = write!(
        js,
        "{{\"subject\":\"g31_vsm_device_probe\",\
         \"device_state\":\"executed\",\
         \"frames\":16,\
         \"host_events_sha256\":\"{}\",\
         \"mark_kernel\":\"vsm_page_mark_project\",\
         \"mark_dispatches\":16,\
         \"mark_frames_matched\":{mark_frames_matched},\
         \"mark_bits_total\":{mark_bits_total},\
         \"mark_word_mismatches\":{mark_word_mism},\
         \"mark_slot_mismatches\":{mark_slot_mism},\
         \"mark_tail_dirty\":{mark_tail},\
         \"mark_distinct_bitmaps\":{},\
         \"mark_depth_is_causal\":{mark_causal},\
         \"mark_all_match\":{mark_all},\
         \"raster_kernel\":\"vsm_depth_raster_mv\",\
         \"raster_dispatches\":{raster_dispatches},\
         \"raster_frames_nonempty\":{},\
         \"raster_frames_matched\":{raster_frames_matched},\
         \"raster_digest_frames_matched\":{raster_digest_matched},\
         \"raster_pages_total\":{raster_pages_total},\
         \"raster_texels_total\":{raster_texels_total},\
         \"raster_bitexact_texels\":{raster_bitexact},\
         \"raster_measured_max_abs\":{:.9e},\
         \"raster_tol\":{:.9e},\
         \"raster_empty_frames_golden_ok\":{empty_golden_ok},\
         \"raster_all_match\":{raster_all},\
         \"sample_kernels\":\"vsm_sample+vsm_sample_local\",\
         \"sample_dispatches\":{sample_dispatches},\
         \"sample_frames_matched\":{sample_frames_matched},\
         \"sample_value_mismatches\":{sample_mism},\
         \"sample_values_total\":{sample_total},\
         \"sample_values_shadowed\":{sample_shadowed},\
         \"frames_with_local\":{frames_with_local},\
         \"sample_all_match\":{sample_all},\
         \"validation_errors\":{validation_errors},\
         \"validation_messenger\":{validation_messenger},\
         \"judge\":\"host(selftest-falsified)\",\
         \"mark_frames\":[{mark_json}],\
         \"raster_frames\":[{raster_json}],\
         \"sample_frames\":[{sample_json}],\
         \"pass\":{pass}}}",
        host.events_sha256,
        distinct.len(),
        rasters.len(),
        raster_max_abs,
        TOL_DEPTH,
    );
    Ok((js, pass))
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn emit(js: &str, out: Option<&Path>) {
    println!("{js}");
    if let Some(p) = out {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(p, format!("{js}\n")).unwrap_or_else(|e| fail(&format!("写 {}: {e}", p.display())));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut selftest = false;
    let mut spv_dir: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--selftest" => selftest = true,
            "--spv-dir" => {
                i += 1;
                spv_dir = Some(PathBuf::from(args.get(i).unwrap_or_else(|| fail("--spv-dir 缺路径"))));
            }
            "--out" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).unwrap_or_else(|| fail("--out 缺路径"))));
            }
            other => fail(&format!("未知参数 {other}(用法:--selftest | --spv-dir <dir> [--out <json>])")),
        }
        i += 1;
    }

    if selftest {
        let (js, pass) = run_selftest();
        emit(&js, out.as_deref());
        std::process::exit(if pass { 0 } else { 1 });
    }

    let Some(dir) = spv_dir else {
        fail("需 --selftest 或 --spv-dir <dir>");
    };
    if gate().is_none() {
        let js = "{\"subject\":\"g31_vsm_device_probe\",\"device_state\":\"skipped_dev_env\",\"pass\":false}";
        emit(js, out.as_deref());
        let require_real = std::env::var("RURIX_REQUIRE_REAL").map(|v| v == "1").unwrap_or(false);
        std::process::exit(if require_real { 1 } else { 0 });
    }
    match run_device(&dir) {
        Ok((js, pass)) => {
            emit(&js, out.as_deref());
            std::process::exit(if pass { 0 } else { 1 });
        }
        Err(e) => {
            let js = format!(
                "{{\"subject\":\"g31_vsm_device_probe\",\"device_state\":\"error\",\"error\":\"{}\",\"pass\":false}}",
                e.replace('"', "'")
            );
            emit(&js, out.as_deref());
            std::process::exit(1);
        }
    }
}
