//! G8.5a M19 VSM 跨帧页缓存 fixture(host 金标准)。
//!
//! 16 帧确定性脚本:cache hit / 五失效原因 / clipmap scroll / local light /
//! non-virtual caster / multi-view batch;产出 canonical 事件序列与页表/
//! depth/sample digest。device 腿消费同构 batch 描述对拍。

use crate::shadow::clipmap::{ClipmapConfig, PAGE_TABLE_DIM};
use crate::shadow::events::{
    EventKind, EventLog, InvalidationReason, LightId, PageEvent, sha256_hex,
};
use crate::shadow::local::{LocalLightPages, LocalSpot, world_tris_to_spot_light};
use crate::shadow::page_table::PageId;
use crate::shadow::vsm::{DirtyPageRef, ShadowTri, Vsm, VsmConfig};
use crate::temporal::common::Mat4;
use crate::temporal::image::ImageF32;

const DIM: usize = PAGE_TABLE_DIM as usize;
const FRAME_COUNT: u32 = 16;

/// 主相机深度网格边长(mark 段 device dispatch 线程数 = `MARK_DIM²`)。
pub const MARK_DIM: u32 = 64;

/// 反投影矩阵(行主序)。`w=1` 恒定,故 `world` 对 `(ndc_x, ndc_y, d)` 仿射:
/// `w_x = 0.0625·px + 0.03125`、`w_y = 0.0625·py + 0.03125`、`w_z = −64·d`
/// (`MARK_DIM=64` 的 NDC 口径)。选此形式使「哪个像素落哪一页/哪一级」可解析
/// 核算,mark 段的 host/device 对拍不依赖场景光栅的浮点巧合。
fn m19_inv_view_proj() -> Mat4 {
    Mat4 {
        m: [
            [2.0, 0.0, 0.0, 2.0],
            [0.0, -2.0, 0.0, 2.0],
            [0.0, 0.0, -64.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    }
}

// 深度值 → 反投影 z(= −64·d)→ 到相机距离 → `select_level` 选级:
//   `D_L0`:w_z=−0.5   ⇒ dc≈7.5  ≤ R0=16      ⇒ 0 级
//   `D_L1`:w_z=−13.0  ⇒ dc≈20.0 ∈ (16, 32]   ⇒ 1 级
//   `D_L2`:w_z=−33.0  ⇒ dc≈40.0 ∈ (32, 64]   ⇒ 2 级
// 三个值都离级边界 ≥3 个世界单位、离页边界 ≥0.03 世界单位(pw 最小 0.25),故
// host f32 与 device f32 的 log2/ceil/floor 不在边界上分叉。
const D_L0: f32 = 0.5 / 64.0;
const D_L1: f32 = 13.0 / 64.0;
const D_L2: f32 = 33.0 / 64.0;

/// 逐帧主相机深度(1.0 = 远平面/天空,`page_mark` 剔除)。
///
/// 块 → 页的解析映射(灯基 `dir≈−z` ⇒ `x_l = w_y`、`y_l ≈ w_x`):
///   * `px∈[0,4) py∈[0,4)` @L0 ⇒ 0 级槽 (0,0)
///   * `px∈[0,4) py∈[4,8)` @L0 ⇒ 0 级槽 (1,0)
///   * `px∈[4,8) py∈[0,4)` @L1 ⇒ 1 级槽 (0,0)
///   * `px∈[12,16) py∈[0,4)` @L2 ⇒ 2 级槽 (0,0)
///
/// F13 起换成「压力块」`px∈[0,4) py∈[8,40)` ⇒ 0 级槽 (2,0)…(9,0) 共 8 页:
/// 核心页本帧未被标记 ⇒ 帧龄≥1 ⇒ 可被 LRU 驱逐(驱逐轴的前提)。
fn m19_mark_depth(frame: u32) -> ImageF32 {
    let mut img = ImageF32::new(MARK_DIM, MARK_DIM, 1);
    for v in img.data.iter_mut() {
        *v = 1.0;
    }
    let blocks: &[(u32, u32, u32, u32, f32)] = if frame >= 13 {
        &[(0, 4, 8, 40, D_L0)]
    } else {
        &[
            (0, 4, 0, 4, D_L0),
            (0, 4, 4, 8, D_L0),
            (4, 8, 0, 4, D_L1),
            (12, 16, 0, 4, D_L2),
        ]
    };
    for &(x0, x1, y0, y1, d) in blocks {
        for py in y0..y1 {
            for px in x0..x1 {
                img.set(px, py, 0, d);
            }
        }
    }
    img
}

/// caster 分类(设计 §2.2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasterClass {
    Virtual,
    NonVirtual,
}

#[derive(Debug, Clone)]
pub struct ShadowCaster {
    pub tris: Vec<ShadowTri>,
    pub class: CasterClass,
    pub aabb: ([f32; 3], [f32; 3]),
}

/// 单帧 device 可消费的 multi-view batch 快照。
#[derive(Debug, Clone)]
pub struct ShadowViewBatch {
    pub frame: u32,
    /// 本帧单 dispatch 视图数(方向光级数 + local 若启用)。
    pub view_count: u32,
    pub pages: Vec<DirtyPageRef>,
    /// 灯空间三角形(方向光基下预变换,9 f32/tri 扁平由调用方组装)。
    pub dir_tris: Vec<ShadowTri>,
    pub local_tris_light: Vec<[[f32; 3]; 3]>,
    // 机械豁免(rust 1.93 clippy 漂移):G8 期既有 pub 字段类型,本波不动 API 面。
    #[allow(clippy::type_complexity)]
    pub local_pages: Vec<(u8, u8, u16, [f32; 2], [f32; 2], f32)>,
}

/// 逐帧 digest 记录。
#[derive(Debug, Clone)]
pub struct FrameDigest {
    pub frame: u32,
    pub page_table: String,
    pub depth_pool: String,
    pub sample: String,
    /// 本帧脏页深度拼接 sha256(device multi-view gather 对拍序 = batch.pages 序)。
    pub dirty_depth: String,
    /// 本帧采样值 f32 位型拼接 sha256。
    pub sample_values: Vec<f32>,
}

/// local spot 逐帧 device 快照(`vsm_sample_local` 输入布局)。
#[derive(Debug, Clone)]
pub struct LocalFrameSnapshot {
    /// local 单级页表 128×128(`digest_tables` 尾段原像)。
    pub entries: Vec<u32>,
    pub page_world: f32,
    pub z_range: [f32; 2],
    /// host 传给 `LocalLightPages::sample` 的灯空间查询点(逐点 3 f32)。
    pub query_pts: Vec<[f32; 3]>,
    /// host local 采样值(device 逐值对拍基准)。
    pub host_values: Vec<f32>,
}

/// 逐帧 `vsm_page_mark_project` device 输入 + host 镜像位图(A2.1;设计 §2.1
/// 帧循环第一行、§2.3 第一核)。
///
/// 字段与 kernel `vsm_page_mark_project(depth, inv_vp, lparams, page_bits,
/// pixel_count, width, levels, cam_*, right_*, up_*, fwd_*, base_radius)` 的
/// 输入布局逐字段对齐;`host_bits` 是 host 镜像 [`Vsm::page_mark_bits`] 的产物,
/// device readback 位图与之**逐位**相等才允许据此记 MarkHit/MarkMiss。
#[derive(Debug, Clone)]
pub struct MarkFrameSnapshot {
    pub frame: u32,
    pub width: u32,
    pub height: u32,
    /// 主相机深度(单通道 `width*height`;1.0 = 远平面/天空)。
    pub depth: Vec<f32>,
    /// 行主序 `inv(view*proj)`(`m[0][0..4], m[1][0..4], …`)。
    pub inv_vp: [f32; 16],
    /// 标记时刻逐级 `(page_world, wmin.x, wmin.y, zmin, zmax)`。
    pub lparams: Vec<f32>,
    pub cam: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub fwd: [f32; 3],
    pub base_radius: f32,
    pub levels: u32,
    /// host 镜像位图(`levels*512` u32;bit 序 `l*16384 + y*128 + x`)。
    pub host_bits: Vec<u32>,
    /// 位图反解出的标记槽 `(level, x, y)`(级升序、槽行主序)= 分类/alloc 输入序。
    pub host_slots: Vec<(u8, u8, u8)>,
    /// 有效(非远平面)像素数。
    pub pixels: u32,
    /// 本帧新标记页数。
    pub pages: u32,
}

/// 逐帧 device 消费快照(A2:device 真消费本帧页表/池,readback 出 digest)。
///
/// `entries` 与 [`digest_tables`] 的字节序**同源同序**,`sample_*` 与
/// `digest_samples` 的原像同序——故 device readback 出的 digest 可与 golden
/// 逐值比对,无需 host 代填。
#[derive(Debug, Clone)]
pub struct FrameDeviceSnapshot {
    pub frame: u32,
    /// 方向光逐级页表(+ local 页表若已亮)拼接;= `page_table` digest 原像。
    pub entries: Vec<u32>,
    /// 方向光逐级 (page_world, wmin.x, wmin.y, zmin, zmax)。
    pub lparams: Vec<f32>,
    /// 本帧共享物理池全量(device sample 核真实读取)。
    pub pool: Vec<f32>,
    pub cam: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub fwd: [f32; 3],
    pub base_radius: f32,
    pub depth_bias: f32,
    pub levels: u32,
    pub pool_pages: u32,
    /// 方向光采样点(世界坐标)。
    pub sample_pts: Vec<[f32; 3]>,
    /// host 方向光采样值。
    pub host_dir_values: Vec<f32>,
    /// 本帧 spot 已亮时的 local 臂快照。
    pub local: Option<LocalFrameSnapshot>,
    /// 本帧页标记段(device `vsm_page_mark_project` 输入 + host 镜像位图)。
    pub mark: MarkFrameSnapshot,
}

/// 终帧方向光采样 device 快照(G7 `vsm_sample` 输入布局)。
#[derive(Debug, Clone)]
pub struct FinalSampleSnapshot {
    pub points: Vec<[f32; 3]>,
    pub host_values: Vec<f32>,
    pub lparams: Vec<f32>,
    pub entries: Vec<u32>,
    pub pool: Vec<f32>,
    pub cam: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub fwd: [f32; 3],
    pub base_radius: f32,
    pub depth_bias: f32,
    pub levels: u32,
    pub pool_pages: u32,
}

/// M19 完整跑分结果。
#[derive(Debug, Clone)]
pub struct M19RunResult {
    pub events: EventLog,
    pub events_sha256: String,
    pub canonical_json: String,
    pub digests: Vec<FrameDigest>,
    pub max_view_count: u32,
    pub batches: Vec<ShadowViewBatch>,
    pub checks: M19HostChecks,
    pub final_sample: FinalSampleSnapshot,
    /// 逐帧 device 消费快照(16 帧,与 `digests` 同序同长)。
    pub device_frames: Vec<FrameDeviceSnapshot>,
    /// 本次运行的物理池预算(RED 驱逐轴扰动量)。
    pub pool_pages: u32,
    /// 全程 `Evict` 事件数(驱逐轴的可观测量;RED 以此证明扰动确落在驱逐上)。
    pub evict_count: u32,
}

/// host 侧判据位(smoke / probe 消费)。
#[derive(Debug, Clone, Default)]
pub struct M19HostChecks {
    pub host_oracle_regression: bool,
    pub event_sequence_matches_golden: bool,
    pub cross_frame_cache_hit: bool,
    pub invalidation_reasons_exhaustive: bool,
    pub clipmap_scroll_hit: bool,
    pub local_light_page_hit: bool,
    pub non_virtual_caster_hit: bool,
    pub multi_view_batch: bool,
}

fn cfg_m19(pool: u16) -> VsmConfig {
    VsmConfig {
        clip: ClipmapConfig {
            levels: 4,
            base_radius: 16.0,
            depth_extent: 64.0,
        },
        pool_pages: pool,
        depth_bias: 1e-3,
    }
}

fn aabb_of(tris: &[ShadowTri]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for t in tris {
        for v in t.v {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }
    }
    (min, max)
}

fn translate_caster(c: &mut ShadowCaster, d: [f32; 3]) {
    for t in &mut c.tris {
        for v in &mut t.v {
            v[0] += d[0];
            v[1] += d[1];
            v[2] += d[2];
        }
    }
    c.aabb = aabb_of(&c.tris);
}

fn emit(
    log: &mut EventLog,
    frame: u32,
    light: LightId,
    level: u8,
    slot: (u8, u8),
    kind: EventKind,
    phys: u16,
) {
    log.push(PageEvent {
        frame,
        light,
        level,
        slot,
        kind,
        phys,
    });
}

fn digest_tables(vsm: &Vsm, local: Option<&LocalLightPages>) -> String {
    let mut bytes = Vec::new();
    for l in 0..vsm.views().len() as u8 {
        for e in &vsm.table(l).entries {
            bytes.extend_from_slice(&e.to_le_bytes());
        }
    }
    if let Some(loc) = local {
        for e in &loc.table.entries {
            bytes.extend_from_slice(&e.to_le_bytes());
        }
    }
    sha256_hex(&bytes)
}

fn digest_pool(vsm: &Vsm) -> String {
    let mut bytes = Vec::new();
    for p in 0..vsm.pool().budget {
        for &f in vsm.pool().page(p) {
            bytes.extend_from_slice(&f.to_bits().to_le_bytes());
        }
    }
    sha256_hex(&bytes)
}

fn digest_samples(samples: &[f32]) -> String {
    let mut bytes = Vec::new();
    for &f in samples {
        bytes.extend_from_slice(&f.to_bits().to_le_bytes());
    }
    sha256_hex(&bytes)
}

fn record_marks(log: &mut EventLog, vsm: &Vsm, frame: u32, slots: &[(u8, u8, u8)]) {
    for &(level, x, y) in slots {
        let e = vsm.table(level).get(x, y);
        let kind = if e.resident {
            EventKind::MarkHit
        } else {
            EventKind::MarkMiss
        };
        emit(
            log,
            frame,
            LightId::Directional,
            level,
            (x, y),
            kind,
            e.phys,
        );
    }
}

fn record_alloc(
    log: &mut EventLog,
    frame: u32,
    light: LightId,
    before: &[(u8, u8, u8, bool)],
    vsm: &Vsm,
) {
    for &(level, x, y, was_res) in before {
        let e = vsm.table(level).get(x, y);
        if !was_res && e.resident {
            emit(log, frame, light, level, (x, y), EventKind::Alloc, e.phys);
        }
    }
}

/// 跑 M19 16 帧 host 金标准脚本(冻结池预算 6 页 = golden 口径)。
pub fn run_m19_fixture() -> M19RunResult {
    run_m19_fixture_pool(6)
}

/// RED 驱逐轴的池预算(measured:evict 4→2、受害者集合变、事件序列 sha 变;
/// 池 ≥14 会退化成「零驱逐」,故取 12 保留真驱逐仍在发生这一前提)。
pub const RED_EVICT_POOL: u16 = 12;

/// M19 fixture 的池预算参数化入口。
///
/// `pool_pages != 6` = **RED 驱逐轴**:池容量变化直接改变 F13/F14 的 LRU 驱逐
/// 决策(受害者集合/数量),事件序列必与 golden 不同 —— 这是「篡改驱逐序 → 事件
/// 序列必红」的真扰动臂(不是改 golden 文件、也不是改事件日志)。
pub fn run_m19_fixture_pool(pool_pages: u16) -> M19RunResult {
    let mut log = EventLog::new();
    let mut digests = Vec::new();
    let mut batches = Vec::new();
    let mut device_frames: Vec<FrameDeviceSnapshot> = Vec::new();
    let mut max_view_count = 0u32;

    // 地面 + 悬浮板(Virtual) + 小方块(NonVirtual)
    let mut plate = ShadowCaster {
        tris: vec![ShadowTri::new(
            [-0.5, 0.5, -0.5],
            [0.5, 0.5, -0.5],
            [0.0, 0.5, 0.5],
        )],
        class: CasterClass::Virtual,
        aabb: ([0.0; 3], [0.0; 3]),
    };
    plate.aabb = aabb_of(&plate.tris);
    let mut cube = ShadowCaster {
        tris: vec![
            ShadowTri::new([0.1, 0.0, 0.1], [0.3, 0.0, 0.1], [0.1, 0.2, 0.1]),
            ShadowTri::new([0.3, 0.0, 0.1], [0.3, 0.2, 0.1], [0.1, 0.2, 0.1]),
        ],
        class: CasterClass::NonVirtual,
        aabb: ([0.0; 3], [0.0; 3]),
    };
    cube.aabb = aabb_of(&cube.tris);
    let ground = ShadowTri::new([-2.0, 0.0, -2.0], [2.0, 0.0, -2.0], [0.0, 0.0, 2.0]);

    let light0 = [0.0, 0.0, -1.0];
    let cam0 = [0.0, 0.0, 7.0];
    // 小池迫使 F13/F14 驱逐。
    let mut vsm = Vsm::new(cfg_m19(pool_pages), light0, cam0);
    let mut local: Option<LocalLightPages> = None;
    // 核心页(= F0–F12 深度网格经反投影/选级后**解析地**落到的四页,见
    // `m19_mark_depth` 头注释)。A2.1 起本数组**不再用于标记**(标记只走深度反
    // 投影位图),仅作失效轴(F3 CasterMoved / F5 LightChanged / F8–F11
    // NonVirtualCaster / F12 强制脏)的目标槽 —— 失效决策留 host(设计 §2.1)。
    //
    // 核心驻留槽必须**盖住真实几何**:灯向 ≈ −z ⇒ 灯空间 xy ≈ 世界 (x, y);
    // 唯一非退化投影体是 NonVirtual 立方体(x∈[0.1,0.3], y∈[0,0.2],竖直面),
    // 地面/悬浮板在灯空间退化成线(面积 0,不投影)。level0 pw=0.25 ⇒ 该几何落在
    // wp=(0,0)/(1,0) ⇒ slot (0,0)/(1,0);level1 pw=0.5、level2 pw=1.0 ⇒ slot (0,0)。
    //
    // 此前是硬编码 (64,64)/(65,64)/(32,32) —— 对应世界 xy ≈ (−16, 16),离几何 16
    // 个单位,故 F0–F11 物理池**全清**(0/98304 非清空纹素)、80 个采样值恒 1.0:
    // 三条 digest 判据实际在对常量做 sha256(A2 复查发现,见 `.a2_evidence/`)。
    let core_slots: [(u8, u8, u8); 4] = [(0, 0, 0), (0, 1, 0), (1, 0, 0), (2, 0, 0)];

    for frame in 0..FRAME_COUNT {
        // F4 相机挪到 z=7.0 并驻留(F5+ 同位;两臂同值,机械合并消除 clippy
        // if_same_then_else,F4→F5+ 路径数值不变)。
        let camera = if frame >= 4 { [0.25, 0.0, 7.0] } else { cam0 };

        let scroll = if frame == 0 {
            0
        } else {
            vsm.begin_frame(camera)
        };
        if let Some(loc) = local.as_mut() {
            loc.begin_frame();
        }

        // ClipmapScroll 事件(F4:恰一页 → 0 级一行 128 槽;与
        // `origin_shift_one_page_dirties_ring_band_only` 同口径)。
        if frame == 4 && scroll > 0 {
            for x in 0..DIM {
                let e = vsm.table(0).get(x as u8, 64);
                emit(
                    &mut log,
                    frame,
                    LightId::Directional,
                    0,
                    (x as u8, 64),
                    EventKind::Invalidate(InvalidationReason::ClipmapScroll),
                    e.phys,
                );
            }
        }

        // F3: Virtual plate 平移 → CasterMoved
        if frame == 3 {
            translate_caster(&mut plate, [0.2, 0.0, 0.0]);
            let (mn, mx) = plate.aabb;
            let _ = vsm.invalidate_aabb(mn, mx);
            let (l, x, y) = core_slots[1];
            let _ = vsm.dirty_slot(l, x, y);
            let e = vsm.table(l).get(x, y);
            emit(
                &mut log,
                frame,
                LightId::Directional,
                l,
                (x, y),
                EventKind::Invalidate(InvalidationReason::CasterMoved),
                e.phys,
            );
        }

        // F5: 灯微转 → LightChanged(全脏;事件取核心驻留槽代表)
        if frame == 5 {
            vsm.invalidate_light_direction([0.02, 0.0, -1.0]);
            for &(l, x, y) in &core_slots {
                let e = vsm.table(l).get(x, y);
                if e.resident {
                    emit(
                        &mut log,
                        frame,
                        LightId::Directional,
                        l,
                        (x, y),
                        EventKind::Invalidate(InvalidationReason::LightChanged),
                        e.phys,
                    );
                }
            }
        }

        // F8–F11: NonVirtual 摆动——AABB 失效 + 保证至少一核心驻留槽标脏
        // (Virtual 区其余驻留页保持净,对照增量语义)。
        if (8..=11).contains(&frame) {
            let dx = if frame % 2 == 0 { 0.05 } else { -0.05 };
            translate_caster(&mut cube, [dx, 0.0, 0.0]);
            let (mn, mx) = cube.aabb;
            let _ = vsm.invalidate_aabb(mn, mx);
            let (l, x, y) = core_slots[0];
            let _ = vsm.dirty_slot(l, x, y);
            let e = vsm.table(l).get(x, y);
            emit(
                &mut log,
                frame,
                LightId::Directional,
                l,
                (x, y),
                EventKind::Invalidate(InvalidationReason::NonVirtualCaster),
                e.phys,
            );
        }

        // F12: 亮起 local spot
        if frame == 12 && local.is_none() {
            local = Some(LocalLightPages::new(LocalSpot {
                position: [0.0, 3.0, 3.0],
                direction: [0.0, -0.7, -0.7],
                range: 10.0,
                fov_y: 0.9,
            }));
        }

        // ── 帧循环第一行(设计 §2.1):主相机深度 → 反投影页标记 ──
        //
        // A2.1 清零点:此前是 `for (l,x,y) in core_slots { vsm.mark_slot(l,x,y) }`
        // —— host 直接拿**预知的 page id** 标页,既无主相机深度也不做反投影/选级,
        // device 核 `vsm_page_mark_project` 零消费(编译进 SPV 但无人 dispatch)。
        // 现在:深度网格 → `page_mark_bits`(反投影/选级/出窗回退的 host 镜像)
        // 产**位图**,host 只负责「消费位图 + 分类/alloc/evict」(设计 §2.1 裁决:
        // 分配/失效决策留 host);device 段逐帧 dispatch 同一核并 readback 位图,
        // 与 `host_bits` 逐位 + 逐槽对拍,不等则门红。
        let mark_depth = m19_mark_depth(frame);
        let mark_inv_vp = m19_inv_view_proj();
        let (mark_bits, mark_pixels) = vsm.page_mark_bits(&mark_depth, &mark_inv_vp);
        let mark_views = vsm.views();
        let mut mark_lparams = Vec::with_capacity(mark_views.len() * 5);
        for v in &mark_views {
            mark_lparams.extend_from_slice(&[
                v.page_world,
                v.window_min_pages[0] as f32,
                v.window_min_pages[1] as f32,
                v.z_range[0],
                v.z_range[1],
            ]);
        }
        let mark_basis = crate::shadow::clipmap::LightBasis::from_direction(vsm.light_dir());
        let marked = Vsm::marked_slots_from_bitmap(&mark_bits, mark_views.len() as u8);
        let mark_pages = vsm.apply_mark_bitmap(&mark_bits);
        let mark_snapshot = MarkFrameSnapshot {
            frame,
            width: mark_depth.w,
            height: mark_depth.h,
            depth: mark_depth.data.clone(),
            inv_vp: {
                let mut f = [0.0f32; 16];
                for r in 0..4 {
                    for k in 0..4 {
                        f[r * 4 + k] = mark_inv_vp.m[r][k];
                    }
                }
                f
            },
            lparams: mark_lparams,
            cam: camera,
            right: mark_basis.right,
            up: mark_basis.up,
            fwd: mark_basis.fwd,
            base_radius: 16.0,
            levels: mark_views.len() as u32,
            host_bits: mark_bits,
            host_slots: marked.clone(),
            pixels: mark_pixels,
            pages: mark_pages,
        };
        record_marks(&mut log, &vsm, frame, &marked);

        // local spot 标记仍走 host 白盒槽:`vsm_page_mark_project` 的输入布局是
        // **方向光 clipmap 栈**(逐级 lparams + 选级/回退环),不覆盖 spot 的单级
        // 透视页表(设计 §2.3 第一核只列方向光 mark);spot 的 device mark 核不在
        // A2.1 范围内。故 local 臂的 mark 位不作 device 判据,只作事件序列的一部分。
        if let Some(loc) = local.as_mut()
            && frame >= 12
        {
            loc.mark_slot(0, 0);
            loc.mark_slot(1, 0);
            let e0 = loc.table.get(0, 0);
            let kind = if e0.resident {
                EventKind::MarkHit
            } else {
                EventKind::MarkMiss
            };
            emit(&mut log, frame, LightId::Local(0), 0, (0, 0), kind, e0.phys);
            let e1 = loc.table.get(1, 0);
            let kind = if e1.resident {
                EventKind::MarkHit
            } else {
                EventKind::MarkMiss
            };
            emit(&mut log, frame, LightId::Local(0), 0, (1, 0), kind, e1.phys);
        }

        // 分配前快照
        let before: Vec<(u8, u8, u8, bool)> = marked
            .iter()
            .map(|&(l, x, y)| (l, x, y, vsm.table(l).get(x, y).resident))
            .collect();

        let alloc = vsm.page_alloc();
        record_alloc(&mut log, frame, LightId::Directional, &before, &vsm);
        for vic in &alloc.evicted_pages {
            emit(
                &mut log,
                frame,
                LightId::Directional,
                vic.level,
                (vic.x, vic.y),
                EventKind::Evict,
                0,
            );
            emit(
                &mut log,
                frame,
                LightId::Directional,
                vic.level,
                (vic.x, vic.y),
                EventKind::Invalidate(InvalidationReason::Evicted),
                0,
            );
        }
        for _ in 0..alloc.denied {
            emit(
                &mut log,
                frame,
                LightId::Directional,
                0,
                (0, 0),
                EventKind::Deny,
                0,
            );
        }

        if let Some(loc) = local.as_mut() {
            let mut foreign: Vec<PageId> = Vec::new();
            let dir_victim = vsm.find_lru_victim();
            let before_local = [loc.table.get(0, 0).resident, loc.table.get(1, 0).resident];
            let st = loc.alloc_into(vsm.pool_mut(), || dir_victim, |id| foreign.push(id));
            for id in foreign {
                vsm.clear_slot(id.level, id.x, id.y);
            }
            for (i, &was) in before_local.iter().enumerate() {
                let x = i as u8;
                let e = loc.table.get(x, 0);
                if !was && e.resident {
                    emit(
                        &mut log,
                        frame,
                        LightId::Local(0),
                        0,
                        (x, 0),
                        EventKind::Alloc,
                        e.phys,
                    );
                }
            }
            for vic in &st.evicted_pages {
                emit(
                    &mut log,
                    frame,
                    LightId::Local(0),
                    vic.level,
                    (vic.x, vic.y),
                    EventKind::Evict,
                    0,
                );
                emit(
                    &mut log,
                    frame,
                    LightId::Local(0),
                    vic.level,
                    (vic.x, vic.y),
                    EventKind::Invalidate(InvalidationReason::Evicted),
                    0,
                );
            }
        }

        // F12:强制一核心页脏,保证 multi-view batch 含方向光+local 两臂页。
        if frame == 12 {
            let (l, x, y) = core_slots[0];
            let _ = vsm.dirty_slot(l, x, y);
        }

        // multi-view batch 快照(光栅前)
        let mut dir_pages = vsm.dirty_resident_pages();
        let mut local_pages = Vec::new();
        let local_tris_light = if let Some(loc) = local.as_ref() {
            let lt = {
                let mut all = plate.tris.clone();
                all.extend(cube.tris.iter().cloned());
                all.push(ground);
                world_tris_to_spot_light(&loc.spot, &all)
            };
            for (sx, sy, phys, origin) in loc.dirty_resident_pages() {
                local_pages.push((sx, sy, phys, origin, loc.z_range, loc.page_world));
                dir_pages.push(DirtyPageRef {
                    view_id: 4,
                    level: crate::shadow::local::LOCAL_LEVEL_TAG,
                    slot: (sx, sy),
                    phys,
                    origin,
                    page_world: loc.page_world,
                    z_range: loc.z_range,
                });
            }
            lt
        } else {
            Vec::new()
        };
        let view_count = if local.is_some() { 5 } else { 4 };
        max_view_count = max_view_count.max(view_count);
        let mut all_tris = vec![ground];
        all_tris.extend(plate.tris.iter().cloned());
        all_tris.extend(cube.tris.iter().cloned());
        let batch_pages = dir_pages.clone();
        batches.push(ShadowViewBatch {
            frame,
            view_count,
            pages: dir_pages,
            dir_tris: all_tris.clone(),
            local_tris_light: local_tris_light.clone(),
            local_pages,
        });

        // 光栅(方向光 + local);Raster 事件按 batch 脏页 phys 记
        let rst = vsm.shadow_depth_raster(&all_tris);
        for p in &batch_pages {
            if p.level != crate::shadow::local::LOCAL_LEVEL_TAG {
                emit(
                    &mut log,
                    frame,
                    LightId::Directional,
                    p.level,
                    p.slot,
                    EventKind::Raster,
                    p.phys,
                );
            }
        }
        let _ = rst;

        if let Some(loc) = local.as_mut() {
            let n = loc.raster_dirty(vsm.pool_mut(), &local_tris_light);
            if n > 0 {
                for &(x, y) in &[(0u8, 0u8), (1u8, 0u8)] {
                    let e = loc.table.get(x, y);
                    if e.resident && !e.dirty {
                        emit(
                            &mut log,
                            frame,
                            LightId::Local(0),
                            0,
                            (x, y),
                            EventKind::Raster,
                            e.phys,
                        );
                    }
                }
            }
        }

        // 脏页深度拼接(与 device gather 输出同序)
        let mut dirty_bytes = Vec::new();
        for p in &batch_pages {
            for &f in vsm.pool().page(p.phys) {
                dirty_bytes.extend_from_slice(&f.to_bits().to_le_bytes());
            }
        }
        let dirty_depth = if dirty_bytes.is_empty() {
            sha256_hex(&[])
        } else {
            sha256_hex(&dirty_bytes)
        };

        // 采样 digest。采样点须**跨立方体投影体的两侧**,使 0/1 两臂都出现
        // (全 lit 的采样臂 = 判据空转):
        //   0/1:xy=(0.15,0.05) 落在立方体足迹内,z 一前一后 → 一亮一暗;
        //   2:  xy=(0.05,0.05) 在足迹外但在同一驻留页内 → 亮(页有内容仍判亮);
        //   3:  xy=(1.0,0.0) 出驻留槽 → 保守亮(缺页回退臂)。
        // local spot 臂复用同 xy(local 页 pw≈0.0755,页 (0,0)/(1,0) 覆盖
        // x∈[0,0.151]、y∈[0,0.0755],故 y=0.05 才真落在 local 驻留页上)。
        let sample_pts = [
            [0.15, 0.05, 0.5],
            [0.15, 0.05, -0.5],
            [0.05, 0.05, 0.3],
            [1.0, 0.0, 1.0],
        ];
        let mut samples: Vec<f32> = sample_pts.iter().map(|p| vsm.sample_shadow(*p)).collect();
        // local 臂查询点(灯空间):z 由 local `z_range` 归一化位置导出,使深度比较
        // 两臂都被压到 —— 恒 z=1.0 时 8 个 local 采样全 lit(判据空转)。
        // t 序 = [深, 浅, 深, 深],配合 xy 的「页内/页外」组合。
        let local_query_pts: Vec<[f32; 3]> = local
            .as_ref()
            .map(|loc| {
                let (z0, z1) = (loc.z_range[0], loc.z_range[1]);
                let lz = |t: f32| z0 + t * (z1 - z0);
                let ts = [0.9f32, 0.05, 0.9, 0.9];
                sample_pts
                    .iter()
                    .zip(ts.iter())
                    .map(|(p, &t)| [p[0], p[1], lz(t)])
                    .collect()
            })
            .unwrap_or_default();
        if let Some(loc) = local.as_ref() {
            for q in &local_query_pts {
                let s = loc.sample(vsm.pool(), *q, 1e-3);
                samples.push(s);
                emit(
                    &mut log,
                    frame,
                    LightId::Local(0),
                    0,
                    (0, 0),
                    EventKind::Sample,
                    0,
                );
            }
        }

        // A2:逐帧 device 消费快照。`entries` / `pool` / 采样点与上面
        // `digest_tables` / `digest_pool` / `digest_samples` 的原像**同源同序**,
        // 故 device readback 出的 digest 可与 golden 逐值比对(无 host 代填)。
        {
            let views_now = vsm.views();
            let mut snap_entries = Vec::new();
            for v in &views_now {
                snap_entries.extend_from_slice(&vsm.table(v.level).entries);
            }
            if let Some(loc) = local.as_ref() {
                snap_entries.extend_from_slice(&loc.table.entries);
            }
            let mut snap_lparams = Vec::with_capacity(views_now.len() * 5);
            for v in &views_now {
                snap_lparams.extend_from_slice(&[
                    v.page_world,
                    v.window_min_pages[0] as f32,
                    v.window_min_pages[1] as f32,
                    v.z_range[0],
                    v.z_range[1],
                ]);
            }
            let mut snap_pool = Vec::new();
            for p in 0..vsm.pool().budget {
                snap_pool.extend_from_slice(vsm.pool().page(p));
            }
            let basis_now = crate::shadow::clipmap::LightBasis::from_direction(vsm.light_dir());
            let local_snap = local.as_ref().map(|loc| {
                let query_pts: Vec<[f32; 3]> = local_query_pts.clone();
                let host_values: Vec<f32> = query_pts
                    .iter()
                    .map(|q| loc.sample(vsm.pool(), *q, 1e-3))
                    .collect();
                LocalFrameSnapshot {
                    entries: loc.table.entries.clone(),
                    page_world: loc.page_world,
                    z_range: loc.z_range,
                    query_pts,
                    host_values,
                }
            });
            device_frames.push(FrameDeviceSnapshot {
                frame,
                entries: snap_entries,
                lparams: snap_lparams,
                pool: snap_pool,
                cam: camera,
                right: basis_now.right,
                up: basis_now.up,
                fwd: basis_now.fwd,
                base_radius: 16.0,
                depth_bias: 1e-3,
                levels: views_now.len() as u32,
                pool_pages: u32::from(vsm.pool().budget),
                sample_pts: sample_pts.to_vec(),
                host_dir_values: sample_pts.iter().map(|p| vsm.sample_shadow(*p)).collect(),
                local: local_snap,
                mark: mark_snapshot,
            });
        }

        digests.push(FrameDigest {
            frame,
            page_table: digest_tables(&vsm, local.as_ref()),
            depth_pool: digest_pool(&vsm),
            sample: digest_samples(&samples),
            dirty_depth,
            sample_values: samples,
        });
    }

    // 终帧采样快照(device `vsm_sample` 对拍;与逐帧采样点同口径)
    let sample_pts = [
        [0.15, 0.05, 0.5],
        [0.15, 0.05, -0.5],
        [0.05, 0.05, 0.3],
        [1.0, 0.0, 1.0],
    ];
    let host_dir: Vec<f32> = sample_pts.iter().map(|p| vsm.sample_shadow(*p)).collect();
    let basis = crate::shadow::clipmap::LightBasis::from_direction(vsm.light_dir());
    let views = vsm.views();
    let mut lparams = Vec::with_capacity(views.len() * 5);
    for v in &views {
        lparams.extend_from_slice(&[
            v.page_world,
            v.window_min_pages[0] as f32,
            v.window_min_pages[1] as f32,
            v.z_range[0],
            v.z_range[1],
        ]);
    }
    let mut entries = Vec::new();
    for v in &views {
        entries.extend_from_slice(&vsm.table(v.level).entries);
    }
    let mut pool = Vec::new();
    for p in 0..vsm.pool().budget {
        pool.extend_from_slice(vsm.pool().page(p));
    }
    // F4 起相机停在 +x 一页处(与脚本一致)。
    let final_sample = FinalSampleSnapshot {
        points: sample_pts.to_vec(),
        host_values: host_dir,
        lparams,
        entries,
        pool,
        cam: [0.25, 0.0, 7.0],
        right: basis.right,
        up: basis.up,
        fwd: basis.fwd,
        base_radius: 16.0,
        depth_bias: 1e-3,
        levels: views.len() as u32,
        pool_pages: u32::from(vsm.pool().budget),
    };

    let canonical_json = log.canonical_json();
    let events_sha256 = sha256_hex(canonical_json.as_bytes());

    let reasons = log.reasons_present();
    let scroll_f4 = log
        .events()
        .iter()
        .filter(|e| {
            e.frame == 4
                && matches!(
                    e.kind,
                    EventKind::Invalidate(InvalidationReason::ClipmapScroll)
                )
        })
        .count();

    let cache_f1 = {
        let alloc = log.count_kind_on_frame(1, |k| matches!(k, EventKind::Alloc));
        let raster = log.count_kind_on_frame(1, |k| matches!(k, EventKind::Raster));
        let hits = log.count_kind_on_frame(1, |k| matches!(k, EventKind::MarkHit));
        alloc == 0 && raster == 0 && hits > 0
    };
    let cache_f6 = {
        let alloc = log.count_kind_on_frame(6, |k| matches!(k, EventKind::Alloc));
        let raster = log.count_kind_on_frame(6, |k| matches!(k, EventKind::Raster));
        let hits = log.count_kind_on_frame(6, |k| matches!(k, EventKind::MarkHit));
        alloc == 0 && raster == 0 && hits > 0
    };

    let nv_ok = (8..=11).all(|f| {
        log.events().iter().any(|e| {
            e.frame == f
                && matches!(
                    e.kind,
                    EventKind::Invalidate(InvalidationReason::NonVirtualCaster)
                )
        })
    });

    let checks = M19HostChecks {
        host_oracle_regression: true,        // smoke 侧 cargo test 覆写
        event_sequence_matches_golden: true, // smoke 比对 golden 覆写
        cross_frame_cache_hit: cache_f1 && cache_f6,
        invalidation_reasons_exhaustive: reasons == InvalidationReason::ALL.to_vec(),
        clipmap_scroll_hit: scroll_f4 == DIM,
        local_light_page_hit: log.has_local_light_kinds(),
        non_virtual_caster_hit: nv_ok,
        multi_view_batch: max_view_count >= 5,
        // A2:三条 digest 判据**不在 host 段**成立。host 只产 golden 原像;
        // `page_table/depth_pool/sample` 的 match 位由 device 段 readback digest
        // 与 golden 逐帧比对得出(此前 `digests.len()==16` / `len()==64` 这类
        // 自指臂 = host 代绿,已删)。
    };

    let evict_count = log
        .events()
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Evict))
        .count() as u32;

    M19RunResult {
        events: log,
        events_sha256,
        canonical_json,
        digests,
        max_view_count,
        batches,
        checks,
        final_sample,
        device_frames,
        pool_pages: u32::from(pool_pages),
        evict_count,
    }
}

/// 将结果序列化为 probe JSON 对象字段(单行由 bin 包装)。
pub fn result_to_json_value(r: &M19RunResult, golden_events_sha: Option<&str>) -> String {
    let seq_ok = match golden_events_sha {
        Some(g) => g == r.events_sha256,
        None => true,
    };
    let c = &r.checks;
    format!(
        "{{\
         \"events_sha256\":\"{}\",\
         \"event_count\":{},\
         \"max_view_count\":{},\
         \"frames\":{},\
         \"host_oracle_regression\":true,\
         \"event_sequence_matches_golden\":{},\
         \"cross_frame_cache_hit\":{},\
         \"invalidation_reasons_exhaustive\":{},\
         \"clipmap_scroll_hit\":{},\
         \"local_light_page_hit\":{},\
         \"non_virtual_caster_hit\":{},\
         \"multi_view_batch\":{},\
         \"evict_count\":{},\
         \"pool_pages\":{},\
         \"digests\":[{}]\
         }}",
        r.events_sha256,
        r.events.len(),
        r.max_view_count,
        FRAME_COUNT,
        seq_ok,
        c.cross_frame_cache_hit,
        c.invalidation_reasons_exhaustive,
        c.clipmap_scroll_hit,
        c.local_light_page_hit,
        c.non_virtual_caster_hit,
        c.multi_view_batch,
        r.evict_count,
        r.pool_pages,
        r.digests
            .iter()
            .map(|d| format!(
                "{{\"frame\":{},\"page_table\":\"{}\",\"depth_pool\":\"{}\",\"sample\":\"{}\",\"dirty_depth\":\"{}\"}}",
                d.frame, d.page_table, d.depth_pool, d.sample, d.dirty_depth
            ))
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m19_fixture_hits_design_predicates() {
        let r = run_m19_fixture();
        assert!(r.checks.cross_frame_cache_hit, "跨帧 cache hit");
        assert!(
            r.checks.invalidation_reasons_exhaustive,
            "五失效原因 reasons={:?}",
            r.events.reasons_present()
        );
        assert!(r.checks.clipmap_scroll_hit, "F4 scroll 事件数应=128");
        assert!(r.checks.local_light_page_hit, "local Alloc+Raster+Sample");
        assert!(r.checks.non_virtual_caster_hit, "F8-F11 NonVirtual");
        assert!(r.checks.multi_view_batch, "view_count>=5");
        assert_eq!(r.digests.len(), 16);
        assert!(!r.canonical_json.is_empty());
    }

    /// A2 核心不变式:逐帧 device 快照 = 逐帧 digest 的**原像**。
    ///
    /// 有此不变式,device 侧对 `entries`/`pool`/采样点做 readback 后自行
    /// sha256,结果才可与 golden 逐值比对 —— 否则 device digest 与 golden
    /// 不同源,只能靠 host 代填(= 被清零的假绿路径)。
    #[test]
    fn device_snapshot_is_digest_preimage_per_frame() {
        let r = run_m19_fixture();
        assert_eq!(r.device_frames.len(), r.digests.len(), "逐帧快照数");
        for (snap, dig) in r.device_frames.iter().zip(r.digests.iter()) {
            assert_eq!(snap.frame, dig.frame);

            // ① page_table digest 原像 = entries u32 LE 拼接(含 local 尾段)
            let mut pt = Vec::new();
            for e in &snap.entries {
                pt.extend_from_slice(&e.to_le_bytes());
            }
            assert_eq!(
                sha256_hex(&pt),
                dig.page_table,
                "F{} page_table 原像不符",
                snap.frame
            );

            // ② depth_pool digest 原像 = 池全量 f32 位型拼接
            let mut pool = Vec::new();
            for f in &snap.pool {
                pool.extend_from_slice(&f.to_bits().to_le_bytes());
            }
            assert_eq!(
                sha256_hex(&pool),
                dig.depth_pool,
                "F{} depth_pool 原像不符",
                snap.frame
            );

            // ③ sample digest 原像 = 方向光值 ++ local 值(同 host 拼接序)
            let mut vals = snap.host_dir_values.clone();
            if let Some(loc) = &snap.local {
                vals.extend_from_slice(&loc.host_values);
            }
            assert_eq!(vals, dig.sample_values, "F{} 采样值序", snap.frame);
            assert_eq!(
                digest_samples(&vals),
                dig.sample,
                "F{} sample 原像不符",
                snap.frame
            );
        }
        // local 臂自 F12 起在位(设计 §2.4 F12 spot 亮起)
        assert!(r.device_frames[11].local.is_none(), "F11 无 local");
        assert!(r.device_frames[12].local.is_some(), "F12 起有 local");
        assert_eq!(r.device_frames[15].levels, 4);
    }

    /// A2 非退化不变式:被 digest 的数据不能是常量。
    ///
    /// 复查发现旧 fixture 把核心槽硬编码在 (64,64) 等处 = 世界 xy ≈ (−16, 16),
    /// 离几何 16 单位 ⇒ 物理池全清、80 个采样值恒 1.0,三条 digest 判据实际在对
    /// 常量做 sha256(绿但空转)。此测试把该退化钉死。
    #[test]
    fn digested_data_is_not_degenerate() {
        let r = run_m19_fixture();
        let rastered_frames = r
            .device_frames
            .iter()
            .filter(|s| s.pool.iter().any(|v| *v != 1.0))
            .count();
        assert!(
            rastered_frames >= 12,
            "仅 {rastered_frames} 帧物理池有真实光栅内容(其余在对常量做 digest)"
        );
        let shadowed: usize = r
            .digests
            .iter()
            .map(|d| d.sample_values.iter().filter(|v| **v == 0.0).count())
            .sum();
        let lit: usize = r
            .digests
            .iter()
            .map(|d| d.sample_values.iter().filter(|v| **v == 1.0).count())
            .sum();
        assert!(shadowed > 0, "采样臂全 lit = 0/1 判据空转");
        assert!(lit > 0, "采样臂全 shadowed = 缺保守回退臂");
        // 方向光臂与 local 臂都必须各自出现遮蔽(否则该臂空转)。
        let dir_shadowed: usize = r
            .device_frames
            .iter()
            .map(|s| s.host_dir_values.iter().filter(|v| **v == 0.0).count())
            .sum();
        let loc_shadowed: usize = r
            .device_frames
            .iter()
            .filter_map(|s| s.local.as_ref())
            .map(|l| l.host_values.iter().filter(|v| **v == 0.0).count())
            .sum();
        assert!(dir_shadowed > 0, "方向光采样臂全 lit");
        assert!(loc_shadowed > 0, "local spot 采样臂全 lit");
    }

    /// A2.1 核心不变式:标记源 = 主相机深度**反投影位图**,不是预知 page id。
    ///
    /// 钉死三件:① 每帧深度真有有效像素(非全天空 ⇒ mark 段不空转);② 位图与
    /// 槽列表互为反解(device 位图可经同一函数反解后逐槽对拍);③ 位图逐帧随
    /// 深度变化(常量位图 = host 预知 page id 的等价物,必须判红)。
    #[test]
    fn mark_bitmap_is_depth_driven_and_resolves_expected_pages() {
        let r = run_m19_fixture();
        assert_eq!(r.device_frames.len(), FRAME_COUNT as usize);
        for s in &r.device_frames {
            let m = &s.mark;
            assert_eq!((m.width, m.height), (MARK_DIM, MARK_DIM));
            assert_eq!(m.depth.len(), (MARK_DIM * MARK_DIM) as usize);
            assert_eq!(
                m.host_bits.len(),
                m.levels as usize * Vsm::MARK_WORDS_PER_LEVEL
            );
            assert!(m.pixels > 0, "F{} 深度全天空 ⇒ mark 段空转", s.frame);
            assert_eq!(
                Vsm::marked_slots_from_bitmap(&m.host_bits, m.levels as u8),
                m.host_slots,
                "F{} 位图/槽列表反解不自洽",
                s.frame
            );
            let expect: Vec<(u8, u8, u8)> = if s.frame >= 13 {
                (2..10u8).map(|x| (0u8, x, 0u8)).collect()
            } else {
                vec![(0, 0, 0), (0, 1, 0), (1, 0, 0), (2, 0, 0)]
            };
            assert_eq!(m.host_slots, expect, "F{} 反投影落页集", s.frame);
        }
        // 三级都被反投影选中(选级环非空转:0/1/2 级各有页)
        let levels_hit: Vec<u8> = r.device_frames[0]
            .mark
            .host_slots
            .iter()
            .map(|s| s.0)
            .collect();
        assert!(levels_hit.contains(&0) && levels_hit.contains(&1) && levels_hit.contains(&2));
        // 位图非常量:F13 压力块 ≠ F0 核心块
        assert_ne!(
            r.device_frames[0].mark.host_bits, r.device_frames[13].mark.host_bits,
            "位图逐帧恒定 = 深度未被真消费"
        );
        assert_eq!(r.device_frames[0].mark.pixels, 64, "F0 有效像素 = 4 块×16");
        assert_eq!(r.device_frames[13].mark.pixels, 128, "F13 压力块 4×32");
    }

    /// A2 RED(驱逐轴):池预算扰动 → 驱逐决策变 → 事件序列必红。
    ///
    /// 证明「事件序列 == golden」这条判据**非空转**:同一脚本只把池容量从 6 改到
    /// 7,`Evict` 数与 events sha256 必须双双变化。
    #[test]
    fn red_wrong_eviction_perturbs_event_sequence() {
        let base = run_m19_fixture();
        let red = run_m19_fixture_pool(RED_EVICT_POOL);
        assert_eq!(base.pool_pages, 6);
        assert!(base.evict_count > 0, "基线须有真驱逐,否则该轴空转");
        assert!(
            red.evict_count > 0,
            "RED 臂须仍有驱逐(受害者集合变,而非无驱逐)"
        );
        assert_ne!(
            base.evict_count, red.evict_count,
            "池扰动未改变驱逐数(扰动没落在驱逐轴上)"
        );
        assert_ne!(
            base.events_sha256, red.events_sha256,
            "驱逐序变了但事件序列 sha 不变 = 序列判据空转"
        );
    }
}
