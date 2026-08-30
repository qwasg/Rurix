// Assisted-by: Claude（G37 W2 visbuffer）
// G37 W2 visbuffer：#74/#111 VisBuffer + classify/resolve 窗口生产证据臂共享体。
//
// 消费方（include! 拼接,g14_3_lane_body.rs 同型;lane body 本体 0-byte 不动）:
//   1) g31_window_present.rs —— 合入提案追加 `include!("g14_3_lane/g31_visbuffer_arm.rs")`
//      + `--visbuffer` 旗标族（合入由主 agent 做,提案见
//      artifacts/day_0830_delivery/w2_wiring/visbuffer/REPORT.md）;
//   2) g31_visbuffer_wiring.rs —— 独立接线 harness（合入前编译/device 冒烟,同一臂
//      函数单源消费,禁旁路复刻）。
//
// 机制链（day_0827 g31_cluster_cull_device --visbuffer 臂机制逐字承接,输入换成
// 真窗口相机 + 真场景簇包）:
//   逐样本相机 → 逐块生产金标准 cut（select_lod_cut_grouped 组共享判定球 +
//   verify_cut_coverage fail-closed 直调）→ compact_draw_args 32px SW/HW 分箱
//   （生产冻结件直调）→ SW 箱 compute 软光栅 device 真跑（M95 u64 原子腿
//   sw_visbuffer_u64_spv,分块 dispatch;u64 atomicMax 结合律跨块累积）→ device
//   双跑位级断言 + host VisBufferCpu oracle 覆盖集合全等断言（M95 oracle 口径:
//   打包深度位受 FMA/ULP 限制不进零容差判据——M95 门已锚 SW/HW device diff=0）
//   → HW 箱 host 保守光栅（HW device 图形腿 = M95 门 diff=0 锚复用登记,不重跑
//   图形管线——harness 同律）→ 哨兵感知 u64 max 合并 → classify/resolve 材质
//   分箱（#111 host 金标准直调;材质 = RXCP 逐簇继承材质 cluster_mat）→
//   merged VisBuffer sha256 digest + 全统计进独立 sidecar JSON。
//
// 诚实边界（evidence note 同文登记）:
//   - 证据臂:presented 面 0-byte（仍 ray 车道出帧）。出帧留窗 = #74 shade 桥
//     （classify/resolve 后简化着色进 presented 链,触 TSR/显示编码/Stage A 锚
//     重锚面）+ #75 生产 tile 化 compute 软光栅（本臂消费的 M95 kernel 为
//     O(tris×pixels) 蛮力,LocalSize 1×1×1——visbuffer 证据画布默认 96×54,
//     生产分辨率出帧物理上须 tile 化 kernel,归 #75 行,不在本臂私造）。
//   - passthrough 源三角（emissive + quad 灯面尾段 + 病态小块）在簇 DAG 外,
//     不进本臂光栅,计数如实登记（#58 同律:光源几何恒 passthrough）。
//   - 逐簇材质 = bake 期叶后代众数（多材质叶簇近似;SLAB_TRI_NONE →
//     VISBUFFER_MAT_NONE 专用槽,与 MATERIAL_INVALID 无效像素哨兵区分）。
//
// 拼接纪律:本文件不在模块顶层 use（lane body 已占 CullCamera/ClusterRecord/
// Mat4/vk/Path 等名字,顶层重复 import = E0252）;全部函数级 use。

/// 无材质簇的材质槽哨兵（cluster_mat == SLAB_TRI_NONE 的窄缓冲映射;
/// 与 material_pass::MATERIAL_INVALID(u16::MAX = 无效像素)错开一位）。
const VISBUFFER_MAT_NONE: u16 = u16::MAX - 1;

/// SW 软光栅单 dispatch 组数上界（M95 kernel LocalSize 1×1×1,groups.x =
/// chunk_tris·W·H;2³⁰ 留 NVIDIA 级 maxComputeWorkGroupCount[0]=2³¹−1 的
/// 半量裕度——分块 = 流序确定性切割,u64 atomicMax 交换结合 ⇒ 跨块累积
/// 与单发等价）。
const VISBUFFER_MAX_GROUPS: u64 = 1 << 30;

/// --visbuffer 臂选项（默认 off = 既有面 0-byte）。
#[allow(dead_code)]
struct VisBufferArmOpt {
    enabled: bool,
    /// VisBuffer 证据画布分辨率（默认 96×54 = harness 同值;蛮力 kernel
    /// O(tris×px),生产分辨率归 #75 tile 化）。
    res_w: u32,
    res_h: u32,
    /// 轨迹等距采样帧数（默认 3 = 首/中/末;窗口臂消费,独立 bin 自给样本）。
    samples: u32,
    /// sidecar JSON 路径（空 = 只打印不落盘;独立文件不动既有 evidence schema,
    /// #58/#95 同律）。
    out_path: String,
}

#[allow(dead_code)]
impl VisBufferArmOpt {
    fn off() -> Self {
        Self {
            enabled: false,
            res_w: 96,
            res_h: 54,
            samples: 3,
            out_path: String::new(),
        }
    }
}

/// 窗口会话相机样本（主循环内零成本采集;device 链循环后跑,不污染
/// real_render_frame_ms 口径）。
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct VisBufferCamSample {
    frame: u32,
    spec: CameraSpec,
    in_w: u32,
    in_h: u32,
}

/// 逐样本机制链统计（measured 如实登记;判据 = 覆盖全等/双跑位级/resolve
/// 恒等三断言 fail-closed,其余不设通过线）。
#[allow(dead_code)]
struct VisBufferFrameStat {
    frame: u32,
    cut_clusters: u32,
    cut_tris: u64,
    sw_clusters: u32,
    hw_clusters: u32,
    /// 投影后实际进流的三角数（clip.w ≤ 1e-20 整三角保守丢弃后）。
    sw_tris: u32,
    hw_tris: u32,
    sw_covered: u32,
    hw_covered: u32,
    merged_covered: u32,
    classify_buckets: u32,
    resolved_pixels: u32,
    /// SW 箱空流时 false（device 零 dispatch,如实登记不冒充）。
    sw_device_ran: bool,
    sw_chunks: u32,
    /// 合并 VisBuffer u64 LE 字节 sha256（覆盖集合 + 深度/簇/三角打包值
    /// 全量 digest;同相机同包确定性锚）。
    visbuffer_digest: String,
    cut_ms: f64,
    project_ms: f64,
    sw_device_ms: f64,
    oracle_ms: f64,
    classify_ms: f64,
}

/// 轨迹等距采样帧集（n=1 → {0};n≥2 → 0..=total-1 等距含首末;去重升序）。
#[allow(dead_code)]
fn visbuffer_sample_frames(total: u32, samples: u32) -> Vec<u32> {
    let last = total.saturating_sub(1);
    let n = samples.max(1);
    if n == 1 || last == 0 {
        return vec![0];
    }
    let mut out: Vec<u32> = (0..n)
        .map(|k| ((u64::from(k) * u64::from(last)) / u64::from(n - 1)) as u32)
        .collect();
    out.dedup();
    out
}

/// 全局分箱表（逐块拼接;一次构建全样本复用）。`records` 仅供
/// `compact_draw_args` 判据消费（center/radius/triangle_count——几何
/// offset 仍块内局部,光栅经 (block, local) 直读块池）。
#[allow(dead_code)]
struct VisBufferTables {
    records: Vec<ClusterRecord>,
    instances: Vec<rurix_render::geometry::gpu_scene::InstanceRecord>,
    block_base: Vec<u32>,
    /// 全局簇 → 材质槽（cluster_mat u32 收窄 u16;SLAB_TRI_NONE →
    /// VISBUFFER_MAT_NONE;越窄域 fail-closed）。
    mat16: Vec<u16>,
}

#[allow(dead_code)]
fn visbuffer_build_tables(pack: &ClusterPack) -> VisBufferTables {
    use rurix_render::geometry::gpu_scene::{IDENTITY_3X4, InstanceRecord, NO_PARENT};
    let mut records = Vec::new();
    let mut instances = Vec::new();
    let mut block_base = Vec::with_capacity(pack.blocks.len());
    let mut mat16 = Vec::new();
    for (bi, b) in pack.blocks.iter().enumerate() {
        let base = records.len() as u32;
        block_base.push(base);
        for (ci, r) in b.records.iter().enumerate() {
            // tri7 域契约（MAX_TRIS_PER_CLUSTER = 128 ⇒ tri 索引 ≤ 127 恰入
            // 7 位;越域 = 簇包契约破坏,fail-closed）。
            if r.triangle_count > 128 {
                fail(&format!(
                    "--visbuffer 块 {bi} 簇 {ci} triangle_count={} 越 tri7 域（≤128 契约破坏）",
                    r.triangle_count
                ));
            }
            records.push(*r);
            let m = b.cluster_mat[ci];
            mat16.push(if m == SLAB_TRI_NONE {
                VISBUFFER_MAT_NONE
            } else if m < u32::from(VISBUFFER_MAT_NONE) {
                m as u16
            } else {
                fail(&format!(
                    "--visbuffer 块 {bi} 簇 {ci} cluster_mat={m} 越 16 位窄缓冲域"
                ))
            });
        }
        instances.push(InstanceRecord {
            transform: IDENTITY_3X4,
            cluster_offset: base,
            cluster_count: b.records.len() as u32,
            material_id: 0,
            flags: 0,
            aabb_min: [-1e9; 3],
            mesh_id: bi as u32,
            aabb_max: [1e9; 3],
            reserved: NO_PARENT,
        });
    }
    VisBufferTables {
        records,
        instances,
        block_base,
        mat16,
    }
}

/// 世界三角 → visbuffer 画布屏幕三角（`visbuffer::raster_cluster_tris` 投影
/// 口径逐字;clip.w ≤ 1e-20 整三角保守丢弃——裁决 4 近平面 P0 简化）。
#[allow(dead_code)]
fn visbuffer_project_tri(
    vp: &Mat4,
    world: &[[f32; 3]; 3],
    vw: u32,
    vh: u32,
) -> Option<[[f32; 3]; 3]> {
    let (w_px, h_px) = (vw as f32, vh as f32);
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

/// 块局部簇第 t 个世界三角（RXCP 几何段:u8 局部索引 + 块顶点池;bistro
/// 三角汤已烘焙世界空间,identity 实例——`verify_cluster_pack` 叶级位级
/// 复核在装配期已过）。
#[allow(dead_code)]
fn visbuffer_cluster_world_tri(b: &ClusterPackBlock, r: &ClusterRecord, t: u32) -> [[f32; 3]; 3] {
    let ti = r.triangle_offset as usize + 3 * t as usize;
    let mut out = [[0.0f32; 3]; 3];
    for k in 0..3 {
        let li = b.triangle_indices[ti + k] as usize + r.vertex_offset as usize;
        out[k] = b.vertices[li];
    }
    out
}

/// 单样本机制链（fail-closed 三断言:SW device 双跑位级 / SW device 覆盖集合
/// 与 host oracle 全等 / resolve 像素数 == 合并覆盖数）。
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn visbuffer_run_sample(
    pack: &ClusterPack,
    tables: &VisBufferTables,
    sw_spv: &[u32],
    sw_entry: &str,
    s: &VisBufferCamSample,
    threshold_px: f32,
    vw: u32,
    vh: u32,
) -> VisBufferFrameStat {
    use rurix_render::geometry::cull::{DEFAULT_BIN_THRESHOLD_PX, VisibleCluster, compact_draw_args};
    use rurix_render::geometry::gpu_scene::IDENTITY_3X4;
    use rurix_render::geometry::material_pass::{MATERIAL_INVALID, classify, resolve};
    use rurix_render::geometry::visbuffer::{VISBUFFER_CLEAR, VisBufferCpu};
    use rurix_render::geometry::visible_cluster_set::{
        MeshDagView, select_lod_cut_grouped, verify_cut_coverage,
    };
    use std::collections::HashMap;

    // ── ① 逐块生产金标准 cut（相机 = 真窗口样本;LOD 判据在内部分辨率——
    //      cluster_lod_frame_stat 同口径;覆盖性机核逐样本 fail-closed）──
    let t_cut = std::time::Instant::now();
    let cam = cluster_cull_camera(&s.spec, s.in_w, s.in_h, threshold_px);
    let mut visible: Vec<VisibleCluster> = Vec::new();
    let mut cut_tris = 0u64;
    for (bi, b) in pack.blocks.iter().enumerate() {
        let view = MeshDagView::new(&b.records, &b.nodes, &b.children)
            .unwrap_or_else(|e| fail(&format!("--visbuffer 块 {bi} DAG 拓扑: {e}")));
        let cut = select_lod_cut_grouped(
            &view,
            &b.cluster_self_lod,
            &b.cluster_parent_lod,
            &IDENTITY_3X4,
            &cam,
        );
        verify_cut_coverage(&view, &cut).unwrap_or_else(|e| {
            fail(&format!(
                "--visbuffer 帧 {} 块 {bi} cut 覆盖性: {e}",
                s.frame
            ))
        });
        for &c in &cut {
            cut_tris += u64::from(b.records[c as usize].triangle_count);
            visible.push(VisibleCluster {
                instance: bi as u32,
                cluster: tables.block_base[bi] + c,
            });
        }
    }
    let cut_ms = t_cut.elapsed().as_secs_f64() * 1e3;
    if visible.is_empty() {
        fail(&format!("--visbuffer 帧 {} cut 可见簇为零（防空接线）", s.frame));
    }
    // cluster27 载荷 = 帧内可见列表下标（Nanite 口径,harness 同律）——27 位域界。
    if visible.len() >= (1 << 27) {
        fail(&format!(
            "--visbuffer 帧 {} 可见簇 {} 越 cluster27 域",
            s.frame,
            visible.len()
        ));
    }

    // ── ② SW/HW 分箱（生产冻结件 compact_draw_args 直调,32px 投影直径阈）──
    let args = compact_draw_args(
        &visible,
        &tables.instances,
        &tables.records,
        &cam,
        DEFAULT_BIN_THRESHOLD_PX,
    );
    let entry_of: HashMap<u32, u32> = visible
        .iter()
        .enumerate()
        .map(|(e, vc)| (vc.cluster, e as u32))
        .collect();
    let cluster_to_material: Vec<u16> = visible
        .iter()
        .map(|vc| tables.mat16[vc.cluster as usize])
        .collect();

    // ── ③ 投影 + 双箱流/oracle（SW = device 流 + host oracle;HW = host
    //      保守光栅,device 图形腿 = M95 门 diff=0 锚复用登记）──
    let t_proj = std::time::Instant::now();
    let vp = Mat4 { m: cam.view_proj };
    let mut sw_tris_f32: Vec<f32> = Vec::new();
    let mut sw_ids: Vec<u32> = Vec::new();
    let mut host_sw = VisBufferCpu::new(vw, vh);
    let mut sw_tri_count = 0u32;
    let mut host_hw = VisBufferCpu::new(vw, vh);
    let mut hw_tri_count = 0u32;
    for (list, is_sw) in [(&args.sw_clusters, true), (&args.hw_clusters, false)] {
        for vc in list.iter() {
            let bi = vc.instance as usize;
            let b = &pack.blocks[bi];
            let local = (vc.cluster - tables.block_base[bi]) as usize;
            let r = &b.records[local];
            let entry = entry_of[&vc.cluster];
            for t in 0..r.triangle_count {
                let world = visbuffer_cluster_world_tri(b, r, t);
                if let Some(screen) = visbuffer_project_tri(&vp, &world, vw, vh) {
                    if is_sw {
                        for sv in &screen {
                            sw_tris_f32.extend_from_slice(sv);
                        }
                        sw_ids.extend_from_slice(&[entry, t]);
                        host_sw.raster_triangle(&screen, entry, t);
                        sw_tri_count += 1;
                    } else {
                        host_hw.raster_triangle(&screen, entry, t);
                        hw_tri_count += 1;
                    }
                }
            }
        }
    }
    let mut oracle_ms = t_proj.elapsed().as_secs_f64() * 1e3;
    let project_ms = oracle_ms; // 投影与 host 光栅同趟（拆分无消费面,合并登记）

    // ── ④ SW compute 软光栅 device 真跑（M95 u64 原子腿;分块 dispatch,
    //      atomicMax 交换结合 ⇒ 跨块累积与单发等价;双跑位级断言）──
    let px_count = u64::from(vw) * u64::from(vh);
    let chunk_tris = (VISBUFFER_MAX_GROUPS / px_count.max(1)).max(1) as usize;
    let sw_chunks = if sw_tri_count == 0 {
        0
    } else {
        (sw_tri_count as usize).div_ceil(chunk_tris) as u32
    };
    let clear_bytes: Vec<u8> = std::iter::repeat_n(VISBUFFER_CLEAR.to_le_bytes(), px_count as usize)
        .flatten()
        .collect();
    let bytes_f32 = |v: &[f32]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
    let bytes_u32 = |v: &[u32]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
    let run_device = || -> Vec<u64> {
        let mut vis_bytes = clear_bytes.clone();
        let mut done = 0usize;
        while done < sw_tri_count as usize {
            let n = chunk_tris.min(sw_tri_count as usize - done);
            let mut bufs: Vec<Vec<u8>> = vec![
                bytes_f32(&sw_tris_f32[done * 9..(done + n) * 9]),
                bytes_u32(&sw_ids[done * 2..(done + n) * 2]),
                vis_bytes,
            ];
            vk::run_compute(
                sw_spv,
                sw_entry,
                &mut bufs,
                &bytes_u32(&[n as u32, vw, vh]),
                [(n as u64 * px_count) as u32, 1, 1],
            )
            .unwrap_or_else(|e| fail(&format!("--visbuffer SW 软光栅 dispatch 失败: {e}")));
            vis_bytes = bufs.pop().expect("vis 缓冲在位");
            done += n;
        }
        vis_bytes
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
            .collect()
    };
    let t_dev = std::time::Instant::now();
    let (dev_sw, sw_device_ran) = if sw_tri_count == 0 {
        (host_sw.data.clone(), false) // 空流:零 dispatch,如实登记
    } else {
        let a = run_device();
        let b = run_device();
        if a != b {
            fail(&format!(
                "--visbuffer 帧 {} SW device 双跑位级漂移（u64 原子 max 序无关性破坏）",
                s.frame
            ));
        }
        (a, true)
    };
    let sw_device_ms = t_dev.elapsed().as_secs_f64() * 1e3;

    // 覆盖集合对拍（M95 oracle 口径:host CPU 光栅 = 覆盖集合金标准;打包
    // 深度位受 FMA/ULP 限制不进零容差判据——M95 已锚 SW/HW device diff=0）。
    let t_or = std::time::Instant::now();
    if sw_device_ran {
        let dev_cover: Vec<bool> = dev_sw.iter().map(|&w| w != VISBUFFER_CLEAR).collect();
        let host_cover: Vec<bool> = host_sw.data.iter().map(|&w| w != VISBUFFER_CLEAR).collect();
        if dev_cover != host_cover {
            let miss = dev_cover
                .iter()
                .zip(&host_cover)
                .position(|(a, b)| a != b)
                .unwrap();
            fail(&format!(
                "--visbuffer 帧 {} SW 覆盖集合失配: 像素 {miss} device={} host={}",
                s.frame, dev_cover[miss], host_cover[miss]
            ));
        }
    }
    oracle_ms += t_or.elapsed().as_secs_f64() * 1e3;
    let count_cover = |v: &[u64]| v.iter().filter(|&&w| w != VISBUFFER_CLEAR).count() as u32;
    let sw_covered = count_cover(&dev_sw);
    let hw_covered = count_cover(&host_hw.data);

    // ── ⑤ 合并（哨兵感知 u64 max = VisBuffer 原子语义）→ classify/resolve
    //      （#111 材质分箱 host 金标准直调）→ digest ──
    let t_cls = std::time::Instant::now();
    let merged = VisBufferCpu {
        w: vw,
        h: vh,
        data: dev_sw
            .iter()
            .zip(&host_hw.data)
            .map(|(&a, &b)| match (a != VISBUFFER_CLEAR, b != VISBUFFER_CLEAR) {
                (true, true) => a.max(b),
                (true, false) => a,
                (false, true) => b,
                (false, false) => VISBUFFER_CLEAR,
            })
            .collect(),
    };
    let merged_covered = count_cover(&merged.data);
    if merged_covered == 0 {
        fail(&format!(
            "--visbuffer 帧 {} 合并零覆盖（cut {} 簇下防空接线）",
            s.frame,
            visible.len()
        ));
    }
    let cls = classify(&merged, &cluster_to_material, 16);
    let resolved = resolve(&merged, &cluster_to_material);
    let resolved_pixels = resolved.iter().filter(|&&m| m != MATERIAL_INVALID).count() as u32;
    if resolved_pixels != merged_covered {
        fail(&format!(
            "--visbuffer 帧 {} resolve 像素数 {resolved_pixels} ≠ 合并覆盖 {merged_covered}（材质解析面破坏）",
            s.frame
        ));
    }
    if cls.buckets.is_empty() {
        fail(&format!(
            "--visbuffer 帧 {} classify 零桶（材质分箱面空转）",
            s.frame
        ));
    }
    let classify_ms = t_cls.elapsed().as_secs_f64() * 1e3;
    let merged_bytes: Vec<u8> = merged.data.iter().flat_map(|w| w.to_le_bytes()).collect();
    let visbuffer_digest = format!("sha256:{}", sha256_hex(&merged_bytes));

    VisBufferFrameStat {
        frame: s.frame,
        cut_clusters: visible.len() as u32,
        cut_tris,
        sw_clusters: args.sw_cluster_count,
        hw_clusters: args.hw_cluster_count,
        sw_tris: sw_tri_count,
        hw_tris: hw_tri_count,
        sw_covered,
        hw_covered,
        merged_covered,
        classify_buckets: cls.buckets.len() as u32,
        resolved_pixels,
        sw_device_ran,
        sw_chunks,
        visbuffer_digest,
        cut_ms,
        project_ms,
        sw_device_ms,
        oracle_ms,
        classify_ms,
    }
}

/// 臂编排（两 bin 单源消费面）:全局表一次构建 → 逐样本机制链 + 打印。
/// 前置:调用方已保证 Vulkan 在场（窗口会话/独立 bin 三态先行）。
#[allow(dead_code)]
fn run_visbuffer_arm(
    tag: &str,
    pack: &ClusterPack,
    opt: &VisBufferArmOpt,
    threshold_px: f32,
    samples: &[VisBufferCamSample],
) -> Vec<VisBufferFrameStat> {
    use rurix_render::geometry::visbuffer_swhw_spv::sw_visbuffer_u64_spv;
    if samples.is_empty() {
        fail("--visbuffer 零相机样本（采集面破坏）");
    }
    let sw_spv = sw_visbuffer_u64_spv();
    let sw_entry =
        vk::entry_point_name(&sw_spv).unwrap_or_else(|| fail("SW SPV 无 OpEntryPoint"));
    let tables = visbuffer_build_tables(pack);
    let mut out = Vec::with_capacity(samples.len());
    for s in samples {
        let st = visbuffer_run_sample(
            pack,
            &tables,
            &sw_spv,
            &sw_entry,
            s,
            threshold_px,
            opt.res_w,
            opt.res_h,
        );
        eprintln!(
            "{tag}: visbuffer 帧 {} cut={}簇/{}tri 分箱 sw/hw={}/{}簇 {}/{}tri 覆盖 sw/hw/merged={}/{}/{} 桶={} device={}块 {:.1}ms（SW 覆盖与 oracle 全等 + 双跑位级;HW device 腿 = M95 diff=0 锚复用）",
            st.frame,
            st.cut_clusters,
            st.cut_tris,
            st.sw_clusters,
            st.hw_clusters,
            st.sw_tris,
            st.hw_tris,
            st.sw_covered,
            st.hw_covered,
            st.merged_covered,
            st.classify_buckets,
            st.sw_chunks,
            st.sw_device_ms,
        );
        out.push(st);
    }
    out
}

/// sidecar JSON（独立文件不动既有 evidence schema——#58/#95 同律;
/// measured 如实登记,通过线 = 链内三断言 fail-closed 已在跑时执行）。
#[allow(dead_code)]
fn visbuffer_stats_json(
    pack: &ClusterPack,
    opt: &VisBufferArmOpt,
    threshold_px: f32,
    stats: &[VisBufferFrameStat],
) -> String {
    use rurix_render::geometry::cull::DEFAULT_BIN_THRESHOLD_PX;
    let total_clusters: usize = pack.blocks.iter().map(|b| b.records.len()).sum();
    let mut sj = String::with_capacity(2048 + stats.len() * 256);
    sj.push_str(&format!(
        "{{\"schema\":\"rurix.g31.visbuffer_stats.v1\",\"resolution\":[{},{}],\"bin_threshold_px\":{DEFAULT_BIN_THRESHOLD_PX},\"tile_size\":16,\"cut_threshold_px\":{threshold_px},\"blocks\":{},\"total_clusters\":{total_clusters},\"passthrough_tris\":{},\"samples\":{},\"sw_device_coverage_match\":true,\"sw_double_run_bitexact\":true,\"hw_leg\":\"host_conservative_model(M95 gate diff=0 anchor reuse)\",\"note\":\"G37 W2 #74/#111 窗口生产证据臂:真窗口相机样本 × 真场景簇包 device 真跑机制链(cut→32px 分箱→SW compute 软光栅 u64 原子→覆盖对拍→合并→classify/resolve);presented 面 0-byte(仍 ray 车道出帧)——出帧留窗 = #74 shade 桥 + #75 生产 tile 化(本臂 M95 蛮力 kernel O(tris×px),证据画布降维);passthrough(emissive+灯面尾段)在簇 DAG 外不进光栅;逐簇材质 = bake 叶后代众数\",\"frames\":[",
        opt.res_w,
        opt.res_h,
        pack.blocks.len(),
        pack.passthrough.len(),
        stats.len(),
    ));
    for (k, st) in stats.iter().enumerate() {
        if k > 0 {
            sj.push(',');
        }
        sj.push_str(&format!(
            "{{\"frame\":{},\"cut_clusters\":{},\"cut_tris\":{},\"sw_clusters\":{},\"hw_clusters\":{},\"sw_tris\":{},\"hw_tris\":{},\"sw_covered\":{},\"hw_covered\":{},\"merged_covered\":{},\"classify_buckets\":{},\"resolved_pixels\":{},\"sw_device_ran\":{},\"sw_chunks\":{},\"visbuffer_digest\":{},\"ms\":{{\"cut\":{:.3},\"project_oracle\":{:.3},\"sw_device\":{:.3},\"coverage_check\":{:.3},\"classify_resolve\":{:.3}}}}}",
            st.frame,
            st.cut_clusters,
            st.cut_tris,
            st.sw_clusters,
            st.hw_clusters,
            st.sw_tris,
            st.hw_tris,
            st.sw_covered,
            st.hw_covered,
            st.merged_covered,
            st.classify_buckets,
            st.resolved_pixels,
            st.sw_device_ran,
            st.sw_chunks,
            jstr(&st.visbuffer_digest),
            st.cut_ms,
            st.project_ms,
            st.sw_device_ms,
            st.oracle_ms - st.project_ms,
            st.classify_ms,
        ));
    }
    sj.push_str("]}");
    sj
}

/// sidecar 落盘 + 汇总打印（窗口合入面的单行调用点;out_path 空 = 只打印）。
#[allow(dead_code)]
fn visbuffer_finish(
    tag: &str,
    pack: &ClusterPack,
    opt: &VisBufferArmOpt,
    threshold_px: f32,
    stats: &[VisBufferFrameStat],
) {
    let dev_total: f64 = stats.iter().map(|s| s.sw_device_ms).sum();
    eprintln!(
        "{tag}: visbuffer 臂 OK samples={} res={}x{} sw_device_ms_total={:.1}（#74/#111 机制链窗口证据臂;presented 面 0-byte,出帧留窗 = #74 shade 桥 + #75 tile 化）",
        stats.len(),
        opt.res_w,
        opt.res_h,
        dev_total,
    );
    if !opt.out_path.is_empty() {
        let sj = visbuffer_stats_json(pack, opt, threshold_px, stats);
        if let Some(parent) = Path::new(&opt.out_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&opt.out_path, sj.as_bytes())
            .unwrap_or_else(|e| fail(&format!("visbuffer sidecar 写盘 {}: {e}", opt.out_path)));
        eprintln!("{tag}: visbuffer 统计 sidecar → {}", opt.out_path);
    }
}
