//! 合成刚体场景构建(G6.3 uc08;RFC-0017 §4.B)——地面(Static 大盒)+ 5 个动态
//! 立方体低空落向地面堆叠(初始高度按 ~1.5 秒内沉降并触发睡眠设计,
//! `MassProps.allow_sleep = true`)+ 1 个远场景动态立方体(独立流送资源,初始
//! 不停驻不插体,驻留沿批插、剧本卸载凭 [`rurix_physics::RemovalReceipt`] 放页)。
//!
//! 几何与 uc06 同款解析式生成器 + `build_dag()` 簇化;GpuScene 实例表沿用
//! 「实例序 = 网格序」契约;物理世界 `WorldDesc::default()`(dt_fixed = 1/60,
//! job_threads 默认)。渲染网格与物理形状同尺寸(盒半长一致),变换单向
//! physics → GpuScene(§4.B 冻结:渲染不回写物理)。

use rurix_geom_build::dag::{ClusterDag, build_dag};
use rurix_geom_build::mesh::TriMesh;
use rurix_physics::{
    BodyDesc, BodyKind, MassProps, PhysicsTransform, ShapeDesc, compose_transform_3x4,
};
use rurix_render::geometry::gpu_scene::{GpuScene, transform_point};
use rurix_render::graph::types::ClusterRecord;
use rurix_render::material::closure::MaterialParams;
use rurix_render::material::table::MaterialTable;
use rurix_render::rt::bvh::{Transform3x4, TriBvh};
use rurix_render::shadow::vsm::ShadowTri;

/// 动态立方体半边长(渲染网格与物理 Box 半长一致)。
pub const CUBE_HALF: f32 = 0.24;
/// 地面盒半长(渲染网格与物理 Static Box 一致;顶面 y = 0)。
pub const GROUND_HALF: [f32; 3] = [8.0, 0.5, 8.0];
/// 远场景网格/实例/资源 id(= 网格序;初始不停驻不插体)。
pub const FAR_ID: u32 = 6;
/// 初始动态立方体数(远场景不计;帧 0 即在物理世界)。
pub const NEAR_DYNAMIC_COUNT: usize = 5;

/// 停靠位姿(远场景实例未驻留/已卸载时的渲染态:相机视锥外、地面之下,
/// 两级剔除零像素、MV 零贡献。GpuScene 无实例移除面,卸载 = 回停靠位姿)。
pub const PARKED_3X4: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, -100.0],
    [0.0, 0.0, 1.0, 0.0],
];

/// 单个离线网格的完整构建产物(剔除/光栅/RT 三路同源消费;对象空间)。
pub struct MeshData {
    /// 网格诊断名(镜像 uc06;消费面在 device_kernels/日志,demo 侧留口)。
    #[allow(dead_code)]
    pub name: &'static str,
    /// 簇层级 DAG(records/vertices/triangle_indices 为 ClusterRecord 契约语义)。
    pub dag: ClusterDag,
    /// 全局簇表内本网格簇段起始偏移。
    pub cluster_offset: u32,
    /// 对象空间 AABB(GpuScene 注册用)。
    pub aabb_min: [f32; 3],
    /// 对象空间 AABB。
    pub aabb_max: [f32; 3],
    /// 对象空间三角形(BLAS(Static 策略)与逐帧世界三角形重建的同一份源)。
    pub object_triangles: Vec<[[f32; 3]; 3]>,
}

/// 全部场景数据(剔除/光栅/材质/RT/VSM/GI/物理的唯一事实来源)。
pub struct Uc08Scene {
    pub meshes: Vec<MeshData>,
    /// 全局簇表(各网格簇段依 `cluster_offset` 拼接;跨网格 page_id 改造唯一)。
    pub clusters: Vec<ClusterRecord>,
    /// 全局顶点池(对象空间;`ClusterRecord.vertex_offset` 以元素计)。
    pub vertices: Vec<[f32; 3]>,
    /// 全局局部索引池(u8 拓宽为 u32;`ClusterRecord.triangle_offset` 以元素计)。
    pub indices: Vec<u32>,
    pub materials: MaterialTable,
    /// 硬编码法线表(材质 id → 法线;uc06 同款诊断面,单层闭合求值改吃 GBuffer
    /// 世界法线,本表留作材质注册旁证)。
    #[allow(dead_code)]
    pub material_normals: Vec<[f32; 3]>,
    /// 每实例初始 3×4(远场景 = [`PARKED_3X4`];GpuScene/TLAS 同源)。
    pub initial_transforms: Vec<[[f32; 4]; 3]>,
    /// 初始物理体描述(地面 + 5 近立方体;序 = [`Self::body_instances`] 序)。
    pub body_descs: Vec<BodyDesc>,
    /// 初始体 → GpuScene 实例映射(body_descs[i] ↔ instances()[body_instances[i]])。
    pub body_instances: Vec<u32>,
    /// 初始体类型(bridge 注册用;与 body_descs 对齐)。
    pub body_kinds: Vec<BodyKind>,
    /// 远场景体描述(驻留沿 `StreamingBridge::insert_page` 批插)。
    pub far_desc: BodyDesc,
    /// 远场景出生位姿 3×4(批插后 app 侧上线写;与 far_desc.transform 同位)。
    pub far_spawn: [[f32; 4]; 3],
}

/// 行主 3×4 → Transform3x4(12 元素行主序)转换(uc06 scene.rs 同款)。
pub fn to_transform3x4(t: &[[f32; 4]; 3]) -> Transform3x4 {
    Transform3x4 {
        m: [
            t[0][0], t[0][1], t[0][2], t[0][3], t[1][0], t[1][1], t[1][2], t[1][3], t[2][0],
            t[2][1], t[2][2], t[2][3],
        ],
    }
}

/// 刚体 3×4 逆(MV 重投影用):旋转块正交 → 逆 = 转置,平移回代 t' = −Rᵀt。
/// 物理体变换恒为刚体(旋转 + 平移,无缩放/剪切),转置公式成立;
/// 非刚体输入不校验(调用契约:仅用于物理体变换)。
pub fn invert_rigid_3x4(t: &[[f32; 4]; 3]) -> [[f32; 4]; 3] {
    // Rᵀ:列变行。
    let rt = [
        [t[0][0], t[1][0], t[2][0]],
        [t[0][1], t[1][1], t[2][1]],
        [t[0][2], t[1][2], t[2][2]],
    ];
    let tr = [t[0][3], t[1][3], t[2][3]];
    let mut it = [0.0f32; 3];
    for (i, row) in rt.iter().enumerate() {
        it[i] = -(row[0] * tr[0] + row[1] * tr[1] + row[2] * tr[2]);
    }
    [
        [rt[0][0], rt[0][1], rt[0][2], it[0]],
        [rt[1][0], rt[1][1], rt[1][2], it[1]],
        [rt[2][0], rt[2][1], rt[2][2], it[2]],
    ]
}

/// 绕 Y 轴 yaw(弧度)的单位四元数(xyzw)。
fn yaw_quat(yaw: f32) -> [f32; 4] {
    let (s, c) = (yaw * 0.5).sin_cos();
    [0.0, s, 0.0, c]
}

/// 动态立方体描述(渲染/物理同半长;睡眠开启 = 零 MV 断言的场景前提)。
pub fn dyn_cube_desc(pos: [f32; 3], yaw: f32) -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Box {
            half_extents: [CUBE_HALF; 3],
        },
        layer: 0,
        mass_props: MassProps {
            mass: 1.0,
            friction: 0.6,
            restitution: 0.0,
            allow_sleep: true,
        },
        ccd: false,
        transform: PhysicsTransform {
            translation: pos,
            rotation: yaw_quat(yaw),
        },
    }
}

/// 静态地面描述(Static Box;顶面 y = 0)。
pub fn ground_desc() -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Static,
        shape: ShapeDesc::Box {
            half_extents: GROUND_HALF,
        },
        layer: 0,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [0.0, -GROUND_HALF[1], 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    }
}

/// 动态立方体对象空间三角形(device 腿顶点供给同源;feature vulkan 档消费,
/// default 档无消费面)。
#[allow(dead_code)]
pub fn cube_object_tris() -> Vec<[[f32; 3]; 3]> {
    let mesh = TriMesh::cube(CUBE_HALF);
    mesh.triangles()
        .iter()
        .map(|t| t.map(|i| mesh.positions[i as usize]))
        .collect()
}

/// 地面渲染网格:n×n 方格 XZ 平面(y = 0,法线 +y,覆盖 `[-half, half]²`)。
/// 细分理由:SW 光栅「任一顶点 clip w ≤ 0 ⇒ 整三角形保守丢弃」(visbuffer.rs
/// 裁决 4)——整片式大三角形必然横跨相机(相机立于场景上方)而全数丢弃,
/// 细分格保证相机前方格子完整可见。与物理地面(Static Box 顶面 y = 0)同平面,
/// 底面/侧面不入渲染(相机不可见;物理碰撞面 = Box,不以渲染网格为据)。
fn ground_mesh(half: f32, n: u32) -> TriMesh {
    assert!(n >= 1, "至少 1×1 方格");
    let mut positions = Vec::with_capacity((n as usize + 1) * (n as usize + 1));
    for i in 0..=n {
        for j in 0..=n {
            let x = -half + 2.0 * half * j as f32 / n as f32;
            let z = -half + 2.0 * half * i as f32 / n as f32;
            positions.push([x, 0.0, z]);
        }
    }
    let vid = |i: u32, j: u32| i * (n + 1) + j;
    let mut indices = Vec::with_capacity(n as usize * n as usize * 6);
    for i in 0..n {
        for j in 0..n {
            let (v00, v10, v11, v01) = (vid(i, j), vid(i, j + 1), vid(i + 1, j + 1), vid(i + 1, j));
            // 绕序使外法线 = +y(朝向天空/相机)。
            indices.extend_from_slice(&[v00, v01, v11]);
            indices.extend_from_slice(&[v00, v11, v10]);
        }
    }
    TriMesh::new(positions, indices)
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

fn build_mesh(name: &'static str, mesh: TriMesh, cluster_offset: u32) -> MeshData {
    let tri_idx: Vec<[u32; 3]> = mesh.triangles();
    let tris: Vec<[[f32; 3]; 3]> = tri_idx
        .iter()
        .map(|t| t.map(|i| mesh.positions[i as usize]))
        .collect();
    let dag = build_dag(&mesh);
    let (aabb_min, aabb_max) = mesh_aabb(&tris);
    MeshData {
        name,
        dag,
        cluster_offset,
        aabb_min,
        aabb_max,
        object_triangles: tris,
    }
}

/// 出生位姿表(动态立方体;序 = 实例 1..=5,远场景实例 6 单独)。
/// 高度设计:最低 0.80m(c4,~0.33s 落地)、最高 2.00m(c3,落上 c1 堆叠),
/// 全部 ~1.5 秒内沉降并触发睡眠(96 帧 = 1.6s 窗口内,实测见 evidence)。
const NEAR_SPAWNS: [([f32; 3], f32); NEAR_DYNAMIC_COUNT] = [
    ([-0.55, 0.90, 0.15], 0.0),
    ([0.35, 1.35, -0.25], 0.3),
    ([-0.50, 2.00, 0.20], 0.0),
    ([0.90, 0.80, 0.55], 0.0),
    ([-0.10, 1.10, 0.80], -0.2),
];

/// 远场景出生位姿(右侧视野内;驻留沿批插后自此下落)。
const FAR_SPAWN_POS: [f32; 3] = [2.20, 1.00, -1.20];

/// 构建全部场景(确定性:解析式几何 + 硬编码出生位姿)。
pub fn build_scene() -> Uc08Scene {
    // 网格序 = 实例序 = 材质序 = 流送资源 id(硬编码契约)。
    let mut meshes = Vec::new();
    let mut offset = 0u32;
    let mut push_mesh = |name: &'static str, mesh: TriMesh, meshes: &mut Vec<MeshData>| {
        let m = build_mesh(name, mesh, offset);
        offset += m.dag.records.len() as u32;
        meshes.push(m);
    };
    push_mesh("ground", ground_mesh(GROUND_HALF[0], 8), &mut meshes);
    for i in 0..NEAR_DYNAMIC_COUNT {
        push_mesh(
            match i {
                0 => "cube1",
                1 => "cube2",
                2 => "cube3",
                3 => "cube4",
                _ => "cube5",
            },
            TriMesh::cube(CUBE_HALF),
            &mut meshes,
        );
    }
    push_mesh("far_cube", TriMesh::cube(CUBE_HALF), &mut meshes);

    // 全局簇表/顶点/索引池(跨网格 page_id 改造为网格 id,= 流送资源 id;
    // **偏移前缀改造**:dag.vertices/triangle_indices 是「簇局部顶点拼接」段
    // (ClusterRecord.vertex_offset/triangle_offset 相对本网格段,dag.rs 契约),
    // 拼接成全局池必须给每条 record 加段前缀,否则 ≥2 号网格的簇全部指到
    // 0 号网格顶点段(光栅取错几何;uc06 同款表单的修正版)。
    let mut clusters: Vec<ClusterRecord> = Vec::new();
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut v_base = 0u32;
    let mut t_base = 0u32;
    for (mid, m) in meshes.iter().enumerate() {
        for r in &m.dag.records {
            let mut r = *r;
            r.page_id = mid as u32;
            r.vertex_offset += v_base;
            r.triangle_offset += t_base;
            clusters.push(r);
        }
        v_base += m.dag.vertices.len() as u32;
        t_base += m.dag.triangle_indices.len() as u32;
        vertices.extend_from_slice(&m.dag.vertices);
        indices.extend(m.dag.triangle_indices.iter().map(|&i| u32::from(i)));
    }

    // 材质七种(单层闭合 32B,逐实例一种;地面灰 + 立方体五色 + 远场景青)。
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
        MaterialParams {
            albedo: [0.20, 0.60, 0.25],
            ..Default::default()
        },
        MaterialParams {
            albedo: [0.50, 0.20, 0.60],
            ..Default::default()
        },
        MaterialParams {
            albedo: [0.75, 0.65, 0.20],
            ..Default::default()
        },
        MaterialParams {
            albedo: [0.15, 0.60, 0.55],
            ..Default::default()
        },
    ] {
        materials.register(&p);
        material_normals.push(p.normal);
    }

    // 初始变换:地面渲染网格顶点已在 y = 0(与物理 Box 顶面同平面)→ 实例
    // 恒等;近立方体出生位姿(与物理初始变换逐位同源 =
    // compose_transform_3x4(desc.transform));远场景停靠。
    let ground_t: [[f32; 4]; 3] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];
    let mut initial_transforms = vec![ground_t];
    let mut body_descs = vec![ground_desc()];
    let mut body_instances = vec![0u32];
    let mut body_kinds = vec![BodyKind::Static];
    for (i, &(pos, yaw)) in NEAR_SPAWNS.iter().enumerate() {
        let d = dyn_cube_desc(pos, yaw);
        initial_transforms.push(compose_transform_3x4(&d.transform));
        body_descs.push(d);
        body_instances.push(i as u32 + 1);
        body_kinds.push(BodyKind::Dynamic);
    }
    initial_transforms.push(PARKED_3X4);
    let far_desc = dyn_cube_desc(FAR_SPAWN_POS, 0.0);
    let far_spawn = compose_transform_3x4(&far_desc.transform);

    Uc08Scene {
        meshes,
        clusters,
        vertices,
        indices,
        materials,
        material_normals,
        initial_transforms,
        body_descs,
        body_instances,
        body_kinds,
        far_desc,
        far_spawn,
    }
}

/// GpuScene 实例表(簇段与全局簇表对齐;实例序 = 网格序契约)。
pub fn build_gpu_scene(scene: &Uc08Scene) -> GpuScene {
    let mut gpu = GpuScene::new();
    for (mid, m) in scene.meshes.iter().enumerate() {
        let mesh_id = gpu.add_mesh(
            m.cluster_offset,
            m.dag.records.len() as u32,
            m.aabb_min,
            m.aabb_max,
        );
        let iid = gpu.add_instance(mesh_id, scene.initial_transforms[mid], mid as u32, 0);
        assert_eq!(iid, mid as u32, "实例序 = 网格序(场景硬编码契约)");
    }
    gpu
}

/// 逐帧世界空间三角形(VSM 深度光栅同源面;排除停靠实例——停靠体在地面
/// 之下 100m,纳入会把 VSM 深度范围吹爆损失精度,且其视觉等价于不存在)。
pub fn world_tris_now(
    scene: &Uc08Scene,
    gpu: &GpuScene,
    parked: &std::collections::HashSet<u32>,
) -> Vec<ShadowTri> {
    let mut out = Vec::new();
    for (iid, m) in scene.meshes.iter().enumerate() {
        if parked.contains(&(iid as u32)) {
            continue;
        }
        let t = gpu.instances()[iid].transform;
        for tri in &m.object_triangles {
            out.push(ShadowTri::new(
                transform_point(&t, tri[0]),
                transform_point(&t, tri[1]),
                transform_point(&t, tri[2]),
            ));
        }
    }
    out
}

/// 场景方向光(光线行进方向,从光源指向场景;与 GI 场景/硬阴影/VSM 共用同一份)。
pub const SUN_DIR: [f32; 3] = [0.35, -0.85, 0.40];
/// 方向光辐射度。
pub const SUN_COLOR: [f32; 3] = [6.0, 5.6, 5.0];
/// 天空常量色(GI 环境项;合成色调淡蓝灰)。
pub const SKY_COLOR: [f32; 3] = [0.28, 0.34, 0.44];
/// VSM 光方向(shadow::clipmap::LightBasis::from_direction 契约 = **光线传播方向**)。
pub const VSM_LIGHT_DIR: [f32; 3] = SUN_DIR;

/// 相机配置(确定不动;相机 MV 恒零 → 物体 MV 成为唯一 MV 源,便于断言)。
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

/// BLAS 构建输入(对象空间;刚体 BLAS 一律 `DynamicPolicy::Static` 零 refit,
/// 运动全部走 TLAS 实例变换,§4.B AS 分级裁决)。
pub fn blas_inputs(m: &MeshData) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
    let pos: Vec<[f32; 3]> = m.object_triangles.iter().flatten().copied().collect();
    let idx: Vec<[u32; 3]> = (0..m.object_triangles.len() as u32)
        .map(|t| [3 * t, 3 * t + 1, 3 * t + 2])
        .collect();
    (pos, idx)
}

/// TriBvh 直建(GI 场景逐帧重建的同源面;与 BLAS 缓存同几何)。
#[allow(dead_code)]
pub fn tri_bvh_of(m: &MeshData) -> TriBvh {
    let (pos, idx) = blas_inputs(m);
    TriBvh::build(&pos, &idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_structure_contract() {
        let s = build_scene();
        assert_eq!(s.meshes.len(), 7, "地面 + 5 近立方体 + 1 远场景");
        assert!(!s.clusters.is_empty());
        assert_eq!(s.materials.len(), 7);
        assert_eq!(s.body_descs.len(), 6, "初始物理体 = 地面 + 5 近立方体");
        assert_eq!(s.body_instances.len(), 6);
        assert_eq!(s.initial_transforms.len(), 7);
        // 每网格 page_id = 网格 id(流送资源 id 语义)。
        for (mid, m) in s.meshes.iter().enumerate() {
            let lo = m.cluster_offset as usize;
            let hi = lo + m.dag.records.len();
            assert!(s.clusters[lo..hi].iter().all(|c| c.page_id as usize == mid));
        }
        // GpuScene 实例序 = 网格序。
        let gpu = build_gpu_scene(&s);
        assert_eq!(gpu.instances().len(), 7);
        // 远场景实例初始停靠(y = -100)。
        assert_eq!(gpu.instances()[FAR_ID as usize].transform[1][3], -100.0);
        // 地面实例变换 = 恒等(渲染网格顶点已在 y = 0,与物理 Box 顶面同平面)。
        assert_eq!(gpu.instances()[0].transform[1][3], 0.0);
        // 近立方体初始变换 = 出生位姿平移。
        assert_eq!(gpu.instances()[1].transform[1][3], 0.90);
        // 全局池偏移前缀改造:每网格簇段的 vertex/triangle 偏移指向本网格段。
        let mut v_base = 0u32;
        let mut t_base = 0u32;
        for m in s.meshes.iter() {
            let lo = m.cluster_offset as usize;
            let hi = lo + m.dag.records.len();
            for c in &s.clusters[lo..hi] {
                assert!(c.vertex_offset >= v_base && c.triangle_offset >= t_base);
                assert!(
                    (c.vertex_offset + c.vertex_count) as usize <= s.vertices.len()
                        && (c.triangle_offset + 3 * c.triangle_count) as usize <= s.indices.len()
                );
            }
            v_base += m.dag.vertices.len() as u32;
            t_base += m.dag.triangle_indices.len() as u32;
        }
    }

    #[test]
    fn rigid_inverse_roundtrip() {
        // 刚体 3×4:invert_rigid_3x4 逐元素满足 M⁻¹·M·p = p(转置公式正确性)。
        let d = dyn_cube_desc([1.0, 2.0, 3.0], 0.7);
        let t = compose_transform_3x4(&d.transform);
        let inv = invert_rigid_3x4(&t);
        for p in [[0.3, -0.2, 0.5], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]] {
            let w = transform_point(&t, p);
            let back = transform_point(&inv, w);
            for i in 0..3 {
                assert!((back[i] - p[i]).abs() < 1e-5, "往返 {p:?} → {back:?}");
            }
        }
    }

    #[test]
    fn world_tris_excludes_parked() {
        let s = build_scene();
        let gpu = build_gpu_scene(&s);
        let parked: std::collections::HashSet<u32> = [FAR_ID].into_iter().collect();
        let tris = world_tris_now(&s, &gpu, &parked);
        let expect: usize = s.meshes[..FAR_ID as usize]
            .iter()
            .map(|m| m.object_triangles.len())
            .sum();
        assert_eq!(tris.len(), expect);
        // 地面顶面 y = 0(世界三角形含 y ≤ 0 的地面盒)。
        let ground_top = tris
            .iter()
            .flat_map(|t| t.v.iter().map(|v| v[1]))
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(ground_top <= 2.0 + CUBE_HALF * 2.0, "场景高度包络合理");
    }
}
