// Assisted-by: Claude（G31+ #77 簇剔除 device kernel 化 + 簇级两阶段 HZB）
//! G31+ #77 簇剔除 + 组共享球 LOD cut + 簇级两阶段 HZB device harness。
//!
//! 链：合成场景（uv_sphere quality 档 DAG × K 平移副本 = 全局簇表,host 单源
//! 生成夹具,G27 体例）→ `kernels/g31_cluster_cull.rx`（rurixc 产 SPV + bin 侧
//! NoContraction 注入）经 `vk::run_compute` 两 dispatch 真跑——
//! pass1（上帧金字塔初剔全簇）→ pass2（本帧金字塔重测遮挡列表,Haar &
//! Aaltonen 2015 / Nanite 两遍语义）→ host 金标准对拍。
//!
//! 判据闭集：
//! ① 判定码序列逐项全等（device vs host 复算;固定槽位顺序无关面）——
//!    金标准 = cull.rs 三关同式 + `select_lod_cut_grouped`（组共享判定球,
//!    生产直调）+ `HzbPyramid::test_rect`（冻结 host 面直调）;
//! ② 最终可见集（pass1 ∪ pass2 可见,排序归一）全等;
//! ③ **零假阳性**：最终被剔簇经 `exact_rect_occluded`（本帧深度逐像素精确
//!    真值裁判）必真;
//! ④ 剔除真实发生（遮挡数 > 0）且 disocclusion 真实发生（pass2 新增可见
//!    > 0——两阶段第二段的存在性证明）;
//! ⑤ device 双跑位级一致;
//! ⑥ `--red-arm tamper`：篡改本帧金字塔单纹素 ⇒ 判定序列必变（构造性
//!    证明消费路径命中）。
//!
//! 三态：无 Vulkan loader/SPV → `skipped_dev_env` 退 0（不冒充 PASS;
//! `RURIX_REQUIRE_REAL=1` 翻硬 FAIL）。evidence JSON 落 `--evidence` 路径
//! （门编排/schema 注册归后续 smoke 波,本 harness 自带判据闭集）。
//!
//! 用法：
//!   g31_cluster_cull_device --spv <g31_cluster_cull.spv> --evidence <path>
//!     [--copies 5] [--error-threshold-px 1.0] [--red-arm tamper]

use std::path::Path;

use rurix_geom_build::dag::{DagBuildParams, build_dag_params};
use rurix_geom_build::lod_bounds::derive_lod_bounds;
use rurix_geom_build::{ClusterDag, TriMesh};
use rurix_render::geometry::cull::{
    CullCamera, DEFAULT_BIN_THRESHOLD_PX, VisibleCluster, compact_draw_args,
};
use rurix_render::geometry::gpu_scene::{IDENTITY_3X4, InstanceRecord, NO_PARENT};
use rurix_render::geometry::hzb::{DepthConvention, HzbPyramid, Occlusion, exact_rect_occluded};
use rurix_render::geometry::material_pass::{classify, resolve};
use rurix_render::geometry::visbuffer::{VISBUFFER_CLEAR, VisBufferCpu};
use rurix_render::geometry::visbuffer_swhw_spv::sw_visbuffer_u64_spv;
use rurix_render::geometry::visible_cluster_set::{
    DagNodeRec, LodBounds, MeshDagView, select_lod_cut_grouped,
};
use rurix_render::graph::types::ClusterRecord;
use rurix_render::temporal::common::{Mat4, look_at_rh, perspective_rh_zo};
use rurix_render::temporal::image::ImageF32;
use rurix_rt::vk;

const TAG: &str = "[g31_cluster_cull_device]";
const SCREEN_W: f32 = 1280.0;
const SCREEN_H: f32 = 720.0;
const ZNEAR: f32 = 0.1;
const DEPTH_W: u32 = 256;
const DEPTH_H: u32 = 144;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_u32(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_u32(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn load_spv(path: &Path) -> Vec<u32> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("SPV 读取 {path:?}: {e}")));
    if bytes.len() % 4 != 0 {
        fail("SPV 字节数非 4 对齐");
    }
    read_u32(&bytes)
}

/// SPIR-V NoContraction 注入（g31_cluster_stream 同律 bin 侧注入）。
fn spv_inject_no_contraction(spv: &[u32]) -> Vec<u32> {
    let mut result_ids: Vec<u32> = Vec::new();
    let mut i = 5usize;
    let mut first_decorate: Option<usize> = None;
    let mut first_type: Option<usize> = None;
    while i < spv.len() {
        let w = spv[i];
        let wc = (w >> 16) as usize;
        let op = w & 0xFFFF;
        if wc == 0 || i + wc > spv.len() {
            fail("SPIR-V 指令流越界（NoContraction 注入）");
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
        .unwrap_or_else(|| fail("SPIR-V 无 annotation/type 段锚"));
    let mut out = Vec::with_capacity(spv.len() + result_ids.len() * 3);
    out.extend_from_slice(&spv[..at]);
    for id in &result_ids {
        out.push((3 << 16) | 71); // OpDecorate, wc=3
        out.push(*id);
        out.push(42); // NoContraction
    }
    out.extend_from_slice(&spv[at..]);
    out
}

// ---------------------------------------------------------------------------
// 合成夹具（host 单源;G27 体例）
// ---------------------------------------------------------------------------

struct Fixture {
    /// 全局簇表（K 副本拼接;10 f32/簇 = kernel cluster_f32 布局）。
    cluster_f32: Vec<f32>,
    /// 逐簇组共享判定球（8 f32/簇 = kernel lod_f32 布局）。
    lod_f32: Vec<f32>,
    /// 副本段（(cluster_offset, cluster_count)）+ 各副本平移后 DAG 视图数据。
    copies: Vec<CopyDag>,
    total_clusters: usize,
    cam: CullCamera,
    view_rows: [f32; 12],
    vp_rows: [f32; 16],
    uv_half_kx: f32,
    uv_half_ky: f32,
    /// 上帧深度（金字塔 A 源）与本帧深度（金字塔 B 源;墙右缘左移 =
    /// disocclusion 面）。reverse-Z [0,1]。
    depth_prev: ImageF32,
    depth_curr: ImageF32,
    /// 基 DAG（--visbuffer 臂几何段消费:顶点/局部索引;副本几何 = 顶点
    /// + copy_dx 偏移）。
    dag: ClusterDag,
    /// 逐副本平移。
    copy_dx: Vec<[f32; 3]>,
}

struct CopyDag {
    offset: u32,
    records: Vec<ClusterRecord>,
    nodes: Vec<DagNodeRec>,
    children: Vec<u32>,
    self_lod: Vec<LodBounds>,
    parent_lod: Vec<LodBounds>,
}

fn translate_record(r: &ClusterRecord, off: [f32; 3]) -> ClusterRecord {
    ClusterRecord {
        center: [
            r.center[0] + off[0],
            r.center[1] + off[1],
            r.center[2] + off[2],
        ],
        ..*r
    }
}

fn build_fixture(copies_n: usize, threshold_px: f32) -> Fixture {
    let mesh = TriMesh::uv_sphere(1.0, 32, 32);
    let dag: ClusterDag = build_dag_params(&mesh, &DagBuildParams::quality());
    let (self_b, parent_b) =
        derive_lod_bounds(&dag).unwrap_or_else(|e| fail(&format!("LOD 球派生: {e}")));
    let mut cluster_f32 = Vec::new();
    let mut lod_f32 = Vec::new();
    let mut copies = Vec::new();
    let mut copy_dx = Vec::new();
    let n = dag.records.len();
    for k in 0..copies_n {
        // 近/远两组副本（偶 = 近排 z 0,奇 = 远排 z −45）:分箱两路
        //（HW 大三角 / SW 小三角）与 LOD 层带都真实非空。
        let off = [
            (k as f32 - (copies_n as f32 - 1.0) * 0.5) * 3.0,
            0.0,
            if k % 2 == 0 { 0.0 } else { -45.0 },
        ];
        copy_dx.push(off);
        let mut records = Vec::with_capacity(n);
        let mut nodes = Vec::with_capacity(n);
        let mut self_lod = Vec::with_capacity(n);
        let mut parent_lod = Vec::with_capacity(n);
        for i in 0..n {
            let r = translate_record(&dag.records[i], off);
            cluster_f32.extend_from_slice(&[
                r.center[0],
                r.center[1],
                r.center[2],
                r.radius,
                r.cone_axis[0],
                r.cone_axis[1],
                r.cone_axis[2],
                r.cone_cutoff,
                r.error,
                // kernel 的 +∞ sentinel 面:f32::MAX 经 ≥1e9 分支等价处理。
                if r.parent_error.is_finite() {
                    r.parent_error
                } else {
                    2.0e9
                },
            ]);
            let sb = self_b[i];
            let pb = parent_b[i];
            lod_f32.extend_from_slice(&[
                sb[0] + off[0],
                sb[1] + off[1],
                sb[2] + off[2],
                sb[3],
                pb[0] + off[0],
                pb[1] + off[1],
                pb[2] + off[2],
                pb[3],
            ]);
            records.push(r);
            nodes.push(DagNodeRec {
                first_child: dag.node(i as u32).first_child,
                child_count: dag.node(i as u32).child_count,
                level: dag.node(i as u32).level,
            });
            self_lod.push(LodBounds {
                center: [sb[0] + off[0], sb[1] + off[1], sb[2] + off[2]],
                radius: sb[3],
            });
            parent_lod.push(LodBounds {
                center: [pb[0] + off[0], pb[1] + off[1], pb[2] + off[2]],
                radius: pb[3],
            });
        }
        copies.push(CopyDag {
            offset: (k * n) as u32,
            records,
            nodes,
            children: dag.children.clone(),
            self_lod,
            parent_lod,
        });
    }
    // 相机（确定性:eye 在 +z 看向原点排;60° fovy）。
    let eye = [0.0f32, 0.0, 9.0];
    let view = look_at_rh(eye, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let proj = perspective_rh_zo(60.0f32.to_radians(), SCREEN_W / SCREEN_H, ZNEAR, 100.0);
    let vp = proj.mul(&view);
    let mut vp_rows = [0.0f32; 16];
    let mut view_rows = [0.0f32; 12];
    for r in 0..4 {
        for c in 0..4 {
            vp_rows[r * 4 + c] = vp.m[r][c];
        }
    }
    for r in 0..3 {
        for c in 0..4 {
            view_rows[r * 4 + c] = view.m[r][c];
        }
    }
    let cam = CullCamera {
        view_proj: vp.m,
        cam_pos: eye,
        screen_height_px: SCREEN_H,
        error_threshold_px: threshold_px,
    };
    // 深度场（reverse-Z:墙 0.9 = viewz≈0.11 很近;背景 0.0 = 无穷远）。
    // 上帧墙覆盖左 55% 屏;本帧墙缩到左 35%（右缘露出 = pass2 disocclusion 面）。
    let wall = |edge: f32| -> ImageF32 {
        ImageF32::from_fn(DEPTH_W, DEPTH_H, 1, move |x, _y, _| {
            let fx = (x as f32 + 0.5) / DEPTH_W as f32;
            if fx < edge { 0.9 } else { 0.0 }
        })
    };
    Fixture {
        cluster_f32,
        lod_f32,
        copies,
        total_clusters: copies_n * n,
        cam,
        view_rows,
        vp_rows,
        uv_half_kx: 0.5 * proj.m[0][0],
        uv_half_ky: 0.5 * proj.m[1][1],
        depth_prev: wall(0.55),
        depth_curr: wall(0.35),
        dag,
        copy_dx,
    }
}

// ---------------------------------------------------------------------------
// host 金标准（判定链 = kernel 字面镜像;LOD 面直调 select_lod_cut_grouped,
// HZB 面直调 HzbPyramid::test_rect——生产冻结件 0-byte 消费）
// ---------------------------------------------------------------------------

/// 球屏幕 rect + reverse-Z 最近深度（kernel 关 4 同式;near_z ≤ znear =
/// 近平面骑跨保守可见 None）。
fn sphere_rect_depth(fx: &Fixture, i: usize) -> Option<([f32; 2], [f32; 2], f32)> {
    let fb = i * 10;
    let (cx, cy, cz) = (
        fx.cluster_f32[fb],
        fx.cluster_f32[fb + 1],
        fx.cluster_f32[fb + 2],
    );
    let radius = fx.cluster_f32[fb + 3];
    let vr = &fx.view_rows;
    let viewz = -(vr[8] * cx + vr[9] * cy + vr[10] * cz + vr[11]);
    let near_z = viewz - radius;
    if near_z <= ZNEAR {
        return None;
    }
    let m = &fx.vp_rows;
    let m0 = m[0] * cx + m[1] * cy + m[2] * cz + m[3];
    let m1 = m[4] * cx + m[5] * cy + m[6] * cz + m[7];
    let mw = m[12] * cx + m[13] * cy + m[14] * cz + m[15];
    if mw <= 1e-6 {
        return None;
    }
    let u = (m0 / mw) * 0.5 + 0.5;
    let v = 0.5 - (m1 / mw) * 0.5;
    let hu = radius * fx.uv_half_kx / near_z;
    let hv = radius * fx.uv_half_ky / near_z;
    let uv_min = [(u - hu).max(0.0), (v - hv).max(0.0)];
    let uv_max = [(u + hu).min(1.0), (v + hv).min(1.0)];
    Some((uv_min, uv_max, ZNEAR / near_z))
}

/// host 判定链（kernel 逐字面镜像;`in_cut` 由 select_lod_cut_grouped 集合
/// 预产——生产金标准直调,禁旁路重算判定公式）。
fn host_decision(fx: &Fixture, i: usize, in_cut: bool, hzb: &HzbPyramid) -> u32 {
    let fb = i * 10;
    let (cx, cy, cz) = (
        fx.cluster_f32[fb],
        fx.cluster_f32[fb + 1],
        fx.cluster_f32[fb + 2],
    );
    let radius = fx.cluster_f32[fb + 3];
    let fr = fx.cam.frustum();
    if !fr.contains_sphere([cx, cy, cz], radius) {
        return 0;
    }
    let cutoff = fx.cluster_f32[fb + 7];
    if cutoff < 1.0 {
        let v = [
            cx - fx.cam.cam_pos[0],
            cy - fx.cam.cam_pos[1],
            cz - fx.cam.cam_pos[2],
        ];
        let dist = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if dist > 1e-6 {
            let a = [
                fx.cluster_f32[fb + 4],
                fx.cluster_f32[fb + 5],
                fx.cluster_f32[fb + 6],
            ];
            let alen = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
            if alen >= 1e-12 {
                let dota = (v[0] / dist) * (a[0] / alen)
                    + (v[1] / dist) * (a[1] / alen)
                    + (v[2] / dist) * (a[2] / alen);
                if dota >= cutoff {
                    return 1;
                }
            }
        }
    }
    if !in_cut {
        return 2;
    }
    match sphere_rect_depth(fx, i) {
        None => 4, // 近平面骑跨保守可见
        Some((mn, mx, nearest)) => {
            if hzb.test_rect(mn, mx, nearest) == Occlusion::Occluded {
                3
            } else {
                4
            }
        }
    }
}

/// 全局 in_cut 集合（逐副本 select_lod_cut_grouped;identity 变换——夹具
/// 世界空间直存,与 kernel 同参）。
fn host_cut_flags(fx: &Fixture) -> Vec<bool> {
    let ident = rurix_render::geometry::gpu_scene::IDENTITY_3X4;
    let mut flags = vec![false; fx.total_clusters];
    for c in &fx.copies {
        let view = MeshDagView::new(&c.records, &c.nodes, &c.children)
            .unwrap_or_else(|e| fail(&format!("夹具 DAG 拓扑: {e}")));
        let cut = select_lod_cut_grouped(&view, &c.self_lod, &c.parent_lod, &ident, &fx.cam);
        for local in cut {
            flags[(c.offset + local) as usize] = true;
        }
    }
    flags
}

// ---------------------------------------------------------------------------
// device 臂
// ---------------------------------------------------------------------------

struct PyramidFlat {
    data: Vec<f32>,
    meta: Vec<u32>, // 3/级 [offset, w, h]
    levels: u32,
}

fn flatten_pyramid(p: &HzbPyramid) -> PyramidFlat {
    let mut data = Vec::new();
    let mut meta = Vec::new();
    for m in &p.mips {
        meta.extend_from_slice(&[data.len() as u32, m.w, m.h]);
        for y in 0..m.h {
            for x in 0..m.w {
                data.push(m.get(x, y, 0));
            }
        }
    }
    PyramidFlat {
        data,
        meta,
        levels: p.mips.len() as u32,
    }
}

struct PassOut {
    decisions: Vec<u32>,
    visible: Vec<u32>,
    occluded: Vec<u32>,
    overflow: u32,
}

#[allow(clippy::too_many_arguments)]
fn run_pass(
    spv: &[u32],
    entry: &str,
    fx: &Fixture,
    pyr: &PyramidFlat,
    input_ids: &[u32],
    threshold_px: f32,
    mode: u32,
) -> PassOut {
    let n = input_ids.len();
    let cap = fx.total_clusters.max(1);
    let mut params = vec![0.0f32; 64];
    let fr = fx.cam.frustum();
    for (pi, p) in fr.planes.iter().enumerate() {
        params[pi * 4..pi * 4 + 4].copy_from_slice(p);
    }
    params[24..27].copy_from_slice(&fx.cam.cam_pos);
    params[27] = fx.cam.view_proj[1][1] * SCREEN_H * 0.5;
    params[28] = threshold_px;
    params[29] = n as f32;
    params[30] = mode as f32;
    params[31] = fx.uv_half_kx;
    params[32] = fx.uv_half_ky;
    params[33] = ZNEAR;
    params[34] = pyr.levels as f32;
    params[35] = cap as f32;
    params[36..52].copy_from_slice(&fx.vp_rows);
    params[52..64].copy_from_slice(&fx.view_rows);
    let mut bufs: Vec<Vec<u8>> = vec![
        bytes_f32(&params),
        bytes_f32(&fx.cluster_f32),
        bytes_f32(&fx.lod_f32),
        bytes_u32(input_ids),
        bytes_f32(&pyr.data),
        bytes_u32(&pyr.meta),
        vec![0u8; 12],
        vec![0u8; n.max(1) * 4],
        vec![0u8; cap * 4],
        vec![0u8; cap * 4],
    ];
    vk::run_compute(spv, entry, &mut bufs, &[], [n.max(1) as u32, 1, 1])
        .unwrap_or_else(|e| fail(&format!("cluster_cull dispatch 失败: {e}")));
    let counters = read_u32(&bufs[6]);
    let decisions = read_u32(&bufs[7]);
    let vis_raw = read_u32(&bufs[8]);
    let occ_raw = read_u32(&bufs[9]);
    let visible = vis_raw[..(counters[0] as usize).min(cap)].to_vec();
    let occluded = occ_raw[..(counters[1] as usize).min(cap)].to_vec();
    PassOut {
        decisions: decisions[..n].to_vec(),
        visible,
        occluded,
        overflow: counters[2],
    }
}

/// 两阶段全流程（device）:pass1 上帧金字塔全簇 → pass2 本帧金字塔重测遮挡。
fn run_two_pass(
    spv: &[u32],
    entry: &str,
    fx: &Fixture,
    pyr_prev: &PyramidFlat,
    pyr_curr: &PyramidFlat,
    threshold_px: f32,
) -> (PassOut, PassOut) {
    let all: Vec<u32> = (0..fx.total_clusters as u32).collect();
    let p1 = run_pass(spv, entry, fx, pyr_prev, &all, threshold_px, 0);
    let mut occ_sorted = p1.occluded.clone();
    occ_sorted.sort_unstable();
    let p2 = run_pass(spv, entry, fx, pyr_curr, &occ_sorted, threshold_px, 1);
    (p1, p2)
}

fn jstr(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "/").replace('"', "'"))
}

// ---------------------------------------------------------------------------
// --visbuffer 臂（G31+ #74/#75/#111 机制链:cut 可见集 → SW/HW 分箱 →
// SW compute 软光栅 device 真跑（M95 u64 原子腿转正消费）→ 覆盖对拍 →
// 合并 → classify/resolve 材质分箱——生产 pass 序接线归后续波,诚实登记）
// ---------------------------------------------------------------------------

const VIS_W: u32 = 96;
const VIS_H: u32 = 54;

struct VisArmOut {
    sw_clusters: u32,
    hw_clusters: u32,
    sw_tris: u32,
    hw_tris: u32,
    sw_covered: u32,
    hw_covered: u32,
    merged_covered: u32,
    classify_tiles: u32,
    classify_buckets: u32,
    resolved_pixels: u32,
}

/// 世界三角 → 屏幕三角（visbuffer::raster_cluster_tris 投影口径逐字;
/// clip.w ≤ 1e-20 整三角保守丢弃）。
fn project_tri(cam_vp: &[[f32; 4]; 4], world: &[[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let vp = Mat4 { m: *cam_vp };
    let (w_px, h_px) = (VIS_W as f32, VIS_H as f32);
    let mut screen = [[0.0f32; 3]; 3];
    for (k, sv) in screen.iter_mut().enumerate() {
        let w = world[k];
        let clip = vp.transform_vec4([w[0], w[1], w[2], 1.0]);
        if clip[3] <= 1e-20 {
            return None;
        }
        let inv_w = 1.0 / clip[3];
        let nx = clip[0] * inv_w;
        let ny = clip[1] * inv_w;
        let nz = (clip[2] * inv_w).clamp(0.0, 1.0);
        *sv = [(nx + 1.0) * 0.5 * w_px, (1.0 - ny) * 0.5 * h_px, nz];
    }
    Some(screen)
}

/// 全局簇 i 的第 t 个世界三角（base DAG 几何段 + 副本平移）。
fn cluster_world_tri(fx: &Fixture, i: usize, t: u32) -> [[f32; 3]; 3] {
    let n = fx.dag.records.len();
    let off = fx.copy_dx[i / n];
    let local = (i % n) as u32;
    let lt = fx.dag.cluster_triangle(local, t);
    let verts = fx.dag.cluster_vertices(local);
    let mut out = [[0.0f32; 3]; 3];
    for k in 0..3 {
        let v = verts[lt[k] as usize];
        out[k] = [v[0] + off[0], v[1] + off[1], v[2] + off[2]];
    }
    out
}

fn run_visbuffer_arm(sw_spv: &[u32], fx: &Fixture, final_visible: &[u32]) -> VisArmOut {
    // ── 分箱（host 金标准 compact_draw_args;32px 投影直径阈值）──
    let n = fx.dag.records.len();
    let all_records: Vec<ClusterRecord> = fx
        .copies
        .iter()
        .flat_map(|c| c.records.iter().copied())
        .collect();
    let instances: Vec<InstanceRecord> = fx
        .copies
        .iter()
        .map(|c| InstanceRecord {
            transform: IDENTITY_3X4,
            cluster_offset: c.offset,
            cluster_count: n as u32,
            material_id: 0,
            flags: 0,
            aabb_min: [-1e9; 3],
            mesh_id: 0,
            aabb_max: [1e9; 3],
            reserved: NO_PARENT,
        })
        .collect();
    let visible: Vec<VisibleCluster> = final_visible
        .iter()
        .map(|&i| VisibleCluster {
            instance: i / n as u32,
            cluster: i,
        })
        .collect();
    let args = compact_draw_args(
        &visible,
        &instances,
        &all_records,
        &fx.cam,
        DEFAULT_BIN_THRESHOLD_PX,
    );
    // ── 帧内可见项下标 = cluster27 载荷（Nanite 口径:可见列表下标,材质经
    //    列表反查;合成材质表 = 全局簇 id % 7 + 1）──
    let entry_of: std::collections::HashMap<u32, u32> = final_visible
        .iter()
        .enumerate()
        .map(|(e, &i)| (i, e as u32))
        .collect();
    let cluster_to_material: Vec<u16> = final_visible
        .iter()
        .map(|&i| (i % 7 + 1) as u16)
        .collect();
    // ── SW 箱:投影三角流 → device SW 软光栅（u64 原子）+ host oracle ──
    let mut sw_tris_f32: Vec<f32> = Vec::new();
    let mut sw_ids: Vec<u32> = Vec::new();
    let mut host_sw = VisBufferCpu::new(VIS_W, VIS_H);
    let mut sw_tri_count = 0u32;
    for vc in &args.sw_clusters {
        let entry = entry_of[&vc.cluster];
        let rec = &all_records[vc.cluster as usize];
        for t in 0..rec.triangle_count {
            let world = cluster_world_tri(fx, vc.cluster as usize, t);
            if let Some(screen) = project_tri(&fx.cam.view_proj, &world) {
                for sv in &screen {
                    sw_tris_f32.extend_from_slice(sv);
                }
                sw_ids.extend_from_slice(&[entry, t]);
                host_sw.raster_triangle(&screen, entry, t);
                sw_tri_count += 1;
            }
        }
    }
    // device SW 光栅（绑定:0 triangles f32[9t] / 1 ids u32[2t] / 2 vis
    // u64[W·H];push consts = tri_count/W/H;dispatch = tri·W·H 蛮力,
    // M95 判定式同构——生产 tile 化归 #75 后续）。
    let entry = vk::entry_point_name(sw_spv).unwrap_or_else(|| fail("SW SPV 无 OpEntryPoint"));
    let clear_bytes: Vec<u8> = std::iter::repeat_n(VISBUFFER_CLEAR.to_le_bytes(), (VIS_W * VIS_H) as usize)
        .flatten()
        .collect();
    let run_sw = || -> Vec<u64> {
        let mut bufs: Vec<Vec<u8>> = vec![
            bytes_f32(&sw_tris_f32),
            bytes_u32(&sw_ids),
            clear_bytes.clone(),
        ];
        vk::run_compute(
            sw_spv,
            &entry,
            &mut bufs,
            &bytes_u32(&[sw_tri_count, VIS_W, VIS_H]),
            [sw_tri_count.max(1) * VIS_W * VIS_H, 1, 1],
        )
        .unwrap_or_else(|e| fail(&format!("SW 软光栅 dispatch 失败: {e}")));
        bufs[2]
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
            .collect()
    };
    let dev_sw = run_sw();
    let dev_sw2 = run_sw();
    if dev_sw != dev_sw2 {
        fail("--visbuffer SW device 双跑位级漂移（u64 原子 max 序无关性破坏）");
    }
    // 覆盖集合对拍（M95 oracle 口径:host CPU 光栅 = 覆盖集合金标准;打包
    // 深度位受 FMA/ULP 限制不进零容差判据——M95 已锚 SW/HW device diff=0）。
    let cover = |v: &[u64]| -> Vec<bool> { v.iter().map(|&w| w != VISBUFFER_CLEAR).collect() };
    let dev_cover = cover(&dev_sw);
    let host_cover = cover(&host_sw.data);
    if dev_cover != host_cover {
        let miss = dev_cover
            .iter()
            .zip(&host_cover)
            .position(|(a, b)| a != b)
            .unwrap();
        fail(&format!(
            "--visbuffer SW 覆盖集合失配: 像素 {miss} device={} host={}",
            dev_cover[miss], host_cover[miss]
        ));
    }
    let sw_covered = dev_cover.iter().filter(|&&c| c).count() as u32;
    // ── HW 箱:host 保守金标准光栅（HW device 腿 = M95 门已锚 diff=0,
    //    本臂复用其结论诚实登记,不重跑图形管线）──
    let mut host_hw = VisBufferCpu::new(VIS_W, VIS_H);
    let mut hw_tri_count = 0u32;
    for vc in &args.hw_clusters {
        let entry = entry_of[&vc.cluster];
        let rec = &all_records[vc.cluster as usize];
        for t in 0..rec.triangle_count {
            let world = cluster_world_tri(fx, vc.cluster as usize, t);
            if let Some(screen) = project_tri(&fx.cam.view_proj, &world) {
                host_hw.raster_triangle(&screen, entry, t);
                hw_tri_count += 1;
            }
        }
    }
    let hw_covered = host_hw.data.iter().filter(|&&w| w != VISBUFFER_CLEAR).count() as u32;
    // ── 合并（u64 max = VisBuffer 原子语义,交换律顺序无关）→ classify/
    //    resolve（#111 材质分箱 host 金标准直调）──
    let merged = VisBufferCpu {
        w: VIS_W,
        h: VIS_H,
        data: dev_sw
            .iter()
            .zip(&host_hw.data)
            .map(|(&a, &b)| {
                // CLEAR 哨兵不参与 max（哨兵值域高于有效载荷——按"有效者胜/
                // 双有效取 max"合并）。
                match (a != VISBUFFER_CLEAR, b != VISBUFFER_CLEAR) {
                    (true, true) => a.max(b),
                    (true, false) => a,
                    (false, true) => b,
                    (false, false) => VISBUFFER_CLEAR,
                }
            })
            .collect(),
    };
    let merged_covered = merged.data.iter().filter(|&&w| w != VISBUFFER_CLEAR).count() as u32;
    let cls = classify(&merged, &cluster_to_material, 16);
    let resolved = resolve(&merged, &cluster_to_material);
    let resolved_pixels = resolved
        .iter()
        .filter(|&&m| m != rurix_render::geometry::material_pass::MATERIAL_INVALID)
        .count() as u32;
    if resolved_pixels != merged_covered {
        fail(&format!(
            "--visbuffer resolve 像素数 {resolved_pixels} ≠ 合并覆盖 {merged_covered}（材质解析面破坏）"
        ));
    }
    // 双跑确定性（classify/resolve 纯函数）。
    let cls2 = classify(&merged, &cluster_to_material, 16);
    if cls.buckets.len() != cls2.buckets.len() {
        fail("--visbuffer classify 双跑漂移");
    }
    VisArmOut {
        sw_clusters: args.sw_cluster_count,
        hw_clusters: args.hw_cluster_count,
        sw_tris: sw_tri_count,
        hw_tris: hw_tri_count,
        sw_covered,
        hw_covered,
        merged_covered,
        classify_tiles: (VIS_W.div_ceil(16)) * (VIS_H.div_ceil(16)),
        classify_buckets: cls.buckets.len() as u32,
        resolved_pixels,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut spv_path = String::from(".tmp/g31_gates/cluster_cull/g31_cluster_cull.spv");
    let mut evidence = String::new();
    let mut copies = 5usize;
    let mut threshold_px = 1.0f32;
    let mut red_arm: Option<String> = None;
    let mut visbuffer_arm = false;
    let mut i = 1;
    while i < args.len() {
        let take = |args: &[String], i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--spv" => spv_path = take(&args, &mut i),
            "--evidence" => evidence = take(&args, &mut i),
            "--copies" => {
                copies = take(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--copies 非 usize"))
            }
            "--error-threshold-px" => {
                threshold_px = take(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--error-threshold-px 非 f32"))
            }
            "--red-arm" => red_arm = Some(take(&args, &mut i)),
            // G31+ #74/#75/#111:cut → SW/HW 分箱 → SW compute 软光栅 device →
            // 覆盖对拍 → 合并 → classify/resolve 材质分箱机制链臂。
            "--visbuffer" => visbuffer_arm = true,
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    // 三态：无 Vulkan / SPV → skipped_dev_env 退 0。
    if !vk::vulkan_available() {
        println!("{TAG}: {{\"state\":\"skipped_dev_env\",\"reason\":\"vulkan loader 不可用\"}}");
        if std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1") {
            fail("RURIX_REQUIRE_REAL=1 但 vulkan 不可用");
        }
        return;
    }
    if !Path::new(&spv_path).is_file() {
        println!("{TAG}: {{\"state\":\"skipped_dev_env\",\"reason\":\"SPV 缺失 {spv_path}\"}}");
        if std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1") {
            fail("RURIX_REQUIRE_REAL=1 但 SPV 缺失");
        }
        return;
    }
    let spv = spv_inject_no_contraction(&load_spv(Path::new(&spv_path)));
    let entry = vk::entry_point_name(&spv).unwrap_or_else(|| fail("SPV 无 OpEntryPoint"));

    let fx = build_fixture(copies, threshold_px);
    let hzb_prev = HzbPyramid::build(&fx.depth_prev, DepthConvention::ReverseZ);
    let hzb_curr = HzbPyramid::build(&fx.depth_curr, DepthConvention::ReverseZ);
    let pyr_prev = flatten_pyramid(&hzb_prev);
    let pyr_curr = flatten_pyramid(&hzb_curr);
    eprintln!(
        "{TAG}: 夹具就绪 copies={copies} clusters={} threshold_px={threshold_px} depth={DEPTH_W}x{DEPTH_H} hzb_levels={}",
        fx.total_clusters, pyr_prev.levels,
    );

    // ── device 两阶段（双跑位级）──
    let (p1a, p2a) = run_two_pass(&spv, &entry, &fx, &pyr_prev, &pyr_curr, threshold_px);
    let (p1b, p2b) = run_two_pass(&spv, &entry, &fx, &pyr_prev, &pyr_curr, threshold_px);
    if p1a.overflow != 0 || p2a.overflow != 0 {
        fail("列表容量溢出（夹具容量面破坏）");
    }
    if p1a.decisions != p1b.decisions || p2a.decisions != p2b.decisions {
        fail("device 双跑判定序列漂移（确定性破坏）");
    }
    let sorted = |v: &[u32]| -> Vec<u32> {
        let mut s = v.to_vec();
        s.sort_unstable();
        s
    };
    if sorted(&p1a.visible) != sorted(&p1b.visible) || sorted(&p2a.visible) != sorted(&p2b.visible)
    {
        fail("device 双跑可见集漂移（确定性破坏）");
    }

    // ── host 金标准复算 ──
    let cut_flags = host_cut_flags(&fx);
    let host_p1: Vec<u32> = (0..fx.total_clusters)
        .map(|i| host_decision(&fx, i, cut_flags[i], &hzb_prev))
        .collect();
    if p1a.decisions != host_p1 {
        let miss = p1a
            .decisions
            .iter()
            .zip(&host_p1)
            .position(|(a, b)| a != b)
            .unwrap();
        fail(&format!(
            "pass1 判定序列失配: 簇 {miss} device={} host={}",
            p1a.decisions[miss], host_p1[miss]
        ));
    }
    let mut occ_sorted = p1a.occluded.clone();
    occ_sorted.sort_unstable();
    let host_p2: Vec<u32> = occ_sorted
        .iter()
        .map(|&i| host_decision(&fx, i as usize, cut_flags[i as usize], &hzb_curr))
        .collect();
    if p2a.decisions != host_p2 {
        let miss = p2a
            .decisions
            .iter()
            .zip(&host_p2)
            .position(|(a, b)| a != b)
            .unwrap();
        fail(&format!(
            "pass2 判定序列失配: 输入项 {miss}（簇 {}）device={} host={}",
            occ_sorted[miss], p2a.decisions[miss], host_p2[miss]
        ));
    }
    // 最终可见集全等（pass1 ∪ pass2）。
    let mut dev_visible = p1a.visible.clone();
    dev_visible.extend_from_slice(&p2a.visible);
    dev_visible.sort_unstable();
    let mut host_visible: Vec<u32> = (0..fx.total_clusters as u32)
        .filter(|&i| host_p1[i as usize] == 4)
        .collect();
    host_visible.extend(
        occ_sorted
            .iter()
            .zip(&host_p2)
            .filter(|&(_, &d)| d == 4)
            .map(|(&i, _)| i),
    );
    host_visible.sort_unstable();
    if dev_visible != host_visible {
        fail(&format!(
            "最终可见集失配: device {} 项 vs host {} 项",
            dev_visible.len(),
            host_visible.len()
        ));
    }

    // ── 判据 ④:剔除与 disocclusion 真实发生 ──
    let final_occluded: Vec<u32> = occ_sorted
        .iter()
        .zip(&p2a.decisions)
        .filter(|&(_, &d)| d == 3)
        .map(|(&i, _)| i)
        .collect();
    if final_occluded.is_empty() {
        fail("剔除数为零（夹具遮挡面失效,防空接线）");
    }
    if p2a.visible.is_empty() {
        fail("pass2 新增可见为零（disocclusion 面失效——两阶段第二段无存在性证明）");
    }

    // ── 判据 ③:零假阳性（exact 裁判,本帧深度逐像素精确真值）──
    for &i in &final_occluded {
        let Some((mn, mx, nearest)) = sphere_rect_depth(&fx, i as usize) else {
            fail(&format!("被剔簇 {i} 近平面骑跨（不应进遮挡箱）"));
        };
        if !exact_rect_occluded(&fx.depth_curr, DepthConvention::ReverseZ, mn, mx, nearest) {
            fail(&format!("零假阳性破坏: 簇 {i} 被剔但精确真值可见"));
        }
    }

    // ── RED 臂（--red-arm tamper）:篡改本帧金字塔单纹素 ⇒ 判定必变 ──
    let mut red_note = String::from("null");
    if red_arm.as_deref() == Some("tamper") {
        // 取首个被剔簇的 rect 中心纹素,拉远(reverse-Z 减小)→ 遮挡判定翻可见。
        let victim = final_occluded[0] as usize;
        let (mn, mx, _) = sphere_rect_depth(&fx, victim).unwrap();
        let cxpx = (((mn[0] + mx[0]) * 0.5) * DEPTH_W as f32) as u32;
        let cypx = (((mn[1] + mx[1]) * 0.5) * DEPTH_H as f32) as u32;
        let tampered_depth = ImageF32::from_fn(DEPTH_W, DEPTH_H, 1, |x, y, _| {
            if x == cxpx.min(DEPTH_W - 1) && y == cypx.min(DEPTH_H - 1) {
                0.001 // 拉远单纹素（reverse-Z 更小 = 更远 ⇒ 保守测试翻可见）
            } else {
                fx.depth_curr.get(x, y, 0)
            }
        });
        let hzb_tampered = HzbPyramid::build(&tampered_depth, DepthConvention::ReverseZ);
        let pyr_tampered = flatten_pyramid(&hzb_tampered);
        let p2t = run_pass(&spv, &entry, &fx, &pyr_tampered, &occ_sorted, threshold_px, 1);
        if p2t.decisions == p2a.decisions {
            fail("RED 臂失效: 篡改金字塔后判定序列不变（消费路径未命中）");
        }
        red_note = format!(
            "{{\"victim\":{victim},\"texel\":[{cxpx},{cypx}],\"flipped\":true}}"
        );
        eprintln!("{TAG}: RED 臂 OK（篡改单纹素 → 判定序列必变,消费路径命中）");
    }

    // ── --visbuffer 臂（#74/#75/#111 机制链;dev_visible = 两阶段最终可见集）──
    let mut vis_note = String::from("null");
    if visbuffer_arm {
        let sw_spv = sw_visbuffer_u64_spv();
        let out = run_visbuffer_arm(&sw_spv, &fx, &dev_visible);
        if out.sw_clusters == 0 || out.hw_clusters == 0 {
            fail(&format!(
                "--visbuffer 分箱空转: sw={} hw={}（近/远两组副本夹具应两箱皆非空）",
                out.sw_clusters, out.hw_clusters
            ));
        }
        if out.sw_covered == 0 {
            fail("--visbuffer SW device 光栅零覆盖（防空接线）");
        }
        if out.classify_buckets == 0 {
            fail("--visbuffer classify 零桶（材质分箱面空转）");
        }
        vis_note = format!(
            "{{\"resolution\":[{VIS_W},{VIS_H}],\"bin_threshold_px\":{DEFAULT_BIN_THRESHOLD_PX},\"sw_clusters\":{},\"hw_clusters\":{},\"sw_tris\":{},\"hw_tris\":{},\"sw_covered\":{},\"hw_covered\":{},\"merged_covered\":{},\"classify_tiles\":{},\"classify_buckets\":{},\"resolved_pixels\":{},\"sw_device_coverage_match\":true,\"sw_double_run_bitexact\":true}}",
            out.sw_clusters,
            out.hw_clusters,
            out.sw_tris,
            out.hw_tris,
            out.sw_covered,
            out.hw_covered,
            out.merged_covered,
            out.classify_tiles,
            out.classify_buckets,
            out.resolved_pixels,
        );
        eprintln!(
            "{TAG}: visbuffer 臂 OK sw_clusters={} hw_clusters={} sw_tris={} hw_tris={} covered(sw/hw/merged)={}/{}/{} buckets={} resolved={}（SW device 覆盖与 host oracle 全等 + 双跑位级;HW device 腿 = M95 门已锚 diff=0 复用登记）",
            out.sw_clusters,
            out.hw_clusters,
            out.sw_tris,
            out.hw_tris,
            out.sw_covered,
            out.hw_covered,
            out.merged_covered,
            out.classify_buckets,
            out.resolved_pixels,
        );
    }

    // ── evidence ──
    let cut_total = cut_flags.iter().filter(|&&f| f).count();
    let summary = format!(
        "{{\"schema\":\"rurix.g31.cluster_cull_device.v1\",\"clusters\":{},\"copies\":{copies},\"threshold_px\":{threshold_px},\"cut_clusters\":{cut_total},\"pass1_visible\":{},\"pass1_occluded\":{},\"pass2_reveal\":{},\"final_occluded\":{},\"decisions_match\":true,\"visible_set_match\":true,\"zero_false_positive\":true,\"double_run_bitexact\":true,\"disocclusion_present\":true,\"red_arm\":{},\"visbuffer\":{},\"spv\":{},\"note\":\"两阶段簇级 HZB(上帧初剔+本帧重测,Haar&Aaltonen 2015/Nanite 语义)device 腿;金标准 = cull.rs 三关 + select_lod_cut_grouped(组共享判定球) + HzbPyramid::test_rect 直调;合成夹具(uv_sphere quality DAG ×{copies} 副本 + 移动墙 disocclusion);--visbuffer = cut→SW/HW 分箱→SW compute 软光栅 device(M95 u64 原子腿)→覆盖对拍→classify/resolve 机制链;生产接线(g31 会话 pass 序)归后续波\"}}",
        fx.total_clusters,
        p1a.visible.len(),
        p1a.occluded.len(),
        p2a.visible.len(),
        final_occluded.len(),
        red_note,
        vis_note,
        jstr(&spv_path),
    );
    if !evidence.is_empty() {
        if let Some(parent) = Path::new(&evidence).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&evidence, summary.as_bytes())
            .unwrap_or_else(|e| fail(&format!("evidence 写盘: {e}")));
    }
    println!(
        "{TAG}: PASS clusters={} cut={cut_total} p1_vis={} p1_occ={} p2_reveal={} final_occ={} （判定序列全等 + 可见集全等 + 零假阳性 + 双跑位级 + disocclusion 在场{}）",
        fx.total_clusters,
        p1a.visible.len(),
        p1a.occluded.len(),
        p2a.visible.len(),
        final_occluded.len(),
        if red_arm.is_some() { " + RED 臂" } else { "" },
    );
}
