// Assisted-by: cursor:claude-fable-5(G31 Bistro PT 真值对照器)
//! G31 Bistro PT 真值对照器:g14_3_lane_body 的 bistro 装配产物(契约装配 +
//! nrm/mr 侧表 + A1 灯光提取)接 G12 生产路径追踪 kernel 的演进 fork
//! `kernels/g31_pt_gt.rx`(主线并行编写;本 bin **不依赖其编译**——仅运行时经
//! `--spv` 装载 SPV)。共享车道体 g14_3_lane_body.rs **0-byte 不动**(include!
//! 逐字共享,g35_particle_lane 第 4 bin 先例);device 执行腿 = G12.4
//! g12_4_ue_pt_parity_render `run_device` 像素带分段 dispatch 逐字同形
//! (chunk_pixels = (1M/spp).clamp(512,16384),RNG 流整帧 host 生成带内直切,
//! 输出带内回读拼帧位级一致)。
//!
//! ## fork kernel 接口(冻结;= 母版 g12_pt_production 加一个 trinrm buffer)
//!
//! 形参序(= run_ray_query_effects buffers 序):tlas, ThreadCtx, rng, mats,
//! tris, **trinrm**, lights, params, out_rgb, out_stats, out_samples,
//! out_converged, out_rr, out_energy。
//! - mats:**12 f32/tri** = [albedo.rgb(装配折叠面 clamp[0,1]),
//!   emission.rgb(SceneData 真值——match 预设也写), flag(light_of_prim+1;
//!   非灯=0), metallic, roughness(mr 侧表;quad 尾段/无材质 = [0,1]),
//!   base.rgb(tri_base 未衰减 baseColor)]。
//! - trinrm:9 f32/tri(assemble nrm 侧表原样;len == 三角数×9 断言)。
//! - tris:9 f32/tri = prod::pack_prod_tris(core)。
//! - lights:17 f32/灯 = prod::pack_prod_lights 后**就地改写槽 15(pad)** 为
//!   点光半径(match 预设 scene.points 的 radius;quad/tri 灯恒 0)。
//! - params:48 f32 = prod::pack_prod_params 后三补丁:① p[16..19)(cam_right)
//!   ×= aspect(= w/h,非方形画幅预乘面);② p[37] = spec_enable;③ p[38] =
//!   nrm_enable。
//!
//! ## 子模式
//!
//! ```text
//! g31_pt_gt --render --lights match|area --tau <f32> --spv <fork.spv>
//!     --out-dir <dir> [--scene bistro-interior] [--contract <c.json>]
//!     [--expect-digest sha256:…] [--gltf <scene.gltf>] [--w 960] [--h 540]
//!     [--spp 64] [--seed <u64>] [--spec on|off] [--smooth-normals on|off]
//!     [--lamp on|off] [--lamp-gain 4.0] [--lamp-k 12]
//! g31_pt_gt --selftest equiv --tau <f32> --spv <fork.spv>
//!     --spv-frozen <g12_pt_production.spv> [--contract <c.json>]
//!     [--gltf <scene.gltf>] [--w 192] [--h 108] [--spp 2] [--seed <u64>]
//! g31_pt_gt --selftest furnace --tau <f32> --spv <fork.spv> [--spp 256]
//!     [--seed <u64>]
//! ```
//!
//! - `--render`:契约 digest 校验(冻结锚或 --expect-digest 显式值)→
//!   assemble_scene_nrm_mr 装配 → match 预设可选 A1 灯提取 append →
//!   tri_base 复算(装配折叠同源一致性自证)→ core ProdScene 校验视图
//!   (`--lights area` = emissive 三角逐 tri 网格光进 NEE 灯表,不加点光防
//!   双计;`--lights match` = 生产车道同灯集:契约+提取点光,emissive 只进
//!   mats12 命中 w=1 无偏路径)→ device 双跑位级断言 → EXR + receipt 落盘。
//! - `--selftest equiv`:bistro area 预设(lamp off)同一 core/RNG/params 跑
//!   fork(gates off)与母版冻结 SPV 两腿,ProdImage 全输出位级相等机核
//!   ——「fork off ⇒ 母版逐位等价」。
//! - `--selftest furnace`:prod::g12_furnace_scene 白炉 6 case(baseline +
//!   spec on 五组 (metallic,roughness))。判据 = 不造能带(≤ baseline×1.05,
//!   glTF 形组合中粗糙介质 ~1%/bounce 超单位面 4 反弹放大实测 +3.4% 如实
//!   登记)+ metal 全低于基线且 r=1 < r=0.5(截断 + 单散射损失物理向)+
//!   dielectric r=1 带 [0.90,1.02];镜面 r=0.05 排序为 4 反弹截断主导,
//!   measured 登记不设断言。
//!
//! 三态:无 Vulkan → `G31_PTGT: SKIP DEV_ENV_DEGRADE` 退 0
//! (RURIX_REQUIRE_REAL=1 翻硬红);判据不符/digest 漂移 → FAIL 退 1。
#![forbid(unsafe_code)]
// 共享体含本 bin 未消费面(TSR/bench 车道、dlss/fsr 双臂、GI 臂、SVT/蒙皮/
// HZB/簇 LOD 面等)——dead_code 豁免如实登记;本 bin 消费面 = 契约解析/
// scene 装配(nrm/mr 侧表)/A1 灯提取/glTF 读取/DDS 均值/EXR 落盘/JSON 工具。
#![allow(dead_code)]

include!("g14_3_lane/g14_3_lane_body.rs");

use rurix_render::gi::path_trace::prod::{
    self, LightDist, ProdConfig, ProdImage, ProdLight, ProdScene, SamplerFamily,
};
use rurix_render::gi::path_trace::{MaterialKind, PtCamera, PtLightQuad};
use rurix_render::rt::bvh::Vec3;

/// 本 bin stdout/stderr 标签(体 TAG 为共享体诊断面,出图协议行用本标签)。
const G31GT_TAG: &str = "G31_PTGT";
/// RNG 流内存守门上界(floats;w·h·spp·(2+6·max_bounces) 超此即拒跑)。
const G31GT_RNG_FLOAT_CAP: u64 = 1_500_000_000;

fn gfail(msg: &str) -> ! {
    eprintln!("{G31GT_TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn gskip(msg: &str) -> ! {
    println!("{G31GT_TAG}: SKIP DEV_ENV_DEGRADE {msg}");
    std::process::exit(0)
}

/// device 环境门(仅 Vulkan 面;资产缺失走 fail-closed 不冒充降级)。
fn dev_gate() {
    if !vk::vulkan_available() {
        if require_real() {
            gfail("无 Vulkan loader/设备面(RURIX_REQUIRE_REAL=1 硬红)");
        }
        gskip("无 Vulkan loader/设备面");
    }
}

fn parse_onoff(name: &str, v: &str) -> bool {
    match v {
        "on" => true,
        "off" => false,
        other => gfail(&format!("{name} 只接受 on|off(得 {other})")),
    }
}

fn clamp01_3(c: [f32; 3]) -> [f32; 3] {
    [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0)]
}

fn read_u32_le(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// SPV 单次读取(字流 + 文件字节 sha256 双出;receipt provenance 同字节源)。
fn load_spv_sha(path: &str) -> (Vec<u32>, String) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| gfail(&format!("SPV 读取 {path}: {e}")));
    if bytes.len() % 4 != 0 {
        gfail(&format!("SPV {path} 字节数非 4 对齐"));
    }
    let words = bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    (words, format!("sha256:{}", sha256_hex(&bytes)))
}

/// RNG 内存守门(fail-closed 带公式;整帧流 host 一次性生成的上界保护)。
fn rng_guard(pixel_count: usize, spp: u32, max_bounces: u32) {
    let stride = prod::prod_sample_stride(max_bounces) as u64;
    let floats = pixel_count as u64 * spp as u64 * stride;
    if floats > G31GT_RNG_FLOAT_CAP {
        gfail(&format!(
            "RNG 流超守门:w·h·spp·(2+6·max_bounces) = {pixel_count}×{spp}×{stride} = {floats} > {G31GT_RNG_FLOAT_CAP} floats"
        ));
    }
}

// ---------------------------------------------------------------------------
// 相机与 core 校验视图(SceneData → ProdScene;G12.4 parity 相机公式逐字)
// ---------------------------------------------------------------------------

/// 契约相机 → PtCamera(G12.4 parity 1211-1227 同式:forward = 契约四元数派生
/// 已在装配期完成;r = norm(f×up0) **UE 一致手性**,u = r×f;宽高 = CLI)。
fn pt_camera_from_spec(cam: &CameraSpec, w: u32, h: u32) -> PtCamera {
    let f = Vec3::new(cam.forward[0], cam.forward[1], cam.forward[2]).normalize();
    let u0 = Vec3::new(cam.up0[0], cam.up0[1], cam.up0[2]);
    let r = f.cross(u0).normalize();
    let u = r.cross(f);
    PtCamera {
        origin: cam.eye,
        forward: f.to_array(),
        right: r.to_array(),
        up: u.to_array(),
        tan_half_fov: (cam.fov_y_rad * 0.5).tan(),
        width: w,
        height: h,
    }
}

/// core 校验视图构建产物(灯半径 = pack 槽 15 改写源;计数 = receipt 登记面)。
struct CoreBuild {
    core: ProdScene,
    /// 逐灯半径(与 core.lights 同序;点光 = SceneData radius,quad/tri 恒 0)。
    light_radius: Vec<f32>,
    n_points: usize,
    n_quads: usize,
    n_tri_lights: usize,
}

/// SceneData → core ProdScene(校验视图;positions/indices 直搬汤)。
///
/// - `match_mode=false`(area 预设):emissive 且 tri_mat≠NONE →
///   Emission + ProdLight::Tri 逐 tri 网格光;quad 尾段(tri_mat==NONE 且
///   emissive)→ ProdLight::Quad + 双尾段 Emission 链接;其余 Lambert+MAX。
///   **不加点光**(防双计)。
/// - `match_mode=true`:全部 tri Lambert(emissive 的 emission 只进 mats12,
///   kernel 命中加 w=1 无偏路径,不进 core 视图/灯表);lights = scene.points
///   逐个 Point(radius 记旁路),quads 非空按 area 同法追加。
fn build_core(scene: &SceneData, match_mode: bool, w: u32, h: u32) -> CoreBuild {
    let n = scene.indices.len();
    let quad_tail_start = n - scene.quads.len() * 2;
    let mut materials: Vec<MaterialKind> = Vec::with_capacity(n);
    let mut light_of_prim: Vec<u32> = vec![u32::MAX; n];
    let mut lights: Vec<ProdLight> = Vec::new();
    let mut light_radius: Vec<f32> = Vec::new();
    let mut n_points = 0usize;
    let mut n_quads = 0usize;
    let mut n_tri_lights = 0usize;
    if match_mode {
        for p in &scene.points {
            lights.push(ProdLight::Point {
                position: p.pos,
                intensity: p.intensity,
            });
            light_radius.push(p.radius);
            n_points += 1;
        }
    }
    for t in 0..n {
        let em = scene.emission[t];
        let emissive = em[0] > 0.0 || em[1] > 0.0 || em[2] > 0.0;
        let alb = clamp01_3(scene.albedo[t]);
        if emissive && scene.tri_mat[t] == SLAB_TRI_NONE {
            // quad 灯面尾段(装配序:全部 mesh 三角之后逐 quad 双三角;若尾段
            // 几何序与 validate 期望 [(p00,p10,p11),(p00,p11,p01)] 不符会在
            // core.validate() fail-closed,如实报错)。
            if t < quad_tail_start {
                gfail(&format!(
                    "三角 {t} 无材质却发光且非 quad 尾段(装配语义破坏)"
                ));
            }
            let off = t - quad_tail_start;
            let q = &scene.quads[off / 2];
            if off % 2 == 0 {
                lights.push(ProdLight::Quad(PtLightQuad {
                    p00: q.p00,
                    e1: q.e1,
                    e2: q.e2,
                    emission: q.le,
                }));
                light_radius.push(0.0);
                n_quads += 1;
            }
            materials.push(MaterialKind::Emission {
                albedo: alb,
                emission: em,
            });
            light_of_prim[t] = (lights.len() - 1) as u32;
        } else if emissive && !match_mode {
            // area 预设:emissive 材质三角 → 三角网格光(G12.4 M163 同法)。
            let idx = scene.indices[t];
            let v0 = scene.positions[idx[0] as usize];
            let v1 = scene.positions[idx[1] as usize];
            let v2 = scene.positions[idx[2] as usize];
            let li = lights.len() as u32;
            lights.push(ProdLight::Tri {
                v0,
                e1: [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]],
                e2: [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]],
                emission: em,
            });
            light_radius.push(0.0);
            n_tri_lights += 1;
            materials.push(MaterialKind::Emission {
                albedo: alb,
                emission: em,
            });
            light_of_prim[t] = li;
        } else {
            // match 预设 emissive 三角也走本臂:emission 真值只进 mats12。
            materials.push(MaterialKind::Lambert { albedo: alb });
        }
    }
    let core = ProdScene {
        name: "g31_ptgt_bistro",
        positions: scene.positions.clone(),
        indices: scene.indices.clone(),
        materials,
        lights,
        camera: pt_camera_from_spec(&scene.camera, w, h),
        t_max: scene.camera.far,
        light_of_prim,
    };
    core.validate()
        .unwrap_or_else(|e| gfail(&format!("core 场景校验: {e}")));
    CoreBuild {
        core,
        light_radius,
        n_points,
        n_quads,
        n_tri_lights,
    }
}

// ---------------------------------------------------------------------------
// tri_base 复算(glTF materials 再读一遍;装配折叠同源一致性自证)
// ---------------------------------------------------------------------------

/// 逐材质 [base(未衰减 baseColor,clamp[0,1]), metallic](装配 MatRec 同口径:
/// baseColorFactor 缺省 [1,1,1,1]、metallicFactor 缺省 1.0、
/// baseColorTexture.index → textures[].source → images[].uri;texture_mean
/// 且有贴图 → 同目录 DDS 字节经 dds_mean_linear_rgb 均值 × factor)。
fn mat_base_table(gltf: &Gltf, gltf_dir: &Path, texture_mean: bool) -> (Vec<[f32; 3]>, Vec<f32>) {
    let image_uris: Vec<Option<String>> = gltf
        .root
        .get("images")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .map(|im| im.get("uri").and_then(|v| v.as_str()).map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let mut mean_cache: Vec<Option<[f32; 3]>> = vec![None; image_uris.len()];
    let mut base_tab: Vec<[f32; 3]> = Vec::new();
    let mut met_tab: Vec<f32> = Vec::new();
    for m in gltf
        .root
        .get("materials")
        .and_then(|v| v.as_array())
        .unwrap_or(&[])
    {
        let pbr = m.get("pbrMetallicRoughness");
        let alb4 = pbr
            .and_then(|p| p.get("baseColorFactor"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .map(|x| x.as_f64().unwrap_or(1.0) as f32)
                    .collect::<Vec<_>>()
            });
        let factor = match alb4 {
            Some(v) if v.len() == 4 => [v[0], v[1], v[2]],
            _ => [1.0, 1.0, 1.0],
        };
        let metallic = pbr
            .and_then(|p| p.get("metallicFactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let img = pbr
            .and_then(|p| p.get("baseColorTexture"))
            .and_then(|t| t.get("index"))
            .and_then(|v| v.as_u64())
            .and_then(|ti| gltf.root.get("textures")?.as_array()?.get(ti as usize))
            .and_then(|tex| tex.get("source"))
            .and_then(|v| v.as_u64())
            .map(|x| x as usize);
        let mut b = factor;
        if texture_mean {
            if let Some(ii) = img {
                if mean_cache.get(ii).is_some_and(|m| m.is_none()) {
                    let uri = image_uris
                        .get(ii)
                        .and_then(|u| u.clone())
                        .unwrap_or_else(|| gfail(&format!("image {ii} 缺 uri(tri_base 复算面)")));
                    let raw = std::fs::read(gltf_dir.join(&uri))
                        .unwrap_or_else(|e| gfail(&format!("纹理 {uri} 读取失败: {e}")));
                    let mean = dds_mean_linear_rgb(&raw)
                        .unwrap_or_else(|e| gfail(&format!("纹理 {uri} DDS 解码失败: {e}")));
                    mean_cache[ii] = Some(mean);
                }
                if let Some(Some(mean)) = mean_cache.get(ii) {
                    b = [
                        mean[0] * factor[0],
                        mean[1] * factor[1],
                        mean[2] * factor[2],
                    ];
                }
            }
        }
        base_tab.push(clamp01_3(b));
        met_tab.push(metallic);
    }
    (base_tab, met_tab)
}

/// 逐 tri base(tri_mat==NONE → base = 装配 albedo;否则查表)+ **一致性自证**:
/// 对 tri_mat≠NONE 且 metallic<0.999 的每个材质抽一 tri 断言
/// |albedo − base×(1−metallic)| ≤ 1e-3 逐通道(装配折叠同源证明,不符 fail)。
fn tri_base_table(scene: &SceneData, base_tab: &[[f32; 3]], met_tab: &[f32]) -> Vec<[f32; 3]> {
    let mut out: Vec<[f32; 3]> = Vec::with_capacity(scene.indices.len());
    let mut checked: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    for t in 0..scene.indices.len() {
        let mi = scene.tri_mat[t];
        if mi == SLAB_TRI_NONE {
            out.push(clamp01_3(scene.albedo[t]));
            continue;
        }
        match base_tab.get(mi as usize) {
            Some(b) => {
                let met = met_tab[mi as usize];
                if met < 0.999 && checked.insert(mi) {
                    let k = 1.0 - met;
                    for c in 0..3 {
                        let want = b[c] * k;
                        let got = scene.albedo[t][c];
                        if (got - want).abs() > 1e-3 {
                            gfail(&format!(
                                "材质 {mi} 三角 {t} 折叠一致性自证失败:装配 albedo[{c}]={got} ≠ base×(1−metallic)={want}(|Δ|>1e-3)"
                            ));
                        }
                    }
                }
                out.push(*b);
            }
            // 越界材质索引:装配同口径回落(albedo=[1,1,1] 全折叠面)。
            None => out.push(clamp01_3(scene.albedo[t])),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// device 输入打包(冻结 kernel 接口;prod 打包面 + 三补丁/槽 15 改写/mats12)
// ---------------------------------------------------------------------------

/// mats 打包:12 f32/tri = [albedo.rgb, emission.rgb, flag(light_of_prim+1;
/// 非灯=0), metallic, roughness, base.rgb](quad 尾段/无材质 tri 的 MR 恒
/// [0,1]——mr 侧表尾段登记 [0,0] 是灯面零增益语义,kernel 接口按冻结面钉
/// [0.0,1.0])。
fn pack_mats12(scene: &SceneData, core: &ProdScene, mr: &[f32], tri_base: &[[f32; 3]]) -> Vec<f32> {
    let n = scene.indices.len();
    if mr.len() != n * 2 {
        gfail(&format!("mr 侧表长度 {} ≠ 三角数×2 {}", mr.len(), n * 2));
    }
    if tri_base.len() != n {
        gfail(&format!("tri_base 长度 {} ≠ 三角数 {n}", tri_base.len()));
    }
    let mut out = Vec::with_capacity(n * 12);
    for t in 0..n {
        out.extend_from_slice(&clamp01_3(scene.albedo[t]));
        out.extend_from_slice(&scene.emission[t]);
        let flag = if core.light_of_prim[t] == u32::MAX {
            0.0
        } else {
            (core.light_of_prim[t] + 1) as f32
        };
        out.push(flag);
        let (mt, rg) = if scene.tri_mat[t] == SLAB_TRI_NONE {
            (0.0, 1.0)
        } else {
            (mr[t * 2], mr[t * 2 + 1])
        };
        out.push(mt);
        out.push(rg);
        out.extend_from_slice(&tri_base[t]);
    }
    out
}

/// furnace 专用 mats12:墙 tri(light_of_prim==MAX)→ 逐 case
/// metallic/roughness、base=[1,1,1]、albedo 沿 core(=1);灯 tri → m=0,r=1,
/// base=[0,0,0]。
fn pack_mats12_furnace(core: &ProdScene, metallic: f32, roughness: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(core.indices.len() * 12);
    for (t, m) in core.materials.iter().enumerate() {
        let (albedo, emission) = match m {
            MaterialKind::Lambert { albedo } => (*albedo, [0.0; 3]),
            MaterialKind::Emission { albedo, emission } => (*albedo, *emission),
            _ => ([0.0; 3], [0.0; 3]),
        };
        out.extend_from_slice(&albedo);
        out.extend_from_slice(&emission);
        let is_light = core.light_of_prim[t] != u32::MAX;
        out.push(if is_light {
            (core.light_of_prim[t] + 1) as f32
        } else {
            0.0
        });
        if is_light {
            out.push(0.0);
            out.push(1.0);
            out.extend_from_slice(&[0.0; 3]);
        } else {
            out.push(metallic);
            out.push(roughness);
            out.extend_from_slice(&[1.0; 3]);
        }
    }
    out
}

/// 灯表打包:prod::pack_prod_lights 后就地改写每灯槽 15(pad)为点光半径
/// (quad/tri 灯与契约点光恒 0;A1 提取灯 >0——kernel 阴影 t 截断消费面)。
fn pack_lights_radius(core: &ProdScene, dist: &LightDist, radius: &[f32]) -> Vec<f32> {
    if radius.len() != core.lights.len() {
        gfail(&format!(
            "灯半径旁路长度 {} ≠ 灯数 {}",
            radius.len(),
            core.lights.len()
        ));
    }
    let mut l = prod::pack_prod_lights(core, dist);
    for (li, r) in radius.iter().enumerate() {
        l[li * 17 + 15] = *r;
    }
    l
}

/// 参数打包:prod::pack_prod_params 后三补丁(① cam_right ×= aspect
/// 非方形画幅预乘;② p[37] = spec_enable;③ p[38] = nrm_enable——均落
/// 母版预留区 [36..48),母版 kernel 不读 0-byte)。
fn pack_params_gt(core: &ProdScene, cfg: &ProdConfig, spec_on: bool, nrm_on: bool) -> Vec<f32> {
    let mut p = prod::pack_prod_params(core, cfg);
    let aspect = core.camera.width as f32 / core.camera.height as f32;
    for k in 16..19 {
        p[k] *= aspect;
    }
    p[37] = if spec_on { 1.0 } else { 0.0 };
    p[38] = if nrm_on { 1.0 } else { 0.0 };
    p
}

// ---------------------------------------------------------------------------
// device 执行腿(G12.4 parity run_device 像素带分段 dispatch 逐字同形;
// gt 腿加 trinrm buffer,frozen 腿 = 母版对照原形)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn run_device_gt(
    core: &ProdScene,
    cfg: &ProdConfig,
    mats12: &[f32],
    trinrm: &[f32],
    lights17: &[f32],
    params48: &[f32],
    spv: &[u32],
    entry: &str,
) -> Result<ProdImage, String> {
    core.validate().map_err(|e| format!("场景校验: {e}"))?;
    cfg.validate().map_err(|e| format!("配置校验: {e}"))?;
    let cam = &core.camera;
    let pixel_count = (cam.width * cam.height) as usize;
    if mats12.len() != core.indices.len() * 12 {
        return Err(format!(
            "mats12 长度 {} ≠ 三角数×12 {}",
            mats12.len(),
            core.indices.len() * 12
        ));
    }
    if trinrm.len() != core.indices.len() * 9 {
        return Err(format!(
            "trinrm 长度 {} ≠ 三角数×9 {}",
            trinrm.len(),
            core.indices.len() * 9
        ));
    }
    if lights17.len() != core.lights.len() * 17 {
        return Err(format!(
            "lights 长度 {} ≠ 灯数×17 {}",
            lights17.len(),
            core.lights.len() * 17
        ));
    }
    if params48.len() != 48 {
        return Err(format!("params 长度 {} ≠ 48", params48.len()));
    }
    rng_guard(pixel_count, cfg.spp, cfg.max_bounces);
    let tris = prod::pack_prod_tris(core);
    let blas_refs: Vec<&[f32]> = vec![&tris];
    let instances = [RayQueryInstanceDesc {
        blas: 0,
        custom_index: 0,
        mask: 0xFF,
        sbt_record_offset: 0,
    }];
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    // 整帧 RNG 流(host 一遍;像素带分段直切连续行段——流布局 (px·spp+s)·stride
    // 像素主序连续,分段 = 整帧位级一致)。
    let stream = prod::sampler::generate(
        cfg.sampler,
        pixel_count,
        cfg.spp,
        cfg.max_bounces,
        cfg.seed,
    );
    let stride = (2 + 6 * cfg.max_bounces) as usize;
    let mats_b = bytes_f32(mats12);
    let tris_b = bytes_f32(&tris);
    let nrm_b = bytes_f32(trinrm);
    let lights_b = bytes_f32(lights17);
    // 像素带分段 dispatch(G12.4 M163 面:单 dispatch 墙钟上界规避 TDR;带内流
    // 直切,输出带内回读拼帧,位级一致)。
    let chunk_pixels = (1_048_576usize / cfg.spp as usize).clamp(512, 16384);
    let mut rgb: Vec<f32> = vec![0.0; pixel_count * 3];
    let mut sum_lum: Vec<f32> = vec![0.0; pixel_count];
    let mut sumsq_lum: Vec<f32> = vec![0.0; pixel_count];
    let mut samples: Vec<u32> = vec![0; pixel_count];
    let mut converged: Vec<f32> = vec![0.0; pixel_count];
    let mut rr_counters: Vec<f32> = vec![0.0; pixel_count * 4];
    let mut energy_levels: Vec<f32> = vec![0.0; pixel_count * 4];
    let mut base = 0usize;
    while base < pixel_count {
        let count = (pixel_count - base).min(chunk_pixels);
        let mut params_b = params48.to_vec();
        params_b[0] = count as f32; // [0] = 本带像素数
        params_b[36] = base as f32; // [36] = 带起点
        let rng_slice =
            &stream[base * cfg.spp as usize * stride..(base + count) * cfg.spp as usize * stride];
        let rng_b = bytes_f32(rng_slice);
        let params_bytes = bytes_f32(&params_b);
        let buffers = [
            vk::RayQueryBufferDesc::Input(&rng_b),
            vk::RayQueryBufferDesc::Input(&mats_b),
            vk::RayQueryBufferDesc::Input(&tris_b),
            vk::RayQueryBufferDesc::Input(&nrm_b),
            vk::RayQueryBufferDesc::Input(&lights_b),
            vk::RayQueryBufferDesc::Input(&params_bytes),
            vk::RayQueryBufferDesc::Output(count * 12),
            vk::RayQueryBufferDesc::Output(count * 8),
            vk::RayQueryBufferDesc::Output(count * 4),
            vk::RayQueryBufferDesc::Output(count * 4),
            vk::RayQueryBufferDesc::Output(count * 16),
            vk::RayQueryBufferDesc::Output(count * 16),
        ];
        let out = vk::run_ray_query_effects(
            &scene_desc,
            &[vk::RayQueryDispatchDesc {
                name: "g31_pt_gt",
                spv,
                entry,
                buffers: &buffers,
                push_constants: &[],
                groups: [count as u32, 1, 1],
            }],
        )?;
        let rb = out
            .readbacks
            .into_iter()
            .next()
            .ok_or("单 dispatch 缺回读")?;
        if rb.len() != 6 {
            return Err(format!("回读路数 {} ≠ 6", rb.len()));
        }
        let crgb = read_f32(&rb[0]);
        let cstats = read_f32(&rb[1]);
        let csamples = read_u32_le(&rb[2]);
        let cconv = read_f32(&rb[3]);
        let crr = read_f32(&rb[4]);
        let cenergy = read_f32(&rb[5]);
        rgb[base * 3..(base + count) * 3].copy_from_slice(&crgb);
        for px in 0..count {
            sum_lum[base + px] = cstats[px * 2];
            sumsq_lum[base + px] = cstats[px * 2 + 1];
        }
        samples[base..base + count].copy_from_slice(&csamples);
        converged[base..base + count].copy_from_slice(&cconv);
        rr_counters[base * 4..(base + count) * 4].copy_from_slice(&crr);
        energy_levels[base * 4..(base + count) * 4].copy_from_slice(&cenergy);
        base += count;
    }
    Ok(ProdImage {
        width: cam.width,
        height: cam.height,
        rgb,
        sum_lum,
        sumsq_lum,
        samples,
        converged,
        rr_counters,
        energy_levels,
        frame_label: "full_reference",
    })
}

/// 母版对照腿(--selftest equiv 专用):G12.4 parity run_device 原形——
/// mats = prod::pack_prod_mats 8 f32/tri、无 trinrm buffer、灯表
/// prod::pack_prod_lights 原样不写半径;params 与 gt 腿同一份传入。
fn run_device_frozen(
    core: &ProdScene,
    dist: &LightDist,
    cfg: &ProdConfig,
    params48: &[f32],
    spv: &[u32],
    entry: &str,
) -> Result<ProdImage, String> {
    core.validate().map_err(|e| format!("场景校验: {e}"))?;
    cfg.validate().map_err(|e| format!("配置校验: {e}"))?;
    let cam = &core.camera;
    let pixel_count = (cam.width * cam.height) as usize;
    if params48.len() != 48 {
        return Err(format!("params 长度 {} ≠ 48", params48.len()));
    }
    rng_guard(pixel_count, cfg.spp, cfg.max_bounces);
    let tris = prod::pack_prod_tris(core);
    let blas_refs: Vec<&[f32]> = vec![&tris];
    let instances = [RayQueryInstanceDesc {
        blas: 0,
        custom_index: 0,
        mask: 0xFF,
        sbt_record_offset: 0,
    }];
    let scene_desc = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &instances,
    };
    let stream = prod::sampler::generate(
        cfg.sampler,
        pixel_count,
        cfg.spp,
        cfg.max_bounces,
        cfg.seed,
    );
    let stride = (2 + 6 * cfg.max_bounces) as usize;
    let mats_b = bytes_f32(&prod::pack_prod_mats(core));
    let tris_b = bytes_f32(&tris);
    let lights_b = bytes_f32(&prod::pack_prod_lights(core, dist));
    let chunk_pixels = (1_048_576usize / cfg.spp as usize).clamp(512, 16384);
    let mut rgb: Vec<f32> = vec![0.0; pixel_count * 3];
    let mut sum_lum: Vec<f32> = vec![0.0; pixel_count];
    let mut sumsq_lum: Vec<f32> = vec![0.0; pixel_count];
    let mut samples: Vec<u32> = vec![0; pixel_count];
    let mut converged: Vec<f32> = vec![0.0; pixel_count];
    let mut rr_counters: Vec<f32> = vec![0.0; pixel_count * 4];
    let mut energy_levels: Vec<f32> = vec![0.0; pixel_count * 4];
    let mut base = 0usize;
    while base < pixel_count {
        let count = (pixel_count - base).min(chunk_pixels);
        let mut params_b = params48.to_vec();
        params_b[0] = count as f32;
        params_b[36] = base as f32;
        let rng_slice =
            &stream[base * cfg.spp as usize * stride..(base + count) * cfg.spp as usize * stride];
        let rng_b = bytes_f32(rng_slice);
        let params_bytes = bytes_f32(&params_b);
        let buffers = [
            vk::RayQueryBufferDesc::Input(&rng_b),
            vk::RayQueryBufferDesc::Input(&mats_b),
            vk::RayQueryBufferDesc::Input(&tris_b),
            vk::RayQueryBufferDesc::Input(&lights_b),
            vk::RayQueryBufferDesc::Input(&params_bytes),
            vk::RayQueryBufferDesc::Output(count * 12),
            vk::RayQueryBufferDesc::Output(count * 8),
            vk::RayQueryBufferDesc::Output(count * 4),
            vk::RayQueryBufferDesc::Output(count * 4),
            vk::RayQueryBufferDesc::Output(count * 16),
            vk::RayQueryBufferDesc::Output(count * 16),
        ];
        let out = vk::run_ray_query_effects(
            &scene_desc,
            &[vk::RayQueryDispatchDesc {
                name: "g12_pt_production",
                spv,
                entry,
                buffers: &buffers,
                push_constants: &[],
                groups: [count as u32, 1, 1],
            }],
        )?;
        let rb = out
            .readbacks
            .into_iter()
            .next()
            .ok_or("单 dispatch 缺回读")?;
        if rb.len() != 6 {
            return Err(format!("回读路数 {} ≠ 6", rb.len()));
        }
        let crgb = read_f32(&rb[0]);
        let cstats = read_f32(&rb[1]);
        let csamples = read_u32_le(&rb[2]);
        let cconv = read_f32(&rb[3]);
        let crr = read_f32(&rb[4]);
        let cenergy = read_f32(&rb[5]);
        rgb[base * 3..(base + count) * 3].copy_from_slice(&crgb);
        for px in 0..count {
            sum_lum[base + px] = cstats[px * 2];
            sumsq_lum[base + px] = cstats[px * 2 + 1];
        }
        samples[base..base + count].copy_from_slice(&csamples);
        converged[base..base + count].copy_from_slice(&cconv);
        rr_counters[base * 4..(base + count) * 4].copy_from_slice(&crr);
        energy_levels[base * 4..(base + count) * 4].copy_from_slice(&cenergy);
        base += count;
    }
    Ok(ProdImage {
        width: cam.width,
        height: cam.height,
        rgb,
        sum_lum,
        sumsq_lum,
        samples,
        converged,
        rr_counters,
        energy_levels,
        frame_label: "full_reference",
    })
}

fn digest_hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// 子模式 A:--render(EXR + receipt 落盘;双跑位级)
// ---------------------------------------------------------------------------

fn run_render(args: &[String]) {
    let mut scene_id = "bistro-interior".to_owned();
    let mut contract_path = DEFAULT_CONTRACT.to_owned();
    let mut expect = String::new();
    let mut gltf_path = String::new();
    let mut lights_preset = String::new();
    let mut w: u32 = 960;
    let mut h: u32 = 540;
    let mut spp: u32 = 64;
    let mut seed: u64 = prod::G12_PROD_SEED;
    let mut tau: f32 = 0.0;
    let mut spec_on = true;
    let mut smooth_on = true;
    let mut lamp_on = true;
    let mut lamp_gain: f32 = 4.0;
    let mut lamp_k: usize = 12;
    let mut spv_path = String::new();
    let mut out_dir = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--render" => {}
            "--scene" => scene_id = take_arg(args, &mut i),
            "--contract" => contract_path = take_arg(args, &mut i),
            "--expect-digest" => expect = take_arg(args, &mut i),
            "--gltf" => gltf_path = take_arg(args, &mut i),
            "--lights" => lights_preset = take_arg(args, &mut i),
            "--w" => {
                w = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--w 非 u32"))
            }
            "--h" => {
                h = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--h 非 u32"))
            }
            "--spp" => {
                spp = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--spp 非 u32"))
            }
            "--seed" => {
                seed = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--seed 非 u64"))
            }
            "--tau" => {
                tau = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--tau 非 f32"))
            }
            "--spec" => spec_on = parse_onoff("--spec", &take_arg(args, &mut i)),
            "--smooth-normals" => {
                smooth_on = parse_onoff("--smooth-normals", &take_arg(args, &mut i))
            }
            "--lamp" => lamp_on = parse_onoff("--lamp", &take_arg(args, &mut i)),
            "--lamp-gain" => {
                lamp_gain = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--lamp-gain 非 f32"))
            }
            "--lamp-k" => {
                lamp_k = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--lamp-k 非 usize"))
            }
            "--spv" => spv_path = take_arg(args, &mut i),
            "--out-dir" => out_dir = take_arg(args, &mut i),
            other => gfail(&format!("--render 未知参数 {other}")),
        }
        i += 1;
    }
    if lights_preset.is_empty() || spv_path.is_empty() || out_dir.is_empty() {
        gfail("--render 参数闭集缺行(--lights match|area / --spv / --out-dir 必填)");
    }
    if !(tau.is_finite() && tau > 0.0) {
        gfail("--tau 必填且须为有限正 f32");
    }
    if w == 0 || h == 0 || spp == 0 {
        gfail("--w/--h/--spp 须 > 0");
    }
    let match_mode = match lights_preset.as_str() {
        "match" => true,
        "area" => false,
        other => gfail(&format!("--lights {other} 越闭集(match|area)")),
    };
    if gltf_path.is_empty() {
        gltf_path = default_gltf(&scene_id).to_owned();
    }
    // ① 契约 → digest 校验(冻结锚或 --expect-digest 显式值;不等 fail)。
    let text = std::fs::read_to_string(&contract_path)
        .unwrap_or_else(|e| gfail(&format!("契约读取 {contract_path}: {e}")));
    let contract = parse_contract(&text).unwrap_or_else(|e| gfail(&e));
    let want = if expect.is_empty() {
        FROZEN_CONTRACT_DIGEST
    } else {
        expect.as_str()
    };
    if contract.digest != want {
        gfail(&format!(
            "契约 digest 不等拒出图:实算 {} ≠ 期望 {want}",
            contract.digest
        ));
    }
    // ② 装配(nrm 9f32/tri + mr 2f32/tri 侧表)。
    let mut nrm: Vec<f32> = Vec::new();
    let mut mr: Vec<f32> = Vec::new();
    let mut scene =
        assemble_scene_nrm_mr(&contract.raw, &scene_id, Path::new(&gltf_path), &mut nrm, &mut mr)
            .unwrap_or_else(|e| gfail(&e));
    // ③ match 预设可选 A1 灯提取 append(area 预设不加灯)。
    let contract_points = scene.points.len();
    let lamp_effective = match_mode && lamp_on;
    if lamp_effective {
        scene = apply_lamp_lights(
            scene,
            &LampOpt {
                enabled: true,
                gain: lamp_gain,
                max_k: lamp_k,
                contrib: 0.0,
                stats_out: String::new(),
            },
        );
    }
    let lamp_appended = scene.points.len() - contract_points;
    if nrm.len() != scene.indices.len() * 9 {
        gfail(&format!(
            "nrm 侧表长度 {} ≠ 三角数×9 {}",
            nrm.len(),
            scene.indices.len() * 9
        ));
    }
    // ④ tri_base 复算 + 装配折叠同源一致性自证。
    let (gltf2, _) = load_gltf(Path::new(&gltf_path)).unwrap_or_else(|e| gfail(&e));
    let gltf_dir = Path::new(&gltf_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let (base_tab, met_tab) = mat_base_table(&gltf2, &gltf_dir, scene.texture_mean_albedo);
    let tri_base = tri_base_table(&scene, &base_tab, &met_tab);
    // ⑤ core 校验视图 + ⑥ 分布/配置。
    let cb = build_core(&scene, match_mode, w, h);
    let dist = prod::build_light_distribution(&cb.core);
    let mut cfg = ProdConfig::production(spp, SamplerFamily::Sobol, tau);
    cfg.seed = seed;
    cfg.adaptive = None;
    // ⑦ 打包 + device 双跑。
    let mats12 = pack_mats12(&scene, &cb.core, &mr, &tri_base);
    let lights17 = pack_lights_radius(&cb.core, &dist, &cb.light_radius);
    let params = pack_params_gt(&cb.core, &cfg, spec_on, smooth_on);
    eprintln!(
        "{G31GT_TAG}: 装配 scene={scene_id} preset={} tris={} lights(p/q/t)={}/{}/{} emissive_tris={} lamp_appended={lamp_appended}",
        if match_mode { "match" } else { "area" },
        cb.core.indices.len(),
        cb.n_points,
        cb.n_quads,
        cb.n_tri_lights,
        scene.emissive_tri_count,
    );
    dev_gate();
    let (spv, spv_sha) = load_spv_sha(&spv_path);
    let entry = vk::entry_point_name(&spv).unwrap_or_else(|| gfail("SPIR-V 无 OpEntryPoint"));
    let t0 = std::time::Instant::now();
    let img_a = run_device_gt(&cb.core, &cfg, &mats12, &nrm, &lights17, &params, &spv, &entry)
        .unwrap_or_else(|e| gfail(&e));
    let img_b = run_device_gt(&cb.core, &cfg, &mats12, &nrm, &lights17, &params, &spv, &entry)
        .unwrap_or_else(|e| gfail(&e));
    let render_s = t0.elapsed().as_secs_f64();
    // ⑧ 双跑位级断言。
    let da = prod::prod_image_digest(&img_a);
    let db = prod::prod_image_digest(&img_b);
    if da != db {
        gfail(&format!(
            "固定 seed 双跑位级漂移(确定性协议违例):sha256:{} ≠ sha256:{}",
            digest_hex(&da),
            digest_hex(&db)
        ));
    }
    // ⑨ EXR + receipt 落盘。
    let preset = if match_mode { "match" } else { "area" };
    let out = PathBuf::from(&out_dir);
    std::fs::create_dir_all(&out).unwrap_or_else(|e| gfail(&format!("输出目录 {out_dir}: {e}")));
    let frame_path = out.join(format!("{scene_id}_ptgt_{preset}_spp{spp}.exr"));
    write_exr(&frame_path, w, h, &img_a.rgb, &contract.digest).unwrap_or_else(|e| gfail(&e));
    let content = frame_content_digest(w, h, 3, &img_a.rgb);
    let mean = img_a.mean_luminance();
    let pixel_count = (w as usize) * (h as usize);
    let chunk_pixels = (1_048_576usize / spp as usize).clamp(512, 16384);
    let bands = pixel_count.div_ceil(chunk_pixels);
    let rng_floats =
        pixel_count as u64 * spp as u64 * prod::prod_sample_stride(cfg.max_bounces) as u64;
    let notes = [
        "贴图均值口径:albedo = DDS mip0 逐 texel sRGB→线性均值 × baseColorFactor(无逐 texel 采样)",
        "无 ambient/sky 项:契约 sun/sky 强度不消费,纯路径追踪直接+间接",
        "glass/透明材质按不透明 Lambert 折叠处理(透射链范围外)",
        "match 预设 emissive 三角不进 NEE 灯表:命中 w=1 无偏路径承载,小灯高方差如实登记",
    ];
    let notes_json = notes.iter().map(|s| jstr(s)).collect::<Vec<_>>().join(", ");
    let receipt = format!(
        "{{\n  \"schema\": \"rurix.g31.pt_gt_receipt.v1\",\n  \"scene_id\": {},\n  \"preset\": {},\n  \"width\": {w},\n  \"height\": {h},\n  \"spp\": {spp},\n  \"seed\": {seed},\n  \"sampler\": \"sobol_class_seed_perturbed\",\n  \"tau\": {tau},\n  \"max_bounces\": 4,\n  \"spec\": {spec_on},\n  \"smooth_normals\": {smooth_on},\n  \"lamp\": {{\"on\": {lamp_effective}, \"gain\": {lamp_gain}, \"k\": {lamp_k}, \"appended\": {lamp_appended}}},\n  \"light_counts\": {{\"points\": {}, \"quads\": {}, \"tris\": {}}},\n  \"tri_count\": {},\n  \"emissive_tri_count\": {},\n  \"rng_floats\": {rng_floats},\n  \"chunk_pixels\": {chunk_pixels},\n  \"bands\": {bands},\n  \"gltf_path\": {},\n  \"gltf_sha256\": {},\n  \"contract_digest\": {},\n  \"spv_sha256\": {},\n  \"frame_file\": {},\n  \"frame_content_digest\": {},\n  \"double_run_bitexact\": true,\n  \"mean_luminance\": {mean},\n  \"render_s\": {render_s},\n  \"ev100\": {},\n  \"exposure_note\": \"EXR 为未曝光 scene-linear;查看需 ×2^(−ev100)\",\n  \"caliber_notes\": [{notes_json}]\n}}\n",
        jstr(&scene_id),
        jstr(preset),
        cb.n_points,
        cb.n_quads,
        cb.n_tri_lights,
        cb.core.indices.len(),
        scene.emissive_tri_count,
        jstr(&gltf_path.replace('\\', "/")),
        jstr(&format!("sha256:{}", scene.gltf_sha256)),
        jstr(&contract.digest),
        jstr(&spv_sha),
        jstr(&frame_path.to_string_lossy().replace('\\', "/")),
        jstr(&content),
        scene.ev100,
    );
    let receipt_path = out.join("pt_receipt.json");
    std::fs::write(&receipt_path, &receipt)
        .unwrap_or_else(|e| gfail(&format!("receipt 落盘: {e}")));
    println!(
        "{G31GT_TAG}: PASS render scene={scene_id} preset={preset} {w}x{h} spp={spp} mean_lum={mean:.6} lights(p/q/t)={}/{}/{} digest={content} double_run=bitexact render_s={render_s:.1}",
        cb.n_points, cb.n_quads, cb.n_tri_lights,
    );
}

// ---------------------------------------------------------------------------
// 子模式 B:--selftest equiv(fork gates off ⇒ 母版逐位等价机核)
// ---------------------------------------------------------------------------

fn run_selftest_equiv(args: &[String]) {
    let mut contract_path = DEFAULT_CONTRACT.to_owned();
    let mut gltf_path = String::new();
    let mut tau: f32 = 0.0;
    let mut spv_path = String::new();
    let mut spv_frozen_path = String::new();
    let mut w: u32 = 192;
    let mut h: u32 = 108;
    let mut spp: u32 = 2;
    let mut seed: u64 = prod::G12_PROD_SEED;
    let mut i = 3; // [bin, --selftest, equiv, ...]
    while i < args.len() {
        match args[i].as_str() {
            "--contract" => contract_path = take_arg(args, &mut i),
            "--gltf" => gltf_path = take_arg(args, &mut i),
            "--tau" => {
                tau = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--tau 非 f32"))
            }
            "--spv" => spv_path = take_arg(args, &mut i),
            "--spv-frozen" => spv_frozen_path = take_arg(args, &mut i),
            "--w" => {
                w = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--w 非 u32"))
            }
            "--h" => {
                h = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--h 非 u32"))
            }
            "--spp" => {
                spp = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--spp 非 u32"))
            }
            "--seed" => {
                seed = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--seed 非 u64"))
            }
            other => gfail(&format!("--selftest equiv 未知参数 {other}")),
        }
        i += 1;
    }
    if spv_path.is_empty() || spv_frozen_path.is_empty() {
        gfail("--selftest equiv 参数闭集缺行(--spv fork / --spv-frozen 母版必填)");
    }
    if !(tau.is_finite() && tau > 0.0) {
        gfail("--tau 必填且须为有限正 f32");
    }
    let scene_id = "bistro-interior";
    if gltf_path.is_empty() {
        gltf_path = default_gltf(scene_id).to_owned();
    }
    // bistro area 预设(lamp off)——同一 core/RNG/params 双腿。
    let text = std::fs::read_to_string(&contract_path)
        .unwrap_or_else(|e| gfail(&format!("契约读取 {contract_path}: {e}")));
    let contract = parse_contract(&text).unwrap_or_else(|e| gfail(&e));
    let mut nrm: Vec<f32> = Vec::new();
    let mut mr: Vec<f32> = Vec::new();
    let scene =
        assemble_scene_nrm_mr(&contract.raw, scene_id, Path::new(&gltf_path), &mut nrm, &mut mr)
            .unwrap_or_else(|e| gfail(&e));
    if nrm.len() != scene.indices.len() * 9 {
        gfail(&format!(
            "nrm 侧表长度 {} ≠ 三角数×9 {}",
            nrm.len(),
            scene.indices.len() * 9
        ));
    }
    // equiv 快路:base 槽喂 albedo 拷贝(spec_en=0 时 kernel F0 面经乘法门
    // ×0 不消费,免二次 glTF/DDS 读——tri_base 复算与折叠一致性自证由
    // --render 腿承载)。
    let tri_base: Vec<[f32; 3]> = scene.albedo.iter().map(|a| clamp01_3(*a)).collect();
    let cb = build_core(&scene, false, w, h);
    let dist = prod::build_light_distribution(&cb.core);
    let mut cfg = ProdConfig::production(spp, SamplerFamily::Sobol, tau);
    cfg.seed = seed;
    cfg.adaptive = None;
    // gt 腿输入:mats12 + 真实 trinrm + 灯表半径全 0(area 无点光,pack 原样);
    // params 两腿同一份(aspect 补丁 + [37]=[38]=0.0 gates off)。
    let mats12 = pack_mats12(&scene, &cb.core, &mr, &tri_base);
    let lights17 = pack_lights_radius(&cb.core, &dist, &cb.light_radius);
    let params = pack_params_gt(&cb.core, &cfg, false, false);
    dev_gate();
    let (spv_gt_raw, _) = load_spv_sha(&spv_path);
    let (spv_fr_raw, _) = load_spv_sha(&spv_frozen_path);
    // 跨模块驱动 FMA 收缩禁面:两腿 SPV 同注 NoContraction(B4 探针同律,
    // 内存后处理文件 0-byte)——不同模块各自收缩模式不同会破坏"同表达式同
    // 求值"的位级前提,注入后共享表达式 DAG 逐 op IEEE 精确同值。
    let spv_gt = spv_inject_no_contraction(&spv_gt_raw);
    let spv_fr = spv_inject_no_contraction(&spv_fr_raw);
    let entry_gt =
        vk::entry_point_name(&spv_gt).unwrap_or_else(|| gfail("fork SPIR-V 无 OpEntryPoint"));
    let entry_fr =
        vk::entry_point_name(&spv_fr).unwrap_or_else(|| gfail("frozen SPIR-V 无 OpEntryPoint"));
    let img_fr = run_device_frozen(&cb.core, &dist, &cfg, &params, &spv_fr, &entry_fr)
        .unwrap_or_else(|e| gfail(&format!("frozen 腿: {e}")));
    let img_gt = run_device_gt(
        &cb.core, &cfg, &mats12, &nrm, &lights17, &params, &spv_gt, &entry_gt,
    )
    .unwrap_or_else(|e| gfail(&format!("gt 腿: {e}")));
    // rgb/sum_lum/sumsq_lum/samples/converged(及 rr/energy)全输出位级相等
    // ——prod_image_digest 覆盖全部输出字节。
    let d_fr = prod::prod_image_digest(&img_fr);
    let d_gt = prod::prod_image_digest(&img_gt);
    if d_fr == d_gt {
        println!(
            "{G31GT_TAG}: PASS selftest=equiv {w}x{h} spp={spp} digest_frozen=sha256:{} digest_gt=sha256:{} bitexact=true",
            digest_hex(&d_fr),
            digest_hex(&d_gt),
        );
    } else {
        // 失配数值诊断(定位面:ULP 级散布 = 跨模块代码生成残差;粗差/聚集 =
        // fork 门算术真 bug)。
        let mut n_diff = 0usize;
        let mut max_abs = 0.0f32;
        let mut max_ulp: u32 = 0;
        let mut first: Option<(usize, f32, f32)> = None;
        for (k, (a, b)) in img_fr.rgb.iter().zip(img_gt.rgb.iter()).enumerate() {
            if a.to_bits() != b.to_bits() {
                n_diff += 1;
                let d = (a - b).abs();
                if d > max_abs {
                    max_abs = d;
                }
                let ulp = a.to_bits().abs_diff(b.to_bits());
                if ulp > max_ulp {
                    max_ulp = ulp;
                }
                if first.is_none() {
                    first = Some((k, *a, *b));
                }
            }
        }
        let sl_diff = img_fr
            .sum_lum
            .iter()
            .zip(img_gt.sum_lum.iter())
            .filter(|(a, b)| a.to_bits() != b.to_bits())
            .count();
        let sq_diff = img_fr
            .sumsq_lum
            .iter()
            .zip(img_gt.sumsq_lum.iter())
            .filter(|(a, b)| a.to_bits() != b.to_bits())
            .count();
        let rr_diff = img_fr
            .rr_counters
            .iter()
            .zip(img_gt.rr_counters.iter())
            .filter(|(a, b)| a.to_bits() != b.to_bits())
            .count();
        eprintln!(
            "{G31GT_TAG}: FAIL selftest=equiv digest_frozen=sha256:{} ≠ digest_gt=sha256:{}(fork gates off ≠ 母版逐位等价)rgb_diff={n_diff}/{} max_abs={max_abs:e} max_ulp={max_ulp} first={first:?} sum_lum_diff={sl_diff} sumsq_diff={sq_diff} rr_diff={rr_diff} samples_eq={} converged_eq={}",
            digest_hex(&d_fr),
            digest_hex(&d_gt),
            img_fr.rgb.len(),
            img_fr.samples == img_gt.samples,
            img_fr.converged == img_gt.converged,
        );
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// 子模式 C:--selftest furnace(GGX 白炉能量守恒带)
// ---------------------------------------------------------------------------

fn run_selftest_furnace(args: &[String]) {
    let mut spv_path = String::new();
    let mut spp: u32 = 256;
    let mut seed: u64 = prod::G12_PROD_SEED;
    let mut tau: f32 = 0.0;
    let mut i = 3; // [bin, --selftest, furnace, ...]
    while i < args.len() {
        match args[i].as_str() {
            "--spv" => spv_path = take_arg(args, &mut i),
            "--spp" => {
                spp = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--spp 非 u32"))
            }
            "--seed" => {
                seed = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--seed 非 u64"))
            }
            "--tau" => {
                tau = take_arg(args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| gfail("--tau 非 f32"))
            }
            other => gfail(&format!("--selftest furnace 未知参数 {other}")),
        }
        i += 1;
    }
    if spv_path.is_empty() {
        gfail("--selftest furnace 参数闭集缺行(--spv 必填)");
    }
    if !(tau.is_finite() && tau > 0.0) {
        gfail("--tau 必填且须为有限正 f32");
    }
    // 白炉 core(64×64 相机,5 白墙 + 天花 quad 灯 Le=4)。
    let core = prod::g12_furnace_scene();
    core.validate()
        .unwrap_or_else(|e| gfail(&format!("furnace 场景校验: {e}")));
    let dist = prod::build_light_distribution(&core);
    let mut cfg = ProdConfig::production(spp, SamplerFamily::Sobol, tau);
    cfg.seed = seed;
    cfg.adaptive = None;
    let zero_radius = vec![0.0f32; core.lights.len()];
    let lights17 = pack_lights_radius(&core, &dist, &zero_radius);
    // trinrm 全 0(nrm off,params[38]=0——kernel 不消费面零值兜底)。
    let trinrm = vec![0.0f32; core.indices.len() * 9];
    dev_gate();
    let (spv, _) = load_spv_sha(&spv_path);
    let entry = vk::entry_point_name(&spv).unwrap_or_else(|| gfail("SPIR-V 无 OpEntryPoint"));
    struct FurnaceCase {
        name: &'static str,
        spec: bool,
        metallic: f32,
        roughness: f32,
    }
    let cases = [
        FurnaceCase { name: "baseline_spec_off", spec: false, metallic: 0.0, roughness: 1.0 },
        FurnaceCase { name: "spec_m0_r100", spec: true, metallic: 0.0, roughness: 1.0 },
        FurnaceCase { name: "spec_m0_r030", spec: true, metallic: 0.0, roughness: 0.3 },
        FurnaceCase { name: "spec_m1_r005", spec: true, metallic: 1.0, roughness: 0.05 },
        FurnaceCase { name: "spec_m1_r050", spec: true, metallic: 1.0, roughness: 0.5 },
        FurnaceCase { name: "spec_m1_r100", spec: true, metallic: 1.0, roughness: 1.0 },
    ];
    let mut means: Vec<f64> = Vec::with_capacity(cases.len());
    for c in &cases {
        let mats12 = pack_mats12_furnace(&core, c.metallic, c.roughness);
        let params = pack_params_gt(&core, &cfg, c.spec, false);
        let img = run_device_gt(&core, &cfg, &mats12, &trinrm, &lights17, &params, &spv, &entry)
            .unwrap_or_else(|e| gfail(&format!("furnace case {}: {e}", c.name)));
        let mean = img.mean_luminance();
        println!(
            "{{\"schema\":\"rurix.g31.pt_gt_furnace.v1\",\"case\":{},\"spec\":{},\"metallic\":{},\"roughness\":{},\"spp\":{spp},\"seed\":{seed},\"tau\":{tau},\"mean_luminance\":{mean}}}",
            jstr(c.name),
            c.spec,
            c.metallic,
            c.roughness,
        );
        means.push(mean);
    }
    let baseline = means[0];
    // ① 全部 mean 有限且 >0。
    for (k, m) in means.iter().enumerate() {
        if !(m.is_finite() && *m > 0.0) {
            gfail(&format!(
                "furnace case {} mean_luminance {m} 非有限/非正",
                cases[k].name
            ));
        }
    }
    // ② 不产能量(带登记):每 case mean ≤ baseline×1.05。glTF 形
    //    (1−F)·diffuse + spec 组合在中粗糙介质区有已知 ~1%/bounce 超单位面
    //    (Kulla-Conty 单散射补偿缺失的对偶),4 反弹炉内均衡放大实测 +3.4%
    //    (m0_r030)——如实登记不冒充严格守恒;>5% 即真造能量违例。
    for (k, m) in means.iter().enumerate() {
        if *m > baseline * 1.05 {
            gfail(&format!(
                "furnace case {} mean {m} > baseline×1.05 = {}(造能量违例)",
                cases[k].name,
                baseline * 1.05
            ));
        }
    }
    // ③ metal 物理面(max_bounces=4 截断口径):全部 metal < baseline
    //    (Fresnel 单散射损失 + 镜面反射链 >4 反弹截断损失);r=1 < r=0.5
    //    (粗金属 G 遮蔽单散射损失主导)。r=0.05 vs r=0.5 排序为截断主导
    //    (镜面链走不完)——measured 登记不设断言。
    let (m005, m050, m100) = (means[3], means[4], means[5]);
    for (nm, m) in [("m1_r005", m005), ("m1_r050", m050), ("m1_r100", m100)] {
        if m >= baseline {
            gfail(&format!(
                "furnace metal {nm} mean {m} ≥ baseline {baseline}(金属应低于白炉基线:截断+单散射损失violated)"
            ));
        }
    }
    if m100 >= m050 {
        gfail(&format!(
            "furnace metal 粗档违例:mean(1,1)={m100} ≥ mean(1,0.5)={m050}(粗金属单散射损失应更大)"
        ));
    }
    // ④ dielectric (0,1) ∈ [0.90,1.02]×baseline(F0=0.04 微扰带)。
    let d = means[1];
    if !(d >= baseline * 0.90 && d <= baseline * 1.02) {
        gfail(&format!(
            "furnace dielectric(0,1) mean {d} 越带 [0.90,1.02]×baseline={baseline}"
        ));
    }
    println!(
        "{G31GT_TAG}: PASS selftest=furnace spp={spp} baseline={baseline:.6} dielectric={d:.6} metal=({m005:.6},{m050:.6},{m100:.6})"
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        gfail("缺子模式(--render / --selftest equiv|furnace)");
    }
    match args[1].as_str() {
        "--render" => run_render(&args),
        "--selftest" => match args.get(2).map(|s| s.as_str()).unwrap_or("") {
            "equiv" => run_selftest_equiv(&args),
            "furnace" => run_selftest_furnace(&args),
            other => gfail(&format!("--selftest {other} 越闭集(equiv|furnace)")),
        },
        other => gfail(&format!("未知子模式 {other}")),
    }
}
