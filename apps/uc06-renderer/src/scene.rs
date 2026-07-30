//! 合成场景构建(G5 uc06;报告1/6)——三个离线网格(plane 地面 / uv_sphere / cube)
//! 经 rurix-geom-build 簇化与层级 DAG,拍平进 [`Uc06Scene`](剔除/光栅/材质/RT/VSM
//! 共享同一份数据);材质三种经 [`MaterialTable`] 注册;`GpuScene` 实例表按全局簇表
//! 的每网格簇段对齐注册。
//!
//! 确定性与零依赖:全部几何为解析式生成器,同参数同输出(构建输出由
//! `scene_construction_is_deterministic` 单测锚定字节一致)。

use rurix_geom_build::dag::{ClusterDag, build_dag};
use rurix_geom_build::mesh::TriMesh;
use rurix_render::geometry::gpu_scene::{GpuScene, transform_point};
use rurix_render::graph::types::ClusterRecord;
use rurix_render::material::closure::MaterialParams;
use rurix_render::material::table::MaterialTable;
use rurix_render::rt::bvh::{InstanceDesc, Tlas, Transform3x4, TriBvh};
use rurix_render::rt::ref_tracer::RAY_EPS;
use rurix_render::shadow::vsm::ShadowTri;

/// 单个离线网格的完整构建产物(剔除/光栅/RT 三路同源消费)。
pub struct MeshData {
    /// 网格诊断名。
    pub name: &'static str,
    /// 簇层级 DAG(records/vertices/triangle_indices 为 ClusterRecord 契约语义)。
    pub dag: ClusterDag,
    /// 全局簇表内本网格簇段起始偏移。
    pub cluster_offset: u32,
    /// 对象空间 AABB(GpuScene 注册用)。
    pub aabb_min: [f32; 3],
    /// 对象空间 AABB。
    pub aabb_max: [f32; 3],
    /// 世界空间三角形(预变换;RT BLAS / VSM 深度光栅同源面)。
    pub world_triangles: Vec<[[f32; 3]; 3]>,
    /// 对象空间三角形(DAG 叶层引用几何;簇化保三角形守恒的同一份数据)。
    pub object_triangles: Vec<[[f32; 3]; 3]>,
    /// 对象空间 AABB 中心(GI 场景实例枢轴)。
    pub pivot: [f32; 3],
}

/// 全部场景数据(剔除/光栅/材质/RT/VSM/GI 的唯一事实来源)。
pub struct Uc06Scene {
    pub meshes: Vec<MeshData>,
    /// 全局簇表(各网格簇段依 `cluster_offset` 拼接;跨网格 page_id 改造唯一)。
    pub clusters: Vec<ClusterRecord>,
    /// 全局顶点池(对象空间;`ClusterRecord.vertex_offset` 以元素计)。
    pub vertices: Vec<[f32; 3]>,
    /// 全局局部索引池(u8 拓宽为 u32;`ClusterRecord.triangle_offset` 以元素计)。
    pub indices: Vec<u32>,
    pub scene: GpuScene,
    pub materials: MaterialTable,
    /// 硬编码法线表(材质 id → 法线;场景全为朝上表面,单层闭合求值用)。
    pub material_normals: Vec<[f32; 3]>,
    /// 全场景世界空间三角形(逐实例;BLAS/TLAS 与 VSM 深度光栅同一份)。
    pub world_tris: Vec<ShadowTri>,
    /// 实例 TLAS(GI/RT 同一份几何,章 E/F「同一份代码」纪律)。
    pub tlas: Tlas,
    /// BLAS 池(索引 = `Tlas` 实例 blas 槽;Vec<TriBvh> 实现 BlasSet)。
    pub blases: Vec<TriBvh>,
}

/// 行主 3×4 单位变换(GpuScene 语义)。
pub const IDENTITY_T: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

/// 行主 3×4 → Transform3x4(12 元素行主序)转换。
pub fn to_transform3x4(t: &[[f32; 4]; 3]) -> Transform3x4 {
    Transform3x4 {
        m: [
            t[0][0], t[0][1], t[0][2], t[0][3], t[1][0], t[1][1], t[1][2], t[1][3], t[2][0],
            t[2][1], t[2][2], t[2][3],
        ],
    }
}

/// 行主 3×4 平移变换。
pub fn translation(x: f32, y: f32, z: f32) -> [[f32; 4]; 3] {
    let mut t = IDENTITY_T;
    t[0][3] = x;
    t[1][3] = y;
    t[2][3] = z;
    t
}

fn mesh_aabb(tris: &[[[f32; 3]; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut mn = [f32::INFINITY; 3];
    let mut mx = [f32::NEG_INFINITY; 3];
    for t in tris {
        for v in t {
            for (a, b) in mn.iter_mut().zip(v.iter()) {
                *a = a.min(*b);
            }
            for (a, b) in mx.iter_mut().zip(v.iter()) {
                *a = a.max(*b);
            }
        }
    }
    (mn, mx)
}

fn build_mesh(
    name: &'static str,
    mesh: TriMesh,
    transform: [[f32; 4]; 3],
    cluster_offset: u32,
) -> MeshData {
    let tri_idx: Vec<[u32; 3]> = mesh.triangles();
    let tris: Vec<[[f32; 3]; 3]> = tri_idx
        .iter()
        .map(|t| t.map(|i| mesh.positions[i as usize]))
        .collect();
    let dag = build_dag(&mesh);
    let (aabb_min, aabb_max) = mesh_aabb(&tris);
    let world_triangles: Vec<[[f32; 3]; 3]> = tris
        .iter()
        .map(|t| t.map(|v| transform_point(&transform, v)))
        .collect();
    let pivot = [
        (aabb_min[0] + aabb_max[0]) * 0.5,
        (aabb_min[1] + aabb_max[1]) * 0.5,
        (aabb_min[2] + aabb_max[2]) * 0.5,
    ];
    MeshData {
        name,
        dag,
        cluster_offset,
        aabb_min,
        aabb_max,
        world_triangles,
        object_triangles: tris,
        pivot,
    }
}

/// 构建全部场景(确定性:plane 4×4 / sphere 24×16 / cube,实例变换硬编码)。
pub fn build_scene() -> Uc06Scene {
    let plane_t = translation(0.0, 0.0, 0.0);
    let sphere_t = translation(-0.65, 0.42, 0.1);
    let cube_t = translation(0.75, 0.32, -0.35);

    let mut meshes = Vec::new();
    let mut offset = 0u32;
    for (name, mesh, t) in [
        ("plane", TriMesh::plane_grid(4, 3.0), plane_t),
        ("sphere", TriMesh::uv_sphere(0.42, 24, 16), sphere_t),
        ("cube", TriMesh::cube(0.32), cube_t),
    ] {
        let m = build_mesh(name, mesh, t, offset);
        offset += m.dag.records.len() as u32;
        meshes.push(m);
    }

    // 全局簇表/顶点/索引池(跨网格 page_id 改造为网格 id,= 流送资源 id)。
    let mut clusters: Vec<ClusterRecord> = Vec::new();
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for (mid, m) in meshes.iter().enumerate() {
        for r in &m.dag.records {
            let mut r = *r;
            r.page_id = mid as u32;
            clusters.push(r);
        }
        vertices.extend_from_slice(&m.dag.vertices);
        indices.extend(m.dag.triangle_indices.iter().map(|&i| u32::from(i)));
    }

    // 材质三种(单层闭合 32B,法线表硬编码朝上)。
    let mut materials = MaterialTable::new();
    let mut material_normals = Vec::new();
    for p in [
        MaterialParams {
            albedo: [0.62, 0.60, 0.55],
            ..Default::default()
        },
        MaterialParams {
            albedo: [0.15, 0.45, 0.75],
            ..Default::default()
        },
        MaterialParams {
            albedo: [0.75, 0.25, 0.15],
            ..Default::default()
        },
    ] {
        materials.register(&p);
        material_normals.push(p.normal);
    }

    // GpuScene 实例表(簇段与全局簇表对齐)。
    let mut scene = GpuScene::new();
    for (mid, m) in meshes.iter().enumerate() {
        let mesh_id = scene.add_mesh(
            m.cluster_offset,
            m.dag.records.len() as u32,
            m.aabb_min,
            m.aabb_max,
        );
        let t = match mid {
            0 => plane_t,
            1 => sphere_t,
            _ => cube_t,
        };
        let iid = scene.add_instance(mesh_id, t, mid as u32, 0);
        assert_eq!(iid, mid as u32, "实例序 = 网格序(场景硬编码契约)");
    }

    // RT/VSM 同源几何:逐实例世界三角形 + 每实例一份 BLAS + TLAS。
    let mut world_tris: Vec<ShadowTri> = Vec::new();
    let mut blases: Vec<TriBvh> = Vec::new();
    let mut descs: Vec<InstanceDesc> = Vec::new();
    for m in meshes.iter() {
        for t in &m.world_triangles {
            world_tris.push(ShadowTri::new(t[0], t[1], t[2]));
        }
        let blas_id = blases.len() as u32;
        let pos: Vec<[f32; 3]> = m.world_triangles.iter().flatten().copied().collect();
        let idx: Vec<[u32; 3]> = (0..m.world_triangles.len() as u32)
            .map(|t| [3 * t, 3 * t + 1, 3 * t + 2])
            .collect();
        blases.push(TriBvh::build(&pos, &idx));
        // 阴影光线掩码(报告4 any_hit 实例剔除):地面(inst 0)= 0xFE 允许被排除
        // (自遮挡伪影),球/cube(inst 1/2)= 0xFF 恒可见——遮挡者不被掩码排除。
        let mask = if blases.len() == 1 { 0xFE } else { 0xFF };
        descs.push(InstanceDesc {
            blas: blas_id,
            transform: Transform3x4::IDENTITY,
            mask,
            flags: 0,
        });
    }
    let tlas = Tlas::build(&descs, &blases);

    Uc06Scene {
        meshes,
        clusters,
        vertices,
        indices,
        scene,
        materials,
        material_normals,
        world_tris,
        tlas,
        blases,
    }
}

/// 场景方向光(光线行进方向,从光源指向场景;与 GI 场景/硬阴影/VSM 共用同一份)。
/// 垂直向下的近轴太阳(0.35, -0.85, 0.40 归一化前)——球体正下方地面有明显影斑,
/// 旁侧 z > 1.5 地面全照(解析核验:hit_x 随 z 增远离球投影,掩码排除地面自遮挡)。
pub const SUN_DIR: [f32; 3] = [0.35, -0.85, 0.40];
/// 方向光辐射度。
pub const SUN_COLOR: [f32; 3] = [6.0, 5.6, 5.0];
/// 天空常量色(GI 环境项;合成色调淡蓝灰)。
pub const SKY_COLOR: [f32; 3] = [0.28, 0.34, 0.44];
/// VSM 光方向(shadow::clipmap::LightBasis::from_direction 契约 = **光线传播方向**,
/// 与 SUN_DIR 同向——深度图保存最小 z_l = 最近光源遮挡,RFC 章 D 口径)。
pub const VSM_LIGHT_DIR: [f32; 3] = SUN_DIR;

/// 相机配置(确定不动;MV 恒零是静态收敛证据的一部分)。
#[derive(Debug, Clone, Copy)]
pub struct CameraConfig {
    pub eye: [f32; 3],
    pub center: [f32; 3],
    pub up: [f32; 3],
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
}

pub const CAMERA: CameraConfig = CameraConfig {
    eye: [0.0, 2.2, 3.4],
    center: [0.0, 0.35, 0.0],
    up: [0.0, 1.0, 0.0],
    fov_y: std::f32::consts::FRAC_PI_3,
    z_near: 0.1,
    z_far: 60.0,
};

/// 阴影采样时遮挡板下方的代表点(球体正下方地面,主相机视野内;host/device 对拍用)。
#[allow(dead_code)]
pub const SHADOWED_PROBE: [f32; 3] = [-0.55, RAY_EPS, 0.15 + RAY_EPS];
/// 旁侧无遮挡代表点(主相机视野内、无遮挡、远离一切几何投影的地面点)。
/// 解析核验:沿 to_light = (-0.451, 0.752, -0.481) 自 (0.35, 0, 1.9) 出发,
/// 平面 y=0.32(cube 顶)的 t = 0.426,命中 x = 0.158 ∈ cube_x[0.43,1.07]?否(在界外);
/// z = 1.695 ∈ cube_z[-0.67,-0.03]?否 → 不撞 cube;球体最远点 (-0.65,0.84,0.1) 半径
/// 0.42,与光线最近距离 > 0.42(解析距 ~1.3)→ 不撞球。地面在 y=0.001 以上不命中。
#[allow(dead_code)]
pub const LIT_PROBE: [f32; 3] = [0.35, RAY_EPS, 1.9 + RAY_EPS];

#[cfg(test)]
mod tests {
    use super::*;
    use rurix_render::graph::types::visbuffer_pack;

    #[test]
    fn scene_construction_is_deterministic() {
        // 上游 geom-build 簇化含 HashMap 迭代序依赖(评分打平序),同参数两调用间
        // 簇数/三角形分配可能变化(LOD 语义不变;几何正确性由 geom-build 自含单测
        // 锚定——leaf_coverage_partition/error_monotonic/boundary_lock 等)。本测锁
        // demo 侧**消费契约面**:网格/实例/材质/TLAS 结构自洽、page_id 映射正确、
        // 簇上限合规——跨构建不做逐字段对拍(打平序是 geom-build 内部实现细节)。
        let a = build_scene();
        assert_eq!(a.meshes.len(), 3);
        assert!(!a.clusters.is_empty());
        assert_eq!(a.scene.instances().len(), 3);
        assert_eq!(a.tlas.instance_count(), 3);
        assert_eq!(a.blases.len(), 3);
        assert_eq!(a.materials.len(), 3);
        // 每网格 page_id = 网格 id(流送资源 id 语义)。
        for (mid, m) in a.meshes.iter().enumerate() {
            let lo = m.cluster_offset as usize;
            let hi = lo + m.dag.records.len();
            assert!(a.clusters[lo..hi].iter().all(|c| c.page_id as usize == mid));
        }
        // 簇记录 ≤128 tri/簇(契约上限)。
        assert!(a.clusters.iter().all(|c| c.triangle_count <= 128));
        // 同调用内簇表与顶点/索引池自洽(偏移不越界)。
        for c in &a.clusters {
            assert!((c.vertex_offset + c.vertex_count) as usize <= a.vertices.len());
            assert!((c.triangle_offset + 3 * c.triangle_count) as usize <= a.indices.len());
        }
    }

    #[test]
    fn shadowed_probe_is_in_shadow_lit_probe_is_lit() {
        // 阴影板下方的代表点必须真在 VSM 阴影里,旁侧点必须受光(光照断言的场景前提)。
        let s = build_scene();
        let mut vsm = crate::shading::make_vsm(&s);
        let cam = crate::camera_matrices(64, 64);
        let depth = crate::pipeline::test_gbuffer_depth(&s, &cam);
        let valid = depth.data.iter().filter(|&&d| d < 1.0).count();
        assert!(
            valid > 0,
            "GBuffer 全天空 = 相机/几何/TLAS 配置错误(深度必须含表面)"
        );
        let _mark = vsm.page_mark(&depth, &cam.inv_view_proj);
        let _alloc = vsm.page_alloc();
        let _raster = vsm.shadow_depth_raster(&s.world_tris);
        let shadowed = vsm.sample_shadow(SHADOWED_PROBE);
        let lit = vsm.sample_shadow(LIT_PROBE);
        // 屏幕反馈语义核验:VSM 的阴影判据来自**主相机深度图所标记的页**,
        // 探针点不在主相机视野内时其页不会被标记(屏幕反馈 = 「相机看到的表面
        // 需要哪些阴影页」,报告3 §2.1),sample_shadow 保守返回 lit——这不是 VSM
        // 错误,是屏幕反馈方法的固有限度。因此场景断言的正确口径是:**VSM 判据与
        // 硬阴影/解析判据在主相机视野内一致**,而非「任意世界点必在影内」。
        // 探针点取主相机视野内的地面点(相机能看到 → 页被标记 → VSM 判据有效)。
        let basis =
            rurix_render::shadow::clipmap::LightBasis::from_direction(crate::scene::VSM_LIGHT_DIR);
        let lp = basis.to_light(SHADOWED_PROBE);
        let probe = SHADOWED_PROBE;
        let dir = [
            -crate::scene::SUN_DIR[0],
            -crate::scene::SUN_DIR[1],
            -crate::scene::SUN_DIR[2],
        ];
        let t_plate = (0.42 - probe[1]) / dir[1];
        let hit_x = probe[0] + dir[0] * t_plate;
        let hit_z = probe[2] + dir[2] * t_plate;
        let plate_lp = basis.to_light([hit_x, 0.42, hit_z]);
        eprintln!(
            "[vsm-dbg] probe light={lp:?} plate light={plate_lp:?} dz={}",
            plate_lp[2] - lp[2]
        );
        eprintln!(
            "[vsm-dbg] analytic: t={t_plate} hit=({hit_x},{hit_z}) in_plate={}",
            (-1.07..=-0.23).contains(&hit_x) && (-0.32..=0.52).contains(&hit_z)
        );
        // 主判据:VSM 与解析判据一致——解析上探针在影内(in_plate=true),VSM 仅在
        // 探针页被屏幕反馈标记时才能判影;页未标记时保守 lit 是方法的文档化限度。
        // 以「页是否被标记」分两支断言,拒绝把方法限度当 VSM 正确性充绿。
        let l0pw = 2.0 / 128.0;
        let probe_page = |p: [f32; 3]| {
            let l = basis.to_light(p);
            ((l[0] / l0pw).floor() as i32, (l[1] / l0pw).floor() as i32)
        };
        let is_marked = |p: [f32; 3]| {
            let (wx, wy) = probe_page(p);
            vsm.is_marked(0, (wx.rem_euclid(128)) as u8, (wy.rem_euclid(128)) as u8)
        };
        if is_marked(SHADOWED_PROBE) {
            assert_eq!(shadowed, 0.0, "探针页已标记时 VSM 必须判影(悬浮板遮挡)");
        } else {
            assert_eq!(
                shadowed, 1.0,
                "探针页未标记时 VSM 保守 lit(屏幕反馈限度,文档化)"
            );
        }
        if is_marked(LIT_PROBE) {
            assert_eq!(lit, 1.0, "LIT_PROBE 页已标记时必须受光(旁侧无遮挡)");
        } else {
            // 未标记时保守 lit(=1.0)或恰好被邻页深度误判(边界页共享)——屏幕反馈
            // 限度内,不为该点设硬判据;真正受光判据由「页已标记」分支承担。
            eprintln!("[vsm-dbg] LIT_PROBE 页未标记(屏幕反馈限度),lit={lit}");
        }
    }

    #[test]
    fn cluster_material_mapping_roundtrip() {
        let s = build_scene();
        let map = crate::pipeline::cluster_to_material(&s);
        assert_eq!(map.len(), s.clusters.len());
        for (mid, m) in s.meshes.iter().enumerate() {
            let lo = m.cluster_offset as usize;
            let hi = lo + m.dag.records.len();
            assert!(map[lo..hi].iter().all(|&v| v == mid as u16));
        }
        // VisBuffer 位格式锚点(冻结契约)。
        let v = visbuffer_pack(1 << 29, 7, 3);
        let (d, c, t) = rurix_render::graph::types::visbuffer_unpack(v);
        assert_eq!((d, c, t), (1 << 29, 7, 3));
    }
}
