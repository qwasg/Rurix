//! G9.3 几何轨 host probe(M92 蒙皮/更新率 + M93 VisibleClusterSet + M95 单源
//! 真相;spec/virtual_geometry.md RXS-0350/0352/0353)。
//!
//! 出口 = host 侧 evidence JSON(stdout 或 `--evidence <path>`):正例全绿 +
//! 负例 RED 臂检出标记 + 计数面(visible_clusters / blas_refit / 档位直方图 /
//! VisBuffer diff / provenance digest)。任一断言失败 → 退出码 1。
//!
//! device 接线点(归 CI 门代理统一真跑,本 probe 不承载):
//! - 蒙皮 device kernel ↔ host 参照逐顶点对拍(定点域容差 0;
//!   `geometry::skinning` 为金标准);
//! - 蒙皮簇 VisBuffer SW/HW 真机 diff=0(rurix-rt render_exec 骨架;
//!   host 侧断言 = `geometry::visbuffer::assert_visbuffer_diff_zero`);
//! - 蒙皮簇保守包围体注入 device 剔除/CLAS 当帧拼装(M94 面)。

use rurix_render::geometry::cull::CullCamera;
use rurix_render::geometry::gpu_scene::InstanceRecord;
use rurix_render::geometry::skinning::{
    ClusterSkinInput, SkinCache, SkinCacheSlot, SkinPalette, SkinnedClusterFrame, SkinningDriver,
    UpdateTier, conservative_skinned_aabb, skin_cluster, verify_bound_containment,
};
use rurix_render::geometry::visible_cluster_set::{
    DagNodeRec, MeshDagView, produce_visible_cluster_set, verify_cut_coverage,
    verify_frame_provenance,
};
use rurix_render::geometry::visbuffer::{
    VisibleSetScene, VisBufferCpu, raster_visible_set, visbuffer_diff_host,
};
use rurix_render::graph::types::ClusterRecord;
use rurix_render::rt::as_manager::{BlasCache, DynamicPolicy};
use rurix_render::shadow::vsm::shadow_tris_from_visible_set;

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn cluster(error: f32, parent_error: f32, page_id: u32) -> ClusterRecord {
    ClusterRecord {
        center: [0.0; 3],
        radius: 0.5,
        cone_axis: [0.0; 3],
        cone_cutoff: 2.0,
        error,
        parent_error,
        vertex_offset: 0,
        triangle_offset: 0,
        vertex_count: 0,
        triangle_count: 0,
        page_id,
        reserved: 0,
    }
}

fn inst_at(t: [f32; 3], cluster_offset: u32, cluster_count: u32) -> InstanceRecord {
    InstanceRecord {
        transform: [
            [1.0, 0.0, 0.0, t[0]],
            [0.0, 1.0, 0.0, t[1]],
            [0.0, 0.0, 1.0, t[2]],
        ],
        cluster_offset,
        cluster_count,
        material_id: 0,
        flags: 0,
        aabb_min: [t[0] - 2.0, t[1] - 2.0, t[2] - 2.0],
        mesh_id: 0,
        aabb_max: [t[0] + 2.0, t[1] + 2.0, t[2] + 2.0],
        reserved: u32::MAX,
    }
}

fn exact_cam() -> CullCamera {
    CullCamera {
        view_proj: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, -0.2],
            [0.0, 0.0, -1.0, 0.0],
        ],
        cam_pos: [0.0, 0.0, 0.0],
        screen_height_px: 1000.0,
        error_threshold_px: 1.0,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut evidence_path: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--evidence" => {
                i += 1;
                evidence_path = Some(args.get(i).expect("--evidence path").clone());
            }
            other => {
                eprintln!("unknown arg {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    let mut failures: Vec<String> = Vec::new();
    let check = |name: &str, ok: bool, failures: &mut Vec<String>| {
        if !ok {
            failures.push(name.to_string());
        }
    };

    // ------------------------------------------------------------------
    // 场景:conformance accept 语料 DAG(R → {A, B};A → {A0, A1})+ 实例 z=−100。
    // ------------------------------------------------------------------
    let records = vec![
        cluster(0.0, 0.5, 1),
        cluster(0.0, 0.5, 1),
        cluster(0.0, 2.0, 2),
        cluster(0.5, 2.0, 3),
        cluster(2.0, f32::INFINITY, 4),
    ];
    let nodes = vec![
        DagNodeRec { first_child: 0, child_count: 0, level: 0 },
        DagNodeRec { first_child: 0, child_count: 0, level: 0 },
        DagNodeRec { first_child: 0, child_count: 0, level: 0 },
        DagNodeRec { first_child: 0, child_count: 2, level: 1 },
        DagNodeRec { first_child: 2, child_count: 2, level: 2 },
    ];
    let children = vec![0u32, 1, 3, 2];
    let mesh = MeshDagView::new(&records, &nodes, &children).expect("拓扑");
    let cam = exact_cam();
    let inst = [inst_at([0.0, 0.0, -100.0], 0, 5)];

    // M93 正例:合法 cut = {A0, A1, B}。
    let set = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[1, 2, 3, 4], &[])
        .expect("合法 cut 产出");
    let cut_ids: Vec<u32> = set.entries.iter().map(|e| e.cluster).collect();
    check("m93.valid_cut", cut_ids == vec![0, 1, 2], &mut failures);
    check("m93.all_visible", set.visible_count() == 3, &mut failures);

    // M93 负例(reject/selection_cut_hole_injected.rx 数据面):空洞 + 重叠。
    let hole_red = verify_cut_coverage(&mesh, &[0, 2]).is_err();
    let overlap_red = verify_cut_coverage(&mesh, &[0, 2, 3]).is_err();
    check("m93.hole_red", hole_red, &mut failures);
    check("m93.overlap_red", overlap_red, &mut failures);

    // M93 兜底:页 1 未驻留 ⇒ {B, A};页到达 ⇒ 转正 {A0, A1, B}。
    let fb_set = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[2, 3, 4], &[])
        .expect("兜底产出");
    let fb_ids: Vec<u32> = fb_set.entries.iter().map(|e| e.cluster).collect();
    check("m93.fallback_cut", fb_ids == vec![2, 3], &mut failures);
    check("m93.fallback_evidence", fb_set.fallback.len() == 2, &mut failures);
    let restored = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 8, &[1, 2, 3, 4], &[])
        .expect("转正产出");
    let restored_ids: Vec<u32> = restored.entries.iter().map(|e| e.cluster).collect();
    check("m93.restored", restored_ids == vec![0, 1, 2], &mut failures);

    // M95 正例:一份三喂 + 帧末 provenance。
    let (raster, rt, vsm) = (set.feed_raster(), set.feed_rt(), set.feed_vsm());
    let provenance_ok = verify_frame_provenance(&set, &raster, &rt, &vsm).is_ok();
    check("m95.provenance", provenance_ok, &mut failures);
    // M95 负例(reject/bypass_single_source_variant.rx 数据面):serial 8 旁路重算。
    let bypass = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 8, &[1, 2, 3, 4], &[])
        .expect("旁路 set");
    let bypass_red = verify_frame_provenance(&set, &raster, &bypass.feed_rt(), &vsm).is_err();
    check("m95.bypass_red", bypass_red, &mut failures);

    // ------------------------------------------------------------------
    // M92:蒙皮驱动(近→静态→远降级→更新点)+ AsStats 计数面。
    // ------------------------------------------------------------------
    // 大底面三角形(z = −100 处覆盖足够屏幕像素,供光栅覆盖冒烟)。
    let vertices: Vec<[f32; 3]> = vec![[-30.0, -30.0, 0.0], [30.0, -30.0, 0.0], [0.0, 30.0, 0.0]];
    let weights: Vec<Vec<(u32, f32)>> =
        vec![vec![(0u32, 1.0f32)], vec![(0, 1.0)], vec![(0, 1.0)]];
    let bones = [0u32];
    let skin_input = ClusterSkinInput {
        max_influences: 1,
        bone_indices: &bones,
        bound_inflation: 0.0,
        rest_aabb_min: [-30.0, -30.0, 0.0],
        rest_aabb_max: [30.0, 30.0, 0.0],
        vertices: &vertices,
        weights: &weights,
    };
    let mut blas = BlasCache::new();
    let blas_id = blas.get_or_build(
        &vertices,
        &[[0u32, 1, 2]],
        DynamicPolicy::Deformable {
            refit_budget_frames: 1,
        },
    );
    let builds0 = blas.stats().blas_builds;
    let mut driver = SkinningDriver::new(1);
    let pose_a = SkinPalette {
        bones: vec![[
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]],
    };
    let pose_b = SkinPalette {
        bones: vec![[
            [1.0, 0.0, 0.0, 2.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ]],
    };
    let mut static_frame_zero_as_build = true;
    let frames = [
        (0u64, 5.0f32, &pose_a),   // 全速更新
        (1, 5.0, &pose_a),         // 静态(姿态 bit-equal 且档位不变)
        (2, 50.0, &pose_b),        // 1/4 档降级(2 % 4 ≠ 0)
        (4, 50.0, &pose_b),        // 1/4 档更新点(4 % 4 == 0)
    ];
    for &(f, dist, pose) in &frames {
        let (rb, bb) = (blas.stats().refits, blas.stats().blas_builds);
        driver
            .drive_frame(
                f,
                &[SkinnedClusterFrame {
                    input: &skin_input,
                    distance_m: dist,
                    blas: blas_id,
                }],
                pose,
                0.5,
                &mut blas,
            )
            .expect("驱动帧");
        if f == 1 && (blas.stats().refits != rb || blas.stats().blas_builds != bb) {
            static_frame_zero_as_build = false;
        }
    }
    check("m92.static_zero_build", static_frame_zero_as_build, &mut failures);
    let as_stats = blas.stats();
    check("m92.refit_counted", as_stats.refits == 2, &mut failures);
    check(
        "m92.no_extra_build",
        as_stats.blas_builds - builds0 == 0,
        &mut failures,
    );
    check(
        "m92.tier_histogram",
        driver.stats.tier_histogram == [2, 0, 0, 2],
        &mut failures,
    );
    // 档位闭集:闭集外声明拒绝。
    check(
        "m92.tier_closed_set",
        UpdateTier::from_period(5).is_err() && UpdateTier::from_period(4).is_ok(),
        &mut failures,
    );
    // 包围体 RED 臂:人为缩小必须检出。
    let shrunk_red = {
        let out = skin_cluster(&skin_input, &pose_b).expect("蒙皮");
        let (lo, hi) = conservative_skinned_aabb(&skin_input, &pose_b).expect("包围体");
        let shrunk = (
            [lo[0] + 2.0, lo[1] + 2.0, lo[2] + 2.0],
            [hi[0] - 2.0, hi[1] - 2.0, hi[2] - 2.0],
        );
        verify_bound_containment(&shrunk, &out).is_err()
    };
    check("m92.shrunk_bound_red", shrunk_red, &mut failures);

    // ------------------------------------------------------------------
    // M95 光栅/VSM 消费腿 + 蒙皮簇 VisBuffer diff=0(host)。
    // ------------------------------------------------------------------
    // 两三角形场景:簇 1(叶 1)蒙皮(skin cache),簇 0/2(叶 0/B)静态;
    // 静态池簇段与可见元素按 (cluster → 槽位) 解析。
    let rest1: Vec<[f32; 3]> = vertices.clone();
    let skinned1 = driver.cache.slots[0].positions.clone();
    let mut voff = 0u32;
    let mut pool: Vec<[f32; 3]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    let mut scene_records: Vec<ClusterRecord> = Vec::new();
    for (ci, base) in records.iter().enumerate() {
        let mut r = *base;
        if ci < 3 {
            // 三叶各 1 三角形;内部节点不带几何(不会被选中)。
            r.vertex_offset = voff;
            r.triangle_offset = idx.len() as u32;
            r.vertex_count = 3;
            r.triangle_count = 1;
            pool.extend_from_slice(&rest1);
            idx.extend_from_slice(&[0, 1, 2]);
            voff += 3;
        }
        scene_records.push(r);
    }
    // skin_slot_of:叶 1(全局簇 1)→ 槽位 0。
    let skin_slot_of = [u32::MAX, 0u32, u32::MAX, u32::MAX, u32::MAX];
    let skin = SkinCache {
        slots: vec![SkinCacheSlot {
            positions: skinned1.clone(),
            bound: ([0.0; 3], [0.0; 3]),
            version: driver.cache.slots[0].version,
            stale_frames: 0,
        }],
    };
    // 直烘对照池:簇 1 段烘入蒙皮后顶点。
    let mut baked = pool.clone();
    baked[3..6].copy_from_slice(&skinned1);
    let vp = [
        [4.0, 0.0, 0.0, 0.0],
        [0.0, 4.0, 0.0, 0.0],
        [0.0, 0.0, -1.0, -0.2],
        [0.0, 0.0, -1.0, 0.0],
    ];
    let mut leg_a = VisBufferCpu::new(16, 16);
    raster_visible_set(
        &mut leg_a,
        &set,
        &VisibleSetScene {
            instances: &inst,
            clusters: &scene_records,
            vertices: &pool,
            indices: &idx,
            view_proj: vp,
            skin: Some(&skin),
            skin_slot_of: &skin_slot_of,
        },
    );
    let mut leg_b = VisBufferCpu::new(16, 16);
    raster_visible_set(
        &mut leg_b,
        &set,
        &VisibleSetScene {
            instances: &inst,
            clusters: &scene_records,
            vertices: &baked,
            indices: &idx,
            view_proj: vp,
            skin: None,
            skin_slot_of: &[u32::MAX; 5],
        },
    );
    let diff = visbuffer_diff_host(&leg_a, &leg_b).expect("尺寸一致");
    check("m95.visbuffer_diff_zero", diff.mismatched == 0, &mut failures);
    check("m95.raster_covered", leg_a.count_valid() > 0, &mut failures);

    let vsm_tris = shadow_tris_from_visible_set(
        &set,
        &inst,
        &scene_records,
        &pool,
        &idx,
        Some(&skin),
        &skin_slot_of,
    );
    check("m95.vsm_tris", vsm_tris.len() == 3, &mut failures);

    // ------------------------------------------------------------------
    // evidence JSON(hand-rolled;零依赖纪律)。
    // ------------------------------------------------------------------
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9g93.geometry_probe.v1\",\n  \"visible_clusters\": {},\n  \
         \"set_digest\": \"{}\",\n  \"provenance_ok\": {},\n  \"hole_red_detected\": {},\n  \
         \"overlap_red_detected\": {},\n  \"bypass_red_detected\": {},\n  \
         \"shrunk_bound_red_detected\": {},\n  \"fallback_records\": {},\n  \
         \"restored_cut\": {:?},\n  \"blas_builds\": {},\n  \"blas_refit\": {},\n  \
         \"tlas_rebuilds\": {},\n  \"anim_update_tier_histogram\": {:?},\n  \
         \"skinned_updates\": {},\n  \"stale_skips\": {},\n  \"static_skips\": {},\n  \
         \"static_frame_zero_as_build\": {},\n  \"visbuffer_diff_mismatched\": {},\n  \
         \"vsm_depth_tris\": {},\n  \"failures\": {:?}\n}}",
        set.visible_count(),
        hex(&set.provenance_digest),
        provenance_ok,
        hole_red,
        overlap_red,
        bypass_red,
        shrunk_red,
        fb_set.fallback.len(),
        restored_ids,
        as_stats.blas_builds,
        as_stats.refits,
        as_stats.tlas_rebuilds,
        driver.stats.tier_histogram,
        driver.stats.skinned_updates,
        driver.stats.stale_skips,
        driver.stats.static_skips,
        static_frame_zero_as_build,
        diff.mismatched,
        vsm_tris.len(),
        failures,
    );
    match evidence_path {
        Some(p) => std::fs::write(&p, &json).expect("写 evidence"),
        None => println!("{json}"),
    }
    if failures.is_empty() {
        std::process::exit(0);
    }
    eprintln!("G9.3 geometry probe FAIL: {failures:?}");
    std::process::exit(1);
}
