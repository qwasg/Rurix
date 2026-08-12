//! G9.3 M94 CLAS×RT 合流 device 对拍 harness(RXS-0351;门
//! `g9.p0.m94.clas_rt_convergence`;U56 探测/主腿 lane)。
//!
//! ## 判据(逐命中一致,容差 0)
//! 确定性簇集场景(`rt_clas::m94_fixture`,6 簇:5 四边形同拓扑 → Cluster
//! Template 实例化臂 + 1 单三角形异拓扑 → 直建臂)× 9 条确定性光线:
//! - **回退腿**(传统 triangles BLAS,per-簇分组 = 按对象,RXS-0351 L2)经
//!   `run_ray_query_effects`(U30 单所有者 `VkAsManager`)真跑;
//! - **主腿**(CLAS 当帧 multi-indirect device 拼装 + 模板实例化 + cluster BLAS
//!   + 标准 KHR TLAS)经 `run_clas_main_leg_ray_query`(U56 lane)真跑;
//! - 双腿命中流(committed/t_bits/cluster_slot/primitive 逐光线)**逐命中一致,
//!   容差 0**,并与 host 金标准(`rt_clas::host_trace_clusters`,Möller–Trumbore
//!   双面最近命中)逐命中一致;命中流 digest 打印为 evidence(沿 G7 RayQuery
//!   对拍体例);
//! - **主腿 not-supported 时**(capability snapshot fail-closed):主腿诚实登记
//!   DEV_ENV_DEGRADE,判据面 = 回退腿 vs host 金标准逐命中一致(替代口径如实
//!   登记),不以 host 模拟充绿。
//!
//! ## RED 臂
//! - `cluster-mismatch`:可见集与装配产物错开一簇 → 装配期核验必 RED
//!   (`rt_clas::verify_visible_blas_consistency`,RXS-0351 L3);
//! - `device-drift`:篡改一簇几何的回退腿 device 命中流 vs 正确 golden →
//!   比较轴必须检出(能红反证);
//! - `leg-switch`:主腿 capability 缺失时 `select_leg(ClasMain)` 必 fail-closed
//!   (L7 禁静默换腿)。
//!
//! ## 静态帧零 AS 构建(L4)
//! 同可见集二次装配 → `ClasAsStats` 构建计数零增量(非零即 RED);harness
//! 打印计数面供 evidence。
//!
//! ## 三态
//! 无 Vulkan loader/设备 → `CLAS_RT: SKIP`(dev-env degrade,退 0,非 fake pass);
//! 判据不符 / RED 轴失效 / validation 报错 → `CLAS_RT: FAIL` 退 1。
//! `RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层裁决;`RURIX_VK_VALIDATION=1`
//! 装载 validation messenger(两 lane 内 fail-closed)。

use rurix_rt::rt_clas::{
    ClasAssembler, ClasLegSupport, HitRecord, RtLeg, hit_stream_digest, host_trace_clusters,
    m94_fixture, m94_fixture_expected, select_leg, verify_visible_blas_consistency,
};
use rurix_rt::vk::{
    RayQueryBufferDesc, RayQueryDispatchDesc, RayQueryInstanceDesc, RayQuerySceneDesc,
    probe_cluster_acceleration_structure, run_clas_main_leg_ray_query, run_ray_query_effects,
};

/// 无设备/加载器(SKIP)信号(镜像 `bin/vk_ray_query` NO_DEVICE_KEYS 纪律)。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "compute queue",
    "vkCreateInstance",
];

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

fn fail(msg: &str) -> ! {
    eprintln!("CLAS_RT: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 手编 ray query compute SPIR-V(双腿同一内核;无外部汇编器,沿 vk_desc_v3 先例)
// ---------------------------------------------------------------------------
//
// set0:binding0 = TLAS(AccelerationStructureKHR)/ binding1 = 光线 SSBO
// (8 f32/光线:ox,oy,oz,dx,dy,dz,tmin,tmax)/ binding2 = 输出 SSBO
// (4 u32/光线:committed,t_bits,cluster_slot_or_geometry_index,primitive)。
// 每 invocation 一条光线(LocalSize 1×1×1,groups.x = 光线数)。
// 指令面:OpTypeRayQueryKHR/Initialize(flags=Opaque,mask=0xFF)/Proceed 循环/
// GetIntersectionType(Committed)!=None 守卫/命中三查询(InstanceId/PrimitiveIndex/T)。
/// 命中槽位来源(双腿统一元组的桥):回退腿 = TLAS 实例槽位(逐簇实例);
/// 主腿 = CLAS geometry index(单实例 cluster BLAS,槽位由 CLAS 携带)。
fn m94_ray_query_spv(slot_from_geometry: bool) -> Vec<u32> {
    fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
        v.push(op | ((ops.len() as u32 + 1) << 16));
        v.extend_from_slice(ops);
    }
    fn words(s: &str) -> Vec<u32> {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        while b.len() % 4 != 0 {
            b.push(0);
        }
        b.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }
    // header: magic / version 1.4(ray query 既有产物同版本)/ generator 0 /
    // bound 100 / schema 0。
    let mut v = vec![0x0723_0203u32, 0x0001_0400, 0, 128, 0];
    inst(&mut v, 17, &[1]); // OpCapability Shader
    inst(&mut v, 17, &[4472]); // OpCapability RayQueryKHR
    let mut ext = vec![];
    ext.extend(words("SPV_KHR_ray_query"));
    inst(&mut v, 10, &ext); // OpExtension "SPV_KHR_ray_query"
    inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, 1];
    ep.extend(words("main"));
    // SPIR-V 1.4 interface 全量枚举静态使用全局变量(gid/TLAS/rays/out)。
    ep.extend_from_slice(&[10, 13, 17, 21]);
    inst(&mut v, 15, &ep); // OpEntryPoint GLCompute %1 "main" ...
    inst(&mut v, 16, &[1, 17, 1, 1, 1]); // OpExecutionMode %1 LocalSize 1 1 1

    // ── 注解 ──
    inst(&mut v, 71, &[10, 11, 28]); // %10 BuiltIn GlobalInvocationId
    inst(&mut v, 71, &[13, 34, 0]); // %13 DescriptorSet 0(TLAS)
    inst(&mut v, 71, &[13, 33, 0]); // %13 Binding 0
    inst(&mut v, 71, &[17, 34, 0]); // %17 DescriptorSet 0(rays)
    inst(&mut v, 71, &[17, 33, 1]); // %17 Binding 1
    inst(&mut v, 71, &[21, 34, 0]); // %21 DescriptorSet 0(out)
    inst(&mut v, 71, &[21, 33, 2]); // %21 Binding 2
    inst(&mut v, 71, &[15, 2]); // %15 Block(rays SSBO struct)
    inst(&mut v, 72, &[15, 0, 35, 0]); // %15 member0 Offset 0
    inst(&mut v, 71, &[19, 2]); // %19 Block(out SSBO struct)
    inst(&mut v, 72, &[19, 0, 35, 0]); // %19 member0 Offset 0
    inst(&mut v, 71, &[14, 6, 4]); // %14 ArrayStride 4
    inst(&mut v, 71, &[18, 6, 4]); // %18 ArrayStride 4

    // ── 类型 / 常量 / 全局变量 ──
    inst(&mut v, 19, &[2]); // %2 = OpTypeVoid
    inst(&mut v, 33, &[3, 2]); // %3 = OpTypeFunction %2
    inst(&mut v, 21, &[4, 32, 0]); // %4 = OpTypeInt 32 0 (u32)
    inst(&mut v, 22, &[5, 32]); // %5 = OpTypeFloat 32
    inst(&mut v, 20, &[6]); // %6 = OpTypeBool
    inst(&mut v, 23, &[7, 5, 3]); // %7 = OpTypeVector %5 3 (vec3f)
    inst(&mut v, 23, &[8, 4, 3]); // %8 = OpTypeVector %4 3 (uvec3)
    inst(&mut v, 32, &[9, 1, 8]); // %9 = OpTypePointer Input %8
    inst(&mut v, 59, &[9, 10, 1]); // %10 = OpVariable %9 Input (gl_GlobalInvocationID)
    inst(&mut v, 5341, &[11]); // %11 = OpTypeAccelerationStructureKHR
    inst(&mut v, 32, &[12, 0, 11]); // %12 = OpTypePointer UniformConstant %11
    inst(&mut v, 59, &[12, 13, 0]); // %13 = OpVariable %12 UniformConstant (TLAS)
    inst(&mut v, 29, &[14, 5]); // %14 = OpTypeRuntimeArray %5 (f32 光线)
    inst(&mut v, 30, &[15, 14]); // %15 = OpTypeStruct %14 (Block)
    inst(&mut v, 32, &[16, 12, 15]); // %16 = OpTypePointer StorageBuffer %15
    inst(&mut v, 59, &[16, 17, 12]); // %17 = OpVariable %16 StorageBuffer (rays)
    inst(&mut v, 29, &[18, 4]); // %18 = OpTypeRuntimeArray %4 (u32 输出)
    inst(&mut v, 30, &[19, 18]); // %19 = OpTypeStruct %18 (Block)
    inst(&mut v, 32, &[20, 12, 19]); // %20 = OpTypePointer StorageBuffer %19
    inst(&mut v, 59, &[20, 21, 12]); // %21 = OpVariable %20 StorageBuffer (out)
    inst(&mut v, 32, &[22, 12, 5]); // %22 = OpTypePointer StorageBuffer %5
    inst(&mut v, 32, &[23, 12, 4]); // %23 = OpTypePointer StorageBuffer %4
    inst(&mut v, 4472, &[24]); // %24 = OpTypeRayQueryKHR
    inst(&mut v, 32, &[25, 7, 24]); // %25 = OpTypePointer Function %24
    inst(&mut v, 43, &[4, 26, 0]); // %26 = u32 0
    inst(&mut v, 43, &[4, 27, 1]); // %27 = u32 1(ray flags Opaque / committed)
    inst(&mut v, 43, &[4, 28, 4]); // %28 = u32 4
    inst(&mut v, 43, &[4, 29, 8]); // %29 = u32 8
    inst(&mut v, 43, &[4, 30, 0xFF]); // %30 = u32 0xFF(cull mask)
    inst(&mut v, 43, &[4, 32, 2]); // %32 = u32 2
    inst(&mut v, 43, &[4, 33, 3]); // %33 = u32 3
    inst(&mut v, 43, &[4, 34, 5]); // %34 = u32 5
    inst(&mut v, 43, &[4, 35, 6]); // %35 = u32 6
    inst(&mut v, 43, &[4, 36, 7]); // %36 = u32 7

    // ── 函数体 ──
    inst(&mut v, 54, &[2, 1, 0, 3]); // %1 = OpFunction %2 None %3
    inst(&mut v, 248, &[40]); // %40 = OpLabel(首块)
    inst(&mut v, 59, &[25, 41, 7]); // %41 = OpVariable %25 Function (ray query)
    inst(&mut v, 61, &[8, 42, 10]); // %42 = load gid (uvec3)
    inst(&mut v, 81, &[4, 43, 42, 0]); // %43 = i = gid.x
    inst(&mut v, 132, &[4, 44, 43, 29]); // %44 = base = i*8
    // 读 8 个 f32:rays[base+k](k=0..7)。
    let offs = [26u32, 27, 32, 33, 28, 34, 35, 36]; // 常量 0..7 的 id
    let mut next_id = 45u32;
    let mut val_ids = [0u32; 8];
    for (k, slot) in val_ids.iter_mut().enumerate() {
        let idx_id = if k == 0 {
            44
        } else {
            let id = next_id;
            next_id += 1;
            inst(&mut v, 128, &[4, id, 44, offs[k]]); // base+k
            id
        };
        let addr_id = next_id;
        next_id += 1;
        inst(&mut v, 65, &[22, addr_id, 17, 26, idx_id]); // &rays[base+k]
        let val_id = next_id;
        next_id += 1;
        inst(&mut v, 61, &[5, val_id, addr_id]); // load f32
        *slot = val_id;
    }
    // val_ids = [ox,oy,oz,dx,dy,dz,tmin,tmax]
    let origin = next_id;
    next_id += 1;
    inst(&mut v, 80, &[7, origin, val_ids[0], val_ids[1], val_ids[2]]); // origin
    let dir = next_id;
    next_id += 1;
    inst(&mut v, 80, &[7, dir, val_ids[3], val_ids[4], val_ids[5]]); // dir
    let as_id = next_id;
    next_id += 1;
    inst(&mut v, 61, &[11, as_id, 13]); // load TLAS
    // OpRayQueryInitializeKHR %rq %as flags=Opaque mask=0xFF origin tmin dir tmax
    inst(
        &mut v,
        4473,
        &[41, as_id, 27, 30, origin, val_ids[6], dir, val_ids[7]],
    );
    let loop_lbl = next_id;
    next_id += 1;
    let cont_lbl = next_id;
    next_id += 1;
    let after_lbl = next_id;
    next_id += 1;
    inst(&mut v, 249, &[loop_lbl]); // OpBranch %loop
    inst(&mut v, 248, &[loop_lbl]); // %loop:
    let cond = next_id;
    next_id += 1;
    inst(&mut v, 4477, &[6, cond, 41]); // %cond = OpRayQueryProceedKHR %rq
    inst(&mut v, 246, &[after_lbl, cont_lbl, 0]); // OpLoopMerge %after %cont None
    inst(&mut v, 250, &[cond, cont_lbl, after_lbl]); // OpBranchConditional
    inst(&mut v, 248, &[cont_lbl]); // %cont:
    inst(&mut v, 249, &[loop_lbl]); // OpBranch %loop
    inst(&mut v, 248, &[after_lbl]); // %after:
    let ty = next_id;
    next_id += 1;
    inst(&mut v, 4479, &[4, ty, 41, 27]); // %ty = GetIntersectionType %rq Committed
    let has = next_id;
    next_id += 1;
    inst(&mut v, 171, &[6, has, ty, 26]); // %has = %ty != 0
    let hit_lbl = next_id;
    next_id += 1;
    let miss_lbl = next_id;
    next_id += 1;
    let merge_lbl = next_id;
    next_id += 1;
    inst(&mut v, 247, &[merge_lbl, 0]); // OpSelectionMerge %merge None
    inst(&mut v, 250, &[has, hit_lbl, miss_lbl]);
    // hit 臂:out[4i+0..4] = [1, t_bits, instance_id, primitive]
    inst(&mut v, 248, &[hit_lbl]);
    let t_id = next_id;
    next_id += 1;
    inst(&mut v, 6018, &[5, t_id, 41, 27]); // committed T
    let inst_id = next_id;
    next_id += 1;
    // 槽位源:回退腿 InstanceId(6020)/ 主腿 GeometryIndex(6022)——单指令分叉。
    let slot_op = if slot_from_geometry { 6022 } else { 6020 };
    inst(&mut v, slot_op, &[4, inst_id, 41, 27]); // committed slot
    let prim_id = next_id;
    next_id += 1;
    inst(&mut v, 6023, &[4, prim_id, 41, 27]); // committed PrimitiveIndex
    let tbits = next_id;
    next_id += 1;
    inst(&mut v, 124, &[4, tbits, t_id]); // bitcast f32→u32
    let o0 = next_id;
    next_id += 1;
    inst(&mut v, 132, &[4, o0, 43, 28]); // o0 = i*4
    let store_vals = [27, tbits, inst_id, prim_id];
    for (j, val) in store_vals.iter().enumerate() {
        let idx = if j == 0 {
            o0
        } else {
            let id = next_id;
            next_id += 1;
            inst(&mut v, 128, &[4, id, o0, offs[j]]); // o0+j
            id
        };
        let addr = next_id;
        next_id += 1;
        inst(&mut v, 65, &[23, addr, 21, 26, idx]); // &out[o0+j]
        inst(&mut v, 62, &[addr, *val]); // store
    }
    inst(&mut v, 249, &[merge_lbl]);
    // miss 臂:out[4i+0..4] 全 0(确定性哨兵)。
    inst(&mut v, 248, &[miss_lbl]);
    let m0 = next_id;
    next_id += 1;
    inst(&mut v, 132, &[4, m0, 43, 28]); // m0 = i*4
    for j in 0..4u32 {
        let idx = if j == 0 {
            m0
        } else {
            let id = next_id;
            next_id += 1;
            inst(&mut v, 128, &[4, id, m0, offs[j as usize]]);
            id
        };
        let addr = next_id;
        next_id += 1;
        inst(&mut v, 65, &[23, addr, 21, 26, idx]);
        inst(&mut v, 62, &[addr, 26]); // store 0
    }
    inst(&mut v, 249, &[merge_lbl]);
    inst(&mut v, 248, &[merge_lbl]); // %merge:
    inst(&mut v, 253, &[]); // OpReturn
    inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

// ---------------------------------------------------------------------------
// 对拍辅助
// ---------------------------------------------------------------------------

/// 光线集 → SSBO 字节(8 f32/光线:ox,oy,oz,dx,dy,dz,tmin,tmax)。
fn rays_to_bytes(rays: &[rurix_rt::rt_clas::Ray]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rays.len() * 32);
    for r in rays {
        for f in [
            r.origin[0],
            r.origin[1],
            r.origin[2],
            r.dir[0],
            r.dir[1],
            r.dir[2],
            r.t_min,
            r.t_max,
        ] {
            out.extend_from_slice(&f.to_le_bytes());
        }
    }
    out
}

/// 输出 SSBO 字节 → 逐光线命中记录(4 u32/光线:committed,t_bits,slot,primitive)。
fn decode_hit_records(bytes: &[u8], n_rays: usize) -> Vec<HitRecord> {
    (0..n_rays)
        .map(|i| {
            let w = |k: usize| {
                u32::from_le_bytes([
                    bytes[(i * 4 + k) * 4],
                    bytes[(i * 4 + k) * 4 + 1],
                    bytes[(i * 4 + k) * 4 + 2],
                    bytes[(i * 4 + k) * 4 + 3],
                ])
            };
            HitRecord {
                committed: w(0) != 0,
                t_bits: w(1),
                cluster_slot: w(2),
                primitive: w(3),
            }
        })
        .collect()
}

/// 逐命中比对(容差 0):任一光线记录分叉 → 首个分叉详报。
fn compare_hits(
    name_a: &str,
    a: &[HitRecord],
    name_b: &str,
    b: &[HitRecord],
) -> Result<(), String> {
    if a.len() != b.len() {
        return Err(format!(
            "{name_a}({}) vs {name_b}({}) 光线数失配",
            a.len(),
            b.len()
        ));
    }
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        if x != y {
            return Err(format!(
                "ray{i} 分叉:{name_a}={x:?} vs {name_b}={y:?}(容差 0 判据)"
            ));
        }
    }
    Ok(())
}

fn main() {
    println!(
        "[vk_clas_rt] G9.3 M94 CLAS×RT 合流对拍 harness(RXS-0351;门 g9.p0.m94.clas_rt_convergence)"
    );

    // ── 步骤 1:capability snapshot(决定主腿形态的证据面)──
    let report = match probe_cluster_acceleration_structure() {
        Ok(r) => r,
        Err(e) => {
            println!("CLAS_RT: SKIP 无 Vulkan 设备/loader({})", e.trim());
            return;
        }
    };
    println!("CLAS_CAP: {}", report.summary_line());
    if let Some(p) = report.properties {
        println!(
            "CLAS_PROPS: maxVerticesPerCluster={} maxTrianglesPerCluster={} \
             clusterScratchByteAlignment={} clusterByteAlignment={} \
             clusterTemplateByteAlignment={} clusterBottomLevelByteAlignment={} \
             clusterTemplateBoundsByteAlignment={} maxClusterGeometryIndex={}",
            p.max_vertices_per_cluster,
            p.max_triangles_per_cluster,
            p.cluster_scratch_byte_alignment,
            p.cluster_byte_alignment,
            p.cluster_template_byte_alignment,
            p.cluster_bottom_level_byte_alignment,
            p.cluster_template_bounds_byte_alignment,
            p.max_cluster_geometry_index,
        );
    }
    let support = if report.main_leg_supported() {
        ClasLegSupport::Supported
    } else {
        ClasLegSupport::NotSupported
    };
    let main_leg_ok = matches!(select_leg(RtLeg::ClasMain, support), Ok(RtLeg::ClasMain));
    if main_leg_ok {
        println!("CLAS: main-leg SUPPORTED → 主腿当帧拼装 + 回退腿对照双跑");
    } else {
        println!(
            "CLAS: main-leg NOT-SUPPORTED → DEV_ENV_DEGRADE 诚实登记;判据面 = 回退腿 vs \
             host 金标准逐命中一致(替代口径,RXS-0351 L2 注记;missing={:?})",
            report.missing()
        );
    }

    // ── 步骤 2:确定性场景 + 当帧拼装(单所有者)+ 静态帧零构建断言 ──
    let fixture = m94_fixture();
    let vis = fixture.visible.clone();
    let rays = fixture.rays.clone();
    let mut asm = ClasAssembler::new(fixture);
    let (assembly_digest, key) = {
        let a = match asm.assemble_frame(&vis) {
            Ok(a) => a,
            Err(e) => fail(&format!("首帧装配: {e}")),
        };
        (a.assembly_digest(), a.key())
    };
    let s1 = asm.stats();
    // 静态帧(同可见集再装配):构建计数零增量(L4;非零即 RED)。
    {
        let a2 = match asm.assemble_frame(&vis) {
            Ok(a) => a,
            Err(e) => fail(&format!("静态帧装配: {e}")),
        };
        if a2.key() != key {
            fail("静态帧键漂移(同集不同键)");
        }
    }
    let s2 = asm.stats();
    if s2.blas_builds != s1.blas_builds
        || s2.clas_builds != s1.clas_builds
        || s2.template_builds != s1.template_builds
    {
        fail(&format!(
            "静态帧 AS 构建计数非零增量(RED;L4):{s1:?} → {s2:?}"
        ));
    }
    println!(
        "CLAS_STATS: blas_builds={} clas_builds={} template_builds={} assemblies={} \
         static_frames={} static_frame_zero_build=1",
        s2.blas_builds, s2.clas_builds, s2.template_builds, s2.assemblies, s2.static_frames
    );
    println!("CLAS_ASSEMBLY_DIGEST: 0x{assembly_digest:016x}");

    // ── 步骤 3:host 金标准(与期望表互验后再作参照)──
    let expected = m94_fixture_expected();
    let host_hits: Vec<HitRecord> = {
        let f = m94_fixture();
        rays.iter()
            .map(|r| host_trace_clusters(&f.clusters, r))
            .collect()
    };
    if let Err(e) = compare_hits("host", &host_hits, "expected", &expected) {
        fail(&format!("host 金标准自校验: {e}"));
    }
    let host_digest = hit_stream_digest(&host_hits);
    println!("CLAS_HOST_DIGEST: 0x{host_digest:016x}");

    // ── 步骤 4:RED-a 装配期错簇核验(L3 锚)──
    {
        let a = asm.assemble_frame(&vis).expect("装配存活");
        let mut drifted = vis.clone();
        drifted[2].cluster_id = 777;
        match verify_visible_blas_consistency(&drifted, a) {
            Err(e) => println!("CLAS_RT: RED-OK cluster-mismatch({e})"),
            Ok(()) => fail("cluster-mismatch RED 失效:错开一簇未被装配期核验拒绝"),
        }
    }
    // ── 步骤 5:RED-c 换腿纪律(L7 锚;纯 host)──
    match select_leg(RtLeg::ClasMain, ClasLegSupport::NotSupported) {
        Err(e) => println!("CLAS_RT: RED-OK leg-switch({e})"),
        Ok(_) => fail("leg-switch RED 失效:主腿能力缺失仍放行"),
    }

    // ── 步骤 6:回退腿 device 真跑(per-簇传统 BLAS × 逐簇实例;槽位 = InstanceId)──
    let spv = m94_ray_query_spv(false);
    let rays_bytes = rays_to_bytes(&rays);
    let n_rays = rays.len();
    let out_len = n_rays * 16;
    let (blas_tris, instances) = {
        let a = asm.assemble_frame(&vis).expect("装配存活");
        let tris = a.fallback_blas_triangles();
        let inst: Vec<RayQueryInstanceDesc> = (0..tris.len() as u32)
            .map(|i| RayQueryInstanceDesc {
                blas: i,
                custom_index: 0,
                mask: 0xFF,
                sbt_record_offset: 0,
            })
            .collect();
        (tris, inst)
    };
    let blas_refs: Vec<&[f32]> = blas_tris.iter().map(Vec::as_slice).collect();
    let scene = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let buffers = [
        RayQueryBufferDesc::Input(&rays_bytes),
        RayQueryBufferDesc::Output(out_len),
    ];
    let fallback_out = match run_ray_query_effects(
        &scene,
        &[RayQueryDispatchDesc {
            name: "m94_hits",
            spv: &spv,
            entry: "main",
            buffers: &buffers,
            push_constants: &[],
            groups: [n_rays as u32, 1, 1],
        }],
    ) {
        Ok(o) => o,
        Err(e) if is_no_device(&e) => {
            println!("CLAS_RT: SKIP device 真跑不可用({})", e.trim());
            return;
        }
        Err(e) => fail(&format!("回退腿执行: {e}")),
    };
    let fallback_readback = fallback_out
        .readbacks
        .first()
        .and_then(|d| d.first())
        .expect("回退腿单 dispatch 单输出");
    let fallback_hits = decode_hit_records(fallback_readback, n_rays);
    let fallback_digest = hit_stream_digest(&fallback_hits);
    println!("CLAS_FALLBACK_DIGEST: 0x{fallback_digest:016x}");
    // 回退腿 vs host 金标准(容差 0;正确性基线的 oracle 锚)。
    if let Err(e) = compare_hits("fallback", &fallback_hits, "host", &host_hits) {
        fail(&format!("回退腿 vs host 金标准: {e}"));
    }
    println!("CLAS_RT: fallback-vs-host 逐命中一致(容差 0,{n_rays} 光线)");

    // ── 步骤 7:RED-b device 级错簇漂移检出(能红反证)──
    {
        // 篡改场景:slot2 簇几何平移(可见集声明不变、内容漂移一簇的 device 形态)。
        let mut tampered = m94_fixture();
        for p in &mut tampered.clusters[2].positions {
            p[0] += 100.0;
        }
        let mut asm2 = ClasAssembler::new(tampered);
        let tris2 = asm2
            .assemble_frame(&vis)
            .expect("篡改装配合法")
            .fallback_blas_triangles();
        let refs2: Vec<&[f32]> = tris2.iter().map(Vec::as_slice).collect();
        let scene2 = RayQuerySceneDesc {
            blas_triangles: &refs2,
            instances: &instances,
        };
        let out2 = match run_ray_query_effects(
            &scene2,
            &[RayQueryDispatchDesc {
                name: "m94_hits",
                spv: &spv,
                entry: "main",
                buffers: &buffers,
                push_constants: &[],
                groups: [n_rays as u32, 1, 1],
            }],
        ) {
            Ok(o) => o,
            Err(e) => fail(&format!("device-drift 场景执行: {e}")),
        };
        let hits2 = decode_hit_records(
            out2.readbacks
                .first()
                .and_then(|d| d.first())
                .expect("同上面"),
            n_rays,
        );
        match compare_hits("tampered", &hits2, "host", &host_hits) {
            Err(e) => println!("CLAS_RT: RED-OK device-drift({e})"),
            Ok(()) => fail("device-drift RED 失效:错开一簇的命中流未被检出"),
        }
    }

    // ── 步骤 8:主腿(CLAS 当帧拼装)device 真跑 + 双腿对拍(槽位 = GeometryIndex)──
    // validation layer 头文件滞后(层 spec <1.4,扩展 570 sType 不在其 VUID 库)
    // 且本进程请求装载层时,主腿 device 创建必被误报——显式 DEV_ENV_DEGRADE 登记
    // (非静默换腿:能力面 main_leg_supported 不变,主腿在 validation=off 真跑)。
    let validation_on = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    let main_leg_layer_lag =
        main_leg_ok && validation_on && report.main_leg_blocked_by_layer_lag();
    if main_leg_layer_lag {
        println!(
            "CLAS_MAIN: DEV_ENV_DEGRADE validation-layer-header-lag(val_layer={} < 1.4, \
             VUID-VkDeviceCreateInfo-pNext-pNext 误报 NV sType;主腿真跑以 validation=off 证据为准)",
            report
                .validation_layer_spec_version
                .map_or("?".to_string(), |v| format!(
                    "{}.{}.{}",
                    v >> 22,
                    (v >> 12) & 0x3ff,
                    v & 0xfff
                )),
        );
    }
    let mut main_digest = None;
    if main_leg_ok && !main_leg_layer_lag {
        let spv_geom = m94_ray_query_spv(true);
        let main_out = {
            let a = asm.assemble_frame(&vis).expect("装配存活");
            match run_clas_main_leg_ray_query(
                a,
                &spv_geom,
                "main",
                &rays_bytes,
                out_len,
                [n_rays as u32, 1, 1],
            ) {
                Ok(o) => o,
                Err(e) => fail(&format!("主腿执行: {e}")),
            }
        };
        let main_hits = decode_hit_records(&main_out.readback, n_rays);
        let d = hit_stream_digest(&main_hits);
        main_digest = Some(d);
        println!("CLAS_MAIN_DIGEST: 0x{d:016x}");
        println!(
            "CLAS_MAIN_EVIDENCE: blas_address=0x{:x} clas_addresses={:?} template_addresses={:?}",
            main_out.blas_address, main_out.clas_addresses, main_out.template_addresses
        );
        // RXS-0351 L2 原始判据:主腿 vs 回退腿逐命中一致(容差 0)。
        if let Err(e) = compare_hits("main", &main_hits, "fallback", &fallback_hits) {
            fail(&format!("CLAS 主腿 vs 回退腿: {e}"));
        }
        if let Err(e) = compare_hits("main", &main_hits, "host", &host_hits) {
            fail(&format!("CLAS 主腿 vs host 金标准: {e}"));
        }
        println!("CLAS_RT: main-vs-fallback 逐命中一致(容差 0,{n_rays} 光线)");
    }

    let parity = if main_digest.is_some() {
        "main==fallback==host"
    } else if main_leg_layer_lag {
        "fallback==host(main SUPPORTED;validation 层滞后 DEV_ENV_DEGRADE,主腿证据见 validation=off 真跑)"
    } else {
        "fallback==host(main DEV_ENV_DEGRADE not-supported)"
    };
    println!(
        "CLAS_RT: PASS {parity} digests[host=0x{host_digest:016x} fallback=0x{fallback_digest:016x} main={}] \
         static_frame_zero_build=1 RED[cluster-mismatch,device-drift,leg-switch]=OK \
         validation={}",
        main_digest.map_or_else(
            || {
                if main_leg_layer_lag {
                    "n/a(layer-lag degrade)".to_string()
                } else {
                    "n/a(not-supported)".to_string()
                }
            },
            |d| format!("0x{d:016x}"),
        ),
        if validation_on {
            "on(0=pass 由 lane 内 messenger fail-closed 承担)"
        } else {
            "off"
        }
    );
}
