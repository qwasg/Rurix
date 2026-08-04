//! G7.5b:RD-038 字面分项行 3「HW 光栅」的 device 腿 —— **真实 Vulkan graphics
//! pipeline**(VS+FS+固定功能光栅)产出 VisBuffer,与 W2 软件光栅
//! (`visbuffer_sw_u64.rx` compute)在**同场景、同投影、同 VisBuffer ABI** 下
//! 整数域逐字对拍 **diff = 0**(验收门 G-G7-7 轴一;CI 步骤 95 device 段)。
//!
//! ## 覆盖规则(RFC-0018 §E 裁定;RXS-0303)
//! 覆盖语义唯一权威 = SW 侧精确 f32 边函数 + top-left(`visbuffer_sw_u64.rx` /
//! host `VisBufferCpu` 字面)。HW 腿 = 保守光栅 **OVERESTIMATE** 超集派发 fragment
//! (`RasterPass.conservative = Some`,pipeline pNext 链
//! `VkPipelineRasterizationConservativeStateCreateInfoEXT`)+ FS 内**逐字复刻**
//! SW 判定过滤(`visbuffer_hw_fs` 语料 = `conformance/vulkan/accept/
//! vk_hw_raster_visbuffer_fs.rx` 同源文本,判定段与 SW kernel 逐字同构);
//! `inside` 不成立不写(无 discard)。DeviceCaps 无该扩展 → **fail-closed 硬红**
//! (RXS-0303 L3,不静默降级、不启用降级臂)。
//!
//! ## 输入同源纪律(diff=0 的前提,设计 §4.2)
//! G7_SCENE_FREEZE 冻结场景(764 三角形/3 实例/冻结相机)在 **128×72**(uc06
//! internal 分辨率,与 SW 基线 9216 词口径一致)上的投影:host 按 `run_frame`
//! 既有剔除序(`cull_clusters` → `compact_draw_args`)+ `raster_clusters` 逐字
//! 同构的投影表达式展开 `triangles`(9 f32/三角形:三顶点屏幕坐标+NDC z)与
//! `ids`(cluster=可见簇序 / tri=簇内序);**同一份**缓冲喂 SW compute 腿(SSBO)
//! 与 HW raster 腿(flat 顶点属性),host `VisBufferCpu` oracle 消费同一份屏幕
//! 坐标 —— 三方对拍:`HW == SW`(G-G7-7 判据本体,diff_pixels == 0,零容差);
//! SW 与 host `VisBufferCpu` **覆盖集合**相等(inside 集对齐)。冻结场景 packed
//! 全屏逐位受 host/GPU FMA 限制(G7.5 残差归因同构),全屏 SW↔host 逐位锚由
//! W2 合成场景(步骤 95 `sw_baseline`)承担。
//!
//! ## 顶点供给(设计 §4.3;provoking vertex / guard-band 裁剪免疫)
//! 每三角形展开 3 顶点(stride = 72B):`pos vec4`(offset 0,屏幕坐标反推 NDC:
//! `ndc = s/half − 1`,z=0.5,w=1,视口无 y 翻转,往返误差 ≤1 ULP 由 OVERESTIMATE
//! 吸收)+ `va/vb/vc vec4`(offset 16/32/48,三顶点屏幕坐标**原始 f32 位型**)+
//! `ids uvec2`(offset 64,`VK_FORMAT_R32G32_UINT`);三顶点的 va/vb/vc/ids
//! **完全相同** → flat 插值取哪个 provoking vertex 都一样(RFC-0018 §E2 论证第 5 项)。
//!
//! ## RED 轴(数据流反证;篡改只落 HW 顶点流,SSBO 输入与 oracle 不动)
//! - `tamper-varying`:选 oracle winner 像素数最多的三角形,交换其 flat `vb`/`vc`
//!   → FS 内 `area` 反号(`area < 0.0` 不成立)→ 该三角形整体不写 → diff 必 > 0;
//! - `tamper-ids`:同一三角形 `ids.cluster += 1` → pack 值漂移 → diff 必 > 0。

use std::collections::BTreeMap;

use rurix_geom_build::Mat4 as GeomMat4;
use rurix_geom_build::cull_ref::{CullView, cull_clusters};
use rurix_render::geometry::cull::{VisibleCluster, compact_draw_args};
use rurix_render::geometry::gpu_scene::transform_point;
use rurix_render::geometry::visbuffer::{VISBUFFER_CLEAR, VisBufferCpu};
use rurix_render::graph::types::visbuffer_unpack;
use rurix_rt::render_exec::{
    self, Bindings, BufferDesc, BufferUsage, ColorAttachmentRef, ComputePass,
    ConservativeRasterDesc, DispatchSpec, DrawSpec, KernelWave, Pass, RasterPass, Readback,
    ResourceDesc, TargetState, TexFormat, TextureDesc, TextureUsage, VertexData,
};

use crate::scene::Uc06Scene;

/// SW compute 腿(W2 既有 kernel,0-byte;与 `device_kernels.rs` 同一产物)。
const SW_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/visbuffer_sw_u64.spv"));
/// HW 腿图形着色对(build.rs 图形编译腿产物;源 = conformance accept 语料同源文本)。
const HW_VS_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/visbuffer_hw_vs.spv"));
const HW_FS_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/visbuffer_hw_fs.spv"));

/// 对拍分辨率(uc06 internal = 256×144/2;SW 基线 9216 词口径,G7_SCENE_FREEZE §3)。
const VIS_W: u32 = 128;
const VIS_H: u32 = 72;
const VIS_WORDS: usize = (VIS_W * VIS_H) as usize;

/// 顶点属性格式(Vulkan 枚举值;SDK 1.3.296 `vulkan_core.h`,vk_triangle 先例同律)。
const FORMAT_R32G32B32A32_SFLOAT: u32 = 109;
const FORMAT_R32G32_UINT: u32 = 101;
/// 顶点布局(设计 §4.3):`(location, format, offset)`,stride = 72B。
const HW_VERTEX_STRIDE: u32 = 72;
const HW_VERTEX_ATTRS: [(u32, u32, u32); 5] = [
    (0, FORMAT_R32G32B32A32_SFLOAT, 0),  // pos
    (1, FORMAT_R32G32B32A32_SFLOAT, 16), // va
    (2, FORMAT_R32G32B32A32_SFLOAT, 32), // vb
    (3, FORMAT_R32G32B32A32_SFLOAT, 48), // vc
    (4, FORMAT_R32G32_UINT, 64),         // ids
];

/// RED 轴篡改类别(只落 HW 顶点流;SSBO 输入与 host oracle 不动)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tamper {
    None,
    /// 交换选中三角形的 flat `vb`/`vc` → FS 判 `area > 0` 整体不写。
    Varying,
    /// 选中三角形 `ids.cluster += 1` → pack 值漂移。
    Ids,
}

/// HW 光栅对拍结果(`--g75-hw-raster` 单行 JSON 与步骤 95 evidence 字段源,设计 §4.5)。
#[derive(Debug, Clone)]
pub struct G75HwRasterResults {
    pub device_name: String,
    /// 展开(剔除后投影有效)三角形数。
    pub triangles: u32,
    /// 对拍词数(= 128×72 = 9216)。
    pub pixels: u32,
    /// SW/HW 逐词不等数(G-G7-7 判据:必须 == 0,零容差)。
    pub diff_pixels: u32,
    /// HW 腿覆盖(≠ clear)词数。
    pub hw_covered_words: u32,
    /// SW 腿覆盖词数(与 HW 相等且 > 0 = 非退化)。
    pub sw_covered_words: u32,
    /// SW 腿正确性锚:与 host `VisBufferCpu` **覆盖集合**逐像素相等(inside 集对齐)。
    /// 冻结场景下 packed 全屏逐位受 host/GPU FMA 限制,全屏 SW↔host 逐位锚由
    /// W2 合成场景 `sw_baseline` 承担;G-G7-7 本体 = HW==SW(`diff_pixels==0`)。
    pub oracle_bitexact: bool,
    /// 本机保守光栅属性快照(运行时实采,RXS-0303 IR1)。
    pub conservative_props: render_exec::ConservativeRasterProps,
    /// 管线形态标识(RFC-0018 §E2 方案 A;降级臂未启用)。
    pub pipeline: &'static str,
    /// FS SPIR-V 声明的 capability 集(产物字节机器解析,非人工声明)。
    pub spirv_caps: Vec<&'static str>,
}

impl G75HwRasterResults {
    pub fn all_pass(&self) -> bool {
        self.diff_pixels == 0
            && self.hw_covered_words == self.sw_covered_words
            && self.sw_covered_words > 0
            && self.oracle_bitexact
    }

    /// 单行 JSON(`--g75-hw-raster` 输出;步骤 95 evidence 字段源)。
    pub fn json(&self) -> String {
        let caps: Vec<String> = self.spirv_caps.iter().map(|c| format!("\"{c}\"")).collect();
        format!(
            "{{\"subject\":\"uc06_g75_hw_raster\",\"device_name\":\"{}\",\
             \"triangles\":{},\"pixels\":{},\"diff_pixels\":{},\
             \"hw_covered_words\":{},\"sw_covered_words\":{},\"oracle_bitexact\":{},\
             \"conservative_props\":{{\"primitive_overestimation_size\":{:.9e},\
             \"max_extra_primitive_overestimation_size\":{:.9e},\
             \"extra_primitive_overestimation_size_granularity\":{:.9e},\
             \"degenerate_triangles_rasterized\":{}}},\
             \"pipeline\":\"{}\",\"spirv_caps\":[{}],\"all_pass\":{}}}",
            self.device_name,
            self.triangles,
            self.pixels,
            self.diff_pixels,
            self.hw_covered_words,
            self.sw_covered_words,
            self.oracle_bitexact,
            self.conservative_props.primitive_overestimation_size,
            self.conservative_props
                .max_extra_primitive_overestimation_size,
            self.conservative_props
                .extra_primitive_overestimation_size_granularity,
            self.conservative_props.degenerate_triangles_rasterized,
            self.pipeline,
            caps.join(","),
            self.all_pass(),
        )
    }
}

// ── 小工具(镜像 device_g75.rs / device_kernels.rs 同名件)──────────────────────

fn storage<'a>(size: usize, data: Option<&'a [u8]>) -> ResourceDesc<'a> {
    ResourceDesc::Buffer(BufferDesc {
        size: size as u64,
        usage: BufferUsage {
            storage: true,
            ..Default::default()
        },
        data,
    })
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_u32(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_u64(v: &[u64]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_u64(b: &[u8]) -> Vec<u64> {
    b.chunks_exact(8)
        .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect()
}

/// 能力门(三态):无 loader / W2 能力链缺失 → `None`(dev-env degrade);
/// 保守光栅扩展不在位 → `Some(Err)`(**fail-closed 硬红**,RXS-0303 L3 ——
/// 本机已探明支持〔RFC-0018 §E 本机探测 rev 1〕,缺失即异常,不启用降级臂)。
fn hw_gate() -> Option<Result<render_exec::DeviceCaps, String>> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("[uc06 G7.5b] SKIP: vulkan loader 不可用(dev-env degrade)");
        return None;
    }
    let caps = render_exec::probe_device_caps().ok()?;
    if let Err(e) = render_exec::require_wave(&caps, KernelWave::W2) {
        eprintln!("[uc06 G7.5b] SKIP: W2 能力链缺失({e})");
        return None;
    }
    if caps.conservative_raster.is_none() {
        return Some(Err(format!(
            "fail-closed: 设备 `{}` 无 VK_EXT_conservative_rasterization(RXS-0303 L3;\
             覆盖超集无保证,不静默降级、降级臂未启用)",
            caps.device_name
        )));
    }
    if !caps.fragment_stores_and_atomics {
        return Some(Err(format!(
            "fail-closed: 设备 `{}` 无 fragmentStoresAndAtomics core feature\
             (FS 写 SSBO/u64 原子前提,VUID-RuntimeSpirv-NonWritable-06340)",
            caps.device_name
        )));
    }
    Some(Ok(caps))
}

/// 冻结场景 → 128×72 投影展开(输入同源面;三方共用的唯一一份数据)。
///
/// 剔除序**逐字镜像** `pipeline::run_frame`:`CullView` → `cull_clusters` →
/// 逐 mesh 逐簇枚举 → `compact_draw_args`(bin 32px)→ hw+sw 链接;投影表达式
/// **逐字镜像** `geometry::visbuffer::raster_clusters`(含 `clip.w ≤ 1e-20`
/// 保守丢弃与 ndc z clamp)。host oracle 消费同一份屏幕坐标逐三角形
/// `raster_triangle`(oracle 代码 0-byte,仅调用)。
fn project_frozen_scene(scene: &Uc06Scene) -> (Vec<f32>, Vec<u32>, VisBufferCpu) {
    let cam = crate::pipeline::camera_matrices(VIS_W, VIS_H);
    let view = CullView::new(
        GeomMat4(cam.view.m),
        GeomMat4(cam.proj.m),
        cam.eye,
        VIS_H as f32,
    );
    let (visible_set, _stats) = cull_clusters(&scene.clusters, &view);
    let visible: Vec<VisibleCluster> = scene
        .meshes
        .iter()
        .enumerate()
        .flat_map(|(iid, m)| {
            let lo = m.cluster_offset;
            let set = &visible_set;
            (0..m.dag.records.len() as u32).filter_map(move |c| {
                let gid = lo + c;
                set.contains(&gid).then_some(VisibleCluster {
                    instance: iid as u32,
                    cluster: gid,
                })
            })
        })
        .collect();
    let draw_args = compact_draw_args(
        &visible,
        scene.scene.instances(),
        &scene.clusters,
        &cam.cull,
        32.0,
    );
    let all_visible: Vec<VisibleCluster> = draw_args
        .hw_clusters
        .iter()
        .chain(draw_args.sw_clusters.iter())
        .cloned()
        .collect();

    let vp = cam.view_proj;
    let (w_px, h_px) = (VIS_W as f32, VIS_H as f32);
    let mut triangles: Vec<f32> = Vec::new();
    let mut ids: Vec<u32> = Vec::new();
    let mut oracle = VisBufferCpu::new(VIS_W, VIS_H);
    for (vis_idx, vc) in all_visible.iter().enumerate() {
        let inst = &scene.scene.instances()[vc.instance as usize];
        let c = &scene.clusters[vc.cluster as usize];
        for t in 0..c.triangle_count {
            let mut screen = [[0.0f32; 3]; 3];
            let mut valid = true;
            for (k, sv) in screen.iter_mut().enumerate() {
                let local = scene.indices[(c.triangle_offset + 3 * t) as usize + k];
                let obj = scene.vertices[(c.vertex_offset + local) as usize];
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
                ids.extend_from_slice(&[vis_idx as u32, t]);
                oracle.raster_triangle(&screen, vis_idx as u32, t);
            }
        }
    }
    (triangles, ids, oracle)
}

/// oracle winner 像素数最多的三角形在展开列表中的下标(RED 轴受害者:保证
/// 篡改后 diff > 0 非退化)。确定性 tie-break = 按 (count, cluster, tri) 全序。
fn dominant_triangle(oracle: &VisBufferCpu, ids: &[u32]) -> Option<usize> {
    let mut counts: BTreeMap<(u32, u32), u32> = BTreeMap::new();
    for &w in &oracle.data {
        if w != VISBUFFER_CLEAR {
            let (_, c, t) = visbuffer_unpack(w);
            *counts.entry((c, t)).or_insert(0) += 1;
        }
    }
    let (&(c, t), _) = counts.iter().max_by_key(|&(&(c, t), &n)| (n, c, t))?;
    ids.chunks_exact(2).position(|p| p[0] == c && p[1] == t)
}

/// 顶点流构建(设计 §4.3):每三角形 3 顶点 × 72B;三顶点 va/vb/vc/ids 完全相同;
/// `pos` 由屏幕坐标反推 NDC(`ndc = s/half − 1`;视口无 y 翻转,z=0.5,w=1)。
/// `tamper`/`victim`:RED 轴对单个三角形的顶点流篡改(SSBO 输入不动)。
fn build_vertex_stream(triangles: &[f32], ids: &[u32], tamper: Tamper, victim: usize) -> Vec<u8> {
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
        let mut vb = [triangles[b + 3], triangles[b + 4], triangles[b + 5], 0.0];
        let mut vc = [triangles[b + 6], triangles[b + 7], triangles[b + 8], 0.0];
        let mut id = [ids[i * 2], ids[i * 2 + 1]];
        if i == victim {
            match tamper {
                Tamper::Varying => std::mem::swap(&mut vb, &mut vc),
                Tamper::Ids => id[0] += 1,
                Tamper::None => {}
            }
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

/// FS SPIR-V 产物的 capability 字面(字节机器解析:OpCapability=17;非人工声明)。
fn fs_spirv_caps() -> Vec<&'static str> {
    let words: Vec<u32> = HW_FS_SPV
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let mut caps = Vec::new();
    let mut i = 5usize;
    while i < words.len() {
        let wc = (words[i] >> 16) as usize;
        if wc == 0 {
            break;
        }
        if (words[i] & 0xffff) == 17 {
            caps.push(match words[i + 1] {
                1 => "Shader",
                11 => "Int64",
                12 => "Int64Atomics",
                _ => "Unknown",
            });
        }
        i += wc;
    }
    caps.sort_unstable();
    caps
}

/// 单帧执行:pass0 = SW compute → `vis_sw`;pass1 = 保守光栅 Raster(VS+FS)→
/// `vis_hw`(两 pass 写互斥 buffer,无跨 pass 依赖);readback 双 buffer。
fn execute_pair_frame(
    triangles: &[f32],
    ids: &[u32],
    vertex_bytes: &[u8],
) -> Result<(Vec<u64>, Vec<u64>), String> {
    let tri_count = (ids.len() / 2) as u32;
    let tris_b = bytes_f32(triangles);
    let ids_b = bytes_u32(ids);
    let clear = bytes_u64(&vec![VISBUFFER_CLEAR; VIS_WORDS]);
    let resources = [
        storage(tris_b.len(), Some(&tris_b)),
        storage(ids_b.len(), Some(&ids_b)),
        storage(clear.len(), Some(&clear)), // 2: vis_sw
        storage(clear.len(), Some(&clear)), // 3: vis_hw
        ResourceDesc::Texture(TextureDesc {
            width: VIS_W,
            height: VIS_H,
            format: TexFormat::Rgba8Unorm,
            usage: TextureUsage {
                color: true,
                ..Default::default()
            },
            data: None,
        }), // 4: dummy color(VisBuffer 走 SSBO;设计 §4.4 最小增量形态)
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "visbuffer_sw_u64",
            spirv: SW_SPV,
            entry: None,
            dispatch: DispatchSpec::Direct([tri_count * VIS_W * VIS_H, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![0, 1, 2],
                push_constants: bytes_u32(&[tri_count, VIS_W, VIS_H]),
                ..Default::default()
            },
        }),
        Pass::Raster(RasterPass {
            name: "visbuffer_hw_raster",
            vs_spirv: HW_VS_SPV,
            fs_spirv: HW_FS_SPV,
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

fn oracle_coverage_matches_sw(sw: &[u64], oracle: &[u64]) -> bool {
    if sw.len() != oracle.len() || sw.len() != VIS_WORDS {
        return false;
    }
    sw.iter()
        .zip(oracle)
        .all(|(&s, &o)| (s == VISBUFFER_CLEAR) == (o == VISBUFFER_CLEAR))
}

fn run_hw(scene: &Uc06Scene, tamper: Tamper) -> Option<Result<G75HwRasterResults, String>> {
    let caps = match hw_gate()? {
        Ok(caps) => caps,
        Err(e) => return Some(Err(e)),
    };
    Some(run_hw_inner(scene, &caps, tamper))
}

fn run_hw_inner(
    scene: &Uc06Scene,
    caps: &render_exec::DeviceCaps,
    tamper: Tamper,
) -> Result<G75HwRasterResults, String> {
    let (triangles, ids, oracle) = project_frozen_scene(scene);
    if ids.is_empty() {
        return Err("冻结场景剔除后零三角形(判据会空转)".to_owned());
    }
    let victim = if tamper == Tamper::None {
        usize::MAX
    } else {
        dominant_triangle(&oracle, &ids).ok_or("oracle 零覆盖,RED 轴无受害三角形(判据会空转)")?
    };
    let vertex_bytes = build_vertex_stream(&triangles, &ids, tamper, victim);
    let (sw, hw) = execute_pair_frame(&triangles, &ids, &vertex_bytes)?;

    let diff_pixels = sw.iter().zip(&hw).filter(|(a, b)| a != b).count() as u32;
    let covered = |v: &[u64]| v.iter().filter(|&&w| w != VISBUFFER_CLEAR).count() as u32;
    // host VisBufferCpu 与 device SW 在冻结场景下因驱动 FMA 收缩,depth30 可差
    // 数 ULP 并改写 atomicMax 胜者(G7.5 残差归因同构);覆盖集合仍精确相等。
    // G-G7-7 本体 = HW==SW(同 GPU FMA 世界,diff_pixels==0);oracle_bitexact =
    // 覆盖集合逐像素相等。全屏 SW↔host packed 逐位锚由 W2 合成场景 sw_baseline 承担。
    let oracle_bitexact = oracle_coverage_matches_sw(&sw, &oracle.data);
    Ok(G75HwRasterResults {
        device_name: caps.device_name.clone(),
        triangles: (ids.len() / 2) as u32,
        pixels: VIS_WORDS as u32,
        diff_pixels,
        hw_covered_words: covered(&hw),
        sw_covered_words: covered(&sw),
        oracle_bitexact,
        conservative_props: caps
            .conservative_raster
            .expect("hw_gate 已保证保守光栅在位"),
        pipeline: "vk-graphics-conservative-raster",
        spirv_caps: fs_spirv_caps(),
    })
}

/// 生产路径:零篡改,判据 = `all_pass`(diff==0 + 覆盖非退化 + oracle 覆盖集合对齐)。
pub fn run_g75_hw_raster(scene: &Uc06Scene) -> Option<Result<G75HwRasterResults, String>> {
    run_hw(scene, Tamper::None)
}

/// RED 轴 ①:篡改 winner 三角形 flat varying(交换 vb/vc → FS 判非正绕不写;
/// SSBO 输入不动)→ diff **必 > 0**。返回 `Some(Ok(diff_pixels))`。
pub fn red_tamper_varying(scene: &Uc06Scene) -> Option<Result<u32, String>> {
    run_hw(scene, Tamper::Varying).map(|r| r.map(|res| res.diff_pixels))
}

/// RED 轴 ②:篡改 winner 三角形 `ids.cluster + 1`(pack 值漂移)→ diff **必 > 0**。
pub fn red_tamper_ids(scene: &Uc06Scene) -> Option<Result<u32, String>> {
    run_hw(scene, Tamper::Ids).map(|r| r.map(|res| res.diff_pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_g75b_hw_raster_diff_zero() {
        let scene = crate::scene::build_scene();
        let Some(res) = run_g75_hw_raster(&scene) else {
            return; // dev-env degrade
        };
        let r = res.expect("G7.5b HW 光栅 device 执行(含保守光栅 fail-closed)");
        assert!(r.all_pass(), "SW/HW 整数域对拍未过: {}", r.json());
    }

    #[test]
    fn device_g75b_red_axes_nonzero() {
        let scene = crate::scene::build_scene();
        let Some(v) = red_tamper_varying(&scene) else {
            return; // dev-env degrade
        };
        let dv = v.expect("RED-varying 执行");
        assert!(dv > 0, "tamper-varying 后 diff 仍为 0(RED 轴失效)");
        let di = red_tamper_ids(&scene)
            .expect("同机应有 device")
            .expect("RED-ids 执行");
        assert!(di > 0, "tamper-ids 后 diff 仍为 0(RED 轴失效)");
    }
}
