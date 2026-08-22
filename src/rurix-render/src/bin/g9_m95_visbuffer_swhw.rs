//! G9.3 M95 蒙皮簇 VisBuffer SW/HW diff=0 device 双腿 harness(RXS-0352;门
//! `g9.p0.m95.single_source_truth` 的蒙皮簇 VisBuffer 判据面)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §2 M95 行「蒙皮簇 VisBuffer SW/HW diff=0 维持」)
//!
//! - **场景**:两簇(静态 C0 + 蒙皮 C1)× 单实例;蒙皮簇顶点经 **M92 device
//!   蒙皮 kernel 真跑**产出(skin cache 槽位语义;与 host Kerbl 参照逐位一致
//!   内嵌断言),静态簇直读静止池——蒙皮簇经 skin cache 顶点进 SW/HW 双路
//!   (RXS-0352 L1 光栅条);
//! - **双腿**:同一帧 `render_exec::execute_frame` 内 SW 精确 compute 腿
//!   ([`visbuffer_swhw_spv::sw_visbuffer_u64_spv`])vs HW 保守光栅图形腿
//!   (OVERESTIMATE 超集 + FS 逐字复刻判定,RXS-0303 体例),同场景同投影同
//!   VisBuffer ABI(128×72,`depth30|cluster27|tri7`);**u64 位级 diff = 0**
//!   (整数域零容差);
//! - **host oracle**:`VisBufferCpu`(skin cache 路 `raster_visible_set` 同式
//!   投影)覆盖集合与 SW/HW 逐像素相等(FMA 残差不进判据,G7.5b 同构);
//! - **帧末 provenance**:三喂 digest 精确一致 + as_manager RT 消费锚
//!   (`rt_blas_input_from_feed`)放行权威 feed / 旁路重算 feed 判 RED
//!   (双世界结构否决,L4/R-G9-8);
//! - **RED 臂(device 侧有效)**:篡改一像素(device 回读副本)⇒ 整数域 diff
//!   检出;篡改 HW 顶点流受害三角形 `ids.cluster += 1`(SSBO 输入与 oracle
//!   不动)⇒ 双腿 diff 必 > 0(能红反证);
//! - `RURIX_VK_VALIDATION=1`:render_exec messenger fail-closed,
//!   `validation_error_total` 必须 = 0。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备 / W2(int64 原子)能力缺失 → `G9_M95_VB: SKIP` +
//! 显式 DEV_ENV_DEGRADE(退 0,非 fake pass);设备**有** Vulkan 但无
//!   `VK_EXT_conservative_rasterization` → **fail-closed 硬红**(RXS-0303 L3,
//!   不静默降级、降级臂未启用);判据不符 / RED 轴失效 → FAIL 退 1。

use rurix_render::geometry::gpu_scene::{InstanceRecord, transform_point};
use rurix_render::geometry::skin_kernel::{self, M92_BOUND_WORDS};
use rurix_render::geometry::skinning::{NormalCone, SkinPalette, skin_cluster};
use rurix_render::geometry::visible_cluster_set::{
    VisibleClusterEntry, VisibleClusterSet, compute_provenance_digest, verify_frame_provenance,
};
use rurix_render::geometry::visbuffer::{
    VISBUFFER_CLEAR, VisBufferCpu, visbuffer_diff_host,
};
use rurix_render::geometry::visbuffer_swhw_spv as vbspv;
use rurix_render::graph::types::visbuffer_unpack;
use rurix_render::rt::as_manager::rt_blas_input_from_feed;
use rurix_render::temporal::common::Mat4;
use rurix_rt::render_exec::{
    self, Bindings, BufferDesc, BufferUsage, ColorAttachmentRef, ComputePass,
    ConservativeRasterDesc, DispatchSpec, DrawSpec, KernelWave, Pass, RasterPass, Readback,
    ResourceDesc, TargetState, TexFormat, TextureDesc, TextureUsage, VertexData,
};
use rurix_rt::vk;

/// 对拍分辨率(与 G7.5b 口径同形;9216 词)。
const VIS_W: u32 = 128;
const VIS_H: u32 = 72;
const VIS_WORDS: usize = (VIS_W * VIS_H) as usize;

/// 顶点属性格式(Vulkan 枚举值;G7.5b 同律)。
const FORMAT_R32G32B32A32_SFLOAT: u32 = 109;
const FORMAT_R32G32_UINT: u32 = 101;
/// 顶点布局(loc, format, offset),stride = 72B(G7.5b 设计 §4.3 同构)。
const HW_VERTEX_STRIDE: u32 = 72;
const HW_VERTEX_ATTRS: [(u32, u32, u32); 5] = [
    (0, FORMAT_R32G32B32A32_SFLOAT, 0),
    (1, FORMAT_R32G32B32A32_SFLOAT, 16),
    (2, FORMAT_R32G32B32A32_SFLOAT, 32),
    (3, FORMAT_R32G32B32A32_SFLOAT, 48),
    (4, FORMAT_R32G32_UINT, 64),
];

fn fail(msg: &str) -> ! {
    eprintln!("G9_M95_VB: FAIL {msg}");
    std::process::exit(1)
}

fn skip(msg: &str) -> ! {
    println!("G9_M95_VB: SKIP DEV_ENV_DEGRADE {msg}");
    std::process::exit(0)
}

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn storage(size: usize, data: Option<&[u8]>) -> ResourceDesc<'_> {
    ResourceDesc::Buffer(BufferDesc {
        size: size as u64,
        usage: BufferUsage {
            storage: true,
            ..Default::default()
        },
        data,
        // G14.10d 加字段后的最小修复:保持既有 host-visible 行为(0-byte)。
        device_local: false,
    })
}

fn spv_bytes(spv: &[u32]) -> Vec<u8> {
    spv.iter().flat_map(|w| w.to_le_bytes()).collect()
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_u32(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_u64(b: &[u8]) -> Vec<u64> {
    b.chunks_exact(8)
        .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect()
}

// ---------------------------------------------------------------------------
// 确定性蒙皮簇场景(两簇一单实例;C1 蒙皮 = device kernel 产物进 skin cache)
// ---------------------------------------------------------------------------

/// 场景面(host oracle 与 device 双腿共用的唯一数据面;投影表达式与
/// `visbuffer::raster_visible_set` 逐字同构)。
struct Scene {
    instances: Vec<InstanceRecord>,
    /// 簇记录(C0 静态 / C1 蒙皮;vertex/triangle 区间引用静态池)。
    clusters: Vec<rurix_render::graph::types::ClusterRecord>,
    /// 静态顶点池(C0 段 0..3;C1 段 3..6 = 静止顶点)。
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
    view_proj: [[f32; 4]; 4],
    /// skin cache 蒙皮后顶点(C1;device kernel 产出)。
    skinned: Vec<[f32; 3]>,
    /// 全局簇 → skin 槽位(C1 → 0)。
    skin_slot_of: [u32; 2],
    set: VisibleClusterSet,
}

/// 构建场景 + 经 M92 device kernel 真跑产出蒙皮顶点(含 host 参照逐位锚)。
fn build_scene_with_device_skin(spv: &[u8]) -> Result<Scene, String> {
    // C0 下半三角形(静止);C1 上半三角形(蒙皮:平移 (+0.5, +0.25, 0) 定点域)。
    let rest0 = [[-1.0f32, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 0.0, 0.0]];
    let rest1 = [[-1.0f32, 1.0, 0.0], [0.0, 3.0, 0.0], [1.0, 1.0, 0.0]];
    let vertices: Vec<[f32; 3]> = vec![rest0[0], rest0[1], rest0[2], rest1[0], rest1[1], rest1[2]];
    let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 1];
    let rec = |voff: u32, toff: u32| rurix_render::graph::types::ClusterRecord {
        center: [0.0; 3],
        radius: 2.0,
        cone_axis: [0.0, 0.0, 1.0],
        cone_cutoff: 2.0,
        error: 0.0,
        parent_error: f32::INFINITY,
        vertex_offset: voff,
        triangle_offset: toff,
        vertex_count: 3,
        triangle_count: 1,
        page_id: 0,
        reserved: 0,
    };
    let clusters = vec![rec(0, 0), rec(3, 3)];
    let instances = vec![InstanceRecord {
        transform: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, -5.0],
        ],
        cluster_offset: 0,
        cluster_count: 2,
        material_id: 0,
        flags: 0,
        aabb_min: [-2.0; 3],
        mesh_id: 0,
        aabb_max: [2.0; 3],
        reserved: u32::MAX,
    }];
    let view_proj = [
        [4.0, 0.0, 0.0, 0.0],
        [0.0, 4.0, 0.0, 0.0],
        [0.0, 0.0, -1.0, -0.2],
        [0.0, 0.0, -1.0, 0.0],
    ];

    // 蒙皮姿态:单骨平移 (+0.5, +0.25, 0)(定点域;host 参照 golden 对拍)。
    let pose = SkinPalette {
        bones: vec![[
            [1.0, 0.0, 0.0, 0.5],
            [0.0, 1.0, 0.0, 0.25],
            [0.0, 0.0, 1.0, 0.0],
        ]],
    };
    let weights: Vec<Vec<(u32, f32)>> =
        vec![vec![(0, 1.0), (0, 0.0)], vec![(0, 1.0), (0, 0.0)], vec![(0, 1.0), (0, 0.0)]];
    let bone_indices = [0u32, 0]; // 定长 2(末位首骨重复 padding)
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; 3];
    let rest_aabb = ([-1.0f32, 1.0, 0.0], [1.0, 3.0, 0.0]);
    let rest_cone = NormalCone {
        axis: [0.0, 0.0, 1.0],
        half_angle: 0.0,
    };
    let host_skinned = {
        let input = rurix_render::geometry::skinning::ClusterSkinInput {
            max_influences: 2,
            bone_indices: &bone_indices,
            bound_inflation: 0.25,
            rest_aabb_min: rest_aabb.0,
            rest_aabb_max: rest_aabb.1,
            vertices: &rest1,
            weights: &weights,
        };
        skin_cluster(&input, &pose).expect("host 参照蒙皮")
    };
    // device kernel 真跑(单簇单 pass;与 host 参照逐位对拍内嵌)。
    let (palette_b, angle_b) = skin_kernel::pack_palette(&pose);
    let pack = skin_kernel::pack_cluster(
        &rest1,
        &normals,
        &weights,
        &bone_indices,
        0.25,
        rest_aabb,
        &rest_cone,
    );
    let resources = [
        storage(pack.rest_pos.len(), Some(&pack.rest_pos)),
        storage(pack.rest_nrm.len(), Some(&pack.rest_nrm)),
        storage(pack.wval.len(), Some(&pack.wval)),
        storage(pack.wbone.len(), Some(&pack.wbone)),
        storage(palette_b.len(), Some(&palette_b)),
        storage(angle_b.len(), Some(&angle_b)),
        storage(pack.cluster_bones.len(), Some(&pack.cluster_bones)),
        storage(pack.rest_pos.len(), None),
        storage(pack.rest_nrm.len(), None),
        storage(M92_BOUND_WORDS * 4, None),
    ];
    let passes = [Pass::Compute(ComputePass {
        name: "m95_skin_c1",
        spirv: spv,
        entry: Some("main"),
        dispatch: DispatchSpec::Direct([pack.n_vertices, 1, 1]),
        bindings: Bindings {
            storage_buffers: (0..10).collect(),
            push_constants: pack.push.clone(),
            ..Default::default()
        },
    })];
    let barriers: [&[(u32, TargetState)]; 1] = [&[]];
    let readbacks = [
        Readback::Buffer {
            res: 7,
            offset: 0,
            size: (pack.n_vertices * 12) as u64,
        },
        Readback::Buffer {
            res: 9,
            offset: 0,
            size: (M92_BOUND_WORDS * 4) as u64,
        },
    ];
    let out = render_exec::execute_frame(&resources, &passes, &barriers, &readbacks)?;
    let device_skinned = skin_kernel::decode_vec3s(&out[0], rest1.len());
    for (i, (d, h)) in device_skinned.iter().zip(host_skinned.iter()).enumerate() {
        if d.map(f32::to_bits) != h.map(f32::to_bits) {
            return Err(format!(
                "蒙皮 kernel 输出 ≠ host 参照(顶点 {i}:device={d:?} host={h:?};容差 0)"
            ));
        }
    }
    let _bound = skin_kernel::decode_bound(&out[1]);

    // VisibleClusterSet(两元素;C1 skin_version = 1)。
    let mut set = VisibleClusterSet {
        frame_serial: 1,
        entries: vec![
            VisibleClusterEntry {
                cluster: 0,
                instance: 0,
                lod_level: 0,
                skin_version: 0,
                page_id: 0,
                visible: true,
            },
            VisibleClusterEntry {
                cluster: 1,
                instance: 0,
                lod_level: 0,
                skin_version: 1,
                page_id: 0,
                visible: true,
            },
        ],
        residency: vec![],
        fallback: vec![],
        provenance_digest: [0; 32],
    };
    set.provenance_digest = compute_provenance_digest(&set);

    Ok(Scene {
        instances,
        clusters,
        vertices,
        indices,
        view_proj,
        skinned: device_skinned,
        skin_slot_of: [u32::MAX, 0],
        set,
    })
}

/// 可见集投影(与 `visbuffer::raster_visible_set` 逐字同式;同时产 host
/// oracle 与 device 双腿输入:9 f32/三角形屏幕坐标 + (entry_idx, tri) ids)。
fn project_visible_set(scene: &Scene) -> (Vec<f32>, Vec<u32>, VisBufferCpu) {
    let vp = Mat4 {
        m: scene.view_proj,
    };
    let (w_px, h_px) = (VIS_W as f32, VIS_H as f32);
    let mut triangles: Vec<f32> = Vec::new();
    let mut ids: Vec<u32> = Vec::new();
    let mut oracle = VisBufferCpu::new(VIS_W, VIS_H);
    for (entry_idx, e) in scene.set.visible_entries() {
        let inst = &scene.instances[e.instance as usize];
        let c = &scene.clusters[e.cluster as usize];
        let slot = scene.skin_slot_of[e.cluster as usize];
        for t in 0..c.triangle_count {
            let mut screen = [[0.0f32; 3]; 3];
            let mut valid = true;
            for (k, sv) in screen.iter_mut().enumerate() {
                let local = scene.indices[(c.triangle_offset + 3 * t) as usize + k];
                let obj = if slot != u32::MAX {
                    scene.skinned[local as usize]
                } else {
                    scene.vertices[(c.vertex_offset + local) as usize]
                };
                let world = transform_point(&inst.transform, obj);
                let clip = vp.transform_vec4([world[0], world[1], world[2], 1.0]);
                if clip[3] <= 1e-20 {
                    valid = false;
                    break;
                }
                let inv_w = 1.0 / clip[3];
                let nx = clip[0] * inv_w;
                let ny = clip[1] * inv_w;
                let nz = (clip[2] * inv_w).clamp(0.0, 1.0);
                *sv = [(nx + 1.0) * 0.5 * w_px, (1.0 - ny) * 0.5 * h_px, nz];
            }
            if valid {
                for v in screen {
                    triangles.extend_from_slice(&v);
                }
                ids.extend_from_slice(&[entry_idx, t]);
                oracle.raster_triangle(&screen, entry_idx, t);
            }
        }
    }
    (triangles, ids, oracle)
}

/// 顶点流构建(G7.5b 设计 §4.3 同构;`tamper_ids` = RED 轴:受害三角形
/// `ids.cluster += 1`,SSBO 输入与 oracle 不动)。
fn build_vertex_stream(
    triangles: &[f32],
    ids: &[u32],
    tamper_victim: Option<usize>,
) -> Vec<u8> {
    let n = ids.len() / 2;
    let (half_w, half_h) = (VIS_W as f32 * 0.5, VIS_H as f32 * 0.5);
    let mut out = Vec::with_capacity(n * 3 * HW_VERTEX_STRIDE as usize);
    let push4 = |o: &mut Vec<u8>, v: [f32; 4]| {
        for f in v {
            o.extend_from_slice(&f.to_le_bytes());
        }
    };
    for i in 0..n {
        let b = i * 9;
        let va = [triangles[b], triangles[b + 1], triangles[b + 2], 0.0];
        let vb = [triangles[b + 3], triangles[b + 4], triangles[b + 5], 0.0];
        let vc = [triangles[b + 6], triangles[b + 7], triangles[b + 8], 0.0];
        let mut id = [ids[i * 2], ids[i * 2 + 1]];
        if tamper_victim == Some(i) {
            id[0] += 1; // RED 轴:cluster27 漂移
        }
        for k in 0..3 {
            let sx = triangles[b + 3 * k];
            let sy = triangles[b + 3 * k + 1];
            push4(&mut out, [sx / half_w - 1.0, sy / half_h - 1.0, 0.5, 1.0]);
            push4(&mut out, va);
            push4(&mut out, vb);
            push4(&mut out, vc);
            out.extend_from_slice(&id[0].to_le_bytes());
            out.extend_from_slice(&id[1].to_le_bytes());
        }
    }
    out
}

/// 单帧双腿执行:pass0 = SW compute → res 2;pass1 = HW 保守光栅 → res 3。
fn execute_pair_frame(
    sw_spv: &[u8],
    vs_spv: &[u8],
    fs_spv: &[u8],
    triangles: &[f32],
    ids: &[u32],
    vertex_bytes: &[u8],
) -> Result<(Vec<u64>, Vec<u64>), String> {
    let tri_count = (ids.len() / 2) as u32;
    let tris_b = bytes_f32(triangles);
    let ids_b = bytes_u32(ids);
    let clear = vec![VISBUFFER_CLEAR; VIS_WORDS];
    let clear_b: Vec<u8> = clear.iter().flat_map(|w| w.to_le_bytes()).collect();
    let resources = [
        storage(tris_b.len(), Some(&tris_b)),
        storage(ids_b.len(), Some(&ids_b)),
        storage(clear_b.len(), Some(&clear_b)), // 2: vis_sw
        storage(clear_b.len(), Some(&clear_b)), // 3: vis_hw
        ResourceDesc::Texture(TextureDesc {
            width: VIS_W,
            height: VIS_H,
            format: TexFormat::Rgba8Unorm,
            usage: TextureUsage {
                color: true,
                ..Default::default()
            },
            data: None,
        }), // 4: dummy color(VisBuffer 走 SSBO)
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "m95_visbuffer_sw_u64",
            spirv: sw_spv,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([tri_count * VIS_W * VIS_H, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![0, 1, 2],
                push_constants: bytes_u32(&[tri_count, VIS_W, VIS_H]),
                ..Default::default()
            },
        }),
        Pass::Raster(RasterPass {
            name: "m95_visbuffer_hw_raster",
            vs_spirv: vs_spv,
            fs_spirv: fs_spv,
            vertex: VertexData::Inline {
                data: vertex_bytes,
                stride: HW_VERTEX_STRIDE,
                attrs: &HW_VERTEX_ATTRS,
            },
            draw: DrawSpec::Direct {
                vertex_count: tri_count * 3,
                instance_count: 1,
                first_vertex: 0,
                first_instance: 0,
            },
            colors: vec![ColorAttachmentRef {
                res: 4,
                clear: Some([0.0, 0.0, 0.0, 1.0]),
            }],
            depth: None, // 深度竞争完全由 u64 atomicMax 承担(与 SW 同构)
            viewport: Some((VIS_W, VIS_H)),
            bindings: Bindings {
                storage_buffers: vec![3],
                push_constants: bytes_u32(&[VIS_W]),
                ..Default::default()
            },
            conservative: Some(ConservativeRasterDesc {
                extra_overestimation: 0.0,
            }),
        }),
    ];
    let barriers: [&[(u32, TargetState)]; 2] = [&[], &[(4, TargetState::ColorAttachmentWrite)]];
    let readbacks = [
        Readback::Buffer {
            res: 2,
            offset: 0,
            size: (VIS_WORDS * 8) as u64,
        },
        Readback::Buffer {
            res: 3,
            offset: 0,
            size: (VIS_WORDS * 8) as u64,
        },
    ];
    let out = render_exec::execute_frame(&resources, &passes, &barriers, &readbacks)?;
    Ok((read_u64(&out[0]), read_u64(&out[1])))
}

/// oracle 覆盖像素数最多的三角形(RED 轴受害者;确定性 tie-break)。
fn dominant_triangle(oracle: &VisBufferCpu, ids: &[u32]) -> Option<usize> {
    let mut best: Option<((u32, u32), u32)> = None; // (cluster,tri) → count
    let mut counts = std::collections::BTreeMap::new();
    for &w in &oracle.data {
        if w != VISBUFFER_CLEAR {
            let (_, c, t) = visbuffer_unpack(w);
            *counts.entry((c, t)).or_insert(0u32) += 1;
        }
    }
    for (&k, &n) in counts.iter() {
        if best.is_none_or(|(bk, bn)| (n, k) > (bn, bk)) {
            best = Some((k, n));
        }
    }
    let (key, _) = best?;
    ids.chunks_exact(2).position(|p| p[0] == key.0 && p[1] == key.1)
}

fn coverage_equal(a: &[u64], b: &[u64]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(&x, &y)| (x == VISBUFFER_CLEAR) == (y == VISBUFFER_CLEAR))
}

fn main() {
    println!(
        "[g9_m95_visbuffer_swhw] G9.3 M95 蒙皮簇 VisBuffer SW/HW diff=0 device 双腿 harness(RXS-0352;门 g9.p0.m95.single_source_truth)"
    );
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut evidence_path: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--evidence" => {
                i += 1;
                evidence_path = Some(args.get(i).expect("--evidence path").clone());
            }
            other => fail(&format!("unknown arg {other}")),
        }
        i += 1;
    }

    // ── 步骤 0:能力门(三态;W2 + 保守光栅 + FS 原子)──
    if !vk::vulkan_available() {
        skip("无 Vulkan loader(dev-env degrade)");
    }
    let caps = match render_exec::probe_device_caps() {
        Ok(c) => c,
        Err(e) => skip(&format!("无 Vulkan 物理设备({})", e.trim())),
    };
    if let Err(e) = render_exec::require_wave(&caps, KernelWave::W2) {
        skip(&format!("W2 能力链缺失({e})"));
    }
    let cons = match caps.conservative_raster {
        Some(p) => p,
        None => fail(&format!(
            "fail-closed: 设备 `{}` 无 VK_EXT_conservative_rasterization(RXS-0303 L3;覆盖超集无保证,不静默降级)",
            caps.device_name
        )),
    };
    if !caps.fragment_stores_and_atomics {
        fail(&format!(
            "fail-closed: 设备 `{}` 无 fragmentStoresAndAtomics(FS 写 SSBO/原子前提)",
            caps.device_name
        ));
    }
    let validation_on = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    println!(
        "G9_M95_VB: device=`{}` W2+conservative+fs_atomics 在位 validation={}",
        caps.device_name,
        if validation_on { "on" } else { "off" }
    );

    // ── 步骤 1:场景 + device 蒙皮(M92 kernel;host 参照逐位锚内嵌)──
    let skin_spv = spv_bytes(&skin_kernel::m92_skin_spv(
        skin_kernel::M92_INFLUENCES,
        skin_kernel::M92_CLUSTER_BONES,
    ));
    let scene = match build_scene_with_device_skin(&skin_spv) {
        Ok(s) => s,
        Err(e) => fail(&format!("蒙皮 scene 构建(device kernel): {e}")),
    };
    let (triangles, ids, oracle) = project_visible_set(&scene);
    if ids.is_empty() {
        fail("场景零三角形(判据空转)");
    }
    let oracle_covered = oracle.count_valid() as u32;
    if oracle_covered == 0 {
        fail("oracle 零覆盖(判据空转)");
    }

    // ── 步骤 2:帧末 provenance + as_manager RT 消费锚 ──
    let (raster, rt, vsm) = (
        scene.set.feed_raster(),
        scene.set.feed_rt(),
        scene.set.feed_vsm(),
    );
    let provenance_ok =
        verify_frame_provenance(&scene.set, &raster, &rt, &vsm).is_ok();
    let rt_input = rt_blas_input_from_feed(&scene.set, &rt);
    let rt_consumed_ok = rt_input.as_ref().is_ok_and(|s| s.len() == 2);
    // 旁路重算 variant(serial 异)⇒ 消费锚必 RED(L4 双世界否决)。
    let mut bypass = scene.set.clone();
    bypass.frame_serial = 2;
    bypass.provenance_digest = compute_provenance_digest(&bypass);
    let rt_bypass_red = rt_blas_input_from_feed(&scene.set, &bypass.feed_rt()).is_err();
    if !provenance_ok || !rt_consumed_ok || !rt_bypass_red {
        fail(&format!(
            "帧末 provenance/消费锚断言失败:provenance={provenance_ok} consumed={rt_consumed_ok} bypass_red={rt_bypass_red}"
        ));
    }

    // ── 步骤 3:SW/HW 双腿真跑 + 位级 diff=0 ──
    let sw_spv = spv_bytes(&vbspv::sw_visbuffer_u64_spv());
    let vs_spv = spv_bytes(&vbspv::hw_visbuffer_vs_spv());
    let fs_spv = spv_bytes(&vbspv::hw_visbuffer_fs_spv());
    let vertex_bytes = build_vertex_stream(&triangles, &ids, None);
    let (sw, hw) = match execute_pair_frame(
        &sw_spv,
        &vs_spv,
        &fs_spv,
        &triangles,
        &ids,
        &vertex_bytes,
    ) {
        Ok(o) => o,
        Err(e) => fail(&format!("双腿执行: {e}")),
    };
    let diff_pixels = sw.iter().zip(&hw).filter(|(a, b)| a != b).count() as u32;
    let covered = |v: &[u64]| v.iter().filter(|&&w| w != VISBUFFER_CLEAR).count() as u32;
    let sw_covered = covered(&sw);
    let hw_covered = covered(&hw);
    let oracle_eq_sw = coverage_equal(&sw, &oracle.data);
    let oracle_eq_hw = coverage_equal(&hw, &oracle.data);
    let sw_digest = rurix_pkg::sha256::digest(
        &sw.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>(),
    );
    let hw_digest = rurix_pkg::sha256::digest(
        &hw.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>(),
    );
    println!(
        "G9_M95_VB: diff_pixels={diff_pixels} sw_covered={sw_covered} hw_covered={hw_covered} oracle_covered={oracle_covered}"
    );
    println!(
        "G9_M95_VB: sw_digest={} hw_digest={}",
        hex(&sw_digest),
        hex(&hw_digest)
    );

    // ── 步骤 4:RED 臂(device 侧有效)──
    // RED-a:篡改一像素(device 回读副本)⇒ 整数域 diff 必检出。
    let red_pixel = {
        let mut tampered = hw.clone();
        let pos = tampered
            .iter()
            .position(|&w| w != VISBUFFER_CLEAR)
            .expect("HW 有覆盖");
        tampered[pos] ^= 1; // tri7 最低位翻转
        let t_u64: Vec<u64> = tampered.clone();
        let a = VisBufferCpu {
            w: VIS_W,
            h: VIS_H,
            data: sw.clone(),
        };
        let b = VisBufferCpu {
            w: VIS_W,
            h: VIS_H,
            data: t_u64,
        };
        match visbuffer_diff_host(&a, &b) {
            Ok(d) => d.mismatched == 1,
            Err(_) => false,
        }
    };
    // RED-b:篡改受害三角形 HW 顶点流 ids.cluster += 1(SSBO/oracle 不动)
    // ⇒ 双腿 diff 必 > 0(device 数据流反证)。
    let victim = dominant_triangle(&oracle, &ids).expect("受害三角形");
    let tampered_stream = build_vertex_stream(&triangles, &ids, Some(victim));
    let red_ids = {
        let (_, hw2) = match execute_pair_frame(
            &sw_spv,
            &vs_spv,
            &fs_spv,
            &triangles,
            &ids,
            &tampered_stream,
        ) {
            Ok(o) => o,
            Err(e) => fail(&format!("RED-b 执行: {e}")),
        };
        let d = sw.iter().zip(&hw2).filter(|(a, b)| a != b).count() as u32;
        println!("G9_M95_VB: RED-b ids 篡改 diff={d}(必须 > 0)");
        d > 0
    };
    if !red_pixel {
        fail("RED-a 失效:篡改一像素未被 diff 检出");
    }
    if !red_ids {
        fail("RED-b 失效:ids 篡改后 diff 仍为 0");
    }

    // ── 步骤 5:判据汇总 + validation 计数面 ──
    let mut failures: Vec<String> = Vec::new();
    if diff_pixels != 0 {
        failures.push(format!("SW/HW 整数域 diff = {diff_pixels} ≠ 0(零容差破坏)"));
    }
    if sw_covered == 0 || hw_covered != sw_covered {
        failures.push(format!("覆盖退化:sw={sw_covered} hw={hw_covered}"));
    }
    if !oracle_eq_sw || !oracle_eq_hw {
        failures.push(format!(
            "oracle 覆盖集合失配:sw={oracle_eq_sw} hw={oracle_eq_hw}"
        ));
    }
    let validation_error_total = if validation_on {
        let n = render_exec::validation_error_total();
        if !render_exec::validation_messenger_installed() {
            failures.push("validation=on 但 messenger 未安装".to_string());
        }
        if n != 0 {
            failures.push(format!("validation error {n} ≠ 0"));
        }
        Some(n)
    } else {
        None
    };

    // ── 步骤 6:evidence JSON ──
    let checks: [(&str, bool); 11] = [
        ("sw_hw_diff_zero", diff_pixels == 0),
        ("sw_hw_coverage_nonzero", sw_covered > 0 && hw_covered == sw_covered),
        ("oracle_coverage_equal_sw", oracle_eq_sw),
        ("oracle_coverage_equal_hw", oracle_eq_hw),
        ("skin_device_bitexact", true), // build_scene_with_device_skin 内嵌逐位锚
        ("provenance_three_feeds", provenance_ok),
        ("rt_feed_consumed_as_manager", rt_consumed_ok),
        ("rt_bypass_red", rt_bypass_red),
        ("red_pixel_tamper", red_pixel),
        ("red_ids_tamper", red_ids),
        (
            "validation_error_zero",
            validation_error_total.is_none_or(|n| n == 0),
        ),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let failures_json: Vec<String> = failures.iter().map(|f| format!("\"{f}\"")).collect();
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m95.visbuffer_swhw.v1\",\n  \
         \"subject\": \"g9_m95_visbuffer_swhw\",\n  \
         \"device_state\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \
         \"validation_error_total\": {}, \"require_real\": {}}},\n  \
         \"checks\": {{{}}},\n  \
         \"resolution\": [{}, {}],\n  \"triangles\": {},\n  \
         \"diff_pixels\": {},\n  \"sw_covered\": {},\n  \"hw_covered\": {},\n  \
         \"oracle_covered\": {},\n  \
         \"sw_digest\": \"{}\",\n  \"hw_digest\": \"{}\",\n  \
         \"set_provenance_digest\": \"{}\",\n  \"rt_blas_input_count\": {},\n  \
         \"conservative_props\": {{\"primitive_overestimation_size\": {:.9e}, \
         \"max_extra_primitive_overestimation_size\": {:.9e}, \
         \"extra_primitive_overestimation_size_granularity\": {:.9e}, \
         \"degenerate_triangles_rasterized\": {}}},\n  \
         \"failures\": [{}]\n}}",
        caps.device_name,
        if validation_on { "on" } else { "off" },
        validation_error_total
            .map(|n| n.to_string())
            .unwrap_or_else(|| "null".to_string()),
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        checks_json.join(", "),
        VIS_W,
        VIS_H,
        ids.len() / 2,
        diff_pixels,
        sw_covered,
        hw_covered,
        oracle_covered,
        hex(&sw_digest),
        hex(&hw_digest),
        hex(&scene.set.provenance_digest),
        rt_input.map(|s| s.len()).unwrap_or(0),
        cons.primitive_overestimation_size,
        cons.max_extra_primitive_overestimation_size,
        cons.extra_primitive_overestimation_size_granularity,
        cons.degenerate_triangles_rasterized,
        failures_json.join(", "),
    );
    if let Some(p) = &evidence_path {
        std::fs::write(p, &json).expect("写 evidence");
    }
    println!("{json}");
    if failures.is_empty() {
        println!(
            "G9_M95_VB: PASS diff=0 sw=hw={} validation={}",
            hex(&sw_digest),
            if validation_on { "on(error=0)" } else { "off" }
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
