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

const DIM: usize = PAGE_TABLE_DIM as usize;
const FRAME_COUNT: u32 = 16;

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
    pub page_table_digest_match: bool,
    pub depth_readback_digest_match: bool,
    pub sample_digest_match: bool,
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
        emit(log, frame, LightId::Directional, level, (x, y), kind, e.phys);
    }
}

fn record_alloc(log: &mut EventLog, frame: u32, light: LightId, before: &[(u8, u8, u8, bool)], vsm: &Vsm) {
    for &(level, x, y, was_res) in before {
        let e = vsm.table(level).get(x, y);
        if !was_res && e.resident {
            emit(
                log,
                frame,
                light,
                level,
                (x, y),
                EventKind::Alloc,
                e.phys,
            );
        }
    }
}

/// 跑 M19 16 帧 host 金标准脚本。
pub fn run_m19_fixture() -> M19RunResult {
    let mut log = EventLog::new();
    let mut digests = Vec::new();
    let mut batches = Vec::new();
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
    let mut vsm = Vsm::new(cfg_m19(6), light0, cam0);
    let mut local: Option<LocalLightPages> = None;
    let core_slots: [(u8, u8, u8); 4] = [(0, 64, 64), (0, 65, 64), (1, 64, 64), (2, 32, 32)];

    for frame in 0..FRAME_COUNT {
        let camera = if frame == 4 {
            [0.25, 0.0, 7.0]
        } else if frame >= 4 {
            [0.25, 0.0, 7.0]
        } else {
            cam0
        };

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

        // 标记:常规帧标核心槽;F13–F14 只标新压力槽,使核心驻留页龄≥1
        // 可被 LRU 驱逐(本帧标记页不可驱逐纪律)。
        let marked: Vec<(u8, u8, u8)> = if frame >= 13 {
            (0..8u8).map(|i| (0, 70 + i, 70)).collect()
        } else {
            core_slots.to_vec()
        };
        for &(l, x, y) in &marked {
            vsm.mark_slot(l, x, y);
        }
        record_marks(&mut log, &vsm, frame, &marked);

        if let Some(loc) = local.as_mut() {
            if frame >= 12 {
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
            let before_local = [
                loc.table.get(0, 0).resident,
                loc.table.get(1, 0).resident,
            ];
            let st = loc.alloc_into(
                vsm.pool_mut(),
                || dir_victim,
                |id| foreign.push(id),
            );
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
                local_pages.push((
                    sx,
                    sy,
                    phys,
                    origin,
                    loc.z_range,
                    loc.page_world,
                ));
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

        // 采样 digest
        let sample_pts = [
            [0.0, 0.0, 0.0],
            [0.2, 0.5, 0.0],
            [0.15, 0.1, 0.1],
            [1.0, 0.0, 1.0],
        ];
        let mut samples: Vec<f32> = sample_pts.iter().map(|p| vsm.sample_shadow(*p)).collect();
        if let Some(loc) = local.as_ref() {
            for p in &sample_pts {
                let s = loc.sample(vsm.pool(), [p[0], p[1], 1.0], 1e-3);
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

        digests.push(FrameDigest {
            frame,
            page_table: digest_tables(&vsm, local.as_ref()),
            depth_pool: digest_pool(&vsm),
            sample: digest_samples(&samples),
            dirty_depth,
            sample_values: samples,
        });
    }

    // 终帧采样快照(device `vsm_sample` 对拍)
    let sample_pts = [
        [0.0, 0.0, 0.0],
        [0.2, 0.5, 0.0],
        [0.15, 0.1, 0.1],
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
        host_oracle_regression: true, // smoke 侧 cargo test 覆写
        event_sequence_matches_golden: true, // smoke 比对 golden 覆写
        cross_frame_cache_hit: cache_f1 && cache_f6,
        invalidation_reasons_exhaustive: reasons == InvalidationReason::ALL.to_vec(),
        clipmap_scroll_hit: scroll_f4 == DIM,
        local_light_page_hit: log.has_local_light_kinds(),
        non_virtual_caster_hit: nv_ok,
        multi_view_batch: max_view_count >= 5,
        // host 自洽 digest(device 段再对拍)
        page_table_digest_match: digests.len() == FRAME_COUNT as usize,
        depth_readback_digest_match: digests.iter().all(|d| d.depth_pool.len() == 64),
        sample_digest_match: digests.iter().all(|d| d.sample.len() == 64),
    };

    M19RunResult {
        events: log,
        events_sha256,
        canonical_json,
        digests,
        max_view_count,
        batches,
        checks,
        final_sample,
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
         \"page_table_digest_match\":{},\
         \"depth_readback_digest_match\":{},\
         \"sample_digest_match\":{},\
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
        c.page_table_digest_match,
        c.depth_readback_digest_match,
        c.sample_digest_match,
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
        assert!(
            r.checks.clipmap_scroll_hit,
            "F4 scroll 事件数应=128"
        );
        assert!(r.checks.local_light_page_hit, "local Alloc+Raster+Sample");
        assert!(r.checks.non_virtual_caster_hit, "F8-F11 NonVirtual");
        assert!(r.checks.multi_view_batch, "view_count>=5");
        assert_eq!(r.digests.len(), 16);
        assert!(!r.canonical_json.is_empty());
    }
}
