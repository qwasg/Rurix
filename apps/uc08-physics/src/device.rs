//! device 腿(G6.3 uc08,feature `vulkan`;render_exec 路径照 uc06 device 腿)——
//! 「物理驱动变换到达 device」的直接证据:
//! 独立小世界(静态地面 + 单动态立方体)P 步物理 + `PhysicsBridge::sync_frame`
//! → 动态实例当前 3×4 喂一次真 raster draw(readback 非零 =
//! `device_pixels_nontrivial`);再 Q 步物理 + sync → 再 draw 一次,两帧
//! readback 像素差异非平凡(`device_motion_pixels_changed`)。
//!
//! 不依赖 .rx 内核:顶点在 host 侧完成「对象空间 → 世界(物理 3×4)→ 裁剪
//! (view_proj)」变换,demo vs/fs 直通(demo_shaders_spv,uc06 同款)。

use rurix_physics::{BodyKind, PhysicsBridge, PhysicsWorld, SyncBudget, WorldDesc};
use rurix_render::geometry::gpu_scene::{GpuScene, transform_point};
use rurix_rt::render_exec;

use crate::pipeline::DeviceLeg;

/// 第一次 draw 前的物理步数(出生姿态附近)。
pub const STEPS_BEFORE_A: u32 = 2;
/// 第二次 draw 前的追加物理步数(累计 26 步 ≈ 0.43s,立方体下落 ~0.9m)。
pub const STEPS_BEFORE_B: u32 = 24;
/// device 腿 draw 尺寸(64×64 Rgba8)。
const DRAW_SIZE: u32 = 64;
/// device 腿立方体出生位姿(主相机视野内;落向地面)。
const DEVICE_SPAWN: [f32; 3] = [0.0, 1.2, 0.3];

/// 物理 → 同步 → 真 draw 对拍。任何 device 失败 = Err(调用方按
/// RURIX_REQUIRE_REAL/loader 缺失纪律裁决红或降级;对拍/断言失败永远硬红)。
pub fn run_device_leg() -> Result<DeviceLeg, String> {
    let caps = render_exec::probe_device_caps().map_err(|e| format!("probe_device_caps: {e}"))?;
    let (vs, fs, _saxpy) = rurix_rt::vk::demo_shaders_spv();
    if vs.is_empty() || fs.is_empty() {
        return Err("demo SPIR-V 资产缺失".into());
    }

    // 独立小世界(与 host 腿互不干扰;同 dt_fixed 位级纪律)。
    let mut world =
        PhysicsWorld::new(WorldDesc::default()).map_err(|e| format!("物理世界创建: {e}"))?;
    let bodies = world
        .add_bodies_batch(&[
            crate::scene::ground_desc(),
            crate::scene::dyn_cube_desc(DEVICE_SPAWN, 0.0),
        ])
        .map_err(|e| format!("批插体: {e}"))?;
    let cube = bodies[1];
    // GpuScene 单实例(bridge 写目标;簇段空——device 腿不光栅 GpuScene,
    // 只取其实例变换槽喂 draw)。
    let mut gpu = GpuScene::new();
    let mesh = gpu.add_mesh(
        0,
        0,
        [-crate::scene::CUBE_HALF; 3],
        [crate::scene::CUBE_HALF; 3],
    );
    let iid = gpu.add_instance(
        mesh,
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
        0,
        0,
    );
    let mut bridge = PhysicsBridge::new();
    bridge.register(cube, iid, BodyKind::Dynamic);

    let dt = world.desc().dt_fixed;
    let cam = crate::pipeline::camera_matrices(DRAW_SIZE, DRAW_SIZE);
    let tris = crate::scene::cube_object_tris();

    // P 步 + sync → draw A。
    for _ in 0..STEPS_BEFORE_A {
        world.step(dt).map_err(|e| format!("物理步 A: {e}"))?;
    }
    let mut budget_a = SyncBudget::new(1024, 4096, 256);
    let _rep_a = bridge.sync_frame(&world, &mut gpu, &mut budget_a);
    let t_a = gpu.instances()[iid as usize].transform;
    let img_a = draw_cube_at(vs, fs, &tris, &t_a, &cam.view_proj)?;

    // Q 步 + sync → draw B(物理驱动变换变化 → 像素差异非平凡)。
    for _ in 0..STEPS_BEFORE_B {
        world.step(dt).map_err(|e| format!("物理步 B: {e}"))?;
    }
    let mut budget_b = SyncBudget::new(1024, 4096, 256);
    let _rep_b = bridge.sync_frame(&world, &mut gpu, &mut budget_b);
    let t_b = gpu.instances()[iid as usize].transform;
    // sync 必须真写入(立方体在下落:A/B 两帧 3×4 平移分量不同)。
    if t_a == t_b {
        return Err("sync 后 A/B 两帧实例变换逐位相同(物理未驱动?)".into());
    }
    let img_b = draw_cube_at(vs, fs, &tris, &t_b, &cam.view_proj)?;

    let pixels_a = count_nonzero(&img_a);
    let pixels_b = count_nonzero(&img_b);
    let changed_pixels = img_a
        .chunks_exact(4)
        .zip(img_b.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count() as u32;
    Ok(DeviceLeg {
        device_name: caps.device_name,
        steps_before_a: STEPS_BEFORE_A,
        steps_before_b: STEPS_BEFORE_A + STEPS_BEFORE_B,
        pixels_a,
        pixels_b,
        changed_pixels,
        device_pixels_nontrivial: pixels_a >= 30 && pixels_b >= 30,
        device_motion_pixels_changed: changed_pixels >= 30,
    })
}

/// 一次真 raster draw:立方体 12 三角形,host 侧完成 对象→世界(3×4)→裁剪
/// (view_proj),Inline 顶点缓冲(stride 32 = pos xyzw + color rgba;demo vs
/// 顶点输入契约,uc06 同款 VUID-07904 合规)。清色全零,返回 Rgba8 readback。
fn draw_cube_at(
    vs: &[u8],
    fs: &[u8],
    tris: &[[[f32; 3]; 3]],
    t: &[[f32; 4]; 3],
    view_proj: &rurix_render::temporal::common::Mat4,
) -> Result<Vec<u8>, String> {
    const FORMAT_R32G32B32A32_SFLOAT: u32 = 109;
    const ATTRS: [(u32, u32, u32); 2] = [
        (0, FORMAT_R32G32B32A32_SFLOAT, 0),
        (1, FORMAT_R32G32B32A32_SFLOAT, 16),
    ];
    // 顶点字节流:逐三角形世界化 → 裁剪空间;面色 = |对象法线|×0.7+0.3(恒非零)。
    let mut verts: Vec<u8> = Vec::with_capacity(tris.len() * 3 * 32);
    for tri in tris {
        let e1 = [
            tri[1][0] - tri[0][0],
            tri[1][1] - tri[0][1],
            tri[1][2] - tri[0][2],
        ];
        let e2 = [
            tri[2][0] - tri[0][0],
            tri[2][1] - tri[0][1],
            tri[2][2] - tri[0][2],
        ];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
        let color = [
            n[0].abs() / len * 0.7 + 0.3,
            n[1].abs() / len * 0.7 + 0.3,
            n[2].abs() / len * 0.7 + 0.3,
            1.0,
        ];
        for v in tri {
            let world = transform_point(t, *v);
            let clip = view_proj.transform_vec4([world[0], world[1], world[2], 1.0]);
            for f in clip {
                verts.extend_from_slice(&f.to_le_bytes());
            }
            for f in color {
                verts.extend_from_slice(&f.to_le_bytes());
            }
        }
    }
    let resources = [render_exec::ResourceDesc::Texture(
        render_exec::TextureDesc {
            width: DRAW_SIZE,
            height: DRAW_SIZE,
            format: render_exec::TexFormat::Rgba8Unorm,
            usage: render_exec::TextureUsage {
                color: true,
                sampled: true,
                ..Default::default()
            },
            data: None,
        },
    )];
    let pass = render_exec::Pass::Raster(render_exec::RasterPass {
        name: "cube",
        vs_spirv: vs,
        fs_spirv: fs,
        vertex: render_exec::VertexData::Inline {
            data: &verts,
            stride: 32,
            attrs: &ATTRS,
        },
        draw: render_exec::DrawSpec::Direct {
            vertex_count: (tris.len() * 3) as u32,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        },
        colors: vec![render_exec::ColorAttachmentRef {
            res: 0,
            clear: Some([0.0, 0.0, 0.0, 0.0]),
        }],
        depth: None,
        viewport: None,
        bindings: render_exec::Bindings::default(),
    });
    let empty: [&[(u32, render_exec::TargetState)]; 1] = [&[]];
    let readbacks = [render_exec::Readback::Texture { res: 0 }];
    let out = render_exec::execute_frame(&resources, &[pass], &empty, &readbacks)
        .map_err(|e| format!("cube draw: {e}"))?;
    Ok(out[0].clone())
}

/// 非零像素计数(清色全零 → 任一通道非零 = 立方体覆盖)。
fn count_nonzero(img: &[u8]) -> u32 {
    img.chunks_exact(4)
        .filter(|p| p.iter().any(|&b| b != 0))
        .count() as u32
}
