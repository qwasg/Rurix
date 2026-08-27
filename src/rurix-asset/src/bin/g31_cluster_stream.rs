// Assisted-by: Kimi-K3（G31+ 波 C Task C11 cluster 流送 P4 四行 + 整合真跑）
//! G31+ 波 C Task C11 cluster 流送 harness（门 `g31.waveC.p4stream`；RD-039
//! cluster 流送 P4 四行承接面；G31_PLUS_COMMERCIAL_RENDERER_TODO §3 #20~#23）。
//!
//! 链：bistro glTF（`rurix-asset::gltf` 严格导入）→ 三角预算内子集（真实
//! bistro 派生场景）→ `build_asset_dag` 簇层级 DAG → `pack_cluster_dag_v2`
//! RXPL v2 页装箱 → RXPD major=2 页集真实落盘（`disk_v2` 加性新面）→
//! `kernels/g31_cluster_stream.rx`（rurixc 产 SPV + bin 侧 NoContraction 注入，
//! SPV 文件 0-byte）经 `vk::run_compute` 逐帧真跑——剔除 pass 产缺页请求
//! （device 请求缓冲读回）→ host 驻留调度（`PriorityIoPool` 异步优先级读 +
//! `StreamingEngine` 三预算 tick + LRU 驻留池）→ 页表/页池镜像上传 → 次帧
//! device 消费校验（页槽首字 checksum）闭环。
//!
//! 双臂：
//! - **reference**（全驻留参考，×2 双跑位级硬门）：池 ≥ 全集，逐帧 digest_seq；
//! - **stream**（强制小驻留池压力臂，冷启动）：池 < 全集，逐帧缺页/回退/
//!   IO/上传 measured；host 金标准逐帧对拍（`lod_cut_with_residency` 归一后
//!   集合全等 + `verify_cut_cover` 覆盖不变量）+ hold 段收敛后 digest 与
//!   reference 逐帧位级一致（回退帧允许 LOD 差，驻留完整帧位级——结构容差
//!   依据 = 一致性 cut 语义）。
//!
//! 三态：无 Vulkan loader/gltf/SPV → `skipped_dev_env` 退 0（不冒充 PASS）。
//!
//! 用法：
//!   g31_cluster_stream --gltf <bistro.gltf> --spv <g31_cluster_stream.spv> \
//!     --pages-dir <dir> --evidence <path> [--orbit-frames N] [--hold-frames N] \
//!     [--pool-slots N] [--max-meshes N] [--max-tris N] [--budget-pages N] \
//!     [--io-workers N] [--error-threshold-px F]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use rurix_asset::geom_build_v2::{check_v2_pages_within_contract, pack_cluster_dag_v2};
use rurix_asset::gltf;
use rurix_asset::gltf::validate::ImportOptions;
use rurix_geom_build::{DagAsset, TriMesh, build_asset_dag};
use rurix_geom_pages::{encode_disk_page_v2, encode_logical_page_v2};
use rurix_render::geometry::cull::{CullCamera, Frustum, VisibleCluster};
use rurix_render::geometry::gpu_scene::{
    IDENTITY_3X4, InstanceRecord, NO_PARENT, compose_transform, transform_point,
};
use rurix_render::graph::types::{ClusterRecord, StreamingBudget};
use rurix_render::streaming::cluster::{
    ClusterPageResource, PageBinding, PriorityIoPool, cluster_page_file_name,
    lod_cut_with_residency, normalize_render_decisions, verify_cut_cover,
};
use rurix_render::streaming::{FEEDBACK_BASE_GEOMETRY_LOD, StreamingEngine};
use rurix_render::temporal::common::{look_at_rh, perspective_rh_zo};
use rurix_rt::vk;

const EVIDENCE_SCHEMA: &str = "rurix.g31.cluster_stream_evidence.v1";
const WORDS_PER_PAGE: usize = 32768; // 128KB / 4
const SCREEN_H: f32 = 1080.0;

// ---------------------------------------------------------------------------
// 小工具
// ---------------------------------------------------------------------------

fn fail(msg: &str) -> ! {
    eprintln!("[g31_cluster_stream]: FAIL {msg}");
    std::process::exit(1)
}

fn sha256_hex(data: &[u8]) -> String {
    let d = rurix_pkg::sha256::digest(data);
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
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
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("SPV 读取失败 {path:?}: {e}")));
    if bytes.len() % 4 != 0 {
        fail("SPV 字节数非 4 对齐");
    }
    read_u32(&bytes)
}

/// SPIR-V NoContraction 注入（g14_3_lane_body 同律 bin 侧注入：禁驱动 FMA
/// 收缩/重关联，保门形逐 op IEEE 位级；SPV 文件 0-byte）。
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
        .unwrap_or_else(|| fail("SPIR-V 无 annotation/type 段锚（NoContraction 注入）"));
    let mut out = Vec::with_capacity(spv.len() + result_ids.len() * 3);
    out.extend_from_slice(&spv[..at]);
    for id in &result_ids {
        out.push(71u32 | (3 << 16));
        out.push(*id);
        out.push(42);
    }
    out.extend_from_slice(&spv[at..]);
    out
}

fn jstr(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// 节点世界变换（glTF TRS/matrix → 3×4 行主）
// ---------------------------------------------------------------------------

fn quat_to_mat3(q: [f32; 4]) -> [[f32; 3]; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let n = (x * x + y * y + z * z + w * w).sqrt().max(1e-12);
    let (x, y, z, w) = (x / n, y / n, z / n, w / n);
    [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w)],
        [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w)],
        [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y)],
    ]
}

fn f32_arr(v: Option<&gltf::json::JsonValue>, n: usize, default: &[f32]) -> Vec<f32> {
    v.and_then(|v| v.as_array())
        .map(|a| {
            (0..n)
                .map(|i| {
                    a.get(i)
                        .and_then(|x| x.as_f64())
                        .map(|x| x as f32)
                        .unwrap_or(default[i])
                })
                .collect()
        })
        .unwrap_or_else(|| default.to_vec())
}

fn node_local_3x4(node: &gltf::json::JsonValue) -> [[f32; 4]; 3] {
    if let Some(mv) = node.get("matrix") {
        let m = f32_arr(Some(mv), 16, &[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
        // glTF matrix = 列主 16 → 行主 3×4。
        return [
            [m[0], m[4], m[8], m[12]],
            [m[1], m[5], m[9], m[13]],
            [m[2], m[6], m[10], m[14]],
        ];
    }
    let t = f32_arr(node.get("translation"), 3, &[0.0, 0.0, 0.0]);
    let q = f32_arr(node.get("rotation"), 4, &[0.0, 0.0, 0.0, 1.0]);
    let s = f32_arr(node.get("scale"), 3, &[1.0, 1.0, 1.0]);
    let r = quat_to_mat3([q[0], q[1], q[2], q[3]]);
    [
        [r[0][0] * s[0], r[0][1] * s[1], r[0][2] * s[2], t[0]],
        [r[1][0] * s[0], r[1][1] * s[1], r[1][2] * s[2], t[1]],
        [r[2][0] * s[0], r[2][1] * s[1], r[2][2] * s[2], t[2]],
    ]
}

/// bistro 节点世界变换提取（bin-local：gltf::json 严格解析器消费；mesh_id →
/// 世界 3×4，首节点胜；多实例引用计数登记）。
fn extract_mesh_transforms(gltf_path: &Path) -> (HashMap<u32, [[f32; 4]; 3]>, u32) {
    let text = std::fs::read_to_string(gltf_path)
        .unwrap_or_else(|e| fail(&format!("glTF 文本读取失败: {e}")));
    let root = gltf::json::parse_str(&text)
        .unwrap_or_else(|e| fail(&format!("glTF JSON 解析失败: {e}")));
    let empty: Vec<gltf::json::JsonValue> = Vec::new();
    let nodes = root
        .get("nodes")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty);
    let mut world: Vec<[[f32; 4]; 3]> = vec![IDENTITY_3X4; nodes.len()];
    fn walk(
        id: usize,
        parent: &[[f32; 4]; 3],
        nodes: &[gltf::json::JsonValue],
        world: &mut Vec<[[f32; 4]; 3]>,
    ) {
        let n = &nodes[id];
        let w = compose_transform(parent, &node_local_3x4(n));
        world[id] = w;
        if let Some(children) = n.get("children").and_then(|v| v.as_array()) {
            for c in children {
                if let Some(ci) = c.as_u32() {
                    walk(ci as usize, &w, nodes, world);
                }
            }
        }
    }
    if let Some(scenes) = root.get("scenes").and_then(|v| v.as_array()) {
        for s in scenes {
            if let Some(roots) = s.get("nodes").and_then(|v| v.as_array()) {
                for r in roots {
                    if let Some(ri) = r.as_u32() {
                        walk(ri as usize, &IDENTITY_3X4, nodes, &mut world);
                    }
                }
            }
        }
    }
    let mut mesh_transform: HashMap<u32, [[f32; 4]; 3]> = HashMap::new();
    let mut multi = 0u32;
    for (id, n) in nodes.iter().enumerate() {
        if let Some(m) = n.get("mesh").and_then(|v| v.as_u32()) {
            if mesh_transform.insert(m, world[id]).is_some() {
                multi += 1;
            }
        }
    }
    (mesh_transform, multi)
}

// ---------------------------------------------------------------------------
// 场景构建（bistro 派生：真实几何子集 + 簇 DAG + RXPD v2 页集）
// ---------------------------------------------------------------------------

struct MeshUnit {
    resource: u32,
    mesh_id: u32,
    primitive_id: u32,
    triangles: u32,
    cluster_offset: u32,
    cluster_count: u32,
    page_count: u32,
    root_pages: Vec<u32>,
    aabb_min: [f32; 3],
    aabb_max: [f32; 3],
}

struct BuiltScene {
    units: Vec<MeshUnit>,
    clusters: Vec<ClusterRecord>,
    bindings: Vec<PageBinding>,
    instances: Vec<InstanceRecord>,
    resource_page_base: Vec<u32>,
    total_pages: u32,
    total_clusters: u32,
    total_tris: u64,
    /// root 页全集（钉住面；池定纲 root/流动分项源）。
    root_page_keys: HashSet<(u32, u32)>,
    page_set_digest: String,
    scene_center: [f32; 3],
    scene_radius: f32,
    mesh_build_ms: f64,
}

fn object_aabb(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut mn = [f32::INFINITY; 3];
    let mut mx = [f32::NEG_INFINITY; 3];
    for p in positions {
        for a in 0..3 {
            mn[a] = mn[a].min(p[a]);
            mx[a] = mx[a].max(p[a]);
        }
    }
    (mn, mx)
}

fn world_aabb_of(t: &[[f32; 4]; 3], lo: [f32; 3], hi: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let mut mn = [f32::INFINITY; 3];
    let mut mx = [f32::NEG_INFINITY; 3];
    for c in 0..8 {
        let p = [
            if c & 1 == 0 { lo[0] } else { hi[0] },
            if c & 2 == 0 { lo[1] } else { hi[1] },
            if c & 4 == 0 { lo[2] } else { hi[2] },
        ];
        let w = transform_point(t, p);
        for a in 0..3 {
            mn[a] = mn[a].min(w[a]);
            mx[a] = mx[a].max(w[a]);
        }
    }
    (mn, mx)
}

fn build_scene(
    gltf_path: &Path,
    pages_dir: &Path,
    max_meshes: usize,
    max_tris: u64,
) -> BuiltScene {
    let t0 = Instant::now();
    let imported = gltf::import_path(gltf_path, &ImportOptions::default())
        .unwrap_or_else(|e| fail(&format!("glTF 导入失败 {gltf_path:?}: {e}")));
    // mesh_id → 节点世界变换（bin-local JSON 面；首节点胜；多实例引用登记计数）。
    let (mesh_transform, multi_instance_meshes) = extract_mesh_transforms(gltf_path);
    // 候选图元（三角数降序、id 升序 tie-break——稳定确定性）。
    let mut cands: Vec<&gltf::validate::ImportedMesh> = imported
        .meshes
        .iter()
        .filter(|m| m.indices.len() >= 12)
        .collect();
    cands.sort_by_key(|m| {
        (
            std::cmp::Reverse(m.indices.len() as u64 / 3),
            m.mesh_id,
            m.primitive_id,
        )
    });
    let mut selected: Vec<&gltf::validate::ImportedMesh> = Vec::new();
    let mut tri_budget = 0u64;
    for c in cands {
        if selected.len() >= max_meshes {
            break;
        }
        let tris = c.indices.len() as u64 / 3;
        if tri_budget + tris > max_tris && !selected.is_empty() {
            continue;
        }
        tri_budget += tris;
        selected.push(c);
    }
    if selected.is_empty() {
        fail("bistro 子集为空（max_meshes/max_tris 约束过紧）");
    }

    let mut out = BuiltScene {
        units: Vec::new(),
        clusters: Vec::new(),
        bindings: Vec::new(),
        instances: Vec::new(),
        resource_page_base: Vec::new(),
        total_pages: 0,
        total_clusters: 0,
        total_tris: 0,
        root_page_keys: HashSet::new(),
        page_set_digest: String::new(),
        scene_center: [0.0; 3],
        scene_radius: 1.0,
        mesh_build_ms: 0.0,
    };
    std::fs::create_dir_all(pages_dir).unwrap_or_else(|e| fail(&format!("pages-dir 创建失败: {e}")));
    let mut page_set_bytes: Vec<u8> = Vec::new();
    for (mi, imp) in selected.iter().enumerate() {
        let resource = mi as u32;
        let mesh = TriMesh {
            positions: imp.positions.clone(),
            indices: imp.indices.clone(),
        };
        let dag = build_asset_dag(&DagAsset::static_mesh(mesh))
            .unwrap_or_else(|e| fail(&format!("mesh {} DAG 构建失败: {e}", imp.mesh_id)));
        let pages = pack_cluster_dag_v2(&dag)
            .unwrap_or_else(|e| fail(&format!("mesh {} 页装箱失败: {e}", imp.mesh_id)));
        check_v2_pages_within_contract(&pages)
            .unwrap_or_else(|e| fail(&format!("mesh {} 页契约超限: {e}", imp.mesh_id)));
        // 页落盘（RXPD v2；page_id = 装箱序 0..n）。
        let mut root_pages = Vec::new();
        let mut cluster_page = vec![u32::MAX; dag.base.records.len()];
        for (pi, page) in pages.iter().enumerate() {
            debug_assert_eq!(page.base.page_id, pi as u64);
            for c in &page.base.clusters {
                cluster_page[c.cluster_id as usize] = pi as u32;
            }
            if page.base.is_root() {
                root_pages.push(pi as u32);
                out.root_page_keys.insert((resource, pi as u32));
            }
            let bytes = encode_disk_page_v2(page);
            page_set_bytes.extend_from_slice(&bytes);
            let path = pages_dir.join(cluster_page_file_name(resource, pi as u32));
            std::fs::write(&path, &bytes)
                .unwrap_or_else(|e| fail(&format!("页落盘失败 {path:?}: {e}")));
        }
        // 页 payload ≤128KB 契约机器复核（入池 payload = RXPL v2 映像）。
        for page in &pages {
            debug_assert!(encode_logical_page_v2(page).len() <= WORDS_PER_PAGE * 4);
        }
        // 父链（DAG children → parent；root = NO_PARENT）。
        let cluster_offset = out.clusters.len() as u32;
        let mut parent = vec![NO_PARENT; dag.base.records.len()];
        for id in 0..dag.base.records.len() as u32 {
            for &c in dag.base.children_of(id) {
                parent[c as usize] = id;
            }
        }
        for (ci, rec) in dag.base.records.iter().enumerate() {
            out.clusters.push(*rec);
            out.bindings.push(PageBinding {
                resource,
                page: cluster_page[ci],
                parent: if parent[ci] == NO_PARENT {
                    NO_PARENT
                } else {
                    parent[ci] + cluster_offset
                },
            });
        }
        // 实例（bistro 节点变换；无节点 = 单位阵如实登记）。
        let transform = mesh_transform
            .get(&imp.mesh_id)
            .copied()
            .unwrap_or(IDENTITY_3X4);
        let (omin, omax) = object_aabb(&imp.positions);
        let (wmin, wmax) = world_aabb_of(&transform, omin, omax);
        let cluster_count = dag.base.records.len() as u32;
        out.instances.push(InstanceRecord {
            transform,
            cluster_offset,
            cluster_count,
            material_id: 0,
            flags: 0,
            aabb_min: wmin,
            mesh_id: imp.mesh_id,
            aabb_max: wmax,
            reserved: NO_PARENT,
        });
        out.resource_page_base.push(out.total_pages);
        out.total_pages += pages.len() as u32;
        out.total_tris += (imp.indices.len() / 3) as u64;
        out.units.push(MeshUnit {
            resource,
            mesh_id: imp.mesh_id,
            primitive_id: imp.primitive_id,
            triangles: (imp.indices.len() / 3) as u32,
            cluster_offset,
            cluster_count,
            page_count: pages.len() as u32,
            root_pages,
            aabb_min: wmin,
            aabb_max: wmax,
        });
        out.total_clusters += cluster_count;
    }
    out.page_set_digest = format!("sha256:{}", sha256_hex(&page_set_bytes));
    // 场景 AABB（选中网格世界 AABB 并集）→ 轨道中心/半径。
    let mut mn = [f32::INFINITY; 3];
    let mut mx = [f32::NEG_INFINITY; 3];
    for u in &out.units {
        for a in 0..3 {
            mn[a] = mn[a].min(u.aabb_min[a]);
            mx[a] = mx[a].max(u.aabb_max[a]);
        }
    }
    for a in 0..3 {
        out.scene_center[a] = (mn[a] + mx[a]) * 0.5;
    }
    let d = [mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]];
    out.scene_radius = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() * 0.5;
    out.mesh_build_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let _ = multi_instance_meshes;
    out
}

// ---------------------------------------------------------------------------
// 相机轨迹（bistro 派生场景轨道 + hold 收敛段；全臂同参确定性）
// ---------------------------------------------------------------------------

/// 轨迹结构（穿越式——流送压力的形状来源）：orbit 段相机沿场景内环道
/// 平移（环半径 0.45×场景半径）且视线朝行进切线——视锥只切前方子场景
/// （后方网格页整页离锥 = 工作集稀疏化;相机推进持续把新区块拉入视锥
/// = 缺页/请求/逐出压力）;hold 段定格末位姿（工作集稳态,池 ≥ hold 工
/// 作集保证收敛;容量 < 全集字面成立）。
fn camera_at(scene: &BuiltScene, frame: u32, orbit_frames: u32, threshold: f32) -> CullCamera {
    let yaw_f = if frame >= orbit_frames {
        orbit_frames.saturating_sub(1)
    } else {
        frame
    };
    let yaw = 2.0 * std::f32::consts::PI * yaw_f as f32 / orbit_frames.max(1) as f32;
    let r = scene.scene_radius * 0.45;
    let eye = [
        scene.scene_center[0] + r * yaw.cos(),
        scene.scene_center[1] + scene.scene_radius * 0.10,
        scene.scene_center[2] + r * yaw.sin(),
    ];
    // 切线朝前（行进方向）+ 适度前瞻距离。
    let look = [
        eye[0] - yaw.sin() * scene.scene_radius * 0.8,
        eye[1] - scene.scene_radius * 0.05,
        eye[2] + yaw.cos() * scene.scene_radius * 0.8,
    ];
    let view = look_at_rh(eye, look, [0.0, 1.0, 0.0]);
    let proj = perspective_rh_zo(60.0_f32.to_radians(), 16.0 / 9.0, 0.1, 10_000.0);
    let vp = proj.mul(&view);
    CullCamera {
        view_proj: vp.m,
        cam_pos: eye,
        screen_height_px: SCREEN_H,
        error_threshold_px: threshold,
    }
}

// ---------------------------------------------------------------------------
// device 臂（run_compute 逐帧真跑 + 读回归一）
// ---------------------------------------------------------------------------

struct DeviceSide {
    spv: Vec<u32>,
    entry: String,
}

struct FrameDeviceOut {
    requests: Vec<(u32, u32, u32, u32)>, // resource, page, importance, cluster
    render: Vec<(u32, u32, u32, u32)>,   // instance, render, selected, fell_back
    checksum: [u32; 2],
    req_overflow: u32,
}

impl DeviceSide {
    fn create(spv_path: &Path) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let spv = spv_inject_no_contraction(&load_spv(spv_path));
        let entry = vk::entry_point_name(&spv).ok_or("SPV 无 OpEntryPoint")?;
        Ok(Self { spv, entry })
    }

    #[allow(clippy::too_many_arguments)]
    fn run_frame(
        &self,
        scene: &BuiltScene,
        cam: &CullCamera,
        page_state: &[u32],
        pool_bytes: &[u8],
        cluster_f32: &[f32],
        cluster_u32: &[u32],
        instances_f32: &[f32],
        instance_range: &[u32],
        max_depth: u32,
    ) -> FrameDeviceOut {
        let frustum = Frustum::from_view_proj(&cam.view_proj);
        let mut params = Vec::with_capacity(40);
        for p in &frustum.planes {
            params.extend_from_slice(p);
        }
        params.extend_from_slice(&cam.cam_pos);
        params.push(cam.view_proj[1][1] * SCREEN_H * 0.5);
        params.push(cam.error_threshold_px);
        params.push(scene.total_clusters as f32);
        params.push(max_depth as f32);
        params.push(scene.total_clusters.max(1) as f32); // request_capacity
        params.push(WORDS_PER_PAGE as f32);
        params.extend_from_slice(&[0.0; 5]);
        debug_assert_eq!(params.len(), 40);

        let req_cap = scene.total_clusters.max(1) as usize;
        let mut bufs = vec![
            bytes_f32(&params),
            bytes_f32(instances_f32),
            bytes_u32(instance_range),
            bytes_f32(cluster_f32),
            bytes_u32(cluster_u32),
            bytes_u32(&scene.resource_page_base),
            bytes_u32(page_state),
            pool_bytes.to_vec(),
            vec![0u8; 8],
            vec![0u8; req_cap * 16],
            vec![0u8; 8],
            vec![0u8; scene.total_clusters.max(1) as usize * 16],
            vec![0u8; 8],
        ];
        vk::run_compute(
            &self.spv,
            &self.entry,
            &mut bufs,
            &[],
            [scene.total_clusters.max(1), 1, 1],
        )
        .unwrap_or_else(|e| fail(&format!("cluster_stream dispatch 失败: {e}")));
        let req_cnt = read_u32(&bufs[8]);
        let reqs_raw = read_u32(&bufs[9]);
        let ren_cnt = read_u32(&bufs[10]);
        let ren_raw = read_u32(&bufs[11]);
        let checksum = read_u32(&bufs[12]);
        let emitted = (req_cnt[0] as usize).min(req_cap);
        let mut requests = Vec::with_capacity(emitted);
        for s in 0..emitted {
            requests.push((
                reqs_raw[s * 4],
                reqs_raw[s * 4 + 1],
                reqs_raw[s * 4 + 2],
                reqs_raw[s * 4 + 3],
            ));
        }
        let chains = ren_cnt[0] as usize;
        let mut render = Vec::with_capacity(chains);
        for s in 0..chains.min(scene.total_clusters as usize) {
            render.push((
                ren_raw[s * 4],
                ren_raw[s * 4 + 1],
                ren_raw[s * 4 + 2],
                ren_raw[s * 4 + 3],
            ));
        }
        FrameDeviceOut {
            requests,
            render,
            checksum: [checksum[0], checksum[1]],
            req_overflow: req_cnt[1],
        }
    }
}

// ---------------------------------------------------------------------------
// 臂公共面
// ---------------------------------------------------------------------------

struct FrameRecord {
    frame: u32,
    digest: String,
    selected: u32,
    /// 选中簇去重页数（驻留工作集页维上界；池容量定纲面）。
    selected_pages: u32,
    /// 选中簇去重流动页数（非 root；流动槽定纲面）。
    selected_flow_pages: u32,
    miss_selected: u32,
    fallback: u32,
    device_requests: u32,
    pages_loaded: u32,
    pages_evicted: u32,
    bytes_io: u64,
    bytes_upload: u64,
    queue_depth: u32,
    resident: u32,
    checksum0: u32,
    checksum1: u32,
    parity_ok: bool,
    cover_ok: bool,
    io_wait_ms: f64,
}

struct ArmResult {
    digest_seq: Vec<String>,
    records: Vec<FrameRecord>,
    io_bytes_total: u64,
    upload_bytes_total: u64,
    miss_frames: u32,
    fallback_frames: u32,
    parity_all: bool,
    cover_all: bool,
    resident_final: u32,
    pool_slots: u32,
    evicted_total: u32,
    req_overflow_total: u32,
}

struct StaticTables {
    cluster_f32: Vec<f32>,
    cluster_u32: Vec<u32>,
    instances_f32: Vec<f32>,
    instance_range: Vec<u32>,
}

fn build_static_tables(scene: &BuiltScene) -> StaticTables {
    let mut cluster_f32 = Vec::with_capacity(scene.clusters.len() * 10);
    let mut cluster_u32 = Vec::with_capacity(scene.clusters.len() * 4);
    for (i, c) in scene.clusters.iter().enumerate() {
        cluster_f32.extend_from_slice(&c.center);
        cluster_f32.push(c.radius);
        cluster_f32.extend_from_slice(&c.cone_axis);
        cluster_f32.push(c.cone_cutoff);
        cluster_f32.push(c.error);
        cluster_f32.push(c.parent_error);
        let b = &scene.bindings[i];
        cluster_u32.push(b.resource);
        cluster_u32.push(b.page);
        cluster_u32.push(b.parent);
        cluster_u32.push(0); // instance 槽（下方按簇段回填）
    }
    let mut instance_range = Vec::with_capacity(scene.instances.len() * 2);
    let mut instances_f32 = Vec::with_capacity(scene.instances.len() * 18);
    for (ii, inst) in scene.instances.iter().enumerate() {
        instance_range.push(inst.cluster_offset);
        instance_range.push(inst.cluster_count);
        // cluster_u32 的 instance 字段回填（与簇段平行）。
        for k in 0..inst.cluster_count as usize {
            cluster_u32[(inst.cluster_offset as usize + k) * 4 + 3] = ii as u32;
        }
        for row in &inst.transform {
            instances_f32.extend_from_slice(row);
        }
        instances_f32.extend_from_slice(&inst.aabb_min);
        instances_f32.extend_from_slice(&inst.aabb_max);
    }
    StaticTables {
        cluster_f32,
        cluster_u32,
        instances_f32,
        instance_range,
    }
}

/// 引擎池 → device 页表/页池镜像（页表更新 + 上传面；每帧同步一次）。
fn sync_device_mirror(
    engine: &StreamingEngine,
    scene: &BuiltScene,
    page_state: &mut [u32],
    pool_bytes: &mut [u8],
) {
    for s in page_state.iter_mut() {
        *s = 0;
    }
    for u in &scene.units {
        for p in 0..u.page_count {
            if let Some(slot) = engine.pool().lookup(u.resource, p) {
                let g = (scene.resource_page_base[u.resource as usize] + p) as usize;
                page_state[g] = slot as u32 + 1;
                let data = engine.pool().slot_data(slot);
                debug_assert!(data.len() <= WORDS_PER_PAGE * 4);
                let base = slot * WORDS_PER_PAGE * 4;
                pool_bytes[base..base + data.len()].copy_from_slice(data);
            }
        }
    }
}

fn resident_count(engine: &StreamingEngine) -> u32 {
    engine.pool().resident_count() as u32
}

fn max_depth_of(scene: &BuiltScene) -> u32 {
    // 父链防御定界 = 全场景最大 DAG 深度上界（层数总和；每链 ≤ 单网格层数）。
    let mut max_level = 0u32;
    for u in &scene.units {
        let mut lvl = 0u32;
        for c in 0..u.cluster_count {
            let g = (u.cluster_offset + c) as usize;
            // level 不在 ClusterRecord；用父链长度量（保守上界 = 簇数）。
            let mut d = 0u32;
            let mut cur = g;
            while scene.bindings[cur].parent != NO_PARENT {
                cur = scene.bindings[cur].parent as usize;
                d += 1;
            }
            lvl = lvl.max(d);
        }
        max_level = max_level.max(lvl + 1);
    }
    max_level.max(1)
}

#[allow(clippy::too_many_arguments)]
fn run_arm(
    label: &str,
    scene: &BuiltScene,
    dev: &DeviceSide,
    tables: &StaticTables,
    engine: &mut StreamingEngine,
    io: Option<&PriorityIoPool>,
    caches: &[std::sync::Arc<std::sync::Mutex<HashMap<u32, Vec<u8>>>>],
    pages_dir: &Path,
    orbit_frames: u32,
    hold_frames: u32,
    threshold: f32,
    budget_pages: u32,
    drain_wait_ms: u64,
) -> ArmResult {
    let total_frames = orbit_frames + hold_frames;
    let max_depth = max_depth_of(scene);
    let mut page_state = vec![0u32; scene.total_pages as usize];
    let mut pool_bytes = vec![0u8; engine.pool().capacity() * WORDS_PER_PAGE * 4];
    let mut records = Vec::with_capacity(total_frames as usize);
    let mut digest_seq = Vec::with_capacity(total_frames as usize);
    let mut submitted: HashSet<(u32, u32)> = HashSet::new();
    let cache_has = |r: u32, p: u32| caches[r as usize].lock().unwrap().contains_key(&p);
    let mut prev_resident: HashSet<(u32, u32)> = HashSet::new();
    for u in &scene.units {
        for p in 0..u.page_count {
            if engine.is_resident(u.resource, p) {
                prev_resident.insert((u.resource, p));
            }
        }
    }
    let mut io_bytes_total = 0u64;
    let mut upload_bytes_total = 0u64;
    let mut evicted_total = 0u32;
    let mut req_overflow_total = 0u32;
    let budget_bytes = u64::from(budget_pages) * 131072;

    for f in 0..total_frames {
        let t_frame = Instant::now();
        let cam = camera_at(scene, f, orbit_frames, threshold);
        // ① 页表/页池镜像 → device 上传面。
        sync_device_mirror(engine, scene, &mut page_state, &mut pool_bytes);
        // ② 剔除 pass device 真跑（产请求 + 渲染决定 + 消费校验）。
        let dout = dev.run_frame(
            scene,
            &cam,
            &page_state,
            &pool_bytes,
            &tables.cluster_f32,
            &tables.cluster_u32,
            &tables.instances_f32,
            &tables.instance_range,
            max_depth,
        );
        req_overflow_total += dout.req_overflow;
        // ③ device 渲染决定归一（与 host 金标准同律单源）+ digest。
        let mut dev_selected: Vec<VisibleCluster> = dout
            .render
            .iter()
            .map(|&(inst, _r, sel, _f)| VisibleCluster {
                instance: inst,
                cluster: sel,
            })
            .collect();
        dev_selected.sort();
        dev_selected.dedup();
        let mut dev_chain: HashMap<(u32, u32), (u32, bool)> =
            HashMap::with_capacity(dout.render.len());
        for &(inst, rend, sel, fell) in &dout.render {
            dev_chain.insert((inst, sel), (rend, fell != 0));
        }
        let dev_nodes: Vec<u32> = dev_selected
            .iter()
            .map(|vc| {
                dev_chain
                    .get(&(vc.instance, vc.cluster))
                    .map(|&(n, _)| n)
                    .unwrap_or(vc.cluster)
            })
            .collect();
        let dev_fell: Vec<bool> = dev_selected
            .iter()
            .map(|vc| {
                dev_chain
                    .get(&(vc.instance, vc.cluster))
                    .map(|&(_, f)| f)
                    .unwrap_or(false)
            })
            .collect();
        let (dev_render, _dev_fallback) =
            normalize_render_decisions(&dev_selected, &dev_nodes, &dev_fell, &scene.bindings);
        let mut dev_pairs: Vec<(u32, u32)> = dev_render.iter().map(|e| (e.instance, e.cluster)).collect();
        dev_pairs.sort();
        let mut dbytes = Vec::with_capacity(dev_pairs.len() * 8 + 8);
        dbytes.extend_from_slice(&(dev_pairs.len() as u32).to_le_bytes());
        dbytes.extend_from_slice(&dout.checksum[0].to_le_bytes());
        for (a, b) in &dev_pairs {
            dbytes.extend_from_slice(&a.to_le_bytes());
            dbytes.extend_from_slice(&b.to_le_bytes());
        }
        let digest = format!("sha256:{}", sha256_hex(&dbytes));
        // ④ host 金标准对拍（同驻留查询语义）。
        let host = lod_cut_with_residency(
            &scene.instances,
            &scene.clusters,
            &scene.bindings,
            &cam,
            f,
            &mut |r, p| engine.is_resident(r, p),
        );
        let mut host_pairs: Vec<(u32, u32)> = host.render.iter().map(|e| (e.instance, e.cluster)).collect();
        host_pairs.sort();
        let parity_ok = host_pairs == dev_pairs;
        let sel_clusters = {
            let vi = rurix_render::geometry::cull::instance_cull(&scene.instances, &cam);
            rurix_render::geometry::cull::cluster_cull(
                &scene.instances,
                &vi,
                &scene.clusters,
                &cam,
            )
        };
        let cover_ok = verify_cut_cover(&sel_clusters, &host.render, &scene.bindings);
        // 诊断（P4_DEBUG_PAIR=1）：全驻留下仍发生合并的链对（组边界共选实证面）。
        if std::env::var("P4_DEBUG_PAIR").as_deref() == Ok("1") && host.fallback_count > 0 {
            for e in &host.render {
                if e.fell_back {
                    let c = &scene.clusters[e.selected as usize];
                    let r = &scene.clusters[e.cluster as usize];
                    let bs = &scene.bindings[e.selected as usize];
                    let br = &scene.bindings[e.cluster as usize];
                    eprintln!(
                        "[dbg] f{f} sel={} (err={} perr={} page r{}p{}) -> render={} (err={} perr={} page r{}p{})",
                        e.selected, c.error, c.parent_error, bs.resource, bs.page,
                        e.cluster, r.error, r.parent_error, br.resource, br.page,
                    );
                }
            }
        }
        // 选中簇去重页数（池工作集页维定纲面；root/流动分项——root 页钉住
        // 不占流动槽,定纲须分项）。
        let (selected_pages, selected_flow_pages) = {
            let mut pages_of_sel: HashSet<(u32, u32)> = HashSet::new();
            for vc in &sel_clusters {
                let b = &scene.bindings[vc.cluster as usize];
                pages_of_sel.insert((b.resource, b.page));
            }
            let flow = pages_of_sel
                .iter()
                .filter(|k| !scene.root_page_keys.contains(k))
                .count() as u32;
            (pages_of_sel.len() as u32, flow)
        };
        if !parity_ok {
            eprintln!(
                "[g31_cluster_stream]: FAIL {label} 帧 {f} device/host 渲染集不符: device {} vs host {}",
                dev_pairs.len(),
                host_pairs.len()
            );
            std::process::exit(1);
        }
        if !cover_ok {
            eprintln!("[g31_cluster_stream]: FAIL {label} 帧 {f} 覆盖不变量破（空洞/重复覆盖）");
            std::process::exit(1);
        }
        // device 请求集 vs host 金标准请求集（一致性核验）。
        let dev_req_set: HashSet<(u32, u32)> = dout.requests.iter().map(|&(r, p, _i, _c)| (r, p)).collect();
        let host_req_set: HashSet<(u32, u32)> = host.requests.iter().map(|q| (q.resource, q.page_index)).collect();
        if dev_req_set != host_req_set {
            eprintln!(
                "[g31_cluster_stream]: FAIL {label} 帧 {f} device/host 请求集不符: device {:?} vs host {:?}",
                dev_req_set, host_req_set
            );
            std::process::exit(1);
        }
        // ⑤ host 驻留调度消费 device 请求缓冲（P4-2 闭环）：派读缺页。
        let mut best_importance: HashMap<(u32, u32), u32> = HashMap::new();
        for &(r, p, imp, _c) in &dout.requests {
            best_importance
                .entry((r, p))
                .and_modify(|e| *e = (*e).max(imp))
                .or_insert(imp);
        }
        if let Some(pool) = io {
            for (&(r, p), &imp) in &best_importance {
                if engine.is_resident(r, p) || submitted.contains(&(r, p)) || cache_has(r, p) {
                    continue;
                }
                pool.submit(
                    pages_dir.join(cluster_page_file_name(r, p)),
                    r,
                    p,
                    FEEDBACK_BASE_GEOMETRY_LOD.saturating_add(imp),
                    f,
                );
                submitted.insert((r, p));
            }
            // 收集完成（hold 段有界等待收敛——真读真等，wait 计量登记）。
            let mut waited = 0u64;
            loop {
                while let Some(c) = pool.try_recv() {
                    caches[c.resource as usize]
                        .lock()
                        .unwrap()
                        .insert(c.page, c.raw);
                }
                let in_hold = f >= orbit_frames;
                let pending = submitted
                    .iter()
                    .any(|k| !cache_has(k.0, k.1) && !engine.is_resident(k.0, k.1));
                if !in_hold || !pending || waited >= drain_wait_ms {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
                waited += 1;
            }
            let _ = waited;
        }
        // ⑥ 反馈 → 引擎三预算 tick（异步缓存为唯一读路径）。两类：
        //   (a) device 请求缓冲中已就绪/已驻留页（缺页加载请求,P4-2 链）;
        //   (b) 本帧选中且已驻留页的零成本触新（LRU 保活——只触新不排队;
        //       缺此则常驻工作集老化被逐 → 池抖动不收敛,bistro 实证）。
        let mut fb = rurix_render::streaming::FeedbackBuilder::new(f);
        for (&(r, p), &imp) in &best_importance {
            if engine.is_resident(r, p) || cache_has(r, p) {
                fb.add(r, p, FEEDBACK_BASE_GEOMETRY_LOD, imp);
            }
        }
        {
            let mut touched: HashSet<(u32, u32)> = HashSet::new();
            for vc in &sel_clusters {
                let b = &scene.bindings[vc.cluster as usize];
                let key = (b.resource, b.page);
                if !best_importance.contains_key(&key)
                    && touched.insert(key)
                    && engine.is_resident(b.resource, b.page)
                {
                    fb.add(b.resource, b.page, FEEDBACK_BASE_GEOMETRY_LOD, 0);
                }
            }
        }
        engine.submit_requests(&fb.build());
        let budget = StreamingBudget {
            io_bytes: budget_bytes,
            transcode_bytes: budget_bytes,
            upload_bytes: budget_bytes,
        };
        let mut report = engine.tick(f, &budget);
        // hold 段收敛排空：就绪页同帧补 tick 至队列清空（定界 64 趟——收敛
        // 不依赖墙钟,只依赖真实完成/真实预算趟数;趟数计量随帧登记）。
        let mut drain_iters = 0u32;
        while f >= orbit_frames && report.queue_depth > 0 && drain_iters < 64 {
            let r2 = engine.tick(f, &budget);
            report.pages_loaded += r2.pages_loaded;
            report.pages_evicted += r2.pages_evicted;
            report.bytes_io += r2.bytes_io;
            report.bytes_transcode += r2.bytes_transcode;
            report.bytes_upload += r2.bytes_upload;
            report.over_budget_stalls += r2.over_budget_stalls;
            report.queue_depth = r2.queue_depth;
            drain_iters += 1;
        }
        io_bytes_total += report.bytes_io;
        upload_bytes_total += report.bytes_upload;
        // 逐出跟踪：被逐页清缓存/submitted（再请求 = 真实重读，IO 量诚实）。
        let mut now_resident: HashSet<(u32, u32)> = HashSet::new();
        for u in &scene.units {
            for p in 0..u.page_count {
                if engine.is_resident(u.resource, p) {
                    now_resident.insert((u.resource, p));
                }
            }
        }
        for key in prev_resident.difference(&now_resident) {
            caches[key.0 as usize].lock().unwrap().remove(&key.1);
            submitted.remove(key);
            evicted_total += 1;
        }
        prev_resident = now_resident;
        records.push(FrameRecord {
            frame: f,
            digest: digest.clone(),
            selected: host.selected_count,
            selected_pages,
            selected_flow_pages,
            miss_selected: host.miss_selected_count,
            fallback: host.fallback_count,
            device_requests: dout.requests.len() as u32,
            pages_loaded: report.pages_loaded,
            pages_evicted: report.pages_evicted,
            bytes_io: report.bytes_io,
            bytes_upload: report.bytes_upload,
            queue_depth: report.queue_depth,
            resident: resident_count(engine),
            checksum0: dout.checksum[0],
            checksum1: dout.checksum[1],
            parity_ok,
            cover_ok,
            io_wait_ms: t_frame.elapsed().as_secs_f64() * 1000.0,
        });
        digest_seq.push(digest);
    }
    let miss_frames = records.iter().filter(|r| r.miss_selected > 0).count() as u32;
    let fallback_frames = records.iter().filter(|r| r.fallback > 0).count() as u32;
    ArmResult {
        digest_seq,
        parity_all: records.iter().all(|r| r.parity_ok),
        cover_all: records.iter().all(|r| r.cover_ok),
        resident_final: records.last().map(|r| r.resident).unwrap_or(0),
        pool_slots: engine.pool().capacity() as u32,
        records,
        io_bytes_total,
        upload_bytes_total,
        miss_frames,
        fallback_frames,
        evicted_total,
        req_overflow_total,
    }
}

// ---------------------------------------------------------------------------
// 优先级探针（P4-4：开工闸前入队 [低×3, 高×1]，单 worker 出队序 measured）
// ---------------------------------------------------------------------------

fn priority_probe(pages_dir: &Path, scene: &BuiltScene) -> (bool, Vec<u32>, u64) {
    // 取前 4 个非 root 页（不足则 FAIL——场景须 ≥4 流动页）。
    let mut cand: Vec<(u32, u32)> = Vec::new();
    'outer: for u in &scene.units {
        for p in 0..u.page_count {
            if !u.root_pages.contains(&p) {
                cand.push((u.resource, p));
                if cand.len() == 4 {
                    break 'outer;
                }
            }
        }
    }
    if cand.len() < 4 {
        fail("优先级探针候选页不足 4（场景过小）");
    }
    let pool = PriorityIoPool::new(1);
    for &(r, p) in &cand[1..] {
        pool.submit(pages_dir.join(cluster_page_file_name(r, p)), r, p, 10, 0);
    }
    let (hr, hp) = cand[0];
    pool.submit(pages_dir.join(cluster_page_file_name(hr, hp)), hr, hp, 999, 0);
    pool.start();
    let mut order = Vec::new();
    for _ in 0..4 {
        let mut got = None;
        for _ in 0..10_000 {
            if let Some(c) = pool.try_recv() {
                got = Some(c);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        order.push(got.unwrap_or_else(|| fail("优先级探针读页超时")).page);
    }
    let bytes = pool.bytes_read_total();
    let log = pool.dequeue_log();
    let ok = order[0] == hp
        && order[1..] == cand[1..].iter().map(|&(_, p)| p).collect::<Vec<_>>()[..]
        && log.first().map(|e| e.priority) == Some(999)
        && bytes > 0;
    (ok, order, bytes)
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut gltf_path = String::new();
    let mut spv_path = String::new();
    let mut pages_dir = String::new();
    let mut evidence_path = String::new();
    let mut orbit_frames = 24u32;
    let mut hold_frames = 8u32;
    let mut pool_slots_override = 0u32;
    let mut max_meshes = 12usize;
    let mut max_tris = 400_000u64;
    let mut budget_pages = 2u32;
    let mut io_workers = 2usize;
    let mut threshold = 1.0f32;
    let mut i = 1;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i).cloned().unwrap_or_else(|| fail("CLI 缺值"))
        };
        match args[i].as_str() {
            "--gltf" => gltf_path = take(&mut i),
            "--spv" => spv_path = take(&mut i),
            "--pages-dir" => pages_dir = take(&mut i),
            "--evidence" => evidence_path = take(&mut i),
            "--orbit-frames" => orbit_frames = take(&mut i).parse().unwrap_or_else(|_| fail("--orbit-frames 非法")),
            "--hold-frames" => hold_frames = take(&mut i).parse().unwrap_or_else(|_| fail("--hold-frames 非法")),
            "--pool-slots" => pool_slots_override = take(&mut i).parse().unwrap_or_else(|_| fail("--pool-slots 非法")),
            "--max-meshes" => max_meshes = take(&mut i).parse().unwrap_or_else(|_| fail("--max-meshes 非法")),
            "--max-tris" => max_tris = take(&mut i).parse().unwrap_or_else(|_| fail("--max-tris 非法")),
            "--budget-pages" => budget_pages = take(&mut i).parse().unwrap_or_else(|_| fail("--budget-pages 非法")),
            "--io-workers" => io_workers = take(&mut i).parse().unwrap_or_else(|_| fail("--io-workers 非法")),
            "--error-threshold-px" => threshold = take(&mut i).parse().unwrap_or_else(|_| fail("--error-threshold-px 非法")),
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    if gltf_path.is_empty() || spv_path.is_empty() || pages_dir.is_empty() || evidence_path.is_empty() {
        fail("参数闭集缺行（gltf/spv/pages-dir/evidence 必给）");
    }
    if orbit_frames < 4 || hold_frames < 4 {
        fail("orbit/hold 帧数下限 4（轨迹/收敛段结构下限）");
    }
    let t0 = Instant::now();

    // ── 三态前置：loader/gltf/SPV 缺 → skipped_dev_env 退 0（不冒充 PASS）──
    let mut degrade: Vec<String> = Vec::new();
    if !vk::vulkan_available() {
        degrade.push("vulkan loader 不可用".into());
    }
    if !Path::new(&gltf_path).is_file() {
        degrade.push(format!("gltf 缺失 {gltf_path}"));
    }
    if !Path::new(&spv_path).is_file() {
        degrade.push(format!("spv 缺失 {spv_path}"));
    }
    if !degrade.is_empty() {
        let reasons = degrade
            .iter()
            .map(|r| jstr(r))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "{{\"schema\":\"rurix.g31.cluster_stream.skip.v1\",\"state\":\"skipped_dev_env\",\"reasons\":[{reasons}]}}"
        );
        return;
    }

    // ── 场景与页集（P4-1：bistro 派生真实页集落盘）──
    let pages_dir_p = PathBuf::from(&pages_dir);
    let scene = build_scene(Path::new(&gltf_path), &pages_dir_p, max_meshes, max_tris);
    if scene.total_pages < 8 {
        fail("页全集 < 8（强制小池压力结构不可达——放大 max-meshes/max-tris）");
    }
    let dev = DeviceSide::create(Path::new(&spv_path))
        .unwrap_or_else(|e| fail(&format!("device 面创建失败: {e}")));
    let tables = build_static_tables(&scene);

    // ── reference 臂（全驻留 ×2 双跑位级；直读模式资源 + 空缓存面）──
    let empty_caches: Vec<std::sync::Arc<std::sync::Mutex<HashMap<u32, Vec<u8>>>>> = (0..scene
        .units
        .len())
        .map(|_| std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())))
        .collect();
    let preload_all = |engine: &mut StreamingEngine| {
        let mut fb = rurix_render::streaming::FeedbackBuilder::new(0);
        for u in &scene.units {
            for p in 0..u.page_count {
                fb.add(u.resource, p, FEEDBACK_BASE_GEOMETRY_LOD, 1);
            }
        }
        engine.submit_requests(&fb.build());
        engine.tick(0, &StreamingBudget {
            io_bytes: u64::MAX,
            transcode_bytes: u64::MAX,
            upload_bytes: u64::MAX,
        });
        for u in &scene.units {
            for p in 0..u.page_count {
                if !engine.is_resident(u.resource, p) {
                    fail("reference 臂全驻留预载未齐（池容量/预算数据面破损）");
                }
            }
        }
    };
    let mut ref_engine = StreamingEngine::new(scene.total_pages as usize);
    for u in &scene.units {
        ref_engine.register_resource(Box::new(ClusterPageResource::new(
            u.resource,
            &pages_dir_p,
            u.page_count,
            u.root_pages.clone(),
        )));
    }
    preload_all(&mut ref_engine);
    let ref_a = run_arm(
        "reference",
        &scene,
        &dev,
        &tables,
        &mut ref_engine,
        None,
        &empty_caches,
        &pages_dir_p,
        orbit_frames,
        hold_frames,
        threshold,
        u32::MAX / 2,
        0,
    );
    // reference 双跑须同引擎初态（第二跑 = 全新引擎同参重建）。
    let mut ref_engine_b = StreamingEngine::new(scene.total_pages as usize);
    for u in &scene.units {
        ref_engine_b.register_resource(Box::new(ClusterPageResource::new(
            u.resource,
            &pages_dir_p,
            u.page_count,
            u.root_pages.clone(),
        )));
    }
    preload_all(&mut ref_engine_b);
    let ref_b = run_arm(
        "reference",
        &scene,
        &dev,
        &tables,
        &mut ref_engine_b,
        None,
        &empty_caches,
        &pages_dir_p,
        orbit_frames,
        hold_frames,
        threshold,
        u32::MAX / 2,
        0,
    );
    let ref_double_bitexact = ref_a.digest_seq == ref_b.digest_seq;
    if !ref_double_bitexact {
        fail("reference 双跑 digest_seq 非位级一致（device 决定性破坏）");
    }

    // ── stream 臂（强制小驻留池 + 冷启动 + 异步优先级 IO）──
    // 池定纲 root/流动分项：root 页钉住占容量（不占流动槽），流动槽 = hold
    // 段选中流动页上界（reference 实测）+1 在途余量——收敛结构保证;池 <
    // 全集 = 强制压力面（orbit 工作集峰超池 ⇒ 缺页/回退/逐出全程发生）。
    let roots_total = scene.root_page_keys.len() as u32;
    if roots_total + 2 >= scene.total_pages {
        fail("流动页不足 2（root 页占满全集——压力结构不可达，放大 max-meshes/max-tris）");
    }
    let hold_flow_max = ref_a.records[orbit_frames as usize..]
        .iter()
        .map(|r| r.selected_flow_pages)
        .max()
        .unwrap_or(1);
    let mut pool_slots = (roots_total + hold_flow_max + 1).clamp(roots_total + 1, scene.total_pages - 1);
    if pool_slots_override > 0 {
        pool_slots = pool_slots_override.clamp(roots_total + 1, scene.total_pages - 1);
    }
    // 异步缓存面（每资源一图；root 预载 = 同步真读盘，字节计量登记）。
    let stream_caches: Vec<std::sync::Arc<std::sync::Mutex<HashMap<u32, Vec<u8>>>>> = (0..scene
        .units
        .len())
        .map(|_| std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())))
        .collect();
    let mut roots_preload_bytes = 0u64;
    let mut stream_engine = StreamingEngine::new(pool_slots as usize);
    for u in &scene.units {
        for &root in &u.root_pages {
            let bytes = std::fs::read(pages_dir_p.join(cluster_page_file_name(u.resource, root)))
                .unwrap_or_else(|e| fail(&format!("root 页预载读取失败: {e}")));
            roots_preload_bytes += bytes.len() as u64;
            stream_caches[u.resource as usize]
                .lock()
                .unwrap()
                .insert(root, bytes);
        }
        stream_engine.register_resource(Box::new(ClusterPageResource::with_cache(
            u.resource,
            &pages_dir_p,
            u.page_count,
            u.root_pages.clone(),
            stream_caches[u.resource as usize].clone(),
        )));
    }
    let io_pool = PriorityIoPool::new(io_workers);
    io_pool.start();
    let stream = run_arm(
        "stream",
        &scene,
        &dev,
        &tables,
        &mut stream_engine,
        Some(&io_pool),
        &stream_caches,
        &pages_dir_p,
        orbit_frames,
        hold_frames,
        threshold,
        budget_pages,
        1000,
    );
    let io_read_bytes = io_pool.bytes_read_total() + roots_preload_bytes;
    let io_submitted = io_pool.submitted_count();
    let io_completed = io_pool.completed_count();

    // ── 收敛判定（结构容差：hold 段末 K 帧位级 = 驻留完整；回退帧允许 LOD 差）──
    let total = (orbit_frames + hold_frames) as usize;
    let mut converge_ok = true;
    let mut converge_bitexact_frames = 0u32;
    for f in (total - 2)..total {
        if stream.digest_seq[f] == ref_a.digest_seq[f] {
            converge_bitexact_frames += 1;
        } else {
            converge_ok = false;
        }
    }
    let mut hold_bitexact = 0u32;
    for f in (total - hold_frames as usize)..total {
        if stream.digest_seq[f] == ref_a.digest_seq[f] {
            hold_bitexact += 1;
        }
    }

    // ── 优先级探针（P4-4 measured）──
    let (probe_ok, probe_order, probe_bytes) = priority_probe(&pages_dir_p, &scene);

    // ── evidence（harness 真跑工作件；门裁决经 smoke 蒸馏）──
    let frames_json = |recs: &[FrameRecord]| -> String {
        let mut s = String::from("[");
        for (i, r) in recs.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"frame\":{},\"digest\":{},\"selected\":{},\"selected_pages\":{},\"selected_flow_pages\":{},\"miss_selected\":{},\"fallback\":{},\"device_requests\":{},\"pages_loaded\":{},\"pages_evicted\":{},\"bytes_io\":{},\"bytes_upload\":{},\"queue_depth\":{},\"resident\":{},\"checksum0\":{},\"checksum1\":{},\"parity_ok\":{},\"cover_ok\":{},\"io_wait_ms\":{:.3}}}",
                r.frame,
                jstr(&r.digest),
                r.selected,
                r.selected_pages,
                r.selected_flow_pages,
                r.miss_selected,
                r.fallback,
                r.device_requests,
                r.pages_loaded,
                r.pages_evicted,
                r.bytes_io,
                r.bytes_upload,
                r.queue_depth,
                r.resident,
                r.checksum0,
                r.checksum1,
                r.parity_ok,
                r.cover_ok,
                r.io_wait_ms
            ));
        }
        s.push(']');
        s
    };
    let digest_seq_json = |seq: &[String]| -> String {
        format!(
            "[{}]",
            seq.iter().map(|d| jstr(d)).collect::<Vec<_>>().join(",")
        )
    };
    let units_json = {
        let mut s = String::from("[");
        for (i, u) in scene.units.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"resource\":{},\"mesh_id\":{},\"primitive_id\":{},\"triangles\":{},\"clusters\":{},\"pages\":{},\"root_pages\":{}}}",
                u.resource,
                u.mesh_id,
                u.primitive_id,
                u.triangles,
                u.cluster_count,
                u.page_count,
                u.root_pages.len()
            ));
        }
        s.push(']');
        s
    };
    let doc = format!(
        "{{\"schema\":{eschema},\"gltf\":{gltf},\"page_set_digest\":{psd},\"scene\":{{\"meshes\":{meshes},\"clusters\":{clusters},\"pages\":{pages},\"triangles\":{tris},\"build_ms\":{bms:.3},\"units\":{units}}},\"orbit_frames\":{of},\"hold_frames\":{hf},\"threshold_px\":{thr},\"reference\":{{\"pool_slots\":{rps},\"digest_seq\":{rseq},\"double_run_bitexact\":{rbit},\"frames\":{rframes},\"parity_all\":{rpar},\"cover_all\":{rcov}}},\"stream\":{{\"pool_slots\":{sps},\"digest_seq\":{sseq},\"frames\":{sframes},\"parity_all\":{spar},\"cover_all\":{scov},\"io_bytes_total\":{sio},\"upload_bytes_total\":{sup},\"miss_frames\":{smiss},\"fallback_frames\":{sfb},\"evicted_total\":{sev},\"resident_final\":{sres},\"io_read_bytes_total\":{sioread},\"io_submitted\":{siosub},\"io_completed\":{siocompl},\"req_overflow_total\":{sovf}}},\"convergence\":{{\"last2_bitexact\":{cok},\"last2_bitexact_frames\":{c2},\"hold_bitexact_frames\":{chold}}},\"priority_probe\":{{\"ok\":{pok},\"order\":[{porder}],\"bytes_read\":{pbytes}}},\"elapsed_ms\":{elapsed:.3}}}",
        eschema = jstr(EVIDENCE_SCHEMA),
        gltf = jstr(&gltf_path.replace('\\', "/")),
        psd = jstr(&scene.page_set_digest),
        meshes = scene.units.len(),
        clusters = scene.total_clusters,
        pages = scene.total_pages,
        tris = scene.total_tris,
        bms = scene.mesh_build_ms,
        units = units_json,
        of = orbit_frames,
        hf = hold_frames,
        thr = threshold,
        rps = ref_a.pool_slots,
        rseq = digest_seq_json(&ref_a.digest_seq),
        rbit = ref_double_bitexact,
        rframes = frames_json(&ref_a.records),
        rpar = ref_a.parity_all,
        rcov = ref_a.cover_all,
        sps = stream.pool_slots,
        sseq = digest_seq_json(&stream.digest_seq),
        sframes = frames_json(&stream.records),
        spar = stream.parity_all,
        scov = stream.cover_all,
        sio = stream.io_bytes_total,
        sup = stream.upload_bytes_total,
        smiss = stream.miss_frames,
        sfb = stream.fallback_frames,
        sev = stream.evicted_total,
        sres = stream.resident_final,
        sioread = io_read_bytes,
        siosub = io_submitted,
        siocompl = io_completed,
        sovf = stream.req_overflow_total,
        cok = converge_ok,
        c2 = converge_bitexact_frames,
        chold = hold_bitexact,
        pok = probe_ok,
        porder = probe_order
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(","),
        pbytes = probe_bytes,
        elapsed = t0.elapsed().as_secs_f64() * 1000.0,
    );
    std::fs::write(&evidence_path, format!("{doc}\n"))
        .unwrap_or_else(|e| fail(&format!("evidence 写盘失败: {e}")));
    println!("[g31_cluster_stream]: PASS evidence={evidence_path}");
}
