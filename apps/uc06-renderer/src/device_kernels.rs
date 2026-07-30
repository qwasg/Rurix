//! RD-038 W1/W2 效果内核的 Vulkan 派发与 host 金标准对拍。

use rurix_rt::render_exec::{
    self, Bindings, BufferDesc, BufferUsage, ComputePass, DispatchSpec, KernelWave, Pass, Readback,
    ResourceDesc,
};

const CULL_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cull.spv"));
const VISBUFFER_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/visbuffer_sw_u64.spv"));
const CLASSIFY_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/classify_resolve.spv"));
const VSM_MARK_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vsm_page_mark.spv"));
const TAA_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/taa.spv"));

fn device_gate(wave: KernelWave) -> Option<render_exec::DeviceCaps> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("[uc06 device kernels] SKIP: vulkan loader 不可用(dev-env degrade)");
        return None;
    }
    let caps = render_exec::probe_device_caps().expect("Vulkan loader 存在时能力探测应成功");
    render_exec::require_wave(&caps, wave)
        .unwrap_or_else(|e| panic!("device kernel fail-closed: {e}"));
    Some(caps)
}

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

fn bytes_u32(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn bytes_u64(values: &[u64]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn bytes_f32(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn read_u32(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
        .collect()
}

fn read_u64(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(8)
        .map(|v| u64::from_le_bytes([v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]]))
        .collect()
}

fn read_f32(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|v| f32::from_le_bytes([v[0], v[1], v[2], v[3]]))
        .collect()
}

fn push_u32(values: &[u32]) -> Vec<u8> {
    bytes_u32(values)
}

use rurix_render::geometry::cull::{CullCamera, cluster_cull, instance_cull};
use rurix_render::geometry::gpu_scene::InstanceRecord;
use rurix_render::geometry::material_pass::{classify, resolve};
use rurix_render::geometry::visbuffer::{VISBUFFER_CLEAR, VisBufferCpu};
use rurix_render::graph::types::ClusterRecord;
use rurix_render::shadow::clipmap::ClipmapConfig;
use rurix_render::shadow::vsm::{Vsm, VsmConfig};
use rurix_render::temporal::common::{look_at_rh, perspective_rh_zo};
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::taa::{TaaParams, taa_resolve};

#[derive(Debug, Clone)]
pub struct KernelMatchResults {
    pub cull_visible_clusters: u32,
    pub visbuffer_matched_words: u32,
    pub classify_matched_pixels: u32,
    pub vsm_marked_pages: u32,
    pub taa_max_err: f32,
}

fn execute_compute(
    name: &'static str,
    spirv: &'static [u8],
    resources: &[ResourceDesc<'_>],
    storage_buffers: Vec<u32>,
    push_constants: Vec<u8>,
    dispatch_x: u32,
    readbacks: &[Readback],
) -> Vec<Vec<u8>> {
    let passes = [Pass::Compute(ComputePass {
        name,
        spirv,
        entry: None,
        dispatch: DispatchSpec::Direct([dispatch_x, 1, 1]),
        bindings: Bindings {
            storage_buffers,
            push_constants,
            ..Default::default()
        },
    })];
    let barriers: [&[(u32, render_exec::TargetState)]; 1] = [&[]];
    render_exec::execute_frame(resources, &passes, &barriers, readbacks)
        .unwrap_or_else(|e| panic!("{name} device dispatch: {e}"))
}

fn cull_camera() -> CullCamera {
    CullCamera {
        view_proj: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, -0.2],
            [0.0, 0.0, -1.0, 0.0],
        ],
        cam_pos: [0.0; 3],
        screen_height_px: 72.0,
        error_threshold_px: 1.0,
    }
}

fn instance(tx: f32, tz: f32, offset: u32, count: u32) -> InstanceRecord {
    InstanceRecord {
        transform: [
            [1.0, 0.0, 0.0, tx],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, tz],
        ],
        cluster_offset: offset,
        cluster_count: count,
        material_id: offset / count,
        flags: 0,
        aabb_min: [tx - 4.0, -4.0, tz - 2.0],
        mesh_id: 0,
        aabb_max: [tx + 4.0, 4.0, tz + 2.0],
        reserved: u32::MAX,
    }
}

fn cluster(center: [f32; 3]) -> ClusterRecord {
    ClusterRecord {
        center,
        radius: 0.4,
        cone_axis: [0.0, 0.0, 1.0],
        cone_cutoff: 2.0,
        error: 0.0,
        parent_error: f32::INFINITY,
        vertex_offset: 0,
        triangle_offset: 0,
        vertex_count: 3,
        triangle_count: 1,
        page_id: 0,
        reserved: 0,
    }
}

pub fn match_w1_cull() -> Option<u32> {
    let _caps = device_gate(KernelWave::W1)?;
    let per_instance = 40u32;
    let instances = [
        instance(0.0, -10.0, 0, per_instance),
        instance(100.0, -10.0, per_instance, per_instance),
        instance(0.0, -20.0, per_instance * 2, per_instance),
    ];
    let mut clusters = Vec::new();
    for i in 0..(per_instance * 3) {
        let local = i % per_instance;
        let x = if local.is_multiple_of(11) {
            80.0
        } else {
            (local % 9) as f32 * 0.7 - 2.8
        };
        clusters.push(cluster([x, (local % 5) as f32 * 0.4 - 0.8, 0.0]));
    }
    let cam = cull_camera();
    let visible_instances = instance_cull(&instances, &cam);
    let mut expected: Vec<(u32, u32)> =
        cluster_cull(&instances, &visible_instances, &clusters, &cam)
            .into_iter()
            .map(|v| (v.instance, v.cluster))
            .collect();
    expected.sort_unstable();

    let mut instance_of = Vec::new();
    let mut spheres = Vec::new();
    for (i, c) in clusters.iter().enumerate() {
        let inst = i as u32 / per_instance;
        instance_of.push(inst);
        let tr = instances[inst as usize].transform;
        spheres.extend_from_slice(&[
            c.center[0] + tr[0][3],
            c.center[1] + tr[1][3],
            c.center[2] + tr[2][3],
            c.radius,
        ]);
    }
    let mut aabbs = Vec::new();
    for i in &instances {
        aabbs.extend_from_slice(&i.aabb_min);
        aabbs.extend_from_slice(&i.aabb_max);
    }
    let instance_of_b = bytes_u32(&instance_of);
    let aabbs_b = bytes_f32(&aabbs);
    let spheres_b = bytes_f32(&spheres);
    let zero = 0u32.to_le_bytes();
    let pair_bytes = clusters.len() * 8;
    let resources = [
        storage(instance_of_b.len(), Some(&instance_of_b)),
        storage(aabbs_b.len(), Some(&aabbs_b)),
        storage(spheres_b.len(), Some(&spheres_b)),
        storage(4, Some(&zero)),
        storage(pair_bytes, None),
    ];
    let readbacks = [
        Readback::Buffer {
            res: 3,
            offset: 0,
            size: 4,
        },
        Readback::Buffer {
            res: 4,
            offset: 0,
            size: pair_bytes as u64,
        },
    ];
    let out = execute_compute(
        "cull",
        CULL_SPV,
        &resources,
        vec![0, 1, 2, 3, 4],
        push_u32(&[clusters.len() as u32]),
        clusters.len() as u32,
        &readbacks,
    );
    let count = read_u32(&out[0])[0] as usize;
    let words = read_u32(&out[1]);
    let mut got: Vec<(u32, u32)> = words[..count * 2]
        .chunks_exact(2)
        .map(|v| (v[0], v[1]))
        .collect();
    got.sort_unstable();
    assert_eq!(got, expected);
    eprintln!(
        "[uc06 device] cull clusters={} visible={} workgroups={}",
        clusters.len(),
        got.len(),
        clusters.len()
    );
    Some(got.len() as u32)
}

fn vis_scene() -> (Vec<f32>, Vec<u32>, VisBufferCpu) {
    let (w, h) = (128u32, 72u32);
    let mut triangles = Vec::new();
    let mut ids = Vec::new();
    let mut host = VisBufferCpu::new(w, h);
    for i in 0..80u32 {
        let x = (i % 10) as f32 * 12.0;
        let y = (i / 10) as f32 * 8.0;
        let z = match i % 3 {
            0 => 0.25,
            1 => 0.5,
            _ => 0.75,
        };
        let tri = [[x, y, z], [x, y + 7.0, z], [x + 10.0, y, z]];
        for v in tri {
            triangles.extend_from_slice(&v);
        }
        ids.extend_from_slice(&[i, i % 127]);
        host.raster_triangle(&tri, i, i % 127);
    }
    (triangles, ids, host)
}

pub fn match_w2_visbuffer_u64() -> Option<u32> {
    let caps = device_gate(KernelWave::W2)?;
    assert!(caps.shader_int64, "W2 设备还须支持核心 shaderInt64");
    let (triangles, ids, expected) = vis_scene();
    let triangles_b = bytes_f32(&triangles);
    let ids_b = bytes_u32(&ids);
    let initial = bytes_u64(&vec![VISBUFFER_CLEAR; 128 * 72]);
    let resources = [
        storage(triangles_b.len(), Some(&triangles_b)),
        storage(ids_b.len(), Some(&ids_b)),
        storage(initial.len(), Some(&initial)),
    ];
    let readbacks = [Readback::Buffer {
        res: 2,
        offset: 0,
        size: initial.len() as u64,
    }];
    let out = execute_compute(
        "visbuffer_sw_u64",
        VISBUFFER_SPV,
        &resources,
        vec![0, 1, 2],
        push_u32(&[80, 128, 72]),
        80 * 128 * 72,
        &readbacks,
    );
    let got = read_u64(&out[0]);
    assert_eq!(got, expected.data, "VisBuffer 必须逐位相等");
    eprintln!(
        "[uc06 device] visbuffer pixels={} covered={} triangles=80 workgroups={}",
        got.len(),
        expected.count_valid(),
        80 * 128 * 72
    );
    Some(got.len() as u32)
}

pub fn match_w1_classify_resolve() -> Option<u32> {
    let caps = device_gate(KernelWave::W1)?;
    assert!(caps.shader_int64, "classify u64 读取需要核心 shaderInt64");
    let (_, _, vis) = vis_scene();
    let c2m: Vec<u16> = (0..80).map(|i| (i % 8) as u16).collect();
    let expected_resolve = resolve(&vis, &c2m);
    let classified = classify(&vis, &c2m, 8);
    let mut expected_counts = [0u32; 16];
    for bucket in classified.buckets {
        expected_counts[bucket.material_slot as usize] += bucket.pixel_count;
    }
    let vis_b = bytes_u64(&vis.data);
    let c2m_u32: Vec<u32> = c2m.iter().map(|&v| u32::from(v)).collect();
    let c2m_b = bytes_u32(&c2m_u32);
    let zero_counts = bytes_u32(&[0u32; 16]);
    let out_size = vis.data.len() * 4;
    let resources = [
        storage(vis_b.len(), Some(&vis_b)),
        storage(c2m_b.len(), Some(&c2m_b)),
        storage(out_size, None),
        storage(zero_counts.len(), Some(&zero_counts)),
    ];
    let readbacks = [
        Readback::Buffer {
            res: 2,
            offset: 0,
            size: out_size as u64,
        },
        Readback::Buffer {
            res: 3,
            offset: 0,
            size: zero_counts.len() as u64,
        },
    ];
    let out = execute_compute(
        "classify_resolve",
        CLASSIFY_SPV,
        &resources,
        vec![0, 1, 2, 3],
        push_u32(&[vis.data.len() as u32]),
        vis.data.len() as u32,
        &readbacks,
    );
    let got_resolve = read_u32(&out[0]);
    assert!(
        got_resolve
            .iter()
            .zip(&expected_resolve)
            .all(|(&a, &b)| a == u32::from(b))
    );
    assert_eq!(read_u32(&out[1]), expected_counts);
    eprintln!(
        "[uc06 device] classify-resolve pixels={} buckets={} materials=8",
        vis.data.len(),
        classified.tile_offsets.last().copied().unwrap_or(0)
    );
    Some(vis.data.len() as u32)
}

pub fn match_w1_vsm_page_mark() -> Option<u32> {
    let _caps = device_gate(KernelWave::W1)?;
    let camera = [0.37, -0.61, 7.0];
    let proj = perspective_rh_zo(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
    let view = look_at_rh(camera, [camera[0], camera[1], 0.0], [0.0, 1.0, 0.0]);
    let vp = proj.mul(&view);
    let inv = vp.inverse().expect("camera matrix invertible");
    let depth = ImageF32::from_fn(2, 2, 1, |x, y, _| {
        let world = [
            camera[0] + (x as f32 - 0.5) * 7.0,
            camera[1] + (0.5 - y as f32) * 7.0,
            0.0,
        ];
        let clip = vp.transform_vec4([world[0], world[1], world[2], 1.0]);
        clip[2] / clip[3]
    });
    let cfg = VsmConfig {
        clip: ClipmapConfig {
            levels: 4,
            base_radius: 16.0,
            depth_extent: 16.0,
        },
        pool_pages: 8,
        depth_bias: 1e-3,
    };
    let mut host = Vsm::new(cfg, [0.0, 0.0, -1.0], camera);
    let stats = host.page_mark(&depth, &inv);
    assert_eq!((stats.pixels, stats.pages), (4, 4));
    let slots = [(11u32, 115u32), (11, 15), (111, 115), (111, 15)];
    for &(x, y) in &slots {
        assert!(host.is_marked(0, x as u8, y as u8));
    }
    let pages: Vec<u32> = slots.iter().map(|&(x, y)| y * 128 + x).collect();
    let pages_b = bytes_u32(&pages);
    let initial = bytes_u32(&[0u32; 2048]);
    let resources = [
        storage(pages_b.len(), Some(&pages_b)),
        storage(initial.len(), Some(&initial)),
    ];
    let readbacks = [Readback::Buffer {
        res: 1,
        offset: 0,
        size: initial.len() as u64,
    }];
    let out = execute_compute(
        "vsm_page_mark",
        VSM_MARK_SPV,
        &resources,
        vec![0, 1],
        push_u32(&[pages.len() as u32]),
        pages.len() as u32,
        &readbacks,
    );
    let got = read_u32(&out[0]);
    let mut expected = vec![0u32; 2048];
    for page in pages {
        expected[(page / 32) as usize] |= 1 << (page % 32);
    }
    assert_eq!(got, expected);
    eprintln!("[uc06 device] vsm mark pixels=4 unique_pages=4");
    Some(4)
}

pub fn match_w1_taa() -> Option<f32> {
    let _caps = device_gate(KernelWave::W1)?;
    let (w, h) = (32u32, 16u32);
    let current = ImageF32::from_fn(w, h, 3, |x, y, ch| {
        0.05 + x as f32 * 0.013 + y as f32 * 0.007 + ch as f32 * 0.11
    });
    let history = ImageF32::from_fn(w, h, 3, |x, y, ch| {
        0.8 - x as f32 * 0.009 + y as f32 * 0.003 - ch as f32 * 0.07
    });
    let motion = ImageF32::from_fn(w, h, 2, |x, _, ch| {
        if ch == 0 {
            if x.is_multiple_of(2) {
                0.25 / w as f32
            } else {
                -0.25 / w as f32
            }
        } else {
            0.25 / h as f32
        }
    });
    let validity = ImageF32::from_fn(
        w,
        h,
        1,
        |x, y, _| {
            if (x + y).is_multiple_of(17) { 0.0 } else { 1.0 }
        },
    );
    let params = TaaParams::default();
    let expected = taa_resolve(&current, &history, &motion, &validity, &params);
    let current_b = bytes_f32(&current.data);
    let history_b = bytes_f32(&history.data);
    let motion_b = bytes_f32(&motion.data);
    let validity_b = bytes_f32(&validity.data);
    let out_size = expected.data.len() * 4;
    let resources = [
        storage(current_b.len(), Some(&current_b)),
        storage(history_b.len(), Some(&history_b)),
        storage(motion_b.len(), Some(&motion_b)),
        storage(validity_b.len(), Some(&validity_b)),
        storage(out_size, None),
    ];
    let mut push = push_u32(&[w, h]);
    push.extend_from_slice(&params.blend_alpha.to_le_bytes());
    let readbacks = [Readback::Buffer {
        res: 4,
        offset: 0,
        size: out_size as u64,
    }];
    let out = execute_compute(
        "taa",
        TAA_SPV,
        &resources,
        vec![0, 1, 2, 3, 4],
        push,
        w * h,
        &readbacks,
    );
    let got = read_f32(&out[0]);
    let max_error = got
        .iter()
        .zip(&expected.data)
        .map(|(&a, &b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(max_error <= 1e-5, "TAA max error {max_error} > 1e-5");
    eprintln!(
        "[uc06 device] taa pixels={} channels={} max_abs_error={max_error:.8}",
        w * h,
        expected.data.len()
    );
    Some(max_error)
}

pub fn run_all_matches() -> Option<KernelMatchResults> {
    Some(KernelMatchResults {
        cull_visible_clusters: match_w1_cull()?,
        visbuffer_matched_words: match_w2_visbuffer_u64()?,
        classify_matched_pixels: match_w1_classify_resolve()?,
        vsm_marked_pages: match_w1_vsm_page_mark()?,
        taa_max_err: match_w1_taa()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_w1_cull_matches_host() {
        let Some(visible) = match_w1_cull() else {
            return;
        };
        assert!(visible > 0);
    }

    #[test]
    fn device_w2_visbuffer_u64_bitexact_host() {
        let Some(words) = match_w2_visbuffer_u64() else {
            return;
        };
        assert_eq!(words, 128 * 72);
    }

    #[test]
    fn device_w1_classify_resolve_matches_host() {
        let Some(pixels) = match_w1_classify_resolve() else {
            return;
        };
        assert_eq!(pixels, 128 * 72);
    }

    #[test]
    fn device_w1_vsm_page_mark_matches_host() {
        let Some(pages) = match_w1_vsm_page_mark() else {
            return;
        };
        assert_eq!(pages, 4);
    }

    #[test]
    fn device_w1_taa_matches_host() {
        let Some(max_err) = match_w1_taa() else {
            return;
        };
        assert!(max_err <= 1e-5);
    }
}
